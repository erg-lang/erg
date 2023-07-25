//! utilities for calling CPython.
//!
//! CPythonを呼び出すためのユーティリティー
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use crate::fn_name_full;
use crate::io::Output;
use crate::pathutil::remove_verbatim;
use crate::serialize::get_magic_num_from_bytes;

#[cfg(unix)]
pub const BUILTIN_PYTHON_MODS: [&str; 176] = [
    "abc",
    "argparse",
    "array",
    "ast",
    "asyncio",
    "atexit",
    "base64",
    "bdb",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cmath",
    "cmd",
    "code",
    "codecs",
    "codeop",
    "collections",
    "colorsys",
    "compileall",
    "concurrent",
    "configparser",
    "contextlib",
    "contextvars",
    "copy",
    "copyreg",
    "cProfile",
    "csv",
    "ctypes",
    "curses",
    "dataclasses",
    "datetime",
    "dbm",
    "decimal",
    "difflib",
    "dis",
    "distutils",
    "doctest",
    "email",
    "encodings",
    "ensurepip",
    "enum",
    "errno",
    "faulthandler",
    "fcntl",
    "filecmp",
    "fileinput",
    "fnmatch",
    "fractions",
    "ftplib",
    "functools",
    "gc",
    "getopt",
    "getpass",
    "gettext",
    "glob",
    "graphlib",
    "grp",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "imaplib",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "lib2to3",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "mailbox",
    "marshal",
    "math",
    "mimetypes",
    "mmap",
    "modulefinder",
    "multiprocessing",
    "netrc",
    "numbers",
    "operator",
    "os",
    "pathlib",
    "pdb",
    "pickle",
    "pickletools",
    "pkgutil",
    "platform",
    "plistlib",
    "poplib",
    "posix",
    "pprint",
    "profile",
    "pstats",
    "pty",
    "pwd",
    "py_compile",
    "pyclbr",
    "pydoc",
    "queue",
    "quopri",
    "random",
    "re",
    "readline",
    "reprlib",
    "resource",
    "rlcompleter",
    "runpy",
    "sched",
    "secrets",
    "select",
    "selectors",
    "shelve",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtplib",
    "socket",
    "socketserver",
    "sqlite3",
    "ssl",
    "stat",
    "statistics",
    "string",
    "stringprep",
    "struct",
    "subprocess",
    "symtable",
    "sys",
    "sysconfig",
    "syslog",
    "tabnanny",
    "tarfile",
    "tempfile",
    "termios",
    "test",
    "textwrap",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "token",
    "tokenize",
    "trace",
    "traceback",
    "tracemalloc",
    "tty",
    "turtle",
    "turtledemo",
    "types",
    "typing",
    "unicodedata",
    "unittest",
    "urllib",
    "uuid",
    "venv",
    "warnings",
    "wave",
    "weakref",
    "webbrowser",
    "wsgiref",
    "xml",
    "xmlrpc",
    "zipapp",
    "zipfile",
    "zipimport",
    "zlib",
    "zoneinfo",
];
#[cfg(windows)]
pub const BUILTIN_PYTHON_MODS: [&str; 171] = [
    "abc",
    "argparse",
    "array",
    "ast",
    "asyncio",
    "atexit",
    "base64",
    "bdb",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cmath",
    "cmd",
    "code",
    "codecs",
    "codeop",
    "collections",
    "colorsys",
    "compileall",
    "concurrent",
    "configparser",
    "contextlib",
    "contextvars",
    "copy",
    "copyreg",
    "cProfile",
    "csv",
    "ctypes",
    "curses",
    "dataclasses",
    "datetime",
    "dbm",
    "decimal",
    "difflib",
    "dis",
    "distutils",
    "doctest",
    "email",
    "encodings",
    "ensurepip",
    "enum",
    "errno",
    "faulthandler",
    "filecmp",
    "fileinput",
    "fnmatch",
    "fractions",
    "ftplib",
    "functools",
    "gc",
    "getopt",
    "getpass",
    "gettext",
    "glob",
    "graphlib",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "imaplib",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "lib2to3",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "mailbox",
    "marshal",
    "math",
    "mimetypes",
    "mmap",
    "modulefinder",
    "msvcrt",
    "multiprocessing",
    "netrc",
    "numbers",
    "operator",
    "os",
    "pathlib",
    "pdb",
    "pickle",
    "pickletools",
    "pkgutil",
    "plistlib",
    "poplib",
    "platform",
    "plistlib",
    "poplib",
    "pprint",
    "profile",
    "pstats",
    "py_compile",
    "pyclbr",
    "pydoc",
    "queue",
    "quopri",
    "random",
    "re",
    "reprlib",
    "rlcompleter",
    "runpy",
    "sched",
    "secrets",
    "select",
    "selectors",
    "shelve",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtplib",
    "socket",
    "socketserver",
    "sqlite3",
    "ssl",
    "stat",
    "statistics",
    "string",
    "stringprep",
    "struct",
    "subprocess",
    "symtable",
    "sys",
    "sysconfig",
    "tabnanny",
    "tarfile",
    "tempfile",
    "test",
    "textwrap",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "token",
    "tokenize",
    "trace",
    "traceback",
    "tracemalloc",
    "turtle",
    "turtledemo",
    "types",
    "typing",
    "unicodedata",
    "unittest",
    "urllib",
    "uuid",
    "venv",
    "warnings",
    "wave",
    "weakref",
    "webbrowser",
    "winreg",
    "winsound",
    "wsgiref",
    "xml",
    "xmlrpc",
    "zipapp",
    "zipfile",
    "zipimport",
    "zlib",
    "zoneinfo",
];
#[cfg(not(any(windows, unix)))]
pub const BUILTIN_PYTHON_MODS: [&str; 165] = [
    "abc",
    "argparse",
    "array",
    "ast",
    "asyncio",
    "atexit",
    "base64",
    "bdb",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cmath",
    "cmd",
    "code",
    "codecs",
    "codeop",
    "collections",
    "colorsys",
    "compileall",
    "concurrent",
    "configparser",
    "contextlib",
    "contextvars",
    "copy",
    "copyreg",
    "cProfile",
    "csv",
    "ctypes",
    "dataclasses",
    "datetime",
    "dbm",
    "decimal",
    "difflib",
    "dis",
    "distutils",
    "doctest",
    "email",
    "encodings",
    "ensurepip",
    "enum",
    "errno",
    "faulthandler",
    "filecmp",
    "fileinput",
    "fnmatch",
    "fractions",
    "ftplib",
    "functools",
    "gc",
    "getopt",
    "getpass",
    "gettext",
    "glob",
    "graphlib",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "imaplib",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "lib2to3",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "mailbox",
    "marshal",
    "math",
    "mimetypes",
    "mmap",
    "modulefinder",
    "multiprocessing",
    "netrc",
    "numbers",
    "operator",
    "os",
    "pathlib",
    "pdb",
    "pickle",
    "pickletools",
    "pkgutil",
    "platform",
    "plistlib",
    "poplib",
    "pprint",
    "profile",
    "pstats",
    "py_compile",
    "pyclbr",
    "pydoc",
    "queue",
    "quopri",
    "random",
    "re",
    "reprlib",
    "rlcompleter",
    "runpy",
    "sched",
    "secrets",
    "select",
    "selectors",
    "shelve",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtplib",
    "socket",
    "socketserver",
    "sqlite3",
    "ssl",
    "stat",
    "statistics",
    "string",
    "stringprep",
    "struct",
    "subprocess",
    "symtable",
    "sys",
    "sysconfig",
    "tabnanny",
    "tarfile",
    "tempfile",
    "test",
    "textwrap",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "token",
    "tokenize",
    "trace",
    "traceback",
    "tracemalloc",
    "turtle",
    "turtledemo",
    "types",
    "typing",
    "unicodedata",
    "unittest",
    "urllib",
    "uuid",
    "venv",
    "warnings",
    "wave",
    "weakref",
    "webbrowser",
    "wsgiref",
    "xml",
    "xmlrpc",
    "zipapp",
    "zipfile",
    "zipimport",
    "zlib",
    "zoneinfo",
];
pub const EXT_PYTHON_MODS: [&str; 7] = [
    "matplotlib",
    "numpy",
    "pandas",
    "requests",
    "setuptools",
    "tqdm",
    "urllib3",
];
pub const EXT_COMMON_ALIAS: [&str; 7] = [
    "mpl",
    "np",
    "pd",
    "requests",
    "setuptools",
    "tqdm",
    "urllib3",
];

