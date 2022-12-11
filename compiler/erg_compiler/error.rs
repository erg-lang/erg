use std::fmt;
use std::fmt::Display;

use erg_common::config::Input;
use erg_common::error::{
    ErrorCore, ErrorDisplay, ErrorKind::*, Location, MultiErrorDisplay, SubMessage,
};
use erg_common::set::Set;
use erg_common::style::{Attribute, Color, StyledStr, StyledString, StyledStrings, Theme, THEME};
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::{
    fmt_iter, fmt_option_map, fmt_vec, impl_display_and_error, impl_stream_for_wrapper,
    switch_lang, Str,
};

use erg_parser::error::{ParserRunnerError, ParserRunnerErrors};

use crate::context::Context;
use crate::hir::{Expr, Identifier, Signature};
use crate::ty::{HasType, Predicate, Type};

pub fn ordinal_num(n: usize) -> String {
    match n.to_string().chars().last().unwrap() {
        '1' => format!("{n}st"),
        '2' => format!("{n}nd"),
        '3' => format!("{n}rd"),
        _ => format!("{n}th"),
    }
}

/// dname is for "double under name"
pub fn binop_to_dname(op: &str) -> &str {
    match op {
        "+" => "__add__",
        "-" => "__sub__",
        "*" => "__mul__",
        "/" => "__div__",
        "//" => "__floordiv__",
        "**" => "__pow__",
        "%" => "__mod__",
        ".." => "__rng__",
        "<.." => "__lorng__",
        "..<" => "__rorng__",
        "<..<" => "__orng__",
        "&&" | "and" => "__and__",
        "||" | "or" => "__or__",
        "^^" => "__xor__",
        "in" => "__in__",
        "contains" => "__contains__",
        "subof" => "__subof__",
        "supof" => "__supof__",
        "is" => "__is__",
        "isnot" => "__isnot__",
        "==" => "__eq__",
        "!=" => "__ne__",
        "<" => "__lt__",
        "<=" => "__le__",
        ">" => "__gt__",
        ">=" => "__ge__",
        other => todo!("no such binary operator: {other}"),
    }
}

pub fn unaryop_to_dname(op: &str) -> &str {
    match op {
        "+" => "__pos__",
        "-" => "__neg__",
        "~" => "__invert__",
        "!" => "__mutate__",
        "..." => "__spread__",
        other => todo!("no such unary operator: {other}"),
    }
}

pub fn readable_name(name: &str) -> &str {
    match name {
        "__add__" => "`+`",
        "__sub__" => "`-`",
        "__mul__" => "`*`",
        "__div__" => "`/`",
        "__floordiv__" => "`//`",
        "__pow__" => "`**`",
        "__mod__" => "`%`",
        "__rng__" => "`..`",
        "__lorng__" => "`<..`",
        "__rorng__" => "`..<`",
        "__orng__" => "`<..<`",
        "__and__" => "`and`",
        "__or__" => "`or`",
        "__in__" => "`in`",
        "__contains__" => "`contains`",
        "__subof__" => "`subof`",
        "__supof__" => "`supof`",
        "__is__" => "`is`",
        "__isnot__" => "`isnot`",
        "__eq__" => "`==`",
        "__ne__" => "`!=`",
        "__lt__" => "`<`",
        "__le__" => "`<=`",
        "__gt__" => "`>`",
        "__ge__" => "`>=`",
        "__pos__" => "`+`",
        "__neg__" => "`-`",
        "__invert__" => "`~`",
        "__mutate__" => "`!`",
        "__spread__" => "`...`",
        other => other,
    }
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub core: Box<ErrorCore>, // ErrorCore is large, so box it
    pub input: Input,
    pub caused_by: String,
    pub theme: Theme,
}

impl_display_and_error!(CompileError);

impl From<ParserRunnerError> for CompileError {
    fn from(err: ParserRunnerError) -> Self {
        Self {
            core: Box::new(err.core),
            input: err.input,
            caused_by: "".to_owned(),
            theme: THEME,
        }
    }
}

impl ErrorDisplay for CompileError {
    fn core(&self) -> &ErrorCore {
        &self.core
    }
    fn input(&self) -> &Input {
        &self.input
    }
    fn caused_by(&self) -> &str {
        &self.caused_by
    }
    fn ref_inner(&self) -> Option<&Self> {
        None
    }
}

// found, error, rhs
const ERR: Color = THEME.colors.error;
// var name, lhs
const WARN: Color = THEME.colors.warning;
// expect, hint
const HINT: Color = THEME.colors.hint;
// url, accentuation
const ACCENT: Color = THEME.colors.accent;
// url and feature = pretty
const UNDERLINE: Attribute = Attribute::Underline;
#[cfg(not(feature = "pretty"))]
const ATTR: Attribute = Attribute::Bold;
#[cfg(feature = "pretty")]
const ATTR: Attribute = Attribute::Underline;

const URL: StyledStr = StyledStr::new(
    "https://github.com/erg-lang/erg",
    Some(ACCENT),
    Some(UNDERLINE),
);

impl CompileError {
    pub fn new(core: ErrorCore, input: Input, caused_by: String) -> Self {
        Self {
            core: Box::new(core),
            input,
            caused_by,
            theme: THEME,
        }
    }

    pub fn compiler_bug(
        errno: usize,
        input: Input,
        loc: Location,
        fn_name: &str,
        line: u32,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("これはErg compilerのバグです、開発者に報告して下さい ({URL})\n\n{fn_name}:{line}より発生"),
                    "simplified_chinese" => format!("这是Erg编译器的错误，请报告给{URL}\n\n原因来自: {fn_name}:{line}"),
                    "traditional_chinese" => format!("這是Erg編譯器的錯誤，請報告給{URL}\n\n原因來自: {fn_name}:{line}"),
                    "english" => format!("this is a bug of the Erg compiler, please report it to {URL}\n\ncaused from: {fn_name}:{line}"),
                ),
                errno,
                CompilerSystemError,
                loc,
            ),
            input,
            "".to_owned(),
        )
    }

    pub fn stack_bug(
        input: Input,
        loc: Location,
        stack_len: u32,
        block_id: usize,
        fn_name: &str,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("\
スタックの要素数が異常です (要素数: {stack_len}, ブロックID: {block_id})
これはコンパイラのバグです、開発者に報告して下さい ({URL})
{fn_name}より発生"),
                "simplified_chinese" => format!("\
堆栈中的元素数无效（元素数: {stack_len}，块id: {block_id}）
这是 Erg 编译器的一个错误，请报告它 ({URL})
起因于: {fn_name}"),
                "traditional_chinese" => format!("\
堆棧中的元素數無效（元素數: {stack_len}，塊id: {block_id}）\n
這是 Erg 編譯器的一個錯誤，請報告它 ({URL})
起因於: {fn_name}"),
                    "english" => format!("\
the number of elements in the stack is invalid (num of elems: {stack_len}, block id: {block_id})\n
this is a bug of the Erg compiler, please report it to {URL}
caused from: {fn_name}"),
                ),
                0,
                CompilerSystemError,
                loc,
            ),
            input,
            "".to_owned(),
        )
    }

    pub fn feature_error(input: Input, loc: Location, name: &str, caused_by: String) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("この機能({name})はまだ正式に提供されていません"),
                    "simplified_chinese" => format!("此功能（{name}）尚未实现"),
                    "traditional_chinese" => format!("此功能（{name}）尚未實現"),
                    "english" => format!("this feature({name}) is not implemented yet"),
                ),
                0,
                FeatureError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn system_exit() -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(Location::Unknown)],
                switch_lang!(
                    "japanese" => "システムを終了します",
                    "simplified_chinese" => "系统正在退出",
                    "traditional_chinese" => "系統正在退出",
                    "english" => "system is exiting",
                ),
                0,
                SystemExit,
                Location::Unknown,
            ),
            Input::Dummy,
            "".to_owned(),
        )
    }
}

pub type TyCheckError = CompileError;

impl TyCheckError {
    pub fn dummy(input: Input, errno: usize) -> Self {
        Self::new(ErrorCore::dummy(errno), input, "".to_string())
    }

