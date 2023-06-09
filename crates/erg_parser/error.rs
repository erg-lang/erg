//! defines `ParseError` and others.
//!
//! パーサーが出すエラーを定義
use std::fmt;

use erg_common::error::{
    ErrorCore, ErrorDisplay, ErrorKind::*, Location, MultiErrorDisplay, SubMessage,
};
use erg_common::io::Input;
use erg_common::style::{Attribute, Color, StyledStr, StyledString, StyledStrings, THEME};
use erg_common::traits::Stream;
use erg_common::{fmt_iter, impl_display_and_error, impl_stream, switch_lang};

use crate::ast::Module;
use crate::token::TokenKind;

#[derive(Debug)]
pub struct LexError(Box<ErrorCore>); // ErrorCore is large, so use Box

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LexError({})", self.0)
    }
}

impl std::error::Error for LexError {}

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

impl_stream!(LexErrors, LexError);

impl fmt::Display for LexErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LexErrors({})", fmt_iter(self.0.iter()))
    }
}

impl std::error::Error for LexErrors {}

const ERR: Color = THEME.colors.error;
const WARN: Color = THEME.colors.warning;
const HINT: Color = THEME.colors.hint;
const ACCENT: Color = THEME.colors.accent;

impl LexError {
    pub fn new(core: ErrorCore) -> Self {
        Self(Box::new(core))
    }

    pub fn set_hint<S: Into<String>>(&mut self, hint: S) {
        if let Some(sub_msg) = self.0.sub_messages.get_mut(0) {
            sub_msg.set_hint(hint)
        }
    }

    pub fn compiler_bug(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        const URL: StyledStr = StyledStr::new(
            "https://github.com/erg-lang/erg",
            Some(ACCENT),
            Some(Attribute::Underline),
        );
        Self::new(ErrorCore::new(
            vec![SubMessage::only_loc(loc)],
            switch_lang!(
                "japanese" => format!("これはErg compilerのバグです、開発者に報告して下さい ({URL})\n{fn_name}:{line}より発生"),
                "simplified_chinese" => format!("这是Erg编译器的一个错误，请报告给{URL}\n原因来自: {fn_name}:{line}"),
                "traditional_chinese" => format!("這是Erg編譯器的一個錯誤，請報告給{URL}\n原因來自: {fn_name}:{line}"),
                "english" => format!("this is a bug of the Erg compiler, please report it to {URL}\ncaused from: {fn_name}:{line}"),
            ),
            errno,
            CompilerSystemError,
            loc,
        ))
    }

    pub fn feature_error(errno: usize, loc: Location, name: &str) -> Self {
        Self::new(ErrorCore::new(
            vec![SubMessage::only_loc(loc)],
            switch_lang!(
                "japanese" => format!("この機能({name})はまだ正式に提供されていません"),
                "simplified_chinese" => format!("此功能（{name}）尚未实现"),
                "traditional_chinese" => format!("此功能（{name}）尚未實現"),
                "english" => format!("this feature({name}) is not implemented yet"),
            ),
            errno,
            FeatureError,
            loc,
        ))
    }

    pub fn simple_syntax_error(errno: usize, loc: Location) -> Self {
        Self::new(ErrorCore::new(
            vec![SubMessage::only_loc(loc)],
            switch_lang!(
                "japanese" => "不正な構文です",
                "simplified_chinese" => "无效的语法",
                "traditional_chinese" => "無效的語法",
                "english" => "invalid syntax",
            ),
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn syntax_error<S: Into<String>>(
        errno: usize,
        loc: Location,
        desc: S,
        hint: Option<String>,
    ) -> Self {
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(loc, vec![], hint)],
            desc,
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn unexpected_token<S: fmt::Display>(
        errno: usize,
        loc: Location,
        expected: S,
        got: TokenKind,
    ) -> Self {
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(loc, vec![], None)],
            switch_lang!(
                "japanese" => format!("{expected}が期待されましたが、{got}となっています"),
                "simplified_chinese" => format!("期待: {expected}，得到: {got}"),
                "traditional_chinese" => format!("期待: {expected}，得到: {got}"),
                "english" => format!("expected: {expected}, got: {got}"),
            ),
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn syntax_warning<S: Into<String>>(
        errno: usize,
        loc: Location,
        desc: S,
        hint: Option<String>,
    ) -> Self {
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(loc, vec![], hint)],
            desc,
            errno,
            SyntaxWarning,
            loc,
        ))
    }

