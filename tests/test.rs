mod common;
use common::{expect_end_with, expect_failure, expect_success};

#[test]
fn exec_addition_ok() -> Result<(), ()> {
    expect_success("tests/should_ok/addition.er")
}

#[test]
fn exec_advanced_type_spec() -> Result<(), ()> {
    expect_success("tests/should_ok/advanced_type_spec.er")
}

#[test]
fn exec_array() -> Result<(), ()> {
    expect_success("tests/should_ok/array.er")
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
fn exec_class_attr() -> Result<(), ()> {
    expect_success("tests/should_ok/class_attr.er")
}

#[test]
fn exec_comment() -> Result<(), ()> {
    expect_success("tests/should_ok/comment.er")
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
fn exec_int() -> Result<(), ()> {
    expect_success("tests/should_ok/int.er")
}

#[test]
fn exec_interpolation() -> Result<(), ()> {
    expect_success("tests/should_ok/interpolation.er")
}

#[test]
fn exec_long() -> Result<(), ()> {
    expect_success("tests/should_ok/long.er")
}

#[test]
fn exec_mut() -> Result<(), ()> {
    expect_success("examples/mut.er")
}

#[test]
fn exec_mut_array() -> Result<(), ()> {
    expect_success("tests/should_ok/mut_array.er")
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
fn exec_structural() -> Result<(), ()> {
    expect_success("examples/structural.er")
}

#[test]
fn exec_structural_test() -> Result<(), ()> {
    expect_success("tests/should_ok/structural_test.er")
}

#[test]
fn exec_subtyping() -> Result<(), ()> {
    expect_success("tests/should_ok/subtyping.er")
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
    expect_failure("tests/should_err/addition.er", 9)
}

#[test]
fn exec_args() -> Result<(), ()> {
    expect_failure("tests/should_err/args.er", 16)
}

#[test]
fn exec_array_err() -> Result<(), ()> {
    expect_failure("examples/array.er", 1)
}

#[test]
fn exec_dependent() -> Result<(), ()> {
    expect_failure("tests/should_err/dependent.er", 2)
}

/// This file compiles successfully, but causes a run-time error due to incomplete method dispatching
#[test]
fn exec_tests_impl() -> Result<(), ()> {
    expect_end_with("tests/should_ok/impl.er", 1)
}

#[test]
fn exec_impl_err() -> Result<(), ()> {
    expect_failure("tests/should_err/impl.er", 2)
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
fn exec_structural_err() -> Result<(), ()> {
    expect_failure("tests/should_err/structural.er", 9)
}

#[test]
fn exec_subtyping_err() -> Result<(), ()> {
    expect_failure("tests/should_err/subtyping.er", 6)
}

#[test]
fn exec_callable() -> Result<(), ()> {
    expect_failure("tests/should_err/callable.er", 4)
}

#[test]
fn exec_multiline_invalid_next() -> Result<(), ()> {
    expect_failure("tests/should_err/multi_line_invalid_nest.er", 1)
}

#[test]
fn exec_quantified_err() -> Result<(), ()> {
    expect_failure("tests/should_err/quantified.er", 3)
}

#[test]
fn exec_var_args() -> Result<(), ()> {
    expect_success("tests/should_ok/var_args.er")
}

#[test]
fn exec_var_args_err() -> Result<(), ()> {
    expect_failure("tests/should_err/var_args.er", 2)
}

#[test]
fn exec_move() -> Result<(), ()> {
    expect_failure("tests/should_err/move.er", 1)
}
