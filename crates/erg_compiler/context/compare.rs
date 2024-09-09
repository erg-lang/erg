//! provides type-comparison
use std::iter::repeat;
use std::option::Option; // conflicting to Type::Option

use erg_common::consts::DEBUG_MODE;
use erg_common::dict::Dict;
use erg_common::set::Set;
use erg_common::style::colors::DEBUG_ERROR;
use erg_common::traits::StructuralEq;
use erg_common::{assume_unreachable, log};
use erg_common::{Str, Triple};

use crate::context::eval::UndoableLinkedList;
use crate::context::initialize::const_func::sub_tpdict_get;
use crate::ty::constructors::{self, and, bounded, not, or, poly, refinement};
use crate::ty::free::{Constraint, FreeTyVar};
use crate::ty::typaram::{TyParam, TyParamOrdering};
use crate::ty::value::ValueObj;
use crate::ty::value::ValueObj::Inf;
use crate::ty::{
    Field, GuardType, Predicate, RefinementType, SubrKind, SubrType, Type, VisibilityModifier,
};
use Predicate as Pred;

use TyParamOrdering::*;
use Type::*;

use crate::context::{Context, Variance};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Credibility {
    Maybe,
    Absolutely,
}

use Credibility::*;

use super::eval::Substituter;
use super::ContextKind;

