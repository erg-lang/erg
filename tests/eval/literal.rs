use crate::eval::{eval, successful_output};

#[test]
fn eval_assert_str() {
    assert_eq!(
        eval("assert \"abcdef\" == \"abcdef\""),
        successful_output("")
    );
}

#[test]
fn eval_assert_interpolation() {
    assert_eq!(
        eval("assert \"1234567890ABC\" == \"\\{1234567890}ABC\""),
        successful_output("")
    );
}

#[test]
fn eval_print_empty() {
    assert_eq!(eval("print! \"\""), successful_output("\n"));
}

#[test]
fn eval_assert_empty() {
    assert_eq!(eval("assert \"\" == \"\""), successful_output(""));
}

#[test]
fn eval_assert_interpolation_2() {
    assert_eq!(
        eval(r#"a = 10;assert "\{2 * 5}" == "\{a}""#),
        successful_output("")
    );
}

#[test]
fn eval_interpolation() {
    assert_eq!(
        eval(r#"print! "    \{"b"}\{False} \{[1]}""#),
        successful_output("    bFalse [1]\n")
    );
}

#[test]
fn eval_interpolation_2() {
    assert_eq!(
        eval(r#"print! "a\{"b"}c\{"d \{" e\{"f"}g\{-2+3}"}"}""#),
        successful_output("abcd  efg1\n")
    );
}

#[test]
fn eval_multiline_string() {
    assert_eq!(
        eval(
            r#"print! """abc
def
    ghi
j kl """"#
        ),
        successful_output("abc\ndef\n    ghi\nj kl \n")
    );
}

#[test]
fn eval_multiline_string_interpolation() {
    assert_eq!(
        eval(
            r#"print! """
    \{()}
a
""""#
        ),
        successful_output("\n    ()\na\n\n")
    );
    // TODO: more diverse characters
}

#[test]
fn eval_invalid_assertion() {
    let output = eval("assert \"abcde\" == \"abcdef\"");
    assert_eq!(output.stdout, "");
    assert!(!output.stderr.is_empty());
    assert_eq!(output.status_code, Some(1));
}

#[test]
fn eval_invalid_closing_string() {
    assert_eq!(eval("print! \"\\\"").status_code, Some(1));
}

#[test]
fn eval_assert_99() {
    assert_eq!(eval("assert 99 == 99"), successful_output(""));
}

#[test]
fn eval_assert_minus2() {
    assert_eq!(eval("assert -2 == -2"), successful_output(""));
}

#[test]
fn eval_minus1000() {
    assert_eq!(eval("print! -1000"), successful_output("-1000\n"));
}

#[test]
fn eval_0_eq_0() {
    assert_eq!(eval("print! 0 == 0"), successful_output("True\n"));
}

// TODO: support big numbers
/*
#[test]
fn eval_bignum() {
    assert_eq!(eval("print! 214748364778473683657867814876187416"), successful_output("214748364778473683657867814876187416\n"));
}

#[test]
fn eval_neg_bignum() {
    assert_eq!(eval("print!(-214748364778473683657867814876187416)"), successful_output("-214748364778473683657867814876187416\n"));
}
*/

#[test]
fn eval_assert_inequality() {
    let result = eval("assert 100 == 1000");
    assert_eq!(result.stdout, "");
    assert!(!result.stderr.is_empty());
    assert_eq!(result.status_code, Some(1));
}

#[test]
fn eval_assert_inequality_2() {
    assert_eq!(eval("assert 10 == 11").status_code, Some(1));
}

#[test]
fn eval_ratio() {
    assert_eq!(eval("print! 0.1234"), successful_output("0.1234\n"));
}
