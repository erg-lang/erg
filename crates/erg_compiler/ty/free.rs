use std::cell::{BorrowError, BorrowMutError, Ref, RefMut};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::Arc;

use erg_common::consts::DEBUG_MODE;
use erg_common::fxhash::FxHasher;
use erg_common::shared::Forkable;
use erg_common::traits::{LimitedDisplay, StructuralEq};
use erg_common::Str;
use erg_common::{addr, addr_eq, log};

use super::typaram::TyParam;
use super::Type;

pub type Level = usize;
pub type Id = usize;

/// HACK: see doc/compiler/inference.md for details
pub const GENERIC_LEVEL: usize = usize::MAX;
static UNBOUND_ID: AtomicUsize = AtomicUsize::new(0);

pub trait HasLevel {
    fn level(&self) -> Option<Level>;
    fn set_level(&self, lev: Level);
    fn set_lower(&self, level: Level) {
        if self.level() < Some(level) {
            self.set_level(level);
        }
    }
    fn lift(&self) {
        if let Some(lev) = self.level() {
            self.set_level(lev.saturating_add(1));
        }
    }
    fn lower(&self) {
        if let Some(lev) = self.level() {
            self.set_level(lev.saturating_sub(1));
        }
    }
    fn generalize(&self) {
        self.set_level(GENERIC_LEVEL);
    }
    fn is_generalized(&self) -> bool {
        self.level() == Some(GENERIC_LEVEL)
    }
}

/// Represents constraints on type variables and type parameters.
///
/// Note that constraints can have circular references. However, type variable (`FreeTyVar`) is defined with various operations avoiding infinite recursion.
///
/// __NOTE__: you should use `Free::get_type/get_subsup` instead of deconstructing the constraint by `match`.
/// Constraints may contain cycles, in which case using `match` to get the contents will cause memory destructions.
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
        self.limited_fmt(f, <Self as LimitedDisplay>::DEFAULT_LIMIT)
    }
}

impl LimitedDisplay for Constraint {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Sandwiched { sub, sup } => match (sub == &Type::Never, sup == &Type::Obj) {
                (true, true) => {
                    write!(f, ": Type")?;
                    if DEBUG_MODE {
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
    /// :> Sub, <: Sup
    pub const fn new_sandwiched(sub: Type, sup: Type) -> Self {
        Self::Sandwiched { sub, sup }
    }

    pub fn named_fmt(&self, f: &mut impl fmt::Write, name: &str, limit: isize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Sandwiched { sub, sup } => match (sub == &Type::Never, sup == &Type::Obj) {
                (true, true) => {
                    write!(f, "{name}: Type")?;
                    Ok(())
                }
                (true, false) => {
                    write!(f, "{name} <: ")?;
                    sup.limited_fmt(f, limit - 1)?;
                    Ok(())
                }
                (false, true) => {
                    write!(f, "{name} :> ")?;
                    sub.limited_fmt(f, limit - 1)?;
                    Ok(())
                }
                (false, false) => {
                    write!(f, "{name} :> ")?;
                    sub.limited_fmt(f, limit - 1)?;
                    write!(f, ", {name} <: ")?;
                    sup.limited_fmt(f, limit - 1)?;
                    Ok(())
                }
            },
            Self::TypeOf(t) => {
                write!(f, "{name}: ")?;
                t.limited_fmt(f, limit - 1)
            }
            Self::Uninited => write!(f, "Never"),
        }
    }

    pub fn new_type_of(t: Type) -> Self {
        if t == Type::Type {
            Self::new_sandwiched(Type::Never, Type::Obj)
        } else {
            Self::TypeOf(t)
        }
    }

    /// <: sup
    pub const fn new_subtype_of(sup: Type) -> Self {
        Self::new_sandwiched(Type::Never, sup)
    }

    /// :> sub
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

    /// :> Sub, <: Sup
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

    /// e.g.
    /// ```erg
    /// old_sub: ?T, constraint: (:> ?T or NoneType, <: Obj)
    /// -> constraint: (:> NoneType, <: Obj)
    /// ```
    pub fn eliminate_subsup_recursion(self, target: &Type) -> Self {
        match self {
            Self::Sandwiched { sub, sup } => {
                if sub.addr_eq(target) && sup.addr_eq(target) {
                    Self::new_type_of(Type::Type)
                } else if sub.addr_eq(target) {
                    let sup = sup.eliminate_subsup(target);
                    Self::new_subtype_of(sup)
                } else if sup.addr_eq(target) {
                    let sub = sub.eliminate_subsup(target);
                    Self::new_supertype_of(sub)
                } else {
                    let sub = sub.eliminate_subsup(target);
                    let sup = sup.eliminate_subsup(target);
                    Self::new_sandwiched(sub, sup)
                }
            }
            Self::TypeOf(t) => Self::new_type_of(t.eliminate_subsup(target)),
            other => other,
        }
    }

    pub fn to_type_constraint(self) -> Constraint {
        match self {
            Self::TypeOf(Type::Type) => Constraint::new_sandwiched(Type::Never, Type::Obj),
            _ => self,
        }
    }
}

pub trait CanbeFree {
    fn unbound_name(&self) -> Option<Str>;
    fn constraint(&self) -> Option<Constraint>;
    fn destructive_update_constraint(&self, constraint: Constraint, in_instantiation: bool);
}

impl<T: CanbeFree + Send + Clone> Free<T> {
    pub fn unbound_name(&self) -> Option<Str> {
        self.borrow().unbound_name()
    }

