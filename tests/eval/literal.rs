use crate::eval::eval_code;

#[test]
fn eval_int() {
    assert!(eval_code("assert 100 == 100").status.success());
    assert_eq!(eval_code("print! \"abc\"").stdout, "abc\n");
    assert_eq!(eval_code("print! \"0.3\"").stdout, "0.3\n");
}
