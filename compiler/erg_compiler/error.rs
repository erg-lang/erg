use std::fmt::Display;
use std::ops::Add;

use erg_common::color::{GREEN, RED, RESET, YELLOW};
use erg_common::config::Input;
use erg_common::error::{ErrorCore, ErrorDisplay, ErrorKind::*, Location, MultiErrorDisplay};
use erg_common::traits::{Locational, Stream};
use erg_common::ty::{Predicate, Type};
use erg_common::{fmt_iter, Str};
use erg_common::{impl_stream_for_wrapper, switch_lang};

use erg_parser::error::{ParserRunnerError, ParserRunnerErrors};

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
    pub caused_by: Str,
}

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
    fn ref_inner(&self) -> Option<&Box<Self>> {
        None
    }
}

impl CompileError {
    pub const fn new(core: ErrorCore, input: Input, caused_by: Str) -> Self {
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
        Self::new(ErrorCore::new(errno, CompilerSystemError, loc, switch_lang!(
            format!("this is a bug of the Erg compiler, please report it to https://github.com/...\ncaused from: {fn_name}:{line}"),
            format!("これはErg compilerのバグです、開発者に報告して下さい (https://github.com/...)\n{fn_name}:{line}より発生")
        ), None), input, "".into())
    }

    pub fn stack_bug(
        input: Input,
        loc: Location,
        stack_len: u32,
        block_id: usize,
        fn_name: &str,
    ) -> Self {
        Self::new(ErrorCore::new(0, CompilerSystemError, loc, switch_lang!(
            format!("the number of elements in the stack is invalid (num of elems: {stack_len}, block id: {block_id})\n\
                    this is a bug of the Erg compiler, please report it (https://github.com/...)\n\
                    caused from: {fn_name}"),
            format!("スタックの要素数が異常です (要素数: {stack_len}, ブロックID: {block_id})\n\
                    これはコンパイラのバグです、開発者に報告して下さい (https://github.com/...)\n\
                    {fn_name}より発生")
        ), None), input, "".into())
    }

    pub fn feature_error(input: Input, loc: Location, name: &str, caused_by: Str) -> Self {
        Self::new(
            ErrorCore::new(
                0,
                FeatureError,
                loc,
                switch_lang!(
                    format!("this feature({name}) is not implemented yet"),
                    format!("この機能({name})はまだ正式に提供されていません")
                ),
                None,
            ),
            input,
            caused_by,
        )
    }
}

#[derive(Debug)]
pub struct TyCheckError {
    pub core: ErrorCore,
    pub caused_by: Str,
}

impl TyCheckError {
    pub const fn new(core: ErrorCore, caused_by: Str) -> Self {
        Self { core, caused_by }
    }

