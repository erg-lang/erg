use std::env::current_dir;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::Duration;

use lsp_types::{
    ConfigurationParams, Diagnostic, DiagnosticSeverity, NumberOrString, Position, ProgressParams,
    ProgressParamsValue, PublishDiagnosticsParams, Range, Url, WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressCreateParams, WorkDoneProgressEnd,
};
use serde_json::json;

use erg_common::consts::PYTHON_MODE;
use erg_common::dict::Dict;
use erg_common::pathutil::{project_entry_dir_of, project_entry_file_of, NormalizedPathBuf};
use erg_common::set::Set;
use erg_common::spawn::{safe_yield, spawn_new_thread};
use erg_common::style::*;
use erg_common::{fn_name, lsp_log};
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::build_package::CheckStatus;
use erg_compiler::erg_parser::ast::Module;
use erg_compiler::erg_parser::error::IncompleteArtifact;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::error::CompileErrors;

use crate::_log;
use crate::channels::WorkerMessage;
use crate::diff::{ASTDiff, HIRDiff};
use crate::server::{DefaultFeatures, ELSResult, RedirectableStdout, Server};
use crate::server::{ASK_AUTO_SAVE_ID, HEALTH_CHECKER_ID};
use crate::util::{self, NormalizedUrl};

#[cfg(unix)]
pub fn is_process_alive(pid: i32) -> bool {
    unsafe {
        // sig 0: check if the process exists
        let alive = libc::kill(pid, 0);
        alive == 0
    }
}
#[cfg(windows)]
pub fn is_process_alive(pid: i32) -> bool {
    unsafe {
        use windows::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_INFORMATION,
        };

        const STILL_ACTIVE: u32 = 0x103u32;

        let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid as u32) else {
            return false;
        };
        let mut code = 0;
        let Ok(_) = GetExitCodeProcess(handle, &mut code) else {
            return false;
        };
        code == STILL_ACTIVE
    }
}
#[cfg(all(not(windows), not(unix)))]
pub fn is_process_alive(_pid: i32) -> bool {
    false
}

#[derive(Debug)]
pub enum BuildASTError {
    NoFile,
    ParseError(IncompleteArtifact),
}

#[derive(Debug)]
pub enum ChangeKind {
    New,
    NoChange,
    Valid,
    Invalid,
}

impl ChangeKind {
    pub const fn is_no_change(&self) -> bool {
        matches!(self, Self::NoChange)
    }
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn build_ast(&self, uri: &NormalizedUrl) -> Result<Module, BuildASTError> {
        let code = self
            .file_cache
            .get_entire_code(uri)
            .map_err(|_| BuildASTError::NoFile)?;
        Parser::parse(code)
            .map(|artifact| artifact.ast)
            .map_err(BuildASTError::ParseError)
    }

    pub(crate) fn change_kind(&self, uri: &NormalizedUrl) -> ChangeKind {
        let deps = self.dependencies_of(uri);
        if deps.is_empty() {
            return ChangeKind::New;
        }
        for dep in deps {
            let Some(old) = self.get_ast(&dep) else {
                return ChangeKind::Invalid;
            };
            if let Ok(new) = self.build_ast(&dep) {
                if !ASTDiff::diff(old, &new).is_nop() {
                    return ChangeKind::Valid;
                }
            } else {
                return ChangeKind::Invalid;
            }
        }
        _log!(self, "no changes: {uri}");
        ChangeKind::NoChange
    }

    pub(crate) fn recheck_file(
        &mut self,
        uri: NormalizedUrl,
        code: impl Into<String>,
    ) -> ELSResult<()> {
        if self.change_kind(&uri).is_no_change() {
            _log!(self, "no changes: {uri}");
            return Ok(());
        }
        // self.clear_cache(&uri);
        let mut checked = Set::new();
        self.check_file(uri, code, &mut checked)?;
        self.send_empty_diagnostics(checked)?;
        Ok(())
    }

