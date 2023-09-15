use std::env::current_dir;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::Duration;

use erg_common::consts::PYTHON_MODE;
use erg_common::dict::Dict;
use erg_common::spawn::{safe_yield, spawn_new_thread};
use erg_common::style::*;
use erg_common::traits::Stream;
use erg_common::{fn_name, lsp_log};
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::ast::Module;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::error::CompileErrors;

use lsp_types::{
    ConfigurationParams, Diagnostic, DiagnosticSeverity, NumberOrString, Position, ProgressParams,
    ProgressParamsValue, PublishDiagnosticsParams, Range, Url, WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressCreateParams, WorkDoneProgressEnd,
};
use serde_json::json;

use crate::_log;
use crate::channels::WorkerMessage;
use crate::diff::{ASTDiff, HIRDiff};
use crate::server::{DefaultFeatures, ELSResult, RedirectableStdout, Server};
use crate::server::{ASK_AUTO_SAVE_ID, HEALTH_CHECKER_ID};
use crate::util::{self, project_root_of, NormalizedUrl};

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn build_ast(&self, uri: &NormalizedUrl) -> Option<Module> {
        let code = self.file_cache.get_entire_code(uri).ok()?;
        Parser::parse(code).ok().map(|artifact| artifact.ast)
    }

    pub(crate) fn any_changes(&self, uri: &NormalizedUrl) -> bool {
        let deps = self.dependencies_of(uri);
        if deps.is_empty() {
            return true;
        }
        for dep in deps {
            let Some(old) = self.get_ast(&dep) else {
                return true;
            };
            if let Some(new) = self.build_ast(&dep) {
                if !ASTDiff::diff(old, &new).is_nop() {
                    return true;
                }
            }
        }
        _log!(self, "no changes: {uri}");
        false
    }

    pub(crate) fn recheck_file(
        &mut self,
        uri: NormalizedUrl,
        code: impl Into<String>,
    ) -> ELSResult<()> {
        if !self.any_changes(&uri) {
            _log!(self, "no changes: {uri}");
            return Ok(());
        }
        // self.clear_cache(&uri);
        self.check_file(uri, code)
    }

    pub(crate) fn check_file(
        &mut self,
        uri: NormalizedUrl,
        code: impl Into<String>,
    ) -> ELSResult<()> {
        _log!(self, "checking {uri}");
        if self.file_cache.editing.borrow().contains(&uri) {
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
        let artifact = match checker.build(code.into(), mode) {
            Ok(artifact) => {
                _log!(
                    self,
                    "checking {uri} passed, found warns: {}",
                    artifact.warns.len()
                );
                let uri_and_diags = self.make_uri_and_diags(artifact.warns.clone());
                // clear previous diagnostics
                self.send_diagnostics(uri.clone().raw(), vec![])?;
                for (uri, diags) in uri_and_diags.into_iter() {
                    _log!(self, "{uri}, warns: {}", diags.len());
                    self.send_diagnostics(uri, diags)?;
                }
                artifact.into()
            }
            Err(artifact) => {
                _log!(self, "found errors: {}", artifact.errors.len());
                _log!(self, "found warns: {}", artifact.warns.len());
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
                artifact
            }
        };
        let ast = self.build_ast(&uri);
        let ctx = checker.pop_context().unwrap();
        if mode == "declare" {
            self.shared
                .py_mod_cache
                .register(path, ast, artifact.object, ctx);
        } else {
            self.shared
                .mod_cache
                .register(path, ast, artifact.object, ctx);
        }
        self.shared.errors.extend(artifact.errors);
        self.shared.warns.extend(artifact.warns);
        let dependents = self.dependents_of(&uri);
        for dep in dependents {
            // _log!(self, "dep: {dep}");
            let code = self.file_cache.get_entire_code(&dep)?.to_string();
            self.check_file(dep, code)?;
        }
        Ok(())
    }

    pub(crate) fn quick_check_file(&mut self, uri: NormalizedUrl) -> ELSResult<()> {
        if self.file_cache.editing.borrow().contains(&uri) {
            _log!(self, "skipped: {uri}");
            return Ok(());
        }
        let Some(old) = self.get_ast(&uri) else {
            crate::_log!(self, "not found");
            return Ok(());
        };
        let Some(new) = self.build_ast(&uri) else {
            crate::_log!(self, "not found");
            return Ok(());
        };
        let ast_diff = ASTDiff::diff(old, &new);
        crate::_log!(self, "diff: {ast_diff}");
        if let Some((mut lowerer, mut irs)) = self.steal_lowerer(&uri) {
            if let Some((hir_diff, hir)) =
                HIRDiff::new(ast_diff, &mut lowerer).zip(irs.hir.as_mut())
            {
                crate::_log!(self, "hir_diff: {hir_diff}");
                hir_diff.update(hir);
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
                        _log!(_self, "Auto saving is enabled");
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
                                let _ = _self.check_file(uri.clone(), code);
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
            if entry.path().extension() == Some(OsStr::new("er")) {
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
                let Some(project_root) = project_root_of(&current_dir().unwrap()) else {
                    _self.flags.workspace_checked.store(true, Ordering::Relaxed);
                    return;
                };
                let src_dir = if project_root.join("src").is_dir() {
                    project_root.join("src")
                } else {
                    project_root
                };
                let Ok(main_uri) = NormalizedUrl::from_file_path(src_dir.join("main.er")) else {
                    _self.flags.workspace_checked.store(true, Ordering::Relaxed);
                    return;
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
                let code = _self.file_cache.get_entire_code(&main_uri).unwrap();
                let _ = _self.check_file(main_uri, code);
                let progress_end = WorkDoneProgressEnd {
                    message: Some(format!("checked {} files", uris.len())),
                };
                let params = ProgressParams {
                    token: token.clone(),
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
            },
            fn_name!(),
        );
    }

    /// Send an empty `workspace/configuration` request periodically.
    /// If there is no response to the request within a certain period of time, terminate the server.
    pub fn start_client_health_checker(&self, receiver: Receiver<WorkerMessage<()>>) {
        const INTERVAL: Duration = Duration::from_secs(5);
        const TIMEOUT: Duration = Duration::from_secs(10);
        if self.stdout_redirect.is_some() {
            return;
        }
        let _self = self.clone();
        // let mut self_ = self.clone();
        // FIXME: close this thread when the server is restarted
        spawn_new_thread(
            move || {
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
                            panic!("Client health check timed out");
                        }
                    }
                }
            },
            "start_client_health_checker_receiver",
        );
    }
}
