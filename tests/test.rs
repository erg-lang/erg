mod common;
use common::{expect_end_with, expect_error_location_and_msg, expect_failure, expect_success};
use erg_common::error::Location;
use erg_common::python_util::env_python_version;

#[test]
fn exec_addition_ok() -> Result<(), ()> {
    expect_success("tests/should_ok/addition.er", 0)
}

#[test]
fn exec_advanced_type_spec() -> Result<(), ()> {
    expect_success("tests/should_ok/advanced_type_spec.er", 5)
}

#[test]
fn exec_array() -> Result<(), ()> {
    expect_success("tests/should_ok/array.er", 0)
}

#[test]
fn exec_array_member() -> Result<(), ()> {
    expect_success("tests/should_ok/array_member.er", 0)
}

#[test]
fn exec_assert_cast_ok() -> Result<(), ()> {
    expect_success("tests/should_ok/assert_cast.er", 0)
}

#[test]
fn exec_associated_types() -> Result<(), ()> {
    expect_success("tests/should_ok/associated_types.er", 0)
}

#[test]
fn exec_class() -> Result<(), ()> {
    expect_success("examples/class.er", 0)
}

#[test]
fn exec_class_test() -> Result<(), ()> {
    expect_success("tests/should_ok/class.er", 0)
}

#[test]
fn exec_class_attr() -> Result<(), ()> {
    expect_success("tests/should_ok/class_attr.er", 2)
}

#[test]
fn exec_closure() -> Result<(), ()> {
    expect_success("tests/should_ok/closure.er", 0)
}

