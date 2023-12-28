//! defines `Type` (type kind).
//! Some structures implement `Display` using `LimitedDisplay`. This is omitted when the display width is somewhat longer.
//! If you want to get the full display, use `LimitedDisplay::to_string_unabbreviated`.
//!
//! `Type`(コンパイラ等で使われる「型」を表現する)を定義する。
//! 各種の構造体は`LimitedDisplay`を使って`Display`が実装されている。これは表示の幅がある程度長くなる場合省略を行う。
//! フルの表示を得たい場合は、`LimitedDisplay::to_string_unabbreviated`を使うこと。
#![allow(clippy::derived_hash_with_manual_eq)]
#![allow(clippy::large_enum_variant)]
pub mod codeobj;
pub mod const_subr;
pub mod constructors;
pub mod deserialize;
pub mod free;
pub mod predicate;
pub mod typaram;
pub mod value;
pub mod vis;

use std::cell::RefMut;
use std::fmt;
use std::ops::{BitAnd, BitOr, Deref, Not, Range, RangeInclusive};
use std::path::PathBuf;

use erg_common::consts::DEBUG_MODE;
use erg_common::dict::Dict;
use erg_common::error::Location;
use erg_common::fresh::FRESH_GEN;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::set::Set;
use erg_common::traits::{LimitedDisplay, Locational, StructuralEq};
use erg_common::{enum_unwrap, fmt_option, ref_addr_eq, set, Str};

use erg_parser::ast::Expr;
use erg_parser::token::TokenKind;

pub use const_subr::*;
use constructors::{dict_t, int_interval, mono};
use free::{CanbeFree, Constraint, Free, FreeKind, FreeTyVar, HasLevel, Level, GENERIC_LEVEL};
pub use predicate::Predicate;
pub use typaram::{IntervalOp, TyParam};
use value::value_set::*;
pub use value::ValueObj;
use value::ValueObj::{Inf, NegInf};
pub use vis::*;

use crate::context::eval::UndoableLinkedList;

use self::constructors::{bounded, free_var, named_free_var, proj_call, subr_t};

pub const STR_OMIT_THRESHOLD: usize = if DEBUG_MODE { 100 } else { 16 };
pub const CONTAINER_OMIT_THRESHOLD: usize = if DEBUG_MODE { 100 } else { 8 };
pub const DEFAULT_PARAMS_THRESHOLD: usize = if DEBUG_MODE { 100 } else { 5 };

/// cloneのコストがあるためなるべく.ref_tを使うようにすること
/// いくつかの構造体は直接Typeを保持していないので、その場合は.tを使う
#[allow(unused_variables)]
pub trait HasType {
    fn ref_t(&self) -> &Type;
    // 関数呼び出しの場合、.ref_t()は戻り値を返し、signature_t()は関数全体の型を返す
    fn signature_t(&self) -> Option<&Type>;
    // 最後にHIR全体の型変数を消すために使う
    fn ref_mut_t(&mut self) -> Option<&mut Type>;
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
            fn ref_mut_t(&mut self) -> Option<&mut Type> {
                Some(&mut self.t)
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
    ($T: ty, delegate $attr: ident) => {
        impl $crate::ty::HasType for $T {
            #[inline]
            fn ref_t(&self) -> &Type {
                &self.$attr.ref_t()
            }
            #[inline]
            fn ref_mut_t(&mut self) -> Option<&mut Type> {
                self.$attr.ref_mut_t()
            }
            #[inline]
            fn signature_t(&self) -> Option<&Type> {
                self.$attr.signature_t()
            }
            #[inline]
            fn signature_mut_t(&mut self) -> Option<&mut Type> {
                self.$attr.signature_mut_t()
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
            fn ref_mut_t(&mut self) -> Option<&mut Type> {
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
pub enum ParamTy {
    Pos(Type),
    Kw { name: Str, ty: Type },
    KwWithDefault { name: Str, ty: Type, default: Type },
}

impl fmt::Display for ParamTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pos(ty) => write!(f, "{ty}"),
            Self::Kw { name, ty } => write!(f, "{name}: {ty}"),
            Self::KwWithDefault { name, ty, default } => {
                write!(f, "{name}: {ty} := {default}")
            }
        }
    }
}

impl ParamTy {
    pub fn kw(name: Str, ty: Type) -> Self {
        if &name[..] == "_" {
            Self::Pos(ty)
        } else {
            Self::Kw { name, ty }
        }
    }

    pub fn pos_or_kw(name: Option<Str>, ty: Type) -> Self {
        match name {
            Some(name) => Self::kw(name, ty),
            None => Self::Pos(ty),
        }
    }

    pub const fn kw_default(name: Str, ty: Type, default: Type) -> Self {
        Self::KwWithDefault { name, ty, default }
    }

    pub fn name(&self) -> Option<&Str> {
        match self {
            Self::Pos(_) => None,
            Self::Kw { name, .. } | Self::KwWithDefault { name, .. } => Some(name),
        }
    }

    pub const fn typ(&self) -> &Type {
        match self {
            Self::Pos(ty) | Self::Kw { ty, .. } | Self::KwWithDefault { ty, .. } => ty,
        }
    }

    pub fn typ_mut(&mut self) -> &mut Type {
        match self {
            Self::Pos(ty) | Self::Kw { ty, .. } | Self::KwWithDefault { ty, .. } => ty,
        }
    }

    pub fn map_type<F>(self, f: F) -> Self
    where
        F: FnOnce(Type) -> Type,
    {
        match self {
            Self::Pos(ty) => Self::Pos(f(ty)),
            Self::Kw { name, ty } => Self::Kw { name, ty: f(ty) },
            Self::KwWithDefault { name, ty, default } => Self::KwWithDefault {
                name,
                ty: f(ty),
                default,
            },
        }
    }

    pub fn try_map_type<F, E>(self, f: F) -> Result<Self, E>
    where
        F: FnOnce(Type) -> Result<Type, E>,
    {
        match self {
            Self::Pos(ty) => Ok(Self::Pos(f(ty)?)),
            Self::Kw { name, ty } => Ok(Self::Kw { name, ty: f(ty)? }),
            Self::KwWithDefault { name, ty, default } => Ok(Self::KwWithDefault {
                name,
                ty: f(ty)?,
                default,
            }),
        }
    }

    pub fn deconstruct(self) -> (Option<Str>, Type, Option<Type>) {
        match self {
            Self::Pos(ty) => (None, ty, None),
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
    pub var_params: Option<Box<ParamTy>>, // TODO: need to have a position (var_params can be specified after default_params)
    pub default_params: Vec<ParamTy>,
    pub kw_var_params: Option<Box<ParamTy>>,
    pub return_t: Box<Type>,
}

impl fmt::Display for SubrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl TryFrom<Type> for SubrType {
    type Error = ();
    fn try_from(t: Type) -> Result<Self, ()> {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => Self::try_from(fv.crack().clone()),
            Type::Subr(st) => Ok(st),
            Type::Quantified(quant) => SubrType::try_from(*quant),
            Type::Refinement(refine) => Self::try_from(*refine.t),
            _ => Err(()),
        }
    }
}

impl<'t> TryFrom<&'t Type> for &'t SubrType {
    type Error = ();
    fn try_from(t: &'t Type) -> Result<&'t SubrType, ()> {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => Self::try_from(fv.unsafe_crack()),
            Type::Subr(st) => Ok(st),
            Type::Quantified(quant) => <&SubrType>::try_from(quant.as_ref()),
            Type::Refinement(refine) => Self::try_from(refine.t.as_ref()),
            _ => Err(()),
        }
    }
}

impl LimitedDisplay for SubrType {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> fmt::Result {
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
            write!(f, "*")?;
            if let Some(name) = var_params.name() {
                write!(f, "{}: ", name)?;
            }
            var_params.typ().limited_fmt(f, limit - 1)?;
        }
        for (i, pt) in self.default_params.iter().enumerate() {
            if limit.is_positive() && i >= DEFAULT_PARAMS_THRESHOLD {
                write!(f, ", ...")?;
                break;
            }
            if i > 0 || !self.non_default_params.is_empty() || self.var_params.is_some() {
                write!(f, ", ")?;
            }
            write!(f, "{} := ", pt.name().unwrap())?;
            pt.typ().limited_fmt(f, limit - 1)?;
        }
        if let Some(kw_var_params) = &self.kw_var_params {
            if !self.non_default_params.is_empty()
                || !self.default_params.is_empty()
                || self.var_params.is_some()
            {
                write!(f, ", ")?;
            }
            write!(f, "**")?;
            if let Some(name) = kw_var_params.name() {
                write!(f, "{}: ", name)?;
            }
            kw_var_params.typ().limited_fmt(f, limit - 1)?;
        }
        write!(f, ") {} ", self.kind.arrow())?;
        self.return_t.limited_fmt(f, limit - 1)
    }
}

impl StructuralEq for SubrType {
    fn structural_eq(&self, other: &Self) -> bool {
        let kw_check = || {
            for lpt in self.default_params.iter() {
                if let Some(rpt) = self
                    .default_params
                    .iter()
                    .find(|rpt| rpt.name() == lpt.name())
                {
                    if !lpt.typ().structural_eq(rpt.typ()) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        };
        let non_defaults_judge = self
            .non_default_params
            .iter()
            .zip(other.non_default_params.iter())
            .all(|(l, r)| l.typ().structural_eq(r.typ()));
        let var_params_judge = self
            .var_params
            .iter()
            .zip(other.var_params.iter())
            .all(|(l, r)| l.typ().structural_eq(r.typ()));
        let return_t_judge = self.return_t.structural_eq(&other.return_t);
        let kw_var_params_judge = self
            .kw_var_params
            .iter()
            .zip(other.kw_var_params.iter())
            .all(|(l, r)| l.typ().structural_eq(r.typ()));
        non_defaults_judge
            && var_params_judge
            && kw_var_params_judge
            && return_t_judge
            && kw_check()
    }
}

impl SubrType {
    pub fn new(
        kind: SubrKind,
        non_default_params: Vec<ParamTy>,
        var_params: Option<ParamTy>,
        default_params: Vec<ParamTy>,
        kw_var_params: Option<ParamTy>,
        return_t: Type,
    ) -> Self {
        Self {
            kind,
            non_default_params,
            var_params: var_params.map(Box::new),
            default_params,
            kw_var_params: kw_var_params.map(Box::new),
            return_t: Box::new(return_t),
        }
    }

    pub fn contains_tvar(&self, target: &FreeTyVar) -> bool {
        self.non_default_params
            .iter()
            .any(|pt| pt.typ().contains_tvar(target))
            || self
                .var_params
                .as_ref()
                .map_or(false, |pt| pt.typ().contains_tvar(target))
            || self
                .default_params
                .iter()
                .any(|pt| pt.typ().contains_tvar(target))
            || self.return_t.contains_tvar(target)
    }

    pub fn contains_type(&self, target: &Type) -> bool {
        self.non_default_params
            .iter()
            .any(|pt| pt.typ().contains_type(target))
            || self
                .var_params
                .as_ref()
                .map_or(false, |pt| pt.typ().contains_type(target))
            || self
                .default_params
                .iter()
                .any(|pt| pt.typ().contains_type(target))
            || self.return_t.contains_type(target)
    }

    pub fn contains_tp(&self, target: &TyParam) -> bool {
        self.non_default_params
            .iter()
            .any(|pt| pt.typ().contains_tp(target))
            || self
                .var_params
                .as_ref()
                .map_or(false, |pt| pt.typ().contains_tp(target))
            || self
                .default_params
                .iter()
                .any(|pt| pt.typ().contains_tp(target))
            || self.return_t.contains_tp(target)
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

    /// ```erg
    /// essential_qnames(|T, U| (T, U) -> Int) == {}
    /// essential_qnames(|T, U| (T, U) -> (T, U)) == {T, U}
    /// essential_qnames(|T, A| (T) -> A(<: T)) == {T}
    /// essential_qnames(|T, U| (T, T) -> U) == {T}
    /// ```
    pub fn essential_qnames(&self) -> Set<Str> {
        let structural_qname = self.non_default_params.iter().find_map(|pt| {
            pt.typ()
                .get_super()
                .map_or(false, |t| t.is_structural())
                .then(|| pt.typ().unbound_name().unwrap())
        });
        let qnames_sets = self
            .non_default_params
            .iter()
            .map(|pt| pt.typ().qnames())
            .chain(self.var_params.iter().map(|pt| pt.typ().qnames()))
            .chain(self.default_params.iter().map(|pt| pt.typ().qnames()))
            .chain([self.return_t.qnames()]);
        Set::multi_intersection(qnames_sets).extended(structural_qname)
    }

    pub fn has_qvar(&self) -> bool {
        self.non_default_params.iter().any(|pt| pt.typ().has_qvar())
            || self
                .var_params
                .as_ref()
                .map_or(false, |pt| pt.typ().has_qvar())
            || self.default_params.iter().any(|pt| pt.typ().has_qvar())
            || self.return_t.has_qvar()
    }

    pub fn has_unbound_var(&self) -> bool {
        self.non_default_params
            .iter()
            .any(|pt| pt.typ().has_unbound_var())
            || self
                .var_params
                .as_ref()
                .map_or(false, |pt| pt.typ().has_unbound_var())
            || self
                .default_params
                .iter()
                .any(|pt| pt.typ().has_unbound_var())
            || self.return_t.has_unbound_var()
    }

    pub fn has_undoable_linked_var(&self) -> bool {
        self.non_default_params
            .iter()
            .any(|pt| pt.typ().has_undoable_linked_var())
            || self
                .var_params
                .as_ref()
                .map_or(false, |pt| pt.typ().has_undoable_linked_var())
            || self
                .default_params
                .iter()
                .any(|pt| pt.typ().has_undoable_linked_var())
            || self.return_t.has_undoable_linked_var()
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
                .map_or(false, |n| &n[..] == "self" || &n[..] == "Self")
            {
                Some(p.typ())
            } else {
                None
            }
        })
    }

    pub fn mut_self_t(&mut self) -> Option<&mut Type> {
        self.non_default_params.first_mut().and_then(|p| {
            if p.name()
                .map_or(false, |n| &n[..] == "self" || &n[..] == "Self")
            {
                Some(p.typ_mut())
            } else {
                None
            }
        })
    }

    pub fn is_method(&self) -> bool {
        self.self_t().is_some()
    }

    pub fn non_var_params(&self) -> impl Iterator<Item = &ParamTy> + Clone {
        if self.var_params.is_some() {
            self.non_default_params.iter().chain([].iter())
        } else {
            self.non_default_params
                .iter()
                .chain(self.default_params.iter())
        }
    }

    /// WARN: This is an infinite iterator
    ///
    /// `self` is not included
    pub fn pos_params(&self) -> impl Iterator<Item = &ParamTy> + Clone {
        let non_defaults = self
            .non_default_params
            .iter()
            .filter(|pt| !pt.name().is_some_and(|n| &n[..] == "self"));
        let defaults = self.default_params.iter();
        if let Some(var_params) = self.var_params.as_ref() {
            non_defaults
                .chain([].iter())
                .chain(std::iter::repeat(var_params.as_ref()))
        } else {
            non_defaults
                .chain(defaults)
                .chain(std::iter::repeat(&ParamTy::Pos(Type::Failure)))
        }
    }

    pub fn param_names(&self) -> impl Iterator<Item = &str> + Clone {
        self.non_default_params
            .iter()
            .chain(self.var_params.as_deref())
            .chain(self.default_params.iter())
            .map(|pt| pt.name().map_or("_", |s| &s[..]))
    }

    pub fn is_no_var(&self) -> bool {
        self.var_params.is_none() && self.kw_var_params.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefineKind {
    Interval { min: TyParam, max: TyParam }, // e.g. {I: Int | I >= 2; I <= 10} 2..10
    Enum(Set<TyParam>),                      // e.g. {I: Int | I == 1 or I == 2} {1, 2}
    Complex,
}

/// e.g.
/// ```erg
/// {I: Int | I >= 0}
/// {_: StrWithLen N | N >= 0}
/// {T: (Int, Int) | T.0 >= 0, T.1 >= 0}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefinementType {
    pub var: Str,
    pub t: Box<Type>,
    pub pred: Box<Predicate>,
}

impl fmt::Display for RefinementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl<'a> TryFrom<&'a Type> for &'a RefinementType {
    type Error = ();
    fn try_from(t: &'a Type) -> Result<&'a RefinementType, ()> {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => Self::try_from(fv.unsafe_crack()),
            Type::Refinement(refine) => Ok(refine),
            _ => Err(()),
        }
    }
}

impl LimitedDisplay for RefinementType {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> std::fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        let first_subj = self.pred.ors().iter().next().and_then(|p| p.subject());
        let is_simple_type = self.t.is_value_class();
        let is_simple_preds = self
            .pred
            .ors()
            .iter()
            .all(|p| p.is_equal() && p.subject() == first_subj);
        if is_simple_type && is_simple_preds {
            write!(f, "{{")?;
            for (i, pred) in self.pred.ors().into_iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                let (_, rhs) = enum_unwrap!(pred, Predicate::Equal { lhs, rhs });
                rhs.limited_fmt(f, limit - 1)?;
            }
            write!(f, "}}")?;
            Ok(())
        } else {
            write!(f, "{{{}: ", self.var)?;
            self.t.limited_fmt(f, limit - 1)?;
            write!(f, " | {}}}", self.pred)
        }
    }
}

impl RefinementType {
    pub fn new(var: Str, t: Type, pred: Predicate) -> Self {
        match t.deconstruct_refinement() {
            Ok((inner_var, inner_t, inner_preds)) => {
                let new_preds = pred.change_subject_name(inner_var.clone());
                Self {
                    var: inner_var,
                    t: Box::new(inner_t),
                    pred: Box::new(inner_preds | new_preds),
                }
            }
            Err(t) => Self {
                var,
                t: Box::new(t),
                pred: Box::new(pred),
            },
        }
    }

