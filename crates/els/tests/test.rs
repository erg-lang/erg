use std::fs::File;
use std::io::Read;
use std::path::Path;

use lsp_types::{
    CompletionContext, CompletionParams, CompletionResponse, CompletionTriggerKind,
    ConfigurationParams, DidChangeTextDocumentParams, DidOpenTextDocumentParams, FoldingRange,
    FoldingRangeKind, FoldingRangeParams, GotoDefinitionParams, Hover, HoverContents, HoverParams,
    Location, MarkedString, Position, Range, ReferenceContext, ReferenceParams, RenameParams,
    SignatureHelp, SignatureHelpContext, SignatureHelpParams, SignatureHelpTriggerKind,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Url, VersionedTextDocumentIdentifier, WorkspaceEdit,
};
use serde::de::Deserialize;
use serde_json::{json, Value};

use els::{NormalizedUrl, Server, TRIGGER_CHARS};
use erg_common::config::ErgConfig;
use erg_common::spawn::safe_yield;

const FILE_A: &str = "tests/a.er";
const FILE_B: &str = "tests/b.er";
const FILE_IMPORTS: &str = "tests/imports.er";

fn add_char(line: u32, character: u32, text: &str) -> TextDocumentContentChangeEvent {
    TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line, character },
            end: Position { line, character },
        }),
        range_length: None,
        text: text.to_string(),
    }
}

fn abs_pos(uri: Url, line: u32, col: u32) -> TextDocumentPositionParams {
    TextDocumentPositionParams {
        text_document: TextDocumentIdentifier::new(uri),
        position: Position {
            line,
            character: col,
        },
    }
}

fn single_range(line: u32, from: u32, to: u32) -> Range {
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

fn parse_msgs(mut input: &str) -> Vec<Value> {
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
            .unwrap_or_else(|| panic!("len: {len}, input: {input}"));
        input = &input[len..];
        msgs.push(serde_json::from_str(msg).unwrap());
    }
    msgs
}

pub struct DummyClient {
    stdout_buffer: gag::BufferRedirect,
    ver: i32,
    server: Server,
}

impl Default for DummyClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyClient {
    pub fn new() -> Self {
        let stdout_buffer = loop {
            // wait until the other thread is finished
            match gag::BufferRedirect::stdout() {
                Ok(stdout_buffer) => break stdout_buffer,
                Err(_) => safe_yield(),
            }
        };
        DummyClient {
            stdout_buffer,
            ver: 0,
            server: Server::new(ErgConfig::default()),
        }
    }

    #[allow(dead_code)]
    fn wait_output(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let mut buf = String::new();
        loop {
            self.stdout_buffer.read_to_string(&mut buf)?;
            if buf.is_empty() {
                safe_yield();
            } else {
                break;
            }
        }
        Ok(buf)
    }

    /// the server periodically outputs health check messages
    fn wait_outputs(&mut self, mut size: usize) -> Result<String, Box<dyn std::error::Error>> {
        let mut buf = String::new();
        loop {
            self.stdout_buffer.read_to_string(&mut buf)?;
            if buf.is_empty() {
                safe_yield();
            } else {
                size -= 1;
                if size == 0 {
                    break;
                }
            }
        }
        Ok(buf)
    }

    fn wait_for<R>(&mut self) -> Result<R, Box<dyn std::error::Error>>
    where
        R: Deserialize<'static>,
    {
        loop {
            let mut buf = String::new();
            self.stdout_buffer.read_to_string(&mut buf)?;
            for msg in parse_msgs(&buf) {
                if msg.get("method").is_some_and(|_| msg.get("id").is_some()) {
                    self.handle_server_request(&msg);
                }
                if let Some(result) = msg
                    .get("result")
                    .cloned()
                    .and_then(|res| R::deserialize(res).ok())
                {
                    return Ok(result);
                }
            }
            safe_yield();
        }
    }

    fn handle_server_request(&mut self, msg: &Value) {
        if let Ok(_params) = ConfigurationParams::deserialize(&msg["params"]) {
            let msg = json!({
                "jsonrpc": "2.0",
                "id": msg["id"].as_i64().unwrap(),
                "result": null,
            });
            self.server.dispatch(msg).unwrap();
        }
    }

