use std::fmt::Display;

use erg_common::astr::AtomicStr;
use erg_common::config::Input;
use erg_common::error::{ErrorCore, ErrorDisplay, ErrorKind::*, Location, MultiErrorDisplay};
use erg_common::set::Set;
use erg_common::style::{Attribute, Color, StrSpan, StringSpan, Theme, THEME};
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::{
    fmt_iter, fmt_option_map, fmt_vec, impl_display_and_error, impl_stream_for_wrapper,
    switch_lang, Str,
};

use erg_parser::error::{ParserRunnerError, ParserRunnerErrors};

use crate::ty::{Predicate, Type};

use crate::hir::{Expr, Identifier, Signature};

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

#[derive(Debug)]
pub struct CompileError {
    pub core: Box<ErrorCore>, // ErrorCore is large, so box it
    pub input: Input,
    pub caused_by: AtomicStr,
    pub theme: Theme,
}

impl_display_and_error!(CompileError);

impl From<ParserRunnerError> for CompileError {
    fn from(err: ParserRunnerError) -> Self {
        Self {
            core: Box::new(err.core),
            input: err.input,
            caused_by: "".into(),
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
    fn theme(&self) -> &Theme {
        &self.theme
    }
    fn caused_by(&self) -> &str {
        &self.caused_by
    }
    fn ref_inner(&self) -> Option<&Self> {
        None
    }
}

const URL: StrSpan = StrSpan::new(
    "https://github.com/erg-lang/erg",
    Some(Color::White),
    Some(Attribute::Underline),
);

impl CompileError {
    pub fn new(core: ErrorCore, input: Input, caused_by: AtomicStr) -> Self {
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
                errno,
                CompilerSystemError,
                loc,
                switch_lang!(
                    "japanese" => format!("これはErg compilerのバグです、開発者に報告して下さい ({URL})\n\n{fn_name}:{line}より発生"),
                    "simplified_chinese" => format!("这是Erg编译器的错误，请报告给{URL}\n\n原因来自: {fn_name}:{line}"),
                    "traditional_chinese" => format!("這是Erg編譯器的錯誤，請報告給{URL}\n\n原因來自: {fn_name}:{line}"),
                    "english" => format!("this is a bug of the Erg compiler, please report it to {URL}\n\ncaused from: {fn_name}:{line}"),
                ),
                None,
            ),
            input,
            "".into(),
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
                0,
                CompilerSystemError,
                loc,
                switch_lang!(
                    "japanese" => format!("スタックの要素数が異常です (要素数: {stack_len}, ブロックID: {block_id})\n\
                            これはコンパイラのバグです、開発者に報告して下さい ({URL})\n\
                            {fn_name}より発生"),
                "simplified_chinese" => format!("堆栈中的元素数无效（元素数: {stack_len}，块id: {block_id}）\n\
                            这是 Erg 编译器的一个错误，请报告它 ({URL})\n\
                            起因于: {fn_name}"),
                "traditional_chinese" => format!("堆棧中的元素數無效（元素數: {stack_len}，塊id: {block_id}）\n\
                            這是 Erg 編譯器的一個錯誤，請報告它 ({URL})\n\
                            起因於: {fn_name}"),
                    "english" => format!("the number of elements in the stack is invalid (num of elems: {stack_len}, block id: {block_id})\n\
                            this is a bug of the Erg compiler, please report it ({URL})\n\
                            caused from: {fn_name}"),
                ),
                None,
            ),
            input,
            "".into(),
        )
    }

    pub fn feature_error(input: Input, loc: Location, name: &str, caused_by: AtomicStr) -> Self {
        Self::new(
            ErrorCore::new(
                0,
                FeatureError,
                loc,
                switch_lang!(
                    "japanese" => format!("この機能({name})はまだ正式に提供されていません"),
                    "simplified_chinese" => format!("此功能（{name}）尚未实现"),
                    "traditional_chinese" => format!("此功能（{name}）尚未實現"),
                    "english" => format!("this feature({name}) is not implemented yet"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn system_exit() -> Self {
        Self::new(
            ErrorCore::new(
                0,
                SystemExit,
                Location::Unknown,
                switch_lang!(
                    "japanese" => "システムを終了します",
                    "simplified_chinese" => "系统正在退出",
                    "traditional_chinese" => "系統正在退出",
                    "english" => "system is exiting",
                ),
                None,
            ),
            Input::Dummy,
            "".into(),
        )
    }
}

pub type TyCheckError = CompileError;

const ERR: Color = THEME.colors.error;
const WARNING: Color = THEME.colors.warning;
const HINT: Color = THEME.colors.hint;

impl TyCheckError {
    pub fn dummy(input: Input, errno: usize) -> Self {
        Self::new(ErrorCore::dummy(errno), input, "".into())
    }

    pub fn unreachable(input: Input, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), input, "".into())
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
                errno,
                CompilerSystemError,
                loc,
                switch_lang!(
                    "japanese" => format!("これはErg compilerのバグです、開発者に報告して下さい ({URL})\n\n{fn_name}:{line}より発生"),
                    "simplified_chinese" => format!("这是Erg编译器的错误，请报告给{URL}\n\n原因来自: {fn_name}:{line}"),
                    "traditional_chinese" => format!("這是Erg編譯器的錯誤，請報告給{URL}\n\n原因來自: {fn_name}:{line}"),
                    "english" => format!("this is a bug of the Erg compiler, please report it to {URL}\n\ncaused from: {fn_name}:{line}"),
                ),
                None,
            ),
            input,
            "".into(),
        )
    }

    pub fn no_type_spec_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}の型が指定されていません"),
                    "simplified_chinese" => format!("{name}的类型未指定"),
                    "traditional_chinese" => format!("{name}的類型未指定"),
                    "english" => format!("the type of {name} is not specified"),
                ),
                None,
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
        caused_by: AtomicStr,
    ) -> Self {
        let param_ts = fmt_iter(param_ts);
        Self::new(
            ErrorCore::new(
                errno,
                NotImplementedError,
                callee.loc(),
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
                None,
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
        caused_by: AtomicStr,
        name: &str,
        nth_param: Option<usize>,
        expect: &Type,
        found: &Type,
        candidates: Option<Set<Type>>,
        hint: Option<AtomicStr>,
    ) -> Self {
        let ord = match nth_param {
            Some(pos) => switch_lang!(
                "japanese" => format!("({pos}番目の引数)"),
                "simplified_chinese" => format!("(第{pos}个参数)"),
                "traditional_chinese" => format!("(第{pos}個參數)"),
                "english" => format!(" (the {} argument)", ordinal_num(pos)),
            ),
            None => "".into(),
        };
        let name = StringSpan::new(
            &format!("{}{}", name, ord),
            Some(WARNING),
            Some(Attribute::Bold),
        );
        let expect = StringSpan::new(&format!("{}", expect), Some(HINT), Some(Attribute::Bold));
        let found = StringSpan::new(&format!("{}", found), Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}の型が違います\n\n予期した型: {expect}\n与えられた型: {found}{}", fmt_option_map!(pre "\n与えられた型の単一化候補: ", candidates, |x: &Set<Type>| x.folded_display())),
                    "simplified_chinese" => format!("{name}的类型不匹配\n\n预期: {expect}\n但找到: {found}{}", fmt_option_map!(pre "\n某一类型的统一候选: ", candidates, |x: &Set<Type>| x.folded_display())),
                    "traditional_chinese" => format!("{name}的類型不匹配\n\n預期: {expect}\n但找到: {found}{}", fmt_option_map!(pre "\n某一類型的統一候選: ", candidates, |x: &Set<Type>| x.folded_display())),
                    "english" => format!("the type of {name} is mismatched\n\nexpected: {expect}\nbut found: {found}{}", fmt_option_map!(pre "\nunification candidates of a given type: ", candidates, |x: &Set<Type>| x.folded_display())),
                ),
                hint,
            ),
            input,
            caused_by,
        )
    }

    pub fn return_type_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
        expect: &Type,
        found: &Type,
    ) -> Self {
        let expect = StringSpan::new(&format!("{}", expect), Some(HINT), Some(Attribute::Bold));
        let found = StringSpan::new(&format!("{}", found), Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}の戻り値の型が違います\n\n予期した型: {expect}\n与えられた型: {found}"),
                    "simplified_chinese" => format!("{name}的返回类型不匹配\n\n预期: {expect}\n但找到: {found}"),
                    "traditional_chinese" => format!("{name}的返回類型不匹配\n\n預期: {expect}\n但找到: {found}"),
                    "english" => format!("the return type of {name} is mismatched\n\nexpected: {expect}\nbut found: {found}"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn uninitialized_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
        t: &Type,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                NameError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}: {t}は宣言されましたが初期化されていません"),
                    "simplified_chinese" => format!("{name}: {t}已声明但未初始化"),
                    "traditional_chinese" => format!("{name}: {t}已宣告但未初始化"),
                    "english" => format!("{name}: {t} is declared but not initialized"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn argument_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        expect: usize,
        found: usize,
    ) -> Self {
        let expect = StringSpan::new(&format!("{}", expect), Some(HINT), Some(Attribute::Bold));
        let found = StringSpan::new(&format!("{}", found), Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("ポジショナル引数の数が違います\n\n予期した個数: {expect}\n与えられた個数: {found}"),
                    "simplified_chinese" => format!("正则参数的数量不匹配\n\n预期: {expect}\n但找到: {found}"),
                    "traditional_chinese" => format!("正則參數的數量不匹配\n\n預期: {expect}\n但找到: {found}"),
                    "english" => format!("the number of positional arguments is mismatched\n\nexpected:  {expect}\nbut found: {found}"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn match_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        expr_t: &Type,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{expr_t}型の全パターンを網羅していません"),
                    "simplified_chinese" => format!("并非所有{expr_t}类型的模式都被涵盖"),
                    "traditional_chinese" => format!("並非所有{expr_t}類型的模式都被涵蓋"),
                    "english" => format!("not all patterns of type {expr_t} are covered"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn infer_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        expr: &str,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{expr}の型が推論できません"),
                    "simplified_chinese" => format!("无法推断{expr}的类型"),
                    "traditional_chinese" => format!("無法推斷{expr}的類型"),
                    "english" => format!("failed to infer the type of {expr}"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn dummy_infer_error(input: Input, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), input, "".into())
    }

    pub fn not_relation(input: Input, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), input, "".into())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn too_many_args_error(
        input: Input,
        errno: usize,
        loc: Location,
        callee_name: &str,
        caused_by: AtomicStr,
        params_len: usize,
        pos_args_len: usize,
        kw_args_len: usize,
    ) -> Self {
        let name = readable_name(callee_name);
        let expect = StringSpan::new(
            &format!("{}", params_len),
            Some(HINT),
            Some(Attribute::Bold),
        );
        let pos_args_len = StringSpan::new(
            &format!("{}", pos_args_len),
            Some(ERR),
            Some(Attribute::Bold),
        );
        let kw_args_len = StringSpan::new(
            &format!("{}", kw_args_len),
            Some(ERR),
            Some(Attribute::Bold),
        );
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!(
                        "{name}に渡された引数の数が多すぎます

必要な引数の合計数: {expect}個
渡された引数の数:   {pos_args_len}個
キーワード引数の数: {kw_args_len}個"
                    ),
                    "simplified_chinese" => format!("传递给{name}的参数过多

: {expect}
: {pos_args_len}
: {kw_args_len}"
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
                None,
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
        caused_by: AtomicStr,
        missing_len: usize,
        missing_params: Vec<Str>,
    ) -> Self {
        let name = readable_name(callee_name);
        let vec_cxt = StringSpan::new(
            &fmt_vec(&missing_params),
            Some(WARNING),
            Some(Attribute::Bold),
        );
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}に渡された引数が{missing_len}個足りません({vec_cxt})" ),
                    "simplified_chinese" => format!("{name}的{missing_len}个位置参数不被传递({vec_cxt})"),
                    "traditional_chinese" => format!("{name}的{missing_len}個位置參數不被傳遞({vec_cxt})"),
                    "english" => format!("missing {missing_len} positional argument(s) for {name}: {vec_cxt}"),
                ),
                None,
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
        caused_by: AtomicStr,
        arg_name: &str,
    ) -> Self {
        let name = readable_name(callee_name);
        let found = StringSpan::new(arg_name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}の引数{found}が複数回渡されています"),
                    "simplified_chinese" => format!("{name}的参数{found}被多次传递"),
                    "traditional_chinese" => format!("{name}的參數{found}被多次傳遞"),
                    "english" => format!("{name}'s argument {found} is passed multiple times"),
                ),
                None,
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
        caused_by: AtomicStr,
        param_name: &str,
    ) -> Self {
        let name = readable_name(callee_name);
        let found = StringSpan::new(param_name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}には予期しないキーワード引数{found}が渡されています"),
                    "simplified_chinese" => format!("{name}得到了意外的关键字参数{found}"),
                    "traditional_chinese" => format!("{name}得到了意外的關鍵字參數{found}"),
                    "english" => format!("{name} got unexpected keyword argument {found}"),
                ),
                None,
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
        caused_by: AtomicStr,
    ) -> Self {
        let lhs_t = StringSpan::new(&format!("{}", lhs_t), Some(WARNING), Some(Attribute::Bold));
        let rhs_t = StringSpan::new(&format!("{}", rhs_t), Some(WARNING), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("型の単一化に失敗しました\n\n左辺: {lhs_t}\n右辺: {rhs_t}"),
                    "simplified_chinese" => format!("类型统一失败\n\n左边: {lhs_t}\n右边: {rhs_t}"),
                    "traditional_chinese" => format!("類型統一失敗\n\n左邊: {lhs_t}\n右邊: {rhs_t}"),
                    "english" => format!("unification failed\n\nlhs: {lhs_t}\nrhs: {rhs_t}"),
                ),
                None,
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
        caused_by: AtomicStr,
    ) -> Self {
        let lhs_t = StringSpan::new(&format!("{}", lhs_t), Some(WARNING), Some(Attribute::Bold));
        let rhs_t = StringSpan::new(&format!("{}", rhs_t), Some(WARNING), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("型の再単一化に失敗しました\n\n左辺: {lhs_t}\n右辺: {rhs_t}"),
                    "simplified_chinese" => format!("重新统一类型失败\n\n左边: {lhs_t}\n右边: {rhs_t}"),
                    "traditional_chinese" => format!("重新統一類型失敗\n\n左邊: {lhs_t}\n右邊: {rhs_t}"),
                    "english" => format!("re-unification failed\n\nlhs: {lhs_t}\nrhs: {rhs_t}"),
                ),
                None,
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
        caused_by: AtomicStr,
    ) -> Self {
        let sub_t = StringSpan::new(&format!("{}", sub_t), Some(WARNING), Some(Attribute::Bold));
        let sup_t = StringSpan::new(&format!("{}", sup_t), Some(WARNING), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("この式の部分型制約を満たせません\n\nサブタイプ: {sub_t}\nスーパータイプ: {sup_t}"),
                    "simplified_chinese" => format!("无法满足此表达式中的子类型约束\n\n子类型: {sub_t}\n超类型: {sup_t}"),
                    "traditional_chinese" => format!("無法滿足此表達式中的子類型約束\n\n子類型: {sub_t}\n超類型: {sup_t}"),
                    "english" => format!("the subtype constraint in this expression cannot be satisfied:\nsubtype: {sub_t}\nsupertype: {sup_t}"),
                ),
                None,
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
        caused_by: AtomicStr,
    ) -> Self {
        let lhs = StringSpan::new(&format!("{}", lhs), Some(WARNING), Some(Attribute::Bold));
        let rhs = StringSpan::new(&format!("{}", rhs), Some(WARNING), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                Location::Unknown,
                switch_lang!(
                    "japanese" => format!("述語式の単一化に失敗しました\n\n左辺: {lhs}\n右辺: {rhs}"),
                    "simplified_chinese" => format!("无法统一谓词表达式\n\n左边: {lhs}\n左边: {rhs}"),
                    "traditional_chinese" => format!("無法統一謂詞表達式\n\n左邊: {lhs}\n左邊: {rhs}"),
                    "english" => format!("predicate unification failed\n\nlhs: {lhs}\nrhs: {rhs}"),
                ),
                None,
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
        caused_by: AtomicStr,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{proj}の候補がありません"),
                    "simplified_chinese" => format!("{proj}没有候选项"),
                    "traditional_chinese" => format!("{proj}沒有候選項"),
                    "english" => format!("no candidate for {proj}"),
                ),
                hint,
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
        caused_by: AtomicStr,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{class}は{trait_}を実装していません"),
                    "simplified_chinese" => format!("{class}没有实现{trait_}"),
                    "traditional_chinese" => format!("{class}沒有實現{trait_}"),
                    "english" => format!("{class} does not implement {trait_}"),
                ),
                hint,
            ),
            input,
            caused_by,
        )
    }

    pub fn method_definition_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
        hint: Option<AtomicStr>,
    ) -> Self {
        let found = StringSpan::new(name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                MethodError,
                loc,
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
                hint,
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
        caused_by: AtomicStr,
        member_name: &str,
        trait_type: &Type,
        expect: &Type,
        found: &Type,
        hint: Option<AtomicStr>,
    ) -> Self {
        let member_name = StringSpan::new(member_name, Some(WARNING), Some(Attribute::Bold));
        let expect = StringSpan::new(&format!("{}", expect), Some(HINT), Some(Attribute::Bold));
        let found = StringSpan::new(&format!("{}", found), Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{member_name}の型が違います\n\n{trait_type}で宣言された型: {expect}\n与えられた型: {found}"),
                    "simplified_chinese" => format!("{member_name}的类型不匹配\n\n在{trait_type}中声明的类型: {expect}\n但找到: {found}"),
                    "traditional_chinese" => format!("{member_name}的類型不匹配\n\n在{trait_type}中聲明的類型: {expect}\n但找到: {found}"),
                    "english" => format!("the type of {member_name} is mismatched\n\ndeclared in {trait_type}: {expect}\nbut found: {found}"),
                ),
                hint,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trait_member_not_defined_error(
        input: Input,
        errno: usize,
        caused_by: AtomicStr,
        member_name: &str,
        trait_type: &Type,
        class_type: &Type,
        hint: Option<AtomicStr>,
    ) -> Self {
        let member_name = StringSpan::new(member_name, Some(WARNING), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                Location::Unknown,
                switch_lang!(
                    "japanese" => format!("{trait_type}の{member_name}が{class_type}で実装されていません"),
                    "simplified_chinese" => format!("{trait_type}中的{member_name}没有在{class_type}中实现"),
                    "traditional_chinese" => format!("{trait_type}中的{member_name}沒有在{class_type}中實現"),
                    "english" => format!("{member_name} of {trait_type} is not implemented in {class_type}"),
                ),
                hint,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn not_in_trait_error(
        input: Input,
        errno: usize,
        caused_by: AtomicStr,
        member_name: &str,
        trait_type: &Type,
        class_type: &Type,
        hint: Option<AtomicStr>,
    ) -> Self {
        let member_name = StringSpan::new(member_name, Some(WARNING), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                Location::Unknown,
                switch_lang!(
                    "japanese" => format!("{class_type}の{member_name}は{trait_type}で宣言されていません"),
                    "simplified_chinese" => format!("{class_type}中的{member_name}没有在{trait_type}中声明"),
                    "traditional_chinese" => format!("{class_type}中的{member_name}沒有在{trait_type}中聲明"),
                    "english" => format!("{member_name} of {class_type} is not declared in {trait_type}"),
                ),
                hint,
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
        caused_by: AtomicStr,
    ) -> Self {
        let found = StringSpan::new(name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("型変数{found}が定義されていません"),
                    "simplified_chinese" => format!("类型变量{found}没有定义"),
                    "traditional_chinese" => format!("類型變量{found}沒有定義"),
                    "english" => format!("type variable {found} is not defined"),
                ),
                None,
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
        caused_by: AtomicStr,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                expr.loc(),
                switch_lang!(
                    "japanese" => format!("{expr}の型を一意に決定できませんでした\n\n候補: {}", fmt_vec(candidates)),
                    "simplified_chinese" => format!("无法确定{expr}的类型\n\n候选: {}", fmt_vec(candidates)),
                    "traditional_chinese" => format!("無法確定{expr}的類型\n\n候選: {}", fmt_vec(candidates)),
                    "english" => format!("cannot determine the type of {expr}\n\ncandidates: {}", fmt_vec(candidates)),
                ),
                Some(
                    switch_lang!(
                        "japanese" => "多相関数の場合は`f|T := Int|`, 型属性の場合は`T|T <: Trait|.X`などのようにして型を指定してください",
                        "simplified_chinese" => "如果是多态函数，请使用`f|T := Int|`，如果是类型属性，请使用`T|T <: Trait|.X`等方式指定类型",
                        "traditional_chinese" => "如果是多型函數，請使用`f|T := Int|`，如果是類型屬性，請使用`T|T <: Trait|.X`等方式指定類型",
                        "english" => "if it is a polymorphic function, use `f|T := Int|`, or if it is a type attribute, use `T|T <: Trait|.X` etc. to specify the type",
                    ).into(),
                ),
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
    pub fn not_const_expr(input: Input, errno: usize, loc: Location, caused_by: AtomicStr) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                NotConstExpr,
                loc,
                switch_lang!(
                    "japanese" => "定数式ではありません",
                    "simplified_chinese" => "不是常量表达式",
                    "traditional_chinese" => "不是常量表達式",
                    "english" => "not a constant expression",
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn invalid_literal(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                SyntaxError,
                loc,
                switch_lang!(
                    "japanese" => "リテラルが不正です",
                    "simplified_chinese" => "字面量不合法",
                    "traditional_chinese" => "字面量不合法",
                    "english" => "invalid literal",
                ),
                None,
            ),
            input,
            caused_by,
        )
    }
}

pub type EffectError = TyCheckError;
pub type EffectErrors = TyCheckErrors;

impl EffectError {
    pub fn has_effect<S: Into<AtomicStr>>(
        input: Input,
        errno: usize,
        expr: &Expr,
        caused_by: S,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                HasEffect,
                expr.loc(),
                switch_lang!(
                    "japanese" => "この式には副作用があります",
                    "simplified_chinese" => "此表达式会产生副作用",
                    "traditional_chinese" => "此表達式會產生副作用",
                    "english" => "this expression causes a side-effect",
                ),
                None,
            ),
            input,
            caused_by.into(),
        )
    }

    pub fn proc_assign_error<S: Into<AtomicStr>>(
        input: Input,
        errno: usize,
        sig: &Signature,
        caused_by: S,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                HasEffect,
                sig.loc(),
                switch_lang!(
                    "japanese" => "プロシージャを通常の変数に代入することはできません",
                    "simplified_chinese" => "不能将过程赋值给普通变量",
                    "traditional_chinese" => "不能將過程賦值給普通變量",
                    "english" => "cannot assign a procedure to a normal variable",
                ),
                Some(
                    switch_lang!(
                        "japanese" => "変数の末尾に`!`をつけてください",
                        "simplified_chinese" => "请在变量名后加上`!`",
                        "traditional_chinese" => "請在變量名後加上`!`",
                        "english" => "add `!` to the end of the variable name",
                    )
                    .into(),
                ),
            ),
            input,
            caused_by.into(),
        )
    }
}

