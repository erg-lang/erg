//! defines a command-line parser for `ergc`.
//!
//! コマンドオプション(パーサー)を定義する
use std::env;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

use crate::help_messages::{command_message, mode_message, OPTIONS};
use crate::levenshtein::get_similar_name;
use crate::normalize_path;
use crate::python_util::{detect_magic_number, get_python_version, PythonVersion};
use crate::random::random;
use crate::serialize::{get_magic_num_from_bytes, get_ver_from_magic_num};
use crate::stdin::GLOBAL_STDIN;
use crate::{power_assert, read_file};

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
            "typecheck" | "lower" => Ok(Self::TypeCheck),
            "fullcheck" | "check" | "checker" => Ok(Self::FullCheck),
            "compile" | "compiler" => Ok(Self::Compile),
            "transpile" | "transpiler" => Ok(Self::Transpile),
            "execute" => Ok(Self::Execute),
            "language-server" => Ok(Self::LanguageServer),
            "byteread" | "read" | "reader" => Ok(Self::Read),
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DummyStdin {
    pub name: String,
    current_line: usize,
    lines: Vec<String>,
}

impl DummyStdin {
    pub fn new(name: String, lines: Vec<String>) -> Self {
        Self {
            name,
            current_line: 0,
            lines,
        }
    }

    pub fn read_line(&mut self) -> String {
        let mut stdout = std::io::stdout();
        if self.current_line >= self.lines.len() {
            stdout.write_all("\n".as_bytes()).unwrap();
            stdout.flush().unwrap();
            return "".to_string();
        }
        let mut line = self.lines[self.current_line].clone();
        self.current_line += 1;
        line.push('\n');
        stdout.write_all(line.as_bytes()).unwrap();
        stdout.flush().unwrap();
        line
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
    REPL(u64),
    DummyREPL(DummyStdin),
    /// same content as cfg.command
    Pipe(u64, String),
    /// from command option | eval
    Str(u64, String),
    Dummy,
}

impl From<PathBuf> for Input {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

impl From<&Path> for Input {
    fn from(path: &Path) -> Self {
        Self::File(path.to_path_buf())
    }
}

impl Input {
    pub const fn is_repl(&self) -> bool {
        matches!(self, Input::REPL(_) | Input::DummyREPL(_))
    }

    pub fn pipe(src: String) -> Self {
        Self::Pipe(random(), src)
    }

    pub fn str(src: String) -> Self {
        Self::Str(random(), src)
    }

    pub fn repl() -> Self {
        Self::REPL(random())
    }
    pub fn lineno(&self) -> usize {
        GLOBAL_STDIN.lineno()
    }

    pub fn block_begin(&self) -> usize {
        GLOBAL_STDIN.block_begin()
    }

    pub fn set_block_begin(&self) {
        GLOBAL_STDIN.set_block_begin(self.lineno())
    }

    pub fn insert_whitespace(&self, whitespace: &str) {
        GLOBAL_STDIN.insert_whitespace(whitespace);
    }

    pub fn set_indent(&self, indent: usize) {
        GLOBAL_STDIN.set_indent(indent);
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Input::File(path) => Some(path),
            _ => None,
        }
    }

    pub const fn id(&self) -> u64 {
        match self {
            Input::File(_) | Input::DummyREPL(_) | Input::Dummy => 0,
            Input::REPL(id) | Input::Pipe(id, _) | Input::Str(id, _) => *id,
        }
    }

    pub fn enclosed_name(&self) -> &str {
        match self {
            Self::File(filename) => filename.to_str().unwrap_or("_"),
            Self::REPL(_) | Self::DummyREPL(_) | Self::Pipe(_, _) => "<stdin>",
            Self::Str(_, _) => "<string>",
            Self::Dummy => "<dummy>",
        }
    }

    pub fn full_path(&self) -> PathBuf {
        match self {
            Self::File(filename) => filename.clone(),
            Self::REPL(id) | Self::Pipe(id, _) => PathBuf::from(format!("stdin_{id}")),
            Self::DummyREPL(dummy) => PathBuf::from(format!("stdin_{}", dummy.name)),
            Self::Str(id, _) => PathBuf::from(format!("string_{id}")),
            Self::Dummy => PathBuf::from("dummy"),
        }
    }

