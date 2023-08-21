use std::fs::File;
use std::io::Read;

use lsp_types::{
    DidChangeTextDocumentParams, FileOperationFilter, FileOperationPattern,
    FileOperationPatternKind, FileOperationRegistrationOptions, OneOf, Position, Range,
    RenameFilesParams, SaveOptions, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, Url, WorkspaceFileOperationsServerCapabilities,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};

use erg_common::dict::Dict;
use erg_common::shared::Shared;
use erg_common::traits::DequeStream;
use erg_compiler::erg_parser::lex::Lexer;
use erg_compiler::erg_parser::token::{Token, TokenStream};

use crate::_log;
use crate::server::ELSResult;
use crate::util::{self, NormalizedUrl};

fn _get_code_from_uri(uri: &Url) -> ELSResult<String> {
    let path = uri
        .to_file_path()
        .or_else(|_| util::denormalize(uri.clone()).to_file_path())
        .map_err(|_| format!("invalid file path: {uri}"))?;
    let mut code = String::new();
    File::open(path.as_path())?.read_to_string(&mut code)?;
    Ok(code)
}

#[derive(Debug, Clone)]
pub struct FileCacheEntry {
    pub code: String,
    pub ver: i32,
    pub token_stream: Option<TokenStream>,
}

impl FileCacheEntry {
    /// line: 0-based
    pub fn get_line(&self, line0: u32) -> Option<String> {
        let mut lines = self.code.lines();
        lines.nth(line0 as usize).map(|s| s.to_string())
    }
}

