//! defines `Type` (type kind).
//!
//! Type(コンパイラ等で使われる「型」を表現する)を定義する
pub mod codeobj;
pub mod constructors;
pub mod deserialize;
pub mod free;
pub mod typaram;
pub mod value;

use std::fmt;
use std::ops::{Range, RangeInclusive};

use erg_common::dict::Dict;
use erg_common::set::Set;
use erg_common::traits::LimitedDisplay;
use erg_common::vis::Field;
use erg_common::{enum_unwrap, fmt_option, fmt_set_split_with, set, Str};

use crate::codeobj::CodeObj;
use crate::free::{fresh_varname, Constraint, Free, FreeKind, FreeTyVar, HasLevel, Level};
use crate::typaram::{IntervalOp, TyParam};
use crate::value::value_set::*;
use crate::value::ValueObj;
use crate::value::ValueObj::{Inf, NegInf};

/// cloneのコストがあるためなるべく.ref_tを使うようにすること
/// いくつかの構造体は直接Typeを保持していないので、その場合は.tを使う
#[allow(unused_variables)]
pub trait HasType {
    fn ref_t(&self) -> &Type;
    // 関数呼び出しの場合、.ref_t()は戻り値を返し、signature_t()は関数全体の型を返す
    fn signature_t(&self) -> Option<&Type>;
    // 最後にHIR全体の型変数を消すために使う
    fn ref_mut_t(&mut self) -> &mut Type;
    fn signature_mut_t(&mut self) -> Option<&mut Type>;
    #[inline]
    fn t(&self) -> Type {
        self.ref_t().clone()
    }
    #[inline]
    fn inner_ts(&self) -> Vec<Type> {
        self.ref_t().inner_ts()
    }
    #[inline]
    fn lhs_t(&self) -> &Type {
        &self.ref_t().non_default_params().unwrap()[0].ty
    }
    #[inline]
    fn rhs_t(&self) -> &Type {
        &self.ref_t().non_default_params().unwrap()[1].ty
    }
}

#[macro_export]
macro_rules! impl_t {
    ($T: ty) => {
        impl $crate::HasType for $T {
            #[inline]
            fn ref_t(&self) -> &Type {
                &self.t
            }
            #[inline]
            fn ref_mut_t(&mut self) -> &mut Type {
                &mut self.t
            }
            #[inline]
            fn signature_t(&self) -> Option<&Type> {
                None
            }
            #[inline]
            fn signature_mut_t(&mut self) -> Option<&mut Type> {
                None
            }
        }
    };
    ($T: ty, $sig_t: ident) => {
        impl $crate::HasType for $T {
            #[inline]
            fn ref_t(&self) -> &Type {
                &self.t
            }
            #[inline]
            fn ref_mut_t(&mut self) -> &mut Type {
                &mut self.t
            }
            #[inline]
            fn signature_t(&self) -> Option<&Type> {
                Some(&self.$sig_t)
            }
            #[inline]
            fn signature_mut_t(&mut self) -> Option<&mut Type> {
                &mut self.$sig_t
            }
        }
    };
}

#[macro_export]
macro_rules! impl_t_for_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        impl $crate::HasType for $Enum {
            fn ref_t(&self) -> &Type {
                match self {
                    $($Enum::$Variant(v) => v.ref_t(),)*
                }
            }
            fn ref_mut_t(&mut self) -> &mut Type {
                match self {
                    $($Enum::$Variant(v) => v.ref_mut_t(),)*
                }
            }
            fn signature_t(&self) -> Option<&Type> {
                match self {
                    $($Enum::$Variant(v) => v.signature_t(),)*
                }
            }
            fn signature_mut_t(&mut self) -> Option<&mut Type> {
                match self {
                    $($Enum::$Variant(v) => v.signature_mut_t(),)*
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserConstSubr {
    code: CodeObj,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuiltinConstSubr {
    subr: fn(Vec<ValueObj>) -> ValueObj,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstSubr {
    User(UserConstSubr),
    Builtin(BuiltinConstSubr),
}

impl ConstSubr {
    pub fn call(&self, args: Vec<ValueObj>) -> ValueObj {
        match self {
            ConstSubr::User(_user) => todo!(),
            ConstSubr::Builtin(builtin) => (builtin.subr)(args),
        }
    }
}

/// TyBoundはtemplateで、Constraintは自由型変数が持つinstance
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyBound {
    // e.g.
    // A :> Int => Sandwiched{sub: Int, mid: A, sup: Obj}
    // A <: {I: Int | I > 0} => Sandwiched{sub: Never, mid: A, sup: {I: Int | I > 0}}
    /// Sub <: Mid <: Sup
    Sandwiched {
        sub: Type,
        mid: Type,
        sup: Type,
    },
    // TyParam::MonoQuantVarに型の情報が含まれているので、boundsからは除去される
    // e.g. N: Nat => Instance{name: N, t: Nat}
    Instance {
        name: Str,
        t: Type,
    },
}

impl fmt::Display for TyBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl LimitedDisplay for TyBound {
    fn limited_fmt(&self, f: &mut fmt::Formatter<'_>, limit: usize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Sandwiched { sub, mid, sup } => match (sub == &Type::Never, sup == &Type::Obj) {
                (true, true) => write!(f, "{mid}: Type (:> Never, <: Obj)"),
                (true, false) => {
                    write!(f, "{mid} <: ")?;
                    sup.limited_fmt(f, limit - 1)
                }
                (false, true) => {
                    write!(f, "{mid} :> ")?;
                    sub.limited_fmt(f, limit - 1)
                }
                (false, false) => {
                    sub.limited_fmt(f, limit - 1)?;
                    write!(f, " <: {mid} <: ")?;
                    sup.limited_fmt(f, limit - 1)
                }
            },
            Self::Instance { name, t } => {
                write!(f, "'{name}: ")?;
                t.limited_fmt(f, limit - 1)
            }
        }
    }
}

impl HasLevel for TyBound {
    fn level(&self) -> Option<usize> {
        todo!()
    }

    fn update_level(&self, level: usize) {
        match self {
            Self::Sandwiched { sub, mid, sup } => {
                sub.update_level(level);
                mid.update_level(level);
                sup.update_level(level);
            }
            Self::Instance { t, .. } => {
                t.update_level(level);
            }
        }
    }

    fn lift(&self) {
        match self {
            Self::Sandwiched { sub, mid, sup } => {
                sub.lift();
                mid.lift();
                sup.lift();
            }
            Self::Instance { t, .. } => {
                t.lift();
            }
        }
    }
}

impl TyBound {
    pub const fn sandwiched(sub: Type, mid: Type, sup: Type) -> Self {
        Self::Sandwiched { sub, mid, sup }
    }

    pub const fn subtype_of(sub: Type, sup: Type) -> Self {
        Self::sandwiched(Type::Never, sub, sup)
    }

    pub const fn static_instance(name: &'static str, t: Type) -> Self {
        Self::Instance {
            name: Str::ever(name),
            t,
        }
    }

    pub fn instance(name: Str, t: Type) -> Self {
        if t == Type::Type {
            Self::sandwiched(Type::Never, Type::mono(name), Type::Obj)
        } else {
            Self::Instance { name, t }
        }
    }

    pub fn mentions_as_instance(&self, name: &str) -> bool {
        matches!(self, Self::Instance{ name: n, .. } if &n[..] == name)
    }

    pub fn mentions_as_mid(&self, name: &str) -> bool {
        matches!(self, Self::Sandwiched{ mid, .. } if &mid.name()[..] == name)
    }

    pub fn has_qvar(&self) -> bool {
        match self {
            Self::Sandwiched { sub, mid, sup } => {
                sub.has_qvar() || mid.has_qvar() || sup.has_qvar()
            }
            Self::Instance { t, .. } => t.has_qvar(),
        }
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::Sandwiched { sub, mid, sup } => {
                sub.has_unbound_var() || mid.has_unbound_var() || sup.has_unbound_var()
            }
            Self::Instance { t, .. } => t.has_unbound_var(),
        }
    }

    pub fn get_type(&self) -> &Type {
        match self {
            Self::Sandwiched { sub, sup, .. } => {
                if sub == &Type::Never && sup == &Type::Obj {
                    &Type::Type
                } else {
                    todo!()
                }
            }
            Self::Instance { t, .. } => t,
        }
    }

    pub fn get_lhs(&self) -> Str {
        match self {
            Self::Sandwiched { mid, .. } => mid.name(),
            Self::Instance { name, .. } => name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Predicate {
    Value(ValueObj), // True/False
    Const(Str),
    /// i == 0 => Eq{ lhs: "i", rhs: 0 }
    Equal {
        lhs: Str,
        rhs: TyParam,
    },
    /// i > 0 => i >= 0+ε => GreaterEqual{ lhs: "i", rhs: 0+ε }
    GreaterEqual {
        lhs: Str,
        rhs: TyParam,
    },
    LessEqual {
        lhs: Str,
        rhs: TyParam,
    },
    NotEqual {
        lhs: Str,
        rhs: TyParam,
    },
    Or(Box<Predicate>, Box<Predicate>),
    And(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>, Box<Predicate>),
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(v) => write!(f, "{v}"),
            Self::Const(c) => write!(f, "{c}"),
            Self::Equal { lhs, rhs } => write!(f, "{lhs} == {rhs}"),
            Self::GreaterEqual { lhs, rhs } => write!(f, "{lhs} >= {rhs}"),
            Self::LessEqual { lhs, rhs } => write!(f, "{lhs} <= {rhs}"),
            Self::NotEqual { lhs, rhs } => write!(f, "{lhs} != {rhs}"),
            Self::Or(l, r) => write!(f, "({l}) or ({r})"),
            Self::And(l, r) => write!(f, "({l}) and ({r})"),
            Self::Not(l, r) => write!(f, "({l}) not ({r})"),
        }
    }
}

impl HasLevel for Predicate {
    fn level(&self) -> Option<usize> {
        match self {
            Self::Value(_) | Self::Const(_) => None,
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.level(),
            Self::And(_lhs, _rhs) | Self::Or(_lhs, _rhs) | Self::Not(_lhs, _rhs) => todo!(),
        }
    }

    fn update_level(&self, level: usize) {
        match self {
            Self::Value(_) | Self::Const(_) => {}
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => {
                rhs.update_level(level);
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.update_level(level);
                rhs.update_level(level);
            }
        }
    }

    fn lift(&self) {
        match self {
            Self::Value(_) | Self::Const(_) => {}
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => {
                rhs.lift();
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.lift();
                rhs.lift();
            }
        }
    }
}

impl Predicate {
    pub const fn eq(lhs: Str, rhs: TyParam) -> Self {
        Self::Equal { lhs, rhs }
    }
    pub const fn ne(lhs: Str, rhs: TyParam) -> Self {
        Self::NotEqual { lhs, rhs }
    }
    /// >=
    pub const fn ge(lhs: Str, rhs: TyParam) -> Self {
        Self::GreaterEqual { lhs, rhs }
    }
    /// <=
    pub const fn le(lhs: Str, rhs: TyParam) -> Self {
        Self::LessEqual { lhs, rhs }
    }

    pub fn and(lhs: Predicate, rhs: Predicate) -> Self {
        Self::And(Box::new(lhs), Box::new(rhs))
    }

    pub fn or(lhs: Predicate, rhs: Predicate) -> Self {
        Self::Or(Box::new(lhs), Box::new(rhs))
    }

    pub fn not(lhs: Predicate, rhs: Predicate) -> Self {
        Self::Not(Box::new(lhs), Box::new(rhs))
    }

    pub fn subject(&self) -> Option<&str> {
        match self {
            Self::Equal { lhs, .. }
            | Self::LessEqual { lhs, .. }
            | Self::GreaterEqual { lhs, .. }
            | Self::NotEqual { lhs, .. } => Some(&lhs[..]),
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                let l = lhs.subject();
                let r = rhs.subject();
                if l != r {
                    todo!()
                } else {
                    l
                }
            }
            _ => None,
        }
    }

    pub fn change_subject_name(self, name: Str) -> Self {
        match self {
            Self::Equal { rhs, .. } => Self::eq(name, rhs),
            Self::GreaterEqual { rhs, .. } => Self::ge(name, rhs),
            Self::LessEqual { rhs, .. } => Self::le(name, rhs),
            Self::NotEqual { rhs, .. } => Self::ne(name, rhs),
            Self::And(lhs, rhs) => Self::and(
                lhs.change_subject_name(name.clone()),
                rhs.change_subject_name(name),
            ),
            Self::Or(lhs, rhs) => Self::or(
                lhs.change_subject_name(name.clone()),
                rhs.change_subject_name(name),
            ),
            Self::Not(lhs, rhs) => Self::not(
                lhs.change_subject_name(name.clone()),
                rhs.change_subject_name(name),
            ),
            _ => self,
        }
    }

    pub fn mentions(&self, name: &str) -> bool {
        match self {
            Self::Const(n) => &n[..] == name,
            Self::Equal { lhs, .. }
            | Self::LessEqual { lhs, .. }
            | Self::GreaterEqual { lhs, .. }
            | Self::NotEqual { lhs, .. } => &lhs[..] == name,
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.mentions(name) || rhs.mentions(name)
            }
            _ => false,
        }
    }

    pub fn can_be_false(&self) -> bool {
        match self {
            Self::Value(l) => matches!(l, ValueObj::Bool(false)),
            Self::Const(_) => todo!(),
            Self::Or(lhs, rhs) => lhs.can_be_false() || rhs.can_be_false(),
            Self::And(lhs, rhs) => lhs.can_be_false() && rhs.can_be_false(),
            Self::Not(lhs, rhs) => lhs.can_be_false() && !rhs.can_be_false(),
            _ => true,
        }
    }

    pub fn has_qvar(&self) -> bool {
        match self {
            Self::Value(_) => false,
            Self::Const(_) => false,
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.has_qvar(),
            Self::Or(lhs, rhs) | Self::And(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.has_qvar() || rhs.has_qvar()
            }
        }
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::Value(_) => false,
            Self::Const(_) => false,
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.has_unbound_var(),
            Self::Or(lhs, rhs) | Self::And(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.has_unbound_var() || rhs.has_unbound_var()
            }
        }
    }

    pub fn min_max<'a>(
        &'a self,
        min: Option<&'a TyParam>,
        max: Option<&'a TyParam>,
    ) -> (Option<&'a TyParam>, Option<&'a TyParam>) {
        match self {
            Predicate::Equal { rhs: _, .. } => todo!(),
            // {I | I <= 1; I <= 2}
            Predicate::LessEqual { rhs, .. } => (
                min,
                max.map(|l: &TyParam| match l.cheap_cmp(rhs) {
                    Some(c) if c.is_ge() => l,
                    Some(_) => rhs,
                    _ => l,
                })
                .or(Some(rhs)),
            ),
            // {I | I >= 1; I >= 2}
            Predicate::GreaterEqual { rhs, .. } => (
                min.map(|l: &TyParam| match l.cheap_cmp(rhs) {
                    Some(c) if c.is_le() => l,
                    Some(_) => rhs,
                    _ => l,
                })
                .or(Some(rhs)),
                max,
            ),
            Predicate::And(_l, _r) => todo!(),
            _ => todo!(),
        }
    }

    pub fn typarams(&self) -> Vec<&TyParam> {
        match self {
            Self::Value(_) | Self::Const(_) => vec![],
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => vec![rhs],
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.typarams().into_iter().chain(rhs.typarams()).collect()
            }
        }
    }

    pub fn typarams_mut(&mut self) -> Vec<&mut TyParam> {
        match self {
            Self::Value(_) | Self::Const(_) => vec![],
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => vec![rhs],
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => lhs
                .typarams_mut()
                .into_iter()
                .chain(rhs.typarams_mut())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamTy {
    pub name: Option<Str>,
    pub ty: Type,
}

impl fmt::Display for ParamTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{}: {}", name, self.ty)
        } else {
            write!(f, "{}", self.ty)
        }
    }
}

