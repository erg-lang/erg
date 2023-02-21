//! provides type-comparison
use std::option::Option; // conflicting to Type::Option

use erg_common::error::MultiErrorDisplay;
use erg_common::style::colors::DEBUG_ERROR;
use erg_common::traits::StructuralEq;

use crate::ty::constructors::{and, not, or, poly};
use crate::ty::free::{Constraint, FreeKind};
use crate::ty::typaram::{OpKind, TyParam, TyParamOrdering};
use crate::ty::value::ValueObj;
use crate::ty::value::ValueObj::Inf;
use crate::ty::{Predicate, RefinementType, SubrKind, SubrType, Type};
use Predicate as Pred;

use erg_common::dict::Dict;
use erg_common::{assume_unreachable, log};
use TyParamOrdering::*;
use Type::*;

use crate::context::cache::{SubtypePair, GLOBAL_TYPE_CACHE};
use crate::context::{Context, TyVarCache, Variance};

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

    pub(crate) fn eq_tp(&self, lhs: &TyParam, rhs: &TyParam, allow_cast: bool) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(lhs), TyParam::Type(rhs)) => {
                return self.same_type_of(lhs, rhs, allow_cast)
            }
            (TyParam::Mono(l), TyParam::Mono(r)) => {
                if let (Some(l), Some(r)) = (self.rec_get_const_obj(l), self.rec_get_const_obj(r)) {
                    return l == r;
                }
            }
            (TyParam::UnaryOp { op: lop, val: lval }, TyParam::UnaryOp { op: rop, val: rval }) => {
                return lop == rop && self.eq_tp(lval, rval, allow_cast);
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
                return lop == rop
                    && self.eq_tp(ll, rl, allow_cast)
                    && self.eq_tp(lr, rr, allow_cast);
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
                        .all(|(l, r)| self.eq_tp(l, r, allow_cast))
            }
            (TyParam::FreeVar(fv), other) | (other, TyParam::FreeVar(fv)) => match &*fv.borrow() {
                FreeKind::Linked(linked) | FreeKind::UndoableLinked { t: linked, .. } => {
                    return self.eq_tp(linked, other, allow_cast);
                }
                FreeKind::Unbound { constraint, .. }
                | FreeKind::NamedUnbound { constraint, .. } => {
                    let t = constraint.get_type().unwrap();
                    if cfg!(feature = "debug") && t == &Uninited {
                        panic!("Uninited type variable: {fv}");
                    }
                    let other_t = self.type_of(other);
                    return self.same_type_of(t, &other_t, allow_cast);
                }
            },
            (TyParam::Value(ValueObj::Type(l)), TyParam::Type(r)) => {
                return self.same_type_of(l.typ(), r.as_ref(), allow_cast);
            }
            (TyParam::Type(l), TyParam::Value(ValueObj::Type(r))) => {
                return self.same_type_of(l.as_ref(), r.typ(), allow_cast);
            }
            (l, r) if l == r => {
                return true;
            }
            (l, r) if l.has_unbound_var() || r.has_unbound_var() => {
                let lt = self.get_tp_t(l).unwrap();
                let rt = self.get_tp_t(r).unwrap();
                return self.same_type_of(&lt, &rt, allow_cast);
            }
            _ => {}
        }
        self.shallow_eq_tp(lhs, rhs)
    }

    pub(crate) fn related(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> bool {
        self.supertype_of(lhs, rhs, allow_cast) || self.subtype_of(lhs, rhs, allow_cast)
    }

    pub(crate) fn supertype_of(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> bool {
        let res = match Self::cheap_supertype_of(lhs, rhs, allow_cast) {
            (Absolutely, judge) => judge,
            (Maybe, judge) => {
                judge
                    || self.structural_supertype_of(lhs, rhs, allow_cast)
                    || self.nominal_supertype_of(lhs, rhs, allow_cast)
            }
        };
        log!("answer: {lhs} {DEBUG_ERROR}:>{RESET} {rhs} == {res}");
        res
    }

    /// e.g.
    /// Named :> Module
    /// => Module.super_types == [Named]
    /// Seq(T) :> Range(T)
    /// => Range(T).super_types == [Eq, Mutate, Seq('T), Output('T)]
    pub(crate) fn subtype_of(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> bool {
        match Self::cheap_subtype_of(lhs, rhs, allow_cast) {
            (Absolutely, judge) => judge,
            (Maybe, judge) => {
                judge
                    || self.structural_subtype_of(lhs, rhs, allow_cast)
                    || self.nominal_subtype_of(lhs, rhs, allow_cast)
            }
        }
    }

    pub(crate) fn same_type_of(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> bool {
        self.supertype_of(lhs, rhs, allow_cast) && self.subtype_of(lhs, rhs, allow_cast)
    }

    pub(crate) fn cheap_supertype_of(
        lhs: &Type,
        rhs: &Type,
        allow_cast: bool,
    ) -> (Credibility, bool) {
        if lhs == rhs {
            return (Absolutely, true);
        }
        match (lhs, rhs) {
            (Obj, _) | (_, Never | Failure) if allow_cast => (Absolutely, true),
            (_, Obj) if lhs.is_simple_class() && allow_cast => (Absolutely, false),
            (Never | Failure, _) if rhs.is_simple_class() && allow_cast => (Absolutely, false),
            (Float | Ratio | Int | Nat | Bool, Bool)
            | (Float | Ratio | Int | Nat, Nat)
            | (Float | Ratio | Int, Int)
            | (Float | Ratio, Ratio)
                if allow_cast =>
            {
                (Absolutely, true)
            }
            (Type, ClassType | TraitType) if allow_cast => (Absolutely, true),
            (Uninited, _) | (_, Uninited) => panic!("used an uninited type variable"),
            (
                Mono(n),
                Subr(SubrType {
                    kind: SubrKind::Func,
                    ..
                }),
            ) if &n[..] == "GenericFunc" && allow_cast => (Absolutely, true),
            (
                Mono(n),
                Subr(SubrType {
                    kind: SubrKind::Proc,
                    ..
                }),
            ) if &n[..] == "GenericProc" && allow_cast => (Absolutely, true),
            (Mono(l), Poly { name: r, .. })
                if &l[..] == "GenericArray" && &r[..] == "Array" && allow_cast =>
            {
                (Absolutely, true)
            }
            (Mono(l), Poly { name: r, .. })
                if &l[..] == "GenericDict" && &r[..] == "Dict" && allow_cast =>
            {
                (Absolutely, true)
            }
            (Mono(l), Mono(r))
                if &l[..] == "GenericCallable"
                    && (&r[..] == "GenericFunc"
                        || &r[..] == "GenericProc"
                        || &r[..] == "GenericFuncMethod"
                        || &r[..] == "GenericProcMethod")
                    && allow_cast =>
            {
                (Absolutely, true)
            }
            (FreeVar(l), FreeVar(r)) => {
                log!(err "{l}/{r}/{}", l.structural_eq(r));
                if l.structural_eq(r) {
                    (Absolutely, true)
                } else {
                    (Maybe, false)
                }
            }
            (_, FreeVar(fv)) | (FreeVar(fv), _) => match fv.get_subsup() {
                Some((Type::Never, Type::Obj)) if allow_cast => (Absolutely, true),
                _ => (Maybe, false),
            },
            (Mono(n), Subr(_) | Quantified(_)) if &n[..] == "GenericCallable" && allow_cast => {
                (Absolutely, true)
            }
            (lhs, rhs) if lhs.is_simple_class() && rhs.is_simple_class() => (Absolutely, false),
            _ => (Maybe, false),
        }
    }

    fn cheap_subtype_of(lhs: &Type, rhs: &Type, allow_cast: bool) -> (Credibility, bool) {
        Self::cheap_supertype_of(rhs, lhs, allow_cast)
    }

    /// make judgments that include supertypes in the same namespace & take into account glue patches
    /// 同一名前空間にある上位型を含めた判定&接着パッチを考慮した判定を行う
    fn nominal_supertype_of(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> bool {
        if !allow_cast && lhs != rhs {
            return false;
        }
        if let Some(res) = self.inquire_cache(lhs, rhs) {
            return res;
        }
        if let (Absolutely, judge) = self.classes_supertype_of(lhs, rhs, allow_cast) {
            self.register_cache(lhs, rhs, judge);
            return judge;
        }
        if let (Absolutely, judge) = self.traits_supertype_of(lhs, rhs, allow_cast) {
            self.register_cache(lhs, rhs, judge);
            return judge;
        }
        self.register_cache(lhs, rhs, false);
        false
    }

    fn nominal_subtype_of(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> bool {
        self.nominal_supertype_of(rhs, lhs, allow_cast)
    }

    pub(crate) fn find_patches_of<'a>(
        &'a self,
        typ: &'a Type,
        allow_cast: bool,
    ) -> impl Iterator<Item = &'a Context> {
        self.all_patches().into_iter().filter(move |ctx| {
            if let ContextKind::Patch(base) = &ctx.kind {
                return self.supertype_of(base, typ, allow_cast);
            }
            false
        })
    }

    fn _find_compatible_glue_patch(
        &self,
        sup: &Type,
        sub: &Type,
        allow_cast: bool,
    ) -> Option<&Context> {
        for patch in self.all_patches().into_iter() {
            if let ContextKind::GluePatch(tr_inst) = &patch.kind {
                if self.subtype_of(sub, &tr_inst.sub_type, allow_cast)
                    && self.subtype_of(&tr_inst.sup_trait, sup, allow_cast)
                {
                    return Some(patch);
                }
            }
        }
        None
    }

    fn classes_supertype_of(
        &self,
        lhs: &Type,
        rhs: &Type,
        allow_cast: bool,
    ) -> (Credibility, bool) {
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
                match Self::cheap_supertype_of(lhs, &rhs_sup, allow_cast) {
                    (Absolutely, true) => {
                        return (Absolutely, true);
                    }
                    (Maybe, _) => {
                        if self.structural_supertype_of(lhs, &rhs_sup, allow_cast) {
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
    fn traits_supertype_of(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> (Credibility, bool) {
        if !self.is_trait(lhs) {
            return (Maybe, false);
        }
        if let Some((_, rhs_ctx)) = self.get_nominal_type_ctx(rhs) {
            for rhs_sup in rhs_ctx.super_traits.iter() {
                // Not `supertype_of` (only structures are compared)
                match Self::cheap_supertype_of(lhs, rhs_sup, allow_cast) {
                    (Absolutely, true) => {
                        return (Absolutely, true);
                    }
                    (Maybe, _) => {
                        if self.structural_supertype_of(lhs, rhs_sup, allow_cast) {
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
    pub(crate) fn structural_supertype_of(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> bool {
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
                            if !self.subtype_of(lpt.typ(), rpt.typ(), allow_cast) {
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
                let return_t_judge = self.supertype_of(&ls.return_t, &rs.return_t, allow_cast); // covariant
                let non_defaults_judge = ls
                    .non_default_params
                    .iter()
                    .zip(rs.non_default_params.iter())
                    .all(|(l, r)| self.subtype_of(l.typ(), r.typ(), allow_cast));
                let var_params_judge = ls
                    .var_params
                    .as_ref()
                    .zip(rs.var_params.as_ref())
                    .map(|(l, r)| self.subtype_of(l.typ(), r.typ(), allow_cast))
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
                (Some((_, l_sup)), Some((r_sub, _))) => {
                    self.supertype_of(&l_sup, &r_sub, allow_cast)
                }
                _ => {
                    if lfv.is_linked() {
                        self.supertype_of(&lfv.crack(), rhs, allow_cast)
                    } else if rfv.is_linked() {
                        self.supertype_of(lhs, &rfv.crack(), allow_cast)
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
                        self.supertype_of(t, rhs, allow_cast)
                    }
                    FreeKind::Unbound { constraint: _, .. }
                    | FreeKind::NamedUnbound { constraint: _, .. } => {
                        if let Some((_sub, sup)) = lfv.get_subsup() {
                            lfv.forced_undoable_link(rhs);
                            let res = self.supertype_of(&sup, rhs, allow_cast);
                            lfv.undo();
                            res
                        } else if let Some(lfvt) = lfv.get_type() {
                            // e.g. lfv: ?L(: Int) is unreachable
                            // but
                            // ?L(: Array(Type, 3)) :> Array(Int, 3)
                            //   => Array(Type, 3) :> Array(Typeof(Int), 3)
                            //   => true
                            let rhs_meta = self.meta_type(rhs);
                            self.supertype_of(&lfvt, &rhs_meta, allow_cast)
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
                    self.supertype_of(lhs, t, allow_cast)
                }
                FreeKind::Unbound { constraint: _, .. }
                | FreeKind::NamedUnbound { constraint: _, .. } => {
                    if let Some((sub, _sup)) = rfv.get_subsup() {
                        rfv.forced_undoable_link(lhs);
                        let res = self.supertype_of(lhs, &sub, allow_cast);
                        rfv.undo();
                        res
                    } else if let Some(rfvt) = rfv.get_type() {
                        let lhs_meta = self.meta_type(lhs);
                        self.supertype_of(&lhs_meta, &rfvt, allow_cast)
                    } else {
                        // constraint is uninitalized
                        log!(err "constraint is uninitialized: {lhs}/{rfv}");
                        true
                    }
                }
            },
            (Type::Record(lhs), Type::Record(rhs)) => {
                for (k, l) in lhs.iter() {
                    if let Some(r) = rhs.get(k) {
                        if !self.supertype_of(l, r, allow_cast) {
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
                    if !self.supertype_of(&Type, t, allow_cast) {
                        return false;
                    }
                }
                true
            }
            (Type, Subr(subr)) => self.supertype_of(&Type, &subr.return_t, allow_cast),
            (Type, Poly { name, params }) | (Poly { name, params }, Type)
                if &name[..] == "Array" || &name[..] == "Set" =>
            {
                let elem_t = self.convert_tp_into_ty(params[0].clone()).unwrap();
                self.supertype_of(&Type, &elem_t, allow_cast)
            }
            (Type, Poly { name, params }) | (Poly { name, params }, Type)
                if &name[..] == "Tuple" =>
            {
                if let Ok(tps) = Vec::try_from(params[0].clone()) {
                    for tp in tps {
                        let Ok(t) = self.convert_tp_into_ty(tp) else {
                            return false;
                        };
                        if !self.supertype_of(&Type, &t, allow_cast) {
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
                    if !self.supertype_of(&Type, &k, allow_cast)
                        || !self.supertype_of(&Type, &v, allow_cast)
                    {
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
                if !self.supertype_of(&l.t, &r.t, allow_cast) {
                    return false;
                }
                let mut r_preds_clone = r.preds.clone();
                for l_pred in l.preds.iter() {
                    for r_pred in r.preds.iter() {
                        if l_pred.subject().unwrap_or("") == &l.var[..]
                            && r_pred.subject().unwrap_or("") == &r.var[..]
                            && self.is_super_pred_of(l_pred, r_pred, allow_cast)
                        {
                            r_preds_clone.remove(r_pred);
                        }
                    }
                }
                r_preds_clone.is_empty()
            }
            (Nat, re @ Refinement(_)) => {
                let nat = Type::Refinement(Nat.into_refinement());
                self.structural_supertype_of(&nat, re, allow_cast)
            }
            (re @ Refinement(_), Nat) => {
                let nat = Type::Refinement(Nat.into_refinement());
                self.structural_supertype_of(re, &nat, allow_cast)
            }
            // Int :> {I: Int | ...} == true
            // Int :> {I: Str| ...} == false
            // Eq({1, 2}) :> {1, 2} (= {I: Int | I == 1 or I == 2})
            // => Eq(Int) :> Eq({1, 2}) :> {1, 2}
            // => true
            // Bool :> {1} == true
            (l, Refinement(r)) => {
                if self.supertype_of(l, &r.t, allow_cast) {
                    return true;
                }
                let l = l.derefine();
                if self.supertype_of(&l, &r.t, allow_cast) {
                    return true;
                }
                let l = Type::Refinement(l.into_refinement());
                self.structural_supertype_of(&l, rhs, allow_cast)
            }
            // ({I: Int | True} :> Int) == true, ({N: Nat | ...} :> Int) == false, ({I: Int | I >= 0} :> Int) == false
            (Refinement(l), r) => {
                if l.preds
                    .iter()
                    .any(|p| p.mentions(&l.var) && p.can_be_false())
                {
                    return false;
                }
                self.supertype_of(&l.t, r, allow_cast)
            }
            (Quantified(l), Quantified(r)) => self.structural_subtype_of(l, r, allow_cast),
            (Quantified(quant), r) => {
                if quant.has_uninited_qvars() {
                    let mut tmp_tv_cache = TyVarCache::new(self.level, self);
                    let inst = self
                        .instantiate_t_inner(*quant.clone(), &mut tmp_tv_cache, &())
                        .unwrap();
                    self.supertype_of(&inst, r, allow_cast)
                } else {
                    self.supertype_of(quant, r, allow_cast)
                }
            }
            (l, Quantified(quant)) => {
                if quant.has_uninited_qvars() {
                    let mut tmp_tv_cache = TyVarCache::new(self.level, self);
                    let inst = self
                        .instantiate_t_inner(*quant.clone(), &mut tmp_tv_cache, &())
                        .unwrap();
                    self.supertype_of(l, &inst, allow_cast)
                } else {
                    self.supertype_of(l, quant, allow_cast)
                }
            }
            // Int or Str :> Str or Int == (Int :> Str && Str :> Int) || (Int :> Int && Str :> Str) == true
            (Or(l_1, l_2), Or(r_1, r_2)) => {
                (self.supertype_of(l_1, r_1, allow_cast) && self.supertype_of(l_2, r_2, allow_cast))
                    || (self.supertype_of(l_1, r_2, allow_cast)
                        && self.supertype_of(l_2, r_1, allow_cast))
            }
            // not Nat :> not Int == true
            (Not(l), Not(r)) => self.subtype_of(l, r, allow_cast),
            // (Int or Str) :> Nat == Int :> Nat || Str :> Nat == true
            // (Num or Show) :> Show == Num :> Show || Show :> Num == true
            (Or(l_or, r_or), rhs) => {
                self.supertype_of(l_or, rhs, allow_cast) || self.supertype_of(r_or, rhs, allow_cast)
            }
            // Int :> (Nat or Str) == Int :> Nat && Int :> Str == false
            (lhs, Or(l_or, r_or)) => {
                self.supertype_of(lhs, l_or, allow_cast) && self.supertype_of(lhs, r_or, allow_cast)
            }
            (And(l_1, l_2), And(r_1, r_2)) => {
                (self.supertype_of(l_1, r_1, allow_cast) && self.supertype_of(l_2, r_2, allow_cast))
                    || (self.supertype_of(l_1, r_2, allow_cast)
                        && self.supertype_of(l_2, r_1, allow_cast))
            }
            // (Num and Show) :> Show == false
            (And(l_and, r_and), rhs) => {
                self.supertype_of(l_and, rhs, allow_cast)
                    && self.supertype_of(r_and, rhs, allow_cast)
            }
            // Show :> (Num and Show) == true
            (lhs, And(l_and, r_and)) => {
                self.supertype_of(lhs, l_and, allow_cast)
                    || self.supertype_of(lhs, r_and, allow_cast)
            }
            // RefMut are invariant
            (Ref(l), Ref(r)) => self.supertype_of(l, r, allow_cast),
            // TはすべてのRef(T)のメソッドを持つので、Ref(T)のサブタイプ
            // REVIEW: RefMut is invariant, maybe
            (Ref(l), r) => self.supertype_of(l, r, allow_cast),
            (RefMut { before: l, .. }, r) => self.supertype_of(l, r, allow_cast),
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
                    self.supertype_of(&lt, &rt, allow_cast)
                        && self
                            .eval_bin_tp(OpKind::Le, llen, rlen)
                            .map(|tp| matches!(tp, TyParam::Value(ValueObj::Bool(true))))
                            .unwrap_or_else(|e| {
                                e.fmt_all_stderr();
                                todo!();
                            })
                } else {
                    self.poly_supertype_of(lhs, lparams, rparams, allow_cast)
                }
            }
            (Proj { .. }, _) => {
                if let Some(cands) = self.get_candidates(lhs) {
                    for cand in cands.into_iter() {
                        if self.supertype_of(&cand, rhs, allow_cast) {
                            return true;
                        }
                    }
                }
                false
            }
            (_, Proj { .. }) => {
                if let Some(cands) = self.get_candidates(rhs) {
                    for cand in cands.into_iter() {
                        if self.supertype_of(lhs, &cand, allow_cast) {
                            return true;
                        }
                    }
                }
                false
            }
            (_l, _r) => false,
        }
    }

    pub(crate) fn poly_supertype_of(
        &self,
        typ: &Type,
        lparams: &[TyParam],
        rparams: &[TyParam],
        allow_cast: bool,
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
            .all(|((lp, rp), variance)| self.supertype_of_tp(lp, rp, *variance, allow_cast))
    }

    fn supertype_of_tp(
        &self,
        lp: &TyParam,
        rp: &TyParam,
        variance: Variance,
        allow_cast: bool,
    ) -> bool {
        if lp == rp {
            return true;
        }
        match (lp, rp, variance) {
            (TyParam::FreeVar(fv), _, _) if fv.is_linked() => {
                self.supertype_of_tp(&fv.crack(), rp, variance, allow_cast)
            }
            (_, TyParam::FreeVar(fv), _) if fv.is_linked() => {
                self.supertype_of_tp(lp, &fv.crack(), variance, allow_cast)
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
                    if !self.supertype_of_tp(l, r, variance, allow_cast) {
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
                        if !self.supertype_of_tp(lv, rv, variance, allow_cast) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (TyParam::Type(l), TyParam::Type(r), Variance::Contravariant) => {
                self.subtype_of(l, r, allow_cast)
            }
            (TyParam::Type(l), TyParam::Type(r), Variance::Covariant) => {
                self.supertype_of(l, r, allow_cast)
            }
            (TyParam::Type(l), TyParam::Type(r), Variance::Invariant) => {
                self.same_type_of(l, r, allow_cast)
            }
            (TyParam::FreeVar(fv), _, _) if fv.is_unbound() => {
                let fv_t = fv.get_type().unwrap();
                let rp_t = self.get_tp_t(rp).unwrap();
                if variance == Variance::Contravariant {
                    self.subtype_of(&fv_t, &rp_t, allow_cast)
                } else if variance == Variance::Covariant {
                    self.supertype_of(&fv_t, &rp_t, allow_cast)
                } else {
                    self.same_type_of(&fv_t, &rp_t, allow_cast)
                }
            }
            _ => self.eq_tp(lp, rp, allow_cast),
        }
    }

    /// lhs <: rhs?
    pub(crate) fn structural_subtype_of(&self, lhs: &Type, rhs: &Type, allow_cast: bool) -> bool {
        self.structural_supertype_of(rhs, lhs, allow_cast)
    }

    pub(crate) fn _structural_same_type_of(
        &self,
        lhs: &Type,
        rhs: &Type,
        allow_cast: bool,
    ) -> bool {
        self.structural_supertype_of(lhs, rhs, allow_cast)
            && self.structural_subtype_of(lhs, rhs, allow_cast)
    }

    pub(crate) fn try_cmp(
        &self,
        l: &TyParam,
        r: &TyParam,
        allow_cast: bool,
    ) -> Option<TyParamOrdering> {
        match (l, r) {
            (TyParam::Value(l), TyParam::Value(r)) =>
                l.try_cmp(r).map(Into::into),
            // TODO: 型を見て判断する
            (TyParam::BinOp{ op, lhs, rhs }, r) => {
                if let Ok(l) = self.eval_bin_tp(*op, lhs.as_ref().clone(), rhs.as_ref().clone()) {
                    self.try_cmp(&l, r, allow_cast)
                } else { Some(Any) }
            },
            (TyParam::FreeVar(fv), p) if fv.is_linked() => {
                self.try_cmp(&fv.crack(), p, allow_cast)
            }
            (p, TyParam::FreeVar(fv)) if fv.is_linked() => {
                self.try_cmp(p, &fv.crack(), allow_cast)
            }
            (
                l @ (TyParam::FreeVar(_) | TyParam::Erased(_)),
                r @ (TyParam::FreeVar(_) | TyParam::Erased(_)),
            ) /* if v.is_unbound() */ => {
                let l_t = self.get_tp_t(l).unwrap();
                let r_t = self.get_tp_t(r).unwrap();
                if self.supertype_of(&l_t, &r_t, allow_cast) || self.subtype_of(&l_t, &r_t, allow_cast) {
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
                        self.try_cmp(&inf, p, allow_cast).unwrap(),
                        self.try_cmp(&sup, p, allow_cast).unwrap()
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
                    match (self.supertype_of(&lt, &pt, allow_cast), self.subtype_of(&lt, &pt, allow_cast)) {
                        (true, true) => Some(Any),
                        (true, false) => Some(Any),
                        (false, true) => Some(NotEqual),
                        (false, false) => Some(NoRelation),
                    }
                }
            }
            (l, r @ (TyParam::Erased(_) | TyParam::FreeVar(_))) =>
                self.try_cmp(r, l, allow_cast).map(|ord| ord.reverse()),
            (_l, _r) => {
                erg_common::fmt_dbg!(_l, _r,);
                None
            },
        }
    }

    /// returns union of two types (A or B)
    pub(crate) fn union(&self, lhs: &Type, rhs: &Type) -> Type {
        let allow_cast = true;
        if lhs == rhs {
            return lhs.clone();
        }
        // `?T or ?U` will not be unified
        // `Set!(?T, 3) or Set(?T, 3)` wii be unified to Set(?T, 3)
        if !lhs.is_unbound_var() && !rhs.is_unbound_var() {
            match (
                self.supertype_of(lhs, rhs, allow_cast),
                self.subtype_of(lhs, rhs, allow_cast),
            ) {
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
                            if self.eq_tp(lp, rp, allow_cast) {
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
        let allow_cast = true;
        if lhs == rhs {
            return lhs.clone();
        }
        // ?T and ?U will not be unified
        if !lhs.is_unbound_var() && !rhs.is_unbound_var() {
            match (
                self.supertype_of(lhs, rhs, allow_cast),
                self.subtype_of(lhs, rhs, allow_cast),
            ) {
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
            // {.i = Int} and {.s = Str} == {.i = Int; .s = Str}
            (Record(l), Record(r)) => Type::Record(l.clone().concat(r.clone())),
            // {i = Int; j = Int} and not {i = Int} == {j = Int}
            // not {i = Int} and {i = Int; j = Int} == {j = Int}
            (other @ Record(rec), Not(t)) | (Not(t), other @ Record(rec)) => match t.as_ref() {
                Type::FreeVar(fv) => self.intersection(&fv.crack(), other),
                Type::Record(rec2) => Type::Record(rec.clone().diff(rec2.clone())),
                _ => Type::Never,
            },
            (l, r) if self.is_trait(l) && self.is_trait(r) => and(l.clone(), r.clone()),
            (_l, _r) => Type::Never,
        }
    }

    fn intersection_refinement(
        &self,
        lhs: &RefinementType,
        rhs: &RefinementType,
    ) -> RefinementType {
        let intersec = self.intersection(&lhs.t, &rhs.t);
        let name = lhs.var.clone();
        let rhs_preds = rhs
            .preds
            .iter()
            .map(|p| p.clone().change_subject_name(name.clone()))
            .collect();
        RefinementType::new(
            lhs.var.clone(),
            intersec,
            lhs.preds.clone().concat(rhs_preds),
        )
    }

    /// returns complement (not A)
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn complement(&self, ty: &Type) -> Type {
        match ty {
            FreeVar(fv) if fv.is_linked() => self.complement(&fv.crack()),
            Not(t) => *t.clone(),
            Refinement(r) => Type::Refinement(r.clone().invert()),
            other => not(other.clone()),
        }
    }

    /// see doc/LANG/compiler/refinement_subtyping.md
    /// ```python
    /// assert is_super_pred({I >= 0}, {I == 0})
    /// assert is_super_pred({T >= 0}, {I == 0})
    /// assert !is_super_pred({I < 0}, {I == 0})
    /// ```
    fn is_super_pred_of(&self, lhs: &Predicate, rhs: &Predicate, allow_cast: bool) -> bool {
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
                .try_cmp(rhs, rhs2, allow_cast)
                .map(|ord| ord.canbe_eq())
                .unwrap_or(false),
            // {T >= 0} :> {T >= 1}, {T >= 0} :> {T == 1}
            (
                Pred::GreaterEqual { rhs, .. },
                Pred::GreaterEqual { rhs: rhs2, .. } | Pred::Equal { rhs: rhs2, .. },
            ) => self
                .try_cmp(rhs, rhs2, allow_cast)
                .map(|ord| ord.canbe_le())
                .unwrap_or(false),
            (
                Pred::LessEqual { rhs, .. },
                Pred::LessEqual { rhs: rhs2, .. } | Pred::Equal { rhs: rhs2, .. },
            ) => self
                .try_cmp(rhs, rhs2, allow_cast)
                .map(|ord| ord.canbe_ge())
                .unwrap_or(false),
            (lhs @ (Pred::GreaterEqual { .. } | Pred::LessEqual { .. }), Pred::And(l, r)) => {
                self.is_super_pred_of(lhs, l, allow_cast)
                    || self.is_super_pred_of(lhs, r, allow_cast)
            }
            (lhs, Pred::Or(l, r)) => {
                self.is_super_pred_of(lhs, l, allow_cast)
                    && self.is_super_pred_of(lhs, r, allow_cast)
            }
            (Pred::Or(l, r), rhs @ (Pred::GreaterEqual { .. } | Pred::LessEqual { .. })) => {
                self.is_super_pred_of(l, rhs, allow_cast)
                    || self.is_super_pred_of(r, rhs, allow_cast)
            }
            (Pred::And(l, r), rhs) => {
                self.is_super_pred_of(l, rhs, allow_cast)
                    && self.is_super_pred_of(r, rhs, allow_cast)
            }
            (lhs, rhs) => todo!("{lhs}/{rhs}"),
        }
    }

    pub(crate) fn is_sub_constraint_of(
        &self,
        l: &Constraint,
        r: &Constraint,
        allow_cast: bool,
    ) -> bool {
        match (l, r) {
            // (?I: Nat) <: (?I: Int)
            (Constraint::TypeOf(lhs), Constraint::TypeOf(rhs)) => {
                self.subtype_of(lhs, rhs, allow_cast)
            }
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
            ) => {
                self.supertype_of(lsub, rsub, allow_cast) && self.subtype_of(lsup, rsup, allow_cast)
            }
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
                                if self.try_cmp(rhs, max, true) == Some(Greater) {
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
                                if self.try_cmp(rhs, min, true) == Some(Less) {
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
        match (
            self.supertype_of(lhs, rhs, true),
            self.subtype_of(lhs, rhs, true),
        ) {
            (true, true) | (true, false) => Some(rhs),
            (false, true) => Some(lhs),
            (false, false) => None,
        }
    }

    pub(crate) fn _max<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> Option<&'t Type> {
        // If they are the same, either one can be returned.
        match (
            self.supertype_of(lhs, rhs, true),
            self.subtype_of(lhs, rhs, true),
        ) {
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
