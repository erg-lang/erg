//! defines `Type` (type kind).
//!
//! Type(コンパイラ等で使われる「型」を表現する)を定義する
#![allow(clippy::derive_hash_xor_eq)]
#![allow(clippy::large_enum_variant)]
pub mod codeobj;
pub mod constructors;
pub mod deserialize;
pub mod free;
pub mod typaram;
pub mod value;

use std::fmt;
use std::ops::{Range, RangeInclusive};
use std::path::PathBuf;

use constructors::dict_t;
use erg_common::dict::Dict;
use erg_common::fresh::fresh_varname;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::set::Set;
use erg_common::traits::LimitedDisplay;
use erg_common::vis::Field;
use erg_common::{enum_unwrap, fmt_option, fmt_set_split_with, set, Str};

use erg_parser::ast::{Block, Params};
use erg_parser::token::TokenKind;

use self::constructors::{int_interval, mono, subr_t};
use self::free::{
    CanbeFree, Constraint, Free, FreeKind, FreeTyVar, HasLevel, Level, GENERIC_LEVEL,
};
use self::typaram::{IntervalOp, TyParam};
use self::value::value_set::*;
use self::value::ValueObj::{Inf, NegInf};
use self::value::{EvalValueResult, ValueObj};

use crate::context::Context;

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
        self.ref_t().non_default_params().unwrap()[0].typ()
    }
    #[inline]
    fn rhs_t(&self) -> &Type {
        self.ref_t().non_default_params().unwrap()[1].typ()
    }
}

