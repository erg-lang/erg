//! provides common components for error handling.
//!
//! エラー処理に関する汎用的なコンポーネントを提供する
use std::cmp;
use std::fmt;
use std::io::{stderr, BufWriter, Write};

use crate::color::*;
use crate::config::Input;
use crate::traits::{Locational, Stream};
use crate::Str;
use crate::{fmt_option, impl_display_from_debug, switch_lang};

/// ErrorKindと言っているが、ErrorだけでなくWarning, Exceptionも含まれる
/// Numbering of this is not specifically related to ErrFmt.errno().
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ErrorKind {
    /* compile errors */
    AssignError = 0,
    AttributeError,
    BytecodeError,
    CompilerSystemError,
    EnvironmentError,
    FeatureError,
    ImportError,
    IndentationError,
    NameError,
    NotImplementedError,
    PatternError,
    SyntaxError,
    TabError,
    TypeError,
    UnboundLocalError,
    PurityError,
    HasEffect,
    MoveError,
    /* compile warnings */
    AttributeWarning = 60,
    CastWarning,
    DeprecationWarning,
    FutureWarning,
    ImportWarning,
    PendingDeprecationWarning,
    SyntaxWarning,
    TypeWarning,
    NameWarning,
    UnusedWarning,
    Warning,
    /* runtime errors */
    ArithmeticError = 100,
    AssertionError,
    BlockingIOError,
    BrokenPipeError,
    BufferError,
    ChildProcessError,
    ConnectionAbortedError,
    ConnectionError,
    ConnectionRefusedError,
    ConnectionResetError,
    EOFError,
    FileExistsError,
    FileNotFoundError,
    IndexError,
    InterruptedError,
    IoError,
    IsADirectoryError,
    KeyError,
    LookupError,
    MemoryError,
    ModuleNotFoundError,
    NotADirectoryError,
    OSError,
    OverflowError,
    PermissionError,
    ProcessLookupError,
    RecursionError,
    ReferenceError,
    RuntimeAttributeError,
    RuntimeError,
    RuntimeTypeError,
    RuntimeUnicodeError,
    TimeoutError,
    UnicodeError,
    UserError,
    ValueError,
    VMSystemError,
    WindowsError,
    ZeroDivisionError,
    /* runtime warnings */
    BytesWarning = 180,
    ResourceWarning,
    RuntimeWarning,
    UnicodeWarning,
    UserWarning,
    /* exceptions */
    BaseException = 200,
    Exception,
    GeneratorExit,
    KeyboardInterrupt,
    StopAsyncIteration,
    StopIteration,
    SystemExit,
    UserException,
}

use ErrorKind::*;

impl_display_from_debug!(ErrorKind);

