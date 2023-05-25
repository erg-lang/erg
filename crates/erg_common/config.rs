//! defines a command-line parser for `ergc`.
//!
//! コマンドオプション(パーサー)を定義する
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

use crate::consts::ERG_MODE;
use crate::env::{erg_py_external_lib_path, erg_pystd_path, erg_std_path, python_site_packages};
use crate::help_messages::{command_message, mode_message, OPTIONS};
use crate::levenshtein::get_similar_name;
use crate::pathutil::add_postfix_foreach;
use crate::python_util::{detect_magic_number, get_python_version, get_sys_path, PythonVersion};
use crate::random::random;
use crate::serialize::{get_magic_num_from_bytes, get_ver_from_magic_num};
use crate::stdin::GLOBAL_STDIN;
use crate::{normalize_path, power_assert, read_file};

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
            "compile" | "compiler" => Ok(Self::Compile),
            "transpile" | "transpiler" => Ok(Self::Transpile),
            "run" | "execute" => Ok(Self::Execute),
            "server" | "language-server" => Ok(Self::LanguageServer),
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
            // workaround: https://github.com/erg-lang/erg/issues/399
            return "exit()".to_string();
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputKind {
    File(PathBuf),
    REPL,
    DummyREPL(DummyStdin),
    /// same content as cfg.command
    Pipe(String),
    /// from command option | eval
    Str(String),
    Dummy,
}

impl InputKind {
    pub const fn is_repl(&self) -> bool {
        matches!(self, Self::REPL | Self::DummyREPL(_))
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            _ => None,
        }
    }

    pub fn enclosed_name(&self) -> &str {
        match self {
            Self::File(filename) => filename.to_str().unwrap_or("_"),
            Self::REPL | Self::DummyREPL(_) | Self::Pipe(_) => "<stdin>",
            Self::Str(_) => "<string>",
            Self::Dummy => "<dummy>",
        }
    }

    pub fn dir(&self) -> PathBuf {
        if let Self::File(path) = self {
            let mut path = path.clone();
            path.pop();
            path
        } else {
            PathBuf::new()
        }
    }
}

/// Since input is not always only from files
/// Unify operations with `Input`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Input {
    pub(crate) kind: InputKind,
    /// Unique id to avoid file name collision
    id: u64,
}

impl From<PathBuf> for Input {
    fn from(path: PathBuf) -> Self {
        Self::file(path)
    }
}

impl From<&Path> for Input {
    fn from(path: &Path) -> Self {
        Self::file(path.to_path_buf())
    }
}

impl Input {
    pub const fn new(kind: InputKind, id: u64) -> Self {
        Self { kind, id }
    }

    pub fn file(path: PathBuf) -> Self {
        Self::new(InputKind::File(path), random())
    }

    pub fn pipe(src: String) -> Self {
        Self::new(InputKind::Pipe(src), random())
    }

    pub fn str(src: String) -> Self {
        Self::new(InputKind::Str(src), random())
    }

    pub fn repl() -> Self {
        Self::new(InputKind::REPL, random())
    }

    pub fn dummy() -> Self {
        Self::new(InputKind::Dummy, random())
    }

    pub fn dummy_repl(stdin: DummyStdin) -> Self {
        Self::new(InputKind::DummyREPL(stdin), random())
    }

    pub const fn is_repl(&self) -> bool {
        self.kind.is_repl()
    }

    pub const fn id(&self) -> u64 {
        self.id
    }

    pub fn path(&self) -> Option<&Path> {
        self.kind.path()
    }

    pub fn dir(&self) -> PathBuf {
        self.kind.dir()
    }