    pub fn constraint(&self) -> Option<Constraint> {
        self.borrow().constraint()
    }
}

#[derive(Debug, Clone, Eq)]
pub enum FreeKind<T> {
    Linked(T),
    UndoableLinked {
        t: T,
        previous: Box<FreeKind<T>>,
        count: usize,
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
            Self::Unbound {
                id,
                lev,
                constraint,
            } => {
                id.hash(state);
                lev.hash(state);
                constraint.hash(state);
            }
            Self::NamedUnbound {
                name,
                lev,
                constraint,
            } => {
                name.hash(state);
                lev.hash(state);
                constraint.hash(state);
            }
        }
    }
}

impl<T: PartialEq> PartialEq for FreeKind<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Linked(t1) | Self::UndoableLinked { t: t1, .. },
                Self::Linked(t2) | Self::UndoableLinked { t: t2, .. },
            ) => t1 == t2,
            (
                Self::Unbound {
                    id: id1,
                    lev: lev1,
                    constraint: c1,
                },
                Self::Unbound {
                    id: id2,
                    lev: lev2,
                    constraint: c2,
                },
            ) => id1 == id2 && lev1 == lev2 && c1 == c2,
            (
                Self::NamedUnbound {
                    name: n1,
                    lev: l1,
                    constraint: c1,
                },
                Self::NamedUnbound {
                    name: n2,
                    lev: l2,
                    constraint: c2,
                },
            ) => n1 == n2 && l1 == l2 && c1 == c2,
            _ => false,
        }
    }
}

impl<T: CanbeFree> FreeKind<T> {
    pub fn unbound_name(&self) -> Option<Str> {
        match self {
            FreeKind::NamedUnbound { name, .. } => Some(name.clone()),
            FreeKind::Unbound { id, .. } => Some(Str::from(format!("%{id}"))),
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t.unbound_name(),
        }
    }

    pub fn constraint(&self) -> Option<Constraint> {
        match self {
            FreeKind::Unbound { constraint, .. } | FreeKind::NamedUnbound { constraint, .. } => {
                Some(constraint.clone())
            }
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t.constraint(),
        }
    }
}

impl<T: LimitedDisplay> fmt::Display for FreeKind<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.limited_fmt(f, <Self as LimitedDisplay>::DEFAULT_LIMIT)
    }
}

