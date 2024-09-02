//! provides type variable related operations
use std::iter::repeat;
use std::mem;
use std::option::Option;
use std::sync::atomic::{AtomicUsize, Ordering};

use erg_common::consts::DEBUG_MODE;
use erg_common::fresh::FRESH_GEN;
use erg_common::traits::Locational;
use erg_common::Str;
#[allow(unused_imports)]
use erg_common::{fmt_vec, fn_name, log};

use crate::context::eval::Substituter;
use crate::ty::constructors::*;
use crate::ty::free::{Constraint, FreeKind, HasLevel, GENERIC_LEVEL};
use crate::ty::typaram::{OpKind, TyParam};
use crate::ty::value::ValueObj;
use crate::ty::{Predicate, SubrType, Type};

use crate::context::{Context, Variance};
use crate::error::{TyCheckError, TyCheckErrors, TyCheckResult};
use crate::type_feature_error;

use Predicate as Pred;
use Type::*;
use ValueObj::{Inf, NegInf};

use super::eval::UndoableLinkedList;
use super::initialize::const_func::sub_tpdict_get;

pub struct Unifier<'c, 'l, 'u, L: Locational> {
    ctx: &'c Context,
    loc: &'l L,
    undoable: Option<&'u UndoableLinkedList>,
    change_generalized: bool,
    recursion_limit: AtomicUsize,
    param_name: Option<Str>,
}

impl<'c, 'l, 'u, L: Locational> Unifier<'c, 'l, 'u, L> {
    pub fn new(
        ctx: &'c Context,
        loc: &'l L,
        undoable: Option<&'u UndoableLinkedList>,
        change_generalized: bool,
        param_name: Option<Str>,
    ) -> Self {
        Self {
            ctx,
            loc,
            undoable,
            change_generalized,
            recursion_limit: AtomicUsize::new(128),
            param_name,
        }
    }
}

