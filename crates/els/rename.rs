use std::collections::HashMap;
use std::time::SystemTime;

use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::dict::Dict;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::varinfo::{AbsLocation, VarKind};

use lsp_types::{RenameParams, TextEdit, Url, WorkspaceEdit};

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