    pub fn no_var_error(
        errno: usize,
        loc: Location,
        name: &str,
        similar_name: Option<String>,
    ) -> Self {
        let hint = similar_name.map(|n| {
            let n = StyledString::new(n, Some(HINT), Some(Attribute::Bold));
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
        });
        let name = StyledString::new(name, Some(ERR), Some(Attribute::Underline));
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(loc, vec![], hint)],
            switch_lang!(
                "japanese" => format!("{name}という変数は定義されていません"),
                "simplified_chinese" => format!("{name}未定义"),
                "traditional_chinese" => format!("{name}未定義"),
                "english" => format!("{name} is not defined"),
            ),
            errno,
            NameError,
            loc,
        ))
    }

    pub fn invalid_chunk_error(errno: usize, loc: Location) -> LexError {
        let msg = switch_lang!(
            "japanese" => "無効な構文です",
            "simplified_chinese" => "无效的语法",
            "traditional_chinese" => "無效的語法",
            "english" => "invalid syntax",
        );
        let hint = switch_lang!(
            "japanese" => "`;`を追加するか改行をしてください",
            "simplified_chinese" => "`;`或应添加换行符",
            "traditional_chinese" => "`;`或應添加換行",
            "english" => "`;` or newline should be added",
        )
        .to_string();
        Self::syntax_error(errno, loc, msg, Some(hint))
    }

    pub fn invalid_arg_decl_error(errno: usize, loc: Location) -> LexError {
        let msg = switch_lang!(
            "japanese" => "連続する要素の宣言が異なります",
            "simplified_chinese" => "应该添加`;`或换行符",
            "traditional_chinese" => "應該添加`;`或換行符",
            "english" => "declaration of sequential elements is invalid",
        );
        let hint = switch_lang!(
            "japanese" => "`,`を追加するか改行をしてください",
            "simplified_chinese" => "应该添加`,`或换行符",
            "traditional_chinese" => "應該添加`,`或換行符",
            "english" => "`,` or newline should be added",
        )
        .to_string();
        Self::syntax_error(errno, loc, msg, Some(hint))
    }

    pub fn invalid_definition_of_last_block(errno: usize, loc: Location) -> LexError {
        Self::syntax_error(
            errno,
            loc,
            switch_lang!(
                "japanese" => "ブロックの終端で変数を定義することは出来ません",
                "simplified_chinese" => "无法在块的末尾定义变量",
                "traditional_chinese" => "無法在塊的末尾定義變量",
                "english" => "cannot define a variable at the end of a block",
            ),
            None,
        )
    }

    pub fn failed_to_analyze_block(errno: usize, loc: Location) -> LexError {
        Self::syntax_error(
            errno,
            loc,
            switch_lang!(
                "japanese" => "ブロックの解析に失敗しました",
                "simplified_chinese" => "无法解析块",
                "traditional_chinese" => "無法解析塊",
                "english" => "failed to parse a block",
            ),
            None,
        )
    }

    pub fn invalid_mutable_symbol(errno: usize, lit: &str, loc: Location) -> LexError {
        let mut expect = StyledStrings::default();
        let expect = switch_lang!(
                "japanese" => {
                    expect.push_str("期待された構文: ");
                    expect.push_str_with_color(&format!("!{lit}"), HINT);
                    expect
                },
                "simplified_chinese" => {
                    expect.push_str("预期语法: ");
                    expect.push_str_with_color(&format!("!{lit}"), HINT);
                    expect
                },
                "traditional_chinese" => {
                    expect.push_str("預期語法: ");
                    expect.push_str_with_color(&format!("!{lit}"), HINT);
                    expect
                },
                "english" => {
                    expect.push_str("expected: ");
                    expect.push_str_with_color(&format!("!{lit}"), HINT);
                    expect
                },
        )
        .to_string();
        let mut found = StyledStrings::default();
        let found = switch_lang!(
                "japanese" => {
                    found.push_str("見つかった構文: ");
                    found.push_str_with_color(&format!("{lit}!"), ERR);
                    found
                },
                "simplified_chinese" => {
                    found.push_str("找到语法: ");
                    found.push_str_with_color(&format!("{lit}!"), ERR);
                    found
                },
                "traditional_chinese" => {
                    found.push_str("找到語法: ");
                    found.push_str_with_color(&format!("{lit}!"), ERR);
                    found
                },
                "english" => {
                    found.push_str("but found: ");
                    found.push_str_with_color(&format!("{lit}!"), ERR);
                    found
                },
        )
        .to_string();
        let main_msg = switch_lang!(
            "japanese" => "無効な可変シンボルです",
            "simplified_chinese" => "无效的可变符号",
            "traditional_chinese" => "無效的可變符號",
            "english" => "invalid mutable symbol",
        );
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(loc, vec![expect, found], None)],
            main_msg,
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn duplicate_elem_warning(errno: usize, loc: Location, elem: String) -> Self {
        let elem = StyledString::new(elem, Some(WARN), Some(Attribute::Underline));
        Self::new(ErrorCore::new(
            vec![SubMessage::only_loc(loc)],
            switch_lang!(
                "japanese" => format!("重複する要素です: {elem}"),
                "simplified_chinese" => format!("{elem}"),
                "traditional_chinese" => format!("{elem}"),
                "english" => format!("duplicated element: {elem}"),
            ),
            errno,
            SyntaxWarning,
            loc,
        ))
    }
}

