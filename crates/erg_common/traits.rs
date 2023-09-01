//! defines common traits used in the compiler.
//!
//! コンパイラ等で汎用的に使われるトレイトを定義する
use std::collections::vec_deque;
use std::collections::VecDeque;
use std::env::consts::{ARCH, OS};
use std::io::{stdout, BufWriter, Write};
use std::mem;
use std::process;
use std::slice::{Iter, IterMut};

use crate::config::ErgConfig;
use crate::consts::{BUILD_DATE, GIT_HASH_SHORT, SEMVER};
use crate::error::{ErrorDisplay, ErrorKind, Location, MultiErrorDisplay};
use crate::io::{Input, InputKind};
use crate::{addr_eq, chomp, log, switch_unreachable};

pub trait DequeStream<T>: Sized {
    fn payload(self) -> VecDeque<T>;
    fn ref_payload(&self) -> &VecDeque<T>;
    fn ref_mut_payload(&mut self) -> &mut VecDeque<T>;

    #[inline]
    fn is_empty(&self) -> bool {
        self.ref_payload().is_empty()
    }

    #[inline]
    fn push(&mut self, elem: T) {
        self.ref_mut_payload().push_back(elem);
    }

    #[inline]
    fn push_front(&mut self, elem: T) {
        self.ref_mut_payload().push_front(elem);
    }

    fn pop_front(&mut self) -> Option<T> {
        self.ref_mut_payload().pop_front()
    }

    #[inline]
    fn get(&self, idx: usize) -> Option<&T> {
        self.ref_payload().get(idx)
    }

    #[inline]
    fn first(&self) -> Option<&T> {
        self.ref_payload().front()
    }

    #[inline]
    fn last(&self) -> Option<&T> {
        self.ref_payload().back()
    }

    #[inline]
    fn iter(&self) -> vec_deque::Iter<'_, T> {
        self.ref_payload().iter()
    }

    #[inline]
    fn len(&self) -> usize {
        self.ref_payload().len()
    }
}

