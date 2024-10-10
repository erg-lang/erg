use erg_common::traits::Locational;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;

use lsp_types::{Position, SelectionRange, SelectionRangeParams};

use crate::_log;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::{loc_to_range, NormalizedUrl};

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_selection_range(
        &mut self,
        params: SelectionRangeParams,
    ) -> ELSResult<Option<Vec<SelectionRange>>> {
        _log!(self, "selection range requested: {params:?}");
        let uri = NormalizedUrl::new(params.text_document.uri);
        let mut res = vec![];
        res.extend(self.get_selection_ranges(&uri, params.positions));
        Ok(Some(res))
    }

    fn get_selection_ranges(
        &self,
        uri: &NormalizedUrl,
        poss: Vec<Position>,
    ) -> Vec<SelectionRange> {
        let mut res = vec![];
        let Some(visitor) = self.get_visitor(uri) else {
            return res;
        };
        for pos in poss {
            let Some(token) = self.file_cache.get_token(uri, pos) else {
                continue;
            };
            let Some(range) = loc_to_range(token.loc()) else {
                continue;
            };
            let mut selection_range = SelectionRange {
                range,
                parent: None,
            };
            let mut parent_range = &mut selection_range.parent;
            let mut opt_expr = visitor.get_min_expr(pos);
            while let Some(expr) = opt_expr {
                let Some(parent) = visitor.get_parent(expr.loc()) else {
                    break;
                };
                let Some(range) = loc_to_range(parent.loc()) else {
                    break;
                };
                *parent_range = Some(Box::new(SelectionRange {
                    range,
                    parent: None,
                }));
                parent_range = &mut parent_range.as_mut().unwrap().parent;
                opt_expr = Some(parent);
            }
            res.push(selection_range);
        }
        res
    }
}
