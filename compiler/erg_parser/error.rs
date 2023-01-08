//! defines `ParseError` and others.
//!
//! パーサーが出すエラーを定義
use std::fmt;

use erg_common::config::Input;
use erg_common::error::{
    ErrorCore, ErrorDisplay, ErrorKind::*, Location, MultiErrorDisplay, SubMessage,
};
use erg_common::style::{Attribute, Color, StyledStr, StyledString, StyledStrings, THEME};
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

const ERR: Color = THEME.colors.error;
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
        err.set_hint("None is got"); // this is not displayed
        err
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

    pub fn failed_to_convert_error(
        errno: usize,
        loc: Location,
        from: &str,
        to: &str,
    ) -> ParseError {
        Self::syntax_error(
            errno,
            loc,
            switch_lang!(
                "japanese" => format!("{}から{}に変換するのに失敗しました", from, to),
                "simplified_chinese" => format!("无法将{}转换为{}", from, to),
                "traditional_chinese" => format!("無法將{}轉換為{}", from, to),
                "english" => format!("failed to convert {} to {}",from, to),
            ),
            None,
        )
    }

    pub fn unexpected_token_error(errno: usize, loc: Location, found: &str) -> ParseError {
        let mut fnd = StyledStrings::default();
        switch_lang!(
            "japanese" =>fnd.push_str("予期しないトークン: "),
            "simplified_chinese" => fnd.push_str("意想不到token: "),
            "traditional_chinese" => fnd.push_str("意想不到token: "),
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
            SyntaxError,
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

    pub fn syntax_sub_massage_error(errno: usize, loc: Location, sub_msg: SubMessage) -> Self {
        Self::new(ErrorCore::new(
            vec![sub_msg],
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

    pub fn invalid_arg_decl_error(errno: usize, loc: Location) -> LexError {
        let msg = switch_lang!(
            "japanese" => "連続する要素の宣言が異なります",
            "simplified_chinese" => "序列元素的声明是无效的",
            "traditional_chinese" => "序列元素的声明是无效的",
            "english" => "the declaration of sequential elements are invalid",
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

    pub fn invalid_token_error(
        errno: usize,
        loc: Location,
        main_msg: &str,
        expect: &str,
        found: &str,
    ) -> LexError {
        let expect = StyledStr::new(expect, Some(HINT), Some(ATTR));
        let expect = switch_lang!(
                "japanese" => format!("expect: {}", expect),
                "simplified_chinese" => format!("期望: {}", expect),
                "traditional_chinese" => format!("期望: {}", expect),
                "english" => format!("expect: {}", expect),
        );
        let found = StyledStr::new(found, Some(HINT), Some(ATTR));
        let found = switch_lang!(
                "japanese" => format!("与えれたトークン: {}", found),
                "simplified_chinese" => format!("预期的token: {}", found),
                "traditional_chinese" => format!("預期的token: {}", found),
                "english" => format!("but found token: {}", found),
        );
        Self::new(ErrorCore::new(
            vec![SubMessage::ambiguous_new(loc, vec![expect, found], None)],
            main_msg,
            errno,
            SyntaxError,
            loc,
        ))
    }
    pub fn invalid_seq_elems_error(errno: usize, loc: Location) -> LexError {
        let msg = switch_lang!(
            "japanese" => "連続する要素の宣言が異なります",
            "simplified_chinese" => "无效的顺序元素声明",
            "traditional_chinese" => "無效的順序元素聲明",
            "english" => "invalid sequential elements declaration",
        );
        let hint = switch_lang!(
            "japanese" => "要素か括弧を追加してください",
            "simplified_chinese" => "删除逗号并添加右括号",
            "traditional_chinese" => "刪除逗號並新增右括號",
            "english" => "remove comma and right bracket should be added",
        )
        .to_string();
        Self::syntax_error(errno, loc, msg, Some(hint))
    }

    pub fn invalid_record_element_err(errno: usize, loc: Location) -> LexError {
        let msg = switch_lang!(
            "japanese" => "レコード型の要素が宣言が異なります",
            "simplified_chinese" => "不同的记录类型元素声明",
            "traditional_chinese" => "不同的記錄類型元素聲明",
            "english" => "different record type element declarations",
        )
        .to_string();
        let hint = switch_lang!(
            "japanese" => {
                let record = StyledStr::new("レコード型", Some(HINT), Some(ATTR));
                let var = StyledStr::new("属性", Some(HINT), Some(ATTR));
                let def = StyledStr::new("属性=リテラル", Some(HINT), Some(ATTR));
                format!("{}では{}か{}のみ使うことができます", record, var, def)
            },
            "simplified_chinese" => {
                let record = StyledStr::new("记录类型", Some(HINT), Some(ATTR));
                let var = StyledStr::new("attr", Some(HINT), Some(ATTR));
                let def = StyledStr::new("attr=lit", Some(HINT), Some(ATTR));
                format!("只有{}或{}可以在{}中使用",  var, def, record)
            },
            "traditional_chinese" => {
                let record = StyledStr::new("記錄類型", Some(HINT), Some(ATTR));
                let var = StyledStr::new("attr", Some(HINT), Some(ATTR));
                let def = StyledStr::new("attr=lit", Some(HINT), Some(ATTR));
                format!("只有{}或{}可以在{}中使用",  var, def, record)
            },
            "english" => {
                let record = StyledStr::new("Record", Some(HINT), Some(ATTR));
                let var = StyledStr::new("attr", Some(HINT), Some(ATTR));
                let def = StyledStr::new("attr=lit", Some(HINT), Some(ATTR));
                format!("only {} or {} can ve used in {}",  var, def, record)
            },
        );
        let sub_msg = SubMessage::ambiguous_new(loc, vec![msg], Some(hint));
        let msg = switch_lang!(
            "japanese" => "連続する要素の宣言が異なります",
            "simplified_chinese" => "无效的顺序元素声明",
            "traditional_chinese" => "無效的順序元素聲明",
            "english" => "invalid sequential elements declaration",
        );
        Self::new(ErrorCore::new(vec![sub_msg], msg, errno, SyntaxError, loc))
    }

    pub fn invalid_syntax_after_at_sign(errno: usize, loc: Location) -> Self {
        Self::new(ErrorCore::new(
            vec![SubMessage::only_loc(loc)],
            switch_lang!(
                "japanese" => "@サインの後ろに不正な構文があります",
                "simplified_chinese" => "无效的语法",
                "traditional_chinese" => "無效的語法",
                "english" => "invalid syntax after @sign",
            ),
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn invalid_class_definition(errno: usize, loc: Location) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "無効なクラスの定義です",
            "simplified_chinese" => "类定义无效",
            "traditional_chinese" => "类定义无效",
            "english" => "invalid Class definition",
        );
        let sub = SubMessage::only_loc(loc);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn invalid_data_class_container(errno: usize, loc: Location, fnd: &str) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "データクラスの中身が異なります",
            "simplified_chinese" => "数据类的内容不同",
            "traditional_chinese" => "數據類的內容不同",
            "english" => "contents of data class are different",
        );
        let expt = StyledStr::new("Record Type", Some(HINT), Some(ATTR));
        let expect = switch_lang!(
            "japanese" => format!("予期した型: {}", expt),
            "simplified_chinese" => format!("期望的类型: {}", expt),
            "traditional_chinese" => format!("期望的類型: {}", expt),
            "english" => format!("expect type: {}", expt),
        );
        let fnd = StyledStr::new(fnd, Some(ERR), Some(ATTR));
        let found = switch_lang!(
            "japanese" => format!("与えられた型: {}", fnd),
            "simplified_chinese" => format!("但找到: {}", fnd),
            "traditional_chinese" => format!("但找到: {}", fnd),
            "english" => format!("but found: {}", fnd),
        );
        let sub = SubMessage::ambiguous_new(loc, vec![expect, found], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn invalid_keyword_error(errno: usize, loc: Location, main_msg: &str) -> ParseError {
        let sub_msg = SubMessage::only_loc(loc);
        Self::new(ErrorCore::new(
            vec![sub_msg],
            main_msg,
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn invalid_accessor_token(errno: usize, loc: Location) -> ParseError {
        let sub = SubMessage::only_loc(loc);
        let msg = switch_lang!(
            "japanese" => "",
            "english" => "",
        );
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn failed_to_found_dict_value(errno: usize, loc: Location) -> ParseError {
        let colon = StyledStr::new(":", Some(HINT), Some(ATTR));
        let hint = switch_lang!(
            "japanese" => format!("{}を追加してください", colon),
            "simplified_chinese" => format!("{}应该被添加", colon),
            "traditional_chinese" => format!("{}应该被添加", colon),
            "english" => format!("{} should be added", colon),
        );
        let sub_msg = SubMessage::ambiguous_new(loc, vec![], Some(hint));
        let main_msg = switch_lang!(
            "japanese" => "辞書型の値の宣言が異なります",
            "simplified_chinese" => "字典类型的值声明是不同的",
            "traditional_chinese" => "字典类型的值声明是不同的",
            "english" => "failed to declare Dict type",
        );
        Self::new(ErrorCore::new(
            vec![sub_msg],
            main_msg,
            errno,
            SyntaxError,
            loc,
        ))
    }

    pub fn unclosed_error(errno: usize, closer: &str, loc: Location) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "右の括弧が足りません",
            "simplified_chinese" => "應該被添加",
            "traditional_chinese" => "缺少右括號",
            "english" => "The right bracket is missing",
        );

        let closer = StyledStr::new(closer, Some(HINT), Some(ATTR));
        let sub_msg = switch_lang!(
            "japanese" => format!("{}を追加してください", closer),
            "simplified_chinese" => format!("{}应该被添加", closer),
            "traditional_chinese" => format!("{}應該被添加", closer),
            "english" => format!("{} should be added", closer),
        );

        let sub = SubMessage::ambiguous_new(loc, vec![sub_msg], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn expect_method_error(errno: usize, loc: Location) -> ParseError {
        let mut expect = StyledStrings::default();
        switch_lang!(
            "japanese" => expect.push_str("期待: "),
            "simplified_chinese" => expect.push_str("期望: "),
            "traditional_chinese" => expect.push_str("期望: "),
            "english" => expect.push_str("expect: "),
        );
        expect.push_str_with_color_and_attribute(
            switch_lang!(
                "japanese" => "メソッド",
                "english" => "method",
            ),
            HINT,
            ATTR,
        );
        let sub_msg = SubMessage::ambiguous_new(loc, vec![expect.to_string()], None);
        Self::syntax_sub_massage_error(errno, loc, sub_msg)
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

    pub fn expect_keyword(errno: usize, loc: Location) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "無効な構文です",
            "simplified_chinese" => "无效的语法",
            "traditional_chinese" => "無效的語法",
            "english" => "invalid syntax",
        );
        let sub_msg = switch_lang!(
            "japanese" => "キーワードが期待されています",
            "simplified_chinese" => "期望关键字",
            "traditional_chinese" => "期望關鍵字",
            "english" => "expect keyword",
        )
        .to_string();
        let sub = SubMessage::ambiguous_new(loc, vec![sub_msg], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, SyntaxError, loc))
    }

    pub fn expect_default_parameter(errno: usize, loc: Location) -> ParseError {
        let msg = switch_lang!(
            "japanese" => "無効な構文です",
            "simplified_chinese" => "无效的语法",
            "traditional_chinese" => "無效的語法",
            "english" => "invalid syntax",
        );
        let sub_msg = switch_lang!(
            "japanese" => "デフォルト引数が期待されています",
            "simplified_chinese" => "期望默认参数",
            "traditional_chinese" => "期望默認參數",
            "english" => "expect default parameter",
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
                format!("予期したトークン: {}、{}、{}", method, lit, newline)
            },
            "simplified_chinese" => {
                let method = StyledStr::new("方法", Some(HINT), Some(ATTR));
                let lit = StyledStr::new("NatLit", Some(HINT), Some(ATTR));
                let newline = StyledStr::new("newline", Some(HINT), Some(ATTR));
                format!("expect: {}, {}, {}", method, lit, newline)
            },
            "traditional_chinese" => {
                let method = StyledStr::new("方法", Some(HINT), Some(ATTR));
                let lit = StyledStr::new("NatLit", Some(HINT), Some(ATTR));
                let newline = StyledStr::new("newline", Some(HINT), Some(ATTR));
                format!("expect: {}, {}, {}", method, lit, newline)
            },
            "english" => {
                let method = StyledStr::new("method", Some(HINT), Some(ATTR));
                let lit = StyledStr::new("NatLit", Some(HINT), Some(ATTR));
                let newline = StyledStr::new("newline", Some(HINT), Some(ATTR));
                format!("expect: {}, {}, {}", method, lit, newline)
            },
        );

        let fnd = switch_lang!(
            "japanese" =>format!("与えられたトークン: {}", StyledStr::new(found, Some(ERR), None)),
            "simplified_chinese" => format!("期望存取器: {}", StyledStr::new(found, Some(ERR), None)),
            "traditional_chinese" => format!("期望存取器: {}", StyledStr::new(found, Some(ERR), None)),
            "english" => format!("but found token: {}", StyledStr::new(found, Some(ERR), None)),
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

    pub fn no_indention(errno: usize, loc: Location) -> ParseError {
        let sub_msg = switch_lang!(
            "japanese" => "インデントを追加してください",
            "simplified_chinese" => "缩进应该被添加",
            "traditional_chinese" => "缩縮進應該被添加",
            "english" => "indent should be added",
        )
        .to_string();
        let msg = switch_lang!(
            "japanese" => "不正な構文です",
            "simplified_chinese" => "无效的语法",
            "traditional_chinese" => "無效的語法",
            "english" => "invalid syntax",
        );
        let sub = SubMessage::ambiguous_new(loc, vec![sub_msg], None);
        Self::new(ErrorCore::new(vec![sub], msg, errno, IndentationError, loc))
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

impl std::error::Error for ParserRunnerErrors {}

impl_stream_for_wrapper!(ParserRunnerErrors, ParserRunnerError);

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
pub type LexerRunnerResult<T> = Result<T, LexerRunnerError>;