    pub fn file_stem(&self) -> String {
        match self {
            Self::File(filename) => filename
                .file_stem()
                .and_then(|f| f.to_str())
                .unwrap_or("_")
                .to_string(),
            Self::REPL(id) | Self::Pipe(id, _) => format!("stdin_{id}"),
            Self::DummyREPL(stdin) => format!("stdin_{}", stdin.name),
            Self::Str(id, _) => format!("string_{id}"),
            Self::Dummy => "dummy".to_string(),
        }
    }

    pub fn filename(&self) -> String {
        match self {
            Self::File(filename) => filename
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("_")
                .to_string(),
            Self::REPL(id) | Self::Pipe(id, _) => format!("stdin_{id}"),
            Self::DummyREPL(stdin) => format!("stdin_{}", stdin.name),
            Self::Str(id, _) => format!("string_{id}"),
            Self::Dummy => "dummy".to_string(),
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
            Self::Pipe(_, s) | Self::Str(_, s) => s.clone(),
            Self::REPL(_) => GLOBAL_STDIN.read(),
            Self::DummyREPL(dummy) => dummy.read_line(),
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
            Self::Pipe(_, s) | Self::Str(_, s) => s.clone(),
            Self::REPL(_) => GLOBAL_STDIN.read(),
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
            Self::Pipe(_, s) | Self::Str(_, s) => s.split('\n').collect::<Vec<_>>()
                [ln_begin - 1..=ln_end - 1]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            Self::REPL(_) => {
                if ln_begin == ln_end {
                    vec![GLOBAL_STDIN.reread()]
                } else {
                    GLOBAL_STDIN.reread_lines(ln_begin, ln_end)
                }
            }
            Self::DummyREPL(dummy) => dummy.reread_lines(ln_begin, ln_end),
            Self::Dummy => panic!("cannot read lines from a dummy file"),
        }
    }

    pub fn reread(&self) -> String {
        match self {
            Self::File(_filename) => todo!(),
            Self::Pipe(_, s) | Self::Str(_, s) => s.clone(),
            Self::REPL(_) => GLOBAL_STDIN.reread().trim_end().to_owned(),
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
    pub py_server_timeout: u64,
    pub quiet_repl: bool,
    pub show_type: bool,
    pub input: Input,
    pub output_dir: Option<&'static str>,
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
            py_server_timeout: 10,
            quiet_repl: false,
            show_type: false,
            input: Input::repl(),
            output_dir: None,
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
            input: Input::File(path),
            ..ErgConfig::default()
        }
    }

    /// clone alias (since the actual clone cost is low)
    #[inline]
    pub fn copy(&self) -> Self {
        self.clone()
    }

    pub fn dump_path(&self) -> PathBuf {
        if let Some(output) = &self.output_dir {
            PathBuf::from(format!("{output}/{}", self.input.filename()))
        } else {
            self.input.full_path()
        }
    }

    pub fn dump_filename(&self) -> String {
        if let Some(output) = &self.output_dir {
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
            input: Input::File(path),
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
                "--output-dir" | "--dest" => {
                    let output_dir = args
                        .next()
                        .expect("the value of `--output-dir` is not passed")
                        .into_boxed_str();
                    cfg.output_dir = Some(Box::leak(output_dir));
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
                "--quiet-startup" | "--quiet-repl" => {
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
                "--build-features" => {
                    #[cfg(feature = "debug")]
                    print!("debug ");
                    #[cfg(feature = "els")]
                    print!("els ");
                    #[cfg(feature = "py_compatible")]
                    print!("py_compatible ");
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
                    let path = PathBuf::from_str(&arg[..])
                        .unwrap_or_else(|_| panic!("invalid file path: {arg}"));
                    let path = normalize_path(path);
                    cfg.input = Input::File(path);
                    if let Some("--") = args.next().as_ref().map(|s| &s[..]) {
                        for arg in args {
                            cfg.runtime_args.push(Box::leak(arg.into_boxed_str()));
                        }
                    }
                    break;
                }
            }
        }
        if cfg.input.is_repl() && cfg.mode != ErgMode::LanguageServer {
            use crate::tty::IsTty;
            let is_stdin_piped = !stdin().is_tty();
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