pub fn opt_which_python() -> Result<String, String> {
    let (cmd, python) = if cfg!(windows) {
        ("where", "python")
    } else {
        ("which", "python3")
    };
    let Ok(out) = Command::new(cmd).arg(python).output() else {
        return Err(format!("{}: {python} not found", fn_name_full!()));
    };
    let Ok(res) = String::from_utf8(out.stdout) else {
        return Err(format!("{}: failed to commnunicate with Python", fn_name_full!()));
    };
    let res = res.split('\n').next().unwrap_or("").replace('\r', "");
    if res.is_empty() {
        return Err(format!("{}: {python} not found", fn_name_full!()));
    } else if res.contains("pyenv") && cfg!(windows) {
        // because pyenv-win does not support `-c` option
        return Err("cannot use pyenv-win".into());
    }
    Ok(res)
}

fn which_python() -> String {
    opt_which_python().unwrap()
}

pub fn detect_magic_number(py_command: &str) -> u32 {
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(py_command)
            .arg("-c")
            .arg("import importlib.util as util;print(util.MAGIC_NUMBER.hex())")
            .output()
            .expect("cannot get the magic number from python")
    } else {
        let exec_command = format!(
            "{py_command} -c 'import importlib.util as util;print(util.MAGIC_NUMBER.hex())'",
        );
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .output()
            .expect("cannot get the magic number from python")
    };
    let s_hex_magic_num = String::from_utf8(out.stdout).unwrap();
    let first_byte = u8::from_str_radix(&s_hex_magic_num[0..=1], 16).unwrap();
    let second_byte = u8::from_str_radix(&s_hex_magic_num[2..=3], 16).unwrap();
    get_magic_num_from_bytes(&[first_byte, second_byte, 0, 0])
}

