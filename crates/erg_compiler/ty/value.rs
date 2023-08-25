//! defines `ValueObj` (used in the compiler, VM).
//!
//! コンパイラ、VM等で使われる(データも保持した)値オブジェクトを定義する
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Neg;
use std::sync::Arc;

use erg_common::dict::Dict;
use erg_common::error::{ErrorCore, ErrorKind, Location};
use erg_common::fresh::FRESH_GEN;
use erg_common::io::Input;
use erg_common::python_util::PythonVersion;
use erg_common::serialize::*;
use erg_common::set::Set;
use erg_common::traits::LimitedDisplay;
use erg_common::{dict, fmt_iter, impl_display_from_debug, log, switch_lang};
use erg_common::{ArcArray, Str};
use erg_parser::ast::{ConstArgs, ConstExpr};

use crate::context::eval::type_from_token_kind;
use crate::context::Context;

use self::value_set::inner_class;

use super::codeobj::CodeObj;
use super::constructors::{array_t, dict_t, refinement, set_t, tuple_t};
use super::typaram::{OpKind, TyParam};
use super::{ConstSubr, Field, HasType, Predicate, Type};
use super::{CONTAINER_OMIT_THRESHOLD, STR_OMIT_THRESHOLD};

pub struct EvalValueError(pub Box<ErrorCore>);

impl From<ErrorCore> for EvalValueError {
    fn from(core: ErrorCore) -> Self {
        Self(Box::new(core))
    }
}

impl From<EvalValueError> for ErrorCore {
    fn from(err: EvalValueError) -> Self {
        *err.0
    }
}

impl EvalValueError {
    pub fn feature_error(_input: Input, loc: Location, name: &str, caused_by: String) -> Self {
        Self::from(ErrorCore::new(
            vec![],
            format!("{name} is not supported yet: {caused_by}"),
            0,
            ErrorKind::FeatureError,
            loc,
        ))
    }
}

pub type EvalValueResult<T> = Result<T, EvalValueError>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClassTypeObj {
    pub t: Type,
    pub base: Option<Box<TypeObj>>,
    pub impls: Option<Box<TypeObj>>,
    pub inited: bool,
}

