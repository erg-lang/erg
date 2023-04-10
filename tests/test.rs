mod common;
use common::{expect_end_with, expect_failure, expect_success};

#[test]
fn exec_addition_ok() -> Result<(), ()> {
    expect_success("tests/should_ok/addition.er", 0)
}

#[test]
fn exec_advanced_type_spec() -> Result<(), ()> {
    expect_success("tests/should_ok/advanced_type_spec.er", 3)
}

#[test]
fn exec_array() -> Result<(), ()> {
    expect_success("tests/should_ok/array.er", 0)
}

#[test]
fn exec_class() -> Result<(), ()> {
    expect_success("examples/class.er", 0)
}

#[test]
fn exec_class_attr() -> Result<(), ()> {
    expect_success("tests/should_ok/class_attr.er", 2)
}

#[test]
fn exec_collection() -> Result<(), ()> {
    expect_success("tests/should_ok/collection.er", 0)
}

#[test]
fn exec_comment() -> Result<(), ()> {
    expect_success("tests/should_ok/comment.er", 0)
}

#[test]
fn exec_control() -> Result<(), ()> {
    expect_success("examples/control.er", 2)
}

#[test]
fn exec_control_expr() -> Result<(), ()> {
    expect_success("tests/should_ok/control_expr.er", 3)
}

#[test]
fn exec_dict() -> Result<(), ()> {
    expect_success("examples/dict.er", 0)
}

#[test]
fn exec_fib() -> Result<(), ()> {
    expect_success("examples/fib.er", 0)
}

#[test]
fn exec_helloworld() -> Result<(), ()> {
    // HACK: When running the test with pre-commit, the exit code is 1 (the cause is unknown)
    if cfg!(feature = "pre-commit") {
        expect_end_with("examples/helloworld.er", 1)
    } else {
        expect_success("examples/helloworld.er", 0)
    }
}

#[test]
fn exec_if() -> Result<(), ()> {
    expect_success("tests/should_ok/if.er", 0)
}

#[test]
fn exec_impl() -> Result<(), ()> {
    expect_success("examples/impl.er", 0)
}

#[test]
fn exec_import() -> Result<(), ()> {
    // 1 warn: a11y
    expect_success("examples/import.er", 1)
}

#[test]
fn exec_inherit() -> Result<(), ()> {
    expect_success("tests/should_ok/inherit.er", 0)
}

#[test]
fn exec_infer_class() -> Result<(), ()> {
    expect_success("tests/should_ok/infer_class.er", 0)
}

#[test]
fn exec_infer_trait() -> Result<(), ()> {
    expect_success("tests/should_ok/infer_trait.er", 0)
}

#[test]
fn exec_int() -> Result<(), ()> {
    expect_success("tests/should_ok/int.er", 0)
}

#[test]
fn exec_interpolation() -> Result<(), ()> {
    expect_success("tests/should_ok/interpolation.er", 0)
}

#[test]
fn exec_long() -> Result<(), ()> {
    expect_success("tests/should_ok/long.er", 257)
}

#[test]
fn exec_mut() -> Result<(), ()> {
    expect_success("examples/mut.er", 0)
}

#[test]
fn exec_mut_array() -> Result<(), ()> {
    expect_success("tests/should_ok/mut_array.er", 0)
}

#[test]
fn exec_nested() -> Result<(), ()> {
    expect_success("tests/should_ok/nested.er", 3)
}

#[test]
fn exec_patch() -> Result<(), ()> {
    expect_success("examples/patch.er", 0)
}

#[test]
fn exec_pattern() -> Result<(), ()> {
    expect_success("tests/should_ok/pattern.er", 0)
}

#[test]
fn exec_pyimport_test() -> Result<(), ()> {
    expect_success("tests/should_ok/pyimport.er", 2)
}

#[test]
fn exec_quantified() -> Result<(), ()> {
    expect_success("examples/quantified.er", 1)
}

#[test]
fn exec_raw_ident() -> Result<(), ()> {
    expect_success("examples/raw_ident.er", 1)
}

#[test]
fn exec_rec() -> Result<(), ()> {
    expect_success("tests/should_ok/rec.er", 0)
}

#[test]
fn exec_record() -> Result<(), ()> {
    expect_success("examples/record.er", 0)
}

#[test]
fn exec_return() -> Result<(), ()> {
    expect_success("tests/should_ok/return.er", 0)
}

#[test]
fn exec_structural_example() -> Result<(), ()> {
    expect_success("examples/structural.er", 0)
}

