//! defines `Type` (type kind).
//!
//! Type(コンパイラ等で使われる「型」を表現する)を定義する
use std::cell::{Ref, RefMut};
use std::cmp::Ordering;
use std::fmt;
use std::mem;
use std::ops::{Add, Sub, Mul, Div, Neg, Range, RangeInclusive};

use crate::{Str, fmt_vec, fmt_vec_split_with, fmt_set_split_with, set};
use crate::codeobj::CodeObj;
use crate::value::ValueObj;
use crate::dict::Dict;
use crate::rccell::RcCell;
use crate::set::Set;
use crate::traits::HasType;
use crate::ty::ValueObj::{NegInf, Inf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OpKind {
    Add,
    Sub,
    Mul,
    Div,
    Pos,
    Neg,
    Invert,
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    Ne,
    Mutate,
}

impl fmt::Display for OpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Pos => write!(f, "+"),
            Self::Neg => write!(f, "-"),
            Self::Invert => write!(f, "~"),
            Self::Gt => write!(f, ">"),
            Self::Lt => write!(f, "<"),
            Self::Ge => write!(f, ">="),
            Self::Le => write!(f, "<="),
            Self::Eq => write!(f, "=="),
            Self::Ne => write!(f, "!="),
            Self::Mutate => write!(f, "!"),
        }
    }
}

pub type Level = usize;
pub type Id = usize;

thread_local! {
    static UNBOUND_ID: RcCell<usize> = RcCell::new(0);
    static REFINEMENT_VAR_ID: RcCell<usize> = RcCell::new(0);
}

pub fn fresh_varname() -> String {
    REFINEMENT_VAR_ID.with(|id| {
        *id.borrow_mut() += 1;
        let i = id.borrow().clone();
        format!("%v{i}")
    })
}

pub fn fresh_param_name() -> String {
    REFINEMENT_VAR_ID.with(|id| {
        *id.borrow_mut() += 1;
        let i = id.borrow().clone();
        format!("%p{i}")
    })
}

pub trait HasLevel {
    fn level(&self) -> Option<Level>;
    fn update_level(&self, level: Level);
    fn lift(&self);
}

// REVIEW: TyBoundと微妙に役割が被っている
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constraint {
    SubtypeOf(Type),
    TypeOf(Type),
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SubtypeOf(ty) => write!(f, "<: {}", ty),
            Self::TypeOf(ty) => write!(f, ": {}", ty),
        }
    }
}

impl Constraint {
    pub fn typ(&self) -> Option<&Type> {
        match self {
            Self::TypeOf(ty) => Some(ty),
            _ => None,
        }
    }

    pub fn super_type(&self) -> Option<&Type> {
        match self {
            Self::SubtypeOf(ty) => Some(ty),
            _ => None,
        }
    }

    pub fn super_type_mut(&mut self) -> Option<&mut Type> {
        match self {
            Self::SubtypeOf(ty) => Some(ty),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FreeKind<T> {
    Linked(T),
    Unbound{ id: Id, lev: Level, constraint: Constraint },
    NamedUnbound{ name: Str, lev: Level, constraint: Constraint },
}

impl<T: fmt::Display> fmt::Display for FreeKind<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Linked(t) => write!(f, "({t})"),
            Self::NamedUnbound{ name, lev, constraint } => write!(f, "?{name}({constraint})[{lev}]"),
            Self::Unbound{ id, lev, constraint }=> write!(f, "?{id}({constraint})[{lev}]"),
        }
    }
}

impl<T> FreeKind<T> {
    pub const fn unbound(id: Id, lev: Level, constraint: Constraint) -> Self {
        Self::Unbound{ id, lev, constraint }
    }

    pub const fn named_unbound(name: Str, lev: Level, constraint: Constraint) -> Self {
        Self::NamedUnbound{ name, lev, constraint }
    }

    pub const fn constraint(&self) -> Option<&Constraint> {
        match self {
            Self::Unbound{ constraint, .. }
            | Self::NamedUnbound{ constraint, .. } => Some(constraint),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Free<T>(RcCell<FreeKind<T>>);

impl<T: fmt::Display> fmt::Display for Free<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.borrow())
    }
}

impl<T: Clone + HasLevel> Free<T> {
    pub fn new(f: FreeKind<T>) -> Self { Self(RcCell::new(f)) }

    pub fn new_unbound(level: Level, constraint: Constraint) -> Self {
        UNBOUND_ID.with(|id| {
            *id.borrow_mut() += 1;
            Self(RcCell::new(FreeKind::unbound(*id.borrow(), level, constraint)))
        })
    }

    pub fn new_named_unbound(name: Str, level: Level, constraint: Constraint) -> Self {
        Self(RcCell::new(FreeKind::named_unbound(name, level, constraint)))
    }

    pub fn new_linked(t: T) -> Self {
        Self(RcCell::new(FreeKind::Linked(t)))
    }

    pub fn link(&self, to: &T) {
        *self.0.borrow_mut() = FreeKind::Linked(to.clone());
    }

    pub fn update_level(&self, level: Level) {
        match &mut *self.0.borrow_mut() {
            FreeKind::Unbound{ lev, .. }
            | FreeKind::NamedUnbound{ lev, .. } if level < *lev => { *lev = level; },
            FreeKind::Linked(t) => { t.update_level(level); },
            _ => {}
        }
    }

    pub fn lift(&self) {
        match &mut *self.0.borrow_mut() {
            FreeKind::Unbound{ lev, .. }
            | FreeKind::NamedUnbound{ lev, .. } => { *lev += 1; },
            FreeKind::Linked(t) => { if let Some(lev) = t.level() { t.update_level(lev+1); } },
        }
    }

    pub fn level(&self) -> Option<Level> {
        match &*self.0.borrow() {
            FreeKind::Unbound{ lev, .. }
            | FreeKind::NamedUnbound{ lev, .. } => Some(*lev),
            FreeKind::Linked(t) => t.level(),
        }
    }

    pub fn update_constraint(&self, new_constraint: Constraint) {
        match &mut *self.0.borrow_mut() {
            FreeKind::Unbound{ constraint, .. }
            | FreeKind::NamedUnbound{ constraint, .. } => {
                *constraint = new_constraint;
            },
            _ => {}
        }
    }

    pub fn unwrap(self) -> T {
        match self.0.clone_inner() {
            FreeKind::Linked(t) => t,
            FreeKind::Unbound{ .. }
            | FreeKind::NamedUnbound{ .. } => panic!("the value is unbounded"),
        }
    }

    /// returns linked type (panic if self is unbounded)
    /// NOTE: check by `.is_linked` before call
    pub fn crack(&self) -> Ref<'_, T> {
        Ref::map(self.0.borrow(), |f| match f {
            FreeKind::Linked(t) => t,
            FreeKind::Unbound{ .. }
            | FreeKind::NamedUnbound{ .. } => panic!("the value is unbounded"),
        })
    }

    pub fn type_of(&self) -> Option<Type> {
        self.0.borrow().constraint()
            .and_then(|c| c.typ().map(|t| t.clone()))
    }

    pub fn subtype_of(&self) -> Option<Type> {
        self.0.borrow().constraint()
            .and_then(|c| c.super_type().map(|t| t.clone()))
    }

    pub fn is_unbound(&self) -> bool {
        matches!(&*self.0.borrow(), FreeKind::Unbound{ .. } | FreeKind::NamedUnbound{ .. })
    }

    pub fn is_linked(&self) -> bool {
        matches!(&*self.0.borrow(), FreeKind::Linked(_))
    }

    pub fn unbound_name(&self) -> Option<Str> {
        match &*self.0.borrow() {
            FreeKind::NamedUnbound{ name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    pub fn borrow(&self) -> Ref<'_, FreeKind<T>> { self.0.borrow() }
    pub fn borrow_mut(&self) -> RefMut<'_, FreeKind<T>> { self.0.borrow_mut() }
}

impl Free<TyParam> {
    pub fn map<F>(&self, f: F) where F: Fn(TyParam) -> TyParam {
        match &mut *self.0.borrow_mut() {
            FreeKind::Unbound{ .. }
            | FreeKind::NamedUnbound{ .. } => panic!("the value is unbounded"),
            FreeKind::Linked(t) => { *t = f(mem::take(t)); },
        }
    }
}

pub type FreeTyVar = Free<Type>;
pub type FreeTyParam = Free<TyParam>;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstObj {
    Value(ValueObj),
    MutValue(RcCell<ValueObj>),
    Subr(ConstSubr),
    Record(Dict<Str, ConstObj>),
    Type(Box<Type>),
}

impl fmt::Display for ConstObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstObj::Value(v) => write!(f, "{v}"),
            ConstObj::MutValue(v) => write!(f, "!{v}"),
            ConstObj::Subr(s) => write!(f, "{s:?}"),
            ConstObj::Record(r) => write!(f, "{r}"),
            ConstObj::Type(t) => write!(f, "{t}"),
        }
    }
}

impl ConstObj {
    pub fn t(t: Type) -> Self { Self::Type(Box::new(t)) }

    pub fn try_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Value(l), Self::Value(r)) => l.try_cmp(r),
            (Self::MutValue(l), Self::MutValue(r)) => l.borrow().try_cmp(&r.borrow()),
            (Self::Value(l), Self::MutValue(r)) => l.try_cmp(&r.borrow()),
            (Self::MutValue(l), Self::Value(r)) => l.borrow().try_cmp(r),
            // TODO: cmp with str
            (_s, _o) => None,
        }
    }
}

