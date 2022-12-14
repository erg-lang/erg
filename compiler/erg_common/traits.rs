//! defines common traits used in the compiler.
//!
//! コンパイラ等で汎用的に使われるトレイトを定義する
use std::env::consts::{ARCH, OS};
use std::io::{stdout, BufWriter, Write};
use std::mem;
use std::process;
use std::slice::{Iter, IterMut};

use crate::config::{ErgConfig, Input, BUILD_DATE, GIT_HASH_SHORT, SEMVER};
use crate::error::{ErrorDisplay, ErrorKind, Location, MultiErrorDisplay};
use crate::{addr_eq, chomp, log, switch_unreachable};

pub trait Stream<T>: Sized {
    fn payload(self) -> Vec<T>;
    fn ref_payload(&self) -> &Vec<T>;
    fn ref_mut_payload(&mut self) -> &mut Vec<T>;

    #[inline]
    fn clear(&mut self) {
        self.ref_mut_payload().clear();
    }

    #[inline]
    fn len(&self) -> usize {
        self.ref_payload().len()
    }

    fn size(&self) -> usize {
        std::mem::size_of::<Vec<T>>() + std::mem::size_of::<T>() * self.ref_payload().capacity()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.ref_payload().is_empty()
    }

    #[inline]
    fn insert(&mut self, idx: usize, elem: T) {
        self.ref_mut_payload().insert(idx, elem);
    }

    #[inline]
    fn remove(&mut self, idx: usize) -> T {
        self.ref_mut_payload().remove(idx)
    }

    #[inline]
    fn push(&mut self, elem: T) {
        self.ref_mut_payload().push(elem);
    }

    fn append<S: Stream<T>>(&mut self, s: &mut S) {
        self.ref_mut_payload().append(s.ref_mut_payload());
    }

    #[inline]
    fn pop(&mut self) -> Option<T> {
        self.ref_mut_payload().pop()
    }

    fn lpop(&mut self) -> Option<T> {
        let len = self.len();
        if len == 0 {
            None
        } else {
            Some(self.ref_mut_payload().remove(0))
        }
    }

    #[inline]
    fn get(&self, idx: usize) -> Option<&T> {
        self.ref_payload().get(idx)
    }

    #[inline]
    fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.ref_mut_payload().get_mut(idx)
    }

    #[inline]
    fn first(&self) -> Option<&T> {
        self.ref_payload().first()
    }

    #[inline]
    fn first_mut(&mut self) -> Option<&mut T> {
        self.ref_mut_payload().first_mut()
    }

    #[inline]
    fn last(&self) -> Option<&T> {
        self.ref_payload().last()
    }

    #[inline]
    fn last_mut(&mut self) -> Option<&mut T> {
        self.ref_mut_payload().last_mut()
    }

    #[inline]
    fn iter(&self) -> Iter<'_, T> {
        self.ref_payload().iter()
    }

    #[inline]
    fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.ref_mut_payload().iter_mut()
    }

    #[inline]
    fn take_all(&mut self) -> Vec<T> {
        self.ref_mut_payload().drain(..).collect()
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.ref_mut_payload().extend(iter);
    }
}

#[macro_export]
macro_rules! impl_displayable_stream_for_wrapper {
    ($Strc: ident, $Inner: ident) => {
        impl $Strc {
            pub const fn new(v: Vec<$Inner>) -> $Strc {
                $Strc(v)
            }
            #[inline]
            pub fn empty() -> $Strc {
                $Strc(Vec::with_capacity(20))
            }
        }

        impl From<Vec<$Inner>> for $Strc {
            #[inline]
            fn from(errs: Vec<$Inner>) -> Self {
                Self(errs)
            }
        }

        impl IntoIterator for $Strc {
            type Item = $Inner;
            type IntoIter = std::vec::IntoIter<Self::Item>;
            fn into_iter(self) -> Self::IntoIter {
                self.payload().into_iter()
            }
        }

        impl FromIterator<$Inner> for $Strc {
            fn from_iter<I: IntoIterator<Item = $Inner>>(iter: I) -> Self {
                $Strc(iter.into_iter().collect())
            }
        }

        impl std::fmt::Display for $Strc {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "[{}]",
                    erg_common::fmt_iter(self.iter()).replace("\n", "\\n")
                )
            }
        }

        impl Default for $Strc {
            #[inline]
            fn default() -> Self {
                Self::empty()
            }
        }

        impl std::ops::Index<usize> for $Strc {
            type Output = $Inner;
            fn index(&self, idx: usize) -> &Self::Output {
                erg_common::traits::Stream::get(self, idx).unwrap()
            }
        }

        impl erg_common::traits::Stream<$Inner> for $Strc {
            #[inline]
            fn payload(self) -> Vec<$Inner> {
                self.0
            }
            #[inline]
            fn ref_payload(&self) -> &Vec<$Inner> {
                &self.0
            }
            #[inline]
            fn ref_mut_payload(&mut self) -> &mut Vec<$Inner> {
                &mut self.0
            }
        }
    };
}

