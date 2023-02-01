use crate::eval::{eval, successful_output};

#[test]
fn eval_string() {
    assert_eq!(
        eval("assert \"abcdef\" == \"abcdef\""),
        successful_output("")
    );
    assert_eq!(
        eval("assert \"1234567890ABC\" == \"\\{1234567890}ABC\""),
        successful_output("")
    );
    assert_eq!(eval("print! \"\""), successful_output("\n"));
    assert_eq!(eval("assert \"\" == \"\""), successful_output(""));
    assert_eq!(eval("print! \"A\""), successful_output("A\n"));
    assert_eq!(
        eval(r#"a = 10;print! "2 * 5 = \{a}""#),
        successful_output("2 * 5 = 10\n")
    );
    assert_eq!(
        eval(r#"print! "    \{"b"}\{False} \{[1]}""#),
        successful_output("    bFalse [1]\n")
    );
    assert_eq!(
        eval(r#"print! "a\{"b"}c\{"d \{" e\{"f"}g\{-2+3}"}"}""#),
        successful_output("abcd  efg1\n")
    );
    assert_eq!(
        eval(
            r#"print! """abc
def
  ghi
j kl """"#
        ),
        successful_output("abc\ndef\n  ghi\nj kl \n")
    );
    assert_eq!(
        eval(
            r#"print! """
  \{()}
a
""""#
        ),
        successful_output("\n  ()\na\n\n")
    );
    // TODO: more diverse characters

    {
        let output = eval("assert \"abcde\" == \"abcdef\"");
        assert_eq!(output.stdout, "");
        assert!(!output.stderr.is_empty());
        assert_eq!(output.status_code, Some(1));
    }
    assert_eq!(eval("print! \"\\\"").status_code, Some(1));
}

#[test]
fn eval_int() {
    assert_eq!(eval("assert 99 == 99"), successful_output(""));
    assert_eq!(eval("print! 256"), successful_output("256\n"));
    // assert_eq!(eval_code("assert -2 == -2"), success_output("")); // failed
    assert_eq!(eval("print! 0"), successful_output("0\n"));
    // assert_eq!(eval_code("print! -1000"), success_output("-1000\n")); // failed
    assert_eq!(eval("print! 0 == 0"), successful_output("True\n"));
    assert_eq!(eval("print! 2147483647"), successful_output("2147483647\n"));
    // assert_eq!(eval("print! 2147483648"), successful_output("2147483648\n")); // should be ok?
    assert_eq!(eval("print!(-2147483648)"), successful_output("-2147483648\n"));
    // assert_eq!(eval("print!(-2147483649)"), successful_output("-2147483649\n")); // should be ok?


    {
        let result = eval("assert 100 == 1000");
        assert_eq!(result.stdout, "");
        assert!(!result.stderr.is_empty());
        assert_eq!(result.status_code, Some(1));
    }
    assert_eq!(eval("assert 10 == 11").status_code, Some(1));
}

#[test]
fn eval_ratio() {
    assert_eq!(eval("print! 0.1234"), successful_output("0.1234\n"));
}
