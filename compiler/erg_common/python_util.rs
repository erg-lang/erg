//! utilities for calling CPython.
//!
//! CPythonを呼び出すためのユーティリティー
use std::process::Command;

use crate::serialize::get_magic_num_from_bytes;

#[cfg(unix)]
pub const BUILTIN_PYTHON_MODS: [&str; 21] = [
    "datetime",
    "glob",
    "http",
    "importlib",
    "io",
    "json",
    "math",
    "os",
    "platform",
    "posix",
    "random",
    "re",
    "shutil",
    "socket",
    "string",
    "subprocess",
    "sys",
    "tarfile",
    "time",
    "urllib",
    "zipfile",
];
#[cfg(not(unix))]
pub const BUILTIN_PYTHON_MODS: [&str; 20] = [
    "datetime",
    "glob",
    "http",
    "importlib",
    "io",
    "json",
    "math",
    "os",
    "platform",
    "random",
    "re",
    "shutil",
    "socket",
    "string",
    "subprocess",
    "sys",
    "tarfile",
    "time",
    "urllib",
    "zipfile",
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
