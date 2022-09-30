//! defines a command-line parser for `ergc`.
//!
//! コマンドオプション(パーサー)を定義する
use std::env;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

use crate::help_messages::{CMD_HELP, MODE_HELP};
use crate::stdin::GLOBAL_STDIN;
use crate::{power_assert, read_file};

pub const SEMVER: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH_SHORT: &str = env!("GIT_HASH_SHORT");
pub const BUILD_DATE: &str = env!("BUILD_DATE");

/// 入力はファイルからだけとは限らないので
/// Inputで操作を一本化する
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Input {
    File(PathBuf),
    REPL,
    /// same content as cfg.command
    Pipe(String),
    /// from command option | eval
    Str(String),
    Dummy,
}

impl Input {
    pub fn is_repl(&self) -> bool {
        matches!(self, Input::REPL)
    }

    pub fn enclosed_name(&self) -> &str {
        match self {
            Self::File(filename) => filename.to_str().unwrap_or("???"),
            Self::REPL | Self::Pipe(_) => "<stdin>",
            Self::Str(_) => "<string>",
            Self::Dummy => "<dummy>",
        }
    }

    /// ファイルに書き出すとき使う
    pub fn filename(&self) -> &str {
        match self {
            Self::File(filename) => filename.to_str().unwrap_or("???"),
            Self::REPL | Self::Pipe(_) => "stdin",
            Self::Str(_) => "string",
            Self::Dummy => "dummy",
        }
    }

    pub fn read(&self) -> String {
        match self {
            Self::File(filename) => {
                let file = match File::open(filename) {
                    Ok(f) => f,
                    Err(e) => {
                        let code = e.raw_os_error().unwrap_or(1);
                        println!(
                            "cannot open '{}': [Errno {code}] {e}",
                            filename.to_string_lossy()
                        );
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
            Self::Pipe(s) | Self::Str(s) => s.clone(),
            Self::REPL => GLOBAL_STDIN.read(),
            Self::Dummy => panic!("cannot read from a dummy file"),
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
                        codes.push(lines.next().unwrap().unwrap());
                    }
                    codes
                }
                Err(_) => vec!["<file not found>".into()],
            },
            Self::Pipe(s) | Self::Str(s) => s.split('\n').collect::<Vec<_>>()
                [ln_begin - 1..=ln_end - 1]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            Self::REPL => GLOBAL_STDIN.reread_lines(ln_begin, ln_end),
            Self::Dummy => panic!("cannot read lines from a dummy file"),
        }
    }

    pub fn reread(&self) -> String {
        match self {
            Self::File(_filename) => todo!(),
            Self::Pipe(s) | Self::Str(s) => s.clone(),
            Self::REPL => GLOBAL_STDIN.reread().trim_end().to_owned(),
            Self::Dummy => panic!("cannot read from a dummy file"),
        }
    }

    pub fn resolve(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        let mut dir = if let Self::File(mut path) = self.clone() {
            path.pop();
            path
        } else {
            PathBuf::new()
        };
        dir.push(path);
        dir.set_extension("er");
        dir.canonicalize().or_else(|_| {
            dir.set_extension("d.er");
            dir.canonicalize()
        })
    }
}

#[derive(Debug, Clone)]
pub struct ErgConfig {
    /// options: lex | parse | compile | exec
    pub mode: &'static str,
    /// optimization level.
    /// * 0: no optimization
    /// * 1 (default): e.g. constant folding, dead code elimination
    /// * 2: e.g. static dispatching, inlining, peephole
    /// * 3: e.g. JIT compiling
    pub opt_level: u8,
    pub dump_as_pyc: bool,
    pub python_ver: Option<u32>,
    pub py_server_timeout: u64,
    pub quiet_startup: bool,
    pub input: Input,
    /// module name to be executed
    pub module: &'static str,
    /// verbosity level for system messages.
    /// * 0: display errors
    /// * 1: display errors and warns
    /// * 2 (default): display errors, warnings and hints
    pub verbose: u8,
    /// needed for `jupyter-erg`
    pub ps1: &'static str,
    pub ps2: &'static str,
}