impl ClassTypeObj {
    pub fn new(t: Type, base: Option<TypeObj>, impls: Option<TypeObj>, inited: bool) -> Self {
        Self {
            t,
            base: base.map(Box::new),
            impls: impls.map(Box::new),
            inited,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InheritedTypeObj {
    pub t: Type,
    pub sup: Box<TypeObj>,
    pub impls: Option<Box<TypeObj>>,
    pub additional: Option<Box<TypeObj>>,
}

impl InheritedTypeObj {
    pub fn new(t: Type, sup: TypeObj, impls: Option<TypeObj>, additional: Option<TypeObj>) -> Self {
        Self {
            t,
            sup: Box::new(sup),
            impls: impls.map(Box::new),
            additional: additional.map(Box::new),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TraitTypeObj {
    pub t: Type,
    pub requires: Box<TypeObj>,
    pub impls: Option<Box<TypeObj>>,
    pub inited: bool,
}

impl TraitTypeObj {
    pub fn new(t: Type, requires: TypeObj, impls: Option<TypeObj>, inited: bool) -> Self {
        Self {
            t,
            requires: Box::new(requires),
            impls: impls.map(Box::new),
            inited,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubsumedTypeObj {
    pub t: Type,
    pub sup: Box<TypeObj>,
    pub impls: Option<Box<TypeObj>>,
    pub additional: Option<Box<TypeObj>>,
}

impl SubsumedTypeObj {
    pub fn new(t: Type, sup: TypeObj, impls: Option<TypeObj>, additional: Option<TypeObj>) -> Self {
        Self {
            t,
            sup: Box::new(sup),
            impls: impls.map(Box::new),
            additional: additional.map(Box::new),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnionTypeObj {
    pub t: Type,
    pub lhs: Box<TypeObj>,
    pub rhs: Box<TypeObj>,
}

impl UnionTypeObj {
    pub fn new(t: Type, lhs: TypeObj, rhs: TypeObj) -> Self {
        Self {
            t,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntersectionTypeObj {
    pub t: Type,
    pub lhs: Box<TypeObj>,
    pub rhs: Box<TypeObj>,
}

impl IntersectionTypeObj {
    pub fn new(t: Type, lhs: TypeObj, rhs: TypeObj) -> Self {
        Self {
            t,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructuralTypeObj {
    pub t: Type,
    pub base: Box<TypeObj>,
}

impl StructuralTypeObj {
    pub fn new(t: Type, base: TypeObj) -> Self {
        Self {
            t,
            base: Box::new(base),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PatchObj {
    pub t: Type,
    pub base: Box<TypeObj>,
    pub impls: Option<Box<TypeObj>>,
}

impl PatchObj {
    pub fn new(t: Type, base: TypeObj, impls: Option<TypeObj>) -> Self {
        Self {
            t,
            base: Box::new(base),
            impls: impls.map(Box::new),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenTypeObj {
    Class(ClassTypeObj),
    Subclass(InheritedTypeObj),
    Trait(TraitTypeObj),
    Subtrait(SubsumedTypeObj),
    Structural(StructuralTypeObj),
    Union(UnionTypeObj),
    Intersection(IntersectionTypeObj),
    Patch(PatchObj),
}

impl fmt::Display for GenTypeObj {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}>", self.typ())
    }
}

impl LimitedDisplay for GenTypeObj {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> std::fmt::Result {
        write!(f, "<")?;
        self.typ().limited_fmt(f, limit)?;
        write!(f, ">")
    }
}

impl GenTypeObj {
    pub fn class(t: Type, require: Option<TypeObj>, impls: Option<TypeObj>, inited: bool) -> Self {
        GenTypeObj::Class(ClassTypeObj::new(t, require, impls, inited))
    }

    pub fn inherited(
        t: Type,
        sup: TypeObj,
        impls: Option<TypeObj>,
        additional: Option<TypeObj>,
    ) -> Self {
        GenTypeObj::Subclass(InheritedTypeObj::new(t, sup, impls, additional))
    }

    pub fn trait_(t: Type, require: TypeObj, impls: Option<TypeObj>, inited: bool) -> Self {
        GenTypeObj::Trait(TraitTypeObj::new(t, require, impls, inited))
    }

    pub fn patch(t: Type, base: TypeObj, impls: Option<TypeObj>) -> Self {
        GenTypeObj::Patch(PatchObj::new(t, base, impls))
    }

    pub fn subsumed(
        t: Type,
        sup: TypeObj,
        impls: Option<TypeObj>,
        additional: Option<TypeObj>,
    ) -> Self {
        GenTypeObj::Subtrait(SubsumedTypeObj::new(t, sup, impls, additional))
    }

    pub fn union(t: Type, lhs: TypeObj, rhs: TypeObj) -> Self {
        GenTypeObj::Union(UnionTypeObj::new(t, lhs, rhs))
    }

    pub fn intersection(t: Type, lhs: TypeObj, rhs: TypeObj) -> Self {
        GenTypeObj::Intersection(IntersectionTypeObj::new(t, lhs, rhs))
    }

    pub fn structural(t: Type, type_: TypeObj) -> Self {
        GenTypeObj::Structural(StructuralTypeObj::new(t, type_))
    }

    pub const fn is_inited(&self) -> bool {
        match self {
            Self::Class(class) => class.inited,
            Self::Trait(trait_) => trait_.inited,
            _ => true,
        }
    }

    pub fn base_or_sup(&self) -> Option<&TypeObj> {
        match self {
            Self::Class(class) => class.base.as_ref().map(AsRef::as_ref),
            Self::Subclass(subclass) => Some(subclass.sup.as_ref()),
            Self::Trait(trait_) => Some(trait_.requires.as_ref()),
            Self::Subtrait(subtrait) => Some(subtrait.sup.as_ref()),
            Self::Structural(type_) => Some(type_.base.as_ref()),
            Self::Patch(patch) => Some(patch.base.as_ref()),
            _ => None,
        }
    }

    pub fn impls(&self) -> Option<&TypeObj> {
        match self {
            Self::Class(class) => class.impls.as_ref().map(|x| x.as_ref()),
            Self::Subclass(subclass) => subclass.impls.as_ref().map(|x| x.as_ref()),
            Self::Subtrait(subtrait) => subtrait.impls.as_ref().map(|x| x.as_ref()),
            Self::Patch(patch) => patch.impls.as_ref().map(|x| x.as_ref()),
            _ => None,
        }
    }

    pub fn impls_mut(&mut self) -> Option<&mut Option<Box<TypeObj>>> {
        match self {
            Self::Class(class) => Some(&mut class.impls),
            Self::Subclass(subclass) => Some(&mut subclass.impls),
            Self::Subtrait(subtrait) => Some(&mut subtrait.impls),
            Self::Patch(patch) => Some(&mut patch.impls),
            _ => None,
        }
    }

    pub fn additional(&self) -> Option<&TypeObj> {
        match self {
            Self::Subclass(subclass) => subclass.additional.as_ref().map(|x| x.as_ref()),
            Self::Subtrait(subtrait) => subtrait.additional.as_ref().map(|x| x.as_ref()),
            _ => None,
        }
    }

    pub fn meta_type(&self) -> Type {
        match self {
            Self::Class(_) | Self::Subclass(_) => Type::ClassType,
            Self::Trait(_) | Self::Subtrait(_) => Type::TraitType,
            Self::Patch(_) => Type::Patch,
            Self::Structural(_) => Type::Type,
            _ => Type::Type,
        }
    }

    pub fn typ(&self) -> &Type {
        match self {
            Self::Class(class) => &class.t,
            Self::Subclass(subclass) => &subclass.t,
            Self::Trait(trait_) => &trait_.t,
            Self::Subtrait(subtrait) => &subtrait.t,
            Self::Structural(struct_) => &struct_.t,
            Self::Union(union_) => &union_.t,
            Self::Intersection(intersection) => &intersection.t,
            Self::Patch(patch) => &patch.t,
        }
    }

    pub fn typ_mut(&mut self) -> &mut Type {
        match self {
            Self::Class(class) => &mut class.t,
            Self::Subclass(subclass) => &mut subclass.t,
            Self::Trait(trait_) => &mut trait_.t,
            Self::Subtrait(subtrait) => &mut subtrait.t,
            Self::Structural(struct_) => &mut struct_.t,
            Self::Union(union_) => &mut union_.t,
            Self::Intersection(intersection) => &mut intersection.t,
            Self::Patch(patch) => &mut patch.t,
        }
    }

    pub fn into_typ(self) -> Type {
        match self {
            Self::Class(class) => class.t,
            Self::Subclass(subclass) => subclass.t,
            Self::Trait(trait_) => trait_.t,
            Self::Subtrait(subtrait) => subtrait.t,
            Self::Structural(struct_) => struct_.t,
            Self::Union(union_) => union_.t,
            Self::Intersection(intersection) => intersection.t,
            Self::Patch(patch) => patch.t,
        }
    }

    pub fn map_t(&mut self, f: impl FnOnce(Type) -> Type) {
        *self.typ_mut() = f(self.typ().clone());
    }

    pub fn try_map_t<E>(&mut self, f: impl FnOnce(Type) -> Result<Type, E>) -> Result<(), E> {
        *self.typ_mut() = f(self.typ().clone())?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq)]
pub enum TypeObj {
    Builtin { t: Type, meta_t: Type },
    Generated(GenTypeObj),
}

impl PartialEq for TypeObj {
    fn eq(&self, other: &Self) -> bool {
        self.typ() == other.typ()
    }
}

impl Hash for TypeObj {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.typ().hash(state);
    }
}

impl fmt::Display for TypeObj {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl LimitedDisplay for TypeObj {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> std::fmt::Result {
        match self {
            TypeObj::Builtin { t, .. } => {
                if cfg!(feature = "debug") {
                    write!(f, "<type ")?;
                    t.limited_fmt(f, limit - 1)?;
                    write!(f, ">")
                } else {
                    t.limited_fmt(f, limit - 1)
                }
            }
            TypeObj::Generated(t) => {
                if cfg!(feature = "debug") {
                    write!(f, "<user type ")?;
                    t.limited_fmt(f, limit - 1)?;
                    write!(f, ">")
                } else {
                    t.limited_fmt(f, limit - 1)
                }
            }
        }
    }
}

impl TypeObj {
    pub fn builtin_type(t: Type) -> Self {
        TypeObj::Builtin {
            t,
            meta_t: Type::Type,
        }
    }

    pub fn builtin_trait(t: Type) -> Self {
        TypeObj::Builtin {
            t,
            meta_t: Type::TraitType,
        }
    }

    pub const fn is_inited(&self) -> bool {
        match self {
            Self::Builtin { .. } => true,
            Self::Generated(gen) => gen.is_inited(),
        }
    }

    pub fn typ(&self) -> &Type {
        match self {
            TypeObj::Builtin { t, .. } => t,
            TypeObj::Generated(t) => t.typ(),
        }
    }

    pub fn typ_mut(&mut self) -> &mut Type {
        match self {
            TypeObj::Builtin { t, .. } => t,
            TypeObj::Generated(t) => t.typ_mut(),
        }
    }

    pub fn into_typ(self) -> Type {
        match self {
            TypeObj::Builtin { t, .. } => t,
            TypeObj::Generated(t) => t.into_typ(),
        }
    }

    pub fn contains_intersec(&self, other: &Type) -> bool {
        match self {
            TypeObj::Builtin { t, .. } => t.contains_intersec(other),
            TypeObj::Generated(t) => t.typ().contains_intersec(other),
        }
    }

    pub fn map_t(&mut self, f: impl FnOnce(Type) -> Type) {
        match self {
            TypeObj::Builtin { t, .. } => *t = f(t.clone()),
            TypeObj::Generated(t) => t.map_t(f),
        }
    }

    pub fn try_map_t<E>(&mut self, f: impl FnOnce(Type) -> Result<Type, E>) -> Result<(), E> {
        match self {
            TypeObj::Builtin { t, .. } => {
                *t = f(t.clone())?;
                Ok(())
            }
            TypeObj::Generated(t) => t.try_map_t(f),
        }
    }
}

/// 値オブジェクト
/// コンパイル時評価ができ、シリアライズも可能
#[derive(Clone, PartialEq, Default)]
pub enum ValueObj {
    Int(i32),
    Nat(u64),
    Float(f64),
    Str(Str),
    Bool(bool),
    Array(ArcArray<ValueObj>),
    Set(Set<ValueObj>),
    Dict(Dict<ValueObj, ValueObj>),
    Tuple(ArcArray<ValueObj>),
    Record(Dict<Field, ValueObj>),
    DataClass {
        name: Str,
        fields: Dict<Field, ValueObj>,
    },
    Code(Box<CodeObj>),
    Subr(ConstSubr),
    Type(TypeObj),
    None,
    Ellipsis,
    NotImplemented,
    NegInf,
    Inf,
    #[default]
    Illegal, // to avoid conversions with TryFrom
}

impl fmt::Debug for ValueObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(i) => {
                if cfg!(feature = "debug") {
                    write!(f, "Int({i})")
                } else {
                    write!(f, "{i}")
                }
            }
            Self::Nat(n) => {
                if cfg!(feature = "debug") {
                    write!(f, "Nat({n})")
                } else {
                    write!(f, "{n}")
                }
            }
            Self::Float(fl) => {
                // In Rust, .0 is shown omitted.
                if fl.fract() < 1e-10 {
                    write!(f, "{fl:.1}")?;
                } else {
                    write!(f, "{fl}")?;
                }
                if cfg!(feature = "debug") {
                    write!(f, "f64")?;
                }
                Ok(())
            }
            Self::Str(s) => write!(f, "\"{}\"", s.escape()),
            Self::Bool(b) => {
                if *b {
                    write!(f, "True")
                } else {
                    write!(f, "False")
                }
            }
            Self::Array(arr) => write!(f, "[{}]", fmt_iter(arr.iter())),
            Self::Dict(dict) => {
                write!(f, "{{")?;
                for (i, (k, v)) in dict.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Self::Tuple(tup) => write!(f, "({})", fmt_iter(tup.iter())),
            Self::Set(st) => write!(f, "{{{}}}", fmt_iter(st.iter())),
            Self::Code(code) => write!(f, "{code}"),
            Self::Record(rec) => {
                write!(f, "{{")?;
                for (i, (k, v)) in rec.iter().enumerate() {
                    if i != 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{k} = {v}")?;
                }
                write!(f, "}}")
            }
            Self::DataClass { name, fields } => {
                write!(f, "{name} {{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i != 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{k} = {v}")?;
                }
                write!(f, "}}")
            }
            Self::Subr(subr) => write!(f, "{subr}"),
            Self::Type(t) => write!(f, "{t}"),
            Self::None => write!(f, "None"),
            Self::Ellipsis => write!(f, "Ellipsis"),
            Self::NotImplemented => write!(f, "NotImplemented"),
            Self::NegInf => write!(f, "-Inf"),
            Self::Inf => write!(f, "Inf"),
            Self::Illegal => write!(f, "<illegal>"),
        }
    }
}

impl_display_from_debug!(ValueObj);

impl LimitedDisplay for ValueObj {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> std::fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Str(s) => {
                if limit.is_positive() && s.len() >= STR_OMIT_THRESHOLD {
                    write!(f, "\"(...)\"")
                } else {
                    write!(f, "\"{}\"", s.escape())
                }
            }
            Self::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    item.limited_fmt(f, limit - 1)?;
                }
                write!(f, "]")
            }
            Self::Dict(dict) => {
                write!(f, "{{")?;
                for (i, (k, v)) in dict.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    k.limited_fmt(f, limit - 1)?;
                    write!(f, ": ")?;
                    v.limited_fmt(f, limit - 1)?;
                }
                write!(f, "}}")
            }
            Self::Tuple(tup) => {
                write!(f, "(")?;
                for (i, item) in tup.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    item.limited_fmt(f, limit - 1)?;
                }
                write!(f, ")")
            }
            Self::Set(st) => {
                write!(f, "{{")?;
                for (i, item) in st.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    item.limited_fmt(f, limit - 1)?;
                }
                write!(f, "}}")
            }
            Self::Record(rec) => {
                write!(f, "{{")?;
                for (i, (field, v)) in rec.iter().enumerate() {
                    if i != 0 {
                        write!(f, "; ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    write!(f, "{field} = ")?;
                    v.limited_fmt(f, limit - 1)?;
                }
                if rec.is_empty() {
                    write!(f, "=")?;
                }
                write!(f, "}}")
            }
            Self::DataClass { name, fields } => {
                write!(f, "{name} {{")?;
                for (i, (field, v)) in fields.iter().enumerate() {
                    if i != 0 {
                        write!(f, "; ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    write!(f, "{field} = ")?;
                    v.limited_fmt(f, limit - 1)?;
                }
                if fields.is_empty() {
                    write!(f, "=")?;
                }
                write!(f, "}}")
            }
            Self::Type(typ) => typ.limited_fmt(f, limit),
            _ => write!(f, "{self}"),
        }
    }
}

impl Eq for ValueObj {}

impl Neg for ValueObj {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        match self {
            Self::Int(i) => Self::Int(-i),
            Self::Nat(n) => Self::Int(-(n as i32)),
            Self::Float(fl) => Self::Float(-fl),
            Self::Inf => Self::NegInf,
            Self::NegInf => Self::Inf,
            other => panic!("cannot negate {other}"),
        }
    }
}

// FIXME:
impl Hash for ValueObj {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Int(i) => i.hash(state),
            Self::Nat(n) => n.hash(state),
            // TODO:
            Self::Float(f) => f.to_bits().hash(state),
            Self::Str(s) => s.hash(state),
            Self::Bool(b) => b.hash(state),
            Self::Array(arr) => arr.hash(state),
            Self::Dict(dict) => dict.hash(state),
            Self::Tuple(tup) => tup.hash(state),
            Self::Set(st) => st.hash(state),
            Self::Code(code) => code.hash(state),
            Self::Record(rec) => rec.hash(state),
            Self::DataClass { name, fields } => {
                name.hash(state);
                fields.hash(state);
            }
            Self::Subr(subr) => subr.hash(state),
            Self::Type(t) => t.hash(state),
            Self::None => {
                "literal".hash(state);
                "None".hash(state)
            }
            Self::Ellipsis => {
                "literal".hash(state);
                "Ellipsis".hash(state)
            }
            Self::NotImplemented => {
                "literal".hash(state);
                "NotImplemented".hash(state)
            }
            Self::NegInf => {
                "literal".hash(state);
                "NegInf".hash(state)
            }
            Self::Inf => {
                "literal".hash(state);
                "Inf".hash(state)
            }
            Self::Illegal => {
                "literal".hash(state);
                "illegal".hash(state)
            }
        }
    }
}

impl From<i32> for ValueObj {
    fn from(item: i32) -> Self {
        if item >= 0 {
            ValueObj::Nat(item as u64)
        } else {
            ValueObj::Int(item)
        }
    }
}

impl From<u64> for ValueObj {
    fn from(item: u64) -> Self {
        ValueObj::Nat(item)
    }
}

impl From<usize> for ValueObj {
    fn from(item: usize) -> Self {
        ValueObj::Nat(item as u64)
    }
}

impl From<f64> for ValueObj {
    fn from(item: f64) -> Self {
        ValueObj::Float(item)
    }
}

impl From<&str> for ValueObj {
    fn from(item: &str) -> Self {
        ValueObj::Str(Str::rc(item))
    }
}

impl From<Str> for ValueObj {
    fn from(item: Str) -> Self {
        ValueObj::Str(item)
    }
}

impl From<bool> for ValueObj {
    fn from(item: bool) -> Self {
        ValueObj::Bool(item)
    }
}

impl From<CodeObj> for ValueObj {
    fn from(item: CodeObj) -> Self {
        ValueObj::Code(Box::new(item))
    }
}

impl<V: Into<ValueObj>> From<Vec<V>> for ValueObj {
    fn from(item: Vec<V>) -> Self {
        ValueObj::Array(ArcArray::from(
            &item.into_iter().map(Into::into).collect::<Vec<_>>()[..],
        ))
    }
}

impl<const N: usize, V: Into<ValueObj>> From<[V; N]> for ValueObj {
    fn from(item: [V; N]) -> Self {
        ValueObj::Array(ArcArray::from(&item.map(Into::into)[..]))
    }
}

impl TryFrom<&ValueObj> for f64 {
    type Error = ();
    fn try_from(val: &ValueObj) -> Result<f64, Self::Error> {
        match val {
            ValueObj::Int(i) => Ok(*i as f64),
            ValueObj::Nat(n) => Ok(*n as f64),
            ValueObj::Float(f) => Ok(*f),
            ValueObj::Inf => Ok(f64::INFINITY),
            ValueObj::NegInf => Ok(f64::NEG_INFINITY),
            ValueObj::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err(()),
        }
    }
}

impl TryFrom<&ValueObj> for usize {
    type Error = ();
    fn try_from(val: &ValueObj) -> Result<usize, Self::Error> {
        match val {
            ValueObj::Int(i) => usize::try_from(*i).map_err(|_| ()),
            ValueObj::Nat(n) => usize::try_from(*n).map_err(|_| ()),
            ValueObj::Float(f) => Ok(*f as usize),
            ValueObj::Bool(b) => Ok(if *b { 1 } else { 0 }),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a ValueObj> for &'a Type {
    type Error = ();
    fn try_from(val: &'a ValueObj) -> Result<Self, ()> {
        match val {
            ValueObj::Type(t) => match t {
                TypeObj::Builtin { t, .. } => Ok(t),
                TypeObj::Generated(gen) => Ok(gen.typ()),
            },
            _ => Err(()),
        }
    }
}

impl HasType for ValueObj {
    fn ref_t(&self) -> &Type {
        panic!("cannot get reference of the const")
    }
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        None
    }
    /// その要素だけの集合型を返す、クラスが欲しい場合は.classで
    #[inline]
    fn t(&self) -> Type {
        let name = FRESH_GEN.fresh_varname();
        let pred = Predicate::eq(name.clone(), TyParam::Value(self.clone()));
        refinement(name, self.class(), pred)
    }
    fn signature_t(&self) -> Option<&Type> {
        None
    }
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        None
    }
}

impl ValueObj {
    pub const fn builtin_class(t: Type) -> Self {
        ValueObj::Type(TypeObj::Builtin {
            t,
            meta_t: Type::ClassType,
        })
    }

    pub const fn builtin_trait(t: Type) -> Self {
        ValueObj::Type(TypeObj::Builtin {
            t,
            meta_t: Type::TraitType,
        })
    }

    pub fn builtin_type(t: Type) -> Self {
        ValueObj::Type(TypeObj::Builtin {
            t,
            meta_t: Type::Type,
        })
    }

    pub const fn gen_t(gen: GenTypeObj) -> Self {
        ValueObj::Type(TypeObj::Generated(gen))
    }

    pub fn range(start: Self, end: Self) -> Self {
        Self::DataClass {
            name: "Range".into(),
            fields: dict! {
                Field::private("start".into()) => start,
                Field::private("end".into()) => end,
                Field::private("step".into()) => Self::None,
            },
        }
    }

    // TODO: add Complex
    pub const fn is_num(&self) -> bool {
        matches!(
            self,
            Self::Float(_) | Self::Int(_) | Self::Nat(_) | Self::Bool(_)
        )
    }

    pub const fn is_float(&self) -> bool {
        matches!(
            self,
            Self::Float(_) | Self::Int(_) | Self::Nat(_) | Self::Bool(_)
        )
    }

    pub const fn is_int(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Nat(_) | Self::Bool(_))
    }

    pub const fn is_nat(&self) -> bool {
        matches!(self, Self::Nat(_) | Self::Bool(_))
    }

    pub const fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    pub const fn is_str(&self) -> bool {
        matches!(self, Self::Str(_))
    }

    pub const fn is_type(&self) -> bool {
        matches!(self, Self::Type(_))
    }

    pub const fn is_inited(&self) -> bool {
        match self {
            Self::Type(t) => t.is_inited(),
            _ => true,
        }
    }

    pub fn from_str(t: Type, mut content: Str) -> Option<Self> {
        match t {
            Type::Int => content.replace('_', "").parse::<i32>().ok().map(Self::Int),
            Type::Nat => {
                let content = content
                    .trim_start_matches('-') // -0 -> 0
                    .replace('_', "");
                if content.len() <= 1 {
                    return content.parse::<u64>().ok().map(Self::Nat);
                }
                match &content[0..=1] {
                    pre @ ("0b" | "0B") => {
                        let content = content.trim_start_matches(pre);
                        u64::from_str_radix(content, 2).ok().map(Self::Nat)
                    }
                    pre @ ("0o" | "0O") => {
                        let content = content.trim_start_matches(pre);
                        u64::from_str_radix(content, 8).ok().map(Self::Nat)
                    }
                    pre @ ("0x" | "0X") => {
                        let content = content.trim_start_matches(pre);
                        u64::from_str_radix(content, 16).ok().map(Self::Nat)
                    }
                    _ => content.parse::<u64>().ok().map(Self::Nat),
                }
            }
            Type::Float => content
                .replace('_', "")
                .parse::<f64>()
                .ok()
                .map(Self::Float),
            // TODO:
            Type::Ratio => content
                .replace('_', "")
                .parse::<f64>()
                .ok()
                .map(Self::Float),
            Type::Str => {
                if &content[..] == "\"\"" {
                    Some(Self::Str(Str::from("")))
                } else {
                    if content.get(..3) == Some("\"\"\"") {
                        content = Str::rc(&content[3..]);
                    } else if content.get(..1) == Some("\"") {
                        content = Str::rc(&content[1..]);
                    }
                    if content.len() >= 3 && content.get(content.len() - 3..) == Some("\"\"\"") {
                        content = Str::rc(&content[..content.len() - 3]);
                    } else if content.len() >= 1 && content.get(content.len() - 1..) == Some("\"") {
                        content = Str::rc(&content[..content.len() - 1]);
                    }
                    Some(Self::Str(content))
                }
            }
            Type::Bool => Some(Self::Bool(&content[..] == "True")),
            Type::NoneType => Some(Self::None),
            Type::Ellipsis => Some(Self::Ellipsis),
            Type::NotImplementedType => Some(Self::NotImplemented),
            Type::Inf => Some(Self::Inf),
            Type::NegInf => Some(Self::NegInf),
            _ => {
                log!(err "{t} {content}");
                None
            }
        }
    }

    pub fn into_bytes(self, python_ver: PythonVersion) -> Vec<u8> {
        match self {
            Self::Int(i) => [vec![DataTypePrefix::Int32 as u8], i.to_le_bytes().to_vec()].concat(),
            // TODO: Natとしてシリアライズ
            Self::Nat(n) => [
                vec![DataTypePrefix::Int32 as u8],
                (n as i32).to_le_bytes().to_vec(),
            ]
            .concat(),
            Self::Float(f) => [
                vec![DataTypePrefix::BinFloat as u8],
                f.to_le_bytes().to_vec(),
            ]
            .concat(),
            Self::Str(s) => str_into_bytes(s, false),
            Self::Bool(true) => vec![DataTypePrefix::True as u8],
            Self::Bool(false) => vec![DataTypePrefix::False as u8],
            // TODO: SmallTuple
            Self::Array(arr) => {
                let mut bytes = Vec::with_capacity(arr.len());
                bytes.push(DataTypePrefix::Tuple as u8);
                bytes.append(&mut (arr.len() as u32).to_le_bytes().to_vec());
                for obj in arr.iter().cloned() {
                    bytes.append(&mut obj.into_bytes(python_ver));
                }
                bytes
            }
            Self::Tuple(tup) => {
                let mut bytes = Vec::with_capacity(tup.len());
                bytes.push(DataTypePrefix::Tuple as u8);
                bytes.append(&mut (tup.len() as u32).to_le_bytes().to_vec());
                for obj in tup.iter().cloned() {
                    bytes.append(&mut obj.into_bytes(python_ver));
                }
                bytes
            }
            Self::None => {
                vec![DataTypePrefix::None as u8]
            }
            Self::Code(c) => c.into_bytes(python_ver),
            // Dict
            other => {
                panic!(
                    "{}",
                    switch_lang!(
                        "japanese" => format!("このオブジェクトはシリアライズできません: {other}"),
                        "simplified_chinese" => format!("此对象无法序列化: {other}"),
                        "traditional_chinese" => format!("此對象無法序列化: {other}"),
                        "english" => format!("this object cannot be serialized: {other}"),
                    )
                )
            }
        }
    }

    pub fn from_const_expr(expr: ConstExpr) -> Self {
        let ConstExpr::Lit(lit) = expr else { todo!() };
        let t = type_from_token_kind(lit.token.kind);
        ValueObj::from_str(t, lit.token.content).unwrap()
    }

    pub fn tuple_from_const_args(args: ConstArgs) -> Self {
        Self::Tuple(Arc::from(&Self::vec_from_const_args(args)[..]))
    }

    pub fn vec_from_const_args(args: ConstArgs) -> Vec<Self> {
        args.deconstruct()
            .0
            .into_iter()
            .map(|elem| Self::from_const_expr(elem.expr))
            .collect::<Vec<_>>()
    }

    pub fn class(&self) -> Type {
        match self {
            Self::Int(_) => Type::Int,
            Self::Nat(_) => Type::Nat,
            Self::Float(_) => Type::Float,
            Self::Str(_) => Type::Str,
            Self::Bool(_) => Type::Bool,
            Self::Array(arr) => array_t(
                // REVIEW: Never?
                arr.iter()
                    .next()
                    .map(|elem| elem.class())
                    .unwrap_or(Type::Never),
                TyParam::value(arr.len()),
            ),
            Self::Dict(dict) => {
                let tp = dict
                    .iter()
                    .map(|(k, v)| (TyParam::value(k.clone()), TyParam::value(v.clone())));
                dict_t(TyParam::Dict(tp.collect()))
            }
            Self::Tuple(tup) => tuple_t(tup.iter().map(|v| v.class()).collect()),
            Self::Set(st) => set_t(inner_class(st), TyParam::value(st.len())),
            Self::Code(_) => Type::Code,
            Self::Record(rec) => {
                Type::Record(rec.iter().map(|(k, v)| (k.clone(), v.class())).collect())
            }
            Self::DataClass { name, .. } => Type::Mono(name.clone()),
            Self::Subr(subr) => subr.sig_t().clone(),
            Self::Type(t_obj) => match t_obj {
                TypeObj::Builtin { meta_t, .. } => meta_t.clone(),
                TypeObj::Generated(gen_t) => gen_t.meta_type(),
            },
            Self::None => Type::NoneType,
            Self::Ellipsis => Type::Ellipsis,
            Self::NotImplemented => Type::NotImplementedType,
            Self::Inf => Type::Inf,
            Self::NegInf => Type::NegInf,
            Self::Illegal => Type::Failure,
        }
    }

    pub fn try_binary(self, other: Self, op: OpKind) -> Option<Self> {
        match op {
            OpKind::Add => self.try_add(other),
            OpKind::Sub => self.try_sub(other),
            OpKind::Mul => self.try_mul(other),
            OpKind::Div => self.try_div(other),
            OpKind::Lt => self.try_lt(other),
            OpKind::Gt => self.try_gt(other),
            OpKind::Le => self.try_le(other),
            OpKind::Ge => self.try_ge(other),
            OpKind::Eq => self.try_eq(other),
            OpKind::Ne => self.try_ne(other),
            _ => None,
        }
    }

    pub fn try_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            return Some(Ordering::Equal);
        }
        match (self, other) {
            (l, r) if l.is_num() && r.is_num() => {
                f64::try_from(l).ok()?.partial_cmp(&f64::try_from(r).ok()?)
            }
            (Self::Inf, n) | (n, Self::NegInf) if n.is_num() => Some(Ordering::Greater),
            (n, Self::Inf) | (Self::NegInf, n) if n.is_num() => Some(Ordering::Less),
            (Self::NegInf, Self::Inf) => Some(Ordering::Less),
            (Self::Inf, Self::NegInf) => Some(Ordering::Greater),
            // REVIEW: 等しいとみなしてよいのか?
            (Self::Inf, Self::Inf) | (Self::NegInf, Self::NegInf) => Some(Ordering::Equal),
            /* (Self::PlusEpsilon(l), r) => l.try_cmp(r)
                .map(|o| if matches!(o, Ordering::Equal) { Ordering::Less } else { o }),
            (l, Self::PlusEpsilon(r)) => l.try_cmp(r)
                .map(|o| if matches!(o, Ordering::Equal) { Ordering::Greater } else { o }),
            */
            (_s, _o) => {
                if let Some(ValueObj::Bool(b)) = _s.clone().try_eq(_o.clone()) {
                    if b {
                        Some(Ordering::Equal)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    // REVIEW: allow_divergenceオプションを付けるべきか?
    pub fn try_add(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l + r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l + r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l + r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l + r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 + r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float(l - r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::Float(l as f64 - r)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::Float(l as f64 - r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::Float(l - r as f64)),
            (Self::Str(l), Self::Str(r)) => Some(Self::Str(Str::from(format!("{l}{r}")))),
            (Self::Array(l), Self::Array(r)) => {
                let arr = Arc::from([l, r].concat());
                Some(Self::Array(arr))
            }
            (Self::Dict(l), Self::Dict(r)) => Some(Self::Dict(l.concat(r))),
            (inf @ (Self::Inf | Self::NegInf), _) | (_, inf @ (Self::Inf | Self::NegInf)) => {
                Some(inf)
            }
            _ => None,
        }
    }

    pub fn try_sub(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l - r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Int(l as i32 - r as i32)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l - r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l - r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 - r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l - r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 - r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l - r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 - r)),
            (inf @ (Self::Inf | Self::NegInf), other)
            | (other, inf @ (Self::Inf | Self::NegInf))
                if other != Self::Inf && other != Self::NegInf =>
            {
                Some(inf)
            }
            _ => None,
        }
    }

    pub fn try_mul(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l * r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l * r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l * r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Int(l * r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 * r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l * r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 * r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l * r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 * r)),
            (Self::Str(l), Self::Nat(r)) => Some(Self::Str(Str::from(l.repeat(r as usize)))),
            (inf @ (Self::Inf | Self::NegInf), _) | (_, inf @ (Self::Inf | Self::NegInf)) => {
                Some(inf)
            }
            _ => None,
        }
    }

    pub fn try_div(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Float(l as f64 / r as f64)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Float(l as f64 / r as f64)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l / r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Float(l as f64 / r as f64)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Float(l as f64 / r as f64)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float(l / r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 / r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l / r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 / r)),
            // TODO: x/±Inf = 0
            _ => None,
        }
    }

    pub fn try_floordiv(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l / r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l / r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float((l / r).floor())),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Int(l / r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 / r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float((l / r as f64).floor())),
            (Self::Nat(l), Self::Float(r)) => Some(Self::Float((l as f64 / r).floor())),
            (Self::Float(l), Self::Int(r)) => Some(Self::Float((l / r as f64).floor())),
            (Self::Int(l), Self::Float(r)) => Some(Self::Float((l as f64 / r).floor())),
            // TODO: x//±Inf = 0
            _ => None,
        }
    }

    pub fn try_gt(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l > r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l > r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l > r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l > r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 > r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l > r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 > r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l > r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 > r)),
            _ => None,
        }
    }

    pub fn try_ge(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l >= r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l >= r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l >= r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l >= r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 >= r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l >= r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 >= r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l >= r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 >= r)),
            _ => None,
        }
    }

