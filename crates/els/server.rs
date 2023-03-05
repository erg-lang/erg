use std::cell::RefCell;
use std::io;
use std::io::{stdin, stdout, BufRead, Read, StdinLock, StdoutLock, Write};
use std::ops::Not;
use std::path::PathBuf;
use std::str::FromStr;

use erg_common::env::erg_path;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::normalize_path;

use erg_compiler::artifact::{BuildRunnable, IncompleteArtifact};
use erg_compiler::build_hir::HIRBuilder;
use erg_compiler::context::{Context, ModuleContext};
use erg_compiler::hir::Expr;
use erg_compiler::module::{SharedCompilerResource, SharedModuleIndex};
use erg_compiler::ty::HasType;

use lsp_types::{
    ClientCapabilities, CodeActionKind, CodeActionOptions, CodeActionProviderCapability,
    CodeLensOptions, CompletionOptions, DidChangeTextDocumentParams, ExecuteCommandOptions,
    HoverProviderCapability, InitializeResult, OneOf, Position, SemanticTokenType,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, SignatureHelpOptions, Url,
    WorkDoneProgressOptions,
};

use crate::file_cache::FileCache;
use crate::hir_visitor::HIRVisitor;
use crate::message::{ErrorMessage, LogMessage, ShowMessage};
use crate::util;

pub type ELSResult<T> = Result<T, Box<dyn std::error::Error>>;

pub type ErgLanguageServer = Server<HIRBuilder>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultFeatures {
    /* LSP features */
    CodeAction,
    CodeLens,
    Completion,
    Diagnostic,
    FindReferences,
    GotoDefinition,
    Hover,
    InlayHint,
    Rename,
    SemanticTokens,
    SignatureHelp,
    /* ELS specific features */
    SmartCompletion,
    DeepCompletion,
}

