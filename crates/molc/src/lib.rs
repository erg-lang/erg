pub mod messages;

use std::fs::File;
use std::io::{stdout, Read, Write};
use std::path::Path;
use std::{collections::HashMap, sync::mpsc::Sender};

use lsp_types::{
    CompletionContext, CompletionParams, CompletionResponse, CompletionTriggerKind,
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, DocumentSymbolParams,
    DocumentSymbolResponse, FoldingRange, FoldingRangeParams, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverParams, InitializeResult, Location, Position, Range,
    ReferenceContext, ReferenceParams, RenameParams, ServerCapabilities, SignatureHelp,
    SignatureHelpContext, SignatureHelpParams, SignatureHelpTriggerKind,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Url, VersionedTextDocumentIdentifier, WorkspaceEdit,
};
use serde::de::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};

use crate::messages::{ErrorMessage, LogMessage, ShowMessage};

fn safe_yield() {
    std::thread::yield_now();
    std::thread::sleep(std::time::Duration::from_millis(10));
}

pub fn add_char(line: u32, character: u32, text: &str) -> TextDocumentContentChangeEvent {
    TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line, character },
            end: Position { line, character },
        }),
        range_length: None,
        text: text.to_string(),
    }
}

pub fn abs_pos(uri: Url, line: u32, col: u32) -> TextDocumentPositionParams {
    TextDocumentPositionParams {
        text_document: TextDocumentIdentifier::new(uri),
        position: Position {
            line,
            character: col,
        },
    }
}

pub fn oneline_range(line: u32, from: u32, to: u32) -> Range {
    Range {
        start: Position {
            line,
            character: from,
        },
        end: Position {
            line,
            character: to,
        },
    }
}

