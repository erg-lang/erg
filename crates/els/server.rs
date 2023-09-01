use std::any::type_name;
use std::io;
use std::io::{stdin, stdout, BufRead, Read, Write};
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc;

use erg_common::config::ErgConfig;
use erg_common::consts::PYTHON_MODE;
use erg_common::dict::Dict;
use erg_common::env::erg_path;
use erg_common::shared::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard, Shared,
};
use erg_common::spawn::spawn_new_thread;
use erg_common::{fn_name, normalize_path};

use erg_compiler::artifact::{BuildRunnable, IncompleteArtifact};
use erg_compiler::build_hir::HIRBuilder;
use erg_compiler::context::{Context, ModuleContext};
use erg_compiler::erg_parser::ast::Module;
use erg_compiler::erg_parser::parse::{Parsable, SimpleParser};
use erg_compiler::hir::HIR;
use erg_compiler::lower::ASTLowerer;
use erg_compiler::module::{SharedCompilerResource, SharedModuleGraph, SharedModuleIndex};
use erg_compiler::ty::HasType;

use lsp_types::request::{
    CodeActionRequest, CodeActionResolveRequest, CodeLensRequest, Completion, ExecuteCommand,
    GotoDefinition, HoverRequest, InlayHintRequest, InlayHintResolveRequest, References, Rename,
    Request, ResolveCompletionItem, SemanticTokensFullRequest, SignatureHelpRequest,
    WillRenameFiles, WorkspaceSymbol,
};
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionProviderCapability, CodeLensOptions,
    CompletionOptions, ConfigurationItem, ConfigurationParams, DidChangeTextDocumentParams,
    DidOpenTextDocumentParams, ExecuteCommandOptions, HoverProviderCapability, InitializeParams,
    InitializeResult, InlayHintOptions, InlayHintServerCapabilities, OneOf, Position,
    SemanticTokenType, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, SignatureHelpOptions,
    WorkDoneProgressOptions,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;

use crate::channels::{SendChannels, Sendable, WorkerMessage};
use crate::completion::CompletionCache;
use crate::file_cache::FileCache;
use crate::hir_visitor::{ExprKind, HIRVisitor};
use crate::message::{ErrorMessage, LSPResult, LogMessage, ShowMessage};
use crate::util::{self, loc_to_pos, NormalizedUrl};

pub const HEALTH_CHECKER_ID: i64 = 10000;
pub const ASK_AUTO_SAVE_ID: i64 = 10001;

pub type ELSResult<T> = Result<T, Box<dyn std::error::Error>>;

pub type ErgLanguageServer = Server<HIRBuilder>;

