//! provides type-comparison
use std::option::Option; // conflicting to Type::Option

use erg_common::dict::Dict;
use erg_common::error::MultiErrorDisplay;
use erg_common::style::colors::DEBUG_ERROR;
use erg_common::traits::StructuralEq;
use erg_common::Str;
use erg_common::{assume_unreachable, log};

use crate::ty::constructors::{and, not, or, poly};
use crate::ty::free::{Constraint, FreeKind};
use crate::ty::typaram::{OpKind, TyParam, TyParamOrdering};
use crate::ty::value::ValueObj;
use crate::ty::value::ValueObj::Inf;
use crate::ty::{Field, GuardType, Predicate, RefinementType, SubrKind, SubrType, Type};
use Predicate as Pred;

use TyParamOrdering::*;
use Type::*;

use crate::context::cache::{SubtypePair, GLOBAL_TYPE_CACHE};
use crate::context::{Context, Variance};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Credibility {
    Maybe,
    Absolutely,
}

use Credibility::*;

use super::ContextKind;

impl Context {
    fn register_cache(&self, sup: &Type, sub: &Type, result: bool) {
        GLOBAL_TYPE_CACHE.register(SubtypePair::new(sub.clone(), sup.clone()), result);
    }

    // TODO: is it impossible to avoid .clone()?
    fn inquire_cache(&self, sup: &Type, sub: &Type) -> Option<bool> {
        let res = GLOBAL_TYPE_CACHE.get(&SubtypePair::new(sub.clone(), sup.clone()));
        if res.is_some() {
            log!(info "cache hit");
        }
        res
    }

