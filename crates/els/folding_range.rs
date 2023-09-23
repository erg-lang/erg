use erg_common::error::Location;
use erg_common::traits::Locational;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::ast::Expr;
use erg_compiler::erg_parser::parse::Parsable;

use lsp_types::{FoldingRange, FoldingRangeKind, FoldingRangeParams};

use crate::_log;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::NormalizedUrl;

fn imports_range(start: &Location, end: &Location) -> Option<FoldingRange> {
    Some(FoldingRange {
        start_line: start.ln_begin()?.saturating_sub(1),
        start_character: start.col_begin(),
        end_line: end.ln_end()?.saturating_sub(1),
        end_character: end.col_end(),
        kind: Some(FoldingRangeKind::Imports),
    })
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_folding_range(
        &mut self,
        params: FoldingRangeParams,
    ) -> ELSResult<Option<Vec<FoldingRange>>> {
        _log!(self, "folding range requested: {params:?}");
        let uri = NormalizedUrl::new(params.text_document.uri);
        let mut res = vec![];
        res.extend(self.fold_imports(&uri));
        Ok(Some(res))
    }

    fn fold_imports(&self, uri: &NormalizedUrl) -> Vec<FoldingRange> {
        let mut res = vec![];
        if let Some(module) = self.build_ast(uri) {
            let mut ranges = vec![];
            for chunk in module.into_iter() {
                match chunk {
                    Expr::Def(def) if def.def_kind().is_import() => {
                        ranges.push(def.loc());
                    }
                    _ => {
                        if !ranges.is_empty() {
                            let start = ranges.first().unwrap();
                            let end = ranges.last().unwrap();
                            res.extend(imports_range(start, end));
                            ranges.clear();
                        }
                    }
                }
            }
            if !ranges.is_empty() {
                let start = ranges.first().unwrap();
                let end = ranges.last().unwrap();
                res.extend(imports_range(start, end));
            }
        }
        res
    }
}
