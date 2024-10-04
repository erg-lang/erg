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
use erg_common::{fmt_iter, fmt_vec_split_with, impl_display_and_error, impl_stream, switch_lang};

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

#[cfg(feature = "pylib")]
impl std::convert::From<LexError> for pyo3::PyErr {
    fn from(err: LexError) -> pyo3::PyErr {
        pyo3::exceptions::PyOSError::new_err(err.to_string())
    }
}

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

impl LexError {
    pub fn loc(&self) -> Location {
        self.0.loc
    }
}

#[cfg_attr(feature = "pylib", pyo3::pyclass)]
#[derive(Debug)]
pub struct LexErrors(Vec<LexError>);

impl_stream!(LexErrors, LexError);

impl fmt::Display for LexErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LexErrors({})", fmt_iter(self.0.iter()))
    }
}

impl std::error::Error for LexErrors {}

#[cfg(feature = "pylib")]
impl std::convert::From<LexErrors> for pyo3::PyErr {
    fn from(errs: LexErrors) -> pyo3::PyErr {
        pyo3::exceptions::PyOSError::new_err(errs[0].to_string())
    }
}

const ERR: Color = THEME.colors.error;
const WARN: Color = THEME.colors.warning;
const HINT: Color = THEME.colors.hint;
#[cfg(not(feature = "pretty"))]
const ATTR: Attribute = Attribute::Bold;
#[cfg(feature = "pretty")]
const ATTR: Attribute = Attribute::Underline;

impl LexError {
    pub fn new(core: ErrorCore) -> Self {
        Self(Box::new(core))
    }

    pub fn set_hint<S: Into<String>>(&mut self, hint: S) {
        if let Some(sub_msg) = self.0.sub_messages.get_mut(0) {
            sub_msg.set_hint(hint)
        }
    }

    /* Parser Bug */
    pub fn compiler_bug(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        let mut err = Self::new(ErrorCore::bug(errno, loc, fn_name, line));
        err.set_hint("parser bug");
        err
    }

    pub fn feature_error(errno: usize, loc: Location, name: &str) -> Self {
        let main_msg = switch_lang!(
            "japanese" => format!("この機能({name})はまだ正式に提供されていません"),
            "simplified_chinese" => format!("此功能（{name}）尚未实现"),
            "traditional_chinese" => format!("此功能（{name}）尚未實現"),
            "english" => format!("this feature({name}) is not implemented yet"),
        );
        let main_msg = StyledStr::new(&main_msg, Some(ERR), Some(ATTR)).to_string();
        Self::new(ErrorCore::new(
            vec![SubMessage::only_loc(loc)],
            main_msg,
            errno,
            FeatureError,
            loc,
        ))
    }

    pub fn invalid_none_match(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        let mut err = Self::new(ErrorCore::bug(errno, loc, fn_name, line));
        err.set_hint("None is got");
        err
    }

    pub fn failed_to_analyze_block(errno: usize, loc: Location) -> LexError {
        Self::new(ErrorCore::new(
            vec![],
            switch_lang!(
                "japanese" => "ブロックの解析に失敗しました",
                "simplified_chinese" => "无法解析块",
                "traditional_chinese" => "無法解析塊",
                "english" => "failed to parse a block",
            ),
            errno,
            CompilerSystemError,
            loc,
        ))
    }

