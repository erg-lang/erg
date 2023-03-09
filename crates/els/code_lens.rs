use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::hir::Expr;

use lsp_types::{CodeLens, CodeLensParams, Url};

use crate::server::{send, send_log, ELSResult, Server};
use crate::util;

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn show_code_lens(&mut self, msg: &Value) -> ELSResult<()> {
        send_log("code lens requested")?;
        let params = CodeLensParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document.uri);
        // TODO: parallelize
        let result = [
            self.send_trait_impls_lens(&uri)?,
            self.send_class_inherits_lens(&uri)?,
        ]
        .concat();
        send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }))
    }

    fn send_trait_impls_lens(&mut self, uri: &Url) -> ELSResult<Vec<CodeLens>> {
        let mut result = vec![];
        if let Some(artifact) = self.artifacts.get(uri) {
            if let Some(hir) = &artifact.object {
                for chunk in hir.module.iter() {
                    match chunk {
                        Expr::Def(def) if def.def_kind().is_trait() => {
                            let trait_loc = &def.sig.ident().vi.def_loc;
                            let Some(range) = util::loc_to_range(trait_loc.loc) else {
                                continue;
                            };
                            let command = self.gen_show_trait_impls_command(trait_loc.clone())?;
                            let lens = CodeLens {
                                range,
                                command: Some(command),
                                data: None,
                            };
                            result.push(lens);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(result)
    }

    fn send_class_inherits_lens(&mut self, _uri: &Url) -> ELSResult<Vec<CodeLens>> {
        Ok(vec![])
    }
}