pub fn parse_msgs(_input: &str) -> Vec<Value> {
    let mut input = _input;
    let mut msgs = Vec::new();
    loop {
        if input.starts_with("Content-Length: ") {
            let idx = "Content-Length: ".len();
            input = &input[idx..];
        } else {
            break;
        }
        let dights = input.find("\r\n").unwrap();
        let len = input[..dights].parse::<usize>().unwrap();
        let idx = dights + "\r\n\r\n".len();
        input = &input[idx..];
        let msg = &input
            .get(..len)
            .unwrap_or_else(|| panic!("len: {len}, input: `{input}` -> _input: `{_input}`"));
        input = &input[len..];
        msgs.push(serde_json::from_str(msg).unwrap());
    }
    msgs
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub trait RedirectableStdout {
    fn sender(&self) -> Option<&Sender<Value>>;

    fn send_stdout<T: ?Sized + Serialize>(&self, message: &T) -> Result<()> {
        if let Some(sender) = self.sender() {
            sender.send(serde_json::to_value(message)?)?;
        } else {
            let msg = serde_json::to_string(message)?;
            let mut stdout = stdout().lock();
            write!(stdout, "Content-Length: {}\r\n\r\n{}", msg.len(), msg)?;
            stdout.flush()?;
        }
        Ok(())
    }

    fn send_log<S: Into<String>>(&self, msg: S) -> Result<()> {
        if cfg!(debug_assertions) || cfg!(feature = "debug") {
            self.send_stdout(&LogMessage::new(msg))
        } else {
            Ok(())
        }
    }

    #[allow(unused)]
    fn send_info<S: Into<String>>(&self, msg: S) -> Result<()> {
        self.send_stdout(&ShowMessage::info(msg))
    }

    fn send_error_info<S: Into<String>>(&self, msg: S) -> Result<()> {
        self.send_stdout(&ShowMessage::error(msg))
    }

    fn send_error<S: Into<String>>(&self, id: Option<i64>, code: i64, msg: S) -> Result<()> {
        self.send_stdout(&ErrorMessage::new(
            id,
            json!({ "code": code, "message": msg.into() }),
        ))
    }

    fn send_invalid_req_error(&self) -> Result<()> {
        self.send_error(None, -32601, "received an invalid request")
    }
}

pub trait LangServer {
    fn dispatch(&mut self, msg: impl Into<Value>) -> Result<()>;
}

pub struct DummyClient<LS: LangServer> {
    server: LS,
    receiver: std::sync::mpsc::Receiver<Value>,
    server_capas: Option<ServerCapabilities>,
    pub responses: Vec<Value>,
    #[allow(clippy::complexity)]
    request_handlers: HashMap<String, Box<dyn Fn(&Value, &mut LS) -> Result<()>>>,
    ver: i32,
    req_id: i64,
}

impl<LS: LangServer> DummyClient<LS> {
    /// The server should send responses to the channel at least during testing.
    pub fn new(server: LS, receiver: std::sync::mpsc::Receiver<Value>) -> Self {
        DummyClient {
            receiver,
            responses: Vec::new(),
            ver: 0,
            req_id: 0,
            server_capas: None,
            request_handlers: HashMap::new(),
            server,
        }
    }

    pub fn add_request_handler(
        &mut self,
        method_name: impl Into<String>,
        handler: impl Fn(&Value, &mut LS) -> Result<()> + 'static,
    ) {
        self.request_handlers
            .insert(method_name.into(), Box::new(handler));
    }

    pub fn remove_request_handler(&mut self, method_name: &str) {
        self.request_handlers.remove(method_name);
    }

    /// Waits for `n` messages to be received.
    /// When a request is received, the registered handler will be executed.
    pub fn wait_messages(&mut self, n: usize) -> Result<()> {
        for _ in 0..n {
            if let Ok(msg) = self.receiver.recv() {
                if msg.get("method").is_some_and(|_| msg.get("id").is_some()) {
                    self.handle_server_request(&msg);
                }
                self.responses.push(msg);
            }
        }
        Ok(())
    }

    /// Waits for a response to the request, where its `id` is expected to be that of `req_id`,
    /// and `req_id` will be incremented if the response is successfully received.
    /// When a request is received, the registered handler will be executed.
    fn wait_for<R>(&mut self) -> Result<R>
    where
        R: Deserialize<'static>,
    {
        loop {
            if let Ok(msg) = self.receiver.recv() {
                if msg.get("method").is_some_and(|_| msg.get("id").is_some()) {
                    self.handle_server_request(&msg);
                }
                self.responses.push(msg);
                let msg = self.responses.last().unwrap();
                if msg.get("id").is_some_and(|val| val == self.req_id) {
                    if let Some(result) = msg
                        .get("result")
                        .cloned()
                        .and_then(|res| R::deserialize(res).ok())
                    {
                        self.req_id += 1;
                        return Ok(result);
                    }
                }
            }
            safe_yield();
        }
    }

    fn handle_server_request(&mut self, msg: &Value) {
        if let Some(method) = msg.get("method").and_then(|val| val.as_str()) {
            if let Some(handler) = self.request_handlers.get(method) {
                if let Err(err) = handler(msg, &mut self.server) {
                    eprintln!("error: {:?}", err);
                }
            }
        }
    }

    /// This will set the server capabilities
    pub fn request_initialize(&mut self) -> Result<InitializeResult> {
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "initialize",
        });
        self.server.dispatch(msg)?;
        let res = self.wait_for::<InitializeResult>()?;
        self.server_capas = Some(res.capabilities.clone());
        Ok(res)
    }

    pub fn notify_open(&mut self, file: &str) -> Result<()> {
        let uri = Url::from_file_path(Path::new(file).canonicalize().unwrap()).unwrap();
        let mut text = String::new();
        File::open(file).unwrap().read_to_string(&mut text)?;
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(uri, "erg".to_string(), self.ver, text),
        };
        self.ver += 1;
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": params,
        });
        self.server.dispatch(msg)?;
        Ok(())
    }

    pub fn notify_change(
        &mut self,
        uri: Url,
        change: TextDocumentContentChangeEvent,
    ) -> Result<()> {
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier::new(uri.clone(), self.ver),
            content_changes: vec![change],
        };
        self.ver += 1;
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": params,
        });
        self.server.dispatch(msg)?;
        Ok(())
    }

    fn is_trigger_char(&self, character: &str) -> bool {
        self.server_capas.as_ref().is_some_and(|cap| {
            cap.completion_provider.as_ref().is_some_and(|comp| {
                comp.trigger_characters
                    .as_ref()
                    .is_some_and(|chars| chars.iter().any(|c| c == character))
            })
        })
    }

    pub fn request_completion(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
        character: &str,
    ) -> Result<Option<CompletionResponse>> {
        let text_document_position = abs_pos(uri, line, col);
        let trigger_kind = if self.is_trigger_char(character) {
            CompletionTriggerKind::TRIGGER_CHARACTER
        } else {
            CompletionTriggerKind::INVOKED
        };
        let trigger_character = self
            .is_trigger_char(character)
            .then_some(character.to_string());
        let context = Some(CompletionContext {
            trigger_kind,
            trigger_character,
        });
        let params = CompletionParams {
            text_document_position,
            context,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "textDocument/completion",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<CompletionResponse>>()
    }

    pub fn request_rename(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>> {
        let text_document_position = abs_pos(uri, line, col);
        let params = RenameParams {
            text_document_position,
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "textDocument/rename",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<WorkspaceEdit>>()
    }

    pub fn request_signature_help(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
        character: &str,
    ) -> Result<Option<SignatureHelp>> {
        let text_document_position_params = abs_pos(uri, line, col);
        let context = SignatureHelpContext {
            trigger_kind: SignatureHelpTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(character.to_string()),
            is_retrigger: false,
            active_signature_help: None,
        };
        let params = SignatureHelpParams {
            text_document_position_params,
            context: Some(context),
            work_done_progress_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "textDocument/signatureHelp",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<SignatureHelp>>()
    }

    pub fn request_hover(&mut self, uri: Url, line: u32, col: u32) -> Result<Option<Hover>> {
        let params = HoverParams {
            text_document_position_params: abs_pos(uri, line, col),
            work_done_progress_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "textDocument/hover",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<Hover>>()
    }

    pub fn request_references(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
    ) -> Result<Option<Vec<Location>>> {
        let context = ReferenceContext {
            include_declaration: false,
        };
        let params = ReferenceParams {
            text_document_position: abs_pos(uri, line, col),
            context,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "textDocument/references",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<Vec<Location>>>()
    }

    pub fn request_goto_definition(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let params = GotoDefinitionParams {
            text_document_position_params: abs_pos(uri, line, col),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "textDocument/definition",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<GotoDefinitionResponse>>()
    }

    pub fn request_folding_range(&mut self, uri: Url) -> Result<Option<Vec<FoldingRange>>> {
        let params = FoldingRangeParams {
            text_document: TextDocumentIdentifier::new(uri),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "textDocument/foldingRange",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<Vec<FoldingRange>>>()
    }

    pub fn request_document_symbols(&mut self, uri: Url) -> Result<Option<DocumentSymbolResponse>> {
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier::new(uri),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "textDocument/documentSymbol",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<DocumentSymbolResponse>>()
    }
}
