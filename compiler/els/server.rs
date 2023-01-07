use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::io::{stdin, stdout, BufRead, Read, StdinLock, StdoutLock, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::SystemTime;

use erg_common::env::erg_path;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::traits::{Locational, Stream};
use erg_common::{normalize_path, style::*};

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::build_hir::HIRBuilder;
use erg_compiler::context::{Context, ModuleContext};
use erg_compiler::erg_parser::token::{Token, TokenCategory, TokenKind};
use erg_compiler::error::CompileErrors;
use erg_compiler::hir::HIR;
use erg_compiler::module::{SharedCompilerResource, SharedModuleIndex};
use erg_compiler::ty::Type;
use erg_compiler::varinfo::{AbsLocation, VarInfo, VarKind};
use erg_compiler::AccessKind;

use lsp_types::{
    ClientCapabilities, CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams,
    Diagnostic, DiagnosticSeverity, GotoDefinitionParams, GotoDefinitionResponse, HoverContents,
    HoverParams, HoverProviderCapability, InitializeResult, MarkedString, OneOf, Position,
    PublishDiagnosticsParams, Range, ReferenceParams, RenameParams, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url, WorkspaceEdit,
};

use crate::hir_visitor::HIRVisitor;
use crate::message::{ErrorMessage, LogMessage};
use crate::util;

pub type ELSResult<T> = Result<T, Box<dyn std::error::Error>>;

pub type ErgLanguageServer = Server<HIRBuilder>;

macro_rules! _log {
    ($($arg:tt)*) => {
        Self::send_log(format!($($arg)*)).unwrap();
    };
}

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
    home: PathBuf,
    erg_path: PathBuf,
    client_capas: ClientCapabilities,
    modules: Dict<Url, ModuleContext>,
    hirs: Dict<Url, Option<HIR>>,
    _checker: std::marker::PhantomData<Checker>,
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            home: normalize_path(std::env::current_dir().unwrap()),
            erg_path: erg_path(), // already normalized
            client_capas: ClientCapabilities::default(),
            modules: Dict::new(),
            hirs: Dict::new(),
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
        result.capabilities.rename_provider = Some(OneOf::Left(true));
        result.capabilities.references_provider = Some(OneOf::Left(true));
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
            "textDocument/rename" => self.rename(msg),
            "textDocument/references" => self.show_references(msg),
            other => Self::send_error(Some(id), -32600, format!("{other} is not supported")),
        }
    }

    fn handle_notification(&mut self, msg: &Value, method: &str) -> ELSResult<()> {
        match method {
            "initialized" => Self::send_log("successfully bound"),
            "exit" => self.exit(),
            "textDocument/didOpen" => {
                let uri = util::parse_and_normalize_url(
                    msg["params"]["textDocument"]["uri"].as_str().unwrap(),
                )?;
                Self::send_log(format!("{method}: {uri}"))?;
                self.check_file(uri, msg["params"]["textDocument"]["text"].as_str().unwrap())
            }
            "textDocument/didSave" => {
                let uri = util::parse_and_normalize_url(
                    msg["params"]["textDocument"]["uri"].as_str().unwrap(),
                )?;
                Self::send_log(format!("{method}: {uri}"))?;
                let code = util::get_code_from_uri(&uri)?;
                self.clear_cache(&uri);
                self.check_file(uri, &code)
            }
            // "textDocument/didChange"
            _ => Self::send_log(format!("received notification: {}", method)),
        }
    }

    fn check_file<S: Into<String>>(&mut self, uri: Url, code: S) -> ELSResult<()> {
        Self::send_log(format!("checking {uri}"))?;
        let path = util::uri_to_path(&uri);
        let mode = if path.to_string_lossy().ends_with(".d.er") {
            "declare"
        } else {
            "exec"
        };
        let mut checker = if let Some(shared) = self.get_shared() {
            Checker::inherit(self.cfg.inherit(path), shared.clone())
        } else {
            Checker::new(self.cfg.inherit(path))
        };
        match checker.build(code.into(), mode) {
            Ok(artifact) => {
                self.hirs.insert(uri.clone(), Some(artifact.object));
                Self::send_log(format!("checking {uri} passed"))?;
                let uri_and_diags = self.make_uri_and_diags(uri.clone(), artifact.warns);
                // clear previous diagnostics
                if uri_and_diags.is_empty() {
                    self.send_diagnostics(uri.clone(), vec![])?;
                }
                for (uri, diags) in uri_and_diags.into_iter() {
                    Self::send_log(format!("{uri}, warns: {}", diags.len()))?;
                    self.send_diagnostics(uri, diags)?;
                }
            }
            Err(mut artifact) => {
                self.hirs.insert(uri.clone(), artifact.object);
                Self::send_log(format!("found errors: {}", artifact.errors.len()))?;
                Self::send_log(format!("found warns: {}", artifact.warns.len()))?;
                artifact.errors.extend(artifact.warns);
                let uri_and_diags = self.make_uri_and_diags(uri.clone(), artifact.errors);
                if uri_and_diags.is_empty() {
                    self.send_diagnostics(uri.clone(), vec![])?;
                }
                for (uri, diags) in uri_and_diags.into_iter() {
                    Self::send_log(format!("{uri}, errs & warns: {}", diags.len()))?;
                    self.send_diagnostics(uri, diags)?;
                }
            }
        }
        if let Some(module) = checker.pop_context() {
            Self::send_log(format!("{uri}: {}", module.context.name))?;
            self.modules.insert(uri.clone(), module);
        }
        let dependents = self.dependents_of(&uri);
        for dep in dependents {
            // _log!("dep: {dep}");
            let code = util::get_code_from_uri(&dep)?;
            self.check_file(dep, code)?;
        }
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
            let err_uri = if let Some(path) = err.input.path() {
                util::normalize_url(Url::from_file_path(path).unwrap())
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
            if let Some((_, diags)) = uri_and_diags.iter_mut().find(|x| x.0 == err_uri) {
                diags.push(diag);
            } else {
                uri_and_diags.push((err_uri, vec![diag]));
            }
        }
        uri_and_diags
    }

    fn get_visitor(&self, uri: &Url) -> Option<HIRVisitor> {
        self.hirs
            .get(uri)?
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
                if let Some(ctx) = self.modules.get(uri).unwrap().scope.get(&ns[..]) {
                    ctxs.push(ctx);
                }
            }
        }
        ctxs.push(&self.modules.get(uri).unwrap().context);
        ctxs
    }

    fn get_receiver_ctxs(&self, uri: &Url, attr_marker_pos: Position) -> ELSResult<Vec<&Context>> {
        let Some(module) = self.modules.get(uri) else {
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
                    .get_visitor(uri)
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

    fn show_completion(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("completion requested: {msg}"))?;
        let params = CompletionParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position.text_document.uri);
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
            self.get_receiver_ctxs(&uri, pos)?
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
                .modules
                .get(&uri)
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
        let uri = util::normalize_url(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        let result = if let Some(token) = util::get_token(uri.clone(), pos)? {
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

    fn get_definition(&mut self, uri: &Url, token: &Token) -> ELSResult<Option<VarInfo>> {
        if !token.category_is(TokenCategory::Symbol) {
            Self::send_log(format!("not symbol: {token}"))?;
            Ok(None)
        } else if let Some(visitor) = self.get_visitor(uri) {
            Ok(visitor.visit_hir_info(token))
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
        let uri = util::normalize_url(params.text_document_position_params.text_document.uri);
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
            match self.get_definition(&uri, &token)? {
                Some(vi) => {
                    if let Some(line) = vi.def_loc.loc.ln_begin() {
                        let file_path = vi.def_loc.module.unwrap();
                        let mut code_block = if cfg!(not(windows)) {
                            let relative = file_path
                                .strip_prefix(&self.home)
                                .unwrap_or(file_path.as_path());
                            let relative =
                                relative.strip_prefix(&self.erg_path).unwrap_or(relative);
                            format!("# {}, line {line}\n", relative.display())
                        } else {
                            // windows' file paths are case-insensitive, so we need to normalize them
                            let lower = file_path.as_os_str().to_ascii_lowercase();
                            let verbatim_removed = lower.to_str().unwrap().replace("\\\\?\\", "");
                            let relative = verbatim_removed
                                .strip_prefix(
                                    self.home.as_os_str().to_ascii_lowercase().to_str().unwrap(),
                                )
                                .unwrap_or_else(|| file_path.as_path().to_str().unwrap())
                                .trim_start_matches(['\\', '/']);
                            let relative = relative
                                .strip_prefix(self.erg_path.to_str().unwrap())
                                .unwrap_or(relative);
                            format!("# {}, line {line}\n", relative)
                        };
                        code_block += util::get_line_from_path(&file_path, line)?.trim_start();
                        if code_block.ends_with(&['=', '>']) {
                            code_block += " ...";
                        }
                        let definition = MarkedString::from_language_code(lang.into(), code_block);
                        contents.push(definition);
                    }
                    let typ = MarkedString::from_language_code(
                        lang.into(),
                        format!("{}: {}", token.content, vi.t),
                    );
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

    fn rename(&mut self, msg: &Value) -> ELSResult<()> {
        let params = RenameParams::deserialize(&msg["params"])?;
        Self::send_log(format!("rename request: {params:?}"))?;
        let uri = util::normalize_url(params.text_document_position.text_document.uri);
        let pos = params.text_document_position.position;
        if let Some(tok) = util::get_token(uri.clone(), pos)? {
            // Self::send_log(format!("token: {tok}"))?;
            if let Some(visitor) = self.get_visitor(&uri) {
                if let Some(vi) = visitor.visit_hir_info(&tok) {
                    // Self::send_log(format!("vi: {vi}"))?;
                    let is_std = vi
                        .def_loc
                        .module
                        .as_ref()
                        .map(|path| path.starts_with(&self.erg_path))
                        .unwrap_or(false);
                    if vi.def_loc.loc.is_unknown() || is_std {
                        let error_reason = match vi.kind {
                            VarKind::Builtin => "this is a builtin variable and cannot be renamed",
                            VarKind::FixedAuto => {
                                "this is a fixed auto variable and cannot be renamed"
                            }
                            _ if is_std => "this is a standard library API and cannot be renamed",
                            _ => "this name cannot be renamed",
                        };
                        return Self::send_error(msg["id"].as_i64(), 0, error_reason);
                    }
                    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
                    Self::commit_change(&mut changes, &vi.def_loc, params.new_name.clone());
                    if let Some(referrers) = self.get_index().get_refs(&vi.def_loc) {
                        // Self::send_log(format!("referrers: {referrers:?}"))?;
                        for referrer in referrers.iter() {
                            Self::commit_change(&mut changes, referrer, params.new_name.clone());
                        }
                    }
                    let dependencies = self.dependencies_of(&uri);
                    for uri in changes.keys() {
                        self.clear_cache(uri);
                    }
                    let timestamps = self.get_timestamps(changes.keys());
                    let edit = WorkspaceEdit::new(changes);
                    Self::send(
                        &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": edit }),
                    )?;
                    for _ in 0..20 {
                        Self::send_log("waiting for file to be modified...")?;
                        if self.all_changed(&timestamps) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    // recheck dependencies and finally the file itself
                    for dep in dependencies {
                        let code = util::get_code_from_uri(&dep)?;
                        self.check_file(dep, code)?;
                    }
                    // dependents are checked after changes are committed
                    return Ok(());
                }
            }
        }
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": Value::Null }),
        )
    }

    fn commit_change(
        changes: &mut HashMap<Url, Vec<TextEdit>>,
        abs_loc: &AbsLocation,
        new_name: String,
    ) {
        if let Some(path) = &abs_loc.module {
            let def_uri = util::normalize_url(Url::from_file_path(path).unwrap());
            let edit = TextEdit::new(util::loc_to_range(abs_loc.loc).unwrap(), new_name);
            if let Some(edits) = changes.get_mut(&def_uri) {
                edits.push(edit);
            } else {
                changes.insert(def_uri, vec![edit]);
            }
        }
    }

    fn get_timestamps<'a, I: Iterator<Item = &'a Url>>(&self, urls: I) -> Dict<Url, SystemTime> {
        urls.map(|url| {
            let timestamp = util::get_metadata_from_uri(url)
                .and_then(|md| Ok(md.modified()?))
                .unwrap();
            (url.clone(), timestamp)
        })
        .collect()
    }

    fn all_changed(&self, timestamps: &Dict<Url, SystemTime>) -> bool {
        timestamps.iter().all(|(url, timestamp)| {
            util::get_metadata_from_uri(url)
                .and_then(|md| Ok(md.modified()? != *timestamp))
                .unwrap_or(false)
        })
    }

    /// self is __included__
    fn dependencies_of(&self, uri: &Url) -> Vec<Url> {
        let graph = &self.get_shared().unwrap().graph;
        let path = util::uri_to_path(uri);
        graph.sort().unwrap();
        let self_node = graph.get_node(&path).unwrap();
        graph
            .iter()
            .filter(|node| node.id == path || self_node.depends_on(&node.id))
            .map(|node| util::normalize_url(Url::from_file_path(&node.id).unwrap()))
            .collect()
    }

    /// self is __not included__
    pub fn dependents_of(&self, uri: &Url) -> Vec<Url> {
        let graph = &self.get_shared().unwrap().graph;
        let path = util::uri_to_path(uri);
        graph
            .iter()
            .filter(|node| node.depends_on(&path))
            .map(|node| util::normalize_url(Url::from_file_path(&node.id).unwrap()))
            .collect()
    }

    fn show_references(&self, msg: &Value) -> ELSResult<()> {
        let params = ReferenceParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document_position.text_document.uri);
        let pos = params.text_document_position.position;
        if let Some(tok) = util::get_token(uri.clone(), pos)? {
            // Self::send_log(format!("token: {tok}"))?;
            if let Some(visitor) = self.get_visitor(&uri) {
                if let Some(vi) = visitor.visit_hir_info(&tok) {
                    let mut refs = vec![];
                    if let Some(referrers) = self.get_index().get_refs(&vi.def_loc) {
                        // Self::send_log(format!("referrers: {referrers:?}"))?;
                        for referrer in referrers.iter() {
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

    fn get_index(&self) -> &SharedModuleIndex {
        self.modules
            .values()
            .next()
            .unwrap()
            .context
            .index()
            .unwrap()
    }

    fn get_shared(&self) -> Option<&SharedCompilerResource> {
        self.modules
            .values()
            .next()
            .and_then(|module| module.context.shared())
    }

    fn clear_cache(&mut self, uri: &Url) {
        self.hirs.remove(uri);
        if let Some(module) = self.modules.remove(uri) {
            if let Some(shared) = module.context.shared() {
                shared.mod_cache.remove(&util::uri_to_path(uri));
            }
        }
    }
}
