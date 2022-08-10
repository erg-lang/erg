//! defines `ParseError` and others.
//!
//! パーサーが出すエラーを定義
use erg_common::{impl_stream_for_wrapper, switch_lang};
use erg_common::Str;
use erg_common::config::Input;
use erg_common::error::{ErrorCore, ErrorDisplay, MultiErrorDisplay, Location, ErrorKind::*};
use erg_common::traits::Stream;

#[derive(Debug)]
pub struct LexError(ErrorCore);

#[derive(Debug)]
pub struct LexErrors(Vec<LexError>);

impl_stream_for_wrapper!(LexErrors, LexError);

impl LexError {
    pub const fn new(core: ErrorCore) -> Self { Self(core) }

    pub fn compiler_bug(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::new(errno, CompilerSystemError, loc, switch_lang!(
            format!("this is a bug of the Erg compiler, please report it to https://github.com/mtshiba/erg\ncaused from: {fn_name}:{line}"),
            format!("これはErg compilerのバグです、開発者に報告して下さい (https://github.com/mtshiba/erg)\n{fn_name}:{line}より発生")
        ), None))
    }

    pub fn feature_error(errno: usize, loc: Location, name: &str) -> Self {
        Self::new(ErrorCore::new(errno, FeatureError, loc, switch_lang!(
            format!("this feature({name}) is not implemented yet"),
            format!("この機能({name})はまだ正式に提供されていません")
        ), None))
    }

    pub fn simple_syntax_error(errno: usize, loc: Location) -> Self {
        Self::new(ErrorCore::new(errno, SyntaxError, loc, switch_lang!("invalid syntax", "不正な構文です"), None))
    }

    pub fn syntax_error<S: Into<Str>>(errno: usize, loc: Location, desc: S, hint: Option<Str>) -> Self {
        Self::new(ErrorCore::new(errno, SyntaxError, loc, desc, hint))
    }

    pub fn syntax_warning<S: Into<Str>>(errno: usize,loc: Location, desc: S, hint: Option<Str>) -> Self {
        Self::new(ErrorCore::new(errno, SyntaxWarning, loc, desc, hint))
    }
}

pub type LexResult<T> = Result<T, LexError>;

pub type ParseError = LexError;
pub type ParseErrors = LexErrors;
pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug)]
pub struct DesugaringError {
    pub core: ErrorCore,
}

impl DesugaringError {
    pub const fn new(core: ErrorCore) -> Self { Self{ core }}
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

impl ErrorDisplay for ParserRunnerError {
    fn core(&self) -> &ErrorCore { &self.core }
    fn input(&self) -> &Input { &self.input }
    fn caused_by(&self) -> &str { "" }
    fn ref_inner(&self) -> Option<&Box<Self>> { None }
}

impl ParserRunnerError {
    pub const fn new(core: ErrorCore, input: Input) -> Self { Self{ core, input } }
}

#[derive(Debug)]
pub struct ParserRunnerErrors(Vec<ParserRunnerError>);

impl_stream_for_wrapper!(ParserRunnerErrors, ParserRunnerError);

impl MultiErrorDisplay<ParserRunnerError> for ParserRunnerErrors {}

impl ParserRunnerErrors {
    pub fn convert(input: &Input, errs: ParseErrors) -> Self {
        Self(errs.into_iter().map(|err| ParserRunnerError::new(err.0, input.clone())).collect())
    }
}

pub type ParserRunnerResult<T> = Result<T, ParserRunnerError>;

pub type LexerRunnerError = ParserRunnerError;
pub type LexerRunnerErrors = ParserRunnerErrors;
pub type LexerRunnerResult<T> = Result<T, LexerRunnerError>;
