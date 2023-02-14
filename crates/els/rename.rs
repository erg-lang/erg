use std::collections::HashMap;
use std::time::SystemTime;

use erg_common::traits::{Locational, Stream};
use erg_compiler::artifact::IncompleteArtifact;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::dict::Dict;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::hir::{Expr, Literal};
use erg_compiler::varinfo::{AbsLocation, VarKind};

use lsp_types::{RenameFilesParams, RenameParams, TextEdit, Url, WorkspaceEdit};

use crate::server::{ELSResult, Server};
use crate::util;

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn rename(&mut self, msg: &Value) -> ELSResult<()> {
        let params = RenameParams::deserialize(&msg["params"])?;
        Self::send_log(format!("rename request: {params:?}"))?;
        let uri = util::normalize_url(params.text_document_position.text_document.uri);
        let pos = params.text_document_position.position;
        if let Some(tok) = self.file_cache.get_token(&uri, pos)? {
            // Self::send_log(format!("token: {tok}"))?;
            if let Some(visitor) = self.get_visitor(&uri) {
                if let Some(vi) = visitor.get_info(&tok) {
                    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
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
                        // identical change (to avoid displaying "no result")
                        Self::commit_change(&mut changes, &vi.def_loc, tok.content.to_string());
                        let edit = WorkspaceEdit::new(changes);
                        Self::send(
                            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": edit }),
                        )?;
                        return Self::send_error_info(error_reason);
                    }
                    Self::commit_change(&mut changes, &vi.def_loc, params.new_name.clone());
                    if let Some(value) = self.get_index().get_refs(&vi.def_loc) {
                        // Self::send_log(format!("referrers: {referrers:?}"))?;
                        for referrer in value.referrers.iter() {
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
    pub fn dependencies_of(&self, uri: &Url) -> Vec<Url> {
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
}

impl<Checker: BuildRunnable> Server<Checker> {
    fn collect_module_changes(
        &mut self,
        old_uri: &Url,
        new_uri: &Url,
    ) -> HashMap<Url, Vec<TextEdit>> {
        let new_path = util::uri_to_path(new_uri)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let new_path = format!("\"{new_path}\"");
        let mut changes = HashMap::new();
        for dep in self.dependents_of(old_uri) {
            let imports = self.search_imports(&dep, old_uri);
            for import in imports.iter() {
                let range = util::loc_to_range(import.loc()).unwrap();
                self.file_cache.ranged_update(&dep, range, &new_path);
            }
            let edits = imports
                .iter()
                .map(|lit| TextEdit::new(util::loc_to_range(lit.loc()).unwrap(), new_path.clone()));
            changes.insert(dep, edits.collect());
        }
        changes
    }

    /// TODO: multi-path imports
    /// returning exprs: import call
    fn search_imports(&self, target: &Url, needle: &Url) -> Vec<&Literal> {
        let needle_module_name = util::uri_to_path(needle)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let mut imports = vec![];
        if let Some(IncompleteArtifact {
            object: Some(hir), ..
        }) = self.artifacts.get(target)
        {
            for chunk in hir.module.iter() {
                match chunk {
                    Expr::Def(def) if def.def_kind().is_import() => {
                        let Some(Expr::Call(import_call)) = def.body.block.first() else {
                            continue;
                        };
                        let module_name = import_call.args.get_left_or_key("Path").unwrap();
                        match module_name {
                            Expr::Lit(lit)
                                if lit
                                    .token
                                    .content
                                    .trim_start_matches('\"')
                                    .trim_end_matches('\"')
                                    == needle_module_name =>
                            {
                                imports.push(lit);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        imports
    }

    pub(crate) fn rename_files(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log("workspace/willRenameFiles request")?;
        let params = RenameFilesParams::deserialize(msg["params"].clone())?;
        let mut edits = HashMap::new();
        for file in &params.files {
            let old_uri = util::normalize_url(Url::parse(&file.old_uri).unwrap());
            let new_uri = util::normalize_url(Url::parse(&file.new_uri).unwrap());
            edits.extend(self.collect_module_changes(&old_uri, &new_uri));
            let Some(entry) = self.artifacts.remove(&old_uri) else {
                continue;
            };
            self.artifacts.insert(new_uri.clone(), entry);
            let Some(entry) = self.modules.remove(&old_uri) else {
                continue;
            };
            self.modules.insert(new_uri, entry);
            if let Some(shared) = self.get_shared() {
                shared.clear_all();
            }
        }
        self.file_cache.rename_files(&params)?;
        let edit = WorkspaceEdit::new(edits);
        Self::send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": edit }))
    }
}
