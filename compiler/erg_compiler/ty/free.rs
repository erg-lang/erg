use std::cell::{Ref, RefMut};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;

use erg_common::shared::Shared;
use erg_common::traits::LimitedDisplay;
use erg_common::Str;
use erg_common::{addr, addr_eq};

use super::typaram::TyParam;
use super::Type;

pub type Level = usize;
pub type Id = usize;

// HACK: see doc/compiler/inference.md for details
pub const GENERIC_LEVEL: usize = usize::MAX;

thread_local! {
    static UNBOUND_ID: Shared<usize> = Shared::new(0);
}

pub trait HasLevel {
    fn level(&self) -> Option<Level>;
    fn set_level(&self, lev: Level);
    fn lower(&self, level: Level) {
        if self.level() < Some(level) {
            self.set_level(level);
        }
    }
    fn lift(&self) {
        if let Some(lev) = self.level() {
            self.set_level(lev.saturating_add(1));
        }
    }
    fn generalize(&self) {
        self.set_level(GENERIC_LEVEL);
    }
    fn is_generalized(&self) -> bool {
        self.level() == Some(GENERIC_LEVEL)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constraint {
    // : Type --> (:> Never, <: Obj)
    // :> Sub --> (:> Sub, <: Obj)
    // <: Sup --> (:> Never, <: Sup)
    /// :> Sub, <: Sup
    Sandwiched {
        sub: Type,
        sup: Type,
    },
    // : Int, ...
    TypeOf(Type),
    Uninited,
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl LimitedDisplay for Constraint {
    fn limited_fmt(&self, f: &mut fmt::Formatter<'_>, limit: usize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Sandwiched { sub, sup } => match (sub == &Type::Never, sup == &Type::Obj) {
                (true, true) => {
                    write!(f, ": Type")?;
                    if cfg!(feature = "debug") {
                        write!(f, "(:> Never, <: Obj)")?;
                    }
                    Ok(())
                }
                (true, false) => {
                    write!(f, "<: ")?;
                    sup.limited_fmt(f, limit - 1)?;
                    Ok(())
                }
                (false, true) => {
                    write!(f, ":> ")?;
                    sub.limited_fmt(f, limit - 1)?;
                    Ok(())
                }
                (false, false) => {
                    write!(f, ":> ")?;
                    sub.limited_fmt(f, limit - 1)?;
                    write!(f, ", <: ")?;
                    sup.limited_fmt(f, limit - 1)?;
                    Ok(())
                }
            },
            Self::TypeOf(t) => {
                write!(f, ": ")?;
                t.limited_fmt(f, limit - 1)
            }
            Self::Uninited => write!(f, "<uninited>"),
        }
    }
}

impl Constraint {
    pub const fn new_sandwiched(sub: Type, sup: Type) -> Self {
        Self::Sandwiched { sub, sup }
    }

    pub fn new_type_of(t: Type) -> Self {
        if t == Type::Type {
            Self::new_sandwiched(Type::Never, Type::Obj)
        } else {
            Self::TypeOf(t)
        }
    }

    pub const fn new_subtype_of(sup: Type) -> Self {
        Self::new_sandwiched(Type::Never, sup)
    }

    pub const fn new_supertype_of(sub: Type) -> Self {
        Self::new_sandwiched(sub, Type::Obj)
    }

    pub const fn is_uninited(&self) -> bool {
        matches!(self, Self::Uninited)
    }

    pub fn lift(&self) {
        match self {
            Self::Sandwiched { sub, sup, .. } => {
                sub.lift();
                sup.lift();
            }
            Self::TypeOf(t) => t.lift(),
            Self::Uninited => {}
        }
    }

    pub fn get_type(&self) -> Option<&Type> {
        match self {
            Self::TypeOf(ty) => Some(ty),
            Self::Sandwiched {
                sub: Type::Never,
                sup: Type::Obj,
                ..
            } => Some(&Type::Type),
            _ => None,
        }
    }

    /// :> Sub
    pub fn get_sub(&self) -> Option<&Type> {
        match self {
            Self::Sandwiched { sub, .. } => Some(sub),
            _ => None,
        }
    }

    /// <: Sup
    pub fn get_super(&self) -> Option<&Type> {
        match self {
            Self::Sandwiched { sup, .. } => Some(sup),
            _ => None,
        }
    }

    pub fn get_sub_sup(&self) -> Option<(&Type, &Type)> {
        match self {
            Self::Sandwiched { sub, sup, .. } => Some((sub, sup)),
            _ => None,
        }
    }