impl ParamTy {
    pub const fn new(name: Option<Str>, ty: Type) -> Self {
        Self { name, ty }
    }

    pub const fn named(name: Str, ty: Type) -> Self {
        Self {
            name: Some(name),
            ty,
        }
    }

    pub const fn anonymous(ty: Type) -> Self {
        Self::new(None, ty)
    }
}

/// e.g.
/// (x: Int, ?base: Int) -> Int
/// => SubrTy{ kind: Func, non_default_params: [x: Int], default_params: [base: Int] return_t: Int }
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubrType {
    pub kind: SubrKind,
    pub non_default_params: Vec<ParamTy>,
    pub default_params: Vec<ParamTy>,
    pub return_t: Box<Type>,
}

impl fmt::Display for SubrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl LimitedDisplay for SubrType {
    fn limited_fmt(&self, f: &mut fmt::Formatter<'_>, limit: usize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        write!(f, "{}(", self.kind.prefix())?;
        for (i, param) in self.non_default_params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", fmt_option!(param.name, post ": "))?;
            param.ty.limited_fmt(f, limit - 1)?;
        }
        for (i, default_param) in self.default_params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{} |= ", default_param.name.as_ref().unwrap(),)?;
            default_param.ty.limited_fmt(f, limit - 1)?;
        }
        write!(f, ") {} ", self.kind.arrow())?;
        self.return_t.limited_fmt(f, limit - 1)
    }
}

impl SubrType {
    pub fn new(
        kind: SubrKind,
        non_default_params: Vec<ParamTy>,
        default_params: Vec<ParamTy>,
        return_t: Type,
    ) -> Self {
        Self {
            kind,
            non_default_params,
            default_params,
            return_t: Box::new(return_t),
        }
    }