    pub fn unreachable(fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), "".into())
    }

    pub fn checker_bug(errno: usize, loc: Location, fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::new(errno, CompilerSystemError, loc, switch_lang!(
            format!("this is a bug of the Erg compiler, please report it to https://github.com/...\ncaused from: {fn_name}:{line}"),
            format!("これはErg compilerのバグです、開発者に報告して下さい (https://github.com/...)\n{fn_name}:{line}より発生")
        ), None), "".into())
    }

    pub fn feature_error(loc: Location, name: &str, caused_by: Str) -> Self {
        Self::new(
            ErrorCore::new(
                0,
                FeatureError,
                loc,
                switch_lang!(
                    format!("this feature({name}) is not implemented yet"),
                    format!("この機能({name})はまだ正式に提供されていません")
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn syntax_error<S: Into<Str>>(
        errno: usize,
        loc: Location,
        caused_by: Str,
        desc: S,
        hint: Option<Str>,
    ) -> Self {
        Self::new(
            ErrorCore::new(errno, SyntaxError, loc, desc, hint),
            caused_by,
        )
    }

    pub fn duplicate_decl_error(loc: Location, caused_by: Str, name: &str) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                0,
                NameError,
                loc,
                switch_lang!(
                    format!("{name} is already declared"),
                    format!("{name}は既に宣言されています")
                ),
                Option::<Str>::None,
            ),
            caused_by,
        )
    }

    pub fn violate_decl_error(
        loc: Location,
        caused_by: Str,
        name: &str,
        spec_t: &Type,
        found_t: &Type,
    ) -> Self {
        let name = readable_name(name);
        Self::new(ErrorCore::new(0, TypeError, loc,
            switch_lang!(
                format!("{name} was declared as {GREEN}{spec_t}{RESET}, but an {RED}{found_t}{RESET} object is assigned"),
                format!("{name}は{GREEN}{spec_t}{RESET}型として宣言されましたが、{RED}{found_t}{RESET}型のオブジェクトが代入されています")
            ), Option::<Str>::None),
            caused_by
        )
    }

    pub fn no_type_spec_error(loc: Location, caused_by: Str, name: &str) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                0,
                TypeError,
                loc,
                switch_lang!(
                    format!("the type of {name} is not specified"),
                    format!("{name}の型が指定されていません")
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn no_var_error(
        loc: Location,
        caused_by: Str,
        name: &str,
        similar_name: Option<&Str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = readable_name(n);
            switch_lang!(
                format!("exists a similar name variable: {n}"),
                format!("似た名前の変数があります: {n}")
            )
            .into()
        });
        Self::new(
            ErrorCore::new(
                0,
                NameError,
                loc,
                switch_lang!(
                    format!("{RED}{name}{RESET} is not defined"),
                    format!("{RED}{name}{RESET}という変数は定義されていません")
                ),
                hint,
            ),
            caused_by,
        )
    }

    pub fn no_attr_error(
        loc: Location,
        caused_by: Str,
        obj_t: &Type,
        name: &str,
        similar_name: Option<&Str>,
    ) -> Self {
        let hint = similar_name.map(|n| {
            let n = readable_name(n);
            switch_lang!(
                format!("has a similar name attribute: {n}"),
                format!("似た名前の属性があります: {n}")
            )
            .into()
        });
        Self::new(
            ErrorCore::new(
                0,
                AttributeError,
                loc,
                switch_lang!(
                    format!("{obj_t} object has no attribute {RED}{name}{RESET}"),
                    format!("{obj_t}型オブジェクトに{RED}{name}{RESET}という属性はありません")
                ),
                hint,
            ),
            caused_by,
        )
    }

    pub fn callable_impl_error<'a, C: Locational + Display>(
        callee: &C,
        param_ts: impl Iterator<Item = &'a Type>,
        caused_by: Str,
    ) -> Self {
        let param_ts = fmt_iter(param_ts);
        Self::new(
            ErrorCore::new(
                0,
                NotImplementedError,
                callee.loc(),
                switch_lang!(
                    format!(
                        "{callee} is not a Callable object that takes {param_ts} as an argument"
                    ),
                    format!(
                        "{callee}は{param_ts}を引数に取る呼び出し可能オブジェクトではありません"
                    )
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn type_mismatch_error(
        loc: Location,
        caused_by: Str,
        name: &str,
        expect: &Type,
        found: &Type,
    ) -> Self {
        Self::new(ErrorCore::new(0, TypeError, loc, switch_lang!(
            format!("the type of {name} is mismatched:\nexpected:  {GREEN}{expect}{RESET}\nbut found: {RED}{found}{RESET}"),
            format!("{name}の型が違います。\n予期した型: {GREEN}{expect}{RESET}\n与えられた型: {RED}{found}{RESET}")
        ), None), caused_by)
    }

    pub fn return_type_error(
        loc: Location,
        caused_by: Str,
        name: &str,
        expect: &Type,
        found: &Type,
    ) -> Self {
        Self::new(ErrorCore::new(0, TypeError, loc, switch_lang!(
            format!("the return type of {name} is mismatched:\nexpected:  {GREEN}{expect}{RESET}\nbut found: {RED}{found}{RESET}"),
            format!("{name}の戻り値の型が違います。\n予期した型: {GREEN}{expect}{RESET}\n与えられた型: {RED}{found}{RESET}")
        ), None), caused_by)
    }

    pub fn uninitialized_error(loc: Location, caused_by: Str, name: &str, t: &Type) -> Self {
        Self::new(
            ErrorCore::new(
                0,
                NameError,
                loc,
                switch_lang!(
                    format!("{name}: {t} is not initialized"),
                    format!("{name}: {t}は初期化されていません")
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn argument_error(loc: Location, caused_by: Str, expect: usize, found: usize) -> Self {
        Self::new(ErrorCore::new(0, TypeError, loc, switch_lang!(
            format!("the number of positional arguments is mismatched:\nexpected:  {GREEN}{expect}{RESET}\nbut found: {RED}{found}{RESET}"),
            format!("ポジショナル引数の数が違います。\n予期した個数: {GREEN}{expect}{RESET}\n与えられた個数: {RED}{found}{RESET}")
        ), None), caused_by)
    }

    pub fn match_error(loc: Location, caused_by: Str, expr_t: &Type) -> Self {
        Self::new(
            ErrorCore::new(
                0,
                TypeError,
                loc,
                switch_lang!(
                    format!("not all patterns of type {expr_t} are covered"),
                    format!("{expr_t}型の全パターンを網羅していません")
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn infer_error(loc: Location, caused_by: Str, expr: &str) -> Self {
        Self::new(
            ErrorCore::new(
                0,
                TypeError,
                loc,
                switch_lang!(
                    format!("failed to infer the type of {expr}"),
                    format!("{expr}の型が推論できません")
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn dummy_infer_error(fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), "".into())
    }

    pub fn not_relation(fn_name: &str, line: u32) -> Self {
        Self::new(ErrorCore::unreachable(fn_name, line), "".into())
    }


    pub fn reassign_error(loc: Location, caused_by: Str, name: &str) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                0,
                AssignError,
                loc,
                switch_lang!(
                    format!("cannot assign twice to the immutable variable {name}"),
                    format!("定数{name}には再代入できません")
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn too_many_args_error(
        loc: Location,
        callee_name: &str,
        caused_by: Str,
        params_len: usize,
        pos_args_len: usize,
        kw_args_len: usize,
    ) -> Self {
        let name = readable_name(callee_name);
        Self::new(
            ErrorCore::new(
                0,
                TypeError,
                loc,
                switch_lang!(
                    format!(
                        "too many arguments for {name}:
total expected params:  {GREEN}{params_len}{RESET}
passed positional args: {RED}{pos_args_len}{RESET}
passed keyword args:    {RED}{kw_args_len}{RESET}"
                    ),
                    format!(
                        "{name}に渡された引数の数が多すぎます。
必要な引数の合計数: {GREEN}{params_len}{RESET}個
渡された引数の数:   {RED}{pos_args_len}{RESET}個
キーワード引数の数: {RED}{kw_args_len}{RESET}個"
                    )
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn multiple_args_error(
        loc: Location,
        callee_name: &str,
        caused_by: Str,
        arg_name: &str,
    ) -> Self {
        let name = readable_name(callee_name);
        Self::new(
            ErrorCore::new(
                0,
                TypeError,
                loc,
                switch_lang!(
                    format!("{name}'s argument {RED}{arg_name}{RESET} is passed multiple times"),
                    format!("{name}の引数{RED}{arg_name}{RESET}が複数回渡されています")
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn unexpected_kw_arg_error(
        loc: Location,
        callee_name: &str,
        caused_by: Str,
        param_name: &str,
    ) -> Self {
        let name = readable_name(callee_name);
        Self::new(
            ErrorCore::new(
                0,
                TypeError,
                loc,
                switch_lang!(
            format!("{name} got unexpected keyword argument {RED}{param_name}{RESET}"),
            format!("{name}には予期しないキーワード引数{RED}{param_name}{RESET}が渡されています")
        ),
                None,
            ),
            caused_by,
        )
    }

    pub fn unused_warning(loc: Location, name: &str, caused_by: Str) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                0,
                UnusedWarning,
                loc,
                switch_lang!(
                    format!("{YELLOW}{name}{RESET} is not used"),
                    format!("{YELLOW}{name}{RESET}は使用されていません")
                ),
                None,
            ),
            caused_by,
        )
    }

    pub fn unification_error(
        lhs_t: &Type,
        rhs_t: &Type,
        lhs_loc: Option<Location>,
        rhs_loc: Option<Location>,
        caused_by: Str,
    ) -> Self {
        let loc = match (lhs_loc, rhs_loc) {
            (Some(l), Some(r)) => Location::pair(l, r),
            (Some(l), None) => l,
            (None, Some(r)) => r,
            (None, None) => Location::Unknown,
        };
        Self::new(ErrorCore::new(0, TypeError, loc, switch_lang!(
            format!("unification failed:\nlhs: {YELLOW}{lhs_t}{RESET}\nrhs: {YELLOW}{rhs_t}{RESET}"),
            format!("型の単一化に失敗しました:\n左辺: {YELLOW}{lhs_t}{RESET}\n右辺: {YELLOW}{rhs_t}{RESET}")
        ), None), caused_by)
    }

    pub fn re_unification_error(
        lhs_t: &Type,
        rhs_t: &Type,
        lhs_loc: Option<Location>,
        rhs_loc: Option<Location>,
        caused_by: Str,
    ) -> Self {
        let loc = match (lhs_loc, rhs_loc) {
            (Some(l), Some(r)) => Location::pair(l, r),
            (Some(l), None) => l,
            (None, Some(r)) => r,
            (None, None) => Location::Unknown,
        };
        Self::new(ErrorCore::new(0, TypeError, loc, switch_lang!(
            format!("re-unification failed:\nlhs: {YELLOW}{lhs_t}{RESET}\nrhs: {YELLOW}{rhs_t}{RESET}"),
            format!("型の再単一化に失敗しました:\n左辺: {YELLOW}{lhs_t}{RESET}\n右辺: {YELLOW}{rhs_t}{RESET}")
        ), None), caused_by)
    }

    pub fn subtyping_error(
        sub_t: &Type,
        sup_t: &Type,
        sub_loc: Option<Location>,
        sup_loc: Option<Location>,
        caused_by: Str,
    ) -> Self {
        let loc = match (sub_loc, sup_loc) {
            (Some(l), Some(r)) => Location::pair(l, r),
            (Some(l), None) => l,
            (None, Some(r)) => r,
            (None, None) => Location::Unknown,
        };
        Self::new(ErrorCore::new(0, TypeError, loc, switch_lang!(
            format!("subtype constraints cannot be satisfied:\nsubtype: {YELLOW}{sub_t}{RESET}\nsupertype: {YELLOW}{sup_t}{RESET}"),
            format!("部分型制約を満たせません:\nサブタイプ: {YELLOW}{sub_t}{RESET}\nスーパータイプ: {YELLOW}{sup_t}{RESET}")
        ), None), caused_by)
    }

    pub fn pred_unification_error(lhs: &Predicate, rhs: &Predicate, caused_by: Str) -> Self {
        Self::new(ErrorCore::new(0, TypeError, Location::Unknown, switch_lang!(
            format!("predicate unification failed:\nlhs: {YELLOW}{lhs}{RESET}\nrhs: {YELLOW}{rhs}{RESET}"),
            format!("述語式の単一化に失敗しました:\n左辺: {YELLOW}{lhs}{RESET}\n右辺: {YELLOW}{rhs}{RESET}")
        ), None), caused_by)
    }

    pub fn has_effect<S: Into<Str>>(expr: &Expr, caused_by: S) -> Self {
        Self::new(
            ErrorCore::new(
                0,
                HasEffect,
                expr.loc(),
                switch_lang!(
                    format!("this expression causes a side-effect"),
                    format!("この式には副作用があります")
                ),
                None,
            ),
            caused_by.into(),
        )
    }

    pub fn move_error<S: Into<Str>>(
        name: &str,
        name_loc: Location,
        moved_loc: Location,
        caused_by: S,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                0,
                MoveError,
                name_loc,
                switch_lang!(
                    format!(
                        "{RED}{name}{RESET} was moved in line {}",
                        moved_loc.ln_begin().unwrap()
                    ),
                    format!(
                        "{RED}{name}{RESET}は{}行目ですでに移動されています",
                        moved_loc.ln_begin().unwrap()
                    )
                ),
                None,
            ),
            caused_by.into(),
        )
    }
}

#[derive(Debug)]
pub struct TyCheckErrors(Vec<TyCheckError>);

impl_stream_for_wrapper!(TyCheckErrors, TyCheckError);

impl From<Vec<TyCheckError>> for TyCheckErrors {
    fn from(errs: Vec<TyCheckError>) -> Self {
        Self(errs)
    }
}

impl Add for TyCheckErrors {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0.into_iter().chain(other.0.into_iter()).collect())
    }
}

impl From<TyCheckError> for TyCheckErrors {
    fn from(err: TyCheckError) -> Self {
        Self(vec![err])
    }
}

pub type TyCheckResult<T> = Result<T, TyCheckError>;
pub type TyCheckWarning = TyCheckError;
pub type TyCheckWarnings = TyCheckErrors;

pub type EvalError = TyCheckError;
pub type EvalErrors = TyCheckErrors;
pub type EvalResult<T> = TyCheckResult<T>;

pub type EffectError = TyCheckError;
pub type EffectErrors = TyCheckErrors;
pub type EffectResult<T> = Result<T, EffectErrors>;

pub type OwnershipError = TyCheckError;
pub type OwnershipErrors = TyCheckErrors;
pub type OwnershipResult<T> = Result<T, OwnershipErrors>;

pub type LowerError = TyCheckError;
pub type LowerWarning = LowerError;
pub type LowerErrors = TyCheckErrors;
pub type LowerWarnings = LowerErrors;
pub type LowerResult<T> = TyCheckResult<T>;

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

pub type CompileResult<T> = Result<T, CompileError>;
pub type CompileWarning = CompileError;
pub type CompileWarnings = CompileErrors;
