#![allow(dead_code)]
use std::path::PathBuf;

use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::python_util::PythonVersion;
use erg_common::spawn::exec_new_thread;
use erg_common::style::{GREEN, RESET};
use erg_common::traits::{Runnable, Stream};

use erg_compiler::error::CompileErrors;

use erg::DummyVM;

pub(crate) fn expect_success(file_path: &'static str) -> Result<(), ()> {
    match exec_vm(file_path) {
        Ok(0) => Ok(()),
        Ok(i) => {
            println!("err: end with {i}");
            Err(())
        }
        Err(errs) => {
            errs.fmt_all_stderr();
            Err(())
        }
    }
}

pub(crate) fn expect_end_with(file_path: &'static str, code: i32) -> Result<(), ()> {
    match exec_vm(file_path) {
        Ok(0) => Err(()),
        Ok(i) => {
            if i == code {
                Ok(())
            } else {
                println!("err: end with {i}");
                Err(())
            }
        }
        Err(errs) => {
            errs.fmt_all_stderr();
            Err(())
        }
    }
}

pub(crate) fn expect_failure(file_path: &'static str, errs_len: usize) -> Result<(), ()> {
    match exec_vm(file_path) {
        Ok(0) => Err(()),
        Ok(_) => Ok(()),
        Err(errs) => {
            errs.fmt_all_stderr();
            if errs.len() == errs_len {
                Ok(())
            } else {
                println!("err: error length is not {errs_len} but {}", errs.len());
                Err(())
            }
        }
    }
}

/// The test is intend to run only on 3.11 for fast execution.
/// To execute on other versions, change the version and magic number.
fn _exec_vm(file_path: &'static str) -> Result<i32, CompileErrors> {
    println!("{GREEN}[test] exec {file_path}{RESET}");
    let mut cfg = ErgConfig::with_main_path(PathBuf::from(file_path));
    cfg.py_command = if cfg!(windows) {
        Some("python")
    } else {
        Some("python3")
    };
    // cfg.target_version = Some(PythonVersion::new(3, Some(8), Some(10))); // your Python's version
    // cfg.py_magic_num = Some(3413); // in (most) 3.8.x
    // cfg.target_version = Some(PythonVersion::new(3, Some(9), Some(0)));
    // cfg.py_magic_num = Some(3425); // in (most) 3.9.x
    // cfg.target_version = Some(PythonVersion::new(3, Some(10), Some(6)));
    // cfg.py_magic_num = Some(3439); // in (most) 3.10.x
    cfg.target_version = Some(PythonVersion::new(3, Some(11), Some(0)));
    cfg.py_magic_num = Some(3495); // in 3.11.0
    let mut vm = DummyVM::new(cfg);
    vm.exec()
}

pub(crate) fn exec_vm(file_path: &'static str) -> Result<i32, CompileErrors> {
    exec_new_thread(move || _exec_vm(file_path))
}
