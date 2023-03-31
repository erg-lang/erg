use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::varinfo::AbsLocation;

use lsp_types::{Position, ReferenceParams, Url};

use crate::server::{send, ELSResult, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_references(&mut self, msg: &Value) -> ELSResult<()> {
        let params = ReferenceParams::deserialize(&msg["params"])?;
        let uri = NormalizedUrl::new(params.text_document_position.text_document.uri);
        let pos = params.text_document_position.position;
        let result = self.show_refs_inner(&uri, pos);
        send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }))
    }

    fn show_refs_inner(&self, uri: &NormalizedUrl, pos: Position) -> Vec<lsp_types::Location> {
        if let Some(tok) = self.file_cache.get_token(uri, pos) {
            // send_log(format!("token: {tok}"))?;
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
        if let Some(value) = self.get_index().get_refs(referee) {
            // send_log(format!("referrers: {referrers:?}"))?;
            for referrer in value.referrers.iter() {
                if let (Some(path), Some(range)) =
                    (&referrer.module, util::loc_to_range(referrer.loc))
                {
                    let ref_uri = Url::from_file_path(path).unwrap();
                    refs.push(lsp_types::Location::new(ref_uri, range));
                }
            }
        }
        refs
    }
}