pub fn env_magic_number() -> u32 {
    detect_magic_number(&which_python())
}

pub fn module_exists(py_command: &str, module: &str) -> bool {
    let code = format!("import importlib; errc = 1 if importlib.util.find_spec('{module}') is None else 0; exit(errc)");
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(py_command)
            .arg("-c")
            .arg(code)
            .output()
            .expect("cannot get module spec")
    } else {
        let exec_command = format!("{py_command} -c '{code}'");
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .output()
            .expect("cannot get module spec")
    };
    out.status.success()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PythonVersion {
    pub major: u8,
    pub minor: Option<u8>,
    pub micro: Option<u8>,
}

impl Default for PythonVersion {
    fn default() -> Self {
        Self::new(3, Some(11), Some(0))
    }
}

impl PythonVersion {
    pub const fn new(major: u8, minor: Option<u8>, micro: Option<u8>) -> Self {
        Self {
            major,
            minor,
            micro,
        }
    }

    pub fn le(&self, other: &Self) -> bool {
        self.major <= other.major
            || (self.major == other.major && self.minor <= other.minor)
            || (self.major == other.major && self.minor == other.minor && self.micro <= other.micro)
    }

    pub fn minor_is(&self, major: u8, minor: u8) -> bool {
        self.major == major && self.minor == Some(minor)
    }

    pub fn to_command(&self) -> String {
        match (self.minor, self.micro) {
            (None, None) => format!("python{}", self.major),
            (Some(minor), None) => format!("python{}.{minor}", self.major),
            (None, Some(_)) => format!("python{}", self.major),
            (Some(minor), Some(micro)) => format!("python{}.{}.{}", self.major, minor, micro),
        }
    }
}

impl std::str::FromStr for PythonVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split('.');
        let major = iter.next().and_then(|i| i.parse().ok()).unwrap_or(3);
        let minor = iter.next().and_then(|i| i.parse().ok());
        let micro = iter.next().and_then(|i| i.parse().ok());
        Ok(Self::new(major, minor, micro))
    }
}

pub fn get_python_version(py_command: &str) -> PythonVersion {
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(py_command)
            .arg("--version")
            .output()
            .expect("cannot get the python version")
    } else {
        let exec_command = format!("{py_command} --version");
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .output()
            .expect("cannot get the python version")
    };
    let s_version = String::from_utf8(out.stdout).unwrap();
    let mut iter = s_version.split(' ');
    let mut iter = iter.nth(1).unwrap().split('.');
    let major = iter.next().and_then(|i| i.parse().ok()).unwrap_or(3);
    let minor = iter.next().and_then(|i| i.parse().ok());
    let micro = iter.next().and_then(|i| i.trim_end().parse().ok());
    PythonVersion {
        major,
        minor,
        micro,
    }
}