/// 型引数
/// データのみ、その評価結果は別に持つ
/// __Info__: 連携型パラメータがあるので、比較には`rec_eq`を使うこと
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyParam {
    ConstObj(ConstObj),
    Type(Box<Type>),
    Array(Vec<TyParam>),
    Tuple(Vec<TyParam>),
    Mono(Str),
    MonoProj{ obj: Box<TyParam>, attr: Str },
    App{ name: Str, args: Vec<TyParam> },
    UnaryOp{ op: OpKind, val: Box<TyParam> },
    BinOp{ op: OpKind, lhs: Box<TyParam>, rhs: Box<TyParam> },
    Erased(Box<Type>),
    MonoQVar(Str),
    PolyQVar{ name: Str, args: Vec<TyParam> },
    FreeVar(FreeTyParam),
    Failure,
}

impl fmt::Display for TyParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConstObj(c) => write!(f, "{c}"),
            Self::Type(t) => write!(f, "{t}"),
            Self::Mono(c) => write!(f, "{c}"),
            Self::MonoProj{ obj, attr } => write!(f, "{obj}.{attr}"),
            Self::Array(a) => write!(f, "[{}]", fmt_vec(a)),
            Self::Tuple(t) => write!(f, "({})", fmt_vec(t)),
            Self::App{ name, args } => write!(f, "{name}({})", fmt_vec(args)),
            Self::MonoQVar(name) => write!(f, "'{name}"),
            Self::PolyQVar{ name, args } => write!(f, "'{name}({})", fmt_vec(args)),
            Self::FreeVar(fv) => write!(f, "{fv}"),
            Self::UnaryOp{ op, val } => write!(f, "{op}{val}"),
            Self::BinOp{ op, lhs, rhs } => write!(f, "{lhs} {op} {rhs}"),
            Self::Erased(t) => write!(f, "_: {t}"),
            Self::Failure => write!(f, "<failure>"),
        }
    }
}

impl Default for TyParam {
    #[inline]
    fn default() -> Self { Self::Failure }
}

impl Add for TyParam {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output { Self::bin(OpKind::Add, self, rhs)}
}

impl Sub for TyParam {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output { Self::bin(OpKind::Sub, self, rhs) }
}

impl Mul for TyParam {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output { Self::bin(OpKind::Mul, self, rhs) }
}

impl Div for TyParam {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output { Self::bin(OpKind::Div, self, rhs) }
}

impl Neg for TyParam {
    type Output = Self;
    fn neg(self) -> Self::Output { Self::unary(OpKind::Neg, self) }
}

impl From<Range<TyParam>> for TyParam {
    fn from(r: Range<TyParam>) -> Self {
        Self::t(Type::int_interval(IntervalOp::RightOpen, r.start, r.end))
    }
}

impl From<Range<&TyParam>> for TyParam {
    fn from(r: Range<&TyParam>) -> Self {
        Self::t(Type::int_interval(IntervalOp::RightOpen, r.start.clone(), r.end.clone()))
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
        Self::t(Type::int_interval(IntervalOp::Closed, start.clone(), end.clone()))
    }
}

impl<V: Into<ValueObj>> From<V> for TyParam {
    fn from(v: V) -> Self { Self::ConstObj(ConstObj::Value(v.into())) }
}

impl HasLevel for TyParam {
    fn level(&self) -> Option<Level> {
        match self {
            Self::Type(t) => t.level(),
            Self::FreeVar(fv) => fv.level(),
            Self::UnaryOp{ val, .. } => val.level(),
            Self::BinOp{ lhs, rhs, .. } => lhs.level().and_then(|l| rhs.level().map(|r| l.max(r))),
            _ => None,
        }
    }

    fn update_level(&self, level: Level) {
        match self {
            Self::FreeVar(fv) => fv.update_level(level),
            Self::UnaryOp{ val, .. } => val.update_level(level),
            Self::BinOp{ lhs, rhs, .. } => {
                lhs.update_level(level);
                rhs.update_level(level);
            },
            Self::App{ args, .. }
            | Self::PolyQVar{ args, .. } => {
                for arg in args.iter() {
                    arg.update_level(level);
                }
            },
            _ => {}
        }
    }

    fn lift(&self) {
        match self {
            Self::FreeVar(fv) => fv.lift(),
            Self::UnaryOp{ val, .. } => val.lift(),
            Self::BinOp{ lhs, rhs, .. } => {
                lhs.lift();
                rhs.lift();
            },
            Self::App{ args, .. }
            | Self::PolyQVar{ args, .. } => {
                for arg in args.iter() {
                    arg.lift();
                }
            },
            _ => {}
        }
    }
}

impl TyParam {
    pub fn t(t: Type) -> Self { Self::Type(Box::new(t)) }

    pub fn mono<S: Into<Str>>(name: S) -> Self { Self::Mono(name.into()) }

    pub fn mono_q<S: Into<Str>>(name: S) -> Self { Self::MonoQVar(name.into()) }

    pub fn mono_proj<S: Into<Str>>(obj: TyParam, attr: S) -> Self {
        Self::MonoProj{ obj: Box::new(obj), attr: attr.into() }
    }

    // TODO: polymorphic type
    pub fn array_t(t: Str, len: TyParam) -> Self { Self::Array(vec![TyParam::t(Type::mono(t)), len]) }

    pub fn free_var(level: usize, t: Type) -> Self {
        Self::FreeVar(FreeTyParam::new_unbound(level, Constraint::TypeOf(t)))
    }

    pub fn named_free_var(name: Str, level: usize, t: Type) -> Self {
        Self::FreeVar(FreeTyParam::new_named_unbound(name, level, Constraint::TypeOf(t)))
    }

    #[inline]
    pub fn value<V: Into<ValueObj>>(v: V) -> Self { Self::ConstObj(ConstObj::Value(v.into())) }

    #[inline]
    pub fn cons<C: Into<ConstObj>>(l: C) -> Self { Self::ConstObj(l.into()) }

    #[inline]
    pub fn unary(op: OpKind, val: TyParam) -> Self { Self::UnaryOp{ op, val: Box::new(val) } }

    #[inline]
    pub fn mutate(self) -> Self { Self::unary(OpKind::Mutate, self) }

    #[inline]
    pub fn bin(op: OpKind, lhs: TyParam, rhs: TyParam) -> Self {
        Self::BinOp{ op, lhs: Box::new(lhs), rhs: Box::new(rhs) }
    }

    pub fn app(name: &'static str, args: Vec<TyParam>) -> Self {
        Self::App{ name: Str::ever(name), args }
    }

    #[inline]
    pub fn erased(t: Type) -> Self { Self::Erased(Box::new(t)) }

    // if self: Ratio, Succ(self) => self+ε
    pub fn succ(self) -> Self { Self::app("Succ", vec![self]) }

    // if self: Ratio, Pred(self) => self-ε
    pub fn pred(self) -> Self { Self::app("Pred", vec![self]) }

