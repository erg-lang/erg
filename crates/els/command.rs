use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_compiler::artifact::BuildRunnable;

use lsp_types::{CodeAction, ExecuteCommandParams};

use crate::server::{ELSResult, Server};

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn execute_command(&self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("execute command requested: {msg}"))?;
        let params = ExecuteCommandParams::deserialize(&msg["params"])?;
        let result: Vec<CodeAction> = match &params.command[..] {
            "els.eliminateUnusedVars" => {
                vec![]
            }
            _ => {
                Self::send_log(format!("unknown command: {}", params.command))?;
                vec![]
            }
        };
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }
}
