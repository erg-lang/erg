//! provides type-comparison
use std::option::Option; // conflicting to Type::Option

use erg_common::error::{Location, MultiErrorDisplay};

use crate::ty::constructors::{and, or};
use crate::ty::free::fresh_varname;
use crate::ty::free::{Constraint, Cyclicity, FreeKind, FreeTyVar};
use crate::ty::typaram::{OpKind, TyParam, TyParamOrdering};
use crate::ty::value::ValueObj;
use crate::ty::value::ValueObj::Inf;
use crate::ty::{Predicate, RefinementType, SubrKind, SubrType, Type};
use Predicate as Pred;

use erg_common::Str;
use erg_common::{assume_unreachable, log, set};
use TyParamOrdering::*;
use Type::*;

use crate::context::cache::{SubtypePair, GLOBAL_TYPE_CACHE};
use crate::context::eval::SubstContext;
use crate::context::instantiate::TyVarContext;
use crate::context::{Context, TraitInstance, Variance};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Credibility {
    Maybe,
    Absolutely,
}

use Credibility::*;

use super::ContextKind;

impl Context {
    fn register_cache(&self, sup: &Type, sub: &Type, result: bool) {
        if sub.is_cachable() && sup.is_cachable() {
            GLOBAL_TYPE_CACHE.register(SubtypePair::new(sub.clone(), sup.clone()), result);
        }
    }

    // TODO: is it impossible to avoid .clone()?
    fn inquire_cache(&self, sup: &Type, sub: &Type) -> Option<bool> {
        if sub.is_cachable() && sup.is_cachable() {
            let res = GLOBAL_TYPE_CACHE.get(&SubtypePair::new(sub.clone(), sup.clone()));
            if res.is_some() {
                log!(info "cache hit");
            }
            res
        } else {
            None
        }
    }