impl<T: LimitedDisplay> LimitedDisplay for FreeKind<T> {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> fmt::Result {
        if limit == 0 {
            return write!(f, "...");
        }
        match self {
            Self::Linked(t) | Self::UndoableLinked { t, .. } => {
                if DEBUG_MODE {
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
                    if DEBUG_MODE {
                        write!(f, "(")?;
                        constraint.limited_fmt(f, limit - 1)?;
                        write!(f, ")")?;
                    }
                } else {
                    write!(f, "?{name}")?;
                    if DEBUG_MODE {
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
                    if DEBUG_MODE {
                        write!(f, "(")?;
                        constraint.limited_fmt(f, limit - 1)?;
                        write!(f, ")")?;
                    }
                } else {
                    write!(f, "?{id}")?;
                    if DEBUG_MODE {
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
        UNBOUND_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self::Unbound {
            id: UNBOUND_ID.load(std::sync::atomic::Ordering::SeqCst),
            lev,
            constraint,
        }
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
            Self::Linked(t) | Self::UndoableLinked { t, .. } => Some(t),
            _ => None,
        }
    }

    pub fn linked_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Linked(t) | Self::UndoableLinked { t, .. } => Some(t),
            _ => None,
        }
    }

    /// SAFETY: carefully ensure that `to` is not a freevar equal to `self`
    ///
    /// e.g.
    /// ```erg
    /// x = ?T
    /// x.replace(Type::Free(?T))
    /// x == (((...)))
    /// ```
    pub fn replace(&mut self, to: T) {
        match self {
            // REVIEW: What if `t` is also an unbound variable?
            Self::Linked(t) | Self::UndoableLinked { t, .. } => {
                *t = to;
            }
            _ => {
                *self = Self::Linked(to);
            }
        }
    }

    pub const fn is_named_unbound(&self) -> bool {
        matches!(self, Self::NamedUnbound { .. })
    }

    pub const fn is_unnamed_unbound(&self) -> bool {
        matches!(self, Self::Unbound { .. })
    }

    pub const fn is_undoable_linked(&self) -> bool {
        matches!(self, Self::UndoableLinked { .. })
    }

    pub fn undo_count(&self) -> usize {
        match self {
            Self::UndoableLinked { count, .. } => *count,
            _ => 0,
        }
    }

    pub fn inc_undo_count(&mut self) {
        #[allow(clippy::single_match)]
        match self {
            Self::UndoableLinked { count, .. } => *count += 1,
            _ => {}
        }
    }

    pub fn dec_undo_count(&mut self) {
        #[allow(clippy::single_match)]
        match self {
            Self::UndoableLinked { count, .. } => *count -= 1,
            _ => {}
        }
    }

    pub fn get_previous(&self) -> Option<&FreeKind<T>> {
        match self {
            Self::UndoableLinked { previous, .. } => Some(previous),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Free<T: Send + Clone> {
    value: Forkable<FreeKind<T>>,
    hash_cache: Arc<AtomicU64>,
}

impl Hash for Free<Type> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(cache) = self.get_hash_cache() {
            state.write_u64(cache);
            return;
        }
        let mut hasher = FxHasher::default();
        let hasher = &mut hasher;
        if let Some(name) = self.unbound_name() {
            name.hash(hasher);
        }
        if let Some(lev) = self.level() {
            lev.hash(hasher);
        }
        if let Some((sub, sup)) = self.get_subsup() {
            self.do_avoiding_recursion(|| {
                sub.hash(hasher);
                sup.hash(hasher);
            });
        } else if let Some(t) = self.get_type() {
            t.hash(hasher);
        } else if self.is_linked() {
            let cracked = self.crack();
            if !Type::FreeVar(self.clone()).addr_eq(&cracked) {
                cracked.hash(hasher);
            } else {
                addr!(self).hash(hasher);
            }
        }
        let hash = hasher.finish();
        self.hash_cache
            .store(hash, std::sync::atomic::Ordering::Relaxed);
        state.write_u64(hash);
    }
}

impl Hash for Free<TyParam> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(name) = self.unbound_name() {
            name.hash(state);
        }
        if let Some(lev) = self.level() {
            lev.hash(state);
        }
        if let Some(t) = self.get_type() {
            t.hash(state);
        } else if self.is_linked() {
            self.crack().hash(state);
        }
    }
}

impl PartialEq for Free<Type> {
    fn eq(&self, other: &Self) -> bool {
        let linked = self.linked_free();
        let this = if let Some(linked) = &linked {
            linked
        } else {
            self
        };
        let linked = other.linked_free();
        let other = if let Some(linked) = &linked {
            linked
        } else {
            other
        };
        if let Some(self_name) = this.unbound_name() {
            if let Some(other_name) = other.unbound_name() {
                if self_name != other_name {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(self_lev) = this.level() {
            if let Some(other_lev) = other.level() {
                if self_lev != other_lev {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some((sub, sup)) = this.get_subsup() {
            if let Some((other_sub, other_sup)) = other.get_subsup() {
                this.dummy_link();
                other.dummy_link();
                let res = sub == other_sub && sup == other_sup;
                this.undo();
                other.undo();
                res
            } else {
                false
            }
        } else if let Some(self_t) = this.get_type() {
            if let Some(other_t) = other.get_type() {
                self_t == other_t
            } else {
                false
            }
        } else if this.is_linked() {
            if other.is_linked() {
                this.crack().eq(&other.crack())
            } else {
                false
            }
        } else {
            // name, level, constraint are equal
            true
        }
    }
}

impl PartialEq for Free<TyParam> {
    fn eq(&self, other: &Self) -> bool {
        let linked = self.linked_free();
        let this = if let Some(linked) = &linked {
            linked
        } else {
            self
        };
        let linked = other.linked_free();
        let other = if let Some(linked) = &linked {
            linked
        } else {
            other
        };
        if let Some(self_name) = this.unbound_name() {
            if let Some(other_name) = other.unbound_name() {
                if self_name != other_name {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(self_lev) = this.level() {
            if let Some(other_lev) = other.level() {
                if self_lev != other_lev {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(self_t) = this.get_type() {
            if let Some(other_t) = other.get_type() {
                self_t == other_t
            } else {
                false
            }
        } else if this.is_linked() {
            if other.is_linked() {
                this.crack().eq(&other.crack())
            } else {
                false
            }
        } else {
            // name, level, constraint are equal
            true
        }
    }
}

impl Eq for Free<Type> {}
impl Eq for Free<TyParam> {}

impl<T: LimitedDisplay + Send + Clone> fmt::Display for Free<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value.borrow())
    }
}

impl<T: LimitedDisplay + Send + Clone> LimitedDisplay for Free<T> {
    fn limited_fmt<W: std::fmt::Write>(&self, f: &mut W, limit: isize) -> fmt::Result {
        self.value.borrow().limited_fmt(f, limit)
    }
}

impl<T: Send + Clone> Free<T> {
    #[track_caller]
    pub fn borrow(&self) -> Ref<'_, FreeKind<T>> {
        self.value.borrow()
    }
    #[track_caller]
    pub fn borrow_mut(&self) -> RefMut<'_, FreeKind<T>> {
        self.value.borrow_mut()
    }
    #[track_caller]
    pub fn try_borrow(&self) -> Result<Ref<'_, FreeKind<T>>, BorrowError> {
        self.value.try_borrow()
    }
    #[track_caller]
    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, FreeKind<T>>, BorrowMutError> {
        self.value.try_borrow_mut()
    }
    /// very unsafe, use `force_replace` instead whenever possible
    pub fn as_ptr(&self) -> *mut FreeKind<T> {
        self.value.as_ptr()
    }
    pub fn forced_as_ref(&self) -> &FreeKind<T> {
        unsafe { self.as_ptr().as_ref() }.unwrap()
    }
    pub fn forced_as_mut(&mut self) -> &mut FreeKind<T> {
        unsafe { self.as_ptr().as_mut() }.unwrap()
    }
}

impl Free<Type> {
    /// (T) => T
    /// ((T)) => T
    pub fn linked_free(&self) -> Option<Free<Type>> {
        let linked = self.get_linked()?;
        let fv = linked.as_free()?;
        if let Some(fv) = fv.linked_free() {
            Some(fv)
        } else {
            Some(self.clone())
        }
    }

    pub fn is_recursive(&self) -> bool {
        Type::FreeVar(self.clone()).is_recursive()
    }

    pub fn get_hash_cache(&self) -> Option<u64> {
        if let Some(linked) = self.get_linked() {
            linked.tyvar_hash_cache()?;
        } else if let Some((sub, sup)) = self.get_subsup() {
            sub.tyvar_hash_cache()
                .and_then(|_| sup.tyvar_hash_cache())?;
        }
        let cache = self.hash_cache.load(std::sync::atomic::Ordering::Relaxed);
        if cache == 0 {
            None
        } else {
            Some(cache)
        }
    }

    /// interior-mut
    fn _do_avoiding_recursion<O, F: FnOnce() -> O>(&self, placeholder: Option<&Type>, f: F) -> O {
        let placeholder = placeholder.unwrap_or(&Type::Failure);
        let is_recursive = self.is_recursive();
        if is_recursive {
            let target = Type::FreeVar(self.clone());
            let placeholder_ = placeholder
                .clone()
                .eliminate_subsup(&target)
                .eliminate_recursion(&target);
            self.undoable_link(&placeholder_);
        }
        let res = f();
        if is_recursive {
            self.undo();
        }
        res
    }

    /// interior-mut
    pub fn do_avoiding_recursion<O>(&self, f: impl FnOnce() -> O) -> O {
        self._do_avoiding_recursion(None, f)
    }

    /// interior-mut
    pub fn do_avoiding_recursion_with<O>(&self, placeholder: &Type, f: impl FnOnce() -> O) -> O {
        self._do_avoiding_recursion(Some(placeholder), f)
    }

    pub fn has_unbound_var(&self) -> bool {
        if self.is_unbound() {
            true
        } else {
            self.crack().has_unbound_var()
        }
    }

    /// interior-mut
    /// if `in_inst_or_gen` is true, constraint will be updated forcibly
    pub(super) fn update_constraint(&self, new_constraint: Constraint, in_inst_or_gen: bool) {
        if new_constraint.get_type() == Some(&Type::Never) {
            panic!("{new_constraint}");
        }
        // self: T
        // new_constraint: (:> T, <: U) => <: U
        if new_constraint.get_sub_sup().is_some_and(|(sub, sup)| {
            sub.contains_tvar_in_constraint(self) || sup.contains_tvar_in_constraint(self)
        }) {
            return;
        }
        match &mut *self.borrow_mut() {
            FreeKind::Unbound {
                lev, constraint, ..
            }
            | FreeKind::NamedUnbound {
                lev, constraint, ..
            } => {
                if !in_inst_or_gen && *lev == GENERIC_LEVEL {
                    log!(err "cannot update the constraint of a generalized type variable");
                    return;
                }
                if addr_eq!(*constraint, new_constraint) {
                    return;
                }
                *constraint = new_constraint;
                self.clear_hash_cache();
            }
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                t.destructive_update_constraint(new_constraint, in_inst_or_gen);
            }
        }
    }
}

impl Free<TyParam> {
    /// (T) => T
    /// ((T)) => T
    pub fn linked_free(&self) -> Option<Free<TyParam>> {
        let linked = self.get_linked()?;
        let fv = linked.as_free()?;
        if let Some(fv) = fv.linked_free() {
            Some(fv)
        } else {
            Some(self.clone())
        }
    }

    pub fn is_recursive(&self) -> bool {
        TyParam::FreeVar(self.clone()).is_recursive()
    }

    fn _do_avoiding_recursion<O, F: FnOnce() -> O>(
        &self,
        placeholder: Option<&TyParam>,
        f: F,
    ) -> O {
        let placeholder = placeholder.unwrap_or(&TyParam::Failure);
        let is_recursive = self.is_recursive();
        if is_recursive {
            let target = TyParam::FreeVar(self.clone());
            let placeholder_ = placeholder.clone().eliminate_recursion(&target);
            self.undoable_link(&placeholder_);
        }
        let res = f();
        if is_recursive {
            self.undo();
        }
        res
    }

    pub fn do_avoiding_recursion<O, F: FnOnce() -> O>(&self, f: F) -> O {
        self._do_avoiding_recursion(None, f)
    }

    pub fn do_avoiding_recursion_with<O, F: FnOnce() -> O>(
        &self,
        placeholder: &TyParam,
        f: F,
    ) -> O {
        self._do_avoiding_recursion(Some(placeholder), f)
    }

    pub fn has_unbound_var(&self) -> bool {
        if self.is_unbound() {
            true
        } else {
            self.crack().has_unbound_var()
        }
    }
}

impl<T: StructuralEq + CanbeFree + Clone + Default + fmt::Debug + Send + Sync + 'static>
    StructuralEq for Free<T>
{
    fn structural_eq(&self, other: &Self) -> bool {
        if let (Some((l, r)), Some((l2, r2))) = (self.get_subsup(), other.get_subsup()) {
            self.dummy_link();
            other.dummy_link();
            let res = l.structural_eq(&l2) && r.structural_eq(&r2);
            self.undo();
            other.undo();
            res
        } else if let (Some(l), Some(r)) = (self.get_type(), other.get_type()) {
            l.structural_eq(&r)
        } else {
            self.constraint_is_uninited() && other.constraint_is_uninited()
        }
    }
}

impl<T: Send + Clone> Free<T> {
    pub fn clone_inner(&self) -> FreeKind<T> {
        self.value.clone_inner()
    }

    pub fn update_init(&mut self) {
        self.value.update_init();
    }
}

impl HasLevel for Free<Type> {
    fn set_level(&self, level: Level) {
        match &mut *self.borrow_mut() {
            FreeKind::Unbound { lev, .. } | FreeKind::NamedUnbound { lev, .. } => {
                if addr_eq!(*lev, level) {
                    return;
                }
                *lev = level;
                self.clear_hash_cache();
            }
            _ => {}
        }
        if let Some(linked) = self.get_linked() {
            linked.set_level(level);
        } else if let Some((sub, sup)) = self.get_subsup() {
            self.do_avoiding_recursion(|| {
                sub.set_level(level);
                sup.set_level(level);
            });
        } else if let Some(t) = self.get_type() {
            t.set_level(level);
        }
    }

    fn level(&self) -> Option<Level> {
        match &*self.borrow() {
            FreeKind::Unbound { lev, .. } | FreeKind::NamedUnbound { lev, .. } => Some(*lev),
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t.level(),
        }
    }
}

impl HasLevel for Free<TyParam> {
    fn set_level(&self, level: Level) {
        match &mut *self.borrow_mut() {
            FreeKind::Unbound { lev, .. } | FreeKind::NamedUnbound { lev, .. } => {
                if addr_eq!(*lev, level) {
                    return;
                }
                *lev = level;
                self.clear_hash_cache();
            }
            _ => {}
        }
        if let Some(linked) = self.get_linked() {
            linked.set_level(level);
        } else if let Some(t) = self.get_type() {
            t.set_level(level);
        }
    }

    fn level(&self) -> Option<Level> {
        match &*self.borrow() {
            FreeKind::Unbound { lev, .. } | FreeKind::NamedUnbound { lev, .. } => Some(*lev),
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                if t.is_recursive() {
                    None
                } else {
                    t.level()
                }
            }
        }
    }
}

impl<T: Send + Clone + CanbeFree> Free<T>
where
    Free<T>: HasLevel,
{
    pub fn deep_clone(&self) -> Self {
        Self::new_named_unbound(
            self.unbound_name().unwrap(),
            self.level().unwrap(),
            self.constraint().unwrap(),
        )
    }
}

impl<T: Send + Clone> Free<T> {
    pub fn new(f: FreeKind<T>) -> Self {
        Self {
            value: Forkable::new(f),
            hash_cache: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn new_unbound(level: Level, constraint: Constraint) -> Self {
        UNBOUND_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            value: Forkable::new(FreeKind::unbound(
                UNBOUND_ID.load(std::sync::atomic::Ordering::SeqCst),
                level,
                constraint,
            )),
            hash_cache: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn new_named_unbound(name: Str, level: Level, constraint: Constraint) -> Self {
        Self {
            value: Forkable::new(FreeKind::named_unbound(name, level, constraint)),
            hash_cache: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn new_linked(t: T) -> Self {
        Self {
            value: Forkable::new(FreeKind::Linked(t)),
            hash_cache: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn clear_hash_cache(&self) {
        self.hash_cache
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// returns linked type (panic if self is unbounded)
    /// NOTE: check by `.is_linked` before call
    /// NOTE: Sometimes a `BorrowMut` error occurs when trying to pass the result of `crack().clone()` as an argument.
    /// Bind it to a variable beforehand.
    #[track_caller]
    pub fn crack(&self) -> Ref<'_, T> {
        Ref::map(self.borrow(), |f| match f {
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t,
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {
                panic!("the value is unbounded")
            }
        })
    }

    #[track_caller]
    pub fn crack_constraint(&self) -> Ref<'_, Constraint> {
        Ref::map(self.borrow(), |f| match f {
            FreeKind::Linked(_) | FreeKind::UndoableLinked { .. } => panic!("the value is linked"),
            FreeKind::Unbound { constraint, .. } | FreeKind::NamedUnbound { constraint, .. } => {
                constraint
            }
        })
    }

    pub fn unsafe_crack(&self) -> &T {
        match unsafe { self.as_ptr().as_ref().unwrap() } {
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t,
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {
                panic!("the value is unbounded")
            }
        }
    }

    pub fn addr_eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl<T: Send + Sync + 'static + Clone> Free<T> {
    pub fn is_linked(&self) -> bool {
        self.borrow().linked().is_some()
    }

    pub fn is_undoable_linked(&self) -> bool {
        self.borrow().is_undoable_linked()
    }

    pub fn is_named_unbound(&self) -> bool {
        self.borrow().is_named_unbound()
    }

    pub fn is_unnamed_unbound(&self) -> bool {
        self.borrow().is_unnamed_unbound()
    }

    /// interior-mut
    #[track_caller]
    pub fn replace(&self, to: FreeKind<T>) {
        // prevent linking to self
        if self.is_linked() && addr_eq!(*self.borrow(), to) {
            return;
        }
        *self.borrow_mut() = to;
        self.clear_hash_cache();
    }
}

impl<T: Clone + Send + Sync + 'static> Free<T> {
    /// interior-mut
    /// SAFETY: use `Type/TyParam::link` instead of this.
    /// This method may cause circular references.
    #[track_caller]
    pub(super) fn link(&self, to: &T) {
        // prevent linking to self
        if self.is_linked() && addr_eq!(*self.crack(), *to) {
            return;
        }
        self.borrow_mut().replace(to.clone());
        self.clear_hash_cache();
    }

    /// interior-mut
    #[track_caller]
    pub(super) fn undoable_link(&self, to: &T) {
        if self.is_linked() && addr_eq!(*self.crack(), *to) {
            return;
        }
        let prev = self.clone_inner();
        let new = FreeKind::UndoableLinked {
            t: to.clone(),
            previous: Box::new(prev),
            count: 0,
        };
        *self.borrow_mut() = new;
        self.clear_hash_cache();
    }

    /// interior-mut
    pub fn undo(&self) {
        let prev = match &mut *self.borrow_mut() {
            FreeKind::UndoableLinked {
                previous, count, ..
            } => {
                if *count > 0 {
                    *count -= 1;
                    self.clear_hash_cache();
                    return;
                }
                *previous.clone()
            }
            _other => panic!("cannot undo"),
        };
        self.replace(prev);
        self.clear_hash_cache();
    }

    pub fn undo_stack_size(&self) -> usize {
        self.borrow().undo_count()
    }

    pub fn inc_undo_count(&self) {
        self.borrow_mut().inc_undo_count();
        self.clear_hash_cache();
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

    pub fn unwrap_linked(&self) -> T {
        match self.clone_inner() {
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => t,
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {
                panic!("the value is unbounded")
            }
        }
    }

    pub fn get_linked(&self) -> Option<T> {
        if !self.is_linked() {
            None
        } else {
            Some(self.crack().clone())
        }
    }

    #[track_caller]
    pub fn get_linked_ref(&self) -> Option<Ref<T>> {
        Ref::filter_map(self.borrow(), |f| match f {
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => Some(t),
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => None,
        })
        .ok()
    }

    #[track_caller]
    pub fn get_linked_refmut(&self) -> Option<RefMut<T>> {
        self.clear_hash_cache();
        RefMut::filter_map(self.borrow_mut(), |f| match f {
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => Some(t),
            FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => None,
        })
        .ok()
    }

    #[track_caller]
    pub fn get_previous(&self) -> Option<Ref<Box<FreeKind<T>>>> {
        Ref::filter_map(self.borrow(), |f| match f {
            FreeKind::UndoableLinked { previous, .. } => Some(previous),
            _ => None,
        })
        .ok()
    }

    pub fn get_undoable_root(&self) -> Option<Ref<FreeKind<T>>> {
        let mut prev = Ref::map(self.get_previous()?, |f| f.as_ref());
        loop {
            match Ref::filter_map(prev, |f| f.get_previous()) {
                Ok(p) => prev = p,
                Err(p) => {
                    prev = p;
                    break;
                }
            }
        }
        Some(prev)
    }

    pub fn detach(&self) -> Self {
        match self.clone().unwrap_unbound() {
            (Some(name), lev, constraint) => Self::new_named_unbound(name, lev, constraint),
            (None, lev, constraint) => Self::new_unbound(lev, constraint),
        }
    }
}

impl<T: Default + Clone + fmt::Debug + Send + Sync + 'static> Free<T> {
    /// interior-mut
    #[track_caller]
    pub fn dummy_link(&self) {
        self.undoable_link(&T::default());
    }
}

impl<T: CanbeFree + Send + Clone> Free<T> {
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

    /// :> Sub, <: Super
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

    pub fn is_unbound_and_sandwiched(&self) -> bool {
        self.is_unbound() && self.constraint_is_sandwiched()
    }

    pub fn is_unbound_and_typed(&self) -> bool {
        self.is_unbound() && self.constraint_is_typeof()
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
}

impl Free<TyParam> {
    pub fn map(&self, f: impl Fn(TyParam) -> TyParam) {
        if let Some(mut linked) = self.get_linked_refmut() {
            let mapped = f(mem::take(&mut *linked));
            *linked = mapped;
        }
    }

    /// interior-mut
    /// if `in_inst_or_gen` is true, constraint will be updated forcibly
    pub fn update_constraint(&self, new_constraint: Constraint, in_inst_or_gen: bool) {
        if new_constraint.get_type() == Some(&Type::Never) {
            panic!("{new_constraint}");
        }
        match &mut *self.borrow_mut() {
            FreeKind::Unbound {
                lev, constraint, ..
            }
            | FreeKind::NamedUnbound {
                lev, constraint, ..
            } => {
                if !in_inst_or_gen && *lev == GENERIC_LEVEL {
                    log!(err "cannot update the constraint of a generalized type variable");
                    return;
                }
                if addr_eq!(*constraint, new_constraint) {
                    return;
                }
                *constraint = new_constraint;
                self.clear_hash_cache();
            }
            FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                t.destructive_update_constraint(new_constraint, in_inst_or_gen);
            }
        }
    }

    /// interior-mut
    pub fn update_type(&self, new_type: Type) {
        let new_constraint = Constraint::new_type_of(new_type);
        self.update_constraint(new_constraint, true);
    }
}

pub type FreeTyVar = Free<Type>;
pub type FreeTyParam = Free<TyParam>;

mod tests {
    #![allow(unused_imports)]
    use erg_common::enable_overflow_stacktrace;

    use crate::ty::constructors::*;
    use crate::ty::*;
    use crate::*;

    #[test]
    fn cmp_freevar() {
        enable_overflow_stacktrace!();
        let t = named_uninit_var("T".into());
        let Type::FreeVar(fv) = t.clone() else {
            unreachable!()
        };
        let constraint = Constraint::new_subtype_of(poly("Add", vec![ty_tp(t.clone())]));
        fv.update_constraint(constraint.clone(), true);
        let u = named_free_var("T".into(), 1, constraint);
        println!("{t} {u}");
        assert_eq!(t, t);
        assert_eq!(t, u);
    }
}