    pub fn unreachable(input: Input, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), input, "".to_string())
    }

    pub fn checker_bug(
        input: Input,
        errno: usize,
        loc: Location,
        fn_name: &str,
        line: u32,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("これはErg compilerのバグです、開発者に報告して下さい ({URL})\n\n{fn_name}:{line}より発生"),
                    "simplified_chinese" => format!("这是Erg编译器的错误，请报告给{URL}\n\n原因来自: {fn_name}:{line}"),
                    "traditional_chinese" => format!("這是Erg編譯器的錯誤，請報告給{URL}\n\n原因來自: {fn_name}:{line}"),
                    "english" => format!("this is a bug of the Erg compiler, please report it to {URL}\n\ncaused from: {fn_name}:{line}"),
                ),
                errno,
                CompilerSystemError,
                loc,
            ),
            input,
            "".to_string(),
        )
    }

    pub fn no_type_spec_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}の型が指定されていません"),
                    "simplified_chinese" => format!("{name}的类型未指定"),
                    "traditional_chinese" => format!("{name}的類型未指定"),
                    "english" => format!("the type of {name} is not specified"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn callable_impl_error<'a, C: Locational + Display>(
        input: Input,
        errno: usize,
        callee: &C,
        param_ts: impl Iterator<Item = &'a Type>,
        caused_by: String,
    ) -> Self {
        let param_ts = fmt_iter(param_ts);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(callee.loc())],
                switch_lang!(
                    "japanese" => format!(
                        "{callee}は{param_ts}を引数に取る呼び出し可能オブジェクトではありません"
                    ),
                    "simplified_chinese" => format!("{callee}不是以{param_ts}作为参数的可调用对象"),
                    "traditional_chinese" => format!("{callee}不是以{param_ts}作為參數的可調用對象"),
                    "english" => format!(
                        "{callee} is not a Callable object that takes {param_ts} as an argument"
                    ),
                ),
                errno,
                NotImplementedError,
                callee.loc(),
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn type_mismatch_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        nth_param: Option<usize>,
        expect: &Type,
        found: &Type,
        candidates: Option<Set<Type>>,
        hint: Option<String>,
    ) -> Self {
        let ord = match nth_param {
            Some(pos) => switch_lang!(
                "japanese" => format!("({pos}番目の引数)"),
                "simplified_chinese" => format!("(第{pos}个参数)"),
                "traditional_chinese" => format!("(第{pos}個參數)"),
                "english" => format!(" (the {} argument)", ordinal_num(pos)),
            ),
            None => "".to_owned(),
        };
        let name = StyledString::new(format!("{}{}", name, ord), Some(WARN), Some(ATTR));
        let mut expct = StyledStrings::default();
        switch_lang!(
            "japanese" => expct.push_str("予期した型: "),
            "simplified_chinese" =>expct.push_str("预期: "),
            "traditional_chinese" => expct.push_str("預期: "),
            "english" => expct.push_str("expected: "),
        );
        expct.push_str_with_color_and_attribute(format!("{}", expect), HINT, ATTR);

        let mut fnd = StyledStrings::default();
        switch_lang!(
            "japanese" => fnd.push_str("与えられた型: "),
            "simplified_chinese" => fnd.push_str("但找到: "),
            "traditional_chinese" => fnd.push_str("但找到: "),
            "english" =>fnd.push_str("but found: "),
        );
        fnd.push_str_with_color_and_attribute(format!("{}", found), ERR, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    loc,
                    vec![expct.to_string(), fnd.to_string()],
                    hint,
                )],
                switch_lang!(
                    "japanese" => format!("{name}の型が違います{}", fmt_option_map!(pre "\n与えられた型の単一化候補: ", candidates, |x: &Set<Type>| x.folded_display())),
                    "simplified_chinese" => format!("{name}的类型不匹配{}", fmt_option_map!(pre "\n某一类型的统一候选: ", candidates, |x: &Set<Type>| x.folded_display())),
                    "traditional_chinese" => format!("{name}的類型不匹配{}", fmt_option_map!(pre "\n某一類型的統一候選: ", candidates, |x: &Set<Type>| x.folded_display())),
                    "english" => format!("the type of {name} is mismatched{}", fmt_option_map!(pre "\nunification candidates of a given type: ", candidates, |x: &Set<Type>| x.folded_display())),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn return_type_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        expect: &Type,
        found: &Type,
    ) -> Self {
        let mut expct = StyledStrings::default();
        switch_lang!(
            "japanese" => expct.push_str("予期した型: "),
            "simplified_chinese" =>expct.push_str("预期: "),
            "traditional_chinese" => expct.push_str("預期: "),
            "english" => expct.push_str("expected: "),
        );
        expct.push_str_with_color_and_attribute(format!("{}", expect), HINT, ATTR);

        let mut fnd = StyledStrings::default();
        switch_lang!(
            "japanese" => fnd.push_str("与えられた型: "),
            "simplified_chinese" => fnd.push_str("但找到: "),
            "traditional_chinese" => fnd.push_str("但找到: "),
            "english" =>fnd.push_str("but found: "),
        );
        fnd.push_str_with_color_and_attribute(format!("{}", found), ERR, ATTR);

        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    loc,
                    vec![expct.to_string(), fnd.to_string()],
                    None,
                )],
                switch_lang!(
                    "japanese" => format!("{name}の戻り値の型が違います"),
                    "simplified_chinese" => format!("{name}的返回类型不匹配"),
                    "traditional_chinese" => format!("{name}的返回類型不匹配"),
                    "english" => format!("the return type of {name} is mismatched"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn uninitialized_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        t: &Type,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}: {t}は宣言されましたが初期化されていません"),
                    "simplified_chinese" => format!("{name}: {t}已声明但未初始化"),
                    "traditional_chinese" => format!("{name}: {t}已宣告但未初始化"),
                    "english" => format!("{name}: {t} is declared but not initialized"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn argument_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        expect: usize,
        found: usize,
    ) -> Self {
        let mut expct = StyledStrings::default();
        switch_lang!(
            "japanese" => expct.push_str("予期した個数: "),
            "simplified_chinese" =>expct.push_str("预期: "),
            "traditional_chinese" => expct.push_str("預期: "),
            "english" => expct.push_str("expected: "),
        );
        expct.push_str_with_color_and_attribute(format!("{}", expect), HINT, ATTR);

        let mut fnd = StyledStrings::default();
        switch_lang!(
            "japanese" => fnd.push_str("与えられた個数: "),
            "simplified_chinese" => fnd.push_str("但找到: "),
            "traditional_chinese" => fnd.push_str("但找到: "),
            "english" =>fnd.push_str("but found: "),
        );
        fnd.push_str_with_color_and_attribute(format!("{}", found), ERR, ATTR);

        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    loc,
                    vec![expct.to_string(), fnd.to_string()],
                    None,
                )],
                switch_lang!(
                    "japanese" => format!("ポジショナル引数の数が違います"),
                    "simplified_chinese" => format!("正则参数的数量不匹配"),
                    "traditional_chinese" => format!("正則參數的數量不匹配"),
                    "english" => format!("the number of positional arguments is mismatched"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn param_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        expect: usize,
        found: usize,
    ) -> Self {
        let mut expct = StyledStrings::default();
        switch_lang!(
            "japanese" => expct.push_str("予期した個数: "),
            "simplified_chinese" =>expct.push_str("预期: "),
            "traditional_chinese" => expct.push_str("預期: "),
            "english" => expct.push_str("expected: "),
        );
        expct.push_str_with_color_and_attribute(format!("{}", expect), HINT, ATTR);

        let mut fnd = StyledStrings::default();
        switch_lang!(
            "japanese" => fnd.push_str("与えられた個数: "),
            "simplified_chinese" => fnd.push_str("但找到: "),
            "traditional_chinese" => fnd.push_str("但找到: "),
            "english" =>fnd.push_str("but found: "),
        );
        fnd.push_str_with_color_and_attribute(format!("{}", found), ERR, ATTR);

        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    loc,
                    vec![expct.to_string(), fnd.to_string()],
                    None,
                )],
                switch_lang!(
                    "japanese" => format!("引数の数が違います"),
                    "simplified_chinese" => format!("参数的数量不匹配"),
                    "traditional_chinese" => format!("參數的數量不匹配"),
                    "english" => format!("the number of parameters is mismatched"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn default_param_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![],
                switch_lang!(
                    "japanese" => format!("{name}はデフォルト引数を受け取りません"),
                    "simplified_chinese" => format!("{name}不接受默认参数"),
                    "traditional_chinese" => format!("{name}不接受預設參數"),
                    "english" => format!("{name} does not accept default parameters"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn match_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        expr_t: &Type,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{expr_t}型の全パターンを網羅していません"),
                    "simplified_chinese" => format!("并非所有{expr_t}类型的模式都被涵盖"),
                    "traditional_chinese" => format!("並非所有{expr_t}類型的模式都被涵蓋"),
                    "english" => format!("not all patterns of type {expr_t} are covered"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn infer_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        expr: &str,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{expr}の型が推論できません"),
                    "simplified_chinese" => format!("无法推断{expr}的类型"),
                    "traditional_chinese" => format!("無法推斷{expr}的類型"),
                    "english" => format!("failed to infer the type of {expr}"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn dummy_infer_error(input: Input, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), input, "".to_owned())
    }

    pub fn not_relation(input: Input, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), input, "".to_owned())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn too_many_args_error(
        input: Input,
        errno: usize,
        loc: Location,
        callee_name: &str,
        caused_by: String,
        params_len: usize,
        pos_args_len: usize,
        kw_args_len: usize,
    ) -> Self {
        let name = readable_name(callee_name);
        let expect = StyledString::new(format!("{}", params_len), Some(HINT), Some(ATTR));
        let pos_args_len = StyledString::new(format!("{}", pos_args_len), Some(ERR), Some(ATTR));
        let kw_args_len = StyledString::new(format!("{}", kw_args_len), Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!(
                        "{name}に渡された引数の数が多すぎます

必要な引数の合計数: {expect}個
渡された引数の数:   {pos_args_len}個
キーワード引数の数: {kw_args_len}個"
                    ),
                    "simplified_chinese" => format!("传递给{name}的参数过多

总的预期参数: {expect}
通过的位置参数: {pos_args_len}
通过了关键字参数: {kw_args_len}"
                    ),
                    "traditional_chinese" => format!("傳遞給{name}的參數過多

所需參數總數: {expect}
遞的參數數量: {pos_args_len}
字參數的數量: {kw_args_len}"
                    ),
                    "english" => format!(
                        "too many arguments for {name}

total expected params:  {expect}
passed positional args: {pos_args_len}
passed keyword args:    {kw_args_len}"
                    ),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn args_missing_error(
        input: Input,
        errno: usize,
        loc: Location,
        callee_name: &str,
        caused_by: String,
        missing_params: Vec<Str>,
    ) -> Self {
        let name = StyledStr::new(readable_name(callee_name), Some(WARN), Some(ATTR));
        let vec_cxt = StyledString::new(&fmt_vec(&missing_params), Some(WARN), Some(ATTR));
        let missing_len = missing_params.len();
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}に渡された引数が{missing_len}個足りません\n不足している引数: {vec_cxt}" ),
                    "simplified_chinese" => format!("{name}的{missing_len}个位置参数不被传递\n缺少的参数: {vec_cxt}" ),
                    "traditional_chinese" => format!("{name}的{missing_len}個位置參數不被傳遞\n缺少的參數: {vec_cxt}" ),
                    "english" => format!("missing {missing_len} positional argument(s) for {name}\nmissing: {vec_cxt}"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn multiple_args_error(
        input: Input,
        errno: usize,
        loc: Location,
        callee_name: &str,
        caused_by: String,
        arg_name: &str,
    ) -> Self {
        let name = StyledStr::new(readable_name(callee_name), Some(WARN), Some(ATTR));
        let found = StyledString::new(arg_name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}の引数{found}が複数回渡されています"),
                    "simplified_chinese" => format!("{name}的参数{found}被多次传递"),
                    "traditional_chinese" => format!("{name}的參數{found}被多次傳遞"),
                    "english" => format!("{name}'s argument {found} is passed multiple times"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn unexpected_kw_arg_error(
        input: Input,
        errno: usize,
        loc: Location,
        callee_name: &str,
        caused_by: String,
        param_name: &str,
    ) -> Self {
        let name = StyledStr::new(readable_name(callee_name), Some(WARN), Some(ATTR));
        let found = StyledString::new(param_name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}には予期しないキーワード引数{found}が渡されています"),
                    "simplified_chinese" => format!("{name}得到了意外的关键字参数{found}"),
                    "traditional_chinese" => format!("{name}得到了意外的關鍵字參數{found}"),
                    "english" => format!("{name} got unexpected keyword argument {found}"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn unification_error(
        input: Input,
        errno: usize,
        lhs_t: &Type,
        rhs_t: &Type,
        loc: Location,
        caused_by: String,
    ) -> Self {
        let mut lhs_typ = StyledStrings::default();
        switch_lang!(
            "japanese" => lhs_typ.push_str("左辺: "),
            "simplified_chinese" => lhs_typ.push_str("左边: "),
            "traditional_chinese" => lhs_typ.push_str("左邊: "),
            "english" => lhs_typ.push_str("lhs: "),
        );
        lhs_typ.push_str_with_color_and_attribute(format!("{}", lhs_t), WARN, ATTR);
        let mut rhs_typ = StyledStrings::default();
        switch_lang!(
            "japanese" => rhs_typ.push_str("右辺: "),
            "simplified_chinese" => rhs_typ.push_str("右边: "),
            "traditional_chinese" => rhs_typ.push_str("右邊: "),
            "english" => rhs_typ.push_str("rhs: "),
        );
        rhs_typ.push_str_with_color_and_attribute(format!("{}", rhs_t), WARN, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    loc,
                    vec![lhs_typ.to_string(), rhs_typ.to_string()],
                    None,
                )],
                switch_lang!(
                    "japanese" => format!("型の単一化に失敗しました"),
                    "simplified_chinese" => format!("类型统一失败"),
                    "traditional_chinese" => format!("類型統一失敗"),
                    "english" => format!("unification failed"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn re_unification_error(
        input: Input,
        errno: usize,
        lhs_t: &Type,
        rhs_t: &Type,
        loc: Location,
        caused_by: String,
    ) -> Self {
        let mut lhs_typ = StyledStrings::default();
        switch_lang!(
            "japanese" => lhs_typ.push_str("左辺: "),
            "simplified_chinese" => lhs_typ.push_str("左边: "),
            "traditional_chinese" => lhs_typ.push_str("左邊: "),
            "english" => lhs_typ.push_str("lhs: "),
        );
        lhs_typ.push_str_with_color_and_attribute(format!("{}", lhs_t), WARN, ATTR);
        let mut rhs_typ = StyledStrings::default();
        switch_lang!(
            "japanese" => rhs_typ.push_str("右辺: "),
            "simplified_chinese" => rhs_typ.push_str("右边: "),
            "traditional_chinese" => rhs_typ.push_str("右邊: "),
            "english" => rhs_typ.push_str("rhs: "),
        );
        rhs_typ.push_str_with_color_and_attribute(format!("{}", rhs_t), WARN, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    loc,
                    vec![lhs_typ.to_string(), rhs_typ.to_string()],
                    None,
                )],
                switch_lang!(
                    "japanese" => format!("型の再単一化に失敗しました"),
                    "simplified_chinese" => format!("重新统一类型失败"),
                    "traditional_chinese" => format!("重新統一類型失敗"),
                    "english" => format!("re-unification failed"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn subtyping_error(
        input: Input,
        errno: usize,
        sub_t: &Type,
        sup_t: &Type,
        loc: Location,
        caused_by: String,
    ) -> Self {
        let mut sub_type = StyledStrings::default();
        switch_lang!(
            "japanese" => sub_type.push_str("部分型: "),
            "simplified_chinese" => sub_type.push_str("超类型: "),
            "simplified_chinese" =>sub_type.push_str("超類型: "),
            "english" => sub_type.push_str("subtype: "),
        );
        sub_type.push_str_with_color_and_attribute(format!("{}", sub_t), HINT, ATTR);

        let mut sup_type = StyledStrings::default();
        switch_lang!(
            "japanese" => sup_type.push_str("汎化型: "),
            "simplified_chinese" => sup_type.push_str("超类型: "),
            "simplified_chinese" => sup_type.push_str("超類型: "),
            "english" =>sup_type.push_str("supertype: "),
        );
        sup_type.push_str_with_color_and_attribute(format!("{}", sup_t), ERR, ATTR);

        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    loc,
                    vec![sub_type.to_string(), sup_type.to_string()],
                    None,
                )],
                switch_lang!(
                    "japanese" => format!("この式の部分型制約を満たせません"),
                    "simplified_chinese" => format!("无法满足此表达式中的子类型约束"),
                    "traditional_chinese" => format!("無法滿足此表達式中的子類型約束"),
                    "english" => format!("the subtype constraint in this expression cannot be satisfied"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn pred_unification_error(
        input: Input,
        errno: usize,
        lhs: &Predicate,
        rhs: &Predicate,
        caused_by: String,
    ) -> Self {
        let mut lhs_uni = StyledStrings::default();
        switch_lang!(
            "japanese" => lhs_uni.push_str("左辺: "),
            "simplified_chinese" => lhs_uni.push_str("左边: "),
            "traditional_chinese" => lhs_uni.push_str("左邊: "),
            "english" => lhs_uni.push_str("lhs: "),
        );
        lhs_uni.push_str_with_color_and_attribute(format!("{}", lhs), HINT, ATTR);
        let mut rhs_uni = StyledStrings::default();
        switch_lang!(
            "japanese" => rhs_uni.push_str("右辺: "),
            "simplified_chinese" => rhs_uni.push_str("右边: "),
            "traditional_chinese" => rhs_uni.push_str("右邊: "),
            "english" => rhs_uni.push_str("rhs: "),
        );
        rhs_uni.push_str_with_color_and_attribute(format!("{}", rhs), ERR, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    Location::Unknown,
                    vec![lhs_uni.to_string(), rhs_uni.to_string()],
                    None,
                )],
                switch_lang!(
                    "japanese" => format!("述語式の単一化に失敗しました"),
                    "simplified_chinese" => format!("无法统一谓词表达式"),
                    "traditional_chinese" => format!("無法統一謂詞表達式"),
                    "english" => format!("predicate unification failed"),
                ),
                errno,
                TypeError,
                Location::Unknown,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_candidate_error(
        input: Input,
        errno: usize,
        proj: &Type,
        loc: Location,
        caused_by: String,
        hint: Option<String>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{proj}の候補がありません"),
                    "simplified_chinese" => format!("{proj}没有候选项"),
                    "traditional_chinese" => format!("{proj}沒有候選項"),
                    "english" => format!("no candidate for {proj}"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_trait_impl_error(
        input: Input,
        errno: usize,
        class: &Type,
        trait_: &Type,
        loc: Location,
        caused_by: String,
        hint: Option<String>,
    ) -> Self {
        let hint = hint.or_else(|| Context::get_simple_type_mismatch_hint(trait_, class));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{class}は{trait_}を実装していません"),
                    "simplified_chinese" => format!("{class}没有实现{trait_}"),
                    "traditional_chinese" => format!("{class}沒有實現{trait_}"),
                    "english" => format!("{class} does not implement {trait_}"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn method_definition_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        hint: Option<String>,
    ) -> Self {
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!(
                        "{found}にメソッドを定義することはできません",
                    ),
                    "simplified_chinese" => format!(
                        "{found}不可定义方法",
                    ),
                    "traditional_chinese" => format!(
                        "{found}不可定義方法",
                    ),
                    "english" => format!(
                        "cannot define methods for {found}",
                    ),
                ),
                errno,
                MethodError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trait_member_type_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        member_name: &str,
        trait_type: &Type,
        expect: &Type,
        found: &Type,
        hint: Option<String>,
    ) -> Self {
        let mut expct = StyledStrings::default();
        switch_lang!(
            "japanese" => {
                expct.push_str_with_color_and_attribute(format!("{}", trait_type), ACCENT, ATTR);
                expct.push_str("で宣言された型: ");
            },
            "simplified_chinese" => {
                expct.push_str_with_color_and_attribute(format!("{}", trait_type), ACCENT, ATTR);
                expct.push_str("中声明的类型: ");
            },
            "traditional_chinese" => {
                expct.push_str_with_color_and_attribute(format!("{}", trait_type), ACCENT, ATTR);
                expct.push_str("中聲明的類型: ");
            },
            "english" => {
                expct.push_str("declared in ");
                expct.push_str_with_color(format!("{}: ", trait_type), ACCENT);
            },
        );
        expct.push_str_with_color(format!("{}", expect), HINT);
        let mut fnd = StyledStrings::default();
        switch_lang!(
            "japanese" => fnd.push_str("与えられた型: "),
            "simplified_chinese" => fnd.push_str("但找到: "),
            "traditional_chinese" => fnd.push_str("但找到: "),
            "english" => fnd.push_str("but found: "),
        );
        fnd.push_str_with_color(format!("{}", found), ERR);
        let member_name = StyledStr::new(member_name, Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    loc,
                    vec![expct.to_string(), fnd.to_string()],
                    hint,
                )],
                switch_lang!(
                    "japanese" => format!("{member_name}の型が違います"),
                    "simplified_chinese" => format!("{member_name}的类型不匹配"),
                    "traditional_chinese" => format!("{member_name}的類型不匹配"),
                    "english" => format!("the type of {member_name} is mismatched"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trait_member_not_defined_error(
        input: Input,
        errno: usize,
        caused_by: String,
        member_name: &str,
        trait_type: &Type,
        class_type: &Type,
        hint: Option<String>,
    ) -> Self {
        let member_name = StyledString::new(member_name, Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(Location::Unknown, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{trait_type}の{member_name}が{class_type}で実装されていません"),
                    "simplified_chinese" => format!("{trait_type}中的{member_name}没有在{class_type}中实现"),
                    "traditional_chinese" => format!("{trait_type}中的{member_name}沒有在{class_type}中實現"),
                    "english" => format!("{member_name} of {trait_type} is not implemented in {class_type}"),
                ),
                errno,
                TypeError,
                Location::Unknown,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn not_in_trait_error(
        input: Input,
        errno: usize,
        caused_by: String,
        member_name: &str,
        trait_type: &Type,
        class_type: &Type,
        hint: Option<String>,
    ) -> Self {
        let member_name = StyledString::new(member_name, Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(Location::Unknown, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{class_type}の{member_name}は{trait_type}で宣言されていません"),
                    "simplified_chinese" => format!("{class_type}中的{member_name}没有在{trait_type}中声明"),
                    "traditional_chinese" => format!("{class_type}中的{member_name}沒有在{trait_type}中聲明"),
                    "english" => format!("{member_name} of {class_type} is not declared in {trait_type}"),
                ),
                errno,
                TypeError,
                Location::Unknown,
            ),
            input,
            caused_by,
        )
    }

    pub fn tyvar_not_defined_error(
        input: Input,
        errno: usize,
        name: &str,
        loc: Location,
        caused_by: String,
    ) -> Self {
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("型変数{found}が定義されていません"),
                    "simplified_chinese" => format!("类型变量{found}没有定义"),
                    "traditional_chinese" => format!("類型變量{found}沒有定義"),
                    "english" => format!("type variable {found} is not defined"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn ambiguous_type_error(
        input: Input,
        errno: usize,
        expr: &(impl Locational + Display),
        candidates: &[Type],
        caused_by: String,
    ) -> Self {
        let hint = Some(
            switch_lang!(
             "japanese" => {
                 let mut s = StyledStrings::default();
                 s.push_str("多相関数の場合は");
                 s.push_str_with_color_and_attribute("f|T := Int|", ACCENT, ATTR);
                 s.push_str(", \n型属性の場合は");
                 s.push_str_with_color_and_attribute("f|T := Trait|.X", ACCENT, ATTR);
                 s
             },
             "simplified_chinese" => {
                 let mut s = StyledStrings::default();
                 s.push_str("如果是多态函数，请使用");
                 s.push_str_with_color_and_attribute("f|T := Int|", ACCENT, ATTR);
                 s.push_str("，\n如果是类型属性，请使用");
                 s.push_str_with_color_and_attribute("f|T := Trait|.X", ACCENT, ATTR);
                 s
            },
            "traditional_chinese" => {
                 let mut s = StyledStrings::default();
                 s.push_str("如果是多型函數，請使用");
                 s.push_str_with_color_and_attribute("f|T := Int|", ACCENT, ATTR);
                 s.push_str("，\n如果是類型屬性，請使用");
                 s.push_str_with_color_and_attribute("f|T := Trait|.X", ACCENT, ATTR);
                 s
            },
            "english" => {
                 let mut s = StyledStrings::default();
                 s.push_str("if it is a polymorphic function, like ");
                 s.push_str_with_color_and_attribute("f|T := Int|", ACCENT, ATTR);
                 s.push_str("\nif it is a type attribute, like ");
                 s.push_str_with_color_and_attribute("f|T := Trait|.X ", ACCENT, ATTR);
                 s
            },
                    )
            .to_string(),
        );
        let sub_msg = switch_lang!(
            "japanese" => "型を指定してください",
            "simplified_chinese" => "方式指定类型",
            "traditional_chinese" => "specify the type",
            "english" => "specify the type",
        );
        let mut candidate = StyledStrings::default();
        switch_lang!(
            "japanese" => candidate.push_str("候補: "),
            "simplified_chinese" => candidate.push_str("候选: "),
            "traditional_chinese" => candidate.push_str("候選: "),
            "english" => candidate.push_str("candidates: "),
        );
        candidate.push_str_with_color_and_attribute(&fmt_vec(candidates), WARN, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    expr.loc(),
                    vec![sub_msg.to_string(), candidate.to_string()],
                    hint,
                )],
                switch_lang!(
                    "japanese" => format!("{expr}の型を一意に決定できませんでした"),
                    "simplified_chinese" => format!("无法确定{expr}的类型"),
                    "traditional_chinese" => format!("無法確定{expr}的類型"),
                    "english" => format!("cannot determine the type of {expr}"),
                ),
                errno,
                TypeError,
                expr.loc(),
            ),
            input,
            caused_by,
        )
    }
}

pub type TyCheckErrors = CompileErrors;
pub type SingleTyCheckResult<T> = Result<T, TyCheckError>;
pub type TyCheckResult<T> = Result<T, TyCheckErrors>;
pub type TyCheckWarning = TyCheckError;
pub type TyCheckWarnings = TyCheckErrors;

pub type EvalError = TyCheckError;
pub type EvalErrors = TyCheckErrors;
pub type EvalResult<T> = TyCheckResult<T>;
pub type SingleEvalResult<T> = SingleTyCheckResult<T>;

impl EvalError {
    pub fn not_const_expr(input: Input, errno: usize, loc: Location, caused_by: String) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => "定数式ではありません",
                    "simplified_chinese" => "不是常量表达式",
                    "traditional_chinese" => "不是常量表達式",
                    "english" => "not a constant expression",
                ),
                errno,
                NotConstExpr,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn invalid_literal(input: Input, errno: usize, loc: Location, caused_by: String) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => "リテラルが不正です",
                    "simplified_chinese" => "字面量不合法",
                    "traditional_chinese" => "字面量不合法",
                    "english" => "invalid literal",
                ),
                errno,
                SyntaxError,
                loc,
            ),
            input,
            caused_by,
        )
    }
}

pub type EffectError = TyCheckError;
pub type EffectErrors = TyCheckErrors;

impl EffectError {
    pub fn has_effect(input: Input, errno: usize, expr: &Expr, caused_by: String) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(expr.loc())],
                switch_lang!(
                    "japanese" => "この式には副作用があります",
                    "simplified_chinese" => "此表达式会产生副作用",
                    "traditional_chinese" => "此表達式會產生副作用",
                    "english" => "this expression causes a side-effect",
                ),
                errno,
                HasEffect,
                expr.loc(),
            ),
            input,
            caused_by,
        )
    }

    pub fn proc_assign_error(
        input: Input,
        errno: usize,
        sig: &Signature,
        caused_by: String,
    ) -> Self {
        let hint = Some(
            switch_lang!(
                "japanese" => {
                let mut s = StyledStrings::default();
                s.push_str("変数の末尾に");
                s.push_str_with_color_and_attribute("!", WARN, ATTR);
                s.push_str("をつけてください");
                s
                },
                "simplified_chinese" => {
                let mut s = StyledStrings::default();
                s.push_str("请在变量名后加上");
                s.push_str_with_color_and_attribute("!", WARN, ATTR);
                s
                },
                "traditional_chinese" => {
                let mut s = StyledStrings::default();
                s.push_str("請在變量名後加上");
                s.push_str_with_color_and_attribute("!", WARN, ATTR);
                s
                },
                "english" => {

                let mut s = StyledStrings::default();
                s.push_str("add ");
                s.push_str_with_color_and_attribute("!", WARN, ATTR);
                s.push_str(" to the end of the variable name");
                s
                },
            )
            .to_string(),
        );
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(sig.loc(), vec![], hint)],
                switch_lang!(
                    "japanese" => "プロシージャを通常の変数に代入することはできません",
                    "simplified_chinese" => "不能将过程赋值给普通变量",
                    "traditional_chinese" => "不能將過程賦值給普通變量",
                    "english" => "cannot assign a procedure to a normal variable",
                ),
                errno,
                HasEffect,
                sig.loc(),
            ),
            input,
            caused_by,
        )
    }

    pub fn touch_mut_error(input: Input, errno: usize, expr: &Expr, caused_by: String) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(expr.loc())],
                switch_lang!(
                    "japanese" => "関数中で可変オブジェクトにアクセスすることは出来ません",
                    "simplified_chinese" => "函数中不能访问可变对象",
                    "traditional_chinese" => "函數中不能訪問可變對象",
                    "english" => "cannot access a mutable object in a function",
                ),
                errno,
                HasEffect,
                expr.loc(),
            ),
            input,
            caused_by,
        )
    }
}

pub type OwnershipError = TyCheckError;
pub type OwnershipErrors = TyCheckErrors;

impl OwnershipError {
    pub fn move_error(
        input: Input,
        errno: usize,
        name: &str,
        name_loc: Location,
        moved_loc: Location,
        caused_by: String,
    ) -> Self {
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(name_loc)],
                switch_lang!(
                    "japanese" => format!(
                        "{found}は{}行目ですでに移動されています",
                        moved_loc.ln_begin().unwrap_or(0)
                    ),
                    "simplified_chinese" => format!(
                        "{found}已移至第{}行",
                        moved_loc.ln_begin().unwrap_or(0)
                    ),
                    "traditional_chinese" => format!(
                        "{found}已移至第{}行",
                        moved_loc.ln_begin().unwrap_or(0)
                    ),
                    "english" => format!(
                        "{found} was moved in line {}",
                        moved_loc.ln_begin().unwrap_or(0)
                    ),
                ),
                errno,
                MoveError,
                name_loc,
            ),
            input,
            caused_by,
        )
    }
}