impl From<&str> for ErrorKind {
    fn from(s: &str) -> ErrorKind {
        match s {
            "AssignError" => Self::AssignError,
            "AttributeError" => Self::AttributeError,
            "BytecodeError" => Self::BytecodeError,
            "CompilerSystemError" => Self::CompilerSystemError,
            "EnvironmentError" => Self::EnvironmentError,
            "FeatureError" => Self::FeatureError,
            "ImportError" => Self::ImportError,
            "IndentationError" => Self::IndentationError,
            "NameError" => Self::NameError,
            "NotImplementedError" => Self::NotImplementedError,
            "PatternError" => Self::PatternError,
            "SyntaxError" => Self::SyntaxError,
            "TabError" => Self::TabError,
            "TypeError" => Self::TypeError,
            "UnboundLocalError" => Self::UnboundLocalError,
            "HasEffect" => Self::HasEffect,
            "PurityError" => Self::PurityError,
            "MoveError" => Self::MoveError,
            "AttributeWarning" => Self::AttributeWarning,
            "CastWarning" => Self::CastWarning,
            "DeprecationWarning" => Self::DeprecationWarning,
            "FutureWarning" => Self::FutureWarning,
            "ImportWarning" => Self::ImportWarning,
            "PendingDeprecationWarning" => Self::PendingDeprecationWarning,
            "SyntaxWarning" => Self::SyntaxWarning,
            "TypeWarning" => Self::TypeWarning,
            "NameWarning" => Self::NameWarning,
            "UnusedWarning" => Self::UnusedWarning,
            "Warning" => Self::Warning,
            "ArithmeticError" => Self::ArithmeticError,
            "AssertionError" => Self::AssertionError,
            "BlockingIOError" => Self::BlockingIOError,
            "BrokenPipeError" => Self::BrokenPipeError,
            "BufferError" => Self::BufferError,
            "ChildProcessError" => Self::ChildProcessError,
            "ConnectionAbortedError" => Self::ConnectionAbortedError,
            "ConnectionError" => Self::ConnectionError,
            "ConnectionRefusedError" => Self::ConnectionRefusedError,
            "ConnectionResetError" => Self::ConnectionResetError,
            "EOFError" => Self::EOFError,
            "FileExistsError" => Self::FileExistsError,
            "FileNotFoundError" => Self::FileNotFoundError,
            "IndexError" => Self::IndexError,
            "InterruptedError" => Self::InterruptedError,
            "IoError" => Self::IoError,
            "IsADirectoryError" => Self::IsADirectoryError,
            "KeyError" => Self::KeyError,
            "LookupError" => Self::LookupError,
            "MemoryError" => Self::MemoryError,
            "ModuleNotFoundError" => Self::ModuleNotFoundError,
            "NotADirectoryError" => Self::NotADirectoryError,
            "OSError" => Self::OSError,
            "OverflowError" => Self::OverflowError,
            "PermissionError" => Self::PermissionError,
            "ProcessLookupError" => Self::ProcessLookupError,
            "RecursionError" => Self::RecursionError,
            "ReferenceError" => Self::ReferenceError,
            "RuntimeAttributeError" => Self::RuntimeAttributeError,
            "RuntimeError" => Self::RuntimeError,
            "RuntimeTypeError" => Self::RuntimeTypeError,
            "RuntimeUnicodeError" => Self::RuntimeUnicodeError,
            "TimeoutError" => Self::TimeoutError,
            "UnicodeError" => Self::UnicodeError,
            "UserError" => Self::UserError,
            "ValueError" => Self::ValueError,
            "VMSystemError" => Self::VMSystemError,
            "WindowsError" => Self::WindowsError,
            "ZeroDivisionError" => Self::ZeroDivisionError,
            "BytesWarning" => Self::BytesWarning,
            "ResourceWarning" => Self::ResourceWarning,
            "RuntimeWarning" => Self::RuntimeWarning,
            "UnicodeWarning" => Self::UnicodeWarning,
            "UserWarning" => Self::UserWarning,
            "BaseException" => Self::BaseException,
            "Exception" => Self::Exception,
            "GeneratorExit" => Self::GeneratorExit,
            "KeyboardInterrupt" => Self::KeyboardInterrupt,
            "StopAsyncIteration" => Self::StopAsyncIteration,
            "StopIteration" => Self::StopIteration,
            "SystemExit" => Self::SystemExit,
            "UserException" => Self::UserException,
            _ => Self::UserError,
        }
    }
}

/// points the location (of an error) in a code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    RangePair {
        ln_begin: usize,
        col_first: (usize, usize),
        ln_end: usize,
        col_second: (usize, usize),
    },
    Range {
        ln_begin: usize,
        col_begin: usize,
        ln_end: usize,
        col_end: usize,
    },
    LineRange(usize, usize),
    Line(usize),
    Unknown,
}

impl Location {
    pub fn concat<L: Locational, R: Locational>(l: &L, r: &R) -> Self {
        match (l.ln_begin(), l.col_begin(), r.ln_end(), r.col_end()) {
            (Some(lb), Some(cb), Some(le), Some(ce)) => Self::range(lb, cb, le, ce),
            (Some(lb), _, Some(le), _) => Self::LineRange(lb, le),
            (Some(l), _, _, _) | (_, _, Some(l), _) => Self::Line(l),
            _ => Self::Unknown,
        }
    }

    pub const fn range(ln_begin: usize, col_begin: usize, ln_end: usize, col_end: usize) -> Self {
        Self::Range {
            ln_begin,
            col_begin,
            ln_end,
            col_end,
        }
    }

