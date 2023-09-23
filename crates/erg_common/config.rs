//! defines a command-line parser for `ergc`.
//!
//! コマンドオプション(パーサー)を定義する
use std::env;
use std::fmt;
use std::io::{stdin, IsTerminal, Read};
use std::path::PathBuf;
use std::process;
use std::str::FromStr;

use crate::help_messages::{command_message, mode_message, OPTIONS};
use crate::io::{Input, Output};
use crate::levenshtein::get_similar_name;
use crate::normalize_path;
use crate::python_util::{detect_magic_number, get_python_version, PythonVersion};
use crate::serialize::{get_magic_num_from_bytes, get_ver_from_magic_num};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErgMode {
    Lex,
    Parse,
    Desugar,
    TypeCheck,
    FullCheck,
    Compile,
    Transpile,
    Execute,
    LanguageServer,
    Read,
}

impl TryFrom<&str> for ErgMode {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, ()> {
        match s {
            "lex" | "lexer" => Ok(Self::Lex),
            "parse" | "parser" => Ok(Self::Parse),
            "desugar" | "desugarer" => Ok(Self::Desugar),
            "typecheck" | "lower" | "tc" => Ok(Self::TypeCheck),
            "fullcheck" | "check" | "checker" => Ok(Self::FullCheck),
            "comp" | "compile" | "compiler" => Ok(Self::Compile),
            "trans" | "transpile" | "transpiler" => Ok(Self::Transpile),
            "run" | "execute" => Ok(Self::Execute),
            "server" | "language-server" => Ok(Self::LanguageServer),
            "byteread" | "read" | "reader" | "dis" => Ok(Self::Read),
            _ => Err(()),
        }
    }
}

impl From<ErgMode> for &str {
    fn from(mode: ErgMode) -> Self {
        match mode {
            ErgMode::Lex => "lex",
            ErgMode::Parse => "parse",
            ErgMode::Desugar => "desugar",
            ErgMode::TypeCheck => "typecheck",
            ErgMode::FullCheck => "fullcheck",
            ErgMode::Compile => "compile",
            ErgMode::Transpile => "transpile",
            ErgMode::Execute => "execute",
            ErgMode::LanguageServer => "language-server",
            ErgMode::Read => "read",
        }
    }
}

impl fmt::Display for ErgMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", <&str>::from(*self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranspileTarget {
    Python,
    Json,
    Toml,
}

