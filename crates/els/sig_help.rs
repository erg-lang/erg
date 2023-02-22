use erg_common::traits::NoTypeDisplay;
use erg_compiler::ty::HasType;
use lsp_types::Position;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::hir::Expr;

use lsp_types::{
    ParameterInformation, ParameterLabel, SignatureHelp, SignatureHelpParams, SignatureInformation,
    Url,
};

use crate::server::{ELSResult, Server};
use crate::util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Paren,
    Comma,
    VBar, // e.g. id|T := Int|
}

impl From<String> for Trigger {
    fn from(s: String) -> Self {
        match s.as_str() {
            "(" | ")" => Trigger::Paren,
            "," => Trigger::Comma,
            "|" => Trigger::VBar,
            _ => unreachable!(),
        }
    }
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_signature_help(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("signature help requested: {msg}"))?;
        let params = SignatureHelpParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        let trigger = params
            .context
            .and_then(|c| c.trigger_character)
            .map(Trigger::from);
        let result = match trigger {
            Some(Trigger::Paren) => self.get_first_help(&uri, pos),
            Some(Trigger::Comma) => self.get_continuous_help(&uri, pos),
            Some(Trigger::VBar) | None => None,
        };
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }

    fn get_first_help(&self, uri: &Url, pos: Position) -> Option<SignatureHelp> {
        if let Some(token) = self.file_cache.get_token_relatively(uri, pos, -1).ok()? {
            if let Some(visitor) = self.get_visitor(uri) {
                match visitor.get_min_expr(&token) {
                    Some(Expr::Call(call)) => {
                        let sig_t = call.signature_t().unwrap();
                        Self::send_log(format!("t: {sig_t}")).unwrap();
                    }
                    Some(Expr::Accessor(acc)) => {
                        let sig_t = acc.ref_t();
                        let mut parameters = vec![];
                        for nd_param in sig_t.non_default_params()? {
                            let param_info = ParameterInformation {
                                label: ParameterLabel::Simple(
                                    nd_param.name().map_or("".to_string(), |s| s.to_string()),
                                ),
                                documentation: None, //Some(Documentation::String(nd_param.typ().to_string())),
                            };
                            parameters.push(param_info);
                        }
                        if let Some(var_params) = sig_t.var_params() {
                            let param_info = ParameterInformation {
                                label: ParameterLabel::Simple(
                                    var_params.name().map_or("".to_string(), |s| s.to_string()),
                                ),
                                documentation: None, //Some(Documentation::String(var_params.typ().to_string())),
                            };
                            parameters.push(param_info);
                        }
                        let info = SignatureInformation {
                            label: format!("{}: {sig_t}", acc.to_string_notype()),
                            documentation: None,
                            parameters: Some(parameters),
                            active_parameter: Some(0),
                        };
                        return Some(SignatureHelp {
                            signatures: vec![info],
                            active_parameter: None,
                            active_signature: None,
                        });
                    }
                    Some(other) => {
                        Self::send_log(format!("other: {other}")).unwrap();
                    }
                    _ => {}
                }
            }
        } else {
            Self::send_log("lex error occurred").unwrap();
        }
        None
    }

    fn get_continuous_help(&self, uri: &Url, pos: Position) -> Option<SignatureHelp> {
        if let Some(token) = self.file_cache.get_token(uri, pos) {
            Self::send_log(format!("comma?: {token}")).unwrap();
            if let Some(visitor) = self.get_visitor(uri) {
                #[allow(clippy::single_match)]
                match visitor.get_min_expr(&token) {
                    Some(Expr::Call(call)) => {
                        Self::send_log(format!("call: {call}")).unwrap();
                    }
                    _ => {}
                }
            }
        } else {
            Self::send_log("lex error occurred").unwrap();
        }
        None
    }
}