    pub fn varargs_idx(&self) -> (Option<usize>, Option<usize>) {
        (
            self.non_default_params
                .iter()
                .position(|t| t.ty.is_varargs()),
            self.default_params.iter().position(|t| t.ty.is_varargs()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefineKind {
    Interval { min: TyParam, max: TyParam }, // e.g. {I: Int | I >= 2; I <= 10} 2..10
    Enum(Set<TyParam>),                      // e.g. {I: Int | I == 1 or I == 2} {1, 2}
    Complex,
}

/// e.g.
/// ```
/// {I: Int | I >= 0}
/// {_: StrWithLen N | N >= 0}
/// {T: (Int, Int) | T.0 >= 0, T.1 >= 0}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefinementType {
    pub var: Str,
    pub t: Box<Type>,
    pub preds: Set<Predicate>,
}

impl fmt::Display for RefinementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl LimitedDisplay for RefinementType {
    fn limited_fmt(&self, f: &mut std::fmt::Formatter<'_>, limit: usize) -> std::fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        write!(f, "{{{}: ", self.var)?;
        self.t.limited_fmt(f, limit - 1)?;
        write!(f, " | {}}}", fmt_set_split_with(&self.preds, "; "))
    }
}

impl RefinementType {
    pub fn new(var: Str, t: Type, preds: Set<Predicate>) -> Self {
        Self {
            var,
            t: Box::new(t),
            preds,
        }
    }

    pub fn bound(&self) -> TyBound {
        TyBound::instance(self.var.clone(), *self.t.clone())
    }
}

/// e.g.
/// ```
/// |T: Type| T -> T == Quantified{ unbound_t: (T -> T), bounds: [T: Type] }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuantifiedType {
    pub unbound_callable: Box<Type>,
    pub bounds: Set<TyBound>,
}

impl fmt::Display for QuantifiedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl LimitedDisplay for QuantifiedType {
    fn limited_fmt(&self, f: &mut fmt::Formatter<'_>, limit: usize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        write!(f, "|")?;
        for (i, bound) in self.bounds.iter().enumerate() {
            if i != 0 {
                write!(f, "; ")?;
            }
            bound.limited_fmt(f, limit - 1)?;
        }
        write!(f, "|")?;
        self.unbound_callable.limited_fmt(f, limit - 1)
    }
}

impl QuantifiedType {
    pub fn new(unbound_callable: Type, bounds: Set<TyBound>) -> Self {
        Self {
            unbound_callable: Box::new(unbound_callable),
            bounds,
        }
    }
}

type SelfType = Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubrKind {
    Func,
    Proc,
    FuncMethod(Box<SelfType>),
    ProcMethod {
        before: Box<SelfType>,
        after: Option<Box<SelfType>>,
    },
}

impl HasLevel for SubrKind {
    fn level(&self) -> Option<Level> {
        todo!()
    }

    fn update_level(&self, level: usize) {
        match self {
            Self::FuncMethod(t) => t.update_level(level),
            Self::ProcMethod { before, after } => {
                before.update_level(level);
                if let Some(t) = after.as_ref() {
                    t.update_level(level);
                }
            }
            _ => {}
        }
    }

    fn lift(&self) {
        match self {
            Self::FuncMethod(t) => t.lift(),
            Self::ProcMethod { before, after } => {
                before.lift();
                if let Some(t) = after.as_ref() {
                    t.lift();
                }
            }
            _ => {}
        }
    }
}

impl SubrKind {
    pub fn fn_met(t: SelfType) -> Self {
        SubrKind::FuncMethod(Box::new(t))
    }

    pub fn pr_met(before: SelfType, after: Option<SelfType>) -> Self {
        Self::ProcMethod {
            before: Box::new(before),
            after: after.map(Box::new),
        }
    }

    pub const fn arrow(&self) -> Str {
        match self {
            Self::Func | Self::FuncMethod(_) => Str::ever("->"),
            Self::Proc | Self::ProcMethod { .. } => Str::ever("=>"),
        }
    }

    pub const fn inner_len(&self) -> usize {
        match self {
            Self::Func | Self::Proc => 0,
            Self::FuncMethod(_) | Self::ProcMethod { .. } => 1,
        }
    }

    pub fn prefix(&self) -> String {
        match self {
            Self::Func | Self::Proc => "".to_string(),
            Self::FuncMethod(t) => format!("{t}."),
            Self::ProcMethod { before, after } => {
                if let Some(after) = after {
                    format!("({before} ~> {after}).")
                } else {
                    format!("{before}.")
                }
            }
        }
    }

    pub fn has_qvar(&self) -> bool {
        match self {
            Self::Func | Self::Proc => false,
            Self::FuncMethod(t) => t.has_qvar(),
            Self::ProcMethod { before, after } => {
                before.has_qvar() || after.as_ref().map(|t| t.has_qvar()).unwrap_or(false)
            }
        }
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::Func | Self::Proc => false,
            Self::FuncMethod(t) => t.has_unbound_var(),
            Self::ProcMethod { before, after } => {
                before.has_unbound_var()
                    || after.as_ref().map(|t| t.has_unbound_var()).unwrap_or(false)
            }
        }
    }

    pub fn same_kind_as(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Func, Self::Func)
                | (Self::Proc, Self::Proc)
                | (Self::FuncMethod(_), Self::FuncMethod(_))
                | (Self::ProcMethod { .. }, Self::ProcMethod { .. })
        )
    }

    pub fn self_t(&self) -> Option<&SelfType> {
        match self {
            Self::FuncMethod(t) | Self::ProcMethod { before: t, .. } => Some(t),
            _ => None,
        }
    }

    pub fn self_t_mut(&mut self) -> Option<&mut SelfType> {
        match self {
            Self::FuncMethod(t) | Self::ProcMethod { before: t, .. } => Some(t),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ownership {
    Owned,
    Ref,
    RefMut,
}

impl Ownership {
    pub const fn is_owned(&self) -> bool {
        matches!(self, Self::Owned)
    }
    pub const fn is_ref(&self) -> bool {
        matches!(self, Self::Ref)
    }
    pub const fn is_refmut(&self) -> bool {
        matches!(self, Self::RefMut)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArgsOwnership {
    Args {
        self_: Option<Ownership>,
        non_defaults: Vec<Ownership>,
        defaults: Vec<Ownership>,
    },
    VarArgs(Ownership), // TODO: defaults
    VarArgsDefault(Ownership),
}

impl ArgsOwnership {
    pub const fn args(
        self_: Option<Ownership>,
        non_defaults: Vec<Ownership>,
        defaults: Vec<Ownership>,
    ) -> Self {
        Self::Args {
            self_,
            non_defaults,
            defaults,
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub enum Type {
    /* Monomorphic (builtin) types */
    Obj, // {=}
    Int,
    Nat,
    Ratio,
    Float,
    Bool,
    Str,
    NoneType,
    Code,
    Module,
    Frame,
    Error,
    Inf,    // {∞}
    NegInf, // {-∞}
    // TODO: PolyType/Class
    Type,
    Class,
    Trait,
    Patch,
    NotImplemented,
    Ellipsis,  // これはクラスのほうで型推論用のマーカーではない
    Never,     // {}
    Mono(Str), // others
    /* Polymorphic types */
    Ref(Box<Type>),
    RefMut(Box<Type>),
    VarArgs(Box<Type>), // ...T
    Subr(SubrType),
    // CallableはProcの上位型なので、変数に!をつける
    Callable {
        param_ts: Vec<Type>,
        return_t: Box<Type>,
    },
    Record(Dict<Field, Type>), // e.g. {x = Int}
    // e.g. {T -> T | T: Type}, {I: Int | I > 0}, {S | N: Nat; S: Str N; N > 1}
    // 区間型と列挙型は篩型に変換される
    // f 0 = ...はf _: {0} == {I: Int | I == 0}のシンタックスシュガー
    // e.g.
    // {0, 1, 2} => {I: Int | I == 0 or I == 1 or I == 2}
    // 1..10 => {I: Int | I >= 1 and I <= 10}
    Refinement(RefinementType),
    // e.g. |T: Type| T -> T
    Quantified(QuantifiedType),
    And(Box<Type>, Box<Type>),
    Not(Box<Type>, Box<Type>),
    Or(Box<Type>, Box<Type>),
    Poly {
        name: Str,
        params: Vec<TyParam>,
    }, // T(params)
    /* Special types (inference-time types) */
    MonoQVar(Str), // QuantifiedTyの中で使う一般化型変数、利便性のためMonoとは区別する
    PolyQVar {
        name: Str,
        params: Vec<TyParam>,
    },
    MonoProj {
        lhs: Box<Type>,
        rhs: Str,
    }, // e.g. T.U
    FreeVar(FreeTyVar), // a reference to the type of other expression, see docs/compiler/inference.md
    Failure,            // when failed to infer (e.g. get the type of `match`)
    /// used to represent `TyParam` is not initialized (see `erg_compiler::context::instantiate_tp`)
    Uninited,
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Obj, Self::Obj)
            | (Self::Int, Self::Int)
            | (Self::Nat, Self::Nat)
            | (Self::Ratio, Self::Ratio)
            | (Self::Float, Self::Float)
            | (Self::Bool, Self::Bool)
            | (Self::Str, Self::Str)
            | (Self::NoneType, Self::NoneType)
            | (Self::Code, Self::Code)
            | (Self::Module, Self::Module)
            | (Self::Frame, Self::Frame)
            | (Self::Error, Self::Error)
            | (Self::Inf, Self::Inf)
            | (Self::NegInf, Self::NegInf)
            | (Self::Type, Self::Type)
            | (Self::Class, Self::Class)
            | (Self::Trait, Self::Trait)
            | (Self::Patch, Self::Patch)
            | (Self::NotImplemented, Self::NotImplemented)
            | (Self::Ellipsis, Self::Ellipsis)
            | (Self::Never, Self::Never) => true,
            (Self::Mono(l), Self::Mono(r)) | (Self::MonoQVar(l), Self::MonoQVar(r)) => l == r,
            (Self::Ref(l), Self::Ref(r))
            | (Self::RefMut(l), Self::RefMut(r))
            | (Self::VarArgs(l), Self::VarArgs(r)) => l == r,
            (Self::Subr(l), Self::Subr(r)) => l == r,
            (
                Self::Callable {
                    param_ts: _lps,
                    return_t: _lr,
                },
                Self::Callable {
                    param_ts: _rps,
                    return_t: _rr,
                },
            ) => todo!(),
            (Self::Record(lhs), Self::Record(rhs)) => {
                for (l_field, l_t) in lhs.iter() {
                    if let Some(r_t) = rhs.get(l_field) {
                        if !(l_t == r_t) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (Self::Refinement(l), Self::Refinement(r)) => l == r,
            (Self::Quantified(l), Self::Quantified(r)) => l == r,
            (Self::And(ll, lr), Self::And(rl, rr))
            | (Self::Not(ll, lr), Self::Not(rl, rr))
            | (Self::Or(ll, lr), Self::Or(rl, rr)) => ll == rl && lr == rr,
            (
                Self::Poly {
                    name: ln,
                    params: lps,
                }
                | Self::PolyQVar {
                    name: ln,
                    params: lps,
                },
                Self::Poly {
                    name: rn,
                    params: rps,
                }
                | Self::PolyQVar {
                    name: rn,
                    params: rps,
                },
            ) => ln == rn && lps == rps,
            (
                Self::MonoProj { lhs, rhs },
                Self::MonoProj {
                    lhs: rlhs,
                    rhs: rrhs,
                },
            ) => lhs == rlhs && rhs == rrhs,
            (Self::FreeVar(l), Self::FreeVar(r)) => l == r,
            (Self::FreeVar(fv), other) => match &*fv.borrow() {
                FreeKind::Linked(t) => t == other,
                _ => false,
            },
            (self_, Self::FreeVar(fv)) => match &*fv.borrow() {
                FreeKind::Linked(t) => t == self_,
                _ => false,
            },
            (Self::Failure, Self::Failure) | (Self::Uninited, Self::Uninited) => true,
            _ => false,
        }
    }
}

impl Eq for Type {}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl LimitedDisplay for Type {
    fn limited_fmt(&self, f: &mut fmt::Formatter<'_>, limit: usize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Mono(name) => write!(f, "{name}"),
            Self::Ref(t) | Self::RefMut(t) => {
                write!(f, "{}(", self.name())?;
                t.limited_fmt(f, limit - 1)?;
                write!(f, ")")
            }
            Self::Callable { param_ts, return_t } => {
                write!(f, "Callable((")?;
                for (i, t) in param_ts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    t.limited_fmt(f, limit - 1)?;
                }
                write!(f, "), ")?;
                return_t.limited_fmt(f, limit - 1)?;
                write!(f, ")")
            }
            Self::Record(attrs) => {
                write!(f, "{{")?;
                if let Some((field, t)) = attrs.iter().next() {
                    write!(f, "{field} = ")?;
                    t.limited_fmt(f, limit - 1)?;
                }
                for (field, t) in attrs.iter().skip(1) {
                    write!(f, "; {field} = ")?;
                    t.limited_fmt(f, limit - 1)?;
                }
                write!(f, "}}")
            }
            Self::Subr(sub) => sub.limited_fmt(f, limit),
            Self::Refinement(refinement) => refinement.limited_fmt(f, limit),
            Self::Quantified(quantified) => quantified.limited_fmt(f, limit),
            Self::And(lhs, rhs) => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " and ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::Not(lhs, rhs) => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " not ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::Or(lhs, rhs) => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " or ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::VarArgs(t) => {
                write!(f, "...")?;
                t.limited_fmt(f, limit - 1)
            }
            Self::Poly { name, params } => {
                write!(f, "{name}(")?;
                for (i, tp) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    tp.limited_fmt(f, limit - 1)?;
                }
                write!(f, ")")
            }
            Self::PolyQVar { name, params } => {
                write!(f, "'{name}(")?;
                for (i, tp) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    tp.limited_fmt(f, limit - 1)?;
                }
                write!(f, ")")
            }
            Self::MonoQVar(name) => write!(f, "'{name}"),
            Self::FreeVar(fv) => fv.limited_fmt(f, limit),
            Self::MonoProj { lhs, rhs } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, ".{rhs}")
            }
            _ => write!(f, "{}", self.name()),
        }
    }
}

