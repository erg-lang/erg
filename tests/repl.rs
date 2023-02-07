mod common;
use common::expect_repl_success;

#[test]
fn exec_repl_helloworld() -> Result<(), ()> {
    expect_repl_success(
        "repl_hello",
        vec!["print! \"hello, world\"".into(), "exit()".into()],
    )
}

#[test]
fn exec_repl_def_func() -> Result<(), ()> {
    expect_repl_success(
        "repl_def",
        vec![
            "f i =".into(),
            "    i + 1".into(),
            "".into(),
            "x = f 2".into(),
            "assert x == 3".into(),
            "exit()".into(),
        ],
    )
}