    /// 型変数の内容を考慮した再帰的(Recursive)比較を行う
    pub fn rec_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Type(l), Self::Type(r)) => l.rec_eq(r),
            (Self::FreeVar(fv), o)
            | (o, Self::FreeVar(fv)) => match &*fv.borrow() {
                FreeKind::Linked(tp) => tp.rec_eq(o),
                _ => self == o,
            },
            (
                Self::MonoProj{ obj: lo, attr: la },
                Self::MonoProj{ obj: ro, attr: ra}
            ) => lo.rec_eq(ro) && la == ra,
            (Self::Array(l), Self::Array(r))
            | (Self::Tuple(l), Self::Tuple(r)) =>
                l.iter().zip(r.iter()).all(|(l, r)| l.rec_eq(r)),
            (
                Self::App{ name: ln, args: lps },
                Self::App{ name: rn, args: rps}
            ) => ln == rn && lps.iter().zip(rps.iter()).all(|(l, r)| l.rec_eq(r)),
            (
                Self::UnaryOp{ op: lop, val: lv },
                Self::UnaryOp{ op: rop, val: rv }
            ) => lop == rop && lv.rec_eq(rv),
            (
                Self::BinOp{ op: lop, lhs: ll, rhs: lr },
                Self::BinOp{ op: rop, lhs: rl, rhs: rr }
            ) => lop == rop && ll.rec_eq(rl) && lr.rec_eq(rr),
            (Self::Erased(l), Self::Erased(r)) => l.rec_eq(r),
            _ => self == other,
        }
    }

    // 定数の比較など環境が必要な場合はContext::try_cmpを使う
    pub fn cheap_cmp(&self, r: &TyParam) -> Option<TyParamOrdering> {
        match (self, r) {
            (Self::Type(l), Self::Type(r)) =>
                if l.rec_eq(r) { Some(TyParamOrdering::Equal) } else { Some(TyParamOrdering::NotEqual) },
            (Self::ConstObj(l), Self::ConstObj(r)) =>
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

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::FreeVar(fv) =>
                if fv.is_unbound() { true } else { fv.crack().has_unbound_var() },
            Self::Type(t) => t.has_unbound_var(),
            Self::MonoProj{ obj, .. } => obj.has_unbound_var(),
            Self::Array(ts)
            | Self::Tuple(ts) => ts.iter().any(|t| t.has_unbound_var()),
            Self::UnaryOp{ val, .. } => val.has_unbound_var(),
            Self::BinOp{ lhs, rhs, .. } =>
                lhs.has_unbound_var() || rhs.has_unbound_var(),
            Self::App{ args, .. }
            | Self::PolyQVar{ args, .. } => args.iter().any(|p| p.has_unbound_var()),
            Self::Erased(t) => t.has_unbound_var(),
            _ => false,
        }
    }

    pub fn has_no_unbound_var(&self) -> bool { !self.has_unbound_var() }

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TyParamOrdering {
    Less,
    Equal,
    Greater,
    LessEqual, // Less or Equal
    NotEqual, // Less or Greater
    GreaterEqual, // Greater or Equal
    Any,
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

impl TyParamOrdering {
    pub const fn is_lt(&self) -> bool { matches!(self, Less | LessEqual | Any) }
    pub const fn is_le(&self) -> bool { matches!(self, Less | Equal | LessEqual | Any) }
    pub const fn is_gt(&self) -> bool { matches!(self, Greater | GreaterEqual | Any) }
    pub const fn is_ge(&self) -> bool { matches!(self, Greater | Equal | GreaterEqual | Any) }
    pub const fn is_eq(&self) -> bool { matches!(self, Equal | Any) }
    pub const fn is_ne(&self) -> bool { matches!(self, Less | Greater | NotEqual | Any) }
    pub const fn reverse(&self) -> Self {
        match self {
            Less => Greater,
            Greater => Less,
            LessEqual => GreaterEqual,
            GreaterEqual => LessEqual,
            Equal => NotEqual,
            NotEqual => Equal,
            Any => Any,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyBound {
    // e.g. A <: Add => Subtype{sub: A, sup: Add}, A <: {a: Int} => Subtype{sub: A, sup: {a: Int}}
    Subtype{ sub: Type, sup: Type },
    // TODO: Supertype{ sup: Type, sub: Type },
    // TyParam::MonoQuantVarに型の情報が含まれているので、boundsからは除去される
    // e.g. N: Nat => Instance{name: N, t: Nat}
    Instance{ name: Str, t: Type },
}

impl fmt::Display for TyBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Subtype{ sub, sup } => write!(f, "{sub} <: {sup}"),
            Self::Instance{ name, t } => write!(f, "'{name}: {t}"),
        }
    }
}

impl HasLevel for TyBound {
    fn level(&self) -> Option<usize> {
        todo!()
    }

    fn update_level(&self, level: usize) {
        match self {
            Self::Subtype{ sub, sup } => {
                sub.update_level(level);
                sup.update_level(level);
            }
            Self::Instance{ t, .. } => { t.update_level(level); },
        }
    }

    fn lift(&self) {
        match self {
            Self::Subtype{ sub, sup } => {
                sub.lift();
                sup.lift();
            }
            Self::Instance{ t, .. } => { t.lift(); },
        }
    }
}

impl TyBound {
    pub const fn subtype(sub: Type, sup: Type) -> Self { Self::Subtype{ sub, sup } }

    pub const fn static_instance(name: &'static str, t: Type) -> Self {
        Self::Instance{ name: Str::ever(name), t }
    }

    pub const fn instance(name: Str, t: Type) -> Self { Self::Instance{ name, t } }

    pub fn mentions_as_instance(&self, name: &str) -> bool {
        matches!(self, Self::Instance{ name: n, .. } if &n[..] == name)
    }

    pub fn mentions_as_subtype(&self, name: &str) -> bool {
        matches!(self, Self::Subtype{ sub, .. } if sub.name() == name)
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::Subtype{ sub, sup } => sub.has_unbound_var() || sup.has_unbound_var(),
            Self::Instance{ t, .. } => t.has_unbound_var(),
        }
    }

    pub const fn t(&self) -> &Type {
        match self {
            Self::Subtype{ sup, .. } => sup,
            Self::Instance{ t, .. } => t,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Predicate {
    Value(ValueObj), // True/False
    Const(Str),
    /// i == 0 => Eq{ lhs: "i", rhs: 0 }
    Equal{ lhs: Str, rhs: TyParam },
    /// i > 0 => i >= 0+ε => GreaterEqual{ lhs: "i", rhs: 0+ε }
    GreaterEqual{ lhs: Str, rhs: TyParam },
    LessEqual{ lhs: Str, rhs: TyParam },
    NotEqual{ lhs: Str, rhs: TyParam },
    Or(Box<Predicate>, Box<Predicate>),
    And(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>, Box<Predicate>),
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(v) => write!(f, "{v}"),
            Self::Const(c) => write!(f, "{c}"),
            Self::Equal{ lhs, rhs } => write!(f, "{lhs} == {rhs}"),
            Self::GreaterEqual{ lhs, rhs } => write!(f, "{lhs} >= {rhs}"),
            Self::LessEqual{ lhs, rhs } => write!(f, "{lhs} <= {rhs}"),
            Self::NotEqual{ lhs, rhs } => write!(f, "{lhs} != {rhs}"),
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
            Self::Equal{ rhs, .. }
            | Self::GreaterEqual{ rhs, .. }
            | Self::LessEqual{ rhs, .. }
            | Self::NotEqual{ rhs, .. } => rhs.level(),
            Self::And(_lhs, _rhs)
            | Self::Or(_lhs, _rhs)
            | Self::Not(_lhs, _rhs) => todo!(),
        }
    }

    fn update_level(&self, level: usize) {
        match self {
            Self::Value(_) | Self::Const(_) => {},
            Self::Equal{ rhs, .. }
            | Self::GreaterEqual{ rhs, .. }
            | Self::LessEqual{ rhs, .. }
            | Self::NotEqual{ rhs, .. } => {
                rhs.update_level(level);
            },
            Self::And(lhs, rhs)
            | Self::Or(lhs, rhs)
            | Self::Not(lhs, rhs) => {
                lhs.update_level(level);
                rhs.update_level(level);
            },
        }
    }

    fn lift(&self) {
        match self {
            Self::Value(_) | Self::Const(_) => {},
            Self::Equal{ rhs, .. }
            | Self::GreaterEqual{ rhs, .. }
            | Self::LessEqual{ rhs, .. }
            | Self::NotEqual{ rhs, .. } => {
                rhs.lift();
            },
            Self::And(lhs, rhs)
            | Self::Or(lhs, rhs)
            | Self::Not(lhs, rhs) => {
                lhs.lift();
                rhs.lift();
            },
        }
    }
}

impl Predicate {
    pub const fn eq(lhs: Str, rhs: TyParam) -> Self { Self::Equal{ lhs, rhs } }
    pub const fn ne(lhs: Str, rhs: TyParam) -> Self { Self::NotEqual{ lhs, rhs } }
    /// >=
    pub const fn ge(lhs: Str, rhs: TyParam) -> Self { Self::GreaterEqual{ lhs, rhs } }
    /// <=
    pub const fn le(lhs: Str, rhs: TyParam) -> Self { Self::LessEqual{ lhs, rhs } }

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
            Self::Equal{ lhs, .. }
            | Self::LessEqual{ lhs, .. }
            | Self::GreaterEqual{ lhs, .. }
            | Self::NotEqual{ lhs, .. } => Some(&lhs[..]),
            Self::And(lhs, rhs)
            | Self::Or(lhs, rhs)
            | Self::Not(lhs, rhs) => {
                let l = lhs.subject();
                let r = rhs.subject();
                if l != r { todo!() }
                else { l }
            },
            _ => None,
        }
    }

    pub fn change_subject_name(self, name: Str) -> Self {
        match self {
            Self::Equal{ rhs, .. } => Self::eq(name, rhs),
            Self::GreaterEqual{ rhs, .. } => Self::ge(name, rhs),
            Self::LessEqual{ rhs, .. } => Self::le(name, rhs),
            Self::NotEqual{ rhs, .. } => Self::ne(name, rhs),
            Self::And(lhs, rhs) => Self::and(lhs.change_subject_name(name.clone()), rhs.change_subject_name(name)),
            Self::Or(lhs, rhs) => Self::or(lhs.change_subject_name(name.clone()), rhs.change_subject_name(name)),
            Self::Not(lhs, rhs) => Self::not(lhs.change_subject_name(name.clone()), rhs.change_subject_name(name)),
            _ => self,
        }
    }

    pub fn mentions(&self, name: &str) -> bool {
        match self {
            Self::Const(n) => &n[..] == name,
            Self::Equal{ lhs, .. }
            | Self::LessEqual{ lhs, .. }
            | Self::GreaterEqual{ lhs, .. }
            | Self::NotEqual{ lhs, .. } => &lhs[..] == name,
            Self::And(lhs, rhs)
            | Self::Or(lhs, rhs)
            | Self::Not(lhs, rhs) => lhs.mentions(name) || rhs.mentions(name),
            _ => false,
        }
    }

    pub fn can_be_false(&self) -> bool {
        match self {
            Self::Value(l) => matches!(l, ValueObj::False),
            Self::Const(_) => todo!(),
            Self::Or(lhs, rhs) => lhs.can_be_false() || rhs.can_be_false(),
            Self::And(lhs, rhs) => lhs.can_be_false() && rhs.can_be_false(),
            Self::Not(lhs, rhs) => lhs.can_be_false() && !rhs.can_be_false(),
            _ => true,
        }
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::Value(_) => false,
            Self::Const(_) => false,
            Self::Equal{ rhs, .. }
            |Self::GreaterEqual{ rhs, .. }
            | Self::LessEqual{ rhs, .. }
            | Self::NotEqual{ rhs, .. } => rhs.has_unbound_var(),
            Self::Or(lhs, rhs)
            | Self::And(lhs, rhs)
            | Self::Not(lhs, rhs) => lhs.has_unbound_var() || rhs.has_unbound_var(),
        }
    }

    pub fn min_max<'a>(&'a self, min: Option<&'a TyParam>, max: Option<&'a TyParam>) -> (Option<&'a TyParam>, Option<&'a TyParam>) {
        match self {
            Predicate::Equal{ rhs: _, .. } => todo!(),
            // {I | I <= 1; I <= 2}
            Predicate::LessEqual { rhs, .. } => {
                (min, max.map(|l: &TyParam| match l.cheap_cmp(rhs) {
                    Some(c) if c.is_ge() => l,
                    Some(_) => rhs,
                    _ => l,
                }).or(Some(rhs)))
            },
            // {I | I >= 1; I >= 2}
            Predicate::GreaterEqual { rhs, .. } => {
                (min.map(|l: &TyParam| match l.cheap_cmp(rhs) {
                    Some(c) if c.is_le() => l,
                    Some(_) => rhs,
                    _ => l,
                }).or(Some(rhs)), max)
            },
            Predicate::And(_l, _r) => todo!(),
            _ => todo!(),
        }
    }

    pub fn typarams(&self) -> Vec<&TyParam> {
        match self {
            Self::Value(_) | Self::Const(_) => vec![],
            Self::Equal{ rhs, .. }
            | Self::GreaterEqual{ rhs, .. }
            | Self::LessEqual{ rhs, .. }
            | Self::NotEqual{ rhs, .. } => vec![rhs],
            Self::And(lhs, rhs)
            | Self::Or(lhs, rhs)
            | Self::Not(lhs, rhs) =>
                lhs.typarams().into_iter().chain(rhs.typarams()).collect(),
        }
    }

    pub fn typarams_mut(&mut self) -> Vec<&mut TyParam> {
        match self {
            Self::Value(_) | Self::Const(_) => vec![],
            Self::Equal{ rhs, .. }
            | Self::GreaterEqual{ rhs, .. }
            | Self::LessEqual{ rhs, .. }
            | Self::NotEqual{ rhs, .. } => vec![rhs],
            Self::And(lhs, rhs)
            | Self::Or(lhs, rhs)
            | Self::Not(lhs, rhs) =>
                lhs.typarams_mut().into_iter().chain(rhs.typarams_mut()).collect(),
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
    pub const fn is_closed(&self) -> bool { matches!(self, Self::Closed) }
    pub const fn is_left_open(&self) -> bool { matches!(self, Self::LeftOpen | Self::Open) }
    pub const fn is_right_open(&self) -> bool { matches!(self, Self::RightOpen | Self::Open) }
    pub const fn is_open(&self) -> bool { matches!(self, Self::Open) }
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
    pub const fn new(name: Option<Str>, ty: Type) -> Self { Self { name, ty } }

    pub const fn anonymous(ty: Type) -> Self { Self::new(None, ty) }
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
        if self.default_params.is_empty() {
            write!(
                f,
                "{}({}) {} {}",
                self.kind.prefix(),
                fmt_vec(&self.non_default_params),
                self.kind.arrow(),
                self.return_t,
            )
        } else {
            write!(
                f,
                "{}({} |= {}) {} {}",
                self.kind.prefix(),
                fmt_vec(&self.non_default_params),
                fmt_vec(&self.default_params),
                self.kind.arrow(),
                self.return_t,
            )
        }
    }
}