    pub(crate) fn check_file(
        &mut self,
        uri: NormalizedUrl,
        code: impl Into<String>,
        checked: &mut Set<NormalizedUrl>,
    ) -> ELSResult<()> {
        _log!(self, "checking {uri}");
        if self.file_cache.editing.borrow().contains(&uri) || checked.contains(&uri) {
            _log!(self, "skipped: {uri}");
            return Ok(());
        }
        let path = util::uri_to_path(&uri);
        let mode = if path.to_string_lossy().ends_with(".d.er") {
            "declare"
        } else {
            "exec"
        };
        let mut checker = self.get_checker(path.clone());
        let (artifact, status) = match checker.build(code.into(), mode) {
            Ok(artifact) => {
                #[cfg(feature = "lint")]
                let mut artifact = artifact;
                #[cfg(feature = "lint")]
                if self
                    .opt_features
                    .contains(&crate::server::OptionalFeatures::Lint)
                {
                    use erg_common::traits::Stream;
                    let mut linter = erg_linter::Linter::new(self.cfg.inherit(path.clone()));
                    let warns = linter.lint(&artifact.object);
                    artifact.warns.extend(warns);
                }
                _log!(
                    self,
                    "checking {uri} passed, found warns: {}",
                    artifact.warns.len()
                );
                self.shared.errors.clear();
                if artifact.warns.is_empty() {
                    self.shared.warns.clear();
                }
                let uri_and_diags = self.make_uri_and_diags(artifact.warns.clone());
                // clear previous diagnostics
                self.send_diagnostics(uri.clone().raw(), vec![])?;
                for (uri, diags) in uri_and_diags.into_iter() {
                    _log!(self, "{uri}, warns: {}", diags.len());
                    self.send_diagnostics(uri, diags)?;
                }
                (artifact.into(), CheckStatus::Succeed)
            }
            Err(artifact) => {
                _log!(self, "found errors: {}", artifact.errors.len());
                _log!(self, "found warns: {}", artifact.warns.len());
                if artifact.warns.is_empty() {
                    self.shared.warns.clear();
                }
                let diags = artifact
                    .errors
                    .clone()
                    .into_iter()
                    .chain(artifact.warns.clone())
                    .collect();
                let uri_and_diags = self.make_uri_and_diags(diags);
                if uri_and_diags.is_empty() {
                    self.send_diagnostics(uri.clone().raw(), vec![])?;
                }
                for (uri, diags) in uri_and_diags.into_iter() {
                    _log!(self, "{uri}, errs & warns: {}", diags.len());
                    self.send_diagnostics(uri, diags)?;
                }
                (artifact, CheckStatus::Failed)
            }
        };
        checked.insert(uri.clone());
        if let Some(files) = artifact.object.as_ref().map(|art| &art.dependencies) {
            checked.extend(
                files
                    .iter()
                    .cloned()
                    .filter_map(|file| NormalizedUrl::from_file_path(file).ok()),
            );
        }
        let ast = match self.build_ast(&uri) {
            Ok(ast) => Some(ast),
            Err(BuildASTError::ParseError(err)) => err.ast,
            _ => None,
        };
        let Some(ctx) = checker.pop_context() else {
            _log!(self, "context not found");
            return Ok(());
        };
        if mode == "declare" {
            self.shared
                .py_mod_cache
                .register(path, ast, artifact.object, ctx, status);
        } else {
            self.shared
                .mod_cache
                .register(path, ast, artifact.object, ctx, status);
        }
        let dependents = self.dependents_of(&uri);
        for dep in dependents {
            // _log!(self, "dep: {dep}");
            let code = self.file_cache.get_entire_code(&dep)?.to_string();
            self.check_file(dep, code, checked)?;
        }
        self.shared.errors.extend(artifact.errors);
        self.shared.warns.extend(artifact.warns);
        Ok(())
    }

    pub(crate) fn send_empty_diagnostics(&self, checked: Set<NormalizedUrl>) -> ELSResult<()> {
        for checked in checked {
            let Ok(path) = checked.to_file_path() else {
                continue;
            };
            let path = NormalizedPathBuf::from(path);
            if self.shared.errors.get(&path).is_empty() && self.shared.warns.get(&path).is_empty() {
                self.send_diagnostics(checked.raw(), vec![])?;
            }
        }
        Ok(())
    }

    // TODO: reset mutable dependent types
    pub(crate) fn quick_check_file(&mut self, uri: NormalizedUrl) -> ELSResult<()> {
        if self.file_cache.editing.borrow().contains(&uri) {
            _log!(self, "skipped: {uri}");
            return Ok(());
        }
        let Some(old) = self.get_ast(&uri) else {
            crate::_log!(self, "AST not found: {uri}");
            return Ok(());
        };
        let new = match self.build_ast(&uri) {
            Ok(ast) => ast,
            Err(BuildASTError::ParseError(err)) => {
                if let Some(new) = err.ast {
                    new
                } else {
                    return Ok(());
                }
            }
            _ => {
                crate::_log!(self, "AST not found: {uri}");
                return Ok(());
            }
        };
        let ast_diff = ASTDiff::diff(old, &new);
        crate::_log!(self, "diff: {ast_diff}");
        if ast_diff.is_nop() {
            return Ok(());
        }
        if let Some((mut lowerer, mut irs)) = self.steal_lowerer(&uri) {
            if let Some((hir_diff, hir)) =
                HIRDiff::new(ast_diff.clone(), &mut lowerer).zip(irs.hir.as_mut())
            {
                crate::_log!(self, "hir_diff: {hir_diff}");
                hir_diff.update(hir);
                if let Some(ast) = irs.ast.as_mut() {
                    ast_diff.update(ast);
                }
            }
            if let Some(hir) = irs.hir.as_mut() {
                HIRDiff::fix(&new, &mut hir.module, &mut lowerer);
            }
            self.restore_lowerer(uri, lowerer, irs);
        }
        // skip checking for dependents
        Ok(())
    }

