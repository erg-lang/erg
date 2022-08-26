use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Range, RangeInclusive, Sub};

use crate::free::{Constraint, FreeKind, FreeTyParam, HasLevel, Level};
use crate::traits::LimitedDisplay;
use crate::ty::Type;
use crate::value::ValueObj;
use crate::Str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OpKind {
    Add,
    Sub,
    Mul,
    Div,
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

/// 型引数
/// データのみ、その評価結果は別に持つ
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
    Mono(Str),
    MonoProj {
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
    MonoQVar(Str),
    PolyQVar {
        name: Str,
        args: Vec<TyParam>,
    },
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
            (Self::Mono(l), Self::Mono(r)) | (Self::MonoQVar(l), Self::MonoQVar(r)) => l == r,
            (
                Self::MonoProj { obj, attr },
                Self::MonoProj {
                    obj: r_obj,
                    attr: r_attr,
                },
            ) => obj == r_obj && attr == r_attr,
            (
                Self::App {
                    name: ln,
                    args: lps,
                }
                | Self::PolyQVar {
                    name: ln,
                    args: lps,
                },
                Self::App {
                    name: rn,
                    args: rps,
                }
                | Self::PolyQVar {
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
            Self::PolyQVar { name, args } => {
                write!(f, "'{}", name)?;
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
            Self::MonoQVar(name) => write!(f, "'{}", name),
            Self::MonoProj { obj, attr } => {
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
        Self::t(Type::int_interval(IntervalOp::RightOpen, r.start, r.end))
    }
}

impl From<Range<&TyParam>> for TyParam {
    fn from(r: Range<&TyParam>) -> Self {
        Self::t(Type::int_interval(
            IntervalOp::RightOpen,
            r.start.clone(),
            r.end.clone(),
        ))
    }
}

impl From<RangeInclusive<TyParam>> for TyParam {
    fn from(r: RangeInclusive<TyParam>) -> Self {
        let (start, end) = r.into_inner();
        Self::t(Type::int_interval(IntervalOp::Closed, start, end))
    }
}

impl From<RangeInclusive<&TyParam>> for TyParam {
    fn from(r: RangeInclusive<&TyParam>) -> Self {
        let (start, end) = r.into_inner();
        Self::t(Type::int_interval(
            IntervalOp::Closed,
            start.clone(),
            end.clone(),
        ))
    }
}

impl<V: Into<ValueObj>> From<V> for TyParam {
    fn from(v: V) -> Self {
        Self::Value(v.into())
    }
}

impl HasLevel for TyParam {
    fn level(&self) -> Option<Level> {
        match self {
            Self::Type(t) => t.level(),
            Self::FreeVar(fv) => fv.level(),
            Self::UnaryOp { val, .. } => val.level(),
            Self::BinOp { lhs, rhs, .. } => lhs.level().and_then(|l| rhs.level().map(|r| l.max(r))),
            _ => None,
        }
    }

    fn update_level(&self, level: Level) {
        match self {
            Self::FreeVar(fv) => fv.update_level(level),
            Self::UnaryOp { val, .. } => val.update_level(level),
            Self::BinOp { lhs, rhs, .. } => {
                lhs.update_level(level);
                rhs.update_level(level);
            }
            Self::App { args, .. } | Self::PolyQVar { args, .. } => {
                for arg in args.iter() {
                    arg.update_level(level);
                }
            }
            _ => {}
        }
    }

    fn lift(&self) {
        match self {
            Self::FreeVar(fv) => fv.lift(),
            Self::UnaryOp { val, .. } => val.lift(),
            Self::BinOp { lhs, rhs, .. } => {
                lhs.lift();
                rhs.lift();
            }
            Self::App { args, .. } | Self::PolyQVar { args, .. } => {
                for arg in args.iter() {
                    arg.lift();
                }
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

    pub fn mono_q<S: Into<Str>>(name: S) -> Self {
        Self::MonoQVar(name.into())
    }

    pub fn mono_proj<S: Into<Str>>(obj: TyParam, attr: S) -> Self {
        Self::MonoProj {
            obj: Box::new(obj),
            attr: attr.into(),
        }
    }

    // TODO: polymorphic type
    pub fn array_t(t: Str, len: TyParam) -> Self {
        Self::Array(vec![TyParam::t(Type::mono(t)), len])
    }

    pub fn free_var(level: usize, t: Type) -> Self {
        let constraint = Constraint::type_of(t);
        Self::FreeVar(FreeTyParam::new_unbound(level, constraint))
    }

    pub fn named_free_var(name: Str, level: usize, t: Type) -> Self {
        let constraint = Constraint::type_of(t);
        Self::FreeVar(FreeTyParam::new_named_unbound(name, level, constraint))
    }

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

    pub fn name(&self) -> Option<Str> {
        match self {
            Self::Type(t) => Some(t.name()),
            Self::Mono(name) => Some(name.clone()),
            Self::MonoQVar(name) => Some(name.clone()),
            _ => None,
        }
    }

    pub fn tvar_name(&self) -> Option<Str> {
        match self {
            Self::Type(t) => t.tvar_name(),
            Self::FreeVar(fv) => fv.unbound_name(),
            Self::MonoQVar(name) => Some(name.clone()),
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
                p.cheap_cmp(&*fv.crack()),
            (Self::FreeVar{ .. } | Self::Erased(_), Self::FreeVar{ .. } | Self::Erased(_))
            /* if v.is_unbound() */ => Some(Any),
            (Self::App{ name, args }, Self::App{ name: rname, args: rargs })
            | (Self::PolyQVar{ name, args }, Self::PolyQVar{ name: rname, args: rargs }) =>
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

    pub fn has_qvar(&self) -> bool {
        match self {
            Self::MonoQVar(_) | Self::PolyQVar { .. } => true,
            Self::FreeVar(fv) => {
                if fv.is_unbound() {
                    true
                } else {
                    fv.crack().has_qvar()
                }
            }
            Self::Type(t) => t.has_qvar(),
            Self::MonoProj { obj, .. } => obj.has_qvar(),
            Self::Array(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.has_qvar()),
            Self::UnaryOp { val, .. } => val.has_qvar(),
            Self::BinOp { lhs, rhs, .. } => lhs.has_qvar() || rhs.has_qvar(),
            Self::App { args, .. } => args.iter().any(|p| p.has_qvar()),
            Self::Erased(t) => t.has_qvar(),
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
            Self::Type(t) => t.has_unbound_var(),
            Self::MonoProj { obj, .. } => obj.has_unbound_var(),
            Self::Array(ts) | Self::Tuple(ts) => ts.iter().any(|t| t.has_unbound_var()),
            Self::UnaryOp { val, .. } => val.has_unbound_var(),
            Self::BinOp { lhs, rhs, .. } => lhs.has_unbound_var() || rhs.has_unbound_var(),
            Self::App { args, .. } | Self::PolyQVar { args, .. } => {
                args.iter().any(|p| p.has_unbound_var())
            }
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
            Self::Erased(_) | Self::MonoQVar(_) => false,
            Self::FreeVar(fv) => !fv.is_unbound(), // != fv.is_linked(),
            _ => true,
        }
    }

    pub fn has_lower_bound(&self) -> bool {
        match self {
            Self::Erased(_) | Self::MonoQVar(_) => false,
            Self::FreeVar(fv) => !fv.is_unbound(),
            _ => true,
        }
    }

    pub fn update_constraint(&self, new_constraint: Constraint) {
        match self {
            Self::Type(t) => t.update_constraint(new_constraint),
            _ => {}
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
