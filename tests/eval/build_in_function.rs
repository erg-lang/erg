//! for performance, 1 function per 1~2 test

use crate::eval::{eval, successful_output};

#[test]
#[ignore]
fn eval_print_1() {
    assert_eq!(eval("print! 1"), successful_output("1\n"));
}

#[test]
#[ignore]
fn eval_print_str_1() {
    assert_eq!(eval("print! \"abc\""), successful_output("abc\n"));
}

#[test]
#[ignore]
fn eval_print_str_2() {
    assert_eq!(eval("print!(\"a\")"), successful_output("a\n"));
}

#[test]
#[ignore]
fn eval_print_ratio() {
    assert_eq!(eval("print! \"0.3\""), successful_output("0.3\n"));
}

#[test]
#[ignore]
fn eval_print_bool() {
    assert_eq!(eval("print! True"), successful_output("True\n"));
}

#[test]
#[ignore]
fn eval_print_unit() {
    assert_eq!(eval("print! (())"), successful_output("()\n"));
}

#[test]
#[ignore]
fn eval_interpolation_1() {
    assert_eq!(
        eval("world = \"world\"\nprint! \"hello \\{world}\""),
        successful_output("hello world\n")
    );
}

#[test]
#[ignore]
fn eval_interpolation_2() {
    assert_eq!(eval("print! \"\\{0.005}\""), successful_output("1/200\n"));
}

#[test]
#[ignore]
fn eval_multiline_str() {
    assert_eq!(
        eval(
            r#"print! """A
B""", "C", """
D""""#
        ),
        successful_output("A\nB C \nD\n")
    );
}

#[test]
#[ignore]
fn eval_keyword_call() {
    assert_eq!(
        eval("print! \"a\", \"b\", 3, end := \"\""),
        successful_output("a b 3")
    );
}

#[test]
#[ignore]
fn eval_invalid_print() {
    let output = eval("print 1"); // print! is correct
    assert_eq!(output.stdout, "");
    assert!(!output.stderr.is_empty());
    assert_eq!(output.status_code, Some(1));
}

#[test]
#[ignore]
fn eval_assign_and_print() {
    assert_eq!(eval("num = -3\nprint! num * 2").stdout, "-6\n");
}

#[test]
#[ignore]
fn eval_assert_true() {
    assert_eq!(eval("assert True"), successful_output(""));
}

#[test]
#[ignore]
fn eval_assert_1() {
    assert_eq!(eval("assert 1"), successful_output(""));
}

#[test]
#[ignore]
fn eval_assign_and_assert() {
    assert_eq!(eval("flag = True\nassert flag"), successful_output(""));
}

#[test]
#[ignore]
fn eval_assert_false() {
    let output = eval("assert False");
    assert_eq!(output.stdout, "");
    assert!(!output.stderr.is_empty());
    assert_eq!(output.status_code, Some(1));
}

#[test]
#[ignore]
fn eval_assert_0point2() {
    assert_eq!(eval("assert 0.2").status_code, Some(1));
}

#[test]
#[ignore]
fn eval_invalid_assert() {
    assert_eq!(eval("assert! True").status_code, Some(1));
}