impl Default for Type {
    fn default() -> Self {
        Self::Failure
    }
}

impl From<Range<TyParam>> for Type {
    fn from(r: Range<TyParam>) -> Self {
        Type::int_interval(IntervalOp::RightOpen, r.start, r.end)
    }
}

impl From<Range<&TyParam>> for Type {
    fn from(r: Range<&TyParam>) -> Self {
        Type::int_interval(IntervalOp::RightOpen, r.start.clone(), r.end.clone())
    }
}

impl From<RangeInclusive<TyParam>> for Type {
    fn from(r: RangeInclusive<TyParam>) -> Self {
        let (start, end) = r.into_inner();
        Type::int_interval(IntervalOp::Closed, start, end)
    }
}

impl From<RangeInclusive<&TyParam>> for Type {
    fn from(r: RangeInclusive<&TyParam>) -> Self {
        let (start, end) = r.into_inner();
        Type::int_interval(IntervalOp::Closed, start.clone(), end.clone())
    }
}

impl From<&str> for Type {
    fn from(item: &str) -> Self {
        match item {
            "Obj" => Self::Obj,
            "Int" => Self::Int,
            "Nat" => Self::Nat,
            "Ratio" => Self::Ratio,
            "Float" => Self::Float,
            "Bool" => Self::Bool,
            "Str" => Self::Str,
            "NoneType" => Self::NoneType,
            "Type" => Self::Type,
            "Class" => Self::Class,
            "Trait" => Self::Trait,
            "Patch" => Self::Patch,
            "Code" => Self::Code,
            "Module" => Self::Module,
            "Frame" => Self::Frame,
            "Error" => Self::Error,
            // "Array" => Self::Array(Box::new(Type::Illegal)),
            "Ellipsis" => Self::Ellipsis,
            "NotImplemented" => Self::NotImplemented,
            "Never" => Self::Never,
            "Inf" => Self::Inf,
            "NegInf" => Self::NegInf,
            "_" => Self::Top(),
            other => Self::Mono(Str::rc(other)),
        }
    }
}

fn get_t_from_tp(tp: &TyParam) -> Option<Type> {
    match tp {
        TyParam::FreeVar(fv) if fv.is_linked() => get_t_from_tp(&fv.crack()),
        TyParam::Type(t) => Some(*t.clone()),
        _ => None,
    }
}

impl HasType for Type {
    #[inline]
    fn ref_t(&self) -> &Type {
        self
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        self
    }
    fn inner_ts(&self) -> Vec<Type> {
        match self {
            Self::Ref(t) | Self::RefMut(t) | Self::VarArgs(t) => {
                vec![t.as_ref().clone()]
            }
            // Self::And(ts) | Self::Or(ts) => ,
            Self::Subr(_sub) => todo!(),
            Self::Callable { param_ts, .. } => param_ts.clone(),
            Self::Poly { params, .. } => params.iter().filter_map(get_t_from_tp).collect(),
            _ => vec![],
        }
    }
    fn signature_t(&self) -> Option<&Type> {
        None
    }
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        None
    }
}

impl HasLevel for Type {
    // FIXME: 複合型のレベル
    fn level(&self) -> Option<usize> {
        match self {
            Self::FreeVar(v) => v.level(),
            _ => None,
        }
    }

    fn update_level(&self, level: Level) {
        match self {
            Self::FreeVar(v) => v.update_level(level),
            Self::Ref(t) | Self::RefMut(t) | Self::VarArgs(t) => t.update_level(level),
            Self::Callable { param_ts, return_t } => {
                for p in param_ts.iter() {
                    p.update_level(level);
                }
                return_t.update_level(level);
            }
            Self::Subr(subr) => {
                subr.kind.update_level(level);
                for p in subr
                    .non_default_params
                    .iter()
                    .chain(subr.default_params.iter())
                {
                    p.ty.update_level(level);
                }
                subr.return_t.update_level(level);
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.update_level(level);
                rhs.update_level(level);
            }
            Self::Record(attrs) => {
                for t in attrs.values() {
                    t.update_level(level);
                }
            }
            Self::Poly { params, .. } => {
                for p in params.iter() {
                    p.update_level(level);
                }
            }
            Self::MonoProj { lhs, .. } => {
                lhs.update_level(level);
            }
            Self::Refinement(refine) => {
                refine.t.update_level(level);
                for pred in refine.preds.iter() {
                    pred.update_level(level);
                }
            }
            Self::Quantified(quant) => {
                quant.unbound_callable.update_level(level);
                for bound in quant.bounds.iter() {
                    bound.update_level(level);
                }
            }
            _ => {}
        }
    }

    fn lift(&self) {
        match self {
            Self::FreeVar(v) => v.lift(),
            Self::Ref(t) | Self::RefMut(t) | Self::VarArgs(t) => t.lift(),
            Self::Callable { param_ts, return_t } => {
                for p in param_ts.iter() {
                    p.lift();
                }
                return_t.lift();
            }
            Self::Subr(subr) => {
                subr.kind.lift();
                for p in subr
                    .non_default_params
                    .iter()
                    .chain(subr.default_params.iter())
                {
                    p.ty.lift();
                }
                subr.return_t.lift();
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.lift();
                rhs.lift();
            }
            Self::Record(attrs) => {
                for t in attrs.values() {
                    t.lift();
                }
            }
            Self::Poly { params, .. } => {
                for p in params.iter() {
                    p.lift();
                }
            }
            Self::MonoProj { lhs, .. } => {
                lhs.lift();
            }
            Self::Refinement(refine) => {
                refine.t.lift();
                for pred in refine.preds.iter() {
                    pred.lift();
                }
            }
            Self::Quantified(quant) => {
                quant.unbound_callable.lift();
                for bound in quant.bounds.iter() {
                    bound.lift();
                }
            }
            _ => {}
        }
    }
}

impl Type {
    pub const OBJ: &'static Self = &Self::Obj;
    pub const NONE: &'static Self = &Self::NoneType;
    pub const NOT_IMPLEMENTED: &'static Self = &Self::NotImplemented;
    pub const ELLIPSIS: &'static Self = &Self::Ellipsis;
    pub const INF: &'static Self = &Self::Inf;
    pub const NEG_INF: &'static Self = &Self::NegInf;
    pub const NEVER: &'static Self = &Self::Never;
    pub const FAILURE: &'static Self = &Self::Failure;

