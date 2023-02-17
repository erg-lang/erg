use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::traits::Locational;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::token::{Token, TokenKind};
use erg_compiler::hir::Expr;

use lsp_types::{CodeAction, CodeActionKind, CodeActionParams, TextEdit, Url, WorkspaceEdit};

use crate::server::{ELSResult, Server};
use crate::util;

impl<Checker: BuildRunnable> Server<Checker> {
    fn gen_eliminate_unused_vars_action(
        &self,
        _msg: &Value,
        params: CodeActionParams,
    ) -> ELSResult<Option<CodeAction>> {
        let uri = util::normalize_url(params.text_document.uri);
        let mut diags = params.context.diagnostics;
        let diag = diags.remove(0);
        let mut map = HashMap::new();
        let Some(visitor) = self.get_visitor(&uri) else {
            Self::send_log("visitor not found")?;
            return Ok(None);
        };
        let Some(artifact) = self.artifacts.get(&uri) else {
            Self::send_log("artifact not found")?;
            return Ok(None);
        };
        let warns = artifact
            .warns
            .iter()
            .filter(|warn| warn.core.main_message.ends_with("is not used"))
            .collect::<Vec<_>>();
        for warn in warns {
            let uri = util::normalize_url(Url::from_file_path(warn.input.full_path()).unwrap());
            let Some(token) = self.file_cache.get_token(&uri, util::loc_to_pos(warn.core.loc).unwrap()) else {
                continue;
            };
            match visitor.get_min_expr(&token) {
                Some(Expr::Def(def)) => {
                    let Some(mut range) = util::loc_to_range(def.loc()) else {
                        Self::send_log("range not found")?;
                        continue;
                    };
                    let next = lsp_types::Range {
                        start: lsp_types::Position {
                            line: range.end.line,
                            character: range.end.character,
                        },
                        end: lsp_types::Position {
                            line: range.end.line,
                            character: range.end.character + 1,
                        },
                    };
                    let code = util::get_ranged_code_from_uri(&uri, next)?;
                    match code.as_ref().map(|s| &s[..]) {
                        None => {
                            // \n
                            range.end.line += 1;
                            range.end.character = 0;
                        }
                        Some(";") => range.end.character += 1,
                        Some(other) => {
                            Self::send_log(format!("? {other}"))?;
                        }
                    }
                    let edit = TextEdit::new(range, "".to_string());
                    map.entry(uri.clone()).or_insert(vec![]).push(edit);
                }
                Some(_) => {}
                None => {
                    let Some(token) = self.file_cache.get_token(&uri, diag.range.start) else {
                        continue;
                    };
                    let Some(vi) = visitor.get_info(&token) else {
                        continue;
                    };
                    if vi.kind.is_parameter() {
                        let edit = TextEdit::new(diag.range, "_".to_string());
                        map.entry(uri.clone()).or_insert(vec![]).push(edit);
                    }
                }
            }
        }
        let edit = WorkspaceEdit::new(map);
        let action = CodeAction {
            title: "Eliminate unused variables".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diag]),
            edit: Some(edit),
            ..Default::default()
        };
        Ok(Some(action))
    }

    fn gen_change_case_action(
        &self,
        token: Token,
        uri: &Url,
        params: CodeActionParams,
    ) -> Option<CodeAction> {
        let new_text = token.content.to_snake_case().to_string();
        let mut map = HashMap::new();
        let visitor = self.get_visitor(uri)?;
        let def_loc = visitor.get_info(&token)?.def_loc;
        let edit = TextEdit::new(util::loc_to_range(def_loc.loc).unwrap(), new_text.clone());
        map.insert(uri.clone(), vec![edit]);
        if let Some(value) = self.get_index().get_refs(&def_loc) {
            for refer in value.referrers.iter() {
                let url = Url::from_file_path(refer.module.as_ref().unwrap()).unwrap();
                let range = util::loc_to_range(refer.loc).unwrap();
                let edit = TextEdit::new(range, new_text.clone());
                map.entry(url).or_insert(vec![]).push(edit);
            }
        }
        let edit = WorkspaceEdit::new(map);
        let action = CodeAction {
            title: format!("Convert to snake case ({} -> {})", token.content, new_text),
            kind: Some(CodeActionKind::REFACTOR),
            diagnostics: Some(params.context.diagnostics),
            edit: Some(edit),
            ..Default::default()
        };
        Some(action)
    }

    fn send_normal_action(
        &self,
        msg: &Value,
        params: CodeActionParams,
    ) -> ELSResult<Vec<CodeAction>> {
        let mut actions = vec![];
        let uri = util::normalize_url(params.text_document.uri.clone());
        if let Some(token) = self.file_cache.get_token(&uri, params.range.start) {
            if token.is(TokenKind::Symbol) && !token.is_const() && !token.content.is_snake_case() {
                let action = self.gen_change_case_action(token, &uri, params.clone());
                actions.extend(action);
            }
        }
        actions.extend(self.send_quick_fix(msg, params)?);
        Ok(actions)
    }

    fn send_quick_fix(&self, msg: &Value, params: CodeActionParams) -> ELSResult<Vec<CodeAction>> {
        let mut result: Vec<CodeAction> = vec![];
        let diags = &params.context.diagnostics;
        if diags.is_empty() {
            return Ok(result);
        }
        if diags.first().unwrap().message.ends_with("is not used") {
            let actions = self.gen_eliminate_unused_vars_action(msg, params)?;
            result.extend(actions);
        }
        Ok(result)
    }

    pub(crate) fn send_code_action(&self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("code action requested: {msg}"))?;
        let params = CodeActionParams::deserialize(&msg["params"])?;
        let result = match params
            .context
            .only
            .as_ref()
            .and_then(|kinds| kinds.first().map(|s| s.as_str()))
        {
            Some("quickfix") => self.send_quick_fix(msg, params)?,
            None => self.send_normal_action(msg, params)?,
            Some(other) => {
                Self::send_log(&format!("Unknown code action requested: {other}"))?;
                vec![]
            }
        };
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }
}