impl SubrType {
    pub fn new(kind: SubrKind, non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, return_t: Type) -> Self {
        Self{ kind, non_default_params, default_params, return_t: Box::new(return_t) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefineKind {
    Interval{ min: TyParam, max: TyParam }, // e.g. {I: Int | I >= 2; I <= 10} 2..10
    Enum(Set<TyParam>), // e.g. {I: Int | I == 1 or I == 2} {1, 2}
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
        write!(f, "{{{}: {} | {}}}", self.var, self.t, fmt_set_split_with(&self.preds, "; "))
    }
}

impl RefinementType {
    pub fn new(var: Str, t: Type, preds: Set<Predicate>) -> Self {
        Self { var, t: Box::new(t), preds }
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
        write!(f, "|{}| {}", &self.bounds, self.unbound_callable)
    }
}

impl QuantifiedType {
    pub fn new(unbound_callable: Type, bounds: Set<TyBound>) -> Self {
        Self { unbound_callable: Box::new(unbound_callable), bounds }
    }
}

type SelfType = Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubrKind {
    Func,
    Proc,
    FuncMethod(Box<SelfType>),
    ProcMethod{ before: Box<SelfType>, after: Option<Box<SelfType>> },
}

impl HasLevel for SubrKind {
    fn level(&self) -> Option<Level> { todo!() }

    fn update_level(&self, level: usize) {
        match self {
            Self::FuncMethod(t) => t.update_level(level),
            Self::ProcMethod{ before, after } => {
                before.update_level(level);
                after.as_ref().map(|t| { t.update_level(level); });
            }
            _ => {}
        }
    }

    fn lift(&self) {
        match self {
            Self::FuncMethod(t) => t.lift(),
            Self::ProcMethod{ before, after } => {
                before.lift();
                after.as_ref().map(|t| { t.lift(); });
            }
            _ => {}
        }
    }
}

impl SubrKind {
    pub fn fn_met(t: SelfType) -> Self { SubrKind::FuncMethod(Box::new(t)) }

    pub fn pr_met(before: SelfType, after: Option<SelfType>) -> Self {
        Self::ProcMethod{ before: Box::new(before), after: after.map(Box::new) }
    }

    pub const fn arrow(&self) -> &str {
        match self {
            Self::Func | Self::FuncMethod(_) => "->",
            Self::Proc | Self::ProcMethod{ .. } => "=>",
        }
    }

    pub const fn inner_len(&self) -> usize {
        match self {
            Self::Func | Self::Proc => 0,
            Self::FuncMethod(_) | Self::ProcMethod{ .. } => 1,
        }
    }

    pub fn prefix(&self) -> String {
        match self {
            Self::Func | Self::Proc => "".to_string(),
            Self::FuncMethod(t) => format!("{t}."),
            Self::ProcMethod{ before, after } =>
                if let Some(after) = after { format!("({before} ~> {after}).") }
                else { format!("{before}.") },
        }
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::Func | Self::Proc => false,
            Self::FuncMethod(t) => t.has_unbound_var(),
            Self::ProcMethod{ before, after } =>
                before.has_unbound_var() || after.as_ref().map(|t| t.has_unbound_var()).unwrap_or(false),
        }
    }

    pub fn same_kind_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Func, Self::Func) | (Self::Proc, Self::Proc)
            | (Self::FuncMethod(_), Self::FuncMethod(_)) | (Self::ProcMethod{ .. }, Self::ProcMethod{ .. }) => true,
            _ => false,
        }
    }

    pub fn self_t(&self) -> Option<&SelfType> {
        match self {
            Self::FuncMethod(t) | Self::ProcMethod{ before: t, .. } => Some(t),
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
    pub const fn is_owned(&self) -> bool { matches!(self, Self::Owned) }
    pub const fn is_ref(&self) -> bool { matches!(self, Self::Ref) }
    pub const fn is_refmut(&self) -> bool { matches!(self, Self::RefMut) }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArgsOwnership {
    Args{ self_: Option<Ownership>, non_defaults: Vec<Ownership>, defaults: Vec<Ownership> },
    VarArgs(Ownership), // TODO: defaults
    VarArgsDefault(Ownership),
}

impl ArgsOwnership {
    pub const fn args(self_: Option<Ownership>, non_defaults: Vec<Ownership>, defaults: Vec<Ownership>) -> Self {
        Self::Args{ self_, non_defaults, defaults }
    }
}

/// NOTE: 連携型変数があるので、比較には`ref_eq`を使うこと
/// Commonが付く型は多相だが中の型をなんでも受け入れるバージョン
/// TODO: MonoArray Int, 3 == PolyArray Int, Int, Int
/// Mut型を作ろうとすると、name() -> &strがうまくいかないので
/// 組み込みMut型は全て書き下す
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /* Monomorphic (builtin) types */
    Obj, // {=}
    ObjMut,
    Int,
    IntMut,
    Nat,
    NatMut,
    Ratio,
    RatioMut,
    Float,
    FloatMut,
    Bool,
    BoolMut,
    Str,
    StrMut,
    NoneType,
    Code,
    Module,
    Frame,
    Error,
    Inf, // {∞}
    NegInf, // {-∞}
    // TODO: PolyType/Class
    Type,
    Class,
    Trait,
    Patch,
    RangeCommon,
    FuncCommon,
    ProcCommon,
    FuncMethodCommon,
    ProcMethodCommon,
    CallableCommon,
    ArrayCommon,
    DictCommon,
    NotImplemented,
    Ellipsis, // これはクラスのほうで型推論用のマーカーではない
    Never, // {}
    Mono(Str), // others
    /* Polymorphic types */
    Range(Box<Type>),
    Iter(Box<Type>),
    Ref(Box<Type>),
    RefMut(Box<Type>),
    Option(Box<Type>),
    OptionMut(Box<Type>),
    Subr(SubrType),
    // CallableはProcの上位型なので、変数に!をつける
    Callable{ param_ts: Vec<Type>, return_t: Box<Type> },
    // e.g.  [Int] == Array{ t: Int, len: _ }, [Int; 3] == Array { t: Int, len: 3 }
    Array{ t: Box<Type>, len: TyParam },
    // TODO: heterogeneous dict
    Dict{ k: Box<Type>, v: Box<Type> },
    Tuple(Vec<Type>),
    Record(Dict<Str, Type>), // e.g. {x = Int}
    // e.g. {T -> T | T: Type}, {I: Int | I > 0}, {S | N: Nat; S: Str N; N > 1}
    // 区間型と列挙型は篩型に変換される
    // f 0 = ...はf _: {0} == {I: Int | I == 0}のシンタックスシュガー
    // e.g.
    // {0, 1, 2} => {I: Int | I == 0 or I == 1 or I == 2}
    // 1..10 => {I: Int | I >= 1 and I <= 10}
    Refinement(RefinementType),
    // e.g. |T: Type| T -> T
    Quantified(QuantifiedType),
    And(Vec<Type>),
    Not(Vec<Type>),
    Or(Vec<Type>),
    VarArgs(Box<Type>), // ...T
    Poly{ name: Str, params: Vec<TyParam> }, // T(params)
    /* Special types (inference-time types) */
    MonoQVar(Str), // QuantifiedTyの中で使う一般化型変数、利便性のためMonoとは区別する
    PolyQVar{ name: Str, params: Vec<TyParam> },
    FreeVar(FreeTyVar), // a reference to the type of other expression, see docs/compiler/inference.md
    MonoProj{ lhs: Box<Type>, rhs: Str }, // e.g. T.U
    ASTOmitted, // call中のcalleeの型など、不要な部分に付ける
    Failure, // when failed to infer
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mono(name) => write!(f, "{name}"),
            Self::Range(t) => write!(f, "Range({t})"),
            Self::RangeCommon => write!(f, "Range(Int)"),
            Self::Iter(t) => write!(f, "Iter({t})"),
            Self::Ref(t) => write!(f, "Ref({t})"),
            Self::RefMut(t) => write!(f, "Ref!({t})"),
            Self::Option(t) => write!(f, "Option({t})"),
            Self::OptionMut(t) => write!(f, "Option!({t})"),
            Self::Subr(sub) => write!(f, "{sub}"),
            Self::Callable{ param_ts, return_t } => {
                write!(f, "Callable(({}), {return_t})", fmt_vec(param_ts))
            }
            Self::Array{ t, len } => write!(f, "[{t}; {len}]"),
            Self::Dict{ k, v } => write!(f, "{{{k}: {v}}}"),
            Self::Tuple(ts) => write!(f, "({})", fmt_vec(ts)),
            Self::Record(attrs) => write!(f, "{{{attrs}}}"),
            Self::Refinement(refinement) => write!(f, "{}", refinement),
            Self::Quantified(quantified) => write!(f, "{}", quantified),
            Self::And(types) => write!(f, "{}", fmt_vec_split_with(types, " and ")),
            Self::Not(types) => write!(f, "{}", fmt_vec_split_with(types, " not ")),
            Self::Or(types) =>  write!(f, "{}", fmt_vec_split_with(types, " or ")),
            Self::VarArgs(t) => write!(f, "...{t}"),
            Self::Poly{ name, params } => write!(f, "{name}({})", fmt_vec(params)),
            Self::MonoQVar(name) => write!(f, "'{name}"),
            Self::FreeVar(v) => write!(f, "{v}"),
            Self::MonoProj{ lhs, rhs } => write!(f, "{lhs}.{rhs}"),
            _ => write!(f, "{}", self.name()),
        }
    }
}