    /// Top := {=}
    #[allow(non_snake_case)]
    pub const fn Top() -> Self {
        Self::Mono(Str::ever("Top"))
    }
    /// Bottom := {}
    #[allow(non_snake_case)]
    pub const fn Bottom() -> Self {
        Self::Mono(Str::ever("Bottom"))
    }

    #[inline]
    pub fn free_var(level: usize, constraint: Constraint) -> Self {
        Self::FreeVar(Free::new_unbound(level, constraint))
    }

    #[inline]
    pub fn named_free_var(name: Str, level: usize, constraint: Constraint) -> Self {
        Self::FreeVar(Free::new_named_unbound(name, level, constraint))
    }

    pub fn array(elem_t: Type, len: TyParam) -> Self {
        Self::poly("Array", vec![TyParam::t(elem_t), len])
    }

    pub fn array_mut(elem_t: Type, len: TyParam) -> Self {
        Self::poly("Array!", vec![TyParam::t(elem_t), len])
    }

    pub fn dict(k_t: Type, v_t: Type) -> Self {
        Self::poly("Dict", vec![TyParam::t(k_t), TyParam::t(v_t)])
    }

    pub fn tuple(args: Vec<Type>) -> Self {
        Self::poly("Tuple", args.into_iter().map(TyParam::t).collect())
    }

    #[inline]
    pub fn var_args(elem_t: Type) -> Self {
        Self::VarArgs(Box::new(elem_t))
    }

    #[inline]
    pub fn range(t: Type) -> Self {
        Self::poly("Range", vec![TyParam::t(t)])
    }

    pub fn enum_t(s: Set<ValueObj>) -> Self {
        assert!(is_homogeneous(&s));
        let name = Str::from(fresh_varname());
        let preds = s
            .iter()
            .map(|o| Predicate::eq(name.clone(), TyParam::value(o.clone())))
            .collect();
        let refine = RefinementType::new(name, inner_class(&s), preds);
        Self::Refinement(refine)
    }

    #[inline]
    pub fn int_interval<P: Into<TyParam>, Q: Into<TyParam>>(op: IntervalOp, l: P, r: Q) -> Self {
        let l = l.into();
        let r = r.into();
        let l = l.try_into().unwrap_or_else(|l| todo!("{l}"));
        let r = r.try_into().unwrap_or_else(|r| todo!("{r}"));
        let name = Str::from(fresh_varname());
        let pred = match op {
            IntervalOp::LeftOpen if l == TyParam::value(NegInf) => Predicate::le(name.clone(), r),
            // l<..r => {I: classof(l) | I >= l+ε and I <= r}
            IntervalOp::LeftOpen => Predicate::and(
                Predicate::ge(name.clone(), TyParam::succ(l)),
                Predicate::le(name.clone(), r),
            ),
            IntervalOp::RightOpen if r == TyParam::value(Inf) => Predicate::ge(name.clone(), l),
            // l..<r => {I: classof(l) | I >= l and I <= r-ε}
            IntervalOp::RightOpen => Predicate::and(
                Predicate::ge(name.clone(), l),
                Predicate::le(name.clone(), TyParam::pred(r)),
            ),
            // l..r => {I: classof(l) | I >= l and I <= r}
            IntervalOp::Closed => Predicate::and(
                Predicate::ge(name.clone(), l),
                Predicate::le(name.clone(), r),
            ),
            IntervalOp::Open if l == TyParam::value(NegInf) && r == TyParam::value(Inf) => {
                return Type::refinement(name, Type::Int, set! {})
            }
            // l<..<r => {I: classof(l) | I >= l+ε and I <= r-ε}
            IntervalOp::Open => Predicate::and(
                Predicate::ge(name.clone(), TyParam::succ(l)),
                Predicate::le(name.clone(), TyParam::pred(r)),
            ),
        };
        Type::refinement(name, Type::Int, set! {pred})
    }

    pub fn iter(t: Type) -> Self {
        Self::poly("Iter", vec![TyParam::t(t)])
    }

    pub fn ref_(t: Type) -> Self {
        Self::Ref(Box::new(t))
    }

    pub fn ref_mut(t: Type) -> Self {
        Self::RefMut(Box::new(t))
    }

    pub fn option(t: Type) -> Self {
        Self::poly("Option", vec![TyParam::t(t)])
    }

    pub fn option_mut(t: Type) -> Self {
        Self::poly("Option!", vec![TyParam::t(t)])
    }

    pub fn subr(
        kind: SubrKind,
        non_default_params: Vec<ParamTy>,
        default_params: Vec<ParamTy>,
        return_t: Type,
    ) -> Self {
        Self::Subr(SubrType::new(
            kind,
            non_default_params,
            default_params,
            return_t,
        ))
    }

    pub fn func(
        non_default_params: Vec<ParamTy>,
        default_params: Vec<ParamTy>,
        return_t: Type,
    ) -> Self {
        Self::Subr(SubrType::new(
            SubrKind::Func,
            non_default_params,
            default_params,
            return_t,
        ))
    }

    pub fn func1(param_t: Type, return_t: Type) -> Self {
        Self::func(vec![ParamTy::anonymous(param_t)], vec![], return_t)
    }

    pub fn kind1(param: Type) -> Self {
        Self::func1(param, Type::Type)
    }

    pub fn func2(l: Type, r: Type, return_t: Type) -> Self {
        Self::func(
            vec![ParamTy::anonymous(l), ParamTy::anonymous(r)],
            vec![],
            return_t,
        )
    }

    pub fn bin_op(l: Type, r: Type, return_t: Type) -> Self {
        Self::nd_func(
            vec![
                ParamTy::named(Str::ever("lhs"), l.clone()),
                ParamTy::named(Str::ever("rhs"), r.clone()),
            ],
            return_t,
        )
    }

    pub fn anon_param_func(
        non_default_params: Vec<Type>,
        default_params: Vec<Type>,
        return_t: Type,
    ) -> Self {
        let non_default_params = non_default_params
            .into_iter()
            .map(ParamTy::anonymous)
            .collect();
        let default_params = default_params.into_iter().map(ParamTy::anonymous).collect();
        Self::func(non_default_params, default_params, return_t)
    }

    pub fn proc(
        non_default_params: Vec<ParamTy>,
        default_params: Vec<ParamTy>,
        return_t: Type,
    ) -> Self {
        Self::Subr(SubrType::new(
            SubrKind::Proc,
            non_default_params,
            default_params,
            return_t,
        ))
    }

    pub fn proc1(param_t: Type, return_t: Type) -> Self {
        Self::proc(vec![ParamTy::anonymous(param_t)], vec![], return_t)
    }

    pub fn proc2(l: Type, r: Type, return_t: Type) -> Self {
        Self::proc(
            vec![ParamTy::anonymous(l), ParamTy::anonymous(r)],
            vec![],
            return_t,
        )
    }

    pub fn anon_param_proc(
        non_default_params: Vec<Type>,
        default_params: Vec<Type>,
        return_t: Type,
    ) -> Self {
        let non_default_params = non_default_params
            .into_iter()
            .map(ParamTy::anonymous)
            .collect();
        let default_params = default_params.into_iter().map(ParamTy::anonymous).collect();
        Self::proc(non_default_params, default_params, return_t)
    }

    pub fn fn_met(
        self_t: Type,
        non_default_params: Vec<ParamTy>,
        default_params: Vec<ParamTy>,
        return_t: Type,
    ) -> Self {
        Self::Subr(SubrType::new(
            SubrKind::FuncMethod(Box::new(self_t)),
            non_default_params,
            default_params,
            return_t,
        ))
    }

    pub fn fn0_met(self_t: Type, return_t: Type) -> Self {
        Self::fn_met(self_t, vec![], vec![], return_t)
    }

    pub fn fn1_met(self_t: Type, input_t: Type, return_t: Type) -> Self {
        Self::fn_met(self_t, vec![ParamTy::anonymous(input_t)], vec![], return_t)
    }

    pub fn anon_param_fn_met(
        self_t: Type,
        non_default_params: Vec<Type>,
        default_params: Vec<Type>,
        return_t: Type,
    ) -> Self {
        let non_default_params = non_default_params
            .into_iter()
            .map(ParamTy::anonymous)
            .collect();
        let default_params = default_params.into_iter().map(ParamTy::anonymous).collect();
        Self::fn_met(self_t, non_default_params, default_params, return_t)
    }

    pub fn pr_met(
        self_before: Type,
        self_after: Option<Type>,
        non_default_params: Vec<ParamTy>,
        default_params: Vec<ParamTy>,
        return_t: Type,
    ) -> Self {
        Self::Subr(SubrType::new(
            SubrKind::pr_met(self_before, self_after),
            non_default_params,
            default_params,
            return_t,
        ))
    }

    pub fn pr0_met(self_before: Type, self_after: Option<Type>, return_t: Type) -> Self {
        Self::pr_met(self_before, self_after, vec![], vec![], return_t)
    }

    pub fn pr1_met(
        self_before: Type,
        self_after: Option<Type>,
        input_t: Type,
        return_t: Type,
    ) -> Self {
        Self::pr_met(
            self_before,
            self_after,
            vec![ParamTy::anonymous(input_t)],
            vec![],
            return_t,
        )
    }