impl From<&str> for DefaultFeatures {
    fn from(s: &str) -> Self {
        match s {
            "codeaction" | "codeAction" | "code-action" => DefaultFeatures::CodeAction,
            "codelens" | "codeLens" | "code-lens" => DefaultFeatures::CodeLens,
            "completion" => DefaultFeatures::Completion,
            "diagnostic" => DefaultFeatures::Diagnostic,
            "hover" => DefaultFeatures::Hover,
            "semantictoken" | "semantictokens" | "semanticToken" | "semanticTokens"
            | "semantic-tokens" => DefaultFeatures::SemanticTokens,
            "rename" => DefaultFeatures::Rename,
            "inlayhint" | "inlayhints" | "inlayHint" | "inlayHints" | "inlay-hint"
            | "inlay-hints" => DefaultFeatures::InlayHint,
            "findreferences" | "findReferences" | "find-references" => {
                DefaultFeatures::FindReferences
            }
            "gotodefinition" | "gotoDefinition" | "goto-completion" => {
                DefaultFeatures::GotoDefinition
            }
            "signaturehelp" | "signatureHelp" | "signature-help" => DefaultFeatures::SignatureHelp,
            "smartcompletion" | "smartCompletion" | "smart-completion" => {
                DefaultFeatures::SmartCompletion
            }
            "deepcompletion" | "deepCompletion" | "deep-completion" => {
                DefaultFeatures::DeepCompletion
            }
            _ => panic!("unknown feature: {s}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionalFeatures {
    CheckOnType,
}

impl From<&str> for OptionalFeatures {
    fn from(s: &str) -> Self {
        match s {
            "checkontype" | "checkOnType" | "check-on-type" => OptionalFeatures::CheckOnType,
            _ => panic!("unknown feature: {s}"),
        }
    }
}

macro_rules! _log {
    ($($arg:tt)*) => {
        send_log(format!($($arg)*)).unwrap();
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

pub(crate) fn send<T: ?Sized + Serialize>(message: &T) -> ELSResult<()> {
    send_stdout(message)
}

pub(crate) fn send_log<S: Into<String>>(msg: S) -> ELSResult<()> {
    send(&LogMessage::new(msg))
}

#[allow(unused)]
pub(crate) fn send_info<S: Into<String>>(msg: S) -> ELSResult<()> {
    send(&ShowMessage::info(msg))
}

pub(crate) fn send_error_info<S: Into<String>>(msg: S) -> ELSResult<()> {
    send(&ShowMessage::error(msg))
}

pub(crate) fn send_error<S: Into<String>>(id: Option<i64>, code: i64, msg: S) -> ELSResult<()> {
    send(&ErrorMessage::new(
        id,
        json!({ "code": code, "message": msg.into() }),
    ))
}

pub(crate) fn send_invalid_req_error() -> ELSResult<()> {
    send_error(None, -32601, "received an invalid request")
}

/// A Language Server, which can be used any object implementing `BuildRunnable` internally by passing it as a generic parameter.
#[derive(Debug)]
pub struct Server<Checker: BuildRunnable = HIRBuilder> {
    pub(crate) cfg: ErgConfig,
    pub(crate) home: PathBuf,
    pub(crate) erg_path: PathBuf,
    pub(crate) client_capas: ClientCapabilities,
    pub(crate) opt_features: Vec<OptionalFeatures>,
    pub(crate) file_cache: FileCache,
    pub(crate) modules: Dict<Url, ModuleContext>,
    pub(crate) artifacts: Dict<Url, IncompleteArtifact>,
    pub(crate) current_sig: Option<Expr>,
    pub(crate) _checker: std::marker::PhantomData<Checker>,
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            home: normalize_path(std::env::current_dir().unwrap()),
            erg_path: erg_path(), // already normalized
            client_capas: ClientCapabilities::default(),
            opt_features: vec![],
            file_cache: FileCache::new(),
            modules: Dict::new(),
            artifacts: Dict::new(),
            current_sig: None,
            _checker: std::marker::PhantomData,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let msg = self.read_message()?;
            self.dispatch(msg)?;
        }
        // Ok(())
    }

    pub const fn mode(&self) -> &str {
        if cfg!(feature = "py_compatible") {
            "pylyzer"
        } else {
            "erg"
        }
    }

    #[allow(clippy::field_reassign_with_default)]
    fn init(&mut self, msg: &Value, id: i64) -> ELSResult<()> {
        send_log("initializing ELS")?;
        // #[allow(clippy::collapsible_if)]
        if msg.get("params").is_some() && msg["params"].get("capabilities").is_some() {
            self.client_capas = ClientCapabilities::deserialize(&msg["params"]["capabilities"])?;
            // send_log(format!("set client capabilities: {:?}", self.client_capas))?;
        }
        let mut args = self.cfg.runtime_args.iter();
        let mut disabled_features = vec![];
        while let Some(&arg) = args.next() {
            if arg == "--disable" {
                if let Some(&feature) = args.next() {
                    disabled_features.push(DefaultFeatures::from(feature));
                }
            } else if arg == "--enable" {
                if let Some(&feature) = args.next() {
                    self.opt_features.push(OptionalFeatures::from(feature));
                }
            }
        }
        let mut result = InitializeResult::default();
        result.capabilities = ServerCapabilities::default();
        self.file_cache.set_capabilities(&mut result.capabilities);
        let mut comp_options = CompletionOptions::default();
        comp_options.trigger_characters =
            Some(vec![".".into(), ":".into(), "(".into(), " ".into()]);
        comp_options.resolve_provider = Some(true);
        result.capabilities.completion_provider = Some(comp_options);
        result.capabilities.rename_provider = Some(OneOf::Left(true));
        result.capabilities.references_provider = Some(OneOf::Left(true));
        result.capabilities.definition_provider = Some(OneOf::Left(true));
        result.capabilities.hover_provider = disabled_features
            .contains(&DefaultFeatures::Hover)
            .not()
            .then_some(HoverProviderCapability::Simple(true));
        result.capabilities.inlay_hint_provider = disabled_features
            .contains(&DefaultFeatures::InlayHint)
            .not()
            .then_some(OneOf::Left(true));
        let mut sema_options = SemanticTokensOptions::default();
        sema_options.range = Some(false);
        sema_options.full = Some(SemanticTokensFullOptions::Bool(true));
        sema_options.legend = SemanticTokensLegend {
            token_types: vec![
                SemanticTokenType::NAMESPACE,
                SemanticTokenType::TYPE,
                SemanticTokenType::CLASS,
                SemanticTokenType::INTERFACE,
                SemanticTokenType::TYPE_PARAMETER,
                SemanticTokenType::PARAMETER,
                SemanticTokenType::VARIABLE,
                SemanticTokenType::PROPERTY,
                SemanticTokenType::FUNCTION,
                SemanticTokenType::METHOD,
                SemanticTokenType::STRING,
                SemanticTokenType::NUMBER,
                SemanticTokenType::OPERATOR,
            ],
            token_modifiers: vec![],
        };
        result.capabilities.semantic_tokens_provider = disabled_features
            .contains(&DefaultFeatures::SemanticTokens)
            .not()
            .then_some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                sema_options,
            ));
        result.capabilities.code_action_provider = if disabled_features
            .contains(&DefaultFeatures::CodeAction)
        {
            None
        } else {
            let options = CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::QUICKFIX, CodeActionKind::REFACTOR]),
                resolve_provider: Some(false),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            });
            Some(options)
        };
        result.capabilities.execute_command_provider = Some(ExecuteCommandOptions {
            commands: vec![format!("{}.eliminate_unused_vars", self.mode())],
            work_done_progress_options: WorkDoneProgressOptions::default(),
        });
        result.capabilities.signature_help_provider = disabled_features
            .contains(&DefaultFeatures::SignatureHelp)
            .not()
            .then_some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string(), "|".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            });
        result.capabilities.code_lens_provider = Some(CodeLensOptions {
            resolve_provider: Some(false),
        });
        send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }

    fn exit(&self) -> ELSResult<()> {
        send_log("exiting ELS")?;
        std::process::exit(0);
    }

    fn shutdown(&self, id: i64) -> ELSResult<()> {
        send_log("shutting down ELS")?;
        send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": json!(null),
        }))
    }

    /// Copied and modified from RLS, https://github.com/rust-lang/rls/blob/master/rls/src/server/io.rs
    fn read_message(&self) -> Result<Value, io::Error> {
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
                    format!("Header '{buffer}' is malformed"),
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
                            format!("Content type '{header_value}' is invalid"),
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

        let s = String::from_utf8(content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Value::from_str(&s)?)
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
            _ => send_invalid_req_error(),
        }
    }

    fn handle_request(&mut self, msg: &Value, id: i64, method: &str) -> ELSResult<()> {
        match method {
            "initialize" => self.init(msg, id),
            "shutdown" => self.shutdown(id),
            "textDocument/completion" => self.show_completion(msg),
            "completionItem/resolve" => self.resolve_completion(msg),
            "textDocument/definition" => self.show_definition(msg),
            "textDocument/hover" => self.show_hover(msg),
            "textDocument/rename" => self.rename(msg),
            "textDocument/references" => self.show_references(msg),
            "textDocument/semanticTokens/full" => self.get_semantic_tokens_full(msg),
            "textDocument/inlayHint" => self.get_inlay_hint(msg),
            "textDocument/codeAction" => self.send_code_action(msg),
            "textDocument/signatureHelp" => self.show_signature_help(msg),
            "textDocument/codeLens" => self.show_code_lens(msg),
            "workspace/willRenameFiles" => self.rename_files(msg),
            "workspace/executeCommand" => self.execute_command(msg),
            other => send_error(Some(id), -32600, format!("{other} is not supported")),
        }
    }

    fn handle_notification(&mut self, msg: &Value, method: &str) -> ELSResult<()> {
        match method {
            "initialized" => send_log("successfully bound"),
            "exit" => self.exit(),
            "textDocument/didOpen" => {
                let uri = util::parse_and_normalize_url(
                    msg["params"]["textDocument"]["uri"].as_str().unwrap(),
                )?;
                send_log(format!("{method}: {uri}"))?;
                let code = msg["params"]["textDocument"]["text"].as_str().unwrap();
                self.file_cache.update(&uri, code.to_string());
                self.check_file(uri, code)
            }
            "textDocument/didSave" => {
                let uri = util::parse_and_normalize_url(
                    msg["params"]["textDocument"]["uri"].as_str().unwrap(),
                )?;
                send_log(format!("{method}: {uri}"))?;
                let code = util::get_code_from_uri(&uri)?;
                self.clear_cache(&uri);
                self.check_file(uri, &code)
            }
            "textDocument/didChange" => {
                let params = DidChangeTextDocumentParams::deserialize(msg["params"].clone())?;
                self.file_cache.incremental_update(params.clone());
                send_log("file cache updated")?;
                if self.opt_features.contains(&OptionalFeatures::CheckOnType) {
                    let uri = util::normalize_url(params.text_document.uri);
                    self.quick_check_file(uri)?;
                }
                Ok(())
            }
            _ => send_log(format!("received notification: {method}")),
        }
    }

    pub(crate) fn get_checker(&self, path: PathBuf) -> Checker {
        if let Some(shared) = self.get_shared() {
            Checker::inherit(self.cfg.inherit(path), shared.clone())
        } else {
            Checker::new(self.cfg.inherit(path))
        }
    }

    pub(crate) fn get_visitor(&self, uri: &Url) -> Option<HIRVisitor> {
        self.artifacts
            .get(uri)?
            .object
            .as_ref()
            .map(|hir| HIRVisitor::new(hir, uri.clone(), !cfg!(feature = "py_compatible")))
    }

    pub(crate) fn get_local_ctx(&self, uri: &Url, pos: Position) -> Vec<&Context> {
        // send_log(format!("scope: {:?}\n", self.module.as_ref().unwrap().scope.keys())).unwrap();
        let mut ctxs = vec![];
        if let Some(visitor) = self.get_visitor(uri) {
            let ns = visitor.get_namespace(pos);
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

    pub(crate) fn get_receiver_ctxs(
        &self,
        uri: &Url,
        attr_marker_pos: Position,
    ) -> ELSResult<Vec<&Context>> {
        let Some(module) = self.modules.get(uri) else {
            return Ok(vec![]);
        };
        let maybe_token = self
            .file_cache
            .get_token_relatively(uri, attr_marker_pos, -2);
        if let Some(token) = maybe_token {
            // send_log(format!("token: {token}"))?;
            let mut ctxs = vec![];
            if let Some(visitor) = self.get_visitor(uri) {
                if let Some(expr) = visitor.get_min_expr(&token) {
                    let type_ctxs = module
                        .context
                        .get_nominal_super_type_ctxs(expr.ref_t())
                        .unwrap_or(vec![]);
                    ctxs.extend(type_ctxs);
                    if let Ok(singular_ctx) = module
                        .context
                        .get_singular_ctx_by_hir_expr(expr, &module.context)
                    {
                        ctxs.push(singular_ctx);
                    }
                } else {
                    send_log("expr not found: {token}")?;
                }
            }
            Ok(ctxs)
        } else {
            send_log("token not found")?;
            Ok(vec![])
        }
    }

    pub(crate) fn get_index(&self) -> &SharedModuleIndex {
        self.modules.values().next().unwrap().context.index()
    }

    pub(crate) fn get_shared(&self) -> Option<&SharedCompilerResource> {
        self.modules
            .values()
            .next()
            .map(|module| module.context.shared())
    }

    pub(crate) fn clear_cache(&mut self, uri: &Url) {
        self.artifacts.remove(uri);
        if let Some(module) = self.modules.remove(uri) {
            let shared = module.context.shared();
            let path = util::uri_to_path(uri);
            shared.mod_cache.remove(&path);
            shared.index.remove_path(&path);
            shared.graph.initialize();
        }
    }
}
