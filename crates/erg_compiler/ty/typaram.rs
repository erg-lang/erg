use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Range, RangeInclusive, Sub};

use erg_common::consts::DEBUG_MODE;
use erg_common::dict::Dict;
use erg_common::set::Set;
use erg_common::traits::{LimitedDisplay, StructuralEq};
use erg_common::{dict, ref_addr_eq, set, Str};

use erg_parser::ast::ConstLambda;
use erg_parser::token::TokenKind;

use crate::context::eval::UndoableLinkedList;

use super::constructors::int_interval;
use super::free::{
    CanbeFree, Constraint, FreeKind, FreeTyParam, FreeTyVar, HasLevel, Level, GENERIC_LEVEL,
};
use super::value::ValueObj;
use super::{Field, ParamTy, ReplaceTable};
use super::{Type, CONTAINER_OMIT_THRESHOLD};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OpKind {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Pow,
    Mod,
    Pos,
    Neg,
    Invert,
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    Ne,
    As,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    ClosedRange,
    LeftOpenRange,
    RightOpenRange,
    OpenRange,
}

impl fmt::Display for OpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::FloorDiv => write!(f, "//"),
            Self::Pow => write!(f, "**"),
            Self::Mod => write!(f, "%"),
            Self::Pos => write!(f, "+"),
            Self::Neg => write!(f, "-"),
            Self::Invert => write!(f, "~"),
            Self::Gt => write!(f, ">"),
            Self::Lt => write!(f, "<"),
            Self::Ge => write!(f, ">="),
            Self::Le => write!(f, "<="),
            Self::Eq => write!(f, "=="),
            Self::Ne => write!(f, "!="),
            Self::As => write!(f, "as"),
            Self::And => write!(f, "and"),
            Self::Or => write!(f, "or"),
            Self::Not => write!(f, "not"),
            Self::BitAnd => write!(f, "&&"),
            Self::BitOr => write!(f, "||"),
            Self::BitXor => write!(f, "^^"),
            Self::Shl => write!(f, "<<"),
            Self::Shr => write!(f, ">>"),
            Self::ClosedRange => write!(f, ".."),
            Self::LeftOpenRange => write!(f, "<.."),
            Self::RightOpenRange => write!(f, "..<"),
            Self::OpenRange => write!(f, "<..<"),
        }
    }
}

impl TryFrom<TokenKind> for OpKind {
    type Error = ();
    fn try_from(tk: TokenKind) -> Result<Self, Self::Error> {
        match tk {
            TokenKind::Plus => Ok(Self::Add),
            TokenKind::Minus => Ok(Self::Sub),
            TokenKind::Star => Ok(Self::Mul),
            TokenKind::Slash => Ok(Self::Div),
            TokenKind::FloorDiv => Ok(Self::FloorDiv),
            TokenKind::Pow => Ok(Self::Pow),
            TokenKind::Mod => Ok(Self::Mod),
            TokenKind::PreBitNot => Ok(Self::Invert),
            TokenKind::Gre => Ok(Self::Gt),
            TokenKind::Less => Ok(Self::Lt),
            TokenKind::GreEq => Ok(Self::Ge),
            TokenKind::LessEq => Ok(Self::Le),
            TokenKind::DblEq => Ok(Self::Eq),
            TokenKind::NotEq => Ok(Self::Ne),
            TokenKind::As => Ok(Self::As),
            TokenKind::AndOp => Ok(Self::And),
            TokenKind::OrOp => Ok(Self::Or),
            TokenKind::BitAnd => Ok(Self::BitAnd),
            TokenKind::BitOr => Ok(Self::BitOr),
            TokenKind::BitXor => Ok(Self::BitXor),
            TokenKind::Shl => Ok(Self::Shl),
            TokenKind::Shr => Ok(Self::Shr),
            TokenKind::Closed => Ok(Self::ClosedRange),
            TokenKind::LeftOpen => Ok(Self::LeftOpenRange),
            TokenKind::RightOpen => Ok(Self::RightOpenRange),
            TokenKind::Open => Ok(Self::OpenRange),
            _ => Err(()),
        }
    }
}

impl OpKind {
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            Self::Gt | Self::Lt | Self::Ge | Self::Le | Self::Eq | Self::Ne
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalOp {
    /// ..
    Closed,
    /// <..
    LeftOpen,
    /// ..<
    RightOpen,
    /// <..<
    Open,
}

impl IntervalOp {
    pub const fn is_closed(&self) -> bool {
        matches!(self, Self::Closed)
    }
    pub const fn is_left_open(&self) -> bool {
        matches!(self, Self::LeftOpen | Self::Open)
    }
    pub const fn is_right_open(&self) -> bool {
        matches!(self, Self::RightOpen | Self::Open)
    }
    pub const fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }
}

impl fmt::Display for IntervalOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => write!(f, ".."),
            Self::LeftOpen => write!(f, "<.."),
            Self::RightOpen => write!(f, "..<"),
            Self::Open => write!(f, "<..<"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyParamLambda {
    pub const_: ConstLambda,
    pub nd_params: Vec<ParamTy>,
    pub var_params: Option<ParamTy>,
    pub d_params: Vec<ParamTy>,
    pub kw_var_params: Option<ParamTy>,
    pub body: Vec<TyParam>,
}

impl fmt::Display for TyParamLambda {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.const_)
    }
}

impl HasLevel for TyParamLambda {
    fn level(&self) -> Option<usize> {
        self.body.iter().filter_map(|tp| tp.level()).min()
    }
    fn set_level(&self, lev: Level) {
        for tp in self.body.iter() {
            tp.set_level(lev);
        }
    }
}

impl StructuralEq for TyParamLambda {
    fn structural_eq(&self, other: &Self) -> bool {
        self.body.len() == other.body.len()
            && self
                .body
                .iter()
                .zip(other.body.iter())
                .all(|(a, b)| a.structural_eq(b))
    }
}

impl TyParamLambda {
    pub const fn new(
        lambda: ConstLambda,
        nd_params: Vec<ParamTy>,
        var_params: Option<ParamTy>,
        d_params: Vec<ParamTy>,
        kw_var_params: Option<ParamTy>,
        body: Vec<TyParam>,
    ) -> Self {
        Self {
            const_: lambda,
            nd_params,
            var_params,
            d_params,
            kw_var_params,
            body,
        }
    }
}

/// # type parameter
/// Unevaluated expressions that types can have inside
///
/// The evaluated one becomes `ValueObj`.
/// * Literal: 1, "aa", True, None, ... (don't use container literals, they can only hold literals)
/// * Type: Int, Add(?R, ?O), ...
/// * Mono: I, N, ...
/// * Attr: math.PI, ...
/// * List: `[1, 2, N]`
/// * Tuple: (1, N, True)
/// * App: List(Int), Fib(10), ...
/// * QuantVar: N: Nat, ...
/// * FreeVar: ?I: Int, ...
/// * UnaryOp: -N, ~B, ...
/// * BinOp: 1 + 1, N * 2, ...
/// * Erased: _: Type, _: Nat, ...
#[derive(Debug, Clone, Hash)]
pub enum TyParam {
    Value(ValueObj),
    Type(Box<Type>),
    List(Vec<TyParam>),
    UnsizedList(Box<TyParam>),
    Tuple(Vec<TyParam>),
    Set(Set<TyParam>),
    Dict(Dict<TyParam, TyParam>),
    Record(Dict<Field, TyParam>),
    DataClass {
        name: Str,
        fields: Dict<Field, TyParam>,
    },
    Lambda(TyParamLambda),
    Mono(Str),
    Proj {
        obj: Box<TyParam>,
        attr: Str,
    },
    ProjCall {
        obj: Box<TyParam>,
        attr: Str,
        args: Vec<TyParam>,
    },
    App {
        name: Str,
        args: Vec<TyParam>,
    },
    UnaryOp {
        op: OpKind,
        val: Box<TyParam>,
    },
    BinOp {
        op: OpKind,
        lhs: Box<TyParam>,
        rhs: Box<TyParam>,
    },
    Erased(Box<Type>),
    FreeVar(FreeTyParam),
    Failure,
}