    pub fn anon_param_pr_met(
        self_before: Type,
        self_after: Option<Type>,
        non_default_params: Vec<Type>,
        default_params: Vec<Type>,
        return_t: Type,
    ) -> Self {
        let non_default_params = non_default_params
            .into_iter()
            .map(ParamTy::anonymous)
            .collect();
        let default_params = default_params.into_iter().map(ParamTy::anonymous).collect();
        Self::pr_met(
            self_before,
            self_after,
            non_default_params,
            default_params,
            return_t,
        )
    }

    /// function type with non-default parameters
    #[inline]
    pub fn nd_func(params: Vec<ParamTy>, ret: Type) -> Type {
        Type::func(params, vec![], ret)
    }

    #[inline]
    pub fn nd_proc(params: Vec<ParamTy>, ret: Type) -> Type {
        Type::proc(params, vec![], ret)
    }

    pub fn callable(param_ts: Vec<Type>, return_t: Type) -> Self {
        Self::Callable {
            param_ts,
            return_t: Box::new(return_t),
        }
    }

    #[inline]
    pub fn mono<S: Into<Str>>(name: S) -> Self {
        Self::Mono(name.into())
    }

    #[inline]
    pub fn mono_q<S: Into<Str>>(name: S) -> Self {
        Self::MonoQVar(name.into())
    }

    #[inline]
    pub fn poly<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Self {
        Self::Poly {
            name: name.into(),
            params,
        }
    }

    #[inline]
    pub fn poly_q<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Self {
        Self::PolyQVar {
            name: name.into(),
            params,
        }
    }

    #[inline]
    pub fn mono_proj<S: Into<Str>>(lhs: Type, rhs: S) -> Self {
        Self::MonoProj {
            lhs: Box::new(lhs),
            rhs: rhs.into(),
        }
    }

    /// ```rust
    /// {I: Int | I >= 0}
    /// => Refinement{
    ///     layout: TyParam::MonoQ "I",
    ///     bounds: [TyBound::Instance("I", "Int")],
    ///     preds: [Predicate::GreaterEqual("I", 0)]
    /// }
    /// ```
    #[inline]
    pub fn refinement(var: Str, t: Type, preds: Set<Predicate>) -> Self {
        Self::Refinement(RefinementType::new(var, t, preds))
    }

    /// quantified((T -> T), T: Type) => |T: Type| T -> T
    pub fn quantified(unbound_t: Type, bounds: Set<TyBound>) -> Self {
        Self::Quantified(QuantifiedType::new(unbound_t, bounds))
    }

    pub fn and(lhs: Self, rhs: Self) -> Self {
        Self::And(Box::new(lhs), Box::new(rhs))
    }

    pub fn or(lhs: Self, rhs: Self) -> Self {
        Self::Or(Box::new(lhs), Box::new(rhs))
    }

    pub fn not(lhs: Self, rhs: Self) -> Self {
        Self::Not(Box::new(lhs), Box::new(rhs))
    }

