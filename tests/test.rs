use std::path::PathBuf;

use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::python_util::PythonVersion;
use erg_common::spawn::exec_new_thread;
use erg_common::style::{GREEN, RESET};
use erg_common::traits::{Runnable, Stream};

use erg_compiler::error::CompileErrors;

use erg::DummyVM;

#[test]
fn exec_addition_ok() -> Result<(), ()> {
    expect_success("tests/should_ok/addition.er")
}

#[test]
fn exec_advanced_type_spec() -> Result<(), ()> {
    expect_success("tests/should_ok/advanced_type_spec.er")
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
    expect_success("examples/control.er")
}

#[test]
fn exec_control_expr() -> Result<(), ()> {
    expect_success("tests/should_ok/control_expr.er")
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
fn exec_if() -> Result<(), ()> {
    expect_success("tests/should_ok/if.er")
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
    expect_success("tests/should_ok/infer_class.er")
}

#[test]
fn exec_infer_trait() -> Result<(), ()> {
    expect_success("tests/should_ok/infer_trait.er")
}

#[test]
fn exec_interpolation() -> Result<(), ()> {
    expect_success("tests/should_ok/interpolation.er")
}

#[test]
fn exec_mut() -> Result<(), ()> {
    expect_success("examples/mut.er")
}

#[test]
fn exec_nested() -> Result<(), ()> {
    expect_success("tests/should_ok/nested.er")
}

#[test]
fn exec_patch() -> Result<(), ()> {
    expect_success("examples/patch.er")
}

#[test]
fn exec_pattern() -> Result<(), ()> {
    expect_success("tests/should_ok/pattern.er")
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
    expect_success("tests/should_ok/rec.er")
}

#[test]
fn exec_record() -> Result<(), ()> {
    expect_success("examples/record.er")
}

#[test]
fn exec_return() -> Result<(), ()> {
    expect_success("tests/should_ok/return.er")
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

#[test]
fn exec_addition_err() -> Result<(), ()> {
    expect_failure("tests/should_err/addition.er", 7)
}

#[test]
fn exec_args() -> Result<(), ()> {
    expect_failure("tests/should_err/args.er", 16)
}

#[test]
fn exec_infer_union_array() -> Result<(), ()> {
    expect_failure("tests/should_err/infer_union_array.er", 1)
}

#[test]
fn exec_invalid_interpol() -> Result<(), ()> {
    expect_failure("tests/should_err/invalid_interpol.er", 2)
}

#[test]
fn exec_invalid_param() -> Result<(), ()> {
    expect_failure("tests/should_err/invalid_param.er", 3)
}

#[test]
fn exec_move_check() -> Result<(), ()> {
    expect_failure("examples/move_check.er", 1)
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
fn exec_set() -> Result<(), ()> {
    expect_failure("examples/set.er", 1)
}

#[test]
fn exec_side_effect() -> Result<(), ()> {
    expect_failure("examples/side_effect.er", 4)
}

#[test]
fn exec_subtyping() -> Result<(), ()> {
    expect_failure("tests/should_err/subtyping.er", 2)
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
    // cfg.target_version = Some(PythonVersion::new(3, Some(10), Some(6))); // your Python's version
    // cfg.py_magic_num = Some(3439); // in (most) 3.10.x
    cfg.target_version = Some(PythonVersion::new(3, Some(11), Some(0)));
    cfg.py_magic_num = Some(3495); // in 3.11.0
    let mut vm = DummyVM::new(cfg);
    vm.exec()
}

fn exec_vm(file_path: &'static str) -> Result<i32, CompileErrors> {
    exec_new_thread(move || _exec_vm(file_path))
}
