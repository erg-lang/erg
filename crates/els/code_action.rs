use std::collections::HashMap;

use erg_common::consts::{ERG_MODE, PYTHON_MODE};
use erg_common::deepen_indent;
use erg_common::traits::{Locational, Stream};
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::erg_parser::token::{Token, TokenKind};
use erg_compiler::hir::Expr;
use erg_compiler::ty::HasType;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    Position, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    fn gen_eliminate_unused_vars_action(
        &self,
        params: &CodeActionParams,
    ) -> ELSResult<Option<CodeAction>> {
        let uri = NormalizedUrl::new(params.text_document.uri.clone());
        let diags = &params.context.diagnostics;
        let Some(diag) = diags.first().cloned() else {
            return Ok(None);
        };
        let mut map = HashMap::new();
        let Some(visitor) = self.get_visitor(&uri) else {
            self.send_log("visitor not found")?;
            return Ok(None);
        };
        let Some(warns) = self.get_warns(&uri) else {
            self.send_log("artifact not found")?;
            return Ok(None);
        };
        let warns = warns
            .iter()
            .filter(|warn| warn.core.main_message.ends_with("is not used"))
            .collect::<Vec<_>>();
        for warn in warns {
            let uri = NormalizedUrl::new(Url::from_file_path(warn.input.full_path()).unwrap());
            let Some(pos) = util::loc_to_pos(warn.core.loc) else {
                continue;
            };
            match visitor.get_min_expr(pos) {
                Some(Expr::Def(def)) => {
                    let Some(mut range) = util::loc_to_range(def.loc()) else {
                        self.send_log("range not found")?;
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
                            crate::_log!(self, "? {other}");
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
        if let Some(value) = self.shared.index.get_refs(&def_loc) {
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

    fn gen_extract_action(&self, params: &CodeActionParams) -> Vec<CodeAction> {
        let mut actions = vec![];
        if params.range.start.line == params.range.end.line {
            if params.range.start.character == params.range.end.character {
                return vec![];
            }
            actions.push(CodeAction {
                title: "Extract into variable".to_string(),
                kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                data: Some(serde_json::to_value(params.clone()).unwrap()),
                ..Default::default()
            });
        }
        actions.push(CodeAction {
            title: "Extract into function".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            data: Some(serde_json::to_value(params.clone()).unwrap()),
            ..Default::default()
        });
        actions
    }

    fn gen_inline_action(&self, params: &CodeActionParams) -> Option<CodeAction> {
        let uri = NormalizedUrl::new(params.text_document.uri.clone());
        let visitor = self.get_visitor(&uri)?;
        match visitor.get_min_expr(params.range.start)? {
            Expr::Def(def) => {
                let title = if def.sig.is_subr() {
                    "Inline function"
                } else {
                    "Inline variable"
                };
                let action = CodeAction {
                    title: title.to_string(),
                    kind: Some(CodeActionKind::REFACTOR_INLINE),
                    data: Some(serde_json::to_value(params.clone()).unwrap()),
                    ..Default::default()
                };
                Some(action)
            }
            Expr::Accessor(acc) => {
                let title = if acc.ref_t().is_subr() {
                    "Inline function"
                } else {
                    "Inline variable"
                };
                let action = CodeAction {
                    title: title.to_string(),
                    kind: Some(CodeActionKind::REFACTOR_INLINE),
                    data: Some(serde_json::to_value(params.clone()).unwrap()),
                    ..Default::default()
                };
                Some(action)
            }
            _ => None,
        }
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
        actions.extend(self.gen_extract_action(params));
        actions.extend(self.gen_inline_action(params));
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
        self.send_log(format!("code action requested: {params:?}"))?;
        let result = match params
            .context
            .only
            .as_ref()
            .and_then(|kinds| kinds.first().map(|s| s.as_str()))
        {
            Some("quickfix") => self.send_quick_fix(&params)?,
            None => self.send_normal_action(&params)?,
            Some(other) => {
                self.send_log(&format!("Unknown code action requested: {other}"))?;
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
        self.send_log(format!("code action resolve requested: {action:?}"))?;
        match &action.title[..] {
            "Extract into function" | "Extract into variable" => {
                self.resolve_extract_action(action)
            }
            "Inline variable" => self.resolve_inline_variable_action(action),
            _ => Ok(action),
        }
    }

    fn resolve_extract_action(&self, mut action: CodeAction) -> ELSResult<CodeAction> {
        let params = action
            .data
            .take()
            .and_then(|v| serde_json::from_value::<CodeActionParams>(v).ok())
            .ok_or("invalid params")?;
        let extract_function = action.title == "Extract into function";
        let uri = NormalizedUrl::new(params.text_document.uri);
        let start = Position::new(params.range.start.line, 0);
        let mut range = Range::new(start, params.range.end);
        let indented_code = self.file_cache.get_ranged(&uri, range)?.unwrap_or_default();
        let indent_len = indented_code.chars().take_while(|c| *c == ' ').count();
        range.start.character = params.range.start.character;
        let code = self.file_cache.get_ranged(&uri, range)?.unwrap_or_default();
        // `    |foo|` (|...| is the selected range) -> `|    foo|`
        let diff = indented_code.trim_end_matches(&code);
        let diff_indent_len = diff.chars().take_while(|c| *c == ' ').count();
        let diff_is_indent = diff.trim().is_empty();
        let code = if diff_is_indent {
            range.start.character = 0;
            indented_code
        } else {
            code
        };
        let body = if extract_function {
            let mut code = deepen_indent(code);
            let should_insert_indent = code.lines().count() == 1 && !diff_is_indent;
            if should_insert_indent {
                code = format!("{}{code}", " ".repeat(diff_indent_len));
            }
            // add `return` to the last line
            if PYTHON_MODE {
                let mut lines = code.lines().collect::<Vec<_>>();
                let mut last_line = lines.last().unwrap().to_string();
                let code_start = last_line.chars().position(|c| c != ' ').unwrap();
                last_line.insert_str(code_start, "return ");
                lines.pop();
                lines.push(&last_line);
                lines.join("\n")
            } else {
                code
            }
        } else {
            code.trim_start().to_string()
        };
        let sig = match (ERG_MODE, extract_function) {
            (true, true) => "new_func() =\n",
            (true, false) => "new_var = ",
            (false, true) => "def new_func():\n",
            (false, false) => "new_var = ",
        };
        let expanded = if extract_function {
            "new_func()"
        } else {
            "new_var"
        };
        let expanded = if range.start.character == 0 {
            format!("{}{expanded}", " ".repeat(indent_len))
        } else {
            expanded.to_string()
        };
        let extracted = format!("{}{sig}{body}\n\n", " ".repeat(indent_len));
        let start = Position::new(range.start.line, 0);
        let edit1 = TextEdit::new(Range::new(start, start), extracted);
        let edit2 = TextEdit::new(Range::new(range.start, range.end), expanded);
        let mut changes = HashMap::new();
        changes.insert(uri.raw(), vec![edit1, edit2]);
        action.edit = Some(WorkspaceEdit::new(changes));
        Ok(action)
    }

    fn resolve_inline_variable_action(&self, mut action: CodeAction) -> ELSResult<CodeAction> {
        let params = action
            .data
            .take()
            .and_then(|v| serde_json::from_value::<CodeActionParams>(v).ok())
            .ok_or("invalid params")?;
        let uri = NormalizedUrl::new(params.text_document.uri.clone());
        let visitor = self.get_visitor(&uri).ok_or("get_visitor")?;
        match visitor
            .get_min_expr(params.range.start)
            .ok_or("get_min_expr")?
        {
            Expr::Def(def) => {
                action.edit = Some(WorkspaceEdit::new(self.inline_var_def(def)));
            }
            Expr::Accessor(acc) => {
                let uri = NormalizedUrl::new(
                    Url::from_file_path(
                        acc.var_info()
                            .def_loc
                            .module
                            .as_ref()
                            .ok_or("def_loc.module")?,
                    )
                    .map_err(|_| "from_file_path")?,
                );
                let visitor = self.get_visitor(&uri).ok_or(format!("{uri} not found"))?;
                let range = util::loc_to_range(acc.var_info().def_loc.loc).ok_or("loc_to_range")?;
                if let Some(Expr::Def(def)) = visitor.get_min_expr(range.start) {
                    action.edit = Some(WorkspaceEdit::new(self.inline_var_def(def)));
                }
            }
            _ => {}
        }
        Ok(action)
    }

    fn inline_var_def(&self, def: &erg_compiler::hir::Def) -> HashMap<Url, Vec<TextEdit>> {
        let mut changes = HashMap::new();
        let mut range = util::loc_to_range(def.loc()).unwrap();
        range.end.character = u32::MAX;
        let delete = TextEdit::new(range, "".to_string());
        let uri = NormalizedUrl::new(
            Url::from_file_path(def.sig.ident().vi.def_loc.module.as_ref().unwrap()).unwrap(),
        );
        let range = util::loc_to_range(def.body.block.loc()).unwrap();
        let code = self.file_cache.get_ranged(&uri, range).unwrap().unwrap();
        let expr = def.body.block.first().unwrap();
        let code = if expr.need_to_be_closed() {
            format!("({code})")
        } else {
            code
        };
        changes.insert(uri.raw(), vec![delete]);
        if let Some(index) = self.shared.index.get_refs(&def.sig.ident().vi.def_loc) {
            for ref_ in index.referrers.iter() {
                let Some(path) = ref_.module.as_ref() else {
                    continue;
                };
                let edit = TextEdit::new(util::loc_to_range(ref_.loc).unwrap(), code.clone());
                let uri = NormalizedUrl::new(Url::from_file_path(path).unwrap());
                changes.entry(uri.raw()).or_insert(vec![]).push(edit);
            }
        }
        changes
    }
}
