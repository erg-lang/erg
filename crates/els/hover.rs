use erg_common::consts::PYTHON_MODE;
use erg_common::lang::LanguageCode;
use erg_common::trim_eliminate_top_indent;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::erg_parser::token::{Token, TokenCategory, TokenKind};
use erg_compiler::hir::Expr;
use erg_compiler::ty::HasType;
use erg_compiler::varinfo::{AbsLocation, VarInfo};

use lsp_types::{Hover, HoverContents, HoverParams, MarkedString, Url};

#[allow(unused)]
use crate::_log;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::{self, NormalizedUrl};

const PROG_LANG: &str = if PYTHON_MODE { "python" } else { "erg" };

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
    let erg_count = contents
        .iter()
        .filter(|marked| language(marked) == Some("erg"))
        .count();
    // only location & type definition, no "erg" code block
    if erg_count == 2 {
        return contents;
    }
    // swap "erg" code block (not type definition) and the last element
    let erg_idx = contents
        .iter()
        .rposition(|marked| language(marked) == Some("erg"));
    if let Some(erg_idx) = erg_idx {
        let last = contents.len().saturating_sub(1);
        if erg_idx != last {
            contents.swap(erg_idx, last);
        }
    }
    contents
}

macro_rules! next {
    ($def_pos: ident, $default_code_block: ident, $contents: ident, $prev_token: ident, $token: ident) => {
        if $def_pos.line == 0 {
            if !$default_code_block.is_empty() {
                let code_block = trim_eliminate_top_indent($default_code_block);
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
                let code_block = trim_eliminate_top_indent($default_code_block);
                $contents.push(MarkedString::from_markdown(code_block));
            }
            break;
        }
        $def_pos.line -= 1;
        continue;
    };
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_hover(&mut self, params: HoverParams) -> ELSResult<Option<Hover>> {
        self.send_log(format!("hover requested : {params:?}"))?;
        let uri = NormalizedUrl::new(params.text_document_position_params.text_document.uri);
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
                        let line0 = line.saturating_sub(1);
                        let Some(file_path) = vi.def_loc.module.as_ref() else {
                            return Ok(None);
                        };
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
                            let verbatim_removed =
                                lower.to_str().unwrap_or_default().replace("\\\\?\\", "");
                            let relative = verbatim_removed
                                .strip_prefix(
                                    self.home
                                        .as_os_str()
                                        .to_ascii_lowercase()
                                        .to_str()
                                        .unwrap_or_default(),
                                )
                                .unwrap_or_else(|| file_path.as_path().to_str().unwrap_or_default())
                                .trim_start_matches(['\\', '/']);
                            let relative = relative
                                .strip_prefix(self.erg_path.to_str().unwrap_or_default())
                                .unwrap_or(relative);
                            format!("# {relative}, line {line}\n")
                        };
                        // display the definition line
                        if vi.kind.is_defined() {
                            let uri = NormalizedUrl::try_from(file_path.as_path())?;
                            code_block += self
                                .file_cache
                                .get_line(&uri, line0)
                                .unwrap_or_default()
                                .trim_start();
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
                    self.show_type_defs(&vi, &mut contents)?;
                    self.show_doc_comment(Some(token), &mut contents, &vi.def_loc)?;
                }
                // not found or not symbol, etc.
                None => {
                    if let Some(visitor) = self.get_visitor(&uri) {
                        if let Some(typ) = visitor.get_min_expr(pos) {
                            let typ = MarkedString::from_language_code(
                                ERG_LANG.into(),
                                format!("{}: {typ}", token.content),
                            );
                            contents.push(typ);
                        }
                    }
                }
            }
            if let Some(visitor) = self.get_visitor(&uri) {
                let url = match visitor.get_min_expr(pos) {
                    Some(Expr::Def(def)) => def
                        .sig
                        .ref_t()
                        .module_path()
                        .and_then(|path| Url::from_file_path(path).ok()),
                    Some(Expr::Call(call)) => {
                        call.call_signature_t().return_t().and_then(|ret_t| {
                            ret_t
                                .module_path()
                                .and_then(|path| Url::from_file_path(path).ok())
                        })
                    }
                    Some(expr) => expr
                        .ref_t()
                        .module_path()
                        .or_else(|| {
                            expr.ref_t()
                                .inner_ts()
                                .into_iter()
                                .find_map(|t| t.module_path())
                        })
                        .and_then(|path| Url::from_file_path(path).ok()),
                    _ => None,
                };
                if let Some(url) = url {
                    let path = url.to_file_path().unwrap().with_extension("");
                    let name = if path.ends_with("__init__") || path.ends_with("__init__.d") {
                        path.parent()
                            .unwrap()
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                    } else {
                        path.file_stem().unwrap().to_string_lossy()
                    };
                    contents.push(MarkedString::from_markdown(format!(
                        "Go to [{name}]({url})",
                    )));
                }
            }
        } else {
            self.send_log("lex error")?;
        }
        Ok(Some(Hover {
            contents: HoverContents::Array(sort_hovers(contents)),
            range: None,
        }))
    }

    fn show_type_defs(&mut self, vi: &VarInfo, contents: &mut Vec<MarkedString>) -> ELSResult<()> {
        let mut defs = "".to_string();
        for inner_t in vi.t.contained_ts() {
            if let Some(path) = &vi.def_loc.module {
                let Ok(def_uri) = util::NormalizedUrl::try_from(path.as_path()) else {
                    continue;
                };
                let Some(module) = ({
                    // self.quick_check_file(def_uri.clone())?;
                    self.get_mod_ctx(&def_uri)
                }) else {
                    continue;
                };
                if let Some((_, vi)) = module.context.get_type_info(&inner_t) {
                    if let Some(url) = vi
                        .def_loc
                        .module
                        .as_ref()
                        .and_then(|path| Url::from_file_path(path).ok())
                    {
                        defs += &format!(
                            "[{}]({url}#L{}) ",
                            inner_t.local_name(),
                            vi.def_loc.loc.ln_begin().unwrap_or(1)
                        );
                    }
                }
            }
        }
        if !defs.is_empty() {
            contents.push(MarkedString::from_markdown(format!("Go to {defs}")));
        }
        Ok(())
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
            let Ok(def_uri) = NormalizedUrl::try_from(module.as_path()) else {
                return Ok(());
            };
            let mut default_code_block = "".to_string();
            let Some(stream) = self.file_cache.get_token_stream(&def_uri) else {
                return Ok(());
            };
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
                    let lang = lang_code(&code_block);
                    if lang.matches_feature() {
                        let code_block = trim_eliminate_top_indent(
                            code_block.trim_start_matches(lang.as_str()).to_string(),
                        );
                        let marked = match lang {
                            LanguageCode::Erg => {
                                MarkedString::from_language_code("erg".into(), code_block)
                            }
                            LanguageCode::Python | LanguageCode::ErgOrPython => {
                                MarkedString::from_language_code("python".into(), code_block)
                            }
                            _ => MarkedString::from_markdown(code_block),
                        };
                        contents.push(marked);
                        if lang.is_pl() {
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
                        let code_block = trim_eliminate_top_indent(default_code_block);
                        contents.push(MarkedString::from_markdown(code_block));
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}
