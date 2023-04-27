use std::collections::HashMap;
use std::path::PathBuf;
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

use lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    RenameFile, RenameFilesParams, RenameParams, ResourceOp, TextDocumentEdit, TextEdit, Url,
    WorkspaceEdit,
};

use crate::server::{send, send_error_info, send_log, ELSResult, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn rename(&mut self, msg: &Value) -> ELSResult<()> {
        let params = RenameParams::deserialize(&msg["params"])?;
        send_log(format!("rename request: {params:?}"))?;
        let uri = NormalizedUrl::new(params.text_document_position.text_document.uri);
        let pos = params.text_document_position.position;
        if let Some(tok) = self.file_cache.get_token(&uri, pos) {
            // send_log(format!("token: {tok}"))?;
            if let Some(visitor) = self.get_visitor(&uri) {
                if let Some(vi) = visitor.get_info(&tok) {
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
                        send(
                            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": edit }),
                        )?;
                        return send_error_info(error_reason);
                    }
                    Self::commit_change(&mut changes, &vi.def_loc, params.new_name.clone());
                    if let Some(value) = self.get_index().get_refs(&vi.def_loc) {
                        // send_log(format!("referrers: {referrers:?}"))?;
                        for referrer in value.referrers.iter() {
                            Self::commit_change(&mut changes, referrer, params.new_name.clone());
                        }
                    }
                    let dependencies = self.dependencies_of(&uri);
                    for uri in changes.keys() {
                        self.clear_cache(&NormalizedUrl::new(uri.clone()));
                    }
                    let timestamps = self.get_timestamps(changes.keys());
                    let edit = WorkspaceEdit::new(changes);
                    send(
                        &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": edit }),
                    )?;
                    for _ in 0..20 {
                        send_log("waiting for file to be modified...")?;
                        if self.all_changed(&timestamps) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    // recheck dependencies and finally the file itself
                    for dep in dependencies {
                        let code = self.file_cache.get_code(&dep)?.to_string();
                        self.check_file(dep, code)?;
                    }
                    // dependents are checked after changes are committed
                    return Ok(());
                }
            }
        }
        send(&json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": Value::Null }))
    }

    fn commit_change(
        changes: &mut HashMap<Url, Vec<TextEdit>>,
        abs_loc: &AbsLocation,
        new_name: String,
    ) {
        if let Some(path) = &abs_loc.module {
            let def_uri = Url::from_file_path(path).unwrap();
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
    pub fn dependencies_of(&self, uri: &NormalizedUrl) -> Vec<NormalizedUrl> {
        let graph = &self.get_shared().unwrap().graph;
        let path = util::uri_to_path(uri);
        graph.sort().unwrap();
        let self_node = graph.get_node(&path).unwrap();
        graph
            .iter()
            .filter(|node| node.id == path || self_node.depends_on(&node.id))
            .map(|node| NormalizedUrl::new(Url::from_file_path(&node.id).unwrap()))
            .collect()
    }

    /// self is __not included__
    pub fn dependents_of(&self, uri: &NormalizedUrl) -> Vec<NormalizedUrl> {
        let graph = &self.get_shared().unwrap().graph;
        let path = util::uri_to_path(uri);
        graph
            .iter()
            .filter(|node| node.depends_on(&path))
            .map(|node| NormalizedUrl::new(Url::from_file_path(&node.id).unwrap()))
            .collect()
    }
}

impl<Checker: BuildRunnable> Server<Checker> {
    fn collect_module_changes(
        &mut self,
        old_uri: &NormalizedUrl,
        new_uri: &NormalizedUrl,
    ) -> HashMap<Url, Vec<TextEdit>> {
        let old_path = util::uri_to_path(old_uri)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let old_path = old_path.trim_end_matches(".d");
        let new_path = util::uri_to_path(new_uri)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let new_path = new_path.trim_end_matches(".d");
        let mut changes = HashMap::new();
        for dep in self.dependents_of(old_uri) {
            let imports = self.search_imports(&dep, old_path);
            for import in imports.iter() {
                let range = util::loc_to_range(import.loc()).unwrap();
                self.file_cache.ranged_update(&dep, range, new_path);
            }
            let edits = imports.iter().map(|lit| {
                TextEdit::new(
                    util::loc_to_range(lit.loc()).unwrap(),
                    lit.token.content.replace(old_path, new_path),
                )
            });
            changes.insert(dep.raw(), edits.collect());
        }
        changes
    }

    /// TODO: multi-path imports
    /// returning exprs: import symbol (string literal)
    fn search_imports(&self, target: &NormalizedUrl, needle_module_name: &str) -> Vec<&Literal> {
        let mut imports = vec![];
        if let Some(IncompleteArtifact {
            object: Some(hir), ..
        }) = self.artifacts.get(target)
        {
            for chunk in hir.module.iter() {
                imports.extend(Self::extract_import_symbols(chunk, needle_module_name));
            }
        }
        imports
    }

    fn extract_import_symbols<'e>(expr: &'e Expr, needle_module_name: &str) -> Vec<&'e Literal> {
        match expr {
            Expr::Def(def) if def.def_kind().is_import() => {
                let Some(Expr::Call(import_call)) = def.body.block.first() else {
                    return vec![];
                };
                let module_name = import_call.args.get_left_or_key("Path").unwrap();
                match module_name {
                    Expr::Lit(lit)
                        if lit
                            .token
                            .content
                            .trim_start_matches('\"')
                            .trim_end_matches('\"')
                            .ends_with(needle_module_name) =>
                    // FIXME: Possibly a submodule of the same name of another module
                    {
                        vec![lit]
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
            let rename = DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                old_uri: Url::parse(&old_uri.as_str().replace(".d.er", ".py")).unwrap(),
                new_uri: Url::parse(&new_uri.as_str().replace(".d.er", ".py")).unwrap(),
                options: None,
                annotation_id: None,
            }));
            renames.push(rename);
        } else if old_uri.as_str().ends_with(".py") {
            let d_er_file = PathBuf::from(old_uri.as_str().replace(".py", ".d.er"));
            if d_er_file.exists() {
                let rename = DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                    old_uri: Url::from_file_path(&d_er_file).unwrap(),
                    new_uri: Url::parse(&new_uri.as_str().replace(".py", ".d.er")).unwrap(),
                    options: None,
                    annotation_id: None,
                }));
                renames.push(rename);
            }
        }
    }

    pub(crate) fn handle_will_rename_files(
        &mut self,
        params: RenameFilesParams,
    ) -> ELSResult<Option<WorkspaceEdit>> {
        send_log("workspace/willRenameFiles request")?;
        let mut edits = HashMap::new();
        let mut renames = vec![];
        for file in &params.files {
            let old_uri = NormalizedUrl::new(Url::parse(&file.old_uri).unwrap());
            let new_uri = NormalizedUrl::new(Url::parse(&file.new_uri).unwrap());
            edits.extend(self.collect_module_changes(&old_uri, &new_uri));
            self.rename_linked_files(&mut renames, &old_uri, &new_uri);
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