pub fn env_python_version() -> PythonVersion {
    get_python_version(&which_python())
}

pub fn get_sys_path(working_dir: Option<&Path>) -> Result<Vec<PathBuf>, std::io::Error> {
    let working_dir = fs::canonicalize(working_dir.unwrap_or(Path::new(""))).unwrap_or_default();
    let working_dir = remove_verbatim(&working_dir);
    let py_command = opt_which_python().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("cannot find python: {e}"),
        )
    })?;
    let code = "import os, sys; print('\\n'.join(map(lambda p: os.path.abspath(p), sys.path)))";
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg("cd")
            .arg(working_dir)
            .arg("&&")
            .arg(py_command)
            .arg("-c")
            .arg(code)
            .output()?
    } else {
        let exec_command = format!("cd {working_dir} && {py_command} -c \"{code}\"");
        Command::new("sh").arg("-c").arg(exec_command).output()?
    };
    let s_sys_path = String::from_utf8(out.stdout).unwrap();
    let res = s_sys_path
        .split('\n')
        .map(|s| PathBuf::from(s.trim().to_string()))
        .collect();
    Ok(res)
}

/// executes over a shell, cause `python` may not exist as an executable file (like pyenv)
pub fn exec_pyc<S: Into<String>, T: Into<Stdio>>(
    file: S,
    py_command: Option<&str>,
    argv: &[&'static str],
    stdout: T,
) -> Option<i32> {
    let command = py_command
        .map(ToString::to_string)
        .unwrap_or_else(which_python);
    let mut out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(command)
            .arg(&file.into())
            .args(argv)
            .stdout(stdout)
            .spawn()
            .expect("cannot execute python")
    } else {
        let exec_command = format!("{command} {} {}", file.into(), argv.join(" "));
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .stdout(stdout)
            .spawn()
            .expect("cannot execute python")
    };
    out.wait().expect("python doesn't work").code()
}

/// evaluates over a shell, cause `python` may not exist as an executable file (like pyenv)
pub fn _eval_pyc<S: Into<String>>(file: S, py_command: Option<&str>) -> String {
    let command = py_command
        .map(ToString::to_string)
        .unwrap_or_else(which_python);
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(command)
            .arg(&file.into())
            .spawn()
            .expect("cannot execute python")
    } else {
        let exec_command = format!("{command} {}", file.into());
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .spawn()
            .expect("cannot execute python")
    };
    let out = out.wait_with_output().expect("python doesn't work");
    String::from_utf8_lossy(&out.stdout).to_string()
}

pub fn exec_py(file: &str) -> Option<i32> {
    let mut child = if cfg!(windows) {
        Command::new(which_python())
            .arg(file)
            .spawn()
            .expect("cannot execute python")
    } else {
        let exec_command = format!("{} {file}", which_python());
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .spawn()
            .expect("cannot execute python")
    };
    child.wait().expect("python doesn't work").code()
}

pub fn env_spawn_py(code: &str) {
    if cfg!(windows) {
        Command::new(which_python())
            .arg("-c")
            .arg(code)
            .spawn()
            .expect("cannot execute python");
    } else {
        let exec_command = format!("{} -c \"{}\"", which_python(), code);
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .spawn()
            .expect("cannot execute python");
    }
}

pub fn spawn_py(py_command: Option<&str>, code: &str) {
    if cfg!(windows) {
        Command::new(py_command.unwrap_or(&which_python()))
            .arg("-c")
            .arg(code)
            .spawn()
            .expect("cannot execute python");
    } else {
        let exec_command = format!("{} -c \"{}\"", py_command.unwrap_or(&which_python()), code);
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .spawn()
            .expect("cannot execute python");
    }
}

pub fn exec_py_code(code: &str, output: Output) -> std::io::Result<ExitStatus> {
    let mut out = if cfg!(windows) {
        Command::new(which_python())
            .arg("-c")
            .arg(code)
            .stdout(output)
            .spawn()
            .expect("cannot execute python")
    } else {
        let exec_command = format!("{} -c \"{code}\"", which_python());
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .stdout(output)
            .spawn()
            .expect("cannot execute python")
    };
    out.wait()
}