    pub fn get_super_mut(&mut self) -> Option<&mut Type> {
        match self {
            Self::Sandwiched { sup, .. } => Some(sup),
            _ => None,
        }
    }
}

pub trait CanbeFree {
    fn unbound_name(&self) -> Option<Str>;
    fn constraint(&self) -> Option<Constraint>;
    fn update_constraint(&self, constraint: Constraint);
}

impl<T: CanbeFree> Free<T> {
    pub fn unbound_name(&self) -> Option<Str> {
        match &*self.borrow() {
            FreeKind::NamedUnbound { name, .. } => Some(name.clone()),
            FreeKind::Unbound { id, .. } => Some(Str::from(format!("%{id}"))),
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t.unbound_name(),
        }
    }

    pub fn constraint(&self) -> Option<Constraint> {
        match &*self.borrow() {
            FreeKind::Unbound { constraint, .. } | FreeKind::NamedUnbound { constraint, .. } => {
                Some(constraint.clone())
            }
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t.constraint(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreeKind<T> {
    Linked(T),
    UndoableLinked {
        t: T,
        previous: Box<FreeKind<T>>,
    },
    Unbound {
        id: Id,
        lev: Level,
        constraint: Constraint,
    },
    NamedUnbound {
        name: Str,
        lev: Level,
        constraint: Constraint,
    },
}

impl<T: Hash> Hash for FreeKind<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Linked(t) | Self::UndoableLinked { t, .. } => t.hash(state),
            Self::Unbound { .. } | Self::NamedUnbound { .. } => addr!(self).hash(state),
        }
    }
}

impl<T: LimitedDisplay> fmt::Display for FreeKind<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, 10)
    }
}

