use std::path::Path;

use erg_common::spawn::safe_yield;
use erg_common::Str;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::ast::{ClassAttr, Expr, Literal};
use erg_compiler::erg_parser::parse::Parsable;

use erg_compiler::erg_parser::token::{Token, TokenKind};
use erg_compiler::ty::Type;
use erg_compiler::varinfo::VarInfo;
use lsp_types::{DocumentLink, DocumentLinkParams, Position, Range, Url};

use crate::_log;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::NormalizedUrl;

fn is_not_symbol(word: &str) -> bool {
    word.chars()
        .all(|c| c.is_ascii_digit() || c.is_ascii_punctuation())
}

fn is_usual_word(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "a" | "the"
            | "an"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "being"
            | "am"
            | "does"
            | "did"
            | "done"
            | "doing"
            | "have"
            | "has"
            | "had"
            | "having"
            | "will"
            | "shall"
            | "would"
            | "should"
            | "may"
            | "might"
            | "must"
            | "can"
            | "could"
            | "need"
            | "used"
            | "to"
            | "of"
            | "in"
            | "on"
            | "at"
            | "by"
            | "with"
            | "from"
            | "about"
            | "between"
            | "among"
            | "into"
            | "onto"
            | "upon"
            | "over"
            | "under"
            | "above"
            | "below"
            | "behind"
            | "beside"
            | "before"
            | "after"
            | "during"
            | "through"
            | "across"
            | "against"
            | "towards"
            | "around"
            | "besides"
            | "like"
            | "near"
            | "till"
            | "until"
            | "throughout"
            | "within"
            | "without"
            | "according"
            | "though"
            | "whereas"
            | "whether"
            | "so"
            | "although"
            | "if"
            | "unless"
            | "because"
            | "for"
            | "since"
            | "while"
    )
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_document_link(
        &mut self,
        params: DocumentLinkParams,
    ) -> ELSResult<Option<Vec<DocumentLink>>> {
        _log!(self, "document link requested: {params:?}");
        while !self.flags.builtin_modules_loaded() {
            safe_yield();
        }
        let uri = NormalizedUrl::new(params.text_document.uri);
        let mut res = vec![];
        res.extend(self.get_document_link(&uri));
        Ok(Some(res))
    }

    fn get_document_link(&self, uri: &NormalizedUrl) -> Vec<DocumentLink> {
        let mut res = vec![];
        if let Some(ast) = self.get_ast(uri) {
            let mut comments = vec![];
            for chunk in ast.block().iter() {
                comments.extend(self.get_doc_comments(chunk));
            }
            for comment in comments {
                res.extend(self.gen_doc_link(uri, comment));
            }
        } else if let Ok(ast) = self.build_ast(uri) {
            let mut comments = vec![];
            for chunk in ast.block().iter() {
                comments.extend(self.get_doc_comments(chunk));
            }
            for comment in comments {
                res.extend(self.gen_doc_link(uri, comment));
            }
        }
        for comment in self.get_comments(uri) {
            res.extend(self.gen_doc_link(uri, &comment));
        }
        res
    }

    fn gen_doc_link(&self, uri: &NormalizedUrl, comment: &Literal) -> Vec<DocumentLink> {
        let mut res = vec![];
        let Some(mod_ctx) = self.get_mod_ctx(uri) else {
            return vec![];
        };
        let mut line = comment.token.lineno.saturating_sub(1);
        let mut col = comment.token.col_begin;
        for li in comment.token.content.split('\n') {
            let li = Str::rc(li);
            let words = li.split_with(&[" ", "'", "\"", "`", "(", ")", "[", "]", "{", "}"]);
            for word in words {
                if word.trim().is_empty() || is_usual_word(word) || is_not_symbol(word) {
                    col += word.len() as u32 + 1;
                    continue;
                }
                let range = Range {
                    start: Position {
                        line,
                        character: col,
                    },
                    end: Position {
                        line,
                        character: col + word.len() as u32,
                    },
                };
                let typ = Type::Mono(Str::rc(word));
                if let Some(path) = self.cfg.input.resolve_path(Path::new(word), &self.cfg) {
                    let target = Url::from_file_path(path).ok();
                    res.push(DocumentLink {
                        range,
                        target,
                        tooltip: Some(format!("module {word}")),
                        data: None,
                    });
                } else if let Some((_, vi)) = mod_ctx.context.get_type_info(&typ) {
                    if let Some(doc) = self.gen_doc_link_from_vi(word, range, vi) {
                        res.push(doc);
                    }
                }
                col += word.len() as u32 + 1;
            }
            line += 1;
            col = 0;
        }
        res
    }

    fn gen_doc_link_from_vi(&self, name: &str, range: Range, vi: &VarInfo) -> Option<DocumentLink> {
        let mut target = if let Some(path) = vi.t.module_path() {
            Url::from_file_path(path).ok()?
        } else {
            Url::from_file_path(vi.def_loc.module.as_ref()?).ok()?
        };
        target.set_fragment(Some(&format!("L{}", vi.def_loc.loc.ln_begin()?)));
        let tooltip = format!("{name}: {}", vi.t);
        Some(DocumentLink {
            range,
            target: Some(target),
            tooltip: Some(tooltip),
            data: None,
        })
    }

    #[allow(clippy::only_used_in_recursion)]
    fn get_doc_comments<'e>(&self, expr: &'e Expr) -> Vec<&'e Literal> {
        match expr {
            Expr::Literal(lit) if lit.is_doc_comment() => vec![lit],
            Expr::Def(def) => {
                let mut comments = vec![];
                for chunk in def.body.block.iter() {
                    comments.extend(self.get_doc_comments(chunk));
                }
                comments
            }
            Expr::Methods(methods) => {
                let mut comments = vec![];
                for chunk in methods.attrs.iter() {
                    match chunk {
                        ClassAttr::Def(def) => {
                            for chunk in def.body.block.iter() {
                                comments.extend(self.get_doc_comments(chunk));
                            }
                        }
                        ClassAttr::Doc(lit) => {
                            comments.push(lit);
                        }
                        _ => {}
                    }
                }
                comments
            }
            Expr::Call(call) => {
                let mut comments = vec![];
                for arg in call.args.pos_args() {
                    comments.extend(self.get_doc_comments(&arg.expr));
                }
                comments
            }
            Expr::Lambda(lambda) => {
                let mut comments = vec![];
                for chunk in lambda.body.iter() {
                    comments.extend(self.get_doc_comments(chunk));
                }
                comments
            }
            Expr::Dummy(dummy) => {
                let mut comments = vec![];
                for chunk in dummy.exprs.iter() {
                    comments.extend(self.get_doc_comments(chunk));
                }
                comments
            }
            Expr::InlineModule(module) => {
                let mut comments = vec![];
                for chunk in module.ast.module.block().iter() {
                    comments.extend(self.get_doc_comments(chunk));
                }
                comments
            }
            _ => vec![],
        }
    }

    fn get_comments(&self, uri: &NormalizedUrl) -> Vec<Literal> {
        let mut lines = vec![];
        if let Ok(code) = self.file_cache.get_entire_code(uri) {
            for (line, li) in code.lines().enumerate() {
                if let Some(com) = li.trim().strip_prefix("#") {
                    let col = li.len() - com.len();
                    let token = Token::new(
                        TokenKind::DocComment,
                        Str::rc(com),
                        line as u32 + 1,
                        col as u32,
                    );
                    lines.push(Literal::new(token));
                }
            }
        }
        lines
    }
}