    pub fn deconstruct(self) -> (Str, Type, Predicate) {
        (self.var, *self.t, *self.pred)
    }

    /// {None}.invert() == {x: Obj | x != None}
    pub fn invert(self) -> Self {
        Self::new(self.var, Type::Obj, !*self.pred)
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
    pub var_params: Option<(Option<Str>, Ownership)>,
    pub defaults: Vec<(Str, Ownership)>,
    pub kw_var_params: Option<(Option<Str>, Ownership)>,
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
            write!(f, ", *")?;
            if let Some(name) = name {
                write!(f, "{name}: {o:?}")?;
            } else {
                write!(f, "{o:?}")?;
            }
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
        var_params: Option<(Option<Str>, Ownership)>,
        defaults: Vec<(Str, Ownership)>,
        kw_var_params: Option<(Option<Str>, Ownership)>,
    ) -> Self {
        Self {
            non_defaults,
            var_params,
            defaults,
            kw_var_params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CastTarget {
    Param {
        nth: usize,
        name: Str,
        loc: Location,
    },
    Var {
        name: Str,
        loc: Location,
    },
    // NOTE: `Expr(Expr)` causes a bad memory access error
    Expr(Box<Expr>),
}

impl fmt::Display for CastTarget {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Param { nth, name, .. } => write!(f, "{name}#{nth}"),
            Self::Var { name, .. } => write!(f, "{name}"),
            Self::Expr(expr) => write!(f, "{expr}"),
        }
    }
}

impl Locational for CastTarget {
    fn loc(&self) -> Location {
        match self {
            Self::Param { loc, .. } => *loc,
            Self::Var { loc, .. } => *loc,
            Self::Expr(expr) => expr.loc(),
        }
    }
}

impl CastTarget {
    pub const fn param(nth: usize, name: Str, loc: Location) -> Self {
        Self::Param { nth, name, loc }
    }

    pub fn expr(expr: Expr) -> Self {
        Self::Expr(Box::new(expr))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GuardType {
    pub namespace: Str,
    pub target: CastTarget,
    pub to: Box<Type>,
}

impl fmt::Display for GuardType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{} in {}}}", self.target, self.to)
    }
}

impl StructuralEq for GuardType {
    fn structural_eq(&self, other: &Self) -> bool {
        self.target == other.target && self.to.structural_eq(&other.to)
    }
}

impl GuardType {
    pub fn new(namespace: Str, target: CastTarget, to: Type) -> Self {
        Self {
            namespace,
            target,
            to: Box::new(to),
        }
    }
}

#[derive(Debug, Clone, Hash, Default)]
pub enum Type {
    /* Monomorphic (builtin) types */
    Obj, // {=}
    Int,
    Nat,
    Ratio,
    Float,
    Complex,
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
    NotImplementedType,
    Ellipsis,  // == classof(...), これはクラスのほうで型推論用のマーカーではない
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
    // Overloaded(Vec<Type>),
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
    Or(Box<Type>, Box<Type>),
    Not(Box<Type>),
    // NOTE: It was found that adding a new variant above `Poly` may cause a subtyping bug,
    // possibly related to enum internal numbering, but the cause is unknown.
    Poly {
        name: Str,
        params: Vec<TyParam>,
    },
    NamedTuple(Vec<(Field, Type)>),
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
    Structural(Box<Type>),
    // used for narrowing the type of a variable. It is treated as a subtype of Bool
    // e.g. `isinstance(x: Obj, Cls: ClassType) -> {x in Cls}`
    Guard(GuardType),
    Bounded {
        sub: Box<Type>,
        sup: Box<Type>,
    },
    FreeVar(FreeTyVar), // a reference to the type of other expression, see docs/compiler/inference.md
    #[default]
    Failure, // indicates a failure of type inference and behaves as `Never`.
    /// used to represent `TyParam` is not initialized (see `erg_compiler::context::instantiate_tp`)
    Uninited,
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        if ref_addr_eq!(self, other) {
            return true;
        }
        match (self, other) {
            (Self::Obj, Self::Obj)
            | (Self::Complex, Self::Complex)
            | (Self::Float, Self::Float)
            | (Self::Ratio, Self::Ratio)
            | (Self::Int, Self::Int)
            | (Self::Nat, Self::Nat)
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
            | (Self::NotImplementedType, Self::NotImplementedType)
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
                    param_ts: lps,
                    return_t: lr,
                },
                Self::Callable {
                    param_ts: rps,
                    return_t: rr,
                },
            ) => {
                lps.len() != rps.len()
                    && lps.iter().zip(rps.iter()).all(|(l, r)| l == r)
                    && (lr == rr)
            }
            (Self::Record(lhs), Self::Record(rhs)) => {
                if lhs.len() != rhs.len() {
                    return false;
                }
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
            (Self::NamedTuple(lhs), Self::NamedTuple(rhs)) => {
                if lhs.len() != rhs.len() {
                    return false;
                }
                for ((l_field, l_t), (r_field, r_t)) in lhs.iter().zip(rhs) {
                    if l_field != r_field || l_t != r_t {
                        return false;
                    }
                }
                true
            }
            (Self::Refinement(l), Self::Refinement(r)) => l == r,
            (Self::Quantified(l), Self::Quantified(r)) => l == r,
            // REVIEW: should use the same way as `structural_eq`?
            (Self::And(ll, lr), Self::And(rl, rr)) | (Self::Or(ll, lr), Self::Or(rl, rr)) => {
                (ll == rl && lr == rr) || (ll == rr && lr == rl)
            }
            (Self::Not(l), Self::Not(r)) => l == r,
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
            (Self::Structural(l), Self::Structural(r)) => l == r,
            (Self::Guard(l), Self::Guard(r)) => l == r,
            (Self::FreeVar(fv), other) if fv.is_linked() => &*fv.crack() == other,
            (_self, Self::FreeVar(fv)) if fv.is_linked() => _self == &*fv.crack(),
            (Self::FreeVar(l), Self::FreeVar(r)) => l == r,
            (Self::Failure, Self::Failure) | (Self::Uninited, Self::Uninited) => true,
            // NoneType == {None}
            (Self::NoneType, Self::Refinement(refine))
            | (Self::Refinement(refine), Self::NoneType) => {
                matches!(
                    refine.pred.as_ref(),
                    Predicate::Equal {
                        rhs: TyParam::Value(ValueObj::None),
                        ..
                    }
                )
            }
            (
                Self::Bounded { sub, sup },
                Self::Bounded {
                    sub: rsub,
                    sup: rsup,
                },
            ) => sub == rsub && sup == rsup,
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
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> fmt::Result {
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
                for (i, (field, t)) in attrs.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    write!(f, "{field} = ")?;
                    t.limited_fmt(f, limit - 1)?;
                }
                if attrs.is_empty() {
                    write!(f, "=")?;
                }
                write!(f, "}}")
            }
            Self::NamedTuple(attrs) => {
                write!(f, "NamedTuple({{")?;
                for (i, (field, t)) in attrs.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    write!(f, "{field} = ")?;
                    t.limited_fmt(f, limit - 1)?;
                }
                if attrs.is_empty() {
                    write!(f, "=")?;
                }
                write!(f, "}})")
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
                        write!(f, ", ")?;
                    }
                    constr.named_fmt(f, name, limit - 1)?;
                }
                write!(f, "|")?;
                quantified.limited_fmt(f, limit - 1)
            }
            Self::And(lhs, rhs) => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " and ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::Not(ty) => {
                write!(f, "not ")?;
                ty.limited_fmt(f, limit - 1)
            }
            Self::Or(lhs, rhs) => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " or ")?;
                rhs.limited_fmt(f, limit - 1)
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
                if lhs.is_union_type() || lhs.is_intersection_type() {
                    write!(f, "(")?;
                    lhs.limited_fmt(f, limit - 1)?;
                    write!(f, ")")?;
                } else {
                    lhs.limited_fmt(f, limit - 1)?;
                }
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
            Self::Structural(ty) => {
                write!(f, "Structural(")?;
                ty.limited_fmt(f, limit - 1)?;
                write!(f, ")")
            }
            Self::Guard(guard) if cfg!(feature = "debug") => {
                write!(f, "Guard({guard})")
            }
            Self::Bounded { sub, sup } => {
                if sub.is_union_type() || sub.is_intersection_type() {
                    write!(f, "(")?;
                    sub.limited_fmt(f, limit - 1)?;
                    write!(f, ")")?;
                } else {
                    sub.limited_fmt(f, limit - 1)?;
                }
                write!(f, "..")?;
                if sup.is_union_type() || sup.is_intersection_type() {
                    write!(f, "(")?;
                    sup.limited_fmt(f, limit - 1)?;
                    write!(f, ")")?;
                } else {
                    sup.limited_fmt(f, limit - 1)?;
                }
                write!(f, "")
            }
            _ => write!(f, "{}", self.qual_name()),
        }
    }
}

