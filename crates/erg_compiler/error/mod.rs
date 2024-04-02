pub mod eval;
pub mod lower;
pub mod tycheck;

use std::fmt;

use erg_common::error::{
    ErrorCore, ErrorDisplay, ErrorKind::*, Location, MultiErrorDisplay, SubMessage,
};
use erg_common::io::Input;
use erg_common::style::{Attribute, Color, StyledStr, StyledString, StyledStrings, Theme, THEME};
use erg_common::traits::{Locational, NoTypeDisplay, Stream};
use erg_common::{impl_display_and_error, impl_stream, switch_lang};

use erg_parser::error::{ParseError, ParseErrors, ParserRunnerError, ParserRunnerErrors};

pub use crate::error::eval::*;
pub use crate::error::lower::*;
pub use crate::error::tycheck::*;
use crate::hir::Expr;
use crate::ty::HasType;

pub(crate) fn concat_result(l: CompileResult<()>, r: CompileResult<()>) -> CompileResult<()> {
    match (l, r) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(r)) => Err(r),
        (Err(l), Ok(())) => Err(l),
        (Err(mut l), Err(mut r)) => {
            l.0.append(&mut r.0);
            Err(l)
        }
    }
}

/// `unreachable!(self: Context)`
#[macro_export]
macro_rules! unreachable_error {
    ($Strcs: ident, $Strc: ident, $ctx: expr) => {
        Err($Strcs::from($Strc::unreachable(
            $ctx.cfg.input.clone(),
            $crate::erg_common::fn_name!(),
            line!(),
        )))
    };
    ($Strc: ident, $ctx: expr) => {
        Err($Strc::unreachable(
            $ctx.cfg.input.clone(),
            $crate::erg_common::fn_name!(),
            line!(),
        ))
    };
}
/// `feature_error!($Strc: struct, ctx: Context, loc: Location, name: &str)`
#[macro_export]
macro_rules! feature_error {
    ($Strcs: ident, $Strc: ident, $ctx: expr, $loc: expr, $name: expr) => {
        Err($Strcs::from($Strc::feature_error(
            $ctx.cfg.input.clone(),
            line!() as usize,
            $loc,
            $name,
            $ctx.caused_by(),
        )))
    };
    ($Strc: ident, $ctx: expr, $loc: expr, $name: expr) => {
        Err($Strc::feature_error(
            $ctx.cfg.input.clone(),
            $loc,
            $name,
            $ctx.caused_by(),
        ))
    };
}
#[macro_export]
macro_rules! type_feature_error {
    ($ctx: expr, $loc: expr, $name: expr) => {
        $crate::feature_error!(TyCheckErrors, TyCheckError, $ctx, $loc, $name)
    };
    (error $ctx: expr, $loc: expr, $name: expr) => {
        $crate::feature_error!(TyCheckError, $ctx, $loc, $name)
    };
}

pub fn ordinal_num(n: usize) -> String {
    match n.to_string().chars().last().unwrap() {
        '1' => format!("{n}st"),
        '2' => format!("{n}nd"),
        '3' => format!("{n}rd"),
        _ => format!("{n}th"),
    }
}

/// dname is for "double under name"
pub fn binop_to_dname(op: &str) -> Option<&str> {
    match op {
        "+" => Some("__add__"),
        "-" => Some("__sub__"),
        "*" => Some("__mul__"),
        "/" => Some("__div__"),
        "//" => Some("__floordiv__"),
        "**" => Some("__pow__"),
        "%" => Some("__mod__"),
        "@" => Some("__matmul__"),
        ".." => Some("__rng__"),
        "<.." => Some("__lorng__"),
        "..<" => Some("__rorng__"),
        "<..<" => Some("__orng__"),
        "&&" | "&" | "and" => Some("__and__"),
        "||" | "|" | "or" => Some("__or__"),
        "^^" | "^" => Some("__xor__"),
        "as" => Some("__as__"),
        // l in r, l notin r -> r contains l, not(r contains l)
        "contains" => Some("__contains__"),
        "subof" => Some("__subof__"),
        "supof" => Some("__supof__"),
        "is!" => Some("__is__!"),
        "isnot!" => Some("__isnot__!"),
        "==" => Some("__eq__"),
        "!=" => Some("__ne__"),
        "<" => Some("__lt__"),
        "<=" => Some("__le__"),
        ">" => Some("__gt__"),
        ">=" => Some("__ge__"),
        "<<" => Some("__lshift__"),
        ">>" => Some("__rshift__"),
        _ => None,
    }
}

