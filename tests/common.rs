#![allow(dead_code)]
use std::path::PathBuf;

use erg_common::config::ErgConfig;
use erg_common::consts::DEBUG_MODE;
use erg_common::error::Location;
use erg_common::io::{DummyStdin, Input, Output};
use erg_common::python_util::PythonVersion;
use erg_common::spawn::exec_new_thread;
use erg_common::style::remove_style;
use erg_common::style::{colors::DEBUG_MAIN, RESET};
use erg_common::traits::{ExitStatus, Runnable};

use erg_compiler::error::CompileErrors;
use erg_compiler::Compiler;

use erg::DummyVM;

pub(crate) fn expect_repl_success(name: &'static str, lines: Vec<String>) -> Result<(), ()> {
    match exec_repl(name, lines) {
        Ok(stat) if stat.succeed() => Ok(()),
        Ok(stat) => {
            println!("err[{name}]: should succeed, but got: {stat:?}");
            Err(())
        }
        Err(errs) => {
            println!("err[{name}]: should succeed, but got compile errors");
            println!("{errs}");
            Err(())
        }
    }
}

pub(crate) fn expect_success(file_path: &'static str, num_warns: usize) -> Result<(), ()> {
    match exec_file(file_path) {
        Ok(stat) if stat.succeed() => {
            if stat.num_warns == num_warns {
                Ok(())
            } else {
                println!(
                    "err[{file_path}]: number of warnings should be {num_warns}, but got {}",
                    stat.num_warns
                );
                Err(())
            }
        }
        Ok(stat) => {
            println!(
                "err[{file_path}]: should succeed, but end with {}",
                stat.code
            );
            Err(())
        }
        Err(errs) => {
            println!("err[{file_path}]: should succeed, but got compile errors");
            println!("{errs}");
            Err(())
        }
    }
}

pub(crate) fn expect_compile_success(file_path: &'static str, num_warns: usize) -> Result<(), ()> {
    match exec_compiler(file_path) {
        Ok(stat) if stat.succeed() => {
            if stat.num_warns == num_warns {
                Ok(())
            } else {
                println!(
                    "err[{file_path}]: number of warnings should be {num_warns}, but got {}",
                    stat.num_warns
                );
                Err(())
            }
        }
        Ok(stat) => {
            println!(
                "err[{file_path}]: should succeed, but end with {}",
                stat.code
            );
            Err(())
        }
        Err(errs) => {
            println!("err[{file_path}]: should succeed, but got compile errors");
            println!("{errs}");
            Err(())
        }
    }
}

pub(crate) fn expect_repl_failure(
    name: &'static str,
    lines: Vec<String>,
    num_errs: usize,
) -> Result<(), ()> {
    match exec_repl(name, lines) {
        Ok(ExitStatus::OK) => Err(()),
        Ok(stat) => {
            if stat.num_errors == num_errs {
                Ok(())
            } else {
                println!(
                    "err[{name}]: number of errors should be {num_errs}, but got {}",
                    stat.num_errors
                );
                Err(())
            }
        }
        Err(errs) => {
            println!("err[{name}]: should succeed, but got compile errors");
            println!("{errs}");
            Err(())
        }
    }
}

pub(crate) fn expect_end_with(
    file_path: &'static str,
    num_warns: usize,
    code: i32,
) -> Result<(), ()> {
    match exec_file(file_path) {
        Ok(stat) if stat.succeed() => {
            println!("err[{file_path}]: should end with {code}, but end with 0");
            Err(())
        }
        Ok(stat) => {
            if stat.code != code {
                println!("err[{file_path}]: end with {}", stat.code);
                return Err(());
            }
            if stat.num_warns != num_warns {
                println!(
                    "err[{file_path}]: number of warnings should be {num_warns}, but got {}",
                    stat.num_warns
                );
                return Err(());
            }
            Ok(())
        }
        Err(errs) => {
            println!("err[{file_path}]: should end with {code}, but got compile errors");
            println!("{errs}");
            Err(())
        }
    }
}

