use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::traits::Locational;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::token::TokenKind;
use erg_compiler::ty::Type;
use erg_compiler::AccessKind;

use lsp_types::{CompletionItem, CompletionItemKind, CompletionParams};

use crate::server::{ELSResult, Server};
use crate::util;

fn set_completion_order(item: &mut CompletionItem) {
    let label = &item.label[..];
    // HACK: let escaped symbols come at the bottom when sorting
    if label.starts_with("__") {
        item.sort_text = Some(format!("\u{10FFFF}_{label}"));
    } else if label.starts_with('_') {
        item.sort_text = Some(format!("\u{10FFFE}_{label}"));
    }
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_completion(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("completion requested: {msg}"))?;
        let params = CompletionParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position.text_document.uri);
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
        Self::send_log(format!("AccessKind: {acc_kind:?}"))?;
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
        // Self::send_log(format!("contexts: {:?}", contexts.iter().map(|ctx| &ctx.name).collect::<Vec<_>>())).unwrap();
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
            if name.ln_begin().unwrap_or(0) > pos.line + 1 {
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
            set_completion_order(&mut item);
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
            result.push(item);
        }
        Self::send_log(format!("completion items: {}", result.len()))?;
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }

    pub(crate) fn resolve_completion(&self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("completion resolve requested: {msg}"))?;
        let item = CompletionItem::deserialize(&msg["params"])?;
        Self::send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": item }))
    }
}
