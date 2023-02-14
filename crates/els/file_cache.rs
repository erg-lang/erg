use std::fs::{metadata, Metadata};

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
use erg_compiler::erg_parser::token::{Token, TokenKind, TokenStream};

use crate::server::ELSResult;
use crate::util;

#[derive(Debug, Clone)]
pub struct FileCacheEntry {
    pub code: String,
    pub metadata: Metadata,
    pub token_stream: Option<TokenStream>,
}

#[derive(Debug, Clone)]
pub struct FileCache {
    pub files: Shared<Dict<Url, FileCacheEntry>>,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            files: Shared::new(Dict::new()),
        }
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

    pub fn get(&self, uri: &Url) -> ELSResult<&FileCacheEntry> {
        let Some(entry) = unsafe { self.files.as_ref() }.get(uri) else {
            let code = util::get_code_from_uri(uri)?;
            self.update(uri, code);
            let entry = unsafe { self.files.as_ref() }.get(uri).ok_or("not found")?;
            return Ok(entry);
        };
        let last_modified = entry.metadata.modified().unwrap();
        let current_modified = metadata(uri.to_file_path().unwrap())
            .unwrap()
            .modified()
            .unwrap();
        if last_modified != current_modified {
            let code = util::get_code_from_uri(uri)?;
            self.update(uri, code);
            unsafe { self.files.as_ref() }
                .get(uri)
                .ok_or("not found".into())
        } else {
            let entry = unsafe { self.files.as_ref() }.get(uri).ok_or("not found")?;
            Ok(entry)
        }
    }

    pub fn get_token_stream(&self, uri: &Url) -> Option<&TokenStream> {
        self.get(uri).ok().and_then(|ent| ent.token_stream.as_ref())
    }

    pub fn get_token_index(&self, uri: &Url, pos: Position) -> ELSResult<Option<usize>> {
        let Some(tokens) = self.get_token_stream(uri) else {
            return Ok(None);
        };
        for (i, tok) in tokens.iter().enumerate() {
            if util::pos_in_loc(tok, pos) {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    pub fn get_token(&self, uri: &Url, pos: Position) -> ELSResult<Option<Token>> {
        let Some(tokens) = self.get_token_stream(uri) else {
            return Ok(None);
        };
        for tok in tokens.iter() {
            if util::pos_in_loc(tok, pos) {
                return Ok(Some(tok.clone()));
            }
        }
        Ok(None)
    }

    pub fn get_token_relatively(
        &self,
        uri: &Url,
        pos: Position,
        offset: isize,
    ) -> ELSResult<Option<Token>> {
        let Some(tokens) = self.get_token_stream(uri) else {
            return Ok(None);
        };
        let Some(index) = self.get_token_index(uri, pos)? else {
            for token in tokens.iter().rev() {
                if !matches!(token.kind, TokenKind::EOF | TokenKind::Dedent | TokenKind::Newline) {
                    return Ok(Some(token.clone()));
                }
            }
            return Ok(None);
        };
        let index = (index as isize + offset) as usize;
        if index < tokens.len() {
            Ok(Some(tokens[index].clone()))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn update(&self, uri: &Url, code: String) {
        let metadata = metadata(uri.to_file_path().unwrap()).unwrap();
        let token_stream = Lexer::from_str(code.clone()).lex().ok();
        self.files.borrow_mut().insert(
            uri.clone(),
            FileCacheEntry {
                code,
                metadata,
                token_stream,
            },
        );
    }

    pub(crate) fn ranged_update(&self, uri: &Url, old: Range, new_code: &str) {
        let Some(entry) = unsafe { self.files.as_mut() }.get_mut(uri) else {
            return;
        };
        let metadata = metadata(uri.to_file_path().unwrap()).unwrap();
        let mut code = entry.code.clone();
        let start = util::pos_to_index(&code, old.start);
        let end = util::pos_to_index(&code, old.end);
        code.replace_range(start..end, new_code);
        let token_stream = Lexer::from_str(code.clone()).lex().ok();
        entry.code = code;
        entry.metadata = metadata;
        entry.token_stream = token_stream;
    }

    pub(crate) fn incremental_update(&self, params: DidChangeTextDocumentParams) {
        let uri = util::normalize_url(params.text_document.uri);
        let Some(entry) = unsafe { self.files.as_mut() }.get_mut(&uri) else {
            return;
        };
        let metadata = metadata(uri.to_file_path().unwrap()).unwrap();
        let mut code = entry.code.clone();
        for change in params.content_changes {
            let range = change.range.unwrap();
            let start = util::pos_to_index(&code, range.start);
            let end = util::pos_to_index(&code, range.end);
            code.replace_range(start..end, &change.text);
        }
        let token_stream = Lexer::from_str(code.clone()).lex().ok();
        entry.code = code;
        entry.metadata = metadata;
        entry.token_stream = token_stream;
    }

    #[allow(unused)]
    pub fn remove(&mut self, uri: &Url) {
        self.files.borrow_mut().remove(uri);
    }

    pub fn rename_files(&mut self, params: &RenameFilesParams) -> ELSResult<()> {
        for file in &params.files {
            let old_uri = util::normalize_url(Url::parse(&file.old_uri).unwrap());
            let new_uri = util::normalize_url(Url::parse(&file.new_uri).unwrap());
            let Some(entry) = self.files.borrow_mut().remove(&old_uri) else {
                continue;
            };
            self.files.borrow_mut().insert(new_uri, entry);
        }
        Ok(())
    }
}