impl<T: LimitedDisplay> LimitedDisplay for FreeKind<T> {
    fn limited_fmt(&self, f: &mut fmt::Formatter<'_>, limit: usize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Linked(t) | Self::UndoableLinked { t, .. } => {
                if cfg!(feature = "debug") {
                    write!(f, "(")?;
                    t.limited_fmt(f, limit)?;
                    write!(f, ")")
                } else {
                    t.limited_fmt(f, limit)
                }
            }
            Self::NamedUnbound {
                name,
                lev,
                constraint,
            } => {
                if *lev == GENERIC_LEVEL {
                    write!(f, "{name}")?;
                    if cfg!(feature = "debug") {
                        write!(f, "(")?;
                        constraint.limited_fmt(f, limit - 1)?;
                        write!(f, ")")?;
                    }
                } else {
                    write!(f, "?{name}")?;
                    if cfg!(feature = "debug") {
                        write!(f, "(")?;
                        constraint.limited_fmt(f, limit - 1)?;
                        write!(f, ")")?;
                        write!(f, "[{lev}]")?;
                    }
                }
                Ok(())
            }
            Self::Unbound {
                id,
                lev,
                constraint,
            } => {
                if *lev == GENERIC_LEVEL {
                    write!(f, "%{id}")?;
                    if cfg!(feature = "debug") {
                        write!(f, "(")?;
                        constraint.limited_fmt(f, limit - 1)?;
                        write!(f, ")")?;
                    }
                } else {
                    write!(f, "?{id}")?;
                    if cfg!(feature = "debug") {
                        write!(f, "(")?;
                        constraint.limited_fmt(f, limit - 1)?;
                        write!(f, ")")?;
                        write!(f, "[{lev}]")?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl<T> FreeKind<T> {
    pub const fn unbound(id: Id, lev: Level, constraint: Constraint) -> Self {
        Self::Unbound {
            id,
            lev,
            constraint,
        }
    }

    pub fn new_unbound(lev: Level, constraint: Constraint) -> Self {
        UNBOUND_ID.with(|id| {
            *id.borrow_mut() += 1;
            Self::Unbound {
                id: *id.borrow(),
                lev,
                constraint,
            }
        })
    }

    pub const fn named_unbound(name: Str, lev: Level, constraint: Constraint) -> Self {
        Self::NamedUnbound {
            name,
            lev,
            constraint,
        }
    }

    pub const fn linked(&self) -> Option<&T> {
        match self {
            Self::Linked(typ) => Some(typ),
            _ => None,
        }
    }

    pub fn linked_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Linked(typ) => Some(typ),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Free<T>(Shared<FreeKind<T>>);

impl<T: LimitedDisplay> fmt::Display for Free<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.borrow())
    }
}

impl<T: LimitedDisplay> LimitedDisplay for Free<T> {
    fn limited_fmt(&self, f: &mut fmt::Formatter<'_>, limit: usize) -> fmt::Result {
        self.0.borrow().limited_fmt(f, limit)
    }
}

impl<T> Free<T> {
    pub fn borrow(&self) -> Ref<'_, FreeKind<T>> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<'_, FreeKind<T>> {
        self.0.borrow_mut()
    }
    /// very unsafe, use `force_replace` instead whenever possible
    pub fn as_ptr(&self) -> *mut FreeKind<T> {
        self.0.as_ptr()
    }
    pub fn forced_as_ref(&self) -> &FreeKind<T> {
        unsafe { self.as_ptr().as_ref() }.unwrap()
    }
    pub fn force_replace(&self, new: FreeKind<T>) {
        // prevent linking to self
        if addr_eq!(*self.borrow(), new) {
            return;
        }
        unsafe {
            *self.0.as_ptr() = new;
        }
    }
    pub fn can_borrow(&self) -> bool {
        self.0.can_borrow()
    }
    pub fn can_borrow_mut(&self) -> bool {
        self.0.can_borrow_mut()
    }
}

impl<T: Clone> Free<T> {
    pub fn clone_inner(&self) -> FreeKind<T> {
        self.0.clone_inner()
    }
}

impl<T: HasLevel> HasLevel for Free<T> {
    fn set_level(&self, level: Level) {
        match unsafe { &mut *self.as_ptr() as &mut FreeKind<T> } {
            FreeKind::Unbound { lev, .. } | FreeKind::NamedUnbound { lev, .. } => {
                if addr_eq!(*lev, level) {
                    return;
                }
                *lev = level;
            }
            FreeKind::Linked(t) => {
                t.set_level(level);
            }
            _ => {}
        }
    }

    fn level(&self) -> Option<Level> {
        match &*self.borrow() {
            FreeKind::Unbound { lev, .. } | FreeKind::NamedUnbound { lev, .. } => Some(*lev),
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t.level(),
        }
    }
}

impl<T: Clone + HasLevel + LimitedDisplay> Free<T> {
    pub fn new(f: FreeKind<T>) -> Self {
        Self(Shared::new(f))
    }

    pub fn new_unbound(level: Level, constraint: Constraint) -> Self {
        UNBOUND_ID.with(|id| {
            *id.borrow_mut() += 1;
            Self(Shared::new(FreeKind::unbound(
                *id.borrow(),
                level,
                constraint,
            )))
        })
    }

    pub fn new_named_unbound(name: Str, level: Level, constraint: Constraint) -> Self {
        Self(Shared::new(FreeKind::named_unbound(
            name, level, constraint,
        )))
    }

    pub fn new_linked(t: T) -> Self {
        Self(Shared::new(FreeKind::Linked(t)))
    }

    pub fn link(&self, to: &T) {
        // prevent linking to self
        if self.is_linked() && addr_eq!(*self.crack(), *to) {
            return;
        }
        *self.borrow_mut() = FreeKind::Linked(to.clone());
    }

    pub fn replace(&self, to: FreeKind<T>) {
        // prevent linking to self
        if self.is_linked() && addr_eq!(*self.borrow(), to) {
            return;
        }
        *self.borrow_mut() = to;
    }

    /// NOTE: Do not use this except to rewrite circular references.
    /// No reference to any type variable may be left behind when rewriting.
    /// However, `get_bound_types` is safe because it does not return references.
    pub fn forced_link(&self, to: &T) {
        // prevent linking to self
        if self.is_linked() && addr_eq!(*self.crack(), *to) {
            return;
        }
        unsafe {
            *self.as_ptr() = FreeKind::Linked(to.clone());
        }
    }

    pub fn undoable_link(&self, to: &T) {
        if self.is_linked() && addr_eq!(*self.crack(), *to) {
            panic!("link to self");
        }
        let prev = self.clone_inner();
        let new = FreeKind::UndoableLinked {
            t: to.clone(),
            previous: Box::new(prev),
        };
        *self.borrow_mut() = new;
    }

    /// NOTE: Do not use this except to rewrite circular references.
    /// No reference to any type variable may be left behind when rewriting.
    /// However, `get_bound_types` is safe because it does not return references.
    pub fn forced_undoable_link(&self, to: &T) {
        if self.is_linked() && addr_eq!(*self.crack(), *to) {
            panic!("link to self");
        }
        let prev = self.clone_inner();
        let new = FreeKind::UndoableLinked {
            t: to.clone(),
            previous: Box::new(prev),
        };
        self.force_replace(new);
    }

    pub fn undo(&self) {
        match &*self.borrow() {
            FreeKind::UndoableLinked { previous, .. } => {
                let prev = *previous.clone();
                self.force_replace(prev);
            }
            other => panic!("cannot undo: {other}"),
        }
    }

    pub fn unwrap_unbound(self) -> (Option<Str>, usize, Constraint) {
        match self.clone_inner() {
            FreeKind::Linked(_) | FreeKind::UndoableLinked { .. } => panic!("the value is linked"),
            FreeKind::Unbound {
                constraint, lev, ..
            } => (None, lev, constraint),
            FreeKind::NamedUnbound {
                name,
                lev,
                constraint,
            } => (Some(name), lev, constraint),
        }
    }

    pub fn unwrap_linked(self) -> T {
        match self.clone_inner() {
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t,
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {
                panic!("the value is unbounded")
            }
        }
    }

    pub fn detach(&self) -> Self {
        match self.clone().unwrap_unbound() {
            (Some(name), lev, constraint) => Self::new_named_unbound(name, lev, constraint),
            (None, lev, constraint) => Self::new_unbound(lev, constraint),
        }
    }

    /// returns linked type (panic if self is unbounded)
    /// NOTE: check by `.is_linked` before call
    pub fn crack(&self) -> Ref<'_, T> {
        Ref::map(self.borrow(), |f| match f {
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t,
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {
                panic!("the value is unbounded")
            }
        })
    }

    pub fn crack_constraint(&self) -> Ref<'_, Constraint> {
        Ref::map(self.borrow(), |f| match f {
            FreeKind::Linked(_) | FreeKind::UndoableLinked { .. } => panic!("the value is linked"),
            FreeKind::Unbound { constraint, .. } | FreeKind::NamedUnbound { constraint, .. } => {
                constraint
            }
        })
    }

