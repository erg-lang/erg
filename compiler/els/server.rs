use std::cell::RefCell;
use std::io;
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
use erg_compiler::context::{Context, ModuleContext};
use erg_compiler::erg_parser::ast::VarName;
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
use crate::util;

pub type ELSResult<T> = Result<T, Box<dyn std::error::Error>>;

pub type ErgLanguageServer = Server<HIRBuilder>;

thread_local! {
    static INPUT: RefCell<StdinLock<'static>> = RefCell::new(stdin().lock());
    static OUTPUT: RefCell<StdoutLock<'static>> = RefCell::new(stdout().lock());
}

fn send_stdout<T: ?Sized + Serialize>(message: &T) -> ELSResult<()> {
    let msg = serde_json::to_string(message)?;
    OUTPUT.with(|out| {
        write!(
            out.borrow_mut(),
            "Content-Length: {}\r\n\r\n{}",
            msg.len(),
            msg
        )?;
        out.borrow_mut().flush()?;
        Ok(())
    })
}

fn read_line() -> io::Result<String> {
    let mut line = String::new();
    INPUT.with(|input| {
        input.borrow_mut().read_line(&mut line)?;
        Ok(line)
    })
}

fn read_exact(len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    INPUT.with(|input| {
        input.borrow_mut().read_exact(&mut buf)?;
        Ok(buf)
    })
}

/// A Language Server, which can be used any object implementing `BuildRunnable` internally by passing it as a generic parameter.
#[derive(Debug)]
pub struct Server<Checker: BuildRunnable = HIRBuilder> {
    cfg: ErgConfig,
    client_capas: ClientCapabilities,
    module: Option<ModuleContext>,
    hir: Option<HIR>, // TODO: should be ModuleCache
    _checker: std::marker::PhantomData<Checker>,
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            client_capas: ClientCapabilities::default(),
            module: None,
            hir: None,
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

    fn send<T: ?Sized + Serialize>(message: &T) -> ELSResult<()> {
        send_stdout(message)
    }

    pub(crate) fn send_log<S: Into<String>>(msg: S) -> ELSResult<()> {
        Self::send(&LogMessage::new(msg))
    }

    fn send_error<S: Into<String>>(id: Option<i64>, code: i64, msg: S) -> ELSResult<()> {
        Self::send(&ErrorMessage::new(
            id,
            json!({ "code": code, "message": msg.into() }),
        ))
    }

    fn send_invalid_req_error() -> ELSResult<()> {
        Self::send_error(None, -32601, "received an invalid request")
    }