#[macro_export]
macro_rules! impl_t {
    ($T: ty) => {
        impl $crate::ty::HasType for $T {
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
        impl $crate::ty::HasType for $Enum {
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
    name: Str,
    params: Params,
    block: Block,
    sig_t: Type,
}

impl UserConstSubr {
    pub const fn new(name: Str, params: Params, block: Block, sig_t: Type) -> Self {
        Self {
            name,
            params,
            block,
            sig_t,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueArgs {
    pub pos_args: Vec<ValueObj>,
    pub kw_args: Dict<Str, ValueObj>,
}

impl ValueArgs {
    pub const fn new(pos_args: Vec<ValueObj>, kw_args: Dict<Str, ValueObj>) -> Self {
        ValueArgs { pos_args, kw_args }
    }

    pub fn remove_left_or_key(&mut self, key: &str) -> Option<ValueObj> {
        if !self.pos_args.is_empty() {
            Some(self.pos_args.remove(0))
        } else {
            self.kw_args.remove(key)
        }
    }
}

#[derive(Clone)]
pub struct BuiltinConstSubr {
    name: &'static str,
    subr: fn(ValueArgs, &Context) -> EvalValueResult<ValueObj>,
    sig_t: Type,
    as_type: Option<Type>,
}

impl std::fmt::Debug for BuiltinConstSubr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinConstSubr")
            .field("name", &self.name)
            .field("sig_t", &self.sig_t)
            .field("as_type", &self.as_type)
            .finish()
    }
}

impl PartialEq for BuiltinConstSubr {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for BuiltinConstSubr {}

impl std::hash::Hash for BuiltinConstSubr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl fmt::Display for BuiltinConstSubr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<built-in const subroutine '{}'>", self.name)
    }
}

impl BuiltinConstSubr {
    pub const fn new(
        name: &'static str,
        subr: fn(ValueArgs, &Context) -> EvalValueResult<ValueObj>,
        sig_t: Type,
        as_type: Option<Type>,
    ) -> Self {
        Self {
            name,
            subr,
            sig_t,
            as_type,
        }
    }

    pub fn call(&self, args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
        (self.subr)(args, ctx)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstSubr {
    User(UserConstSubr),
    Builtin(BuiltinConstSubr),
}

impl fmt::Display for ConstSubr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstSubr::User(subr) => {
                write!(f, "<user-defined const subroutine '{}'>", subr.name)
            }
            ConstSubr::Builtin(subr) => write!(f, "{subr}"),
        }
    }
}

impl ConstSubr {
    pub fn sig_t(&self) -> &Type {
        match self {
            ConstSubr::User(user) => &user.sig_t,
            ConstSubr::Builtin(builtin) => &builtin.sig_t,
        }
    }

    /// ConstSubr{sig_t: Int -> {Int}, ..}.as_type() == Int -> Int
    pub fn as_type(&self) -> Option<Type> {
        match self {
            ConstSubr::User(user) => {
                let Type::Subr(subr) = &user.sig_t else { return None };
                if let Type::Refinement(refine) = subr.return_t.as_ref() {
                    if refine.preds.len() == 1 {
                        let pred = refine.preds.iter().next().unwrap().clone();
                        if let Predicate::Equal { rhs, .. } = pred {
                            let return_t = Type::try_from(rhs).ok()?;
                            let var_params = subr.var_params.as_ref().map(|t| t.as_ref());
                            return Some(subr_t(
                                subr.kind,
                                subr.non_default_params.clone(),
                                var_params.cloned(),
                                subr.default_params.clone(),
                                return_t,
                            ));
                        }
                    }
                }
                None
            }
            ConstSubr::Builtin(builtin) => builtin.as_type.clone(),
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

    fn set_level(&self, level: usize) {
        match self {
            Self::Value(_) | Self::Const(_) => {}
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => {
                rhs.set_level(level);
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.set_level(level);
                rhs.set_level(level);
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

    pub fn is_equal(&self) -> bool {
        matches!(self, Self::Equal { .. })
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

    pub fn qvars(&self) -> Set<(Str, Constraint)> {
        match self {
            Self::Value(_) | Self::Const(_) => set! {},
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.qvars(),
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.qvars().concat(rhs.qvars())
            }
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

    pub fn is_cachable(&self) -> bool {
        match self {
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.is_cachable(),
            Self::Or(lhs, rhs) | Self::And(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.is_cachable() && rhs.is_cachable()
            }
            _ => true,
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
pub enum ParamTy {
    Pos { name: Option<Str>, ty: Type },
    Kw { name: Str, ty: Type },
    KwWithDefault { name: Str, ty: Type, default: Type },
}

impl fmt::Display for ParamTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pos { name, ty } => {
                if let Some(name) = name {
                    write!(f, "{}", name)?;
                }
                write!(f, ": {}", ty)
            }
            Self::Kw { name, ty } => write!(f, "{}: {}", name, ty),
            Self::KwWithDefault { name, ty, default } => {
                write!(f, "{}: {} := {}", name, ty, default)
            }
        }
    }
}

impl ParamTy {
    pub const fn pos(name: Option<Str>, ty: Type) -> Self {
        Self::Pos { name, ty }
    }

    pub const fn kw(name: Str, ty: Type) -> Self {
        Self::Kw { name, ty }
    }

    pub const fn kw_default(name: Str, ty: Type, default: Type) -> Self {
        Self::KwWithDefault { name, ty, default }
    }

    pub const fn anonymous(ty: Type) -> Self {
        Self::pos(None, ty)
    }

    pub fn name(&self) -> Option<&Str> {
        match self {
            Self::Pos { name, .. } => name.as_ref(),
            Self::Kw { name, .. } | Self::KwWithDefault { name, .. } => Some(name),
        }
    }

    pub const fn typ(&self) -> &Type {
        match self {
            Self::Pos { ty, .. } | Self::Kw { ty, .. } | Self::KwWithDefault { ty, .. } => ty,
        }
    }

    pub fn typ_mut(&mut self) -> &mut Type {
        match self {
            Self::Pos { ty, .. } | Self::Kw { ty, .. } | Self::KwWithDefault { ty, .. } => ty,
        }
    }

    pub fn deconstruct(self) -> (Option<Str>, Type, Option<Type>) {
        match self {
            Self::Pos { name, ty } => (name, ty, None),
            Self::Kw { name, ty } => (Some(name), ty, None),
            Self::KwWithDefault { name, ty, default } => (Some(name), ty, Some(default)),
        }
    }
}

/// e.g.
/// (x: Int, ?base: Int) -> Int
/// => SubrTy{ kind: Func, non_default_params: [x: Int], default_params: [base: Int] return_t: Int }
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubrType {
    pub kind: SubrKind,
    pub non_default_params: Vec<ParamTy>,
    pub var_params: Option<Box<ParamTy>>,
    pub default_params: Vec<ParamTy>,
    // var_kw_params: Option<(Str, Box<Type>)>,
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
        write!(f, "(")?;
        for (i, param) in self.non_default_params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", fmt_option!(param.name(), post ": "))?;
            param.typ().limited_fmt(f, limit - 1)?;
        }
        if let Some(var_params) = &self.var_params {
            if !self.non_default_params.is_empty() {
                write!(f, ", ")?;
            }
            write!(f, "...")?;
            var_params.typ().limited_fmt(f, limit - 1)?;
        }
        for pt in self.default_params.iter() {
            write!(f, ", {} := ", pt.name().unwrap())?;
            pt.typ().limited_fmt(f, limit - 1)?;
        }
        write!(f, ") {} ", self.kind.arrow())?;
        self.return_t.limited_fmt(f, limit - 1)
    }
}

impl SubrType {
    pub fn new(
        kind: SubrKind,
        non_default_params: Vec<ParamTy>,
        var_params: Option<ParamTy>,
        default_params: Vec<ParamTy>,
        return_t: Type,
    ) -> Self {
        Self {
            kind,
            non_default_params,
            var_params: var_params.map(Box::new),
            default_params,
            return_t: Box::new(return_t),
        }
    }

    pub fn contains_tvar(&self, name: &str) -> bool {
        self.non_default_params
            .iter()
            .any(|pt| pt.typ().contains_tvar(name))
            || self
                .var_params
                .as_ref()
                .map(|pt| pt.typ().contains_tvar(name))
                .unwrap_or(false)
            || self
                .default_params
                .iter()
                .any(|pt| pt.typ().contains_tvar(name))
            || self.return_t.contains_tvar(name)
    }

    pub fn qvars(&self) -> Set<(Str, Constraint)> {
        let mut qvars = Set::new();
        for pt in self.non_default_params.iter() {
            qvars.extend(pt.typ().qvars());
        }
        if let Some(var_params) = &self.var_params {
            qvars.extend(var_params.typ().qvars());
        }
        for pt in self.default_params.iter() {
            qvars.extend(pt.typ().qvars());
        }
        qvars.extend(self.return_t.qvars());
        qvars
    }

    pub fn has_qvar(&self) -> bool {
        self.non_default_params.iter().any(|pt| pt.typ().has_qvar())
            || self
                .var_params
                .as_ref()
                .map(|pt| pt.typ().has_qvar())
                .unwrap_or(false)
            || self.default_params.iter().any(|pt| pt.typ().has_qvar())
            || self.return_t.has_qvar()
    }

    pub fn typarams(&self) -> Vec<TyParam> {
        [
            self.non_default_params
                .iter()
                .map(|pt| TyParam::t(pt.typ().clone()))
                .collect::<Vec<_>>(),
            self.var_params
                .as_ref()
                .map(|pt| TyParam::t(pt.typ().clone()))
                .into_iter()
                .collect(),
            self.default_params
                .iter()
                .map(|pt| TyParam::t(pt.typ().clone()))
                .collect(),
        ]
        .concat()
    }

    pub fn self_t(&self) -> Option<&Type> {
        self.non_default_params.first().and_then(|p| {
            if p.name()
                .map(|n| &n[..] == "self" || &n[..] == "Self")
                .unwrap_or(false)
            {
                Some(p.typ())
            } else {
                None
            }
        })
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
        let first_subj = self.preds.iter().next().and_then(|p| p.subject());
        let is_simple_type = self.t.is_simple_class();
        let is_simple_preds = self
            .preds
            .iter()
            .all(|p| p.is_equal() && p.subject() == first_subj);
        if is_simple_type && is_simple_preds {
            write!(f, "{{")?;
            for pred in self.preds.iter() {
                let (_, rhs) = enum_unwrap!(pred, Predicate::Equal { lhs, rhs });
                write!(f, "{}, ", rhs)?;
            }
            write!(f, "}}")
        } else {
            write!(f, "{{{}: ", self.var)?;
            self.t.limited_fmt(f, limit - 1)?;
            write!(f, " | {}}}", fmt_set_split_with(&self.preds, "; "))
        }
    }
}

impl RefinementType {
    pub fn new(var: Str, t: Type, preds: Set<Predicate>) -> Self {
        match t.deconstruct_refinement() {
            Ok((inner_var, inner_t, inner_preds)) => {
                let new_preds = preds
                    .into_iter()
                    .map(|pred| pred.change_subject_name(inner_var.clone()))
                    .collect::<Set<_>>();
                Self {
                    var: inner_var,
                    t: Box::new(inner_t),
                    preds: inner_preds.concat(new_preds),
                }
            }
            Err(t) => Self {
                var,
                t: Box::new(t),
                preds,
            },
        }
    }

    pub fn deconstruct(self) -> (Str, Type, Set<Predicate>) {
        (self.var, *self.t, self.preds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubrKind {
    Func,
    Proc,
}

impl From<TokenKind> for SubrKind {
    fn from(op_kind: TokenKind) -> Self {
        match op_kind {
            TokenKind::FuncArrow => Self::Func,
            TokenKind::ProcArrow => Self::Proc,
            _ => panic!("invalid token kind for subr kind"),
        }
    }
}

impl SubrKind {
    pub const fn arrow(&self) -> Str {
        match self {
            Self::Func => Str::ever("->"),
            Self::Proc => Str::ever("=>"),
        }
    }

    pub fn is_func(&self) -> bool {
        matches!(self, Self::Func)
    }
    pub fn is_proc(&self) -> bool {
        matches!(self, Self::Proc)
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
pub struct ArgsOwnership {
    pub non_defaults: Vec<(Option<Str>, Ownership)>,
    pub var_params: Option<(Str, Ownership)>,
    pub defaults: Vec<(Str, Ownership)>,
}

impl fmt::Display for ArgsOwnership {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(")?;
        for (i, (name, o)) in self.non_defaults.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            if let Some(name) = name {
                write!(f, "{name}: {o:?}")?;
            } else {
                write!(f, "{o:?}")?;
            }
        }
        if let Some((name, o)) = self.var_params.as_ref() {
            write!(f, ", ...{name}: {o:?}")?;
        }
        for (name, o) in self.defaults.iter() {
            write!(f, ", {name} := {o:?}")?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

impl ArgsOwnership {
    pub const fn new(
        non_defaults: Vec<(Option<Str>, Ownership)>,
        var_params: Option<(Str, Ownership)>,
        defaults: Vec<(Str, Ownership)>,
    ) -> Self {
        Self {
            non_defaults,
            var_params,
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
    Frame,
    Error,
    Inf,    // {∞}
    NegInf, // {-∞}
    // TODO: PolyType/Class
    Type,
    ClassType,
    TraitType,
    Patch,
    NotImplemented,
    Ellipsis,  // これはクラスのほうで型推論用のマーカーではない
    Never,     // {}
    Mono(Str), // the name is fully qualified (e.g. <module>::C, foo.D)
    /* Polymorphic types */
    Ref(Box<Type>),
    RefMut {
        before: Box<Type>,
        after: Option<Box<Type>>,
    },
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
    Quantified(Box<Type>),
    And(Box<Type>, Box<Type>),
    Not(Box<Type>, Box<Type>),
    Or(Box<Type>, Box<Type>),
    Poly {
        name: Str,
        params: Vec<TyParam>,
    },
    /* Special types (inference-time types) */
    Proj {
        lhs: Box<Type>,
        rhs: Str,
    }, // e.g. T.U
    ProjCall {
        lhs: Box<TyParam>,
        attr_name: Str,
        args: Vec<TyParam>,
    }, // e.g. Ts.__getitem__(N)
    FreeVar(FreeTyVar), // a reference to the type of other expression, see docs/compiler/inference.md
    Failure,            // indicates a failure of type inference and behaves as `Never`.
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
            | (Self::Frame, Self::Frame)
            | (Self::Error, Self::Error)
            | (Self::Inf, Self::Inf)
            | (Self::NegInf, Self::NegInf)
            | (Self::Type, Self::Type)
            | (Self::ClassType, Self::ClassType)
            | (Self::TraitType, Self::TraitType)
            | (Self::Patch, Self::Patch)
            | (Self::NotImplemented, Self::NotImplemented)
            | (Self::Ellipsis, Self::Ellipsis)
            | (Self::Never, Self::Never) => true,
            (Self::Mono(l), Self::Mono(r)) => l == r,
            (Self::Ref(l), Self::Ref(r)) => l == r,
            (
                Self::RefMut {
                    before: l1,
                    after: l2,
                },
                Self::RefMut {
                    before: r1,
                    after: r2,
                },
            ) => l1 == r1 && l2 == r2,
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
                },
                Self::Poly {
                    name: rn,
                    params: rps,
                },
            ) => ln == rn && lps == rps,
            (
                Self::Proj { lhs, rhs },
                Self::Proj {
                    lhs: rlhs,
                    rhs: rrhs,
                },
            ) => lhs == rlhs && rhs == rrhs,
            (
                Self::ProjCall {
                    lhs,
                    attr_name,
                    args,
                },
                Self::ProjCall {
                    lhs: r,
                    attr_name: rn,
                    args: ra,
                },
            ) => lhs == r && attr_name == rn && args == ra,
            (Self::FreeVar(fv), other) if fv.is_linked() => &*fv.crack() == other,
            (_self, Self::FreeVar(fv)) if fv.is_linked() => _self == &*fv.crack(),
            (Self::FreeVar(l), Self::FreeVar(r)) => l == r,
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
            Self::FreeVar(fv) => fv.limited_fmt(f, limit),
            Self::Mono(name) => write!(f, "{name}"),
            Self::Ref(t) => {
                write!(f, "{}(", self.qual_name())?;
                t.limited_fmt(f, limit - 1)?;
                write!(f, ")")
            }
            Self::RefMut { before, after } => {
                write!(f, "{}(", self.qual_name())?;
                before.limited_fmt(f, limit - 1)?;
                if let Some(after) = after {
                    write!(f, " ~> ")?;
                    after.limited_fmt(f, limit - 1)?;
                }
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
            Self::Quantified(quantified) => {
                let qvars = quantified.qvars();
                if limit == 0 {
                    return write!(f, "...");
                }
                write!(f, "|")?;
                for (i, (name, constr)) in qvars.iter().enumerate() {
                    if i != 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{name}")?;
                    constr.limited_fmt(f, limit - 1)?;
                }
                write!(f, "|")?;
                quantified.limited_fmt(f, limit - 1)
            }
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
                write!(f, "(")?;
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " or ")?;
                rhs.limited_fmt(f, limit - 1)?;
                write!(f, ")")
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
            Self::Proj { lhs, rhs } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, ".{rhs}")
            }
            Self::ProjCall {
                lhs,
                attr_name,
                args,
            } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, ".{attr_name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    arg.limited_fmt(f, limit - 1)?;
                }
                write!(f, ")")
            }
            _ => write!(f, "{}", self.qual_name()),
        }
    }
}

impl CanbeFree for Type {
    fn unbound_name(&self) -> Option<Str> {
        if let Type::FreeVar(fv) = self {
            fv.unbound_name()
        } else {
            None
        }
    }

    fn constraint(&self) -> Option<Constraint> {
        if let Type::FreeVar(fv) = self {
            fv.constraint()
        } else {
            None
        }
    }

    fn update_constraint(&self, new_constraint: Constraint) {
        if let Self::FreeVar(fv) = self {
            fv.update_constraint(new_constraint);
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
        int_interval(IntervalOp::RightOpen, r.start, r.end)
    }
}

impl From<Range<&TyParam>> for Type {
    fn from(r: Range<&TyParam>) -> Self {
        int_interval(IntervalOp::RightOpen, r.start.clone(), r.end.clone())
    }
}

impl From<RangeInclusive<TyParam>> for Type {
    fn from(r: RangeInclusive<TyParam>) -> Self {
        let (start, end) = r.into_inner();
        int_interval(IntervalOp::Closed, start, end)
    }
}

impl From<RangeInclusive<&TyParam>> for Type {
    fn from(r: RangeInclusive<&TyParam>) -> Self {
        let (start, end) = r.into_inner();
        int_interval(IntervalOp::Closed, start.clone(), end.clone())
    }
}

impl From<Dict<Type, Type>> for Type {
    fn from(d: Dict<Type, Type>) -> Self {
        let d = d
            .into_iter()
            .map(|(k, v)| (TyParam::t(k), TyParam::t(v)))
            .collect();
        dict_t(TyParam::Dict(d))
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
            Self::Ref(t) => {
                vec![t.as_ref().clone()]
            }
            Self::RefMut { before, .. } => {
                // REVIEW:
                vec![before.as_ref().clone()]
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
    fn level(&self) -> Option<usize> {
        match self {
            Self::FreeVar(v) => v.level(),
            Self::Ref(t) => t.level(),
            Self::RefMut { before, after } => {
                let bl = before.level();
                if let Some(after) = after {
                    bl.zip(after.level()).map(|(a, b)| a.min(b))
                } else {
                    bl
                }
            }
            Self::Callable { param_ts, return_t } => {
                let min = param_ts
                    .iter()
                    .filter_map(|t| t.level())
                    .min()
                    .unwrap_or(GENERIC_LEVEL);
                let min = return_t.level().unwrap_or(GENERIC_LEVEL).min(min);
                if min == GENERIC_LEVEL {
                    None
                } else {
                    Some(min)
                }
            }
            Self::Subr(subr) => {
                let nd_min = subr
                    .non_default_params
                    .iter()
                    .filter_map(|p| p.typ().level())
                    .min();
                let v_min = subr.var_params.iter().filter_map(|p| p.typ().level()).min();
                let d_min = subr
                    .default_params
                    .iter()
                    .filter_map(|p| p.typ().level())
                    .min();
                let ret_min = subr.return_t.level();
                [nd_min, v_min, d_min, ret_min]
                    .iter()
                    .filter_map(|o| *o)
                    .min()
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                let l = lhs
                    .level()
                    .unwrap_or(GENERIC_LEVEL)
                    .min(rhs.level().unwrap_or(GENERIC_LEVEL));
                if l == GENERIC_LEVEL {
                    None
                } else {
                    Some(l)
                }
            }
            Self::Record(attrs) => attrs.values().filter_map(|t| t.level()).min(),
            Self::Poly { params, .. } => params.iter().filter_map(|p| p.level()).min(),
            Self::Proj { lhs, .. } => lhs.level(),
            Self::ProjCall { lhs, args, .. } => {
                let lev = lhs.level().unwrap_or(GENERIC_LEVEL);
                let min = args
                    .iter()
                    .filter_map(|tp| tp.level())
                    .min()
                    .unwrap_or(GENERIC_LEVEL);
                let min = lev.min(min);
                if min == GENERIC_LEVEL {
                    None
                } else {
                    Some(min)
                }
            }
            Self::Refinement(refine) => {
                let lev = refine.t.level().unwrap_or(GENERIC_LEVEL);
                let min = refine
                    .preds
                    .iter()
                    .filter_map(|p| p.level())
                    .min()
                    .unwrap_or(GENERIC_LEVEL);
                let min = lev.min(min);
                if min == GENERIC_LEVEL {
                    None
                } else {
                    Some(min)
                }
            }
            Self::Quantified(quant) => quant.level(),
            _ => None,
        }
    }

    fn set_level(&self, level: Level) {
        match self {
            Self::FreeVar(v) => v.set_level(level),
            Self::Ref(t) => t.set_level(level),
            Self::RefMut { before, after } => {
                before.set_level(level);
                if let Some(after) = after {
                    after.set_level(level);
                }
            }
            Self::Callable { param_ts, return_t } => {
                for p in param_ts.iter() {
                    p.set_level(level);
                }
                return_t.set_level(level);
            }
            Self::Subr(subr) => {
                for pt in subr.non_default_params.iter() {
                    pt.typ().set_level(level);
                }
                if let Some(pt) = subr.var_params.as_ref() {
                    pt.typ().set_level(level);
                }
                for pt in subr.default_params.iter() {
                    pt.typ().set_level(level);
                }
                subr.return_t.set_level(level);
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) | Self::Not(lhs, rhs) => {
                lhs.set_level(level);
                rhs.set_level(level);
            }
            Self::Record(attrs) => {
                for t in attrs.values() {
                    t.set_level(level);
                }
            }
            Self::Poly { params, .. } => {
                for p in params.iter() {
                    p.set_level(level);
                }
            }
            Self::Proj { lhs, .. } => {
                lhs.set_level(level);
            }
            Self::Refinement(refine) => {
                refine.t.set_level(level);
                for pred in refine.preds.iter() {
                    pred.set_level(level);
                }
            }
            Self::Quantified(quant) => {
                quant.set_level(level);
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

    /// 本来は型環境が必要
    pub fn mutate(self) -> Self {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                fv.link(&t.mutate());
                Self::FreeVar(fv)
            }
            Self::Int => mono("Int!"),
            Self::Nat => mono("Nat!"),
            Self::Ratio => mono("Ratio!"),
            Self::Float => mono("Float!"),
            Self::Bool => mono("Bool!"),
            Self::Str => mono("Str!"),
            other if other.is_mut_type() => other,
            _t => todo!("{_t}"),
        }
    }

    pub fn quantify(self) -> Self {
        Self::Quantified(Box::new(self))
    }

    pub fn is_simple_class(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_simple_class(),
            Self::Obj
            | Self::Int
            | Self::Nat
            | Self::Ratio
            | Self::Float
            | Self::Bool
            | Self::Str
            | Self::NoneType
            | Self::Code
            | Self::Frame
            | Self::Error
            | Self::Inf
            | Self::NegInf
            | Self::Type
            | Self::ClassType
            | Self::TraitType
            | Self::Patch
            | Self::NotImplemented
            | Self::Ellipsis
            | Self::Never => true,
            _ => false,
        }
    }

    /// Procedure
    pub fn is_procedure(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_procedure(),
            Self::Callable { .. } => true,
            Self::Quantified(t) => t.is_procedure(),
            Self::Subr(subr) if subr.kind == SubrKind::Proc => true,
            Self::Refinement(refine) =>
                refine.t.is_procedure() || refine.preds.iter().any(|pred|
                    matches!(pred, Predicate::Equal{ rhs, .. } if pred.mentions(&refine.var) && rhs.qual_name().map(|n| n.ends_with('!')).unwrap_or(false))
                ),
            _ => false,
        }
    }

    pub fn is_mut_type(&self) -> bool {
        match self {
            Self::FreeVar(fv) => {
                if fv.is_linked() {
                    fv.crack().is_mut_type()
                } else {
                    fv.unbound_name().unwrap().ends_with('!')
                }
            }
            Self::Mono(name) | Self::Poly { name, .. } | Self::Proj { rhs: name, .. } => {
                name.ends_with('!')
            }
            Self::Refinement(refine) => refine.t.is_mut_type(),
            _ => false,
        }
    }

    pub fn is_nonelike(&self) -> bool {
        match self {
            Self::Never | Self::Failure => true,
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_nonelike(),
            Self::NoneType => true,
            Self::Poly { name, params, .. } if &name[..] == "Option" || &name[..] == "Option!" => {
                let inner_t = enum_unwrap!(params.first().unwrap(), TyParam::Type);
                inner_t.is_nonelike()
            }
            Self::Poly { name, params, .. } if &name[..] == "Tuple" => params.is_empty(),
            Self::Refinement(refine) => refine.t.is_nonelike(),
            _ => false,
        }
    }

    pub fn is_intersection_type(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_intersection_type(),
            Self::Or(_, _) => true,
            Self::Refinement(refine) => refine.t.is_intersection_type(),
            _ => false,
        }
    }

    pub fn is_refinement(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_refinement(),
            Self::Refinement(_) => true,
            _ => false,
        }
    }

    pub fn is_record(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_record(),
            Self::Record(_) => true,
            Self::Refinement(refine) => refine.t.is_record(),
            _ => false,
        }
    }

    pub fn is_module(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_module(),
            Self::Refinement(refine) => refine.t.is_module(),
            Self::Poly { name, .. } => {
                &name[..] == "PyModule" || &name[..] == "Module" || &name[..] == "ModuleType"
            }
            _ => false,
        }
    }

    pub fn is_py_module(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_py_module(),
            Self::Refinement(refine) => refine.t.is_py_module(),
            Self::Poly { name, .. } => &name[..] == "PyModule" || &name[..] == "ModuleType",
            _ => false,
        }
    }

    pub fn is_quantified(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_quantified(),
            Self::Quantified(_) => true,
            Self::Refinement(refine) => refine.t.is_quantified(),
            _ => false,
        }
    }

    pub fn contains_tvar(&self, name: &str) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_tvar(name),
            Self::FreeVar(fv) if fv.constraint_is_typeof() => {
                fv.unbound_name().map(|n| &n[..] == name).unwrap_or(false)
            }
            Self::FreeVar(fv) => {
                fv.unbound_name().map(|n| &n[..] == name).unwrap_or(false)
                    || fv
                        .get_subsup()
                        .map(|(sub, sup)| sub.contains_tvar(name) || sup.contains_tvar(name))
                        .unwrap_or(false)
            }
            Self::Poly { params, .. } => {
                for param in params.iter() {
                    match param {
                        TyParam::Type(t) if t.contains_tvar(name) => {
                            return true;
                        }
                        _ => {}
                    }
                }
                false
            }
            Self::Subr(subr) => subr.contains_tvar(name),
            // TODO: preds
            Self::Refinement(refine) => refine.t.contains_tvar(name),
            _ => false,
        }
    }

    pub fn args_ownership(&self) -> ArgsOwnership {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().args_ownership(),
            Self::Refinement(refine) => refine.t.args_ownership(),
            Self::Subr(subr) => {
                let mut nd_args = vec![];
                for nd_param in subr.non_default_params.iter() {
                    let ownership = match nd_param.typ() {
                        Self::Ref(_) => Ownership::Ref,
                        Self::RefMut { .. } => Ownership::RefMut,
                        _ => Ownership::Owned,
                    };
                    nd_args.push((nd_param.name().cloned(), ownership));
                }
                let var_args = subr
                    .var_params
                    .as_ref()
                    .map(|t| (t.name().unwrap().clone(), t.typ().ownership()));
                let mut d_args = vec![];
                for d_param in subr.default_params.iter() {
                    let ownership = match d_param.typ() {
                        Self::Ref(_) => Ownership::Ref,
                        Self::RefMut { .. } => Ownership::RefMut,
                        _ => Ownership::Owned,
                    };
                    d_args.push((d_param.name().unwrap().clone(), ownership));
                }
                ArgsOwnership::new(nd_args, var_args, d_args)
            }
            _ => todo!(),
        }
    }

    pub fn ownership(&self) -> Ownership {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().ownership(),
            Self::Refinement(refine) => refine.t.ownership(),
            Self::Ref(_) => Ownership::Ref,
            Self::RefMut { .. } => Ownership::RefMut,
            _ => Ownership::Owned,
        }
    }

    pub fn qual_name(&self) -> Str {
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
            Self::ClassType => Str::ever("ClassType"),
            Self::TraitType => Str::ever("TraitType"),
            Self::Patch => Str::ever("Patch"),
            Self::Code => Str::ever("Code"),
            Self::Frame => Str::ever("Frame"),
            Self::Error => Str::ever("Error"),
            Self::Inf => Str::ever("Inf"),
            Self::NegInf => Str::ever("NegInf"),
            Self::Mono(name) => name.clone(),
            Self::And(_, _) => Str::ever("And"),
            Self::Not(_, _) => Str::ever("Not"),
            Self::Or(_, _) => Str::ever("Or"),
            Self::Ref(_) => Str::ever("Ref"),
            Self::RefMut { .. } => Str::ever("RefMut"),
            Self::Subr(SubrType {
                kind: SubrKind::Func,
                ..
            }) => Str::ever("Func"),
            Self::Subr(SubrType {
                kind: SubrKind::Proc,
                ..
            }) => Str::ever("Proc"),
            Self::Callable { .. } => Str::ever("Callable"),
            Self::Record(_) => Str::ever("Record"),
            Self::Poly { name, .. } => name.clone(),
            // NOTE: compiler/codegen/convert_to_python_methodでクラス名を使うため、こうすると都合が良い
            Self::Refinement(refine) => refine.t.qual_name(),
            Self::Quantified(_) => Str::ever("Quantified"),
            Self::Ellipsis => Str::ever("Ellipsis"),
            Self::NotImplemented => Str::ever("NotImplemented"),
            Self::Never => Str::ever("Never"),
            Self::FreeVar(fv) => match &*fv.borrow() {
                FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t.qual_name(),
                FreeKind::NamedUnbound { name, .. } => name.clone(),
                FreeKind::Unbound { id, .. } => Str::from(format!("%{id}")),
            },
            Self::Proj { .. } => Str::ever("Proj"),
            Self::ProjCall { .. } => Str::ever("ProjCall"),
            Self::Failure => Str::ever("Failure"),
            Self::Uninited => Str::ever("Uninited"),
        }
    }

    pub fn local_name(&self) -> Str {
        match self {
            Self::Mono(name) | Self::Poly { name, .. } => {
                let namespaces = name.split_with(&[".", "::"]);
                Str::rc(namespaces.last().unwrap())
            }
            _ => self.qual_name(),
        }
    }

    /// assert!((A and B).contains_intersec(B))
    pub fn contains_intersec(&self, typ: &Type) -> bool {
        match self {
            Type::And(t1, t2) => t1.contains_intersec(typ) || t2.contains_intersec(typ),
            _ => self == typ,
        }
    }

    pub fn union_types(&self) -> Option<(Type, Type)> {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => fv.crack().union_types(),
            Type::Refinement(refine) => refine.t.union_types(),
            Type::Or(t1, t2) => Some((*t1.clone(), *t2.clone())),
            _ => None,
        }
    }

    /// assert!((A or B).contains_union(B))
    pub fn contains_union(&self, typ: &Type) -> bool {
        match self {
            Type::Or(t1, t2) => t1.contains_union(typ) || t2.contains_union(typ),
            _ => self == typ,
        }
    }

    pub fn tvar_name(&self) -> Option<Str> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().tvar_name(),
            Self::FreeVar(fv) => fv.unbound_name(),
            _ => None,
        }
    }

    pub fn q_constraint(&self) -> Option<Constraint> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                fv.forced_as_ref().linked().unwrap().q_constraint()
            }
            Self::FreeVar(fv) if fv.is_generalized() => fv.constraint(),
            _ => None,
        }
    }

    pub const fn is_free_var(&self) -> bool {
        matches!(self, Self::FreeVar(_))
    }

    pub const fn is_callable(&self) -> bool {
        matches!(self, Self::Subr { .. } | Self::Callable { .. })
    }

    pub fn is_unbound_var(&self) -> bool {
        matches!(self, Self::FreeVar(fv) if fv.is_unbound() || fv.crack().is_unbound_var())
    }

    /// See also: `is_monomorphized`
    pub fn is_monomorphic(&self) -> bool {
        matches!(self.typarams_len(), Some(0) | None)
    }

    /// `Set(Int, 3)` is not monomorphic but monomorphized
    pub fn is_monomorphized(&self) -> bool {
        matches!(self.typarams_len(), Some(0) | None)
            || (self.has_no_qvar() && self.has_no_unbound_var())
    }

    pub fn into_refinement(self) -> RefinementType {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => fv.crack().clone().into_refinement(),
            Type::Nat => {
                let var = Str::from(fresh_varname());
                RefinementType::new(
                    var.clone(),
                    Type::Int,
                    set! {Predicate::ge(var, TyParam::value(0))},
                )
            }
            Type::Bool => {
                let var = Str::from(fresh_varname());
                RefinementType::new(
                    var.clone(),
                    Type::Int,
                    set! {Predicate::ge(var.clone(), TyParam::value(true)), Predicate::le(var, TyParam::value(false))},
                )
            }
            Type::Refinement(r) => r,
            t => {
                let var = Str::from(fresh_varname());
                RefinementType::new(var, t, set! {})
            }
        }
    }

    pub fn deconstruct_refinement(self) -> Result<(Str, Type, Set<Predicate>), Type> {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => fv.crack().clone().deconstruct_refinement(),
            Type::Refinement(r) => Ok(r.deconstruct()),
            _ => Err(self),
        }
    }

    pub fn qvars(&self) -> Set<(Str, Constraint)> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.forced_as_ref().linked().unwrap().qvars(),
            Self::FreeVar(fv) if !fv.constraint_is_uninited() => set! {
                (fv.unbound_name().unwrap(), fv.constraint().unwrap())
            },
            Self::Ref(t) => t.qvars(),
            Self::RefMut { before, after } => before
                .qvars()
                .concat(after.as_ref().map(|t| t.qvars()).unwrap_or_else(|| set! {})),
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.qvars().concat(rhs.qvars())
            }
            Self::Callable { param_ts, return_t } => param_ts
                .iter()
                .fold(set! {}, |acc, t| acc.concat(t.qvars()))
                .concat(return_t.qvars()),
            Self::Subr(subr) => subr.qvars(),
            Self::Record(r) => r.values().fold(set! {}, |acc, t| acc.concat(t.qvars())),
            Self::Refinement(refine) => refine.t.qvars().concat(
                refine
                    .preds
                    .iter()
                    .fold(set! {}, |acc, pred| acc.concat(pred.qvars())),
            ),
            Self::Quantified(quant) => quant.qvars(),
            Self::Poly { params, .. } => params
                .iter()
                .fold(set! {}, |acc, tp| acc.concat(tp.qvars())),
            Self::Proj { lhs, .. } => lhs.qvars(),
            Self::ProjCall { lhs, args, .. } => lhs
                .qvars()
                .concat(args.iter().fold(set! {}, |acc, tp| acc.concat(tp.qvars()))),
            _ => set! {},
        }
    }

    /// if the type is polymorphic
    pub fn has_qvar(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_generalized() => true,
            Self::FreeVar(fv) => {
                if fv.is_unbound() {
                    if let Some((sub, sup)) = fv.get_subsup() {
                        fv.undoable_link(&Type::Obj);
                        let res_sub = sub.has_qvar();
                        let res_sup = sup.has_qvar();
                        fv.undo();
                        res_sub || res_sup
                    } else {
                        let opt_t = fv.get_type();
                        opt_t.map(|t| t.has_qvar()).unwrap_or(false)
                    }
                } else {
                    fv.crack().has_qvar()
                }
            }
            Self::Ref(t) => t.has_qvar(),
            Self::RefMut { before, after } => {
                before.has_qvar() || after.as_ref().map(|t| t.has_qvar()).unwrap_or(false)
            }
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.has_qvar() || rhs.has_qvar()
            }
            Self::Callable { param_ts, return_t } => {
                param_ts.iter().any(|t| t.has_qvar()) || return_t.has_qvar()
            }
            Self::Subr(subr) => subr.has_qvar(),
            Self::Record(r) => r.values().any(|t| t.has_qvar()),
            Self::Refinement(refine) => {
                refine.t.has_qvar() || refine.preds.iter().any(|pred| pred.has_qvar())
            }
            Self::Quantified(quant) => quant.has_qvar(),
            Self::Poly { params, .. } => params.iter().any(|tp| tp.has_qvar()),
            Self::Proj { lhs, .. } => lhs.has_qvar(),
            Self::ProjCall { lhs, args, .. } => {
                lhs.has_qvar() || args.iter().any(|tp| tp.has_qvar())
            }
            _ => false,
        }
    }

    pub fn has_no_qvar(&self) -> bool {
        !self.has_qvar()
    }

    pub fn is_cachable(&self) -> bool {
        match self {
            Self::FreeVar(_) => false,
            Self::Ref(t) => t.is_cachable(),
            Self::RefMut { before, after } => {
                before.is_cachable() && after.as_ref().map(|t| t.is_cachable()).unwrap_or(true)
            }
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.is_cachable() && rhs.is_cachable()
            }
            Self::Callable { param_ts, return_t } => {
                param_ts.iter().all(|t| t.is_cachable()) && return_t.is_cachable()
            }
            Self::Subr(subr) => {
                subr.non_default_params
                    .iter()
                    .all(|pt| pt.typ().is_cachable())
                    && subr
                        .var_params
                        .as_ref()
                        .map(|pt| pt.typ().is_cachable())
                        .unwrap_or(false)
                    && subr.default_params.iter().all(|pt| pt.typ().is_cachable())
                    && subr.return_t.is_cachable()
            }
            Self::Record(r) => r.values().all(|t| t.is_cachable()),
            Self::Refinement(refine) => {
                refine.t.is_cachable() && refine.preds.iter().all(|p| p.is_cachable())
            }
            Self::Quantified(quant) => quant.is_cachable(),
            Self::Poly { params, .. } => params.iter().all(|p| p.is_cachable()),
            Self::Proj { lhs, .. } => lhs.is_cachable(),
            _ => true,
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
            Self::Ref(t) => t.has_unbound_var(),
            Self::RefMut { before, after } => {
                before.has_unbound_var()
                    || after.as_ref().map(|t| t.has_unbound_var()).unwrap_or(false)
            }
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.has_unbound_var() || rhs.has_unbound_var()
            }
            Self::Callable { param_ts, return_t } => {
                param_ts.iter().any(|t| t.has_unbound_var()) || return_t.has_unbound_var()
            }
            Self::Subr(subr) => {
                subr.non_default_params
                    .iter()
                    .any(|pt| pt.typ().has_unbound_var())
                    || subr
                        .var_params
                        .as_ref()
                        .map(|pt| pt.typ().has_unbound_var())
                        .unwrap_or(false)
                    || subr
                        .default_params
                        .iter()
                        .any(|pt| pt.typ().has_unbound_var())
                    || subr.return_t.has_unbound_var()
            }
            Self::Record(r) => r.values().any(|t| t.has_unbound_var()),
            Self::Refinement(refine) => {
                refine.t.has_unbound_var() || refine.preds.iter().any(|p| p.has_unbound_var())
            }
            Self::Quantified(quant) => quant.has_unbound_var(),
            Self::Poly { params, .. } => params.iter().any(|p| p.has_unbound_var()),
            Self::Proj { lhs, .. } => lhs.has_no_unbound_var(),
            _ => false,
        }
    }

    pub fn has_no_unbound_var(&self) -> bool {
        !self.has_unbound_var()
    }

    pub fn typarams_len(&self) -> Option<usize> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().typarams_len(),
            Self::Refinement(refine) => refine.t.typarams_len(),
            // REVIEW:
            Self::Ref(_) | Self::RefMut { .. } => Some(1),
            Self::And(_, _) | Self::Or(_, _) | Self::Not(_, _) => Some(2),
            Self::Subr(subr) => Some(
                subr.non_default_params.len()
                    + subr.var_params.as_ref().map(|_| 1).unwrap_or(0)
                    + subr.default_params.len()
                    + 1,
            ),
            Self::Callable { param_ts, .. } => Some(param_ts.len() + 1),
            Self::Poly { params, .. } => Some(params.len()),
            _ => None,
        }
    }

    pub fn container_len(&self) -> Option<usize> {
        log!(err "{self}");
        match self {
            Self::Poly { name, params } => match &name[..] {
                "Array" => {
                    if let TyParam::Value(ValueObj::Nat(n)) = &params[0] {
                        Some(*n as usize)
                    } else {
                        None
                    }
                }
                "Tuple" => Some(params.len()),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn typarams(&self) -> Vec<TyParam> {
        match self {
            Self::FreeVar(f) if f.is_linked() => f.crack().typarams(),
            Self::FreeVar(_unbound) => vec![],
            Self::Refinement(refine) => refine.t.typarams(),
            Self::Ref(t) | Self::RefMut { before: t, .. } => vec![TyParam::t(*t.clone())],
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                vec![TyParam::t(*lhs.clone()), TyParam::t(*rhs.clone())]
            }
            Self::Subr(subr) => subr.typarams(),
            Self::Callable { param_ts: _, .. } => todo!(),
            Self::Poly { params, .. } => params.clone(),
            _ => vec![],
        }
    }

    pub fn self_t(&self) -> Option<&Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => unsafe { fv.as_ptr().as_ref() }
                .unwrap()
                .linked()
                .and_then(|t| t.self_t()),
            Self::Refinement(refine) => refine.t.self_t(),
            Self::Subr(subr) => subr.self_t(),
            Self::Quantified(quant) => quant.self_t(),
            _ => None,
        }
    }

    pub fn non_default_params(&self) -> Option<&Vec<ParamTy>> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => unsafe { fv.as_ptr().as_ref() }
                .unwrap()
                .linked()
                .and_then(|t| t.non_default_params()),
            Self::Refinement(refine) => refine.t.non_default_params(),
            Self::Subr(SubrType {
                non_default_params, ..
            }) => Some(non_default_params),
            Self::Callable { param_ts: _, .. } => todo!(),
            _ => None,
        }
    }

    pub fn var_args(&self) -> Option<&ParamTy> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => unsafe { fv.as_ptr().as_ref() }
                .unwrap()
                .linked()
                .and_then(|t| t.var_args()),
            Self::Refinement(refine) => refine.t.var_args(),
            Self::Subr(SubrType {
                var_params: var_args,
                ..
            }) => var_args.as_deref(),
            Self::Callable { param_ts: _, .. } => todo!(),
            _ => None,
        }
    }

    pub fn default_params(&self) -> Option<&Vec<ParamTy>> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => unsafe { fv.as_ptr().as_ref() }
                .unwrap()
                .linked()
                .and_then(|t| t.default_params()),
            Self::Refinement(refine) => refine.t.default_params(),
            Self::Subr(SubrType { default_params, .. }) => Some(default_params),
            _ => None,
        }
    }

    pub fn return_t(&self) -> Option<&Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => unsafe { fv.as_ptr().as_ref() }
                .unwrap()
                .linked()
                .and_then(|t| t.return_t()),
            Self::Refinement(refine) => refine.t.return_t(),
            Self::Subr(SubrType { return_t, .. }) | Self::Callable { return_t, .. } => {
                Some(return_t)
            }
            // NOTE: Quantified could return a quantified type variable.
            // At least in situations where this function is needed, self cannot be Quantified.
            Self::Quantified(quant) => {
                if quant.return_t().unwrap().is_generalized() {
                    todo!("quantified return type (recursive function type inference)")
                }
                quant.return_t()
            }
            _ => None,
        }
    }

    pub fn mut_return_t(&mut self) -> Option<&mut Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => unsafe { fv.as_ptr().as_mut() }
                .unwrap()
                .linked_mut()
                .and_then(|t| t.mut_return_t()),
            Self::Refinement(refine) => refine.t.mut_return_t(),
            Self::Subr(SubrType { return_t, .. }) | Self::Callable { return_t, .. } => {
                Some(return_t)
            }
            // Self::Quantified(quant) => quant.unbound_callable.mut_return_t(),
            _ => None,
        }
    }

    pub fn derefine(&self) -> Type {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().derefine(),
            Self::FreeVar(fv) => {
                let name = fv.unbound_name().unwrap();
                let level = fv.level().unwrap();
                if let Some((sub, sup)) = fv.get_subsup() {
                    let constraint = Constraint::new_sandwiched(sub.derefine(), sup.derefine());
                    // not `.update_constraint`
                    Self::FreeVar(Free::new_named_unbound(name, level, constraint))
                } else {
                    let t = fv.get_type().unwrap().derefine();
                    let constraint = Constraint::new_type_of(t);
                    Self::FreeVar(Free::new_named_unbound(name, level, constraint))
                }
            }
            Self::Refinement(refine) => refine.t.as_ref().clone(),
            Self::Poly { name, params } => {
                let params = params
                    .iter()
                    .map(|tp| match tp {
                        TyParam::Type(t) => TyParam::t(t.derefine()),
                        other => other.clone(),
                    })
                    .collect();
                Self::Poly {
                    name: name.clone(),
                    params,
                }
            }
            Self::Ref(t) => Self::Ref(Box::new(t.derefine())),
            Self::RefMut { before, after } => Self::RefMut {
                before: Box::new(before.derefine()),
                after: after.as_ref().map(|t| Box::new(t.derefine())),
            },
            Self::And(l, r) => {
                let l = l.derefine();
                let r = r.derefine();
                Self::And(Box::new(l), Box::new(r))
            }
            Self::Or(l, r) => {
                let l = l.derefine();
                let r = r.derefine();
                Self::Or(Box::new(l), Box::new(r))
            }
            Self::Not(l, r) => {
                let l = l.derefine();
                let r = r.derefine();
                Self::Not(Box::new(l), Box::new(r))
            }
            other => other.clone(),
        }
    }

    pub fn replace(self, target: &Type, to: &Type) -> Type {
        if &self == target {
            return to.clone();
        }
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().clone().replace(target, to),
            Self::Refinement(mut refine) => {
                refine.t = Box::new(refine.t.replace(target, to));
                Self::Refinement(refine)
            }
            Self::Record(mut rec) => {
                for v in rec.values_mut() {
                    *v = std::mem::take(v).replace(target, to);
                }
                Self::Record(rec)
            }
            Self::Subr(mut subr) => {
                for nd in subr.non_default_params.iter_mut() {
                    *nd.typ_mut() = std::mem::take(nd.typ_mut()).replace(target, to);
                }
                if let Some(var) = subr.var_params.as_mut() {
                    *var.as_mut().typ_mut() =
                        std::mem::take(var.as_mut().typ_mut()).replace(target, to);
                }
                for d in subr.default_params.iter_mut() {
                    *d.typ_mut() = std::mem::take(d.typ_mut()).replace(target, to);
                }
                subr.return_t = Box::new(subr.return_t.replace(target, to));
                Self::Subr(subr)
            }
            Self::Callable { param_ts, return_t } => {
                let param_ts = param_ts
                    .into_iter()
                    .map(|t| t.replace(target, to))
                    .collect();
                let return_t = Box::new(return_t.replace(target, to));
                Self::Callable { param_ts, return_t }
            }
            Self::Quantified(quant) => quant.replace(target, to).quantify(),
            Self::Poly { name, params } => {
                let params = params
                    .into_iter()
                    .map(|tp| match tp {
                        TyParam::Type(t) => TyParam::t(t.replace(target, to)),
                        other => other,
                    })
                    .collect();
                Self::Poly { name, params }
            }
            Self::Ref(t) => Self::Ref(Box::new(t.replace(target, to))),
            Self::RefMut { before, after } => Self::RefMut {
                before: Box::new(before.replace(target, to)),
                after: after.map(|t| Box::new(t.replace(target, to))),
            },
            Self::And(l, r) => {
                let l = l.replace(target, to);
                let r = r.replace(target, to);
                Self::And(Box::new(l), Box::new(r))
            }
            Self::Or(l, r) => {
                let l = l.replace(target, to);
                let r = r.replace(target, to);
                Self::Or(Box::new(l), Box::new(r))
            }
            Self::Not(l, r) => {
                let l = l.replace(target, to);
                let r = r.replace(target, to);
                Self::Not(Box::new(l), Box::new(r))
            }
            other => other,
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
    Set,
    SetMut,
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
                "Set" | "Set!" => Self::Set,
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
            (Type::Refinement(refine), r) => Self::new(&refine.t, r),
            (l, Type::Refinement(refine)) => Self::new(l, &refine.t),
            (_, _) => Self::Others,
        }
    }
}