    pub(crate) fn eq_tp(&self, lhs: &TyParam, rhs: &TyParam) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(lhs), TyParam::Type(rhs)) => return self.same_type_of(lhs, rhs),
            (TyParam::Mono(l), TyParam::Mono(r)) => {
                if let (Some(l), Some(r)) = (self.rec_get_const_obj(l), self.rec_get_const_obj(r)) {
                    return l == r;
                }
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
                FreeKind::Linked(linked) | FreeKind::UndoableLinked { t: linked, .. } => {
                    return self.eq_tp(linked, other);
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
            (TyParam::Value(ValueObj::Type(l)), TyParam::Type(r)) => {
                return self.same_type_of(l.typ(), r.as_ref());
            }
            (TyParam::Type(l), TyParam::Value(ValueObj::Type(r))) => {
                return self.same_type_of(l.as_ref(), r.typ());
            }
            (l, r) if l == r => {
                return true;
            }
            (l, r) if l.has_unbound_var() || r.has_unbound_var() => {
                let lt = self.get_tp_t(l).unwrap();
                let rt = self.get_tp_t(r).unwrap();
                return self.same_type_of(&lt, &rt);
            }
            _ => {}
        }
        self.shallow_eq_tp(lhs, rhs)
    }

    pub(crate) fn related(&self, lhs: &Type, rhs: &Type) -> bool {
        self.supertype_of(lhs, rhs) || self.subtype_of(lhs, rhs)
    }

    /// lhs :> rhs ?
    pub(crate) fn supertype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        let res = match Self::cheap_supertype_of(lhs, rhs) {
            (Absolutely, judge) => judge,
            (Maybe, judge) => {
                judge
                    || self.structural_supertype_of(lhs, rhs)
                    || self.nominal_supertype_of(lhs, rhs)
            }
        };
        log!("answer: {lhs} {DEBUG_ERROR}:>{RESET} {rhs} == {res}");
        res
    }

    /// lhs <: rhs ?
    ///
    /// e.g.
    /// ```erg
    /// Named :> Module
    /// => Module.super_types == [Named]
    ///
    /// Seq(T) :> Range(T)
    /// => Range(T).super_types == [Eq, Mutate, Seq(T), Output(T)]
    /// ```
    pub fn subtype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        match Self::cheap_subtype_of(lhs, rhs) {
            (Absolutely, judge) => judge,
            (Maybe, judge) => {
                judge || self.structural_subtype_of(lhs, rhs) || self.nominal_subtype_of(lhs, rhs)
            }
        }
    }

    pub(crate) fn same_type_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.supertype_of(lhs, rhs) && self.subtype_of(lhs, rhs)
    }

    pub(crate) fn cheap_supertype_of(lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        if lhs == rhs {
            return (Absolutely, true);
        }
        match (lhs, rhs) {
            (Obj, _) | (_, Never | Failure) => (Absolutely, true),
            (_, Obj) if lhs.is_simple_class() => (Absolutely, false),
            (Never | Failure, _) if rhs.is_simple_class() => (Absolutely, false),
            (Float | Ratio | Int | Nat | Bool, Bool)
            | (Float | Ratio | Int | Nat, Nat)
            | (Float | Ratio | Int, Int)
            | (Float | Ratio, Ratio) => (Absolutely, true),
            (Type, ClassType | TraitType) => (Absolutely, true),
            (
                Mono(n),
                Subr(SubrType {
                    kind: SubrKind::Func,
                    ..
                }),
            ) if &n[..] == "GenericFunc" => (Absolutely, true),
            (
                Mono(n),
                Subr(SubrType {
                    kind: SubrKind::Proc,
                    ..
                }),
            ) if &n[..] == "GenericProc" => (Absolutely, true),
            (Mono(l), Poly { name: r, .. }) if &l[..] == "GenericArray" && &r[..] == "Array" => {
                (Absolutely, true)
            }
            (Mono(l), Poly { name: r, .. }) if &l[..] == "GenericDict" && &r[..] == "Dict" => {
                (Absolutely, true)
            }
            (Mono(l), Mono(r))
                if &l[..] == "GenericCallable"
                    && (&r[..] == "GenericFunc"
                        || &r[..] == "GenericProc"
                        || &r[..] == "GenericFuncMethod"
                        || &r[..] == "GenericProcMethod") =>
            {
                (Absolutely, true)
            }
            (FreeVar(l), FreeVar(r)) => {
                if l.structural_eq(r) {
                    (Absolutely, true)
                } else {
                    (Maybe, false)
                }
            }
            (_, FreeVar(fv)) | (FreeVar(fv), _) => match fv.get_subsup() {
                Some((Type::Never, Type::Obj)) => (Absolutely, true),
                _ => (Maybe, false),
            },
            (Mono(n), Subr(_) | Quantified(_)) if &n[..] == "GenericCallable" => (Absolutely, true),
            (lhs, rhs) if lhs.is_simple_class() && rhs.is_simple_class() => (Absolutely, false),
            _ => (Maybe, false),
        }
    }

    fn cheap_subtype_of(lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        Self::cheap_supertype_of(rhs, lhs)
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
        self.register_cache(lhs, rhs, false);
        false
    }

    fn nominal_subtype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.nominal_supertype_of(rhs, lhs)
    }

    pub(crate) fn find_patches_of<'a>(
        &'a self,
        typ: &'a Type,
    ) -> impl Iterator<Item = &'a Context> {
        self.all_patches().into_iter().filter(move |ctx| {
            if let ContextKind::Patch(base) = &ctx.kind {
                return self.supertype_of(base, typ);
            }
            false
        })
    }

    fn _find_compatible_glue_patch(&self, sup: &Type, sub: &Type) -> Option<&Context> {
        for patch in self.all_patches().into_iter() {
            if let ContextKind::GluePatch(tr_impl) = &patch.kind {
                if self.subtype_of(sub, &tr_impl.sub_type)
                    && self.subtype_of(&tr_impl.sup_trait, sup)
                {
                    return Some(patch);
                }
            }
        }
        None
    }

    fn classes_supertype_of(&self, lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        if !self.is_class(lhs) || !self.is_class(rhs) {
            return (Maybe, false);
        }
        if let Some((_, ty_ctx)) = self.get_nominal_type_ctx(rhs) {
            for rhs_sup in ty_ctx.super_classes.iter() {
                let rhs_sup = if rhs_sup.has_qvar() {
                    let rhs = match rhs {
                        Type::Ref(t) => t,
                        Type::RefMut { before, .. } => before,
                        other => other,
                    };
                    // let subst_ctx = SubstContext::new(rhs, self, Location::Unknown);
                    // subst_ctx.substitute(rhs_sup.clone()).unwrap()
                    rhs.clone()
                } else {
                    rhs_sup.clone()
                };
                // Not `supertype_of` (only structures are compared)
                match Self::cheap_supertype_of(lhs, &rhs_sup) {
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
    // e.g. Eq :> ?L or ?R (if ?L <: Eq and ?R <: Eq)
    fn traits_supertype_of(&self, lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        if !self.is_trait(lhs) {
            return (Maybe, false);
        }
        if let Some((_, rhs_ctx)) = self.get_nominal_type_ctx(rhs) {
            for rhs_sup in rhs_ctx.super_traits.iter() {
                // Not `supertype_of` (only structures are compared)
                match Self::cheap_supertype_of(lhs, rhs_sup) {
                    (Absolutely, true) => {
                        return (Absolutely, true);
                    }
                    (Maybe, _) => {
                        if self.structural_supertype_of(lhs, rhs_sup) {
                            return (Absolutely, true);
                        }
                    }
                    _ => {}
                }
            }
        }
        (Maybe, false)
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
            // Proc :> Func if params are compatible
            (Subr(ls), Subr(rs)) if ls.kind == rs.kind || ls.kind.is_proc() => {
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
                let same_params_len = ls.non_default_params.len() == rs.non_default_params.len()
                    // REVIEW:
                    && ls.default_params.len() == rs.default_params.len();
                let return_t_judge = self.supertype_of(&ls.return_t, &rs.return_t); // covariant
                let non_defaults_judge = ls
                    .non_default_params
                    .iter()
                    .zip(rs.non_default_params.iter())
                    .all(|(l, r)| self.subtype_of(l.typ(), r.typ()));
                let var_params_judge = ls
                    .var_params
                    .as_ref()
                    .zip(rs.var_params.as_ref())
                    .map(|(l, r)| self.subtype_of(l.typ(), r.typ()))
                    .unwrap_or(true);
                same_params_len
                    && return_t_judge
                    && non_defaults_judge
                    && var_params_judge
                    && kw_check() // contravariant
            }
            // ?T(<: Nat) !:> ?U(:> Int)
            // ?T(<: Nat) :> ?U(<: Int) (?U can be smaller than ?T)
            (FreeVar(lfv), FreeVar(rfv)) => match (lfv.get_subsup(), rfv.get_subsup()) {
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
            // ?P(<: Mul ?P) :> Int
            //   => ?P.undoable_link(Int)
            //   => Mul Int :> Int
            (FreeVar(lfv), rhs) => {
                match &*lfv.borrow() {
                    FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                        self.supertype_of(t, rhs)
                    }
                    FreeKind::Unbound { constraint: _, .. }
                    | FreeKind::NamedUnbound { constraint: _, .. } => {
                        if let Some((_sub, sup)) = lfv.get_subsup() {
                            lfv.forced_undoable_link(rhs);
                            let res = self.supertype_of(&sup, rhs);
                            lfv.undo();
                            res
                        } else if let Some(lfvt) = lfv.get_type() {
                            // e.g. lfv: ?L(: Int) is unreachable
                            // but
                            // ?L(: Array(Type, 3)) :> Array(Int, 3)
                            //   => Array(Type, 3) :> Array(Typeof(Int), 3)
                            //   => true
                            let rhs_meta = self.meta_type(rhs);
                            self.supertype_of(&lfvt, &rhs_meta)
                        } else {
                            // constraint is uninitalized
                            log!(err "constraint is uninitialized: {lfv}/{rhs}");
                            true
                        }
                    }
                }
            }
            (lhs, FreeVar(rfv)) => match &*rfv.borrow() {
                FreeKind::Linked(t) | FreeKind::UndoableLinked { t, .. } => {
                    self.supertype_of(lhs, t)
                }
                FreeKind::Unbound { constraint: _, .. }
                | FreeKind::NamedUnbound { constraint: _, .. } => {
                    if let Some((sub, _sup)) = rfv.get_subsup() {
                        rfv.forced_undoable_link(lhs);
                        let res = self.supertype_of(lhs, &sub);
                        rfv.undo();
                        res
                    } else if let Some(rfvt) = rfv.get_type() {
                        let lhs_meta = self.meta_type(lhs);
                        self.supertype_of(&lhs_meta, &rfvt)
                    } else {
                        // constraint is uninitalized
                        log!(err "constraint is uninitialized: {lhs}/{rfv}");
                        true
                    }
                }
            },
            (Type::Record(lhs), Type::Record(rhs)) => {
                for (l_k, l_t) in lhs.iter() {
                    if let Some((r_k, r_t)) = rhs.get_key_value(l_k) {
                        // public <: private (private fields cannot be public)
                        if (l_k.vis.is_public() && r_k.vis.is_private())
                            || !self.supertype_of(l_t, r_t)
                        {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (Type, Record(rec)) => {
                for (_, t) in rec.iter() {
                    if !self.supertype_of(&Type, t) {
                        return false;
                    }
                }
                true
            }
            (Bool, Guard { .. }) => true,
            (Type, Subr(subr)) => self.supertype_of(&Type, &subr.return_t),
            (Type, Poly { name, params }) | (Poly { name, params }, Type)
                if &name[..] == "Array" || &name[..] == "Set" =>
            {
                let elem_t = self.convert_tp_into_ty(params[0].clone()).unwrap();
                self.supertype_of(&Type, &elem_t)
            }
            (Type, Poly { name, params }) | (Poly { name, params }, Type)
                if &name[..] == "Tuple" =>
            {
                if let Ok(tps) = Vec::try_from(params[0].clone()) {
                    for tp in tps {
                        let Ok(t) = self.convert_tp_into_ty(tp) else {
                            return false;
                        };
                        if !self.supertype_of(&Type, &t) {
                            return false;
                        }
                    }
                }
                false
            }
            (Type, Poly { name, params }) | (Poly { name, params }, Type)
                if &name[..] == "Dict" =>
            {
                // HACK: e.g. ?D: GenericDict
                let Ok(dict) = Dict::try_from(params[0].clone()) else {
                    return false;
                };
                for (k, v) in dict.into_iter() {
                    let Ok(k) = self.convert_tp_into_ty(k) else {
                        return false;
                    };
                    let Ok(v) = self.convert_tp_into_ty(v) else {
                        return false;
                    };
                    if !self.supertype_of(&Type, &k) || !self.supertype_of(&Type, &v) {
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
            // ({I: Int | I >= 0} :> {F: Float | F >= 0}) == false,
            // {1, 2, 3} :> {1, } == true
            (Refinement(l), Refinement(r)) => {
                // no relation or l.t <: r.t (not equal)
                if !self.supertype_of(&l.t, &r.t) {
                    return false;
                }
                self.is_super_pred_of(&l.pred, &r.pred)
            }
            (Nat, re @ Refinement(_)) => {
                let nat = Type::Refinement(Nat.into_refinement());
                self.structural_supertype_of(&nat, re)
            }
            (re @ Refinement(_), Nat) => {
                let nat = Type::Refinement(Nat.into_refinement());
                self.structural_supertype_of(re, &nat)
            }
            (Structural(_), Refinement(refine)) => self.supertype_of(lhs, &refine.t),
            // Int :> {I: Int | ...} == true
            // Int :> {I: Str| ...} == false
            // Eq({1, 2}) :> {1, 2} (= {I: Int | I == 1 or I == 2})
            // => Eq(Int) :> Eq({1, 2}) :> {1, 2}
            // => true
            // Bool :> {1} == true
            // Bool :> {2} == false
            (l, Refinement(r)) => {
                if self.supertype_of(l, &r.t) {
                    return true;
                }
                let l = l.derefine();
                if self.supertype_of(&l, &r.t) {
                    return true;
                }
                let l = Type::Refinement(l.into_refinement());
                self.structural_supertype_of(&l, rhs)
            }
            // ({I: Int | True} :> Int) == true, ({N: Nat | ...} :> Int) == false, ({I: Int | I >= 0} :> Int) == false
            (Refinement(l), r) => {
                if l.pred.mentions(&l.var) && l.pred.can_be_false() {
                    return false;
                }
                self.supertype_of(&l.t, r)
            }
            (Quantified(_), Quantified(_)) => {
                let Ok(l) = self.instantiate_dummy(lhs.clone()) else {
                    return false;
                };
                let Ok(r) = self.instantiate_dummy(rhs.clone()) else {
                    return false;
                };
                self.sub_unify(&r, &l, &(), None).is_ok()
            }
            // (|T: Type| T -> T) !<: Obj -> Never
            (Quantified(_), r) => {
                let Ok(inst) = self.instantiate_dummy(lhs.clone()) else {
                    return false;
                };
                self.sub_unify(r, &inst, &(), None).is_ok()
            }
            (l, Quantified(_)) => {
                let Ok(inst) = self.instantiate_dummy(rhs.clone()) else {
                    return false;
                };
                self.sub_unify(&inst, l, &(), None).is_ok()
            }
            // Int or Str :> Str or Int == (Int :> Str && Str :> Int) || (Int :> Int && Str :> Str) == true
            (Or(l_1, l_2), Or(r_1, r_2)) => {
                if l_1.is_union_type() && self.supertype_of(l_1, rhs) {
                    return true;
                }
                if l_2.is_union_type() && self.supertype_of(l_2, rhs) {
                    return true;
                }
                (self.supertype_of(l_1, r_1) && self.supertype_of(l_2, r_2))
                    || (self.supertype_of(l_1, r_2) && self.supertype_of(l_2, r_1))
            }
            // not Nat :> not Int == true
            (Not(l), Not(r)) => self.subtype_of(l, r),
            // (Int or Str) :> Nat == Int :> Nat || Str :> Nat == true
            // (Num or Show) :> Show == Num :> Show || Show :> Num == true
            (Or(l_or, r_or), rhs) => self.supertype_of(l_or, rhs) || self.supertype_of(r_or, rhs),
            // Int :> (Nat or Str) == Int :> Nat && Int :> Str == false
            (lhs, Or(l_or, r_or)) => self.supertype_of(lhs, l_or) && self.supertype_of(lhs, r_or),
            (And(l_1, l_2), And(r_1, r_2)) => {
                if l_1.is_intersection_type() && self.supertype_of(l_1, rhs) {
                    return true;
                }
                if l_2.is_intersection_type() && self.supertype_of(l_2, rhs) {
                    return true;
                }
                (self.supertype_of(l_1, r_1) && self.supertype_of(l_2, r_2))
                    || (self.supertype_of(l_1, r_2) && self.supertype_of(l_2, r_1))
            }
            // (Num and Show) :> Show == false
            (And(l_and, r_and), rhs) => {
                self.supertype_of(l_and, rhs) && self.supertype_of(r_and, rhs)
            }
            // Show :> (Num and Show) == true
            (lhs, And(l_and, r_and)) => {
                self.supertype_of(lhs, l_and) || self.supertype_of(lhs, r_and)
            }
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
                    let lt = self.convert_tp_into_ty(lparams[0].clone()).unwrap();
                    let rt = self.convert_tp_into_ty(rparams[0].clone()).unwrap();
                    let llen = lparams[1].clone();
                    let rlen = rparams[1].clone();
                    self.supertype_of(&lt, &rt)
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
            (Structural(l), Structural(r)) => self.structural_supertype_of(l, r),
            // TODO: If visibility does not match, it should be reported as a cause of an error
            (Structural(l), r) => {
                if self.supertype_of(l, r) {
                    return true;
                }
                let r_fields = self.fields(r);
                for (l_field, l_ty) in self.fields(l) {
                    if let Some((r_field, r_ty)) = r_fields.get_key_value(&l_field) {
                        let compatible = self.supertype_of(&l_ty, r_ty);
                        if r_field.vis != l_field.vis || !compatible {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (_l, _r) => false,
        }
    }

    pub fn fields(&self, t: &Type) -> Dict<Field, Type> {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => self.fields(&fv.crack()),
            Type::Record(fields) => fields.clone(),
            Type::Refinement(refine) => self.fields(&refine.t),
            Type::Structural(t) => self.fields(t),
            other => {
                let Some((_, ctx)) = self.get_nominal_type_ctx(other) else {
                    return Dict::new();
                };
                ctx.type_dir(self)
                    .into_iter()
                    .map(|(name, vi)| {
                        (
                            Field::new(vi.vis.modifier.clone(), name.inspect().clone()),
                            vi.t.clone(),
                        )
                    })
                    .collect()
            }
        }
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
        let (_, ctx) = self
            .get_nominal_type_ctx(typ)
            .unwrap_or_else(|| panic!("{typ} is not found"));
        let variances = ctx.type_params_variance();
        debug_assert_eq!(lparams.len(), variances.len());
        lparams
            .iter()
            .zip(rparams.iter())
            .zip(variances.iter())
            .all(|((lp, rp), variance)| self.supertype_of_tp(lp, rp, *variance))
    }

    fn supertype_of_tp(&self, lp: &TyParam, rp: &TyParam, variance: Variance) -> bool {
        if lp == rp {
            return true;
        }
        match (lp, rp, variance) {
            (TyParam::FreeVar(fv), _, _) if fv.is_linked() => {
                self.supertype_of_tp(&fv.crack(), rp, variance)
            }
            (_, TyParam::FreeVar(fv), _) if fv.is_linked() => {
                self.supertype_of_tp(lp, &fv.crack(), variance)
            }
            // _: Type :> T == true
            (TyParam::Erased(t), TyParam::Type(_), _)
            | (TyParam::Type(_), TyParam::Erased(t), _)
                if t.as_ref() == &Type =>
            {
                true
            }
            (TyParam::Array(lp), TyParam::Array(rp), _)
            | (TyParam::Tuple(lp), TyParam::Tuple(rp), _) => {
                for (l, r) in lp.iter().zip(rp.iter()) {
                    if !self.supertype_of_tp(l, r, variance) {
                        return false;
                    }
                }
                true
            }
            // {Int: Str} :> {Int: Str, Bool: Int}
            (TyParam::Dict(ld), TyParam::Dict(rd), _) => {
                if ld.len() > rd.len() {
                    return false;
                }
                for (k, lv) in ld.iter() {
                    if let Some(rv) = rd.get(k) {
                        if !self.supertype_of_tp(lv, rv, variance) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (TyParam::Type(l), TyParam::Type(r), Variance::Contravariant) => self.subtype_of(l, r),
            (TyParam::Type(l), TyParam::Type(r), Variance::Covariant) => self.supertype_of(l, r),
            (TyParam::Type(l), TyParam::Type(r), Variance::Invariant) => self.same_type_of(l, r),
            (TyParam::FreeVar(fv), _, _) if fv.is_unbound() => {
                let Some(fv_t) = fv.get_type() else {
                    return false;
                };
                let rp_t = match self.get_tp_t(rp) {
                    Ok(t) => t,
                    Err(err) => {
                        log!("supertype_of_tp: {err}");
                        return false;
                    }
                };
                if variance == Variance::Contravariant {
                    self.subtype_of(&fv_t, &rp_t)
                } else if variance == Variance::Covariant {
                    self.supertype_of(&fv_t, &rp_t)
                } else {
                    self.same_type_of(&fv_t, &rp_t)
                }
            }
            _ => self.eq_tp(lp, rp),
        }
    }

    /// lhs <: rhs?
    pub(crate) fn structural_subtype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.structural_supertype_of(rhs, lhs)
    }

    pub(crate) fn _structural_same_type_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.structural_supertype_of(lhs, rhs) && self.structural_subtype_of(lhs, rhs)
    }

    pub(crate) fn try_cmp(&self, l: &TyParam, r: &TyParam) -> Option<TyParamOrdering> {
        if l == r {
            return Some(Equal);
        }
        match (l, r) {
            (TyParam::Value(l), TyParam::Value(r)) =>
                l.try_cmp(r).map(Into::into),
            // TODO: 型を見て判断する
            (TyParam::BinOp{ op, lhs, rhs }, r) => {
                if let Ok(l) = self.eval_bin_tp(*op, lhs.as_ref().clone(), rhs.as_ref().clone()) {
                    self.try_cmp(&l, r)
                } else { Some(Any) }
            },
            (TyParam::FreeVar(fv), p) if fv.is_linked() => {
                self.try_cmp(&fv.crack(), p)
            }
            (p, TyParam::FreeVar(fv)) if fv.is_linked() => {
                self.try_cmp(p, &fv.crack())
            }
            (
                l @ (TyParam::FreeVar(_) | TyParam::Erased(_)),
                r @ (TyParam::FreeVar(_) | TyParam::Erased(_)),
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
            (l @ (TyParam::Erased(_) | TyParam::FreeVar(_)), p) => {
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
            (l, r @ (TyParam::Erased(_) | TyParam::FreeVar(_))) =>
                self.try_cmp(r, l).map(|ord| ord.reverse()),
            (_l, _r) => {
                erg_common::fmt_dbg!(_l, _r,);
                None
            },
        }
    }

    /// Returns union of two types (`A or B`).
    /// If `A` and `B` have a subtype relationship, it is equal to `max(A, B)`.
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
            (FreeVar(fv), other) | (other, FreeVar(fv)) if fv.is_linked() => {
                self.union(&fv.crack(), other)
            }
            (Refinement(l), Refinement(r)) => Type::Refinement(self.union_refinement(l, r)),
            (Structural(l), Structural(r)) => self.union(l, r).structuralize(),
            (t, Type::Never) | (Type::Never, t) => t.clone(),
            // Array({1, 2}, 2), Array({3, 4}, 2) ==> Array({1, 2, 3, 4}, 2)
            (
                Type::Poly {
                    name: ln,
                    params: lps,
                },
                Type::Poly {
                    name: rn,
                    params: rps,
                },
            ) if ln == rn => {
                debug_assert_eq!(lps.len(), rps.len());
                let mut unified_params = vec![];
                for (lp, rp) in lps.iter().zip(rps.iter()) {
                    match (lp, rp) {
                        (TyParam::Type(l), TyParam::Type(r)) => {
                            unified_params.push(TyParam::t(self.union(l, r)))
                        }
                        (_, _) => {
                            if self.eq_tp(lp, rp) {
                                unified_params.push(lp.clone());
                            } else {
                                return or(lhs.clone(), rhs.clone());
                            }
                        }
                    }
                }
                poly(ln, unified_params)
            }
            (l, r) => or(l.clone(), r.clone()),
        }
    }

    fn union_refinement(&self, lhs: &RefinementType, rhs: &RefinementType) -> RefinementType {
        // TODO: warn if lhs.t !:> rhs.t && rhs.t !:> lhs.t
        let union = self.union(&lhs.t, &rhs.t);
        let name = lhs.var.clone();
        let rhs_pred = rhs.pred.clone().change_subject_name(name);
        // FIXME: predの包含関係も考慮する
        RefinementType::new(lhs.var.clone(), union, *lhs.pred.clone() | rhs_pred)
    }

    /// Returns intersection of two types (`A and B`).
    /// If `A` and `B` have a subtype relationship, it is equal to `min(A, B)`.
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
            (FreeVar(fv), other) | (other, FreeVar(fv)) if fv.is_linked() => {
                self.intersection(&fv.crack(), other)
            }
            (Refinement(l), Refinement(r)) => Type::Refinement(self.intersection_refinement(l, r)),
            (other, Refinement(refine)) | (Refinement(refine), other) => {
                let other = other.clone().into_refinement();
                let intersec = self.intersection_refinement(&other, refine);
                self.try_squash_refinement(intersec)
                    .unwrap_or_else(Type::Refinement)
            }
            (Structural(l), Structural(r)) => self.intersection(l, r).structuralize(),
            (Guard(_), Guard(_)) => and(lhs.clone(), rhs.clone()),
            // {.i = Int} and {.s = Str} == {.i = Int; .s = Str}
            (Record(l), Record(r)) => Type::Record(l.clone().concat(r.clone())),
            // {i = Int; j = Int} and not {i = Int} == {j = Int}
            // not {i = Int} and {i = Int; j = Int} == {j = Int}
            (other @ Record(rec), Not(t)) | (Not(t), other @ Record(rec)) => match t.as_ref() {
                Type::FreeVar(fv) => self.intersection(&fv.crack(), other),
                Type::Record(rec2) => Type::Record(rec.clone().diff(rec2)),
                _ => Type::Never,
            },
            (l, r) if self.is_trait(l) && self.is_trait(r) => and(l.clone(), r.clone()),
            (_, Not(r)) => self.diff(lhs, r),
            (Not(l), _) => self.diff(rhs, l),
            (_l, _r) => Type::Never,
        }
    }

    /// ```erg
    /// {I: Int | I > 0} and {I: Int | I < 10} == {I: Int | I > 0 and I < 10}
    /// {x: Int or NoneType | True} and {x: Obj | x != None} == {x: Int or NoneType | x != None} (== Int)
    /// ```
    fn intersection_refinement(
        &self,
        lhs: &RefinementType,
        rhs: &RefinementType,
    ) -> RefinementType {
        let intersec = self.intersection(&lhs.t, &rhs.t);
        let name = lhs.var.clone();
        let rhs_pred = rhs.pred.clone().change_subject_name(name);
        RefinementType::new(lhs.var.clone(), intersec, *lhs.pred.clone() & rhs_pred)
    }

    /// ```erg
    /// {x: Int | True}.try_squash() == Ok(Int)
    /// {x: Int or NoneType | x != None}.squash() == Ok(Int)
    /// {x: Str or Bool | x != False}.squash() == Err({x: Str or Bool | x != False})
    /// {x: Str or Bool | x != True and x != False}.squash() == Ok(Str)
    /// {x: Nat or {-1} | x != 2}.squash() == Err({x: Int | (x >= 0 or x == -1) and x != 2 })
    /// ```
    pub(crate) fn try_squash_refinement(
        &self,
        refine: RefinementType,
    ) -> Result<Type, RefinementType> {
        let unions = refine.t.union_types();
        let complement = Type::from(self.type_from_pred(refine.pred.clone().invert()));
        let union = unions
            .into_iter()
            .filter(|t| !self.subtype_of(t, &complement))
            .fold(Never, |union, t| self.union(&union, &t));
        if &union != refine.t.as_ref() {
            Ok(union)
        } else {
            Err(refine)
        }
    }

    /// (x == 1) => {x: Int | x == 1}
    /// (x == c) where c: Str => {x: Str | x == c}
    fn type_from_pred(&self, pred: Predicate) -> RefinementType {
        let t = self.get_pred_type(&pred);
        let name = pred.subject().unwrap_or("_");
        RefinementType::new(Str::rc(name), t, pred)
    }

    fn get_pred_type(&self, pred: &Predicate) -> Type {
        match pred {
            Predicate::Equal { rhs, .. }
            | Predicate::NotEqual { rhs, .. }
            | Predicate::GreaterEqual { rhs, .. }
            | Predicate::LessEqual { rhs, .. } => self.get_tp_t(rhs).unwrap_or(Obj),
            Predicate::Not(pred) => self.get_pred_type(pred),
            Predicate::Value(val) => val.class(),
            // x == 1 or x == "a" => Int or Str
            Predicate::Or(lhs, rhs) => {
                self.union(&self.get_pred_type(lhs), &self.get_pred_type(rhs))
            }
            // REVIEW:
            Predicate::And(lhs, rhs) => {
                self.intersection(&self.get_pred_type(lhs), &self.get_pred_type(rhs))
            }
            Predicate::Const(name) => todo!("get_pred_type({name})"),
        }
    }

    /// returns complement (not A)
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn complement(&self, ty: &Type) -> Type {
        match ty {
            FreeVar(fv) if fv.is_linked() => self.complement(&fv.crack()),
            Not(t) => *t.clone(),
            Refinement(r) => Type::Refinement(r.clone().invert()),
            Guard(guard) => Type::Guard(GuardType::new(
                guard.var.clone(),
                self.complement(&guard.to),
            )),
            Or(l, r) => self.intersection(&self.complement(l), &self.complement(r)),
            And(l, r) => self.union(&self.complement(l), &self.complement(r)),
            other => not(other.clone()),
        }
    }

    /// Returns difference of two types (`A - B` == `A and not B`).
    /// ```erg
    /// (A or B).diff(B) == A
    /// ```
    pub fn diff(&self, lhs: &Type, rhs: &Type) -> Type {
        match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
            (true, true) => return Type::Never, // lhs = rhs
            (false, false) => return lhs.clone(),
            _ => {}
        }
        match lhs {
            Type::FreeVar(fv) if fv.is_linked() => self.diff(&fv.crack(), rhs),
            // Type::And(l, r) => self.intersection(&self.diff(l, rhs), &self.diff(r, rhs)),
            Type::Or(l, r) => self.union(&self.diff(l, rhs), &self.diff(r, rhs)),
            _ => lhs.clone(),
        }
    }

    /// see doc/LANG/compiler/refinement_subtyping.md
    /// ```python
    /// assert is_super_pred({I >= 0}, {I == 0})
    /// assert is_super_pred({T >= 0}, {I == 0})
    /// assert !is_super_pred({I < 0}, {I == 0})
    /// ```
    fn is_super_pred_of(&self, lhs: &Predicate, rhs: &Predicate) -> bool {
        if lhs == rhs {
            return true;
        }
        match (lhs, rhs) {
            (Pred::Value(ValueObj::Bool(b)), _) => *b,
            (_, Pred::Value(ValueObj::Bool(b))) => !b,
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
            // 0..59 :> 1..20 == { I >= 0 and I < 60 } :> { I >= 1 and I < 20 }
            (Pred::And(l1, r1), Pred::And(l2, r2)) => {
                (self.is_super_pred_of(l1, l2) && self.is_super_pred_of(r1, r2))
                    || (self.is_super_pred_of(l1, r2) && self.is_super_pred_of(r1, l2))
            }
            (Pred::Or(l1, r1), Pred::Or(l2, r2)) => {
                (self.is_super_pred_of(l1, l2) && self.is_super_pred_of(r1, r2))
                    || (self.is_super_pred_of(l1, r2) && self.is_super_pred_of(r1, l2))
            }
            (lhs, Pred::And(l, r)) => {
                self.is_super_pred_of(lhs, l) || self.is_super_pred_of(lhs, r)
            }
            (lhs, Pred::Or(l, r)) => self.is_super_pred_of(lhs, l) && self.is_super_pred_of(lhs, r),
            (Pred::Or(l, r), rhs) => self.is_super_pred_of(l, rhs) || self.is_super_pred_of(r, rhs),
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
                for pred in refine.pred.ands() {
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
                for pred in refine.pred.ands() {
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

    /// If lhs and rhs are in a subtype relation, return the smaller one
    /// Return None if they are not related
    /// lhsとrhsが包含関係にあるとき小さいほうを返す
    /// 関係なければNoneを返す
    pub(crate) fn min<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> Option<&'t Type> {
        // If they are the same, either one can be returned.
        match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
            (true, true) | (true, false) => Some(rhs),
            (false, true) => Some(lhs),
            (false, false) => None,
        }
    }

    pub(crate) fn _max<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> Option<&'t Type> {
        // If they are the same, either one can be returned.
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
