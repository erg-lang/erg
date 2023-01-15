use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::traits::Locational;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::ty::Type;
use erg_compiler::AccessKind;

use lsp_types::{CompletionItem, CompletionItemKind, CompletionParams};

use crate::server::{ELSResult, Server};
use crate::util;

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
            self.get_local_ctx(&uri, pos)
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
}