impl CanbeFree for Type {
    fn unbound_name(&self) -> Option<Str> {
        if let Some(fv) = self.as_free() {
            fv.unbound_name()
        } else {
            None
        }
    }

    fn constraint(&self) -> Option<Constraint> {
        if let Some(fv) = self.as_free() {
            fv.constraint()
        } else {
            None
        }
    }

    fn destructive_update_constraint(&self, new_constraint: Constraint, in_instantiation: bool) {
        if let Some(fv) = self.as_free() {
            fv.update_constraint(new_constraint, in_instantiation);
        }
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

impl From<SubrType> for Type {
    fn from(subr: SubrType) -> Self {
        Self::Subr(subr)
    }
}

impl From<RefinementType> for Type {
    fn from(refine: RefinementType) -> Self {
        Self::Refinement(refine)
    }
}

impl<'t> TryFrom<&'t Type> for &'t FreeTyVar {
    type Error = ();
    fn try_from(t: &'t Type) -> Result<&'t FreeTyVar, ()> {
        match t {
            Type::FreeVar(fv) => Ok(fv),
            Type::Refinement(refine) => Self::try_from(refine.t.as_ref()),
            _ => Err(()),
        }
    }
}

impl From<Dict<Field, Type>> for Type {
    fn from(rec: Dict<Field, Type>) -> Self {
        Type::Record(rec)
    }
}

impl BitAnd for Type {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self::And(Box::new(self), Box::new(rhs))
    }
}

impl BitOr for Type {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self::Or(Box::new(self), Box::new(rhs))
    }
}

impl Not for Type {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self::Not(Box::new(self))
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
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        Some(self)
    }
    fn inner_ts(&self) -> Vec<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().inner_ts(),
            Self::Ref(t) => {
                vec![t.as_ref().clone()]
            }
            Self::RefMut { before, .. } => {
                // REVIEW:
                vec![before.as_ref().clone()]
            }
            Self::NamedTuple(tys) => tys.iter().map(|(_, t)| t.clone()).collect(),
            Self::Subr(sub) => sub
                .default_params
                .iter()
                .map(|pt| pt.typ().clone())
                .chain(sub.var_params.as_deref().map(|pt| pt.typ().clone()))
                .chain(sub.non_default_params.iter().map(|pt| pt.typ().clone()))
                .chain([*sub.return_t.clone()])
                .collect(),
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
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
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
            Self::Not(ty) => ty.level(),
            Self::Record(attrs) => attrs.values().filter_map(|t| t.level()).min(),
            Self::NamedTuple(attrs) => attrs.iter().filter_map(|(_, t)| t.level()).min(),
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
                let min = refine.pred.level().unwrap_or(GENERIC_LEVEL);
                let min = lev.min(min);
                if min == GENERIC_LEVEL {
                    None
                } else {
                    Some(min)
                }
            }
            Self::Structural(ty) => ty.level(),
            Self::Guard(guard) => guard.to.level(),
            Self::Quantified(quant) => quant.level(),
            Self::Bounded { sub, sup } => {
                let sub_min = sub.level().unwrap_or(GENERIC_LEVEL);
                let sup_min = sup.level().unwrap_or(GENERIC_LEVEL);
                let min = sub_min.min(sup_min);
                if min == GENERIC_LEVEL {
                    None
                } else {
                    Some(min)
                }
            }
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
            Self::Quantified(quant) => {
                quant.set_level(level);
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.set_level(level);
                rhs.set_level(level);
            }
            Self::Not(ty) => ty.set_level(level),
            Self::Record(attrs) => {
                for t in attrs.values() {
                    t.set_level(level);
                }
            }
            Self::NamedTuple(attrs) => {
                for (_, t) in attrs.iter() {
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
                refine.pred.set_level(level);
            }
            Self::ProjCall { lhs, args, .. } => {
                lhs.set_level(level);
                for arg in args.iter() {
                    arg.set_level(level);
                }
            }
            Self::Structural(ty) => ty.set_level(level),
            Self::Guard(guard) => guard.to.set_level(level),
            Self::Bounded { sub, sup } => {
                sub.set_level(level);
                sup.set_level(level);
            }
            _ => {}
        }
    }
}

