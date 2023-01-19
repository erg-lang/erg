use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process;

use crate::normalize_path;
use crate::stdin::GLOBAL_STDIN;
use crate::{power_assert, read_file};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DummyStdin {
    current_line: usize,
    lines: Vec<String>,
}

impl DummyStdin {
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            current_line: 0,
            lines,
        }
    }

    pub fn read_line(&mut self) -> Option<String> {
        if self.current_line >= self.lines.len() {
            return None;
        }
        let line = self.lines[self.current_line].clone();
        self.current_line += 1;
        Some(line)
    }

    pub fn reread_lines(&self, ln_begin: usize, ln_end: usize) -> Vec<String> {
        self.lines[ln_begin - 1..=ln_end - 1].to_vec()
    }

    pub fn reread(&self) -> Option<String> {
        self.lines.get(self.current_line).cloned()
    }
}

/// Since input is not always only from files
/// Unify operations with `Input`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Input {
    File(PathBuf),
    Url(String),
    REPL,
    DummyREPL(DummyStdin),
    /// same content as cfg.command
    Pipe(String),
    /// from command option | eval
    Str(String),
    Dummy,
}

impl Input {
    pub fn is_repl(&self) -> bool {
        matches!(self, Input::REPL | Input::DummyREPL(_))
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Input::File(path) => Some(path),
            _ => None,
        }
    }

    pub fn enclosed_name(&self) -> &str {
        match self {
            Self::File(filename) => filename.to_str().unwrap_or("_"),
            Self::Url(url) => url.as_str(),
            Self::REPL | Self::DummyREPL(_) | Self::Pipe(_) => "<stdin>",
            Self::Str(_) => "<string>",
            Self::Dummy => "<dummy>",
        }
    }

    pub fn full_path(&self) -> &str {
        match self {
            Self::File(filename) => filename.to_str().unwrap_or("_"),
            Self::Url(url) => url.as_str(),
            Self::REPL | Self::DummyREPL(_) | Self::Pipe(_) => "stdin",
            Self::Str(_) => "string",
            Self::Dummy => "dummy",
        }
    }

    pub fn file_stem(&self) -> &str {
        match self {
            Self::File(filename) => filename.file_stem().and_then(|f| f.to_str()).unwrap_or("_"),
            Self::Url(url) => url.as_str(),
            Self::REPL | Self::DummyREPL(_) | Self::Pipe(_) => "stdin",
            Self::Str(_) => "string",
            Self::Dummy => "dummy",
        }
    }

    pub fn filename(&self) -> &str {
        match self {
            Self::File(filename) => filename.file_name().and_then(|f| f.to_str()).unwrap_or("_"),
            Self::Url(url) => url.split('/').last().unwrap_or("_"),
            Self::REPL | Self::DummyREPL(_) | Self::Pipe(_) => "stdin",
            Self::Str(_) => "string",
            Self::Dummy => "dummy",
        }
    }

    pub fn read(&mut self) -> String {
        match self {
            Self::File(filename) => {
                let file = match File::open(&filename) {
                    Ok(f) => f,
                    Err(e) => {
                        let code = e.raw_os_error().unwrap_or(1);
                        let lossy = filename.to_str().unwrap().to_string();
                        println!("cannot open '{lossy}': [Errno {code}] {e}",);
                        process::exit(code);
                    }
                };
                match read_file(file) {
                    Ok(s) => s,
                    Err(e) => {
                        let code = e.raw_os_error().unwrap_or(1);
                        println!(
                            "cannot read '{}': [Errno {code}] {e}",
                            filename.to_string_lossy()
                        );
                        process::exit(code);
                    }
                }
            }
            #[cfg(feature = "input_url")]
            Self::Url(url) => {
                let resp = match minreq::get(&url[..]).send() {
                    Ok(r) => r,
                    Err(e) => {
                        println!("cannot open '{url}': {e}");
                        process::exit(1);
                    }
                };
                match resp.as_str() {
                    Ok(s) => s.to_string(),
                    Err(e) => {
                        println!("cannot read '{url}': {e}");
                        process::exit(1);
                    }
                }
            }
            #[cfg(not(feature = "input_url"))]
            Self::Url(_) => {
                println!("cannot open url: feature 'input_url' is not enabled");
                process::exit(1);
            }
            Self::Pipe(s) | Self::Str(s) => s.clone(),
            Self::REPL => GLOBAL_STDIN.read(),
            Self::DummyREPL(dummy) => dummy.read_line().unwrap_or_default(),
            Self::Dummy => panic!("cannot read from a dummy file"),
        }
    }

    pub fn read_non_dummy(&self) -> String {
        match self {
            Self::File(filename) => {
                let file = match File::open(filename) {
                    Ok(f) => f,
                    Err(e) => {
                        let code = e.raw_os_error().unwrap_or(1);
                        let lossy = filename.to_str().unwrap().to_string();
                        println!("cannot open '{lossy}': [Errno {code}] {e}",);
                        process::exit(code);
                    }
                };
                match read_file(file) {
                    Ok(s) => s,
                    Err(e) => {
                        let code = e.raw_os_error().unwrap_or(1);
                        println!(
                            "cannot read '{}': [Errno {code}] {e}",
                            filename.to_string_lossy()
                        );
                        process::exit(code);
                    }
                }
            }
            #[cfg(feature = "input_url")]
            Self::Url(url) => {
                let resp = match minreq::get(&url[..]).send() {
                    Ok(r) => r,
                    Err(e) => {
                        println!("cannot open '{url}': {e}");
                        process::exit(1);
                    }
                };
                match resp.as_str() {
                    Ok(s) => s.to_string(),
                    Err(e) => {
                        println!("cannot read '{url}': {e}");
                        process::exit(1);
                    }
                }
            }
            #[cfg(not(feature = "input_url"))]
            Self::Url(_) => {
                println!("cannot open url: feature 'input_url' is not enabled");
                process::exit(1);
            }
            Self::Pipe(s) | Self::Str(s) => s.clone(),
            Self::REPL => GLOBAL_STDIN.read(),
            Self::Dummy | Self::DummyREPL(_) => panic!("cannot read from a dummy file"),
        }
    }

    pub fn reread_lines(&self, ln_begin: usize, ln_end: usize) -> Vec<String> {
        power_assert!(ln_begin, >=, 1);
        match self {
            Self::File(filename) => match File::open(filename) {
                Ok(file) => {
                    let mut codes = vec![];
                    let mut lines = BufReader::new(file).lines().skip(ln_begin - 1);
                    for _ in ln_begin..=ln_end {
                        codes.push(lines.next().unwrap_or_else(|| Ok("".to_string())).unwrap());
                    }
                    codes
                }
                Err(_) => vec!["<file not found>".into()],
            },
            Self::Url(_) => todo!(),
            Self::Pipe(s) | Self::Str(s) => s.split('\n').collect::<Vec<_>>()
                [ln_begin - 1..=ln_end - 1]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            Self::REPL => GLOBAL_STDIN.reread_lines(ln_begin, ln_end),
            Self::DummyREPL(dummy) => dummy.reread_lines(ln_begin, ln_end),
            Self::Dummy => panic!("cannot read lines from a dummy file"),
        }
    }

    pub fn reread(&self) -> String {
        match self {
            Self::File(_filename) => todo!(),
            Self::Url(_) => todo!(),
            Self::Pipe(s) | Self::Str(s) => s.clone(),
            Self::REPL => GLOBAL_STDIN.reread().trim_end().to_owned(),
            Self::DummyREPL(dummy) => dummy.reread().unwrap_or_default(),
            Self::Dummy => panic!("cannot read from a dummy file"),
        }
    }

    pub fn local_resolve(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        let mut dir = if let Self::File(mut path) = self.clone() {
            path.pop();
            path
        } else {
            PathBuf::new()
        };
        dir.push(path);
        dir.set_extension("er");
        let path = dir
            .canonicalize()
            .or_else(|_| {
                dir.pop();
                dir.push(path);
                dir.push("__init__.er"); // {path}/__init__.er
                dir.canonicalize()
            })
            .or_else(|_| {
                dir.pop(); // {path}
                dir.set_extension("d.er");
                dir.canonicalize()
            })
            .or_else(|_| {
                dir.pop(); // {path}.d.er
                dir.push(format!("{}.d", path.display())); // {path}.d
                dir.push("__init__.d.er"); // {path}.d/__init__.d.er
                dir.canonicalize()
            })
            .or_else(|_| {
                dir.pop(); // {path}.d
                dir.pop();
                dir.push("__pycache__");
                dir.push(path);
                dir.set_extension("d.er"); // __pycache__/{path}.d.er
                dir.canonicalize()
            })?;
        Ok(normalize_path(path))
    }

    pub fn local_py_resolve(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        let mut dir = if let Self::File(mut path) = self.clone() {
            path.pop();
            path
        } else {
            PathBuf::new()
        };
        dir.push(path);
        dir.set_extension("py");
        let path = dir.canonicalize().or_else(|_| {
            dir.pop();
            dir.push(path);
            dir.push("__init__.py"); // {path}/__init__.er
            dir.canonicalize()
        })?;
        Ok(normalize_path(path))
    }
}
