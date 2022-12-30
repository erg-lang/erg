use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Range, RangeInclusive, Sub};
use std::rc::Rc;

use erg_common::dict::Dict;
use erg_common::set;
use erg_common::set::Set;
use erg_common::traits::LimitedDisplay;
use erg_common::Str;
use erg_common::{dict, log};

use super::constructors::int_interval;
use super::free::{CanbeFree, Constraint, FreeKind, FreeTyParam, HasLevel, Level, GENERIC_LEVEL};
use super::value::ValueObj;
use super::Type;

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
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Mutate,
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
            Self::And => write!(f, "and"),
            Self::Or => write!(f, "or"),
            Self::BitAnd => write!(f, "&&"),
            Self::BitOr => write!(f, "||"),
            Self::BitXor => write!(f, "^^"),
            Self::Shl => write!(f, "<<"),
            Self::Shr => write!(f, ">>"),
            Self::Mutate => write!(f, "!"),
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

/// type argument
/// This is an expression, not a evaluation result
/// * Literal: 1, "aa", True, None, ... (don't use container literals, they can only hold literals)
/// * Type: Int, Add(?R, ?O), ...
/// * Mono: I, N, ...
/// * Attr: math.PI, ...
/// * Array: `[1, 2, N]`
/// * Tuple: (1, N, True)
/// * App: Array(Int), Fib(10), ...
/// * QuantVar: N: Nat, ...
/// * FreeVar: ?I: Int, ...
/// * UnaryOp: -N, ~B, ...
/// * BinOp: 1 + 1, N * 2, ...
/// * Erased: _: Type, _: Nat, ...
#[derive(Debug, Clone, Hash)]
pub enum TyParam {
    Value(ValueObj),
    Type(Box<Type>),
    Array(Vec<TyParam>),
    Tuple(Vec<TyParam>),
    Set(Set<TyParam>),
    Dict(Dict<TyParam, TyParam>),
    Mono(Str),
    Proj {
        obj: Box<TyParam>,
        attr: Str,
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
            (Self::Array(l), Self::Array(r)) => l == r,
            (Self::Tuple(l), Self::Tuple(r)) => l == r,
            (Self::Dict(l), Self::Dict(r)) => l == r,
            (Self::Set(l), Self::Set(r)) => l == r,
            (Self::Mono(l), Self::Mono(r)) => l == r,
            (
                Self::Proj { obj, attr },
                Self::Proj {
                    obj: r_obj,
                    attr: r_attr,
                },
            ) => obj == r_obj && attr == r_attr,
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
    fn limited_fmt(&self, f: &mut fmt::Formatter<'_>, limit: usize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Value(v) => write!(f, "{v}"),
            Self::Failure => write!(f, "<Failure>"),
            Self::Type(t) => t.limited_fmt(f, limit - 1),
            Self::FreeVar(fv) => fv.limited_fmt(f, limit - 1),
            Self::UnaryOp { op, val } => {
                write!(f, "{}", op)?;
                val.limited_fmt(f, limit - 1)
            }
            Self::BinOp { op, lhs, rhs } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " {} ", op)?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::App { name, args } => {
                write!(f, "{}", name)?;
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
            Self::Mono(name) => write!(f, "{}", name),
            Self::Proj { obj, attr } => {
                write!(f, "{}.", obj)?;
                write!(f, "{}", attr)
            }
            Self::Array(arr) => {
                write!(f, "[")?;
                for (i, t) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    t.limited_fmt(f, limit - 1)?;
                }
                write!(f, "]")
            }
            Self::Set(st) => {
                write!(f, "{{")?;
                for (i, t) in st.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    t.limited_fmt(f, limit - 1)?;
                }
                write!(f, "}}")
            }
            Self::Dict(dict) => write!(f, "{dict}"),
            Self::Tuple(tuple) => {
                write!(f, "(")?;
                for (i, t) in tuple.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
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
            _ => None,
        }
    }

    fn constraint(&self) -> Option<Constraint> {
        match self {
            TyParam::FreeVar(fv) => fv.constraint(),
            TyParam::Type(t) => t.constraint(),
            _ => None,
        }
    }

    fn update_constraint(&self, new_constraint: Constraint) {
        match self {
            Self::FreeVar(fv) => {
                fv.update_constraint(new_constraint);
            }
            Self::Type(t) => {
                t.update_constraint(new_constraint);
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

impl TryFrom<TyParam> for ValueObj {
    type Error = ();
    fn try_from(tp: TyParam) -> Result<Self, ()> {
        match tp {
            TyParam::Array(tps) => {
                let mut vals = vec![];
                for tp in tps {
                    vals.push(ValueObj::try_from(tp)?);
                }
                Ok(ValueObj::Array(Rc::from(vals)))
            }
            TyParam::Tuple(tps) => {
                let mut vals = vec![];
                for tp in tps {
                    vals.push(ValueObj::try_from(tp)?);
                }
                Ok(ValueObj::Tuple(Rc::from(vals)))
            }
            TyParam::Dict(tps) => {
                let mut vals = dict! {};
                for (k, v) in tps {
                    vals.insert(ValueObj::try_from(k)?, ValueObj::try_from(v)?);
                }
                Ok(ValueObj::Dict(vals))
            }
            TyParam::FreeVar(fv) if fv.is_linked() => ValueObj::try_from(fv.crack().clone()),
            TyParam::Type(t) => Ok(ValueObj::builtin_t(*t)),
            TyParam::Value(v) => Ok(v),
            _ => {
                log!(err "Expected value, got {:?}", tp);
                Err(())
            }
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
            TyParam::Array(tps) => Ok(tps),
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
            // TODO: Array, Dict, Set
            _ => Err(()),
        }
    }
}

impl TryFrom<TyParam> for Type {
    type Error = ();
    fn try_from(tp: TyParam) -> Result<Type, ()> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                Type::try_from(fv.forced_as_ref().linked().unwrap().clone()).map_err(|_| ())
            }
            TyParam::Type(t) => Ok(*t),
            TyParam::Value(v) => Type::try_from(v),
            // TODO: Array, Dict, Set
            _ => Err(()),
        }
    }
}