    pub fn enclosed_name(&self) -> &str {
        self.kind.enclosed_name()
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

    pub fn file_stem(&self) -> String {
        match &self.kind {
            InputKind::File(filename) => format!(
                "{}_{}",
                filename.file_stem().and_then(|f| f.to_str()).unwrap_or("_"),
                self.id
            ),
            InputKind::REPL | InputKind::Pipe(_) => format!("stdin_{}", self.id),
            InputKind::DummyREPL(stdin) => format!("stdin_{}_{}", stdin.name, self.id),
            InputKind::Str(_) => format!("string_{}", self.id),
            InputKind::Dummy => "dummy".to_string(),
        }
    }

    pub fn full_path(&self) -> PathBuf {
        match &self.kind {
            InputKind::File(filename) => {
                PathBuf::from(format!("{}_{}", filename.display(), self.id))
            }
            _ => PathBuf::from(self.file_stem()),
        }
    }

    pub fn filename(&self) -> String {
        match &self.kind {
            InputKind::File(filename) => format!(
                "{}_{}",
                filename.file_name().and_then(|f| f.to_str()).unwrap_or("_"),
                self.id
            ),
            _ => self.file_stem(),
        }
    }

    pub fn unescaped_file_stem(&self) -> &str {
        match &self.kind {
            InputKind::File(filename) => {
                filename.file_stem().and_then(|f| f.to_str()).unwrap_or("_")
            }
            InputKind::REPL | InputKind::Pipe(_) => "stdin",
            InputKind::DummyREPL(_stdin) => "stdin",
            InputKind::Str(_) => "string",
            InputKind::Dummy => "dummy",
        }
    }

    pub fn unescaped_filename(&self) -> &str {
        match &self.kind {
            InputKind::File(filename) => {
                filename.file_name().and_then(|f| f.to_str()).unwrap_or("_")
            }
            InputKind::REPL | InputKind::Pipe(_) => "stdin",
            InputKind::DummyREPL(_stdin) => "stdin",
            InputKind::Str(_) => "string",
            InputKind::Dummy => "dummy",
        }
    }

    pub fn unescaped_path(&self) -> &Path {
        match &self.kind {
            InputKind::File(filename) => filename.as_path(),
            InputKind::REPL | InputKind::Pipe(_) => Path::new("stdin"),
            InputKind::DummyREPL(_stdin) => Path::new("stdin"),
            InputKind::Str(_) => Path::new("string"),
            InputKind::Dummy => Path::new("dummy"),
        }
    }

    pub fn read(&mut self) -> String {
        match &mut self.kind {
            InputKind::File(filename) => {
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
            InputKind::Pipe(s) | InputKind::Str(s) => s.clone(),
            InputKind::REPL => GLOBAL_STDIN.read(),
            InputKind::DummyREPL(dummy) => dummy.read_line(),
            InputKind::Dummy => panic!("cannot read from a dummy file"),
        }
    }

    pub fn try_read(&mut self) -> std::io::Result<String> {
        match &mut self.kind {
            InputKind::File(filename) => {
                let file = File::open(filename)?;
                read_file(file)
            }
            InputKind::Pipe(s) | InputKind::Str(s) => Ok(s.clone()),
            InputKind::REPL => Ok(GLOBAL_STDIN.read()),
            InputKind::DummyREPL(dummy) => Ok(dummy.read_line()),
            InputKind::Dummy => panic!("cannot read from a dummy file"),
        }
    }

    pub fn read_non_dummy(&self) -> String {
        match &self.kind {
            InputKind::File(filename) => {
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
            InputKind::Pipe(s) | InputKind::Str(s) => s.clone(),
            InputKind::REPL => GLOBAL_STDIN.read(),
            InputKind::Dummy | InputKind::DummyREPL(_) => panic!("cannot read from a dummy file"),
        }
    }

    pub fn reread_lines(&self, ln_begin: usize, ln_end: usize) -> Vec<String> {
        power_assert!(ln_begin, >=, 1);
        match &self.kind {
            InputKind::File(filename) => match File::open(filename) {
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
            InputKind::Pipe(s) | InputKind::Str(s) => s.split('\n').collect::<Vec<_>>()
                [ln_begin - 1..=ln_end - 1]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            InputKind::REPL => {
                if ln_begin == ln_end {
                    vec![GLOBAL_STDIN.reread()]
                } else {
                    GLOBAL_STDIN.reread_lines(ln_begin, ln_end)
                }
            }
            InputKind::DummyREPL(dummy) => dummy.reread_lines(ln_begin, ln_end),
            InputKind::Dummy => panic!("cannot read lines from a dummy file"),
        }
    }

    pub fn reread(&self) -> String {
        match &self.kind {
            InputKind::File(path) => {
                let mut reader = BufReader::new(File::open(path).unwrap());
                let mut buf = String::new();
                reader.read_to_string(&mut buf).unwrap();
                buf
            }
            InputKind::Pipe(s) | InputKind::Str(s) => s.clone(),
            InputKind::REPL => GLOBAL_STDIN.reread().trim_end().to_owned(),
            InputKind::DummyREPL(dummy) => dummy.reread().unwrap_or_default(),
            InputKind::Dummy => panic!("cannot read from a dummy file"),
        }
    }

    pub fn sys_path(&self) -> Vec<PathBuf> {
        get_sys_path(self.unescaped_path().parent())
    }

    /// resolution order:
    /// 1. `{path/to}.er`
    /// 2. `{path/to}/__init__.er`
    fn resolve_local(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        let mut dir = self.dir();
        dir.push(path);
        dir.set_extension("er"); // {path/to}.er
        let path = dir.canonicalize().or_else(|_| {
            dir.pop(); // {path}
            dir.push(path.iter().last().unwrap_or_default()); // {path/to}
            dir.push("__init__.er"); // -> {path/to}/__init__.er
            dir.canonicalize()
        })?;
        Ok(normalize_path(path))
    }

    fn resolve_local_decl(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        self._resolve_local_decl(path).or_else(|_| {
            let path = add_postfix_foreach(path, ".d");
            self._resolve_local_decl(&path)
        })
    }

    /// resolution order:
    /// 1. `{path/to}.d.er`
    /// 2. `{path/to}/__init__.d.er`
    /// 3. `{path}/__pycache__/{to}.d.er`
    /// 4. `{path/to}/__pycache__/__init__.d.er`
    fn _resolve_local_decl(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        let mut dir = self.dir();
        let mut comps = path.components();
        let last = comps
            .next_back()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "path is empty"))?;
        let last_path = Path::new(&last);
        dir.push(comps);
        dir.push(last_path);
        dir.set_extension("d.er"); // {path/to}.d.er
        let path = dir
            .canonicalize()
            .or_else(|_| {
                dir.pop(); // {path/to}.d.er -> {path}
                dir.push(last_path); // -> {path/to}
                dir.push("__init__.d.er"); // -> {path/to}/__init__.d.er
                dir.canonicalize()
            })
            .or_else(|_| {
                dir.pop(); // -> {path/to}
                dir.pop(); // -> {path}
                dir.push("__pycache__"); // -> {path}/__pycache__
                dir.push(last_path); // -> {path}/__pycache__/{to}
                dir.set_extension("d.er"); // -> {path}/__pycache__/{to}.d.er
                dir.canonicalize()
            })
            .or_else(|_| {
                dir.pop(); // -> {path}/__pycache__
                dir.pop(); // -> {path}
                dir.push(last_path); // -> {path/to}
                dir.push("__pycache__"); // -> {path/to}/__pycache__
                dir.push("__init__.d.er"); // -> {path/to}/__pycache__/__init__.d.er
                dir.canonicalize()
            })?;
        Ok(normalize_path(path))
    }

    fn resolve_local_py(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        let mut dir = self.dir();
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

    pub fn resolve_py(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        if ERG_MODE || path.starts_with("./") {
            if let Ok(path) = self.resolve_local_py(path) {
                return Ok(path);
            }
        }
        for sys_path in self.sys_path() {
            let mut dir = sys_path;
            dir.push(path);
            dir.set_extension("py");
            if dir.exists() {
                return Ok(normalize_path(dir));
            }
            dir.pop();
            dir.push(path);
            dir.push("__init__.py");
            if dir.exists() {
                return Ok(normalize_path(dir));
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("cannot find module `{}`", path.display()),
        ))
    }

    pub fn resolve_path(&self, path: &Path) -> Option<PathBuf> {
        self.resolve_real_path(path)
            .or_else(|| self.resolve_decl_path(path))
    }

    /// resolution order:
    /// 1. `./{path/to}.er`
    /// 2. `./{path/to}/__init__.er`
    /// 3. `std/{path/to}.er`
    /// 4. `std/{path/to}/__init__.er`
    pub fn resolve_real_path(&self, path: &Path) -> Option<PathBuf> {
        if let Ok(path) = self.resolve_local(path) {
            Some(path)
        } else if let Ok(path) = erg_std_path()
            .join(format!("{}.er", path.display()))
            .canonicalize()
        {
            Some(normalize_path(path))
        } else if let Ok(path) = erg_std_path()
            .join(format!("{}", path.display()))
            .join("__init__.er")
            .canonicalize()
        {
            Some(normalize_path(path))
        } else {
            None
        }
    }

    /// resolution order:
    /// 1.  `{path/to}.d.er`
    /// 2.  `{path/to}/__init__.d.er`
    /// 3.  `{path}/__pycache__/{to}.d.er`
    /// 4.  `{path/to}/__pycache__/__init__.d.er`
    /// 5.  `{path.d/to.d}/__init__.d.er`
    /// 6.  `{path.d/to.d}/__pycache__/__init__.d.er`
    /// 7.  `std/{path/to}.d.er`
    /// 8.  `std/{path/to}/__init__.d.er`
    /// 9.  `site-packages/{path}/__pycache__/{to}.d.er`
    /// 10. `site-packages/{path/to}/__pycache__/__init__.d.er`
    pub fn resolve_decl_path(&self, path: &Path) -> Option<PathBuf> {
        if let Ok(path) = self.resolve_local_decl(path) {
            Some(path)
        } else {
            let py_roots = [erg_pystd_path, erg_py_external_lib_path];
            for root in py_roots {
                if let Some(path) = Self::resolve_std_decl_path(root(), path) {
                    return Some(path);
                }
            }
            for site_packages in python_site_packages() {
                if let Some(path) = Self::resolve_site_pkgs_decl_path(site_packages, path) {
                    return Some(path);
                }
            }
            None
        }
    }

    /// 1. `site-packages/{path/to}.d.er`
    /// 2. `site-packages/{path.d/to.d}/__init__.d.er`
    fn resolve_std_decl_path(root: PathBuf, path: &Path) -> Option<PathBuf> {
        let mut path = add_postfix_foreach(path, ".d");
        path.set_extension("d.er"); // set_extension overrides the previous one
        if let Ok(path) = root.join(&path).canonicalize() {
            Some(normalize_path(path))
        // d.er -> .d
        } else if let Ok(path) = root
            .join({
                path.set_extension("");
                path
            })
            .join("__init__.d.er")
            .canonicalize()
        {
            Some(normalize_path(path))
        } else {
            None
        }
    }

    /// 1. `site-packages/__pycache__/{path/to}.d.er`
    /// 2. `site-packages/{path/to}/__pycache__/__init__.d.er`
    ///
    /// e.g. `toml/encoder`
    ///     -> `site-packages/toml/__pycache__/encoder.d.er`, `site-packages/toml/encoder/__pycache__/__init__.d.er`
    fn resolve_site_pkgs_decl_path(site_packages: PathBuf, path: &Path) -> Option<PathBuf> {
        let dir = path.parent().unwrap_or_else(|| Path::new(""));
        let mut file_path = PathBuf::from(path.file_stem().unwrap_or_default());
        file_path.set_extension("d.er"); // set_extension overrides the previous one
        if let Ok(path) = site_packages
            .join(dir)
            .join("__pycache__")
            .join(&file_path)
            .canonicalize()
        {
            Some(normalize_path(path))
        } else if let Ok(path) = site_packages
            .join(path)
            .join("__pycache__")
            .join("__init__.d.er")
            .canonicalize()
        {
            Some(normalize_path(path))
        } else {
            None
        }
    }

    pub fn try_push_path(mut path: PathBuf, add: &Path) -> Result<PathBuf, String> {
        path.pop(); // __init__.d.er
        if let Ok(path) = path.join(add).canonicalize() {
            Ok(normalize_path(path))
        } else if let Ok(path) = path.join(format!("{}.d.er", add.display())).canonicalize() {
            Ok(normalize_path(path))
        } else if let Ok(path) = path
            .join(format!("{}.d", add.display()))
            .join("__init__.d.er")
            .canonicalize()
        {
            Ok(normalize_path(path))
        } else {
            Err(format!("{} // {}", path.display(), add.display()))
        }
    }

    pub fn decl_file_is(&self, decl_path: &Path) -> bool {
        let mut py_path = self.unescaped_path().to_path_buf();
        py_path.set_extension("d.er");
        if decl_path == py_path {
            return true;
        }
        let last = py_path.file_name().unwrap_or_default().to_os_string();
        py_path.pop();
        py_path.push("__pycache__");
        py_path.push(last);
        decl_path == py_path
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
                /* Commands */
                "lex" | "parse" | "desugar" | "typecheck" | "check" | "compile" | "transpile"
                | "run" | "execute" | "server" | "tc" => {
                    cfg.mode = ErgMode::try_from(&arg[..]).unwrap();
                }
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
                    let path = PathBuf::from_str(&arg[..])
                        .unwrap_or_else(|_| panic!("invalid file path: {arg}"));
                    let path = normalize_path(path);
                    cfg.input = Input::file(path);
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