#[test]
fn exec_coercion() -> Result<(), ()> {
    expect_success("tests/should_ok/coercion.er", 0)
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
fn exec_comprehension() -> Result<(), ()> {
    expect_success("tests/should_ok/comprehension.er", 0)
}

#[test]
fn exec_comptime() -> Result<(), ()> {
    expect_success("tests/should_ok/comptime.er", 12)
}

#[test]
fn exec_container_class() -> Result<(), ()> {
    expect_success("tests/should_ok/container_class.er", 0)
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
fn exec_decimal() -> Result<(), ()> {
    expect_success("tests/should_ok/decimal.er", 0)
}

#[test]
fn exec_default_param() -> Result<(), ()> {
    expect_success("tests/should_ok/default_param.er", 0)
}

#[test]
fn exec_dependent() -> Result<(), ()> {
    expect_success("tests/should_ok/dependent.er", 0)
}

#[test]
fn exec_dict() -> Result<(), ()> {
    expect_success("examples/dict.er", 0)
}

#[test]
fn exec_dict_test() -> Result<(), ()> {
    expect_success("tests/should_ok/dict.er", 0)
}

#[test]
fn exec_empty_check() -> Result<(), ()> {
    expect_success("tests/should_ok/dyn_type_check.er", 0)
}

#[test]
fn exec_use_ansicolor() -> Result<(), ()> {
    expect_success("examples/use_ansicolor.er", 0)
}

#[test]
fn exec_use_exception() -> Result<(), ()> {
    expect_success("examples/use_exception.er", 0)
}

#[test]
fn exec_fast_value() -> Result<(), ()> {
    expect_success("tests/should_ok/fast_value.er", 0)
}

#[test]
fn exec_fib() -> Result<(), ()> {
    expect_success("examples/fib.er", 0)
}

#[test]
fn exec_helloworld() -> Result<(), ()> {
    // HACK: When running the test with Windows, the exit code is 1 (the cause is unknown)
    if cfg!(windows) && env_python_version().unwrap().minor >= Some(8) {
        expect_end_with("examples/helloworld.er", 0, 1)
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
    // 2 warns: a11y
    expect_success("examples/import.er", 2)
}

#[test]
fn exec_import_cyclic() -> Result<(), ()> {
    expect_success("tests/should_ok/cyclic/import.er", 0)
}

#[test]
fn exec_index() -> Result<(), ()> {
    expect_success("tests/should_ok/index.er", 0)
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
fn exec_infer_method() -> Result<(), ()> {
    expect_success("tests/should_ok/infer_method.er", 0)
}

#[test]
fn exec_infer_trait() -> Result<(), ()> {
    expect_success("tests/should_ok/infer_trait.er", 0)
}

#[test]
fn exec_init_del() -> Result<(), ()> {
    expect_success("examples/init_del.er", 0)
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
fn exec_list() -> Result<(), ()> {
    expect_success("examples/list.er", 0)
}

#[test]
fn exec_long() -> Result<(), ()> {
    expect_success("tests/should_ok/long.er", 257)
}

#[test]
fn exec_magic() -> Result<(), ()> {
    expect_success("examples/magic.er", 0)
}

#[test]
fn exec_mangling() -> Result<(), ()> {
    expect_success("tests/should_ok/mangling.er", 0)
}

#[test]
fn exec_many_import() -> Result<(), ()> {
    expect_success("tests/should_ok/many_import/many_import.er", 0)
}

#[test]
fn exec_map() -> Result<(), ()> {
    expect_success("tests/should_ok/map.er", 0)
}

#[test]
fn exec_match() -> Result<(), ()> {
    expect_success("tests/should_ok/match.er", 0)
}

#[test]
fn exec_method() -> Result<(), ()> {
    expect_success("tests/should_ok/method.er", 0)
}

#[test]
fn exec_move() -> Result<(), ()> {
    expect_success("tests/should_ok/move.er", 0)
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
fn exec_mut_dict() -> Result<(), ()> {
    expect_success("tests/should_ok/mut_dict.er", 0)
}

#[test]
fn exec_nested() -> Result<(), ()> {
    expect_success("tests/should_ok/nested.er", 3)
}

#[test]
fn exec_never() -> Result<(), ()> {
    expect_success("tests/should_ok/never.er", 0)
}

#[test]
fn exec_operators() -> Result<(), ()> {
    expect_success("tests/should_ok/operators.er", 0)
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
    // HACK: When running the test with Windows, the exit code is 1 (the cause is unknown)
    if cfg!(windows) && env_python_version().unwrap().minor < Some(8) {
        expect_end_with("tests/should_ok/pyimport.er", 2, 1)
    } else {
        expect_success("tests/should_ok/pyimport.er", 2)
    }
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
fn exec_record_err() -> Result<(), ()> {
    expect_failure("tests/should_err/record.er", 0, 1)
}

#[test]
fn exec_refinement() -> Result<(), ()> {
    expect_success("tests/should_ok/refinement.er", 0)
}

#[test]
fn exec_refinement_class() -> Result<(), ()> {
    expect_success("tests/should_ok/refinement_class.er", 0)
}

#[test]
fn exec_return() -> Result<(), ()> {
    expect_success("tests/should_ok/return.er", 0)
}

#[test]
fn exec_self_reference() -> Result<(), ()> {
    expect_success("tests/should_ok/self_reference.er", 0)
}

#[test]
fn exec_self_type() -> Result<(), ()> {
    expect_success("tests/should_ok/self_type.er", 0)
}

#[test]
fn exec_slice() -> Result<(), ()> {
    expect_success("tests/should_ok/slice.er", 0)
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
fn exec_sym_op() -> Result<(), ()> {
    expect_success("tests/should_ok/sym_op.er", 0)
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
fn exec_unit_test() -> Result<(), ()> {
    expect_success("examples/unit_test.er", 0)
}

#[test]
fn exec_unpack() -> Result<(), ()> {
    expect_success("examples/unpack.er", 0)
}

#[test]
fn exec_unused_import() -> Result<(), ()> {
    expect_success("tests/should_ok/many_import/unused_import.er", 2)
}

#[test]
fn exec_use_itertools() -> Result<(), ()> {
    expect_success("tests/should_ok/use_itertools.er", 0)
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
fn exec_var_kwargs() -> Result<(), ()> {
    expect_success("tests/should_ok/var_kwargs.er", 0)
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
fn exec_advanced_type_spec_err() -> Result<(), ()> {
    expect_failure("tests/should_err/advanced_type_spec.er", 0, 1)
}

#[test]
fn exec_args() -> Result<(), ()> {
    expect_failure("tests/should_err/args.er", 0, 19)
}

#[test]
fn exec_array_err() -> Result<(), ()> {
    expect_failure("examples/array.er", 0, 1)
}

#[test]
fn exec_array_member_err() -> Result<(), ()> {
    expect_failure("tests/should_err/array_member.er", 0, 3)
}

#[test]
fn exec_as() -> Result<(), ()> {
    expect_failure("tests/should_err/as.er", 0, 6)
}

#[test]
fn exec_assert_cast() -> Result<(), ()> {
    expect_failure("examples/assert_cast.er", 0, 3)
}

#[test]
fn exec_assert_cast_err() -> Result<(), ()> {
    expect_end_with("tests/should_err/assert_cast.er", 0, 1)
}

#[test]
fn exec_class_attr_err() -> Result<(), ()> {
    expect_failure("tests/should_err/class_attr.er", 1, 1)
}

#[test]
fn exec_collection_err() -> Result<(), ()> {
    expect_failure("tests/should_err/collection.er", 0, 5)
}

#[test]
fn exec_dependent_err() -> Result<(), ()> {
    expect_failure("tests/should_err/dependent.er", 0, 5)
}

#[test]
fn exec_dict_err() -> Result<(), ()> {
    expect_failure("tests/should_err/dict.er", 0, 2)
}

#[test]
fn exec_err_import() -> Result<(), ()> {
    expect_failure("tests/should_err/err_import.er", 0, 9)
}

/// This file compiles successfully, but causes a run-time error due to incomplete method dispatching
#[test]
fn exec_tests_impl() -> Result<(), ()> {
    expect_end_with("tests/should_ok/impl.er", 0, 1)
}

#[test]
fn exec_impl_err() -> Result<(), ()> {
    expect_failure("tests/should_err/impl.er", 2, 2)
}

#[test]
fn exec_import_err() -> Result<(), ()> {
    expect_failure("tests/should_err/import.er", 0, 2)
}

#[test]
fn exec_import_cyclic_err() -> Result<(), ()> {
    expect_failure("tests/should_err/cyclic/import.er", 0, 1)
}

#[test]
fn exec_infer_union_array() -> Result<(), ()> {
    expect_failure("tests/should_err/infer_union_array.er", 2, 1)
}

#[test]
fn exec_init_del_err() -> Result<(), ()> {
    expect_failure("tests/should_err/init_del.er", 0, 1)
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
fn exec_iterator() -> Result<(), ()> {
    expect_success("tests/should_ok/iterator.er", 0)
}

#[test]
fn exec_move_check() -> Result<(), ()> {
    expect_failure("examples/move_check.er", 1, 1)
}

#[test]
fn exec_pyimport() -> Result<(), ()> {
    if cfg!(unix) {
        expect_end_with("examples/pyimport.er", 8, 111)
    } else {
        expect_failure("examples/pyimport.er", 8, 1)
    }
}

#[test]
fn exec_pyimport_err() -> Result<(), ()> {
    expect_failure("tests/should_err/pyimport.er", 0, 2)
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
fn exec_side_effect_test() -> Result<(), ()> {
    expect_failure("tests/should_err/side_effect.er", 6, 5)
}

#[test]
fn exec_structural_err() -> Result<(), ()> {
    expect_failure("tests/should_err/structural.er", 1, 9)
}

#[test]
fn exec_subtyping_err() -> Result<(), ()> {
    // NOTE: The content of some errors is semantically redundant and can be reduced.
    expect_failure("tests/should_err/subtyping.er", 0, 18)
}

#[test]
fn exec_tuple_err() -> Result<(), ()> {
    expect_failure("tests/should_err/tuple.er", 0, 1)
}

#[test]
fn exec_callable() -> Result<(), ()> {
    expect_failure("tests/should_err/callable.er", 0, 6)
}

#[test]
fn exec_method_err() -> Result<(), ()> {
    expect_failure("tests/should_err/method.er", 0, 2)
}

#[test]
fn exec_move_err() -> Result<(), ()> {
    expect_failure("tests/should_err/move.er", 1, 2)
}

#[test]
fn exec_multiline_invalid_next() -> Result<(), ()> {
    expect_failure("tests/should_err/multi_line_invalid_nest.er", 0, 1)
}

#[test]
fn exec_mut_err() -> Result<(), ()> {
    expect_failure("tests/should_err/mut.er", 0, 1)
}

#[test]
fn exec_mut_array_err() -> Result<(), ()> {
    expect_failure("tests/should_err/mut_array.er", 0, 5)
}

#[test]
fn exec_mut_dict_err() -> Result<(), ()> {
    expect_failure("tests/should_err/mut_dict.er", 0, 3)
}

#[test]
fn exec_quantified_err() -> Result<(), ()> {
    expect_failure("tests/should_err/quantified.er", 0, 3)
}

#[test]
fn exec_recursive_fn_err() -> Result<(), ()> {
    expect_failure("tests/should_err/recursive_fn.er", 0, 2)
}

#[test]
fn exec_refinement_err() -> Result<(), ()> {
    expect_failure("tests/should_err/refinement.er", 0, 9)
}

#[test]
fn exec_refinement_class_err() -> Result<(), ()> {
    expect_failure("tests/should_err/refinement_class.er", 0, 2)
}

#[test]
fn exec_var_args_err() -> Result<(), ()> {
    expect_failure("tests/should_err/var_args.er", 0, 3)
}

#[test]
fn exec_var_kwargs_err() -> Result<(), ()> {
    expect_failure("tests/should_err/var_kwargs.er", 0, 2)
}

#[test]
fn exec_visibility() -> Result<(), ()> {
    expect_failure("tests/should_err/visibility.er", 2, 7)
}

#[test]
fn exec_err_loc() -> Result<(), ()> {
    expect_error_location_and_msg(
        "tests/should_err/err_loc.er",
        vec![
            (Location::range(2, 11, 2, 16), None),
            (Location::range(7, 11, 7, 12), None),
            (
                Location::range(13, 21, 13, 27),
                Some("Int object has no attribute method"),
            ),
            (Location::range(10, 11, 10, 16), None),
        ],
    )
}

#[test]
fn test_semver() -> Result<(), ()> {
    expect_success("crates/erg_compiler/lib/std/semver.er", 0)
}
