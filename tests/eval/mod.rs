use erg_common::style::{colors::DEBUG_MAIN, RESET};
use std::process::Command;

mod basic;
mod literal;

pub(crate) struct CommandOutput {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) status: std::process::ExitStatus,
}

pub(crate) fn eval_code(code: &'static str) -> CommandOutput {
    println!("{DEBUG_MAIN}[test] eval:\n{code}{RESET}");
    let output = Command::new(env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME"))))
        .args(["-c", code])
        .output()
        .unwrap();
    CommandOutput {
        stdout: String::from_utf8(output.stdout)
            .unwrap()
            .replace("\r\n", "\n"),
        stderr: String::from_utf8(output.stderr)
            .unwrap()
            .replace("\r\n", "\n"),
        status: output.status,
    }
}