pub type OwnershipError = TyCheckError;
pub type OwnershipErrors = TyCheckErrors;

impl OwnershipError {
    pub fn move_error<S: Into<AtomicStr>>(
        input: Input,
        errno: usize,
        name: &str,
        name_loc: Location,
        moved_loc: Location,
        caused_by: S,
    ) -> Self {
        let found = StringSpan::new(name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                MoveError,
                name_loc,
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
                None,
            ),
            input,
            caused_by.into(),
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
    pub fn syntax_error<S: Into<AtomicStr>>(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        desc: S,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self::new(
            ErrorCore::new(errno, SyntaxError, loc, desc, hint),
            input,
            caused_by,
        )
    }

    pub fn duplicate_decl_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                errno,
                NameError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}は既に宣言されています"),
                    "simplified_chinese" => format!("{name}已声明"),
                    "traditional_chinese" => format!("{name}已聲明"),
                    "english" => format!("{name} is already declared"),
                ),
                Option::<AtomicStr>::None,
            ),
            input,
            caused_by,
        )
    }

    pub fn duplicate_definition_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                errno,
                NameError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}は既に定義されています"),
                    "simplified_chinese" => format!("{name}已定义"),
                    "traditional_chinese" => format!("{name}已定義"),
                    "english" => format!("{name} is already defined"),
                ),
                Option::<AtomicStr>::None,
            ),
            input,
            caused_by,
        )
    }

    pub fn violate_decl_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
        spec_t: &Type,
        found_t: &Type,
    ) -> Self {
        let name = readable_name(name);
        let expect = StringSpan::new(&format!("{}", spec_t), Some(HINT), Some(Attribute::Bold));
        let found = StringSpan::new(&format!("{}", found_t), Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}は{expect}型として宣言されましたが、{found}型のオブジェクトが代入されています"),
                    "simplified_chinese" => format!("{name}被声明为{expect}，但分配了一个{found}对象"),
                    "traditional_chinese" => format!("{name}被聲明為{expect}，但分配了一個{found}對象"),
                    "english" => format!("{name} was declared as {expect}, but an {found} object is assigned"),
                ),
                Option::<AtomicStr>::None,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_var_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
            .into()
        });
        let found = StringSpan::new(name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                NameError,
                loc,
                switch_lang!(
                    "japanese" => format!("{found}という変数は定義されていません"),
                    "simplified_chinese" => format!("{found}未定义"),
                    "traditional_chinese" => format!("{found}未定義"),
                    "english" => format!("{found} is not defined"),
                ),
                hint,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_attr_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
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
            .into()
        });
        let found = StringSpan::new(name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                AttributeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{obj_t}型オブジェクトに{found}という属性はありません"),
                    "simplified_chinese" => format!("{obj_t}对象没有属性{found}"),
                    "traditional_chinese" => format!("{obj_t}對像沒有屬性{found}"),
                    "english" => format!("{obj_t} object has no attribute {found}"),
                ),
                hint,
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
        caused_by: AtomicStr,
        obj_name: &str,
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
            .into()
        });
        let found = StringSpan::new(name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                AttributeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{obj_name}(: {obj_t})に{found}という属性はありません"),
                    "simplified_chinese" => format!("{obj_name}(: {obj_t})没有属性{found}"),
                    "traditional_chinese" => format!("{obj_name}(: {obj_t})沒有屬性{found}"),
                    "english" => format!("{obj_name}(: {obj_t}) has no attribute {found}"),
                ),
                hint,
            ),
            input,
            caused_by,
        )
    }

    pub fn reassign_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
    ) -> Self {
        let name = StringSpan::new(readable_name(name), Some(WARNING), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                AssignError,
                loc,
                switch_lang!(
                    "japanese" => format!("変数{name}に複数回代入することはできません"),
                    "simplified_chinese" => format!("不能为变量{name}分配多次"),
                    "traditional_chinese" => format!("不能為變量{name}分配多次"),
                    "english" => format!("variable {name} cannot be assigned more than once"),
                ),
                None,
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
        caused_by: AtomicStr,
    ) -> Self {
        let name = StringSpan::new(readable_name(name), Some(WARNING), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                UnusedWarning,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}は使用されていません"),
                    "simplified_chinese" => format!("{name}未使用"),
                    "traditional_chinese" => format!("{name}未使用"),
                    "english" => format!("{name} is not used"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn del_error(input: Input, errno: usize, ident: &Identifier, caused_by: AtomicStr) -> Self {
        let name = StringSpan::new(
            readable_name(ident.inspect()),
            Some(WARNING),
            Some(Attribute::Bold),
        );
        Self::new(
            ErrorCore::new(
                errno,
                NameError,
                ident.loc(),
                switch_lang!(
                    "japanese" => format!("{name}は削除できません"),
                    "simplified_chinese" => format!("{name}不能删除"),
                    "traditional_chinese" => format!("{name}不能刪除"),
                    "english" => format!("{name} cannot be deleted"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn visibility_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
        vis: Visibility,
    ) -> Self {
        let name = readable_name(name);
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
        let found = StringSpan::new(name, Some(ERR), Some(Attribute::Bold));
        Self::new(
            ErrorCore::new(
                errno,
                VisibilityError,
                loc,
                switch_lang!(
                    "japanese" => format!("{found}は{visibility}変数です"),
                    "simplified_chinese" => format!("{found}是{visibility}变量",),
                    "traditional_chinese" => format!("{found}是{visibility}變量",),
                    "english" => format!("{found} is {visibility} variable",),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn override_error<S: Into<AtomicStr>>(
        input: Input,
        errno: usize,
        name: &str,
        name_loc: Location,
        superclass: &Type,
        caused_by: S,
    ) -> Self {
        let name = StringSpan::new(name, Some(ERR), Some(Attribute::Bold));
        let superclass = StringSpan::new(
            &format!("{}", superclass),
            Some(WARNING),
            Some(Attribute::Bold),
        );
        Self::new(
            ErrorCore::new(
                errno,
                NameError,
                name_loc,
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
                Some(switch_lang!(
                    "japanese" => "デフォルトでオーバーライドはできません(`Override`デコレータを使用してください)",
                    "simplified_chinese" => "默认不可重写(请使用`Override`装饰器)",
                    "traditional_chinese" => "默認不可重寫(請使用`Override`裝飾器)",
                    "english" => "cannot override by default (use `Override` decorator)",
                ).into()),
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
        caused_by: AtomicStr,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                InheritanceError,
                loc,
                switch_lang!(
                    "japanese" => format!("{class}は継承できません"),
                    "simplified_chinese" => format!("{class}不可继承"),
                    "traditional_chinese" => format!("{class}不可繼承"),
                    "english" => format!("{class} is not inheritable"),
                ),
                None,
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
        caused_by: AtomicStr,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self::new(
            ErrorCore::new(errno, IoError, loc, desc, hint),
            input,
            caused_by,
        )
    }

    pub fn module_env_error(
        input: Input,
        errno: usize,
        mod_name: &str,
        loc: Location,
        caused_by: AtomicStr,
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
        caused_by: AtomicStr,
        similar_erg_mod: Option<Str>,
        similar_py_mod: Option<Str>,
    ) -> Self {
        let hint = match (similar_erg_mod, similar_py_mod) {
            (Some(erg), Some(py)) => {
                let erg = StringSpan::new(&erg, Some(WARNING), Some(Attribute::Bold));
                let py = StringSpan::new(&py, Some(WARNING), Some(Attribute::Bold));
                Some(format!(
                "similar name erg module {erg} and python module {py} exists (to import python modules, use `pyimport`)",
            ))
            }
            (Some(erg), None) => {
                let erg = StringSpan::new(&erg, Some(WARNING), Some(Attribute::Bold));
                Some(format!("similar name erg module exists: {erg}"))
            }
            (None, Some(py)) => {
                let py = StringSpan::new(&py, Some(WARNING), Some(Attribute::Bold));
                Some(format!("similar name python module exists: {py} (to import python modules, use `pyimport`)"))
            }
            (None, None) => None,
        };
        let hint = hint.map(AtomicStr::from);
        Self::file_error(input, errno, desc, loc, caused_by, hint)
    }

    pub fn inner_typedef_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("型はトップレベルで定義されなければなりません"),
                    "simplified_chinese" => format!("类型必须在顶层定义"),
                    "traditional_chinese" => format!("類型必須在頂層定義"),
                    "english" => format!("types must be defined at the top level"),
                ),
                None,
            ),
            input,
            caused_by,
        )
    }

    pub fn declare_error(input: Input, errno: usize, loc: Location, caused_by: AtomicStr) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                SyntaxError,
                loc,
                switch_lang!(
                    "japanese" => format!("d.erファイル内では宣言、別名定義のみが許可されています"),
                    "simplified_chinese" => format!("在d.er文件中只允许声明和别名定义"),
                    "traditional_chinese" => format!("在d.er文件中只允許聲明和別名定義"),
                    "english" => format!("declarations and alias definitions are only allowed in d.er files"),
                ),
                None,
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
        caused_by: AtomicStr,
        name: &str,
        cast_to: &Type,
        hint: Option<AtomicStr>,
    ) -> Self {
        let name = StringSpan::new(name, Some(WARNING), Some(Attribute::Bold));
        let found = StringSpan::new(
            &format!("{}", cast_to),
            Some(WARNING),
            Some(Attribute::Bold),
        );
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}の型を{found}にキャストすることはできません"),
                    "simplified_chinese" => format!("{name}的类型无法转换为{found}"),
                    "traditional_chinese" => format!("{name}的類型無法轉換為{found}"),
                    "english" => format!("the type of {name} cannot be cast to {found}"),
                ),
                hint,
            ),
            input,
            caused_by,
        )
    }
}

#[derive(Debug)]
pub struct CompileErrors(Vec<CompileError>);

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

    use erg_common::{config::Input, error::Location};

    use crate::ty::Type;

    use super::TyCheckError;

    #[test]
    fn default_arg_error_test() {
        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 0,
            ln_end: 1,
            col_end: 5,
        };
        let input = Input::Pipe("a = 1".to_string());
        let caused_by = "File name here basically";
        let desc = "Some kinds of error description here";
        let hint = Some("Some hint massage here\n".into());

        let err = TyCheckError::syntax_error(input, 0, loc, caused_by.into(), desc, hint);
        print!("{}", err);

        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 24,
            ln_end: 1,
            col_end: 27,
        };
        let input = Input::Pipe("Person = Class { name = Str }".to_string());
        let caused_by = "File name here basically";
        let err = TyCheckError::args_missing_error(
            input,
            0,
            loc,
            "\"Callee name here\"",
            caused_by.into(),
            0,
            vec!["sample".into(), "args".into(), "here".into()],
        );
        print!("{}", err);

        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 0,
            ln_end: 3,
            col_end: 5,
        };
        let input = Input::Pipe(
            "\
if True:
    sample
    end
"
            .to_string(),
        );
        let caused_by = "File name here basically";
        let err = TyCheckError::args_missing_error(
            input,
            0,
            loc,
            "\"Callee name here\"",
            caused_by.into(),
            0,
            vec!["sample".into(), "args".into(), "here".into()],
        );
        print!("{}", err);

        let loc = Location::RangePair {
            ln_first: (1, 2),
            col_first: (0, 1),
            ln_second: (4, 4),
            col_second: (9, 10),
        };
        let input = Input::Pipe(
            "\
a: Nat = 1
a.ownership_is_moved()

function(a)
"
            .to_string(),
        );
        let err = TyCheckError::checker_bug(input, 0, loc, "file_name", 0);
        print!("{}", err);

        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 0,
            ln_end: 1,
            col_end: 3,
        };
        let input = Input::Pipe("add(x, y):Nat = x - y".to_string());
        let err = TyCheckError::checker_bug(input, 0, loc, "file_name", 0);
        print!("{}", err);

        let loc = Location::Range {
            ln_begin: 1,
            col_begin: 11,
            ln_end: 1,
            col_end: 14,
        };
        let expect = Type::Nat;
        let found = Type::Obj;
        let input = Input::Pipe("add(x, y): Nat = x - y".to_string());
        let caused_by = "File name here basically";
        let err = TyCheckError::return_type_error(
            input,
            0,
            loc,
            caused_by.into(),
            "name",
            &expect,
            &found,
        );
        print!("{}", err);

        let input = Input::Pipe("Dummy code here".to_string());
        let err = TyCheckError::unreachable(input, "file name here", 1);
        print!("{}", err);
    }
}