    fn make_uri_and_diags(&mut self, errors: CompileErrors) -> Vec<(Url, Vec<Diagnostic>)> {
        let mut uri_and_diags: Vec<(Url, Vec<Diagnostic>)> = vec![];
        for err in errors.into_iter() {
            let loc = err.core.get_loc_with_fallback();
            let res_uri = Url::from_file_path(
                err.input
                    .path()
                    .canonicalize()
                    .unwrap_or(err.input.path().to_path_buf()),
            );
            let Ok(err_uri) = res_uri else {
                crate::_log!(self, "failed to get uri: {}", err.input.path().display());
                continue;
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
            let source = if PYTHON_MODE { "pylyzer" } else { "els" };
            let diag = Diagnostic::new(
                Range::new(start, end),
                Some(severity),
                Some(NumberOrString::String(format!("E{}", err.core.errno))),
                Some(source.to_string()),
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

    fn send_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) -> ELSResult<()> {
        if self
            .disabled_features
            .contains(&DefaultFeatures::Diagnostics)
        {
            return Ok(());
        }
        let params = PublishDiagnosticsParams::new(uri, diagnostics, None);
        if self
            .init_params
            .capabilities
            .text_document
            .as_ref()
            .map(|doc| doc.publish_diagnostics.is_some())
            .unwrap_or(false)
        {
            self.send_stdout(&json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": params,
            }))?;
        } else {
            self.send_log("the client does not support diagnostics")?;
        }
        Ok(())
    }

    /// Periodically send diagnostics without a request from the server.
    /// This is necessary to perform reactive error highlighting in editors such as Vim, where no action is taken until the buffer is saved.
    pub(crate) fn start_auto_diagnostics(&mut self) {
        let mut _self = self.clone();
        spawn_new_thread(
            move || {
                while !_self.flags.client_initialized() {
                    safe_yield();
                }
                let mut file_vers = Dict::<NormalizedUrl, i32>::new();
                loop {
                    if _self
                        .client_answers
                        .borrow()
                        .get(&ASK_AUTO_SAVE_ID)
                        .is_some_and(|val| {
                            val["result"].as_array().and_then(|a| a[0].as_str())
                                == Some("afterDelay")
                        })
                    {
                        // _log!(_self, "Auto saving is enabled");
                        break;
                    }
                    for uri in _self.file_cache.entries() {
                        let Some(latest_ver) = _self.file_cache.get_ver(&uri) else {
                            continue;
                        };
                        let Some(&ver) = file_vers.get(&uri) else {
                            file_vers.insert(uri.clone(), latest_ver);
                            continue;
                        };
                        if latest_ver != ver {
                            if let Ok(code) = _self.file_cache.get_entire_code(&uri) {
                                let mut checked = Set::new();
                                let _ = _self.check_file(uri.clone(), code, &mut checked);
                                _self.send_empty_diagnostics(checked).unwrap();
                                file_vers.insert(uri, latest_ver);
                            }
                        }
                    }
                    sleep(Duration::from_millis(500));
                }
            },
            fn_name!(),
        );
    }

    fn project_files(dir: PathBuf) -> Vec<NormalizedUrl> {
        let mut uris = vec![];
        let Ok(read_dir) = dir.read_dir() else {
            return uris;
        };
        for entry in read_dir {
            let Ok(entry) = entry else {
                continue;
            };
            if entry.path().extension() == Some(OsStr::new("er"))
                || (PYTHON_MODE && entry.path().extension() == Some(OsStr::new("py")))
            {
                if let Ok(uri) = NormalizedUrl::from_file_path(entry.path()) {
                    uris.push(uri);
                }
            } else if entry.path().is_dir() {
                uris.extend(Self::project_files(entry.path()));
            }
        }
        uris
    }

    pub(crate) fn start_workspace_diagnostics(&mut self) {
        let mut _self = self.clone();
        spawn_new_thread(
            move || {
                macro_rules! work_done {
                    ($token: expr) => {{
                        let progress_end = WorkDoneProgressEnd {
                            message: Some(format!("checked 0 files")),
                        };
                        let params = ProgressParams {
                            token: $token.clone(),
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(progress_end)),
                        };
                        _self
                            .send_stdout(&json!({
                                "jsonrpc": "2.0",
                                "method": "$/progress",
                                "params": params,
                            }))
                            .unwrap();
                        _self.flags.workspace_checked.store(true, Ordering::Relaxed);
                        return;
                    }};
                    ($token: expr, $uris: expr) => {{
                        let progress_end = WorkDoneProgressEnd {
                            message: Some(format!("checked {} files", $uris.len())),
                        };
                        let params = ProgressParams {
                            token: $token.clone(),
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(progress_end)),
                        };
                        _self
                            .send_stdout(&json!({
                                "jsonrpc": "2.0",
                                "method": "$/progress",
                                "params": params,
                            }))
                            .unwrap();
                        _self.flags.workspace_checked.store(true, Ordering::Relaxed);
                        return;
                    }};
                }
                while !_self.flags.client_initialized() {
                    safe_yield();
                }
                let token = NumberOrString::String("els/start_workspace_diagnostics".to_string());
                let progress_token = WorkDoneProgressCreateParams {
                    token: token.clone(),
                };
                let _ = _self.send_stdout(&json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "window/workDoneProgress/create",
                    "params": progress_token,
                }));
                let Ok(current_dir) = current_dir() else {
                    work_done!(token)
                };
                let Some(src_dir) = project_entry_dir_of(&current_dir) else {
                    work_done!(token);
                };
                let Some(main_path) = project_entry_file_of(&current_dir) else {
                    work_done!(token);
                };
                let Ok(main_uri) = NormalizedUrl::from_file_path(main_path) else {
                    work_done!(token);
                };
                let uris = Self::project_files(src_dir);
                let progress_begin = WorkDoneProgressBegin {
                    title: "Checking workspace".to_string(),
                    cancellable: Some(false),
                    message: Some(format!("checking {} files ...", uris.len())),
                    percentage: Some(0),
                };
                let params = ProgressParams {
                    token: token.clone(),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(progress_begin)),
                };
                _self
                    .send_stdout(&json!({
                        "jsonrpc": "2.0",
                        "method": "$/progress",
                        "params": params,
                    }))
                    .unwrap();
                if let Ok(code) = _self.file_cache.get_entire_code(&main_uri) {
                    let mut checked = Set::new();
                    let _ = _self.check_file(main_uri.clone(), code, &mut checked);
                    _self.send_empty_diagnostics(checked).unwrap();
                }
                work_done!(token, uris);
            },
            fn_name!(),
        );
    }

    /// Send an empty `workspace/configuration` request periodically.
    /// If there is no response to the request within a certain period of time, terminate the server.
    pub fn start_client_health_checker(&self, receiver: Receiver<WorkerMessage<()>>) {
        const INTERVAL: Duration = Duration::from_secs(10);
        const TIMEOUT: Duration = Duration::from_secs(30);
        if self.stdout_redirect.is_some() {
            return;
        }
        let Some(client_pid) = self.init_params.process_id.map(|x| x as i32) else {
            return;
        };
        let _self = self.clone();
        // FIXME: close this thread when the server is restarted
        spawn_new_thread(
            move || {
                while !_self.flags.client_initialized() {
                    safe_yield();
                }
                loop {
                    // self.send_log("checking client health").unwrap();
                    let params = ConfigurationParams { items: vec![] };
                    _self
                        .send_stdout(&json!({
                            "jsonrpc": "2.0",
                            "id": HEALTH_CHECKER_ID,
                            "method": "workspace/configuration",
                            "params": params,
                        }))
                        .unwrap();
                    sleep(INTERVAL);
                }
            },
            "start_client_health_checker_sender",
        );
        spawn_new_thread(
            move || {
                loop {
                    match receiver.recv_timeout(TIMEOUT) {
                        Ok(WorkerMessage::Kill) => {
                            break;
                        }
                        Ok(_) => {
                            // self.send_log("client health check passed").unwrap();
                        }
                        Err(_) => {
                            lsp_log!("Client health check timed out");
                            // lsp_log!(self, "Restart the server");
                            // _log!(self, "Restart the server");
                            // send_error_info("Something went wrong, ELS has been restarted").unwrap();
                            // self_.restart();
                            if !is_process_alive(client_pid) {
                                lsp_log!("Client seems to be dead");
                                panic!("Client seems to be dead");
                            }
                        }
                    }
                }
            },
            "start_client_health_checker_receiver",
        );
    }
}