impl Default for Type {
    fn default() -> Self { Self::Failure }
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
            "Obj!" => Self::ObjMut,
            "Int" => Self::Int,
            "Int!" => Self::IntMut,
            "Nat" => Self::Nat,
            "Nat!" => Self::NatMut,
            "Ratio" => Self::Ratio,
            "Ratio!" => Self::RatioMut,
            "Float" => Self::Float,
            "Float!" => Self::FloatMut,
            "Bool" => Self::Bool,
            "Bool!" => Self::BoolMut,
            "Str" => Self::Str,
            "Str!" => Self::StrMut,
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
            "_" => Self::Top(),
            _ => todo!(),
        }
    }
}

impl HasType for Type {
    #[inline]
    fn ref_t(&self) -> &Type { self }
    fn inner_ts(&self) -> Vec<Type> {
        match self {
            Self::RangeCommon => vec![Type::Int],
            Self::Dict{k, v} => vec![
                k.as_ref().clone(),
                v.as_ref().clone()
            ],
            Self::Ref(t)
            | Self::RefMut(t)
            | Self::Option(t)
            | Self::OptionMut(t)
            | Self::Range(t)
            | Self::Iter(t)
            | Self::Array{ t, .. }
            | Self::VarArgs(t) => vec![t.as_ref().clone()],
            // Self::And(ts) | Self::Or(ts) => ,
            Self::Subr(_sub) => todo!(),
            | Self::Callable{ param_ts, .. }
            | Self::Tuple(param_ts) => param_ts.clone(),
            Self::Poly{ .. } => {
                todo!()
            },
            _ => vec![],
        }
    }
    fn signature_t(&self) -> Option<&Type> { None }
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
            Self::Ref(t)
            | Self::RefMut(t)
            | Self::Option(t)
            | Self::OptionMut(t)
            | Self::Range(t)
            | Self::Iter(t)
            | Self::Array{ t, .. }
            | Self::VarArgs(t) => t.update_level(level),
            Self::Callable{ param_ts, return_t } => {
                for p in param_ts.iter() {
                    p.update_level(level);
                }
                return_t.update_level(level);
            }
            Self::Subr(subr) => {
                subr.kind.update_level(level);
                for p in subr.non_default_params.iter()
                    .chain(subr.default_params.iter()) {
                    p.ty.update_level(level);
                }
                subr.return_t.update_level(level);
            }
            Self::And(ts)
            | Self::Or(ts)
            | Self::Not(ts)
            | Self::Tuple(ts) => {
                for t in ts.iter() {
                    t.update_level(level);
                }
            },
            Self::Dict{ k, v } => {
                k.update_level(level);
                v.update_level(level);
            },
            Self::Record(attrs) => {
                for t in attrs.values() {
                    t.update_level(level);
                }
            },
            Self::Poly{ params, .. } => {
                for p in params.iter() {
                    p.update_level(level);
                }
            },
            Self::MonoProj{ lhs, .. } => {
                lhs.update_level(level);
            },
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
            _ => {},
        }
    }

    fn lift(&self) {
        match self {
            Self::FreeVar(v) => v.lift(),
            Self::Ref(t)
            | Self::RefMut(t)
            | Self::Option(t)
            | Self::OptionMut(t)
            | Self::Range(t)
            | Self::Iter(t)
            | Self::Array{ t, .. }
            | Self::VarArgs(t) => t.lift(),
            Self::Callable{ param_ts, return_t } => {
                for p in param_ts.iter() {
                    p.lift();
                }
                return_t.lift();
            }
            Self::Subr(subr) => {
                subr.kind.lift();
                for p in subr.non_default_params.iter()
                    .chain(subr.default_params.iter()) {
                    p.ty.lift();
                }
                subr.return_t.lift();
            }
            Self::And(ts)
            | Self::Or(ts)
            | Self::Not(ts)
            | Self::Tuple(ts) => {
                for t in ts.iter() {
                    t.lift();
                }
            },
            Self::Dict{ k, v } => {
                k.lift();
                v.lift();
            },
            Self::Record(attrs) => {
                for t in attrs.values() {
                    t.lift();
                }
            },
            Self::Poly{ params, .. } => {
                for p in params.iter() {
                    p.lift();
                }
            },
            Self::MonoProj{ lhs, .. } => { lhs.lift(); },
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
            _ => {},
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
    pub const fn Top() -> Self { Self::Mono(Str::ever("Top")) }
    /// Bottom := {}
    #[allow(non_snake_case)]
    pub const fn Bottom() -> Self { Self::Mono(Str::ever("Bottom")) }

    #[inline]
    pub fn free_var(level: usize, constraint: Constraint) -> Self {
        Self::FreeVar(Free::new_unbound(level, constraint))
    }

    #[inline]
    pub fn named_free_var(name: Str, level: usize, constraint: Constraint) -> Self {
        Self::FreeVar(Free::new_named_unbound(name, level, constraint))
    }

    #[inline]
    pub fn array(elem_t: Type, len: TyParam) -> Self { Self::Array{ t: Box::new(elem_t), len } }

    #[inline]
    pub fn dict(k_t: Type, v_t: Type) -> Self {
        Self::Dict{ k: Box::new(k_t), v: Box::new(v_t) }
    }

    #[inline]
    pub fn var_args(elem_t: Type) -> Self { Self::VarArgs(Box::new(elem_t)) }

    #[inline]
    pub fn range(t: Type) -> Self { Self::Range(Box::new(t)) }

    pub fn enum_t(s: Set<ValueObj>) -> Self {
        assert!(s.is_homogeneous());
        let name = Str::from(fresh_varname());
        let preds = s.iter()
            .map(|o| Predicate::eq(name.clone(), TyParam::value(o.clone())))
            .collect();
        let refine = RefinementType::new(name, s.inner_class(), preds);
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
                Predicate::le(name.clone(), r)
            ),
            IntervalOp::RightOpen if r == TyParam::value(Inf) => Predicate::ge(name.clone(), l),
            // l..<r => {I: classof(l) | I >= l and I <= r-ε}
            IntervalOp::RightOpen => Predicate::and(
                Predicate::ge(name.clone(), l),
                Predicate::le(name.clone(), TyParam::pred(r))
            ),
            // l..r => {I: classof(l) | I >= l and I <= r}
            IntervalOp::Closed => Predicate::and(
                Predicate::ge(name.clone(), l),
                Predicate::le(name.clone(), r)
            ),
            IntervalOp::Open if l == TyParam::value(NegInf) && r == TyParam::value(Inf) => {
                return Type::refinement(name.clone(), Type::Int, set!{})
            },
            // l<..<r => {I: classof(l) | I >= l+ε and I <= r-ε}
            IntervalOp::Open => Predicate::and(
                Predicate::ge(name.clone(), TyParam::succ(l)),
                Predicate::le(name.clone(), TyParam::pred(r))
            ),
        };
        Type::refinement(name.clone(), Type::Int, set!{pred})
    }

    pub fn iter(t: Type) -> Self { Self::Iter(Box::new(t)) }

    pub fn refer(t: Type) -> Self { Self::Ref(Box::new(t)) }

    pub fn ref_mut(t: Type) -> Self { Self::RefMut(Box::new(t)) }

    pub fn option(t: Type) -> Self { Self::Option(Box::new(t)) }

    pub fn option_mut(t: Type) -> Self { Self::OptionMut(Box::new(t)) }

    pub fn subr(kind: SubrKind, non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, return_t: Type) -> Self {
        Self::Subr(SubrType::new(kind, non_default_params, default_params, return_t))
    }

    pub fn func(non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, return_t: Type) -> Self {
        Self::Subr(SubrType::new(SubrKind::Func, non_default_params, default_params, return_t))
    }

    pub fn func1(param_t: Type, return_t: Type) -> Self { Self::func(vec![ParamTy::anonymous(param_t)], vec![], return_t) }

    pub fn kind1(param: Type) -> Self { Self::func1(param, Type::Type) }

    pub fn func2(l: Type, r: Type, return_t: Type) -> Self {
        Self::func(vec![ParamTy::anonymous(l), ParamTy::anonymous(r)], vec![], return_t)
    }

    pub fn anon_param_func(non_default_params: Vec<Type>, default_params: Vec<Type>, return_t: Type) -> Self {
        let non_default_params = non_default_params.into_iter().map(ParamTy::anonymous).collect();
        let default_params = default_params.into_iter().map(ParamTy::anonymous).collect();
        Self::func(non_default_params, default_params, return_t)
    }

    pub fn proc(non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, return_t: Type) -> Self {
        Self::Subr(SubrType::new(SubrKind::Proc, non_default_params, default_params, return_t))
    }

    pub fn proc1(param_t: Type, return_t: Type) -> Self {
        Self::proc(vec![ParamTy::anonymous(param_t)], vec![], return_t)
    }

    pub fn proc2(l: Type, r: Type, return_t: Type) -> Self {
        Self::proc(vec![ParamTy::anonymous(l), ParamTy::anonymous(r)], vec![], return_t)
    }

    pub fn anon_param_proc(non_default_params: Vec<Type>, default_params: Vec<Type>, return_t: Type) -> Self {
        let non_default_params = non_default_params.into_iter().map(ParamTy::anonymous).collect();
        let default_params = default_params.into_iter().map(ParamTy::anonymous).collect();
        Self::proc(non_default_params, default_params, return_t)
    }

    pub fn fn_met(self_t: Type, non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, return_t: Type) -> Self {
        Self::Subr(SubrType::new(SubrKind::FuncMethod(Box::new(self_t)), non_default_params, default_params, return_t))
    }

    pub fn fn0_met(self_t: Type, return_t: Type) -> Self {
        Self::fn_met(self_t, vec![], vec![], return_t)
    }

    pub fn fn1_met(self_t: Type, input_t: Type, return_t: Type) -> Self {
        Self::fn_met(self_t, vec![ParamTy::anonymous(input_t)],  vec![], return_t)
    }

    pub fn anon_param_fn_met(self_t: Type, non_default_params: Vec<Type>, default_params: Vec<Type>, return_t: Type) -> Self {
        let non_default_params = non_default_params.into_iter().map(ParamTy::anonymous).collect();
        let default_params = default_params.into_iter().map(ParamTy::anonymous).collect();
        Self::fn_met(self_t, non_default_params, default_params, return_t)
    }

    pub fn pr_met(self_before: Type, self_after: Option<Type>, non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, return_t: Type) -> Self {
        Self::Subr(SubrType::new(SubrKind::pr_met(self_before, self_after), non_default_params, default_params, return_t))
    }

    pub fn pr0_met(self_before: Type, self_after: Option<Type>, return_t: Type) -> Self {
        Self::pr_met(self_before, self_after, vec![], vec![], return_t)
    }

    pub fn pr1_met(self_before: Type, self_after: Option<Type>, input_t: Type, return_t: Type) -> Self {
        Self::pr_met(self_before, self_after, vec![ParamTy::anonymous(input_t)], vec![], return_t)
    }

    pub fn anon_param_pr_met(self_before: Type, self_after: Option<Type>, non_default_params: Vec<Type>, default_params: Vec<Type>, return_t: Type) -> Self {
        let non_default_params = non_default_params.into_iter().map(ParamTy::anonymous).collect();
        let default_params = default_params.into_iter().map(ParamTy::anonymous).collect();
        Self::pr_met(self_before, self_after, non_default_params, default_params, return_t)
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
        Self::Callable{ param_ts, return_t: Box::new(return_t) }
    }

    #[inline]
    pub fn mono<S: Into<Str>>(name: S) -> Self { Self::Mono(name.into()) }

    #[inline]
    pub fn mono_q<S: Into<Str>>(name: S) -> Self { Self::MonoQVar(name.into()) }

    #[inline]
    pub fn poly<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Self {
        Self::Poly{ name: name.into(), params }
    }

    #[inline]
    pub fn poly_q<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Self {
        Self::PolyQVar{ name: name.into(), params }
    }

    #[inline]
    pub fn mono_proj<S: Into<Str>>(lhs: Type, rhs: S) -> Self {
        Self::MonoProj{ lhs: Box::new(lhs), rhs: rhs.into() }
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

    pub fn mutate(self) -> Self {
        match self {
            Self::Int => Self::IntMut,
            Self::Nat => Self::NatMut,
            Self::Ratio => Self::RatioMut,
            Self::Float => Self::FloatMut,
            Self::Bool => Self::BoolMut,
            Self::Str => Self::StrMut,
            Self::Option(t) => Self::OptionMut(t),
            Self::Array{ t, len } =>
                Self::poly("Array!", vec![TyParam::t(*t), len.mutate()]),
            _ => todo!(),
        }
    }

    pub fn is_mut(&self) -> bool {
        match self {
            Self::FreeVar(fv) => if fv.is_linked() {
                fv.crack().is_mut()
            } else {
                fv.unbound_name().unwrap().ends_with("!")
            },
            Self::IntMut
            | Self::NatMut
            | Self::RatioMut
            | Self::FloatMut
            | Self::BoolMut
            | Self::StrMut
            | Self::OptionMut(_) => true,
            Self::Mono(name)
            | Self::MonoQVar(name)
            | Self::Poly { name, .. }
            | Self::PolyQVar { name, .. }
            | Self::MonoProj { rhs: name, .. } => name.ends_with("!"),
            _ => false,
        }
    }

    pub fn is_nonelike(&self) -> bool {
        match self {
            Self::NoneType => true,
            Self::Option(t)
            | Self::OptionMut(t) => t.is_nonelike(),
            Self::Tuple(ts) => ts.len() == 0,
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
                        Self::VarArgs(t) => { return ArgsOwnership::VarArgs(t.ownership()) },
                        _ => Ownership::Owned,
                    };
                    nd_args.push(ownership);
                }
                let mut d_args = vec![];
                for d_param in subr.default_params.iter() {
                    let ownership = match &d_param.ty {
                        Self::Ref(_) => Ownership::Ref,
                        Self::RefMut(_) => Ownership::RefMut,
                        Self::VarArgs(t) => { return ArgsOwnership::VarArgsDefault(t.ownership()) },
                        _ => Ownership::Owned,
                    };
                    d_args.push(ownership);
                }
                ArgsOwnership::args(self_, nd_args, d_args)
            },
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

    pub fn rec_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::FreeVar(v), other)
            | (other, Self::FreeVar(v)) => match &*v.borrow() {
                FreeKind::Linked(t) => t.rec_eq(other),
                _ => self == other,
            },
            (Self::Range(l), Self::Range(r))
            | (Self::Iter(l), Self::Iter(r))
            | (Self::Ref(l), Self::Ref(r))
            | (Self::RefMut(l), Self::RefMut(r))
            | (Self::Option(l), Self::Option(r))
            | (Self::OptionMut(l), Self::OptionMut(r)) => l.rec_eq(r),
            (Self::Subr(l), Self::Subr(r)) => {
                match (&l.kind, &r.kind) {
                    (SubrKind::Func, SubrKind::Func)
                    | (SubrKind::Proc, SubrKind::Proc) => {},
                    (SubrKind::FuncMethod(l), SubrKind::FuncMethod(r)) if !l.rec_eq(r.as_ref()) => { return false },
                    (SubrKind::ProcMethod{ before, after }, SubrKind::ProcMethod{ before: rbefore, after: rafter })
                        if !before.rec_eq(rbefore.as_ref())
                        || !after.as_ref().zip(rafter.as_ref()).map(|(l, r)| l.rec_eq(r)).unwrap_or(false) => { return false },
                    _ => { return false },
                }
                if !l.default_params.iter().zip(r.default_params.iter()).all(|(l, r)| {
                    l.name == r.name && l.ty.rec_eq(&r.ty)
                }) { return false }
                if !l.non_default_params.iter().zip(r.non_default_params.iter()).all(|(l, r)| {
                    l.name == r.name && l.ty.rec_eq(&r.ty)
                }) { return false }
                l.return_t.rec_eq(&r.return_t)
            },
            (
                Self::Callable{ param_ts: _lps, return_t: _lr },
                Self::Callable{ param_ts: _rps, return_t: _rr },
            ) => todo!(),
            (
                Self::Array{ t: lt, len: ll},
                Self::Array{ t: rt, len: rl }
            ) => lt.rec_eq(rt) && ll.rec_eq(rl),
            (
                Self::Dict{ k: lk, v: lv },
                Self::Dict{ k: rk, v: rv }
            ) => lk.rec_eq(rk) && lv.rec_eq(rv),
            (Self::Record(_l), Self::Record(_r)) => todo!(),
            (Self::Refinement(l), Self::Refinement(r)) => {
                l.t.rec_eq(&r.t) && &l.preds == &r.preds
            },
            (Self::Quantified(l), Self::Quantified(r)) => {
                l.unbound_callable.rec_eq(&r.unbound_callable) && &l.bounds == &r.bounds
            },
            (Self::Tuple(l), Self::Tuple(r))
            | (Self::And(l), Self::And(r))
            | (Self::Not(l), Self::Not(r))
            | (Self::Or(l), Self::Or(r)) => l.iter().zip(r.iter()).all(|(l, r)| l.rec_eq(r)),
            (Self::VarArgs(l), Self::VarArgs(r)) => l.rec_eq(r),
            (
                Self::Poly{ name: ln, params: lps } | Self::PolyQVar{ name: ln, params: lps },
                Self::Poly{ name: rn, params: rps } | Self::PolyQVar{ name: rn, params: rps },
            ) => ln == rn && lps.iter().zip(rps.iter()).all(|(l, r)| l.rec_eq(r)),
            (Self::MonoProj{ lhs, rhs }, Self::MonoProj{ lhs: rlhs, rhs: rrhs }) => {
                lhs.rec_eq(rlhs) && rhs == rrhs
            },
            _ => self == other,
        }
    }

    /// 共通部分(A and B)を返す
    /// 型同士の包含関係はここでは検査しない(TypeCheckerでする)
    pub fn intersection(lhs: &Self, rhs: &Self) -> Self {
        if lhs == rhs { return lhs.clone() }
        match (lhs, rhs) {
            // { .i: Int } and { .s: Str } == { .i: Int, .s: Str }
            (Self::Record(l), Self::Record(r)) => {
                Self::Record(l.clone().concat(r.clone()))
            }
            (Self::And(ts), t)
            | (t, Self::And(ts)) => Self::And([vec![t.clone()], ts.clone()].concat()),
            (t, Self::Obj) | (Self::Obj, t) => t.clone(),
            (_, Self::Never) | (Self::Never, _) => Self::Never,
            (l, r) => Self::And(vec![l.clone(), r.clone()]),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Obj => "Obj",
            Self::ObjMut => "Obj!",
            Self::Int => "Int",
            Self::IntMut => "Int!",
            Self::Nat => "Nat",
            Self::NatMut => "Nat!",
            Self::Ratio => "Ratio",
            Self::RatioMut => "Ratio!",
            Self::Float => "Float",
            Self::FloatMut => "Float!",
            Self::Bool => "Bool",
            Self::BoolMut => "Bool!",
            Self::Str => "Str",
            Self::StrMut => "Str!",
            Self::NoneType => "None",
            Self::Type => "Type",
            Self::Class => "Class",
            Self::Trait => "Trait",
            Self::Patch => "Patch",
            Self::Code => "Code",
            Self::Module => "Module",
            Self::Frame => "Frame",
            Self::Error => "Error",
            Self::Inf => "Inf",
            Self::NegInf => "NegInf",
            Self::Mono(name)
            | Self::MonoQVar(name) => name,
            Self::Range(_) | Self::RangeCommon => "Range",
            Self::Iter(_) => "Iter",
            Self::And(_) => "And",
            Self::Not(_) => "Not",
            Self::Or(_) => "Or",
            Self::Ref(_) => "Ref",
            Self::RefMut(_) => "Ref!",
            Self::Option(_) => "Option",
            Self::OptionMut(_) => "Option!",
            Self::Subr(SubrType{ kind: SubrKind::Func, .. }) | Self::FuncCommon => "Func",
            Self::Subr(SubrType{ kind: SubrKind::Proc, .. }) | Self::ProcCommon => "Proc",
            Self::Subr(SubrType{ kind: SubrKind::FuncMethod(_), .. }) | Self::FuncMethodCommon => "FuncMethod",
            Self::Subr(SubrType{ kind: SubrKind::ProcMethod{ .. }, .. }) | Self::ProcMethodCommon => "ProcMethod",
            Self::Callable{ .. } | Self::CallableCommon => "Callable",
            Self::Array{ .. } | Self::ArrayCommon => "Array",
            Self::Dict{ .. } | Self::DictCommon => "Dict",
            Self::Tuple(..) => "Tuple",
            Self::Record(_) => "Record",
            Self::VarArgs(_) => "VarArgs",
            Self::Poly{ name, .. }
            | Self::PolyQVar{ name, .. } => &*name,
            // NOTE: compiler/codegen/convert_to_python_methodでクラス名を使うため、こうすると都合が良い
            Self::Refinement(refine)=> refine.t.name(),
            Self::Quantified(_) => "Quantified",
            Self::Ellipsis => "Ellipsis",
            Self::NotImplemented => "NotImplemented",
            Self::Never => "Never",
            Self::FreeVar(_) => "?", // TODO: 中身がSomeなら表示したい
            Self::MonoProj{ .. } => "MonoProj",
            Self::ASTOmitted => "ASTOmitted",
            Self::Failure => "<Failure>",
        }
    }

    pub const fn is_free_var(&self) -> bool {
        matches!(self, Self::FreeVar(_))
    }

    pub const fn is_varargs(&self) -> bool { matches!(self, Self::VarArgs(_)) }

    pub fn is_monomorphic(&self) -> bool { self.typaram_len() == 0 }

    pub const fn is_callable(&self) -> bool {
        matches!(self, Self::Subr{ .. } | Self::Callable{ .. })
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::FreeVar(fv) =>
                if fv.is_unbound() { true } else { fv.crack().has_unbound_var() },
            Self::Range(t)
            | Self::Iter(t)
            | Self::Ref(t)
            | Self::RefMut(t)
            | Self::Option(t)
            | Self::OptionMut(t)
            | Self::VarArgs(t) => t.has_unbound_var(),
            Self::And(param_ts)
            | Self::Not(param_ts)
            | Self::Or(param_ts) => param_ts.iter().any(|t| t.has_unbound_var()),
            Self::Array{ t, len } => t.has_unbound_var() || len.has_unbound_var(),
            Self::Dict{ k, v } => k.has_unbound_var() || v.has_unbound_var(),
            Self::Callable{ param_ts, return_t } => {
                param_ts.iter().any(|t| t.has_unbound_var()) || return_t.has_unbound_var()
            },
            Self::Subr(subr) => {
                subr.kind.has_unbound_var()
                || subr.non_default_params.iter().any(|p| p.ty.has_unbound_var())
                || subr.default_params.iter().any(|p| p.ty.has_unbound_var())
                || subr.return_t.has_unbound_var()
            },
            Self::Record(r) => r.values().any(|t| t.has_unbound_var()),
            Self::Refinement(refine) =>
                refine.t.has_unbound_var()
                || refine.preds.iter().any(|p| p.has_unbound_var()),
            Self::Quantified(quant) =>
                quant.unbound_callable.has_unbound_var()
                || quant.bounds.iter().any(|b| b.has_unbound_var()),
            Self::Poly{ params, .. }
            | Self::PolyQVar{ params, .. }=> params.iter().any(|p| p.has_unbound_var()),
            Self::MonoProj{ lhs, .. } => lhs.has_no_unbound_var(),
            _ => false,
        }
    }

    pub fn has_no_unbound_var(&self) -> bool { !self.has_unbound_var() }

    pub fn typaram_len(&self) -> usize {
        match self {
            Self::Range(_)
            | Self::Iter(_)
            | Self::Option(_)
            | Self::OptionMut(_) => 1,
            Self::Array{ .. } | Self::Dict{ .. } => 2,
            Self::And(param_ts)
            | Self::Or(param_ts)
            | Self::Tuple(param_ts) => param_ts.len() + 1,
            Self::Subr(subr) =>
                subr.kind.inner_len() + subr.non_default_params.len() + subr.default_params.len() + 1,
            Self::Callable{ param_ts, .. } => param_ts.len() + 1,
            Self::Poly{ params, .. }
            | Self::PolyQVar{ params, .. } => params.len(),
            _ => 0,
        }
    }

    pub fn typarams(&self) -> Vec<TyParam> {
        match self {
            Self::FreeVar(f) if f.is_linked() => f.crack().typarams(),
            Self::FreeVar(_unbound) => todo!(),
            Self::Range(t)
            | Self::Iter(t)
            | Self::Ref(t)
            | Self::RefMut(t)
            | Self::Option(t)
            | Self::OptionMut(t) => vec![TyParam::t(*t.clone())],
            Self::Array{ t, len } => vec![TyParam::t(*t.clone()), len.clone()],
            Self::Dict{ k, v } => vec![TyParam::t(*k.clone()), TyParam::t(*v.clone())],
            Self::And(param_ts)
            | Self::Or(param_ts)
            | Self::Not(param_ts)
            | Self::Tuple(param_ts) => param_ts.iter().map(|t| TyParam::t(t.clone())).collect(),
            Self::Subr(subr) => if let Some(self_t) = subr.kind.self_t() {
                [
                    vec![TyParam::t(self_t.clone())],
                    subr.non_default_params.iter().map(|t| TyParam::t(t.ty.clone())).collect(),
                    subr.default_params.iter().map(|t| TyParam::t(t.ty.clone())).collect(),
                ].concat()
            } else {
                [
                    subr.non_default_params.iter().map(|t| TyParam::t(t.ty.clone())).collect::<Vec<_>>(),
                    subr.default_params.iter().map(|t| TyParam::t(t.ty.clone())).collect(),
                ].concat()
            },
            Self::Callable{ param_ts: _, .. } => todo!(),
            Self::Poly{ params, .. }
            | Self::PolyQVar{ params, .. } => params.clone(),
            _ => vec![],
        }
    }

    pub const fn self_t(&self) -> Option<&Type> {
        match self {
            Self::Subr(SubrType{ kind:
                SubrKind::FuncMethod(self_t) | SubrKind::ProcMethod{ before: self_t, .. },
                ..
            }) => Some(self_t),
            _ => None,
        }
    }

    pub const fn non_default_params(&self) -> Option<&Vec<ParamTy>> {
        match self {
            Self::Subr(SubrType{ non_default_params, .. }) => Some(non_default_params),
            Self::Callable{ param_ts: _, .. } => todo!(),
            _ => None,
        }
    }

    pub const fn default_params(&self) -> Option<&Vec<ParamTy>> {
        match self {
            Self::Subr(SubrType{ default_params, .. }) => Some(default_params),
            _ => None,
        }
    }

    pub const fn return_t(&self) -> Option<&Type> {
        match self {
            Self::Subr(SubrType{ return_t, .. })
            | Self::Callable{ return_t, .. } => Some(return_t),
            _ => None,
        }
    }

    pub fn mut_return_t(&mut self) -> Option<&mut Type> {
        match self {
            Self::Subr(SubrType{ return_t, .. })
            | Self::Callable{ return_t, .. } => Some(return_t),
            _ => None,
        }
    }
}