pub type LexResult<T> = Result<T, LexError>;

pub type ParseError = LexError;
pub type ParseErrors = LexErrors;
pub type ParseWarning = LexError;
pub type ParseWarnings = LexErrors;
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

impl_stream!(DesugaringErrors, DesugaringError);

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

impl std::error::Error for ParserRunnerErrors {}

impl_stream!(ParserRunnerErrors, ParserRunnerError);

impl MultiErrorDisplay<ParserRunnerError> for ParserRunnerErrors {}

impl fmt::Display for ParserRunnerErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_all(f)
    }
}

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
pub type ParserRunnerWarning = ParserRunnerError;
pub type ParserRunnerWarnings = ParserRunnerErrors;
pub type LexerRunnerResult<T> = Result<T, LexerRunnerError>;

#[derive(Debug)]
pub struct CompleteArtifact<A = Module, Es = ParseErrors> {
    pub ast: A,
    pub warns: Es,
}

impl<A, Es> CompleteArtifact<A, Es> {
    pub fn new(ast: A, warns: Es) -> Self {
        Self { ast, warns }
    }
}

#[derive(Debug)]
pub struct IncompleteArtifact<A = Module, Es = ParseErrors> {
    pub ast: Option<A>,
    pub warns: Es,
    pub errors: Es,
}

impl<A> From<ParserRunnerErrors> for IncompleteArtifact<A, ParserRunnerErrors> {
    fn from(value: ParserRunnerErrors) -> IncompleteArtifact<A, ParserRunnerErrors> {
        IncompleteArtifact::new(None, ParserRunnerErrors::empty(), value)
    }
}

impl<A> From<LexErrors> for IncompleteArtifact<A, ParseErrors> {
    fn from(value: LexErrors) -> IncompleteArtifact<A, ParseErrors> {
        IncompleteArtifact::new(None, ParseErrors::empty(), value)
    }
}

impl<A, Es> IncompleteArtifact<A, Es> {
    pub fn new(ast: Option<A>, warns: Es, errors: Es) -> Self {
        Self { ast, warns, errors }
    }

    pub fn map_errs<U>(self, f: impl Fn(Es) -> U) -> IncompleteArtifact<A, U> {
        IncompleteArtifact {
            ast: self.ast,
            warns: f(self.warns),
            errors: f(self.errors),
        }
    }

    pub fn map_mod<U>(self, f: impl Fn(A) -> U) -> IncompleteArtifact<U, Es> {
        IncompleteArtifact {
            ast: self.ast.map(f),
            warns: self.warns,
            errors: self.errors,
        }
    }
}

#[derive(Debug)]
pub struct ErrorArtifact<Es = ParseErrors> {
    pub warns: Es,
    pub errors: Es,
}

impl<Es> ErrorArtifact<Es> {
    pub fn new(warns: Es, errors: Es) -> ErrorArtifact<Es> {
        Self { warns, errors }
    }
}