pub type LowerError = TyCheckError;
pub type LowerWarning = LowerError;
pub type LowerErrors = TyCheckErrors;
pub type LowerWarnings = LowerErrors;
pub type LowerResult<T> = TyCheckResult<T>;
pub type SingleLowerResult<T> = SingleTyCheckResult<T>;

impl LowerError {
    pub fn syntax_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        desc: String,
        hint: Option<String>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                desc,
                errno,
                SyntaxError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn unused_expr_warning(input: Input, errno: usize, expr: &Expr, caused_by: String) -> Self {
        let desc = switch_lang!(
            "japanese" => format!("式の評価結果(: {})が使われていません", expr.ref_t()),
            "simplified_chinese" => format!("表达式评估结果(: {})未使用", expr.ref_t()),
            "traditional_chinese" => format!("表達式評估結果(: {})未使用", expr.ref_t()),
            "english" => format!("the evaluation result of the expression (: {}) is not used", expr.ref_t()),
        );
        let discard = StyledString::new("discard", Some(HINT), Some(ATTR));
        let hint = switch_lang!(
            "japanese" => format!("値を使わない場合は、{discard}関数を使用してください"),
            "simplified_chinese" => format!("如果您不想使用该值，请使用{discard}函数"),
            "traditional_chinese" => format!("如果您不想使用該值，請使用{discard}函數"),
            "english" => format!("if you don't use the value, use {discard} function"),
        );
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(expr.loc(), vec![], Some(hint))],
                desc,
                errno,
                UnusedWarning,
                expr.loc(),
            ),
            input,
            caused_by,
        )
    }

    pub fn duplicate_decl_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}は既に宣言されています"),
                    "simplified_chinese" => format!("{name}已声明"),
                    "traditional_chinese" => format!("{name}已聲明"),
                    "english" => format!("{name} is already declared"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn duplicate_definition_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}は既に定義されています"),
                    "simplified_chinese" => format!("{name}已定义"),
                    "traditional_chinese" => format!("{name}已定義"),
                    "english" => format!("{name} is already defined"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn violate_decl_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        spec_t: &Type,
        found_t: &Type,
    ) -> Self {
        let name = StyledString::new(readable_name(name), Some(WARN), None);
        let expect = StyledString::new(format!("{}", spec_t), Some(HINT), Some(ATTR));
        let found = StyledString::new(format!("{}", found_t), Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}は{expect}型として宣言されましたが、{found}型のオブジェクトが代入されています"),
                    "simplified_chinese" => format!("{name}被声明为{expect}，但分配了一个{found}对象"),
                    "traditional_chinese" => format!("{name}被聲明為{expect}，但分配了一個{found}對象"),
                    "english" => format!("{name} was declared as {expect}, but an {found} object is assigned"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_var_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = StyledStr::new(n, Some(HINT), Some(ATTR));
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{found}という変数は定義されていません"),
                    "simplified_chinese" => format!("{found}未定义"),
                    "traditional_chinese" => format!("{found}未定義"),
                    "english" => format!("{found} is not defined"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn access_before_def_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        defined_line: usize,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = StyledStr::new(n, Some(HINT), Some(ATTR));
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("定義({defined_line}行目)より前で{found}を参照することは出来ません"),
                    "simplified_chinese" => format!("在{found}定义({defined_line}行)之前引用是不允许的"),
                    "traditional_chinese" => format!("在{found}定義({defined_line}行)之前引用是不允許的"),
                    "english" => format!("cannot access {found} before its definition (line {defined_line})"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn access_deleted_var_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        del_line: usize,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = StyledStr::new(n, Some(HINT), Some(ATTR));
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("削除された変数{found}を参照することは出来ません({del_line}行目で削除)"),
                    "simplified_chinese" => format!("不能引用已删除的变量{found}({del_line}行)"),
                    "traditional_chinese" => format!("不能引用已刪除的變量{found}({del_line}行)"),
                    "english" => format!("cannot access deleted variable {found} (deleted at line {del_line})"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_type_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = StyledStr::new(n, Some(HINT), Some(ATTR));
            switch_lang!(
                "japanese" => format!("似た名前の型があります: {n}"),
                "simplified_chinese" => format!("存在相同名称类型: {n}"),
                "traditional_chinese" => format!("存在相同名稱類型: {n}"),
                "english" => format!("exists a similar name type: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{found}という型は定義されていません"),
                    "simplified_chinese" => format!("{found}未定义"),
                    "traditional_chinese" => format!("{found}未定義"),
                    "english" => format!("Type {found} is not defined"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn type_not_found(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        typ: &Type,
    ) -> Self {
        let typ = StyledString::new(&typ.to_string(), Some(ERR), Some(ATTR));
        let hint = Some(switch_lang!(
            "japanese" => format!("恐らくこれはErgコンパイラのバグです、{URL}へ報告してください"),
            "simplified_chinese" => format!("这可能是Erg编译器的错误，请报告给{URL}"),
            "traditional_chinese" => format!("這可能是Erg編譯器的錯誤，請報告給{URL}"),
            "english" => format!("This may be a bug of Erg compiler, please report to {URL}"),
        ));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{typ}という型は定義されていません"),
                    "simplified_chinese" => format!("{typ}未定义"),
                    "traditional_chinese" => format!("{typ}未定義"),
                    "english" => format!("Type {typ} is not defined"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_attr_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        obj_t: &Type,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let hint = similar_name.map(|n| {
            switch_lang!(
                "japanese" => format!("似た名前の属性があります: {n}"),
                "simplified_chinese" => format!("具有相同名称的属性: {n}"),
                "traditional_chinese" => format!("具有相同名稱的屬性: {n}"),
                "english" => format!("has a similar name attribute: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{obj_t}型オブジェクトに{found}という属性はありません"),
                    "simplified_chinese" => format!("{obj_t}对象没有属性{found}"),
                    "traditional_chinese" => format!("{obj_t}對像沒有屬性{found}"),
                    "english" => format!("{obj_t} object has no attribute {found}"),
                ),
                errno,
                AttributeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn singular_no_attr_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        obj_name: &str,
        obj_t: &Type,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let hint = similar_name.map(|n| {
            let n = StyledStr::new(n, Some(HINT), Some(ATTR));
            switch_lang!(
                "japanese" => format!("似た名前の属性があります: {n}"),
                "simplified_chinese" => format!("具有相同名称的属性: {n}"),
                "traditional_chinese" => format!("具有相同名稱的屬性: {n}"),
                "english" => format!("has a similar name attribute: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{obj_name}(: {obj_t})に{found}という属性はありません"),
                    "simplified_chinese" => format!("{obj_name}(: {obj_t})没有属性{found}"),
                    "traditional_chinese" => format!("{obj_name}(: {obj_t})沒有屬性{found}"),
                    "english" => format!("{obj_name}(: {obj_t}) has no attribute {found}"),
                ),
                errno,
                AttributeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn reassign_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = StyledStr::new(readable_name(name), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("変数{name}に複数回代入することはできません"),
                    "simplified_chinese" => format!("不能为变量{name}分配多次"),
                    "traditional_chinese" => format!("不能為變量{name}分配多次"),
                    "english" => format!("variable {name} cannot be assigned more than once"),
                ),
                errno,
                AssignError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn unused_warning(
        input: Input,
        errno: usize,
        loc: Location,
        name: &str,
        caused_by: String,
    ) -> Self {
        let name = StyledString::new(readable_name(name), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}は使用されていません"),
                    "simplified_chinese" => format!("{name}未使用"),
                    "traditional_chinese" => format!("{name}未使用"),
                    "english" => format!("{name} is not used"),
                ),
                errno,
                UnusedWarning,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn del_error(input: Input, errno: usize, ident: &Identifier, caused_by: String) -> Self {
        let name = StyledString::new(readable_name(ident.inspect()), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(ident.loc())],
                switch_lang!(
                    "japanese" => format!("{name}は削除できません"),
                    "simplified_chinese" => format!("{name}不能删除"),
                    "traditional_chinese" => format!("{name}不能刪除"),
                    "english" => format!("{name} cannot be deleted"),
                ),
                errno,
                NameError,
                ident.loc(),
            ),
            input,
            caused_by,
        )
    }

    pub fn visibility_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        vis: Visibility,
    ) -> Self {
        let visibility = if vis.is_private() {
            switch_lang!(
                "japanese" => "非公開",
                "simplified_chinese" => "私有",
                "traditional_chinese" => "私有",
                "english" => "private",
            )
        } else {
            switch_lang!(
                "japanese" => "公開",
                "simplified_chinese" => "公有",
                "traditional_chinese" => "公有",
                "english" => "public",
            )
        };
        let found = StyledString::new(readable_name(name), Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{found}は{visibility}変数です"),
                    "simplified_chinese" => format!("{found}是{visibility}变量",),
                    "traditional_chinese" => format!("{found}是{visibility}變量",),
                    "english" => format!("{found} is {visibility} variable",),
                ),
                errno,
                VisibilityError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn override_error<S: Into<String>>(
        input: Input,
        errno: usize,
        name: &str,
        name_loc: Location,
        superclass: &Type,
        caused_by: S,
    ) -> Self {
        let name = StyledString::new(name, Some(ERR), Some(ATTR));
        let superclass = StyledString::new(format!("{}", superclass), Some(WARN), Some(ATTR));
        let hint = Some(
            switch_lang!(
                "japanese" => {
                    let mut ovr = StyledStrings::default();
                    ovr.push_str_with_color_and_attribute("@Override", HINT, ATTR);
                    ovr.push_str("デコレータを使用してください");
                    ovr
            },
                "simplified_chinese" => {
                    let mut ovr = StyledStrings::default();
                    ovr.push_str("请使用");
                    ovr.push_str_with_color_and_attribute("@Override", HINT, ATTR);
                    ovr.push_str("装饰器");
                    ovr
                },
                "traditional_chinese" => {
                    let mut ovr = StyledStrings::default();
                    ovr.push_str("請使用");
                    ovr.push_str_with_color_and_attribute("@Override", HINT, ATTR);
                    ovr.push_str("裝飾器");
                    ovr
                },
                "english" => {
                    let mut ovr = StyledStrings::default();
                    ovr.push_str("use ");
                    ovr.push_str_with_color_and_attribute("@Override", HINT, ATTR);
                    ovr.push_str(" decorator");
                    ovr
                },
            )
            .to_string(),
        );
        let sub_msg = switch_lang!(
            "japanese" => "デフォルトでオーバーライドはできません",
            "simplified_chinese" => "默认不可重写",
            "simplified_chinese" => "默認不可重寫",
            "english" => "cannot override by default",
        );
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    name_loc,
                    vec![sub_msg.to_string()],
                    hint,
                )],
                switch_lang!(
                    "japanese" => format!(
                        "{name}は{superclass}で既に定義されています",
                    ),
                    "simplified_chinese" => format!(
                        "{name}已在{superclass}中定义",
                    ),
                    "traditional_chinese" => format!(
                        "{name}已在{superclass}中定義",
                    ),
                    "english" => format!(
                        "{name} is already defined in {superclass}",
                    ),
                ),
                errno,
                NameError,
                name_loc,
            ),
            input,
            caused_by.into(),
        )
    }

    pub fn inheritance_error(
        input: Input,
        errno: usize,
        class: String,
        loc: Location,
        caused_by: String,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{class}は継承できません"),
                    "simplified_chinese" => format!("{class}不可继承"),
                    "traditional_chinese" => format!("{class}不可繼承"),
                    "english" => format!("{class} is not inheritable"),
                ),
                errno,
                InheritanceError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn file_error(
        input: Input,
        errno: usize,
        desc: String,
        loc: Location,
        caused_by: String,
        hint: Option<String>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                desc,
                errno,
                IoError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn module_env_error(
        input: Input,
        errno: usize,
        mod_name: &str,
        loc: Location,
        caused_by: String,
    ) -> Self {
        let desc = switch_lang!(
            "japanese" => format!("{mod_name}モジュールはお使いの環境をサポートしていません"),
            "simplified_chinese" => format!("{mod_name}模块不支持您的环境"),
            "traditional_chinese" => format!("{mod_name}模塊不支持您的環境"),
            "english" => format!("module {mod_name} is not supported in your environment"),
        );
        Self::file_error(input, errno, desc, loc, caused_by, None)
    }

    pub fn import_error(
        input: Input,
        errno: usize,
        desc: String,
        loc: Location,
        caused_by: String,
        similar_erg_mod: Option<Str>,
        similar_py_mod: Option<Str>,
    ) -> Self {
        let mut erg_str = StyledStrings::default();
        let mut py_str = StyledStrings::default();
        let hint = switch_lang!(
        "japanese" => {
            match (similar_erg_mod, similar_py_mod) {
                (Some(erg), Some(py)) => {
                    erg_str.push_str("似た名前のergモジュールが存在します: ");
                    erg_str.push_str_with_color_and_attribute(erg, HINT, ATTR);
                    py_str.push_str("似た名前のpythonモジュールが存在します: ");
                    py_str.push_str_with_color_and_attribute(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("pythonのモジュールをインポートするためには");
                    hint.push_str_with_color_and_attribute("pyimport", ACCENT, ATTR);
                    hint.push_str("を使用してください");
                    Some(hint.to_string())
                }
                (Some(erg), None) => {
                    erg_str.push_str("似た名前のergモジュールが存在します");
                    erg_str.push_str_with_color_and_attribute(erg, ACCENT, ATTR);
                    None
                }
                (None, Some(py)) => {
                    py_str.push_str("似た名前のpythonモジュールが存在します");
                    py_str.push_str_with_color_and_attribute(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("pythonのモジュールをインポートするためには");
                    hint.push_str_with_color_and_attribute("pyimport", ACCENT, ATTR);
                    hint.push_str("を使用してください");
                    Some(hint.to_string())
                }
                (None, None) => None,
            }
        },
        "simplified_chinese" => {
            match (similar_erg_mod, similar_py_mod) {
                (Some(erg), Some(py)) => {
                    erg_str.push_str("存在相似名称的erg模块: ");
                    erg_str.push_str_with_color_and_attribute(erg, HINT, ATTR);
                    py_str.push_str("存在相似名称的python模块: ");
                    py_str.push_str_with_color_and_attribute(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要导入python模块,请使用");
                    hint.push_str_with_color_and_attribute("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (Some(erg), None) => {
                    erg_str.push_str("存在相似名称的erg模块: ");
                    erg_str.push_str_with_color_and_attribute(erg, HINT, ATTR);
                    None
                }
                (None, Some(py)) => {
                    py_str.push_str("存在相似名称的python模块: ");
                    py_str.push_str_with_color_and_attribute(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要导入python模块,请使用");
                    hint.push_str_with_color_and_attribute("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (None, None) => None,
            }
        },
        "traditional_chinese" => {
            match (similar_erg_mod, similar_py_mod) {
                (Some(erg), Some(py)) => {
                    erg_str.push_str("存在類似名稱的erg模塊: ");
                    erg_str.push_str_with_color_and_attribute(erg, HINT, ATTR);
                    py_str.push_str("存在類似名稱的python模塊: ");
                    py_str.push_str_with_color_and_attribute(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要導入python模塊, 請使用");
                    hint.push_str_with_color_and_attribute("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (Some(erg), None) => {
                    erg_str.push_str("存在類似名稱的erg模塊: ");
                    erg_str.push_str_with_color_and_attribute(erg, HINT, ATTR);
                    None
                }
                (None, Some(py)) => {
                    py_str.push_str("存在類似名稱的python模塊: ");
                    py_str.push_str_with_color_and_attribute(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要導入python模塊, 請使用");
                    hint.push_str_with_color_and_attribute("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (None, None) => None,
            }
        },
        "english" => {
            match (similar_erg_mod, similar_py_mod) {
                (Some(erg), Some(py)) => {
                    erg_str.push_str("similar name erg module exists: ");
                    erg_str.push_str_with_color_and_attribute(erg, HINT, ATTR);
                    py_str.push_str("similar name python module exists: ");
                    py_str.push_str_with_color_and_attribute(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("to import python modules, use ");
                    hint.push_str_with_color_and_attribute("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (Some(erg), None) => {
                    erg_str.push_str("similar name erg module exists: ");
                    erg_str.push_str_with_color_and_attribute(erg, HINT, ATTR);
                    None
                }
                (None, Some(py)) => {
                    py_str.push_str("similar name python module exits: ");
                    py_str.push_str_with_color_and_attribute(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("to import python modules, use ");
                    hint.push_str_with_color_and_attribute("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (None, None) => None,
            }
        },
        );
        // .to_string().is_empty() is not necessarily empty because there are Color or Attribute that are not displayed
        let msg = match (erg_str.is_empty(), py_str.is_empty()) {
            (false, false) => vec![erg_str.to_string(), py_str.to_string()],
            (false, true) => vec![erg_str.to_string()],
            (true, false) => vec![py_str.to_string()],
            (true, true) => vec![],
        };
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, msg, hint)],
                desc,
                errno,
                ImportError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn inner_typedef_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("型はトップレベルで定義されなければなりません"),
                    "simplified_chinese" => format!("类型必须在顶层定义"),
                    "traditional_chinese" => format!("類型必須在頂層定義"),
                    "english" => format!("types must be defined at the top level"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn declare_error(input: Input, errno: usize, loc: Location, caused_by: String) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("d.erファイル内では宣言、別名定義のみが許可されています"),
                    "simplified_chinese" => format!("在d.er文件中只允许声明和别名定义"),
                    "traditional_chinese" => format!("在d.er文件中只允許聲明和別名定義"),
                    "english" => format!("declarations and alias definitions are only allowed in d.er files"),
                ),
                errno,
                SyntaxError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn invalid_type_cast_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        cast_to: &Type,
        hint: Option<String>,
    ) -> Self {
        let name = StyledString::new(name, Some(WARN), Some(ATTR));
        let found = StyledString::new(format!("{}", cast_to), Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{name}の型を{found}にキャストすることはできません"),
                    "simplified_chinese" => format!("{name}的类型无法转换为{found}"),
                    "traditional_chinese" => format!("{name}的類型無法轉換為{found}"),
                    "english" => format!("the type of {name} cannot be cast to {found}"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }
}

#[derive(Debug, Clone)]
pub struct CompileErrors(Vec<CompileError>);

impl std::error::Error for CompileErrors {}

impl_stream_for_wrapper!(CompileErrors, CompileError);

impl From<ParserRunnerErrors> for CompileErrors {
    fn from(err: ParserRunnerErrors) -> Self {
        Self(err.into_iter().map(CompileError::from).collect())
    }
}

impl From<Vec<CompileError>> for CompileErrors {
    fn from(errs: Vec<CompileError>) -> Self {
        Self(errs)
    }
}

impl From<CompileError> for CompileErrors {
    fn from(err: CompileError) -> Self {
        Self(vec![err])
    }
}

impl MultiErrorDisplay<CompileError> for CompileErrors {}

impl fmt::Display for CompileErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_all(f)
    }
}

impl CompileErrors {
    pub fn flush(&mut self) -> Self {
        Self(self.0.drain(..).collect())
    }
}

pub type SingleCompileResult<T> = Result<T, CompileError>;
pub type CompileResult<T> = Result<T, CompileErrors>;
pub type CompileWarning = CompileError;
pub type CompileWarnings = CompileErrors;

#[cfg(test)]
mod test {
    use super::TyCheckError;
    use crate::{
        error::{CompileError, EvalError, LowerError},
        hir::Identifier,
        ty::{Predicate, Type},
        varinfo::VarInfo,
    };
    use erg_common::{config::Input, error::Location};
    use erg_parser::ast::VarName;

    // These Erg codes are not correct grammar.
    // This test make sure sub_msg and hint are displayed correctly.
    #[test]
    fn default_error_format_confirmation() {
        let mut errors = Vec::new();

        let input = Input::Pipe("stack bug error".to_owned());
        let loc = Location::Line(1);
        let err = CompileError::stack_bug(input, loc, 0, 0, "FileName");
        errors.push(err);

        let input = Input::Pipe("checker bug error".to_owned());
        let errno = 0;
        let err = TyCheckError::checker_bug(input, errno, Location::Unknown, "name", 1);
        errors.push(err);

        let loc = Location::LineRange(1, 3);
        let input = Input::Pipe("args\nmissing\nerror".to_string());
        let caused_by = "<caused_by>";
        let err = TyCheckError::args_missing_error(
            input,
            errno,
            loc,
            "\"Callee name here\"",
            caused_by.into(),
            vec!["sample".into(), "args".into(), "here".into()],
        );
        errors.push(err);

        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 0,
            ln_end: 1,
            col_end: 17,
        };
        let expect = Type::Nat;
        let found = Type::Int;
        let input = Input::Pipe("return type error".to_string());
        let name = "name";
        let err = TyCheckError::return_type_error(
            input,
            errno,
            loc,
            caused_by.to_string(),
            name,
            &expect,
            &found,
        );
        errors.push(err);

        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 0,
            ln_end: 1,
            col_end: 4,
        };
        let expect = Type::Nat;
        let found = Type::Int;
        let input = Input::Pipe("type mismatch error".to_string());
        let err = TyCheckError::type_mismatch_error(
            input,
            errno,
            loc,
            caused_by.into(),
            name,
            Some(1),
            &expect,
            &found,
            None,
            Some("hint message here".to_owned()),
        );
        errors.push(err);

        let input = Input::Pipe(
            "too_many_args_error(some_long_name_variable_1,
    some_long_name_variable_2,
    some_long_name_variable_3,
    some_long_name_variable_4) ="
                .to_string(),
        );
        let loc = Location::LineRange(1, 4);
        let callee_name = "callee name";
        let params_len = 3;
        let pos_args_len = 4;
        let kw_args_len = 4;
        let err = TyCheckError::too_many_args_error(
            input,
            errno,
            loc,
            callee_name,
            caused_by.to_string(),
            params_len,
            pos_args_len,
            kw_args_len,
        );
        errors.push(err);

        let input = Input::Pipe("argument error".to_string());
        let loc = Location::range(1, 0, 1, 8);
        let err = TyCheckError::argument_error(input, errno, loc, caused_by.to_string(), 1, 2);
        errors.push(err);

        let input = Input::Pipe("Nat <: Int <: Ratio".to_string());
        let loc = Location::range(1, 0, 1, 10);
        let sub_t = &Type::Nat;
        let sup_t = &Type::Int;
        let err =
            TyCheckError::subtyping_error(input, errno, sub_t, sup_t, loc, caused_by.to_string());
        errors.push(err);

        let input = Input::Pipe("pred unification error".to_string());
        let lhs = &Predicate::Const("Str".into());
        let rhs = &Predicate::Const("Nat".into());
        let err =
            TyCheckError::pred_unification_error(input, errno, lhs, rhs, caused_by.to_string());
        errors.push(err);

        let input = Input::Pipe("Trait member type error".to_string());
        let errno = 0;
        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 0,
            ln_end: 1,
            col_end: 5,
        };
        let t_ty = &Type::Float;
        let exp = &Type::Nat;
        let fnd = &Type::Obj;
        let err = TyCheckError::trait_member_type_error(
            input,
            errno,
            loc,
            caused_by.to_string(),
            "member name",
            t_ty,
            exp,
            fnd,
            Some("hint message here".to_string()),
        );
        errors.push(err);

        let input = Input::Pipe("trait member not defined error".to_string());
        let member_name = "member name";
        let trait_type = &Type::ClassType;
        let class_type = &Type::Ellipsis;
        let hint = Some("hint message here".to_string());
        let err = TyCheckError::trait_member_not_defined_error(
            input,
            errno,
            caused_by.to_string(),
            member_name,
            trait_type,
            class_type,
            hint,
        );
        errors.push(err);

        let input = Input::Pipe("singular no attribute error".to_string());
        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 0,
            ln_end: 1,
            col_end: 8,
        };
        let obj_name = "ojb name";
        let obj_t = Type::Bool;
        let name = "name";
        let similar_name = Some("similar name");
        let err = LowerError::singular_no_attr_error(
            input,
            errno,
            loc,
            caused_by.to_string(),
            obj_name,
            &obj_t,
            name,
            similar_name,
        );
        errors.push(err);

        let input = Input::Pipe("ambiguous type error".to_string());
        let expr = Identifier::new(
            Some(erg_parser::token::Token {
                kind: erg_parser::token::TokenKind::EOF,
                content: "expr_content".into(),
                lineno: 1,
                col_begin: 1,
            }),
            VarName::from_str("variable_name".into()),
            None,
            VarInfo::new(
                Type::Nat,
                crate::varinfo::Mutability::Const,
                erg_common::vis::Visibility::Private,
                crate::varinfo::VarKind::Builtin,
                None,
                None,
                None,
            ),
        );
        let candidates = &[Type::Nat, Type::Inf, Type::Bool];
        let err =
            EvalError::ambiguous_type_error(input, errno, &expr, candidates, caused_by.to_string());
        errors.push(err);

        let input = Input::Pipe("invalid type cast error".to_string());
        let loc = Location::range(1, 8, 1, 17);
        let cast_to = Type::Error;
        let hint = Some("hint message here".to_string());
        let err = EvalError::invalid_type_cast_error(
            input,
            errno,
            loc,
            caused_by.to_string(),
            name,
            &cast_to,
            hint,
        );
        errors.push(err);

        let input = Input::Pipe("override error".to_string());
        let name_loc = Location::range(1, 0, 1, 8);
        let superclass = &Type::Failure;
        let err = TyCheckError::override_error(
            input,
            errno,
            name,
            name_loc,
            superclass,
            caused_by.to_string(),
        );
        errors.push(err);

        let input = Input::Pipe("visibility error".to_string());
        let loc = Location::Line(1);
        let vis = erg_common::vis::Visibility::Private;
        let err =
            TyCheckError::visibility_error(input, errno, loc, caused_by.to_string(), name, vis);
        errors.push(err);

        let input = Input::Pipe("import nunpy as np".to_string());
        let errno = 0;
        let desc = "nunpy is not defined".to_string();
        let loc = Location::range(1, 7, 1, 12);
        let similar_erg_mod = Some("numpyer".into());
        let similar_py_mod = Some("numpy".into());
        let err = TyCheckError::import_error(
            input.clone(),
            errno,
            desc.clone(),
            loc,
            caused_by.to_string(),
            similar_erg_mod.clone(),
            similar_py_mod.clone(),
        );
        errors.push(err);

        let err = TyCheckError::import_error(
            input.clone(),
            errno,
            desc.clone(),
            loc,
            caused_by.to_string(),
            None,
            similar_py_mod,
        );
        errors.push(err);

        let err = TyCheckError::import_error(
            input.clone(),
            errno,
            desc.clone(),
            loc,
            caused_by.to_string(),
            similar_erg_mod,
            None,
        );
        errors.push(err);

        let err =
            TyCheckError::import_error(input, errno, desc, loc, caused_by.to_string(), None, None);
        errors.push(err);

        for err in errors.into_iter() {
            print!("{err}");
        }
    }
}