#[test]
fn exec_structural() -> Result<(), ()> {
    expect_success("tests/should_ok/structural.er", 0)
}

#[test]
fn exec_subtyping() -> Result<(), ()> {
    expect_success("tests/should_ok/subtyping.er", 0)
}

#[test]
fn exec_trait() -> Result<(), ()> {
    expect_success("examples/trait.er", 0)
}

#[test]
fn exec_tuple() -> Result<(), ()> {
    expect_success("examples/tuple.er", 0)
}

#[test]
fn exec_unpack() -> Result<(), ()> {
    expect_success("examples/unpack.er", 0)
}

#[test]
fn exec_use_py() -> Result<(), ()> {
    expect_success("examples/use_py.er", 0)
}

#[test]
fn exec_var_args() -> Result<(), ()> {
    expect_success("tests/should_ok/var_args.er", 0)
}

#[test]
fn exec_with() -> Result<(), ()> {
    expect_success("examples/with.er", 0)
}

#[test]
fn exec_addition_err() -> Result<(), ()> {
    expect_failure("tests/should_err/addition.er", 3, 9)
}

#[test]
fn exec_args() -> Result<(), ()> {
    expect_failure("tests/should_err/args.er", 0, 16)
}

#[test]
fn exec_array_err() -> Result<(), ()> {
    expect_failure("examples/array.er", 0, 1)
}

#[test]
fn exec_assert_cast() -> Result<(), ()> {
    expect_failure("examples/assert_cast.er", 0, 2)
}

#[test]
fn exec_collection_err() -> Result<(), ()> {
    expect_failure("tests/should_err/collection.er", 0, 4)
}

#[test]
fn exec_dependent() -> Result<(), ()> {
    expect_failure("tests/should_err/dependent.er", 0, 2)
}

/// This file compiles successfully, but causes a run-time error due to incomplete method dispatching
#[test]
fn exec_tests_impl() -> Result<(), ()> {
    expect_end_with("tests/should_ok/impl.er", 1)
}

#[test]
fn exec_impl_err() -> Result<(), ()> {
    expect_failure("tests/should_err/impl.er", 2, 2)
}

#[test]
fn exec_infer_union_array() -> Result<(), ()> {
    expect_failure("tests/should_err/infer_union_array.er", 2, 1)
}

#[test]
fn exec_invalid_interpol() -> Result<(), ()> {
    expect_failure("tests/should_err/invalid_interpol.er", 0, 2)
}

#[test]
fn exec_invalid_param() -> Result<(), ()> {
    expect_failure("tests/should_err/invalid_param.er", 0, 3)
}

#[test]
fn exec_move_check() -> Result<(), ()> {
    expect_failure("examples/move_check.er", 1, 1)
}

#[test]
fn exec_pyimport() -> Result<(), ()> {
    if cfg!(unix) {
        expect_end_with("examples/pyimport.er", 111)
    } else {
        expect_failure("examples/pyimport.er", 8, 1)
    }
}

#[test]
fn exec_set() -> Result<(), ()> {
    expect_failure("examples/set.er", 3, 1)
}

#[test]
fn exec_side_effect() -> Result<(), ()> {
    expect_failure("examples/side_effect.er", 5, 4)
}

#[test]
fn exec_structural_err() -> Result<(), ()> {
    expect_failure("tests/should_err/structural.er", 1, 9)
}

#[test]
fn exec_subtyping_err() -> Result<(), ()> {
    expect_failure("tests/should_err/subtyping.er", 0, 11)
}

#[test]
fn exec_callable() -> Result<(), ()> {
    expect_failure("tests/should_err/callable.er", 0, 4)
}

#[test]
fn exec_multiline_invalid_next() -> Result<(), ()> {
    expect_failure("tests/should_err/multi_line_invalid_nest.er", 0, 1)
}

#[test]
fn exec_quantified_err() -> Result<(), ()> {
    expect_failure("tests/should_err/quantified.er", 0, 3)
}

#[test]
fn exec_refinement() -> Result<(), ()> {
    expect_failure("tests/should_err/refinement.er", 0, 4)
}

#[test]
fn exec_var_args_err() -> Result<(), ()> {
    expect_failure("tests/should_err/var_args.er", 0, 3)
}

#[test]
fn exec_visibility() -> Result<(), ()> {
    expect_failure("tests/should_err/visibility.er", 2, 6)
}

#[test]
fn exec_move() -> Result<(), ()> {
    expect_failure("tests/should_err/move.er", 1, 1)
}
