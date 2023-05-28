use std::collections::HashMap;

use erg_common::consts::ERG_MODE;
use erg_common::deepen_indent;
use erg_common::traits::Locational;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::token::{Token, TokenKind};
use erg_compiler::hir::Expr;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    Position, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::server::{send_log, ELSResult, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable> Server<Checker> {
    fn gen_eliminate_unused_vars_action(
        &self,
        params: &CodeActionParams,
    ) -> ELSResult<Option<CodeAction>> {
        let uri = NormalizedUrl::new(params.text_document.uri.clone());
        let diags = &params.context.diagnostics;
        let Some(diag) = diags.get(0).cloned() else {
            return Ok(None);
        };
        let mut map = HashMap::new();
        let Some(visitor) = self.get_visitor(&uri) else {
            send_log("visitor not found")?;
            return Ok(None);
        };
        let Some(result) = self.analysis_result.get(&uri) else {
            send_log("artifact not found")?;
            return Ok(None);
        };
        let warns = result
            .artifact
            .warns
            .iter()
            .filter(|warn| warn.core.main_message.ends_with("is not used"))
            .collect::<Vec<_>>();
        for warn in warns {
            let uri = NormalizedUrl::new(Url::from_file_path(warn.input.full_path()).unwrap());
            let Some(pos) = util::loc_to_pos(warn.core.loc) else {
                continue;
            };
            let Some(token) = self.file_cache.get_token(&uri, pos) else {
                continue;
            };
            match visitor.get_min_expr(&token) {
                Some(Expr::Def(def)) => {
                    let Some(mut range) = util::loc_to_range(def.loc()) else {
                        send_log("range not found")?;
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
                    let code = self.file_cache.get_ranged(&uri, next)?;
                    match code.as_ref().map(|s| &s[..]) {
                        None => {
                            // \n
                            range.end.line += 1;
                            range.end.character = 0;
                        }
                        Some(";") => range.end.character += 1,
                        Some(other) => {
                            send_log(format!("? {other}"))?;
                        }
                    }
                    let edit = TextEdit::new(range, "".to_string());
                    map.entry(uri.clone().raw()).or_insert(vec![]).push(edit);
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
                        map.entry(uri.clone().raw()).or_insert(vec![]).push(edit);
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
        uri: &NormalizedUrl,
        params: CodeActionParams,
    ) -> Option<CodeAction> {
        let new_text = token.content.to_snake_case().to_string();
        let mut map = HashMap::new();
        let visitor = self.get_visitor(uri)?;
        let def_loc = visitor.get_info(&token)?.def_loc;
        let edit = TextEdit::new(util::loc_to_range(def_loc.loc)?, new_text.clone());
        map.insert(uri.clone().raw(), vec![edit]);
        if let Some(value) = self.get_index().and_then(|ind| ind.get_refs(&def_loc)) {
            for refer in value.referrers.iter() {
                let url = Url::from_file_path(refer.module.as_ref()?).ok()?;
                let range = util::loc_to_range(refer.loc)?;
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

    fn gen_extract_function_action(&self, params: &CodeActionParams) -> Option<CodeAction> {
        let action = CodeAction {
            title: "Extract into function".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            data: Some(serde_json::to_value(params.clone()).unwrap()),
            ..Default::default()
        };
        Some(action)
    }

    fn send_normal_action(&self, params: &CodeActionParams) -> ELSResult<Vec<CodeAction>> {
        let mut actions = vec![];
        let uri = NormalizedUrl::new(params.text_document.uri.clone());
        if let Some(token) = self.file_cache.get_token(&uri, params.range.start) {
            if token.is(TokenKind::Symbol) && !token.is_const() && !token.content.is_snake_case() {
                let action = self.gen_change_case_action(token, &uri, params.clone());
                actions.extend(action);
            }
        }
        actions.extend(self.send_quick_fix(params)?);
        actions.extend(self.gen_extract_function_action(params));
        Ok(actions)
    }

    fn send_quick_fix(&self, params: &CodeActionParams) -> ELSResult<Vec<CodeAction>> {
        let mut result: Vec<CodeAction> = vec![];
        let diags = &params.context.diagnostics;
        if diags.is_empty() {
            return Ok(result);
        }
        if diags
            .first()
            .map_or(false, |diag| diag.message.ends_with("is not used"))
        {
            let actions = self.gen_eliminate_unused_vars_action(params)?;
            result.extend(actions);
        }
        Ok(result)
    }

    pub(crate) fn handle_code_action(
        &mut self,
        params: CodeActionParams,
    ) -> ELSResult<Option<CodeActionResponse>> {
        send_log(format!("code action requested: {params:?}"))?;
        let result = match params
            .context
            .only
            .as_ref()
            .and_then(|kinds| kinds.first().map(|s| s.as_str()))
        {
            Some("quickfix") => self.send_quick_fix(&params)?,
            None => self.send_normal_action(&params)?,
            Some(other) => {
                send_log(&format!("Unknown code action requested: {other}"))?;
                vec![]
            }
        };
        Ok(Some(
            result
                .into_iter()
                .map(CodeActionOrCommand::CodeAction)
                .collect(),
        ))
    }

    pub(crate) fn handle_code_action_resolve(
        &mut self,
        action: CodeAction,
    ) -> ELSResult<CodeAction> {
        send_log(format!("code action resolve requested: {action:?}"))?;
        match &action.title[..] {
            "Extract into function" => self.resolve_extract_function_action(action),
            _ => Ok(action),
        }
    }

    fn resolve_extract_function_action(&self, mut action: CodeAction) -> ELSResult<CodeAction> {
        let params = action
            .data
            .take()
            .and_then(|v| serde_json::from_value::<CodeActionParams>(v).ok())
            .ok_or("invalid params")?;
        let uri = NormalizedUrl::new(params.text_document.uri);
        let range = Range::new(Position::new(params.range.start.line, 0), params.range.end);
        let code = self.file_cache.get_ranged(&uri, range)?.unwrap_or_default();
        let indent_len = code.chars().take_while(|c| *c == ' ').count();
        let code = deepen_indent(code);
        let new_func = if ERG_MODE {
            format!("{}new_func() =\n{code}\n\n", " ".repeat(indent_len))
        } else {
            format!("{}def new_func():\n{code}\n\n", " ".repeat(indent_len))
        };
        let edit1 = TextEdit::new(range, new_func);
        let edit2 = TextEdit::new(
            Range::new(range.end, range.end),
            format!("{}new_func()", " ".repeat(indent_len)),
        );
        let mut changes = HashMap::new();
        changes.insert(uri.raw(), vec![edit1, edit2]);
        action.edit = Some(WorkspaceEdit::new(changes));
        Ok(action)
    }
}
