use std::fs::File;
use std::io::{self, BufReader};
use std::io::{stdin, stdout, BufRead, Read, StdinLock, StdoutLock, Write};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;

use erg_common::config::{ErgConfig, Input};
use erg_common::style::*;
use erg_common::traits::{Locational, Stream};

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::build_hir::HIRBuilder;
use erg_compiler::context::Context;
use erg_compiler::erg_parser::ast::VarName;
use erg_compiler::erg_parser::lex::Lexer;
use erg_compiler::erg_parser::token::{Token, TokenCategory, TokenKind};
use erg_compiler::error::CompileErrors;
use erg_compiler::hir::HIR;
use erg_compiler::ty::Type;
use erg_compiler::varinfo::VarInfo;
use erg_compiler::AccessKind;

use lsp_types::{
    ClientCapabilities, CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams,
    Diagnostic, DiagnosticSeverity, GotoDefinitionParams, GotoDefinitionResponse, HoverContents,
    HoverParams, HoverProviderCapability, InitializeResult, MarkedString, OneOf, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
};

use crate::hir_visitor::HIRVisitor;
use crate::message::{ErrorMessage, LogMessage};

type ELSResult<T> = Result<T, Box<dyn std::error::Error>>;

pub type ErgLanguageServer = Server<HIRBuilder>;

/// A Language Server, which can be used any object implementing `BuildRunnable` internally by passing it as a generic parameter.
#[derive(Debug)]
pub struct Server<Checker: BuildRunnable = HIRBuilder> {
    cfg: ErgConfig,
    client_capas: ClientCapabilities,
    context: Option<Context>,
    hir: Option<HIR>, // TODO: should be ModuleCache
    input: StdinLock<'static>,
    output: StdoutLock<'static>,
    _checker: std::marker::PhantomData<Checker>,
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub fn new(cfg: ErgConfig) -> Self {
        let input = stdin().lock();
        let output = stdout().lock();
        Self {
            cfg,
            client_capas: ClientCapabilities::default(),
            context: None,
            hir: None,
            input,
            output,
            _checker: std::marker::PhantomData,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let msg = Value::from_str(&self.read_message()?)?;
            self.dispatch(msg)?;
        }
        // Ok(())
    }

    fn send<T: ?Sized + Serialize>(&mut self, message: &T) -> ELSResult<()> {
        let msg = serde_json::to_string(message)?;
        write!(self.output, "Content-Length: {}\r\n\r\n{}", msg.len(), msg)?;
        self.output.flush()?;
        Ok(())
    }

    fn send_log<S: Into<String>>(&mut self, msg: S) -> ELSResult<()> {
        self.send(&LogMessage::new(msg))
    }

    fn send_error<S: Into<String>>(&mut self, id: Option<i64>, code: i64, msg: S) -> ELSResult<()> {
        self.send(&ErrorMessage::new(
            id,
            json!({ "code": code, "message": msg.into() }),
        ))
    }

    fn send_invalid_req_error(&mut self) -> ELSResult<()> {
        self.send_error(None, -32601, "received an invalid request")
    }