pub type Handler<Server, Params, Out> = fn(&mut Server, Params) -> ELSResult<Out>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultFeatures {
    /* LSP features */
    CodeAction,
    CodeLens,
    Completion,
    Diagnostics,
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
            "diagnostic" | "diagnostics" => DefaultFeatures::Diagnostics,
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

#[macro_export]
macro_rules! _log {
    ($($arg:tt)*) => {
        let s = format!($($arg)*);
        $crate::server::send_log(format!("{}@{}: {s}", file!(), line!())).unwrap();
    };
}

fn send_stdout<T: ?Sized + Serialize>(message: &T) -> ELSResult<()> {
    let msg = serde_json::to_string(message)?;
    let mut stdout = stdout().lock();
    write!(stdout, "Content-Length: {}\r\n\r\n{}", msg.len(), msg)?;
    stdout.flush()?;
    Ok(())
}

fn read_line() -> io::Result<String> {
    let mut line = String::new();
    stdin().lock().read_line(&mut line)?;
    Ok(line)
}

fn read_exact(len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    stdin().lock().read_exact(&mut buf)?;
    Ok(buf)
}

pub(crate) fn send<T: ?Sized + Serialize>(message: &T) -> ELSResult<()> {
    send_stdout(message)
}

pub(crate) fn send_log<S: Into<String>>(msg: S) -> ELSResult<()> {
    if cfg!(debug_assertions) || cfg!(feature = "debug") {
        send(&LogMessage::new(msg))
    } else {
        Ok(())
    }
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

#[derive(Debug)]
pub struct AnalysisResult {
    pub ast: Module,
    pub artifact: IncompleteArtifact,
}

impl AnalysisResult {
    pub fn new(ast: Module, artifact: IncompleteArtifact) -> Self {
        Self { ast, artifact }
    }
}

pub const TRIGGER_CHARS: [&str; 4] = [".", ":", "(", " "];

#[derive(Debug, Clone, Default)]
pub struct AnalysisResultCache(Shared<Dict<NormalizedUrl, AnalysisResult>>);

impl AnalysisResultCache {
    pub fn new() -> Self {
        Self(Shared::new(Dict::new()))
    }

    pub fn insert(&self, uri: NormalizedUrl, result: AnalysisResult) {
        self.0.borrow_mut().insert(uri, result);
    }

    pub fn get(&self, uri: &NormalizedUrl) -> Option<MappedRwLockReadGuard<AnalysisResult>> {
        if self.0.borrow().get(uri).is_none() {
            None
        } else {
            Some(RwLockReadGuard::map(self.0.borrow(), |dict| {
                dict.get(uri).unwrap()
            }))
        }
    }

    pub fn get_mut(&self, uri: &NormalizedUrl) -> Option<MappedRwLockWriteGuard<AnalysisResult>> {
        if self.0.borrow().get(uri).is_none() {
            None
        } else {
            Some(RwLockWriteGuard::map(self.0.borrow_mut(), |dict| {
                dict.get_mut(uri).unwrap()
            }))
        }
    }

    pub fn get_ast(&self, uri: &NormalizedUrl) -> Option<MappedRwLockReadGuard<Module>> {
        self.get(uri)
            .map(|r| MappedRwLockReadGuard::map(r, |r| &r.ast))
    }

    pub fn get_mut_hir(&self, uri: &NormalizedUrl) -> Option<MappedRwLockWriteGuard<HIR>> {
        self.get_mut(uri).and_then(|r| {
            if r.artifact.object.is_none() {
                None
            } else {
                Some(MappedRwLockWriteGuard::map(r, |r| {
                    r.artifact.object.as_mut().unwrap()
                }))
            }
        })
    }

    pub fn get_artifact(
        &self,
        uri: &NormalizedUrl,
    ) -> Option<MappedRwLockReadGuard<IncompleteArtifact>> {
        self.get(uri)
            .map(|r| MappedRwLockReadGuard::map(r, |r| &r.artifact))
    }

    pub fn get_hir(&self, uri: &NormalizedUrl) -> Option<MappedRwLockReadGuard<HIR>> {
        self.get(uri).and_then(|r| {
            if r.artifact.object.is_none() {
                None
            } else {
                Some(MappedRwLockReadGuard::map(r, |r| {
                    r.artifact.object.as_ref().unwrap()
                }))
            }
        })
    }

    pub fn remove(&self, uri: &NormalizedUrl) -> Option<AnalysisResult> {
        self.0.borrow_mut().remove(uri)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModuleCache(Shared<Dict<NormalizedUrl, ModuleContext>>);

impl ModuleCache {
    pub fn new() -> Self {
        Self(Shared::new(Dict::new()))
    }

    pub fn get(&self, uri: &NormalizedUrl) -> Option<&ModuleContext> {
        let _ref = self.0.borrow();
        let ref_ = unsafe { self.0.as_ptr().as_ref() };
        ref_.unwrap().get(uri)
    }

    pub fn get_mut(&self, uri: &NormalizedUrl) -> Option<&mut ModuleContext> {
        let _ref = self.0.borrow_mut();
        let ref_ = unsafe { self.0.as_ptr().as_mut() };
        ref_.unwrap().get_mut(uri)
    }

    pub fn insert(&self, uri: NormalizedUrl, module: ModuleContext) {
        self.0.borrow_mut().insert(uri, module);
    }

    pub fn remove(&self, uri: &NormalizedUrl) -> Option<ModuleContext> {
        self.0.borrow_mut().remove(uri)
    }

    pub fn values(&self) -> std::collections::hash_map::Values<NormalizedUrl, ModuleContext> {
        let _ref = self.0.borrow();
        let ref_ = unsafe { self.0.as_ptr().as_ref() };
        ref_.unwrap().values()
    }
}

/// A Language Server, which can be used any object implementing `BuildRunnable` internally by passing it as a generic parameter.
#[derive(Debug)]
pub struct Server<Checker: BuildRunnable = HIRBuilder, Parser: Parsable = SimpleParser> {
    pub(crate) cfg: ErgConfig,
    pub(crate) home: PathBuf,
    pub(crate) erg_path: PathBuf,
    pub(crate) init_params: InitializeParams,
    pub(crate) client_answers: Shared<Dict<i64, Value>>,
    pub(crate) disabled_features: Vec<DefaultFeatures>,
    pub(crate) opt_features: Vec<OptionalFeatures>,
    pub(crate) file_cache: FileCache,
    pub(crate) comp_cache: CompletionCache,
    // TODO: remove modules, analysis_result, and add `shared: SharedCompilerResource`
    pub(crate) modules: ModuleCache,
    pub(crate) analysis_result: AnalysisResultCache,
    pub(crate) channels: Option<SendChannels>,
    pub(crate) _parser: std::marker::PhantomData<fn() -> Parser>,
    pub(crate) _checker: std::marker::PhantomData<fn() -> Checker>,
}

impl<C: BuildRunnable, P: Parsable> Clone for Server<C, P> {
    fn clone(&self) -> Self {
        Self {
            cfg: self.cfg.clone(),
            home: self.home.clone(),
            erg_path: self.erg_path.clone(),
            init_params: self.init_params.clone(),
            client_answers: self.client_answers.clone(),
            disabled_features: self.disabled_features.clone(),
            opt_features: self.opt_features.clone(),
            file_cache: self.file_cache.clone(),
            comp_cache: self.comp_cache.clone(),
            modules: self.modules.clone(),
            analysis_result: self.analysis_result.clone(),
            channels: self.channels.clone(),
            _parser: std::marker::PhantomData,
            _checker: std::marker::PhantomData,
        }
    }
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            comp_cache: CompletionCache::new(cfg.copy()),
            cfg,
            home: normalize_path(std::env::current_dir().unwrap_or_default()),
            erg_path: erg_path().clone(), // already normalized
            init_params: InitializeParams::default(),
            client_answers: Shared::new(Dict::new()),
            disabled_features: vec![],
            opt_features: vec![],
            file_cache: FileCache::new(),
            modules: ModuleCache::new(),
            analysis_result: AnalysisResultCache::new(),
            channels: None,
            _parser: std::marker::PhantomData,
            _checker: std::marker::PhantomData,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let msg = self.read_message()?;
            if let Err(err) = self.dispatch(msg) {
                send_error_info(format!("err: {err:?}"))?;
            }
        }
        // Ok(())
    }

    pub const fn mode(&self) -> &str {
        if PYTHON_MODE {
            "pylyzer"
        } else {
            "erg"
        }
    }

    #[allow(clippy::field_reassign_with_default)]
    fn init(&mut self, msg: &Value, id: i64) -> ELSResult<()> {
        send_log("initializing ELS")?;
        if msg.get("params").is_some() && msg["params"].get("capabilities").is_some() {
            self.init_params = InitializeParams::deserialize(&msg["params"])?;
            // send_log(format!("set client capabilities: {:?}", self.client_capas))?;
        }
        let mut args = self.cfg.runtime_args.iter();
        while let Some(&arg) = args.next() {
            if arg == "--disable" {
                if let Some(&feature) = args.next() {
                    self.disabled_features.push(DefaultFeatures::from(feature));
                }
            } else if arg == "--enable" {
                if let Some(&feature) = args.next() {
                    self.opt_features.push(OptionalFeatures::from(feature));
                }
            }
        }
        let mut result = InitializeResult::default();
        result.capabilities = self.init_capabilities();
        self.init_services();
        send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }

    #[allow(clippy::field_reassign_with_default)]
    fn init_capabilities(&mut self) -> ServerCapabilities {
        let mut capabilities = ServerCapabilities::default();
        self.file_cache.set_capabilities(&mut capabilities);
        let mut comp_options = CompletionOptions::default();
        comp_options.trigger_characters = Some(TRIGGER_CHARS.map(String::from).to_vec());
        comp_options.resolve_provider = Some(true);
        capabilities.completion_provider = Some(comp_options);
        capabilities.rename_provider = Some(OneOf::Left(true));
        capabilities.references_provider = Some(OneOf::Left(true));
        capabilities.definition_provider = Some(OneOf::Left(true));
        capabilities.hover_provider = self
            .disabled_features
            .contains(&DefaultFeatures::Hover)
            .not()
            .then_some(HoverProviderCapability::Simple(true));
        capabilities.inlay_hint_provider = self
            .disabled_features
            .contains(&DefaultFeatures::InlayHint)
            .not()
            .then_some(OneOf::Right(InlayHintServerCapabilities::Options(
                InlayHintOptions {
                    resolve_provider: Some(true),
                    ..Default::default()
                },
            )));
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
        capabilities.semantic_tokens_provider = self
            .disabled_features
            .contains(&DefaultFeatures::SemanticTokens)
            .not()
            .then_some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                sema_options,
            ));
        capabilities.code_action_provider = if self
            .disabled_features
            .contains(&DefaultFeatures::CodeAction)
        {
            None
        } else {
            let options = CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::QUICKFIX, CodeActionKind::REFACTOR]),
                resolve_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            });
            Some(options)
        };
        capabilities.execute_command_provider = Some(ExecuteCommandOptions {
            commands: vec![format!("{}.eliminate_unused_vars", self.mode())],
            work_done_progress_options: WorkDoneProgressOptions::default(),
        });
        capabilities.signature_help_provider = self
            .disabled_features
            .contains(&DefaultFeatures::SignatureHelp)
            .not()
            .then_some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string(), "|".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            });
        capabilities.code_lens_provider = Some(CodeLensOptions {
            resolve_provider: Some(false),
        });
        capabilities.workspace_symbol_provider = Some(OneOf::Left(true));
        capabilities
    }

    pub(crate) fn ask_auto_save(&self) -> ELSResult<()> {
        let params = ConfigurationParams {
            items: vec![ConfigurationItem {
                scope_uri: None,
                section: Some("files.autoSave".to_string()),
            }],
        };
        send(&json!({
            "jsonrpc": "2.0",
            "id": ASK_AUTO_SAVE_ID,
            "method": "workspace/configuration",
            "params": params,
        }))
    }

    fn start_language_services(&mut self) {
        let (senders, receivers) = SendChannels::new();
        self.channels = Some(senders);
        self.start_service::<Completion>(receivers.completion, Self::handle_completion);
        self.start_service::<ResolveCompletionItem>(
            receivers.resolve_completion,
            Self::handle_resolve_completion,
        );
        self.start_service::<GotoDefinition>(
            receivers.goto_definition,
            Self::handle_goto_definition,
        );
        self.start_service::<SemanticTokensFullRequest>(
            receivers.semantic_tokens_full,
            Self::handle_semantic_tokens_full,
        );
        self.start_service::<InlayHintRequest>(receivers.inlay_hint, Self::handle_inlay_hint);
        self.start_service::<InlayHintResolveRequest>(
            receivers.inlay_hint_resolve,
            Self::handle_inlay_hint_resolve,
        );
        self.start_service::<HoverRequest>(receivers.hover, Self::handle_hover);
        self.start_service::<References>(receivers.references, Self::handle_references);
        self.start_service::<CodeLensRequest>(receivers.code_lens, Self::handle_code_lens);
        self.start_service::<CodeActionRequest>(receivers.code_action, Self::handle_code_action);
        self.start_service::<CodeActionResolveRequest>(
            receivers.code_action_resolve,
            Self::handle_code_action_resolve,
        );
        self.start_service::<SignatureHelpRequest>(
            receivers.signature_help,
            Self::handle_signature_help,
        );
        self.start_service::<WillRenameFiles>(
            receivers.will_rename_files,
            Self::handle_will_rename_files,
        );
        self.start_service::<ExecuteCommand>(
            receivers.execute_command,
            Self::handle_execute_command,
        );
        self.start_service::<WorkspaceSymbol>(
            receivers.workspace_symbol,
            Self::handle_workspace_symbol,
        );
        self.start_client_health_checker(receivers.health_check);
    }

    fn init_services(&mut self) {
        self.start_language_services();
        self.start_auto_diagnostics();
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

    #[allow(unused)]
    pub(crate) fn restart(&mut self) {
        self.file_cache.clear();
        self.comp_cache.clear();
        self.modules = ModuleCache::new();
        self.analysis_result = AnalysisResultCache::new();
        self.channels.as_ref().unwrap().close();
        self.start_language_services();
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

    pub fn dispatch(&mut self, msg: Value) -> ELSResult<()> {
        match (
            msg.get("id").and_then(|i| i.as_i64()),
            msg.get("method").and_then(|m| m.as_str()),
        ) {
            (Some(id), Some(method)) => self.handle_request(&msg, id, method),
            (Some(id), None) => self.handle_response(id, &msg),
            (None, Some(notification)) => self.handle_notification(&msg, notification),
            _ => send_invalid_req_error(),
        }
    }

    fn parse_send<R>(&self, id: i64, msg: &Value) -> ELSResult<()>
    where
        R: lsp_types::request::Request + 'static,
        R::Result: Serialize,
        Server<Checker, Parser>: Sendable<R>,
    {
        let params = R::Params::deserialize(&msg["params"])?;
        self.send(id, params);
        Ok(())
    }

    fn start_service<R>(
        &self,
        receiver: mpsc::Receiver<WorkerMessage<R::Params>>,
        handler: Handler<Server<Checker, Parser>, R::Params, R::Result>,
    ) where
        R: lsp_types::request::Request + 'static,
        R::Params: Send,
        R::Result: Serialize,
    {
        let mut _self = self.clone();
        spawn_new_thread(
            move || loop {
                let msg = receiver.recv().unwrap();
                match msg {
                    WorkerMessage::Request(id, params) => match handler(&mut _self, params) {
                        Ok(result) => {
                            let _ = send(&LSPResult::new(id, result));
                        }
                        Err(err) => {
                            let _ = send(&ErrorMessage::new(
                                Some(id),
                                format!("err from {}: {err}", type_name::<R>()).into(),
                            ));
                        }
                    },
                    WorkerMessage::Kill => {
                        break;
                    }
                }
            },
            fn_name!(),
        );
    }

    fn handle_request(&mut self, msg: &Value, id: i64, method: &str) -> ELSResult<()> {
        match method {
            "initialize" => self.init(msg, id),
            "shutdown" => self.shutdown(id),
            Rename::METHOD => self.rename(msg),
            Completion::METHOD => self.parse_send::<Completion>(id, msg),
            ResolveCompletionItem::METHOD => self.parse_send::<ResolveCompletionItem>(id, msg),
            GotoDefinition::METHOD => self.parse_send::<GotoDefinition>(id, msg),
            HoverRequest::METHOD => self.parse_send::<HoverRequest>(id, msg),
            References::METHOD => self.parse_send::<References>(id, msg),
            SemanticTokensFullRequest::METHOD => {
                self.parse_send::<SemanticTokensFullRequest>(id, msg)
            }
            InlayHintRequest::METHOD => self.parse_send::<InlayHintRequest>(id, msg),
            InlayHintResolveRequest::METHOD => self.parse_send::<InlayHintResolveRequest>(id, msg),
            CodeActionRequest::METHOD => self.parse_send::<CodeActionRequest>(id, msg),
            CodeActionResolveRequest::METHOD => {
                self.parse_send::<CodeActionResolveRequest>(id, msg)
            }
            SignatureHelpRequest::METHOD => self.parse_send::<SignatureHelpRequest>(id, msg),
            CodeLensRequest::METHOD => self.parse_send::<CodeLensRequest>(id, msg),
            WillRenameFiles::METHOD => self.parse_send::<WillRenameFiles>(id, msg),
            ExecuteCommand::METHOD => self.parse_send::<ExecuteCommand>(id, msg),
            WorkspaceSymbol::METHOD => self.parse_send::<WorkspaceSymbol>(id, msg),
            other => send_error(Some(id), -32600, format!("{other} is not supported")),
        }
    }

    fn handle_notification(&mut self, msg: &Value, method: &str) -> ELSResult<()> {
        match method {
            "initialized" => {
                self.ask_auto_save()?;
                send_log("successfully bound")
            }
            "exit" => self.exit(),
            "textDocument/didOpen" => {
                let params = DidOpenTextDocumentParams::deserialize(msg["params"].clone())?;
                let uri = NormalizedUrl::new(params.text_document.uri);
                send_log(format!("{method}: {uri}"))?;
                let code = params.text_document.text;
                let ver = params.text_document.version;
                self.file_cache.update(&uri, code.clone(), Some(ver));
                self.check_file(uri, code)
            }
            "textDocument/didSave" => {
                let uri =
                    NormalizedUrl::parse(msg["params"]["textDocument"]["uri"].as_str().unwrap())?;
                send_log(format!("{method}: {uri}"))?;
                let code = self.file_cache.get_entire_code(&uri)?;
                self.clear_cache(&uri);
                self.check_file(uri, code)
            }
            "textDocument/didChange" => {
                let params = DidChangeTextDocumentParams::deserialize(msg["params"].clone())?;
                // Check before updating, because `x.`/`x::` will result in an error
                // Checking should only be performed when needed for completion, i.e., when a trigger character is entered or at the beginning of a line
                if TRIGGER_CHARS.contains(&&params.content_changes[0].text[..])
                    || params.content_changes[0]
                        .range
                        .is_some_and(|r| r.start.character == 0)
                {
                    let uri = NormalizedUrl::new(params.text_document.uri.clone());
                    // TODO: reset mutable dependent types
                    self.quick_check_file(uri)?;
                }
                self.file_cache.incremental_update(params);
                Ok(())
            }
            _ => send_log(format!("received notification: {method}")),
        }
    }

    fn handle_response(&mut self, id: i64, msg: &Value) -> ELSResult<()> {
        match id {
            HEALTH_CHECKER_ID => {
                self.channels
                    .as_ref()
                    .unwrap()
                    .health_check
                    .send(WorkerMessage::Request(0, ()))?;
            }
            _ => {
                _log!("msg: {msg}");
                if msg.get("error").is_none() {
                    self.client_answers.borrow_mut().insert(id, msg.clone());
                }
            }
        }
        Ok(())
    }

    /// TODO: Reuse cache.
    /// Because of the difficulty of caching "transitional types" such as assert casting and mutable dependent types,
    /// the cache is deleted after each analysis.
    pub(crate) fn get_checker(&self, path: PathBuf) -> Checker {
        if let Some(shared) = self.get_shared() {
            let shared = shared.clone();
            shared.clear(&path);
            Checker::inherit(self.cfg.inherit(path), shared)
        } else {
            Checker::new(self.cfg.inherit(path))
        }
    }

    pub(crate) fn steal_lowerer(&mut self, uri: &NormalizedUrl) -> Option<ASTLowerer> {
        let module = self.modules.remove(uri)?;
        Some(ASTLowerer::new_with_ctx(module))
    }

    pub(crate) fn restore_lowerer(&mut self, uri: NormalizedUrl, mut lowerer: ASTLowerer) {
        let module = lowerer.pop_mod_ctx().unwrap();
        if let Some(m) = self.modules.get_mut(&uri) {
            *m = module;
        } else {
            self.modules.insert(uri, module);
        }
    }

    pub(crate) fn get_visitor(&self, uri: &NormalizedUrl) -> Option<HIRVisitor> {
        self.analysis_result
            .get_hir(uri)
            .map(|hir| HIRVisitor::new(hir, &self.file_cache, uri.clone()))
    }

    pub(crate) fn get_searcher(&self, uri: &NormalizedUrl, kind: ExprKind) -> Option<HIRVisitor> {
        self.analysis_result
            .get_hir(uri)
            .map(|hir| HIRVisitor::new_searcher(hir, &self.file_cache, uri.clone(), kind))
    }

    pub(crate) fn get_local_ctx(&self, uri: &NormalizedUrl, pos: Position) -> Vec<&Context> {
        let mut ctxs = vec![];
        if let Some(mod_ctx) = &self.modules.get(uri) {
            if let Some(visitor) = self.get_visitor(uri) {
                // FIXME:
                let mut ns = visitor.get_namespace(pos);
                if &mod_ctx.context.name[..] == "<module>" {
                    ns[0] = "<module>".into();
                }
                for i in 1..ns.len() {
                    let ns = ns[..=ns.len() - i].join("");
                    if let Some(ctx) = mod_ctx.scope.get(&ns[..]) {
                        ctxs.push(ctx);
                    }
                }
            }
            ctxs.push(&mod_ctx.context);
        }
        let builtin_ctx = self.get_builtin_module();
        ctxs.extend(builtin_ctx);
        ctxs
    }

    pub(crate) fn get_neighbor_ctxs(&self, uri: &NormalizedUrl) -> Vec<&Context> {
        let mut ctxs = vec![];
        if let Ok(dir) = uri
            .to_file_path()
            .and_then(|p| p.parent().unwrap().read_dir().map_err(|_| ()))
        {
            for neighbor in dir {
                let Ok(neighbor) = neighbor else {
                    continue;
                };
                let uri = NormalizedUrl::from_file_path(neighbor.path()).unwrap();
                if let Some(mod_ctx) = &self.modules.get(&uri) {
                    ctxs.push(&mod_ctx.context);
                }
            }
        }
        ctxs
    }

    pub(crate) fn get_receiver_ctxs(
        &self,
        uri: &NormalizedUrl,
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
                if let Some(expr) =
                    loc_to_pos(token.loc()).and_then(|pos| visitor.get_min_expr(pos))
                {
                    let type_ctxs = module
                        .context
                        .get_nominal_super_type_ctxs(expr.ref_t())
                        .unwrap_or(vec![]);
                    ctxs.extend(type_ctxs);
                    if let Ok(singular_ctxs) = module
                        .context
                        .get_singular_ctxs_by_hir_expr(expr, &module.context)
                    {
                        ctxs.extend(singular_ctxs);
                    }
                } else {
                    _log!("expr not found: {token}");
                }
            }
            Ok(ctxs)
        } else {
            send_log("token not found")?;
            Ok(vec![])
        }
    }

    pub(crate) fn get_index(&self) -> Option<&SharedModuleIndex> {
        self.modules
            .values()
            .next()
            .map(|module| module.context.index())
    }

    pub(crate) fn get_graph(&self) -> Option<&SharedModuleGraph> {
        self.modules
            .values()
            .next()
            .map(|module| module.context.graph())
    }

    pub(crate) fn get_shared(&self) -> Option<&SharedCompilerResource> {
        self.modules
            .values()
            .next()
            .map(|module| module.context.shared())
    }

    pub(crate) fn get_builtin_module(&self) -> Option<&Context> {
        self.get_shared()
            .and_then(|mode| mode.mod_cache.raw_ref_ctx(Path::new("<builtins>")))
            .map(|mc| &mc.context)
    }

    pub(crate) fn clear_cache(&mut self, uri: &NormalizedUrl) {
        self.analysis_result.remove(uri);
        if let Some(module) = self.modules.remove(uri) {
            let shared = module.context.shared();
            let path = util::uri_to_path(uri);
            shared.clear(&path);
        }
    }
}
