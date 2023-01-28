use crate::eval::eval_code;

#[test]
fn eval_print() {
    assert_eq!(eval_code("print! 1").stdout, "1\n");
    assert!(!eval_code("print 0").stderr.is_empty());
    assert_eq!(eval_code("print! \"abc\"").stdout, "abc\n");
    assert_eq!(eval_code("print! \"0.3\"").stdout, "0.3\n");
    assert_eq!(eval_code("num = -3\nprint! num * 2").stdout, "-6\n");
    assert_eq!(eval_code("print True").status.code().unwrap(), 1);
}

#[test]
fn eval_assert() {
    assert!(eval_code("assert True").status.success());
    assert!(!eval_code("assert False").status.success());
    assert_eq!(eval_code("assert 1").status.code().unwrap(), 0);
    assert!(!eval_code("assert 0.2").status.success());
    assert!(eval_code("flag = True\nassert flag").status.success());
}
