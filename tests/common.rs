#![allow(dead_code)]
use std::path::PathBuf;

use erg_common::config::{DummyStdin, ErgConfig, Input};
use erg_common::error::MultiErrorDisplay;
use erg_common::python_util::PythonVersion;
use erg_common::spawn::exec_new_thread;
use erg_common::style::{GREEN, RESET};
use erg_common::traits::{ExitStatus, Runnable, Stream};

use erg_compiler::error::CompileErrors;

use erg::DummyVM;

pub(crate) fn expect_repl_success(name: &'static str, lines: Vec<String>) -> Result<(), ()> {
    match exec_repl(name, lines) {
        Ok(ExitStatus::OK) => Ok(()),
        Ok(stat) => {
            println!("err: should succeed, but got: {stat:?}");
            Err(())
        }
        Err(errs) => {
            println!("err: should succeed, but got compile errors");
            errs.fmt_all_stderr();
            Err(())
        }
    }
}

pub(crate) fn expect_success(file_path: &'static str) -> Result<(), ()> {
    match exec_file(file_path) {
        Ok(0) => Ok(()),
        Ok(i) => {
            println!("err: should succeed, but end with {i}");
            Err(())
        }
        Err(errs) => {
            println!("err: should succeed, but got compile errors");
            errs.fmt_all_stderr();
            Err(())
        }
    }
}

pub(crate) fn expect_end_with(file_path: &'static str, code: i32) -> Result<(), ()> {
    match exec_file(file_path) {
        Ok(0) => {
            println!("err: should end with {code}, but end with 0");
            Err(())
        }
        Ok(i) => {
            if i == code {
                Ok(())
            } else {
                println!("err: end with {i}");
                Err(())
            }
        }
        Err(errs) => {
            println!("err: should end with {code}, but got compile errors");
            errs.fmt_all_stderr();
            Err(())
        }
    }
}

pub(crate) fn expect_failure(file_path: &'static str, errs_len: usize) -> Result<(), ()> {
    match exec_file(file_path) {
        Ok(0) => {
            println!("err: should fail, but end with 0");
            Err(())
        }
        Ok(_) => Ok(()),
        Err(errs) => {
            errs.fmt_all_stderr();
            if errs.len() == errs_len {
                Ok(())
            } else {
                println!(
                    "err: number of errors should be {errs_len}, but got {}",
                    errs.len()
                );
                Err(())
            }
        }
    }
}

fn set_cfg(mut cfg: ErgConfig) -> ErgConfig {
    cfg.py_command = if cfg!(windows) {
        Some("python")
    } else {
        Some("python3")
    };
    let py_ver_minor = env!("PYTHON_VERSION_MINOR").parse::<u8>().unwrap();
    let py_ver_micro = env!("PYTHON_VERSION_MICRO").parse::<u8>().unwrap();
    let py_magic_num = env!("PYTHON_MAGIC_NUMBER").parse::<u32>().unwrap();
    cfg.target_version = Some(PythonVersion::new(
        3,
        Some(py_ver_minor),
        Some(py_ver_micro),
    ));
    cfg.py_magic_num = Some(py_magic_num);
    cfg
}

/// The test is intend to run only on 3.11 for fast execution.
/// To execute on other versions, change the version and magic number.
fn _exec_file(file_path: &'static str) -> Result<i32, CompileErrors> {
    println!("{GREEN}[test] exec {file_path}{RESET}");
    let cfg = ErgConfig::with_main_path(PathBuf::from(file_path));
    let mut vm = DummyVM::new(set_cfg(cfg));
    vm.exec()
}

/// WARN: You must quit REPL manually (use `:exit`, `:quit` or call something shutdowns the interpreter)
pub fn _exec_repl(name: &'static str, lines: Vec<String>) -> Result<ExitStatus, CompileErrors> {
    println!("{GREEN}[test] exec dummy REPL: {lines:?}{RESET}");
    let cfg = ErgConfig {
        input: Input::DummyREPL(DummyStdin::new(name.to_string(), lines)),
        quiet_repl: true,
        ..Default::default()
    };
    let stat = <DummyVM as Runnable>::run(set_cfg(cfg));
    Ok(stat)
}

pub(crate) fn exec_file(file_path: &'static str) -> Result<i32, CompileErrors> {
    exec_new_thread(move || _exec_file(file_path))
}

pub(crate) fn exec_repl(
    name: &'static str,
    lines: Vec<String>,
) -> Result<ExitStatus, CompileErrors> {
    exec_new_thread(move || _exec_repl(name, lines))
}