#[macro_export]
macro_rules! impl_stream_for_wrapper {
    ($Strc: ident, $Inner: ident) => {
        impl $Strc {
            pub const fn new(v: Vec<$Inner>) -> $Strc {
                $Strc(v)
            }
            pub const fn empty() -> $Strc {
                $Strc(Vec::new())
            }
            #[inline]
            pub fn with_capacity(capacity: usize) -> $Strc {
                $Strc(Vec::with_capacity(capacity))
            }
        }

        impl Default for $Strc {
            #[inline]
            fn default() -> $Strc {
                $Strc::with_capacity(0)
            }
        }

        impl std::ops::Index<usize> for $Strc {
            type Output = $Inner;
            fn index(&self, idx: usize) -> &Self::Output {
                erg_common::traits::Stream::get(self, idx).unwrap()
            }
        }

        impl From<$Strc> for Vec<$Inner> {
            fn from(item: $Strc) -> Vec<$Inner> {
                item.payload()
            }
        }

        impl IntoIterator for $Strc {
            type Item = $Inner;
            type IntoIter = std::vec::IntoIter<Self::Item>;
            fn into_iter(self) -> Self::IntoIter {
                self.payload().into_iter()
            }
        }

        impl FromIterator<$Inner> for $Strc {
            fn from_iter<I: IntoIterator<Item = $Inner>>(iter: I) -> Self {
                $Strc(iter.into_iter().collect())
            }
        }

        impl $crate::traits::Stream<$Inner> for $Strc {
            #[inline]
            fn payload(self) -> Vec<$Inner> {
                self.0
            }
            #[inline]
            fn ref_payload(&self) -> &Vec<$Inner> {
                &self.0
            }
            #[inline]
            fn ref_mut_payload(&mut self) -> &mut Vec<$Inner> {
                &mut self.0
            }
        }
    };
}

#[macro_export]
macro_rules! impl_stream {
    ($Strc: ident, $Inner: ident, $field: ident) => {
        impl $crate::traits::Stream<$Inner> for $Strc {
            #[inline]
            fn payload(self) -> Vec<$Inner> {
                self.$field
            }
            #[inline]
            fn ref_payload(&self) -> &Vec<$Inner> {
                &self.$field
            }
            #[inline]
            fn ref_mut_payload(&mut self) -> &mut Vec<$Inner> {
                &mut self.$field
            }
        }

        impl std::ops::Index<usize> for $Strc {
            type Output = $Inner;
            fn index(&self, idx: usize) -> &Self::Output {
                erg_common::traits::Stream::get(self, idx).unwrap()
            }
        }

        impl From<$Strc> for Vec<$Inner> {
            fn from(item: $Strc) -> Vec<$Inner> {
                item.payload()
            }
        }

        impl IntoIterator for $Strc {
            type Item = $Inner;
            type IntoIter = std::vec::IntoIter<Self::Item>;
            fn into_iter(self) -> Self::IntoIter {
                self.payload().into_iter()
            }
        }
    };
}

pub trait ImmutableStream<T>: Sized {
    fn ref_payload(&self) -> &[T];
    fn capacity(&self) -> usize;

    #[inline]
    fn len(&self) -> usize {
        self.ref_payload().len()
    }

    fn size(&self) -> usize {
        std::mem::size_of::<Vec<T>>() + std::mem::size_of::<T>() * self.capacity()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.ref_payload().is_empty()
    }

    #[inline]
    fn get(&self, idx: usize) -> Option<&T> {
        self.ref_payload().get(idx)
    }

    #[inline]
    fn first(&self) -> Option<&T> {
        self.ref_payload().first()
    }

    #[inline]
    fn last(&self) -> Option<&T> {
        self.ref_payload().last()
    }

    #[inline]
    fn iter(&self) -> Iter<'_, T> {
        self.ref_payload().iter()
    }
}

