use erg_common::traits::{DequeStream, LimitedDisplay, Locational, NoTypeDisplay};
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::erg_parser::token::{Token, TokenCategory, TokenKind};
use erg_compiler::hir::{Call, Expr};
use erg_compiler::ty::{HasType, ParamTy};

use lsp_types::{
    ParameterInformation, ParameterLabel, Position, SignatureHelp, SignatureHelpContext,
    SignatureHelpParams, SignatureHelpTriggerKind, SignatureInformation,
};

use crate::hir_visitor::GetExprKind;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::{loc_to_pos, pos_to_loc, NormalizedUrl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Paren,
    Comma,
    VBar, // e.g. id|T := Int|
}

impl From<String> for Trigger {
    fn from(s: String) -> Self {
        match s.as_str() {
            "(" => Trigger::Paren,
            "," => Trigger::Comma,
            "|" => Trigger::VBar,
            _ => unreachable!(),
        }
    }
}

pub enum ParamKind {
    NonDefault,
    VarArgs,
    Default,
}

fn get_end(start: usize, pt: &ParamTy, kind: ParamKind) -> usize {
    let pad = match kind {
        ParamKind::NonDefault => 2, // 2: `: `
        ParamKind::VarArgs => 3,    // 3: `*{name}: `
        ParamKind::Default => 4,    // 4: ` := `
    };
    start + pt.name().map(|n| n.len() + pad).unwrap_or(0) + pt.typ().to_string_unabbreviated().len()
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_signature_help(
        &mut self,
        params: SignatureHelpParams,
    ) -> ELSResult<Option<SignatureHelp>> {
        self.send_log(format!("signature help requested: {params:?}"))?;
        let uri = NormalizedUrl::new(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        if matches!(
            params.context.as_ref().map(|ctx| &ctx.trigger_kind),
            Some(&SignatureHelpTriggerKind::CONTENT_CHANGE | &SignatureHelpTriggerKind::INVOKED)
        ) {
            let Some(ctx) = params.context.as_ref() else {
                return Ok(None);
            };
            let help = self.resend_help(&uri, pos, ctx);
            return Ok(help);
        }
        let trigger = params
            .context
            .and_then(|c| c.trigger_character)
            .map(Trigger::from);
        let result = match trigger {
            Some(Trigger::Paren) => self.get_first_help(&uri, pos),
            Some(Trigger::Comma) => self.get_continuous_help(&uri, pos),
            Some(Trigger::VBar) | None => None,
        };
        Ok(result)
    }

    pub(crate) fn get_min_expr(
        &self,
        uri: &NormalizedUrl,
        pos: Position,
        offset: isize,
    ) -> Option<(Token, Expr)> {
        let token = self.file_cache.get_token_relatively(uri, pos, offset)?;
        crate::_log!(self, "token: {token}");
        if let Some(visitor) = self.get_visitor(uri) {
            if let Some(expr) = visitor.get_min_expr(loc_to_pos(token.loc())?) {
                return Some((token, expr.clone()));
            }
        }
        None
    }

    pub(crate) fn get_min<E: TryFrom<Expr> + GetExprKind>(
        &self,
        uri: &NormalizedUrl,
        pos: Position,
    ) -> Option<E> {
        self.get_searcher(uri, E::KIND)
            .and_then(|visitor| visitor.get_min_expr(pos).cloned())
            .and_then(|expr| E::try_from(expr).ok())
    }

    pub(crate) fn nth(&self, uri: &NormalizedUrl, call: &Call, pos: Position) -> usize {
        let origin_loc = call
            .args
            .paren
            .as_ref()
            .map(|(l, _)| l.loc())
            .unwrap_or_else(|| call.obj.loc());
        let loc = pos_to_loc(pos);
        let tks = self.file_cache.get_token_stream(uri).unwrap_or_default();
        let mut paren = 0usize;
        // we should use the latest commas
        let commas = tks
            .iter()
            .skip_while(|&tk| tk.loc() <= origin_loc)
            .filter(|tk| {
                // skip `,` of [1, ...]
                match tk.category() {
                    TokenCategory::LEnclosure => {
                        paren += 1;
                    }
                    TokenCategory::REnclosure => {
                        paren = paren.saturating_sub(1);
                    }
                    _ => {}
                }
                paren == 0 && tk.is(TokenKind::Comma) && tk.loc() <= loc
            })
            .collect::<Vec<_>>();
        commas.len()
    }

    fn resend_help(
        &mut self,
        uri: &NormalizedUrl,
        pos: Position,
        ctx: &SignatureHelpContext,
    ) -> Option<SignatureHelp> {
        if let Some(token) = self.file_cache.get_token(uri, pos) {
            crate::_log!(self, "token: {token}");
            if let Some(call) = self.get_min::<Call>(uri, pos) {
                if call.ln_begin() > token.ln_begin() || call.ln_end() < token.ln_end() {
                    return None;
                }
                let nth = self.nth(uri, &call, pos) as u32;
                return self.make_sig_help(call.obj.as_ref(), nth);
            } else {
                crate::_log!(self, "failed to get the call");
            }
        } else {
            crate::_log!(self, "failed to get the token");
        }
        ctx.active_signature_help.clone()
    }

    fn get_first_help(&mut self, uri: &NormalizedUrl, pos: Position) -> Option<SignatureHelp> {
        if let Some((_token, Expr::Accessor(acc))) = self.get_min_expr(uri, pos, -2) {
            return self.make_sig_help(&acc, 0);
        } else {
            crate::_log!(self, "lex error occurred");
        }
        None
    }

    fn get_continuous_help(&mut self, uri: &NormalizedUrl, pos: Position) -> Option<SignatureHelp> {
        if let Some(call) = self.get_min::<Call>(uri, pos) {
            let nth = self.nth(uri, &call, pos) as u32 + 1;
            let help = self.make_sig_help(call.obj.as_ref(), nth);
            return help;
        } else {
            crate::_log!(self, "failed to get continuous help");
        }
        None
    }

    fn make_sig_help<S: HasType + NoTypeDisplay>(
        &self,
        sig: &S,
        nth: u32,
    ) -> Option<SignatureHelp> {
        let sig_t = sig.ref_t();
        let mut parameters = vec![];
        let sig = sig.to_string_notype();
        let label = format!("{sig}: {}", sig_t.to_string_unabbreviated());
        let mut end = sig.len() + 1; // +1: (
        for nd_param in sig_t.non_default_params()? {
            let start = end + 2;
            end = get_end(start, nd_param, ParamKind::NonDefault);
            let param_info = ParameterInformation {
                label: ParameterLabel::LabelOffsets([start as u32, end as u32]),
                documentation: None, //Some(Documentation::String(nd_param.typ().to_string())),
            };
            parameters.push(param_info);
        }
        if let Some(var_params) = sig_t.var_params() {
            let start = end + 2;
            end = get_end(start, var_params, ParamKind::VarArgs);
            let param_info = ParameterInformation {
                label: ParameterLabel::LabelOffsets([start as u32, end as u32]),
                documentation: None, //Some(Documentation::String(var_params.typ().to_string())),
            };
            parameters.push(param_info);
        }
        let var_args_nth = if sig_t.var_params().is_some() {
            sig_t.non_default_params()?.len() as u32
        } else {
            u32::MAX
        };
        for d_params in sig_t.default_params()? {
            let start = end + 2;
            end = get_end(start, d_params, ParamKind::Default);
            let param_info = ParameterInformation {
                label: ParameterLabel::LabelOffsets([start as u32, end as u32]),
                documentation: None, //Some(Documentation::String(d_params.typ().to_string())),
            };
            parameters.push(param_info);
        }
        let nth = (parameters.len().saturating_sub(1) as u32)
            .min(nth)
            .min(var_args_nth);
        let info = SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: Some(nth),
        };
        Some(SignatureHelp {
            signatures: vec![info],
            active_parameter: None,
            active_signature: None,
        })
    }
}