pub mod type_constrs {
    use crate::ty::*;

    #[inline]
    pub const fn param_t(name: &'static str, ty: Type) -> ParamTy {
        ParamTy::new(Some(Str::ever(name)), ty)
    }

    #[inline]
    pub const fn anon(ty: Type) -> ParamTy { ParamTy::anonymous(ty) }

    #[inline]
    pub fn mono<S: Into<Str>>(name: S) -> Type { Type::mono(name) }

    #[inline]
    pub fn mono_q<S: Into<Str>>(name: S) -> Type { Type::mono_q(name) }

    #[inline]
    pub fn poly<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Type { Type::poly(name, params) }

    #[inline]
    pub fn poly_q<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Type { Type::poly_q(name, params) }

    #[inline]
    pub fn func(non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, ret: Type) -> Type {
        Type::func(non_default_params, default_params, ret)
    }

    #[inline]
    pub fn proc(non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, ret: Type) -> Type {
        Type::proc(non_default_params, default_params, ret)
    }

    #[inline]
    pub fn nd_func(params: Vec<ParamTy>, ret: Type) -> Type { Type::nd_func(params, ret) }

    #[inline]
    pub fn nd_proc(params: Vec<ParamTy>, ret: Type) -> Type { Type::nd_proc(params, ret) }

