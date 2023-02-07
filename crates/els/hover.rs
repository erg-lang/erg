use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::lang::LanguageCode;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::token::{Token, TokenCategory, TokenKind};
use erg_compiler::varinfo::AbsLocation;

use lsp_types::{HoverContents, HoverParams, MarkedString, Url};

use crate::server::{ELSResult, Server};
use crate::util;

macro_rules! next {
    ($def_pos: ident, $default_code_block: ident, $contents: ident, $prev_token: ident, $token: ident) => {
        if $def_pos.line == 0 {
            if !$default_code_block.is_empty() {
                $contents.push(MarkedString::from_markdown($default_code_block));
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
                $contents.push(MarkedString::from_markdown($default_code_block));
            }
            break;
        }
        $def_pos.line -= 1;
        continue;
    };
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_hover(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("hover requested : {msg}"))?;
        let lang = if cfg!(feature = "py_compatible") {
            "python"
        } else {
            "erg"
        };
        let params = HoverParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        let mut contents = vec![];
        let opt_tok = self.file_cache.get_token(&uri, pos)?;
        let opt_token = if let Some(token) = opt_tok {
            match token.category() {
                TokenCategory::StrInterpRight => {
                    self.file_cache.get_token_relatively(&uri, pos, -1)?
                }
                TokenCategory::StrInterpLeft => {
                    self.file_cache.get_token_relatively(&uri, pos, 1)?
                }
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
                        let definition = MarkedString::from_language_code(lang.into(), code_block);
                        contents.push(definition);
                    }
                    let typ = MarkedString::from_language_code(
                        lang.into(),
                        format!("{}: {}", token.content, vi.t),
                    );
                    contents.push(typ);
                    self.show_doc_comment(token, &mut contents, &vi.def_loc)?;
                }
                // not found or not symbol, etc.
                None => {
                    if let Some(visitor) = self.get_visitor(&uri) {
                        if let Some(typ) = visitor.get_t(&token) {
                            let typ = MarkedString::from_language_code(
                                lang.into(),
                                format!("{}: {typ}", token.content),
                            );
                            contents.push(typ);
                        }
                    }
                }
            }
        } else {
            Self::send_log("lex error")?;
        }
        let result = json!({ "contents": HoverContents::Array(contents) });
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }

    fn show_doc_comment(
        &self,
        var_token: Token,
        contents: &mut Vec<MarkedString>,
        def_loc: &AbsLocation,
    ) -> ELSResult<()> {
        if let Some(module) = def_loc.module.as_ref() {
            let mut def_pos = util::loc_to_range(def_loc.loc).unwrap().start;
            def_pos.line = def_pos.line.saturating_sub(1);
            let def_uri = util::normalize_url(Url::from_file_path(module).unwrap());
            let mut default_code_block = "".to_string();
            let stream = util::get_token_stream(def_uri)?;
            let mut prev_token = Token::DUMMY;
            loop {
                let Some(token) = util::get_token_from_stream(&stream, def_pos)? else {
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
                    // TODO: support other languages
                    let lang = if code_block.starts_with("japanese\n") {
                        LanguageCode::Japanese
                    } else {
                        LanguageCode::English
                    };
                    if lang.matches_feature() {
                        let code_block = code_block.trim_start_matches(lang.as_str()).to_string();
                        contents.push(MarkedString::from_markdown(code_block));
                        break;
                    } else {
                        if lang.is_en() {
                            default_code_block = code_block;
                        }
                        next!(def_pos, default_code_block, contents, prev_token, token);
                    }
                } else if token == var_token {
                    next!(def_pos, default_code_block, contents, prev_token, token);
                } else {
                    if token.category_is(TokenCategory::Separator) {
                        next!(def_pos, default_code_block, contents, prev_token, token);
                    }
                    if !default_code_block.is_empty() {
                        contents.push(MarkedString::from_markdown(default_code_block));
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}