    pub(crate) fn eq_tp(&self, lhs: &TyParam, rhs: &TyParam) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(lhs), TyParam::Type(rhs)) => return self.same_type_of(lhs, rhs),
            (TyParam::Mono(l), TyParam::Mono(r)) => {
                if let (Some(l), Some(r)) = (self.rec_get_const_obj(l), self.rec_get_const_obj(r)) {
                    return l == r;
                }
            }
            (TyParam::MonoQVar(name), _other) | (_other, TyParam::MonoQVar(name)) => {
                log!(err "comparing '{name} and {_other}");
                panic!("Not instantiated type parameter: {name}")
            }
            (TyParam::UnaryOp { op: lop, val: lval }, TyParam::UnaryOp { op: rop, val: rval }) => {
                return lop == rop && self.eq_tp(lval, rval);
            }
            (
                TyParam::BinOp {
                    op: lop,
                    lhs: ll,
                    rhs: lr,
                },
                TyParam::BinOp {
                    op: rop,
                    lhs: rl,
                    rhs: rr,
                },
            ) => {
                return lop == rop && self.eq_tp(ll, rl) && self.eq_tp(lr, rr);
            }
            (
                TyParam::App {
                    name: ln,
                    args: largs,
                },
                TyParam::App {
                    name: rn,
                    args: rargs,
                },
            ) => {
                return ln == rn
                    && largs.len() == rargs.len()
                    && largs
                        .iter()
                        .zip(rargs.iter())
                        .all(|(l, r)| self.eq_tp(l, r))
            }
            (TyParam::FreeVar(fv), other) | (other, TyParam::FreeVar(fv)) => match &*fv.borrow() {
                FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                    return self.eq_tp(t, other);
                }
                FreeKind::Unbound { constraint, .. }
                | FreeKind::NamedUnbound { constraint, .. } => {
                    let t = constraint.get_type().unwrap();
                    if cfg!(feature = "debug") && t == &Uninited {
                        panic!("Uninited type variable: {fv}");
                    }
                    let other_t = self.type_of(other);
                    return self.same_type_of(t, &other_t);
                }
            },
            (l, r) if l == r => {
                return true;
            }
            _ => {}
        }
        self.shallow_eq_tp(lhs, rhs)
    }

    pub(crate) fn related(&self, lhs: &Type, rhs: &Type) -> bool {
        self.supertype_of(lhs, rhs) || self.subtype_of(lhs, rhs)
    }

    pub(crate) fn supertype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        let res = match self.cheap_supertype_of(lhs, rhs) {
            (Absolutely, judge) => judge,
            (Maybe, judge) => {
                judge
                    || self.structural_supertype_of(lhs, rhs)
                    || self.nominal_supertype_of(lhs, rhs)
            }
        };
        log!("answer: {lhs} {RED}:>{RESET} {rhs} == {res}");
        res
    }

    /// e.g.
    /// Named :> Module
    /// => Module.super_types == [Named]
    /// Seq(T) :> Range(T)
    /// => Range(T).super_types == [Eq, Mutate, Seq('T), Output('T)]
    pub(crate) fn subtype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        match self.cheap_subtype_of(lhs, rhs) {
            (Absolutely, judge) => judge,
            (Maybe, judge) => {
                judge || self.structural_subtype_of(lhs, rhs) || self.nominal_subtype_of(lhs, rhs)
            }
        }
    }

    pub(crate) fn same_type_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.supertype_of(lhs, rhs) && self.subtype_of(lhs, rhs)
    }

    pub(crate) fn cheap_supertype_of(&self, lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        if lhs == rhs {
            return (Absolutely, true);
        }
        match (lhs, rhs) {
            (Obj, _) | (_, Never) => (Absolutely, true),
            (_, Obj) if !lhs.is_unbound_var() => (Absolutely, false),
            (Never, _) if !rhs.is_unbound_var() => (Absolutely, false),
            (Float | Ratio | Int | Nat | Bool, Bool)
            | (Float | Ratio | Int | Nat, Nat)
            | (Float | Ratio | Int, Int)
            | (Float | Ratio, Ratio)
            | (Float, Float) => (Absolutely, true),
            (Type, ClassType | TraitType) => (Absolutely, true),
            (Type, Record(rec)) => (
                Absolutely,
                rec.iter().all(|(_, attr)| self.supertype_of(&Type, attr)),
            ),
            (Type::Uninited, _) | (_, Type::Uninited) => panic!("used an uninited type variable"),
            (Type, Subr(subr)) => (
                Absolutely,
                subr.non_default_params
                    .iter()
                    .all(|pt| self.supertype_of(&Type, pt.typ()))
                    && subr
                        .default_params
                        .iter()
                        .all(|pt| self.supertype_of(&Type, pt.typ()))
                    && subr
                        .var_params
                        .as_ref()
                        .map(|va| self.supertype_of(&Type, va.typ()))
                        .unwrap_or(true)
                    && self.supertype_of(&Type, &subr.return_t),
            ),
            (
                Type::Mono(n),
                Subr(SubrType {
                    kind: SubrKind::Func,
                    ..
                }),
            ) if &n[..] == "GenericFunc" => (Absolutely, true),
            (
                Type::Mono(n),
                Subr(SubrType {
                    kind: SubrKind::Proc,
                    ..
                }),
            ) if &n[..] == "GenericProc" => (Absolutely, true),
            (Type::Mono(l), Type::Poly { name: r, .. })
                if &l[..] == "GenericArray" && &r[..] == "Array" =>
            {
                (Absolutely, true)
            }
            (Type::Mono(l), Type::Poly { name: r, .. })
                if &l[..] == "GenericDict" && &r[..] == "Dict" =>
            {
                (Absolutely, true)
            }
            (Type::Mono(l), Type::Mono(r))
                if &l[..] == "GenericCallable"
                    && (&r[..] == "GenericFunc"
                        || &r[..] == "GenericProc"
                        || &r[..] == "GenericFuncMethod"
                        || &r[..] == "GenericProcMethod") =>
            {
                (Absolutely, true)
            }
            (_, Type::FreeVar(fv)) | (Type::FreeVar(fv), _) => match fv.get_bound_types() {
                Some((Type::Never, Type::Obj)) => (Absolutely, true),
                _ => (Maybe, false),
            },
            (Type::Mono(n), Subr(_)) if &n[..] == "GenericCallable" => (Absolutely, true),
            (lhs, rhs) if lhs.is_simple_class() && rhs.is_simple_class() => (Absolutely, false),
            _ => (Maybe, false),
        }
    }

    fn cheap_subtype_of(&self, lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        self.cheap_supertype_of(rhs, lhs)
    }

    /// make judgments that include supertypes in the same namespace & take into account glue patches
    /// 同一名前空間にある上位型を含めた判定&接着パッチを考慮した判定を行う
    fn nominal_supertype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        if let Some(res) = self.inquire_cache(lhs, rhs) {
            return res;
        }
        if let (Absolutely, judge) = self.classes_supertype_of(lhs, rhs) {
            self.register_cache(lhs, rhs, judge);
            return judge;
        }
        if let (Absolutely, judge) = self.traits_supertype_of(lhs, rhs) {
            self.register_cache(lhs, rhs, judge);
            return judge;
        }
        // FIXME: rec_get_patch
        for patch in self.patches.values() {
            if let ContextKind::GluePatch(tr_inst) = &patch.kind {
                if tr_inst.sub_type.has_qvar() || tr_inst.sup_trait.has_qvar() {
                    todo!("{tr_inst}");
                } else {
                    // e.g.
                    // P = Patch X, Impl: Ord
                    // Rhs <: X => Rhs <: Ord
                    // Ord <: Lhs => Rhs <: Ord <: Lhs
                    if self.supertype_of(&tr_inst.sub_type, rhs)
                        && self.subtype_of(&tr_inst.sup_trait, lhs)
                    {
                        self.register_cache(lhs, rhs, true);
                        return true;
                    }
                }
            }
        }
        self.register_cache(lhs, rhs, false);
        false
    }

    fn nominal_subtype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.nominal_supertype_of(rhs, lhs)
    }

    fn classes_supertype_of(&self, lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        if !self.is_class(lhs) || !self.is_class(rhs) {
            return (Maybe, false);
        }
        if let Some(ty_ctx) = self.get_nominal_type_ctx(rhs) {
            for rhs_sup in ty_ctx.super_classes.iter() {
                let rhs_sup = if rhs_sup.has_qvar() {
                    let rhs = match rhs {
                        Type::Ref(t) => t,
                        Type::RefMut { before, .. } => before,
                        other => other,
                    };
                    let subst_ctx = SubstContext::new(rhs, self, Location::Unknown);
                    subst_ctx.substitute(rhs_sup.clone()).unwrap()
                } else {
                    rhs_sup.clone()
                };
                // Not `supertype_of` (only structures are compared)
                match self.cheap_supertype_of(lhs, &rhs_sup) {
                    (Absolutely, true) => {
                        return (Absolutely, true);
                    }
                    (Maybe, _) => {
                        if self.structural_supertype_of(lhs, &rhs_sup) {
                            return (Absolutely, true);
                        }
                    }
                    _ => {}
                }
            }
        }
        (Maybe, false)
    }

    // e.g. Eq(Nat) :> Nat
    // Nat.super_traits = [Add(Nat), Eq(Nat), Sub(Float), ...]
    fn traits_supertype_of(&self, lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        if !self.is_trait(lhs) {
            return (Maybe, false);
        }
        if let Some(rhs_ctx) = self.get_nominal_type_ctx(rhs) {
            for rhs_sup in rhs_ctx.super_traits.iter() {
                let rhs_sup = if rhs_sup.has_qvar() {
                    let rhs = match rhs {
                        Type::Ref(t) => t,
                        Type::RefMut { before, .. } => before,
                        other => other,
                    };
                    let subst_ctx = SubstContext::new(rhs, self, Location::Unknown);
                    subst_ctx.substitute(rhs_sup.clone()).unwrap()
                } else {
                    rhs_sup.clone()
                };
                // Not `supertype_of` (only structures are compared)
                match self.cheap_supertype_of(lhs, &rhs_sup) {
                    (Absolutely, true) => {
                        return (Absolutely, true);
                    }
                    (Maybe, _) => {
                        if self.structural_supertype_of(lhs, &rhs_sup) {
                            return (Absolutely, true);
                        }
                    }
                    _ => {}
                }
            }
        }
        (Maybe, false)
    }

    /// ```python
    /// assert sup_conforms(?E(<: Eq(?E)), arg: Nat, sup_trait: Eq(Nat))
    /// assert sup_conforms(?E(<: Eq(?R)), arg: T, sup_trait: Eq(U))
    /// ```
    fn sup_conforms(&self, free: &FreeTyVar, arg: &Type, sup_trait: &Type) -> bool {
        let (_sub, sup) = free.get_bound_types().unwrap();
        free.forced_undoable_link(arg);
        let judge = self.supertype_of(&sup, sup_trait);
        free.undo();
        judge
    }

    /// assert!(sup_conforms(?E(<: Eq(?E)), {Nat, Eq(Nat)}))
    /// assert!(sup_conforms(?E(<: Eq(?R)), {Nat, Eq(T)}))
    fn _sub_conforms(&self, free: &FreeTyVar, inst_pair: &TraitInstance) -> bool {
        let (_sub, sup) = free.get_bound_types().unwrap();
        log!(info "{free}");
        free.forced_undoable_link(&inst_pair.sub_type);
        log!(info "{free}");
        let judge = self.subtype_of(&sup, &inst_pair.sup_trait);
        free.undo();
        log!(info "{free}");
        judge
    }

    /// lhs :> rhs?
    /// ```python
    /// assert supertype_of(Int, Nat) # i: Int = 1 as Nat
    /// assert supertype_of(Bool, Bool)
    /// ```
    /// This function does not consider the nominal subtype relation.
    /// Use `supertype_of` for complete judgement.
    /// 単一化、評価等はここでは行わない、スーパータイプになる可能性があるかだけ判定する
    /// ので、lhsが(未連携)型変数の場合は単一化せずにtrueを返す
    pub(crate) fn structural_supertype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        match (lhs, rhs) {
            (Subr(ls), Subr(rs)) if ls.kind == rs.kind => {
                let kw_check = || {
                    for lpt in ls.default_params.iter() {
                        if let Some(rpt) = rs
                            .default_params
                            .iter()
                            .find(|rpt| rpt.name() == lpt.name())
                        {
                            if !self.subtype_of(lpt.typ(), rpt.typ()) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    true
                };
                // () -> Never <: () -> Int <: () -> Object
                // (Object) -> Int <: (Int) -> Int <: (Never) -> Int
                ls.non_default_params.len() == rs.non_default_params.len()
                // REVIEW:
                && ls.default_params.len() == rs.default_params.len()
                && self.supertype_of(&ls.return_t, &rs.return_t) // covariant
                && ls.non_default_params.iter()
                    .zip(rs.non_default_params.iter())
                    .all(|(l, r)| self.subtype_of(l.typ(), r.typ()))
                && ls.var_params.as_ref().zip(rs.var_params.as_ref()).map(|(l, r)| {
                    self.subtype_of(l.typ(), r.typ())
                }).unwrap_or(true)
                && kw_check() // contravariant
            }
            // ?T(<: Nat) !:> ?U(:> Int)
            // ?T(<: Nat) :> ?U(<: Int) (?U can be smaller than ?T)
            (FreeVar(lfv), FreeVar(rfv)) => match (lfv.get_bound_types(), rfv.get_bound_types()) {
                (Some((_, l_sup)), Some((r_sub, _))) => self.supertype_of(&l_sup, &r_sub),
                _ => {
                    if lfv.is_linked() {
                        self.supertype_of(&lfv.crack(), rhs)
                    } else if rfv.is_linked() {
                        self.supertype_of(lhs, &rfv.crack())
                    } else {
                        false
                    }
                }
            },
            // true if it can be a supertype, false if it cannot (due to type constraints)
            // No type constraints are imposed here, as subsequent type decisions are made according to the possibilities
            (FreeVar(lfv), rhs) => {
                match &*lfv.borrow() {
                    FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                        self.supertype_of(t, rhs)
                    }
                    FreeKind::Unbound { constraint, .. }
                    | FreeKind::NamedUnbound { constraint, .. } => match constraint {
                        // `(?T <: Int) :> Nat` can be true, `(?T <: Nat) :> Int` is false
                        // `(?T <: Eq(?T)) :> Nat` can be true, but this requires special judgment
                        // `(?T :> X) :> Y` is true
                        // `(?T :> Str) :> Int` is true (?T :> Str or Int)
                        // `(Nat <: ?T <: Ratio) :> Nat` can be true
                        Constraint::Sandwiched { sup, cyclicity, .. } => match cyclicity {
                            Cyclicity::Not => self.supertype_of(sup, rhs),
                            Cyclicity::Super => self.cyclic_supertype_of(lfv, rhs),
                            _ => todo!(),
                        },
                        // (?v: Type, rhs): OK
                        // (?v: Nat, rhs): Something wrong
                        // Class <: Type, but Nat <!: Type (Nat: Type)
                        Constraint::TypeOf(t) => {
                            if self.supertype_of(&Type, t) {
                                true
                            } else {
                                panic!()
                            }
                        }
                        Constraint::Uninited => unreachable!(),
                    },
                }
            }
            (lhs, FreeVar(rfv)) => {
                match &*rfv.borrow() {
                    FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                        self.supertype_of(lhs, t)
                    }
                    FreeKind::Unbound { constraint, .. }
                    | FreeKind::NamedUnbound { constraint, .. } => match constraint {
                        // ?T cannot be `Never`
                        // `Nat :> (?T <: Int)` can be true
                        // `Int :> (?T <: Nat)` can be true
                        // * so sup is unrelated
                        // `Str :> (?T <: Int)` is false
                        // `Int :> (?T :> Nat)` can be true, `Nat :> (?T :> Int)` is false
                        // `Int :> (Nat <: ?T <: Ratio)` can be true, `Nat :> (Int <: ?T <: Ratio)` is false
                        //  Eq(Int) :> ?M(:> Int, <: Mul(?M) and Add(?M))
                        Constraint::Sandwiched {
                            sub,
                            sup: _,
                            cyclicity: _,
                        } => self.supertype_of(lhs, sub),
                        Constraint::TypeOf(t) => {
                            if self.supertype_of(&Type, t) {
                                true
                            } else {
                                panic!()
                            }
                        }
                        Constraint::Uninited => unreachable!(),
                    },
                }
            }
            (Type, Record(rec)) => {
                for (_, t) in rec.iter() {
                    if !self.supertype_of(&Type, t) {
                        return false;
                    }
                }
                true
            }
            (Type::Record(lhs), Type::Record(rhs)) => {
                for (k, l) in lhs.iter() {
                    if let Some(r) = rhs.get(k) {
                        if !self.supertype_of(l, r) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            // REVIEW: maybe this is incomplete
            // ({I: Int | I >= 0} :> {N: Int | N >= 0}) == true,
            // ({I: Int | I >= 0} :> {I: Int | I >= 1}) == true,
            // ({I: Int | I >= 0} :> {N: Nat | N >= 1}) == true,
            // ({I: Int | I > 1 or I < -1} :> {I: Int | I >= 0}) == false,
            (Refinement(l), Refinement(r)) => {
                if !self.supertype_of(&l.t, &r.t) {
                    return false;
                }
                let mut r_preds_clone = r.preds.clone();
                for l_pred in l.preds.iter() {
                    for r_pred in r.preds.iter() {
                        if l_pred.subject().unwrap_or("") == &l.var[..]
                            && r_pred.subject().unwrap_or("") == &r.var[..]
                            && self.is_super_pred_of(l_pred, r_pred)
                        {
                            r_preds_clone.remove(r_pred);
                        }
                    }
                }
                r_preds_clone.is_empty()
            }
            (Nat, re @ Refinement(_)) => {
                let nat = Type::Refinement(self.into_refinement(Nat));
                self.structural_supertype_of(&nat, re)
            }
            (re @ Refinement(_), Nat) => {
                let nat = Type::Refinement(self.into_refinement(Nat));
                self.structural_supertype_of(re, &nat)
            }
            // Int :> {I: Int | ...} == true
            // Int :> {I: Str| ...} == false
            // Eq({1, 2}) :> {1, 2} (= {I: Int | I == 1 or I == 2})
            // => Eq(Int) :> Eq({1, 2}) :> {1, 2}
            // => true
            (l, Refinement(r)) => {
                if self.supertype_of(l, &r.t) {
                    return true;
                }
                let l = l.derefine();
                self.supertype_of(&l, &r.t)
            }
            // ({I: Int | True} :> Int) == true, ({N: Nat | ...} :> Int) == false, ({I: Int | I >= 0} :> Int) == false
            (Refinement(l), r) => {
                if l.preds.is_empty() {
                    unreachable!()
                }
                if l.preds
                    .iter()
                    .any(|p| p.mentions(&l.var) && p.can_be_false())
                {
                    return false;
                }
                self.supertype_of(&l.t, r)
            }
            (Quantified(l), Quantified(r)) => {
                // REVIEW: maybe this should be `unreachable`
                let l_tv_ctx = TyVarContext::new(self.level, l.bounds.clone(), self);
                let r_tv_ctx = TyVarContext::new(self.level, r.bounds.clone(), self);
                let l_callable = self
                    .instantiate_t(
                        l.unbound_callable.as_ref().clone(),
                        &l_tv_ctx,
                        Location::Unknown,
                    )
                    .unwrap();
                let r_callable = self
                    .instantiate_t(
                        r.unbound_callable.as_ref().clone(),
                        &r_tv_ctx,
                        Location::Unknown,
                    )
                    .unwrap();
                self.structural_supertype_of(&l_callable, &r_callable)
            }
            (Quantified(q), r) => {
                // REVIEW: maybe this should be `unreachable`
                let tmp_tv_ctx = TyVarContext::new(self.level, q.bounds.clone(), self);
                let q_callable = self
                    .instantiate_t(
                        q.unbound_callable.as_ref().clone(),
                        &tmp_tv_ctx,
                        Location::Unknown,
                    )
                    .unwrap();
                self.structural_supertype_of(&q_callable, r)
            }
            // (Int or Str) :> Nat == Int :> Nat || Str :> Nat == true
            // (Num or Show) :> Show == Num :> Show || Show :> Num == true
            (Or(l_or, r_or), rhs) => self.supertype_of(l_or, rhs) || self.supertype_of(r_or, rhs),
            // Int :> (Nat or Str) == Int :> Nat && Int :> Str == false
            (lhs, Or(l_or, r_or)) => self.supertype_of(lhs, l_or) && self.supertype_of(lhs, r_or),
            // (Num and Show) :> Show == false
            (And(l_and, r_and), rhs) => {
                self.supertype_of(l_and, rhs) && self.supertype_of(r_and, rhs)
            }
            // Show :> (Num and Show) == true
            (lhs, And(l_and, r_and)) => {
                self.supertype_of(lhs, l_and) || self.supertype_of(lhs, r_and)
            }
            (_lhs, Not(_, _)) => todo!(),
            (Not(_, _), _rhs) => todo!(),
            // RefMut are invariant
            (Ref(l), Ref(r)) => self.supertype_of(l, r),
            // TはすべてのRef(T)のメソッドを持つので、Ref(T)のサブタイプ
            // REVIEW: RefMut is invariant, maybe
            (Ref(l), r) => self.supertype_of(l, r),
            (RefMut { before: l, .. }, r) => self.supertype_of(l, r),
            // `Eq(Set(T, N)) :> Set(T, N)` will be false, such cases are judged by nominal_supertype_of
            (
                Poly {
                    name: ln,
                    params: lparams,
                },
                Poly {
                    name: rn,
                    params: rparams,
                },
            ) => {
                if ln != rn || lparams.len() != rparams.len() {
                    return false;
                }
                // [Int; 2] :> [Int; 3]
                if &ln[..] == "Array" || &ln[..] == "Set" {
                    let lt = &lparams[0].as_type().unwrap();
                    let rt = &rparams[0].as_type().unwrap();
                    let llen = &lparams[1];
                    let rlen = &rparams[1];
                    self.supertype_of(lt, rt)
                        && self
                            .eval_bin_tp(OpKind::Le, llen, rlen)
                            .map(|tp| matches!(tp, TyParam::Value(ValueObj::Bool(true))))
                            .unwrap_or_else(|e| {
                                e.fmt_all_stderr();
                                todo!();
                            })
                } else {
                    self.poly_supertype_of(lhs, lparams, rparams)
                }
            }
            (MonoQVar(name), r) | (PolyQVar { name, .. }, r) => {
                panic!("internal error: not instantiated type variable: '{name}, r: {r}")
            }
            (l, MonoQVar(name)) | (l, PolyQVar { name, .. }) => {
                panic!("internal error: not instantiated type variable: '{name}, l: {l}")
            }
            (Proj { .. }, _) => {
                if let Some(cands) = self.get_candidates(lhs) {
                    for cand in cands.into_iter() {
                        if self.supertype_of(&cand, rhs) {
                            return true;
                        }
                    }
                }
                false
            }
            (_, Proj { .. }) => {
                if let Some(cands) = self.get_candidates(rhs) {
                    for cand in cands.into_iter() {
                        if self.supertype_of(lhs, &cand) {
                            return true;
                        }
                    }
                }
                false
            }
            (_l, _r) => false,
        }
    }

    pub(crate) fn cyclic_supertype_of(&self, lhs: &FreeTyVar, rhs: &Type) -> bool {
        let subst_ctx = SubstContext::new(rhs, self, Location::Unknown);
        if let Some(super_traits) = self.get_nominal_type_ctx(rhs).map(|ctx| &ctx.super_traits) {
            for super_trait in super_traits {
                let sup_trait = if super_trait.has_qvar() {
                    subst_ctx.substitute(super_trait.clone()).unwrap()
                } else {
                    super_trait.clone()
                };
                if self.sup_conforms(lhs, rhs, &sup_trait) {
                    return true;
                }
            }
        }
        if let Some(super_classes) = self.get_super_classes(rhs) {
            for super_class in super_classes {
                let sup_class = if super_class.has_qvar() {
                    subst_ctx.substitute(super_class).unwrap()
                } else {
                    super_class
                };
                if self.cyclic_supertype_of(lhs, &sup_class) {
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn poly_supertype_of(
        &self,
        typ: &Type,
        lparams: &[TyParam],
        rparams: &[TyParam],
    ) -> bool {
        log!(
            "poly_supertype_of: {typ}, {}, {}",
            erg_common::fmt_vec(lparams),
            erg_common::fmt_vec(rparams)
        );
        let ctx = self
            .get_nominal_type_ctx(typ)
            .unwrap_or_else(|| panic!("{typ} is not found"));
        let variances = ctx.type_params_variance();
        debug_assert_eq!(lparams.len(), variances.len());
        lparams
            .iter()
            .zip(rparams.iter())
            .zip(variances.iter())
            .all(|((lp, rp), variance)| match (lp, rp, variance) {
                (TyParam::Type(l), TyParam::Type(r), Variance::Contravariant) => {
                    self.subtype_of(l, r)
                }
                (TyParam::Type(l), TyParam::Type(r), Variance::Covariant) => {
                    // if matches!(r.as_ref(), &Type::Refinement(_)) { log!(info "{l}, {r}, {}", self.structural_supertype_of(l, r, bounds, Some(lhs_variance))); }
                    self.supertype_of(l, r)
                }
                (TyParam::FreeVar(fv), _, _) if fv.is_unbound() => {
                    let fv_t = fv.get_type().unwrap();
                    let rp_t = self.get_tp_t(rp).unwrap();
                    log!("{fv_t}, {rp_t}, {variance}");
                    if variance == &Variance::Contravariant {
                        self.subtype_of(&fv_t, &rp_t)
                    } else if variance == &Variance::Covariant {
                        self.supertype_of(&fv_t, &rp_t)
                    } else {
                        self.same_type_of(&fv_t, &rp_t)
                    }
                }
                // Invariant
                _ => self.eq_tp(lp, rp),
            })
    }

    /// lhs <: rhs?
    pub(crate) fn structural_subtype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.structural_supertype_of(rhs, lhs)
    }

    pub(crate) fn _structural_same_type_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.structural_supertype_of(lhs, rhs) && self.structural_subtype_of(lhs, rhs)
    }

    pub(crate) fn try_cmp(&self, l: &TyParam, r: &TyParam) -> Option<TyParamOrdering> {
        match (l, r) {
            (TyParam::Value(l), TyParam::Value(r)) =>
                l.try_cmp(r).map(Into::into),
            // TODO: 型を見て判断する
            (TyParam::BinOp{ op, lhs, rhs }, r) => {
                if let Ok(l) = self.eval_bin_tp(*op, lhs, rhs) {
                    self.try_cmp(&l, r)
                } else { Some(Any) }
            },
            (TyParam::FreeVar(fv), p) if fv.is_linked() => {
                self.try_cmp(&*fv.crack(), p)
            }
            (p, TyParam::FreeVar(fv)) if fv.is_linked() => {
                self.try_cmp(p, &*fv.crack())
            }
            (
                l @ (TyParam::FreeVar(_) | TyParam::Erased(_) | TyParam::MonoQVar(_)),
                r @ (TyParam::FreeVar(_) | TyParam::Erased(_) | TyParam::MonoQVar(_)),
            ) /* if v.is_unbound() */ => {
                let l_t = self.get_tp_t(l).unwrap();
                let r_t = self.get_tp_t(r).unwrap();
                if self.supertype_of(&l_t, &r_t) || self.subtype_of(&l_t, &r_t) {
                    Some(Any)
                } else { Some(NotEqual) }
            },
            // Intervalとしてのl..rはl<=rであることが前提となっている
            // try_cmp((n: 1..10), 1) -> Some(GreaterEqual)
            // try_cmp((n: 0..2), 1) -> Some(Any)
            // try_cmp((n: 2.._), 1) -> Some(Greater)
            // try_cmp((n: -1.._), 1) -> Some(Any)
            // try_cmp((n: ?K), "a") -> Some(Any)
            // try_cmp((n: Int), "a") -> Some(NotEqual)
            (l @ (TyParam::Erased(_) | TyParam::FreeVar(_) | TyParam::MonoQVar(_)), p) => {
                let lt = self.get_tp_t(l).unwrap();
                let pt = self.get_tp_t(p).unwrap();
                let l_inf = self.inf(&lt);
                let l_sup = self.sup(&lt);
                if let (Some(inf), Some(sup)) = (l_inf, l_sup) {
                    // (n: Int, 1) -> (-inf..inf, 1) -> (cmp(-inf, 1), cmp(inf, 1)) -> (Less, Greater) -> Any
                    // (n: 5..10, 2) -> (cmp(5..10, 2), cmp(5..10, 2)) -> (Greater, Greater) -> Greater
                    match (
                        self.try_cmp(&inf, p).unwrap(),
                        self.try_cmp(&sup, p).unwrap()
                    ) {
                        (Less, Less) => Some(Less),
                        (Less, Equal) => Some(LessEqual),
                        (Less, LessEqual) => Some(LessEqual),
                        (Less, NotEqual) => Some(NotEqual),
                        (Less, Greater | GreaterEqual | Any) => Some(Any),
                        (Equal, Less) => assume_unreachable!(),
                        (Equal, Equal) => Some(Equal),
                        (Equal, Greater) => Some(GreaterEqual),
                        (Equal, LessEqual) => Some(Equal),
                        (Equal, NotEqual) => Some(GreaterEqual),
                        (Equal, GreaterEqual | Any) => Some(GreaterEqual),
                        (Greater, Less) => assume_unreachable!(),
                        (Greater, Equal) => assume_unreachable!(),
                        (Greater, Greater | NotEqual | GreaterEqual | Any) => Some(Greater),
                        (Greater, LessEqual) => assume_unreachable!(),
                        (LessEqual, Less) => assume_unreachable!(),
                        (LessEqual, Equal | LessEqual) => Some(LessEqual),
                        (LessEqual, Greater | NotEqual | GreaterEqual | Any) => Some(Any),
                        (NotEqual, Less) => Some(Less),
                        (NotEqual, Equal | LessEqual) => Some(LessEqual),
                        (NotEqual, Greater | GreaterEqual | Any) => Some(Any),
                        (NotEqual, NotEqual) => Some(NotEqual),
                        (GreaterEqual, Less) => assume_unreachable!(),
                        (GreaterEqual, Equal | LessEqual) => Some(Equal),
                        (GreaterEqual, Greater | NotEqual | GreaterEqual | Any) => Some(GreaterEqual),
                        (Any, Less) => Some(Less),
                        (Any, Equal | LessEqual) => Some(LessEqual),
                        (Any, Greater | NotEqual | GreaterEqual | Any) => Some(Any),
                        (l, r) =>
                            todo!("cmp({inf}, {sup}) = {l:?}, cmp({inf}, {sup}) = {r:?}"),
                    }
                } else {
                    match (self.supertype_of(&lt, &pt), self.subtype_of(&lt, &pt)) {
                        (true, true) => Some(Any),
                        (true, false) => Some(Any),
                        (false, true) => Some(NotEqual),
                        (false, false) => Some(NoRelation),
                    }
                }
            }
            (l, r @ (TyParam::Erased(_) | TyParam::MonoQVar(_) | TyParam::FreeVar(_))) =>
                self.try_cmp(r, l).map(|ord| ord.reverse()),
            (_l, _r) => {
                erg_common::fmt_dbg!(_l, _r,);
                None
            },
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn into_refinement(&self, t: Type) -> RefinementType {
        match t {
            Nat => {
                let var = Str::from(fresh_varname());
                RefinementType::new(
                    var.clone(),
                    Int,
                    set! {Predicate::ge(var, TyParam::value(0))},
                )
            }
            Refinement(r) => r,
            t => {
                let var = Str::from(fresh_varname());
                RefinementType::new(var, t, set! {})
            }
        }
    }

    /// returns union of two types (A or B)
    pub(crate) fn union(&self, lhs: &Type, rhs: &Type) -> Type {
        if lhs == rhs {
            return lhs.clone();
        }
        // `?T or ?U` will not be unified
        // `Set!(?T, 3) or Set(?T, 3)` wii be unified to Set(?T, 3)
        if !lhs.is_unbound_var() && !rhs.is_unbound_var() {
            match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
                (true, true) => return lhs.clone(),  // lhs = rhs
                (true, false) => return lhs.clone(), // lhs :> rhs
                (false, true) => return rhs.clone(),
                (false, false) => {}
            }
        }
        match (lhs, rhs) {
            (FreeVar(lfv), FreeVar(rfv)) if lfv.is_linked() && rfv.is_linked() => {
                self.union(&lfv.crack(), &rfv.crack())
            }
            (Refinement(l), Refinement(r)) => Type::Refinement(self.union_refinement(l, r)),
            (t, Type::Never) | (Type::Never, t) => t.clone(),
            (t, Refinement(r)) | (Refinement(r), t) => {
                let t = self.into_refinement(t.clone());
                Type::Refinement(self.union_refinement(&t, r))
            }
            (l, r) => or(l.clone(), r.clone()),
        }
    }

    fn union_refinement(&self, lhs: &RefinementType, rhs: &RefinementType) -> RefinementType {
        // TODO: warn if lhs.t !:> rhs.t && rhs.t !:> lhs.t
        let union = self.union(&lhs.t, &rhs.t);
        let name = lhs.var.clone();
        let rhs_preds = rhs
            .preds
            .iter()
            .map(|p| p.clone().change_subject_name(name.clone()))
            .collect();
        // FIXME: predの包含関係も考慮する
        RefinementType::new(lhs.var.clone(), union, lhs.preds.clone().concat(rhs_preds))
    }

    /// returns intersection of two types (A and B)
    pub(crate) fn intersection(&self, lhs: &Type, rhs: &Type) -> Type {
        if lhs == rhs {
            return lhs.clone();
        }
        // ?T and ?U will not be unified
        if !lhs.is_unbound_var() && !rhs.is_unbound_var() {
            match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
                (true, true) => return lhs.clone(),  // lhs = rhs
                (true, false) => return rhs.clone(), // lhs :> rhs
                (false, true) => return lhs.clone(),
                (false, false) => {}
            }
        }
        match (lhs, rhs) {
            (FreeVar(lfv), FreeVar(rfv)) if lfv.is_linked() && rfv.is_linked() => {
                self.intersection(&lfv.crack(), &rfv.crack())
            }
            // {.i = Int} and {.s = Str} == {.i = Int; .s = Str}
            (Type::Record(l), Type::Record(r)) => Type::Record(l.clone().concat(r.clone())),
            (l, r) if self.is_trait(l) && self.is_trait(r) => and(l.clone(), r.clone()),
            (_l, _r) => Type::Never,
        }
    }

    /// see doc/LANG/compiler/refinement_subtyping.md
    /// ```python
    /// assert is_super_pred({I >= 0}, {I == 0})
    /// assert is_super_pred({T >= 0}, {I == 0})
    /// assert !is_super_pred({I < 0}, {I == 0})
    /// ```
    fn is_super_pred_of(&self, lhs: &Predicate, rhs: &Predicate) -> bool {
        match (lhs, rhs) {
            (Pred::LessEqual { rhs, .. }, _) if !rhs.has_upper_bound() => true,
            (Pred::GreaterEqual { rhs, .. }, _) if !rhs.has_lower_bound() => true,
            (
                Pred::Equal { .. },
                Pred::GreaterEqual { .. } | Pred::LessEqual { .. } | Pred::NotEqual { .. },
            )
            | (Pred::LessEqual { .. }, Pred::GreaterEqual { .. })
            | (Pred::GreaterEqual { .. }, Pred::LessEqual { .. })
            | (Pred::NotEqual { .. }, Pred::Equal { .. }) => false,
            (Pred::Equal { rhs, .. }, Pred::Equal { rhs: rhs2, .. })
            | (Pred::NotEqual { rhs, .. }, Pred::NotEqual { rhs: rhs2, .. }) => self
                .try_cmp(rhs, rhs2)
                .map(|ord| ord.canbe_eq())
                .unwrap_or(false),
            // {T >= 0} :> {T >= 1}, {T >= 0} :> {T == 1}
            (
                Pred::GreaterEqual { rhs, .. },
                Pred::GreaterEqual { rhs: rhs2, .. } | Pred::Equal { rhs: rhs2, .. },
            ) => self
                .try_cmp(rhs, rhs2)
                .map(|ord| ord.canbe_le())
                .unwrap_or(false),
            (
                Pred::LessEqual { rhs, .. },
                Pred::LessEqual { rhs: rhs2, .. } | Pred::Equal { rhs: rhs2, .. },
            ) => self
                .try_cmp(rhs, rhs2)
                .map(|ord| ord.canbe_ge())
                .unwrap_or(false),
            (lhs @ (Pred::GreaterEqual { .. } | Pred::LessEqual { .. }), Pred::And(l, r)) => {
                self.is_super_pred_of(lhs, l) || self.is_super_pred_of(lhs, r)
            }
            (lhs, Pred::Or(l, r)) => self.is_super_pred_of(lhs, l) && self.is_super_pred_of(lhs, r),
            (Pred::Or(l, r), rhs @ (Pred::GreaterEqual { .. } | Pred::LessEqual { .. })) => {
                self.is_super_pred_of(l, rhs) || self.is_super_pred_of(r, rhs)
            }
            (Pred::And(l, r), rhs) => {
                self.is_super_pred_of(l, rhs) && self.is_super_pred_of(r, rhs)
            }
            (lhs, rhs) => todo!("{lhs}/{rhs}"),
        }
    }

    pub(crate) fn is_sub_constraint_of(&self, l: &Constraint, r: &Constraint) -> bool {
        match (l, r) {
            // (?I: Nat) <: (?I: Int)
            (Constraint::TypeOf(lhs), Constraint::TypeOf(rhs)) => self.subtype_of(lhs, rhs),
            // (?T <: Int) <: (?T: Type)
            (Constraint::Sandwiched { sub: Never, .. }, Constraint::TypeOf(Type)) => true,
            // (Int <: ?T) <: (Nat <: ?U)
            // (?T <: Nat) <: (?U <: Int)
            // (Int <: ?T <: Ratio) <: (Nat <: ?U <: Complex)
            // TODO: deny cyclic constraint
            (
                Constraint::Sandwiched {
                    sub: lsub,
                    sup: lsup,
                    ..
                },
                Constraint::Sandwiched {
                    sub: rsub,
                    sup: rsup,
                    ..
                },
            ) => self.supertype_of(lsub, rsub) && self.subtype_of(lsup, rsup),
            _ => false,
        }
    }

    #[inline]
    fn type_of(&self, p: &TyParam) -> Type {
        self.get_tp_t(p).unwrap_or(Type::Obj)
    }

    // sup/inf({±∞}) = ±∞ではあるが、Inf/NegInfにはOrdを実装しない
    fn sup(&self, t: &Type) -> Option<TyParam> {
        match t {
            Int | Nat | Float => Some(TyParam::value(Inf)),
            Refinement(refine) => {
                let mut maybe_max = None;
                for pred in refine.preds.iter() {
                    match pred {
                        Pred::LessEqual { lhs, rhs } | Pred::Equal { lhs, rhs }
                            if lhs == &refine.var =>
                        {
                            if let Some(max) = &maybe_max {
                                if self.try_cmp(rhs, max) == Some(Greater) {
                                    maybe_max = Some(rhs.clone());
                                }
                            } else {
                                maybe_max = Some(rhs.clone());
                            }
                        }
                        _ => {}
                    }
                }
                maybe_max
            }
            _other => None,
        }
    }

    fn inf(&self, t: &Type) -> Option<TyParam> {
        match t {
            Int | Float => Some(TyParam::value(-Inf)),
            Nat => Some(TyParam::value(0usize)),
            Refinement(refine) => {
                let mut maybe_min = None;
                for pred in refine.preds.iter() {
                    match pred {
                        Predicate::GreaterEqual { lhs, rhs } | Predicate::Equal { lhs, rhs }
                            if lhs == &refine.var =>
                        {
                            if let Some(min) = &maybe_min {
                                if self.try_cmp(rhs, min) == Some(Less) {
                                    maybe_min = Some(rhs.clone());
                                }
                            } else {
                                maybe_min = Some(rhs.clone());
                            }
                        }
                        _ => {}
                    }
                }
                maybe_min
            }
            _other => None,
        }
    }

    /// lhsとrhsが包含関係にあるとき小さいほうを返す
    /// 関係なければNoneを返す
    pub(crate) fn min<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> Option<&'t Type> {
        // 同じならどちらを返しても良い
        match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
            (true, true) | (true, false) => Some(rhs),
            (false, true) => Some(lhs),
            (false, false) => None,
        }
    }

    pub(crate) fn _max<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> Option<&'t Type> {
        // 同じならどちらを返しても良い
        match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
            (true, true) | (true, false) => Some(lhs),
            (false, true) => Some(rhs),
            (false, false) => None,
        }
    }

    pub(crate) fn cmp_t<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> TyParamOrdering {
        match self.min(lhs, rhs) {
            Some(l) if l == lhs => TyParamOrdering::Less,
            Some(_) => TyParamOrdering::Greater,
            None => TyParamOrdering::NoRelation,
        }
    }
}
