use std::cell::RefCell;
use std::thread::LocalKey;

#[cfg(not(feature = "full-repl"))]
use std::io::{stdin, BufRead, BufReader};

#[cfg(feature = "full-repl")]
use crossterm::{
    cursor::MoveToColumn,
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode},
    terminal::{Clear, ClearType},
};
#[cfg(feature = "full-repl")]
use std::process::Command;
#[cfg(feature = "full-repl")]
use std::process::Output;

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
    #[cfg(feature = "full-repl")]
    history_input_position: usize,
    indent: u16,
}

impl StdinReader {
    #[cfg(all(feature = "full-repl", target_os = "linux"))]
    fn access_clipboard() -> Option<Output> {
        if let Ok(str) = std::fs::read("/proc/sys/kernel/osrelease") {
            if let Ok(str) = std::str::from_utf8(&str) {
                if str.to_ascii_lowercase().contains("microsoft") {
                    return Some(
                        Command::new("powershell")
                            .args(["get-clipboard"])
                            .output()
                            .expect("failed to get clipboard"),
                    );
                }
            }
        }
        match Command::new("xsel")
            .args(["--output", "--clipboard"])
            .output()
        {
            Ok(output) => Some(output),
            Err(_) => {
                execute!(
                    std::io::stdout(),
                    Print("You need to install `xsel` to use the paste feature on Linux desktop"),
                )
                .unwrap();
                None
            }
        }
    }
    #[cfg(all(feature = "full-repl", target_os = "macos"))]
    fn access_clipboard() -> Option<Output> {
        Some(
            Command::new("pbpast")
                .output()
                .expect("failed to get clipboard"),
        )
    }

    #[cfg(all(feature = "full-repl", target_os = "windows"))]
    fn access_clipboard() -> Option<Output> {
        Some(
            Command::new("powershell")
                .args(["get-clipboard"])
                .output()
                .expect("failed to get clipboard"),
        )
    }

    #[cfg(not(feature = "full-repl"))]
    pub fn read(&mut self) -> String {
        let mut line = "".to_string();
        let stdin = stdin();
        let mut reader = BufReader::new(stdin.lock());
        reader.read_line(&mut line).unwrap();
        self.lineno += 1;
        self.buf.push(line.trim_end().to_string());
        self.buf.last().cloned().unwrap_or_default()
    }

    #[cfg(feature = "full-repl")]
    pub fn read(&mut self) -> String {
        enable_raw_mode().unwrap();
        let mut output = std::io::stdout();
        let mut line = String::new();
        self.input(&mut line).unwrap();
        disable_raw_mode().unwrap();
        execute!(output, MoveToColumn(0)).unwrap();
        self.lineno += 1;
        self.buf.push(line);
        self.buf.last().cloned().unwrap_or_default()
    }

    #[cfg(feature = "full-repl")]
    fn input(&mut self, line: &mut String) -> std::io::Result<()> {
        let mut position = 0;
        let mut consult_history = false;
        let mut stdout = std::io::stdout();
        while let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = read()?
        {
            consult_history = false;
            match (code, modifiers) {
                (KeyCode::Char('z'), KeyModifiers::CONTROL)
                | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    println!();
                    line.clear();
                    line.push_str(":exit");
                    return Ok(());
                }
                (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                    let output = match Self::access_clipboard() {
                        None => {
                            continue;
                        }
                        Some(output) => output,
                    };
                    let clipboard = {
                        let this = String::from_utf8_lossy(&output.stdout).to_string();
                        this.trim_matches(|c: char| c.is_whitespace())
                            .to_string()
                            .replace(['\n', '\r'], "")
                            .replace(|c: char| c.len_utf8() >= 2, "")
                    };
                    line.insert_str(position, &clipboard);
                    position += clipboard.len();
                }
                (_, KeyModifiers::CONTROL) => continue,
                (KeyCode::Tab, _) => {
                    line.insert_str(position, "    ");
                    position += 4;
                }
                (KeyCode::Home, _) => {
                    position = 0;
                }
                (KeyCode::End, _) => {
                    position = line.len();
                }
                (KeyCode::Backspace, _) => {
                    if position == 0 {
                        continue;
                    }
                    line.remove(position - 1);
                    position -= 1;
                }
                (KeyCode::Delete, _) => {
                    if position == line.len() {
                        continue;
                    }
                    line.remove(position);
                }
                (KeyCode::Up, _) => {
                    consult_history = true;
                    if self.history_input_position == 0 {
                        continue;
                    }
                    self.history_input_position -= 1;
                    execute!(stdout, MoveToColumn(4), Clear(ClearType::UntilNewLine))?;
                    if let Some(l) = self.buf.get(self.history_input_position) {
                        position = l.len();
                        line.clear();
                        line.push_str(l);
                    }
                }
                (KeyCode::Down, _) => {
                    if self.history_input_position == self.buf.len() {
                        continue;
                    }
                    if self.history_input_position == self.buf.len() - 1 {
                        *line = "".to_string();
                        position = 0;
                        self.history_input_position += 1;
                        execute!(
                            stdout,
                            MoveToColumn(4),
                            Clear(ClearType::UntilNewLine),
                            MoveToColumn(self.indent * 4),
                            Print(line.to_owned()),
                            MoveToColumn(self.indent * 4 + position as u16)
                        )?;
                        continue;
                    }
                    self.history_input_position += 1;
                    execute!(stdout, MoveToColumn(4), Clear(ClearType::UntilNewLine))?;
                    if let Some(l) = self.buf.get(self.history_input_position) {
                        position = l.len();
                        line.clear();
                        line.push_str(l);
                    }
                }
                (KeyCode::Left, _) => {
                    if position == 0 {
                        continue;
                    }
                    position -= 1;
                }
                (KeyCode::Right, _) => {
                    if position == line.len() {
                        continue;
                    }
                    position += 1;
                }
                (KeyCode::Enter, _) => {
                    println!();
                    break;
                }
                // TODO: check a full-width char and possible to insert
                (KeyCode::Char(c), _) if c.len_utf8() < 2 => {
                    line.insert(position, c);
                    position += 1;
                }
                _ => {}
            }
            execute!(
                stdout,
                MoveToColumn(4),
                Clear(ClearType::UntilNewLine),
                MoveToColumn(self.indent * 4),
                Print(line.to_owned()),
                MoveToColumn(self.indent * 4 + position as u16)
            )?;
        }
        if !consult_history {
            self.history_input_position = self.buf.len() + 1;
        }
        Ok(())
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
    static READER: RefCell<StdinReader> = RefCell::new(StdinReader{
        block_begin: 1,
        lineno: 1,
        buf: vec![],
        #[cfg(feature = "full-repl")]
        history_input_position: 1,
        indent: 1
    });
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

    pub fn set_indent(&'static self, n: usize) {
        self.0.with(|s| s.borrow_mut().indent = n as u16);
    }

    pub fn insert_whitespace(&'static self, whitespace: &str) {
        self.0.with(|s| {
            if let Some(line) = s.borrow_mut().last_line() {
                line.insert_str(0, whitespace);
            }
        })
    }
}