    pub fn is_mono_q(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_mono_q(),
            Self::MonoQVar(_) => true,
            _ => false,
        }
    }

    /// 本来は型環境が必要
    pub fn mutate(self) -> Self {
        match self {
            Self::Int => Self::mono("Int!"),
            Self::Nat => Self::mono("Nat!"),
            Self::Ratio => Self::mono("Ratio!"),
            Self::Float => Self::mono("Float!"),
            Self::Bool => Self::mono("Bool!"),
            Self::Str => Self::mono("Str!"),
            _ => todo!(),
        }
    }

    pub fn is_basic_class(&self) -> bool {
        match self {
            Self::Obj
            | Self::Int
            | Self::Nat
            | Self::Ratio
            | Self::Float
            | Self::Bool
            | Self::Str
            | Self::NoneType
            | Self::Code
            | Self::Module
            | Self::Frame
            | Self::Error
            | Self::Inf
            | Self::NegInf
            | Self::Type
            | Self::Class
            | Self::Trait
            | Self::Patch
            | Self::NotImplemented
            | Self::Ellipsis
            | Self::Never
            | Self::Subr(_)
            | Self::Record(_) => true,
            _ => false,
        }
    }

    pub fn is_mut(&self) -> bool {
        match self {
            Self::FreeVar(fv) => {
                if fv.is_linked() {
                    fv.crack().is_mut()
                } else {
                    fv.unbound_name().unwrap().ends_with('!')
                }
            }
            Self::Mono(name)
            | Self::MonoQVar(name)
            | Self::Poly { name, .. }
            | Self::PolyQVar { name, .. }
            | Self::MonoProj { rhs: name, .. } => name.ends_with('!'),
            _ => false,
        }
    }

    pub fn is_nonelike(&self) -> bool {
        match self {
            Self::NoneType => true,
            Self::Poly { name, params } if &name[..] == "Option" || &name[..] == "Option!" => {
                let inner_t = enum_unwrap!(params.first().unwrap(), TyParam::Type);
                inner_t.is_nonelike()
            }
            Self::Poly { name, params } if &name[..] == "Tuple" => params.is_empty(),
            _ => false,
        }
    }

    pub fn args_ownership(&self) -> ArgsOwnership {
        match self {
            Self::Subr(subr) => {
                let self_ = subr.kind.self_t().map(|t| t.ownership());
                let mut nd_args = vec![];
                for nd_param in subr.non_default_params.iter() {
                    let ownership = match &nd_param.ty {
                        Self::Ref(_) => Ownership::Ref,
                        Self::RefMut(_) => Ownership::RefMut,
                        Self::VarArgs(t) => return ArgsOwnership::VarArgs(t.ownership()),
                        _ => Ownership::Owned,
                    };
                    nd_args.push(ownership);
                }
                let mut d_args = vec![];
                for d_param in subr.default_params.iter() {
                    let ownership = match &d_param.ty {
                        Self::Ref(_) => Ownership::Ref,
                        Self::RefMut(_) => Ownership::RefMut,
                        Self::VarArgs(t) => return ArgsOwnership::VarArgsDefault(t.ownership()),
                        _ => Ownership::Owned,
                    };
                    d_args.push(ownership);
                }
                ArgsOwnership::args(self_, nd_args, d_args)
            }
            _ => todo!(),
        }
    }

    pub fn ownership(&self) -> Ownership {
        match self {
            Self::Ref(_) => Ownership::Ref,
            Self::RefMut(_) => Ownership::RefMut,
            _ => Ownership::Owned,
        }
    }

    /// 共通部分(A and B)を返す
    /// 型同士の包含関係はここでは検査しない(TypeCheckerでする)
    pub fn intersection(lhs: &Self, rhs: &Self) -> Self {
        if lhs == rhs {
            return lhs.clone();
        }
        match (lhs, rhs) {
            // { .i: Int } and { .s: Str } == { .i: Int, .s: Str }
            (Self::Record(l), Self::Record(r)) => Self::Record(l.clone().concat(r.clone())),
            (t, Self::Obj) | (Self::Obj, t) => t.clone(),
            (_, Self::Never) | (Self::Never, _) => Self::Never,
            (l, r) => Self::and(l.clone(), r.clone()),
        }
    }

    pub fn name(&self) -> Str {
        match self {
            Self::Obj => Str::ever("Obj"),
            Self::Int => Str::ever("Int"),
            Self::Nat => Str::ever("Nat"),
            Self::Ratio => Str::ever("Ratio"),
            Self::Float => Str::ever("Float"),
            Self::Bool => Str::ever("Bool"),
            Self::Str => Str::ever("Str"),
            Self::NoneType => Str::ever("NoneType"),
            Self::Type => Str::ever("Type"),
            Self::Class => Str::ever("Class"),
            Self::Trait => Str::ever("Trait"),
            Self::Patch => Str::ever("Patch"),
            Self::Code => Str::ever("Code"),
            Self::Module => Str::ever("Module"),
            Self::Frame => Str::ever("Frame"),
            Self::Error => Str::ever("Error"),
            Self::Inf => Str::ever("Inf"),
            Self::NegInf => Str::ever("NegInf"),
            Self::Mono(name) | Self::MonoQVar(name) => name.clone(),
            Self::And(_, _) => Str::ever("And(_, _)"),
            Self::Not(_, _) => Str::ever("Not(_, _)"),
            Self::Or(_, _) => Str::ever("Or(_, _)"),
            Self::Ref(_) => Str::ever("Ref(_)"),
            Self::RefMut(_) => Str::ever("RefMut(_)"),
            Self::Subr(SubrType {
                kind: SubrKind::Func,
                ..
            }) => Str::ever("Func"),
            Self::Subr(SubrType {
                kind: SubrKind::Proc,
                ..
            }) => Str::ever("Proc"),
            Self::Subr(SubrType {
                kind: SubrKind::FuncMethod(_),
                ..
            }) => Str::ever("FuncMethod"),
            Self::Subr(SubrType {
                kind: SubrKind::ProcMethod { .. },
                ..
            }) => Str::ever("ProcMethod"),
            Self::Callable { .. } => Str::ever("Callable { .. }"),
            Self::Record(_) => Str::ever("Record(_)"),
            Self::VarArgs(_) => Str::ever("VarArgs(_)"),
            Self::Poly { name, .. } | Self::PolyQVar { name, .. } => name.clone(),
            // NOTE: compiler/codegen/convert_to_python_methodでクラス名を使うため、こうすると都合が良い
            Self::Refinement(refine) => refine.t.name(),
            Self::Quantified(_) => Str::ever("Quantified(_)"),
            Self::Ellipsis => Str::ever("Ellipsis"),
            Self::NotImplemented => Str::ever("NotImplemented"),
            Self::Never => Str::ever("Never"),
            Self::FreeVar(_fv) => match &*_fv.borrow() {
                FreeKind::Linked(l) => l.name(),
                FreeKind::NamedUnbound { name, .. } => name.clone(),
                FreeKind::Unbound { id, .. } => Str::from(id.to_string()),
            }, // TODO: 中身がSomeなら表示したい
            Self::MonoProj { .. } => Str::ever("MonoProj { .. }"),
            Self::Failure => Str::ever("Failure"),
            Self::Uninited => Str::ever("Uninited"),
        }
    }

    pub fn tvar_name(&self) -> Option<Str> {
        match self {
            Self::FreeVar(fv) => fv.unbound_name(),
            Self::MonoQVar(name) => Some(name.clone()),
            _ => None,
        }
    }

    pub const fn is_free_var(&self) -> bool {
        matches!(self, Self::FreeVar(_))
    }

    pub const fn is_varargs(&self) -> bool {
        matches!(self, Self::VarArgs(_))
    }

    pub fn is_monomorphic(&self) -> bool {
        self.typarams_len() == 0
    }

    pub const fn is_callable(&self) -> bool {
        matches!(self, Self::Subr { .. } | Self::Callable { .. })
    }

    pub fn is_tyvar(&self) -> bool {
        matches!(self, Self::FreeVar(fv) if fv.is_unbound())
    }

    pub fn has_qvar(&self) -> bool {
        match self {
            Self::MonoQVar(_) | Self::PolyQVar { .. } => true,
            Self::FreeVar(fv) => {
                if fv.is_unbound() {
                    false
                } else {
                    fv.crack().has_qvar()
                }
            }
            Self::Ref(t) | Self::RefMut(t) | Self::VarArgs(t) => t.has_qvar(),
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.has_qvar() || rhs.has_qvar()
            }
            Self::Callable { param_ts, return_t } => {
                param_ts.iter().any(|t| t.has_qvar()) || return_t.has_qvar()
            }
            Self::Subr(subr) => {
                subr.kind.has_qvar()
                    || subr.non_default_params.iter().any(|pt| pt.ty.has_qvar())
                    || subr.default_params.iter().any(|pt| pt.ty.has_qvar())
                    || subr.return_t.has_qvar()
            }
            Self::Record(r) => r.values().any(|t| t.has_qvar()),
            Self::Refinement(refine) => {
                refine.t.has_qvar() || refine.preds.iter().any(|pred| pred.has_qvar())
            }
            Self::Quantified(quant) => {
                quant.unbound_callable.has_unbound_var()
                    || quant.bounds.iter().any(|tb| tb.has_qvar())
            }
            Self::Poly { params, .. } => params.iter().any(|tp| tp.has_qvar()),
            Self::MonoProj { lhs, .. } => lhs.has_qvar(),
            _ => false,
        }
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::FreeVar(fv) => {
                if fv.is_unbound() {
                    true
                } else {
                    fv.crack().has_unbound_var()
                }
            }
            Self::Ref(t) | Self::RefMut(t) | Self::VarArgs(t) => t.has_unbound_var(),
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.has_unbound_var() || rhs.has_unbound_var()
            }
            Self::Callable { param_ts, return_t } => {
                param_ts.iter().any(|t| t.has_unbound_var()) || return_t.has_unbound_var()
            }
            Self::Subr(subr) => {
                subr.kind.has_unbound_var()
                    || subr
                        .non_default_params
                        .iter()
                        .any(|p| p.ty.has_unbound_var())
                    || subr.default_params.iter().any(|p| p.ty.has_unbound_var())
                    || subr.return_t.has_unbound_var()
            }
            Self::Record(r) => r.values().any(|t| t.has_unbound_var()),
            Self::Refinement(refine) => {
                refine.t.has_unbound_var() || refine.preds.iter().any(|p| p.has_unbound_var())
            }
            Self::Quantified(quant) => {
                quant.unbound_callable.has_unbound_var()
                    || quant.bounds.iter().any(|b| b.has_unbound_var())
            }
            Self::Poly { params, .. } | Self::PolyQVar { params, .. } => {
                params.iter().any(|p| p.has_unbound_var())
            }
            Self::MonoProj { lhs, .. } => lhs.has_no_unbound_var(),
            _ => false,
        }
    }

    pub fn has_no_unbound_var(&self) -> bool {
        !self.has_unbound_var()
    }

    pub fn typarams_len(&self) -> usize {
        match self {
            // REVIEw:
            Self::Ref(_) | Self::RefMut(_) => 1,
            Self::And(_, _) | Self::Or(_, _) | Self::Not(_, _) => 2,
            Self::Subr(subr) => {
                subr.kind.inner_len()
                    + subr.non_default_params.len()
                    + subr.default_params.len()
                    + 1
            }
            Self::Callable { param_ts, .. } => param_ts.len() + 1,
            Self::Poly { params, .. } | Self::PolyQVar { params, .. } => params.len(),
            _ => 0,
        }
    }

    pub fn typarams(&self) -> Vec<TyParam> {
        match self {
            Self::FreeVar(f) if f.is_linked() => f.crack().typarams(),
            Self::FreeVar(_unbound) => vec![],
            Self::Ref(t) | Self::RefMut(t) => vec![TyParam::t(*t.clone())],
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                vec![TyParam::t(*lhs.clone()), TyParam::t(*rhs.clone())]
            }
            Self::Subr(subr) => {
                if let Some(self_t) = subr.kind.self_t() {
                    [
                        vec![TyParam::t(self_t.clone())],
                        subr.non_default_params
                            .iter()
                            .map(|t| TyParam::t(t.ty.clone()))
                            .collect(),
                        subr.default_params
                            .iter()
                            .map(|t| TyParam::t(t.ty.clone()))
                            .collect(),
                    ]
                    .concat()
                } else {
                    [
                        subr.non_default_params
                            .iter()
                            .map(|t| TyParam::t(t.ty.clone()))
                            .collect::<Vec<_>>(),
                        subr.default_params
                            .iter()
                            .map(|t| TyParam::t(t.ty.clone()))
                            .collect(),
                    ]
                    .concat()
                }
            }
            Self::Callable { param_ts: _, .. } => todo!(),
            Self::Poly { params, .. } | Self::PolyQVar { params, .. } => params.clone(),
            _ => vec![],
        }
    }

    pub const fn self_t(&self) -> Option<&Type> {
        match self {
            Self::Subr(SubrType {
                kind: SubrKind::FuncMethod(self_t) | SubrKind::ProcMethod { before: self_t, .. },
                ..
            }) => Some(self_t),
            _ => None,
        }
    }

    pub const fn non_default_params(&self) -> Option<&Vec<ParamTy>> {
        match self {
            Self::Subr(SubrType {
                non_default_params, ..
            }) => Some(non_default_params),
            Self::Callable { param_ts: _, .. } => todo!(),
            _ => None,
        }
    }

    pub const fn default_params(&self) -> Option<&Vec<ParamTy>> {
        match self {
            Self::Subr(SubrType { default_params, .. }) => Some(default_params),
            _ => None,
        }
    }

    pub const fn return_t(&self) -> Option<&Type> {
        match self {
            Self::Subr(SubrType { return_t, .. }) | Self::Callable { return_t, .. } => {
                Some(return_t)
            }
            _ => None,
        }
    }

    pub fn mut_return_t(&mut self) -> Option<&mut Type> {
        match self {
            Self::Subr(SubrType { return_t, .. }) | Self::Callable { return_t, .. } => {
                Some(return_t)
            }
            _ => None,
        }
    }

    pub fn update_constraint(&self, new_constraint: Constraint) {
        match self {
            Self::FreeVar(fv) => {
                fv.update_constraint(new_constraint);
            }
            _ => {}
        }
    }
}

/// バイトコード命令で、in-place型付けをするオブジェクト
/// MaybeBigがついている場合、固定長でない可能性あり(実行時検査が必要)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TypeCode {
    Int32 = 1,
    Nat64,
    Float64,
    Bool,
    Str,
    StrMut,
    Array, // 要素数は検査済みなので、気にする必要はない
    ArrayMut,
    // Dict,
    Func,
    Proc,
    MaybeBigInt,
    MaybeBigNat,
    MaybeBigFloat,
    MaybeBigStr,
    Other,
    Illegal,
}

// TODO:
impl From<&Type> for TypeCode {
    fn from(arg: &Type) -> Self {
        match arg {
            Type::Int => Self::Int32,
            Type::Nat => Self::Nat64,
            Type::Float => Self::Float64,
            Type::Bool => Self::Bool,
            Type::Str => Self::Str,
            Type::Mono(name) => match &name[..] {
                "Int!" => Self::Int32,
                "Nat!" => Self::Nat64,
                "Float!" => Self::Float64,
                "Bool!" => Self::Bool,
                "Str!" => Self::Str,
                _ => Self::Other,
            },
            Type::Poly { name, .. } => match &name[..] {
                "Array" | "Array!" => Self::Array,
                "Func" => Self::Func,
                "Proc" => Self::Proc,
                _ => Self::Other,
            },
            Type::Refinement(refine) => Self::from(&*refine.t),
            _ => Self::Other,
        }
    }
}

