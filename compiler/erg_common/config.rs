//! defines a command-line parser for `ergc`.
//!
//! コマンドオプション(パーサー)を定義する
use std::env;
use std::env::consts::{ARCH, OS};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;

use crate::lazy::Lazy;
use crate::stdin;
use crate::Str;
use crate::{power_assert, read_file};

pub const SEMVER: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH_SHORT: &str = env!("GIT_HASH_SHORT");
pub const BUILD_DATE: &str = env!("BUILD_DATE");
/// TODO: タグを含める
pub const BUILD_INFO: Lazy<String> =
    Lazy::new(|| format!("(tags/?:{GIT_HASH_SHORT}, {BUILD_DATE}) on {ARCH}/{OS}"));

/// 入力はファイルからだけとは限らないので
/// Inputで操作を一本化する
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Input {
    /// filename
    File(Str),
    REPL,
    /// same content as cfg.command
    Pipe(Str),
    /// from command option | eval
    Str(Str),
    Dummy,
}

impl Input {
    pub fn enclosed_name(&self) -> &str {
        match self {
            Self::File(filename) => &filename[..],
            Self::REPL | Self::Pipe(_) => "<stdin>",
            Self::Str(_) => "<string>",
            Self::Dummy => "<dummy>",
        }
    }

    /// ファイルに書き出すとき使う
    pub fn filename(&self) -> &str {
        match self {
            Self::File(filename) => &filename[..],
            Self::REPL | Self::Pipe(_) => "stdin",
            Self::Str(_) => "string",
            Self::Dummy => "dummy",
        }
    }

    pub fn read(&self) -> Str {
        match self {
            Self::File(filename) => {
                let file = match File::open(&filename[..]) {
                    Ok(f) => f,
                    Err(e) => {
                        let code = e.raw_os_error().unwrap_or(1);
                        println!("cannot open '{filename}': [Errno {code}] {e}");
                        process::exit(code);
                    }
                };
                let src = match read_file(file) {
                    Ok(s) => s,
                    Err(e) => {
                        let code = e.raw_os_error().unwrap_or(1);
                        println!("cannot read '{filename}': [Errno {code}] {e}");
                        process::exit(code);
                    }
                };
                Str::from(src)
            }
            Self::Pipe(s) | Self::Str(s) => s.clone(),
            Self::REPL => stdin::read(),
            Self::Dummy => panic!("cannot read from a dummy file"),
        }
    }

    pub fn reread_lines(&self, ln_begin: usize, ln_end: usize) -> Vec<Str> {
        power_assert!(ln_begin, >=, 1);
        match self {
            Self::File(filename) => match File::open(&filename[..]) {
                Ok(file) => {
                    let mut codes = vec![];
                    let mut lines = BufReader::new(file).lines().skip(ln_begin - 1);
                    for _ in ln_begin..=ln_end {
                        codes.push(Str::from(lines.next().unwrap().unwrap()));
                    }
                    codes
                }
                Err(_) => vec!["<file not found>".into()],
            },
            Self::Pipe(s) | Self::Str(s) => s.split('\n').collect::<Vec<_>>()
                [ln_begin - 1..=ln_end - 1]
                .iter()
                .map(|s| Str::rc(*s))
                .collect(),
            Self::REPL => stdin::reread_lines(ln_begin, ln_end),
            Self::Dummy => panic!("cannot read lines from a dummy file"),
        }
    }

    pub fn reread(&self) -> Str {
        match self {
            Self::File(_filename) => todo!(),
            Self::Pipe(s) | Self::Str(s) => s.clone(),
            Self::REPL => stdin::reread(),
            Self::Dummy => panic!("cannot read from a dummy file"),
        }
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
    pub input: Input,
    pub module: &'static str,
    /// verbosity level for system messages.
    /// * 0: display errors
    /// * 1: display errors and warns
    /// * 2 (default): display errors, warnings and hints
    pub verbose: u8,
}

impl Default for ErgConfig {
    #[inline]
    fn default() -> Self {
        Self::new("exec", 1, false, None, Input::REPL, "<module>", 2)
    }
}

impl ErgConfig {
    pub const fn new(
        mode: &'static str,
        opt_level: u8,
        dump_as_pyc: bool,
        python_ver: Option<u32>,
        input: Input,
        module: &'static str,
        verbose: u8,
    ) -> Self {
        Self {
            mode,
            opt_level,
            dump_as_pyc,
            python_ver,
            input,
            module,
            verbose,
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
            match &arg[..] {
                "-c" => {
                    cfg.input = Input::Str(Str::from(args.next().unwrap()));
                }
                "--dump-as-pyc" => {
                    cfg.dump_as_pyc = true;
                }
                "-?" | "-h" | "--help" => {
                    println!("erg [option] ... [-c cmd | -m mod | file | -] [arg] ...");
                    // TODO:
                    process::exit(0);
                }
                "-m" => {
                    cfg.module = Box::leak(args.next().unwrap().into_boxed_str());
                }
                "--mode" => {
                    cfg.mode = Box::leak(args.next().unwrap().into_boxed_str());
                }
                "-o" | "--opt-level" | "--optimization-level" => {
                    cfg.opt_level = args.next().unwrap().parse::<u8>().unwrap();
                }
                "-p" | "--py-ver" | "--python-version" => {
                    cfg.python_ver = Some(args.next().unwrap().parse::<u32>().unwrap());
                }
                "--verbose\n" => {
                    cfg.verbose = args.next().unwrap().parse::<u8>().unwrap();
                }
                "-V" | "--version" => {
                    println!("Erg {}", env!("CARGO_PKG_VERSION"));
                    process::exit(0);
                }
                other if other.starts_with('-') => {
                    panic!("invalid option: {other}");
                }
                _ => {
                    cfg.input = Input::File(Str::from(arg));
                    break;
                }
            }
        }
        cfg
    }
}