    #[inline]
    pub fn fn0_met(self_t: Type, return_t: Type) -> Type { Type::fn0_met(self_t, return_t) }

    #[inline]
    pub fn fn1_met(self_t: Type, input_t: Type, return_t: Type) -> Type { Type::fn1_met(self_t, input_t, return_t) }

    #[inline]
    pub fn quant(unbound_t: Type, bounds: Set<TyBound>) -> Type { Type::quantified(unbound_t, bounds) }

    #[inline]
    pub fn instance(name: Str, t: Type) -> TyBound { TyBound::instance(name, t) }

    #[inline]
    pub fn static_instance(name: &'static str, t: Type) -> TyBound { TyBound::static_instance(name, t) }

    #[inline]
    pub fn subtype(sub: Type, sup: Type) -> TyBound { TyBound::subtype(sub, sup) }

    #[inline]
    pub fn mono_q_tp<S: Into<Str>>(name: S) -> TyParam { TyParam::mono_q(name) }

    #[inline]
    pub fn mono_tp<S: Into<Str>>(name: S) -> TyParam { TyParam::mono(name) }

    #[inline]
    pub fn ty_tp(t: Type) -> TyParam { TyParam::t(t) }

    #[inline]
    pub fn value<V: Into<ValueObj>>(v: V) -> TyParam { TyParam::value(v) }
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
            Type::Array{ .. } => Self::Array,
            Type::FuncCommon => Self::Func,
            Type::ProcCommon => Self::Proc,
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
            65 | _ => Self::Illegals,
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
            (Type::Int, Type::Array{ .. }) => Self::IntArray,
            (Type::Int, Type::FuncCommon) => Self::IntFunc,
            (Type::Int, Type::ProcCommon) => Self::IntProc,
            (Type::Nat, Type::Int) => Self::NatInt,
            (Type::Nat, Type::Nat) => Self::NatNat,
            (Type::Nat, Type::Float) => Self::NatFloat,
            (Type::Nat, Type::Str) => Self::NatStr,
            (Type::Nat, Type::Bool) => Self::NatBool,
            (Type::Nat, Type::Array{ .. }) => Self::NatArray,
            (Type::Nat, Type::FuncCommon) => Self::NatFunc,
            (Type::Nat, Type::ProcCommon) => Self::NatProc,
            (Type::Float, Type::Int) => Self::FloatInt,
            (Type::Float, Type::Nat) => Self::FloatNat,
            (Type::Float, Type::Float) => Self::FloatFloat,
            (Type::Float, Type::Str) => Self::FloatStr,
            (Type::Float, Type::Bool) => Self::FloatBool,
            (Type::Float, Type::Array{ .. }) => Self::FloatArray,
            (Type::Float, Type::FuncCommon) => Self::FloatFunc,
            (Type::Float, Type::ProcCommon) => Self::FloatProc,
            (Type::Bool, Type::Int) => Self::BoolInt,
            (Type::Bool, Type::Nat) => Self::BoolNat,
            (Type::Bool, Type::Float) => Self::BoolFloat,
            (Type::Bool, Type::Str) => Self::BoolStr,
            (Type::Bool, Type::Bool) => Self::BoolBool,
            (Type::Bool, Type::Array{ .. }) => Self::BoolArray,
            (Type::Bool, Type::FuncCommon) => Self::BoolFunc,
            (Type::Bool, Type::ProcCommon) => Self::BoolProc,
            (Type::Str, Type::Int) => Self::StrInt,
            (Type::Str, Type::Nat) => Self::StrNat,
            (Type::Str, Type::Float) => Self::StrFloat,
            (Type::Str, Type::Bool) => Self::StrBool,
            (Type::Str, Type::Str) => Self::StrStr,
            (Type::Str, Type::Array{ .. }) => Self::StrArray,
            (Type::Str, Type::FuncCommon) => Self::StrFunc,
            (Type::Str, Type::ProcCommon) => Self::StrProc,
            // 要素数は検査済みなので、気にする必要はない
            (Type::Array{ .. }, Type::Int) => Self::ArrayInt,
            (Type::Array{ .. }, Type::Nat) => Self::ArrayNat,
            (Type::Array{ .. }, Type::Float) => Self::ArrayFloat,
            (Type::Array{ .. }, Type::Str) => Self::ArrayStr,
            (Type::Array{ .. }, Type::Bool) => Self::ArrayBool,
            (Type::Array{ .. }, Type::Array{ .. }) => Self::ArrayArray,
            (Type::Array{ .. }, Type::FuncCommon) => Self::ArrayFunc,
            (Type::Array{ .. }, Type::ProcCommon) => Self::ArrayProc,
            (Type::FuncCommon, Type::Int) => Self::FuncInt,
            (Type::FuncCommon, Type::Nat) => Self::FuncNat,
            (Type::FuncCommon, Type::Float) => Self::FuncFloat,
            (Type::FuncCommon, Type::Str) => Self::FuncStr,
            (Type::FuncCommon, Type::Bool) => Self::FuncBool,
            (Type::FuncCommon, Type::Array{ .. }) => Self::FuncArray,
            (Type::FuncCommon, Type::FuncCommon) => Self::FuncFunc,
            (Type::FuncCommon, Type::ProcCommon) => Self::FuncProc,
            (Type::ProcCommon, Type::Int) => Self::ProcInt,
            (Type::ProcCommon, Type::Nat) => Self::ProcNat,
            (Type::ProcCommon, Type::Float) => Self::ProcFloat,
            (Type::ProcCommon, Type::Str) => Self::ProcStr,
            (Type::ProcCommon, Type::Bool) => Self::ProcBool,
            (Type::ProcCommon, Type::Array{ .. }) => Self::ProcArray,
            (Type::ProcCommon, Type::ProcCommon) => Self::ProcProc,
            (_, _) => Self::Others,
        }
    }
}
