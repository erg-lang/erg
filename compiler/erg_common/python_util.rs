//! utilities for calling CPython.
//!
//! CPythonを呼び出すためのユーティリティー
use std::process::Command;

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
    "binhex",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cmath",
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
pub const BUILTIN_PYTHON_MODS: [&str; 170] = [
    "argparse",
    "array",
    "ast",
    "asyncio",
    "atexit",
    "base64",
    "bdb",
    "binascii",
    "binhex",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cmath",
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

pub fn which_python() -> String {
    let (cmd, python) = if cfg!(windows) {
        ("where", "python")
    } else {
        ("which", "python3")
    };
    let out = Command::new(cmd)
        .arg(python)
        .output()
        .expect("python not found");
    let res = String::from_utf8(out.stdout).unwrap();
    let res = res.split('\n').next().unwrap_or("").replace('\r', "");
    if res.is_empty() {
        println!("python not found");
        std::process::exit(1);
    } else if res.contains("pyenv") {
        println!("cannot use pyenv");
        std::process::exit(1);
    }
    res
}

pub fn detect_magic_number() -> u32 {
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(which_python())
            .arg("-c")
            .arg("import importlib.util as util;print(util.MAGIC_NUMBER.hex())")
            .output()
            .expect("cannot get the magic number from python")
    } else {
        let python_command = format!(
            "{} -c 'import importlib.util as util;print(util.MAGIC_NUMBER.hex())'",
            which_python()
        );
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .output()
            .expect("cannot get the magic number from python")
    };
    let s_hex_magic_num = String::from_utf8(out.stdout).unwrap();
    let first_byte = u8::from_str_radix(&s_hex_magic_num[0..=1], 16).unwrap();
    let second_byte = u8::from_str_radix(&s_hex_magic_num[2..=3], 16).unwrap();
    get_magic_num_from_bytes(&[first_byte, second_byte, 0, 0])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PythonVersion {
    pub major: u8,
    pub minor: u8,
    pub micro: u8,
}

impl PythonVersion {
    pub const fn new(major: u8, minor: u8, micro: u8) -> Self {
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
        self.major == major && self.minor == minor
    }
}

pub fn python_version() -> PythonVersion {
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(which_python())
            .arg("-c")
            .arg("import sys;print(sys.version_info.major, sys.version_info.minor, sys.version_info.micro)")
            .output()
            .expect("cannot get the python version")
    } else {
        let python_command = format!(
            "{} -c 'import sys;print(sys.version_info.major, sys.version_info.minor, sys.version_info.micro)'",
            which_python()
        );
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .output()
            .expect("cannot get the python version")
    };
    let s_version = String::from_utf8(out.stdout).unwrap();
    let mut iter = s_version.split(' ');
    let major = iter.next().unwrap().parse().unwrap();
    let minor = iter.next().unwrap().parse().unwrap();
    let micro = iter.next().unwrap().trim_end().parse().unwrap();
    PythonVersion {
        major,
        minor,
        micro,
    }
}

/// executes over a shell, cause `python` may not exist as an executable file (like pyenv)
pub fn exec_pyc<S: Into<String>>(file: S) -> Option<i32> {
    let mut out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(which_python())
            .arg(&file.into())
            .spawn()
            .expect("cannot execute python")
    } else {
        let python_command = format!("{} {}", which_python(), file.into());
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .spawn()
            .expect("cannot execute python")
    };
    out.wait().expect("python doesn't work").code()
}

/// evaluates over a shell, cause `python` may not exist as an executable file (like pyenv)
pub fn eval_pyc<S: Into<String>>(file: S) -> String {
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(which_python())
            .arg(&file.into())
            .spawn()
            .expect("cannot execute python")
    } else {
        let python_command = format!("{} {}", which_python(), file.into());
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .spawn()
            .expect("cannot execute python")
    };
    let out = out.wait_with_output().expect("python doesn't work");
    String::from_utf8_lossy(&out.stdout).to_string()
}

pub fn exec_py(code: &str) -> Option<i32> {
    let mut child = if cfg!(windows) {
        Command::new(which_python())
            .arg("-c")
            .arg(code)
            .spawn()
            .expect("cannot execute python")
    } else {
        let python_command = format!("{} -c \"{}\"", which_python(), code);
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .spawn()
            .expect("cannot execute python")
    };
    child.wait().expect("python doesn't work").code()
}

pub fn spawn_py(code: &str) {
    if cfg!(windows) {
        Command::new(which_python())
            .arg("-c")
            .arg(code)
            .spawn()
            .expect("cannot execute python");
    } else {
        let python_command = format!("{} -c \"{}\"", which_python(), code);
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .spawn()
            .expect("cannot execute python");
    }
}