    pub fn is_linked(&self) -> bool {
        matches!(
            &*self.borrow(),
            FreeKind::Linked(_) | FreeKind::UndoableLinked { .. }
        )
    }

    pub fn is_undoable_linked(&self) -> bool {
        matches!(&*self.borrow(), FreeKind::UndoableLinked { .. })
    }
}

impl<T: CanbeFree> Free<T> {
    pub fn get_type(&self) -> Option<Type> {
        self.constraint().and_then(|c| c.get_type().cloned())
    }

    /// <: Super
    pub fn get_super(&self) -> Option<Type> {
        self.constraint().and_then(|c| c.get_super().cloned())
    }

    /// :> Sub
    pub fn get_sub(&self) -> Option<Type> {
        self.constraint().and_then(|c| c.get_sub().cloned())
    }

    pub fn get_subsup(&self) -> Option<(Type, Type)> {
        self.constraint()
            .and_then(|c| c.get_sub_sup().map(|(sub, sup)| (sub.clone(), sup.clone())))
    }

    pub fn is_unbound(&self) -> bool {
        matches!(
            &*self.borrow(),
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. }
        )
    }

    pub fn constraint_is_typeof(&self) -> bool {
        self.constraint()
            .map(|c| c.get_type().is_some())
            .unwrap_or(false)
    }

    pub fn constraint_is_sandwiched(&self) -> bool {
        self.constraint()
            .map(|c| c.get_sub_sup().is_some())
            .unwrap_or(false)
    }

    pub fn constraint_is_uninited(&self) -> bool {
        self.constraint().map(|c| c.is_uninited()).unwrap_or(false)
    }

    pub fn update_constraint(&self, new_constraint: Constraint) {
        match unsafe { &mut *self.as_ptr() as &mut FreeKind<T> } {
            FreeKind::Unbound { constraint, .. } | FreeKind::NamedUnbound { constraint, .. } => {
                if addr_eq!(*constraint, new_constraint) {
                    return;
                }
                *constraint = new_constraint;
            }
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                t.update_constraint(new_constraint);
            }
        }
    }
}

impl Free<TyParam> {
    pub fn map<F>(&self, f: F)
    where
        F: Fn(TyParam) -> TyParam,
    {
        match unsafe { &mut *self.as_ptr() as &mut FreeKind<TyParam> } {
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {
                panic!("the value is unbounded")
            }
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                *t = f(mem::take(t));
            }
        }
    }
}

pub type FreeTyVar = Free<Type>;
pub type FreeTyParam = Free<TyParam>;
