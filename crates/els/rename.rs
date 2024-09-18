use std::collections::HashMap;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    RenameFile, RenameFilesParams, RenameParams, ResourceOp, TextDocumentEdit, TextEdit, Url,
    WorkspaceEdit,
};

use erg_common::dict::Dict;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::hir::{Expr, Literal};
use erg_compiler::varinfo::{AbsLocation, VarKind};

#[allow(unused_imports)]
use crate::_log;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn rename(&mut self, msg: &Value) -> ELSResult<()> {
        let params = RenameParams::deserialize(&msg["params"])?;
        let id = msg["id"].as_i64().unwrap();
        self.send_log(format!("rename request: {params:?}"))?;
        let uri = NormalizedUrl::new(params.text_document_position.text_document.uri);
        let pos = params.text_document_position.position;
        if let Some(tok) = self.file_cache.get_symbol(&uri, pos) {
            // _log!(self, "tok: {tok}");
            if let Some(vi) = self
                .get_visitor(&uri)
                .and_then(|visitor| visitor.get_info(&tok))
            {
                let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
                let is_std = vi
                    .def_loc
                    .module
                    .as_ref()
                    .map(|path| path.starts_with(&self.erg_path))
                    .unwrap_or(false);
                let kind = if vi.t.is_method() {
                    "method"
                } else if vi.t.is_subr() {
                    "subroutine"
                } else {
                    "variable"
                };
                if vi.def_loc.loc.is_unknown() || is_std {
                    let error_reason = match vi.kind {
                        VarKind::Builtin => {
                            format!("this is a builtin {kind} and cannot be renamed")
                        }
                        VarKind::FixedAuto => {
                            format!("this is a fixed auto {kind} and cannot be renamed")
                        }
                        _ if is_std => {
                            "this is a standard library API and cannot be renamed".to_string()
                        }
                        _ => format!("this {kind} cannot be renamed"),
                    };
                    let edit = WorkspaceEdit::new(changes);
                    self.send_stdout(&json!({ "jsonrpc": "2.0", "id": id, "result": edit }))?;
                    return self.send_error_info(error_reason);
                }
                Self::commit_change(&mut changes, &vi.def_loc, params.new_name.clone());
                if let Some(value) = self.shared.index.get_refs(&vi.def_loc) {
                    // self.send_log(format!("referrers: {referrers:?}"))?;
                    for referrer in value.referrers.iter() {
                        Self::commit_change(&mut changes, referrer, params.new_name.clone());
                    }
                }
                let dependencies = self.dependencies_of(&uri);
                self.file_cache
                    .editing
                    .borrow_mut()
                    .extend(dependencies.clone());
                for uri in changes.keys() {
                    self.clear_cache(&NormalizedUrl::new(uri.clone()));
                }
                let timestamps = self.get_timestamps(changes.keys());
                let edit = WorkspaceEdit::new(changes);
                self.send_stdout(&json!({ "jsonrpc": "2.0", "id": id, "result": edit }))?;
                for _ in 0..20 {
                    self.send_log("waiting for file to be modified...")?;
                    if self.all_changed(&timestamps) {
                        break;
                    }
                    sleep(Duration::from_millis(50));
                }
                let mut checked = Set::new();
                // recheck dependencies and finally the file itself
                for dep in dependencies {
                    let code = self.file_cache.get_entire_code(&dep)?.to_string();
                    self.check_file(dep.clone(), code, &mut checked)?;
                    self.file_cache.editing.borrow_mut().remove(&dep);
                }
                self.send_empty_diagnostics(checked)?;
                // dependents are checked after changes are committed
                return Ok(());
            }
        }
        self.send_stdout(&json!({ "jsonrpc": "2.0", "id": id, "result": Value::Null }))
    }

    fn commit_change(
        changes: &mut HashMap<Url, Vec<TextEdit>>,
        abs_loc: &AbsLocation,
        new_name: String,
    ) {
        if let Some(path) = &abs_loc.module {
            let Ok(def_uri) = Url::from_file_path(path) else {
                return;
            };
            let Some(range) = util::loc_to_range(abs_loc.loc) else {
                return;
            };
            let edit = TextEdit::new(range, new_name);
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
                .unwrap_or(SystemTime::now());
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

    /// self is __included__.
    /// if self is not in the graph, return empty vec
    pub fn dependencies_of(&self, uri: &NormalizedUrl) -> Vec<NormalizedUrl> {
        let graph = &self.shared.graph;
        let path = NormalizedPathBuf::from(util::uri_to_path(uri));
        if let Err(err) = graph.sort() {
            // maybe key not found == self is not in the graph
            crate::_log!(self, "err: {err}");
            return vec![];
        };
        let Some(self_node) = graph.get_node(&path) else {
            return vec![];
        };
        graph
            .ref_inner()
            .iter()
            .filter(|node| node.id == path || self_node.depends_on(&node.id))
            .filter_map(|node| {
                Some(NormalizedUrl::new(
                    Url::from_file_path(node.id.to_path_buf()).ok()?,
                ))
            })
            .collect()
    }

    /// self is __not included__
    pub fn dependents_of(&self, uri: &NormalizedUrl) -> Vec<NormalizedUrl> {
        let graph = &self.shared.graph;
        let path = NormalizedPathBuf::from(util::uri_to_path(uri));
        graph
            .ref_inner()
            .iter()
            .filter(|node| node.depends_on(&path))
            .filter_map(|node| {
                Some(NormalizedUrl::new(
                    Url::from_file_path(node.id.to_path_buf()).ok()?,
                ))
            })
            .collect()
    }
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    fn collect_module_changes(
        &mut self,
        old_uri: &NormalizedUrl,
        new_uri: &NormalizedUrl,
    ) -> HashMap<Url, Vec<TextEdit>> {
        let mut changes = HashMap::new();
        let old_path = util::uri_to_path(old_uri)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let old_path = old_path.trim_end_matches(".d");
        let new_path = util::uri_to_path(new_uri)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if old_path.is_empty() || new_path.is_empty() {
            return changes;
        }
        let new_path = new_path.trim_end_matches(".d");
        for dep in self.dependents_of(old_uri) {
            let imports = self.search_imports(&dep, old_path);
            let edits = imports.iter().filter_map(|lit| {
                Some(TextEdit::new(
                    util::loc_to_range(lit.loc())?,
                    lit.token.content.replace(old_path, new_path),
                ))
            });
            changes.insert(dep.raw(), edits.collect());
        }
        changes
    }

    /// TODO: multi-path imports
    /// returning exprs: import symbol (string literal)
    fn search_imports(&self, target: &NormalizedUrl, needle_module_name: &str) -> Vec<Literal> {
        let mut imports = vec![];
        if let Some(hir) = self.get_hir(target) {
            for chunk in hir.module.iter() {
                imports.extend(Self::extract_import_symbols(chunk, needle_module_name));
            }
        }
        imports
    }

    fn extract_import_symbols(expr: &Expr, needle_module_name: &str) -> Vec<Literal> {
        match expr {
            Expr::Def(def) if def.def_kind().is_import() => {
                let Some(Expr::Call(import_call)) = def.body.block.first() else {
                    return vec![];
                };
                let Some(module_name) = import_call.args.get_left_or_key("Path") else {
                    return vec![];
                };
                match module_name {
                    Expr::Literal(lit)
                        if lit
                            .token
                            .content
                            .trim_start_matches('\"')
                            .trim_end_matches('\"')
                            .ends_with(needle_module_name) =>
                    // FIXME: Possibly a submodule of the same name of another module
                    {
                        vec![lit.clone()]
                    }
                    _ => vec![],
                }
            }
            _ => vec![],
        }
    }

    fn rename_linked_files(
        &self,
        renames: &mut Vec<DocumentChangeOperation>,
        old_uri: &NormalizedUrl,
        new_uri: &NormalizedUrl,
    ) {
        if old_uri.as_str().ends_with(".d.er") {
            let Ok(old_uri) = Url::parse(&old_uri.as_str().replace(".d.er", ".py")) else {
                return;
            };
            let Ok(new_uri) = Url::parse(&new_uri.as_str().replace(".d.er", ".py")) else {
                return;
            };
            let rename = DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                old_uri,
                new_uri,
                options: None,
                annotation_id: None,
            }));
            renames.push(rename);
        } else if old_uri.as_str().ends_with(".py") {
            let d_er_file = PathBuf::from(old_uri.as_str().replace(".py", ".d.er"));
            if d_er_file.exists() {
                let Ok(old_uri) = Url::from_file_path(&d_er_file) else {
                    return;
                };
                let Ok(new_uri) = Url::parse(&new_uri.as_str().replace(".py", ".d.er")) else {
                    return;
                };
                let rename = DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                    old_uri,
                    new_uri,
                    options: None,
                    annotation_id: None,
                }));
                renames.push(rename);
            }
        }
    }

    /// Rename .er files and rewrite the imports of the dependent files.
    /// This does not update `file_cache`, the editing is done by a `didChange` request.
    pub(crate) fn handle_will_rename_files(
        &mut self,
        params: RenameFilesParams,
    ) -> ELSResult<Option<WorkspaceEdit>> {
        self.send_log("workspace/willRenameFiles request")?;
        let mut edits = HashMap::new();
        let mut renames = vec![];
        for file in &params.files {
            let Ok(old) = Url::parse(&file.old_uri) else {
                continue;
            };
            let old_uri = NormalizedUrl::new(old);
            let Ok(new) = Url::parse(&file.new_uri) else {
                continue;
            };
            let new_uri = NormalizedUrl::new(new);
            edits.extend(self.collect_module_changes(&old_uri, &new_uri));
            self.rename_linked_files(&mut renames, &old_uri, &new_uri);
            let Some(entry) = self.remove_module_entry(&old_uri) else {
                continue;
            };
            self.insert_module_entry(new_uri.clone(), entry);
            let Some(entry) = self.steal_entry(&old_uri) else {
                continue;
            };
            self.shared.rename_path(
                &old_uri.to_file_path().unwrap().into(),
                new_uri.to_file_path().unwrap().into(),
            );
            self.restore_entry(new_uri, entry);
        }
        self.file_cache.rename_files(&params)?;
        let changes = {
            let edits = edits.into_iter().map(|(uri, edits)| {
                let text_document = OptionalVersionedTextDocumentIdentifier { uri, version: None };
                let edit = TextDocumentEdit {
                    text_document,
                    edits: edits.into_iter().map(OneOf::Left).collect(),
                };
                DocumentChangeOperation::Edit(edit)
            });
            let ops = edits.chain(renames).collect();
            DocumentChanges::Operations(ops)
        };
        let edit = WorkspaceEdit {
            document_changes: Some(changes),
            ..Default::default()
        };
        Ok(Some(edit))
    }
}
