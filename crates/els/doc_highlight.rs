use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;

use lsp_types::{DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams, Position};

use crate::_log;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::{loc_to_range, NormalizedUrl};

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_document_highlight(
        &mut self,
        params: DocumentHighlightParams,
    ) -> ELSResult<Option<Vec<DocumentHighlight>>> {
        _log!(self, "document highlight requested: {params:?}");
        let uri = NormalizedUrl::new(params.text_document_position_params.text_document.uri);
        let mut res = vec![];
        res.extend(
            self.get_document_highlight(&uri, params.text_document_position_params.position),
        );
        Ok(Some(res))
    }

    fn get_document_highlight(&self, uri: &NormalizedUrl, pos: Position) -> Vec<DocumentHighlight> {
        let mut res = vec![];
        let Some(visitor) = self.get_visitor(uri) else {
            return res;
        };
        if let Some(tok) = self.file_cache.get_symbol(uri, pos) {
            if let Some(vi) = visitor.get_info(&tok) {
                if let Some(range) = loc_to_range(vi.def_loc.loc) {
                    res.push(DocumentHighlight {
                        range,
                        kind: Some(DocumentHighlightKind::WRITE),
                    });
                }
                for reference in self.get_refs_from_abs_loc(&vi.def_loc) {
                    res.push(DocumentHighlight {
                        range: reference.range,
                        kind: Some(DocumentHighlightKind::READ),
                    });
                }
            }
        }
        res
    }
}