impl<'c, 'l, 'u, L: Locational> Unifier<'c, 'l, 'u, L> {
    /// ```erg
    /// occur(?T, ?T) ==> OK
    /// occur(?T(<: ?U), ?U) ==> OK
    /// occur(?T, ?U(:> ?T)) ==> OK
    /// occur(X -> ?T, ?T) ==> Error
    /// occur(X -> ?T, X -> ?T) ==> OK
    /// occur(?T, ?T -> X) ==> Error
    /// occur(?T, Option(?T)) ==> Error
    /// occur(?T or Int, Int or ?T) ==> OK
    /// occur(?T(<: Str) or ?U(<: Int), ?T(<: Str)) ==> Error
    /// occur(?T(<: ?U or Y), ?U) ==> OK
    /// occur(?T, ?T.Output) ==> OK
    /// ```
    fn occur(&self, maybe_sub: &Type, maybe_sup: &Type) -> TyCheckResult<()> {
        if maybe_sub == maybe_sup {
            return Ok(());
        } else if let Some(sup) = maybe_sub.get_super() {
            if &sup == maybe_sup {
                return Ok(());
            }
        } else if let Some(sub) = maybe_sup.get_sub() {
            if &sub == maybe_sub {
                return Ok(());
            }
        }
        match (maybe_sub, maybe_sup) {
            (FreeVar(fv), _) if fv.is_linked() => self.occur(fv.unsafe_crack(), maybe_sup),
            (_, FreeVar(fv)) if fv.is_linked() => self.occur(maybe_sub, fv.unsafe_crack()),
            (Subr(subr), FreeVar(fv)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(default_t, maybe_sup)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(var_params.typ(), maybe_sup)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(non_default_t, maybe_sup)?;
                }
                self.occur_inner(&subr.return_t, maybe_sup)?;
                Ok(())
            }
            (FreeVar(fv), Subr(subr)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(maybe_sub, default_t)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(maybe_sub, var_params.typ())?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(maybe_sub, non_default_t)?;
                }
                self.occur_inner(maybe_sub, &subr.return_t)?;
                Ok(())
            }
            (Subr(lhs), Subr(rhs)) => {
                for (lhs, rhs) in lhs
                    .default_params
                    .iter()
                    .map(|pt| pt.typ())
                    .zip(rhs.default_params.iter().map(|pt| pt.typ()))
                {
                    self.occur(lhs, rhs)?;
                }
                if let Some(lhs) = lhs.var_params.as_ref() {
                    if let Some(rhs) = rhs.var_params.as_ref() {
                        self.occur(lhs.typ(), rhs.typ())?;
                    }
                }
                for (lhs, rhs) in lhs
                    .non_default_params
                    .iter()
                    .map(|pt| pt.typ())
                    .zip(rhs.non_default_params.iter().map(|pt| pt.typ()))
                {
                    self.occur(lhs, rhs)?;
                }
                self.occur(&lhs.return_t, &rhs.return_t)?;
                Ok(())
            }
            /*(Poly { params, .. }, FreeVar(fv)) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| {
                    if let TyParam::Type(t) = tp {
                        Some(t)
                    } else {
                        None
                    }
                }) {
                    self.occur_inner(param, maybe_sup)?;
                }
                Ok(())
            }*/
            (FreeVar(fv), Poly { params, .. }) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| <&Type>::try_from(tp).ok()) {
                    self.occur_inner(maybe_sub, param)?;
                }
                Ok(())
            }
            (Or(l, r), Or(l2, r2)) | (And(l, r), And(l2, r2)) => self
                .occur(l, l2)
                .and(self.occur(r, r2))
                .or(self.occur(l, r2).and(self.occur(r, l2))),
            (lhs, Or(l, r)) | (lhs, And(l, r)) => {
                self.occur_inner(lhs, l)?;
                self.occur_inner(lhs, r)
            }
            (Or(l, r), rhs) | (And(l, r), rhs) => {
                self.occur_inner(l, rhs)?;
                self.occur_inner(r, rhs)
            }
            _ => Ok(()),
        }
    }

    fn occur_inner(&self, maybe_sub: &Type, maybe_sup: &Type) -> TyCheckResult<()> {
        match (maybe_sub, maybe_sup) {
            (FreeVar(fv), _) if fv.is_linked() => self.occur_inner(fv.unsafe_crack(), maybe_sup),
            (_, FreeVar(fv)) if fv.is_linked() => self.occur_inner(maybe_sub, fv.unsafe_crack()),
            (FreeVar(sub), FreeVar(sup)) => {
                if sub.addr_eq(sup) {
                    Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )))
                } else {
                    if let Some((sub_t, _sup_t)) = sub.get_subsup() {
                        sub.do_avoiding_recursion(|| {
                            // occur(?T(<: ?U or Y), ?U) ==> OK
                            self.occur_inner(&sub_t, maybe_sup)
                            // self.occur_inner(&sup_t, maybe_sup)
                        })?;
                    }
                    if let Some((sub_t, sup_t)) = sup.get_subsup() {
                        sup.do_avoiding_recursion(|| {
                            // occur(?U, ?T(:> ?U or Y)) ==> OK
                            self.occur_inner(maybe_sub, &sub_t)?;
                            self.occur_inner(maybe_sub, &sup_t)
                        })?;
                    }
                    Ok(())
                }
            }
            (Subr(subr), FreeVar(fv)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(default_t, maybe_sup)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(var_params.typ(), maybe_sup)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(non_default_t, maybe_sup)?;
                }
                self.occur_inner(&subr.return_t, maybe_sup)?;
                Ok(())
            }
            (FreeVar(fv), Subr(subr)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(maybe_sub, default_t)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(maybe_sub, var_params.typ())?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(maybe_sub, non_default_t)?;
                }
                self.occur_inner(maybe_sub, &subr.return_t)?;
                Ok(())
            }
            (Subr(lhs), Subr(rhs)) => {
                for (lhs, rhs) in lhs
                    .default_params
                    .iter()
                    .map(|pt| pt.typ())
                    .zip(rhs.default_params.iter().map(|pt| pt.typ()))
                {
                    self.occur_inner(lhs, rhs)?;
                }
                if let Some(lhs) = lhs.var_params.as_ref() {
                    if let Some(rhs) = rhs.var_params.as_ref() {
                        self.occur_inner(lhs.typ(), rhs.typ())?;
                    }
                }
                for (lhs, rhs) in lhs
                    .non_default_params
                    .iter()
                    .map(|pt| pt.typ())
                    .zip(rhs.non_default_params.iter().map(|pt| pt.typ()))
                {
                    self.occur_inner(lhs, rhs)?;
                }
                self.occur_inner(&lhs.return_t, &rhs.return_t)?;
                Ok(())
            }
            (Poly { params, .. }, FreeVar(fv)) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| <&Type>::try_from(tp).ok()) {
                    self.occur_inner(param, maybe_sup)?;
                }
                Ok(())
            }
            (FreeVar(fv), Poly { params, .. }) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| <&Type>::try_from(tp).ok()) {
                    self.occur_inner(maybe_sub, param)?;
                }
                Ok(())
            }
            (lhs, Or(l, r)) | (lhs, And(l, r)) => {
                self.occur_inner(lhs, l)?;
                self.occur_inner(lhs, r)
            }
            (Or(l, r), rhs) | (And(l, r), rhs) => {
                self.occur_inner(l, rhs)?;
                self.occur_inner(r, rhs)
            }
            _ => Ok(()),
        }
    }

    fn sub_unify_value(&self, maybe_sub: &ValueObj, maybe_sup: &ValueObj) -> TyCheckResult<()> {
        match (maybe_sub, maybe_sup) {
            (ValueObj::Type(sub), ValueObj::Type(sup)) => self.sub_unify(sub.typ(), sup.typ()),
            (ValueObj::UnsizedList(sub), ValueObj::UnsizedList(sup)) => {
                self.sub_unify_value(sub, sup)
            }
            (ValueObj::List(sub), ValueObj::List(sup))
            | (ValueObj::Tuple(sub), ValueObj::Tuple(sup)) => {
                for (l, r) in sub.iter().zip(sup.iter()) {
                    self.sub_unify_value(l, r)?;
                }
                Ok(())
            }
            (ValueObj::Dict(sub), ValueObj::Dict(sup)) => {
                if sub.len() == 1 && sup.len() == 1 {
                    let sub_key = sub.keys().next().unwrap();
                    let sup_key = sup.keys().next().unwrap();
                    self.sub_unify_value(sub_key, sup_key)?;
                    let sub_value = sub.values().next().unwrap();
                    let sup_value = sup.values().next().unwrap();
                    self.sub_unify_value(sub_value, sup_value)?;
                    return Ok(());
                }
                for (sub_k, sub_v) in sub.iter() {
                    if let Some(sup_v) = sup.get(sub_k) {
                        self.sub_unify_value(sub_v, sup_v)?;
                    } else {
                        log!(err "{sup} does not have key {sub_k}");
                        return Err(TyCheckErrors::from(TyCheckError::feature_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            self.loc.loc(),
                            &format!("unifying {sub} and {sup}"),
                            self.ctx.caused_by(),
                        )));
                    }
                }
                Ok(())
            }
            (ValueObj::Set(sub), ValueObj::Set(sup)) => {
                if sub.len() == 1 && sup.len() == 1 {
                    let sub = sub.iter().next().unwrap();
                    let sup = sup.iter().next().unwrap();
                    self.sub_unify_value(sub, sup)?;
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::feature_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        self.loc.loc(),
                        &format!("unifying {sub} and {sup}"),
                        self.ctx.caused_by(),
                    )))
                }
            }
            (ValueObj::Record(sub), ValueObj::Record(sup)) => {
                for (sub_k, sub_v) in sub.iter() {
                    if let Some(sup_v) = sup.get(sub_k) {
                        self.sub_unify_value(sub_v, sup_v)?;
                    } else {
                        log!(err "{sup} does not have field {sub_k}");
                        return Err(TyCheckErrors::from(TyCheckError::feature_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            self.loc.loc(),
                            &format!("unifying {sub} and {sup}"),
                            self.ctx.caused_by(),
                        )));
                    }
                }
                Ok(())
            }
            (
                ValueObj::DataClass {
                    name: sub_name,
                    fields: sub_fields,
                },
                ValueObj::DataClass {
                    name: sup_name,
                    fields: sup_fields,
                },
            ) => {
                if sub_name == sup_name {
                    for (sub_k, sub_v) in sub_fields.iter() {
                        if let Some(sup_v) = sup_fields.get(sub_k) {
                            self.sub_unify_value(sub_v, sup_v)?;
                        } else {
                            log!(err "{maybe_sup} does not have field {sub_k}");
                            return Err(TyCheckErrors::from(TyCheckError::feature_error(
                                self.ctx.cfg.input.clone(),
                                line!() as usize,
                                self.loc.loc(),
                                &format!("unifying {maybe_sub} and {maybe_sup}"),
                                self.ctx.caused_by(),
                            )));
                        }
                    }
                    Ok(())
                } else {
                    type_feature_error!(
                        self.ctx,
                        self.loc.loc(),
                        &format!("unifying {maybe_sub} and {maybe_sup}")
                    )
                }
            }
            _ => Ok(()),
        }
    }

    /// allow_divergence = trueにすると、Num型変数と±Infの単一化を許す
    fn sub_unify_tp(
        &self,
        maybe_sub: &TyParam,
        maybe_sup: &TyParam,
        _variance: Option<Variance>,
        allow_divergence: bool,
    ) -> TyCheckResult<()> {
        if maybe_sub.has_no_unbound_var()
            && maybe_sup.has_no_unbound_var()
            && maybe_sub == maybe_sup
        {
            return Ok(());
        }
        match (maybe_sub, maybe_sup) {
            (TyParam::Type(sub), TyParam::Type(sup)) => self.sub_unify(sub, sup),
            (TyParam::Value(ValueObj::Type(sub)), TyParam::Type(sup)) => {
                self.sub_unify(sub.typ(), sup)
            }
            (TyParam::Type(sub), TyParam::Value(ValueObj::Type(sup))) => {
                self.sub_unify(sub, sup.typ())
            }
            (TyParam::Value(sub), TyParam::Value(sup)) => self.sub_unify_value(sub, sup),
            (TyParam::FreeVar(sub_fv), TyParam::FreeVar(sup_fv))
                if sub_fv.is_unbound() && sup_fv.is_unbound() =>
            {
                if sub_fv.level().unwrap() > sup_fv.level().unwrap() {
                    if !sub_fv.is_generalized() {
                        maybe_sub.link(maybe_sup, self.undoable);
                    }
                } else if !sup_fv.is_generalized() {
                    maybe_sup.link(maybe_sub, self.undoable);
                }
                Ok(())
            }
            (TyParam::FreeVar(sub_fv), _)
                if !self.change_generalized && sub_fv.is_generalized() =>
            {
                Ok(())
            }
            (TyParam::FreeVar(sub_fv), sup_tp) => {
                if let Some(l) = sub_fv.get_linked() {
                    return self.sub_unify_tp(&l, sup_tp, _variance, allow_divergence);
                }
                // sub_fvを参照しないようcloneする(あとでborrow_mutするため)
                let Some(fv_t) = sub_fv.constraint().unwrap().get_type().cloned() else {
                    return Err(TyCheckErrors::from(TyCheckError::feature_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        self.loc.loc(),
                        &format!("unifying {sub_fv} and {sup_tp}"),
                        self.ctx.caused_by(),
                    )));
                };
                let tp_t = self.ctx.get_tp_t(sup_tp)?;
                if self.ctx.supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if sub_fv.level() < Some(self.ctx.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t);
                        if self
                            .ctx
                            .is_sub_constraint_of(&sub_fv.constraint().unwrap(), &new_constraint)
                            || sub_fv.constraint().unwrap().get_type() == Some(&Type)
                        {
                            maybe_sub.update_constraint(new_constraint, self.undoable, false);
                        }
                    } else {
                        maybe_sub.link(sup_tp, self.undoable);
                    }
                    Ok(())
                } else if allow_divergence
                    && (self.ctx.eq_tp(sup_tp, &TyParam::value(Inf))
                        || self.ctx.eq_tp(sup_tp, &TyParam::value(NegInf)))
                    && self.ctx.subtype_of(&fv_t, &mono("Num"))
                {
                    maybe_sub.link(sup_tp, self.undoable);
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::feature_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        self.loc.loc(),
                        &format!("unifying {sub_fv} and {sup_tp}"),
                        self.ctx.caused_by(),
                    )))
                }
            }
            (_, TyParam::FreeVar(sup_fv))
                if !self.change_generalized && sup_fv.is_generalized() =>
            {
                Ok(())
            }
            (sub_tp, TyParam::FreeVar(sup_fv)) => {
                match &*sup_fv.borrow() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.sub_unify_tp(l, sub_tp, _variance, allow_divergence);
                    }
                    FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {}
                } // &fv is dropped
                  // fvを参照しないようにcloneする(あとでborrow_mutするため)
                let Some(fv_t) = sup_fv.constraint().unwrap().get_type().cloned() else {
                    return Err(TyCheckErrors::from(TyCheckError::feature_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        self.loc.loc(),
                        &format!("unifying {sub_tp} and {sup_fv}"),
                        self.ctx.caused_by(),
                    )));
                };
                let tp_t = self.ctx.get_tp_t(sub_tp)?;
                if self.ctx.supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if sup_fv.level() < Some(self.ctx.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t);
                        if self
                            .ctx
                            .is_sub_constraint_of(&sup_fv.constraint().unwrap(), &new_constraint)
                            || sup_fv.constraint().unwrap().get_type() == Some(&Type)
                        {
                            maybe_sup.update_constraint(new_constraint, self.undoable, false);
                        }
                    } else {
                        maybe_sup.link(sub_tp, self.undoable);
                    }
                    // self.sub_unify(&tp_t, &fv_t)
                    Ok(())
                } else if allow_divergence
                    && (self.ctx.eq_tp(sub_tp, &TyParam::value(Inf))
                        || self.ctx.eq_tp(sub_tp, &TyParam::value(NegInf)))
                    && self.ctx.subtype_of(&fv_t, &mono("Num"))
                {
                    maybe_sup.link(sub_tp, self.undoable);
                    // self.sub_unify(&tp_t, &fv_t)
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::feature_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        self.loc.loc(),
                        &format!("unifying {sub_tp} and {sup_fv}"),
                        self.ctx.caused_by(),
                    )))
                }
            }
            (TyParam::UnaryOp { op: lop, val: lval }, TyParam::UnaryOp { op: rop, val: rval })
                if lop == rop =>
            {
                self.sub_unify_tp(lval, rval, _variance, allow_divergence)
            }
            (
                TyParam::BinOp { op: lop, lhs, rhs },
                TyParam::BinOp {
                    op: rop,
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if lop == rop => {
                self.sub_unify_tp(lhs, lhs2, _variance, allow_divergence)?;
                self.sub_unify_tp(rhs, rhs2, _variance, allow_divergence)
            }
            (sub, TyParam::Erased(t)) => {
                let sub_t = self.ctx.get_tp_t(sub)?;
                if self.ctx.subtype_of(&sub_t, t) {
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        &sub_t,
                        t,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )))
                }
            }
            (TyParam::Erased(t), TyParam::Type(sup)) => {
                if self.ctx.subtype_of(t, &Type::Type) {
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        t,
                        sup,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )))
                }
            }
            (sub, TyParam::Type(sup)) => {
                let l = self.ctx.convert_tp_into_type(sub.clone()).map_err(|_| {
                    TyCheckError::tp_to_type_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        sub,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )
                })?;
                self.sub_unify(&l, sup)?;
                Ok(())
            }
            (TyParam::Erased(t), sup) => {
                let sup_t = self.ctx.get_tp_t(sup)?;
                if self.ctx.subtype_of(t, &sup_t) {
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        t,
                        &sup_t,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )))
                }
            }
            (TyParam::Type(sub), sup) => {
                let r = self.ctx.convert_tp_into_type(sup.clone()).map_err(|_| {
                    TyCheckError::tp_to_type_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        sup,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )
                })?;
                self.sub_unify(sub, &r)?;
                Ok(())
            }
            (TyParam::List(sub), TyParam::List(sup))
            | (TyParam::Tuple(sub), TyParam::Tuple(sup)) => {
                for (l, r) in sub.iter().zip(sup.iter()) {
                    self.sub_unify_tp(l, r, _variance, allow_divergence)?;
                }
                Ok(())
            }
            (TyParam::Dict(sub), TyParam::Dict(sup)) => {
                for (sub_k, sub_v) in sub.iter() {
                    if let Some(sup_v) = sup
                        .get(sub_k)
                        .or_else(|| sub_tpdict_get(sup, sub_k, self.ctx))
                    {
                        // self.sub_unify_tp(sub_k, sup_k, _variance, loc, allow_divergence)?;
                        self.sub_unify_tp(sub_v, sup_v, _variance, allow_divergence)?;
                    } else {
                        log!(err "{sup} does not have key {sub_k}");
                        // TODO:
                        return Err(TyCheckErrors::from(TyCheckError::feature_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            self.loc.loc(),
                            &format!("unifying {sub} and {sup}"),
                            self.ctx.caused_by(),
                        )));
                    }
                }
                Ok(())
            }
            (TyParam::Record(sub), TyParam::Record(sup)) => {
                for (sub_k, sub_v) in sub.iter() {
                    if let Some(sup_v) = sup.get(sub_k) {
                        self.sub_unify_tp(sub_v, sup_v, _variance, allow_divergence)?;
                    } else {
                        log!(err "{sup} does not have field {sub_k}");
                        return Err(TyCheckErrors::from(TyCheckError::feature_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            self.loc.loc(),
                            &format!("unifying {sub} and {sup}"),
                            self.ctx.caused_by(),
                        )));
                    }
                }
                Ok(())
            }
            (
                TyParam::ProjCall { obj, attr, args },
                TyParam::ProjCall {
                    obj: o2,
                    attr: a2,
                    args: args2,
                },
            ) => {
                if attr == a2 {
                    self.sub_unify_tp(obj, o2, _variance, allow_divergence)?;
                    for (l, r) in args.iter().zip(args2.iter()) {
                        self.sub_unify_tp(l, r, _variance, allow_divergence)?;
                    }
                    Ok(())
                } else {
                    if DEBUG_MODE {
                        todo!()
                    }
                    Ok(())
                }
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
            ) if ln == rn => {
                for (l, r) in largs.iter().zip(rargs.iter()) {
                    self.sub_unify_tp(l, r, _variance, allow_divergence)?;
                }
                Ok(())
            }
            (TyParam::Lambda(sub_l), TyParam::Lambda(sup_l)) => {
                for (sup_nd, sub_nd) in sup_l.nd_params.iter().zip(sub_l.nd_params.iter()) {
                    self.sub_unify(sub_nd.typ(), sup_nd.typ())?;
                }
                if let Some((sup_var, sub_var)) =
                    sup_l.var_params.as_ref().zip(sub_l.var_params.as_ref())
                {
                    self.sub_unify(sub_var.typ(), sup_var.typ())?;
                }
                for (sup_d, sub_d) in sup_l.d_params.iter().zip(sub_l.d_params.iter()) {
                    self.sub_unify(sub_d.typ(), sup_d.typ())?;
                }
                if let Some((sup_kw_var, sub_kw_var)) = sup_l
                    .kw_var_params
                    .as_ref()
                    .zip(sub_l.kw_var_params.as_ref())
                {
                    self.sub_unify(sub_kw_var.typ(), sup_kw_var.typ())?;
                }
                for (sub_expr, sup_expr) in sub_l.body.iter().zip(sup_l.body.iter()) {
                    self.sub_unify_tp(sub_expr, sup_expr, _variance, allow_divergence)?;
                }
                Ok(())
            }
            (l, TyParam::Value(sup)) => {
                let sup = match Context::convert_value_into_tp(sup.clone()) {
                    Ok(r) => r,
                    Err(tp) => {
                        return type_feature_error!(
                            self.ctx,
                            self.loc.loc(),
                            &format!("unifying {l} and {tp}")
                        )
                    }
                };
                self.sub_unify_tp(maybe_sub, &sup, _variance, allow_divergence)
            }
            (TyParam::Value(sub), r) => {
                let sub = match Context::convert_value_into_tp(sub.clone()) {
                    Ok(l) => l,
                    Err(tp) => {
                        return type_feature_error!(
                            self.ctx,
                            self.loc.loc(),
                            &format!("unifying {tp} and {r}")
                        )
                    }
                };
                self.sub_unify_tp(&sub, maybe_sup, _variance, allow_divergence)
            }
            (l, r) => {
                log!(err "{l} / {r}");
                type_feature_error!(self.ctx, self.loc.loc(), &format!("unifying {l} and {r}"))
            }
        }
    }

    /// predは正規化されているとする
    fn sub_unify_pred(&self, sub_pred: &Predicate, sup_pred: &Predicate) -> TyCheckResult<()> {
        match (sub_pred, sup_pred) {
            (Pred::Const(_), Pred::Const(_)) => Ok(()),
            (Pred::Value(sub), Pred::Value(sup)) => self.sub_unify_value(sub, sup),
            (Pred::Equal { rhs, .. }, Pred::Equal { rhs: rhs2, .. })
            | (Pred::GreaterEqual { rhs, .. }, Pred::GreaterEqual { rhs: rhs2, .. })
            | (Pred::LessEqual { rhs, .. }, Pred::LessEqual { rhs: rhs2, .. })
            | (Pred::NotEqual { rhs, .. }, Pred::NotEqual { rhs: rhs2, .. }) => {
                self.sub_unify_tp(rhs, rhs2, None, false)
            }
            (Pred::And(l1, r1), Pred::And(l2, r2)) | (Pred::Or(l1, r1), Pred::Or(l2, r2)) => {
                match (self.sub_unify_pred(l1, l2), self.sub_unify_pred(r1, r2)) {
                    (Ok(()), Ok(())) => Ok(()),
                    (Ok(()), Err(e)) | (Err(e), Ok(())) | (Err(e), Err(_)) => Err(e),
                }
            }
            (Pred::Not(l), Pred::Not(r)) => self.sub_unify_pred(r, l),
            // sub_unify_pred(I == M, I <= ?N(: Nat)) ==> ?N(: M..)
            (Pred::Equal { rhs, .. }, Pred::LessEqual { rhs: rhs2, .. }) => {
                self.coerce_greater_than(rhs2, rhs)
            }
            // sub_unify_pred(I >= 0, I >= ?M and I <= ?N) ==> ?M => 0, ?N => Inf
            (Pred::GreaterEqual { rhs, .. }, Pred::And(l, r))
            | (Predicate::And(l, r), Pred::GreaterEqual { rhs, .. }) => {
                match (l.as_ref(), r.as_ref()) {
                    (
                        Pred::GreaterEqual { rhs: ge_rhs, .. },
                        Pred::LessEqual { rhs: le_rhs, .. },
                    )
                    | (
                        Pred::LessEqual { rhs: le_rhs, .. },
                        Pred::GreaterEqual { rhs: ge_rhs, .. },
                    ) => {
                        self.sub_unify_tp(rhs, ge_rhs, None, false)?;
                        self.sub_unify_tp(le_rhs, &TyParam::value(Inf), None, true)
                    }
                    _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        sub_pred,
                        sup_pred,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    ))),
                }
            }
            (Pred::LessEqual { rhs, .. }, Pred::And(l, r))
            | (Pred::And(l, r), Pred::LessEqual { rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual { rhs: ge_rhs, .. }, Pred::LessEqual { rhs: le_rhs, .. })
                | (Pred::LessEqual { rhs: le_rhs, .. }, Pred::GreaterEqual { rhs: ge_rhs, .. }) => {
                    self.sub_unify_tp(rhs, le_rhs, None, false)?;
                    self.sub_unify_tp(ge_rhs, &TyParam::value(NegInf), None, true)
                }
                _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                    self.ctx.cfg.input.clone(),
                    line!() as usize,
                    sub_pred,
                    sup_pred,
                    self.loc.loc(),
                    self.ctx.caused_by(),
                ))),
            },
            (Pred::Equal { rhs, .. }, Pred::And(l, r))
            | (Pred::And(l, r), Pred::Equal { rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual { rhs: ge_rhs, .. }, Pred::LessEqual { rhs: le_rhs, .. })
                | (Pred::LessEqual { rhs: le_rhs, .. }, Pred::GreaterEqual { rhs: ge_rhs, .. }) => {
                    self.sub_unify_tp(rhs, le_rhs, None, false)?;
                    self.sub_unify_tp(rhs, ge_rhs, None, false)
                }
                _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                    self.ctx.cfg.input.clone(),
                    line!() as usize,
                    sub_pred,
                    sup_pred,
                    self.loc.loc(),
                    self.ctx.caused_by(),
                ))),
            },
            (
                Predicate::GeneralEqual { lhs, rhs },
                Predicate::GeneralEqual {
                    lhs: sup_lhs,
                    rhs: sup_rhs,
                },
            )
            | (
                Predicate::GeneralNotEqual { lhs, rhs },
                Predicate::GeneralNotEqual {
                    lhs: sup_lhs,
                    rhs: sup_rhs,
                },
            )
            | (
                Predicate::GeneralGreaterEqual { lhs, rhs },
                Predicate::GeneralGreaterEqual {
                    lhs: sup_lhs,
                    rhs: sup_rhs,
                },
            )
            | (
                Predicate::GeneralLessEqual { lhs, rhs },
                Predicate::GeneralLessEqual {
                    lhs: sup_lhs,
                    rhs: sup_rhs,
                },
            ) => {
                self.sub_unify_pred(lhs, sup_lhs)?;
                self.sub_unify_pred(rhs, sup_rhs)
            }
            (
                Pred::Call { receiver, args, .. },
                Pred::Call {
                    receiver: sup_receiver,
                    args: sup_args,
                    ..
                },
            ) => {
                self.sub_unify_tp(receiver, sup_receiver, None, false)?;
                for (l, r) in args.iter().zip(sup_args.iter()) {
                    self.sub_unify_tp(l, r, None, false)?;
                }
                Ok(())
            }
            (call @ Predicate::Call { .. }, Predicate::Value(ValueObj::Bool(b)))
            | (Predicate::Value(ValueObj::Bool(b)), call @ Predicate::Call { .. }) => {
                if let Ok(Predicate::Value(ValueObj::Bool(evaled))) =
                    self.ctx.eval_pred(call.clone())
                {
                    if &evaled == b {
                        return Ok(());
                    }
                }
                Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                    self.ctx.cfg.input.clone(),
                    line!() as usize,
                    sub_pred,
                    sup_pred,
                    self.loc.loc(),
                    self.ctx.caused_by(),
                )))
            }
            _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                self.ctx.cfg.input.clone(),
                line!() as usize,
                sub_pred,
                sup_pred,
                self.loc.loc(),
                self.ctx.caused_by(),
            ))),
        }
    }

    fn coerce_greater_than(&self, target: &TyParam, value: &TyParam) -> TyCheckResult<()> {
        match target {
            TyParam::FreeVar(_fv) => {
                if let Ok(evaled) = self.ctx.eval_tp(value.clone()) {
                    let pred = Predicate::ge(FRESH_GEN.fresh_varname(), evaled);
                    let new_type = self.ctx.type_from_pred(pred);
                    let new_constr = Constraint::new_type_of(Type::from(new_type));
                    target.update_constraint(new_constr, self.undoable, false);
                }
                Ok(())
            }
            TyParam::BinOp {
                op: OpKind::Sub,
                lhs,
                rhs,
            } => {
                let value = TyParam::bin(OpKind::Add, value.clone(), *rhs.clone());
                self.coerce_greater_than(lhs, &value)
            }
            _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                self.ctx.cfg.input.clone(),
                line!() as usize,
                &Pred::eq("_".into(), value.clone()),
                &Pred::le("_".into(), target.clone()),
                self.loc.loc(),
                self.ctx.caused_by(),
            ))),
        }
    }

    /// Assuming that `sub` is a subtype of `sup`, fill in the type variable to satisfy the assumption.
    ///
    /// When comparing arguments and parameter, the left side (`sub`) is the argument (found) and the right side (`sup`) is the parameter (expected).
    ///
    /// The parameter type must be a supertype of the argument type.
    /// ```python
    /// sub_unify({I: Int | I == 0}, ?T(<: Ord)): (/* OK */)
    /// sub_unify(Int, ?T(:> Nat)): (?T :> Int)
    /// sub_unify(Nat, ?T(:> Int)): (/* OK */)
    /// sub_unify(Nat, Add(?R)): (?R => Nat, Nat.Output => Nat)
    /// sub_unify([?T; 0], Mutate): (/* OK */)
    /// ```
    fn sub_unify(&self, maybe_sub: &Type, maybe_sup: &Type) -> TyCheckResult<()> {
        log!(info "trying {}sub_unify:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}", self.undoable.map_or("", |_| "undoable_"));
        if self.recursion_limit.fetch_sub(1, Ordering::SeqCst) == 0 {
            self.recursion_limit.store(128, Ordering::SeqCst);
            log!(err "recursion limit exceeded: {maybe_sub} / {maybe_sup}");
            return Err(TyCheckError::recursion_limit(
                self.ctx.cfg.input.clone(),
                line!() as usize,
                self.loc.loc(),
                fn_name!(),
                line!(),
            )
            .into());
        }
        // In this case, there is no new information to be gained
        // この場合、特に新しく得られる情報はない
        if maybe_sub == &Type::Never || maybe_sup == &Type::Obj || maybe_sup.addr_eq(maybe_sub) {
            log!(info "no-op:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
            return Ok(());
        }
        // API definition was failed and inspection is useless after this
        if maybe_sub == &Type::Failure || maybe_sup == &Type::Failure {
            log!(info "no-op:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
            return Ok(());
        }
        self.occur(maybe_sub, maybe_sup).map_err(|err| {
            log!(err "occur error: {maybe_sub} / {maybe_sup}");
            err
        })?;
        let maybe_sub_is_sub = self.ctx.subtype_of(maybe_sub, maybe_sup);
        if !maybe_sub_is_sub {
            log!(err "{maybe_sub} !<: {maybe_sup}");
            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                self.ctx.cfg.input.clone(),
                line!() as usize,
                self.loc.loc(),
                self.ctx.caused_by(),
                self.param_name.as_ref().unwrap_or(&Str::ever("_")),
                None,
                maybe_sup,
                maybe_sub,
                self.ctx.get_candidates(maybe_sub),
                self.ctx.get_simple_type_mismatch_hint(maybe_sup, maybe_sub),
            )));
        } else if maybe_sub.has_no_unbound_var() && maybe_sup.has_no_unbound_var() {
            log!(info "no-op:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
            return Ok(());
        }
        match (maybe_sub, maybe_sup) {
            (FreeVar(sub_fv), _) if sub_fv.is_linked() => {
                self.sub_unify(sub_fv.unsafe_crack(), maybe_sup)?;
            }
            (_, FreeVar(sup_fv)) if sup_fv.is_linked() => {
                self.sub_unify(maybe_sub, sup_fv.unsafe_crack())?;
            }
            // lfv's sup can be shrunk (take min), rfv's sub can be expanded (take union)
            // lfvのsupは縮小可能(minを取る)、rfvのsubは拡大可能(unionを取る)
            // sub_unify(?T[0](:> Never, <: Int), ?U[1](:> Never, <: Nat)): (/* ?U[1] --> ?T[0](:> Never, <: Nat))
            // sub_unify(?T[1](:> Never, <: Nat), ?U[0](:> Never, <: Int)): (/* ?T[1] --> ?U[0](:> Never, <: Nat))
            // sub_unify(?T[0](:> Never, <: Str), ?U[1](:> Never, <: Int)): (?T[0](:> Never, <: Str and Int) --> Error!)
            // sub_unify(?T[0](:> Int, <: Add()), ?U[1](:> Never, <: Mul())): (?T[0](:> Int, <: Add() and Mul()))
            // sub_unify(?T[0](:> Str, <: Obj), ?U[1](:> Int, <: Obj)): (/* ?U[1] --> ?T[0](:> Str or Int) */)
            (FreeVar(sub_fv), FreeVar(sup_fv))
                if sub_fv.constraint_is_sandwiched() && sup_fv.constraint_is_sandwiched() =>
            {
                if !self.change_generalized && (sub_fv.is_generalized() || sup_fv.is_generalized())
                {
                    log!(info "generalized:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
                    return Ok(());
                }
                let (lsub, lsup) = sub_fv.get_subsup().unwrap();
                let (rsub, rsup) = sup_fv.get_subsup().unwrap();
                // ?T(<: Add(?T))
                // ?U(:> {1, 2}, <: Add(?U)) ==> {1, 2}
                sup_fv.dummy_link();
                sub_fv.dummy_link();
                if lsub.qual_name() == rsub.qual_name() {
                    for (lps, rps) in lsub.typarams().iter().zip(rsub.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, false).map_err(|errs| {
                            sup_fv.undo();
                            sub_fv.undo();
                            errs
                        })?;
                    }
                }
                // lsup: Add(?X(:> Int)), rsup: Add(?Y(:> Nat))
                //   => lsup: Add(?X(:> Int)), rsup: Add((?X(:> Int)))
                if lsup.qual_name() == rsup.qual_name() {
                    for (lps, rps) in lsup.typarams().iter().zip(rsup.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, false).map_err(|errs| {
                            sup_fv.undo();
                            sub_fv.undo();
                            errs
                        })?;
                    }
                }
                sup_fv.undo();
                sub_fv.undo();
                let intersec = self.ctx.intersection(&lsup, &rsup);
                if intersec == Type::Never {
                    return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )));
                }
                let union = self.ctx.union(&lsub, &rsub);
                if lsub.union_size().max(rsub.union_size()) < union.union_size() {
                    let (l, r) = union.union_pair().unwrap_or((lsub, rsub.clone()));
                    let unified = self.unify(&l, &r);
                    if unified.is_none() {
                        let maybe_sub = self.ctx.readable_type(maybe_sub.clone());
                        let union = self.ctx.readable_type(union);
                        return Err(TyCheckErrors::from(TyCheckError::implicit_widening_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            self.loc.loc(),
                            self.ctx.caused_by(),
                            &maybe_sub,
                            &union,
                        )));
                    }
                }
                // e.g. intersec == Int, rsup == Add(?T)
                //   => ?T(:> Int)
                if !(intersec.is_recursive() && rsup.is_recursive()) {
                    self.sub_unify(&intersec, &rsup)?;
                }
                self.sub_unify(&rsub, &union)?;
                // self.sub_unify(&intersec, &lsup, loc, param_name)?;
                // self.sub_unify(&lsub, &union, loc, param_name)?;
                match sub_fv
                    .level()
                    .unwrap_or(GENERIC_LEVEL)
                    .cmp(&sup_fv.level().unwrap_or(GENERIC_LEVEL))
                {
                    std::cmp::Ordering::Less => {
                        if sup_fv.level().unwrap_or(GENERIC_LEVEL) == GENERIC_LEVEL {
                            maybe_sup.update_tyvar(union, intersec, self.undoable, false);
                            maybe_sub.link(maybe_sup, self.undoable);
                        } else {
                            maybe_sub.update_tyvar(union, intersec, self.undoable, false);
                            maybe_sup.link(maybe_sub, self.undoable);
                        }
                    }
                    std::cmp::Ordering::Greater => {
                        if sub_fv.level().unwrap_or(GENERIC_LEVEL) == GENERIC_LEVEL {
                            maybe_sub.update_tyvar(union, intersec, self.undoable, false);
                            maybe_sup.link(maybe_sub, self.undoable);
                        } else {
                            maybe_sup.update_tyvar(union, intersec, self.undoable, false);
                            maybe_sub.link(maybe_sup, self.undoable);
                        }
                    }
                    std::cmp::Ordering::Equal => {
                        // choose named one
                        if sup_fv.is_named_unbound() {
                            maybe_sup.update_tyvar(union, intersec, self.undoable, false);
                            maybe_sub.link(maybe_sup, self.undoable);
                        } else {
                            maybe_sub.update_tyvar(union, intersec, self.undoable, false);
                            maybe_sup.link(maybe_sub, self.undoable);
                        }
                    }
                }
            }
            (
                Bounded {
                    sub: lsub,
                    sup: lsup,
                },
                FreeVar(sup_fv),
            ) if sup_fv.constraint_is_sandwiched() => {
                if !self.change_generalized && sup_fv.is_generalized() {
                    log!(info "generalized:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
                    return Ok(());
                }
                let (rsub, rsup) = sup_fv.get_subsup().unwrap();
                // ?T(<: Add(?T))
                // ?U(:> {1, 2}, <: Add(?U)) ==> {1, 2}
                sup_fv.dummy_link();
                if lsub.qual_name() == rsub.qual_name() {
                    for (lps, rps) in lsub.typarams().iter().zip(rsub.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, false).map_err(|errs| {
                            sup_fv.undo();
                            errs
                        })?;
                    }
                }
                // lsup: Add(?X(:> Int)), rsup: Add(?Y(:> Nat))
                //   => lsup: Add(?X(:> Int)), rsup: Add((?X(:> Int)))
                if lsup.qual_name() == rsup.qual_name() {
                    for (lps, rps) in lsup.typarams().iter().zip(rsup.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, false).map_err(|errs| {
                            sup_fv.undo();
                            errs
                        })?;
                    }
                }
                sup_fv.undo();
                let intersec = self.ctx.intersection(lsup, &rsup);
                if intersec == Type::Never {
                    return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )));
                }
                let union = self.ctx.union(lsub, &rsub);
                if lsub.union_size().max(rsub.union_size()) < union.union_size() {
                    let (l, r) = union.union_pair().unwrap_or((*lsub.clone(), rsub.clone()));
                    let unified = self.unify(&l, &r);
                    if unified.is_none() {
                        let maybe_sub = self.ctx.readable_type(maybe_sub.clone());
                        let union = self.ctx.readable_type(union);
                        return Err(TyCheckErrors::from(TyCheckError::implicit_widening_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            self.loc.loc(),
                            self.ctx.caused_by(),
                            &maybe_sub,
                            &union,
                        )));
                    }
                }
                // e.g. intersec == Int, rsup == Add(?T)
                //   => ?T(:> Int)
                self.sub_unify(&intersec, &rsup)?;
                self.sub_unify(&rsub, &union)?;
                // self.sub_unify(&intersec, &lsup, loc, param_name)?;
                // self.sub_unify(&lsub, &union, loc, param_name)?;
                maybe_sup.update_tyvar(union, intersec, self.undoable, false);
            }
            // (Int or ?T) <: (?U or Int)
            // OK: (Int <: Int); (?T <: ?U)
            // NG: (Int <: ?U); (?T <: Int)
            (Or(l1, r1), Or(l2, r2)) | (And(l1, r1), And(l2, r2)) => {
                if self.ctx.subtype_of(l1, l2) && self.ctx.subtype_of(r1, r2) {
                    let (l_sup, r_sup) = if !l1.is_unbound_var()
                        && !r2.is_unbound_var()
                        && self.ctx.subtype_of(l1, r2)
                    {
                        (r2, l2)
                    } else {
                        (l2, r2)
                    };
                    self.sub_unify(l1, l_sup)?;
                    self.sub_unify(r1, r_sup)?;
                } else {
                    self.sub_unify(l1, r2)?;
                    self.sub_unify(r1, l2)?;
                }
            }
            // NG: Nat <: ?T or Int ==> Nat or Int (?T = Nat)
            // OK: Nat <: ?T or Int ==> ?T or Int
            (sub, Or(l, r))
                if l.is_unbound_var()
                    && !sub.is_unbound_var()
                    && !r.is_unbound_var()
                    && self.ctx.subtype_of(sub, r) => {}
            (sub, Or(l, r))
                if r.is_unbound_var()
                    && !sub.is_unbound_var()
                    && !l.is_unbound_var()
                    && self.ctx.subtype_of(sub, l) => {}
            // e.g. Structural({ .method = (self: T) -> Int })/T
            (Structural(sub), FreeVar(sup_fv))
                if sup_fv.is_unbound() && sub.contains_tvar(sup_fv) => {}
            (_, FreeVar(sup_fv)) if !self.change_generalized && sup_fv.is_generalized() => {}
            (_, FreeVar(sup_fv)) if sup_fv.is_unbound() => {
                // * sub_unify(Nat, ?E(<: Eq(?E)))
                // sub !<: l => OK (sub will widen)
                // sup !:> l => Error
                // * sub_unify(Str,   ?T(:> _,     <: Int)): (/* Error */)
                // * sub_unify(Ratio, ?T(:> _,     <: Int)): (/* Error */)
                // sub = max(l, sub) if max exists
                // * sub_unify(Nat,   ?T(:> Int,   <: _)): (/* OK */)
                // * sub_unify(Int,   ?T(:> Nat,   <: Obj)): (?T(:> Int, <: Obj))
                // * sub_unify(Nat,   ?T(:> Never, <: Add(?R))): (?T(:> Nat, <: Add(?R))
                // sub = union(l, sub) if max does not exist
                // * sub_unify(Str,   ?T(:> Int,   <: Obj)): (?T(:> Str or Int, <: Obj))
                // * sub_unify({0},   ?T(:> {1},   <: Nat)): (?T(:> {0, 1}, <: Nat))
                // * sub_unify(Bool,  ?T(<: Bool or Y)): (?T == Bool)
                // * sub_unify(Float, ?T(<: Structural{ .imag = ?U })) ==> ?U == Float
                if let Type::Refinement(refine) = maybe_sub {
                    if refine.t.addr_eq(maybe_sup) {
                        return Ok(());
                    }
                }
                if let Some((sub, mut sup)) = sup_fv.get_subsup() {
                    if sup.is_structural() || !sup.is_recursive() {
                        self.sub_unify(maybe_sub, &sup)?;
                    }
                    let new_sub = self.ctx.union(maybe_sub, &sub);
                    // Expanding to an Or-type is prohibited by default
                    // This increases the quality of error reporting
                    // (Try commenting out this part and run tests/should_err/subtyping.er to see the error report changes on lines 29-30)
                    if maybe_sub.union_size().max(sub.union_size()) < new_sub.union_size()
                        && new_sub.ors().iter().any(|t| !t.is_unbound_var())
                    {
                        let (l, r) = new_sub.union_pair().unwrap_or((maybe_sub.clone(), sub));
                        let unified = self.unify(&l, &r);
                        if let Some(unified) = unified {
                            log!("unify({l}, {r}) == {unified}");
                        } else {
                            let maybe_sub = self.ctx.readable_type(maybe_sub.clone());
                            let new_sub = self.ctx.readable_type(new_sub);
                            return Err(TyCheckErrors::from(
                                TyCheckError::implicit_widening_error(
                                    self.ctx.cfg.input.clone(),
                                    line!() as usize,
                                    self.loc.loc(),
                                    self.ctx.caused_by(),
                                    &maybe_sub,
                                    &new_sub,
                                ),
                            ));
                        }
                    }
                    if sup.contains_union(&new_sub) {
                        maybe_sup.link(&new_sub, self.undoable); // Bool <: ?T <: Bool or Y ==> ?T == Bool
                    } else {
                        maybe_sup.update_tyvar(new_sub, mem::take(&mut sup), self.undoable, true);
                    }
                }
                // sub_unify(Nat, ?T(: Type)): (/* ?T(:> Nat) */)
                else if let Some(ty) = sup_fv.get_type() {
                    if self.ctx.supertype_of(&Type, &ty) {
                        let constr = Constraint::new_supertype_of(maybe_sub.clone());
                        maybe_sup.update_constraint(constr, self.undoable, true);
                    } else {
                        todo!("{maybe_sub} <: {maybe_sup}")
                    }
                }
            }
            (FreeVar(sub_fv), Structural(struct_sup)) if sub_fv.is_unbound() => {
                let Some((sub, sup)) = sub_fv.get_subsup() else {
                    log!(err "{sub_fv} is not a type variable");
                    return Ok(());
                };
                let sub_fields = self.ctx.fields(maybe_sub);
                for (sup_field, sup_ty) in self.ctx.fields(struct_sup) {
                    sub_fv.dummy_link();
                    if let Some((_, sub_ty)) = sub_fields.get_key_value(&sup_field) {
                        self.sub_unify(sub_ty, &sup_ty).map_err(|errs| {
                            sub_fv.undo();
                            errs
                        })?;
                    } else if !self.ctx.subtype_of(&sub, &Never) {
                        sub_fv.undo();
                        maybe_sub.coerce(self.undoable);
                        return self.sub_unify(maybe_sub, maybe_sup);
                    } else {
                        // e.g. ?T / Structural({ .method = (self: ?T) -> Int })
                        let constr = Constraint::new_sandwiched(
                            sub.clone(),
                            self.ctx.intersection(&sup, maybe_sup),
                        );
                        sub_fv.undo();
                        sub_fv.update_constraint(constr, false);
                    }
                }
            }
            (FreeVar(sub_fv), Ref(sup)) if sub_fv.is_unbound() => {
                self.sub_unify(maybe_sub, sup)?;
            }
            (FreeVar(sub_fv), _) if !self.change_generalized && sub_fv.is_generalized() => {}
            (FreeVar(sub_fv), _) if sub_fv.is_unbound() => {
                // sub !<: r => Error
                // * sub_unify(?T(:> Int,   <: _), Nat): (/* Error */)
                // * sub_unify(?T(:> Nat,   <: _), Str): (/* Error */)
                // sup !:> r => Error
                // * sub_unify(?T(:> _, <: Str), Int): (/* Error */)
                // * sub_unify(?T(:> _, <: Int), Nat): (/* Error */)
                // sub <: r, sup :> r => sup = min(sup, r) if min exists
                // * sub_unify(?T(:> Never, <: Nat), Int): (/* OK */)
                // * sub_unify(?T(:> Nat,   <: Obj), Int): (?T(:> Nat,   <: Int))
                // sup = intersection(sup, r) if min does not exist
                // * sub_unify(?T(<: {1}), {0}): (* ?T == Never *)
                // * sub_unify(?T(<: Eq and Ord), Show): (?T(<: Eq and Ord and Show))
                // * sub_unify(?T(:> [Int; 4]), [Int, _]): (* ?T == [Int; 4] *)
                if let Some((mut sub, sup)) = sub_fv.get_subsup() {
                    if sup.is_structural() {
                        return Ok(());
                    }
                    let sub = mem::take(&mut sub);
                    let new_sup = if let Some(new_sup) = self.ctx.min(&sup, maybe_sup).either() {
                        new_sup.clone()
                    } else {
                        self.ctx.intersection(&sup, maybe_sup)
                    };
                    self.sub_unify(&sub, &new_sup)?;
                    // ?T(:> Int, <: Int) ==> ?T == Int
                    // ?T(:> List(Int, 3), <: List(?T, ?N)) ==> ?T == List(Int, 3)
                    // ?T(:> List(Int, 3), <: Indexable(?K, ?V)) ==> ?T(:> List(Int, 3), <: Indexable(0..2, Int))
                    if !sub.is_refinement()
                        && new_sup.qual_name() == sub.qual_name()
                        && !new_sup.is_unbound_var()
                        && !sub.is_unbound_var()
                    {
                        maybe_sub.link(&sub, self.undoable);
                    } else {
                        maybe_sub.update_tyvar(sub, new_sup, self.undoable, true);
                    }
                }
                // sub_unify(?T(: Type), Int): (?T(<: Int))
                else if let Some(ty) = sub_fv.get_type() {
                    if self.ctx.supertype_of(&Type, &ty) {
                        let constr = Constraint::new_subtype_of(maybe_sup.clone());
                        maybe_sub.update_constraint(constr, self.undoable, true);
                    } else {
                        todo!("{maybe_sub} <: {maybe_sup}")
                    }
                }
            }
            (Record(sub_rec), Record(sup_rec)) => {
                for (k, l) in sub_rec.iter() {
                    if let Some(r) = sup_rec.get(k) {
                        self.sub_unify(l, r)?;
                    } else {
                        return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            maybe_sub,
                            maybe_sup,
                            self.loc.loc(),
                            self.ctx.caused_by(),
                        )));
                    }
                }
            }
            (NamedTuple(sub_tup), NamedTuple(sup_tup)) => {
                for ((_, lt), (_, rt)) in sub_tup.iter().zip(sup_tup.iter()) {
                    self.sub_unify(lt, rt)?;
                }
            }
            (Subr(sub_subr), Subr(sup_subr)) => {
                // (Int, *Int) -> ... <: (T, U, V) -> ...
                if let Some(sub_var) = sub_subr.var_params.as_deref() {
                    sub_subr
                        .non_default_params
                        .iter()
                        .chain(repeat(sub_var))
                        .zip(sup_subr.non_default_params.iter())
                        .try_for_each(|(sub, sup)| {
                            // contravariant
                            self.sub_unify(sup.typ(), sub.typ())
                        })?;
                } else {
                    // (self: Self, Int) -> ... <: T -> ...
                    let sub_params = if !sup_subr.is_method() && sub_subr.is_method() {
                        sub_subr
                            .non_default_params
                            .iter()
                            .skip(1)
                            .chain(&sub_subr.default_params)
                    } else {
                        #[allow(clippy::iter_skip_zero)]
                        sub_subr
                            .non_default_params
                            .iter()
                            .skip(0)
                            .chain(&sub_subr.default_params)
                    };
                    sub_params
                        .zip(sup_subr.non_default_params.iter())
                        .try_for_each(|(sub, sup)| {
                            // contravariant
                            self.sub_unify(sup.typ(), sub.typ())
                        })?;
                }
                sub_subr
                    .var_params
                    .iter()
                    .zip(sup_subr.var_params.iter())
                    .try_for_each(|(sub, sup)| {
                        // contravariant
                        self.sub_unify(sup.typ(), sub.typ())
                    })?;
                for sup_pt in sup_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == sup_pt.name())
                    {
                        // contravariant
                        self.sub_unify(sup_pt.typ(), sub_pt.typ())?;
                    } else {
                        let param_name = sup_pt.name().map_or("_", |s| &s[..]);
                        let similar_param = erg_common::levenshtein::get_similar_name(
                            sup_subr
                                .default_params
                                .iter()
                                .map(|pt| pt.name().map_or("_", |s| &s[..])),
                            param_name,
                        );
                        return Err(TyCheckErrors::from(
                            TyCheckError::default_param_not_found_error(
                                self.ctx.cfg.input.clone(),
                                line!() as usize,
                                self.loc.loc(),
                                self.ctx.caused_by(),
                                param_name,
                                similar_param,
                            ),
                        ));
                    }
                }
                // covariant
                self.sub_unify(&sub_subr.return_t, &sup_subr.return_t)?;
            }
            (Quantified(sub_subr), Subr(sup_subr)) => {
                let Ok(sub_subr) = <&SubrType>::try_from(sub_subr.as_ref()) else {
                    unreachable!()
                };
                sub_subr
                    .non_default_params
                    .iter()
                    .zip(sup_subr.non_default_params.iter())
                    .try_for_each(|(sub, sup)| {
                        if !self.change_generalized && sub.typ().is_generalized() {
                            Ok(())
                        }
                        // contravariant
                        else {
                            self.sub_unify(sup.typ(), sub.typ())
                        }
                    })?;
                for sup_pt in sup_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == sup_pt.name())
                    {
                        if !self.change_generalized && sup_pt.typ().is_generalized() {
                            continue;
                        }
                        // contravariant
                        self.sub_unify(sup_pt.typ(), sub_pt.typ())?;
                    } else {
                        todo!("{maybe_sub} <: {maybe_sup}")
                    }
                }
                if !sub_subr.return_t.is_generalized() {
                    // covariant
                    self.sub_unify(&sub_subr.return_t, &sup_subr.return_t)?;
                }
            }
            (Subr(sub_subr), Quantified(sup_subr)) => {
                let Ok(sup_subr) = <&SubrType>::try_from(sup_subr.as_ref()) else {
                    unreachable!()
                };
                sub_subr
                    .non_default_params
                    .iter()
                    .zip(sup_subr.non_default_params.iter())
                    .try_for_each(|(sub, sup)| {
                        // contravariant
                        if !self.change_generalized && sup.typ().is_generalized() {
                            Ok(())
                        } else {
                            self.sub_unify(sup.typ(), sub.typ())
                        }
                    })?;
                for sup_pt in sup_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == sup_pt.name())
                    {
                        // contravariant
                        if !self.change_generalized && sup_pt.typ().is_generalized() {
                            continue;
                        }
                        self.sub_unify(sup_pt.typ(), sub_pt.typ())?;
                    } else {
                        todo!("{maybe_sub} <: {maybe_sup}")
                    }
                }
                if !sup_subr.return_t.is_generalized() {
                    // covariant
                    self.sub_unify(&sub_subr.return_t, &sup_subr.return_t)?;
                }
            }
            (
                Poly {
                    name: ln,
                    params: lps,
                },
                Poly {
                    name: rn,
                    params: rps,
                },
            ) => {
                // e.g. Set(?T) <: Eq(Set(?T))
                //      List(Str) <: Iterable(Str)
                //      Zip(T, U) <: Iterable(Tuple([T, U]))
                if ln != rn {
                    self.nominal_sub_unify(maybe_sub, maybe_sup)?;
                } else {
                    for (l_maybe_sub, r_maybe_sup) in lps.iter().zip(rps.iter()) {
                        self.sub_unify_tp(l_maybe_sub, r_maybe_sup, None, false)?;
                    }
                }
            }
            (Structural(sub), Structural(sup)) => {
                self.sub_unify(sub, sup)?;
            }
            (Guard(sub), Guard(sup)) => {
                self.sub_unify(&sub.to, &sup.to)?;
            }
            (sub, Structural(sup)) => {
                let sub_fields = self.ctx.fields(sub);
                for (sup_field, sup_ty) in self.ctx.fields(sup) {
                    if let Some((_, sub_ty)) = sub_fields.get_key_value(&sup_field) {
                        self.sub_unify(sub_ty, &sup_ty)?;
                    } else {
                        return Err(TyCheckErrors::from(TyCheckError::no_attr_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            self.loc.loc(),
                            self.ctx.caused_by(),
                            sub,
                            &sup_field.symbol,
                            self.ctx.get_similar_attr(sub, &sup_field.symbol),
                        )));
                    }
                }
            }
            // (X or Y) <: Z is valid when X <: Z and Y <: Z
            (Or(l, r), _) => {
                self.sub_unify(l, maybe_sup)?;
                self.sub_unify(r, maybe_sup)?;
            }
            // X <: (Y and Z) is valid when X <: Y and X <: Z
            (_, And(l, r)) => {
                self.sub_unify(maybe_sub, l)?;
                self.sub_unify(maybe_sub, r)?;
            }
            // (X and Y) <: Z is valid when X <: Z or Y <: Z
            (And(l, r), _) => {
                if self.ctx.subtype_of(l, maybe_sup) {
                    self.sub_unify(l, maybe_sup)?;
                } else {
                    self.sub_unify(r, maybe_sup)?;
                }
            }
            // X <: (Y or Z) is valid when X <: Y or X <: Z
            (_, Or(l, r)) => {
                if self.ctx.subtype_of(maybe_sub, l) {
                    self.sub_unify(maybe_sub, l)?;
                } else {
                    self.sub_unify(maybe_sub, r)?;
                }
            }
            (Ref(sub), Ref(sup)) => {
                self.sub_unify(sub, sup)?;
            }
            (_, Ref(t)) => {
                self.sub_unify(maybe_sub, t)?;
            }
            (RefMut { before: l, .. }, RefMut { before: r, .. }) => {
                self.sub_unify(l, r)?;
            }
            (_, RefMut { before, .. }) => {
                self.sub_unify(maybe_sub, before)?;
            }
            (_, Proj { lhs, rhs }) => {
                if let Ok(evaled) =
                    self.ctx
                        .eval_proj(*lhs.clone(), rhs.clone(), self.ctx.level, self.loc)
                {
                    if maybe_sup != &evaled {
                        self.sub_unify(maybe_sub, &evaled)?;
                    }
                }
            }
            (Proj { lhs, rhs }, _) => {
                if let Ok(evaled) =
                    self.ctx
                        .eval_proj(*lhs.clone(), rhs.clone(), self.ctx.level, self.loc)
                {
                    if maybe_sub != &evaled {
                        self.sub_unify(&evaled, maybe_sup)?;
                    }
                }
            }
            (
                _,
                ProjCall {
                    lhs,
                    attr_name,
                    args,
                },
            ) => {
                if let Some(evaled) = self
                    .ctx
                    .eval_proj_call(*lhs.clone(), attr_name.clone(), args.clone(), self.loc)
                    .ok()
                    .and_then(|tp| self.ctx.convert_tp_into_type(tp).ok())
                {
                    if maybe_sup != &evaled {
                        self.sub_unify(maybe_sub, &evaled)?;
                    }
                }
            }
            (
                ProjCall {
                    lhs,
                    attr_name,
                    args,
                },
                _,
            ) => {
                if let Some(evaled) = self
                    .ctx
                    .eval_proj_call(*lhs.clone(), attr_name.clone(), args.clone(), self.loc)
                    .ok()
                    .and_then(|tp| self.ctx.convert_tp_into_type(tp).ok())
                {
                    if maybe_sub != &evaled {
                        self.sub_unify(&evaled, maybe_sup)?;
                    }
                }
            }
            // TODO: Judgment for any number of preds
            (Refinement(sub), Refinement(sup)) => {
                // {I: Int or Str | I == 0} <: {I: Int}
                if self.ctx.subtype_of(&sub.t, &sup.t) {
                    self.sub_unify(&sub.t, &sup.t)?;
                }
                if sup.pred.as_ref() == &Predicate::TRUE {
                    self.sub_unify(&sub.t, &sup.t)?;
                    return Ok(());
                }
                self.sub_unify_pred(&sub.pred, &sup.pred)?;
            }
            // {I: Int | I >= 1} <: Nat == {I: Int | I >= 0}
            (Refinement(_), sup) => {
                let sup = sup.clone().into_refinement();
                self.sub_unify(maybe_sub, &Type::Refinement(sup))?;
            }
            (sub, Refinement(_)) => {
                if let Some(sub) = sub.to_singleton() {
                    self.sub_unify(&Type::Refinement(sub), maybe_sup)?;
                } else {
                    let sub = sub.clone().into_refinement();
                    self.sub_unify(&Type::Refinement(sub), maybe_sup)?;
                }
            }
            (Subr(_) | Record(_), Type) => {}
            (Guard(_), Bool) | (Bool, Guard(_)) => {}
            // REVIEW: correct?
            (Poly { name, .. }, Type) if &name[..] == "List" || &name[..] == "Tuple" => {}
            (Poly { .. }, _) => {
                if maybe_sub.has_no_qvar() && maybe_sup.has_no_qvar() {
                    return Ok(());
                }
                self.nominal_sub_unify(maybe_sub, maybe_sup)?;
            }
            (_, Poly { .. }) => {
                self.nominal_sub_unify(maybe_sub, maybe_sup)?;
            }
            (Subr(_), Mono(name)) if &name[..] == "Subroutine" => {}
            _ => {
                return type_feature_error!(
                    self.ctx,
                    self.loc.loc(),
                    &format!(
                        "{maybe_sub} can be a subtype of {maybe_sup}, but failed to semi-unify"
                    )
                )
            }
        }
        log!(info "sub_unified:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
        Ok(())
    }

    /// e.g. `maybe_sub: Vec, maybe_sup: Iterable T (Vec <: Iterable Int, T <: Int)`
    ///
    /// TODO: Current implementation is inefficient because coercion is performed twice with `subtype_of` in `sub_unify`
    fn nominal_sub_unify(&self, maybe_sub: &Type, maybe_sup: &Type) -> TyCheckResult<()> {
        debug_assert_ne!(maybe_sub.qual_name(), maybe_sup.qual_name());
        if let Some(sub_ctx) = self.ctx.get_nominal_type_ctx(maybe_sub) {
            let sub_def_t = &sub_ctx.typ;
            // e.g.
            // maybe_sub: Zip(Int, Str)
            // sub_def_t: Zip(T, U) ==> Zip(Int, Str)
            // super_traits: [Iterable((T, U)), ...] ==> [Iterable((Int, Str)), ...]
            let _substituter = Substituter::substitute_typarams(self.ctx, sub_def_t, maybe_sub)?;
            let sups = if self.ctx.is_class(maybe_sup) || self.ctx.is_trait(maybe_sub) {
                sub_ctx.super_classes.iter()
            } else {
                sub_ctx.super_traits.iter()
            };
            // A trait may be over-implemented.
            // Choose from the more specialized implementations,
            // but there may also be trait implementations that have no subtype relationship at all.
            // e.g. Vector <: Mul(Vector) and Mul(Nat)
            let mut compatibles = vec![];
            if sups.clone().count() == 0 {
                compatibles.push(&sub_ctx.typ);
            }
            for sup_of_sub in sups {
                if sup_of_sub.qual_name() == maybe_sup.qual_name()
                    && self.ctx.subtype_of(sup_of_sub, maybe_sup)
                {
                    if !compatibles.is_empty() {
                        let mut idx = compatibles.len();
                        for (i, comp) in compatibles.iter().enumerate() {
                            if self.ctx.subtype_of(sup_of_sub, comp) {
                                idx = i;
                                break;
                            }
                        }
                        compatibles.insert(idx, sup_of_sub);
                    } else {
                        compatibles.push(sup_of_sub);
                    }
                }
            }
            let sup_params = maybe_sup.typarams();
            'l: for sup_of_sub in compatibles {
                let _substituter = Substituter::substitute_self(sup_of_sub, maybe_sub, self.ctx);
                let sub_instance = self.ctx.instantiate_def_type(sup_of_sub)?;
                let sub_params = sub_instance.typarams();
                let variances = self
                    .ctx
                    .get_nominal_type_ctx(&sub_instance)
                    .map(|ctx| ctx.type_params_variance().into_iter().map(Some).collect())
                    .unwrap_or(vec![None; sup_params.len()]);
                let list = UndoableLinkedList::new();
                for (l_maybe_sub, r_maybe_sup) in sub_params.iter().zip(sup_params.iter()) {
                    list.push_tp(l_maybe_sub);
                    list.push_tp(r_maybe_sup);
                }
                // debug_power_assert!(variances.len(), >=, sup_params.len(), "{sub_instance} / {maybe_sup}");
                let unifier = Unifier::new(self.ctx, self.loc, Some(&list), false, None);
                for ((l_maybe_sub, r_maybe_sup), variance) in sub_params
                    .iter()
                    .zip(sup_params.iter())
                    .zip(variances.iter().chain(repeat(&None)))
                {
                    if unifier
                        .sub_unify_tp(l_maybe_sub, r_maybe_sup, *variance, false)
                        .is_err()
                    {
                        // retry with coercions
                        l_maybe_sub.coerce(Some(&list));
                        r_maybe_sup.coerce(Some(&list));
                        if unifier
                            .sub_unify_tp(l_maybe_sub, r_maybe_sup, *variance, false)
                            .is_err()
                        {
                            log!(err "failed to unify {l_maybe_sub} <: {r_maybe_sup}?");
                            continue 'l;
                        }
                    }
                }
                drop(list);
                for ((l_maybe_sub, r_maybe_sup), variance) in sub_params
                    .iter()
                    .zip(sup_params.iter())
                    .zip(variances.into_iter().chain(repeat(None)))
                {
                    self.sub_unify_tp(l_maybe_sub, r_maybe_sup, variance, false)?;
                }
                return Ok(());
            }
            log!(err "no compatible supertype found: {maybe_sub} <: {maybe_sup}");
        }
        Err(TyCheckErrors::from(TyCheckError::unification_error(
            self.ctx.cfg.input.clone(),
            line!() as usize,
            maybe_sub,
            maybe_sup,
            self.loc.loc(),
            self.ctx.caused_by(),
        )))
    }

    /// Unify two types into a single type based on the subtype relation.
    ///
    /// Error if they can't unify without upcasting both types (derefining is allowed) or using Or types
    /// ```erg
    /// unify(Int, Nat) == Some(Int)
    /// unify(Int, Str) == None
    /// unify({1.2}, Nat) == Some(Float)
    /// unify(Nat, Int!) == Some(Int)
    /// unify(Eq, Int) == None
    /// unify(Int or Str, Int) == Some(Int or Str)
    /// unify(Int or Str, NoneType) == None
    /// ```
    fn unify(&self, lhs: &Type, rhs: &Type) -> Option<Type> {
        match (lhs, rhs) {
            (Type::Or(l, r), other) | (other, Type::Or(l, r)) => {
                if let Some(t) = self.unify(l, other) {
                    return self.unify(&t, l);
                } else if let Some(t) = self.unify(r, other) {
                    return self.unify(&t, l);
                }
                return None;
            }
            (Type::FreeVar(fv), _) if fv.is_linked() => return self.unify(fv.unsafe_crack(), rhs),
            (_, Type::FreeVar(fv)) if fv.is_linked() => return self.unify(lhs, fv.unsafe_crack()),
            // TODO: unify(?T, ?U) ?
            (Type::FreeVar(_), Type::FreeVar(_)) => {}
            (Type::FreeVar(fv), _) if fv.constraint_is_sandwiched() => {
                let sub = fv.get_sub()?;
                return self.unify(&sub, rhs);
            }
            (_, Type::FreeVar(fv)) if fv.constraint_is_sandwiched() => {
                let sub = fv.get_sub()?;
                return self.unify(lhs, &sub);
            }
            (Type::Refinement(lhs), Type::Refinement(rhs)) => {
                if let Some(_union) = self.unify(&lhs.t, &rhs.t) {
                    return Some(self.ctx.union_refinement(lhs, rhs).into());
                }
            }
            _ => {}
        }
        let l_sups = self.ctx.get_super_classes(lhs)?;
        let r_sups = self.ctx.get_super_classes(rhs)?;
        for l_sup in l_sups {
            if self.ctx.supertype_of(&l_sup, &Obj) {
                continue;
            }
            for r_sup in r_sups.clone() {
                if self.ctx.supertype_of(&r_sup, &Obj) {
                    continue;
                }
                if let Some(t) = self.ctx.max(&l_sup, &r_sup).either() {
                    return Some(t.clone());
                }
            }
        }
        None
    }
}

