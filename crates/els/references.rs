use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::varinfo::AbsLocation;

use lsp_types::{Location, Position, ReferenceParams, Url};

use crate::_log;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_references(
        &mut self,
        params: ReferenceParams,
    ) -> ELSResult<Option<Vec<Location>>> {
        _log!(self, "references: {params:?}");
        let uri = NormalizedUrl::new(params.text_document_position.text_document.uri);
        let pos = params.text_document_position.position;
        let result = self.show_refs_inner(&uri, pos);
        Ok(Some(result))
    }

    fn show_refs_inner(&self, uri: &NormalizedUrl, pos: Position) -> Vec<lsp_types::Location> {
        if let Some(tok) = self.file_cache.get_symbol(uri, pos) {
            if let Some(visitor) = self.get_visitor(uri) {
                if let Some(vi) = visitor.get_info(&tok) {
                    return self.get_refs_from_abs_loc(&vi.def_loc);
                }
            }
        }
        vec![]
    }

    pub(crate) fn get_refs_from_abs_loc(&self, referee: &AbsLocation) -> Vec<lsp_types::Location> {
        let mut refs = vec![];
        if let Some(value) = self.shared.index.get_refs(referee) {
            if value.vi.def_loc == AbsLocation::unknown() {
                return vec![];
            }
            for referrer in value.referrers.iter() {
                if let (Some(path), Some(range)) =
                    (&referrer.module, util::loc_to_range(referrer.loc))
                {
                    let Ok(ref_uri) = Url::from_file_path(path) else {
                        continue;
                    };
                    refs.push(lsp_types::Location::new(ref_uri, range));
                }
            }
        }
        refs
    }
}