    fn send_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) -> ELSResult<()> {
        let params = PublishDiagnosticsParams::new(uri, diagnostics, None);
        if self
            .client_capas
            .text_document
            .as_ref()
            .map(|doc| doc.publish_diagnostics.is_some())
            .unwrap_or(false)
        {
            Self::send(&json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": params,
            }))?;
        } else {
            Self::send_log("the client does not support diagnostics")?;
        }
        Ok(())
    }

    #[allow(clippy::field_reassign_with_default)]
    fn init(&mut self, msg: &Value, id: i64) -> ELSResult<()> {
        Self::send_log("initializing ELS")?;
        // #[allow(clippy::collapsible_if)]
        if msg.get("params").is_some() && msg["params"].get("capabilities").is_some() {
            self.client_capas = ClientCapabilities::deserialize(&msg["params"]["capabilities"])?;
            // Self::send_log(format!("set client capabilities: {:?}", self.client_capas))?;
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
        Self::send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }

    fn exit(&self) -> ELSResult<()> {
        Self::send_log("exiting ELS")?;
        std::process::exit(0);
    }

    fn shutdown(&self, id: i64) -> ELSResult<()> {
        Self::send_log("shutting down ELS")?;
        Self::send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": json!(null),
        }))
    }

    /// Copied and modified from RLS, https://github.com/rust-lang/rls/blob/master/rls/src/server/io.rs
    fn read_message(&self) -> Result<String, io::Error> {
        // Read in the "Content-Length: xx" part.
        let mut size: Option<usize> = None;
        loop {
            let buffer = read_line()?;

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

        let content = read_exact(size)?;

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
            _ => Self::send_invalid_req_error(),
        }
    }

    fn handle_request(&mut self, msg: &Value, id: i64, method: &str) -> ELSResult<()> {
        match method {
            "initialize" => self.init(msg, id),
            "shutdown" => self.shutdown(id),
            "textDocument/completion" => self.show_completion(msg),
            "textDocument/definition" => self.show_definition(msg),
            "textDocument/hover" => self.show_hover(msg),
            other => Self::send_error(Some(id), -32600, format!("{other} is not supported")),
        }
    }

    fn handle_notification(&mut self, msg: &Value, method: &str) -> ELSResult<()> {
        match method {
            "initialized" => Self::send_log("successfully bound"),
            "exit" => self.exit(),
            "textDocument/didOpen" => {
                let uri = Url::parse(msg["params"]["textDocument"]["uri"].as_str().unwrap())?;
                Self::send_log(format!("{method}: {uri}"))?;
                self.check_file(uri, msg["params"]["textDocument"]["text"].as_str().unwrap())
            }
            "textDocument/didSave" => {
                let uri = Url::parse(msg["params"]["textDocument"]["uri"].as_str().unwrap())?;
                Self::send_log(format!("{method}: {uri}"))?;
                let code = util::get_code_from_uri(&uri)?;
                self.check_file(uri, &code)
            }
            // "textDocument/didChange"
            _ => Self::send_log(format!("received notification: {}", method)),
        }
    }

    fn check_file<S: Into<String>>(&mut self, uri: Url, code: S) -> ELSResult<()> {
        Self::send_log(format!("checking {uri}"))?;
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
                Self::send_log(format!("checking {uri} passed"))?;
                let uri_and_diags = self.make_uri_and_diags(uri.clone(), artifact.warns);
                // clear previous diagnostics
                if uri_and_diags.is_empty() {
                    self.send_diagnostics(uri, vec![])?;
                }
                for (uri, diags) in uri_and_diags.into_iter() {
                    Self::send_log(format!("{uri}, warns: {}", diags.len()))?;
                    self.send_diagnostics(uri, diags)?;
                }
            }
            Err(mut artifact) => {
                self.hir = artifact.object;
                Self::send_log(format!("found errors: {}", artifact.errors.len()))?;
                Self::send_log(format!("found warns: {}", artifact.warns.len()))?;
                artifact.errors.extend(artifact.warns);
                let uri_and_diags = self.make_uri_and_diags(uri.clone(), artifact.errors);
                if uri_and_diags.is_empty() {
                    self.send_diagnostics(uri, vec![])?;
                }
                for (uri, diags) in uri_and_diags.into_iter() {
                    Self::send_log(format!("{uri}, errs & warns: {}", diags.len()))?;
                    self.send_diagnostics(uri, diags)?;
                }
            }
        }
        self.module = checker.pop_context();
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
                loc.ln_begin().unwrap_or(1) - 1,
                loc.col_begin().unwrap_or(0),
            );
            let end = Position::new(loc.ln_end().unwrap_or(1) - 1, loc.col_end().unwrap_or(0));
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

    fn get_visitor(&self, uri: &Url) -> Option<HIRVisitor> {
        self.hir
            .as_ref()
            .map(|hir| HIRVisitor::new(hir, uri.clone(), !cfg!(feature = "py_compatible")))
    }

    fn get_local_ctx(&self, uri: &Url, pos: Position) -> Vec<&Context> {
        // Self::send_log(format!("scope: {:?}\n", self.module.as_ref().unwrap().scope.keys())).unwrap();
        let mut ctxs = vec![];
        if let Some(visitor) = self.get_visitor(uri) {
            let ns = visitor.get_namespace(pos);
            Self::send_log(format!("ns: {ns:?}")).unwrap();
            for i in 1..ns.len() {
                let ns = ns[..=ns.len() - i].join("");
                if let Some(ctx) = self.module.as_ref().unwrap().scope.get(&ns[..]) {
                    ctxs.push(ctx);
                }
            }
        }
        ctxs.push(&self.module.as_ref().unwrap().context);
        ctxs
    }

    fn show_completion(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("completion requested: {msg}"))?;
        let params = CompletionParams::deserialize(&msg["params"])?;
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let trigger = params
            .context
            .as_ref()
            .and_then(|comp_ctx| comp_ctx.trigger_character.as_ref().map(|s| &s[..]));
        let acc_kind = match trigger {
            Some(".") => AccessKind::Attr,
            Some(":") => AccessKind::Attr, // or type ascription
            _ => AccessKind::Name,
        };
        Self::send_log(format!("AccessKind: {acc_kind:?}"))?;
        let mut result: Vec<CompletionItem> = vec![];
        let contexts = if acc_kind.is_local() {
            self.get_local_ctx(&uri, pos)
        } else {
            self.get_receiver_ctxs(uri, pos)?
        };
        // Self::send_log(format!("contexts: {:?}", contexts.iter().map(|ctx| &ctx.name).collect::<Vec<_>>())).unwrap();
        for (name, vi) in contexts.into_iter().flat_map(|ctx| ctx.dir()) {
            if acc_kind.is_attr() && vi.vis.is_private() {
                continue;
            }
            // don't show overriden items
            if result
                .iter()
                .any(|item| item.label[..] == name.inspect()[..])
            {
                continue;
            }
            // don't show future defined items
            if name.ln_begin().unwrap_or(0) > pos.line + 1 {
                continue;
            }
            let readable_t = self
                .module
                .as_ref()
                .map(|module| {
                    module
                        .context
                        .readable_type(vi.t.clone(), vi.kind.is_parameter())
                })
                .unwrap_or_else(|| vi.t.clone());
            let mut item = CompletionItem::new_simple(name.to_string(), readable_t.to_string());
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
        Self::send_log(format!("completion items: {}", result.len()))?;
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }

    fn show_definition(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("definition requested: {msg}"))?;
        let params = GotoDefinitionParams::deserialize(&msg["params"])?;
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let result = if let Some(token) = util::get_token(uri.clone(), pos)? {
            let prev = util::get_token_relatively(uri.clone(), pos, -1)?;
            // TODO: check attribute
            if prev
                .map(|t| t.is(TokenKind::Dot) || t.is(TokenKind::DblColon))
                .unwrap_or(false)
            {
                Self::send_log("attribute")?;
                GotoDefinitionResponse::Array(vec![])
            } else if let Some((name, _vi)) = self.get_definition(&token)? {
                match util::loc_to_range(name.loc()) {
                    Some(range) => {
                        Self::send_log("found")?;
                        GotoDefinitionResponse::Array(vec![lsp_types::Location::new(uri, range)])
                    }
                    None => {
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

    fn get_definition(&mut self, token: &Token) -> ELSResult<Option<(VarName, VarInfo)>> {
        if !token.category_is(TokenCategory::Symbol) {
            Self::send_log(format!("not symbol: {token}"))?;
            Ok(None)
        } else if let Some((name, vi)) = self
            .module
            .as_ref()
            .and_then(|module| module.context.get_var_info(token.inspect()))
        {
            Ok(Some((name.clone(), vi.clone())))
        } else {
            Self::send_log("not found")?;
            Ok(None)
        }
    }

    fn show_hover(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("hover requested : {msg}"))?;
        let lang = if cfg!(feature = "py_compatible") {
            "python"
        } else {
            "erg"
        };
        let params = HoverParams::deserialize(&msg["params"])?;
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let mut contents = vec![];
        let opt_tok = util::get_token(uri.clone(), pos)?;
        let opt_token = if let Some(token) = opt_tok {
            match token.category() {
                TokenCategory::StrInterpRight => util::get_token_relatively(uri.clone(), pos, -1)?,
                TokenCategory::StrInterpLeft => util::get_token_relatively(uri.clone(), pos, 1)?,
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
                        let code_block = util::get_line_from_uri(&uri, line)?;
                        let definition = MarkedString::from_language_code(lang.into(), code_block);
                        contents.push(definition);
                    }
                    let typ =
                        MarkedString::from_language_code(lang.into(), format!("{name}: {}", vi.t));
                    contents.push(typ);
                }
                // not found or not symbol, etc.
                None => {
                    if let Some(visitor) = self.get_visitor(&uri) {
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
            Self::send_log("lex error")?;
        }
        let result = json!({ "contents": HoverContents::Array(contents) });
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }

    fn get_receiver_ctxs(&self, uri: Url, attr_marker_pos: Position) -> ELSResult<Vec<&Context>> {
        let Some(module) = self.module.as_ref() else {
            return Ok(vec![]);
        };
        let maybe_token = util::get_token_relatively(uri.clone(), attr_marker_pos, -1)?;
        if let Some(token) = maybe_token {
            if token.is(TokenKind::Symbol) {
                let var_name = token.inspect();
                Self::send_log(format!("{} name: {var_name}", line!()))?;
                Ok(module.context.get_receiver_ctxs(var_name))
            } else {
                Self::send_log(format!("non-name token: {token}"))?;
                if let Some(typ) = self
                    .get_visitor(&uri)
                    .and_then(|visitor| visitor.visit_hir_t(&token))
                {
                    let t_name = typ.qual_name();
                    Self::send_log(format!("type: {t_name}"))?;
                    Ok(module.context.get_receiver_ctxs(&t_name))
                } else {
                    Ok(vec![])
                }
            }
        } else {
            Self::send_log("token not found")?;
            Ok(vec![])
        }
    }
}
