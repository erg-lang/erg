//! provides common components for error handling.
//!
//! エラー処理に関する汎用的なコンポーネントを提供する
use std::cmp;
use std::fmt;
use std::io::{stderr, BufWriter, Write as _};

use crate::astr::AtomicStr;
use crate::config::Input;
use crate::style::Attribute;
use crate::style::Characters;
use crate::style::Color;
use crate::style::StyledStr;
use crate::style::StyledString;
use crate::style::StyledStrings;
use crate::style::Theme;
use crate::traits::{Locational, Stream};
use crate::{impl_display_from_debug, switch_lang};

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
    NotConstExpr,
    InheritanceError,
    VisibilityError,
    MethodError,
    DummyError,
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

impl ErrorKind {
    pub fn is_warning(&self) -> bool {
        (60..=100).contains(&(*self as u8)) || (180..=200).contains(&(*self as u8))
    }

    pub fn is_error(&self) -> bool {
        (0..=59).contains(&(*self as u8)) || (100..=179).contains(&(*self as u8))
    }

    pub fn is_exception(&self) -> bool {
        (200..=255).contains(&(*self as u8))
    }
}

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

///
/// Points the location (of an error) in a code.
/// The beginning and end of each row and column where the error occurred.
/// Basically, the beginning and end of each row and column where the error occurred is kept.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Location {
    ///
    /// Error used when the error is caused by a discrepancy with a code on another line
    ///
    /// # Example
    ///
    /// Ownership error
    ///
    /// ```erg
    /// a: Nat = 1
    /// a.consume_ownership() // move occurs
    ///
    /// function(a) // borrowed after moved
    /// ```
    ///
    /// `a` moves ownership in a method(or function) that are defined and consume it.
    ///
    /// ```erg
    /// Location::RangePair {
    ///     ln_first: (2, 2),
    ///     col_first: (0, 1),
    ///     ln_second: (4, 4),
    ///     col_second: (9, 10),
    /// }
    /// ```
    ///
    RangePair {
        ln_first: (usize, usize),
        col_first: (usize, usize),
        ln_second: (usize, usize),
        col_second: (usize, usize),
    },
    ///
    /// Location used for basic errors
    /// ```erg
    /// // erg
    /// a = 1
    /// a = 2
    /// // Value assigned to the structure
    /// Location::Range {
    ///    ln_begin: 2,
    ///    col_begin: 0,
    ///    ln_end: 2,
    ///    col_end: 1,
    /// }
    /// ```
    ///
    Range {
        ln_begin: usize,
        col_begin: usize,
        ln_end: usize,
        col_end: usize,
    },
    /// Used for loss of location information when desugared.
    /// If there are guaranteed to be multiple rows
    LineRange(usize, usize),
    /// Used when Location information is lost when desugared
    /// If it is guaranteed to be a single line
    Line(usize),
    /// Used by default in case of loss of Location information
    #[default]
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
            ln_first: (lhs.ln_begin().unwrap(), lhs.ln_end().unwrap()),
            col_first: (lhs.col_begin().unwrap(), lhs.col_end().unwrap()),
            ln_second: (rhs.ln_begin().unwrap(), rhs.ln_end().unwrap()),
            col_second: (rhs.col_begin().unwrap(), rhs.col_end().unwrap()),
        }
    }

    pub const fn ln_begin(&self) -> Option<usize> {
        match self {
            Self::RangePair {
                ln_first: (ln_begin, _),
                ..
            }
            | Self::Range { ln_begin, .. }
            | Self::LineRange(ln_begin, _)
            | Self::Line(ln_begin) => Some(*ln_begin),
            Self::Unknown => None,
        }
    }

    pub const fn ln_end(&self) -> Option<usize> {
        match self {
            Self::RangePair {
                ln_second: (_, ln_end),
                ..
            }
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

/// In Erg, common parts used by error.
/// Must be wrap when to use.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorCore {
    pub errno: usize,
    pub kind: ErrorKind,
    pub loc: Location,
    pub desc: AtomicStr,
    pub hint: Option<AtomicStr>,
}

impl ErrorCore {
    pub fn new<S: Into<AtomicStr>>(
        errno: usize,
        kind: ErrorKind,
        loc: Location,
        desc: S,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self {
            errno,
            kind,
            loc,
            desc: desc.into(),
            hint,
        }
    }

    pub fn dummy(errno: usize) -> Self {
        Self::new(
            errno,
            DummyError,
            Location::Line(errno as usize),
            "<dummy>",
            None,
        )
    }

    pub fn unreachable(fn_name: &str, line: u32) -> Self {
        Self::bug(line as usize, Location::Line(line as usize), fn_name, line)
    }

    pub fn bug(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        const URL: StyledStr = StyledStr::new(
            "https://github.com/erg-lang/erg",
            Some(Color::White),
            Some(Attribute::Underline),
        );

        Self::new(
            errno,
            CompilerSystemError,
            loc,
            switch_lang!(
                "japanese" => format!("これはErgのバグです、開発者に報告して下さい({URL})\n{fn_name}:{line}より発生"),
                "simplified_chinese" => format!("这是Erg的bug，请报告给{URL}\n原因来自: {fn_name}:{line}"),
                "traditional_chinese" => format!("这是Erg的bug，请报告给{URL}\n原因来自: {fn_name}:{line}"),
                "english" => format!("this is a bug of Erg, please report it to {URL}\ncaused from: {fn_name}:{line}"),
            ),
            None,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn format_context<E: ErrorDisplay + ?Sized>(
    e: &E,
    ln_begin: usize,
    ln_end: usize,
    col_begin: usize,
    col_end: usize,
    err_color: Color,
    gutter_color: Color,
    // for formatting points
    chars: &Characters,
    // kinds of error for specify the color
    mark: char,
) -> String {
    let mark = mark.to_string();
    let codes = if e.input().is_repl() {
        vec![e.input().reread()]
    } else {
        e.input().reread_lines(ln_begin, ln_end)
    };
    let mut context = StyledStrings::default();
    let final_step = ln_end - ln_begin;
    let max_digit = ln_end.to_string().len();
    let (vbreak, vbar) = chars.gutters();
    let offset = format!("{} {} ", &" ".repeat(max_digit), vbreak);
    for (i, lineno) in (ln_begin..=ln_end).enumerate() {
        context.push_str_with_color(
            &format!("{:<max_digit$} {vbar} ", lineno, vbar = vbar),
            gutter_color,
        );
        context.push_str(&codes[i]);
        context.push_str("\n");
        context.push_str_with_color(&offset, gutter_color);
        if i == 0 && i == final_step {
            context.push_str(&" ".repeat(col_begin));
            context.push_str_with_color(
                &mark.repeat(cmp::max(1, col_end.saturating_sub(col_begin))),
                err_color,
            );
        } else if i == 0 {
            context.push_str(&" ".repeat(col_begin));
            context.push_str_with_color(
                &mark.repeat(cmp::max(1, codes[i].len().saturating_sub(col_begin))),
                err_color,
            );
        } else if i == final_step {
            context.push_str_with_color(&mark.repeat(col_end), err_color);
        } else {
            context.push_str_with_color(&mark.repeat(cmp::max(1, codes[i].len())), err_color);
        }
        context.push_str("\n");
    }
    context.push_str_with_color(&offset, gutter_color);
    context.push_str(&" ".repeat(col_end - 1));
    context.push_str_with_color(&chars.left_bottom_line(), err_color);
    context.to_string()
}

/// format:
/// ```txt
/// Error[#{.errno}]: File {file}, line {.loc (as line)}, in {.caused_by}
/// {.loc (as line)}| {src}
/// {pointer}
/// {.kind}: {.desc}
///
/// {.hint}
///
/// ```
///
/// example:
/// ```txt
/// Error[#2223]: File <stdin>, line 1, in <module>
///
/// 1 | 100 = i
///     ---
///       ╰─ SyntaxError: cannot assign to 100
///
/// hint: hint message here
///
/// ```
pub trait ErrorDisplay {
    fn core(&self) -> &ErrorCore;
    fn input(&self) -> &Input;
    /// Colors and indication char for each type(error, warning, exception)
    fn theme(&self) -> &Theme;
    /// The block name the error caused.
    /// This will be None if the error occurred before semantic analysis.
    /// As for the internal error, do not put the fn name here.
    fn caused_by(&self) -> &str;
    /// the previous error that caused this error.
    fn ref_inner(&self) -> Option<&Self>;

    fn write_to_stderr(&self) {
        let mut stderr = stderr();
        self.write_to(&mut stderr)
    }

    fn write_to<W: std::io::Write>(&self, w: &mut W) {
        let mut writer = BufWriter::new(w);
        writer.write_all(self.show().as_bytes()).unwrap();
        writer.flush().unwrap();
        if let Some(inner) = self.ref_inner() {
            inner.write_to_stderr()
        }
    }

    fn show(&self) -> String {
        let theme = self.theme();
        let ((color, mark), kind) = if self.core().kind.is_error() {
            (theme.error(), "Error")
        } else if self.core().kind.is_warning() {
            (theme.warning(), "Warning")
        } else {
            (theme.exception(), "Exception")
        };

        let (gutter_color, chars) = theme.characters();
        let kind = StyledString::new(
            &chars.error_kind_format(kind, self.core().errno),
            Some(color),
            Some(Attribute::Bold),
        );

        //  When hint is None, hint desc is "" and empty line is displayed, but hint is Some(...), hint desc is "..." and filled by text
        if let Some(hint) = self.core().hint.as_ref() {
            let (hint_color, _) = theme.hint();
            let mut hints = StyledStrings::default();
            hints.push_str_with_color_and_attribute("hint: ", hint_color, Attribute::Bold);
            hints.push_str(hint);
            format!(
                "\
{}
{}{}: {}

{}

",
                self.format_header(kind),
                self.format_code_and_pointer(color, gutter_color, mark, chars),
                self.core().kind,
                self.core().desc,
                hints,
            )
        } else {
            format!(
                "\
{}
{}{}: {}

",
                self.format_header(kind),
                self.format_code_and_pointer(color, gutter_color, mark, chars),
                self.core().kind,
                self.core().desc,
            )
        }
    }

    /// for fmt::Display
    fn format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let theme = self.theme();
        let ((color, mark), kind) = if self.core().kind.is_error() {
            (theme.error(), "Error")
        } else if self.core().kind.is_warning() {
            (theme.warning(), "Warning")
        } else {
            (theme.exception(), "Exception")
        };
        let (gutter_color, chars) = theme.characters();
        let kind = StyledString::new(
            &chars.error_kind_format(kind, self.core().errno),
            Some(color),
            Some(Attribute::Bold),
        );

        //  When hint is None, hint desc is "" and empty line is displayed, but hint is Some(...), hint desc is "..." and filled by text
        if let Some(hint) = self.core().hint.as_ref() {
            let (hint_color, _) = theme.hint();
            let mut hints = StyledStrings::default();
            hints.push_str_with_color_and_attribute("hint: ", hint_color, Attribute::Bold);
            hints.push_str(hint);
            writeln!(
                f,
                "\
{}
{}{}: {}

{}

",
                self.format_header(kind),
                self.format_code_and_pointer(color, gutter_color, mark, chars),
                self.core().kind,
                self.core().desc,
                hints,
            )?;
        } else {
            writeln!(
                f,
                "\
{}
{}{}: {}

",
                self.format_header(kind),
                self.format_code_and_pointer(color, gutter_color, mark, chars),
                self.core().kind,
                self.core().desc,
            )?;
        }
        if let Some(inner) = self.ref_inner() {
            inner.format(f)
        } else {
            Ok(())
        }
    }

    fn format_header(&self, kind: StyledString) -> String {
        let loc = match self.core().loc {
            Location::Range {
                ln_begin, ln_end, ..
            } if ln_begin == ln_end => format!(", line {ln_begin}"),
            Location::Range {
                ln_begin, ln_end, ..
            }
            | Location::LineRange(ln_begin, ln_end) => format!(", line {ln_begin}..{ln_end}"),
            Location::RangePair {
                ln_first: (l1, l2),
                ln_second: (l3, l4),
                ..
            } => format!(", line {l1}..{l2}, {l3}..{l4}"),
            Location::Line(lineno) => format!(", line {lineno}"),
            Location::Unknown => "".to_string(),
        };
        let caused_by = if self.caused_by() != "" {
            format!(", in {}", self.caused_by())
        } else {
            "".to_string()
        };
        format!(
            "{kind}: File {input}{loc}{caused_by}\n",
            input = self.input().enclosed_name(),
        )
    }

    fn format_code_and_pointer(
        &self,
        err_color: Color,
        gutter_color: Color,
        mark: char,
        chars: &Characters,
    ) -> String {
        match self.core().loc {
            // TODO: Current implementation does not allow for multiple descriptions of errors to be given at each location
            // In the future, this will be implemented in a different structure that can handle multiple lines and files
            Location::RangePair {
                ln_first,
                col_first,
                ln_second,
                col_second,
            } => {
                format_context(
                    self,
                    ln_first.0,
                    ln_first.1,
                    col_first.0,
                    col_first.1,
                    err_color,
                    gutter_color,
                    chars,
                    mark,
                ) +
                "\n" // TODO: dealing with error chains
                    + &format_context(
                        self,
                        ln_second.0,
                        ln_second.1,
                        col_second.0,
                        col_second.1,
                    err_color,
                    gutter_color,
                        chars,
                        mark,
                    )
            }
            Location::Range {
                ln_begin,
                col_begin,
                ln_end,
                col_end,
            } => format_context(
                self,
                ln_begin,
                ln_end,
                col_begin,
                col_end,
                err_color,
                gutter_color,
                chars,
                mark,
            ),
            Location::LineRange(ln_begin, ln_end) => {
                let (_, vbar) = chars.gutters();
                let mut cxt = StyledStrings::default();
                let codes = if self.input().is_repl() {
                    vec![self.input().reread()]
                } else {
                    self.input().reread_lines(ln_begin, ln_end)
                };
                let mark = mark.to_string();
                for (i, lineno) in (ln_begin..=ln_end).enumerate() {
                    cxt.push_str_with_color(&format!("{lineno} {}", vbar), err_color);
                    cxt.push_str(&codes[i]);
                    cxt.push_str("\n");
                    cxt.push_str(&" ".repeat(lineno.to_string().len() + 3)); // +3 means ` | `
                    cxt.push_str_with_color(
                        &mark.repeat(cmp::max(1, codes[i].len())),
                        gutter_color,
                    );
                    cxt.push_str("\n");
                }
                cxt.to_string()
            }
            Location::Line(lineno) => {
                let (_, vbar) = chars.gutters();
                let code = if self.input().is_repl() {
                    self.input().reread()
                } else {
                    self.input().reread_lines(lineno, lineno).remove(0)
                };
                let mut cxt = StyledStrings::default();
                cxt.push_str_with_color(&format!(" {lineno} {} ", vbar), gutter_color);
                cxt.push_str(&code);
                cxt.push_str("\n");
                cxt.to_string()
            }
            Location::Unknown => match self.input() {
                Input::File(_) => "\n".to_string(),

                other => {
                    let (_, vbar) = chars.gutters();
                    let mut cxt = StyledStrings::default();
                    cxt.push_str_with_color(&format!(" ? {}", vbar), gutter_color);
                    cxt.push_str(&other.reread());
                    cxt.to_string()
                }
            },
        }
    }
}

#[macro_export]
macro_rules! impl_display_and_error {
    ($Strc: ident) => {
        impl std::fmt::Display for $Strc {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                $crate::error::ErrorDisplay::format(self, f)
            }
        }

        impl std::error::Error for $Strc {}
    };
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
