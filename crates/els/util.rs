use std::fmt;
use std::fs::{metadata, Metadata};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use erg_common::consts::CASE_SENSITIVE;
use erg_common::normalize_path;
use erg_common::traits::{DequeStream, Locational};

use erg_compiler::erg_parser::token::{Token, TokenStream};

use erg_compiler::varinfo::AbsLocation;
use lsp_types::{Position, Range, Url};

use crate::server::ELSResult;

/// See also: `erg_common::pathutil::NormalizedPathBuf`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedUrl(Url);

impl fmt::Display for NormalizedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for NormalizedUrl {
    type Err = Box<dyn std::error::Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NormalizedUrl::parse(s)
    }
}

impl std::ops::Deref for NormalizedUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Url> for NormalizedUrl {
    fn as_ref(&self) -> &Url {
        &self.0
    }
}

impl TryFrom<&Path> for NormalizedUrl {
    type Error = Box<dyn std::error::Error>;
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        NormalizedUrl::from_file_path(path)
    }
}

impl NormalizedUrl {
    fn normalize(url: &str) -> String {
        // FIXME: Some directories are case-sensitive even in Windows
        if cfg!(windows) && !CASE_SENSITIVE {
            url.replace("c%3A", "C:").to_lowercase()
        } else if !CASE_SENSITIVE {
            url.to_lowercase()
        } else {
            url.to_string()
        }
    }

    pub fn new(url: Url) -> NormalizedUrl {
        Self(Url::parse(&Self::normalize(url.as_str())).unwrap())
    }

    pub fn parse(uri: &str) -> ELSResult<NormalizedUrl> {
        Ok(NormalizedUrl(Url::parse(&Self::normalize(uri))?))
    }

    pub fn from_file_path<P: AsRef<Path>>(path: P) -> ELSResult<Self> {
        Ok(NormalizedUrl::new(Url::from_file_path(&path).map_err(
            |_| format!("Invalid path: {}", path.as_ref().display()),
        )?))
    }

    pub fn raw(self) -> Url {
        self.0
    }
}

pub(crate) fn loc_to_range(loc: erg_common::error::Location) -> Option<Range> {
    let start = Position::new(loc.ln_begin()?.saturating_sub(1), loc.col_begin()?);
    let end = Position::new(loc.ln_end()?.saturating_sub(1), loc.col_end()?);
    Some(Range::new(start, end))
}

pub(crate) fn loc_to_pos(loc: erg_common::error::Location) -> Option<Position> {
    // FIXME: should `Position::new(loc.ln_begin()? - 1, loc.col_begin()?)`
    // but completion doesn't work (because the newline will be included)
    let start = Position::new(loc.ln_begin()?.saturating_sub(1), loc.col_begin()? + 1);
    Some(start)
}

pub fn pos_to_loc(pos: Position) -> erg_common::error::Location {
    erg_common::error::Location::range(
        pos.line + 1,
        pos.character.saturating_sub(1),
        pos.line + 1,
        pos.character,
    )
}

pub(crate) fn pos_in_loc<L: Locational>(loc: &L, pos: Position) -> bool {
    let ln_begin = loc.ln_begin().unwrap_or(0);
    let ln_end = loc.ln_end().unwrap_or(0);
    let in_lines = (ln_begin..=ln_end).contains(&(pos.line + 1));
    if ln_begin == ln_end {
        in_lines
            && (loc.col_begin().unwrap_or(0)..loc.col_end().unwrap_or(0)).contains(&pos.character)
    } else {
        in_lines
    }
}

pub(crate) fn roughly_pos_in_loc<L: Locational>(loc: &L, pos: Position) -> bool {
    let ln_begin = loc.ln_begin().unwrap_or(0);
    let ln_end = loc.ln_end().unwrap_or(0);
    let in_lines = (ln_begin..=ln_end).contains(&(pos.line + 1));
    if ln_begin == ln_end {
        in_lines
            && (loc.col_begin().unwrap_or(0)..=loc.col_end().unwrap_or(0)).contains(&pos.character)
    } else {
        in_lines
    }
}

pub(crate) fn pos_to_byte_index(src: &str, pos: Position) -> usize {
    if src.is_empty() {
        return 0;
    }
    let mut line = 0;
    let mut col = 0;
    for (index, c) in src.char_indices() {
        if line == pos.line && col == pos.character {
            return index;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    // EOF
    src.char_indices().last().unwrap().0 + 1
}

pub(crate) fn get_token_from_stream(
    stream: &TokenStream,
    pos: Position,
) -> ELSResult<Option<Token>> {
    for token in stream.iter() {
        if pos_in_loc(token, pos) {
            return Ok(Some(token.clone()));
        }
    }
    Ok(None)
}

pub(crate) fn get_metadata_from_uri(uri: &Url) -> ELSResult<Metadata> {
    let path = uri
        .to_file_path()
        .map_err(|_| "failed to convert uri to path")?;
    Ok(metadata(path)?)
}

pub(crate) fn uri_to_path(uri: &NormalizedUrl) -> PathBuf {
    normalize_path(
        uri.to_file_path()
            .unwrap_or_else(|_| denormalize(uri.clone().raw()).to_file_path().unwrap()),
    )
}

pub(crate) fn denormalize(uri: Url) -> Url {
    Url::parse(&uri.as_str().replace("c:", "file:///c%3A")).unwrap()
}

pub(crate) fn abs_loc_to_lsp_loc(loc: &AbsLocation) -> Option<lsp_types::Location> {
    let uri = Url::from_file_path(loc.module.as_ref()?).ok()?;
    let range = loc_to_range(loc.loc)?;
    Some(lsp_types::Location::new(uri, range))
}