impl StructuralEq for Type {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::FreeVar(fv), other) | (other, Self::FreeVar(fv)) if fv.is_linked() => {
                fv.crack().structural_eq(other)
            }
            (Self::FreeVar(fv), Self::FreeVar(fv2)) => fv.structural_eq(fv2),
            (Self::Refinement(refine), Self::Refinement(refine2)) => {
                refine.t.structural_eq(&refine2.t) && refine.pred.structural_eq(&refine2.pred)
            }
            (Self::Record(rec), Self::Record(rec2)) => {
                if rec.len() != rec2.len() {
                    return false;
                }
                for (k, v) in rec.iter() {
                    if let Some(v2) = rec2.get(k) {
                        if !v.structural_eq(v2) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (Self::NamedTuple(rec), Self::NamedTuple(rec2)) => {
                if rec.len() != rec2.len() {
                    return false;
                }
                for ((k, v), (k2, v2)) in rec.iter().zip(rec2) {
                    if k != k2 || !v.structural_eq(v2) {
                        return false;
                    }
                }
                true
            }
            (Self::Subr(subr), Self::Subr(subr2)) => subr.structural_eq(subr2),
            (
                Self::Callable { param_ts, return_t },
                Self::Callable {
                    param_ts: param_ts2,
                    return_t: return_t2,
                },
            ) => {
                param_ts
                    .iter()
                    .zip(param_ts2.iter())
                    .all(|(t, t2)| t.structural_eq(t2))
                    && return_t.structural_eq(return_t2)
            }
            (Self::Quantified(quant), Self::Quantified(quant2)) => quant.structural_eq(quant2),
            (
                Self::Poly { name, params },
                Self::Poly {
                    name: name2,
                    params: params2,
                },
            ) => {
                name == name2
                    && params
                        .iter()
                        .zip(params2)
                        .all(|(p, p2)| p.structural_eq(p2))
            }
            (Self::Ref(t), Self::Ref(t2)) => t.structural_eq(t2),
            (
                Self::RefMut { before, after },
                Self::RefMut {
                    before: before2,
                    after: after2,
                },
            ) => {
                before.structural_eq(before2)
                    && after
                        .as_ref()
                        .zip(after2.as_ref())
                        .map_or(true, |(a, b)| a.structural_eq(b))
            }
            (
                Self::Proj { lhs, rhs },
                Self::Proj {
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) => lhs.structural_eq(lhs2) && rhs == rhs2,
            (
                Self::ProjCall {
                    lhs,
                    attr_name,
                    args,
                },
                Self::ProjCall {
                    lhs: lhs2,
                    attr_name: attr_name2,
                    args: args2,
                },
            ) => {
                lhs.structural_eq(lhs2)
                    && attr_name == attr_name2
                    && args
                        .iter()
                        .zip(args2.iter())
                        .all(|(a, b)| a.structural_eq(b))
            }
            (Self::Structural(l), Self::Structural(r)) => l.structural_eq(r),
            (Self::Guard(l), Self::Guard(r)) => l.structural_eq(r),
            // NG: (l.structural_eq(l2) && r.structural_eq(r2))
            //     || (l.structural_eq(r2) && r.structural_eq(l2))
            (Self::And(_, _), Self::And(_, _)) => {
                let self_ands = self.ands();
                let other_ands = other.ands();
                if self_ands.len() != other_ands.len() {
                    return false;
                }
                for l_val in self_ands.iter() {
                    if other_ands
                        .get_by(l_val, |l, r| l.structural_eq(r))
                        .is_none()
                    {
                        return false;
                    }
                }
                true
            }
            (Self::Or(_, _), Self::Or(_, _)) => {
                let self_ors = self.ors();
                let other_ors = other.ors();
                if self_ors.len() != other_ors.len() {
                    return false;
                }
                for l_val in self_ors.iter() {
                    if other_ors.get_by(l_val, |l, r| l.structural_eq(r)).is_none() {
                        return false;
                    }
                }
                true
            }
            (Self::Not(ty), Self::Not(ty2)) => ty.structural_eq(ty2),
            (
                Self::Bounded { sub, sup },
                Self::Bounded {
                    sub: sub2,
                    sup: sup2,
                },
            ) => sub.structural_eq(sub2) && sup.structural_eq(sup2),
            _ => self == other,
        }
    }
}

impl Type {
    pub const OBJ: &'static Self = &Self::Obj;
    pub const NONE: &'static Self = &Self::NoneType;
    pub const NOT_IMPLEMENTED: &'static Self = &Self::NotImplementedType;
    pub const ELLIPSIS: &'static Self = &Self::Ellipsis;
    pub const INF: &'static Self = &Self::Inf;
    pub const NEG_INF: &'static Self = &Self::NegInf;
    pub const NEVER: &'static Self = &Self::Never;
    pub const FAILURE: &'static Self = &Self::Failure;

    // TODO: this method should be defined in Context
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
            Self::Complex => mono("Complex!"),
            Self::Bool => mono("Bool!"),
            Self::Str => mono("Str!"),
            other if other.is_mut_type() => other,
            _t => todo!("{_t}"),
        }
    }

    pub fn immutate(&self) -> Option<Self> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                if let Some(t) = t.immutate() {
                    fv.link(&t);
                    Some(Self::FreeVar(fv.clone()))
                } else {
                    None
                }
            }
            Self::Mono(name) => match &name[..] {
                "Int!" => Some(Self::Int),
                "Nat!" => Some(Self::Nat),
                "Ratio!" => Some(Self::Ratio),
                "Float!" => Some(Self::Float),
                "Complex!" => Some(Self::Complex),
                "Bool!" => Some(Self::Bool),
                "Str!" => Some(Self::Str),
                _ => None,
            },
            Self::Poly { name, params } => match &name[..] {
                "Array!" => Some(Self::Poly {
                    name: "Array".into(),
                    params: params.clone(),
                }),
                "Set!" => Some(Self::Poly {
                    name: "Set".into(),
                    params: params.clone(),
                }),
                "Dict!" => Some(Self::Poly {
                    name: "Dict".into(),
                    params: params.clone(),
                }),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn quantify(self) -> Self {
        debug_assert!(self.is_subr(), "{self} is not subr");
        match self {
            Self::And(lhs, rhs) => lhs.quantify() & rhs.quantify(),
            other => Self::Quantified(Box::new(other)),
        }
    }

    pub fn proj<S: Into<Str>>(self, attr: S) -> Self {
        Self::Proj {
            lhs: Box::new(self),
            rhs: attr.into(),
        }
    }

    pub fn structuralize(self) -> Self {
        Self::Structural(Box::new(self))
    }

    pub fn is_mono_value_class(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_mono_value_class(),
            Self::Obj
            | Self::Int
            | Self::Nat
            | Self::Ratio
            | Self::Float
            | Self::Complex
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
            | Self::NotImplementedType
            | Self::Ellipsis
            | Self::Never => true,
            _ => false,
        }
    }

    /// value class := mono value object class | (Array | Set)(value class)
    pub fn is_value_class(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_value_class(),
            Self::Refinement(refine) => refine.t.is_value_class(),
            Self::Poly { name, params } => {
                if &name[..] == "Array" || &name[..] == "Set" {
                    let elem_t = <&Type>::try_from(params.first().unwrap()).unwrap();
                    elem_t.is_value_class()
                } else {
                    false
                }
            }
            _ => self.is_mono_value_class(),
        }
    }

    pub fn is_mut_value_class(&self) -> bool {
        self.immutate().is_some_and(|t| t.is_value_class())
    }

    /// Procedure
    pub fn is_procedure(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_procedure(),
            Self::Callable { .. } => true,
            Self::Quantified(t) => t.is_procedure(),
            Self::Subr(subr) if subr.kind == SubrKind::Proc => true,
            Self::Refinement(refine) => refine.t.is_procedure(),
            Self::And(lhs, rhs) => lhs.is_procedure() && rhs.is_procedure(),
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
            Self::Bounded { sup, .. } => sup.is_nonelike(),
            _ => false,
        }
    }

    pub fn is_union_type(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_union_type(),
            Self::Or(_, _) => true,
            Self::Refinement(refine) => refine.t.is_union_type(),
            _ => false,
        }
    }

    pub fn is_projection(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_projection(),
            Self::Proj { .. } | Self::ProjCall { .. } => true,
            _ => false,
        }
    }

    pub fn is_intersection_type(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_intersection_type(),
            Self::And(_, _) => true,
            Self::Refinement(refine) => refine.t.is_intersection_type(),
            _ => false,
        }
    }

    pub fn union_size(&self) -> usize {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().union_size(),
            Self::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = fv.get_subsup().unwrap();
                fv.do_avoiding_recursion(|| sub.union_size().max(sup.union_size()))
            }
            // Or(Or(Int, Str), Nat) == 3
            Self::Or(l, r) => l.union_size() + r.union_size(),
            Self::Refinement(refine) => refine.t.union_size(),
            Self::Ref(t) => t.union_size(),
            Self::RefMut { before, after: _ } => before.union_size(),
            Self::And(lhs, rhs) => lhs.union_size().max(rhs.union_size()),
            Self::Not(ty) => ty.union_size(),
            Self::Callable { param_ts, return_t } => param_ts
                .iter()
                .map(|t| t.union_size())
                .max()
                .unwrap_or(1)
                .max(return_t.union_size()),
            Self::Subr(subr) => subr
                .non_default_params
                .iter()
                .map(|pt| pt.typ().union_size())
                .chain(subr.var_params.as_ref().map(|pt| pt.typ().union_size()))
                .chain(subr.default_params.iter().map(|pt| pt.typ().union_size()))
                .max()
                .unwrap_or(1)
                .max(subr.return_t.union_size()),
            Self::Record(r) => r.values().map(|t| t.union_size()).max().unwrap_or(1),
            Self::NamedTuple(r) => r.iter().map(|(_, t)| t.union_size()).max().unwrap_or(1),
            Self::Quantified(quant) => quant.union_size(),
            Self::Poly { params, .. } => params.iter().map(|p| p.union_size()).max().unwrap_or(1),
            Self::Proj { lhs, .. } => lhs.union_size(),
            Self::ProjCall { lhs, args, .. } => lhs
                .union_size()
                .max(args.iter().map(|t| t.union_size()).max().unwrap_or(1)),
            Self::Structural(ty) => ty.union_size(),
            Self::Guard(guard) => guard.to.union_size(),
            Self::Bounded { sub, sup } => sub.union_size().max(sup.union_size()),
            _ => 1,
        }
    }

    pub fn is_refinement(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_refinement(),
            Self::Refinement(_) => true,
            Self::And(l, r) => l.is_refinement() && r.is_refinement(),
            _ => false,
        }
    }

    pub fn is_singleton_refinement(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_singleton_refinement(),
            Self::Refinement(refine) => matches!(refine.pred.as_ref(), Predicate::Equal { .. }),
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
        self.is_py_module() || self.is_erg_module()
    }

    pub fn is_erg_module(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_erg_module(),
            Self::Refinement(refine) => refine.t.is_erg_module(),
            Self::Poly { name, .. } => &name[..] == "Module",
            _ => false,
        }
    }

    pub fn is_py_module(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_py_module(),
            Self::Refinement(refine) => refine.t.is_py_module(),
            Self::Poly { name, .. } => &name[..] == "PyModule",
            _ => false,
        }
    }

    pub fn is_method(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_method(),
            Self::Refinement(refine) => refine.t.is_method(),
            Self::Subr(subr) => subr.is_method(),
            Self::Quantified(quant) => quant.is_method(),
            Self::And(l, r) => l.is_method() && r.is_method(),
            _ => false,
        }
    }

    pub fn is_subr(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_subr(),
            Self::Subr(_) => true,
            Self::Quantified(quant) => quant.is_subr(),
            Self::Refinement(refine) => refine.t.is_subr(),
            Self::And(l, r) => l.is_subr() && r.is_subr(),
            _ => false,
        }
    }

    pub fn is_quantified_subr(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_quantified_subr(),
            Self::Quantified(_) => true,
            Self::Refinement(refine) => refine.t.is_quantified_subr(),
            Self::And(l, r) => l.is_quantified_subr() && r.is_quantified_subr(),
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_array(),
            Self::Poly { name, .. } => &name[..] == "Array",
            Self::Refinement(refine) => refine.t.is_array(),
            _ => false,
        }
    }

    pub fn is_array_mut(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_array_mut(),
            Self::Poly { name, .. } => &name[..] == "Array!",
            Self::Refinement(refine) => refine.t.is_array_mut(),
            _ => false,
        }
    }

    pub fn is_iterable(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_iterable(),
            Self::Poly { name, .. } => &name[..] == "Iterable",
            Self::Refinement(refine) => refine.t.is_iterable(),
            _ => false,
        }
    }

    pub fn is_tuple(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_tuple(),
            Self::Poly { name, .. } => &name[..] == "Tuple",
            Self::Refinement(refine) => refine.t.is_tuple(),
            _ => false,
        }
    }

    pub fn is_set(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_set(),
            Self::Poly { name, .. } => &name[..] == "Set",
            Self::Refinement(refine) => refine.t.is_set(),
            _ => false,
        }
    }

    pub fn is_dict(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_dict(),
            Self::Poly { name, .. } => &name[..] == "Dict",
            Self::Refinement(refine) => refine.t.is_dict(),
            _ => false,
        }
    }

    pub fn is_dict_mut(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_dict_mut(),
            Self::Poly { name, .. } => &name[..] == "Dict!",
            Self::Refinement(refine) => refine.t.is_dict_mut(),
            _ => false,
        }
    }

    pub fn is_ref(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_ref(),
            Self::Ref(_) => true,
            Self::Refinement(refine) => refine.t.is_ref(),
            _ => false,
        }
    }

    pub fn ref_inner(&self) -> Option<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().ref_inner(),
            Self::Ref(t) => Some(t.as_ref().clone()),
            Self::Refinement(refine) => refine.t.ref_inner(),
            _ => None,
        }
    }

    pub fn is_refmut(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_refmut(),
            Self::RefMut { .. } => true,
            Self::Refinement(refine) => refine.t.is_refmut(),
            _ => false,
        }
    }

    pub fn ref_mut_inner(&self) -> Option<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().ref_mut_inner(),
            Self::RefMut { before, .. } => Some(before.as_ref().clone()),
            Self::Refinement(refine) => refine.t.ref_mut_inner(),
            _ => None,
        }
    }

    pub fn is_structural(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_structural(),
            Self::Structural(_) => true,
            Self::Refinement(refine) => refine.t.is_structural(),
            _ => false,
        }
    }

    pub fn is_failure(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_failure(),
            Self::Refinement(refine) => refine.t.is_failure(),
            Self::Failure => true,
            _ => false,
        }
    }

    pub fn is_class_type(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_class_type(),
            Self::Refinement(refine) => refine.t.is_class_type(),
            Self::ClassType => true,
            _ => false,
        }
    }

    pub fn is_type(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_type(),
            Self::Refinement(refine) => refine.t.is_type(),
            Self::ClassType | Self::TraitType | Self::Type => true,
            Self::Quantified(q) => q.is_type(),
            Self::Subr(subr) => subr.return_t.is_type(),
            _ => false,
        }
    }

    pub fn is_poly_type_meta(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_poly_type_meta(),
            Self::Refinement(refine) => refine.t.is_poly_type_meta(),
            Self::Quantified(q) => q.is_poly_type_meta(),
            Self::Subr(subr) => subr.return_t.is_type(),
            _ => false,
        }
    }

    pub fn as_free(&self) -> Option<&FreeTyVar> {
        <&FreeTyVar>::try_from(self).ok()
    }

    pub fn contains_tvar(&self, target: &FreeTyVar) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_tvar(target),
            Self::FreeVar(fv) if fv.constraint_is_typeof() => {
                ref_addr_eq!(fv.forced_as_ref(), target.forced_as_ref())
            }
            Self::FreeVar(fv) => {
                ref_addr_eq!(fv.forced_as_ref(), target.forced_as_ref())
                    || fv
                        .get_subsup()
                        .map(|(sub, sup)| {
                            fv.do_avoiding_recursion(|| {
                                sub.contains_tvar(target) || sup.contains_tvar(target)
                            })
                        })
                        .unwrap_or(false)
            }
            Self::Record(rec) => rec.iter().any(|(_, t)| t.contains_tvar(target)),
            Self::NamedTuple(rec) => rec.iter().any(|(_, t)| t.contains_tvar(target)),
            Self::Poly { params, .. } => params.iter().any(|tp| tp.contains_tvar(target)),
            Self::Quantified(t) => t.contains_tvar(target),
            Self::Subr(subr) => subr.contains_tvar(target),
            // TODO: preds
            Self::Refinement(refine) => refine.t.contains_tvar(target),
            Self::Structural(ty) => ty.contains_tvar(target),
            Self::Proj { lhs, .. } => lhs.contains_tvar(target),
            Self::ProjCall { lhs, args, .. } => {
                lhs.contains_tvar(target) || args.iter().any(|t| t.contains_tvar(target))
            }
            Self::And(lhs, rhs) => lhs.contains_tvar(target) || rhs.contains_tvar(target),
            Self::Or(lhs, rhs) => lhs.contains_tvar(target) || rhs.contains_tvar(target),
            Self::Not(t) => t.contains_tvar(target),
            Self::Ref(t) => t.contains_tvar(target),
            Self::RefMut { before, after } => {
                before.contains_tvar(target)
                    || after.as_ref().map_or(false, |t| t.contains_tvar(target))
            }
            Self::Bounded { sub, sup } => sub.contains_tvar(target) || sup.contains_tvar(target),
            _ => false,
        }
    }

    pub fn contains_type(&self, target: &Type) -> bool {
        if self == target {
            // This operation can also be performed for recursive types
            return true;
        }
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_type(target),
            Self::FreeVar(fv) => {
                fv.get_subsup().map_or(false, |(sub, sup)| {
                    fv.do_avoiding_recursion(|| {
                        sub.contains_type(target) || sup.contains_type(target)
                    })
                }) || fv.get_type().map_or(false, |t| t.contains_type(target))
            }
            Self::Record(rec) => rec.iter().any(|(_, t)| t.contains_type(target)),
            Self::NamedTuple(rec) => rec.iter().any(|(_, t)| t.contains_type(target)),
            Self::Poly { params, .. } => params.iter().any(|tp| tp.contains_type(target)),
            Self::Quantified(t) => t.contains_type(target),
            Self::Subr(subr) => subr.contains_type(target),
            // TODO: preds
            Self::Refinement(refine) => refine.t.contains_type(target),
            Self::Structural(ty) => ty.contains_type(target),
            Self::Proj { lhs, .. } => lhs.contains_type(target),
            Self::ProjCall { lhs, args, .. } => {
                lhs.contains_type(target) || args.iter().any(|t| t.contains_type(target))
            }
            Self::And(lhs, rhs) => lhs.contains_type(target) || rhs.contains_type(target),
            Self::Or(lhs, rhs) => lhs.contains_type(target) || rhs.contains_type(target),
            Self::Not(t) => t.contains_type(target),
            Self::Ref(t) => t.contains_type(target),
            Self::RefMut { before, after } => {
                before.contains_type(target)
                    || after.as_ref().map_or(false, |t| t.contains_type(target))
            }
            Self::Bounded { sub, sup } => sub.contains_type(target) || sup.contains_type(target),
            _ => false,
        }
    }

    pub fn contains_tp(&self, target: &TyParam) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_tp(target),
            Self::FreeVar(fv) => {
                fv.get_subsup().map_or(false, |(sub, sup)| {
                    fv.do_avoiding_recursion(|| sub.contains_tp(target) || sup.contains_tp(target))
                }) || fv.get_type().map_or(false, |t| t.contains_tp(target))
            }
            Self::Record(rec) => rec.iter().any(|(_, t)| t.contains_tp(target)),
            Self::NamedTuple(rec) => rec.iter().any(|(_, t)| t.contains_tp(target)),
            Self::Poly { params, .. } => params.iter().any(|tp| tp.contains_tp(target)),
            Self::Quantified(t) => t.contains_tp(target),
            Self::Subr(subr) => subr.contains_tp(target),
            // TODO: preds
            Self::Refinement(refine) => refine.t.contains_tp(target),
            Self::Structural(ty) => ty.contains_tp(target),
            Self::Proj { lhs, .. } => lhs.contains_tp(target),
            Self::ProjCall { lhs, args, .. } => {
                lhs.contains_tp(target) || args.iter().any(|t| t.contains_tp(target))
            }
            Self::And(lhs, rhs) => lhs.contains_tp(target) || rhs.contains_tp(target),
            Self::Or(lhs, rhs) => lhs.contains_tp(target) || rhs.contains_tp(target),
            Self::Not(t) => t.contains_tp(target),
            Self::Ref(t) => t.contains_tp(target),
            Self::RefMut { before, after } => {
                before.contains_tp(target)
                    || after.as_ref().map_or(false, |t| t.contains_tp(target))
            }
            Self::Bounded { sub, sup } => sub.contains_tp(target) || sup.contains_tp(target),
            _ => false,
        }
    }

    pub fn is_recursive(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_recursive(),
            Self::FreeVar(fv) => fv.get_subsup().map_or(false, |(sub, sup)| {
                sub.contains_type(self) || sup.contains_type(self)
            }),
            Self::Record(rec) => rec.iter().any(|(_, t)| t.contains_type(self)),
            Self::NamedTuple(rec) => rec.iter().any(|(_, t)| t.contains_type(self)),
            Self::Poly { params, .. } => params.iter().any(|tp| tp.contains_type(self)),
            Self::Quantified(t) => t.contains_type(self),
            Self::Subr(subr) => subr.contains_type(self),
            Self::Refinement(refine) => refine.t.contains_type(self),
            Self::Structural(ty) => ty.contains_type(self),
            Self::Proj { lhs, .. } => lhs.contains_type(self),
            Self::ProjCall { lhs, args, .. } => {
                lhs.contains_type(self) || args.iter().any(|t| t.contains_type(self))
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.contains_type(self) || rhs.contains_type(self)
            }
            Self::Not(t) => t.contains_type(self),
            Self::Ref(t) => t.contains_type(self),
            Self::RefMut { before, after } => {
                before.contains_type(self)
                    || after.as_ref().map_or(false, |t| t.contains_type(self))
            }
            Self::Bounded { sub, sup } => sub.contains_type(self) || sup.contains_type(self),
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
                    .map(|t| (t.name().cloned(), t.typ().ownership()));
                let mut d_args = vec![];
                for d_param in subr.default_params.iter() {
                    let ownership = match d_param.typ() {
                        Self::Ref(_) => Ownership::Ref,
                        Self::RefMut { .. } => Ownership::RefMut,
                        _ => Ownership::Owned,
                    };
                    d_args.push((d_param.name().unwrap().clone(), ownership));
                }
                let kw_var_args = subr
                    .kw_var_params
                    .as_ref()
                    .map(|t| (t.name().cloned(), t.typ().ownership()));
                ArgsOwnership::new(nd_args, var_args, d_args, kw_var_args)
            }
            Self::Quantified(quant) => quant.args_ownership(),
            other => todo!("{other}"),
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

    /// full name of the type, if the type is a normal nominal type, then returns the inner `name`
    /// ```
    /// # use erg_compiler::ty::{Type, TyParam};
    /// # use erg_compiler::ty::constructors::*;
    /// let i = mono("Int!");
    /// assert_eq!(&i.qual_name()[..], "Int!");
    /// assert_eq!(&i.local_name()[..], "Int!");
    /// let t = mono("http.client.Response");
    /// assert_eq!(&t.qual_name()[..], "http.client.Response");
    /// assert_eq!(&t.local_name()[..], "Response");
    /// let r = Type::from(TyParam::from(1)..TyParam::from(10));
    /// assert_eq!(&r.qual_name()[..], "Int");
    /// ```
    pub fn qual_name(&self) -> Str {
        match self {
            Self::Obj => Str::ever("Obj"),
            Self::Int => Str::ever("Int"),
            Self::Nat => Str::ever("Nat"),
            Self::Ratio => Str::ever("Ratio"),
            Self::Float => Str::ever("Float"),
            Self::Complex => Str::ever("Complex"),
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
            Self::Not(_) => Str::ever("Not"),
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
            Self::NamedTuple(_) => Str::ever("NamedTuple"),
            Self::Poly { name, .. } => name.clone(),
            // NOTE: compiler/codegen/convert_to_python_methodでクラス名を使うため、こうすると都合が良い
            Self::Refinement(refine) => refine.t.qual_name(),
            Self::Quantified(_) => Str::ever("Quantified"),
            Self::Ellipsis => Str::ever("Ellipsis"),
            Self::NotImplementedType => Str::ever("NotImplementedType"),
            Self::Never => Str::ever("Never"),
            Self::FreeVar(fv) => match &*fv.borrow() {
                FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t.qual_name(),
                FreeKind::NamedUnbound { name, .. } => name.clone(),
                FreeKind::Unbound { id, .. } => Str::from(format!("%{id}")),
            },
            Self::Proj { .. } => Str::ever("Proj"),
            Self::ProjCall { .. } => Str::ever("ProjCall"),
            Self::Structural(_) => Str::ever("Structural"),
            Self::Guard { .. } => Str::ever("Bool"),
            Self::Bounded { sub, .. } => sub.qual_name(),
            Self::Failure => Str::ever("Failure"),
            Self::Uninited => Str::ever("Uninited"),
        }
    }

    /// ```
    /// # use erg_compiler::ty::constructors::*;
    /// let i = mono("Int!");
    /// assert_eq!(&i.namespace()[..], "");
    /// let t = mono("http.client.Response");
    /// assert_eq!(&t.namespace()[..], "http.client");
    /// ```
    pub fn namespace(&self) -> Str {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().namespace(),
            Self::Refinement(refine) => refine.t.namespace(),
            Self::Mono(name) | Self::Poly { name, .. } => {
                let namespaces = name.split_with(&[".", "::"]);
                if namespaces.len() > 1 {
                    Str::rc(
                        name.trim_end_matches(namespaces.last().unwrap())
                            .trim_end_matches('.')
                            .trim_end_matches("::"),
                    )
                } else {
                    Str::ever("")
                }
            }
            _ => Str::ever(""),
        }
    }

    /// local name of the type
    /// ```
    /// # use erg_compiler::ty::constructors::*;
    /// let i = mono("Int!");
    /// assert_eq!(&i.qual_name()[..], "Int!");
    /// assert_eq!(&i.local_name()[..], "Int!");
    /// let t = mono("http.client.Response");
    /// assert_eq!(&t.qual_name()[..], "http.client.Response");
    /// assert_eq!(&t.local_name()[..], "Response");
    /// ```
    pub fn local_name(&self) -> Str {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().local_name(),
            Self::Refinement(refine) => refine.t.local_name(),
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
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_intersec(typ),
            Self::Refinement(refine) => refine.t.contains_intersec(typ),
            Self::And(t1, t2) => t1.contains_intersec(typ) || t2.contains_intersec(typ),
            _ => self == typ,
        }
    }

    pub fn union_pair(&self) -> Option<(Type, Type)> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().union_pair(),
            Self::Refinement(refine) => refine.t.union_pair(),
            Self::Or(t1, t2) => Some((*t1.clone(), *t2.clone())),
            _ => None,
        }
    }

    /// assert!((A or B).contains_union(B))
    pub fn contains_union(&self, typ: &Type) -> bool {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => fv.crack().contains_union(typ),
            Type::Refinement(refine) => refine.t.contains_union(typ),
            Type::Or(t1, t2) => t1.contains_union(typ) || t2.contains_union(typ),
            _ => self == typ,
        }
    }

    pub fn intersection_types(&self) -> Vec<Type> {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => fv.crack().intersection_types(),
            Type::Refinement(refine) => refine.t.intersection_types(),
            Type::Quantified(tys) => tys
                .intersection_types()
                .into_iter()
                .map(|t| t.quantify())
                .collect(),
            Type::And(t1, t2) => {
                let mut types = t1.intersection_types();
                types.extend(t2.intersection_types());
                types
            }
            _ => vec![self.clone()],
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

    /// <: Super
    pub fn get_super(&self) -> Option<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().get_super(),
            Self::FreeVar(fv) if fv.is_unbound() => fv.get_super(),
            _ => None,
        }
    }

    /// :> Sub
    pub fn get_sub(&self) -> Option<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().get_sub(),
            Self::FreeVar(fv) if fv.is_unbound() => fv.get_sub(),
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

    pub fn is_named_unbound_var(&self) -> bool {
        matches!(self, Self::FreeVar(fv) if fv.is_named_unbound() || (fv.is_linked() && fv.crack().is_named_unbound_var()))
    }

    pub fn is_unnamed_unbound_var(&self) -> bool {
        matches!(self, Self::FreeVar(fv) if fv.is_unnamed_unbound() || (fv.is_linked() && fv.crack().is_unnamed_unbound_var()))
    }

    /// ```erg
    /// assert (?T or ?U).totally_unbound()
    /// ```
    pub fn is_totally_unbound(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_unbound() => true,
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_totally_unbound(),
            Self::Or(t1, t2) | Self::And(t1, t2) => {
                t1.is_totally_unbound() && t2.is_totally_unbound()
            }
            Self::Not(t) => t.is_totally_unbound(),
            _ => false,
        }
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

    /// TODO:
    /// ```erg
    /// Nat == {x: Int | x >= 0}
    /// Nat or {-1} == {x: Int | x >= 0 or x == -1}
    /// Int == {_: Int | True}
    /// ```
    pub fn into_refinement(self) -> RefinementType {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => fv.crack().clone().into_refinement(),
            Type::Nat => {
                let var = FRESH_GEN.fresh_varname();
                RefinementType::new(
                    var.clone(),
                    Type::Int,
                    Predicate::ge(var, TyParam::value(0)),
                )
            }
            Type::Bool => {
                let var = FRESH_GEN.fresh_varname();
                RefinementType::new(
                    var.clone(),
                    Type::Int,
                    Predicate::le(var.clone(), TyParam::value(true))
                        & Predicate::ge(var, TyParam::value(false)),
                )
            }
            Type::Refinement(r) => r,
            t => RefinementType::new(Str::ever("_"), t, Predicate::TRUE),
        }
    }

    /// ```erg
    /// { .x = {Int} } == {{ .x = Int }}
    /// K({Int}) == {K(Int)} # TODO
    /// ```
    pub fn to_singleton(&self) -> Option<RefinementType> {
        match self {
            Type::Record(rec) if rec.values().all(|t| t.is_singleton_refinement()) => {
                let mut new_rec = Dict::new();
                for (k, t) in rec.iter() {
                    if let Some(t) = t
                        .singleton_value()
                        .and_then(|tp| <&Type>::try_from(tp).ok())
                    {
                        new_rec.insert(k.clone(), t.clone());
                    } else if DEBUG_MODE {
                        todo!("{t}");
                    }
                }
                let t = Type::Record(new_rec);
                Some(RefinementType::new(
                    Str::ever("_"),
                    Type::Type,
                    Predicate::eq(Str::ever("_"), TyParam::t(t)),
                ))
            }
            _ => None,
        }
    }

    pub fn deconstruct_refinement(self) -> Result<(Str, Type, Predicate), Type> {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => fv.crack().clone().deconstruct_refinement(),
            Type::Refinement(r) => Ok(r.deconstruct()),
            _ => Err(self),
        }
    }

    /// Fix type variables at their lower bound
    /// ```erg
    /// i: ?T(:> Int)
    /// assert i.Real == 1
    /// i: (Int)
    /// ```
    ///
    /// ```erg
    /// ?T(:> ?U(:> Int)).coerce(): ?T == ?U == Int
    /// ```
    pub fn destructive_coerce(&self) {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => {
                fv.crack().destructive_coerce();
            }
            Type::FreeVar(fv) if fv.is_unbound() => {
                let (sub, _sup) = fv.get_subsup().unwrap();
                sub.destructive_coerce();
                self.destructive_link(&sub);
            }
            Type::And(l, r) | Type::Or(l, r) => {
                l.destructive_coerce();
                r.destructive_coerce();
            }
            Type::Not(l) => l.destructive_coerce(),
            Type::Poly { params, .. } => {
                for p in params {
                    if let Ok(t) = <&Type>::try_from(p) {
                        t.destructive_coerce();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn undoable_coerce(&self, list: &UndoableLinkedList) {
        match self {
            Type::FreeVar(fv) if fv.is_linked() => {
                fv.crack().undoable_coerce(list);
            }
            Type::FreeVar(fv) if fv.is_unbound() => {
                let (sub, _sup) = fv.get_subsup().unwrap();
                sub.undoable_coerce(list);
                self.undoable_link(&sub, list);
            }
            Type::And(l, r) | Type::Or(l, r) => {
                l.undoable_coerce(list);
                r.undoable_coerce(list);
            }
            Type::Not(l) => l.undoable_coerce(list),
            Type::Poly { params, .. } => {
                for p in params {
                    if let Ok(t) = <&Type>::try_from(p) {
                        t.undoable_coerce(list);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn coerce(&self, list: Option<&UndoableLinkedList>) {
        if let Some(list) = list {
            self.undoable_coerce(list);
        } else {
            self.destructive_coerce();
        }
    }

    /// returns top-level qvars.
    /// see also: `qvars_inner`
    pub fn qvars(&self) -> Set<(Str, Constraint)> {
        match self {
            Self::Quantified(quant) => quant.qvars_inner(),
            _ => self.qvars_inner(),
        }
    }

    /// ```erg
    /// (|T|(T) -> T).qvars() == {T}
    /// (|T|(T) -> T).qvars_inner() == {}
    /// ((|T|(T) -> T) and (Int -> Int)).qvars() == {}
    /// ```
    fn qvars_inner(&self) -> Set<(Str, Constraint)> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.unsafe_crack().qvars_inner(),
            Self::FreeVar(fv) if !fv.constraint_is_uninited() => {
                let base = set! {(fv.unbound_name().unwrap(), fv.constraint().unwrap())};
                if let Some((sub, sup)) = fv.get_subsup() {
                    fv.do_avoiding_recursion(|| {
                        base.concat(sub.qvars_inner()).concat(sup.qvars_inner())
                    })
                } else if let Some(ty) = fv.get_type() {
                    fv.do_avoiding_recursion(|| base.concat(ty.qvars_inner()))
                } else {
                    base
                }
            }
            Self::Ref(ty) => ty.qvars_inner(),
            Self::RefMut { before, after } => before.qvars_inner().concat(
                after
                    .as_ref()
                    .map(|t| t.qvars_inner())
                    .unwrap_or_else(|| set! {}),
            ),
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => lhs.qvars_inner().concat(rhs.qvars_inner()),
            Self::Not(ty) => ty.qvars_inner(),
            Self::Callable { param_ts, return_t } => param_ts
                .iter()
                .fold(set! {}, |acc, t| acc.concat(t.qvars_inner()))
                .concat(return_t.qvars_inner()),
            Self::Subr(subr) => subr.qvars(),
            Self::Record(r) => r
                .values()
                .fold(set! {}, |acc, t| acc.concat(t.qvars_inner())),
            Self::NamedTuple(r) => r
                .iter()
                .fold(set! {}, |acc, (_, t)| acc.concat(t.qvars_inner())),
            Self::Refinement(refine) => refine.t.qvars_inner().concat(refine.pred.qvars()),
            // ((|T| T -> T) and U).qvars() == U.qvars()
            // Self::Quantified(quant) => quant.qvars(),
            Self::Poly { params, .. } => params
                .iter()
                .fold(set! {}, |acc, tp| acc.concat(tp.qvars())),
            Self::Proj { lhs, .. } => lhs.qvars_inner(),
            Self::ProjCall { lhs, args, .. } => lhs
                .qvars()
                .concat(args.iter().fold(set! {}, |acc, tp| acc.concat(tp.qvars()))),
            Self::Structural(ty) => ty.qvars_inner(),
            Self::Guard(guard) => guard.to.qvars_inner(),
            Self::Bounded { sub, sup } => sub.qvars_inner().concat(sup.qvars_inner()),
            _ => set! {},
        }
    }

    pub fn qnames(&self) -> Set<Str> {
        self.qvars().into_iter().map(|(n, _)| n).collect()
    }

    pub fn has_uninited_qvars(&self) -> bool {
        self.qvars().iter().any(|(_, c)| c.is_uninited())
    }

    pub fn is_qvar(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_qvar(),
            Self::FreeVar(fv) if fv.is_generalized() => true,
            _ => false,
        }
    }

    /// if the type is polymorphic
    /// ```erg
    /// assert ('T -> Int).has_qvar()
    /// assert not (|T| T -> T).has_qvar()
    /// ```
    pub fn has_qvar(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().has_qvar(),
            Self::FreeVar(fv) if fv.is_unbound() && fv.is_generalized() => true,
            Self::FreeVar(fv) => {
                if let Some((sub, sup)) = fv.get_subsup() {
                    fv.do_avoiding_recursion(|| sub.has_qvar() || sup.has_qvar())
                } else {
                    let opt_t = fv.get_type();
                    opt_t.map_or(false, |t| t.has_qvar())
                }
            }
            Self::Ref(ty) => ty.has_qvar(),
            Self::RefMut { before, after } => {
                before.has_qvar() || after.as_ref().map(|t| t.has_qvar()).unwrap_or(false)
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => lhs.has_qvar() || rhs.has_qvar(),
            Self::Not(ty) => ty.has_qvar(),
            Self::Callable { param_ts, return_t } => {
                param_ts.iter().any(|t| t.has_qvar()) || return_t.has_qvar()
            }
            Self::Subr(subr) => subr.has_qvar(),
            // Self::Quantified(quant) => quant.has_qvar(),
            Self::Record(r) => r.values().any(|t| t.has_qvar()),
            Self::NamedTuple(r) => r.iter().any(|(_, t)| t.has_qvar()),
            Self::Refinement(refine) => refine.t.has_qvar() || refine.pred.has_qvar(),
            Self::Poly { params, .. } => params.iter().any(|tp| tp.has_qvar()),
            Self::Proj { lhs, .. } => lhs.has_qvar(),
            Self::ProjCall { lhs, args, .. } => {
                lhs.has_qvar() || args.iter().any(|tp| tp.has_qvar())
            }
            Self::Structural(ty) => ty.has_qvar(),
            Self::Guard(guard) => guard.to.has_qvar(),
            Self::Bounded { sub, sup } => sub.has_qvar() || sup.has_qvar(),
            _ => false,
        }
    }

    pub fn is_undoable_linked_var(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_undoable_linked() => true,
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().has_undoable_linked_var(),
            _ => false,
        }
    }

    pub fn has_undoable_linked_var(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_undoable_linked() => true,
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().has_undoable_linked_var(),
            Self::FreeVar(fv) => {
                if let Some((sub, sup)) = fv.get_subsup() {
                    fv.do_avoiding_recursion(|| {
                        sub.has_undoable_linked_var() || sup.has_undoable_linked_var()
                    })
                } else {
                    let opt_t = fv.get_type();
                    opt_t.map_or(false, |t| t.has_undoable_linked_var())
                }
            }
            Self::Ref(ty) => ty.has_undoable_linked_var(),
            Self::RefMut { before, after } => {
                before.has_undoable_linked_var()
                    || after
                        .as_ref()
                        .map(|t| t.has_undoable_linked_var())
                        .unwrap_or(false)
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.has_undoable_linked_var() || rhs.has_undoable_linked_var()
            }
            Self::Not(ty) => ty.has_undoable_linked_var(),
            Self::Callable { param_ts, return_t } => {
                param_ts.iter().any(|t| t.has_undoable_linked_var())
                    || return_t.has_undoable_linked_var()
            }
            Self::Subr(subr) => subr.has_undoable_linked_var(),
            Self::Quantified(quant) => quant.has_undoable_linked_var(),
            Self::Record(r) => r.values().any(|t| t.has_undoable_linked_var()),
            Self::NamedTuple(r) => r.iter().any(|(_, t)| t.has_undoable_linked_var()),
            Self::Refinement(refine) => {
                refine.t.has_undoable_linked_var() || refine.pred.has_undoable_linked_var()
            }
            Self::Poly { params, .. } => params.iter().any(|tp| tp.has_undoable_linked_var()),
            Self::Proj { lhs, .. } => lhs.has_undoable_linked_var(),
            Self::ProjCall { lhs, args, .. } => {
                lhs.has_undoable_linked_var() || args.iter().any(|tp| tp.has_undoable_linked_var())
            }
            Self::Structural(ty) => ty.has_undoable_linked_var(),
            Self::Guard(guard) => guard.to.has_undoable_linked_var(),
            Self::Bounded { sub, sup } => {
                sub.has_undoable_linked_var() || sup.has_undoable_linked_var()
            }
            _ => false,
        }
    }

    pub fn has_no_qvar(&self) -> bool {
        !self.has_qvar()
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
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.has_unbound_var() || rhs.has_unbound_var()
            }
            Self::Not(ty) => ty.has_unbound_var(),
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
            Self::NamedTuple(r) => r.iter().any(|(_, t)| t.has_unbound_var()),
            Self::Refinement(refine) => refine.t.has_unbound_var() || refine.pred.has_unbound_var(),
            Self::Quantified(quant) => quant.has_unbound_var(),
            Self::Poly { params, .. } => params.iter().any(|p| p.has_unbound_var()),
            Self::Proj { lhs, .. } => lhs.has_unbound_var(),
            Self::ProjCall { lhs, args, .. } => {
                lhs.has_unbound_var() || args.iter().any(|t| t.has_unbound_var())
            }
            Self::Structural(ty) => ty.has_unbound_var(),
            Self::Guard(guard) => guard.to.has_unbound_var(),
            Self::Bounded { sub, sup } => sub.has_unbound_var() || sup.has_unbound_var(),
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
            Self::And(_, _) | Self::Or(_, _) => Some(2),
            Self::Not(_) => Some(1),
            Self::Subr(subr) => Some(
                subr.non_default_params.len()
                    + subr.var_params.as_ref().map(|_| 1).unwrap_or(0)
                    + subr.default_params.len()
                    + 1,
            ),
            Self::Callable { param_ts, .. } => Some(param_ts.len() + 1),
            Self::Poly { params, .. } => Some(params.len()),
            Self::Proj { lhs, .. } => lhs.typarams_len(),
            Self::ProjCall { args, .. } => Some(1 + args.len()),
            Self::Structural(ty) => ty.typarams_len(),
            _ => None,
        }
    }

    pub fn singleton_value(&self) -> Option<&TyParam> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.unsafe_crack().singleton_value(),
            Self::Refinement(refine) => {
                if let Predicate::Equal { rhs, .. } = refine.pred.as_ref() {
                    Some(rhs)
                } else {
                    None
                }
            }
            Self::NoneType => Some(&TyParam::Value(ValueObj::None)),
            Self::Ellipsis => Some(&TyParam::Value(ValueObj::Ellipsis)),
            _ => None,
        }
    }

    pub fn refinement_values(&self) -> Option<Vec<&TyParam>> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.unsafe_crack().refinement_values(),
            Self::Refinement(refine) => Some(refine.pred.possible_values()),
            _ => None,
        }
    }

    pub fn container_len(&self) -> Option<usize> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().container_len(),
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
            Self::NamedTuple(r) => Some(r.len()),
            _ => None,
        }
    }

    pub fn typarams(&self) -> Vec<TyParam> {
        match self {
            Self::FreeVar(f) if f.is_linked() => f.crack().typarams(),
            Self::FreeVar(_unbound) => vec![],
            Self::Refinement(refine) => refine.t.typarams(),
            Self::Ref(t) | Self::RefMut { before: t, .. } => vec![TyParam::t(*t.clone())],
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                vec![TyParam::t(*lhs.clone()), TyParam::t(*rhs.clone())]
            }
            Self::Not(t) => vec![TyParam::t(*t.clone())],
            Self::Subr(subr) => subr.typarams(),
            Self::Quantified(quant) => quant.typarams(),
            Self::Callable { param_ts: _, .. } => todo!(),
            Self::NamedTuple(r) => r.iter().map(|(_, t)| TyParam::t(t.clone())).collect(),
            Self::Poly { params, .. } => params.clone(),
            Self::Proj { lhs, .. } => lhs.typarams(),
            Self::ProjCall { lhs, args, .. } => {
                [vec![*lhs.clone()], args.deref().to_vec()].concat()
            }
            Self::Structural(ty) => ty.typarams(),
            _ => vec![],
        }
    }

    pub fn self_t(&self) -> Option<&Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                fv.forced_as_ref().linked().and_then(|t| t.self_t())
            }
            Self::Refinement(refine) => refine.t.self_t(),
            Self::Subr(subr) => subr.self_t(),
            Self::Quantified(quant) => quant.self_t(),
            _ => None,
        }
    }

    pub fn non_default_params(&self) -> Option<&Vec<ParamTy>> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv
                .forced_as_ref()
                .linked()
                .and_then(|t| t.non_default_params()),
            Self::Refinement(refine) => refine.t.non_default_params(),
            Self::Subr(SubrType {
                non_default_params, ..
            }) => Some(non_default_params),
            Self::Quantified(quant) => quant.non_default_params(),
            Self::Callable { param_ts: _, .. } => todo!(),
            _ => None,
        }
    }

    pub fn var_params(&self) -> Option<&ParamTy> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                fv.forced_as_ref().linked().and_then(|t| t.var_params())
            }
            Self::Refinement(refine) => refine.t.var_params(),
            Self::Subr(SubrType {
                var_params: var_args,
                ..
            }) => var_args.as_deref(),
            Self::Quantified(quant) => quant.var_params(),
            Self::Callable { param_ts: _, .. } => todo!(),
            _ => None,
        }
    }

    pub fn default_params(&self) -> Option<&Vec<ParamTy>> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                fv.forced_as_ref().linked().and_then(|t| t.default_params())
            }
            Self::Refinement(refine) => refine.t.default_params(),
            Self::Subr(SubrType { default_params, .. }) => Some(default_params),
            Self::Quantified(quant) => quant.default_params(),
            _ => None,
        }
    }

    pub fn non_var_params(&self) -> Option<impl Iterator<Item = &ParamTy> + Clone> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                fv.forced_as_ref().linked().and_then(|t| t.non_var_params())
            }
            Self::Refinement(refine) => refine.t.non_var_params(),
            Self::Subr(subr) => Some(subr.non_var_params()),
            Self::Quantified(quant) => quant.non_var_params(),
            _ => None,
        }
    }

    pub fn return_t(&self) -> Option<&Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                fv.forced_as_ref().linked().and_then(|t| t.return_t())
            }
            Self::Refinement(refine) => refine.t.return_t(),
            Self::Subr(SubrType { return_t, .. }) | Self::Callable { return_t, .. } => {
                Some(return_t)
            }
            // NOTE: Quantified could return a quantified type variable.
            // At least in situations where this function is needed, self cannot be Quantified.
            Self::Quantified(quant) => {
                if quant.return_t().unwrap().is_generalized() {
                    log!(err "quantified return type (recursive function type inference)");
                }
                quant.return_t()
            }
            Self::Failure => Some(&Type::Failure),
            _ => None,
        }
    }

    pub fn tyvar_mut_return_t(&mut self) -> Option<RefMut<Type>> {
        match self {
            Self::FreeVar(fv)
                if fv.is_linked() && fv.get_linked().unwrap().return_t().is_some() =>
            {
                Some(RefMut::map(fv.borrow_mut(), |fk| {
                    fk.linked_mut().unwrap().mut_return_t().unwrap()
                }))
            }
            _ => None,
        }
    }

    pub fn mut_return_t(&mut self) -> Option<&mut Type> {
        match self {
            Self::Refinement(refine) => refine.t.mut_return_t(),
            Self::Subr(SubrType { return_t, .. }) | Self::Callable { return_t, .. } => {
                Some(return_t)
            }
            Self::Quantified(quant) => {
                if quant.return_t().unwrap().is_generalized() {
                    log!(err "quantified return type (recursive function type inference)");
                }
                quant.mut_return_t()
            }
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
                    // if fv == ?T(:> {1, 2}, <: Sub(?T)), derefine() will cause infinite loop
                    // so we need to force linking
                    fv.do_avoiding_recursion(|| {
                        let constraint = Constraint::new_sandwiched(sub.derefine(), sup.derefine());
                        Self::FreeVar(Free::new_named_unbound(name, level, constraint))
                    })
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
            Self::Record(rec) => {
                let rec = rec.iter().map(|(k, v)| (k.clone(), v.derefine())).collect();
                Self::Record(rec)
            }
            Self::NamedTuple(r) => {
                let r = r.iter().map(|(k, v)| (k.clone(), v.derefine())).collect();
                Self::NamedTuple(r)
            }
            Self::And(l, r) => l.derefine() & r.derefine(),
            Self::Or(l, r) => l.derefine() | r.derefine(),
            Self::Not(ty) => !ty.derefine(),
            Self::Proj { lhs, rhs } => lhs.derefine().proj(rhs.clone()),
            Self::ProjCall {
                lhs,
                attr_name,
                args,
            } => {
                let lhs = match lhs.as_ref() {
                    TyParam::Type(t) => TyParam::t(t.derefine()),
                    other => other.clone(),
                };
                let args = args
                    .iter()
                    .map(|arg| match arg {
                        TyParam::Type(t) => TyParam::t(t.derefine()),
                        other => other.clone(),
                    })
                    .collect();
                proj_call(lhs, attr_name.clone(), args)
            }
            Self::Structural(ty) => ty.derefine().structuralize(),
            Self::Guard(guard) => Self::Guard(GuardType::new(
                guard.namespace.clone(),
                guard.target.clone(),
                guard.to.derefine(),
            )),
            Self::Bounded { sub, sup } => Self::Bounded {
                sub: Box::new(sub.derefine()),
                sup: Box::new(sup.derefine()),
            },
            other => other.clone(),
        }
    }

    pub fn eliminate(self, target: &Type) -> Self {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                t.eliminate(target)
            }
            Self::FreeVar(ref fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = fv.get_subsup().unwrap();
                let sub = sub.eliminate(target);
                let sup = sup.eliminate(target);
                self.update_tyvar(sub, sup, None, false);
                self
            }
            Self::And(l, r) => {
                if l.addr_eq(target) {
                    return r.eliminate(target);
                } else if r.addr_eq(target) {
                    return l.eliminate(target);
                }
                l.eliminate(target) & r.eliminate(target)
            }
            Self::Or(l, r) => {
                if l.addr_eq(target) {
                    return r.eliminate(target);
                } else if r.addr_eq(target) {
                    return l.eliminate(target);
                }
                l.eliminate(target) | r.eliminate(target)
            }
            other => other,
        }
    }

    pub fn replace(self, target: &Type, to: &Type) -> Type {
        let table = ReplaceTable::make(target, to);
        table.replace(self)
    }

    /// ```erg
    /// (Failure -> Int).replace_failure() == (Obj -> Int)
    /// (Int -> Failure).replace_failure() == (Int -> Never)
    /// Array(Failure, 3).replace_failure() == Array(Never, 3)
    /// ```
    pub fn replace_failure(&self) -> Type {
        match self {
            Self::Quantified(quant) => quant.replace_failure().quantify(),
            Self::Subr(subr) => {
                let non_default_params = subr
                    .non_default_params
                    .iter()
                    .map(|pt| {
                        pt.clone()
                            .map_type(|t| t.replace(&Self::Failure, &Self::Obj))
                    })
                    .collect();
                let var_params = subr.var_params.as_ref().map(|pt| {
                    pt.clone()
                        .map_type(|t| t.replace(&Self::Failure, &Self::Obj))
                });
                let default_params = subr
                    .default_params
                    .iter()
                    .map(|pt| {
                        pt.clone()
                            .map_type(|t| t.replace(&Self::Failure, &Self::Obj))
                    })
                    .collect();
                let kw_var_params = subr.kw_var_params.as_ref().map(|pt| {
                    pt.clone()
                        .map_type(|t| t.replace(&Self::Failure, &Self::Obj))
                });
                let return_t = subr.return_t.clone().replace(&Self::Failure, &Self::Never);
                subr_t(
                    subr.kind,
                    non_default_params,
                    var_params,
                    default_params,
                    kw_var_params,
                    return_t,
                )
            }
            // TODO: consider variances
            _ => self.clone().replace(&Self::Failure, &Self::Never),
        }
    }

    fn _replace(mut self, target: &Type, to: &Type) -> Type {
        if self.structural_eq(target) {
            self = to.clone();
        }
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().clone()._replace(target, to),
            Self::FreeVar(fv) => {
                let fv_clone = fv.deep_clone();
                if let Some((sub, sup)) = fv_clone.get_subsup() {
                    fv.dummy_link();
                    fv_clone.dummy_link();
                    let sub = sub._replace(target, to);
                    let sup = sup._replace(target, to);
                    fv.undo();
                    fv_clone.undo();
                    fv_clone.update_constraint(Constraint::new_sandwiched(sub, sup), true);
                } else if let Some(ty) = fv_clone.get_type() {
                    fv_clone
                        .update_constraint(Constraint::new_type_of(ty._replace(target, to)), true);
                }
                Self::FreeVar(fv_clone)
            }
            Self::Refinement(mut refine) => {
                refine.t = Box::new(refine.t._replace(target, to));
                Self::Refinement(refine)
            }
            Self::Record(mut rec) => {
                for v in rec.values_mut() {
                    *v = std::mem::take(v)._replace(target, to);
                }
                Self::Record(rec)
            }
            Self::NamedTuple(mut r) => {
                for (_, v) in r.iter_mut() {
                    *v = std::mem::take(v)._replace(target, to);
                }
                Self::NamedTuple(r)
            }
            Self::Subr(mut subr) => {
                for nd in subr.non_default_params.iter_mut() {
                    *nd.typ_mut() = std::mem::take(nd.typ_mut())._replace(target, to);
                }
                if let Some(var) = subr.var_params.as_mut() {
                    *var.as_mut().typ_mut() =
                        std::mem::take(var.as_mut().typ_mut())._replace(target, to);
                }
                for d in subr.default_params.iter_mut() {
                    *d.typ_mut() = std::mem::take(d.typ_mut())._replace(target, to);
                }
                subr.return_t = Box::new(subr.return_t._replace(target, to));
                Self::Subr(subr)
            }
            Self::Callable { param_ts, return_t } => {
                let param_ts = param_ts
                    .into_iter()
                    .map(|t| t._replace(target, to))
                    .collect();
                let return_t = Box::new(return_t._replace(target, to));
                Self::Callable { param_ts, return_t }
            }
            Self::Quantified(quant) => quant._replace(target, to).quantify(),
            Self::Poly { name, params } => {
                let params = params
                    .into_iter()
                    .map(|tp| tp.replace(target, to))
                    .collect();
                Self::Poly { name, params }
            }
            Self::Ref(t) => Self::Ref(Box::new(t._replace(target, to))),
            Self::RefMut { before, after } => Self::RefMut {
                before: Box::new(before._replace(target, to)),
                after: after.map(|t| Box::new(t._replace(target, to))),
            },
            Self::And(l, r) => l._replace(target, to) & r._replace(target, to),
            Self::Or(l, r) => l._replace(target, to) | r._replace(target, to),
            Self::Not(ty) => !ty._replace(target, to),
            Self::Proj { lhs, rhs } => lhs._replace(target, to).proj(rhs),
            Self::ProjCall {
                lhs,
                attr_name,
                args,
            } => {
                let args = args.into_iter().map(|tp| tp.replace(target, to)).collect();
                proj_call(lhs.replace(target, to), attr_name, args)
            }
            Self::Structural(ty) => ty._replace(target, to).structuralize(),
            Self::Guard(guard) => Self::Guard(GuardType::new(
                guard.namespace,
                guard.target.clone(),
                guard.to._replace(target, to),
            )),
            Self::Bounded { sub, sup } => Self::Bounded {
                sub: Box::new(sub._replace(target, to)),
                sup: Box::new(sup._replace(target, to)),
            },
            other => other,
        }
    }

    /// TyParam::Value(ValueObj::Type(_)) => TyParam::Type
    pub fn normalize(self) -> Self {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().clone().normalize(),
            Self::Poly { name, params } => {
                let params = params.into_iter().map(|tp| tp.normalize()).collect();
                Self::Poly { name, params }
            }
            Self::Subr(mut subr) => {
                for nd in subr.non_default_params.iter_mut() {
                    *nd.typ_mut() = std::mem::take(nd.typ_mut()).normalize();
                }
                if let Some(var) = subr.var_params.as_mut() {
                    *var.as_mut().typ_mut() = std::mem::take(var.as_mut().typ_mut()).normalize();
                }
                for d in subr.default_params.iter_mut() {
                    *d.typ_mut() = std::mem::take(d.typ_mut()).normalize();
                }
                subr.return_t = Box::new(subr.return_t.normalize());
                Self::Subr(subr)
            }
            Self::Proj { lhs, rhs } => lhs.normalize().proj(rhs),
            Self::ProjCall {
                lhs,
                attr_name,
                args,
            } => {
                let args = args.into_iter().map(|tp| tp.normalize()).collect();
                proj_call(lhs.normalize(), attr_name, args)
            }
            Self::Ref(t) => Self::Ref(Box::new(t.normalize())),
            Self::RefMut { before, after } => Self::RefMut {
                before: Box::new(before.normalize()),
                after: after.map(|t| Box::new(t.normalize())),
            },
            Self::Record(mut rec) => {
                for v in rec.values_mut() {
                    *v = std::mem::take(v).normalize();
                }
                Self::Record(rec)
            }
            Self::NamedTuple(mut r) => {
                for (_, v) in r.iter_mut() {
                    *v = std::mem::take(v).normalize();
                }
                Self::NamedTuple(r)
            }
            Self::And(l, r) => l.normalize() & r.normalize(),
            Self::Or(l, r) => l.normalize() | r.normalize(),
            Self::Not(ty) => !ty.normalize(),
            Self::Structural(ty) => ty.normalize().structuralize(),
            Self::Guard(guard) => Self::Guard(GuardType::new(
                guard.namespace,
                guard.target,
                guard.to.normalize(),
            )),
            Self::Bounded { sub, sup } => Self::Bounded {
                sub: Box::new(sub.normalize()),
                sup: Box::new(sup.normalize()),
            },
            other => other,
        }
    }

    /// ```erg
    /// assert Int.lower_bounded() == Int
    /// assert ?T(:> Str).lower_bounded() == Str
    /// assert (?T(:> Str) or ?U(:> Int)).lower_bounded() == (Str or Int)
    /// ```
    pub fn lower_bounded(&self) -> Type {
        if let Ok(free) = <&FreeTyVar>::try_from(self) {
            free.get_sub().unwrap_or(self.clone())
        } else {
            match self {
                Self::And(l, r) => l.lower_bounded() & r.lower_bounded(),
                Self::Or(l, r) => l.lower_bounded() | r.lower_bounded(),
                Self::Not(ty) => !ty.lower_bounded(),
                _ => self.clone(),
            }
        }
    }

    fn addr_eq(&self, other: &Type) -> bool {
        match (self, other) {
            (Self::FreeVar(slf), _) if slf.is_linked() => slf.crack().addr_eq(other),
            (_, Self::FreeVar(otr)) if otr.is_linked() => otr.crack().addr_eq(self),
            (Self::FreeVar(slf), Self::FreeVar(otr)) => slf.addr_eq(otr),
            _ => ref_addr_eq!(self, other),
        }
    }

    /// interior-mut
    pub(crate) fn destructive_link(&self, to: &Type) {
        if self.addr_eq(to) {
            return;
        }
        if self.level() == Some(GENERIC_LEVEL) {
            panic!("{self} is fixed");
        }
        let to = to.clone().eliminate(self);
        match self {
            Self::FreeVar(fv) => fv.link(&to),
            Self::Refinement(refine) => refine.t.destructive_link(&to),
            _ => panic!("{self} is not a free variable"),
        }
    }

    /// interior-mut
    ///
    /// `inc/dec_undo_count` due to the number of `substitute_typarams/undo_typarams` must be matched
    pub(crate) fn undoable_link(&self, to: &Type, list: &UndoableLinkedList) {
        list.push_t(self);
        if self.addr_eq(to) {
            self.inc_undo_count();
            return;
        }
        match self {
            Self::FreeVar(fv) => fv.undoable_link(to),
            Self::Refinement(refine) => refine.t.undoable_link(to, list),
            _ => panic!("{self} is not a free variable"),
        }
    }

    pub(crate) fn link(&self, to: &Type, list: Option<&UndoableLinkedList>) {
        if let Some(list) = list {
            self.undoable_link(to, list);
        } else {
            self.destructive_link(to);
        }
    }

    pub(crate) fn undo(&self) {
        match self {
            Self::FreeVar(fv) if fv.is_undoable_linked() => fv.undo(),
            /*Self::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = fv.get_subsup().unwrap();
                sub.undo();
                sup.undo();
            }
            Self::Poly { params, .. } => {
                for param in params {
                    param.undo();
                }
            }*/
            _ => {}
        }
    }

    pub(crate) fn undoable_update_constraint(
        &self,
        new_constraint: Constraint,
        list: &UndoableLinkedList,
    ) {
        let level = self.level().unwrap();
        let new = if let Some(name) = self.unbound_name() {
            named_free_var(name, level, new_constraint)
        } else {
            free_var(level, new_constraint)
        };
        self.undoable_link(&new, list);
    }

    pub(crate) fn update_constraint(
        &self,
        new_constraint: Constraint,
        list: Option<&UndoableLinkedList>,
        in_instantiation: bool,
    ) {
        let new_constraint = new_constraint.eliminate_recursion(self);
        if let Some(list) = list {
            self.undoable_update_constraint(new_constraint, list);
        } else {
            self.destructive_update_constraint(new_constraint, in_instantiation);
        }
    }

    pub(crate) fn update_tyvar(
        &self,
        new_sub: Type,
        new_sup: Type,
        list: Option<&UndoableLinkedList>,
        in_instantiation: bool,
    ) {
        if new_sub == new_sup {
            self.link(&new_sub, list);
        } else {
            let new_constraint = Constraint::new_sandwiched(new_sub, new_sup);
            self.update_constraint(new_constraint, list, in_instantiation);
        }
    }

    fn inc_undo_count(&self) {
        match self {
            Self::FreeVar(fv) => fv.inc_undo_count(),
            Self::Refinement(refine) => refine.t.inc_undo_count(),
            _ => panic!("{self} is not a free variable"),
        }
    }

    pub fn into_bounded(&self) -> Type {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().clone().into_bounded(),
            Self::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = fv.get_subsup().unwrap();
                bounded(sub, sup)
            }
            Self::Refinement(refine) => refine.t.as_ref().clone().into_bounded(),
            _ => self.clone(),
        }
    }

    /// ```erg
    /// Add.ands() == {Add}
    /// (Add and Sub).ands() == {Add, Sub}
    /// ```
    pub fn ands(&self) -> Set<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().ands(),
            Self::Refinement(refine) => refine.t.ands(),
            Self::And(l, r) => l.ands().union(&r.ands()),
            _ => set![self.clone()],
        }
    }

    /// ```erg
    /// Int.ors() == {Int}
    /// (Int or Str).ors() == {Int, Str}
    /// ```
    pub fn ors(&self) -> Set<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().ors(),
            Self::Refinement(refine) => refine.t.ors(),
            Self::Or(l, r) => l.ors().union(&r.ors()),
            _ => set![self.clone()],
        }
    }

    /// ```erg
    /// Int.contained_ts() == {Int}
    /// Array(Array(Int)).contained_ts() == {Array(Int), Int}
    /// (Int or Str).contained_ts() == {Int, Str}
    /// ```
    pub fn contained_ts(&self) -> Set<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contained_ts(),
            Self::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = fv.get_subsup().unwrap();
                fv.do_avoiding_recursion(|| {
                    set! { self.clone() }
                        .union(&sub.contained_ts())
                        .union(&sup.contained_ts())
                })
            }
            Self::Refinement(refine) => refine.t.contained_ts(),
            Self::Ref(t) => t.contained_ts(),
            Self::RefMut { before, .. } => before.contained_ts(),
            Self::Subr(sub) => {
                let mut ts = set! {};
                for nd in sub.non_default_params.iter() {
                    ts.extend(nd.typ().contained_ts());
                }
                if let Some(var) = sub.var_params.as_ref() {
                    ts.extend(var.typ().contained_ts());
                }
                for d in sub.default_params.iter() {
                    ts.extend(d.typ().contained_ts());
                }
                ts.extend(sub.return_t.contained_ts());
                ts
            }
            Self::Callable { param_ts, .. } => {
                param_ts.iter().flat_map(|t| t.contained_ts()).collect()
            }
            Self::And(l, r) | Self::Or(l, r) => l.contained_ts().union(&r.contained_ts()),
            Self::Not(t) => t.contained_ts(),
            Self::Bounded { sub, sup } => sub.contained_ts().union(&sup.contained_ts()),
            Self::Quantified(ty) | Self::Structural(ty) => ty.contained_ts(),
            Self::Record(rec) => rec.values().flat_map(|t| t.contained_ts()).collect(),
            Self::NamedTuple(r) => r.iter().flat_map(|(_, t)| t.contained_ts()).collect(),
            Self::Proj { lhs, .. } => lhs.contained_ts(),
            Self::ProjCall { lhs, args, .. } => {
                let mut ts = set! {};
                ts.extend(lhs.contained_ts());
                ts.extend(args.iter().flat_map(|tp| tp.contained_ts()));
                ts
            }
            Self::Poly { params, .. } => {
                let mut ts = set! { self.clone() };
                ts.extend(params.iter().flat_map(|tp| tp.contained_ts()));
                ts
            }
            _ => set! { self.clone() },
        }
    }

    pub fn dereference(&mut self) {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => {
                let new = fv.crack().clone();
                *self = new;
                self.dereference();
            }
            Self::FreeVar(fv) if fv.is_generalized() => {
                fv.update_init();
            }
            // TODO: T(:> X, <: Y).dereference()
            Self::Refinement(refine) => refine.t.dereference(),
            Self::Ref(t) => {
                t.dereference();
            }
            Self::RefMut { before, after } => {
                before.dereference();
                if let Some(after) = after.as_mut() {
                    after.dereference();
                }
            }
            Self::Subr(sub) => {
                for nd in sub.non_default_params.iter_mut() {
                    nd.typ_mut().dereference();
                }
                if let Some(var) = sub.var_params.as_mut() {
                    var.typ_mut().dereference();
                }
                for d in sub.default_params.iter_mut() {
                    d.typ_mut().dereference();
                }
                sub.return_t.dereference();
            }
            Self::Callable { param_ts, return_t } => {
                for t in param_ts.iter_mut() {
                    t.dereference();
                }
                return_t.dereference();
            }
            Self::And(l, r) | Self::Or(l, r) => {
                l.dereference();
                r.dereference();
            }
            Self::Not(ty) => {
                ty.dereference();
            }
            Self::Bounded { sub, sup } => {
                sub.dereference();
                sup.dereference();
            }
            Self::Quantified(ty) | Self::Structural(ty) => {
                ty.dereference();
            }
            Self::Record(rec) => {
                for v in rec.values_mut() {
                    v.dereference();
                }
            }
            Self::NamedTuple(r) => {
                for (_, v) in r.iter_mut() {
                    v.dereference();
                }
            }
            Self::Proj { lhs, .. } => {
                lhs.dereference();
            }
            Self::ProjCall { lhs, args, .. } => {
                lhs.dereference();
                for arg in args.iter_mut() {
                    arg.dereference();
                }
            }
            Self::Poly { params, .. } => {
                for param in params.iter_mut() {
                    param.dereference();
                }
            }
            Self::Guard(guard) => {
                guard.to.dereference();
            }
            _ => {}
        }
    }

    pub fn module_path(&self) -> Option<PathBuf> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().module_path(),
            Self::Refinement(refine) => refine.t.module_path(),
            _ if self.is_module() => {
                let tps = self.typarams();
                let Some(TyParam::Value(ValueObj::Str(path))) = tps.get(0) else {
                    return None;
                };
                Some(PathBuf::from(&path[..]))
            }
            _ => None,
        }
    }
}