impl PartialEq for TyParam {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Value(l), Self::Value(r)) => l == r,
            (Self::Type(l), Self::Type(r)) => l == r,
            (Self::List(l), Self::List(r)) => l == r,
            (Self::UnsizedList(l), Self::UnsizedList(r)) => l == r,
            (Self::Tuple(l), Self::Tuple(r)) => l == r,
            (Self::Dict(l), Self::Dict(r)) => l.linear_eq(r),
            (Self::Record(l), Self::Record(r)) => l == r,
            (
                Self::DataClass {
                    name: ln,
                    fields: lfs,
                },
                Self::DataClass {
                    name: rn,
                    fields: rfs,
                },
            ) => ln == rn && lfs == rfs,
            (Self::Set(l), Self::Set(r)) => l.linear_eq(r),
            (Self::Lambda(l), Self::Lambda(r)) => l == r,
            (Self::Mono(l), Self::Mono(r)) => l == r,
            (
                Self::Proj { obj, attr },
                Self::Proj {
                    obj: r_obj,
                    attr: r_attr,
                },
            ) => obj == r_obj && attr == r_attr,
            (
                Self::ProjCall { obj, attr, args },
                Self::ProjCall {
                    obj: r_obj,
                    attr: r_attr,
                    args: r_args,
                },
            ) => obj == r_obj && attr == r_attr && args == r_args,
            (
                Self::App {
                    name: ln,
                    args: lps,
                },
                Self::App {
                    name: rn,
                    args: rps,
                },
            ) => ln == rn && lps == rps,
            (
                Self::UnaryOp { op, val },
                Self::UnaryOp {
                    op: r_op,
                    val: r_val,
                },
            ) => op == r_op && val == r_val,
            (
                Self::BinOp { op, lhs, rhs },
                Self::BinOp {
                    op: r_op,
                    lhs: r_lhs,
                    rhs: r_rhs,
                },
            ) => op == r_op && lhs == r_lhs && rhs == r_rhs,
            (Self::Erased(l), Self::Erased(r)) => l == r,
            (Self::FreeVar(l), Self::FreeVar(r)) => l == r,
            (Self::FreeVar(fv), other) => match &*fv.borrow() {
                FreeKind::Linked(t) => t == other,
                _ => false,
            },
            (self_, Self::FreeVar(fv)) => match &*fv.borrow() {
                FreeKind::Linked(t) => t == self_,
                _ => false,
            },
            (Self::Failure, Self::Failure) => true,
            (Self::Type(l), Self::Value(ValueObj::Type(r))) => l.as_ref() == r.typ(),
            (Self::Value(ValueObj::Type(l)), Self::Type(r)) => l.typ() == r.as_ref(),
            _ => false,
        }
    }
}

impl Eq for TyParam {}

impl fmt::Display for TyParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl LimitedDisplay for TyParam {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Value(v) => v.limited_fmt(f, limit),
            Self::Failure => write!(f, "<Failure>"),
            Self::Type(t) => t.limited_fmt(f, limit),
            Self::FreeVar(fv) => fv.limited_fmt(f, limit),
            Self::UnaryOp { op, val } => {
                write!(f, "{op}")?;
                val.limited_fmt(f, limit - 1)
            }
            Self::BinOp { op, lhs, rhs } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " {op} ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::App { name, args } => {
                write!(f, "{name}")?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    arg.limited_fmt(f, limit - 1)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            Self::Erased(t) => {
                write!(f, "_: ")?;
                t.limited_fmt(f, limit - 1)
            }
            Self::Mono(name) => write!(f, "{name}"),
            Self::Proj { obj, attr } => {
                obj.limited_fmt(f, limit - 1)?;
                write!(f, ".")?;
                write!(f, "{attr}")
            }
            Self::ProjCall { obj, attr, args } => {
                write!(f, "(")?;
                obj.limited_fmt(f, limit - 1)?;
                write!(f, ").")?;
                write!(f, "{attr}")?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    arg.limited_fmt(f, limit - 1)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            Self::List(lis) => {
                write!(f, "[")?;
                for (i, t) in lis.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    t.limited_fmt(f, limit - 1)?;
                }
                write!(f, "]")
            }
            Self::UnsizedList(elem) => {
                write!(f, "[")?;
                elem.limited_fmt(f, limit - 1)?;
                write!(f, "; _]")
            }
            Self::Set(st) => {
                write!(f, "{{")?;
                for (i, t) in st.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    t.limited_fmt(f, limit - 1)?;
                }
                write!(f, "}}")
            }
            Self::Dict(dict) => {
                write!(f, "{{")?;
                for (i, (k, v)) in dict.iter().enumerate() {
                    if i > 0 {
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
            Self::Record(rec) => {
                write!(f, "{{")?;
                for (i, (field, v)) in rec.iter().enumerate() {
                    if i > 0 {
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
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    write!(f, "{field} = ")?;
                    v.limited_fmt(f, limit - 1)?;
                }
                write!(f, "}}")?;
                Ok(())
            }
            Self::Lambda(lambda) => write!(f, "{lambda}"),
            Self::Tuple(tuple) => {
                write!(f, "(")?;
                for (i, t) in tuple.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if limit.is_positive() && i >= CONTAINER_OMIT_THRESHOLD {
                        write!(f, "...")?;
                        break;
                    }
                    t.limited_fmt(f, limit - 1)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl CanbeFree for TyParam {
    fn unbound_name(&self) -> Option<Str> {
        match self {
            TyParam::FreeVar(fv) => fv.unbound_name(),
            TyParam::Type(t) => t.unbound_name(),
            TyParam::Value(ValueObj::Type(ty)) => ty.typ().unbound_name(),
            _ => None,
        }
    }

    fn constraint(&self) -> Option<Constraint> {
        match self {
            TyParam::FreeVar(fv) => fv.constraint(),
            TyParam::Type(t) => t.constraint(),
            TyParam::Value(ValueObj::Type(ty)) => ty.typ().constraint(),
            _ => None,
        }
    }

    fn destructive_update_constraint(&self, new_constraint: Constraint, in_instantiation: bool) {
        match self {
            Self::FreeVar(fv) => {
                fv.update_constraint(new_constraint, in_instantiation);
            }
            Self::Type(t) => {
                t.destructive_update_constraint(new_constraint, in_instantiation);
            }
            Self::Value(ValueObj::Type(ty)) => {
                ty.typ()
                    .destructive_update_constraint(new_constraint, in_instantiation);
            }
            _ => {}
        }
    }
}

impl Default for TyParam {
    #[inline]
    fn default() -> Self {
        Self::Failure
    }
}

impl Add for TyParam {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::bin(OpKind::Add, self, rhs)
    }
}

impl Sub for TyParam {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::bin(OpKind::Sub, self, rhs)
    }
}

impl Mul for TyParam {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::bin(OpKind::Mul, self, rhs)
    }
}

impl Div for TyParam {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::bin(OpKind::Div, self, rhs)
    }
}

impl Neg for TyParam {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::unary(OpKind::Neg, self)
    }
}

impl From<Range<TyParam>> for TyParam {
    fn from(r: Range<TyParam>) -> Self {
        Self::t(int_interval(IntervalOp::RightOpen, r.start, r.end))
    }
}

impl From<Range<&TyParam>> for TyParam {
    fn from(r: Range<&TyParam>) -> Self {
        Self::t(int_interval(
            IntervalOp::RightOpen,
            r.start.clone(),
            r.end.clone(),
        ))
    }
}

impl From<RangeInclusive<TyParam>> for TyParam {
    fn from(r: RangeInclusive<TyParam>) -> Self {
        let (start, end) = r.into_inner();
        Self::t(int_interval(IntervalOp::Closed, start, end))
    }
}

impl From<RangeInclusive<&TyParam>> for TyParam {
    fn from(r: RangeInclusive<&TyParam>) -> Self {
        let (start, end) = r.into_inner();
        Self::t(int_interval(IntervalOp::Closed, start.clone(), end.clone()))
    }
}

impl<V: Into<ValueObj>> From<V> for TyParam {
    fn from(v: V) -> Self {
        Self::Value(v.into())
    }
}

impl From<Dict<Type, Type>> for TyParam {
    fn from(v: Dict<Type, Type>) -> Self {
        Self::Dict(
            v.into_iter()
                .map(|(k, v)| (TyParam::t(k), TyParam::t(v)))
                .collect(),
        )
    }
}

impl<'t> TryFrom<&'t TyParam> for &'t FreeTyParam {
    type Error = ();
    fn try_from(t: &'t TyParam) -> Result<&'t FreeTyParam, ()> {
        match t {
            TyParam::FreeVar(fv) => Ok(fv),
            _ => Err(()),
        }
    }
}

