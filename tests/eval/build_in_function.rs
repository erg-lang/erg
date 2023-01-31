use crate::eval::{eval, successful_output};

#[test]
fn eval_print() {
    assert_eq!(eval("print! 1"), successful_output("1\n"));
    assert_eq!(eval("print! \"abc\""), successful_output("abc\n"));
    assert_eq!(
        eval("world = \"world\"\nprint! \"hello \\{world}\""),
        successful_output("hello world\n")
    );
    assert_eq!(eval("print! \"0.3\""), successful_output("0.3\n"));
    assert_eq!(eval("print! True"), successful_output("True\n"));
    assert_eq!(eval("print! (())"), successful_output("()\n"));
    assert_eq!(eval("print! \"\\{0.005}\""), successful_output("0.005\n"));
    assert_eq!(
        eval(
            r#"print! """A
B""", "C", """
D""""#
        ),
        successful_output("A\nB C \nD\n")
    );
    assert_eq!(eval("print!(\"a\")"), successful_output("a\n"));
    assert_eq!(
        eval("print! \"a\", \"b\", 3, end := \"\""),
        successful_output("a b 3")
    );

    {
        let output = eval("print 1");
        assert_eq!(output.stdout, "");
        assert!(!output.stderr.is_empty());
        assert_eq!(output.status_code, Some(1));
    }
    assert_eq!(eval("num = -3\nprint! num * 2").stdout, "-6\n");
}

#[test]
fn eval_assert() {
    assert_eq!(eval("assert True"), successful_output(""));
    assert_eq!(eval("assert 1"), successful_output(""));
    assert_eq!(eval("flag = True\nassert flag"), successful_output(""));

    {
        let output = eval("assert False");
        assert_eq!(output.stdout, "");
        assert!(!output.stderr.is_empty());
        assert_eq!(output.status_code, Some(1));
    }
    assert_eq!(eval("assert 0.2").status_code, Some(1));
    assert_eq!(eval("assert! True").status_code, Some(1));
}
