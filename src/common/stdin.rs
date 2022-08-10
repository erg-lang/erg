use std::io::{stdin, BufReader, BufRead};
use std::cell::RefCell;

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
        self.buf[ln_begin-1..=ln_end-1].to_vec()
    }
}

thread_local! {
    pub static READER: RefCell<StdinReader> = RefCell::new(StdinReader{ lineno: 0, buf: vec![] });
}

pub fn read() -> Str {
    READER.with(|s| s.borrow_mut().read())
}

pub fn reread() -> Str {
    READER.with(|s| s.borrow().reread())
}

pub fn reread_lines(ln_begin: usize, ln_end: usize) -> Vec<Str> {
    READER.with(|s| s.borrow_mut().reread_lines(ln_begin, ln_end))
}