    pub fn unexpected_token_error(errno: usize, loc: Location, found: &str) -> ParseError {
        let mut fnd = StyledStrings::default();
        switch_lang!(
            "japanese" =>fnd.push_str("予期しないトークンです: "),
            "simplified_chinese" => fnd.push_str("意外的token: "),
            "traditional_chinese" => fnd.push_str("意外的token: "),
            "english" => fnd.push_str("unexpected token: "),
        );
        fnd.push_str_with_color(found, ERR);
        let main_msg = switch_lang!(
            "japanese" => "無効な構文です",
            "simplified_chinese" => "无效的语法",
            "traditional_chinese" => "無效的語法",
            "english" => "invalid syntax",
        );
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(loc, vec![fnd.to_string()], None)],
            main_msg,
            errno,
            CompilerSystemError,
            loc,
        ))
    }

    /* Parser Errors */
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

    pub fn expect_next_line_error(errno: usize, loc: Location, caused: &str) -> Self {
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(
                loc,
                vec![caused.to_string()],
                None,
            )],
            switch_lang!(
                "japanese" => "ブロックが期待されていますが、EOFが与えられました",
                "simplified_chinese" => "该块是预期的，但是是 EOF",
                "traditional_chinese" => "該塊是預期的，但是是 EOF",
                "english" => "The block is expected, but is EOF",
            ),
            errno,
            ExpectNextLine,
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
                "simplified_chinese" => format!("期望: {expected}，得到: {got}"),
                "traditional_chinese" => format!("期望: {expected}，得到: {got}"),
                "english" => format!("expected: {expected}, got: {got}"),
            ),
            errno,
            SyntaxError,
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
            "japanese" => "セミコロンを追加するか改行をしてください",
            "simplified_chinese" => "应该添加分号或换行符",
            "traditional_chinese" => "應該添加分號或換行符",
            "english" => "semicolon or newline should be added",
        )
        .to_string();
        Self::syntax_error(errno, loc, msg, Some(hint))
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

    pub fn invalid_token_error(
        errno: usize,
        loc: Location,
        main_msg: &str,
        expect: &str,
        found: &str,
    ) -> LexError {
        let expect = StyledStr::new(expect, Some(HINT), Some(ATTR));
        let expect = switch_lang!(
                "japanese" => format!("予期したトークン: {expect}"),
                "simplified_chinese" => format!("期望: {expect}"),
                "traditional_chinese" => format!("期望: {expect}"),
                "english" => format!("expect: {expect}"),
        );
        let found = StyledStr::new(found, Some(ERR), Some(ATTR));
        let found = switch_lang!(
                "japanese" => format!("与えられた: {found}"),
                "simplified_chinese" => format!("但找到: {found}"),
                "traditional_chinese" => format!("但找到: {found}"),
                "english" => format!("but found: {found}"),
        );
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(loc, vec![expect, found], None)],
            main_msg,
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn invalid_seq_elems_error<S: fmt::Display>(
        errno: usize,
        loc: Location,
        expected: &[S],
        got: TokenKind,
    ) -> LexError {
        let hint = switch_lang!(
            "japanese" => format!("{}が期待されましたが、{got}が与えれられました", fmt_vec_split_with(expected, " または ")),
            "simplified_chinese" => format!("期望: {}，但找到: {got}", fmt_vec_split_with(expected, " 或 ")),
            "traditional_chinese" => format!("期望: {}，但找到: {got}", fmt_vec_split_with(expected, " 或 ")),
            "english" => format!("expect: {}, got: {got}", fmt_vec_split_with(expected, " or ")),
        );
        let msg = switch_lang!(
            "japanese" => "要素の列挙の仕方が不正です",
            "simplified_chinese" => "无效的Sequential元素声明",
            "traditional_chinese" => "無效的Sequential元素聲明",
            "english" => "invalid sequential elements declaration",
        );
        Self::syntax_error(errno, loc, msg, Some(hint))
    }

    pub fn invalid_record_element_err(errno: usize, loc: Location) -> LexError {
        let msg = switch_lang!(
            "japanese" => "レコード型要素の列挙の仕方が不正です",
            "simplified_chinese" => "无效的Record类型元素声明",
            "traditional_chinese" => "無效的Record類型元素聲明",
            "english" => "invalid record type element declarations",
        )
        .to_string();
        let hint = switch_lang!(
            "japanese" => {
                let record = StyledStr::new("レコード型", Some(HINT), Some(ATTR));
                let var = StyledStr::new("属性", Some(HINT), Some(ATTR));
                let def = StyledStr::new("属性=リテラル", Some(HINT), Some(ATTR));
                let obj = StyledStr::new("属性=オブジェクト", Some(HINT), Some(ATTR));
                format!("{record}では{var}か{def}、{obj}のみ宣言することができます")
            },
            "simplified_chinese" => {
                let record = StyledStr::new("Record类型", Some(HINT), Some(ATTR));
                let var = StyledStr::new("属性", Some(HINT), Some(ATTR));
                let def = StyledStr::new("属性=lit", Some(HINT), Some(ATTR));
                let obj = StyledStr::new("属性=object", Some(HINT), Some(ATTR));
                format!("{record}中只能声明 {var}、{def} 或 {obj}")
            },
            "traditional_chinese" => {
                let record = StyledStr::new("Record類型", Some(HINT), Some(ATTR));
                let var = StyledStr::new("屬性", Some(HINT), Some(ATTR));
                let def = StyledStr::new("屬性=lit", Some(HINT), Some(ATTR));
                let obj = StyledStr::new("屬性=object", Some(HINT), Some(ATTR));
                format!("{record}中只能声明 {var}、{def} 或 {obj}")
            },
            "english" => {
                let record = StyledStr::new("Record", Some(HINT), Some(ATTR));
                let var = StyledStr::new("attr", Some(HINT), Some(ATTR));
                let def = StyledStr::new("attr=lit", Some(HINT), Some(ATTR));
                let obj = StyledStr::new("attr=object", Some(HINT), Some(ATTR));
                format!("only {var}, {def} or {obj} can be declared in {record}")
            },
        );
        Self::syntax_error(errno, loc, msg, Some(hint))
    }

    pub fn invalid_data_pack_definition(errno: usize, loc: Location, fnd: &str) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "データクラスの中身が異なります",
            "simplified_chinese" => "数据类的内容无效",
            "traditional_chinese" => "數據類的內容無效",
            "english" => "invalid contents of data class",
        );
        let expt = StyledStr::new("Record Type", Some(HINT), Some(ATTR));
        let expect = switch_lang!(
            "japanese" => format!("予期した型: {expt}"),
            "simplified_chinese" => format!("期望的类型: {expt}"),
            "traditional_chinese" => format!("期望的類型: {expt}"),
            "english" => format!("expect type: {expt}"),
        );
        let fnd = StyledStr::new(fnd, Some(ERR), Some(ATTR));
        let found = switch_lang!(
            "japanese" => format!("与えられた型: {fnd}"),
            "simplified_chinese" => format!("但找到: {fnd}"),
            "traditional_chinese" => format!("但找到: {fnd}"),
            "english" => format!("but found: {fnd}"),
        );
        let sub = SubMessage::ambiguous_new(loc, vec![expect, found], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn expect_keyword(errno: usize, loc: Location) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "キーワードが指定されていません",
            "simplified_chinese" => "未指定关键字",
            "traditional_chinese" => "未指定關鍵字",
            "english" => "keyword is not specified",
        );
        let keyword = StyledStr::new("keyword", Some(HINT), Some(ATTR));
        let sub_msg = switch_lang!(
            "japanese" => format!("予期した: {keyword}"),
            "simplified_chinese" => format!("期望: {keyword}"),
            "traditional_chinese" => format!("期望: {keyword}"),
            "english" => format!("expect: {keyword}"),
        );
        let sub = SubMessage::ambiguous_new(loc, vec![sub_msg], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn invalid_non_default_parameter(errno: usize, loc: Location) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "非デフォルト引数はデフォルト引数の後に指定できません",
            "simplified_chinese" => "默认实参后面跟着非默认实参",
            "traditional_chinese" => "默認實參後面跟著非默認實參",
            "english" => "non-default argument follows default argument",
        );

        let walrus = StyledStr::new(":=", Some(HINT), Some(ATTR));
        let sub_msg = switch_lang!(
            "japanese" => format!("{walrus}を使用してください"),
            "simplified_chinese" => format!("应该使用{walrus}"),
            "traditional_chinese" => format!("應該使用{walrus}"),
            "english" => format!("{walrus} should be used"),
        );
        let sub = SubMessage::ambiguous_new(loc, vec![sub_msg], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn invalid_colon_style(errno: usize, loc: Location) -> ParseError {
        let desc = switch_lang!(
            "japanese" => "コロンの後ろでカンマを使った区切りは使うことができません",
            "simplified_chinese" => "冒号后不能使用逗号分隔符",
            "traditional_chinese" => "冒號後不能使用逗號分隔符",
            "english" => "comma delimiters cannot be used after a colon",
        );
        let hint = switch_lang!(
            "japanese" => "カンマを削除してください",
            "simplified_chinese" => "应该去掉逗号",
            "traditional_chinese" => "應該去掉逗號",
            "english" => "comma should be removed",
        )
        .to_string();
        Self::syntax_error(errno, loc, desc, Some(hint))
    }

    pub fn unclosed_error(errno: usize, loc: Location, closer: &str, ty: &str) -> ParseError {
        let msg = switch_lang!(
            "japanese" => format!("{ty}が{closer}で閉じられていません"),
            "simplified_chinese" => format!("{ty}没有用{closer}关闭"),
            "traditional_chinese" => format!("{ty}没有用{closer}關閉"),
            "english" => format!("{ty} is not closed with a {closer}"),
        );

        let closer = StyledStr::new(closer, Some(HINT), Some(ATTR));
        let sub_msg = switch_lang!(
            "japanese" => format!("{closer}を追加してください"),
            "simplified_chinese" => format!("{closer}应该被添加"),
            "traditional_chinese" => format!("{closer}應該被添加"),
            "english" => format!("{closer} should be added"),
        );

        let sub = SubMessage::ambiguous_new(loc, vec![sub_msg], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn expect_method_error(errno: usize, loc: Location) -> ParseError {
        let mut expect = StyledStrings::default();
        switch_lang!(
            "japanese" => expect.push_str("予期した: "),
            "simplified_chinese" => expect.push_str("期望: "),
            "traditional_chinese" => expect.push_str("期望: "),
            "english" => expect.push_str("expect: "),
        );
        expect.push_str_with_color_and_attr(
            switch_lang!(
                "japanese" => "メソッド",
                "simplified_chinese" => "方法",
                "traditional_chinese" => "方法",
                "english" => "method",
            ),
            HINT,
            ATTR,
        );
        let sub_msg = SubMessage::ambiguous_new(loc, vec![expect.to_string()], None);
        let main_msg = switch_lang!(
            "japanese" => "クラスメソッドの定義が必要です",
            "simplified_chinese" => "需要类方法定义",
            "traditional_chinese" => "需要類方法定義",
            "english" => "class method definitions are needed",
        );
        Self::new(ErrorCore::new(
            vec![sub_msg],
            main_msg,
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn expect_accessor(errno: usize, loc: Location) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "無効な構文です",
            "simplified_chinese" => "无效的语法",
            "traditional_chinese" => "無效的語法",
            "english" => "invalid syntax",
        );
        let sub_msg = switch_lang!(
            "japanese" => "アクセッサ―が期待されています",
            "simplified_chinese" => "期望存取器",
            "traditional_chinese" => "期望存取器",
            "english" => "expect accessor",
        )
        .to_string();
        let sub = SubMessage::ambiguous_new(loc, vec![sub_msg], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn invalid_acc_chain(errno: usize, loc: Location, found: &str) -> ParseError {
        let expt = switch_lang!(
            "japanese" => {
                let method = StyledStr::new("メソッド", Some(HINT), Some(ATTR));
                let lit = StyledStr::new("NatLit", Some(HINT), Some(ATTR));
                let newline = StyledStr::new("改行", Some(HINT), Some(ATTR));
                let arr = StyledStr::new("リスト", Some(HINT), Some(ATTR));
                format!("予期: {method}、{lit}、{newline}、{arr}")
            },
            "simplified_chinese" => {
                let method = StyledStr::new("方法", Some(HINT), Some(ATTR));
                let lit = StyledStr::new("NatLit", Some(HINT), Some(ATTR));
                let newline = StyledStr::new("换行", Some(HINT), Some(ATTR));
                let arr = StyledStr::new("数组", Some(HINT), Some(ATTR));
                format!("期望: {method}, {lit}, {newline}, {arr}")
            },
            "traditional_chinese" => {
                let method = StyledStr::new("方法", Some(HINT), Some(ATTR));
                let lit = StyledStr::new("NatLit", Some(HINT), Some(ATTR));
                let newline = StyledStr::new("换行", Some(HINT), Some(ATTR));
                let arr = StyledStr::new("數組", Some(HINT), Some(ATTR));
                format!("期望: {method}, {lit}, {newline}, {arr}")
            },
            "english" => {
                let method = StyledStr::new("method", Some(HINT), Some(ATTR));
                let lit = StyledStr::new("NatLit", Some(HINT), Some(ATTR));
                let newline = StyledStr::new("newline", Some(HINT), Some(ATTR));
                let arr = StyledStr::new("list", Some(HINT), Some(ATTR));
                format!("expect: {method}, {lit}, {newline}, {arr}")
            },
        );

        let fnd = switch_lang!(
            "japanese" =>format!("与えられた: {}", StyledStr::new(found, Some(ERR), None)),
            "simplified_chinese" => format!("但找到: {}", StyledStr::new(found, Some(ERR), None)),
            "traditional_chinese" => format!("但找到: {}", StyledStr::new(found, Some(ERR), None)),
            "english" => format!("but found: {}", StyledStr::new(found, Some(ERR), None)),
        );
        let sub = SubMessage::ambiguous_new(loc, vec![expt, fnd], None);
        let msg = switch_lang!(
            "japanese" => "無効なアクセス呼び出しです",
            "simplified_chinese" => "无效的访问调用",
            "traditional_chinese" => "無效的訪問調用",
            "english" => "invalid access call",
        )
        .to_string();
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn expect_type_specified(errno: usize, loc: Location) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "型指定が不正です",
            "traditional_chinese" => "无效的类型说明",
            "simplified_chinese" => "無效的類型說明",
            "english" => "invalid type specification",
        );
        Self::syntax_error(errno, loc, msg, None)
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

impl From<ParserRunnerError> for LexError {
    fn from(err: ParserRunnerError) -> Self {
        Self::new(err.core)
    }
}

#[derive(Debug)]
pub struct ParserRunnerErrors(Vec<ParserRunnerError>);

impl std::error::Error for ParserRunnerErrors {}

impl_stream!(ParserRunnerErrors, ParserRunnerError);

impl MultiErrorDisplay<ParserRunnerError> for ParserRunnerErrors {}

impl From<ParserRunnerErrors> for LexErrors {
    fn from(errs: ParserRunnerErrors) -> Self {
        Self(errs.0.into_iter().map(LexError::from).collect())
    }
}

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

    pub fn map<U>(self, f: impl FnOnce(A) -> U) -> CompleteArtifact<U, Es> {
        CompleteArtifact {
            ast: f(self.ast),
            warns: self.warns,
        }
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

    pub fn map_mod<U>(self, f: impl FnOnce(A) -> U) -> IncompleteArtifact<U, Es> {
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
