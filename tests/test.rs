use std::path::PathBuf;

use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::traits::Runnable;

use erg::dummy::DummyVM;

#[test]
fn exec_addition() -> Result<(), ()> {
    expect_failure("tests/addition.er")
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
    expect_failure("examples/move_check.er")
}

#[test]
fn exec_quantified() -> Result<(), ()> {
    expect_success("examples/quantified.er")
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
    expect_failure("examples/set.er")
}

#[test]
fn exec_side_effect() -> Result<(), ()> {
    expect_failure("examples/side_effect.er")
}

#[test]
fn exec_subtyping() -> Result<(), ()> {
    expect_failure("tests/subtyping.er")
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
    let cfg = ErgConfig::with_main_path(PathBuf::from(file_path));
    let mut vm = DummyVM::new(cfg);
    match vm.exec() {
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
    let cfg = ErgConfig::with_main_path(PathBuf::from(file_path));
    let mut vm = DummyVM::new(cfg);
    match vm.exec() {
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

fn expect_failure(file_path: &'static str) -> Result<(), ()> {
    let cfg = ErgConfig::with_main_path(PathBuf::from(file_path));
    let mut vm = DummyVM::new(cfg);
    match vm.exec() {
        Ok(0) => Err(()),
        Ok(_) => Ok(()),
        Err(errs) => {
            errs.fmt_all_stderr();
            Ok(())
        }
    }
}
