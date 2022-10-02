use std::fmt::Display;

use erg_common::astr::AtomicStr;
use erg_common::color::{GREEN, RED, RESET, YELLOW};
use erg_common::config::Input;
use erg_common::error::{ErrorCore, ErrorDisplay, ErrorKind::*, Location, MultiErrorDisplay};
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::{
    fmt_iter, fmt_option_map, fmt_vec, impl_display_and_error, impl_stream_for_wrapper,
    switch_lang, Str,
};

use erg_parser::error::{ParserRunnerError, ParserRunnerErrors};

use erg_type::{Predicate, Type};

use crate::hir::Expr;

/// dname is for "double under name"
pub fn binop_to_dname(op: &str) -> &str {
    match op {
        "+" => "__add__",
        "-" => "__sub__",
        "*" => "__mul__",
        "/" => "__div__",
        "**" => "__pow__",
        "%" => "__mod__",
        ".." => "__rng__",
        "<.." => "__lorng__",
        "..<" => "__rorng__",
        "<..<" => "__orng__",
        "and" => "__and__",
        "or" => "__or__",
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
        _ => todo!(),
    }
}

pub fn unaryop_to_dname(op: &str) -> &str {
    match op {
        "+" => "__pos__",
        "-" => "__neg__",
        "~" => "__invert__",
        "!" => "__mutate__",
        "..." => "__spread__",
        _ => todo!(),
    }
}

