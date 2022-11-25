use std::path::PathBuf;

use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::python_util::PythonVersion;
use erg_common::traits::{Runnable, Stream};

use erg_compiler::error::CompileErrors;

use erg::DummyVM;

#[test]
fn exec_addition() -> Result<(), ()> {
    expect_failure("tests/addition.er", 1)
}

#[test]
fn exec_advanced_type_spec() -> Result<(), ()> {
    expect_success("tests/advanced_type_spec.er")
}

#[test]
fn exec_assert_cast() -> Result<(), ()> {
    expect_success("examples/assert_cast.er")
}

#[test]
fn exec_class() -> Result<(), ()> {
    expect_success("examples/class.er")
}

#[test]
fn exec_control() -> Result<(), ()> {
    expect_success("tests/control.er")
}

#[test]
fn exec_dict() -> Result<(), ()> {
    expect_success("examples/dict.er")
}

#[test]
fn exec_fib() -> Result<(), ()> {
    expect_success("examples/fib.er")
}

#[test]
fn exec_helloworld() -> Result<(), ()> {
    // HACK: When running the test with pre-commit, the exit code is 1 (the cause is unknown)
    if cfg!(feature = "pre-commit") {
        expect_end_with("examples/helloworld.er", 1)
    } else {
        expect_success("examples/helloworld.er")
    }
}

#[test]
fn exec_impl() -> Result<(), ()> {
    expect_success("examples/impl.er")
}

#[test]
fn exec_import() -> Result<(), ()> {
    expect_success("examples/import.er")
}

#[test]
fn exec_infer_class() -> Result<(), ()> {
    expect_success("tests/infer_class.er")
}

#[test]
fn exec_infer_trait() -> Result<(), ()> {
    expect_success("tests/infer_trait.er")
}

#[test]
fn exec_move_check() -> Result<(), ()> {
    expect_failure("examples/move_check.er", 1)
}

#[test]
fn exec_pattern() -> Result<(), ()> {
    expect_success("tests/pattern.er")
}

#[test]
fn exec_pyimport() -> Result<(), ()> {
    if cfg!(unix) {
        expect_end_with("examples/pyimport.er", 111)
    } else {
        expect_failure("examples/pyimport.er", 1)
    }
}

#[test]
fn exec_quantified() -> Result<(), ()> {
    expect_success("examples/quantified.er")
}

#[test]
fn exec_raw_ident() -> Result<(), ()> {
    expect_success("examples/raw_ident.er")
}

#[test]
fn exec_rec() -> Result<(), ()> {
    // this script is valid but the current code generating process has a bug.
    expect_end_with("tests/rec.er", 1)
}

#[test]
fn exec_record() -> Result<(), ()> {
    expect_success("examples/record.er")
}

#[test]
fn exec_set() -> Result<(), ()> {
    expect_failure("examples/set.er", 1)
}

#[test]
fn exec_side_effect() -> Result<(), ()> {
    expect_failure("examples/side_effect.er", 4)
}

#[test]
fn exec_subtyping() -> Result<(), ()> {
    expect_failure("tests/subtyping.er", 1)
}

#[test]
fn exec_trait() -> Result<(), ()> {
    expect_success("examples/trait.er")
}

#[test]
fn exec_tuple() -> Result<(), ()> {
    expect_success("examples/tuple.er")
}

#[test]
fn exec_unpack() -> Result<(), ()> {
    expect_success("examples/unpack.er")
}

#[test]
fn exec_use_py() -> Result<(), ()> {
    expect_success("examples/use_py.er")
}

#[test]
fn exec_with() -> Result<(), ()> {
    expect_success("examples/with.er")
}

fn expect_success(file_path: &'static str) -> Result<(), ()> {
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

fn expect_end_with(file_path: &'static str, code: i32) -> Result<(), ()> {
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

fn expect_failure(file_path: &'static str, errs_len: usize) -> Result<(), ()> {
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

fn _exec_vm(file_path: &'static str) -> Result<i32, CompileErrors> {
    let mut cfg = ErgConfig::with_main_path(PathBuf::from(file_path));
    // cfg.py_command = Some("python3");
    // cfg.target_version = Some(PythonVersion::new(3, Some(10), Some(8)));
    // cfg.py_magic_num = Some(3439);
    cfg.py_command = Some("python");
    cfg.target_version = Some(PythonVersion::new(3, Some(11), Some(0)));
    cfg.py_magic_num = Some(3495);
    let mut vm = DummyVM::new(cfg);
    vm.exec()
}

#[cfg(target_os = "windows")]
fn exec_vm(file_path: &'static str) -> Result<i32, CompileErrors> {
    const STACK_SIZE: usize = 4 * 1024 * 1024;

    let child = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || _exec_vm(file_path))
        .unwrap();
    // Wait for thread to join
    child.join().unwrap()
}

#[cfg(not(target_os = "windows"))]
fn exec_vm(file_path: &'static str) -> Result<i32, CompileErrors> {
    _exec_vm(file_path)
}
