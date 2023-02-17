use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::token::{Token, TokenCategory};
use erg_compiler::varinfo::VarInfo;

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Url};

use crate::server::{ELSResult, Server};
use crate::util;

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn get_definition(
        &mut self,
        uri: &Url,
        token: &Token,
    ) -> ELSResult<Option<VarInfo>> {
        if !token.category_is(TokenCategory::Symbol) {
            Self::send_log(format!("not symbol: {token}"))?;
            Ok(None)
        } else if let Some(visitor) = self.get_visitor(uri) {
            Ok(visitor.get_info(token))
        } else {
            Self::send_log("not found")?;
            Ok(None)
        }
    }

    pub(crate) fn show_definition(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("definition requested: {msg}"))?;
        let params = GotoDefinitionParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        let result = if let Some(token) = self.file_cache.get_token(&uri, pos) {
            if let Some(vi) = self.get_definition(&uri, &token)? {
                match (vi.def_loc.module, util::loc_to_range(vi.def_loc.loc)) {
                    (Some(path), Some(range)) => {
                        let def_uri = util::normalize_url(Url::from_file_path(path).unwrap());
                        Self::send_log("found")?;
                        GotoDefinitionResponse::Array(vec![lsp_types::Location::new(
                            def_uri, range,
                        )])
                    }
                    _ => {
                        Self::send_log("not found (maybe builtin)")?;
                        GotoDefinitionResponse::Array(vec![])
                    }
                }
            } else {
                GotoDefinitionResponse::Array(vec![])
            }
        } else {
            Self::send_log("lex error occurred")?;
            GotoDefinitionResponse::Array(vec![])
        };
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }
}