/// バイトコード命令で、in-place型付けをするオブジェクトペア
/// とりあえずは必要性の高いペアから登録する
/// 全ての式の型が確認されているので、戻り値の型は不要
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TypePair {
    IntInt = 1,
    IntNat,
    IntFloat,
    IntStr,
    IntBool,
    IntArray,
    IntFunc,
    IntProc,
    NatInt,
    NatNat,
    NatFloat,
    NatStr,
    NatBool,
    NatArray,
    NatFunc,
    NatProc,
    FloatInt,
    FloatNat,
    FloatFloat,
    FloatStr,
    FloatBool,
    FloatArray,
    FloatFunc,
    FloatProc,
    BoolInt,
    BoolNat,
    BoolFloat,
    BoolStr,
    BoolBool,
    BoolArray,
    BoolFunc,
    BoolProc,
    StrInt,
    StrNat,
    StrFloat,
    StrBool,
    StrStr,
    StrArray,
    StrFunc,
    StrProc,
    // 要素数は検査済みなので、気にする必要はない
    ArrayInt,
    ArrayNat,
    ArrayFloat,
    ArrayStr,
    ArrayBool,
    ArrayArray,
    ArrayFunc,
    ArrayProc,
    FuncInt,
    FuncNat,
    FuncFloat,
    FuncStr,
    FuncBool,
    FuncArray,
    FuncFunc,
    FuncProc,
    ProcInt,
    ProcNat,
    ProcFloat,
    ProcStr,
    ProcBool,
    ProcArray,
    ProcFunc,
    ProcProc,
    Others,
    Illegals,
}

impl From<u8> for TypePair {
    fn from(code: u8) -> Self {
        match code {
            1 => Self::IntInt,
            2 => Self::IntNat,
            3 => Self::IntFloat,
            4 => Self::IntStr,
            5 => Self::IntBool,
            6 => Self::IntArray,
            7 => Self::IntFunc,
            8 => Self::IntProc,
            9 => Self::NatInt,
            10 => Self::NatNat,
            11 => Self::NatFloat,
            12 => Self::NatStr,
            13 => Self::NatBool,
            14 => Self::NatArray,
            15 => Self::NatFunc,
            16 => Self::NatProc,
            17 => Self::FloatInt,
            18 => Self::FloatNat,
            19 => Self::FloatFloat,
            20 => Self::FloatStr,
            21 => Self::FloatBool,
            22 => Self::FloatArray,
            23 => Self::FloatFunc,
            24 => Self::FloatProc,
            25 => Self::BoolInt,
            26 => Self::BoolNat,
            27 => Self::BoolFloat,
            28 => Self::BoolStr,
            29 => Self::BoolBool,
            30 => Self::BoolArray,
            31 => Self::BoolFunc,
            32 => Self::BoolProc,
            33 => Self::StrInt,
            34 => Self::StrNat,
            35 => Self::StrFloat,
            36 => Self::StrBool,
            37 => Self::StrStr,
            38 => Self::StrArray,
            39 => Self::StrFunc,
            40 => Self::StrProc,
            // 要素数は検査済みなので、気にする必要はない
            41 => Self::ArrayInt,
            42 => Self::ArrayNat,
            43 => Self::ArrayFloat,
            44 => Self::ArrayStr,
            45 => Self::ArrayBool,
            46 => Self::ArrayArray,
            47 => Self::ArrayFunc,
            48 => Self::ArrayProc,
            49 => Self::FuncInt,
            50 => Self::FuncNat,
            51 => Self::FuncFloat,
            52 => Self::FuncStr,
            53 => Self::FuncBool,
            54 => Self::FuncArray,
            55 => Self::FuncFunc,
            56 => Self::FuncProc,
            57 => Self::ProcInt,
            58 => Self::ProcNat,
            59 => Self::ProcFloat,
            60 => Self::ProcStr,
            61 => Self::ProcBool,
            62 => Self::ProcArray,
            63 => Self::ProcProc,
            64 => Self::Others,
            _ => Self::Illegals,
        }
    }
}

// TODO:
impl TypePair {
    pub fn new(lhs: &Type, rhs: &Type) -> Self {
        match (lhs, rhs) {
            (Type::Int, Type::Int) => Self::IntInt,
            (Type::Int, Type::Nat) => Self::IntNat,
            (Type::Int, Type::Float) => Self::IntFloat,
            (Type::Int, Type::Str) => Self::IntStr,
            (Type::Int, Type::Bool) => Self::IntBool,
            (Type::Int, Type::Poly { name, .. }) if &name[..] == "Array" => Self::IntArray,
            (Type::Int, Type::Poly { name, .. }) if &name[..] == "Func" => Self::IntFunc,
            (Type::Int, Type::Poly { name, .. }) if &name[..] == "Proc" => Self::IntProc,
            (Type::Nat, Type::Int) => Self::NatInt,
            (Type::Nat, Type::Nat) => Self::NatNat,
            (Type::Nat, Type::Float) => Self::NatFloat,
            (Type::Nat, Type::Str) => Self::NatStr,
            (Type::Nat, Type::Bool) => Self::NatBool,
            (Type::Nat, Type::Poly { name, .. }) if &name[..] == "Array" => Self::NatArray,
            (Type::Nat, Type::Poly { name, .. }) if &name[..] == "Func" => Self::NatFunc,
            (Type::Nat, Type::Poly { name, .. }) if &name[..] == "Proc" => Self::NatProc,
            (Type::Float, Type::Int) => Self::FloatInt,
            (Type::Float, Type::Nat) => Self::FloatNat,
            (Type::Float, Type::Float) => Self::FloatFloat,
            (Type::Float, Type::Str) => Self::FloatStr,
            (Type::Float, Type::Bool) => Self::FloatBool,
            (Type::Float, Type::Poly { name, .. }) if &name[..] == "Array" => Self::FloatArray,
            (Type::Float, Type::Poly { name, .. }) if &name[..] == "Func" => Self::FloatFunc,
            (Type::Float, Type::Poly { name, .. }) if &name[..] == "Proc" => Self::FloatProc,
            (Type::Bool, Type::Int) => Self::BoolInt,
            (Type::Bool, Type::Nat) => Self::BoolNat,
            (Type::Bool, Type::Float) => Self::BoolFloat,
            (Type::Bool, Type::Str) => Self::BoolStr,
            (Type::Bool, Type::Bool) => Self::BoolBool,
            (Type::Bool, Type::Poly { name, .. }) if &name[..] == "Array" => Self::BoolArray,
            (Type::Bool, Type::Poly { name, .. }) if &name[..] == "Func" => Self::BoolFunc,
            (Type::Bool, Type::Poly { name, .. }) if &name[..] == "Proc" => Self::BoolProc,
            (Type::Str, Type::Int) => Self::StrInt,
            (Type::Str, Type::Nat) => Self::StrNat,
            (Type::Str, Type::Float) => Self::StrFloat,
            (Type::Str, Type::Bool) => Self::StrBool,
            (Type::Str, Type::Str) => Self::StrStr,
            (Type::Str, Type::Poly { name, .. }) if &name[..] == "Array" => Self::StrArray,
            (Type::Str, Type::Poly { name, .. }) if &name[..] == "Func" => Self::StrFunc,
            (Type::Str, Type::Poly { name, .. }) if &name[..] == "Proc" => Self::StrProc,
            // 要素数は検査済みなので、気にする必要はない
            (Type::Poly { name, .. }, Type::Int) if &name[..] == "Array" => Self::ArrayInt,
            (Type::Poly { name, .. }, Type::Nat) if &name[..] == "Array" => Self::ArrayNat,
            (Type::Poly { name, .. }, Type::Float) if &name[..] == "Array" => Self::ArrayFloat,
            (Type::Poly { name, .. }, Type::Str) if &name[..] == "Array" => Self::ArrayStr,
            (Type::Poly { name, .. }, Type::Bool) if &name[..] == "Array" => Self::ArrayBool,
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Array" && &rn[..] == "Array" =>
            {
                Self::ArrayArray
            }
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Array" && &rn[..] == "Func" =>
            {
                Self::ArrayFunc
            }
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Array" && &rn[..] == "Proc" =>
            {
                Self::ArrayProc
            }
            (Type::Poly { name, .. }, Type::Int) if &name[..] == "Func" => Self::FuncInt,
            (Type::Poly { name, .. }, Type::Nat) if &name[..] == "Func" => Self::FuncNat,
            (Type::Poly { name, .. }, Type::Float) if &name[..] == "Func" => Self::FuncFloat,
            (Type::Poly { name, .. }, Type::Str) if &name[..] == "Func" => Self::FuncStr,
            (Type::Poly { name, .. }, Type::Bool) if &name[..] == "Func" => Self::FuncBool,
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Func" && &rn[..] == "Array" =>
            {
                Self::FuncArray
            }
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Func" && &rn[..] == "Func" =>
            {
                Self::FuncFunc
            }
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Func" && &rn[..] == "Proc" =>
            {
                Self::FuncProc
            }
            (Type::Poly { name, .. }, Type::Int) if &name[..] == "Proc" => Self::ProcInt,
            (Type::Poly { name, .. }, Type::Nat) if &name[..] == "Proc" => Self::ProcNat,
            (Type::Poly { name, .. }, Type::Float) if &name[..] == "Proc" => Self::ProcFloat,
            (Type::Poly { name, .. }, Type::Str) if &name[..] == "Proc" => Self::ProcStr,
            (Type::Poly { name, .. }, Type::Bool) if &name[..] == "Proc" => Self::ProcBool,
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Proc" && &rn[..] == "Array" =>
            {
                Self::ProcArray
            }
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Proc" && &rn[..] == "Func" =>
            {
                Self::ProcFunc
            }
            (Type::Poly { name: ln, .. }, Type::Poly { name: rn, .. })
                if &ln[..] == "Proc" && &rn[..] == "Proc" =>
            {
                Self::ProcProc
            }
            (Type::Refinement(refine), r) => Self::new(&*refine.t, r),
            (l, Type::Refinement(refine)) => Self::new(l, &*refine.t),
            (_, _) => Self::Others,
        }
    }
}