    fn send_diagnostics(&mut self, uri: Url, diagnostics: Vec<Diagnostic>) -> ELSResult<()> {
        let params = PublishDiagnosticsParams::new(uri, diagnostics, None);
        if self
            .client_capas
            .text_document
            .as_ref()
            .map(|doc| doc.publish_diagnostics.is_some())
            .unwrap_or(false)
        {
            self.send(&json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": params,
            }))?;
        } else {
            self.send_log("the client does not support diagnostics")?;
        }
        Ok(())
    }

    #[allow(clippy::field_reassign_with_default)]
    fn init(&mut self, msg: &Value, id: i64) -> ELSResult<()> {
        self.send_log("initializing ELS")?;
        // #[allow(clippy::collapsible_if)]
        if msg.get("params").is_some() && msg["params"].get("capabilities").is_some() {
            self.client_capas = ClientCapabilities::deserialize(&msg["params"]["capabilities"])?;
            // self.send_log(format!("set client capabilities: {:?}", self.client_capas))?;
        }
        let mut result = InitializeResult::default();
        result.capabilities = ServerCapabilities::default();
        result.capabilities.text_document_sync =
            Some(TextDocumentSyncCapability::from(TextDocumentSyncKind::FULL));
        let mut comp_options = CompletionOptions::default();
        comp_options.trigger_characters = Some(vec![".".to_string(), ":".to_string()]);
        result.capabilities.completion_provider = Some(comp_options);
        result.capabilities.definition_provider = Some(OneOf::Left(true));
        result.capabilities.hover_provider = Some(HoverProviderCapability::Simple(true));
        result.capabilities.inlay_hint_provider = Some(OneOf::Left(true));
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }

    fn exit(&mut self) -> ELSResult<()> {
        self.send_log("exiting ELS")?;
        std::process::exit(0);
    }

    fn shutdown(&mut self, id: i64) -> ELSResult<()> {
        self.send_log("shutting down ELS")?;
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": json!(null),
        }))
    }

    /// Copied and modified from RLS, https://github.com/rust-lang/rls/blob/master/rls/src/server/io.rs
    fn read_message(&mut self) -> Result<String, io::Error> {
        // Read in the "Content-Length: xx" part.
        let mut size: Option<usize> = None;
        loop {
            let mut buffer = String::new();
            self.input.read_line(&mut buffer)?;

            // End of input.
            if buffer.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "EOF encountered in the middle of reading LSP headers",
                ));
            }

            // Header section is finished, break from the loop.
            if buffer == "\r\n" {
                break;
            }

            let res: Vec<&str> = buffer.split(' ').collect();

            // Make sure header is valid.
            if res.len() != 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Header '{}' is malformed", buffer),
                ));
            }
            let header_name = res[0].to_lowercase();
            let header_value = res[1].trim();

            match header_name.as_ref() {
                "content-length:" => {
                    size = Some(header_value.parse::<usize>().map_err(|_e| {
                        io::Error::new(io::ErrorKind::InvalidData, "Couldn't read size")
                    })?);
                }
                "content-type:" => {
                    if header_value != "utf8" && header_value != "utf-8" {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Content type '{}' is invalid", header_value),
                        ));
                    }
                }
                // Ignore unknown headers (specification doesn't say what to do in this case).
                _ => (),
            }
        }
        let size = match size {
            Some(size) => size,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Message is missing 'content-length' header",
                ));
            }
        };

        let mut content = vec![0; size];
        self.input.read_exact(&mut content)?;

        String::from_utf8(content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn dispatch(&mut self, msg: Value) -> ELSResult<()> {
        match (
            msg.get("id").and_then(|i| i.as_i64()),
            msg.get("method").and_then(|m| m.as_str()),
        ) {
            (Some(id), Some(method)) => self.handle_request(&msg, id, method),
            (Some(_id), None) => {
                // ignore at this time
                Ok(())
            }
            (None, Some(notification)) => self.handle_notification(&msg, notification),
            _ => self.send_invalid_req_error(),
        }
    }

    fn handle_request(&mut self, msg: &Value, id: i64, method: &str) -> ELSResult<()> {
        match method {
            "initialize" => self.init(msg, id),
            "shutdown" => self.shutdown(id),
            "textDocument/completion" => self.show_completion(msg),
            "textDocument/definition" => self.show_definition(msg),
            "textDocument/hover" => self.show_hover(msg),
            other => self.send_error(Some(id), -32600, format!("{other} is not supported")),
        }
    }

    fn handle_notification(&mut self, msg: &Value, method: &str) -> ELSResult<()> {
        match method {
            "initialized" => self.send_log("successfully bound"),
            "exit" => self.exit(),
            "textDocument/didOpen" => {
                let uri = Url::parse(msg["params"]["textDocument"]["uri"].as_str().unwrap())?;
                self.send_log(format!("{method}: {uri}"))?;
                self.check_file(uri, msg["params"]["textDocument"]["text"].as_str().unwrap())
            }
            "textDocument/didSave" => {
                let uri = Url::parse(msg["params"]["textDocument"]["uri"].as_str().unwrap())?;
                self.send_log(format!("{method}: {uri}"))?;
                let path = uri.to_file_path().unwrap();
                let mut code = String::new();
                File::open(path)?.read_to_string(&mut code)?;
                self.check_file(uri, &code)
            }
            // "textDocument/didChange"
            _ => self.send_log(format!("received notification: {}", method)),
        }
    }

    fn check_file<S: Into<String>>(&mut self, uri: Url, code: S) -> ELSResult<()> {
        self.send_log(format!("checking {uri}"))?;
        let path = uri.to_file_path().unwrap();
        let mode = if path.to_string_lossy().ends_with(".d.er") {
            "declare"
        } else {
            "exec"
        };
        let mut checker = Checker::new(self.cfg.inherit(path));
        match checker.build(code.into(), mode) {
            Ok(artifact) => {
                self.hir = Some(artifact.object);
                self.send_log(format!("checking {uri} passed"))?;
                let uri_and_diags = self.make_uri_and_diags(uri.clone(), artifact.warns);
                // clear previous diagnostics
                if uri_and_diags.is_empty() {
                    self.send_diagnostics(uri, vec![])?;
                }
                for (uri, diags) in uri_and_diags.into_iter() {
                    self.send_log(format!("{uri}, warns: {}", diags.len()))?;
                    self.send_diagnostics(uri, diags)?;
                }
            }
            Err(mut artifact) => {
                self.hir = artifact.object;
                self.send_log(format!("found errors: {}", artifact.errors.len()))?;
                self.send_log(format!("found warns: {}", artifact.warns.len()))?;
                artifact.errors.extend(artifact.warns);
                let uri_and_diags = self.make_uri_and_diags(uri.clone(), artifact.errors);
                if uri_and_diags.is_empty() {
                    self.send_diagnostics(uri, vec![])?;
                }
                for (uri, diags) in uri_and_diags.into_iter() {
                    self.send_log(format!("{uri}, errs & warns: {}", diags.len()))?;
                    self.send_diagnostics(uri, diags)?;
                }
            }
        }
        self.context = checker.pop_context();
        Ok(())
    }

    fn make_uri_and_diags(
        &mut self,
        uri: Url,
        errors: CompileErrors,
    ) -> Vec<(Url, Vec<Diagnostic>)> {
        let mut uri_and_diags: Vec<(Url, Vec<Diagnostic>)> = vec![];
        for err in errors.into_iter() {
            let loc = err.core.get_loc_with_fallback();
            let uri = if let Input::File(path) = err.input {
                Url::from_file_path(path).unwrap()
            } else {
                uri.clone()
            };
            let mut message = remove_style(&err.core.main_message);
            for sub in err.core.sub_messages {
                for msg in sub.get_msg() {
                    message.push('\n');
                    message.push_str(&remove_style(msg));
                }
                if let Some(hint) = sub.get_hint() {
                    message.push('\n');
                    message.push_str("hint: ");
                    message.push_str(&remove_style(hint));
                }
            }
            let start = Position::new(
                loc.ln_begin().unwrap_or(1) as u32 - 1,
                loc.col_begin().unwrap_or(0) as u32,
            );
            let end = Position::new(
                loc.ln_end().unwrap_or(1) as u32 - 1,
                loc.col_end().unwrap_or(0) as u32,
            );
            let severity = if err.core.kind.is_warning() {
                DiagnosticSeverity::WARNING
            } else {
                DiagnosticSeverity::ERROR
            };
            let diag = Diagnostic::new(
                Range::new(start, end),
                Some(severity),
                None,
                None,
                message,
                None,
                None,
            );
            if let Some((_, diags)) = uri_and_diags.iter_mut().find(|x| x.0 == uri) {
                diags.push(diag);
            } else {
                uri_and_diags.push((uri, vec![diag]));
            }
        }
        uri_and_diags
    }

    fn show_completion(&mut self, msg: &Value) -> ELSResult<()> {
        self.send_log(format!("completion requested: {msg}"))?;
        let params = CompletionParams::deserialize(&msg["params"])?;
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let trigger = params
            .context
            .as_ref()
            .and_then(|ctx| ctx.trigger_character.as_ref().map(|s| &s[..]));
        let acc = match trigger {
            Some(".") => AccessKind::Attr,
            Some(":") => AccessKind::Attr, // or type ascription
            _ => AccessKind::Name,
        };
        self.send_log(format!("AccessKind: {acc:?}"))?;
        let mut result = vec![];
        let context = if acc.is_local() {
            self.context.as_ref().unwrap()
        } else if let Some(ctx) = self.get_receiver_ctx(uri, pos)? {
            ctx
        } else {
            self.send(
                &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
            )?;
            return Ok(());
        };
        for (name, vi) in context.dir().into_iter() {
            if &context.name[..] != "<module>" && vi.vis.is_private() {
                continue;
            }
            let mut item = CompletionItem::new_simple(name.to_string(), vi.t.to_string());
            item.kind = match &vi.t {
                Type::Subr(subr) if subr.self_t().is_some() => Some(CompletionItemKind::METHOD),
                Type::Quantified(quant) if quant.self_t().is_some() => {
                    Some(CompletionItemKind::METHOD)
                }
                Type::Subr(_) | Type::Quantified(_) => Some(CompletionItemKind::FUNCTION),
                Type::ClassType => Some(CompletionItemKind::CLASS),
                Type::TraitType => Some(CompletionItemKind::INTERFACE),
                t if &t.qual_name()[..] == "Module" || &t.qual_name()[..] == "GenericModule" => {
                    Some(CompletionItemKind::MODULE)
                }
                _ if vi.muty.is_const() => Some(CompletionItemKind::CONSTANT),
                _ => Some(CompletionItemKind::VARIABLE),
            };
            result.push(item);
        }
        self.send_log(format!("completion items: {}", result.len()))?;
        self.send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }))
    }

    fn show_definition(&mut self, msg: &Value) -> ELSResult<()> {
        self.send_log(format!("definition requested: {msg}"))?;
        let params = GotoDefinitionParams::deserialize(&msg["params"])?;
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let result = if let Some(token) = Self::get_token(uri.clone(), pos)? {
            let prev = self.get_token_relatively(uri.clone(), pos, -1)?;
            // TODO: check attribute
            if prev
                .map(|t| t.is(TokenKind::Dot) || t.is(TokenKind::DblColon))
                .unwrap_or(false)
            {
                self.send_log("attribute")?;
                GotoDefinitionResponse::Array(vec![])
            } else if let Some((name, _vi)) = self.get_definition(&token)? {
                match Self::loc_to_range(name.loc()) {
                    Some(range) => {
                        self.send_log("found")?;
                        GotoDefinitionResponse::Array(vec![lsp_types::Location::new(uri, range)])
                    }
                    None => {
                        self.send_log("not found (maybe builtin)")?;
                        GotoDefinitionResponse::Array(vec![])
                    }
                }
            } else {
                GotoDefinitionResponse::Array(vec![])
            }
        } else {
            self.send_log("lex error occurred")?;
            GotoDefinitionResponse::Array(vec![])
        };
        self.send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }))
    }

    fn get_definition(&mut self, token: &Token) -> ELSResult<Option<(VarName, VarInfo)>> {
        if !token.category_is(TokenCategory::Symbol) {
            self.send_log(format!("not symbol: {token}"))?;
            Ok(None)
        } else if let Some((name, vi)) = self
            .context
            .as_ref()
            .and_then(|ctx| ctx.get_var_info(token.inspect()))
        {
            Ok(Some((name.clone(), vi.clone())))
        } else {
            self.send_log("not found")?;
            Ok(None)
        }
    }

    fn show_hover(&mut self, msg: &Value) -> ELSResult<()> {
        self.send_log(format!("hover requested : {msg}"))?;
        let lang = if cfg!(feature = "py_compatible") {
            "python"
        } else {
            "erg"
        };
        let params = HoverParams::deserialize(&msg["params"])?;
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let mut contents = vec![];
        let opt_tok = Self::get_token(uri.clone(), pos)?;
        let opt_token = if let Some(token) = opt_tok {
            match token.category() {
                TokenCategory::StrInterpRight => self.get_token_relatively(uri.clone(), pos, -1)?,
                TokenCategory::StrInterpLeft => self.get_token_relatively(uri.clone(), pos, 1)?,
                // TODO: StrInterpMid
                _ => Some(token),
            }
        } else {
            None
        };
        if let Some(token) = opt_token {
            match self.get_definition(&token)? {
                Some((name, vi)) => {
                    if let Some(line) = name.ln_begin() {
                        let path = uri.to_file_path().unwrap();
                        let code_block = BufReader::new(File::open(path)?)
                            .lines()
                            .nth(line - 1)
                            .unwrap_or_else(|| Ok(String::new()))?;
                        let definition = MarkedString::from_language_code(lang.into(), code_block);
                        contents.push(definition);
                    }
                    let typ =
                        MarkedString::from_language_code(lang.into(), format!("{name}: {}", vi.t));
                    contents.push(typ);
                }
                // not found or not symbol, etc.
                None => {
                    if let Some(hir) = &self.hir {
                        let visitor = HIRVisitor::new(hir, !cfg!(feature = "py_compatible"));
                        if let Some(typ) = visitor.visit_hir_t(&token) {
                            let typ = MarkedString::from_language_code(
                                lang.into(),
                                format!("{}: {typ}", token.content),
                            );
                            contents.push(typ);
                        }
                    }
                }
            }
        } else {
            self.send_log("lex error")?;
        }
        let result = json!({ "contents": HoverContents::Array(contents) });
        self.send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }))
    }

    fn loc_to_range(loc: erg_common::error::Location) -> Option<Range> {
        let start = Position::new(loc.ln_begin()? as u32 - 1, loc.col_begin()? as u32);
        let end = Position::new(loc.ln_end()? as u32 - 1, loc.col_end()? as u32);
        Some(Range::new(start, end))
    }

    fn pos_in_loc<L: Locational>(loc: &L, pos: Position) -> bool {
        (loc.ln_begin().unwrap_or(0)..=loc.ln_end().unwrap_or(0)).contains(&(pos.line as usize + 1))
            && (loc.col_begin().unwrap_or(0)..=loc.col_end().unwrap_or(0))
                .contains(&(pos.character as usize))
    }

    fn get_token(uri: Url, pos: Position) -> ELSResult<Option<Token>> {
        // FIXME: detect change
        let mut timeout = 300;
        let path = uri.to_file_path().unwrap();
        loop {
            let mut code = String::new();
            File::open(path.as_path())?.read_to_string(&mut code)?;
            if let Ok(tokens) = Lexer::from_str(code).lex() {
                let mut token = None;
                for tok in tokens.into_iter() {
                    if Self::pos_in_loc(&tok, pos) {
                        token = Some(tok);
                        break;
                    }
                }
                if token.is_some() {
                    return Ok(token);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            timeout -= 1;
            if timeout == 0 {
                return Ok(None);
            }
        }
    }

    /// plus_minus: 0 => same as get_token
    fn get_token_relatively(
        &mut self,
        uri: Url,
        pos: Position,
        plus_minus: isize,
    ) -> ELSResult<Option<Token>> {
        // FIXME: detect change
        let mut timeout = 300;
        let path = uri.to_file_path().unwrap();
        loop {
            let mut code = String::new();
            File::open(path.as_path())?.read_to_string(&mut code)?;
            if let Ok(tokens) = Lexer::from_str(code).lex() {
                let mut found_index = None;
                for (i, tok) in tokens.iter().enumerate() {
                    if Self::pos_in_loc(tok, pos) {
                        found_index = Some(i);
                        break;
                    }
                }
                if let Some(idx) = found_index {
                    if let Some(token) =
                        tokens.into_iter().nth((idx as isize + plus_minus) as usize)
                    {
                        if !token.is(TokenKind::Newline) {
                            return Ok(Some(token));
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            timeout -= 1;
            if timeout == 0 {
                return Ok(None);
            }
        }
    }

    fn get_receiver_ctx(
        &mut self,
        uri: Url,
        attr_marker_pos: Position,
    ) -> ELSResult<Option<&Context>> {
        let maybe_token = self.get_token_relatively(uri, attr_marker_pos, -1)?;
        if let Some(token) = maybe_token {
            if token.is(TokenKind::Symbol) {
                let var_name = token.inspect();
                self.send_log(format!("name: {var_name}"))?;
                let ctx = self
                    .context
                    .as_ref()
                    .and_then(|ctx| ctx.get_receiver_ctx(var_name))
                    .or_else(|| {
                        let opt_t = self.hir.as_ref().and_then(|hir| {
                            let visitor = HIRVisitor::new(hir, !cfg!(feature = "py_compatible"));
                            visitor.visit_hir_t(&token)
                        });
                        opt_t.and_then(|t| {
                            self.context
                                .as_ref()
                                .and_then(|ctx| ctx.get_receiver_ctx(&t.to_string()))
                        })
                    });
                Ok(ctx)
            } else {
                self.send_log(format!("non-name token: {token}"))?;
                if let Some(typ) = self.hir.as_ref().and_then(|hir| {
                    let visitor = HIRVisitor::new(hir, !cfg!(feature = "py_compatible"));
                    visitor.visit_hir_t(&token)
                }) {
                    let t_name = typ.qual_name();
                    self.send_log(format!("type: {t_name}"))?;
                    let ctx = self
                        .context
                        .as_ref()
                        .and_then(|ctx| ctx.get_receiver_ctx(&t_name));
                    Ok(ctx)
                } else {
                    Ok(None)
                }
            }
        } else {
            self.send_log("token not found")?;
            Ok(None)
        }
    }
}