pub fn readable_name(name: &str) -> &str {
    match name {
        "__add__" => "`+`",
        "__sub__" => "`-`",
        "__mul__" => "`*`",
        "__div__" => "`/`",
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
    pub core: ErrorCore,
    pub input: Input,
    pub caused_by: AtomicStr,
}

impl_display_and_error!(CompileError);

impl From<ParserRunnerError> for CompileError {
    fn from(err: ParserRunnerError) -> Self {
        Self {
            core: err.core,
            input: err.input,
            caused_by: "".into(),
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

impl CompileError {
    pub const fn new(core: ErrorCore, input: Input, caused_by: AtomicStr) -> Self {
        Self {
            core,
            input,
            caused_by,
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
                    "japanese" => format!("これはErg compilerのバグです、開発者に報告して下さい (https://github.com/erg-lang/erg)\n{fn_name}:{line}より発生"),
                    "simplified_chinese" => format!("这是Erg编译器的错误，请报告给https://github.com/erg-lang/erg\n原因来自：{fn_name}:{line}"),
                    "traditional_chinese" => format!("這是Erg編譯器的錯誤，請報告給https://github.com/erg-lang/erg\n原因來自：{fn_name}:{line}"),
                    "english" => format!("this is a bug of the Erg compiler, please report it to https://github.com/erg-lang/erg\ncaused from: {fn_name}:{line}"),
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
                            これはコンパイラのバグです、開発者に報告して下さい (https://github.com/erg-lang/erg)\n\
                            {fn_name}より発生"),
                "simplified_chinese" => format!("堆栈中的元素数无效（元素数：{stack_len}，块id：{block_id}）\n\
                            这是 Erg 编译器的一个错误，请报告它 (https://github.com/erg-lang/erg)\n\
                            起因于：{fn_name}"),
                "traditional_chinese" => format!("堆棧中的元素數無效（元素數：{stack_len}，塊id：{block_id}）\n\
                            這是 Erg 編譯器的一個錯誤，請報告它 (https://github.com/erg-lang/erg)\n\
                            起因於：{fn_name}"),
                    "english" => format!("the number of elements in the stack is invalid (num of elems: {stack_len}, block id: {block_id})\n\
                            this is a bug of the Erg compiler, please report it (https://github.com/erg-lang/erg)\n\
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
                    "japanese" => format!("これはErg compilerのバグです、開発者に報告して下さい (https://github.com/erg-lang/erg)\n{fn_name}:{line}より発生"),
                    "simplified_chinese" => format!("这是Erg编译器的错误，请报告给https://github.com/erg-lang/erg\n原因来自：{fn_name}:{line}"),
                    "traditional_chinese" => format!("這是Erg編譯器的錯誤，請報告給https://github.com/erg-lang/erg\n原因來自：{fn_name}:{line}"),
                    "english" => format!("this is a bug of the Erg compiler, please report it to https://github.com/erg-lang/erg\ncaused from: {fn_name}:{line}"),
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
        expect: &Type,
        found: &Type,
        candidates: Option<Set<Type>>,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{YELLOW}{name}{RESET}の型が違います。\n予期した型: {GREEN}{expect}{RESET}\n与えられた型: {RED}{found}{RESET}\n{}", fmt_option_map!(pre "与えられた型の単一化候補:\n", candidates, |x: &Set<Type>| x.folded_display())),
                    "simplified_chinese" => format!("{YELLOW}{name}{RESET}的类型不匹配：\n预期：{GREEN}{expect}{RESET}\n但找到：{RED}{found}{RESET}\n{}", fmt_option_map!(pre "某一类型的统一候选: \n", candidates, |x: &Set<Type>| x.folded_display())),
                    "traditional_chinese" => format!("{YELLOW}{name}{RESET}的類型不匹配：\n預期：{GREEN}{expect}{RESET}\n但找到：{RED}{found}{RESET}\n{}", fmt_option_map!(pre "某一類型的統一候選\n", candidates, |x: &Set<Type>| x.folded_display())),
                    "english" => format!("the type of {YELLOW}{name}{RESET} is mismatched:\nexpected:  {GREEN}{expect}{RESET}\nbut found: {RED}{found}{RESET}\n{}", fmt_option_map!(pre "unification candidates of a given type:\n", candidates, |x: &Set<Type>| x.folded_display())),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}の戻り値の型が違います。\n予期した型: {GREEN}{expect}{RESET}\n与えられた型: {RED}{found}{RESET}"),
                    "simplified_chinese" => format!("{name}的返回类型不匹配：\n预期：{GREEN}{expect}{RESET}\n但找到：{RED}{found}{RESET}"),
                    "traditional_chinese" => format!("{name}的返回類型不匹配：\n預期：{GREEN}{expect}{RESET}\n但找到：{RED}{found}{RESET}"),
                    "english" => format!("the return type of {name} is mismatched:\nexpected:  {GREEN}{expect}{RESET}\nbut found: {RED}{found}{RESET}"),
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
                    "simplified_chinese" => format!("{name}：{t}已声明但未初始化"),
                    "traditional_chinese" => format!("{name}：{t}已宣告但未初始化"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("ポジショナル引数の数が違います。\n予期した個数: {GREEN}{expect}{RESET}\n与えられた個数: {RED}{found}{RESET}"),
                    "simplified_chinese" => format!("正则参数的数量不匹配：\n预期：{GREEN}{expect}{RESET}\n但找到：{RED}{found}{RESET}"),
                    "traditional_chinese" => format!("正則參數的數量不匹配：\n預期：{GREEN}{expect}{RESET}\n但找到：{RED}{found}{RESET}"),
                    "english" => format!("the number of positional arguments is mismatched:\nexpected:  {GREEN}{expect}{RESET}\nbut found: {RED}{found}{RESET}"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!(
                        "{name}に渡された引数の数が多すぎます。
必要な引数の合計数: {GREEN}{params_len}{RESET}個
渡された引数の数:   {RED}{pos_args_len}{RESET}個
キーワード引数の数: {RED}{kw_args_len}{RESET}個"
                    ),
                    "simplified_chinese" => format!("传递给{name}的参数过多。
                    所需参数总数：{GREEN}{params_len}{RESET}
                    传递的参数数量：{RED}{pos_args_len}{RESET}
                    关键字参数的数量：{RED}{kw_args_len}{RESET}
                    "
                    ),
                    "traditional_chinese" => format!("傳遞給{name}的參數過多。
                    所需參數總數：{GREEN}{params_len}{RESET}
                    傳遞的參數數量：{RED}{pos_args_len}{RESET}
                    關鍵字參數的數量：{RED}{kw_args_len}{RESET}
                    "
                    ),
                    "english" => format!(
                        "too many arguments for {name}:
total expected params:  {GREEN}{params_len}{RESET}
passed positional args: {RED}{pos_args_len}{RESET}
passed keyword args:    {RED}{kw_args_len}{RESET}"
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}に渡された引数が{missing_len}個足りません({YELLOW}{}{RESET})。", fmt_vec(&missing_params)),
                    "simplified_chinese" => format!("{name}的{missing_len}个位置参数不被传递。({YELLOW}{}{RESET})。", fmt_vec(&missing_params)),
                    "traditional_chinese" => format!("{name}的{missing_len}個位置參數不被傳遞。({YELLOW}{}{RESET})。", fmt_vec(&missing_params)),
                    "english" => format!("missing {missing_len} positional argument(s) for {name}: {YELLOW}{}{RESET}", fmt_vec(&missing_params)),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}の引数{RED}{arg_name}{RESET}が複数回渡されています"),
                    "simplified_chinese" => format!("{name}的参数{RED}{arg_name}{RESET}被多次传递"),
                    "traditional_chinese" => format!("{name}的參數{RED}{arg_name}{RESET}被多次傳遞"),
                    "english" => format!("{name}'s argument {RED}{arg_name}{RESET} is passed multiple times"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}には予期しないキーワード引数{RED}{param_name}{RESET}が渡されています"),
                    "simplified_chinese" => format!("{name}得到了意外的关键字参数{RED}{param_name}{RESET}"),
                    "traditional_chinese" => format!("{name}得到了意外的關鍵字參數{RED}{param_name}{RESET}"),
                    "english" => format!("{name} got unexpected keyword argument {RED}{param_name}{RESET}"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("型の単一化に失敗しました:\n左辺: {YELLOW}{lhs_t}{RESET}\n右辺: {YELLOW}{rhs_t}{RESET}"),
                    "simplified_chinese" => format!("类型统一失败：\n左边：{YELLOW}{lhs_t}{RESET}\n右边：{YELLOW}{rhs_t}{RESET}"),
                    "traditional_chinese" => format!("類型統一失敗：\n左邊：{YELLOW}{lhs_t}{RESET}\n右邊：{YELLOW}{rhs_t}{RESET}"),
                    "english" => format!("unification failed:\nlhs: {YELLOW}{lhs_t}{RESET}\nrhs: {YELLOW}{rhs_t}{RESET}"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("型の再単一化に失敗しました:\n左辺: {YELLOW}{lhs_t}{RESET}\n右辺: {YELLOW}{rhs_t}{RESET}"),
                    "simplified_chinese" => format!("重新统一类型失败：\n左边：{YELLOW}{lhs_t}{RESET}\n右边：{YELLOW}{rhs_t}{RESET}"),
                    "traditional_chinese" => format!("重新統一類型失敗：\n左邊：{YELLOW}{lhs_t}{RESET}\n右邊：{YELLOW}{rhs_t}{RESET}"),
                    "english" => format!("re-unification failed:\nlhs: {YELLOW}{lhs_t}{RESET}\nrhs: {YELLOW}{rhs_t}{RESET}"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("この式の部分型制約を満たせません:\nサブタイプ: {YELLOW}{sub_t}{RESET}\nスーパータイプ: {YELLOW}{sup_t}{RESET}"),
                    "simplified_chinese" => format!("无法满足此表达式中的子类型约束：\n子类型：{YELLOW}{sub_t}{RESET}\n超类型：{YELLOW}{sup_t}{RESET}"),
                    "traditional_chinese" => format!("無法滿足此表達式中的子類型約束：\n子類型：{YELLOW}{sub_t}{RESET}\n超類型：{YELLOW}{sup_t}{RESET}"),
                    "english" => format!("the subtype constraint in this expression cannot be satisfied:\nsubtype: {YELLOW}{sub_t}{RESET}\nsupertype: {YELLOW}{sup_t}{RESET}"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                Location::Unknown,
                switch_lang!(
                    "japanese" => format!("述語式の単一化に失敗しました:\n左辺: {YELLOW}{lhs}{RESET}\n右辺: {YELLOW}{rhs}{RESET}"),
                    "simplified_chinese" => format!("无法统一谓词表达式：\n左边：{YELLOW}{lhs}{RESET}\n左边：{YELLOW}{rhs}{RESET}"),
                    "traditional_chinese" => format!("無法統一謂詞表達式：\n左邊：{YELLOW}{lhs}{RESET}\n左邊：{YELLOW}{rhs}{RESET}"),
                    "english" => format!("predicate unification failed:\nlhs: {YELLOW}{lhs}{RESET}\nrhs: {YELLOW}{rhs}{RESET}"),
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

    pub fn method_definition_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: AtomicStr,
        name: &str,
        hint: Option<AtomicStr>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                errno,
                MethodError,
                loc,
                switch_lang!(
                    "japanese" => format!(
                        "{RED}{name}{RESET}にメソッドを定義することはできません",
                    ),
                    "simplified_chinese" => format!(
                        "{RED}{name}{RESET}不可定义方法",
                    ),
                    "traditional_chinese" => format!(
                        "{RED}{name}{RESET}不可定義方法",
                    ),
                    "english" => format!(
                        "cannot define methods for {RED}{name}{RESET}",
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{YELLOW}{member_name}{RESET}の型が違います。\n{trait_type}で宣言された型: {GREEN}{expect}{RESET}\n与えられた型: {RED}{found}{RESET}"),
                    "simplified_chinese" => format!("{YELLOW}{member_name}{RESET}的类型不匹配：\n在{trait_type}中声明的类型：{GREEN}{expect}{RESET}\n但找到：{RED}{found}{RESET}"),
                    "traditional_chinese" => format!("{YELLOW}{member_name}{RESET}的類型不匹配：\n在{trait_type}中聲明的類型：{GREEN}{expect}{RESET}\n但找到：{RED}{found}{RESET}"),
                    "english" => format!("the type of {YELLOW}{member_name}{RESET} is mismatched:\ndeclared in {trait_type}: {GREEN}{expect}{RESET}\nbut found: {RED}{found}{RESET}"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                Location::Unknown,
                switch_lang!(
                    "japanese" => format!("{trait_type}の{YELLOW}{member_name}{RESET}が{class_type}で実装されていません"),
                    "simplified_chinese" => format!("{trait_type}中的{YELLOW}{member_name}{RESET}没有在{class_type}中实现"),
                    "traditional_chinese" => format!("{trait_type}中的{YELLOW}{member_name}{RESET}沒有在{class_type}中實現"),
                    "english" => format!("{YELLOW}{member_name}{RESET} of {trait_type} is not implemented in {class_type}"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                Location::Unknown,
                switch_lang!(
                    "japanese" => format!("{class_type}の{YELLOW}{member_name}{RESET}は{trait_type}で宣言されていません"),
                    "simplified_chinese" => format!("{class_type}中的{YELLOW}{member_name}{RESET}没有在{trait_type}中声明"),
                    "traditional_chinese" => format!("{class_type}中的{YELLOW}{member_name}{RESET}沒有在{trait_type}中聲明"),
                    "english" => format!("{YELLOW}{member_name}{RESET} of {class_type} is not declared in {trait_type}"),
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("型変数{RED}{name}{RESET}が定義されていません"),
                    "simplified_chinese" => format!("类型变量{RED}{name}{RESET}没有定义"),
                    "traditional_chinese" => format!("類型變量{RED}{name}{RESET}沒有定義"),
                    "english" => format!("type variable {RED}{name}{RESET} is not defined"),
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
                    "japanese" => format!("{expr}の型を一意に決定できませんでした\n候補: {}", fmt_vec(candidates)),
                    "simplified_chinese" => format!("无法确定{expr}的类型\n候选：{}", fmt_vec(candidates)),
                    "traditional_chinese" => format!("無法確定{expr}的類型\n候選：{}", fmt_vec(candidates)),
                    "english" => format!("cannot determine the type of {expr}\ncandidates: {}", fmt_vec(candidates)),
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
        Self::new(
            ErrorCore::new(
                errno,
                MoveError,
                name_loc,
                switch_lang!(
                    "japanese" => format!(
                        "{RED}{name}{RESET}は{}行目ですでに移動されています",
                        moved_loc.ln_begin().unwrap_or(0)
                    ),
                    "simplified_chinese" => format!(
                        "{RED}{name}{RESET}已移至第{}行",
                        moved_loc.ln_begin().unwrap_or(0)
                    ),
                    "traditional_chinese" => format!(
                        "{RED}{name}{RESET}已移至第{}行",
                        moved_loc.ln_begin().unwrap_or(0)
                    ),
                    "english" => format!(
                        "{RED}{name}{RESET} was moved in line {}",
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
        Self::new(
            ErrorCore::new(
                errno,
                TypeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{name}は{GREEN}{spec_t}{RESET}型として宣言されましたが、{RED}{found_t}{RESET}型のオブジェクトが代入されています"),
                    "simplified_chinese" => format!("{name}被声明为{GREEN}{spec_t}{RESET}，但分配了一个{RED}{found_t}{RESET}对象"),
                    "traditional_chinese" => format!("{name}被聲明為{GREEN}{spec_t}{RESET}，但分配了一個{RED}{found_t}{RESET}對象"),
                    "english" => format!("{name} was declared as {GREEN}{spec_t}{RESET}, but an {RED}{found_t}{RESET} object is assigned"),
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
            let n = readable_name(n);
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量：{n}"),
                "traditional_chinese" => format!("存在相同名稱變量：{n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
            .into()
        });
        Self::new(
            ErrorCore::new(
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
            let n = readable_name(n);
            switch_lang!(
                "japanese" => format!("似た名前の属性があります: {n}"),
                "simplified_chinese" => format!("具有相同名称的属性：{n}"),
                "traditional_chinese" => format!("具有相同名稱的屬性：{n}"),
                "english" => format!("has a similar name attribute: {n}"),
            )
            .into()
        });
        Self::new(
            ErrorCore::new(
                errno,
                AttributeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{obj_t}型オブジェクトに{RED}{name}{RESET}という属性はありません"),
                    "simplified_chinese" => format!("{obj_t}对象没有属性{RED}{name}{RESET}"),
                    "traditional_chinese" => format!("{obj_t}對像沒有屬性{RED}{name}{RESET}"),
                    "english" => format!("{obj_t} object has no attribute {RED}{name}{RESET}"),
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
            let n = readable_name(n);
            switch_lang!(
                "japanese" => format!("似た名前の属性があります: {n}"),
                "simplified_chinese" => format!("具有相同名称的属性：{n}"),
                "traditional_chinese" => format!("具有相同名稱的屬性：{n}"),
                "english" => format!("has a similar name attribute: {n}"),
            )
            .into()
        });
        Self::new(
            ErrorCore::new(
                errno,
                AttributeError,
                loc,
                switch_lang!(
                    "japanese" => format!("{obj_name}(: {obj_t})に{RED}{name}{RESET}という属性はありません"),
                    "simplified_chinese" => format!("{obj_name}(: {obj_t})没有属性{RED}{name}{RESET}"),
                    "traditional_chinese" => format!("{obj_name}(: {obj_t})沒有屬性{RED}{name}{RESET}"),
                    "english" => format!("{obj_name}(: {obj_t}) has no attribute {RED}{name}{RESET}"),
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
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                errno,
                AssignError,
                loc,
                switch_lang!(
                    "japanese" => format!("変数{YELLOW}{name}{RESET}に再代入されています"),
                    "simplified_chinese" => format!("不能为变量{YELLOW}{name}{RESET}分配两次"),
                    "traditional_chinese" => format!("不能為變量{YELLOW}{name}{RESET}分配兩次"),
                    "english" => format!("cannot assign twice to the variable {YELLOW}{name}{RESET}"),
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
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                errno,
                UnusedWarning,
                loc,
                switch_lang!(
                    "japanese" => format!("{YELLOW}{name}{RESET}は使用されていません"),
                    "simplified_chinese" => format!("{YELLOW}{name}{RESET}未使用"),
                    "traditional_chinese" => format!("{YELLOW}{name}{RESET}未使用"),
                    "english" => format!("{YELLOW}{name}{RESET} is not used"),
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
        Self::new(
            ErrorCore::new(
                errno,
                VisibilityError,
                loc,
                switch_lang!(
                    "japanese" => format!("{RED}{name}{RESET}は{visibility}変数です"),
                    "simplified_chinese" => format!("{RED}{name}{RESET}是{visibility}变量",),
                    "traditional_chinese" => format!("{RED}{name}{RESET}是{visibility}變量",),
                    "english" => format!("{RED}{name}{RESET} is {visibility} variable",),
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
        Self::new(
            ErrorCore::new(
                errno,
                NameError,
                name_loc,
                switch_lang!(
                    "japanese" => format!(
                        "{RED}{name}{RESET}は{YELLOW}{superclass}{RESET}で既に定義されています",
                    ),
                    "simplified_chinese" => format!(
                        "{RED}{name}{RESET}已在{YELLOW}{superclass}{RESET}中定义",
                    ),
                    "traditional_chinese" => format!(
                        "{RED}{name}{RESET}已在{YELLOW}{superclass}{RESET}中定義",
                    ),
                    "english" => format!(
                        "{RED}{name}{RESET} is already defined in {YELLOW}{superclass}{RESET}",
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
            (Some(erg), Some(py)) => Some(format!(
                "similar name erg module {YELLOW}{erg}{RESET} and python module {YELLOW}{py}{RESET} exists (to import python modules, use `pyimport`)",
            )),
            (Some(erg), None) => Some(format!("similar name erg module exists: {YELLOW}{erg}{RESET}")),
            (None, Some(py)) => Some(format!("similar name python module exists: {YELLOW}{py}{RESET} (to import python modules, use `pyimport`)")),
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
