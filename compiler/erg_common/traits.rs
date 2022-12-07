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

#[derive(Debug, PartialEq, Eq)]
pub enum BlockKind {
    Main,          // now_block Vec must contain this
    Assignment,    // =
    Lambda,        // =>, ->, do:, do!:
    MultiLineStr,  // """
    ClassAttr,     // ::
    ClassAttrDecl, // .
    None,          // one line
    Error,         // parser error
}

pub struct VM {
    codes: String,
    now_block: Vec<BlockKind>,
}

impl VM {
    fn new() -> Self {
        Self {
            codes: String::new(),
            now_block: vec![BlockKind::Main],
        }
    }
    fn push_code(&mut self, src: &str) {
        self.codes.push_str(src)
    }
    fn clear(&mut self) {
        self.codes = String::new();
        self.now_block = vec![BlockKind::Main];
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
    fn finish(&mut self); // called when the :exit command is received.
    /// Erase all but immutable information.
    fn initialize(&mut self);
    /// Erase information that will no longer be meaningful in the next iteration
    fn clear(&mut self);
    fn eval(&mut self, src: String) -> Result<String, Self::Errs>;
    fn exec(&mut self) -> Result<i32, Self::Errs>;

    fn expect_block(&self, src: &str) -> BlockKind;

    fn input(&self) -> &Input {
        &self.cfg().input
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
                let mut vm = VM::new();
                loop {
                    let lst_bk = vm.now_block.last().unwrap();
                    let whitespace = if lst_bk == &BlockKind::MultiLineStr {
                        String::new()
                    } else {
                        "    ".repeat(vm.now_block.len() - 1)
                    };
                    if vm.now_block.len() > 1 {
                        output.write_all(instance.ps2().as_bytes()).unwrap();
                        output.write_all(whitespace.as_str().as_bytes()).unwrap();
                        output.flush().unwrap();
                    } else {
                        output.write_all(instance.ps1().as_bytes()).unwrap();
                        output.flush().unwrap();
                    }
                    let line = chomp(&instance.input().read());
                    match &line[..] {
                        ":quit" | ":q" | ":exit" | ":e" => {
                            instance.quit_successfully(output);
                        }
                        ":clear" | ":c" => {
                            output.write_all("\x1b[2J\x1b[1;1H".as_bytes()).unwrap();
                            output.flush().unwrap();
                            vm.clear();
                            instance.clear();
                            continue;
                        }
                        "" => {
                            // Execute after block ends
                            if vm.now_block.len() == 2 {
                                vm.now_block.pop();
                            } else if vm.now_block.len() > 1 {
                                vm.now_block.pop();
                                continue;
                            }
                            match instance.eval(mem::take(&mut vm.codes)) {
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
                            instance.input().init_block_begin();
                            instance.clear();
                            vm.clear();
                            continue;
                        }
                        _ => {}
                    }
                    let line = if let Some(comment_start) = line.find('#') {
                        &line[..comment_start]
                    } else {
                        &line[..]
                    };
                    let bk = instance.expect_block(line);
                    match bk {
                        // eval and cause error
                        BlockKind::None => {
                            let bk = vm.now_block.last().unwrap();
                            if bk == &BlockKind::MultiLineStr {
                                vm.push_code(line);
                                vm.push_code("\n");
                                continue;
                            }
                            vm.push_code(whitespace.as_str());
                            vm.push_code(line);
                            vm.push_code("\n");
                        }
                        BlockKind::Error => {
                            instance.clear();
                            vm.clear();
                            continue;
                        }
                        BlockKind::MultiLineStr => {
                            vm.now_block.push(BlockKind::MultiLineStr);
                            vm.push_code(line);
                            vm.push_code("\n");
                        }
                        _ => {
                            if vm.now_block.len() == 1 {
                                instance.input().set_block_begin();
                            }
                            if let Some(lst_bk) = vm.now_block.last() {
                                if lst_bk != &BlockKind::MultiLineStr {
                                    vm.push_code(whitespace.as_str());
                                    instance.input().insert_whitespace(whitespace.as_str());
                                    vm.now_block.push(bk);
                                }
                                vm.push_code(line);
                                vm.push_code("\n");
                                continue;
                            }
                        }
                    }

                    // single eval
                    if vm.now_block.len() == 1 {
                        match instance.eval(mem::take(&mut vm.codes)) {
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
                        instance.input().init_block_begin();
                        instance.clear();
                        vm.clear();
                    }
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
