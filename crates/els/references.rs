use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_compiler::artifact::BuildRunnable;

use lsp_types::{ReferenceParams, Url};

use crate::server::{ELSResult, Server};
use crate::util;

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_references(&mut self, msg: &Value) -> ELSResult<()> {
        let params = ReferenceParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position.text_document.uri);
        let pos = params.text_document_position.position;
        if let Some(tok) = self.file_cache.get_token(&uri, pos) {
            // Self::send_log(format!("token: {tok}"))?;
            if let Some(visitor) = self.get_visitor(&uri) {
                if let Some(vi) = visitor.get_info(&tok) {
                    let mut refs = vec![];
                    if let Some(value) = self.get_index().get_refs(&vi.def_loc) {
                        // Self::send_log(format!("referrers: {referrers:?}"))?;
                        for referrer in value.referrers.iter() {
                            if let (Some(path), Some(range)) =
                                (&referrer.module, util::loc_to_range(referrer.loc))
                            {
                                let ref_uri =
                                    util::normalize_url(Url::from_file_path(path).unwrap());
                                refs.push(lsp_types::Location::new(ref_uri, range));
                            }
                        }
                    }
                    Self::send(
                        &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": refs }),
                    )?;
                    return Ok(());
                }
            }
        }
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": Value::Null }),
        )
    }
}