impl Default for ErgConfig {
    #[inline]
    fn default() -> Self {
        Self {
            mode: "exec",
            opt_level: 1,
            dump_as_pyc: false,
            python_ver: None,
            py_server_timeout: 10,
            quiet_startup: false,
            input: Input::REPL,
            module: "<module>",
            verbose: 2,
            ps1: ">>> ",
            ps2: "... ",
        }
    }
}

impl ErgConfig {
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            module: Box::leak(path.to_str().unwrap().to_string().into_boxed_str()),
            input: Input::File(path),
            ..ErgConfig::default()
        }
    }

    /// cloneのエイリアス(実際のcloneコストは低いので)
    #[inline]
    pub fn copy(&self) -> Self {
        self.clone()
    }

    pub fn parse() -> Self {
        let mut args = env::args();
        args.next(); // "ergc"
        let mut cfg = Self::default();
        // ループ内でnextするのでforにしないこと
        while let Some(arg) = args.next() {
            let next_arg = args.next();
            match &arg[..] {
                "-c" if next_arg.is_some() => {
                    cfg.input = Input::Str(next_arg.unwrap());
                }
                "--dump-as-pyc" => {
                    cfg.dump_as_pyc = true;
                }
                "-?" | "-h" | "--help" => {
                    // TODO:
                    println!("{}", CMD_HELP);
                    process::exit(0);
                }
                "-m" if next_arg.is_some() => {
                    cfg.module = Box::leak(next_arg.unwrap().into_boxed_str());
                }
                "--mode" if next_arg.is_some() => {
                    let mode = next_arg.unwrap();
                    if let "-?" | "-h" | "--help" = &mode[..] {
                        println!("{}", MODE_HELP);
                        process::exit(0);
                    }
                    cfg.mode = Box::leak(mode.into_boxed_str());
                }
                "--ps1" if next_arg.is_some() => {
                    cfg.ps1 = Box::leak(next_arg.unwrap().into_boxed_str());
                }
                "--ps2" if next_arg.is_some() => {
                    cfg.ps2 = Box::leak(next_arg.unwrap().into_boxed_str());
                }
                "-o" | "--opt-level" | "--optimization-level" if next_arg.is_some() => {
                    cfg.opt_level = next_arg.unwrap().parse::<u8>().unwrap();
                }
                "-p" | "--py-ver" | "--python-version" if next_arg.is_some() => {
                    if let Ok(ver) = next_arg.unwrap().parse::<u32>() {
                        cfg.python_ver = Some(ver)
                    }
                }
                "--py-server-timeout" if next_arg.is_some() => {
                    if let Ok(time) = next_arg.unwrap().parse::<u64>() {
                        cfg.py_server_timeout = time;
                    }
                }
                "--quiet-startup" => {
                    cfg.quiet_startup = true;
                }
                "--verbose" if next_arg.is_some() => {
                    if let Ok(vr) = next_arg.unwrap().parse::<u8>() {
                        cfg.verbose = vr;
                    }
                }
                "-V" | "--version" => {
                    println!("Erg {}", env!("CARGO_PKG_VERSION"));
                    process::exit(0);
                }
                other if other.starts_with('-') => {
                    panic!("invalid option: {other}");
                }
                _ => {
                    cfg.input = Input::File(
                        PathBuf::from_str(&arg[..])
                            .unwrap_or_else(|_| panic!("invalid file path: {}", arg)),
                    );
                    break;
                }
            }
        }
        if cfg.input == Input::REPL {
            let is_stdin_piped = atty::isnt(atty::Stream::Stdin);
            let input = if is_stdin_piped {
                let mut buffer = String::new();
                stdin().read_to_string(&mut buffer).unwrap();
                Input::Pipe(buffer)
            } else {
                Input::REPL
            };
            cfg.input = input;
        }
        cfg
    }
}
