use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::impl_u8_enum;
use erg_common::traits::Locational;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::token::TokenKind;
use erg_compiler::ty::Type;
use erg_compiler::varinfo::AbsLocation;
use erg_compiler::AccessKind;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, Documentation, MarkedString,
    MarkupContent, MarkupKind,
};

use crate::server::{ELSResult, Server};
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

impl_u8_enum! { CompletionOrder;
    TypeMatched,
    SingularAttr,
    Normal,
    Escaped,
    DoubleEscaped,
}

impl CompletionOrder {
    pub fn from_label(label: &str) -> Self {
        if label.starts_with("__") {
            Self::DoubleEscaped
        } else if label.starts_with('_') {
            Self::Escaped
        } else {
            Self::Normal
        }
    }

    pub fn mangle(&self, label: &str) -> String {
        format!("{}_{label}", u8::from(*self))
    }

    fn set(item: &mut CompletionItem) {
        let order = Self::from_label(&item.label[..]);
        item.sort_text = Some(order.mangle(&item.label));
    }
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_completion(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("completion requested: {msg}"))?;
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
            CompletionOrder::set(&mut item);
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
        Self::send_log(format!("completion items: {}", result.len()))?;
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }

    pub(crate) fn resolve_completion(&self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("completion resolve requested: {msg}"))?;
        let mut item = CompletionItem::deserialize(&msg["params"])?;
        if let Some(data) = &item.data {
            let mut contents = vec![];
            let Ok(def_loc) = data.as_str().unwrap().parse::<AbsLocation>() else {
                return Self::send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": item }));
            };
            self.show_doc_comment(None, &mut contents, &def_loc)?;
            let mut contents = contents.into_iter().map(mark_to_string).collect::<Vec<_>>();
            contents.sort_by_key(|cont| markdown_order(cont));
            item.documentation = Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: contents.join("\n"),
            }));
        }
        Self::send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": item }))
    }
}