pub fn unaryop_to_dname(op: &str) -> Option<&str> {
    match op {
        "+" => Some("__pos__"),
        "-" => Some("__neg__"),
        "~" => Some("__invert__"),
        "!" => Some("__mutate__"),
        "..." => Some("__spread__"),
        _ => None,
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
        "__matmul__" => "`@`",
        "__rng__" => "`..`",
        "__lorng__" => "`<..`",
        "__rorng__" => "`..<`",
        "__orng__" => "`<..<`",
        "__as__" => "`as`",
        "__and__" => "`and`", // TODO: `&&` if not boolean
        "__or__" => "`or`",   // TODO: `||` if not boolean
        "__contains__" => "`contains`",
        "__subof__" => "`subof`",
        "__supof__" => "`supof`",
        "__is__!" => "`is!`",
        "__isnot__!" => "`isnot!`",
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
        "__lshift__" => "`<<`",
        "__rshift__" => "`>>`",
        other => other,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompileError {
    pub core: Box<ErrorCore>, // ErrorCore is large, so box it
    pub input: Input,
    pub caused_by: String,
    pub theme: Theme,
}

impl_display_and_error!(CompileError);

impl From<std::io::Error> for CompileError {
    fn from(value: std::io::Error) -> Self {
        Self {
            core: Box::new(ErrorCore::new(
                vec![SubMessage::only_loc(Location::Unknown)],
                value.to_string(),
                0,
                IoError,
                Location::Unknown,
            )),
            input: Input::str("".into()),
            caused_by: "".to_owned(),
            theme: THEME,
        }
    }
}

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

impl From<CompileError> for ParserRunnerErrors {
    fn from(err: CompileError) -> Self {
        Self::new(vec![err.into()])
    }
}

impl From<CompileError> for ParserRunnerError {
    fn from(err: CompileError) -> Self {
        Self {
            core: *err.core,
            input: err.input,
        }
    }
}

#[cfg(feature = "pylib")]
impl std::convert::From<CompileError> for pyo3::PyErr {
    fn from(err: CompileError) -> pyo3::PyErr {
        pyo3::exceptions::PyOSError::new_err(err.to_string())
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

    pub fn feature_error(
        input: Input,
        errno: usize,
        loc: Location,
        name: &str,
        caused_by: String,
    ) -> Self {
        Self::new(
            ErrorCore::new(
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
            Input::dummy(),
            "".to_owned(),
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

    pub fn constructor_destructor_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => "このオブジェクトのコンストラクタとデストラクタは副作用があるため，関数内で呼び出すことはできません",
                    "simplified_chinese" => "此对象的构造函数和析构函数有副作用，因此不能在函数内调用",
                    "traditional_chinese" => "此對象的構造函數和析構函數有副作用，因此不能在函數內調用",
                    "english" => "the constructor and destructor of this object have side-effects, so they cannot be called inside a function",
                ),
                errno,
                HasEffect,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn proc_assign_error(input: Input, errno: usize, loc: Location, caused_by: String) -> Self {
        let hint = Some(
            switch_lang!(
                "japanese" => {
                let mut s = StyledStrings::default();
                s.push_str("変数の末尾に");
                s.push_str_with_color_and_attr("!", WARN, ATTR);
                s.push_str("をつけてください");
                s
                },
                "simplified_chinese" => {
                let mut s = StyledStrings::default();
                s.push_str("请在变量名后加上");
                s.push_str_with_color_and_attr("!", WARN, ATTR);
                s
                },
                "traditional_chinese" => {
                let mut s = StyledStrings::default();
                s.push_str("請在變量名後加上");
                s.push_str_with_color_and_attr("!", WARN, ATTR);
                s
                },
                "english" => {

                let mut s = StyledStrings::default();
                s.push_str("add ");
                s.push_str_with_color_and_attr("!", WARN, ATTR);
                s.push_str(" to the end of the variable name");
                s
                },
            )
            .to_string(),
        );
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => "プロシージャを通常の変数に代入することはできません",
                    "simplified_chinese" => "不能将过程赋值给普通变量",
                    "traditional_chinese" => "不能將過程賦值給普通變量",
                    "english" => "cannot assign a procedure to a normal variable",
                ),
                errno,
                HasEffect,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn touch_mut_error(input: Input, errno: usize, expr: &Expr, caused_by: String) -> Self {
        let (hint, def_loc) = match expr {
            Expr::Accessor(acc)
                if acc.root_obj().map_or(false, |obj| {
                    obj.var_info().is_some_and(|vi| vi.is_parameter()) && !obj.ref_t().is_ref()
                }) =>
            {
                let def_loc = acc.root_obj().unwrap().var_info().unwrap().def_loc.loc;
                let msg = format!(
                    "add `ref` in the parameter definition of `{}`",
                    acc.root_obj().unwrap().to_string_notype()
                );
                (Some(msg), Some(def_loc))
            }
            _ => (None, None),
        };
        let fst =
            SubMessage::ambiguous_new(expr.loc(), vec!["invalid access here".to_string()], None);
        let sub = if let Some(def_loc) = def_loc {
            vec![fst, SubMessage::ambiguous_new(def_loc, vec![], hint)]
        } else {
            vec![fst]
        };
        Self::new(
            ErrorCore::new(
                sub,
                switch_lang!(
                    "japanese" => format!("関数中で可変オブジェクト(: {})にアクセスすることは出来ません", expr.ref_t()),
                    "simplified_chinese" => format!("函数中不能访问可变对象(: {})", expr.ref_t()),
                    "traditional_chinese" => format!("函數中不能訪問可變對象(: {})", expr.ref_t()),
                    "english" => format!("cannot access a mutable object (: {}) in a function", expr.ref_t()),
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

pub type OwnershipError = CompileError;
pub type OwnershipErrors = CompileErrors;

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

#[derive(Debug, Clone)]
pub struct CompileErrors(Vec<CompileError>);

impl std::error::Error for CompileErrors {}

impl_stream!(CompileErrors, CompileError);

impl From<std::io::Error> for CompileErrors {
    fn from(value: std::io::Error) -> Self {
        Self::from(vec![value.into()])
    }
}

impl From<ParserRunnerErrors> for CompileErrors {
    fn from(err: ParserRunnerErrors) -> Self {
        Self(err.into_iter().map(CompileError::from).collect())
    }
}

impl From<CompileErrors> for ParserRunnerErrors {
    fn from(err: CompileErrors) -> Self {
        Self::new(err.into_iter().map(|e| e.into()).collect())
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

impl From<CompileError> for ParseError {
    fn from(err: CompileError) -> Self {
        Self::new(*err.core)
    }
}

impl From<CompileErrors> for ParseErrors {
    fn from(err: CompileErrors) -> Self {
        Self::new(err.into_iter().map(ParseError::from).collect())
    }
}

#[cfg(feature = "pylib")]
impl std::convert::From<CompileErrors> for pyo3::PyErr {
    fn from(errs: CompileErrors) -> pyo3::PyErr {
        pyo3::exceptions::PyOSError::new_err(format!("{} errors occurred\n{errs}", errs.len()))
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

    pub fn take(&mut self) -> Self {
        self.flush()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub type SingleCompileResult<T> = Result<T, CompileError>;
pub type CompileResult<T> = Result<T, CompileErrors>;
pub type CompileWarning = CompileError;
pub type CompileWarnings = CompileErrors;

#[cfg(test)]
mod test {
    use crate::{
        context::ContextKind,
        error::*,
        hir::Identifier,
        ty::{Predicate, Type},
        varinfo::{AbsLocation, VarInfo},
    };
    use erg_common::{error::Location, io::Input};
    use erg_parser::ast::{VarName, VisModifierSpec};

    // These Erg codes are not correct grammar.
    // This test make sure sub_msg and hint are displayed correctly.
    #[test]
    fn default_error_format_confirmation() {
        let mut errors = Vec::new();

        let input = Input::pipe("stack bug error".to_owned());
        let loc = Location::Line(1);
        let err = CompileError::stack_bug(input, loc, 0, 0, "FileName");
        errors.push(err);

        let input = Input::pipe("checker bug error".to_owned());
        let errno = 0;
        let err = TyCheckError::checker_bug(input, errno, Location::Unknown, "name", 1);
        errors.push(err);

        let loc = Location::LineRange(1, 3);
        let input = Input::pipe("args\nmissing\nerror".to_string());
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
        let input = Input::pipe("return type error".to_string());
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
        let input = Input::pipe("type mismatch error".to_string());
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

        let input = Input::pipe(
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

        let input = Input::pipe("argument error".to_string());
        let loc = Location::range(1, 0, 1, 8);
        let err = TyCheckError::argument_error(input, errno, loc, caused_by.to_string(), 1, 2);
        errors.push(err);

        let input = Input::pipe("Nat <: Int <: Ratio".to_string());
        let loc = Location::range(1, 0, 1, 10);
        let sub_t = &Type::Nat;
        let sup_t = &Type::Int;
        let err =
            TyCheckError::subtyping_error(input, errno, sub_t, sup_t, loc, caused_by.to_string());
        errors.push(err);

        let input = Input::pipe("pred unification error".to_string());
        let lhs = &Predicate::Const("Str".into());
        let rhs = &Predicate::Const("Nat".into());
        let err = TyCheckError::pred_unification_error(
            input,
            errno,
            lhs,
            rhs,
            Location::Unknown,
            caused_by.to_string(),
        );
        errors.push(err);

        let input = Input::pipe("Trait member type error".to_string());
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

        let input = Input::pipe("trait member not defined error".to_string());
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
            Location::Unknown,
        );
        errors.push(err);

        let input = Input::pipe("singular no attribute error".to_string());
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

        let input = Input::pipe("ambiguous type error".to_string());
        let raw = erg_parser::ast::Identifier::new(
            VisModifierSpec::Private,
            VarName::from_str("variable_name".into()),
        );
        let expr = Identifier::new(
            raw,
            None,
            VarInfo::new(
                Type::Nat,
                crate::varinfo::Mutability::Const,
                crate::ty::Visibility::DUMMY_PRIVATE,
                crate::varinfo::VarKind::Builtin,
                None,
                ContextKind::Dummy,
                None,
                AbsLocation::unknown(),
            ),
        );
        let candidates = &[Type::Nat, Type::Inf, Type::Bool];
        let err =
            EvalError::ambiguous_type_error(input, errno, &expr, candidates, caused_by.to_string());
        errors.push(err);

        let input = Input::pipe("invalid type cast error".to_string());
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
            &cast_to,
            hint,
        );
        errors.push(err);

        let input = Input::pipe("override error".to_string());
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

        let input = Input::pipe("visibility error".to_string());
        let loc = Location::Line(1);
        let vis = crate::ty::Visibility::DUMMY_PRIVATE;
        let err =
            TyCheckError::visibility_error(input, errno, loc, caused_by.to_string(), name, vis);
        errors.push(err);

        let input = Input::pipe("import nunpy as np".to_string());
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
