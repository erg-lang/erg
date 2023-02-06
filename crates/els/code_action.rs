use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::traits::Locational;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::hir::Expr;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, Diagnostic, TextEdit, Url, WorkspaceEdit,
};

use crate::server::{ELSResult, Server};
use crate::util;

impl<Checker: BuildRunnable> Server<Checker> {
    fn perform_eliminate_unused_vars_action(
        &self,
        _msg: &Value,
        uri: &Url,
        diag: Diagnostic,
    ) -> ELSResult<Vec<CodeAction>> {
        let mut map = HashMap::new();
        let Some(visitor) = self.get_visitor(uri) else {
            Self::send_log("visitor not found")?;
            return Ok(vec![]);
        };
        let Some(expr) = visitor.get_min_expr(diag.range) else {
            Self::send_log("expr not found")?;
            return Ok(vec![]);
        };
        if let Expr::Def(def) = expr {
            let mut range = util::loc_to_range(def.loc()).unwrap();
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
            let code = util::get_ranged_code_from_uri(uri, next)?;
            match code.as_ref().map(|s| &s[..]) {
                None => {
                    range.end.line += 1;
                    range.end.character = 0;
                }
                Some(";") => range.end.character += 1,
                Some(other) => {
                    Self::send_log(format!("? {other}"))?;
                }
            }
            let edit = TextEdit::new(range, "".to_string());
            map.insert(uri.clone(), vec![edit]);
            let edit = WorkspaceEdit::new(map);
            let action = CodeAction {
                title: "Eliminate unused variables".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag]),
                edit: Some(edit),
                ..Default::default()
            };
            Ok(vec![action])
        } else {
            Ok(vec![])
        }
    }

    pub(crate) fn perform_code_action(&self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("code action requested: {msg}"))?;
        let params = CodeActionParams::deserialize(&msg["params"])?;
        let result = match params
            .context
            .only
            .as_ref()
            .and_then(|kinds| kinds.first().map(|s| s.as_str()))
        {
            Some("erg.eliminate_unused_vars") => self.perform_quick_fix(msg, params)?,
            Some("quickfix") => self.perform_quick_fix(msg, params)?,
            _ => {
                Self::send_log("Unknown code action requested")?;
                vec![]
            }
        };
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }

    fn perform_quick_fix(
        &self,
        msg: &Value,
        params: CodeActionParams,
    ) -> ELSResult<Vec<CodeAction>> {
        let mut result: Vec<CodeAction> = vec![];
        let uri = util::normalize_url(params.text_document.uri);
        for diag in params.context.diagnostics.into_iter() {
            match &diag.message {
                unused if unused.ends_with("is not used") => {
                    result.extend(self.perform_eliminate_unused_vars_action(msg, &uri, diag)?);
                }
                _ => {
                    Self::send_log(&format!("Unknown diagnostic for action: {}", diag.message))?;
                }
            }
        }
        Ok(result)
    }
}
