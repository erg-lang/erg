use erg_common::style::{colors::DEBUG_MAIN, RESET};
use std::process::{Command, Stdio};

mod build_in_function;
mod literal;

#[derive(PartialEq, Debug)]
pub(crate) struct CommandOutput {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) status_code: Option<i32>,
}

fn successful_output(stdout: &str) -> CommandOutput {
    CommandOutput {
        stdout: stdout.into(),
        stderr: "".into(),
        status_code: Some(0),
    }
}

pub(crate) fn eval(code: &'static str) -> CommandOutput {
    println!("{DEBUG_MAIN}[test] eval:\n{code}{RESET}");
    let output = Command::new(env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME"))))
        .args(["-c", code])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command")
        .wait_with_output()
        .expect("failed to wait for output");
    CommandOutput {
        stdout: String::from_utf8(output.stdout)
            .expect("failed to convert stdout to string")
            .replace("\r\n", "\n"),
        stderr: String::from_utf8(output.stderr)
            .expect("failed to convert stderr to string")
            .replace("\r\n", "\n"),
        status_code: output.status.code(),
    }
}
