//! utilities for calling CPython.
//!
//! CPythonを呼び出すためのユーティリティー
use std::process::Command;

use crate::serialize::get_magic_num_from_bytes;

pub fn which_python() -> String {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let out = Command::new("which")
        .arg(python)
        .output()
        .expect("python not found");
    let res = String::from_utf8(out.stdout)
        .unwrap()
        .replace("\n", "")
        .replace("\r", "");
    if res == "" {
        panic!("python not found");
    }
    dbg!(&res);
    res
}

pub fn detect_magic_number() -> u32 {
    let command = if cfg!(windows) { "cmd" } else { "sh" };
    let arg = if cfg!(windows) { "/C" } else { "-c" };
    let out = Command::new(command)
        .arg(arg)
        .arg(which_python())
        .arg("-c")
        .arg("import importlib.util as util;print(util.MAGIC_NUMBER.hex())")
        .output()
        .expect("cannot get the magic number from python");
    dbg!(&out);
    let s_hex_magic_num = String::from_utf8(out.stdout).unwrap();
    let first_byte = u8::from_str_radix(&s_hex_magic_num[0..=1], 16).unwrap();
    let second_byte = u8::from_str_radix(&s_hex_magic_num[2..=3], 16).unwrap();
    get_magic_num_from_bytes(&[first_byte, second_byte, 0, 0])
}

pub fn exec_pyc<S: Into<String>>(file: S) {
    // executes over a shell, cause `python` may not exist as an executable file (like pyenv)
    let command = if cfg!(windows) { "cmd" } else { "sh" };
    let arg = if cfg!(windows) { "/C" } else { "-c" };
    let mut out = Command::new(command)
        .arg(arg)
        .arg(which_python())
        .arg(&file.into())
        .spawn()
        .expect("python not found");
    out.wait().expect("python doesn't work");
}

pub fn eval_pyc<S: Into<String>>(file: S) -> String {
    // executes over a shell, cause `python` may not exist as an executable file (like pyenv)
    let command = if cfg!(windows) { "cmd" } else { "sh" };
    let arg = if cfg!(windows) { "/C" } else { "-c" };
    let out = Command::new(command)
        .arg(arg)
        .arg(which_python())
        .arg(&file.into())
        .spawn()
        .expect("python not found");
    let out = out.wait_with_output().expect("python doesn't work");
    String::from_utf8(out.stdout).expect("failed to decode python output")
}
