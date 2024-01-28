use std::fmt;
use std::ops::{BitAnd, BitOr, Not};

#[allow(unused_imports)]
use erg_common::log;
use erg_common::set::Set;
use erg_common::traits::{LimitedDisplay, StructuralEq};
use erg_common::{fmt_option, set, Str};

use super::free::{Constraint, HasLevel};
use super::typaram::TyParam;
use super::value::ValueObj;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Predicate {
    Value(ValueObj), // True/False
    Const(Str),
    Call {
        receiver: TyParam,
        name: Option<Str>,
        args: Vec<TyParam>,
    },
    /// i == 0 => Eq{ lhs: "i", rhs: 0 }
    Equal {
        lhs: Str,
        rhs: TyParam,
    },
    /// i > 0 == i >= 0 and i != 0
    GreaterEqual {
        lhs: Str,
        rhs: TyParam,
    },
    // i < 0 == i <= 0 and i != 0
    LessEqual {
        lhs: Str,
        rhs: TyParam,
    },
    NotEqual {
        lhs: Str,
        rhs: TyParam,
    },
    GeneralEqual {
        lhs: Box<Predicate>,
        rhs: Box<Predicate>,
    },
    GeneralLessEqual {
        lhs: Box<Predicate>,
        rhs: Box<Predicate>,
    },
    GeneralGreaterEqual {
        lhs: Box<Predicate>,
        rhs: Box<Predicate>,
    },
    GeneralNotEqual {
        lhs: Box<Predicate>,
        rhs: Box<Predicate>,
    },
    Or(Box<Predicate>, Box<Predicate>),
    And(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(v) => write!(f, "{v}"),
            Self::Const(c) => write!(f, "{c}"),
            Self::Call {
                receiver,
                name,
                args,
            } => {
                write!(
                    f,
                    "{receiver}{}({})",
                    fmt_option!(pre ".", name),
                    args.iter()
                        .map(|a| format!("{a}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::Equal { lhs, rhs } => write!(f, "{lhs} == {rhs}"),
            Self::GreaterEqual { lhs, rhs } => write!(f, "{lhs} >= {rhs}"),
            Self::LessEqual { lhs, rhs } => write!(f, "{lhs} <= {rhs}"),
            Self::NotEqual { lhs, rhs } => write!(f, "{lhs} != {rhs}"),
            Self::GeneralEqual { lhs, rhs } => write!(f, "{lhs} == {rhs}"),
            Self::GeneralLessEqual { lhs, rhs } => write!(f, "{lhs} <= {rhs}"),
            Self::GeneralGreaterEqual { lhs, rhs } => write!(f, "{lhs} >= {rhs}"),
            Self::GeneralNotEqual { lhs, rhs } => write!(f, "{lhs} != {rhs}"),
            Self::Or(l, r) => write!(f, "({l}) or ({r})"),
            Self::And(l, r) => write!(f, "({l}) and ({r})"),
            Self::Not(p) => write!(f, "not ({p})"),
        }
    }
}

impl LimitedDisplay for Predicate {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> std::fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Value(v) => v.limited_fmt(f, limit),
            Self::Const(c) => write!(f, "{c}"),
            // TODO:
            Self::Call {
                receiver,
                name,
                args,
            } => {
                write!(
                    f,
                    "{receiver}{}({})",
                    fmt_option!(pre ".", name),
                    args.iter()
                        .map(|a| format!("{a}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::Equal { lhs, rhs } => {
                write!(f, "{lhs} == ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::GreaterEqual { lhs, rhs } => {
                write!(f, "{lhs} >= ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::LessEqual { lhs, rhs } => {
                write!(f, "{lhs} <= ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::NotEqual { lhs, rhs } => {
                write!(f, "{lhs} != ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::GeneralEqual { lhs, rhs } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " == ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::GeneralLessEqual { lhs, rhs } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " <= ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::GeneralGreaterEqual { lhs, rhs } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " >= ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::GeneralNotEqual { lhs, rhs } => {
                lhs.limited_fmt(f, limit - 1)?;
                write!(f, " != ")?;
                rhs.limited_fmt(f, limit - 1)
            }
            Self::Or(l, r) => {
                write!(f, "(")?;
                l.limited_fmt(f, limit - 1)?;
                write!(f, ") or (")?;
                r.limited_fmt(f, limit - 1)?;
                write!(f, ")")
            }
            Self::And(l, r) => {
                write!(f, "(")?;
                l.limited_fmt(f, limit - 1)?;
                write!(f, ") and (")?;
                r.limited_fmt(f, limit - 1)?;
                write!(f, ")")
            }
            Self::Not(p) => {
                write!(f, "not (")?;
                p.limited_fmt(f, limit - 1)?;
                write!(f, ")")
            }
        }
    }
}

impl StructuralEq for Predicate {
    fn structural_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Equal { rhs, .. }, Self::Equal { rhs: r2, .. })
            | (Self::NotEqual { rhs, .. }, Self::NotEqual { rhs: r2, .. })
            | (Self::GreaterEqual { rhs, .. }, Self::GreaterEqual { rhs: r2, .. })
            | (Self::LessEqual { rhs, .. }, Self::LessEqual { rhs: r2, .. }) => {
                rhs.structural_eq(r2)
            }
            (
                Self::Call {
                    receiver,
                    name,
                    args,
                },
                Self::Call {
                    receiver: r,
                    name: n,
                    args: a,
                },
            ) => {
                receiver.structural_eq(r)
                    && name == n
                    && args.iter().zip(a.iter()).all(|(l, r)| l.structural_eq(r))
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
            (Self::Not(p1), Self::Not(p2)) => p1.structural_eq(p2),
            _ => self == other,
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
            Self::GeneralEqual { lhs, rhs }
            | Self::GeneralLessEqual { lhs, rhs }
            | Self::GeneralGreaterEqual { lhs, rhs }
            | Self::GeneralNotEqual { lhs, rhs } => {
                lhs.level().zip(rhs.level()).map(|(a, b)| a.min(b))
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.level().zip(rhs.level()).map(|(a, b)| a.min(b))
            }
            Self::Not(p) => p.level(),
            Self::Call { receiver, args, .. } => receiver
                .level()
                .zip(args.iter().map(|a| a.level().unwrap_or(usize::MAX)).min())
                .map(|(a, b)| a.min(b)),
        }
    }

    fn set_level(&self, level: usize) {
        match self {
            Self::Value(_) | Self::Const(_) => {}
            Self::Call { receiver, args, .. } => {
                receiver.set_level(level);
                for arg in args {
                    arg.set_level(level);
                }
            }
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => {
                rhs.set_level(level);
            }
            Self::GeneralEqual { lhs, rhs }
            | Self::GeneralLessEqual { lhs, rhs }
            | Self::GeneralGreaterEqual { lhs, rhs }
            | Self::GeneralNotEqual { lhs, rhs } => {
                lhs.set_level(level);
                rhs.set_level(level);
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.set_level(level);
                rhs.set_level(level);
            }
            Self::Not(p) => {
                p.set_level(level);
            }
        }
    }
}

impl BitAnd for Predicate {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::and(self, rhs)
    }
}

impl BitOr for Predicate {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::or(self, rhs)
    }
}

impl Not for Predicate {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.invert()
    }
}

impl Predicate {
    pub const TRUE: Predicate = Predicate::Value(ValueObj::Bool(true));
    pub const FALSE: Predicate = Predicate::Value(ValueObj::Bool(false));

    pub fn general_eq(lhs: Predicate, rhs: Predicate) -> Self {
        Self::GeneralEqual {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn general_ge(lhs: Predicate, rhs: Predicate) -> Self {
        Self::GeneralGreaterEqual {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn general_le(lhs: Predicate, rhs: Predicate) -> Self {
        Self::GeneralLessEqual {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn general_ne(lhs: Predicate, rhs: Predicate) -> Self {
        Self::GeneralNotEqual {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn call(receiver: TyParam, name: Option<Str>, args: Vec<TyParam>) -> Self {
        Self::Call {
            receiver,
            name,
            args,
        }
    }

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

    /// > (>= and !=)
    pub fn gt(lhs: Str, rhs: TyParam) -> Self {
        Self::and(Self::ge(lhs.clone(), rhs.clone()), Self::ne(lhs, rhs))
    }

    /// <=
    pub const fn le(lhs: Str, rhs: TyParam) -> Self {
        Self::LessEqual { lhs, rhs }
    }

    // < (<= and !=)
    pub fn lt(lhs: Str, rhs: TyParam) -> Self {
        Self::and(Self::le(lhs.clone(), rhs.clone()), Self::ne(lhs, rhs))
    }

    pub fn and(lhs: Predicate, rhs: Predicate) -> Self {
        match (lhs, rhs) {
            (Predicate::Value(ValueObj::Bool(true)), p) => p,
            (p, Predicate::Value(ValueObj::Bool(true))) => p,
            (Predicate::Value(ValueObj::Bool(false)), _)
            | (_, Predicate::Value(ValueObj::Bool(false))) => Predicate::FALSE,
            (Predicate::And(l, r), other) | (other, Predicate::And(l, r)) => {
                if l.as_ref() == &other {
                    *r & other
                } else if r.as_ref() == &other {
                    *l & other
                } else {
                    Self::And(Box::new(Self::And(l, r)), Box::new(other))
                }
            }
            (p1, p2) => {
                if p1 == p2 {
                    p1
                } else {
                    Self::And(Box::new(p1), Box::new(p2))
                }
            }
        }
    }

    pub fn or(lhs: Predicate, rhs: Predicate) -> Self {
        match (lhs, rhs) {
            (Predicate::Value(ValueObj::Bool(true)), _)
            | (_, Predicate::Value(ValueObj::Bool(true))) => Predicate::TRUE,
            (Predicate::Value(ValueObj::Bool(false)), p) => p,
            (p, Predicate::Value(ValueObj::Bool(false))) => p,
            (Predicate::Or(l, r), other) | (other, Predicate::Or(l, r)) => {
                if l.as_ref() == &other {
                    *r | other
                } else if r.as_ref() == &other {
                    *l | other
                } else {
                    Self::Or(Box::new(Self::Or(l, r)), Box::new(other))
                }
            }
            // I == 1 or I >= 1 => I >= 1
            (
                Predicate::Equal { lhs, rhs },
                Predicate::GreaterEqual {
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if lhs == lhs2 && rhs == rhs2 => Self::ge(lhs, rhs),
            (p1, p2) => {
                if p1 == p2 {
                    p1
                } else {
                    Self::Or(Box::new(p1), Box::new(p2))
                }
            }
        }
    }

    pub fn is_equal(&self) -> bool {
        matches!(self, Self::Equal { .. })
    }

    pub fn consist_of_equal(&self) -> bool {
        match self {
            Self::Equal { .. } => true,
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.consist_of_equal() && rhs.consist_of_equal()
            }
            Self::Not(pred) => pred.consist_of_equal(),
            _ => false,
        }
    }

    pub fn ands(&self) -> Set<&Predicate> {
        match self {
            Self::And(lhs, rhs) => {
                let mut set = lhs.ands();
                set.extend(rhs.ands());
                set
            }
            _ => set! { self },
        }
    }

    pub fn ors(&self) -> Set<&Predicate> {
        match self {
            Self::Or(lhs, rhs) => {
                let mut set = lhs.ors();
                set.extend(rhs.ors());
                set
            }
            _ => set! { self },
        }
    }

    pub fn subject(&self) -> Option<&str> {
        match self {
            Self::Equal { lhs, .. }
            | Self::LessEqual { lhs, .. }
            | Self::GreaterEqual { lhs, .. }
            | Self::NotEqual { lhs, .. } => Some(&lhs[..]),
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                let l = lhs.subject();
                let r = rhs.subject();
                if l != r {
                    log!(err "{l:?} != {r:?}");
                    None
                } else {
                    l
                }
            }
            Self::Not(pred) => pred.subject(),
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
            Self::Not(pred) => Self::not(pred.change_subject_name(name)),
            Self::GeneralEqual { lhs, rhs } => Self::general_eq(
                lhs.change_subject_name(name.clone()),
                rhs.change_subject_name(name),
            ),
            Self::GeneralGreaterEqual { lhs, rhs } => Self::general_ge(
                lhs.change_subject_name(name.clone()),
                rhs.change_subject_name(name),
            ),
            Self::GeneralLessEqual { lhs, rhs } => Self::general_le(
                lhs.change_subject_name(name.clone()),
                rhs.change_subject_name(name),
            ),
            Self::GeneralNotEqual { lhs, rhs } => Self::general_ne(
                lhs.change_subject_name(name.clone()),
                rhs.change_subject_name(name),
            ),
            _ => self,
        }
    }

    pub fn substitute(self, var: &str, tp: &TyParam) -> Self {
        match self {
            Self::Equal { lhs, .. } if &lhs == var => Self::eq(lhs, tp.clone()),
            Self::GreaterEqual { lhs, .. } if &lhs == var => Self::ge(lhs, tp.clone()),
            Self::LessEqual { lhs, .. } if &lhs == var => Self::le(lhs, tp.clone()),
            Self::NotEqual { lhs, .. } if &lhs == var => Self::ne(lhs, tp.clone()),
            Self::Equal { lhs, rhs } => Self::eq(lhs, rhs.substitute(var, tp)),
            Self::GreaterEqual { lhs, rhs } => Self::ge(lhs, rhs.substitute(var, tp)),
            Self::LessEqual { lhs, rhs } => Self::le(lhs, rhs.substitute(var, tp)),
            Self::NotEqual { lhs, rhs } => Self::ne(lhs, rhs.substitute(var, tp)),
            Self::And(lhs, rhs) => Self::and(lhs.substitute(var, tp), rhs.substitute(var, tp)),
            Self::Or(lhs, rhs) => Self::or(lhs.substitute(var, tp), rhs.substitute(var, tp)),
            Self::Not(pred) => Self::not(pred.substitute(var, tp)),
            Self::GeneralEqual { lhs, rhs } => {
                Self::general_eq(lhs.substitute(var, tp), rhs.substitute(var, tp))
            }
            Self::GeneralGreaterEqual { lhs, rhs } => {
                Self::general_ge(lhs.substitute(var, tp), rhs.substitute(var, tp))
            }
            Self::GeneralLessEqual { lhs, rhs } => {
                Self::general_le(lhs.substitute(var, tp), rhs.substitute(var, tp))
            }
            Self::GeneralNotEqual { lhs, rhs } => {
                Self::general_ne(lhs.substitute(var, tp), rhs.substitute(var, tp))
            }
            Self::Call {
                receiver,
                name,
                args,
            } => {
                let receiver = receiver.substitute(var, tp);
                let mut new_args = vec![];
                for arg in args {
                    new_args.push(arg.substitute(var, tp));
                }
                Self::Call {
                    receiver,
                    name,
                    args: new_args,
                }
            }
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
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => lhs.mentions(name) || rhs.mentions(name),
            _ => false,
        }
    }

    pub fn can_be_false(&self) -> Option<bool> {
        match self {
            Self::Value(l) => Some(matches!(l, ValueObj::Bool(false))),
            Self::Const(_) => None,
            Self::Or(lhs, rhs) => Some(lhs.can_be_false()? || rhs.can_be_false()?),
            Self::And(lhs, rhs) => Some(lhs.can_be_false()? && rhs.can_be_false()?),
            Self::Not(pred) => Some(!pred.can_be_false()?),
            _ => Some(true),
        }
    }

    pub fn qvars(&self) -> Set<(Str, Constraint)> {
        match self {
            Self::Value(_) | Self::Const(_) => set! {},
            Self::Call { receiver, args, .. } => {
                let mut set = receiver.qvars();
                for arg in args {
                    set.extend(arg.qvars());
                }
                set
            }
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.qvars(),
            Self::GeneralEqual { lhs, rhs }
            | Self::GeneralLessEqual { lhs, rhs }
            | Self::GeneralGreaterEqual { lhs, rhs }
            | Self::GeneralNotEqual { lhs, rhs } => {
                lhs.qvars().concat(rhs.qvars()).into_iter().collect()
            }
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => lhs.qvars().concat(rhs.qvars()),
            Self::Not(pred) => pred.qvars(),
        }
    }

    pub fn has_qvar(&self) -> bool {
        match self {
            Self::Value(_) => false,
            Self::Const(_) => false,
            Self::Call { receiver, args, .. } => {
                receiver.has_qvar() || args.iter().any(|a| a.has_qvar())
            }
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.has_qvar(),
            Self::GeneralEqual { lhs, rhs }
            | Self::GeneralLessEqual { lhs, rhs }
            | Self::GeneralGreaterEqual { lhs, rhs }
            | Self::GeneralNotEqual { lhs, rhs } => lhs.has_qvar() || rhs.has_qvar(),
            Self::Or(lhs, rhs) | Self::And(lhs, rhs) => lhs.has_qvar() || rhs.has_qvar(),
            Self::Not(pred) => pred.has_qvar(),
        }
    }

    pub fn has_unbound_var(&self) -> bool {
        match self {
            Self::Value(_) => false,
            Self::Const(_) => false,
            Self::Call { receiver, args, .. } => {
                receiver.has_unbound_var() || args.iter().any(|a| a.has_unbound_var())
            }
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.has_unbound_var(),
            Self::GeneralEqual { lhs, rhs }
            | Self::GeneralLessEqual { lhs, rhs }
            | Self::GeneralGreaterEqual { lhs, rhs }
            | Self::GeneralNotEqual { lhs, rhs } => lhs.has_unbound_var() || rhs.has_unbound_var(),
            Self::Or(lhs, rhs) | Self::And(lhs, rhs) => {
                lhs.has_unbound_var() || rhs.has_unbound_var()
            }
            Self::Not(pred) => pred.has_unbound_var(),
        }
    }

    pub fn has_undoable_linked_var(&self) -> bool {
        match self {
            Self::Value(_) => false,
            Self::Const(_) => false,
            Self::Call { receiver, args, .. } => {
                receiver.has_undoable_linked_var()
                    || args.iter().any(|a| a.has_undoable_linked_var())
            }
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => rhs.has_undoable_linked_var(),
            Self::GeneralEqual { lhs, rhs }
            | Self::GeneralLessEqual { lhs, rhs }
            | Self::GeneralGreaterEqual { lhs, rhs }
            | Self::GeneralNotEqual { lhs, rhs } => {
                lhs.has_undoable_linked_var() || rhs.has_undoable_linked_var()
            }
            Self::Or(lhs, rhs) | Self::And(lhs, rhs) => {
                lhs.has_undoable_linked_var() || rhs.has_undoable_linked_var()
            }
            Self::Not(pred) => pred.has_undoable_linked_var(),
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
            // REVIEW: Should the receiver be included?
            Self::Call { args, .. } => {
                let mut vec = vec![];
                vec.extend(args);
                vec
            }
            Self::Equal { rhs, .. }
            | Self::GreaterEqual { rhs, .. }
            | Self::LessEqual { rhs, .. }
            | Self::NotEqual { rhs, .. } => vec![rhs],
            Self::GeneralEqual { .. }
            | Self::GeneralLessEqual { .. }
            | Self::GeneralGreaterEqual { .. }
            | Self::GeneralNotEqual { .. } => vec![],
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.typarams().into_iter().chain(rhs.typarams()).collect()
            }
            Self::Not(pred) => pred.typarams(),
        }
    }

    pub fn invert(self) -> Self {
        match self {
            Self::Value(ValueObj::Bool(b)) => Self::Value(ValueObj::Bool(!b)),
            Self::Equal { lhs, rhs } => Self::ne(lhs, rhs),
            Self::GreaterEqual { lhs, rhs } => Self::lt(lhs, rhs),
            Self::LessEqual { lhs, rhs } => Self::gt(lhs, rhs),
            Self::NotEqual { lhs, rhs } => Self::eq(lhs, rhs),
            Self::Not(pred) => *pred,
            other => Self::Not(Box::new(other)),
        }
    }

    pub fn possible_tps(&self) -> Vec<&TyParam> {
        match self {
            Self::Or(lhs, rhs) => [lhs.possible_tps(), rhs.possible_tps()].concat(),
            Self::Equal { rhs, .. } => vec![rhs],
            _ => vec![],
        }
    }

    pub fn possible_values(&self) -> Vec<&ValueObj> {
        match self {
            // Self::GeneralEqual { lhs, rhs } => [lhs.values(), rhs.values()].concat(),
            Self::Equal {
                rhs: TyParam::Value(value),
                ..
            } => vec![value],
            Self::Or(lhs, rhs) => [lhs.possible_values(), rhs.possible_values()].concat(),
            _ => vec![],
        }
    }
}
