use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use erg_common::normalize_path;
use erg_common::traits::{DequeStream, Locational};

use erg_compiler::erg_parser::token::{Token, TokenStream};

use lsp_types::{Position, Range, Url};

use crate::file_cache::_get_code_from_uri;
use crate::server::ELSResult;

pub fn loc_to_range(loc: erg_common::error::Location) -> Option<Range> {
    let start = Position::new(loc.ln_begin()?.saturating_sub(1), loc.col_begin()?);
    let end = Position::new(loc.ln_end()?.saturating_sub(1), loc.col_end()?);
    Some(Range::new(start, end))
}

pub fn loc_to_pos(loc: erg_common::error::Location) -> Option<Position> {
    // FIXME: should `Position::new(loc.ln_begin()? - 1, loc.col_begin()?)`
    // but completion doesn't work (because the newline will be included)
    let start = Position::new(loc.ln_begin()?.saturating_sub(1), loc.col_begin()? + 1);
    Some(start)
}

pub fn _pos_to_loc(pos: Position) -> erg_common::error::Location {
    erg_common::error::Location::range(
        pos.line + 1,
        pos.character.saturating_sub(1),
        pos.line + 1,
        pos.character,
    )
}

pub fn pos_in_loc<L: Locational>(loc: &L, pos: Position) -> bool {
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

pub fn pos_to_byte_index(src: &str, pos: Position) -> usize {
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

pub fn get_token_from_stream(stream: &TokenStream, pos: Position) -> ELSResult<Option<Token>> {
    for token in stream.iter() {
        if pos_in_loc(token, pos) {
            return Ok(Some(token.clone()));
        }
    }
    Ok(None)
}

pub fn get_ranged_code_from_uri(uri: &Url, range: Range) -> ELSResult<Option<String>> {
    let path = uri.to_file_path().unwrap();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut code = String::new();
    for (i, line) in reader.lines().enumerate() {
        if i >= range.start.line as usize && i <= range.end.line as usize {
            let line = line?;
            if i == range.start.line as usize && i == range.end.line as usize {
                if line.len() < range.end.character as usize {
                    return Ok(None);
                }
                code.push_str(&line[range.start.character as usize..range.end.character as usize]);
            } else if i == range.start.line as usize {
                code.push_str(&line[range.start.character as usize..]);
                code.push('\n');
            } else if i == range.end.line as usize {
                if line.len() < range.end.character as usize {
                    return Ok(None);
                }
                code.push_str(&line[..range.end.character as usize]);
            } else {
                code.push_str(&line);
                code.push('\n');
            }
        }
    }
    Ok(Some(code))
}

pub fn get_line_from_uri(uri: &Url, line: u32) -> ELSResult<String> {
    let code = _get_code_from_uri(uri)?;
    let line = code
        .lines()
        .nth(line.saturating_sub(1) as usize)
        .unwrap_or("");
    Ok(line.to_string())
}

pub fn get_metadata_from_uri(uri: &Url) -> ELSResult<std::fs::Metadata> {
    let path = uri.to_file_path().unwrap();
    Ok(std::fs::metadata(path)?)
}

pub fn get_line_from_path(path: &Path, line: u32) -> ELSResult<String> {
    let mut code = String::new();
    File::open(path)?.read_to_string(&mut code)?;
    let line = code
        .lines()
        .nth(line.saturating_sub(1) as usize)
        .unwrap_or("");
    Ok(line.to_string())
}

pub fn parse_and_normalize_url(uri: &str) -> ELSResult<Url> {
    Ok(Url::parse(&uri.replace("c%3A", "C:").to_lowercase())?)
}

pub fn normalize_url(url: Url) -> Url {
    Url::parse(&url.as_str().replace("c%3A", "C:").to_lowercase()).unwrap()
}

pub fn uri_to_path(uri: &Url) -> std::path::PathBuf {
    normalize_path(uri.to_file_path().unwrap())
}
