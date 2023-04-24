//! provides common components for error handling.
//!
//! エラー処理に関する汎用的なコンポーネントを提供する
use std::cmp::{self, Ordering};
use std::fmt;
use std::io::{stderr, BufWriter, Write as _};

use crate::config::{Input, InputKind};
use crate::style::Attribute;
use crate::style::Characters;
use crate::style::Color;
use crate::style::StyledStr;
use crate::style::StyledStrings;
use crate::style::Theme;
use crate::style::THEME;
use crate::traits::{Locational, Stream};
use crate::{impl_display_from_debug, switch_lang};

/// This includes not only Error but also Warning, Exception
/// Numbering of this is not specifically related to ErrFmt.errno().
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ErrorKind {
    /* compile errors */
    AssignError = 0,
    AttributeError = 1,
    BytecodeError = 2,
    CompilerSystemError = 3,
    EnvironmentError = 4,
    FeatureError = 5,
    ImportError = 6,
    IndentationError = 7,
    NameError = 8,
    NotImplementedError = 9,
    PatternError = 10,
    SyntaxError = 11,
    TabError = 12,
    TypeError = 13,
    UnboundLocalError = 14,
    PurityError = 15,
    HasEffect = 16,
    MoveError = 17,
    NotConstExpr = 18,
    InheritanceError = 19,
    VisibilityError = 20,
    MethodError = 21,
    DummyError = 22,
    /* compile warnings */
    AttributeWarning = 60,
    CastWarning = 61,
    DeprecationWarning = 62,
    FutureWarning = 63,
    ImportWarning = 64,
    PendingDeprecationWarning = 65,
    SyntaxWarning = 66,
    TypeWarning = 67,
    NameWarning = 68,
    UnusedWarning = 69,
    Warning = 70,
    /* runtime errors */
    ArithmeticError = 100,
    AssertionError = 101,
    BlockingIOError = 102,
    BrokenPipeError = 103,
    BufferError = 104,
    ChildProcessError = 105,
    ConnectionAbortedError = 106,
    ConnectionError = 107,
    ConnectionRefusedError = 108,
    ConnectionResetError = 109,
    EOFError = 110,
    FileExistsError = 111,
    FileNotFoundError = 112,
    IndexError = 113,
    InterruptedError = 114,
    IoError = 115,
    IsADirectoryError = 116,
    KeyError = 117,
    LookupError = 118,
    MemoryError = 119,
    ModuleNotFoundError = 120,
    NotADirectoryError = 121,
    OSError = 122,
    OverflowError = 123,
    PermissionError = 124,
    ProcessLookupError = 125,
    RecursionError = 126,
    ReferenceError = 127,
    RuntimeAttributeError = 128,
    RuntimeError = 129,
    RuntimeTypeError = 130,
    RuntimeUnicodeError = 131,
    TimeoutError = 132,
    UnicodeError = 133,
    UserError = 134,
    ValueError = 135,
    VMSystemError = 136,
    WindowsError = 137,
    ZeroDivisionError = 138,
    /* runtime warnings */
    BytesWarning = 180,
    ResourceWarning = 181,
    RuntimeWarning = 182,
    UnicodeWarning = 183,
    UserWarning = 184,
    /* exceptions */
    BaseException = 200,
    Exception = 201,
    GeneratorExit = 202,
    KeyboardInterrupt = 203,
    StopAsyncIteration = 204,
    StopIteration = 205,
    SystemExit = 206,
    UserException = 207,
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
        ln_begin: u32,
        col_begin: u32,
        ln_end: u32,
        col_end: u32,
    },
    /// Used for loss of location information when desugared.
    /// If there are guaranteed to be multiple rows
    LineRange(u32, u32),
    /// Used when Location information is lost when desugared
    /// If it is guaranteed to be a single line
    Line(u32),
    /// Used by default in case of loss of Location information
    #[default]
    Unknown,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Range {
                ln_begin,
                col_begin,
                ln_end,
                col_end,
            } => write!(f, "{ln_begin}:{col_begin}-{ln_end}:{col_end}"),
            Self::LineRange(ln_begin, ln_end) => write!(f, "{ln_begin}:?-{ln_end}:?"),
            Self::Line(ln) => write!(f, "{ln}:?-{ln}:?"),
            Self::Unknown => write!(f, "?"),
        }
    }
}

