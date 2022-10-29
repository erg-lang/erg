//! defines a command-line parser for `ergc`.
//!
//! コマンドオプション(パーサー)を定義する
use std::env;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

use crate::help_messages::{command_message, mode_message};
use crate::python_util::PythonVersion;
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
                        codes.push(lines.next().unwrap_or_else(|| Ok("".to_string())).unwrap());
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

    pub fn local_resolve(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        let mut dir = if let Self::File(mut path) = self.clone() {
            path.pop();
            path
        } else {
            PathBuf::new()
        };
        dir.push(path);
        dir.set_extension("er");
        dir.canonicalize()
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
    pub no_std: bool,
    pub py_magic_num: Option<u32>,
    pub py_command: Option<&'static str>,
    pub target_version: Option<PythonVersion>,
    pub py_server_timeout: u64,
    pub quiet_startup: bool,
    pub show_type: bool,
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
            no_std: false,
            py_magic_num: None,
            py_command: None,
            target_version: None,
            py_server_timeout: 10,
            quiet_startup: false,
            show_type: false,
            input: Input::REPL,
            module: "<module>",
            verbose: 2,
            ps1: ">>> ",
            ps2: "... ",
        }
    }
}

impl ErgConfig {
    pub fn with_main_path(path: PathBuf) -> Self {
        Self {
            module: "<module>",
            input: Input::File(path),
            ..ErgConfig::default()
        }
    }

    pub fn with_module_path(path: PathBuf) -> Self {
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
            match &arg[..] {
                "-c" | "--code" => {
                    cfg.input = Input::Str(args.next().expect("the value of `-c` is not passed"));
                }
                "--check" => {
                    cfg.mode = "check";
                }
                "--compile" | "--dump-as-pyc" => {
                    cfg.mode = "compile";
                }
                "--no-std" => {
                    cfg.no_std = true;
                }
                "-?" | "-h" | "--help" => {
                    println!("{}", command_message());
                    if let "--mode" = args.next().as_ref().map(|s| &s[..]).unwrap_or("") {
                        println!("{}", mode_message());
                    }
                    process::exit(0);
                }
                "-m" | "--module" => {
                    let module = args
                        .next()
                        .expect("the value of `-m` is not passed")
                        .into_boxed_str();
                    cfg.module = Box::leak(module);
                }
                "--mode" => {
                    let mode = args.next().expect("the value of `--mode` is not passed");
                    if let "-?" | "-h" | "--help" = &mode[..] {
                        println!("{}", mode_message());
                        process::exit(0);
                    }
                    cfg.mode = Box::leak(mode.into_boxed_str());
                }
                "--ps1" => {
                    let ps1 = args
                        .next()
                        .expect("the value of `--ps1` is not passed")
                        .into_boxed_str();
                    cfg.ps1 = Box::leak(ps1);
                }
                "--ps2" => {
                    let ps2 = args
                        .next()
                        .expect("the value of `--ps2` is not passed")
                        .into_boxed_str();
                    cfg.ps2 = Box::leak(ps2);
                }
                "-o" | "--opt-level" | "--optimization-level" => {
                    cfg.opt_level = args
                        .next()
                        .expect("the value of `-o` is not passed")
                        .parse::<u8>()
                        .expect("the value of `-o` is not a number");
                }
                "--py-command" | "--python-command" => {
                    let py_command = args
                        .next()
                        .expect("the value of `--py-command` is not passed")
                        .parse::<String>()
                        .expect("the value of `-py-command` is not a valid Python command");
                    cfg.py_command = Some(Box::leak(py_command.into_boxed_str()));
                }
                "--py-magic-num" | "--python-magic-number" => {
                    let py_magic_num = args
                        .next()
                        .expect("the value of `--py-magic-num` is not passed")
                        .parse::<u32>()
                        .expect("the value of `--py-magic-num` is not a number");
                    cfg.py_magic_num = Some(py_magic_num);
                }
                "--py-server-timeout" => {
                    cfg.py_server_timeout = args
                        .next()
                        .expect("the value of `--py-server-timeout` is not passed")
                        .parse::<u64>()
                        .expect("the value of `--py-server-timeout` is not a number");
                }
                "--quiet-startup" => {
                    cfg.quiet_startup = true;
                }
                "-t" | "--show-type" => {
                    cfg.show_type = true;
                }
                "--target-version" => {
                    let target_version = args
                        .next()
                        .expect("the value of `--target-version` is not passed")
                        .parse::<PythonVersion>()
                        .expect("the value of `--target-version` is not a valid Python version");
                    cfg.target_version = Some(target_version);
                }
                "--verbose" => {
                    cfg.verbose = args
                        .next()
                        .expect("the value of `--verbose` is not passed")
                        .parse::<u8>()
                        .expect("the value of `--verbose` is not a number");
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
            use crate::tty::IsTty;
            let is_stdin_piped = !stdin().is_tty();
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