#[macro_export]
macro_rules! impl_displayable_deque_stream_for_wrapper {
    ($Strc: ident, $Inner: ident) => {
        impl $Strc {
            pub const fn new(v: VecDeque<$Inner>) -> $Strc {
                $Strc(v)
            }
            pub fn empty() -> $Strc {
                $Strc(VecDeque::new())
            }
            #[inline]
            pub fn with_capacity(capacity: usize) -> $Strc {
                $Strc(VecDeque::with_capacity(capacity))
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
                erg_common::traits::DequeStream::get(self, idx).unwrap()
            }
        }

        impl From<$Strc> for VecDeque<$Inner> {
            fn from(item: $Strc) -> VecDeque<$Inner> {
                item.payload()
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

        impl IntoIterator for $Strc {
            type Item = $Inner;
            type IntoIter = std::collections::vec_deque::IntoIter<Self::Item>;
            fn into_iter(self) -> Self::IntoIter {
                self.payload().into_iter()
            }
        }

        impl FromIterator<$Inner> for $Strc {
            fn from_iter<I: IntoIterator<Item = $Inner>>(iter: I) -> Self {
                $Strc(iter.into_iter().collect())
            }
        }

        impl $crate::traits::DequeStream<$Inner> for $Strc {
            #[inline]
            fn payload(self) -> VecDeque<$Inner> {
                self.0
            }
            #[inline]
            fn ref_payload(&self) -> &VecDeque<$Inner> {
                &self.0
            }
            #[inline]
            fn ref_mut_payload(&mut self) -> &mut VecDeque<$Inner> {
                &mut self.0
            }
        }
    };
}

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

    fn split_off(&mut self, at: usize) -> Vec<T> {
        self.ref_mut_payload().split_off(at)
    }

    fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        self.ref_mut_payload().retain(f);
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

        impl std::ops::IndexMut<usize> for $Strc {
            fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
                erg_common::traits::Stream::get_mut(self, idx).unwrap()
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

        impl $Strc {
            pub fn iter(&self) -> std::slice::Iter<'_, $Inner> {
                self.ref_payload().iter()
            }
            pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, $Inner> {
                self.ref_mut_payload().iter_mut()
            }
        }
    };
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

        impl $Strc {
            pub fn iter(&self) -> std::slice::Iter<'_, $Inner> {
                self.ref_payload().iter()
            }
            pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, $Inner> {
                self.ref_mut_payload().iter_mut()
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
    /// If `limit` was set to a negative value, it will be displayed without abbreviation.
    /// FIXME:
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> std::fmt::Result;
    fn to_string_unabbreviated(&self) -> String {
        let mut s = "".to_string();
        self.limited_fmt(&mut s, -1).unwrap();
        s
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BlockKind {
    Main,       // now_block Vec must contain this
    Assignment, // =
    ClassPriv,  // ::
    ClassPub,   // .
    ColonCall,  // :
    Error,      // parser error
    Lambda,     // =>, ->, do, do!
    // not block
    AtMark,       // @
    MultiLineStr, // """
    ClassDef,     // class definition
    None,         // one line
}

pub fn from_str(bk: &str) -> BlockKind {
    match bk {
        "Assignment" => BlockKind::Assignment,
        "AtMark" => BlockKind::AtMark,
        "ClassPriv" => BlockKind::ClassPriv,
        "ClassPub" => BlockKind::ClassPub,
        "ColonCall" => BlockKind::ColonCall,
        "Error" => BlockKind::Error,
        "Lambda" => BlockKind::Lambda,
        "MultiLineStr" => BlockKind::MultiLineStr,
        "ClassDef" => BlockKind::ClassDef,
        "None" => BlockKind::None,
        _ => unimplemented!("Failed to convert to BlockKind"),
    }
}

pub struct VirtualMachine {
    pub codes: String,
    pub now_block: Vec<BlockKind>,
    pub now: BlockKind,
    pub length: usize,
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            codes: String::new(),
            now_block: vec![BlockKind::Main],
            now: BlockKind::Main,
            length: 1,
        }
    }

    pub fn push_block_kind(&mut self, bk: BlockKind) {
        // Don't add block to @ after @
        if self.now == BlockKind::AtMark && bk == BlockKind::AtMark {
            return;
        }
        // Change from AtMark to ClassDef or Assignment
        if (bk == BlockKind::ClassDef || bk == BlockKind::Assignment)
            && self.now == BlockKind::AtMark
        {
            self.now_block.pop();
        }
        // Change from ClassDef to ClassPriv or ClassPub
        if (bk == BlockKind::ClassPriv || bk == BlockKind::ClassPub)
            && self.now == BlockKind::ClassDef
        {
            self.now_block.pop();
        }
        self.now = bk;
        self.now_block.push(bk);
        if bk == BlockKind::AtMark || bk == BlockKind::ClassDef {
            return;
        }
        if bk == BlockKind::MultiLineStr {
            self.length = 1;
            return;
        }
        self.length += 1;
    }

    pub fn remove_block_kind(&mut self) {
        self.now_block.pop().unwrap();
        self.now = *self.now_block.last().unwrap();
        self.length -= 1;
    }

    pub fn push_code(&mut self, src: &str) {
        self.codes.push_str(src)
    }

    pub fn clear(&mut self) {
        self.codes = String::new();
        self.now_block = vec![BlockKind::Main];
        self.now = BlockKind::Main;
        self.length = 1;
    }

    pub fn indent(&mut self) -> String {
        if self.now == BlockKind::MultiLineStr {
            String::new()
        } else if self.length == 0 {
            self.length = 1;
            "    ".repeat(0)
        } else {
            "    ".repeat(self.length - 1) // Except MainBlock
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ExitStatus {
    pub code: i32,
    pub num_warns: usize,
    pub num_errors: usize,
}

impl ExitStatus {
    pub const OK: ExitStatus = ExitStatus::new(0, 0, 0);
    pub const ERR1: ExitStatus = ExitStatus::new(1, 0, 0);

    pub const fn new(code: i32, num_warns: usize, num_errors: usize) -> Self {
        Self {
            code,
            num_warns,
            num_errors,
        }
    }
    pub const fn compile_passed(num_warns: usize) -> Self {
        Self::new(0, num_warns, 0)
    }

    pub const fn succeed(&self) -> bool {
        self.code == 0 && self.num_errors == 0
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
    fn exec(&mut self) -> Result<ExitStatus, Self::Errs>;
    fn expect_block(&self, src: &str) -> BlockKind {
        let multi_line_str = "\"\"\"";
        if src.contains(multi_line_str) && src.rfind(multi_line_str) == src.find(multi_line_str) {
            return BlockKind::MultiLineStr;
        }
        if src.trim_start().starts_with('@') {
            return BlockKind::AtMark;
        }
        if src.ends_with("do!:") && !src.starts_with("do!:") {
            return BlockKind::Lambda;
        }
        if src.ends_with("do:") && !src.starts_with("do:") {
            return BlockKind::Lambda;
        }
        if src.ends_with(':') && !src.starts_with(':') {
            return BlockKind::Lambda;
        }
        if src.ends_with('=') && !src.starts_with('=') {
            return BlockKind::Assignment;
        }
        if src.ends_with('.') && !src.starts_with('.') {
            return BlockKind::ClassPub;
        }
        if src.ends_with("::") && !src.starts_with("::") {
            return BlockKind::ClassPriv;
        }
        if src.ends_with("=>") && !src.starts_with("=>") {
            return BlockKind::Lambda;
        }
        if src.ends_with("->") && !src.starts_with("->") {
            return BlockKind::Lambda;
        }
        if src.contains("Class") || src.contains("Inherit") {
            return BlockKind::ClassDef;
        }
        BlockKind::None
    }
    fn input(&self) -> &Input {
        &self.cfg().input
    }
    fn set_input(&mut self, input: Input) {
        self.cfg_mut().input = input;
    }
    fn start_message(&self) -> String {
        if GIT_HASH_SHORT.is_empty() {
            format!("{} {SEMVER} ({BUILD_DATE}) on {ARCH}/{OS}\n", Self::NAME)
        } else {
            format!(
                "{} {SEMVER} ({GIT_HASH_SHORT}, {BUILD_DATE}) on {ARCH}/{OS}\n",
                Self::NAME
            )
        }
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

    fn run(cfg: ErgConfig) -> ExitStatus {
        let quiet_repl = cfg.quiet_repl;
        let mut num_errors = 0;
        let mut instance = Self::new(cfg);
        let res = match &instance.input().kind {
            InputKind::File(_) | InputKind::Pipe(_) | InputKind::Str(_) => instance.exec(),
            InputKind::REPL | InputKind::DummyREPL(_) => {
                let output = stdout();
                let mut output = BufWriter::new(output.lock());
                if !quiet_repl {
                    log!(info_f output, "The REPL has started.\n");
                    output
                        .write_all(instance.start_message().as_bytes())
                        .unwrap();
                }
                output.flush().unwrap();
                let mut vm = VirtualMachine::new();
                loop {
                    let indent = vm.indent();
                    if vm.now_block.len() > 1 {
                        output.write_all(instance.ps2().as_bytes()).unwrap();
                        output.write_all(indent.as_str().as_bytes()).unwrap();
                        output.flush().unwrap();
                    } else {
                        output.write_all(instance.ps1().as_bytes()).unwrap();
                        output.flush().unwrap();
                    }
                    instance.cfg().input.set_indent(vm.length);
                    let line = chomp(&instance.cfg_mut().input.read());
                    let line = line.trim_end();
                    match line {
                        ":quit" | ":exit" => {
                            instance.quit_successfully(output);
                        }
                        ":clear" | ":cln" => {
                            output.write_all("\x1b[2J\x1b[1;1H".as_bytes()).unwrap();
                            output.flush().unwrap();
                            vm.clear();
                            instance.clear();
                            continue;
                        }
                        "" => {
                            // eval after the end of the block
                            if vm.now_block.len() == 2 {
                                vm.remove_block_kind();
                            } else if vm.now_block.len() > 1 {
                                vm.remove_block_kind();
                                vm.push_code("\n");
                                continue;
                            }
                            match instance.eval(mem::take(&mut vm.codes)) {
                                Ok(out) if out.is_empty() => continue,
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
                                    num_errors += errs.len();
                                    errs.write_all_stderr();
                                }
                            }
                            instance.input().set_block_begin();
                            instance.clear();
                            vm.clear();
                            continue;
                        }
                        _ => {}
                    }
                    let line = if let Some(comment_start) = line.find('#') {
                        &line[..comment_start]
                    } else {
                        line
                    };
                    let bk = instance.expect_block(line);
                    match bk {
                        BlockKind::None if vm.now == BlockKind::AtMark => {
                            if let Some(eq) = line.find('=') {
                                if let Some(class) = line.find("Class") {
                                    if eq < class {
                                        vm.push_code(indent.as_str());
                                        instance.input().insert_whitespace(indent.as_str());
                                        vm.push_code(line);
                                        vm.push_code("\n");
                                        vm.push_block_kind(bk);
                                        continue;
                                    }
                                }
                                vm.push_code(indent.as_str());
                                instance.input().insert_whitespace(indent.as_str());
                                vm.push_code(line);
                                vm.push_code("\n");
                                continue;
                            }
                            // Intentionally code will be evaluated and make an error
                            vm.now = BlockKind::Main;
                        }
                        BlockKind::ClassDef | BlockKind::Assignment
                            if vm.now == BlockKind::AtMark =>
                        {
                            vm.push_block_kind(bk);
                            vm.push_code(indent.as_str());
                            instance.input().insert_whitespace(indent.as_str());
                            vm.push_code(line);
                            vm.push_code("\n");
                            continue;
                        }
                        // Intentionally code will be evaluated and make an error
                        _ if vm.now == BlockKind::AtMark => {
                            vm.push_code(line);
                            vm.push_code("\n");
                            vm.now = BlockKind::Main;
                        }
                        BlockKind::None if vm.now == BlockKind::MultiLineStr => {
                            vm.push_code(line);
                            vm.push_code("\n");
                            continue;
                        }
                        // single eval
                        BlockKind::None => {
                            vm.push_code(indent.as_str());
                            instance.input().insert_whitespace(indent.as_str());
                            vm.push_code(line);
                            vm.push_code("\n");
                        }
                        BlockKind::Error => {
                            vm.push_code(indent.as_str());
                            instance.input().insert_whitespace(indent.as_str());
                            vm.push_code(line);
                            vm.now = BlockKind::Main;
                            vm.now_block = vec![BlockKind::Main];
                        }
                        // end of MultiLineStr
                        BlockKind::MultiLineStr if vm.now == BlockKind::MultiLineStr => {
                            vm.remove_block_kind();
                            vm.length = vm.now_block.len();
                            vm.push_code(line);
                            vm.push_code("\n");
                        }
                        // start of MultiLineStr
                        BlockKind::MultiLineStr => {
                            vm.push_block_kind(BlockKind::MultiLineStr);
                            vm.push_code(indent.as_str());
                            instance.input().insert_whitespace(indent.as_str());
                            vm.push_code(line);
                            vm.push_code("\n");
                            continue;
                        }
                        // block is expected but string
                        _ if vm.now == BlockKind::MultiLineStr => {
                            vm.push_code(line);
                            vm.push_code("\n");
                            continue;
                        }
                        // expect block
                        _ => {
                            if vm.length == 1 {
                                instance.input().set_block_begin();
                            }
                            vm.push_code(indent.as_str());
                            instance.input().insert_whitespace(indent.as_str());
                            vm.push_block_kind(bk);
                            vm.push_code(line);
                            vm.push_code("\n");
                            continue;
                        }
                    }

                    if vm.now == BlockKind::Main {
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
                                    return ExitStatus::new(0, 0, num_errors);
                                }
                                num_errors += errs.len();
                                errs.write_all_stderr();
                            }
                        }
                        instance.input().set_block_begin();
                        instance.clear();
                        vm.clear();
                    }
                }
            }
            InputKind::Dummy => switch_unreachable!(),
        };
        match res {
            Ok(status) => status,
            Err(errs) => {
                num_errors += errs.len();
                errs.write_all_stderr();
                ExitStatus::new(1, 0, num_errors)
            }
        }
    }
}

pub trait Locational {
    /// NOTE: `loc` cannot be treated as a light method when `self` is a large grammatical element.
    /// If possible, delay the computation by passing `&impl Locational` or other means.
    fn loc(&self) -> Location;

    /// 1-origin
    fn ln_begin(&self) -> Option<u32> {
        match self.loc() {
            Location::Range { ln_begin, .. } | Location::LineRange(ln_begin, _) => Some(ln_begin),
            Location::Line(lineno) => Some(lineno),
            Location::Unknown => None,
        }
    }

    fn ln_end(&self) -> Option<u32> {
        match self.loc() {
            Location::Range { ln_end, .. } | Location::LineRange(_, ln_end) => Some(ln_end),
            Location::Line(lineno) => Some(lineno),
            Location::Unknown => None,
        }
    }

    /// 0-origin
    fn col_begin(&self) -> Option<u32> {
        match self.loc() {
            Location::Range { col_begin, .. } => Some(col_begin),
            _ => None,
        }
    }

    fn col_end(&self) -> Option<u32> {
        match self.loc() {
            Location::Range { col_end, .. } => Some(col_end),
            _ => None,
        }
    }
}

impl<L: Locational> Locational for Option<L> {
    fn loc(&self) -> Location {
        match self {
            Some(l) => l.loc(),
            None => Location::Unknown,
        }
    }
}

impl<L: Locational, R: Locational> Locational for Result<&L, &R> {
    fn loc(&self) -> Location {
        match self {
            Ok(l) => l.loc(),
            Err(r) => r.loc(),
        }
    }
}

impl<L: Locational, R: Locational> Locational for (&L, &R) {
    fn loc(&self) -> Location {
        Location::concat(self.0, self.1)
    }
}

impl Locational for () {
    fn loc(&self) -> Location {
        Location::Unknown
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
                let begin_loc = self.$begin.loc();
                let end_loc = self.$end.loc();
                match (
                    begin_loc.ln_begin(),
                    begin_loc.col_begin(),
                    end_loc.ln_end(),
                    end_loc.col_end(),
                ) {
                    (Some(lb), Some(cb), Some(le), Some(ce)) => Location::range(lb, cb, le, ce),
                    (Some(lb), _, Some(le), _) => Location::LineRange(lb, le),
                    (Some(l), _, _, _) | (_, _, Some(l), _) => Location::Line(l),
                    _ => Location::Unknown,
                }
            }
            fn ln_begin(&self) -> Option<u32> {
                self.$begin.ln_begin()
            }
            fn ln_end(&self) -> Option<u32> {
                self.$end.ln_end()
            }
            fn col_begin(&self) -> Option<u32> {
                self.$begin.col_begin()
            }
            fn col_end(&self) -> Option<u32> {
                self.$end.col_end()
            }
        }
    };
    ($T: ty, lossy $begin: ident, $end: ident) => {
        impl Locational for $T {
            fn loc(&self) -> Location {
                let begin_loc = self.$begin.loc();
                let end_loc = self.$end.loc();
                if begin_loc.is_unknown() {
                    return end_loc;
                }
                match (
                    begin_loc.ln_begin(),
                    begin_loc.col_begin(),
                    end_loc.ln_end(),
                    end_loc.col_end(),
                ) {
                    (Some(lb), Some(cb), Some(le), Some(ce)) => Location::range(lb, cb, le, ce),
                    (Some(lb), _, Some(le), _) => Location::LineRange(lb, le),
                    (Some(l), _, _, _) | (_, _, Some(l), _) => Location::Line(l),
                    _ => Location::Unknown,
                }
            }
        }
    };
    ($T: ty, $begin: ident, $middle: ident, $end: ident) => {
        impl Locational for $T {
            fn loc(&self) -> Location {
                let begin_loc = self.$begin.loc();
                let end_loc = self.$end.loc();
                if begin_loc.is_unknown() && end_loc.is_unknown() {
                    return self.$middle.loc();
                }
                match (
                    begin_loc.ln_begin(),
                    begin_loc.col_begin(),
                    end_loc.ln_end(),
                    end_loc.col_end(),
                ) {
                    (Some(lb), Some(cb), Some(le), Some(ce)) => Location::range(lb, cb, le, ce),
                    (Some(lb), _, Some(le), _) => Location::LineRange(lb, le),
                    (Some(l), _, _, _) | (_, _, Some(l), _) => Location::Line(l),
                    _ => Location::Unknown,
                }
            }
            fn ln_begin(&self) -> Option<u32> {
                self.$begin.ln_begin()
            }
            fn ln_end(&self) -> Option<u32> {
                self.$end.ln_end()
            }
            fn col_begin(&self) -> Option<u32> {
                self.$begin.col_begin()
            }
            fn col_end(&self) -> Option<u32> {
                self.$end.col_end()
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
macro_rules! impl_from_trait_for_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        $(
            impl From<$Variant> for $Enum {
                fn from(v: $Variant) -> Self {
                    $Enum::$Variant(v)
                }
            }
        )*
    }
}

#[macro_export]
macro_rules! impl_try_from_trait_for_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        $(
            impl TryFrom<$Enum> for $Variant {
                type Error = $Enum;
                fn try_from(from: $Enum) -> Result<Self, Self::Error> {
                    match from {
                        Expr::$Variant(to) => Ok(to),
                        _ => Err(from),
                    }
                }
            }
        )*
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

/// Equivalent to `is` in Python
pub trait AddrEq {
    #[inline]
    fn addr_eq(&self, other: &Self) -> bool {
        addr_eq!(self, other)
    }
}

pub trait StructuralEq {
    fn structural_eq(&self, other: &Self) -> bool;
}

pub trait __Str__ {
    fn __str__(&self) -> String;
}