impl std::str::FromStr for Location {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "?" {
            return Ok(Self::Unknown);
        }
        // ln_begin:col_begin-ln_end:col_end
        let mut comps = s.split('-');
        let mut comp1 = comps.next().ok_or(())?.split(':');
        let mut comp2 = comps.next().ok_or(())?.split(':');
        let ln_begin = comp1.next().unwrap().parse::<u32>().map_err(|_| ())?;
        let col_begin = comp1.next().unwrap().parse::<u32>();
        let ln_end = comp2.next().unwrap().parse::<u32>().map_err(|_| ())?;
        let col_end = comp2.next().unwrap().parse::<u32>();
        match (col_begin, col_end) {
            (Ok(col_begin), Ok(col_end)) => Ok(Self::Range {
                ln_begin,
                col_begin,
                ln_end,
                col_end,
            }),
            _ if ln_begin == ln_end => Ok(Self::Line(ln_begin)),
            _ => Ok(Self::LineRange(ln_begin, ln_end)),
        }
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Location) -> Ordering {
        if self.ln_end() < other.ln_begin() {
            Ordering::Less
        } else if other.ln_end() < self.ln_begin() {
            Ordering::Greater
        } else if self.ln_begin() == self.ln_end() && other.ln_begin() == other.ln_end() {
            // assert_eq!(self.line_begin, other.line_begin);
            // assert_eq!(self.line_end, other.line_end);
            if self.col_end() < other.col_begin() {
                Ordering::Less
            } else if other.col_end() < self.col_begin() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        } else {
            Ordering::Equal
        }
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Location) -> Option<Ordering> {
        if self.is_unknown() || other.is_unknown() {
            None
        } else {
            Some(self.cmp(other))
        }
    }
}

impl Locational for Location {
    fn loc(&self) -> Self {
        *self
    }
}

impl Location {
    pub fn concat<L: Locational, R: Locational>(l: &L, r: &R) -> Self {
        let l_loc = l.loc();
        let r_loc = r.loc();
        match (
            l_loc.ln_begin(),
            l_loc.col_begin(),
            r_loc.ln_end(),
            r_loc.col_end(),
        ) {
            (Some(lb), Some(cb), Some(le), Some(ce)) => Self::range(lb, cb, le, ce),
            (Some(lb), _, Some(le), _) => Self::LineRange(lb, le),
            (Some(l), _, _, _) | (_, _, Some(l), _) => Self::Line(l),
            _ => Self::Unknown,
        }
    }

    pub fn stream<L: Locational>(ls: &[L]) -> Self {
        if ls.is_empty() {
            return Self::Unknown;
        };
        let Some(first_known) = ls.iter().find(|l| !l.loc().is_unknown()) else {
            return Self::Unknown;
        };
        let Some(last_known) = ls.iter().rev().find(|l| !l.loc().is_unknown()) else {
            return Self::Unknown;
        };
        Self::concat(first_known, last_known)
    }

    pub const fn range(ln_begin: u32, col_begin: u32, ln_end: u32, col_end: u32) -> Self {
        Self::Range {
            ln_begin,
            col_begin,
            ln_end,
            col_end,
        }
    }

    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    pub const fn unknown_or(&self, other: Self) -> Self {
        if self.is_unknown() {
            other
        } else {
            *self
        }
    }

    pub const fn ln_begin(&self) -> Option<u32> {
        match self {
            Self::Range { ln_begin, .. } | Self::LineRange(ln_begin, _) | Self::Line(ln_begin) => {
                Some(*ln_begin)
            }
            Self::Unknown => None,
        }
    }