impl HasLevel for TyParam {
    fn level(&self) -> Option<Level> {
        match self {
            Self::Type(t) => t.level(),
            Self::FreeVar(fv) => fv.level(),
            Self::Array(tps) | Self::Tuple(tps) => tps.iter().filter_map(|tp| tp.level()).min(),
            Self::Dict(tps) => tps
                .iter()
                .map(|(k, v)| {
                    k.level()
                        .unwrap_or(GENERIC_LEVEL)
                        .min(v.level().unwrap_or(GENERIC_LEVEL))
                })
                .min(),
            Self::Set(tps) => tps.iter().filter_map(|tp| tp.level()).min(),
            Self::Proj { obj, .. } => obj.level(),
            Self::App { args, .. } => args.iter().filter_map(|tp| tp.level()).min(),
            Self::UnaryOp { val, .. } => val.level(),
            Self::BinOp { lhs, rhs, .. } => lhs.level().and_then(|l| rhs.level().map(|r| l.min(r))),
            _ => None,
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
            Self::Array(tps) => {
                for tp in tps {
                    tp.set_level(level);
                }
            }
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
            _ => {}
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

    pub fn proj<S: Into<Str>>(obj: TyParam, attr: S) -> Self {
        Self::Proj {
            obj: Box::new(obj),
            attr: attr.into(),
        }
    }

    pub fn free_var(level: usize, t: Type) -> Self {
        let constraint = Constraint::new_type_of(t);
        Self::FreeVar(FreeTyParam::new_unbound(level, constraint))
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
    pub fn mutate(self) -> Self {
        Self::unary(OpKind::Mutate, self)
    }

    #[inline]
    pub fn bin(op: OpKind, lhs: TyParam, rhs: TyParam) -> Self {
        Self::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn app(name: &'static str, args: Vec<TyParam>) -> Self {
        Self::App {
            name: Str::ever(name),
            args,
        }
    }

    #[inline]
    pub fn erased(t: Type) -> Self {
        Self::Erased(Box::new(t))
    }

    // if self: Ratio, Succ(self) => self+ε
    pub fn succ(self) -> Self {
        Self::app("Succ", vec![self])
    }

    // if self: Ratio, Pred(self) => self-ε
    pub fn pred(self) -> Self {
        Self::app("Pred", vec![self])
    }

    pub fn qual_name(&self) -> Option<Str> {
        match self {
            Self::Type(t) => Some(t.qual_name()),
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().qual_name(),
            Self::FreeVar(fv) if fv.is_generalized() => fv.unbound_name(),
            Self::Mono(name) => Some(name.clone()),
            _ => None,
        }
    }

    pub fn tvar_name(&self) -> Option<Str> {
        match self {
            Self::Type(t) => t.tvar_name(),
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().tvar_name(),
            Self::FreeVar(fv) => fv.unbound_name(),
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

    pub fn coerce(&self) {
        match self {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                fv.crack().coerce();
            }
            TyParam::Type(t) => t.coerce(),
            _ => {}
        }
    }

    pub fn qvars(&self) -> Set<(Str, Constraint)> {
        match self {
            Self::FreeVar(fv) if !fv.constraint_is_uninited() => {
                set! { (fv.unbound_name().unwrap(), fv.constraint().unwrap()) }
            }
            Self::FreeVar(fv) if fv.is_linked() => fv.forced_as_ref().linked().unwrap().qvars(),
            Self::Type(t) => t.qvars(),
            Self::Proj { obj, .. } => obj.qvars(),
            Self::Array(ts) | Self::Tuple(ts) => {
                ts.iter().fold(set! {}, |acc, t| acc.concat(t.qvars()))
            }
            Self::Set(ts) => ts.iter().fold(set! {}, |acc, t| acc.concat(t.qvars())),
            Self::Dict(ts) => ts.iter().fold(set! {}, |acc, (k, v)| {
                acc.concat(k.qvars().concat(v.qvars()))
            }),
            Self::UnaryOp { val, .. } => val.qvars(),
            Self::BinOp { lhs, rhs, .. } => lhs.qvars().concat(rhs.qvars()),
            Self::App { args, .. } => args.iter().fold(set! {}, |acc, p| acc.concat(p.qvars())),
            Self::Erased(t) => t.qvars(),
            _ => set! {},
        }
    }

    pub fn has_qvar(&self) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_generalized() => true,
            Self::FreeVar(fv) => {
                if fv.is_unbound() {
                    false
                } else {
                    fv.crack().has_qvar()
                }
            }
            Self::Type(t) => t.has_qvar(),
            Self::Proj { obj, .. } => obj.has_qvar(),
            Self::Array(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.has_qvar()),
            Self::Set(ts) => ts.iter().any(|t| t.has_qvar()),
            Self::Dict(ts) => ts.iter().any(|(k, v)| k.has_qvar() || v.has_qvar()),
            Self::UnaryOp { val, .. } => val.has_qvar(),
            Self::BinOp { lhs, rhs, .. } => lhs.has_qvar() || rhs.has_qvar(),
            Self::App { args, .. } => args.iter().any(|p| p.has_qvar()),
            Self::Erased(t) => t.has_qvar(),
            _ => false,
        }
    }

    pub fn contains_var(&self, name: &str) -> bool {
        match self {
            Self::FreeVar(fv) if fv.is_unbound() => {
                fv.unbound_name().as_ref().map(|s| &s[..]) == Some(name)
            }
            Self::FreeVar(fv) if fv.is_linked() => fv.crack().contains_var(name),
            Self::Type(t) => t.contains_tvar(name),
            Self::Erased(t) => t.contains_tvar(name),
            Self::Proj { obj, .. } => obj.contains_var(name),
            Self::Array(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.contains_var(name)),
            Self::Set(ts) => ts.iter().any(|t| t.contains_var(name)),
            Self::Dict(ts) => ts
                .iter()
                .any(|(k, v)| k.contains_var(name) || v.contains_var(name)),
            Self::UnaryOp { val, .. } => val.contains_var(name),
            Self::BinOp { lhs, rhs, .. } => lhs.contains_var(name) || rhs.contains_var(name),
            Self::App { args, .. } => args.iter().any(|p| p.contains_var(name)),
            _ => false,
        }
    }

    pub fn is_cachable(&self) -> bool {
        match self {
            Self::FreeVar(_) => false,
            Self::Type(t) => t.is_cachable(),
            Self::Proj { obj, .. } => obj.is_cachable(),
            Self::Array(ts) => ts.iter().all(|t| t.is_cachable()),
            Self::Tuple(ts) => ts.iter().all(|t| t.is_cachable()),
            Self::Set(ts) => ts.iter().all(|t| t.is_cachable()),
            Self::Dict(kv) => kv.iter().all(|(k, v)| k.is_cachable() && v.is_cachable()),
            Self::UnaryOp { val, .. } => val.is_cachable(),
            Self::BinOp { lhs, rhs, .. } => lhs.is_cachable() && rhs.is_cachable(),
            Self::App { args, .. } => args.iter().all(|p| p.is_cachable()),
            Self::Erased(t) => t.is_cachable(),
            _ => true,
        }
    }

    pub fn is_unbound_var(&self) -> bool {
        matches!(self, Self::FreeVar(fv) if fv.is_unbound() || fv.crack().is_unbound_var())
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
            Self::Type(t) => t.has_unbound_var(),
            Self::Proj { obj, .. } => obj.has_unbound_var(),
            Self::Array(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.has_unbound_var()),
            Self::Set(ts) => ts.iter().any(|t| t.has_unbound_var()),
            Self::Dict(kv) => kv
                .iter()
                .any(|(k, v)| k.has_unbound_var() || v.has_unbound_var()),
            Self::UnaryOp { val, .. } => val.has_unbound_var(),
            Self::BinOp { lhs, rhs, .. } => lhs.has_unbound_var() || rhs.has_unbound_var(),
            Self::App { args, .. } => args.iter().any(|p| p.has_unbound_var()),
            Self::Erased(t) => t.has_unbound_var(),
            _ => false,
        }
    }

    pub fn has_no_unbound_var(&self) -> bool {
        !self.has_unbound_var()
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
