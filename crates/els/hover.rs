use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::lang::LanguageCode;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::token::{Token, TokenCategory, TokenKind};
use erg_compiler::varinfo::AbsLocation;

use lsp_types::{HoverContents, HoverParams, MarkedString, Url};

use crate::server::{send, send_log, ELSResult, Server};
use crate::util;

const PROG_LANG: &str = if cfg!(feature = "py_compatible") {
    "python"
} else {
    "erg"
};

const ERG_LANG: &str = "erg";

fn lang_code(code: &str) -> LanguageCode {
    code.lines()
        .next()
        .unwrap_or("")
        .parse()
        .unwrap_or(LanguageCode::English)
}

fn language(marked: &MarkedString) -> Option<&str> {
    match marked {
        MarkedString::String(_) => None,
        MarkedString::LanguageString(ls) => Some(&ls.language),
    }
}

fn sort_hovers(mut contents: Vec<MarkedString>) -> Vec<MarkedString> {
    // swap "erg" code block (not type definition) and the last element
    let erg_idx = contents
        .iter()
        .rposition(|marked| language(marked) == Some("erg"));
    if let Some(erg_idx) = erg_idx {
        let last = contents.len() - 1;
        if erg_idx != last {
            contents.swap(erg_idx, last);
        }
    }
    contents
}

fn eliminate_top_indent(code: String) -> String {
    let trimmed = code.trim_start_matches('\n');
    if !trimmed.starts_with(' ') {
        return code;
    }
    let mut indent = 0;
    let mut result = String::new();
    for line in trimmed.lines() {
        if indent == 0 {
            indent = line.chars().take_while(|c| *c == ' ').count();
        }
        if line.len() > indent {
            result.push_str(&line[indent..]);
        }
        result.push('\n');
    }
    if !result.is_empty() {
        result.pop();
    }
    result
}