impl Context {
    pub(crate) fn occur(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, None, false, None);
        unifier.occur(maybe_sub, maybe_sup)
    }

    pub(crate) fn sub_unify_tp(
        &self,
        maybe_sub: &TyParam,
        maybe_sup: &TyParam,
        variance: Option<Variance>,
        loc: &impl Locational,
        is_structural: bool,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, None, false, None);
        unifier.sub_unify_tp(maybe_sub, maybe_sup, variance, is_structural)
    }

    /// Use `undoable_sub_unify` to temporarily impose type constraints.
    pub(crate) fn sub_unify(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, None, false, param_name.cloned());
        unifier.sub_unify(maybe_sub, maybe_sup)
    }

    pub(crate) fn sub_unify_with_coercion(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, None, false, param_name.cloned());
        unifier.sub_unify(maybe_sub, maybe_sup).or_else(|err| {
            log!(err "{err}");
            maybe_sub.coerce(unifier.undoable);
            maybe_sub.coerce(unifier.undoable);
            let maybe_sub = self
                .eval_t_params(maybe_sub.clone(), self.level, loc)
                .map_err(|(_, errs)| errs)?;
            let maybe_sup = self
                .eval_t_params(maybe_sup.clone(), self.level, loc)
                .map_err(|(_, errs)| errs)?;
            unifier.sub_unify(&maybe_sub, &maybe_sup)
        })
    }

    /// This will rewrite generalized type variables.
    pub(crate) fn force_sub_unify(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, None, true, param_name.cloned());
        unifier.sub_unify(maybe_sub, maybe_sup)
    }

    pub(crate) fn undoable_sub_unify(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
        list: &UndoableLinkedList,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, Some(list), false, param_name.cloned());
        unifier.sub_unify(maybe_sub, maybe_sup)
    }

    pub(crate) fn unify(&self, lhs: &Type, rhs: &Type) -> Option<Type> {
        let unifier = Unifier::new(self, &(), None, false, None);
        unifier.unify(lhs, rhs)
    }
}

#[cfg(test)]
mod test {
    use crate::context::unify::{mono_q, subtypeof, type_q};
    use crate::fn_t;

    use super::Type;
    use Type::*;

    #[test]
    fn test_occur() {
        let ctx = super::Context::default();
        let unifier = super::Unifier::new(&ctx, &(), None, false, None);

        assert!(unifier.occur(&Type, &Type).is_ok());
        let t = type_q("T");
        assert!(unifier.occur(&t, &t).is_ok());
        let or_t = t.clone() | Type;
        let or2_t = Type | t.clone();
        assert!(unifier.occur(&Int, &(Int | Str)).is_ok());
        assert!(unifier.occur(&t, &or_t).is_err());
        assert!(unifier.occur(&or_t, &or2_t).is_ok());
        let subr_t = fn_t!(Type => t.clone());
        assert!(unifier.occur(&t, &subr_t).is_err());
        assert!(unifier.occur(&subr_t, &subr_t).is_ok());

        let u = mono_q("U", subtypeof(t.clone() | Int));
        assert!(unifier.occur(&u, &t).is_ok());
    }
}