pub trait LimitedDisplay {
    fn limited_fmt(&self, f: &mut std::fmt::Formatter<'_>, limit: usize) -> std::fmt::Result;
}

// for Runnable::run
fn expect_block(src: &str) -> bool {
    let src = src.trim_end();
    src.ends_with(&['.', '=', ':'])
        || src.ends_with("->")
        || src.ends_with("=>")
        // when `"""` are on the same line
        // e.g. """something"""
        || src.contains("\"\"\"") && src.find("\"\"\"") == src.rfind("\"\"\"")
}

// In the REPL, it is invalid for these symbols to be at the beginning of a line
fn expect_invalid_block(src: &str) -> bool {
    let src = src.trim_start();
    src.starts_with(['.', '=', ':']) || src.starts_with("->") || src.starts_with("=>")
}

fn is_in_the_expected_block(src: &str, lines: &str, in_block: &mut bool) -> bool {
    if lines.contains("\"\"\"") {
        if src.ends_with("\"\"\"") {
            *in_block = false;
            false
        } else {
            true
        }
    } else {
        *in_block = false;
        !expect_invalid_block(src) || src.starts_with(' ') && lines.contains('\n')
    }
}

/// This trait implements REPL (Read-Eval-Print-Loop) automatically
/// The `exec` method is called for file input, etc.
pub trait Runnable: Sized + Default {
    type Err: ErrorDisplay;
    type Errs: MultiErrorDisplay<Self::Err>;
    const NAME: &'static str;
    fn new(cfg: ErgConfig) -> Self;
    fn cfg(&self) -> &ErgConfig;
    fn cfg_mut(&mut self) -> &mut ErgConfig;
    fn finish(&mut self); // called when the :exit command is received.
    /// Erase all but immutable information.
    fn initialize(&mut self);
    /// Erase information that will no longer be meaningful in the next iteration
    fn clear(&mut self);
    fn eval(&mut self, src: String) -> Result<String, Self::Errs>;
    fn exec(&mut self) -> Result<i32, Self::Errs>;

    fn input(&self) -> &Input {
        &self.cfg().input
    }
    fn set_input(&mut self, input: Input) {
        self.cfg_mut().input = input;
    }
    fn start_message(&self) -> String {
        format!(
            "{} {SEMVER} (tags/?:{GIT_HASH_SHORT}, {BUILD_DATE}) on {ARCH}/{OS}\n",
            Self::NAME
        )
    }
    fn ps1(&self) -> String {
        self.cfg().ps1.to_string()
    }
    fn ps2(&self) -> String {
        self.cfg().ps2.to_string()
    }

    #[inline]
    fn quit(&mut self, code: i32) -> ! {
        self.finish();
        process::exit(code);
    }

    fn quit_successfully(&mut self, mut output: BufWriter<std::io::StdoutLock>) -> ! {
        self.finish();
        if !self.cfg().quiet_repl {
            log!(info_f output, "The REPL has finished successfully.\n");
        }
        process::exit(0);
    }

    fn run(cfg: ErgConfig) {
        let quiet_repl = cfg.quiet_repl;
        let mut instance = Self::new(cfg);
        let res = match instance.input() {
            Input::File(_) | Input::Pipe(_) | Input::Str(_) => instance.exec(),
            Input::REPL => {
                let output = stdout();
                let mut output = BufWriter::new(output.lock());
                if !quiet_repl {
                    log!(info_f output, "The REPL has started.\n");
                    output
                        .write_all(instance.start_message().as_bytes())
                        .unwrap();
                }
                output.write_all(instance.ps1().as_bytes()).unwrap();
                output.flush().unwrap();
                let mut in_block = false;
                let mut lines = String::new();
                loop {
                    let line = chomp(&instance.input().read());
                    match &line[..] {
                        ":quit" | ":exit" => {
                            instance.quit_successfully(output);
                        }
                        ":clear" => {
                            output.write_all("\x1b[2J\x1b[1;1H".as_bytes()).unwrap();
                            output.flush().unwrap();
                            output.write_all(instance.ps1().as_bytes()).unwrap();
                            output.flush().unwrap();
                            instance.clear();
                            continue;
                        }
                        _ => {}
                    }
                    let line = if let Some(comment_start) = line.find('#') {
                        &line[..comment_start]
                    } else {
                        &line[..]
                    };
                    lines.push_str(line);

                    if in_block {
                        if is_in_the_expected_block(line, &lines, &mut in_block) {
                            lines += "\n";
                            output.write_all(instance.ps2().as_bytes()).unwrap();
                            output.flush().unwrap();
                            continue;
                        }
                    } else if expect_block(line) {
                        in_block = true;
                        lines += "\n";
                        output.write_all(instance.ps2().as_bytes()).unwrap();
                        output.flush().unwrap();
                        continue;
                    }

                    match instance.eval(mem::take(&mut lines)) {
                        Ok(out) => {
                            output.write_all((out + "\n").as_bytes()).unwrap();
                            output.flush().unwrap();
                        }
                        Err(errs) => {
                            if errs
                                .first()
                                .map(|e| e.core().kind == ErrorKind::SystemExit)
                                .unwrap_or(false)
                            {
                                instance.quit_successfully(output);
                            }
                            errs.fmt_all_stderr();
                        }
                    }
                    output.write_all(instance.ps1().as_bytes()).unwrap();
                    output.flush().unwrap();
                    instance.clear();
                }
            }
            Input::Dummy => switch_unreachable!(),
        };
        if let Err(e) = res {
            e.fmt_all_stderr();
            instance.quit(1);
        }
    }
}

pub trait Locational {
    fn loc(&self) -> Location;

