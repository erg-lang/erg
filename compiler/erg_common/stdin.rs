use std::cell::RefCell;
use std::io::{stdin, BufRead, BufReader};
use std::thread::LocalKey;

use crate::Str;

pub struct StdinReader {
    pub lineno: usize,
    buf: Vec<Str>,
}

impl StdinReader {
    pub fn read(&mut self) -> Str {
        let mut buf = "".to_string();
        let stdin = stdin();
        let mut reader = BufReader::new(stdin.lock());
        reader.read_line(&mut buf).unwrap();
        self.lineno += 1;
        self.buf.push(buf.into());
        self.buf.last().unwrap().clone()
    }

    pub fn reread(&self) -> Str {
        self.buf.last().unwrap().clone()
    }

    pub fn reread_lines(&self, ln_begin: usize, ln_end: usize) -> Vec<Str> {
        self.buf[ln_begin - 1..=ln_end - 1].to_vec()
    }
}

thread_local! {
    static READER: RefCell<StdinReader> = RefCell::new(StdinReader{ lineno: 0, buf: vec![] });
}

#[derive(Debug)]
pub struct GlobalStdin(LocalKey<RefCell<StdinReader>>);

pub static GLOBAL_STDIN: GlobalStdin = GlobalStdin(READER);

impl GlobalStdin {
    pub fn read(&'static self) -> Str {
        self.0.with(|s| s.borrow_mut().read())
    }

    pub fn reread(&'static self) -> Str {
        self.0.with(|s| s.borrow().reread())
    }

    pub fn reread_lines(&'static self, ln_begin: usize, ln_end: usize) -> Vec<Str> {
        self.0
            .with(|s| s.borrow_mut().reread_lines(ln_begin, ln_end))
    }
}