impl<'t> TryFrom<&'t TyParam> for &'t FreeTyVar {
    type Error = ();
    fn try_from(t: &'t TyParam) -> Result<&'t FreeTyVar, ()> {
        match t {
            TyParam::Type(ty) => <&FreeTyVar>::try_from(ty.as_ref()),
            TyParam::Value(ValueObj::Type(ty)) => <&FreeTyVar>::try_from(ty.typ()),
            _ => Err(()),
        }
    }
}

impl TryFrom<TyParam> for Dict<TyParam, TyParam> {
    type Error = ();
    fn try_from(tp: TyParam) -> Result<Self, ()> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => Dict::try_from(fv.crack().clone()),
            TyParam::Dict(tps) => Ok(tps),
            _ => Err(()),
        }
    }
}

impl TryFrom<TyParam> for Vec<TyParam> {
    type Error = ();
    fn try_from(tp: TyParam) -> Result<Self, ()> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => Vec::try_from(fv.crack().clone()),
            TyParam::List(tps) => Ok(tps),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a TyParam> for &'a Type {
    type Error = ();
    fn try_from(tp: &'a TyParam) -> Result<&'a Type, ()> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                <&'a Type>::try_from(fv.forced_as_ref().linked().unwrap())
            }
            TyParam::Type(t) => Ok(t.as_ref()),
            TyParam::Value(v) => <&Type>::try_from(v),
            // TODO: List, Dict, Set
            _ => Err(()),
        }
    }
}

impl TryFrom<&TyParam> for usize {
    type Error = ();
    fn try_from(tp: &TyParam) -> Result<Self, ()> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => usize::try_from(&*fv.crack()),
            TyParam::Value(v) => usize::try_from(v),
            _ => Err(()),
        }
    }
}

impl HasLevel for TyParam {
    fn level(&self) -> Option<Level> {
        match self {
            Self::Type(t) => t.level(),
            Self::FreeVar(fv) => fv.level(),
            Self::List(tps) | Self::Tuple(tps) => tps.iter().filter_map(|tp| tp.level()).min(),
            Self::UnsizedList(tp) => tp.level(),
            Self::Dict(tps) => tps
                .iter()
                .map(|(k, v)| {
                    k.level()
                        .unwrap_or(GENERIC_LEVEL)
                        .min(v.level().unwrap_or(GENERIC_LEVEL))
                })
                .min(),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => rec
                .iter()
                .map(|(_, v)| v.level().unwrap_or(GENERIC_LEVEL))
                .min(),
            Self::Lambda(lambda) => lambda.level(),
            Self::Set(tps) => tps.iter().filter_map(|tp| tp.level()).min(),
            Self::Proj { obj, .. } => obj.level(),
            Self::ProjCall { obj, args, .. } => obj.level().and_then(|l| {
                args.iter()
                    .filter_map(|tp| tp.level())
                    .min()
                    .map(|r| l.min(r))
            }),
            Self::App { args, .. } => args.iter().filter_map(|tp| tp.level()).min(),
            Self::UnaryOp { val, .. } => val.level(),
            Self::BinOp { lhs, rhs, .. } => lhs.level().and_then(|l| rhs.level().map(|r| l.min(r))),
            Self::Value(val) => val.level(),
            Self::Erased(ty) => ty.level(),
            Self::Mono(_) | Self::Failure => None,
        }
    }

    fn set_level(&self, level: Level) {
        match self {
            Self::Type(t) => t.set_level(level),
            Self::FreeVar(fv) => fv.set_level(level),
            Self::Dict(tps) => {
                for (k, v) in tps.iter() {
                    k.set_level(level);
                    v.set_level(level);
                }
            }
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                for (_, v) in rec.iter() {
                    v.set_level(level);
                }
            }
            Self::List(tps) => {
                for tp in tps {
                    tp.set_level(level);
                }
            }
            Self::UnsizedList(tp) => tp.set_level(level),
            Self::Tuple(tps) => {
                for tp in tps {
                    tp.set_level(level);
                }
            }
            Self::Set(tps) => {
                for tp in tps.iter() {
                    tp.set_level(level);
                }
            }
            Self::Lambda(lambda) => lambda.set_level(level),
            Self::UnaryOp { val, .. } => val.set_level(level),
            Self::BinOp { lhs, rhs, .. } => {
                lhs.set_level(level);
                rhs.set_level(level);
            }
            Self::App { args, .. } => {
                for arg in args.iter() {
                    arg.set_level(level);
                }
            }
            Self::Proj { obj, .. } => {
                obj.set_level(level);
            }
            Self::ProjCall { obj, args, .. } => {
                obj.set_level(level);
                for arg in args.iter() {
                    arg.set_level(level);
                }
            }
            Self::Value(val) => val.set_level(level),
            Self::Erased(ty) => ty.set_level(level),
            Self::Mono(_) | Self::Failure => {}
        }
    }
}

