use serde::Serialize;

pub use molc::messages::ErrorMessage;

#[derive(Serialize)]
pub struct LSPResult<R: Serialize> {
    jsonrpc: String,
    id: i64,
    result: R,
}

impl<R: Serialize> LSPResult<R> {
    pub fn new(id: i64, result: R) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result,
        }
    }
}
