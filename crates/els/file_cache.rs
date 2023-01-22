use std::fs::Metadata;

use lsp_types::Url;

use erg_common::dict::Dict;
use erg_compiler::erg_parser::token::TokenStream;

#[derive(Debug, Clone)]
pub struct FileCacheEntry {
    pub code: String,
    pub metadata: Metadata,
    pub token_stream: TokenStream,
}

#[derive(Debug, Clone)]
pub struct FileCache {
    pub files: Dict<Url, FileCacheEntry>,
}
