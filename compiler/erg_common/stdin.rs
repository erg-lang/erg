use std::cell::RefCell;
use std::io::{stdin, BufRead, BufReader};
use std::thread::LocalKey;

/// e.g.
/// ```erg
/// >>> print! 1
/// >>>
/// >>> while! False, do!:
/// >>>    print! ""
/// >>>
/// ```
/// ↓
///
/// `{ lineno: 5, buf: ["print! 1\n", "\n", "while! False, do!:\n", "print! \"\"\n", "\n"] }`
#[derive(Debug)]
pub struct StdinReader {
    block_begin: usize,
    lineno: usize,
    buf: Vec<String>,
}

impl StdinReader {
    pub fn read(&mut self) -> String {
        let mut buf = "".to_string();
        let stdin = stdin();
        let mut reader = BufReader::new(stdin.lock());
        reader.read_line(&mut buf).unwrap();
        self.lineno += 1;
        self.buf.push(buf);
        self.buf.last().cloned().unwrap_or_default()
    }

    pub fn reread(&self) -> String {
        self.buf.last().cloned().unwrap_or_default()
    }

    pub fn reread_lines(&self, ln_begin: usize, ln_end: usize) -> Vec<String> {
        self.buf[ln_begin - 1..=ln_end - 1].to_vec()
    }

    pub fn last_line(&mut self) -> Option<&mut String> {
        self.buf.last_mut()
    }
}

thread_local! {
    static READER: RefCell<StdinReader> = RefCell::new(StdinReader{ block_begin: 0, lineno: 0, buf: vec![] });
}

#[derive(Debug)]
pub struct GlobalStdin(LocalKey<RefCell<StdinReader>>);

pub static GLOBAL_STDIN: GlobalStdin = GlobalStdin(READER);

impl GlobalStdin {
    pub fn read(&'static self) -> String {
        self.0.with(|s| s.borrow_mut().read())
    }

    pub fn reread(&'static self) -> String {
        self.0.with(|s| s.borrow().reread())
    }

    pub fn reread_lines(&'static self, ln_begin: usize, ln_end: usize) -> Vec<String> {
        self.0
            .with(|s| s.borrow_mut().reread_lines(ln_begin, ln_end))
    }

    pub fn lineno(&'static self) -> usize {
        self.0.with(|s| s.borrow().lineno)
    }

    pub fn block_begin(&'static self) -> usize {
        self.0.with(|s| s.borrow().block_begin)
    }

    pub fn set_block_begin(&'static self, n: usize) {
        self.0.with(|s| s.borrow_mut().block_begin = n);
    }

    pub fn insert_whitespace(&'static self, whitespace: &str) {
        self.0.with(|s| {
            if let Some(line) = s.borrow_mut().last_line() {
                line.insert_str(0, whitespace);
            }
        })
    }
}