    pub fn pair(lhs: Self, rhs: Self) -> Self {
        Self::RangePair {
            ln_begin: lhs.ln_begin().unwrap(),
            col_first: (lhs.col_begin().unwrap(), lhs.col_end().unwrap()),
            ln_end: rhs.ln_end().unwrap(),
            col_second: (rhs.col_begin().unwrap(), rhs.col_end().unwrap()),
        }
    }

    pub const fn ln_begin(&self) -> Option<usize> {
        match self {
            Self::RangePair { ln_begin, .. }
            | Self::Range { ln_begin, .. }
            | Self::LineRange(ln_begin, _)
            | Self::Line(ln_begin) => Some(*ln_begin),
            Self::Unknown => None,
        }
    }

    pub const fn ln_end(&self) -> Option<usize> {
        match self {
            Self::RangePair { ln_end, .. }
            | Self::Range { ln_end, .. }
            | Self::LineRange(ln_end, _)
            | Self::Line(ln_end) => Some(*ln_end),
            Self::Unknown => None,
        }
    }

    pub const fn col_begin(&self) -> Option<usize> {
        match self {
            Self::RangePair {
                col_first: (col_begin, _),
                ..
            }
            | Self::Range { col_begin, .. } => Some(*col_begin),
            _ => None,
        }
    }

    pub const fn col_end(&self) -> Option<usize> {
        match self {
            Self::RangePair {
                col_second: (_, col_end),
                ..
            }
            | Self::Range { col_end, .. } => Some(*col_end),
            _ => None,
        }
    }
}

/// Erg内で使われるエラーの共通部分
/// 使用する場合は必ずwrapすること
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorCore {
    pub errno: usize,
    pub kind: ErrorKind,
    pub loc: Location,
    pub desc: Str,
    pub hint: Option<Str>,
}

impl ErrorCore {
    pub fn new<S: Into<Str>>(
        errno: usize,
        kind: ErrorKind,
        loc: Location,
        desc: S,
        hint: Option<Str>,
    ) -> Self {
        Self {
            errno,
            kind,
            loc,
            desc: desc.into(),
            hint,
        }
    }

    pub fn unreachable(fn_name: &str, line: u32) -> Self {
        Self::bug(line as usize, Location::Unknown, fn_name, line)
    }

    pub fn bug(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        Self::new(errno, CompilerSystemError, loc, switch_lang!(
            format!("this is a bug of Erg, please report it to https://github.com/...\ncaused from: {fn_name}:{line}"),
            format!("これはErgのバグです、開発者に報告して下さい (https://github.com/...)\n{fn_name}:{line}より発生")
        ), None)
    }
}

pub const VBAR_UNICODE: &'static str = "│";
pub const VBAR_BREAK_UNICODE: &'static str = "·";

/// format:
/// ```console
/// Error[#{.errno}]: File {file}, line {.loc (as line)}, in {.caused_by}
/// {.loc (as line)}| {src}
/// {pointer}
/// {.kind}: {.desc}
/// ```
///
/// example:
/// ```console
/// Error[#12]: File <stdin>, line 1, in <module>
/// 1| 100 = i
///    ^^^
/// SyntaxError: cannot assign to 100
/// ```
pub trait ErrorDisplay {
    fn core(&self) -> &ErrorCore;
    fn input(&self) -> &Input;
    /// The block name the error caused.
    /// This will be None if the error occurred before semantic analysis.
    /// As for the internal error, do not put the fn name here.
    fn caused_by(&self) -> &str;
    /// the previous error that caused this error.
    fn ref_inner(&self) -> Option<&Box<Self>>;

    fn write_to_stderr(&self) {
        let mut writer = BufWriter::new(stderr());
        writer
            .write_all(
                format!(
                    "{}{}{}: {}{}\n",
                    self.format_header(),
                    self.format_code_and_pointer(),
                    self.core().kind,
                    self.core().desc,
                    fmt_option!(pre format!("\n{GREEN}hint{RESET}: "), &self.core().hint),
                )
                .as_bytes(),
            )
            .unwrap();
        writer.flush().unwrap();
        if let Some(inner) = self.ref_inner() {
            inner.write_to_stderr()
        }
    }