impl StructuralEq for TyParam {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Type(l), Self::Type(r)) => l.structural_eq(r),
            (Self::List(l), Self::List(r)) => l.iter().zip(r).all(|(l, r)| l.structural_eq(r)),
            (Self::UnsizedList(l), Self::UnsizedList(r)) => l.structural_eq(r),
            (Self::Tuple(l), Self::Tuple(r)) => l.iter().zip(r).all(|(l, r)| l.structural_eq(r)),
            (Self::Dict(l), Self::Dict(r)) => {
                if l.len() != r.len() {
                    return false;
                }
                for (key, val) in l.iter() {
                    if let Some(r_val) = r.get_by(key, |l, r| l.structural_eq(r)) {
                        if !val.structural_eq(r_val) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (Self::Record(l), Self::Record(r)) => {
                if l.len() != r.len() {
                    return false;
                }
                for (l_field, l_val) in l.iter() {
                    if let Some((r_field, r_val)) = r.get_key_value(l_field) {
                        if l_field.vis != r_field.vis || !l_val.structural_eq(r_val) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (
                Self::DataClass { name, fields },
                Self::DataClass {
                    name: r_name,
                    fields: r_fields,
                },
            ) => {
                if name != r_name || fields.len() != r_fields.len() {
                    return false;
                }
                for (l_field, l_val) in fields.iter() {
                    if let Some((r_field, r_val)) = r_fields.get_key_value(l_field) {
                        if l_field.vis != r_field.vis || !l_val.structural_eq(r_val) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (Self::Set(l), Self::Set(r)) => {
                if l.len() != r.len() {
                    return false;
                }
                for l_val in l.iter() {
                    if r.get_by(l_val, |l, r| l.structural_eq(r)).is_none() {
                        return false;
                    }
                }
                true
            }
            (Self::Lambda(l), Self::Lambda(r)) => l.structural_eq(r),
            (
                Self::Proj { obj, attr },
                Self::Proj {
                    obj: r_obj,
                    attr: r_attr,
                },
            ) => obj.structural_eq(r_obj) && attr == r_attr,
            (
                Self::ProjCall { obj, attr, args },
                Self::ProjCall {
                    obj: r_obj,
                    attr: r_attr,
                    args: r_args,
                },
            ) => {
                obj.structural_eq(r_obj)
                    && attr == r_attr
                    && args.iter().zip(r_args).all(|(l, r)| l.structural_eq(r))
            }
            (
                Self::App {
                    name: ln,
                    args: lps,
                },
                Self::App {
                    name: rn,
                    args: rps,
                },
            ) => ln == rn && lps.iter().zip(rps).all(|(l, r)| l.structural_eq(r)),
            (
                Self::UnaryOp { op, val },
                Self::UnaryOp {
                    op: r_op,
                    val: r_val,
                },
            ) => op == r_op && val.structural_eq(r_val),
            (
                Self::BinOp { op, lhs, rhs },
                Self::BinOp {
                    op: r_op,
                    lhs: r_lhs,
                    rhs: r_rhs,
                },
            ) => op == r_op && lhs.structural_eq(r_lhs) && rhs.structural_eq(r_rhs),
            (Self::Erased(l), Self::Erased(r)) => l.structural_eq(r),
            (Self::FreeVar(fv), other) | (other, Self::FreeVar(fv)) if fv.is_linked() => {
                fv.crack().structural_eq(other)
            }
            (Self::FreeVar(l), Self::FreeVar(r)) => l.structural_eq(r),
            (Self::Type(l), Self::Value(ValueObj::Type(r))) => l.structural_eq(r.typ()),
            (Self::Value(ValueObj::Type(l)), Self::Type(r)) => l.typ().structural_eq(r),
            _ => self == other,
        }
    }
}

impl TyParam {
    pub fn t(t: Type) -> Self {
        Self::Type(Box::new(t))
    }

    pub fn mono<S: Into<Str>>(name: S) -> Self {
        Self::Mono(name.into())
    }

    pub fn mono_q<S: Into<Str>>(name: S, constr: Constraint) -> Self {
        Self::named_free_var(name.into(), crate::ty::free::GENERIC_LEVEL, constr)
    }

    pub fn proj<S: Into<Str>>(self, attr: S) -> Self {
        Self::Proj {
            obj: Box::new(self),
            attr: attr.into(),
        }
    }

    pub fn proj_call(self, attr: Str, args: Vec<TyParam>) -> Self {
        Self::ProjCall {
            obj: Box::new(self),
            attr,
            args,
        }
    }

    pub fn range(start: Self, end: Self) -> Self {
        Self::DataClass {
            name: "Range".into(),
            fields: dict! {
                Field::private("start".into()) => start,
                Field::private("end".into()) => end,
                Field::private("step".into()) => ValueObj::None.into(),
            },
        }
    }

    pub fn free_instance(level: usize, t: Type) -> Self {
        let constraint = Constraint::new_type_of(t);
        Self::FreeVar(FreeTyParam::new_unbound(level, constraint))
    }

    pub fn free_var(level: usize, constr: Constraint) -> Self {
        Self::FreeVar(FreeTyParam::new_unbound(level, constr))
    }

    pub fn named_free_var(name: Str, level: usize, constr: Constraint) -> Self {
        Self::FreeVar(FreeTyParam::new_named_unbound(name, level, constr))
    }

    /// NOTE: Always add postfix when entering numbers. For example, `value(1)` will be of type Int.
    #[inline]
    pub fn value<V: Into<ValueObj>>(v: V) -> Self {
        Self::Value(v.into())
    }

    #[inline]
    pub fn unary(op: OpKind, val: TyParam) -> Self {
        Self::UnaryOp {
            op,
            val: Box::new(val),
        }
    }

    #[inline]
    pub fn bin(op: OpKind, lhs: TyParam, rhs: TyParam) -> Self {
        Self::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn app(name: Str, args: Vec<TyParam>) -> Self {
        Self::App { name, args }
    }

    #[inline]
    pub fn erased(t: Type) -> Self {
        Self::Erased(Box::new(t))
    }

    pub fn unsized_list(elem: TyParam) -> Self {
        Self::UnsizedList(Box::new(elem))
    }

    // if self: Ratio, Succ(self) => self+ε
    pub fn succ(self) -> Self {
        Self::app("succ".into(), vec![self])
    }

    // if self: Ratio, Pred(self) => self-ε
    pub fn pred(self) -> Self {
        Self::app("pred".into(), vec![self])
    }

    pub fn qual_name(&self) -> Option<Str> {
        match self {
            Self::Type(t) => Some(t.qual_name()),
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().qual_name(),
            Self::FreeVar(fv) if fv.is_generalized() => fv.unbound_name(),
            Self::Mono(name) => Some(name.clone()),
            Self::Value(ValueObj::Type(t)) => Some(t.typ().qual_name()),
            _ => None,
        }
    }

    pub fn local_name(&self) -> Option<Str> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().local_name(),
            Self::Mono(name) | Self::App { name, .. } => {
                let namespaces = name.split_with(&[".", "::"]);
                Some(Str::rc(namespaces.last().unwrap()))
            }
            _ => self.qual_name(),
        }
    }

    pub fn tvar_name(&self) -> Option<Str> {
        match self {
            Self::Type(t) => t.tvar_name(),
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().tvar_name(),
            Self::FreeVar(fv) => fv.unbound_name(),
            Self::Value(ValueObj::Type(t)) => t.typ().tvar_name(),
            _ => None,
        }
    }

    // 定数の比較など環境が必要な場合はContext::try_cmpを使う
    pub fn cheap_cmp(&self, r: &TyParam) -> Option<TyParamOrdering> {
        match (self, r) {
            (Self::Type(l), Self::Type(r)) =>
                if l == r { Some(TyParamOrdering::Equal) } else { Some(TyParamOrdering::NotEqual) },
            (Self::Value(l), Self::Value(r)) =>
                l.try_cmp(r).map(Into::into),
            (Self::FreeVar(fv), p) if fv.is_linked() =>
                fv.crack().cheap_cmp(p),
            (p, Self::FreeVar(fv)) if fv.is_linked() =>
                p.cheap_cmp(&fv.crack()),
            (Self::FreeVar{ .. } | Self::Erased(_), Self::FreeVar{ .. } | Self::Erased(_))
            /* if v.is_unbound() */ => Some(Any),
            (Self::App{ name, args }, Self::App{ name: rname, args: rargs }) =>
                if name == rname
                && args.len() == rargs.len()
                && args.iter().zip(rargs.iter()).all(|(l, r)| l.cheap_cmp(r) == Some(Equal)) {
                    Some(TyParamOrdering::Equal)
                } else {
                    Some(TyParamOrdering::NotEqual)
                },
            (l, r @ (Self::Erased(_) | Self::Mono{ .. } | Self::FreeVar{ .. })) =>
                r.cheap_cmp(l).map(|ord| ord.reverse()),
            _ => None,
        }
    }

    pub fn coerce(&self, list: Option<&UndoableLinkedList>) {
        if let Some(list) = list {
            self.undoable_coerce(list);
        } else {
            self.destructive_coerce();
        }
    }

    pub fn undoable_coerce(&self, list: &UndoableLinkedList) {
        match self {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                fv.crack().undoable_coerce(list);
            }
            TyParam::FreeVar(fv) if fv.is_unbound_and_typed() => {
                let typ = fv.get_type().unwrap();
                typ.undoable_coerce(list);
            }
            TyParam::Type(t) => t.undoable_coerce(list),
            TyParam::Value(ValueObj::Type(t)) => t.typ().undoable_coerce(list),
            // TODO:
            _ => {}
        }
    }

    pub fn destructive_coerce(&self) {
        match self {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                fv.crack().destructive_coerce();
            }
            TyParam::Type(t) => t.destructive_coerce(),
            TyParam::Value(ValueObj::Type(t)) => t.typ().destructive_coerce(),
            _ => {}
        }
    }

    pub fn qvars(&self) -> Set<(Str, Constraint)> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.forced_as_ref().linked().unwrap().qvars(),
            Self::FreeVar(fv) if !fv.constraint_is_uninited() => {
                let base = set! {(fv.unbound_name().unwrap(), fv.constraint().unwrap())};
                if let Some(ty) = fv.get_type() {
                    base.concat(ty.qvars())
                } else {
                    base
                }
            }
            Self::FreeVar(_) => set! {},
            Self::Type(t) => t.qvars(),
            Self::Proj { obj, .. } => obj.qvars(),
            Self::ProjCall { obj, args, .. } => args
                .iter()
                .fold(obj.qvars(), |acc, arg| acc.concat(arg.qvars())),
            Self::List(ts) | Self::Tuple(ts) => {
                ts.iter().fold(set! {}, |acc, t| acc.concat(t.qvars()))
            }
            Self::UnsizedList(elem) => elem.qvars(),
            Self::Set(ts) => ts.iter().fold(set! {}, |acc, t| acc.concat(t.qvars())),
            Self::Dict(ts) => ts.iter().fold(set! {}, |acc, (k, v)| {
                acc.concat(k.qvars().concat(v.qvars()))
            }),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => rec
                .iter()
                .fold(set! {}, |acc, (_, v)| acc.concat(v.qvars())),
            Self::Lambda(lambda) => lambda
                .body
                .iter()
                .fold(set! {}, |acc, t| acc.concat(t.qvars())),
            Self::UnaryOp { val, .. } => val.qvars(),
            Self::BinOp { lhs, rhs, .. } => lhs.qvars().concat(rhs.qvars()),
            Self::App { args, .. } => args.iter().fold(set! {}, |acc, p| acc.concat(p.qvars())),
            Self::Erased(t) => t.qvars(),
            Self::Value(val) => val.qvars(),
            Self::Mono(_) | Self::Failure => set! {},
        }
    }

    pub fn has_qvar(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_unbound() && fv.is_generalized() => true,
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().has_qvar(),
            Self::FreeVar(_) => false,
            Self::Type(t) => t.has_qvar(),
            Self::Proj { obj, .. } => obj.has_qvar(),
            Self::ProjCall { obj, args, .. } => obj.has_qvar() || args.iter().any(|t| t.has_qvar()),
            Self::List(tps) | Self::Tuple(tps) => tps.iter().any(|tp| tp.has_qvar()),
            Self::UnsizedList(elem) => elem.has_qvar(),
            Self::Set(tps) => tps.iter().any(|tp| tp.has_qvar()),
            Self::Dict(tps) => tps.iter().any(|(k, v)| k.has_qvar() || v.has_qvar()),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().any(|(_, tp)| tp.has_qvar())
            }
            Self::Lambda(lambda) => lambda.body.iter().any(|tp| tp.has_qvar()),
            Self::UnaryOp { val, .. } => val.has_qvar(),
            Self::BinOp { lhs, rhs, .. } => lhs.has_qvar() || rhs.has_qvar(),
            Self::App { args, .. } => args.iter().any(|p| p.has_qvar()),
            Self::Erased(t) => t.has_qvar(),
            Self::Value(val) => val.has_qvar(),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn contains_tvar(&self, target: &FreeTyVar) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_tvar(target),
            Self::FreeVar(fv) => fv.get_type().map_or(false, |t| t.contains_tvar(target)),
            Self::Type(t) => t.contains_tvar(target),
            Self::Erased(t) => t.contains_tvar(target),
            Self::Proj { obj, .. } => obj.contains_tvar(target),
            Self::ProjCall { obj, args, .. } => {
                obj.contains_tvar(target) || args.iter().any(|t| t.contains_tvar(target))
            }
            Self::List(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.contains_tvar(target)),
            Self::UnsizedList(elem) => elem.contains_tvar(target),
            Self::Set(ts) => ts.iter().any(|t| t.contains_tvar(target)),
            Self::Dict(ts) => ts
                .iter()
                .any(|(k, v)| k.contains_tvar(target) || v.contains_tvar(target)),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().any(|(_, tp)| tp.contains_tvar(target))
            }
            Self::Lambda(lambda) => lambda.body.iter().any(|tp| tp.contains_tvar(target)),
            Self::UnaryOp { val, .. } => val.contains_tvar(target),
            Self::BinOp { lhs, rhs, .. } => lhs.contains_tvar(target) || rhs.contains_tvar(target),
            Self::App { args, .. } => args.iter().any(|p| p.contains_tvar(target)),
            Self::Value(val) => val.contains_tvar(target),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn contains_type(&self, target: &Type) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_type(target),
            Self::FreeVar(fv) => fv.get_type().map_or(false, |t| t.contains_type(target)),
            Self::Type(t) => t.contains_type(target),
            Self::Erased(t) => t.contains_type(target),
            Self::Proj { obj, .. } => obj.contains_type(target),
            Self::ProjCall { obj, args, .. } => {
                obj.contains_type(target) || args.iter().any(|t| t.contains_type(target))
            }
            Self::List(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.contains_type(target)),
            Self::UnsizedList(elem) => elem.contains_type(target),
            Self::Set(ts) => ts.iter().any(|t| t.contains_type(target)),
            Self::Dict(ts) => ts
                .iter()
                .any(|(k, v)| k.contains_type(target) || v.contains_type(target)),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().any(|(_, tp)| tp.contains_type(target))
            }
            Self::Lambda(lambda) => lambda.body.iter().any(|tp| tp.contains_type(target)),
            Self::UnaryOp { val, .. } => val.contains_type(target),
            Self::BinOp { lhs, rhs, .. } => lhs.contains_type(target) || rhs.contains_type(target),
            Self::App { args, .. } => args.iter().any(|p| p.contains_type(target)),
            Self::Value(val) => val.contains_type(target),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn has_type_satisfies(&self, f: impl Fn(&Type) -> bool + Copy) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().has_type_satisfies(f),
            Self::FreeVar(fv) => fv.get_type().map_or(false, |t| f(&t)),
            Self::Type(t) => f(t),
            Self::Erased(t) => f(t),
            Self::Proj { obj, .. } => obj.has_type_satisfies(f),
            Self::ProjCall { obj, args, .. } => {
                obj.has_type_satisfies(f) || args.iter().any(|tp| tp.has_type_satisfies(f))
            }
            Self::List(ts) | Self::Tuple(ts) => ts.iter().any(|tp| tp.has_type_satisfies(f)),
            Self::UnsizedList(elem) => elem.has_type_satisfies(f),
            Self::Set(ts) => ts.iter().any(|tp| tp.has_type_satisfies(f)),
            Self::Dict(ts) => ts
                .iter()
                .any(|(k, v)| k.has_type_satisfies(f) || v.has_type_satisfies(f)),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.values().any(|tp| tp.has_type_satisfies(f))
            }
            Self::Lambda(lambda) => lambda.body.iter().any(|tp| tp.has_type_satisfies(f)),
            Self::UnaryOp { val, .. } => val.has_type_satisfies(f),
            Self::BinOp { lhs, rhs, .. } => lhs.has_type_satisfies(f) || rhs.has_type_satisfies(f),
            Self::App { args, .. } => args.iter().any(|p| p.has_type_satisfies(f)),
            Self::Value(val) => val.has_type_satisfies(f),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn contains_tp(&self, target: &TyParam) -> bool {
        if self == target {
            return true;
        }
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_tp(target),
            Self::FreeVar(fv) => fv.get_type().map_or(false, |t| t.contains_tp(target)),
            Self::Type(t) => t.contains_tp(target),
            Self::Erased(t) => t.contains_tp(target),
            Self::Proj { obj, .. } => obj.contains_tp(target),
            Self::ProjCall { obj, args, .. } => {
                obj.contains_tp(target) || args.iter().any(|t| t.contains_tp(target))
            }
            Self::List(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.contains_tp(target)),
            Self::UnsizedList(elem) => elem.contains_tp(target),
            Self::Set(ts) => ts.iter().any(|t| t.contains_tp(target)),
            Self::Dict(ts) => ts
                .iter()
                .any(|(k, v)| k.contains_tp(target) || v.contains_tp(target)),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().any(|(_, tp)| tp.contains_tp(target))
            }
            Self::Lambda(lambda) => lambda.body.iter().any(|tp| tp.contains_tp(target)),
            Self::UnaryOp { val, .. } => val.contains_tp(target),
            Self::BinOp { lhs, rhs, .. } => lhs.contains_tp(target) || rhs.contains_tp(target),
            Self::App { args, .. } => args.iter().any(|p| p.contains_tp(target)),
            Self::Value(val) => val.contains_tp(target),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn contains_value(&self, target: &ValueObj) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_value(target),
            Self::FreeVar(fv) => fv.get_type().map_or(false, |t| t.contains_value(target)),
            Self::Type(t) => t.contains_value(target),
            Self::Erased(t) => t.contains_value(target),
            Self::Proj { obj, .. } => obj.contains_value(target),
            Self::ProjCall { obj, args, .. } => {
                obj.contains_value(target) || args.iter().any(|t| t.contains_value(target))
            }
            Self::List(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.contains_value(target)),
            Self::UnsizedList(elem) => elem.contains_value(target),
            Self::Set(ts) => ts.iter().any(|t| t.contains_value(target)),
            Self::Dict(ts) => ts
                .iter()
                .any(|(k, v)| k.contains_value(target) || v.contains_value(target)),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().any(|(_, tp)| tp.contains_value(target))
            }
            Self::Lambda(lambda) => lambda.body.iter().any(|tp| tp.contains_value(target)),
            Self::UnaryOp { val, .. } => val.contains_value(target),
            Self::BinOp { lhs, rhs, .. } => {
                lhs.contains_value(target) || rhs.contains_value(target)
            }
            Self::App { args, .. } => args.iter().any(|p| p.contains_value(target)),
            Self::Value(val) => val.contains(target),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn contains_failure(&self) -> bool {
        self.contains_tp(&TyParam::Failure)
            || self.contains_type(&Type::Failure)
            || self.contains_value(&ValueObj::Failure)
    }

    pub fn is_recursive(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_recursive(),
            Self::FreeVar(fv) => fv.get_type().map_or(false, |t| t.contains_tp(self)),
            Self::Proj { obj, .. } => obj.contains_tp(self),
            Self::ProjCall { obj, args, .. } => {
                obj.contains_tp(self) || args.iter().any(|t| t.contains_tp(self))
            }
            Self::BinOp { lhs, rhs, .. } => lhs.contains_tp(self) || rhs.contains_tp(self),
            Self::UnaryOp { val, .. } => val.contains_tp(self),
            Self::App { args, .. } => args.iter().any(|t| t.contains_tp(self)),
            Self::Lambda(lambda) => lambda.body.iter().any(|t| t.contains_tp(self)),
            Self::List(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.contains_tp(self)),
            Self::UnsizedList(elem) => elem.contains_tp(self),
            Self::Set(ts) => ts.iter().any(|t| t.contains_tp(self)),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().any(|(_, t)| t.contains_tp(self))
            }
            Self::Dict(ts) => ts
                .iter()
                .any(|(k, v)| k.contains_tp(self) || v.contains_tp(self)),
            Self::Type(t) => t.contains_tp(self),
            Self::Value(val) => val.contains_tp(self),
            Self::Erased(t) => t.contains_tp(self),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn is_unbound_var(&self) -> bool {
        match self {
            Self::FreeVar(fv) => fv.is_unbound() || fv.crack().is_unbound_var(),
            Self::Type(t) => t.is_unbound_var(),
            Self::Value(ValueObj::Type(t)) => t.typ().is_unbound_var(),
            _ => false,
        }
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::FreeVar(fv) => fv.has_unbound_var(),
            Self::Type(t) => t.has_unbound_var(),
            Self::Proj { obj, .. } => obj.has_unbound_var(),
            Self::ProjCall { obj, args, .. } => {
                obj.has_unbound_var() || args.iter().any(|t| t.has_unbound_var())
            }
            Self::List(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.has_unbound_var()),
            Self::UnsizedList(elem) => elem.has_unbound_var(),
            Self::Set(ts) => ts.iter().any(|t| t.has_unbound_var()),
            Self::Dict(kv) => kv
                .iter()
                .any(|(k, v)| k.has_unbound_var() || v.has_unbound_var()),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().any(|(_, v)| v.has_unbound_var())
            }
            Self::Lambda(lambda) => lambda.body.iter().any(|t| t.has_unbound_var()),
            Self::UnaryOp { val, .. } => val.has_unbound_var(),
            Self::BinOp { lhs, rhs, .. } => lhs.has_unbound_var() || rhs.has_unbound_var(),
            Self::App { args, .. } => args.iter().any(|p| p.has_unbound_var()),
            Self::Erased(t) => t.has_unbound_var(),
            Self::Value(val) => val.has_unbound_var(),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn has_no_unbound_var(&self) -> bool {
        !self.has_unbound_var()
    }

    pub fn has_undoable_linked_var(&self) -> bool {
        match self {
            Self::FreeVar(fv) => fv.is_undoable_linked(),
            Self::Type(t) => t.has_undoable_linked_var(),
            Self::Proj { obj, .. } => obj.has_undoable_linked_var(),
            Self::ProjCall { obj, args, .. } => {
                obj.has_undoable_linked_var() || args.iter().any(|t| t.has_undoable_linked_var())
            }
            Self::List(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.has_undoable_linked_var()),
            Self::UnsizedList(elem) => elem.has_undoable_linked_var(),
            Self::Set(ts) => ts.iter().any(|t| t.has_undoable_linked_var()),
            Self::Dict(kv) => kv
                .iter()
                .any(|(k, v)| k.has_undoable_linked_var() || v.has_undoable_linked_var()),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().any(|(_, v)| v.has_undoable_linked_var())
            }
            Self::Lambda(lambda) => lambda.body.iter().any(|t| t.has_undoable_linked_var()),
            Self::UnaryOp { val, .. } => val.has_undoable_linked_var(),
            Self::BinOp { lhs, rhs, .. } => {
                lhs.has_undoable_linked_var() || rhs.has_undoable_linked_var()
            }
            Self::App { args, .. } => args.iter().any(|p| p.has_undoable_linked_var()),
            Self::Erased(t) => t.has_undoable_linked_var(),
            Self::Value(val) => val.has_undoable_linked_var(),
            Self::Mono(_) | Self::Failure => false,
        }
    }

    pub fn union_size(&self) -> usize {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().union_size(),
            Self::Type(t) => t.union_size(),
            Self::Proj { obj, .. } => obj.union_size(),
            Self::ProjCall { obj, args, .. } => obj
                .union_size()
                .max(args.iter().map(|t| t.union_size()).max().unwrap_or(1)),
            Self::List(ts) | Self::Tuple(ts) => {
                ts.iter().map(|t| t.union_size()).max().unwrap_or(1)
            }
            Self::UnsizedList(elem) => elem.union_size(),
            Self::Set(ts) => ts.iter().map(|t| t.union_size()).max().unwrap_or(1),
            Self::Dict(kv) => kv
                .iter()
                .map(|(k, v)| k.union_size().max(v.union_size()))
                .max()
                .unwrap_or(1),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                rec.iter().map(|(_, v)| v.union_size()).max().unwrap_or(1)
            }
            Self::Lambda(lambda) => lambda
                .body
                .iter()
                .map(|t| t.union_size())
                .max()
                .unwrap_or(1),
            Self::UnaryOp { val, .. } => val.union_size(),
            Self::BinOp { lhs, rhs, .. } => lhs.union_size().max(rhs.union_size()),
            Self::App { args, .. } => args.iter().map(|p| p.union_size()).max().unwrap_or(1),
            Self::Erased(t) => t.union_size(),
            Self::Value(ValueObj::Type(t)) => t.typ().union_size(),
            _ => 1,
        }
    }

    pub fn has_upper_bound(&self) -> bool {
        match self {
            // TODO: 型によっては上限がある
            // また、上限がないもの同士の加算等も上限はない
            Self::Erased(_) => false,
            Self::FreeVar(fv) => !fv.is_unbound(), // != fv.is_linked(),
            _ => true,
        }
    }

    pub fn has_lower_bound(&self) -> bool {
        match self {
            Self::Erased(_) => false,
            Self::FreeVar(fv) => !fv.is_unbound(),
            _ => true,
        }
    }

    pub fn is_erased(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_erased(),
            Self::Erased(_) => true,
            _ => false,
        }
    }

    pub fn is_type(&self) -> bool {
        match self {
            Self::Type(_) => true,
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().is_type(),
            Self::Value(ValueObj::Type(_)) => true,
            _ => false,
        }
    }

    pub fn substitute(self, var: &str, to: &TyParam) -> TyParam {
        if self.qual_name().is_some_and(|n| n == var) {
            return to.clone();
        }
        self.map(&mut |tp| tp.substitute(var, to))
    }

    pub fn replace(self, target: &TyParam, to: &TyParam) -> TyParam {
        let table = ReplaceTable::make_tp(target, to);
        table.replace_tp(self)
    }

    pub(crate) fn _replace(self, target: &TyParam, to: &TyParam) -> TyParam {
        if &self == target {
            return to.clone();
        }
        match self {
            TyParam::Type(t) => TyParam::t(t._replace_tp(target, to)),
            TyParam::Value(val) => TyParam::value(val.replace_tp(target, to)),
            self_ => self_.map(&mut |tp| tp._replace(target, to)),
        }
    }

    pub fn replace_t(self, target: &Type, to: &Type) -> TyParam {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.unwrap_linked().replace_t(target, to),
            Self::FreeVar(fv) if fv.get_type().is_some() => {
                let typ = fv.get_type().unwrap()._replace(target, to);
                fv.update_type(typ);
                Self::FreeVar(fv)
            }
            Self::Type(t) => Self::t(t._replace(target, to)),
            Self::Erased(t) => Self::erased(t._replace(target, to)),
            Self::Value(val) => Self::Value(val.replace_t(target, to)),
            _ => self.map(&mut |tp| tp.replace_t(target, to)),
        }
    }

    pub fn eliminate_t(self, target: &Type) -> Self {
        self.replace_t(target, &Type::Never)
    }

    /// TyParam::Value(ValueObj::Type(_)) => TyParam::Type
    pub fn normalize(self) -> TyParam {
        match self {
            TyParam::Value(ValueObj::Type(obj)) => TyParam::t(obj.typ().clone().normalize()),
            TyParam::Type(t) => TyParam::t(t.normalize()),
            other => other,
        }
    }

    fn addr_eq(&self, other: &TyParam) -> bool {
        match (self, other) {
            (Self::FreeVar(slf), _) if slf.is_linked() => slf.crack().addr_eq(other),
            (_, Self::FreeVar(otr)) if otr.is_linked() => otr.crack().addr_eq(self),
            (Self::FreeVar(slf), Self::FreeVar(otr)) => slf.addr_eq(otr),
            (Self::Type(slf), Self::Type(otr)) => slf.addr_eq(otr),
            (Self::Value(ValueObj::Type(slf)), Self::Value(ValueObj::Type(otr))) => {
                slf.typ().addr_eq(otr.typ())
            }
            (Self::Type(slf), Self::Value(ValueObj::Type(otr))) => slf.addr_eq(otr.typ()),
            (Self::Value(ValueObj::Type(slf)), Self::Type(otr)) => slf.typ().addr_eq(otr),
            _ => ref_addr_eq!(self, other),
        }
    }

    /// interior-mut
    pub(crate) fn destructive_link(&self, to: &TyParam) {
        if self.addr_eq(to) {
            return;
        }
        if self.level() == Some(GENERIC_LEVEL) {
            if DEBUG_MODE {
                panic!("{self} is fixed");
            }
            return;
        }
        match self {
            Self::FreeVar(fv) => {
                if to.contains_tp(self) {
                    let to = to.clone().eliminate_recursion(self);
                    fv.link(&to);
                } else {
                    fv.link(to);
                }
            }
            Self::Type(t) => {
                if let Ok(to) = <&Type>::try_from(to) {
                    t.destructive_link(to);
                } else if DEBUG_MODE {
                    panic!("{to} is not a type");
                }
            }
            Self::Value(ValueObj::Type(t)) => {
                if let Ok(to) = <&Type>::try_from(to) {
                    t.typ().destructive_link(to);
                } else if DEBUG_MODE {
                    panic!("{to} is not a type");
                }
            }
            _ => {
                if DEBUG_MODE {
                    panic!("{self} is not a free variable")
                }
            }
        }
    }

    /// interior-mut
    pub(crate) fn undoable_link(&self, to: &TyParam, list: &UndoableLinkedList) {
        list.push_tp(self);
        if self.addr_eq(to) {
            self.inc_undo_count();
            return;
        }
        match self {
            Self::FreeVar(fv) => {
                if to.contains_tp(self) {
                    let to = to.clone().eliminate_recursion(self);
                    fv.undoable_link(&to);
                } else {
                    fv.undoable_link(to);
                }
            }
            Self::Type(t) => {
                if let Ok(to) = <&Type>::try_from(to) {
                    t.undoable_link(to, list);
                } else if DEBUG_MODE {
                    panic!("{to} is not a type");
                }
            }
            Self::Value(ValueObj::Type(t)) => {
                if let Ok(to) = <&Type>::try_from(to) {
                    t.typ().undoable_link(to, list);
                } else if DEBUG_MODE {
                    panic!("{to} is not a type");
                }
            }
            _ => {
                if DEBUG_MODE {
                    panic!("{self} is not a free variable")
                }
            }
        }
    }

    pub(crate) fn link(&self, to: &TyParam, list: Option<&UndoableLinkedList>) {
        if let Some(list) = list {
            self.undoable_link(to, list);
        } else {
            self.destructive_link(to);
        }
    }

    pub(crate) fn undo(&self) {
        match self {
            Self::FreeVar(fv) if fv.is_undoable_linked() => fv.undo(),
            Self::Type(t) => t.undo(),
            Self::Value(ValueObj::Type(t)) => t.typ().undo(),
            /*Self::App { args, .. } => {
                for arg in args {
                    arg.undo();
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
        let Some(level) = self.level() else {
            if DEBUG_MODE {
                todo!();
            }
            return;
        };
        let new = if let Some(name) = self.unbound_name() {
            Self::named_free_var(name, level, new_constraint)
        } else {
            Self::free_var(level, new_constraint)
        };
        self.undoable_link(&new, list);
    }

    pub(crate) fn update_constraint(
        &self,
        new_constraint: Constraint,
        list: Option<&UndoableLinkedList>,
        in_instantiation: bool,
    ) {
        if let Some(list) = list {
            self.undoable_update_constraint(new_constraint, list);
        } else {
            self.destructive_update_constraint(new_constraint, in_instantiation);
        }
    }

    fn inc_undo_count(&self) {
        match self {
            Self::FreeVar(fv) => fv.inc_undo_count(),
            _ => panic!("{self} is not a free variable"),
        }
    }

    pub fn typarams(&self) -> Vec<TyParam> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().typarams(),
            Self::Type(t) => t.typarams(),
            Self::Value(val) => val.typarams(),
            Self::App { args, .. } => args.clone(),
            _ => vec![],
        }
    }

    pub fn contained_ts(&self) -> Set<Type> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contained_ts(),
            Self::Type(t) => t.contained_ts(),
            Self::Value(val) => val.contained_ts(),
            Self::App { args, .. } => args
                .iter()
                .fold(set! {}, |acc, p| acc.concat(p.contained_ts())),
            _ => set! {},
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
            Self::FreeVar(_) => {}
            Self::Type(t) => t.dereference(),
            Self::Value(val) => val.dereference(),
            Self::App { args, .. } => {
                for arg in args {
                    arg.dereference();
                }
            }
            Self::Proj { obj, .. } => obj.dereference(),
            Self::ProjCall { obj, args, .. } => {
                obj.dereference();
                for arg in args {
                    arg.dereference();
                }
            }
            Self::List(ts) | Self::Tuple(ts) => {
                for t in ts {
                    t.dereference();
                }
            }
            Self::UnsizedList(elem) => elem.dereference(),
            Self::Set(ts) => {
                let ts_ = std::mem::take(ts);
                *ts = ts_
                    .into_iter()
                    .map(|mut t| {
                        t.dereference();
                        t
                    })
                    .collect();
            }
            Self::Dict(ts) => {
                let ts_ = std::mem::take(ts);
                *ts = ts_
                    .into_iter()
                    .map(|(mut k, mut v)| {
                        k.dereference();
                        v.dereference();
                        (k, v)
                    })
                    .collect();
            }
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => {
                for (_, t) in rec.iter_mut() {
                    t.dereference();
                }
            }
            Self::Lambda(lambda) => {
                for t in &mut lambda.body {
                    t.dereference();
                }
            }
            Self::UnaryOp { val, .. } => val.dereference(),
            Self::BinOp { lhs, rhs, .. } => {
                lhs.dereference();
                rhs.dereference();
            }
            Self::Erased(t) => t.dereference(),
            Self::Mono(_) | Self::Failure => {}
        }
    }

    pub fn namespace(&self) -> Str {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().namespace(),
            Self::Mono(name) | Self::App { name, .. } => {
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

    pub fn variables(&self) -> Set<Str> {
        match self {
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().variables(),
            Self::FreeVar(fv) if fv.get_type().is_some() => fv.get_type().unwrap().variables(),
            Self::FreeVar(_) => set! {},
            Self::Mono(name) => set! { name.clone() },
            Self::App { name, args } => {
                let mut set = set! { name.clone() };
                for arg in args {
                    set.merge(arg.variables());
                }
                set
            }
            Self::List(tps) | Self::Tuple(tps) => {
                tps.iter().fold(set! {}, |acc, t| acc.concat(t.variables()))
            }
            Self::Set(tps) => tps.iter().fold(set! {}, |acc, t| acc.concat(t.variables())),
            Self::Record(rec) | Self::DataClass { fields: rec, .. } => rec
                .iter()
                .fold(set! {}, |acc, (_, v)| acc.concat(v.variables())),
            Self::Dict(tps) => tps.iter().fold(set! {}, |acc, (k, v)| {
                acc.concat(k.variables().concat(v.variables()))
            }),
            Self::UnsizedList(elem) => elem.variables(),
            Self::BinOp { lhs, rhs, .. } => lhs.variables().concat(rhs.variables()),
            Self::UnaryOp { val, .. } => val.variables(),
            Self::Lambda(lambda) => lambda
                .body
                .iter()
                .fold(set! {}, |acc, t| acc.concat(t.variables())),
            Self::Proj { obj, .. } => obj.variables(),
            Self::ProjCall { obj, args, .. } => {
                let mut set = obj.variables();
                for arg in args {
                    set.merge(arg.variables());
                }
                set
            }
            Self::Type(t) | Self::Erased(t) => t.variables(),
            Self::Value(val) => val.variables(),
            Self::Failure => set! {},
        }
    }

    /// For recursive function
    pub fn map(self, f: &mut impl FnMut(TyParam) -> TyParam) -> TyParam {
        match self {
            TyParam::FreeVar(fv) if fv.is_linked() => f(fv.unwrap_linked()),
            TyParam::FreeVar(fv) if fv.get_type().is_some() => {
                let typ = fv.get_type().unwrap();
                fv.update_type(typ.map_tp(f));
                TyParam::FreeVar(fv)
            }
            TyParam::FreeVar(_) => self,
            TyParam::App { name, args } => {
                let new_args = args.into_iter().map(f).collect::<Vec<_>>();
                TyParam::app(name, new_args)
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let new_lhs = f(*lhs);
                let new_rhs = f(*rhs);
                TyParam::bin(op, new_lhs, new_rhs)
            }
            TyParam::UnaryOp { op, val } => TyParam::unary(op, f(*val)),
            TyParam::UnsizedList(elem) => TyParam::unsized_list(f(*elem)),
            TyParam::List(tps) => TyParam::List(tps.into_iter().map(f).collect()),
            TyParam::Tuple(tps) => TyParam::Tuple(tps.into_iter().map(f).collect()),
            TyParam::Set(tps) => TyParam::Set(tps.into_iter().map(f).collect()),
            TyParam::Dict(tps) => {
                let new_tps = tps.into_iter().map(|(k, v)| (f(k), f(v))).collect();
                TyParam::Dict(new_tps)
            }
            TyParam::Record(rec) => {
                let new_rec = rec.into_iter().map(|(k, v)| (k, f(v))).collect();
                TyParam::Record(new_rec)
            }
            TyParam::DataClass { name, fields } => {
                let new_fields = fields.into_iter().map(|(k, v)| (k, f(v))).collect();
                TyParam::DataClass {
                    name,
                    fields: new_fields,
                }
            }
            TyParam::Lambda(lambda) => {
                let new_body = lambda.body.into_iter().map(f).collect();
                TyParam::Lambda(TyParamLambda {
                    body: new_body,
                    ..lambda
                })
            }
            TyParam::Proj { obj, attr } => TyParam::Proj {
                obj: Box::new(f(*obj)),
                attr,
            },
            TyParam::ProjCall { obj, attr, args } => TyParam::ProjCall {
                obj: Box::new(f(*obj)),
                attr,
                args: args.into_iter().map(f).collect::<Vec<_>>(),
            },
            TyParam::Value(val) => TyParam::Value(val.map_tp(f)),
            TyParam::Type(t) => TyParam::t(t.map_tp(f)),
            TyParam::Erased(t) => TyParam::erased(t.map_tp(f)),
            TyParam::Mono(_) | TyParam::Failure => self,
        }
    }

    pub fn map_t(self, f: &mut impl FnMut(Type) -> Type) -> TyParam {
        match self {
            TyParam::FreeVar(fv) if fv.is_linked() => fv.unwrap_linked().map_t(f),
            TyParam::FreeVar(fv) if fv.get_type().is_some() => {
                let typ = fv.get_type().unwrap();
                fv.update_type(f(typ));
                TyParam::FreeVar(fv)
            }
            TyParam::FreeVar(_) => self,
            TyParam::App { name, args } => {
                let new_args = args.into_iter().map(|tp| tp.map_t(f)).collect::<Vec<_>>();
                TyParam::app(name, new_args)
            }
            TyParam::BinOp { op, lhs, rhs } => TyParam::bin(op, lhs.map_t(f), rhs.map_t(f)),
            TyParam::UnaryOp { op, val } => TyParam::unary(op, val.map_t(f)),
            TyParam::UnsizedList(elem) => TyParam::unsized_list(elem.map_t(f)),
            TyParam::List(tps) => TyParam::List(tps.into_iter().map(|tp| tp.map_t(f)).collect()),
            TyParam::Tuple(tps) => TyParam::Tuple(tps.into_iter().map(|tp| tp.map_t(f)).collect()),
            TyParam::Set(tps) => TyParam::Set(tps.into_iter().map(|tp| tp.map_t(f)).collect()),
            TyParam::Dict(tps) => {
                let new_tps = tps
                    .into_iter()
                    .map(|(k, v)| (k.map_t(f), v.map_t(f)))
                    .collect();
                TyParam::Dict(new_tps)
            }
            TyParam::Record(rec) => {
                let new_rec = rec.into_iter().map(|(k, v)| (k, v.map_t(f))).collect();
                TyParam::Record(new_rec)
            }
            TyParam::DataClass { name, fields } => {
                let new_fields = fields.into_iter().map(|(k, v)| (k, v.map_t(f))).collect();
                TyParam::DataClass {
                    name,
                    fields: new_fields,
                }
            }
            TyParam::Lambda(lambda) => {
                let new_body = lambda.body.into_iter().map(|tp| tp.map_t(f)).collect();
                TyParam::Lambda(TyParamLambda {
                    body: new_body,
                    ..lambda
                })
            }
            TyParam::Proj { obj, attr } => TyParam::Proj {
                obj: Box::new(obj.map_t(f)),
                attr,
            },
            TyParam::ProjCall { obj, attr, args } => {
                let new_args = args.into_iter().map(|tp| tp.map_t(f)).collect::<Vec<_>>();
                TyParam::ProjCall {
                    obj: Box::new(obj.map_t(f)),
                    attr,
                    args: new_args,
                }
            }
            TyParam::Value(val) => TyParam::Value(val.map_t(f)),
            TyParam::Type(t) => TyParam::t(f(*t)),
            TyParam::Erased(t) => TyParam::erased(f(*t)),
            TyParam::Mono(_) | TyParam::Failure => self,
        }
    }

    pub fn is_free_var(&self) -> bool {
        match self {
            Self::FreeVar(_) => true,
            Self::Type(t) => t.is_free_var(),
            Self::Value(ValueObj::Type(t)) => t.typ().is_free_var(),
            _ => false,
        }
    }

    pub fn eliminate_recursion(self, target: &TyParam) -> Self {
        if self.is_free_var() && self.addr_eq(target) {
            return Self::Failure;
        }
        self.map(&mut |tp| tp.eliminate_recursion(target))
    }

    pub fn as_free(&self) -> Option<&FreeTyParam> {
        <&FreeTyParam>::try_from(self).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TyParamOrdering {
    Less,
    Equal,
    Greater,
    LessEqual,    // Less or Equal
    NotEqual,     // Less or Greater
    GreaterEqual, // Greater or Equal
    Any,
    NoRelation,
}

use TyParamOrdering::*;

impl From<Ordering> for TyParamOrdering {
    fn from(o: Ordering) -> Self {
        match o {
            Ordering::Less => Less,
            Ordering::Equal => Equal,
            Ordering::Greater => Greater,
        }
    }
}

impl TryFrom<TyParamOrdering> for Ordering {
    type Error = ();
    fn try_from(o: TyParamOrdering) -> Result<Self, Self::Error> {
        match o {
            Less => Ok(Ordering::Less),
            Equal => Ok(Ordering::Equal),
            Greater => Ok(Ordering::Greater),
            _ => Err(()),
        }
    }
}

impl TyParamOrdering {
    pub const fn canbe_eq(self) -> bool {
        matches!(self, LessEqual | GreaterEqual | Equal | Any)
    }
    pub const fn canbe_lt(self) -> bool {
        matches!(self, Less | LessEqual | NotEqual | Any)
    }
    pub const fn canbe_gt(self) -> bool {
        matches!(self, Greater | GreaterEqual | NotEqual | Any)
    }
    pub const fn canbe_le(self) -> bool {
        matches!(self, Less | LessEqual | Equal | Any)
    }
    pub const fn canbe_ge(self) -> bool {
        matches!(self, Greater | GreaterEqual | Equal | Any)
    }
    pub const fn canbe_ne(self) -> bool {
        matches!(self, NotEqual | Any)
    }
    pub const fn is_lt(&self) -> bool {
        matches!(self, Less | LessEqual | Any)
    }
    pub const fn is_le(&self) -> bool {
        matches!(self, Less | Equal | LessEqual | Any)
    }
    pub const fn is_gt(&self) -> bool {
        matches!(self, Greater | GreaterEqual | Any)
    }
    pub const fn is_ge(&self) -> bool {
        matches!(self, Greater | Equal | GreaterEqual | Any)
    }
    pub const fn is_eq(&self) -> bool {
        matches!(self, Equal | Any)
    }
    pub const fn is_ne(&self) -> bool {
        matches!(self, Less | Greater | NotEqual | Any)
    }
    pub const fn reverse(&self) -> Self {
        match self {
            Less => Greater,
            Greater => Less,
            LessEqual => GreaterEqual,
            GreaterEqual => LessEqual,
            Equal => NotEqual,
            NotEqual => Equal,
            Any | NoRelation => Any,
        }
    }
}
