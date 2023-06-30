use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Number, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMessage {
    jsonrpc: String,
    id: Option<Number>,
    error: Value,
}

impl ErrorMessage {
    #[allow(dead_code)]
    pub fn new<N: Into<Number>>(id: Option<N>, error: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.map(|i| i.into()),
            error,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogMessage {
    jsonrpc: String,
    method: String,
    params: Value,
}

impl LogMessage {
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: "window/logMessage".into(),
            params: json! {
                {
                    "type": 3,
                    "message": message.into(),
                }
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowMessage {
    jsonrpc: String,
    method: String,
    params: Value,
}

impl ShowMessage {
    #[allow(unused)]
    pub fn info<S: Into<String>>(message: S) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: "window/showMessage".into(),
            params: json! {
                {
                    "type": 3,
                    "message": message.into(),
                }
            },
        }
    }

    pub fn error<S: Into<String>>(message: S) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: "window/showMessage".into(),
            params: json! {
                {
                    "type": 1,
                    "message": message.into(),
                }
            },
        }
    }
}

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