    fn ln_begin(&self) -> Option<usize> {
        match self.loc() {
            Location::Range { ln_begin, .. } | Location::LineRange(ln_begin, _) => Some(ln_begin),
            Location::Line(lineno) => Some(lineno),
            Location::Unknown => None,
        }
    }

    fn ln_end(&self) -> Option<usize> {
        match self.loc() {
            Location::Range { ln_end, .. } | Location::LineRange(_, ln_end) => Some(ln_end),
            Location::Line(lineno) => Some(lineno),
            Location::Unknown => None,
        }
    }

    fn col_begin(&self) -> Option<usize> {
        match self.loc() {
            Location::Range { col_begin, .. } => Some(col_begin),
            _ => None,
        }
    }

    fn col_end(&self) -> Option<usize> {
        match self.loc() {
            Location::Range { col_end, .. } => Some(col_end),
            _ => None,
        }
    }
}

#[macro_export]
macro_rules! impl_locational_for_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        impl erg_common::traits::Locational for $Enum {
            fn loc(&self) -> erg_common::error::Location {
                match self {
                    $($Enum::$Variant(v) => v.loc(),)*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! impl_locational {
    ($T: ty, $begin: ident, $end: ident) => {
        impl Locational for $T {
            fn loc(&self) -> Location {
                match (
                    self.$begin.ln_begin(),
                    self.$begin.col_begin(),
                    self.$end.ln_end(),
                    self.$end.col_end(),
                ) {
                    (Some(lb), Some(cb), Some(le), Some(ce)) => Location::range(lb, cb, le, ce),
                    (Some(lb), _, Some(le), _) => Location::LineRange(lb, le),
                    (Some(l), _, _, _) | (_, _, Some(l), _) => Location::Line(l),
                    _ => Location::Unknown,
                }
            }
        }
    };
    ($T: ty, $inner: ident) => {
        impl Locational for $T {
            fn loc(&self) -> Location {
                self.$inner.loc()
            }
        }
    };
}

pub trait NestedDisplay {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result;
}

pub trait NoTypeDisplay {
    fn to_string_notype(&self) -> String;
}

/// `impl<T: NestedDisplay> Display for T NestedDisplay`はorphan-ruleに違反するので個別定義する
#[macro_export]
macro_rules! impl_display_from_nested {
    ($T: ty) => {
        impl std::fmt::Display for $T {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.fmt_nest(f, 0)
            }
        }
    };
}

/// For Decl, Def, Call, etc., which can occupy a line by itself
#[macro_export]
macro_rules! impl_nested_display_for_chunk_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        impl $crate::traits::NestedDisplay for $Enum {
            fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
                write!(f, "{}", "    ".repeat(level))?;
                match self {
                    $($Enum::$Variant(v) => v.fmt_nest(f, level),)*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! impl_nested_display_for_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        impl $crate::traits::NestedDisplay for $Enum {
            fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
                match self {
                    $($Enum::$Variant(v) => v.fmt_nest(f, level),)*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! impl_no_type_display_for_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        impl $crate::traits::NoTypeDisplay for $Enum {
            fn to_string_notype(&self) -> String {
                match self {
                    $($Enum::$Variant(v) => v.to_string_notype(),)*
                }
            }
        }
    }
}

/// Pythonではis演算子に相当
pub trait AddrEq {
    #[inline]
    fn addr_eq(&self, other: &Self) -> bool {
        addr_eq!(self, other)
    }
}

pub trait __Str__ {
    fn __str__(&self) -> String;
}