    /// fmt::Display実装用
    fn format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}{}{}: {}{}",
            self.format_header(),
            self.format_code_and_pointer(),
            self.core().kind,
            self.core().desc,
            fmt_option!(pre format!("\n{GREEN}hint{RESET}: "), &self.core().hint),
        )?;
        if let Some(inner) = self.ref_inner() {
            inner.format(f)
        } else {
            Ok(())
        }
    }

    fn format_header(&self) -> String {
        let kind = self.core().kind as u8;
        let (color, err_or_warn) = if kind < 100 {
            (RED, "Error")
        } else if 100 <= kind && kind < 150 {
            (YELLOW, "Warning")
        } else if 150 <= kind && kind < 200 {
            (DEEP_RED, "Error")
        } else {
            ("", "Exception")
        };
        let loc = match self.core().loc {
            Location::Range {
                ln_begin, ln_end, ..
            } if ln_begin == ln_end => format!(", line {ln_begin}"),
            Location::RangePair {
                ln_begin, ln_end, ..
            }
            | Location::Range {
                ln_begin, ln_end, ..
            }
            | Location::LineRange(ln_begin, ln_end) => format!(", line {ln_begin}..{ln_end}"),
            Location::Line(lineno) => format!(", line {lineno}"),
            Location::Unknown => "".to_string(),
        };
        let caused_by = if self.caused_by() != "" {
            format!(", in {}", self.caused_by())
        } else {
            "".to_string()
        };
        format!(
            "{color}{err_or_warn}[#{errno:>04}]{RESET}: File {input}{loc}{caused_by}\n",
            errno = self.core().errno,
            input = self.input().enclosed_name(),
        )
    }

    fn format_code_and_pointer(&self) -> String {
        match self.core().loc {
            Location::RangePair { .. } => todo!(),
            Location::Range {
                ln_begin,
                col_begin,
                ln_end,
                col_end,
            } => {
                let codes = if self.input() == &Input::REPL {
                    vec![self.input().reread()]
                } else {
                    self.input().reread_lines(ln_begin, ln_end)
                };
                let mut res = CYAN.to_string();
                let final_step = ln_end - ln_begin;
                for (i, lineno) in (ln_begin..=ln_end).enumerate() {
                    let mut pointer = " ".repeat(lineno.to_string().len() + 2); // +2 means `| `
                    if i == 0 && i == final_step {
                        pointer += &" ".repeat(col_begin);
                        pointer += &"^".repeat(cmp::max(1, col_end - col_begin));
                    } else if i == 0 {
                        pointer += &" ".repeat(col_begin);
                        pointer += &"^".repeat(cmp::max(1, codes[i].len() - col_begin));
                    } else if i == final_step {
                        pointer += &"^".repeat(col_end);
                    } else {
                        pointer += &"^".repeat(cmp::max(1, codes[i].len()));
                    }
                    res += &format!(
                        "{lineno}{VBAR_UNICODE} {code}\n{pointer}\n",
                        code = codes[i]
                    );
                }
                res + RESET
            }
            Location::LineRange(_begin, _end) => {
                todo!()
            }
            Location::Line(lineno) => {
                let code = if self.input() == &Input::REPL {
                    self.input().reread()
                } else {
                    self.input().reread_lines(lineno, lineno).remove(0)
                };
                format!("{CYAN}{lineno}{VBAR_UNICODE} {code}\n{RESET}")
            }
            Location::Unknown => match self.input() {
                Input::File(_) => "\n".to_string(),
                other => format!(
                    "{CYAN}?{VBAR_UNICODE} {code}\n{RESET}",
                    code = other.reread()
                ),
            },
        }
    }
}

pub trait MultiErrorDisplay<Item: ErrorDisplay>: Stream<Item> {
    fn fmt_all_stderr(&self) {
        for err in self.iter() {
            err.write_to_stderr();
        }
    }

    fn fmt_all(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for err in self.iter() {
            err.format(f)?;
        }
        write!(f, "")
    }
}