/// Stores the contents of the file on-memory.
/// This struct can save changes in real-time & incrementally.
#[derive(Debug, Clone)]
pub struct FileCache {
    pub files: Shared<Dict<NormalizedUrl, FileCacheEntry>>,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            files: Shared::new(Dict::new()),
        }
    }

    pub fn clear(&self) {
        self.files.borrow_mut().clear();
    }

    fn load_once(&self, uri: &NormalizedUrl) -> ELSResult<()> {
        if self.files.borrow_mut().get(uri).is_some() {
            return Ok(());
        }
        let code = _get_code_from_uri(uri)?;
        self.update(uri, code, None);
        Ok(())
    }

    pub(crate) fn set_capabilities(&mut self, capabilities: &mut ServerCapabilities) {
        let workspace_folders = WorkspaceFoldersServerCapabilities {
            supported: Some(true),
            change_notifications: Some(OneOf::Left(true)),
        };
        let file_op = FileOperationRegistrationOptions {
            filters: vec![
                FileOperationFilter {
                    scheme: Some(String::from("file")),
                    pattern: FileOperationPattern {
                        glob: String::from("**/*.{er,py}"),
                        matches: Some(FileOperationPatternKind::File),
                        options: None,
                    },
                },
                FileOperationFilter {
                    scheme: Some(String::from("file")),
                    pattern: FileOperationPattern {
                        glob: String::from("**"),
                        matches: Some(FileOperationPatternKind::Folder),
                        options: None,
                    },
                },
            ],
        };
        capabilities.workspace = Some(WorkspaceServerCapabilities {
            workspace_folders: Some(workspace_folders),
            file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                will_rename: Some(file_op),
                ..Default::default()
            }),
        });
        let sync_option = TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::INCREMENTAL),
            save: Some(SaveOptions::default().into()),
            ..Default::default()
        };
        capabilities.text_document_sync = Some(TextDocumentSyncCapability::Options(sync_option));
    }

    /// This method clones and returns the entire file.
    /// If you only need part of the file, use `get_ranged` or `get_line` instead.
    pub fn get_entire_code(&self, uri: &NormalizedUrl) -> ELSResult<String> {
        self.load_once(uri)?;
        Ok(self
            .files
            .borrow_mut()
            .get(uri)
            .ok_or("not found")?
            .code
            .clone())
    }

    pub fn get_token_stream(&self, uri: &NormalizedUrl) -> Option<TokenStream> {
        let _ = self.load_once(uri);
        self.files.borrow_mut().get(uri)?.token_stream.clone()
    }

    pub fn get_token(&self, uri: &NormalizedUrl, pos: Position) -> Option<Token> {
        let _ = self.load_once(uri);
        let ent = self.files.borrow_mut();
        let tokens = ent.get(uri)?.token_stream.as_ref()?;
        for tok in tokens.iter() {
            if util::pos_in_loc(tok, pos) {
                return Some(tok.clone());
            }
        }
        None
    }

    pub fn get_token_relatively(
        &self,
        uri: &NormalizedUrl,
        pos: Position,
        offset: isize,
    ) -> Option<Token> {
        let _ = self.load_once(uri);
        let ent = self.files.borrow_mut();
        let tokens = ent.get(uri)?.token_stream.as_ref()?;
        let index = (|| {
            for (i, tok) in tokens.iter().enumerate() {
                if util::pos_in_loc(tok, pos) {
                    return Some(i);
                }
            }
            None
        })()?;
        let index = (index as isize + offset) as usize;
        if index < tokens.len() {
            Some(tokens[index].clone())
        } else {
            None
        }
    }

    /// 0-based
    pub(crate) fn get_line(&self, uri: &NormalizedUrl, line0: u32) -> Option<String> {
        let _ = self.load_once(uri);
        self.files.borrow_mut().get(uri)?.get_line(line0)
    }

    pub(crate) fn get_ranged(
        &self,
        uri: &NormalizedUrl,
        range: Range,
    ) -> ELSResult<Option<String>> {
        self.load_once(uri)?;
        let ent = self.files.borrow_mut();
        let file = ent.get(uri).ok_or("not found")?;
        let mut code = String::new();
        for (i, line) in file.code.lines().enumerate() {
            if i >= range.start.line as usize && i <= range.end.line as usize {
                if i == range.start.line as usize && i == range.end.line as usize {
                    if line.len() < range.end.character as usize {
                        return Ok(None);
                    }
                    code.push_str(
                        &line[range.start.character as usize..range.end.character as usize],
                    );
                } else if i == range.start.line as usize {
                    code.push_str(&line[range.start.character as usize..]);
                    code.push('\n');
                } else if i == range.end.line as usize {
                    if line.len() < range.end.character as usize {
                        return Ok(None);
                    }
                    code.push_str(&line[..range.end.character as usize]);
                } else {
                    code.push_str(line);
                    code.push('\n');
                }
            }
        }
        Ok(Some(code))
    }

    pub(crate) fn update(&self, uri: &NormalizedUrl, code: String, ver: Option<i32>) {
        let ent = self.files.borrow_mut();
        let entry = ent.get(uri);
        if let Some(entry) = entry {
            if ver.map_or(false, |ver| ver <= entry.ver) {
                // crate::_log!("171: double update detected: {ver:?}, {}, code:\n{}", entry.ver, entry.code);
                return;
            }
        }
        let token_stream = Lexer::from_str(code.clone()).lex().ok();
        let ver = ver.unwrap_or({
            if let Some(entry) = entry {
                entry.ver
            } else {
                1
            }
        });
        drop(ent);
        self.files.borrow_mut().insert(
            uri.clone(),
            FileCacheEntry {
                code,
                ver,
                token_stream,
            },
        );
    }

    pub(crate) fn _ranged_update(&self, uri: &NormalizedUrl, old: Range, new_code: &str) {
        let mut ent = self.files.borrow_mut();
        let Some(entry) = ent.get_mut(uri) else {
            return;
        };
        let mut code = entry.code.clone();
        let start = util::pos_to_byte_index(&code, old.start);
        let end = util::pos_to_byte_index(&code, old.end);
        code.replace_range(start..end, new_code);
        let token_stream = Lexer::from_str(code.clone()).lex().ok();
        entry.code = code;
        // entry.ver += 1;
        entry.token_stream = token_stream;
    }

    pub(crate) fn incremental_update(&self, params: DidChangeTextDocumentParams) {
        let uri = NormalizedUrl::new(params.text_document.uri);
        let mut ent = self.files.borrow_mut();
        let Some(entry) = ent.get_mut(&uri) else {
            return;
        };
        if entry.ver >= params.text_document.version {
            // crate::_log!("212: double update detected {}, {}, code:\n{}", entry.ver, params.text_document.version, entry.code);
            return;
        }
        let mut code = entry.code.clone();
        for change in params.content_changes {
            let Some(range) = change.range else {
                continue;
            };
            let start = util::pos_to_byte_index(&code, range.start);
            let end = util::pos_to_byte_index(&code, range.end);
            code.replace_range(start..end, &change.text);
        }
        let token_stream = Lexer::from_str(code.clone()).lex().ok();
        entry.code = code;
        entry.ver = params.text_document.version;
        entry.token_stream = token_stream;
    }

    #[allow(unused)]
    pub fn remove(&mut self, uri: &NormalizedUrl) {
        self.files.borrow_mut().remove(uri);
    }

    pub fn rename_files(&mut self, params: &RenameFilesParams) -> ELSResult<()> {
        for file in &params.files {
            let Ok(old_uri) = NormalizedUrl::parse(&file.old_uri) else {
                _log!("failed to parse old uri: {}", file.old_uri);
                continue;
            };
            let Ok(new_uri) = NormalizedUrl::parse(&file.new_uri) else {
                _log!("failed to parse new uri: {}", file.new_uri);
                continue;
            };
            let Some(entry) = self.files.borrow_mut().remove(&old_uri) else {
                _log!("failed to find old uri: {}", file.old_uri);
                continue;
            };
            self.files.borrow_mut().insert(new_uri, entry);
        }
        Ok(())
    }

    pub fn entries(&self) -> Vec<NormalizedUrl> {
        self.files.borrow().keys().cloned().collect()
    }

    pub fn get_ver(&self, uri: &NormalizedUrl) -> Option<i32> {
        self.files.borrow().get(uri).map(|x| x.ver)
    }
}