impl Context {
    pub(crate) fn eq_tp(&self, lhs: &TyParam, rhs: &TyParam) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(lhs), TyParam::Type(rhs))
            | (TyParam::Erased(lhs), TyParam::Erased(rhs)) => return self.same_type_of(lhs, rhs),
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
            (TyParam::FreeVar(fv), other) | (other, TyParam::FreeVar(fv)) if fv.is_linked() => {
                return self.eq_tp(&fv.get_linked().unwrap(), other)
            }
            (TyParam::FreeVar(fv), other) | (other, TyParam::FreeVar(fv))
                if fv.get_type().is_some() =>
            {
                let t = fv.get_type().unwrap();
                if DEBUG_MODE && t == Uninited {
                    panic!("Uninited type variable: {fv}");
                }
                let other_t = self.type_of(other);
                return self.same_type_of(&t, &other_t);
            }
            (TyParam::Value(ValueObj::Type(l)), TyParam::Type(r)) => {
                return self.same_type_of(l.typ(), r.as_ref());
            }
            (TyParam::Type(l), TyParam::Value(ValueObj::Type(r))) => {
                return self.same_type_of(l.as_ref(), r.typ());
            }
            (l, r) if l.has_unbound_var() || r.has_unbound_var() => {
                let Ok(lt) = self.get_tp_t(l) else {
                    return false;
                };
                let Ok(rt) = self.get_tp_t(r) else {
                    return false;
                };
                return self.same_type_of(&lt, &rt);
            }
            _ => {}
        }
        self.shallow_eq_tp(lhs, rhs)
    }

    pub(crate) fn related(&self, lhs: &Type, rhs: &Type) -> bool {
        self.supertype_of(lhs, rhs) || self.subtype_of(lhs, rhs)
    }

    pub(crate) fn _related_tp(&self, lhs: &TyParam, rhs: &TyParam) -> bool {
        self._subtype_of_tp(lhs, rhs, Variance::Covariant)
            || self.supertype_of_tp(lhs, rhs, Variance::Covariant)
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
            (_, Obj) if lhs.is_mono_value_class() => (Absolutely, false),
            (Never | Failure, _) if rhs.is_mono_value_class() => (Absolutely, false),
            (Complex | Float | Ratio | Int | Nat | Bool, Bool)
            | (Complex | Float | Ratio | Int | Nat, Nat)
            | (Complex | Float | Ratio | Int, Int)
            | (Complex | Float | Ratio, Ratio)
            | (Complex | Float, Float) => (Absolutely, true),
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
            (Mono(l), Poly { name: r, .. }) if &l[..] == "GenericList" && &r[..] == "List" => {
                (Absolutely, true)
            }
            (Mono(l), Poly { name: r, .. }) if &l[..] == "GenericDict" && &r[..] == "Dict" => {
                (Absolutely, true)
            }
            (Mono(l), Mono(r))
                if &l[..] == "Subroutine"
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
            (Mono(n), Subr(_) | Quantified(_))
                if &n[..] == "Subroutine" || &n[..] == "GenericCallable" =>
            {
                (Absolutely, true)
            }
            (lhs, rhs) if lhs.is_mono_value_class() && rhs.is_mono_value_class() => {
                (Absolutely, false)
            }
            _ => (Maybe, false),
        }
    }

    fn cheap_subtype_of(lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        Self::cheap_supertype_of(rhs, lhs)
    }

    /// make judgments that include supertypes in the same namespace & take into account glue patches
    /// 同一名前空間にある上位型を含めた判定&接着パッチを考慮した判定を行う
    fn nominal_supertype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        if let (Absolutely, judge) = self.classes_supertype_of(lhs, rhs) {
            return judge;
        }
        if let (Absolutely, judge) = self.traits_supertype_of(lhs, rhs) {
            return judge;
        }
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

    fn _nominal_subtype_of<'c>(
        &'c self,
        lhs: &Type,
        rhs: &Type,
        get_types: impl Fn(&'c Context) -> &'c [Type],
    ) -> (Credibility, bool) {
        if let Some(ty_ctx) = self.get_nominal_type_ctx(rhs) {
            let typ = &ty_ctx.typ;
            let substitute = typ.has_qvar();
            let overwrite = typ.has_undoable_linked_var();
            let _substituter = if overwrite {
                match Substituter::overwrite_typarams(self, typ, rhs) {
                    Ok(subs) => Some(subs),
                    Err(err) => {
                        if DEBUG_MODE {
                            panic!("{typ} / {rhs}: err: {err}");
                        }
                        None
                    }
                }
            } else if substitute {
                match Substituter::substitute_typarams(self, typ, rhs) {
                    Ok(subs) => Some(subs),
                    Err(err) => {
                        if DEBUG_MODE {
                            panic!("{typ} / {rhs}: err: {err}");
                        }
                        None
                    }
                }
            } else {
                None
            };
            for rhs_sup in get_types(ty_ctx) {
                let _subs = Substituter::substitute_self(rhs_sup, rhs, self);
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

    fn classes_supertype_of(&self, lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        if !self.is_class(lhs) || !self.is_class(rhs) {
            return (Maybe, false);
        }
        self._nominal_subtype_of(lhs, rhs, |ty_ctx| &ty_ctx.super_classes)
    }

    // e.g. Eq(Nat) :> Nat
    // Nat.super_traits = [Add(Nat), Eq(Nat), Sub(Float), ...]
    // e.g. Eq :> ?L or ?R (if ?L <: Eq and ?R <: Eq)
    fn traits_supertype_of(&self, lhs: &Type, rhs: &Type) -> (Credibility, bool) {
        if !self.is_trait(lhs) {
            return (Maybe, false);
        }
        let (cred, judge) = self._nominal_subtype_of(lhs, rhs, |ty_ctx| &ty_ctx.super_traits[..]);
        if judge {
            return (cred, judge);
        }
        self._nominal_subtype_of(lhs, rhs, |ty_ctx| &ty_ctx.super_classes[..])
    }

    /// lhs :> rhs?
    /// ```python
    /// assert supertype_of(Int, Nat) # i: Int = 1 as Nat
    /// assert supertype_of(Bool, Bool)
    /// ```
    /// This function does not consider the nominal subtype relation.
    /// Use `supertype_of` for complete judgement.
    /// 単一化、評価等はここでは行わない、スーパータイプになる **可能性があるか** だけ判定する
    /// ので、lhsが(未連携)型変数の場合は単一化せずにtrueを返す
    pub(crate) fn structural_supertype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        match (lhs, rhs) {
            // Proc :> Func if params are compatible
            // * default params can be omitted (e.g. (Int, x := Int) -> Int <: (Int) -> Int)
            // * and default params can be non-default (e.g. (Int, x := Int) -> Int <: (Int, Int) -> Int)
            (Subr(ls), Subr(rs)) if ls.kind == rs.kind || ls.kind.is_proc() => {
                let default_check = || {
                    for lpt in ls.default_params.iter() {
                        if let Some(rpt) = rs
                            .default_params
                            .iter()
                            .find(|rpt| rpt.name() == lpt.name())
                        {
                            if !self.subtype_of(lpt.typ(), rpt.typ()) {
                                return false;
                            }
                        } else if let Some(kw) = rs.kw_var_params.as_ref() {
                            if !self.subtype_of(lpt.typ(), kw.typ()) {
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
                // (Int, n := Int) -> Int <: (Int, Int) -> Int
                // (Int, n := Int, m := Int) -> Int <: (Int, Int) -> Int
                // (Int, n := Int) -> Int <!: (Int, Int, Int) -> Int
                // (*Int) -> Int <: (Int, Int) -> Int
                // (self: Self, T) -> U <: T -> U
                let len_judge = ls.non_default_params.len()
                    <= rs.non_default_params.len() + rs.default_params.len()
                    || rs.var_params.is_some();
                // && ls.default_params.len() <= rs.default_params.len();
                let rhs_ret = rs
                    .return_t
                    .clone()
                    .replace_params(rs.param_names(), ls.param_names());
                let return_t_judge = self.supertype_of(&ls.return_t, &rhs_ret); // covariant
                let non_defaults_judge = if let Some(r_var) = rs.var_params.as_deref() {
                    ls.non_default_params
                        .iter()
                        .zip(repeat(r_var))
                        .all(|(l, r)| self.subtype_of(l.typ(), r.typ()))
                } else {
                    let rs_params = if !ls.is_method() && rs.is_method() {
                        rs.non_default_params
                            .iter()
                            .skip(1)
                            .chain(&rs.default_params)
                    } else {
                        #[allow(clippy::iter_skip_zero)]
                        rs.non_default_params
                            .iter()
                            .skip(0)
                            .chain(&rs.default_params)
                    };
                    ls.non_default_params
                        .iter()
                        .zip(rs_params)
                        .all(|(l, r)| self.subtype_of(l.typ(), r.typ()))
                };
                let var_params_judge = ls
                    .var_params
                    .as_ref()
                    .zip(rs.var_params.as_ref())
                    .map(|(l, r)| self.subtype_of(l.typ(), r.typ()))
                    .unwrap_or(true);
                len_judge
                    && return_t_judge
                    && non_defaults_judge
                    && var_params_judge
                    && default_check() // contravariant
            }
            // ?T(<: Int) :> ?U(:> Nat)
            // ?T(<: Int) :> ?U(:> Int)
            // ?T(<: Nat) !:> ?U(:> Int) (if the upper bound of LHS is smaller than the lower bound of RHS, LHS cannot not be a supertype)
            // ?T(<: Nat) :> ?U(<: Int) (?U can be smaller than ?T)
            // ?T(:> ?U) :> ?U
            // ?U :> ?T(<: ?U)
            // ?T(: {Int, Str}) :> ?U(<: Int)
            (FreeVar(lfv), FreeVar(rfv)) => match (lfv.get_subsup(), rfv.get_subsup()) {
                (Some((_, l_sup)), Some((r_sub, _))) => self.supertype_of(&l_sup, &r_sub),
                (Some((l_sub, _)), None) if &l_sub == rhs => true,
                (None, Some((_, r_sup))) if lhs == &r_sup => true,
                _ => {
                    let lfvt = lfv.get_type();
                    // lfv: T: {Int, Str}, rhs: Int
                    if let Some(tys) = lfvt.as_ref().and_then(|t| t.refinement_values()) {
                        for tp in tys {
                            let Ok(ty) = self.convert_tp_into_type(tp.clone()) else {
                                continue;
                            };
                            if self.supertype_of(&ty, rhs) {
                                return true;
                            }
                        }
                    }
                    if lfv.is_linked() {
                        self.supertype_of(&lfv.crack(), rhs)
                    } else if rfv.is_linked() {
                        self.supertype_of(lhs, &rfv.crack())
                    } else {
                        false
                    }
                }
            },
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
            (
                ProjCall {
                    lhs: l,
                    attr_name,
                    args,
                },
                _,
            ) => {
                if let Ok(evaled) = self.eval_proj_call_t(
                    *l.clone(),
                    attr_name.clone(),
                    args.clone(),
                    self.level,
                    &(),
                ) {
                    if lhs != &evaled {
                        return self.supertype_of(&evaled, rhs);
                    }
                }
                false
            }
            (
                _,
                ProjCall {
                    lhs: r,
                    attr_name,
                    args,
                },
            ) => {
                if let Ok(evaled) = self.eval_proj_call_t(
                    *r.clone(),
                    attr_name.clone(),
                    args.clone(),
                    self.level,
                    &(),
                ) {
                    if &evaled != rhs {
                        return self.supertype_of(lhs, &evaled);
                    }
                }
                false
            }
            // true if it can be a supertype, false if it cannot (due to type constraints)
            // No type constraints are imposed here, as subsequent type decisions are made according to the possibilities
            // ?P(<: Mul ?P) :> Int
            //   => ?P.undoable_link(Int)
            //   => Mul Int :> Int
            (FreeVar(lfv), rhs) => {
                if let Some(t) = lfv.get_linked() {
                    return self.supertype_of(&t, rhs);
                }
                if let Some((_sub, sup)) = lfv.get_subsup() {
                    lfv.do_avoiding_recursion_with(rhs, || self.supertype_of(&sup, rhs))
                } else if let Some(lfvt) = lfv.get_type() {
                    // lfv: T: {Int, Str}, rhs: Int
                    if let Some(tys) = lfvt.refinement_values() {
                        for tp in tys {
                            let Ok(ty) = self.convert_tp_into_type(tp.clone()) else {
                                continue;
                            };
                            if self.supertype_of(&ty, rhs) {
                                return true;
                            }
                        }
                    }
                    // e.g. lfv: ?L(: Int) is unreachable
                    // but
                    // ?L(: List(Type, 3)) :> List(Int, 3)
                    //   => List(Type, 3) :> List(Typeof(Int), 3)
                    //   => true
                    let rhs_meta = self.meta_type(rhs);
                    self.supertype_of(&lfvt, &rhs_meta)
                } else {
                    // constraint is uninitialized
                    log!(err "constraint is uninitialized: {lfv}/{rhs}");
                    true
                }
            }
            (lhs, FreeVar(rfv)) => {
                if let Some(t) = rfv.get_linked() {
                    return self.supertype_of(lhs, &t);
                }
                if let Some((sub, _sup)) = rfv.get_subsup() {
                    rfv.do_avoiding_recursion_with(lhs, || self.supertype_of(lhs, &sub))
                } else if let Some(rfvt) = rfv.get_type() {
                    let lhs_meta = self.meta_type(lhs);
                    self.supertype_of(&lhs_meta, &rfvt)
                } else {
                    // constraint is uninitialized
                    log!(err "constraint is uninitialized: {lhs}/{rfv}");
                    true
                }
            }
            (Record(lhs), Record(rhs)) => {
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
            (NamedTuple(lhs), NamedTuple(rhs)) => {
                for ((l_k, l_t), (r_k, r_t)) in lhs.iter().zip(rhs.iter()) {
                    if (l_k.vis.is_public() && r_k.vis.is_private()) || !self.supertype_of(l_t, r_t)
                    {
                        return false;
                    }
                }
                true
            }
            (Bool, Guard { .. }) => true,
            (Guard(lhs), Guard(rhs)) => {
                lhs.target == rhs.target && self.supertype_of(&lhs.to, &rhs.to)
            }
            (Mono(n), NamedTuple(_)) => {
                &n[..] == "Tuple"
                    || &n[..] == "GenericNamedTuple"
                    || &n[..] == "HomogenousTuple"
                    || &n[..] == "GenericTuple"
            }
            (Mono(n), Record(_)) => &n[..] == "Record",
            (ty @ (Type | ClassType | TraitType), Record(rec)) => {
                for (_, t) in rec.iter() {
                    if !self.supertype_of(ty, t) {
                        return false;
                    }
                }
                true
            }
            (ty @ (Type | ClassType | TraitType), Subr(subr)) => {
                self.supertype_of(ty, &subr.return_t)
            }
            (Type | ClassType | TraitType, Poly { name, params }) if &name[..] == "Set" => {
                self.convert_tp_into_value(params[0].clone()).is_ok()
            }
            (Type | ClassType, Poly { name, params }) if &name[..] == "Range" => {
                self.convert_tp_into_value(params[0].clone()).is_ok()
            }
            (ty @ (Type | ClassType | TraitType), Poly { name, params })
                if &name[..] == "List" || &name[..] == "UnsizedList" || &name[..] == "Set" =>
            {
                let Ok(elem_t) = self.convert_tp_into_type(params[0].clone()) else {
                    return false;
                };
                self.supertype_of(ty, &elem_t)
            }
            (ty @ (Type | ClassType | TraitType), Poly { name, params })
                if &name[..] == "Tuple" =>
            {
                // Type :> Tuple Ts == Type :> Ts
                // e.g. Type :> Tuple [Int, Str] == false
                //      Type :> Tuple [Type, Type] == true
                if let Ok(arr_t) = self.convert_tp_into_type(params[0].clone()) {
                    return self.supertype_of(ty, &arr_t);
                } else if let Ok(tps) = Vec::try_from(params[0].clone()) {
                    for tp in tps {
                        let Ok(t) = self.convert_tp_into_type(tp) else {
                            return false;
                        };
                        if !self.supertype_of(ty, &t) {
                            return false;
                        }
                    }
                }
                true
            }
            (ty @ (Type | ClassType | TraitType), Poly { name, params }) if &name[..] == "Dict" => {
                // Type :> Dict T == Type :> T
                // e.g.
                //      Type :> Dict {"a": 1} == false
                //      Type :> Dict {Str: Int} == true
                //      Type :> Dict {Type: Type} == true
                if let Ok(_dict_t) = self.convert_tp_into_type(params[0].clone()) {
                    return true;
                }
                // HACK: e.g. ?D: GenericDict
                let Ok(dict) = Dict::try_from(params[0].clone()) else {
                    return false;
                };
                for (k, v) in dict.into_iter() {
                    let Ok(k) = self.convert_tp_into_type(k) else {
                        return false;
                    };
                    let Ok(v) = self.convert_tp_into_type(v) else {
                        return false;
                    };
                    if !self.supertype_of(ty, &k) || !self.supertype_of(ty, &v) {
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
                    let refined = l.t.clone().into_refinement();
                    if !self.supertype_of(&refined.t, &r.t) {
                        return false;
                    }
                }
                for tp in r.pred.possible_tps() {
                    let substituted = l.pred.clone().substitute(&l.var, tp);
                    if self.bool_eval_pred(substituted).is_ok_and(|b| b) {
                        return true;
                    }
                }
                if self.is_super_pred_of(&l.pred, &r.pred) {
                    true
                } else {
                    let list = UndoableLinkedList::new();
                    for tp in l.t.typarams() {
                        list.push_tp(&tp);
                    }
                    let _ = self.undoable_sub_unify(&r.t, &l.t, &(), &list, None);
                    self.is_super_pred_of(&l.pred, &r.pred)
                }
            }
            (Nat | Bool, re @ Refinement(_)) => {
                let refine = Type::Refinement(lhs.clone().into_refinement());
                self.structural_supertype_of(&refine, re)
            }
            (re @ Refinement(_), Nat | Bool) => {
                let refine = Type::Refinement(rhs.clone().into_refinement());
                self.structural_supertype_of(re, &refine)
            }
            (Structural(_), Refinement(refine)) => self.supertype_of(lhs, &refine.t),
            // Int :> {I: Int | ...} == true
            // Int :> {I: Str| ...} == false
            // Bool :> {1} == true
            // Bool :> {2} == false
            // [2, 3]: {A: List(Nat) | A.prod() == 6}
            // List({1, 2}, _) :> {[3, 4]} == false
            (l, Refinement(r)) => {
                // Type / {S: Set(Str) | S == {"a", "b"}}
                // TODO: GeneralEq
                if let Pred::Equal { rhs, .. } = r.pred.as_ref() {
                    if self.subtype_of(l, &Type) && self.convert_tp_into_type(rhs.clone()).is_ok() {
                        return true;
                    }
                }
                if self.supertype_of(l, &r.t) {
                    return true;
                } else if self.subtype_of(l, &r.t) {
                    return false;
                }
                if r.t.is_monomorphic() {
                    let l = l.derefine();
                    if self.supertype_of(&l, &r.t) {
                        return true;
                    }
                }
                let l = Type::Refinement(l.clone().into_refinement());
                self.structural_supertype_of(&l, rhs)
            }
            // {1, 2, 3} :> {1} or {2, 3} == true
            (Refinement(_refine), Or(l, r)) => {
                self.supertype_of(lhs, l) && self.supertype_of(lhs, r)
            }
            // ({I: Int | True} :> Int) == true
            // {N: Nat | ...} :> Int) == false
            // ({I: Int | I >= 0} :> Int) == false
            // {U(: Type)} :> { .x = {Int} }(== {{ .x = Int }}) == true
            // {[1]} == List({1}, 1) :> List({1}, _) == true
            (Refinement(l), r) => {
                if let Some(r) = r.to_singleton() {
                    return self.structural_supertype_of(lhs, &Type::Refinement(r));
                } else if let Some(l) = self.refinement_to_poly(l) {
                    if &l != lhs {
                        return self.supertype_of(&l, r);
                    }
                }
                if l.pred.mentions(&l.var) {
                    match l.pred.can_be_false() {
                        Some(true) => {
                            return false;
                        }
                        None => {
                            log!(err "evaluating {}", l.pred);
                        }
                        Some(false) => {}
                    }
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
                    log!(err "instantiation failed: {lhs}");
                    return false;
                };
                self.sub_unify(r, &inst, &(), None).is_ok()
            }
            (l, Quantified(_)) => {
                let Ok(inst) = self.instantiate_dummy(rhs.clone()) else {
                    log!(err "instantiation failed: {rhs}");
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
            // Not(Eq) :> Float == !(Eq :> Float) == true
            (Not(_), Obj) => false,
            (Not(l), rhs) => !self.supertype_of(l, rhs),
            // Ref T :> RefMut T :> T
            (Ref(l), Ref(r)) => self.supertype_of(l, r),
            (Ref(l), RefMut { before: r, .. }) => self.supertype_of(l, r),
            (RefMut { before: l, .. }, RefMut { before: r, .. }) => self.supertype_of(l, r),
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
                if &ln[..] == "List" || &ln[..] == "Set" {
                    let Ok(lt) = self.convert_tp_into_type(lparams[0].clone()) else {
                        return false;
                    };
                    let Ok(rt) = self.convert_tp_into_type(rparams[0].clone()) else {
                        return false;
                    };
                    let llen = lparams[1].clone();
                    let rlen = rparams[1].clone();
                    self.supertype_of(&lt, &rt)
                        && self
                            .try_cmp(&llen, &rlen)
                            .map(|ord| ord.canbe_eq() || ord.canbe_lt())
                            .unwrap_or(false)
                } else {
                    self.poly_supertype_of(lhs, lparams, rparams)
                }
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
                        if r_field.vis != l_field.vis || !self.supertype_of(&l_ty, r_ty) {
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
            Type::NamedTuple(fields) => fields.iter().cloned().collect(),
            Type::Refinement(refine) => self.fields(&refine.t),
            Type::Structural(t) => self.fields(t),
            Type::Or(l, r) => {
                let l_fields = self.fields(l);
                let r_fields = self.fields(r);
                let l_field_names = l_fields.keys().collect::<Set<_>>();
                let r_field_names = r_fields.keys().collect::<Set<_>>();
                let field_names = l_field_names.intersection(&r_field_names);
                let mut fields = Dict::new();
                for (name, l_t, r_t) in field_names
                    .iter()
                    .map(|&name| (name, &l_fields[name], &r_fields[name]))
                {
                    let union = self.union(l_t, r_t);
                    fields.insert(name.clone(), union);
                }
                fields
            }
            other => {
                let Some(ctx) = self.get_nominal_type_ctx(other) else {
                    return Dict::new();
                };
                let mod_fields = if let Some(path) = other.module_path() {
                    self.get_mod_with_path(&path)
                        .map_or(Dict::new(), |ctx| ctx.local_dir())
                } else {
                    Dict::new()
                };
                ctx.type_dir(self)
                    .into_iter()
                    .chain(mod_fields)
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
            "poly_supertype_of: {}\nlps: {}\nrps: {}",
            typ.qual_name(),
            erg_common::fmt_vec(lparams),
            erg_common::fmt_vec(rparams)
        );
        let Some(ctx) = self.get_nominal_type_ctx(typ) else {
            if DEBUG_MODE {
                panic!("{typ} is not found");
            } else {
                return false;
            }
        };
        let variances = ctx.type_params_variance();
        debug_assert_eq!(
            lparams.len(),
            variances.len(),
            "{} / {variances:?}",
            erg_common::fmt_vec(lparams)
        );
        lparams
            .iter()
            .zip(rparams.iter())
            .zip(variances.iter().chain(repeat(&Variance::Invariant)))
            .all(|((lp, rp), variance)| self.supertype_of_tp(lp, rp, *variance))
    }

    fn _subtype_of_tp(&self, lp: &TyParam, rp: &TyParam, variance: Variance) -> bool {
        self.supertype_of_tp(rp, lp, variance)
    }

    fn supertype_of_tp(&self, sup_p: &TyParam, sub_p: &TyParam, variance: Variance) -> bool {
        if sup_p == sub_p {
            return true;
        }
        match (sup_p, sub_p) {
            (TyParam::FreeVar(fv), _) if fv.is_linked() => {
                self.supertype_of_tp(&fv.crack(), sub_p, variance)
            }
            (_, TyParam::FreeVar(fv)) if fv.is_linked() => {
                self.supertype_of_tp(sup_p, &fv.crack(), variance)
            }
            (TyParam::Erased(t), _) => match variance {
                Variance::Contravariant => {
                    let rhs = self.get_tp_t(sub_p).unwrap_or(Obj);
                    self.subtype_of(t, &rhs)
                }
                // REVIEW: invariant type parameters check
                Variance::Covariant | Variance::Invariant => {
                    let rhs = self.get_tp_t(sub_p).unwrap_or(Obj);
                    self.supertype_of(t, &rhs)
                }
            },
            (_, TyParam::Erased(t)) => match variance {
                Variance::Contravariant => self.subtype_of(&self.get_tp_t(sup_p).unwrap_or(Obj), t),
                Variance::Covariant | Variance::Invariant => {
                    self.supertype_of(&self.get_tp_t(sup_p).unwrap_or(Obj), t)
                }
            },
            (TyParam::List(sup), TyParam::List(sub))
            | (TyParam::Tuple(sup), TyParam::Tuple(sub)) => {
                if sup.len() > sub.len() || (variance.is_invariant() && sup.len() != sub.len()) {
                    return false;
                }
                for (sup_p, sub_p) in sup.iter().zip(sub.iter()) {
                    if !self.supertype_of_tp(sup_p, sub_p, variance) {
                        return false;
                    }
                }
                true
            }
            // {Int: Str} :> {Int: Str, Bool: Int}
            (TyParam::Dict(sup_d), TyParam::Dict(sub_d)) => {
                if sup_d.len() > sub_d.len()
                    || (variance.is_invariant() && sup_d.len() != sub_d.len())
                {
                    return false;
                }
                for (sub_k, sub_v) in sub_d.iter() {
                    if let Some(sup_v) = sup_d
                        .linear_get(sub_k)
                        .or_else(|| sub_tpdict_get(sup_d, sub_k, self))
                    {
                        if !self.supertype_of_tp(sup_v, sub_v, variance) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (TyParam::Record(sup_r), TyParam::Record(sub_r)) => {
                if sup_r.len() > sub_r.len()
                    || (variance.is_invariant() && sup_r.len() != sub_r.len())
                {
                    return false;
                }
                for (sub_k, sub_v) in sub_r.iter() {
                    if let Some(sup_v) = sup_r.get(sub_k) {
                        if !self.supertype_of_tp(sup_v, sub_v, variance) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            (TyParam::UnsizedList(sup), TyParam::UnsizedList(sub)) => {
                self.supertype_of_tp(sup, sub, variance)
            }
            (
                TyParam::DataClass { name, fields },
                TyParam::DataClass {
                    name: sub_name,
                    fields: sub_fields,
                },
            ) => {
                if name != sub_name || fields.len() != sub_fields.len() {
                    return false;
                }
                for (sup_tp, sub_tp) in fields.values().zip(sub_fields.values()) {
                    if !self.supertype_of_tp(sup_tp, sub_tp, variance) {
                        return false;
                    }
                }
                true
            }
            (TyParam::Type(sup), TyParam::Type(sub)) => match variance {
                Variance::Contravariant => self.subtype_of(sup, sub),
                Variance::Covariant => self.supertype_of(sup, sub),
                Variance::Invariant => self.same_type_of(sup, sub),
            },
            (
                TyParam::App { name, args },
                TyParam::App {
                    name: sub_name,
                    args: sub_args,
                },
            ) => {
                if name != sub_name || args.len() != sub_args.len() {
                    return false;
                }
                for (sup_p, sub_p) in args.iter().zip(sub_args.iter()) {
                    if !self.supertype_of_tp(sup_p, sub_p, variance) {
                        return false;
                    }
                }
                true
            }
            (TyParam::Lambda(sup_l), TyParam::Lambda(sub_l)) => {
                for (sup_nd, sub_nd) in sup_l.nd_params.iter().zip(sub_l.nd_params.iter()) {
                    if !self.subtype_of(sup_nd.typ(), sub_nd.typ()) {
                        return false;
                    }
                }
                if let Some((sup_var, sub_var)) =
                    sup_l.var_params.as_ref().zip(sub_l.var_params.as_ref())
                {
                    if !self.subtype_of(sup_var.typ(), sub_var.typ()) {
                        return false;
                    }
                }
                for (sup_d, sub_d) in sup_l.d_params.iter().zip(sub_l.d_params.iter()) {
                    if !self.subtype_of(sup_d.typ(), sub_d.typ()) {
                        return false;
                    }
                }
                if let Some((sup_kw_var, sub_kw_var)) = sup_l
                    .kw_var_params
                    .as_ref()
                    .zip(sub_l.kw_var_params.as_ref())
                {
                    if !self.subtype_of(sup_kw_var.typ(), sub_kw_var.typ()) {
                        return false;
                    }
                }
                true
            }
            (TyParam::FreeVar(fv), _) if fv.is_unbound() => {
                let Some(fv_t) = fv.get_type() else {
                    return false;
                };
                let sub_t = match self.get_tp_t(sub_p) {
                    Ok(t) => t,
                    Err(err) => {
                        log!("supertype_of_tp: {err}");
                        Type::Obj
                    }
                };
                if variance == Variance::Contravariant {
                    self.subtype_of(&fv_t, &sub_t)
                } else {
                    // REVIEW: covariant, invariant
                    self.supertype_of(&fv_t, &sub_t)
                }
            }
            (_, TyParam::FreeVar(fv)) if fv.is_unbound() => {
                let Some(fv_t) = fv.get_type() else {
                    return false;
                };
                let sup_t = match self.get_tp_t(sup_p) {
                    Ok(t) => t,
                    Err(err) => {
                        log!("supertype_of_tp: {err}");
                        Type::Obj
                    }
                };
                if variance == Variance::Contravariant {
                    self.subtype_of(&sup_t, &fv_t)
                } else {
                    // REVIEW: covariant, invariant
                    self.supertype_of(&sup_t, &fv_t)
                }
            }
            (TyParam::Value(sup), _) => {
                if let Ok(sup) = Self::convert_value_into_tp(sup.clone()) {
                    self.supertype_of_tp(&sup, sub_p, variance)
                } else {
                    self.eq_tp(sup_p, sub_p)
                }
            }
            (_, TyParam::Value(sub)) => {
                if let Ok(sub) = Self::convert_value_into_tp(sub.clone()) {
                    self.supertype_of_tp(sup_p, &sub, variance)
                } else {
                    self.eq_tp(sup_p, sub_p)
                }
            }
            (TyParam::ProjCall { obj, attr, args }, _) => {
                if let Ok(evaled) =
                    self.eval_proj_call(obj.as_ref().clone(), attr.clone(), args.clone(), &())
                {
                    if sup_p != &evaled {
                        return self.supertype_of_tp(&evaled, sub_p, variance);
                    }
                }
                false
            }
            (_, TyParam::ProjCall { obj, attr, args }) => {
                if let Ok(evaled) =
                    self.eval_proj_call(obj.as_ref().clone(), attr.clone(), args.clone(), &())
                {
                    if sub_p != &evaled {
                        return self.supertype_of_tp(sup_p, &evaled, variance);
                    }
                }
                false
            }
            _ => {
                match (
                    self.convert_tp_into_type(sup_p.clone()),
                    self.convert_tp_into_type(sub_p.clone()),
                ) {
                    (Ok(sup), Ok(sub)) => {
                        return match variance {
                            Variance::Contravariant => self.subtype_of(&sup, &sub),
                            Variance::Covariant => self.supertype_of(&sup, &sub),
                            Variance::Invariant => self.same_type_of(&sup, &sub),
                        };
                    }
                    (Err(le), Err(re)) => {
                        log!(err "cannot convert {le}, {re} to types")
                    }
                    (Err(err), _) | (_, Err(err)) => {
                        log!(err "cannot convert {err} to a type");
                    }
                }
                self.eq_tp(sup_p, sub_p)
            }
        }
    }

    pub(crate) fn covariant_supertype_of_tp(&self, lp: &TyParam, rp: &TyParam) -> bool {
        self.supertype_of_tp(lp, rp, Variance::Covariant)
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
            (TyParam::Type(l), TyParam::Type(r))
            | (TyParam::Erased(l), TyParam::Erased(r)) =>
                self.same_type_of(l, r).then_some(Equal),
            (TyParam::Type(l), TyParam::Value(ValueObj::Type(r))) =>
                self.same_type_of(l, r.typ()).then_some(Equal),
            (TyParam::Value(ValueObj::Type(l)), TyParam::Type(r)) =>
                self.same_type_of(l.typ(), r).then_some(Equal),
            // TODO: 型を見て判断する
            (TyParam::BinOp{ op, lhs, rhs }, r) => {
                if let Ok(evaled) = self.eval_bin_tp(*op, lhs.as_ref().clone(), rhs.as_ref().clone()) {
                    // ?N + 1 == ?N + 1
                    if &evaled == l {
                        Some(Any)
                    } else {
                        self.try_cmp(&evaled, r)
                    }
                } else { Some(Any) }
            },
            (TyParam::UnaryOp { op, val }, r) => {
                if let Ok(evaled) = self.eval_unary_tp(*op, val.as_ref().clone()) {
                    // -?N == -?N
                    if &evaled == l {
                        Some(Any)
                    } else {
                        self.try_cmp(&evaled, r)
                    }
                } else { Some(Any) }
            },
            (l, TyParam::BinOp{ op, lhs, rhs }) => {
                if let Ok(evaled) = self.eval_bin_tp(*op, lhs.as_ref().clone(), rhs.as_ref().clone()) {
                    if &evaled == r {
                        Some(Any)
                    } else {
                        self.try_cmp(l, &evaled)
                    }
                } else { Some(Any) }
            },
            (l, TyParam::UnaryOp { op, val }) => {
                if let Ok(evaled) = self.eval_unary_tp(*op, val.as_ref().clone()) {
                    if &evaled == r {
                        Some(Any)
                    } else {
                        self.try_cmp(l, &evaled)
                    }
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
                let l_t = self.get_tp_t(l).ok()?;
                let r_t = self.get_tp_t(r).ok()?;
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
                let lt = self.get_tp_t(l).ok()?;
                let pt = self.get_tp_t(p).ok()?;
                let l_inf = self.inf(&lt);
                let l_sup = self.sup(&lt);
                if let (Some(inf), Some(sup)) = (l_inf, l_sup) {
                    let (Some(l), Some(r)) = (self.try_cmp(&inf, p), self.try_cmp(&sup, p)) else {
                        log!(err "{inf}, {sup}, {p}");
                        return None;
                    };
                    // (n: Int, 1) -> (-inf..inf, 1) -> (cmp(-inf, 1), cmp(inf, 1)) -> (Less, Greater) -> Any
                    // (n: 5..10, 2) -> (cmp(5..10, 2), cmp(5..10, 2)) -> (Greater, Greater) -> Greater
                    match (l, r) {
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
                        (l, r) => {
                            if DEBUG_MODE {
                                todo!("cmp({inf}, {sup}) = {l:?}, cmp({inf}, {sup}) = {r:?}");
                            }
                            None
                        },
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
            (TyParam::App { name, args }, r) => {
                self.eval_app(name.clone(), args.clone()).ok()
                    .and_then(|tp| self.try_cmp(&tp, r))
            }
            (l, TyParam::App { name, args }) => {
                self.eval_app(name.clone(), args.clone()).ok()
                    .and_then(|tp| self.try_cmp(l, &tp))
            }
            (_l, _r) => {
                erg_common::fmt_dbg!(_l, _r,);
                None
            },
        }
    }

    /// Returns union of two types (`A or B`).
    /// If `A` and `B` have a subtype relationship, it is equal to `max(A, B)`.
    /// ```erg
    /// union(Nat, Int) == Int
    /// union(Int, Str) == Int or Str
    /// union(?T(<: Str), ?U(<: Int)) == ?T or ?U
    /// union(List(Int, 2), List(Str, 2)) == List(Int or Str, 2)
    /// union(List(Int, 2), List(Str, 3)) == List(Int, 2) or List(Int, 3)
    /// union({ .a = Int }, { .a = Str }) == { .a = Int or Str }
    /// union({ .a = Int }, { .a = Int; .b = Int }) == { .a = Int } or { .a = Int; .b = Int } # not to lost `b` information
    /// union((A and B) or C) == (A or C) and (B or C)
    /// ```
    pub(crate) fn union(&self, lhs: &Type, rhs: &Type) -> Type {
        if lhs == rhs {
            return lhs.clone();
        }
        match (lhs, rhs) {
            (FreeVar(fv), other) | (other, FreeVar(fv)) if fv.is_linked() => {
                self.union(&fv.crack(), other)
            }
            (Refinement(l), Refinement(r)) => Type::Refinement(self.union_refinement(l, r)),
            (Refinement(refine), other) | (other, Refinement(refine))
                if other.qual_name() == refine.t.qual_name() =>
            {
                let union = self.union(other, &refine.t);
                if union.is_union_type() {
                    self.simple_union(lhs, rhs)
                } else {
                    union
                }
            }
            (Record(l), Record(r)) if l.len() == r.len() && l.len() == 1 => {
                let mut union = Dict::new();
                for (l_k, l_v) in l.iter() {
                    if let Some((r_k, r_v)) = r.get_key_value(l_k) {
                        let field = match (&l_k.vis, &r_k.vis) {
                            (VisibilityModifier::Private, _) | (_, VisibilityModifier::Private) => {
                                Field::new(VisibilityModifier::Private, l_k.symbol.clone())
                            }
                            // TODO: scope unification
                            (l, r) if l == r => l_k.clone(),
                            _ => continue,
                        };
                        union.insert(field, self.union(l_v, r_v));
                    }
                }
                if union.is_empty() {
                    self.simple_union(lhs, rhs)
                } else {
                    Record(union)
                }
            }
            (Guard(l), Guard(r)) => {
                if l.namespace == r.namespace && l.target == r.target {
                    let to = self.union(&l.to, &r.to);
                    Guard(GuardType::new(l.namespace.clone(), l.target.clone(), to))
                } else {
                    or(lhs.clone(), rhs.clone())
                }
            }
            (Structural(l), Structural(r)) => self.union(l, r).structuralize(),
            // Int..Obj or Nat..Obj ==> Int..Obj
            // Str..Obj or Int..Obj ==> Str..Obj or Int..Obj
            (
                Bounded { sub, sup },
                Bounded {
                    sub: sub2,
                    sup: sup2,
                },
            ) => match (self.max(sub, sub2).either(), self.min(sup, sup2).either()) {
                (Some(sub), Some(sup)) => bounded(sub.clone(), sup.clone()),
                _ => self.simple_union(lhs, rhs),
            },
            (other, or @ Or(_, _)) | (or @ Or(_, _), other) => self.union_add(or, other),
            // (A and B) or C ==> (A or C) and (B or C)
            (and_t @ And(_, _), other) | (other, and_t @ And(_, _)) => {
                let ands = and_t.ands();
                let mut t = Type::Obj;
                for branch in ands.iter() {
                    let union = self.union(branch, other);
                    t = and(t, union);
                }
                t
            }
            (t, Type::Never) | (Type::Never, t) => t.clone(),
            // REVIEW: variance?
            // List({1, 2}, 2), List({3, 4}, 2) ==> List({1, 2, 3, 4}, 2)
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
                    if let Some(union) = self.union_tp(lp, rp) {
                        unified_params.push(union);
                    } else {
                        return self.simple_union(lhs, rhs);
                    }
                }
                poly(ln, unified_params)
            }
            _ => self.simple_union(lhs, rhs),
        }
    }

    /// ```erg
    /// union_tp(1, 1) => Some(1)
    /// union_tp(1, 2) => None
    /// union_tp(?N, 2) => Some(2) # REVIEW:
    /// ```
    pub(crate) fn union_tp(&self, lhs: &TyParam, rhs: &TyParam) -> Option<TyParam> {
        match (lhs, rhs) {
            (TyParam::Value(ValueObj::Type(l)), TyParam::Value(ValueObj::Type(r))) => {
                Some(TyParam::t(self.union(l.typ(), r.typ())))
            }
            (TyParam::Value(ValueObj::Type(l)), TyParam::Type(r)) => {
                Some(TyParam::t(self.union(l.typ(), r)))
            }
            (TyParam::Type(l), TyParam::Value(ValueObj::Type(r))) => {
                Some(TyParam::t(self.union(l, r.typ())))
            }
            (TyParam::Type(l), TyParam::Type(r)) => Some(TyParam::t(self.union(l, r))),
            (TyParam::List(l), TyParam::List(r)) => {
                let mut tps = vec![];
                for (l, r) in l.iter().zip(r.iter()) {
                    if let Some(tp) = self.union_tp(l, r) {
                        tps.push(tp);
                    } else {
                        return None;
                    }
                }
                Some(TyParam::List(tps))
            }
            (fv @ TyParam::FreeVar(f), other) | (other, fv @ TyParam::FreeVar(f))
                if f.is_unbound() =>
            {
                let fv_t = self.get_tp_t(fv).ok()?.derefine();
                let other_t = self.get_tp_t(other).ok()?.derefine();
                if self.same_type_of(&fv_t, &other_t) {
                    Some(other.clone())
                } else {
                    None
                }
            }
            (_, _) => {
                if self.eq_tp(lhs, rhs) {
                    Some(lhs.clone())
                } else {
                    None
                }
            }
        }
    }

    /// ```erg
    /// union_add(Int or ?T(:> NoneType), Nat) == Int or ?T
    /// union_add(Nat or ?T(:> NoneType), Int) == Int or ?T
    /// union_add(Int or ?T(:> NoneType), Str) == Int or ?T or Str
    /// ```
    fn union_add(&self, union: &Type, elem: &Type) -> Type {
        let ors = union.ors();
        let bounded = ors.iter().map(|t| t.lower_bounded());
        for t in bounded {
            if self.supertype_of(&t, elem) {
                return union.clone();
            } /* else if ors.contains(&t) && self.subtype_of(&t, elem) {
                return constructors::ors(ors.exclude(&t).include(elem.clone()));
              } */
        }
        or(union.clone(), elem.clone())
    }

    /// ```erg
    /// simple_union(?T, ?U) == ?T or ?U
    /// union(Set!(?T(<: Int), 3), Set(?U(<: Nat), 3)) == Set(?T, 3)
    /// simple_union(?T(<: Int), Int) == Int or ?T
    /// simple_union(?T(:> Int), Int) == ?T
    /// ```
    fn simple_union(&self, lhs: &Type, rhs: &Type) -> Type {
        if let Ok(free) = <&FreeTyVar>::try_from(lhs) {
            free.dummy_link();
            let res = if !rhs.is_totally_unbound()
                && self.supertype_of(&free.get_sub().unwrap_or(Never), rhs)
            {
                lhs.clone()
            } else {
                or(lhs.clone(), rhs.clone())
            };
            free.undo();
            res
        } else if let Ok(free) = <&FreeTyVar>::try_from(rhs) {
            free.dummy_link();
            let res = if !lhs.is_totally_unbound()
                && self.supertype_of(&free.get_sub().unwrap_or(Never), lhs)
            {
                rhs.clone()
            } else {
                or(lhs.clone(), rhs.clone())
            };
            free.undo();
            res
        } else {
            if lhs.is_totally_unbound() || rhs.is_totally_unbound() {
                return or(lhs.clone(), rhs.clone());
            }
            match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
                (true, true) => lhs.clone(),  // lhs = rhs
                (true, false) => lhs.clone(), // lhs :> rhs
                (false, true) => rhs.clone(),
                (false, false) => or(lhs.clone(), rhs.clone()),
            }
        }
    }

    pub(crate) fn union_pred(&self, lhs: Predicate, rhs: Predicate) -> Predicate {
        match (
            self.is_super_pred_of(&lhs, &rhs),
            self.is_sub_pred_of(&lhs, &rhs),
        ) {
            (true, true) => lhs,  // lhs = rhs
            (true, false) => lhs, // lhs :> rhs
            (false, true) => rhs,
            (false, false) => lhs | rhs,
        }
    }

    pub(crate) fn intersection_pred(&self, lhs: Predicate, rhs: Predicate) -> Predicate {
        match (
            self.is_super_pred_of(&lhs, &rhs),
            self.is_sub_pred_of(&lhs, &rhs),
        ) {
            (true, true) => lhs,
            (true, false) => rhs,
            (false, true) => lhs,
            (false, false) => lhs & rhs,
        }
    }

    pub(crate) fn union_refinement(
        &self,
        lhs: &RefinementType,
        rhs: &RefinementType,
    ) -> RefinementType {
        // TODO: warn if lhs.t !:> rhs.t && rhs.t !:> lhs.t
        let union = self.union(&lhs.t, &rhs.t);
        let name = lhs.var.clone();
        let rhs_pred = rhs.pred.clone().change_subject_name(name);
        let union_pred = self.union_pred(*lhs.pred.clone(), rhs_pred);
        RefinementType::new(lhs.var.clone(), union, union_pred)
    }

    /// Returns intersection of two types (`A and B`).
    /// If `A` and `B` have a subtype relationship, it is equal to `min(A, B)`.
    pub(crate) fn intersection(&self, lhs: &Type, rhs: &Type) -> Type {
        if lhs == rhs {
            return lhs.clone();
        }
        match (lhs, rhs) {
            (FreeVar(fv), other) | (other, FreeVar(fv)) if fv.is_linked() => {
                self.intersection(&fv.crack(), other)
            }
            (Refinement(l), Refinement(r)) => Type::Refinement(self.intersection_refinement(l, r)),
            (Structural(l), Structural(r)) => self.intersection(l, r).structuralize(),
            (Guard(l), Guard(r)) => {
                if l.namespace == r.namespace && l.target == r.target {
                    let to = self.intersection(&l.to, &r.to);
                    Guard(GuardType::new(l.namespace.clone(), l.target.clone(), to))
                } else {
                    and(lhs.clone(), rhs.clone())
                }
            }
            // {.i = Int} and {.s = Str} == {.i = Int; .s = Str}
            (Record(l), Record(r)) => Type::Record(l.clone().concat(r.clone())),
            // {i = Int; j = Int} and not {i = Int} == {j = Int}
            // not {i = Int} and {i = Int; j = Int} == {j = Int}
            (other @ Record(rec), Not(t)) | (Not(t), other @ Record(rec)) => match t.as_ref() {
                Type::FreeVar(fv) => self.intersection(&fv.crack(), other),
                Type::Record(rec2) => Type::Record(rec.clone().diff(rec2)),
                _ => Type::Never,
            },
            (_, Not(r)) => self.diff(lhs, r),
            (Not(l), _) => self.diff(rhs, l),
            // A and B and A == A and B
            (other, and @ And(_, _)) | (and @ And(_, _), other) => {
                self.intersection_add(and, other)
            }
            // (A or B) and C == (A and C) or (B and C)
            (or_t @ Or(_, _), other) | (other, or_t @ Or(_, _)) => {
                let ors = or_t.ors();
                if ors.iter().any(|t| t.has_unbound_var()) {
                    return self.simple_intersection(lhs, rhs);
                }
                let mut t = Type::Never;
                for branch in ors.iter() {
                    let isec = self.intersection(branch, other);
                    t = self.union(&t, &isec);
                }
                t
            }
            // REVIEW: variance?
            // Array({1, 2, 3}) and Array({2, 3, 4}) == Array({2, 3})
            (
                Poly {
                    name: ln,
                    params: lps,
                },
                Poly {
                    name: rn,
                    params: rps,
                },
            ) if ln == rn && self.is_class(lhs) => {
                debug_assert_eq!(lps.len(), rps.len());
                let mut new_params = vec![];
                for (lp, rp) in lps.iter().zip(rps.iter()) {
                    if let Some(intersec) = self.intersection_tp(lp, rp) {
                        new_params.push(intersec);
                    } else {
                        return self.simple_intersection(lhs, rhs);
                    }
                }
                poly(ln.clone(), new_params)
            }
            (other, Refinement(refine)) | (Refinement(refine), other) => {
                let other = other.clone().into_refinement();
                let intersec = self.intersection_refinement(&other, refine);
                self.try_squash_refinement(intersec)
                    .unwrap_or_else(Type::Refinement)
            }
            // overloading
            (l, r) if l.is_subr() && r.is_subr() => and(lhs.clone(), rhs.clone()),
            _ => self.simple_intersection(lhs, rhs),
        }
    }

    pub(crate) fn intersection_tp(&self, lhs: &TyParam, rhs: &TyParam) -> Option<TyParam> {
        match (lhs, rhs) {
            (TyParam::Value(ValueObj::Type(l)), TyParam::Value(ValueObj::Type(r))) => {
                Some(TyParam::t(self.intersection(l.typ(), r.typ())))
            }
            (TyParam::Value(ValueObj::Type(l)), TyParam::Type(r)) => {
                Some(TyParam::t(self.intersection(l.typ(), r)))
            }
            (TyParam::Type(l), TyParam::Value(ValueObj::Type(r))) => {
                Some(TyParam::t(self.intersection(l, r.typ())))
            }
            (TyParam::Type(l), TyParam::Type(r)) => Some(TyParam::t(self.intersection(l, r))),
            (TyParam::List(l), TyParam::List(r)) => {
                let mut tps = vec![];
                for (l, r) in l.iter().zip(r.iter()) {
                    if let Some(tp) = self.intersection_tp(l, r) {
                        tps.push(tp);
                    } else {
                        return None;
                    }
                }
                Some(TyParam::List(tps))
            }
            (fv @ TyParam::FreeVar(f), other) | (other, fv @ TyParam::FreeVar(f))
                if f.is_unbound() =>
            {
                let fv_t = self.get_tp_t(fv).ok()?.derefine();
                let other_t = self.get_tp_t(other).ok()?.derefine();
                if self.same_type_of(&fv_t, &other_t) {
                    Some(other.clone())
                } else {
                    None
                }
            }
            (_, _) => {
                if self.eq_tp(lhs, rhs) {
                    Some(lhs.clone())
                } else {
                    None
                }
            }
        }
    }

    /// ```erg
    /// intersection_add(Nat and ?T(:> NoneType), Int) == Nat and ?T
    /// intersection_add(Int and ?T(:> NoneType), Nat) == Nat and ?T
    /// intersection_add(Int and ?T(:> NoneType), Str) == Never
    /// ```
    fn intersection_add(&self, intersection: &Type, elem: &Type) -> Type {
        let ands = intersection.ands();
        let bounded = ands.iter().map(|t| t.lower_bounded());
        for t in bounded {
            if self.subtype_of(&t, elem) {
                return intersection.clone();
            } else if self.supertype_of(&t, elem) {
                return constructors::ands(ands.linear_exclude(&t).include(elem.clone()));
            }
        }
        and(intersection.clone(), elem.clone())
    }

    fn simple_intersection(&self, lhs: &Type, rhs: &Type) -> Type {
        // ?T and ?U will not be unified
        if lhs.is_unbound_var() || rhs.is_unbound_var() {
            and(lhs.clone(), rhs.clone())
        } else {
            match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
                (true, true) => lhs.clone(),  // lhs = rhs
                (true, false) => rhs.clone(), // lhs :> rhs
                (false, true) => lhs.clone(),
                (false, false) => {
                    if self.is_trait(lhs) && self.is_trait(rhs) {
                        and(lhs.clone(), rhs.clone())
                    } else {
                        Type::Never
                    }
                }
            }
        }
    }

    /// ```erg
    /// narrow_type_by_pred(Int or NoneType, x != None) == Int
    /// ```
    #[allow(clippy::only_used_in_recursion)]
    fn narrow_type_by_pred(&self, t: Type, pred: &Predicate) -> Type {
        match (t, pred) {
            (
                Type::Or(l, r),
                Predicate::NotEqual {
                    rhs: TyParam::Value(v),
                    ..
                },
            ) if v.is_none() => {
                let l = self.narrow_type_by_pred(*l, pred);
                let r = self.narrow_type_by_pred(*r, pred);
                if l.is_nonetype() {
                    r
                } else if r.is_nonetype() {
                    l
                } else {
                    or(l, r)
                }
            }
            (Type::Refinement(refine), _) => {
                let t = self.narrow_type_by_pred(*refine.t, pred);
                refinement(refine.var.clone(), t, *refine.pred)
            }
            (other, Predicate::And(l, r)) => {
                let other = self.narrow_type_by_pred(other, l);
                self.narrow_type_by_pred(other, r)
            }
            // TODO:
            (other, _) => other,
        }
    }

    /// ```erg
    /// {I: Int | I > 0} and {I: Int | I < 10} == {I: Int | I > 0 and I < 10}
    /// {x: Int or NoneType | True} and {x: Obj | x != None} == {x: Int or NoneType | x != None} (== Int)
    /// {x: Nat or None | x == 1 or x == None} and {x: Int | True} == {x: Int | x == 1}
    /// ```
    fn intersection_refinement(
        &self,
        lhs: &RefinementType,
        rhs: &RefinementType,
    ) -> RefinementType {
        let intersec = self.intersection(&lhs.t, &rhs.t);
        let name = lhs.var.clone();
        let rhs_pred = rhs.pred.clone().change_subject_name(name);
        let intersection_pred = self.intersection_pred(*lhs.pred.clone(), rhs_pred);
        let intersec = self.narrow_type_by_pred(intersec, &intersection_pred);
        let Some(pred) =
            self.eliminate_type_mismatched_preds(&lhs.var, &intersec, intersection_pred)
        else {
            return RefinementType::new(lhs.var.clone(), intersec, Predicate::TRUE);
        };
        RefinementType::new(lhs.var.clone(), intersec, pred)
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
        let unions = refine.t.ors();
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
    pub(crate) fn type_from_pred(&self, pred: Predicate) -> RefinementType {
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
            Predicate::Call { receiver, name, .. } => {
                let receiver_t = self.get_tp_t(receiver).unwrap_or(Obj);
                if let Some(name) = name {
                    let Some(ctx) = self.get_nominal_type_ctx(&receiver_t) else {
                        return Obj;
                    };
                    if let Some((_, method)) = ctx.get_var_info(name) {
                        method.t.return_t().cloned().unwrap_or(Obj)
                    } else {
                        Obj
                    }
                } else {
                    receiver_t.return_t().cloned().unwrap_or(Obj)
                }
            }
            Predicate::Attr { receiver, name } => {
                let receiver_t = self.get_tp_t(receiver).unwrap_or(Obj);
                let Some(ctx) = self.get_nominal_type_ctx(&receiver_t) else {
                    return Obj;
                };
                if let Some((_, field)) = ctx.get_var_info(name) {
                    field.t.clone()
                } else {
                    Obj
                }
            }
            // REVIEW
            Predicate::GeneralEqual { rhs, .. }
            | Predicate::GeneralGreaterEqual { rhs, .. }
            | Predicate::GeneralLessEqual { rhs, .. }
            | Predicate::GeneralNotEqual { rhs, .. } => self.get_pred_type(rhs),
            // x == 1 or x == "a" => Int or Str
            Predicate::Or(lhs, rhs) => {
                self.union(&self.get_pred_type(lhs), &self.get_pred_type(rhs))
            }
            // REVIEW:
            Predicate::And(lhs, rhs) => {
                self.intersection(&self.get_pred_type(lhs), &self.get_pred_type(rhs))
            }
            Predicate::Const(name) => {
                if let Some(value) = self.rec_get_const_obj(name) {
                    value.class()
                } else if DEBUG_MODE {
                    todo!("get_pred_type({name})");
                } else {
                    Obj
                }
            }
            Predicate::Failure => Type::Failure,
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
                guard.namespace.clone(),
                guard.target.clone(),
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

    // {I >= 0} :> {I >= 1}
    // mode == "and", reduced: {I >= 0}, pred: {I >= 1}
    // => reduced: {I >= 1} ((I >= 0 and I >= 1) == I >= 1)
    // mode == "and", reduced: {I >= 1}, pred: {I >= 0}
    // => reduced: {I >= 1}
    // mode == "or", reduced: {I >= 0}, pred: {I >= 1}
    // => reduced: {I >= 0} ((I >= 0 or I >= 1) == I >= 0)
    fn reduce_preds<'s>(&self, mode: &str, preds: Set<&'s Predicate>) -> Set<&'s Predicate> {
        let mut reduced = Set::<&'s Predicate>::new();
        for pred in preds {
            // remove unnecessary pred
            // TODO: remove all unnecessary preds
            if let Some(&old) = reduced.iter().find(|existing| match mode {
                "and" => self.is_super_pred_of(existing, pred),
                "or" => self.is_sub_pred_of(existing, pred),
                _ => unreachable!(),
            }) {
                reduced.linear_remove(old);
            }
            // insert if necessary
            if reduced.iter().all(|existing| match mode {
                "and" => !self.is_sub_pred_of(existing, pred),
                "or" => !self.is_super_pred_of(existing, pred),
                _ => unreachable!(),
            }) {
                reduced.insert(pred);
            }
        }
        reduced
    }

    fn eliminate_type_mismatched_preds(
        &self,
        var: &str,
        t: &Type,
        pred: Predicate,
    ) -> Option<Predicate> {
        match pred {
            Predicate::Equal { ref lhs, ref rhs }
            | Predicate::NotEqual { ref lhs, ref rhs }
            | Predicate::GreaterEqual { ref lhs, ref rhs }
            | Predicate::LessEqual { ref lhs, ref rhs }
                if lhs == var =>
            {
                let rhs_t = self.get_tp_t(rhs).unwrap_or(Obj);
                if !self.subtype_of(&rhs_t, t) {
                    None
                } else {
                    Some(pred)
                }
            }
            Predicate::And(l, r) => {
                let l = self.eliminate_type_mismatched_preds(var, t, *l);
                let r = self.eliminate_type_mismatched_preds(var, t, *r);
                match (l, r) {
                    (Some(l), Some(r)) => Some(l & r),
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    (None, None) => None,
                }
            }
            Predicate::Or(l, r) => {
                let l = self.eliminate_type_mismatched_preds(var, t, *l);
                let r = self.eliminate_type_mismatched_preds(var, t, *r);
                match (l, r) {
                    (Some(l), Some(r)) => Some(l | r),
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    (None, None) => None,
                }
            }
            _ => Some(pred),
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
            (
                Pred::Equal { .. },
                Pred::GreaterEqual { .. } | Pred::LessEqual { .. } | Pred::NotEqual { .. },
            )
            | (Pred::LessEqual { .. }, Pred::GreaterEqual { .. })
            | (Pred::GreaterEqual { .. }, Pred::LessEqual { .. }) => false,
            // I != 1 == All - {1} :> I == 2
            (Pred::NotEqual { rhs: rhs_ne, .. }, Pred::Equal { rhs: rhs_eq, .. }) => {
                rhs_ne != rhs_eq
            }
            // I != 1 == All - {1} :> I >= 2
            (Pred::NotEqual { rhs: rhs_ne, .. }, Pred::GreaterEqual { rhs: rhs_ge, .. }) => {
                self.try_cmp(rhs_ne, rhs_ge).is_some_and(|ord| ord.is_lt())
            }
            // I != 1 == All - {1} :> I <= 0
            (Pred::NotEqual { rhs: rhs_ne, .. }, Pred::LessEqual { rhs: rhs_le, .. }) => {
                self.try_cmp(rhs_ne, rhs_le).is_some_and(|ord| ord.is_gt())
            }
            (Pred::NotEqual { rhs, .. }, Pred::NotEqual { rhs: rhs2, .. }) => self
                .try_cmp(rhs, rhs2)
                .map(|ord| ord.canbe_eq())
                .unwrap_or(false),
            (Pred::Equal { rhs, .. }, Pred::Equal { rhs: rhs2, .. }) => {
                self.supertype_of_tp(rhs, rhs2, Variance::Covariant)
                    || self
                        .try_cmp(rhs, rhs2)
                        .map(|ord| ord.canbe_eq())
                        .unwrap_or(false)
            }
            (
                Pred::GeneralEqual { lhs, rhs },
                Pred::GeneralEqual {
                    lhs: lhs2,
                    rhs: rhs2,
                },
            )
            | (
                Pred::GeneralGreaterEqual { lhs, rhs },
                Pred::GeneralGreaterEqual {
                    lhs: lhs2,
                    rhs: rhs2,
                },
            )
            | (
                Pred::GeneralLessEqual { lhs, rhs },
                Pred::GeneralLessEqual {
                    lhs: lhs2,
                    rhs: rhs2,
                },
            )
            | (
                Pred::GeneralNotEqual { lhs, rhs },
                Pred::GeneralNotEqual {
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) => self.is_super_pred_of(lhs, lhs2) && self.is_super_pred_of(rhs, rhs2),
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
            // NG: (self.is_super_pred_of(l1, l2) && self.is_super_pred_of(r1, r2))
            //     || (self.is_super_pred_of(l1, r2) && self.is_super_pred_of(r1, l2))
            (Pred::And(_, _), Pred::And(_, _)) => {
                let lhs_ands = self.reduce_preds("and", lhs.ands());
                let rhs_ands = self.reduce_preds("and", rhs.ands());
                for r_val in rhs_ands.iter() {
                    if lhs_ands
                        .get_by(r_val, |l, r| self.is_super_pred_of(l, r))
                        .is_none()
                    {
                        return false;
                    }
                }
                true
            }
            // {I == 0 or I == 1 or I == 2} :> {I == 1 or I == 0}
            // {I == 1 or I == 0} !:> {I == 0 or I == 1 or I == 3}
            // NG: (self.is_super_pred_of(l1, l2) && self.is_super_pred_of(r1, r2))
            //     || (self.is_super_pred_of(l1, r2) && self.is_super_pred_of(r1, l2))
            (Pred::Or(_, _), Pred::Or(_, _)) => {
                let lhs_ors = self.reduce_preds("or", lhs.ors());
                let rhs_ors = self.reduce_preds("or", rhs.ors());
                for r_val in rhs_ors.iter() {
                    if lhs_ors
                        .get_by(r_val, |l, r| self.is_super_pred_of(l, r))
                        .is_none()
                    {
                        return false;
                    }
                }
                true
            }
            (
                Pred::Call {
                    receiver,
                    name,
                    args,
                },
                Pred::Call {
                    receiver: sub_receiver,
                    name: name2,
                    args: args2,
                },
            ) => {
                name == name2
                    && args.len() == args2.len()
                    && self.supertype_of_tp(receiver, sub_receiver, Variance::Covariant)
                    && args
                        .iter()
                        .zip(args2.iter())
                        .all(|(l, r)| self.supertype_of_tp(l, r, Variance::Covariant))
            }
            (pred @ Predicate::Call { .. }, Predicate::Value(ValueObj::Bool(b))) => {
                if let Ok(Predicate::Value(ValueObj::Bool(evaled))) = self.eval_pred(pred.clone()) {
                    b == &evaled
                } else {
                    false
                }
            }
            (Pred::Value(ValueObj::Bool(b)), _) => *b,
            (_, Pred::Value(ValueObj::Bool(b))) => !b,
            (Pred::LessEqual { rhs, .. }, _) if !rhs.has_upper_bound() => true,
            (Pred::GreaterEqual { rhs, .. }, _) if !rhs.has_lower_bound() => true,
            (lhs, Pred::And(l, r)) => {
                self.is_super_pred_of(lhs, l) || self.is_super_pred_of(lhs, r)
            }
            (lhs, Pred::Or(l, r)) => self.is_super_pred_of(lhs, l) && self.is_super_pred_of(lhs, r),
            (Pred::Or(l, r), rhs) => self.is_super_pred_of(l, rhs) || self.is_super_pred_of(r, rhs),
            (Pred::And(l, r), rhs) => {
                self.is_super_pred_of(l, rhs) && self.is_super_pred_of(r, rhs)
            }
            (lhs, rhs) => {
                if DEBUG_MODE {
                    log!("{lhs}/{rhs}");
                }
                false
            }
        }
    }

    fn is_sub_pred_of(&self, lhs: &Predicate, rhs: &Predicate) -> bool {
        self.is_super_pred_of(rhs, lhs)
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
    pub(crate) fn sup(&self, t: &Type) -> Option<TyParam> {
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

    pub(crate) fn inf(&self, t: &Type) -> Option<TyParam> {
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
    pub(crate) fn min<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> Triple<&'t Type, &'t Type> {
        // If they are the same, either one can be returned.
        match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
            (true, true) | (true, false) => Triple::Err(rhs),
            (false, true) => Triple::Ok(lhs),
            (false, false) => Triple::None,
        }
    }

    pub(crate) fn max<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> Triple<&'t Type, &'t Type> {
        // If they are the same, either one can be returned.
        match (self.supertype_of(lhs, rhs), self.subtype_of(lhs, rhs)) {
            (true, true) | (true, false) => Triple::Ok(lhs),
            (false, true) => Triple::Err(rhs),
            (false, false) => Triple::None,
        }
    }

    pub(crate) fn cmp_t<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> TyParamOrdering {
        match self.min(lhs, rhs) {
            Triple::Ok(_) => TyParamOrdering::Less,
            Triple::Err(_) => TyParamOrdering::Greater,
            Triple::None => TyParamOrdering::NoRelation,
        }
    }

    /// {[]} == {A: List(Never, _) | A == []} => List(Never, 0)
    pub(crate) fn refinement_to_poly(&self, refine: &RefinementType) -> Option<Type> {
        if refine.t.is_monomorphic() {
            return None;
        }
        let Predicate::Equal { lhs, rhs } = refine.pred.as_ref() else {
            return None;
        };
        if &refine.var != lhs {
            return None;
        }
        match &refine.t.qual_name()[..] {
            "List" => self.get_tp_t(rhs).ok().map(|t| t.derefine()),
            _ => None,
        }
    }
}