macro_rules! next {
    ($def_pos: ident, $default_code_block: ident, $contents: ident, $prev_token: ident, $token: ident) => {
        if $def_pos.line == 0 {
            if !$default_code_block.is_empty() {
                let code_block = eliminate_top_indent($default_code_block);
                $contents.push(MarkedString::from_markdown(code_block));
            }
            break;
        }
        $prev_token = $token;
        $def_pos.line -= 1;
        continue;
    };
    ($def_pos: ident, $default_code_block: ident, $contents: ident) => {
        if $def_pos.line == 0 {
            if !$default_code_block.is_empty() {
                let code_block = eliminate_top_indent($default_code_block);
                $contents.push(MarkedString::from_markdown(code_block));
            }
            break;
        }
        $def_pos.line -= 1;
        continue;
    };
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_hover(&mut self, msg: &Value) -> ELSResult<()> {
        send_log(format!("hover requested : {msg}"))?;
        let params = HoverParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        let mut contents = vec![];
        let opt_tok = self.file_cache.get_token(&uri, pos);
        let opt_token = if let Some(token) = opt_tok {
            match token.category() {
                TokenCategory::StrInterpRight => {
                    self.file_cache.get_token_relatively(&uri, pos, -1)
                }
                TokenCategory::StrInterpLeft => self.file_cache.get_token_relatively(&uri, pos, 1),
                // TODO: StrInterpMid
                _ => Some(token),
            }
        } else {
            None
        };
        if let Some(token) = opt_token {
            match self.get_definition(&uri, &token)? {
                Some(vi) => {
                    if let Some(line) = vi.def_loc.loc.ln_begin() {
                        let file_path = vi.def_loc.module.as_ref().unwrap();
                        let mut code_block = if cfg!(not(windows)) {
                            let relative = file_path
                                .strip_prefix(&self.home)
                                .unwrap_or(file_path.as_path());
                            let relative =
                                relative.strip_prefix(&self.erg_path).unwrap_or(relative);
                            format!("# {}, line {line}\n", relative.display())
                        } else {
                            // windows' file paths are case-insensitive, so we need to normalize them
                            let lower = file_path.as_os_str().to_ascii_lowercase();
                            let verbatim_removed = lower.to_str().unwrap().replace("\\\\?\\", "");
                            let relative = verbatim_removed
                                .strip_prefix(
                                    self.home.as_os_str().to_ascii_lowercase().to_str().unwrap(),
                                )
                                .unwrap_or_else(|| file_path.as_path().to_str().unwrap())
                                .trim_start_matches(['\\', '/']);
                            let relative = relative
                                .strip_prefix(self.erg_path.to_str().unwrap())
                                .unwrap_or(relative);
                            format!("# {relative}, line {line}\n")
                        };
                        // display the definition line
                        if vi.kind.is_defined() {
                            code_block += util::get_line_from_path(file_path, line)?.trim_start();
                            match code_block.chars().last() {
                                Some('=' | '>') => {
                                    code_block += " ...";
                                }
                                Some('(') => {
                                    code_block += "...) = ...";
                                }
                                _ => {}
                            }
                        }
                        let definition =
                            MarkedString::from_language_code(PROG_LANG.into(), code_block);
                        contents.push(definition);
                    }
                    let typ = MarkedString::from_language_code(
                        ERG_LANG.into(),
                        format!("{}: {}", token.content, vi.t),
                    );
                    contents.push(typ);
                    self.show_doc_comment(Some(token), &mut contents, &vi.def_loc)?;
                }
                // not found or not symbol, etc.
                None => {
                    if let Some(visitor) = self.get_visitor(&uri) {
                        if let Some(typ) = visitor.get_min_expr(&token) {
                            let typ = MarkedString::from_language_code(
                                ERG_LANG.into(),
                                format!("{}: {typ}", token.content),
                            );
                            contents.push(typ);
                        }
                    }
                }
            }
        } else {
            send_log("lex error")?;
        }
        let result = json!({ "contents": HoverContents::Array(sort_hovers(contents)) });
        send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }))
    }

    pub(crate) fn show_doc_comment(
        &self,
        var_token: Option<Token>,
        contents: &mut Vec<MarkedString>,
        def_loc: &AbsLocation,
    ) -> ELSResult<()> {
        if let Some(module) = def_loc.module.as_ref() {
            let mut def_pos = match util::loc_to_range(def_loc.loc) {
                Some(range) => range.end,
                None => {
                    return Ok(());
                }
            };
            def_pos.line = def_pos.line.saturating_sub(1);
            let def_uri = util::normalize_url(Url::from_file_path(module).unwrap());
            let mut default_code_block = "".to_string();
            let Some(stream) = self.file_cache.get_token_stream(&def_uri) else {
                return Ok(());
            };
            let mut prev_token = Token::DUMMY;
            loop {
                let Some(token) = util::get_token_from_stream(stream, def_pos)? else {
                    next!(def_pos, default_code_block, contents);
                };
                if token.deep_eq(&prev_token) {
                    next!(def_pos, default_code_block, contents, prev_token, token);
                }
                if token.is(TokenKind::DocComment) {
                    let code_block = token
                        .content
                        .trim_start_matches("'''")
                        .trim_end_matches("'''")
                        .to_string();
                    let lang = lang_code(&code_block);
                    if lang.matches_feature() {
                        let code_block = eliminate_top_indent(
                            code_block.trim_start_matches(lang.as_str()).to_string(),
                        );
                        let marked = match lang {
                            LanguageCode::Erg => {
                                MarkedString::from_language_code("erg".into(), code_block)
                            }
                            LanguageCode::Python => {
                                MarkedString::from_language_code("python".into(), code_block)
                            }
                            _ => MarkedString::from_markdown(code_block),
                        };
                        contents.push(marked);
                        if lang.is_erg() {
                            next!(def_pos, default_code_block, contents, prev_token, token);
                        } else {
                            break;
                        }
                    } else {
                        if lang.is_en() {
                            default_code_block = code_block;
                        }
                        next!(def_pos, default_code_block, contents, prev_token, token);
                    }
                } else if var_token.as_ref() == Some(&token) {
                    // multiple pattern def
                    next!(def_pos, default_code_block, contents, prev_token, token);
                } else {
                    if token.category_is(TokenCategory::Separator) {
                        next!(def_pos, default_code_block, contents, prev_token, token);
                    }
                    if !default_code_block.is_empty() {
                        let code_block = eliminate_top_indent(default_code_block);
                        contents.push(MarkedString::from_markdown(code_block));
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}