    pub const fn ln_end(&self) -> Option<u32> {
        match self {
            Self::Range { ln_end, .. } | Self::LineRange(ln_end, _) | Self::Line(ln_end) => {
                Some(*ln_end)
            }
            Self::Unknown => None,
        }
    }

    pub const fn col_begin(&self) -> Option<u32> {
        match self {
            Self::Range { col_begin, .. } => Some(*col_begin),
            _ => None,
        }
    }

    pub const fn col_end(&self) -> Option<u32> {
        match self {
            Self::Range { col_end, .. } => Some(*col_end),
            _ => None,
        }
    }

    pub const fn length(&self) -> Option<u32> {
        match self {
            Self::Range {
                col_begin, col_end, ..
            } => Some(*col_end - *col_begin),
            _ => None,
        }
    }

    /// ```
    /// # use erg_common::error::Location;
    /// let loc = Location::range(1, 3, 1, 7);
    /// assert_eq!(loc.ln_begin(), Some(1));
    /// assert!(loc.contains(Location::range(1, 4, 1, 5)));
    /// let loc = Location::range(1, 3, 3, 2);
    /// assert!(loc.contains(Location::range(1, 4, 1, 5)));
    /// assert!(!loc.contains(Location::range(1, 4, 3, 5)));
    /// assert!(loc.contains(Location::range(1, 4, 2, 5)));
    /// assert!(!loc.contains(Location::range(1, 2, 2, 5)));
    /// ```
    pub fn contains(&self, other: Self) -> bool {
        match (*self, other) {
            (
                Self::Range {
                    ln_begin: lb1,
                    col_begin: cb1,
                    ln_end: le1,
                    col_end: ce1,
                },
                Self::Range {
                    ln_begin: lb2,
                    col_begin: cb2,
                    ln_end: le2,
                    col_end: ce2,
                },
            ) => {
                let same_start_line = lb1 == lb2;
                let same_end_line = le1 == le2;
                if same_start_line && same_end_line {
                    cb1 <= cb2 && ce1 >= ce2
                } else if same_start_line {
                    cb1 <= cb2 && le1 >= le2
                } else if same_end_line {
                    lb1 <= lb2 && ce1 >= ce2
                } else {
                    lb1 <= lb2 && le1 >= le2
                }
            }
            _ => false,
        }
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
    sub_msg: &[String],
    hint: Option<&String>,
) -> String {
    let mark = mark.to_string();
    let codes = e.input().reread_lines(ln_begin, ln_end);
    let mut context = StyledStrings::default();
    let final_step = ln_end - ln_begin;
    let max_digit = ln_end.to_string().len();
    let (vbreak, vbar) = chars.gutters();
    let offset = format!("{} {} ", &" ".repeat(max_digit), vbreak);
    for (i, lineno) in (ln_begin..=ln_end).enumerate() {
        context.push_str_with_color(&format!("{lineno:<max_digit$} {vbar} "), gutter_color);
        context.push_str(codes.get(i).unwrap_or(&String::new()));
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

    let msg_num = sub_msg.len().saturating_sub(1);
    for (i, msg) in sub_msg.iter().enumerate() {
        context.push_str_with_color(&offset, gutter_color);
        context.push_str(&" ".repeat(col_end.saturating_sub(1)));
        if i == msg_num && hint.is_none() {
            context.push_str_with_color(&chars.left_bottom_line(), err_color);
        } else {
            context.push_str_with_color(&chars.left_cross(), err_color);
        }
        context.push_str(msg);
        context.push_str("\n")
    }
    if let Some(hint) = hint {
        context.push_str_with_color(&offset, gutter_color);
        context.push_str(&" ".repeat(col_end.saturating_sub(1)));
        context.push_str_with_color(&chars.left_bottom_line(), err_color);
        context.push_str(hint);
        context.push_str("\n")
    }
    context.to_string() + "\n"
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubMessage {
    pub loc: Location,
    pub msg: Vec<String>,
    pub hint: Option<String>,
}

impl SubMessage {
    ///
    /// Used when the msg or hint is empty.
    /// `msg` is type of `Vec<String>` instead of `Option<String>` because it can be used when there are multiple `msg`s as well as multiple lines.
    /// # Example
    /// ```
    /// # use erg_common::error::{Location, SubMessage};
    /// # use erg_common::style::{Color, StyledString};
    /// let loc = Location::Line(1);
    /// let msg = SubMessage::ambiguous_new(loc, vec![], None); // this code same as only_loc()
    ///
    /// let hint = Some("hint message here".to_string());
    /// let msg = SubMessage::ambiguous_new(loc, vec![], hint);
    /// /* example
    ///    -------
    ///          `- hint message here
    /// */
    ///
    /// let hint = Some("hint here".to_string());
    /// let first = StyledString::new("1th message", Some(Color::Red), None);
    /// let second = StyledString::new("2th message", Some(Color::White), None);
    /// let nth = StyledString::new("nth message", Some(Color::Green), None);
    /// let msg = SubMessage::ambiguous_new(
    ///     loc,
    ///     vec![
    ///         first.to_string(),
    ///         second.to_string(),
    ///         // ...,
    ///         nth.to_string(),
    ///     ],
    ///     hint);
    /// /* example
    ///    -------
    ///          :- 1th message
    ///          :- 2th message
    ///                :
    ///          :- nth message
    ///          `- hint here
    /// */
    ///
    /// ```
    ///
    pub fn ambiguous_new(loc: Location, msg: Vec<String>, hint: Option<String>) -> Self {
        Self { loc, msg, hint }
    }

    ///
    /// Used when only Location is fixed.
    /// In this case, error position is just modified
    /// # Example
    /// ```
    /// # use erg_common::error::{Location, SubMessage};
    /// let loc = Location::Line(1);
    /// let sub_msg = SubMessage::only_loc(loc);
    /// ```
    pub fn only_loc(loc: Location) -> Self {
        Self {
            loc,
            msg: Vec::new(),
            hint: None,
        }
    }

    pub fn set_hint<S: Into<String>>(&mut self, hint: S) {
        self.hint = Some(hint.into());
    }

    pub fn get_hint(&self) -> Option<&str> {
        self.hint.as_deref()
    }

    pub fn get_msg(&self) -> &[String] {
        self.msg.as_ref()
    }

    // Line breaks are not included except for line breaks that signify the end of a sentence.
    // In other words, do not include blank lines for formatting purposes.
    fn format_code_and_pointer<E: ErrorDisplay + ?Sized>(
        &self,
        e: &E,
        err_color: Color,
        gutter_color: Color,
        mark: char,
        chars: &Characters,
    ) -> String {
        match self.loc.unknown_or(e.core().loc) {
            Location::Range {
                ln_begin,
                col_begin,
                ln_end,
                col_end,
            } => format_context(
                e,
                ln_begin as usize,
                ln_end as usize,
                col_begin as usize,
                col_end as usize,
                err_color,
                gutter_color,
                chars,
                mark,
                &self.msg,
                self.hint.as_ref(),
            ),
            Location::LineRange(ln_begin, ln_end) => {
                let (vbreak, vbar) = chars.gutters();
                let mut cxt = StyledStrings::default();
                let codes = e.input().reread_lines(ln_begin as usize, ln_end as usize);
                let mark = mark.to_string();
                for (i, lineno) in (ln_begin..=ln_end).enumerate() {
                    cxt.push_str_with_color(&format!("{lineno} {vbar} "), gutter_color);
                    cxt.push_str(codes.get(i).unwrap_or(&String::new()));
                    cxt.push_str("\n");
                    cxt.push_str_with_color(
                        &format!("{} {}", &" ".repeat(lineno.to_string().len()), vbreak),
                        gutter_color,
                    );
                    cxt.push_str(&" ".repeat(lineno.to_string().len()));
                    cxt.push_str_with_color(&mark.repeat(cmp::max(1, codes[i].len())), err_color);
                    cxt.push_str("\n");
                }
                cxt.push_str("\n");
                for msg in self.msg.iter() {
                    cxt.push_str(msg);
                    cxt.push_str("\n");
                }
                if let Some(hint) = self.hint.as_ref() {
                    cxt.push_str(hint);
                    cxt.push_str("\n");
                }
                cxt.to_string()
            }
            Location::Line(lineno) => {
                let input = e.input();
                let (_, vbar) = chars.gutters();
                let code = input
                    .reread_lines(lineno as usize, lineno as usize)
                    .remove(0);
                let mut cxt = StyledStrings::default();
                cxt.push_str_with_color(&format!(" {lineno} {vbar} "), gutter_color);
                cxt.push_str(&code);
                cxt.push_str("\n");
                for msg in self.msg.iter() {
                    cxt.push_str(msg);
                    cxt.push_str("\n");
                }
                if let Some(hint) = self.hint.as_ref() {
                    cxt.push_str(hint);
                    cxt.push_str("\n");
                }
                cxt.push_str("\n");
                cxt.to_string()
            }
            Location::Unknown => match &e.input().kind {
                InputKind::File(_) => "\n".to_string(),
                _other => {
                    let (_, vbar) = chars.gutters();
                    let mut cxt = StyledStrings::default();
                    cxt.push_str_with_color(&format!(" ? {vbar} "), gutter_color);
                    cxt.push_str(&e.input().reread());
                    cxt.push_str("\n");
                    for msg in self.msg.iter() {
                        cxt.push_str(msg);
                        cxt.push_str("\n");
                    }
                    if let Some(hint) = self.hint.as_ref() {
                        cxt.push_str(hint);
                        cxt.push_str("\n");
                    }
                    cxt.push_str("\n");
                    cxt.to_string()
                }
            },
        }
    }
}

/// In Erg, common parts used by error.
/// Must be wrap when to use.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorCore {
    pub sub_messages: Vec<SubMessage>,
    pub main_message: String,
    pub errno: usize,
    pub kind: ErrorKind,
    pub loc: Location,
    theme: Theme,
}

impl fmt::Display for ErrorCore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ErrorCore {}

impl ErrorCore {
    pub fn new<S: Into<String>>(
        sub_messages: Vec<SubMessage>,
        main_message: S,
        errno: usize,
        kind: ErrorKind,
        loc: Location,
    ) -> Self {
        Self {
            sub_messages,
            main_message: main_message.into(),
            errno,
            kind,
            loc,
            theme: THEME,
        }
    }

    pub fn dummy(errno: usize) -> Self {
        Self::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            "<dummy>",
            errno,
            DummyError,
            Location::Unknown,
        )
    }

    pub fn unreachable(fn_name: &str, line: u32) -> Self {
        Self::bug(line as usize, Location::Line(line), fn_name, line)
    }

    pub fn bug(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        const URL: StyledStr = StyledStr::new(
            "https://github.com/erg-lang/erg",
            Some(Color::White),
            Some(Attribute::Underline),
        );

        let m_msg = switch_lang!(
            "japanese" => format!("これはErgのバグです、開発者に報告して下さい({URL})\n{fn_name}:{line}より発生"),
            "simplified_chinese" => format!("这是Erg的bug，请报告给{URL}\n原因来自: {fn_name}:{line}"),
            "traditional_chinese" => format!("这是Erg的bug，请报告给{URL}\n原因来自: {fn_name}:{line}"),
            "english" => format!("this is a bug of Erg, please report it to {URL}\ncaused from: {fn_name}:{line}"),
        );
        Self::new(
            vec![SubMessage::only_loc(loc)],
            m_msg,
            errno,
            CompilerSystemError,
            loc,
        )
    }

    pub fn get_loc_with_fallback(&self) -> Location {
        if self.loc == Location::Unknown {
            for sub in &self.sub_messages {
                if sub.loc != Location::Unknown {
                    return sub.loc;
                }
            }
            Location::Unknown
        } else {
            self.loc
        }
    }

    pub fn get_hint(&self) -> Option<&str> {
        for sub in self.sub_messages.iter() {
            if let Some(hint) = &sub.hint {
                return Some(hint);
            }
        }
        None
    }

    pub fn fmt_header(&self, color: Color, caused_by: &str, input: &str) -> String {
        let loc = match self.loc {
            Location::Range {
                ln_begin, ln_end, ..
            } if ln_begin == ln_end => format!(", line {ln_begin}"),
            Location::Range {
                ln_begin, ln_end, ..
            }
            | Location::LineRange(ln_begin, ln_end) => format!(", line {ln_begin}..{ln_end}"),
            Location::Line(lineno) => format!(", line {lineno}"),
            Location::Unknown => "".to_string(),
        };
        let kind = if self.kind.is_error() {
            "Error"
        } else if self.kind.is_warning() {
            "Warning"
        } else {
            "Exception"
        };
        let kind = self.theme.characters.error_kind_format(kind, self.errno);
        format!(
            "{kind}: File {input}{loc}, {caused_by}",
            kind = StyledStr::new(&kind, Some(color), Some(Attribute::Bold))
        )
    }

    fn specified_theme(&self) -> (Color, char) {
        let (color, mark) = if self.kind.is_error() {
            self.theme.error()
        } else if self.kind.is_warning() {
            self.theme.warning()
        } else {
            self.theme.exception()
        };
        (color, mark)
    }
}

/// format:
/// ```txt
/// Error[#{.errno}]: File {file}, line {.loc (as line)}, in {.caused_by}
///
/// {.loc (as line)}| {src}
/// {offset}        : {pointer}
/// {offset}        :         {sub_msgs}
/// {offset}        :         {.hint}
///
/// {.kind}: {.desc}
///
/// ```
///
/// example:
/// ```txt
/// Error[#2223]: File <stdin>, line 1, in <module>
///
/// 1 │ 100 = i
///   · ---
///   ·   │─ sub_msg1: first sub message here
///   ·   │─ sub_msg2: second sub message here
///   ·   ╰─ hint: hint message here
///
/// SyntaxError: cannot assign to 100
///
/// ```
pub trait ErrorDisplay {
    fn core(&self) -> &ErrorCore;
    fn input(&self) -> &Input;
    /// The block name the error caused.
    /// This will be None if the error occurred before semantic analysis.
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
        let core = self.core();
        let (color, mark) = core.specified_theme();
        let (gutter_color, chars) = core.theme.characters();
        let mut msg = String::new();
        msg += &core.fmt_header(color, self.caused_by(), self.input().kind.enclosed_name());
        msg += "\n\n";
        for sub_msg in &core.sub_messages {
            msg += &sub_msg.format_code_and_pointer(self, color, gutter_color, mark, chars);
        }
        if core.sub_messages.is_empty() {
            let sub_msg = SubMessage::ambiguous_new(self.core().loc, vec![], None);
            msg += &sub_msg.format_code_and_pointer(self, color, gutter_color, mark, chars);
        }
        msg += &core.kind.to_string();
        msg += ": ";
        msg += &core.main_message;
        msg += "\n\n";
        msg
    }

    /// for fmt::Display
    fn format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let core = self.core();
        let (color, mark) = core.specified_theme();
        let (gutter_color, chars) = core.theme.characters();
        write!(
            f,
            "{}\n\n",
            core.fmt_header(color, self.caused_by(), self.input().kind.enclosed_name())
        )?;
        for sub_msg in &core.sub_messages {
            write!(
                f,
                "{}",
                &sub_msg.format_code_and_pointer(self, color, gutter_color, mark, chars)
            )?;
        }
        write!(f, "{}\n\n", core.main_message)?;
        if let Some(inner) = self.ref_inner() {
            inner.format(f)
        } else {
            Ok(())
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