pub(crate) fn expect_failure(
    file_path: &'static str,
    num_warns: usize,
    num_errs: usize,
) -> Result<(), ()> {
    match exec_file(file_path) {
        Ok(stat) if stat.succeed() => {
            println!("err[{file_path}]: should fail, but end with 0");
            Err(())
        }
        Ok(stat) => {
            if stat.num_warns == num_warns {
                Ok(())
            } else {
                println!(
                    "err[{file_path}]: number of warnings should be {num_warns}, but got {}",
                    stat.num_warns
                );
                Err(())
            }
        }
        Err(errs) => {
            if errs.len() == num_errs {
                Ok(())
            } else {
                println!(
                    "err[{file_path}]: number of errors should be {num_errs}, but got {}",
                    errs.len()
                );
                println!("{errs}");
                Err(())
            }
        }
    }
}

pub(crate) fn expect_error_location_and_msg(
    file_path: &'static str,
    locs: Vec<(Location, Option<&str>)>,
) -> Result<(), ()> {
    match exec_compiler(file_path) {
        Ok(_) => {
            println!("err[{file_path}]: compilation should fail, but end with 0");
            Err(())
        }
        Err(errs) => {
            for (err, (loc, msg)) in errs.into_iter().zip(locs) {
                if err.core.loc != loc {
                    println!(
                        "err[{file_path}]: error location should be {loc}, but got {}",
                        err.core.loc
                    );
                    return Err(());
                }
                if msg.is_some_and(|m| remove_style(&err.core.main_message) != m) {
                    println!(
                        "err[{file_path}]: error message should be {:?}, but got {:?}",
                        msg, err.core.main_message
                    );
                    return Err(());
                }
            }
            Ok(())
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
fn _exec_file(file_path: &'static str) -> Result<ExitStatus, CompileErrors> {
    println!("{DEBUG_MAIN}[test] exec {file_path}{RESET}");
    let mut cfg = ErgConfig::with_main_path(PathBuf::from(file_path));
    cfg.output = if DEBUG_MODE {
        Output::stdout()
    } else {
        Output::Null
    };
    let mut vm = DummyVM::new(set_cfg(cfg));
    vm.exec()
}

/// WARN: You must quit REPL manually (use `:exit`, `:quit` or call something shutdowns the interpreter)
pub fn _exec_repl(name: &'static str, lines: Vec<String>) -> Result<ExitStatus, CompileErrors> {
    println!("{DEBUG_MAIN}[test] exec dummy REPL: {lines:?}{RESET}");
    let cfg = ErgConfig {
        input: Input::dummy_repl(DummyStdin::new(name.to_string(), lines)),
        quiet_repl: true,
        ..Default::default()
    };
    let stat = <DummyVM as Runnable>::run(set_cfg(cfg));
    Ok(stat)
}

pub fn _exec_compiler(file_path: &'static str) -> Result<ExitStatus, CompileErrors> {
    println!("{DEBUG_MAIN}[test] exec compiler: {file_path}{RESET}");
    let cfg = ErgConfig::with_main_path(PathBuf::from(file_path));
    let mut compiler = Compiler::new(set_cfg(cfg));
    compiler.exec()
}

pub(crate) fn exec_file(file_path: &'static str) -> Result<ExitStatus, CompileErrors> {
    exec_new_thread(move || _exec_file(file_path), file_path)
}

pub(crate) fn exec_repl(
    name: &'static str,
    lines: Vec<String>,
) -> Result<ExitStatus, CompileErrors> {
    exec_new_thread(move || _exec_repl(name, lines), name)
}

pub(crate) fn exec_compiler(file_path: &'static str) -> Result<ExitStatus, CompileErrors> {
    exec_new_thread(move || _exec_compiler(file_path), file_path)
}