impl From<&str> for TranspileTarget {
    fn from(s: &str) -> Self {
        match s {
            "python" | "py" => Self::Python,
            "json" => Self::Json,
            "toml" => Self::Toml,
            _ => panic!("unsupported transpile target: {s}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErgConfig {
    pub mode: ErgMode,
    /// optimization level.
    /// * 0: no optimization
    /// * 1 (default): e.g. constant folding, dead code elimination
    /// * 2: e.g. static dispatching, inlining, peephole
    /// * 3: e.g. JIT compiling
    pub opt_level: u8,
    pub no_std: bool,
    pub py_magic_num: Option<u32>, // the magic number cannot be uniquely determined from `target_version`
    pub py_command: Option<&'static str>,
    pub target_version: Option<PythonVersion>,
    pub transpile_target: Option<TranspileTarget>,
    pub py_server_timeout: u64,
    pub quiet_repl: bool,
    pub show_type: bool,
    pub input: Input,
    pub output: Output,
    pub dist_dir: Option<&'static str>,
    /// module name to be executed
    pub module: &'static str,
    /// verbosity level for system messages.
    /// * 0: display errors, warns
    /// * 1 (default): display errors, warnings and hints
    pub verbose: u8,
    /// needed for `jupyter-erg`
    pub ps1: &'static str,
    pub ps2: &'static str,
    pub runtime_args: Vec<&'static str>,
}

impl Default for ErgConfig {
    #[inline]
    fn default() -> Self {
        Self {
            mode: ErgMode::Execute,
            opt_level: 1,
            no_std: false,
            py_magic_num: None,
            py_command: None,
            target_version: None,
            transpile_target: None,
            py_server_timeout: 10,
            quiet_repl: false,
            show_type: false,
            input: Input::repl(),
            output: Output::stdout(),
            dist_dir: None,
            module: "<module>",
            verbose: 1,
            ps1: ">>> ",
            ps2: "... ",
            runtime_args: vec![],
        }
    }
}

impl ErgConfig {
    pub fn with_main_path(path: PathBuf) -> Self {
        let path = normalize_path(path);
        Self {
            module: "<module>",
            input: Input::file(path),
            ..ErgConfig::default()
        }
    }

    pub fn string(src: String) -> Self {
        Self {
            input: Input::str(src),
            ..ErgConfig::default()
        }
    }

    /// clone alias (since the actual clone cost is low)
    #[inline]
    pub fn copy(&self) -> Self {
        self.clone()
    }

    pub fn dump_path(&self) -> PathBuf {
        if let Some(output) = &self.dist_dir {
            PathBuf::from(format!("{output}/{}", self.input.filename()))
        } else {
            self.input.full_path().to_path_buf()
        }
    }

    pub fn dump_filename(&self) -> String {
        if let Some(output) = &self.dist_dir {
            format!("{output}/{}", self.input.filename())
        } else {
            self.input.filename()
        }
    }

    pub fn dump_pyc_path(&self) -> PathBuf {
        let mut dump_path = self.dump_path();
        dump_path.set_extension("pyc");
        dump_path
    }

    pub fn dump_pyc_filename(&self) -> String {
        let dump_filename = self.dump_filename();
        if dump_filename.ends_with(".er") {
            dump_filename.replace(".er", ".pyc")
        } else {
            dump_filename + ".pyc"
        }
    }

    pub fn inherit(&self, path: PathBuf) -> Self {
        let path = normalize_path(path);
        Self {
            module: Box::leak(path.to_str().unwrap().to_string().into_boxed_str()),
            input: Input::file(path),
            ..self.copy()
        }
    }

    pub fn parse() -> Self {
        let mut args = env::args();
        args.next(); // "ergc"
        let mut cfg = Self::default();
        // not `for` because we need to consume the next argument
        while let Some(arg) = args.next() {
            match &arg[..] {
                /* Options */
                "--" => {
                    for arg in args {
                        cfg.runtime_args.push(Box::leak(arg.into_boxed_str()));
                    }
                    break;
                }
                "-c" | "--code" => {
                    cfg.input = Input::str(args.next().expect("the value of `-c` is not passed"));
                }
                "--check" => {
                    cfg.mode = ErgMode::FullCheck;
                }
                "--compile" | "--dump-as-pyc" => {
                    cfg.mode = ErgMode::Compile;
                }
                "--language-server" => {
                    cfg.mode = ErgMode::LanguageServer;
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
                    cfg.mode = ErgMode::try_from(&mode[..]).unwrap_or_else(|_| {
                        eprintln!("invalid mode: {mode}");
                        process::exit(1);
                    });
                }
                "--ping" => {
                    println!("pong");
                    process::exit(0);
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
                "--output-dir" | "--dest" | "--dist" | "--dest-dir" | "--dist-dir" => {
                    let output_dir = args
                        .next()
                        .expect("the value of `--output-dir` is not passed")
                        .into_boxed_str();
                    cfg.dist_dir = Some(Box::leak(output_dir));
                }
                "--py-command" | "--python-command" => {
                    let py_command = args
                        .next()
                        .expect("the value of `--py-command` is not passed")
                        .parse::<String>()
                        .expect("the value of `-py-command` is not a valid Python command");
                    cfg.py_magic_num = Some(detect_magic_number(&py_command));
                    cfg.target_version = Some(get_python_version(&py_command));
                    cfg.py_command = Some(Box::leak(py_command.into_boxed_str()));
                }
                "--hex-py-magic-num" | "--hex-python-magic-number" => {
                    let s_hex_magic_num = args
                        .next()
                        .expect("the value of `--hex-py-magic-num` is not passed");
                    let first_byte = u8::from_str_radix(&s_hex_magic_num[0..=1], 16).unwrap();
                    let second_byte = u8::from_str_radix(&s_hex_magic_num[2..=3], 16).unwrap();
                    let py_magic_num = get_magic_num_from_bytes(&[first_byte, second_byte, 0, 0]);
                    cfg.py_magic_num = Some(py_magic_num);
                    cfg.target_version = Some(get_ver_from_magic_num(py_magic_num));
                }
                "--py-magic-num" | "--python-magic-number" => {
                    let py_magic_num = args
                        .next()
                        .expect("the value of `--py-magic-num` is not passed")
                        .parse::<u32>()
                        .expect("the value of `--py-magic-num` is not a number");
                    cfg.py_magic_num = Some(py_magic_num);
                    cfg.target_version = Some(get_ver_from_magic_num(py_magic_num));
                }
                "--py-server-timeout" => {
                    cfg.py_server_timeout = args
                        .next()
                        .expect("the value of `--py-server-timeout` is not passed")
                        .parse::<u64>()
                        .expect("the value of `--py-server-timeout` is not a number");
                }
                "-q" | "--quiet-startup" | "--quiet-repl" => {
                    cfg.quiet_repl = true;
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
                "--transpile-target" | "--target" => {
                    let transpile_target = args
                        .next()
                        .expect("the value of `--transpile-target` is not passed")
                        .into_boxed_str();
                    cfg.transpile_target = Some(TranspileTarget::from(&transpile_target[..]));
                }
                "-v" | "--verbose" => {
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
                "--build-features" => {
                    #[cfg(feature = "debug")]
                    print!("debug ");
                    #[cfg(feature = "els")]
                    print!("els ");
                    #[cfg(feature = "py_compat")]
                    print!("py_compat ");
                    #[cfg(feature = "japanese")]
                    print!("japanese ");
                    #[cfg(feature = "simplified_chinese")]
                    print!("simplified_chinese ");
                    #[cfg(feature = "traditional_chinese")]
                    print!("traditional_chinese ");
                    #[cfg(feature = "unicode")]
                    print!("unicode ");
                    #[cfg(feature = "pretty")]
                    print!("pretty ");
                    #[cfg(feature = "large_thread")]
                    print!("large_thread");
                    println!();
                    process::exit(0);
                }
                other if other.starts_with('-') => {
                    if let Some(option) = get_similar_name(OPTIONS.iter().copied(), other) {
                        eprintln!("invalid option: {other} (did you mean `{option}`?)");
                    } else {
                        eprintln!("invalid option: {other}");
                    }
                    eprintln!(
                        "
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

    For more information try `erg --help`"
                    );
                    process::exit(2);
                }
                _ => {
                    if let Ok(mode) = ErgMode::try_from(&arg[..]) {
                        cfg.mode = mode;
                    } else {
                        let path = PathBuf::from_str(&arg[..])
                            .unwrap_or_else(|_| panic!("invalid file path: {arg}"));
                        let path = normalize_path(path);
                        cfg.input = Input::file(path);
                        match args.next().as_ref().map(|s| &s[..]) {
                            Some("--") => {
                                for arg in args {
                                    cfg.runtime_args.push(Box::leak(arg.into_boxed_str()));
                                }
                            }
                            Some(some) => {
                                println!("invalid argument: {some}");
                                println!("Do not pass options after the file path. If you want to pass runtime arguments, use `--` before them.");
                                process::exit(1);
                            }
                            _ => {}
                        }
                        break;
                    }
                }
            }
        }
        if cfg.input.is_repl() && cfg.mode != ErgMode::LanguageServer {
            let is_stdin_piped = !stdin().is_terminal();
            let input = if is_stdin_piped {
                let mut buffer = String::new();
                stdin().read_to_string(&mut buffer).unwrap();
                Input::pipe(buffer)
            } else {
                Input::repl()
            };
            cfg.input = input;
        }
        cfg
    }
}