    pub fn try_lt(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l < r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l < r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l < r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l < r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from((l as i32) < r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l < r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from((l as f64) < r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l < r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from((l as f64) < r)),
            _ => None,
        }
    }

    pub fn try_le(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l <= r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l <= r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l <= r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l <= r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from((l as i32) <= r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l <= r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from((l as f64) <= r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l <= r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from((l as f64) <= r)),
            _ => None,
        }
    }

    pub fn try_eq(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l == r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l == r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l == r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l == r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 == r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l == r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 == r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l == r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 == r)),
            (Self::Str(l), Self::Str(r)) => Some(Self::from(l == r)),
            (Self::Bool(l), Self::Bool(r)) => Some(Self::from(l == r)),
            (Self::Type(l), Self::Type(r)) => Some(Self::from(l == r)),
            // TODO:
            _ => None,
        }
    }

    pub fn try_ne(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l != r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l != r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l != r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l != r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 != r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l != r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 != r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l != r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 != r)),
            (Self::Str(l), Self::Str(r)) => Some(Self::from(l != r)),
            (Self::Bool(l), Self::Bool(r)) => Some(Self::from(l != r)),
            (Self::Type(l), Self::Type(r)) => Some(Self::from(l != r)),
            _ => None,
        }
    }

    pub fn try_or(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Some(Self::from(l || r)),
            _ => None,
        }
    }

    pub fn try_get_attr(&self, attr: &Field) -> Option<Self> {
        match self {
            Self::Type(typ) => match typ {
                TypeObj::Builtin { t: builtin, .. } => {
                    log!(err "TODO: {builtin}{attr}");
                    None
                }
                TypeObj::Generated(gen) => match gen.typ() {
                    Type::Record(rec) => {
                        let t = rec.get(attr)?;
                        Some(ValueObj::builtin_type(t.clone()))
                    }
                    _ => None,
                },
            },
            Self::Record(rec) => {
                let v = rec.get(attr)?;
                Some(v.clone())
            }
            _ => None,
        }
    }

    pub fn as_type(&self, ctx: &Context) -> Option<TypeObj> {
        match self {
            Self::Type(t) => Some(t.clone()),
            Self::Record(rec) => {
                let mut attr_ts = dict! {};
                for (k, v) in rec.iter() {
                    attr_ts.insert(k.clone(), v.as_type(ctx)?.typ().clone());
                }
                Some(TypeObj::builtin_type(Type::Record(attr_ts)))
            }
            Self::Subr(subr) => subr.as_type(ctx).map(TypeObj::builtin_type),
            Self::Array(elems) | Self::Tuple(elems) => {
                log!(err "as_type({})", erg_common::fmt_vec(elems));
                None
            }
            Self::Dict(elems) => {
                log!(err "as_type({elems})");
                None
            }
            _other => None,
        }
    }
}

pub mod value_set {
    use crate::ty::{Type, ValueObj};
    use erg_common::set::Set;

    // false -> SyntaxError
    pub fn is_homogeneous(set: &Set<ValueObj>) -> bool {
        if let Some(first) = set.iter().next() {
            let l_first = first.class();
            // `Set` iteration order is guaranteed (if not changed)
            set.iter().skip(1).all(|c| c.class() == l_first)
        } else {
            true
        }
    }

    pub fn inner_class(set: &Set<ValueObj>) -> Type {
        set.iter()
            .next()
            .map(|elem| elem.class())
            .unwrap_or(Type::Never)
    }

    pub fn max(set: &Set<ValueObj>) -> Option<ValueObj> {
        if !is_homogeneous(set) {
            return None;
        }
        set.iter()
            .max_by(|x, y| x.try_cmp(y).unwrap())
            .map(Clone::clone)
    }

    pub fn min(set: &Set<ValueObj>) -> Option<ValueObj> {
        if !is_homogeneous(set) {
            return None;
        }
        set.iter()
            .min_by(|x, y| x.try_cmp(y).unwrap())
            .map(Clone::clone)
    }
}
