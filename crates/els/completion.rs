use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::impl_u8_enum;
use erg_common::traits::Locational;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::context::Context;
use erg_compiler::erg_parser::token::TokenKind;
use erg_compiler::hir::Expr;
use erg_compiler::ty::{HasType, ParamTy, Type};
use erg_compiler::varinfo::{AbsLocation, VarInfo};
use erg_compiler::AccessKind;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, Documentation, MarkedString,
    MarkupContent, MarkupKind,
};

use crate::server::{send, send_log, ELSResult, Server};
use crate::util;

fn mark_to_string(mark: MarkedString) -> String {
    match mark {
        MarkedString::String(s) => s,
        MarkedString::LanguageString(ls) => format!("```{}\n{}\n```", ls.language, ls.value),
    }
}

fn markdown_order(block: &str) -> usize {
    if block.starts_with("```") {
        usize::MAX
    } else {
        0
    }
}

impl_u8_enum! { CompletionOrder; i32;
    TypeMatched = -8,
    NameMatched = -2,
    Normal = 1000000,
    Builtin = 1,
    Escaped = 4,
    DoubleEscaped = 16,
}

pub struct CompletionOrderSetter<'b> {
    vi: &'b VarInfo,
    arg_pt: Option<&'b ParamTy>,
    mod_ctx: &'b Context, // for subtype judgement, not for variable lookup
    label: String,
}

impl<'b> CompletionOrderSetter<'b> {
    pub fn new(
        vi: &'b VarInfo,
        arg_pt: Option<&'b ParamTy>,
        mod_ctx: &'b Context,
        label: String,
    ) -> Self {
        Self {
            vi,
            arg_pt,
            mod_ctx,
            label,
        }
    }

    pub fn score(&self) -> i32 {
        let mut orders = vec![CompletionOrder::Normal];
        if self.label.starts_with("__") {
            orders.push(CompletionOrder::DoubleEscaped);
        } else if self.label.starts_with('_') {
            orders.push(CompletionOrder::Escaped);
        }
        if self.vi.kind.is_builtin() {
            orders.push(CompletionOrder::Builtin);
        }
        if self
            .arg_pt
            .map_or(false, |pt| pt.name().map(|s| &s[..]) == Some(&self.label))
        {
            orders.push(CompletionOrder::NameMatched);
        }
        if self.arg_pt.map_or(false, |pt| {
            self.mod_ctx.subtype_of(&self.vi.t, pt.typ(), true)
        }) {
            orders.push(CompletionOrder::TypeMatched);
        }
        orders.into_iter().map(i32::from).sum()
    }

    pub fn mangle(&self) -> String {
        let score = self.score();
        format!("{}_{}", char::from_u32(score as u32).unwrap(), self.label)
    }

    fn set(&self, item: &mut CompletionItem) {
        item.sort_text = Some(self.mangle());
    }
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_completion(&mut self, msg: &Value) -> ELSResult<()> {
        send_log(format!("completion requested: {msg}"))?;
        let params = CompletionParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position.text_document.uri);
        let path = util::uri_to_path(&uri);
        let pos = params.text_document_position.position;
        let trigger = params
            .context
            .as_ref()
            .and_then(|comp_ctx| comp_ctx.trigger_character.as_ref().map(|s| &s[..]));
        let acc_kind = match trigger {
            Some(".") => AccessKind::Attr,
            Some(":") => AccessKind::Attr, // or type ascription
            _ => AccessKind::Name,
        };
        send_log(format!("AccessKind: {acc_kind:?}"))?;
        let mut result: Vec<CompletionItem> = vec![];
        let contexts = if acc_kind.is_local() {
            let prev_token = self.file_cache.get_token_relatively(&uri, pos, -1)?;
            if prev_token
                .as_ref()
                .map(|t| t.is(TokenKind::Dot) || t.is(TokenKind::DblColon))
                .unwrap_or(false)
            {
                let dot_pos = util::loc_to_pos(prev_token.unwrap().loc()).unwrap();
                self.get_receiver_ctxs(&uri, dot_pos)?
            } else {
                self.get_local_ctx(&uri, pos)
            }
        } else {
            self.get_receiver_ctxs(&uri, pos)?
        };
        let arg_pt = self.get_min_expr(&uri, pos, -1).and_then(|(token, expr)| {
            if let Expr::Call(call) = expr {
                let sig_t = call.obj.t();
                let nth = self.nth(&uri, call.args.loc(), &token);
                sig_t.non_default_params()?.get(nth).cloned()
            } else {
                None
            }
        });
        let mod_ctx = &self.modules.get(&uri).unwrap().context;
        for (name, vi) in contexts.into_iter().flat_map(|ctx| ctx.dir()) {
            if acc_kind.is_attr() && vi.vis.is_private() {
                continue;
            }
            // don't show overriden items
            if result
                .iter()
                .any(|item| item.label[..] == name.inspect()[..])
            {
                continue;
            }
            // don't show future defined items
            if vi.def_loc.module.as_ref() == Some(&path)
                && name.ln_begin().unwrap_or(0) > pos.line + 1
            {
                continue;
            }
            let readable_t = self
                .modules
                .get(&uri)
                .map(|module| {
                    module
                        .context
                        .readable_type(vi.t.clone(), vi.kind.is_parameter())
                })
                .unwrap_or_else(|| vi.t.clone());
            let mut item = CompletionItem::new_simple(name.to_string(), readable_t.to_string());
            CompletionOrderSetter::new(vi, arg_pt.as_ref(), mod_ctx, item.label.clone())
                .set(&mut item);
            item.kind = match &vi.t {
                Type::Subr(subr) if subr.self_t().is_some() => Some(CompletionItemKind::METHOD),
                Type::Quantified(quant) if quant.self_t().is_some() => {
                    Some(CompletionItemKind::METHOD)
                }
                Type::Subr(_) | Type::Quantified(_) => Some(CompletionItemKind::FUNCTION),
                Type::ClassType => Some(CompletionItemKind::CLASS),
                Type::TraitType => Some(CompletionItemKind::INTERFACE),
                t if &t.qual_name()[..] == "Module" || &t.qual_name()[..] == "GenericModule" => {
                    Some(CompletionItemKind::MODULE)
                }
                _ if vi.muty.is_const() => Some(CompletionItemKind::CONSTANT),
                _ => Some(CompletionItemKind::VARIABLE),
            };
            item.data = Some(Value::String(vi.def_loc.to_string()));
            result.push(item);
        }
        send_log(format!("completion items: {}", result.len()))?;
        send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }))
    }

    pub(crate) fn resolve_completion(&self, msg: &Value) -> ELSResult<()> {
        send_log(format!("completion resolve requested: {msg}"))?;
        let mut item = CompletionItem::deserialize(&msg["params"])?;
        if let Some(data) = &item.data {
            let mut contents = vec![];
            let Ok(def_loc) = data.as_str().unwrap().parse::<AbsLocation>() else {
                return send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": item }));
            };
            self.show_doc_comment(None, &mut contents, &def_loc)?;
            let mut contents = contents.into_iter().map(mark_to_string).collect::<Vec<_>>();
            contents.sort_by_key(|cont| markdown_order(cont));
            item.documentation = Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: contents.join("\n"),
            }));
        }
        send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": item }))
    }
}
