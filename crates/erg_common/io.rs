use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::process::Stdio;

use crate::consts::{ERG_MODE, EXPERIMENTAL_MODE};
use crate::env::{
    erg_path, erg_py_external_lib_path, erg_pystd_path, erg_std_path, python_site_packages,
};
use crate::pathutil::{add_postfix_foreach, remove_postfix};
use crate::python_util::get_sys_path;
use crate::random::random;
use crate::stdin::GLOBAL_STDIN;
use crate::{normalize_path, power_assert, read_file};

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

    pub fn path(&self) -> &Path {
        match self {
            Self::File(filename) => filename.as_path(),
            Self::REPL | Self::Pipe(_) => Path::new("<stdin>"),
            Self::DummyREPL(_stdin) => Path::new("<stdin>"),
            Self::Str(_) => Path::new("<string>"),
            Self::Dummy => Path::new("<dummy>"),
        }
    }

    pub fn as_str(&self) -> &str {
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
            if path.parent().is_none() {
                PathBuf::from(".")
            } else {
                path
            }
        } else {
            PathBuf::from(".")
        }
    }

    pub fn project_root(&self) -> Option<PathBuf> {
        if let Self::File(path) = self {
            let mut parent = path.clone();
            while parent.pop() {
                if parent.join("package.er").exists() {
                    return Some(parent);
                }
            }
            None
        } else {
            None
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

    pub fn dir(&self) -> PathBuf {
        self.kind.dir()
    }

    pub fn project_root(&self) -> Option<PathBuf> {
        self.kind.project_root()
    }

    pub fn enclosed_name(&self) -> &str {
        self.kind.as_str()
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
            InputKind::File(filename) => filename
                .file_stem()
                .and_then(|f| f.to_str())
                .unwrap_or("_")
                .trim_end_matches(".d")
                .to_string(),
            InputKind::REPL | InputKind::Pipe(_) => "<stdin>".to_string(),
            InputKind::DummyREPL(stdin) => format!("<stdin_{}>", stdin.name),
            InputKind::Str(_) => "<string>".to_string(),
            InputKind::Dummy => "<dummy>".to_string(),
        }
    }

    pub fn full_path(&self) -> PathBuf {
        match &self.kind {
            InputKind::File(filename) => filename.clone(),
            _ => PathBuf::from(self.file_stem()),
        }
    }

    pub fn filename(&self) -> String {
        match &self.kind {
            InputKind::File(filename) => filename
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("_")
                .trim_end_matches(".d")
                .to_string(),
            _ => self.file_stem(),
        }
    }

    pub fn path(&self) -> &Path {
        self.kind.path()
    }

    pub fn module_name(&self) -> String {
        match &self.kind {
            InputKind::File(filename) => {
                let file_stem = if filename.file_stem() == Some(OsStr::new("__init__"))
                    || filename.file_stem() == Some(OsStr::new("__init__.d"))
                {
                    filename.parent().and_then(|p| p.file_stem())
                } else {
                    filename.file_stem()
                };
                file_stem
                    .and_then(|f| f.to_str())
                    .unwrap_or("_")
                    .trim_end_matches(".d")
                    .to_string()
            }
            InputKind::REPL | InputKind::Pipe(_) => "<stdin>".to_string(),
            InputKind::DummyREPL(stdin) => stdin.name.clone(),
            InputKind::Str(_) => "<string>".to_string(),
            InputKind::Dummy => "<dummy>".to_string(),
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

    pub fn sys_path(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        get_sys_path(self.path().parent())
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

    fn resolve_local_decl(&self, dir: PathBuf, path: &Path) -> Result<PathBuf, std::io::Error> {
        self._resolve_local_decl(dir.clone(), path).or_else(|_| {
            let path = add_postfix_foreach(path, ".d");
            self._resolve_local_decl(dir, &path)
        })
    }

    /// resolution order:
    /// 1. `{path/to}.d.er`
    /// 2. `{path/to}/__init__.d.er`
    /// 3. `{path}/__pycache__/{to}.d.er`
    /// 4. `{path/to}/__pycache__/__init__.d.er`
    fn _resolve_local_decl(
        &self,
        mut dir: PathBuf,
        path: &Path,
    ) -> Result<PathBuf, std::io::Error> {
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
        for sys_path in self.sys_path()? {
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
            if !EXPERIMENTAL_MODE {
                break;
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
    /// (and repeat for the project root)
    /// 7.  `std/{path/to}.d.er`
    /// 8.  `std/{path/to}/__init__.d.er`
    /// 9.  `site-packages/{path}/__pycache__/{to}.d.er`
    /// 10. `site-packages/{path/to}/__pycache__/__init__.d.er`
    pub fn resolve_decl_path(&self, path: &Path) -> Option<PathBuf> {
        if let Ok(path) = self.resolve_local_decl(self.dir(), path) {
            return Some(path);
        }
        // e.g.
        // root: lib/external/pandas.d, path: pandas/core/frame
        // -> lib/external/pandas.d/core/frame
        // root: lib/external/pandas.d, path: pandas
        // -> lib/external/pandas.d
        // root: lib/external/pandas.d, path: contextlib
        // -> NO
        if let Some((root, first)) = self.project_root().zip(path.components().next()) {
            if root.ends_with(first) || remove_postfix(root.clone(), ".d").ends_with(first) {
                let path = path.iter().skip(1).collect::<PathBuf>();
                if let Ok(path) = self.resolve_local_decl(root, &path) {
                    return Some(path);
                }
            }
        }
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

    /// 1. `site-packages/{path/to}.d.er`
    /// 2. `site-packages/{path.d/to.d}/__init__.d.er`
    fn resolve_std_decl_path(root: &Path, path: &Path) -> Option<PathBuf> {
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
    fn resolve_site_pkgs_decl_path(site_packages: &Path, path: &Path) -> Option<PathBuf> {
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
        let mut py_path = self.path().to_path_buf();
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

#[derive(Debug)]
pub enum Output {
    Stdout(Stdout),
    File(File, String),
    Null,
}

impl Clone for Output {
    fn clone(&self) -> Self {
        match self {
            Self::Null => Self::Null,
            Self::Stdout(_) => Self::stdout(),
            Self::File(_, filename) => Self::file(filename.clone()),
        }
    }
}

impl std::io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Output::Stdout(stdout) => stdout.write(buf),
            Output::File(file, _) => file.write(buf),
            Output::Null => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Output::Stdout(stdout) => stdout.flush(),
            Output::File(file, _) => file.flush(),
            Output::Null => Ok(()),
        }
    }
}

impl From<Output> for Stdio {
    fn from(output: Output) -> Self {
        match output {
            Output::Stdout(_stdout) => Stdio::inherit(),
            Output::File(file, _) => Stdio::from(file),
            Output::Null => Stdio::null(),
        }
    }
}

impl Output {
    pub fn stdout() -> Self {
        Self::Stdout(std::io::stdout())
    }

    pub fn file(filename: String) -> Self {
        Self::File(File::open(&filename).unwrap(), filename)
    }
}

/// Log to `$ERG_PATH/els.log`
pub fn lsp_log(file: &str, line: u32, msg: &str) {
    let file_path = erg_path().join("els.log");
    let mut f = if file_path.exists() {
        File::options().append(true).open(file_path).unwrap()
    } else {
        File::create(file_path).unwrap()
    };
    f.write_all(format!("{file}@{line}: {msg}\n").as_bytes())
        .unwrap();
}

#[macro_export]
macro_rules! lsp_log {
    ($($arg:tt)*) => {
        let msg = format!($($arg)*);
        $crate::io::lsp_log(file!(), line!(), &msg);
    };
}
