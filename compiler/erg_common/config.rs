//! defines a command-line parser for `ergc`.
//!
//! コマンドオプション(パーサー)を定義する
use std::env;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::process;

use crate::stdin::GLOBAL_STDIN;
use crate::{power_assert, read_file};

pub const SEMVER: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH_SHORT: &str = env!("GIT_HASH_SHORT");
pub const BUILD_DATE: &str = env!("BUILD_DATE");

/// 入力はファイルからだけとは限らないので
/// Inputで操作を一本化する
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Input {
    /// filename
    File(String),
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

    pub fn read(&self) -> String {
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
                match read_file(file) {
                    Ok(s) => s,
                    Err(e) => {
                        let code = e.raw_os_error().unwrap_or(1);
                        println!("cannot read '{filename}': [Errno {code}] {e}");
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
            Self::File(filename) => match File::open(&filename[..]) {
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
        let is_stdin_piped: bool = atty::isnt(atty::Stream::Stdin);
        let input = if is_stdin_piped {
            let mut buffer = String::new();
            stdin().read_to_string(&mut buffer).unwrap();
            Input::Pipe(buffer)
        } else {
            Input::REPL
        };
        Self {
            mode: "exec",
            opt_level: 1,
            dump_as_pyc: false,
            python_ver: None,
            py_server_timeout: 10,
            quiet_startup: false,
            input,
            module: "<module>",
            verbose: 2,
            ps1: ">>> ",
            ps2: "... ",
        }
    }
}

impl ErgConfig {
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
                    cfg.input = Input::Str(args.next().unwrap());
                }
                "--dump-as-pyc" => {
                    cfg.dump_as_pyc = true;
                }
                "-?" | "-h" | "--help" => {
                    println!("erg [option] ... [-c cmd | -m mod | file | -] [arg] ...");
                    println!("-c cmd : program passed in as string");
                    println!("-m mod : module to be executed");
                    println!("-?/-h  : show this help");
                    println!("--dump-as-pyc: dump as .pyc file");
                    println!("--mode lex|parse|compile|exec: execution mode");
                    println!("--opt-level/-o 0|1|2|3: optimization level");
                    println!("--python-version/-p (uint 32 number): Python version");
                    println!(
                        "--py-server-timeout (uint 64 number): timeout for the Python REPL server"
                    );
                    println!("--verbose 0|1|2: verbosity level");
                    println!("--version/-V: show version");
                    println!("file : program read from script file");
                    // TODO:
                    process::exit(0);
                }
                "-m" => {
                    cfg.module = Box::leak(args.next().unwrap().into_boxed_str());
                }
                "--mode" => {
                    cfg.mode = Box::leak(args.next().unwrap().into_boxed_str());
                }
                "--ps1" => {
                    cfg.ps1 = Box::leak(args.next().unwrap().into_boxed_str());
                }
                "--ps2" => {
                    cfg.ps2 = Box::leak(args.next().unwrap().into_boxed_str());
                }
                "-o" | "--opt-level" | "--optimization-level" => {
                    cfg.opt_level = args.next().unwrap().parse::<u8>().unwrap();
                }
                "-p" | "--py-ver" | "--python-version" => {
                    cfg.python_ver = Some(args.next().unwrap().parse::<u32>().unwrap());
                }
                "--py-server-timeout" => {
                    cfg.py_server_timeout = args.next().unwrap().parse::<u64>().unwrap();
                }
                "--quiet-startup" => {
                    cfg.quiet_startup = true;
                }
                "--verbose" => {
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
                    cfg.input = Input::File(arg);
                    break;
                }
            }
        }
        cfg
    }
}
