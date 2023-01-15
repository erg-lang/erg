use std::fs::File;
use std::io::Read;
use std::path::Path;

use erg_common::normalize_path;
use erg_common::traits::{DequeStream, Locational};

use erg_compiler::erg_parser::lex::Lexer;
use erg_compiler::erg_parser::token::{Token, TokenStream};

use lsp_types::{Position, Range, Url};

use crate::server::ELSResult;

pub fn loc_to_range(loc: erg_common::error::Location) -> Option<Range> {
    let start = Position::new(loc.ln_begin()? - 1, loc.col_begin()?);
    let end = Position::new(loc.ln_end()? - 1, loc.col_end()?);
    Some(Range::new(start, end))
}

pub fn pos_in_loc<L: Locational>(loc: &L, pos: Position) -> bool {
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

pub fn get_token_stream(uri: Url) -> ELSResult<TokenStream> {
    let mut code = String::new();
    let path = uri.to_file_path().unwrap();
    File::open(path.as_path())?.read_to_string(&mut code)?;
    Ok(Lexer::from_str(code).lex()?)
}

pub fn get_token_from_stream(stream: &TokenStream, pos: Position) -> ELSResult<Option<Token>> {
    for token in stream.iter() {
        if pos_in_loc(token, pos) {
            return Ok(Some(token.clone()));
        }
    }
    Ok(None)
}

pub fn get_token_index(uri: Url, pos: Position) -> ELSResult<Option<usize>> {
    let mut timeout = 300;
    let path = uri.to_file_path().unwrap();
    loop {
        let mut code = String::new();
        File::open(path.as_path())?.read_to_string(&mut code)?;
        if let Ok(tokens) = Lexer::from_str(code).lex() {
            for (i, tok) in tokens.iter().enumerate() {
                if pos_in_loc(tok, pos) {
                    return Ok(Some(i));
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        timeout -= 1;
        if timeout == 0 {
            return Ok(None);
        }
    }
}

pub fn get_token(uri: Url, pos: Position) -> ELSResult<Option<Token>> {
    let index = get_token_index(uri.clone(), pos)?;
    if let Some(idx) = index {
        Ok(get_token_stream(uri)?.get(idx).cloned())
    } else {
        Ok(None)
    }
}

/// plus_minus: 0 => same as get_token
pub fn get_token_relatively(
    uri: Url,
    pos: Position,
    plus_minus: isize,
) -> ELSResult<Option<Token>> {
    let index = get_token_index(uri.clone(), pos)?;
    if let Some(idx) = index {
        Ok(get_token_stream(uri)?
            .get((idx as isize + plus_minus) as usize)
            .cloned())
    } else {
        Ok(None)
    }
}

pub fn get_code_from_uri(uri: &Url) -> ELSResult<String> {
    let path = uri.to_file_path().unwrap();
    let mut code = String::new();
    File::open(path.as_path())?.read_to_string(&mut code)?;
    Ok(code)
}

pub fn get_line_from_uri(uri: &Url, line: u32) -> ELSResult<String> {
    let code = get_code_from_uri(uri)?;
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
