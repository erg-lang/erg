use serde_json::json;

use erg_common::style::*;
use erg_common::traits::Stream;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::Parser;
use erg_compiler::error::CompileErrors;

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Url};

use crate::server::{send, send_log, ELSResult, Server};
use crate::util;

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn check_file<S: Into<String>>(&mut self, uri: Url, code: S) -> ELSResult<()> {
        send_log(format!("checking {uri}"))?;
        let path = util::uri_to_path(&uri);
        let mode = if path.to_string_lossy().ends_with(".d.er") {
            "declare"
        } else {
            "exec"
        };
        let mut checker = self.get_checker(path);
        match checker.build(code.into(), mode) {
            Ok(artifact) => {
                send_log(format!("checking {uri} passed"))?;
                let uri_and_diags = self.make_uri_and_diags(uri.clone(), artifact.warns.clone());
                // clear previous diagnostics
                self.send_diagnostics(uri.clone(), vec![])?;
                for (uri, diags) in uri_and_diags.into_iter() {
                    send_log(format!("{uri}, warns: {}", diags.len()))?;
                    self.send_diagnostics(uri, diags)?;
                }
                self.artifacts.insert(uri.clone(), artifact.into());
            }
            Err(artifact) => {
                send_log(format!("found errors: {}", artifact.errors.len()))?;
                send_log(format!("found warns: {}", artifact.warns.len()))?;
                let diags = artifact
                    .errors
                    .clone()
                    .into_iter()
                    .chain(artifact.warns.clone().into_iter())
                    .collect();
                let uri_and_diags = self.make_uri_and_diags(uri.clone(), diags);
                if uri_and_diags.is_empty() {
                    self.send_diagnostics(uri.clone(), vec![])?;
                }
                for (uri, diags) in uri_and_diags.into_iter() {
                    send_log(format!("{uri}, errs & warns: {}", diags.len()))?;
                    self.send_diagnostics(uri, diags)?;
                }
                self.artifacts.insert(uri.clone(), artifact);
            }
        }
        if let Some(module) = checker.pop_context() {
            send_log(format!("{uri}: {}", module.context.name))?;
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

    pub(crate) fn quick_check_file(&mut self, uri: Url) -> ELSResult<()> {
        // send_log(format!("checking {uri}"))?;
        let mut parser = Parser::new(self.file_cache.get_token_stream(&uri).unwrap().clone());
        if parser.parse().is_err() {
            return Ok(());
        }
        let path = util::uri_to_path(&uri);
        let code = &self.file_cache.get(&uri).unwrap().code;
        let mode = if path.to_string_lossy().ends_with(".d.er") {
            "declare"
        } else {
            "exec"
        };
        let mut checker = self.get_checker(path);
        match checker.build(code.into(), mode) {
            Ok(artifact) => {
                self.artifacts.insert(uri.clone(), artifact.into());
            }
            Err(artifact) => {
                self.artifacts.insert(uri.clone(), artifact);
            }
        }
        if let Some(module) = checker.pop_context() {
            self.modules.insert(uri.clone(), module);
        }
        let dependents = self.dependents_of(&uri);
        for dep in dependents {
            // _log!("dep: {dep}");
            self.quick_check_file(dep)?;
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

    fn send_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) -> ELSResult<()> {
        let params = PublishDiagnosticsParams::new(uri, diagnostics, None);
        if self
            .client_capas
            .text_document
            .as_ref()
            .map(|doc| doc.publish_diagnostics.is_some())
            .unwrap_or(false)
        {
            send(&json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": params,
            }))?;
        } else {
            send_log("the client does not support diagnostics")?;
        }
        Ok(())
    }
}
