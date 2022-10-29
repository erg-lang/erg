//! defines `ParseError` and others.
//!
//! パーサーが出すエラーを定義
use erg_common::astr::AtomicStr;
use erg_common::config::Input;
use erg_common::error::{ErrorCore, ErrorDisplay, ErrorKind::*, Location, MultiErrorDisplay};
use erg_common::style::{RED, RESET};
use erg_common::traits::Stream;
use erg_common::{impl_display_and_error, impl_stream_for_wrapper, switch_lang};

#[derive(Debug)]
pub struct LexError(Box<ErrorCore>); // ErrorCore is large, so use Box

impl From<ErrorCore> for LexError {
    fn from(core: ErrorCore) -> Self {
        Self(Box::new(core))
    }
}

impl From<LexError> for ErrorCore {
    fn from(err: LexError) -> Self {
        *err.0
    }
}

#[derive(Debug)]
pub struct LexErrors(Vec<LexError>);

impl_stream_for_wrapper!(LexErrors, LexError);

impl LexError {
    pub fn new(core: ErrorCore) -> Self {
        Self(Box::new(core))
    }

    pub fn set_hint<S: Into<AtomicStr>>(&mut self, hint: S) {
        self.0.hint = Some(hint.into());
    }

    pub fn compiler_bug(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::new(
            errno,
            CompilerSystemError,
            loc,
            switch_lang!(
                "japanese" => format!("これはErg compilerのバグです、開発者に報告して下さい (https://github.com/erg-lang/erg)\n{fn_name}:{line}より発生"),
                "simplified_chinese" => format!("这是Erg编译器的一个错误，请报告给https://github.com/erg-lang/erg\n原因来自: {fn_name}:{line}"),
                "traditional_chinese" => format!("這是Erg編譯器的一個錯誤，請報告給https://github.com/erg-lang/erg\n原因來自: {fn_name}:{line}"),
                "english" => format!("this is a bug of the Erg compiler, please report it to https://github.com/erg-lang/erg\ncaused from: {fn_name}:{line}"),
            ),
            None,
        ))
    }

    pub fn feature_error(errno: usize, loc: Location, name: &str) -> Self {
        Self::new(ErrorCore::new(
            errno,
            FeatureError,
            loc,
            switch_lang!(
                "japanese" => format!("この機能({name})はまだ正式に提供されていません"),
                "simplified_chinese" => format!("此功能（{name}）尚未实现"),
                "traditional_chinese" => format!("此功能（{name}）尚未實現"),
                "english" => format!("this feature({name}) is not implemented yet"),
            ),
            None,
        ))
    }

    pub fn simple_syntax_error(errno: usize, loc: Location) -> Self {
        Self::new(ErrorCore::new(
            errno,
            SyntaxError,
            loc,
            switch_lang!(
                "japanese" => "不正な構文です",
                "simplified_chinese" => "无效的语法",
                "traditional_chinese" => "無效的語法",
                "english" => "invalid syntax",
            ),
            None,
        ))
    }

    pub fn syntax_error<S: Into<AtomicStr>>(
        errno: usize,
        loc: Location,
        desc: S,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self::new(ErrorCore::new(errno, SyntaxError, loc, desc, hint))
    }

    pub fn syntax_warning<S: Into<AtomicStr>>(
        errno: usize,
        loc: Location,
        desc: S,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self::new(ErrorCore::new(errno, SyntaxWarning, loc, desc, hint))
    }

    pub fn no_var_error(
        errno: usize,
        loc: Location,
        name: &str,
        similar_name: Option<String>,
    ) -> Self {
        let hint = similar_name.map(|n| {
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
            .into()
        });
        Self::new(ErrorCore::new(
            errno,
            NameError,
            loc,
            switch_lang!(
                "japanese" => format!("{RED}{name}{RESET}という変数は定義されていません"),
                "simplified_chinese" => format!("{RED}{name}{RESET}未定义"),
                "traditional_chinese" => format!("{RED}{name}{RESET}未定義"),
                "english" => format!("{RED}{name}{RESET} is not defined"),
            ),
            hint,
        ))
    }
}

pub type LexResult<T> = Result<T, LexError>;

pub type ParseError = LexError;
pub type ParseErrors = LexErrors;
pub type ParseResult<T> = Result<T, ()>;

#[derive(Debug)]
pub struct DesugaringError {
    pub core: ErrorCore,
}

impl DesugaringError {
    pub const fn new(core: ErrorCore) -> Self {
        Self { core }
    }
}

#[derive(Debug)]
pub struct DesugaringErrors(Vec<DesugaringError>);

impl_stream_for_wrapper!(DesugaringErrors, DesugaringError);

pub type DesugaringResult<T> = Result<T, DesugaringError>;

#[derive(Debug)]
pub struct ParserRunnerError {
    pub core: ErrorCore,
    pub input: Input,
}

impl_display_and_error!(ParserRunnerError);

impl ErrorDisplay for ParserRunnerError {
    fn core(&self) -> &ErrorCore {
        &self.core
    }
    fn input(&self) -> &Input {
        &self.input
    }
    fn caused_by(&self) -> &str {
        ""
    }
    fn ref_inner(&self) -> Option<&Self> {
        None
    }
}

impl ParserRunnerError {
    pub const fn new(core: ErrorCore, input: Input) -> Self {
        Self { core, input }
    }
}

#[derive(Debug)]
pub struct ParserRunnerErrors(Vec<ParserRunnerError>);

impl_stream_for_wrapper!(ParserRunnerErrors, ParserRunnerError);

impl MultiErrorDisplay<ParserRunnerError> for ParserRunnerErrors {}

impl ParserRunnerErrors {
    pub fn convert(input: &Input, errs: ParseErrors) -> Self {
        Self(
            errs.into_iter()
                .map(|err| ParserRunnerError::new(*err.0, input.clone()))
                .collect(),
        )
    }
}

pub type ParserRunnerResult<T> = Result<T, ParserRunnerError>;

pub type LexerRunnerError = ParserRunnerError;
pub type LexerRunnerErrors = ParserRunnerErrors;
pub type LexerRunnerResult<T> = Result<T, LexerRunnerError>;