pub struct ReplaceTable<'t> {
    rules: Vec<(&'t Type, &'t Type)>,
}

impl<'t> ReplaceTable<'t> {
    pub fn make(target: &'t Type, to: &'t Type) -> Self {
        let mut self_ = ReplaceTable { rules: vec![] };
        self_.iterate(target, to);
        self_
    }

    pub fn replace(&self, mut ty: Type) -> Type {
        for (target, to) in self.rules.iter() {
            ty = ty._replace(target, to);
        }
        ty
    }

    fn iterate(&mut self, target: &'t Type, to: &'t Type) {
        match (target, to) {
            (
                Type::Poly { name, params },
                Type::Poly {
                    name: name2,
                    params: params2,
                },
            ) if name == name2 => {
                for (t1, t2) in params.iter().zip(params2.iter()) {
                    self.iterate_tp(t1, t2);
                }
            }
            (Type::Subr(lsub), Type::Subr(rsub)) => {
                for (lnd, rnd) in lsub
                    .non_default_params
                    .iter()
                    .zip(rsub.non_default_params.iter())
                {
                    self.iterate(lnd.typ(), rnd.typ());
                }
                for (lv, rv) in lsub.var_params.iter().zip(rsub.var_params.iter()) {
                    self.iterate(lv.typ(), rv.typ());
                }
                for (ld, rd) in lsub.default_params.iter().zip(rsub.default_params.iter()) {
                    self.iterate(ld.typ(), rd.typ());
                }
                self.iterate(lsub.return_t.as_ref(), rsub.return_t.as_ref());
            }
            (Type::Quantified(quant), Type::Quantified(quant2)) => {
                self.iterate(quant, quant2);
            }
            (
                Type::Proj { lhs, rhs },
                Type::Proj {
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if rhs == rhs2 => {
                self.iterate(lhs, lhs2);
            }
            (Type::Record(rec), Type::Record(rec2)) => {
                for (l, r) in rec.values().zip(rec2.values()) {
                    self.iterate(l, r);
                }
            }
            (Type::NamedTuple(r), Type::NamedTuple(r2)) => {
                for ((_, l), (_, r)) in r.iter().zip(r2.iter()) {
                    self.iterate(l, r);
                }
            }
            (Type::And(l, r), Type::And(l2, r2)) => {
                self.iterate(l, l2);
                self.iterate(r, r2);
            }
            (Type::Or(l, r), Type::Or(l2, r2)) => {
                self.iterate(l, l2);
                self.iterate(r, r2);
            }
            (Type::Not(t), Type::Not(t2)) => {
                self.iterate(t, t2);
            }
            (Type::Ref(t), Type::Ref(t2)) => {
                self.iterate(t, t2);
            }
            (
                Type::RefMut { before, after },
                Type::RefMut {
                    before: before2,
                    after: after2,
                },
            ) => {
                self.iterate(before, before2);
                if let (Some(after), Some(after2)) = (after.as_ref(), after2.as_ref()) {
                    self.iterate(after, after2);
                }
            }
            (Type::Structural(t), Type::Structural(t2)) => {
                self.iterate(t, t2);
            }
            (Type::Guard(guard), Type::Guard(guard2)) => {
                self.iterate(&guard.to, &guard2.to);
            }
            (
                Type::Bounded { sub, sup },
                Type::Bounded {
                    sub: sub2,
                    sup: sup2,
                },
            ) => {
                self.iterate(sub, sub2);
                self.iterate(sup, sup2);
            }
            _ => {}
        }
        self.rules.push((target, to));
    }

    fn iterate_tp(&mut self, target: &'t TyParam, to: &'t TyParam) {
        match (target, to) {
            (TyParam::FreeVar(fv), to) if fv.is_linked() => self.iterate_tp(fv.unsafe_crack(), to),
            (TyParam::Value(ValueObj::Type(target)), TyParam::Value(ValueObj::Type(to))) => {
                self.iterate(target.typ(), to.typ());
            }
            (TyParam::Type(t1), TyParam::Type(t2)) => self.iterate(t1, t2),
            (TyParam::Value(ValueObj::Type(t1)), TyParam::Type(t2)) => {
                self.iterate(t1.typ(), t2);
            }
            (TyParam::Type(t1), TyParam::Value(ValueObj::Type(t2))) => {
                self.iterate(t1, t2.typ());
            }
            _ => {}
        }
    }
}

/// Opcode used when Erg implements its own processor
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