    fn request_initialize(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
        });
        self.server.dispatch(msg)?;
        let buf = self.wait_outputs(2)?;
        // eprintln!("`{}`", buf);
        Ok(buf)
    }

    fn notify_open(&mut self, file: &str) -> Result<String, Box<dyn std::error::Error>> {
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
        let buf = self.wait_outputs(1)?;
        // eprintln!("open: `{}`", buf);
        Ok(buf)
    }

    fn notify_change(
        &mut self,
        uri: Url,
        change: TextDocumentContentChangeEvent,
    ) -> Result<String, Box<dyn std::error::Error>> {
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
        let buf = self.wait_outputs(1)?;
        // eprintln!("{}: `{}`", line!(), buf);
        Ok(buf)
    }

    fn request_completion(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
        character: &str,
    ) -> Result<CompletionResponse, Box<dyn std::error::Error>> {
        let text_document_position = abs_pos(uri, line, col);
        let trigger_kind = if TRIGGER_CHARS.contains(&character) {
            CompletionTriggerKind::TRIGGER_CHARACTER
        } else {
            CompletionTriggerKind::INVOKED
        };
        let trigger_character = TRIGGER_CHARS
            .contains(&character)
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
            "id": 1,
            "method": "textDocument/completion",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<CompletionResponse>()
    }

    fn request_rename(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
        new_name: &str,
    ) -> Result<WorkspaceEdit, Box<dyn std::error::Error>> {
        let text_document_position = abs_pos(uri, line, col);
        let params = RenameParams {
            text_document_position,
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/rename",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<WorkspaceEdit>()
    }

    fn request_signature_help(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
        character: &str,
    ) -> Result<SignatureHelp, Box<dyn std::error::Error>> {
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
            "id": 1,
            "method": "textDocument/signatureHelp",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<SignatureHelp>()
    }

    fn request_hover(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
    ) -> Result<Hover, Box<dyn std::error::Error>> {
        let params = HoverParams {
            text_document_position_params: abs_pos(uri, line, col),
            work_done_progress_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Hover>()
    }

    fn request_references(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
    ) -> Result<Vec<Location>, Box<dyn std::error::Error>> {
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
            "id": 1,
            "method": "textDocument/references",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Vec<Location>>()
    }

    fn request_goto_definition(
        &mut self,
        uri: Url,
        line: u32,
        col: u32,
    ) -> Result<Vec<Location>, Box<dyn std::error::Error>> {
        let params = GotoDefinitionParams {
            text_document_position_params: abs_pos(uri, line, col),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/definition",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Vec<Location>>()
    }

    fn request_folding_range(
        &mut self,
        uri: Url,
    ) -> Result<Option<Vec<FoldingRange>>, Box<dyn std::error::Error>> {
        let params = FoldingRangeParams {
            text_document: TextDocumentIdentifier::new(uri),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/foldingRange",
            "params": params,
        });
        self.server.dispatch(msg)?;
        self.wait_for::<Option<Vec<FoldingRange>>>()
    }
}

#[test]
fn test_open() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let result = client.notify_open(FILE_A)?;
    assert!(result.contains("tests/a.er passed, found warns: 0"));
    Ok(())
}

#[test]
fn test_completion() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    client.notify_change(uri.clone().raw(), add_char(2, 0, "x"))?;
    client.notify_change(uri.clone().raw(), add_char(2, 1, "."))?;
    let resp = client.request_completion(uri.raw(), 2, 2, ".")?;
    if let CompletionResponse::Array(items) = resp {
        assert!(items.len() >= 40);
        assert!(items.iter().any(|item| item.label == "abs"));
        Ok(())
    } else {
        Err(format!("not items: {resp:?}").into())
    }
}

#[test]
fn test_neighbor_completion() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    client.notify_open(FILE_B)?;
    let resp = client.request_completion(uri.raw(), 2, 0, "n")?;
    if let CompletionResponse::Array(items) = resp {
        assert!(items.len() >= 40);
        assert!(items
            .iter()
            .any(|item| item.label == "neighbor (import from b)"));
        Ok(())
    } else {
        Err(format!("not items: {resp:?}").into())
    }
}

#[test]
fn test_rename() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let edit = client.request_rename(uri.clone().raw(), 1, 5, "y")?;
    assert!(edit
        .changes
        .is_some_and(|changes| changes.values().next().unwrap().len() == 2));
    Ok(())
}

#[test]
fn test_signature_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    client.notify_change(uri.clone().raw(), add_char(2, 0, "assert"))?;
    client.notify_change(uri.clone().raw(), add_char(2, 6, "("))?;
    let help = client.request_signature_help(uri.raw(), 2, 7, "(")?;
    assert_eq!(help.signatures.len(), 1);
    let sig = &help.signatures[0];
    assert_eq!(sig.label, "::assert: (test: Bool, msg := Str) -> NoneType");
    assert_eq!(sig.active_parameter, Some(0));
    Ok(())
}

#[test]
fn test_hover() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let hover = client.request_hover(uri.raw(), 1, 4)?;
    let HoverContents::Array(contents) = hover.contents else {
        todo!()
    };
    assert_eq!(contents.len(), 2);
    let MarkedString::LanguageString(content) = &contents[0] else {
        todo!()
    };
    assert!(
        content.value == "# tests/a.er, line 1\nx = 1"
            || content.value == "# tests\\a.er, line 1\nx = 1"
    );
    let MarkedString::LanguageString(content) = &contents[1] else {
        todo!()
    };
    assert_eq!(content.value, "x: {1}");
    Ok(())
}

#[test]
fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let locations = client.request_references(uri.raw(), 1, 4)?;
    assert_eq!(locations.len(), 1);
    assert_eq!(&locations[0].range, &single_range(1, 4, 5));
    Ok(())
}

#[test]
fn test_goto_definition() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let locations = client.request_goto_definition(uri.raw(), 1, 4)?;
    assert_eq!(locations.len(), 1);
    assert_eq!(&locations[0].range, &single_range(0, 0, 1));
    Ok(())
}

#[test]
fn test_folding_range() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DummyClient::new();
    client.request_initialize()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_IMPORTS).canonicalize()?)?;
    client.notify_open(FILE_IMPORTS)?;
    let ranges = client.request_folding_range(uri.raw())?.unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(
        &ranges[0],
        &FoldingRange {
            start_line: 0,
            start_character: Some(0),
            end_line: 3,
            end_character: Some(22),
            kind: Some(FoldingRangeKind::Imports),
        }
    );
    Ok(())
}
