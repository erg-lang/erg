//! provides type variable related operations
use std::iter::repeat;
use std::mem;
use std::option::Option;

use erg_common::consts::DEBUG_MODE;
use erg_common::fresh::FRESH_GEN;
use erg_common::traits::Locational;
#[allow(unused_imports)]
use erg_common::{dict, fmt_vec, fn_name, log};
use erg_common::{get_hash, set_recursion_limit, Str};

use crate::context::eval::Substituter;
use crate::context::instantiate::TyVarCache;
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
            param_name,
        }
    }
}

impl<L: Locational> Unifier<'_, '_, '_, L> {
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
    /// occur(?T, ?T or Int) ==> Error
    /// ```
    fn occur(&self, maybe_sub: &Type, maybe_super: &Type) -> TyCheckResult<()> {
        if maybe_sub == maybe_super {
            return Ok(());
        } else if let Some(sup) = maybe_sub.get_super() {
            if &sup == maybe_super {
                return Ok(());
            }
        } else if let Some(sub) = maybe_super.get_sub() {
            if &sub == maybe_sub {
                return Ok(());
            }
        }
        match (maybe_sub, maybe_super) {
            (FreeVar(fv), _) if fv.is_linked() => self.occur(&fv.unwrap_linked(), maybe_super),
            (_, FreeVar(fv)) if fv.is_linked() => self.occur(maybe_sub, &fv.unwrap_linked()),
            (Subr(subr), FreeVar(fv)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(default_t, maybe_super)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(var_params.typ(), maybe_super)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(non_default_t, maybe_super)?;
                }
                self.occur_inner(&subr.return_t, maybe_super)?;
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
            // FIXME: This is not correct, we must visit all permutations of the types
            (And(l, _), And(r, _)) if l.len() == r.len() => {
                let mut r = r.clone();
                for _ in 0..r.len() {
                    if l.iter()
                        .zip(r.iter())
                        .all(|(l, r)| self.occur_inner(l, r).is_ok())
                    {
                        return Ok(());
                    }
                    r.rotate_left(1);
                }
                Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                    self.ctx.cfg.input.clone(),
                    line!() as usize,
                    maybe_sub,
                    maybe_super,
                    self.loc.loc(),
                    self.ctx.caused_by(),
                )))
            }
            (Or(l), Or(r)) if l.len() == r.len() => {
                let l = l.to_vec();
                let mut r = r.to_vec();
                for _ in 0..r.len() {
                    if l.iter()
                        .zip(r.iter())
                        .all(|(l, r)| self.occur_inner(l, r).is_ok())
                    {
                        return Ok(());
                    }
                    r.rotate_left(1);
                }
                Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                    self.ctx.cfg.input.clone(),
                    line!() as usize,
                    maybe_sub,
                    maybe_super,
                    self.loc.loc(),
                    self.ctx.caused_by(),
                )))
            }
            (lhs, And(tys, _)) => {
                for ty in tys.iter() {
                    self.occur_inner(lhs, ty)?;
                }
                Ok(())
            }
            (lhs, Or(tys)) => {
                for ty in tys.iter() {
                    self.occur_inner(lhs, ty)?;
                }
                Ok(())
            }
            (And(tys, _), rhs) => {
                for ty in tys.iter() {
                    self.occur_inner(ty, rhs)?;
                }
                Ok(())
            }
            (Or(tys), rhs) => {
                for ty in tys.iter() {
                    self.occur_inner(ty, rhs)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn occur_inner(&self, maybe_sub: &Type, maybe_sup: &Type) -> TyCheckResult<()> {
        match (maybe_sub, maybe_sup) {
            (FreeVar(fv), _) if fv.is_linked() => self.occur_inner(&fv.unwrap_linked(), maybe_sup),
            (_, FreeVar(fv)) if fv.is_linked() => self.occur_inner(maybe_sub, &fv.unwrap_linked()),
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
            (lhs, And(tys, _)) => {
                for ty in tys.iter() {
                    self.occur_inner(lhs, ty)?;
                }
                Ok(())
            }
            (lhs, Or(tys)) => {
                for ty in tys.iter() {
                    self.occur_inner(lhs, ty)?;
                }
                Ok(())
            }
            (And(tys, _), rhs) => {
                for ty in tys.iter() {
                    self.occur_inner(ty, rhs)?;
                }
                Ok(())
            }
            (Or(tys), rhs) => {
                for ty in tys.iter() {
                    self.occur_inner(ty, rhs)?;
                }
                Ok(())
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
                    // contravariant
                    self.sub_unify_value(sup_key, sub_key)?;
                    let sub_value = sub.values().next().unwrap();
                    let sup_value = sup.values().next().unwrap();
                    self.sub_unify_value(sub_value, sup_value)?;
                    return Ok(());
                }
                for (sub_k, sub_v) in sub.iter() {
                    if let Some(sup_v) = sup.linear_get(sub_k) {
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
                if sub.len() == 1 && sup.len() == 1 {
                    let sub_key = sub.keys().next().unwrap();
                    let sup_key = sup.keys().next().unwrap();
                    // contravariant
                    self.sub_unify_tp(sup_key, sub_key, _variance, allow_divergence)?;
                    let sub_value = sub.values().next().unwrap();
                    let sup_value = sup.values().next().unwrap();
                    self.sub_unify_tp(sub_value, sup_value, _variance, allow_divergence)?;
                    return Ok(());
                }
                for (sub_k, sub_v) in sub.iter() {
                    if let Some(sup_v) = sup
                        .linear_get(sub_k)
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
            (Pred::And(l1, r1), Pred::And(l2, r2)) => {
                match (self.sub_unify_pred(l1, l2), self.sub_unify_pred(r1, r2)) {
                    (Ok(()), Ok(())) => Ok(()),
                    (Ok(()), Err(e)) | (Err(e), Ok(())) | (Err(e), Err(_)) => Err(e),
                }
            }
            (Pred::Or(l_preds), Pred::Or(r_preds)) => {
                let mut l_preds_ = l_preds.clone();
                let mut r_preds_ = r_preds.clone();
                for l_pred in l_preds {
                    if r_preds_.linear_remove(l_pred) {
                        l_preds_.linear_remove(l_pred);
                    }
                }
                for l_pred in l_preds_.iter() {
                    for r_pred in r_preds_.iter() {
                        if self.ctx.is_sub_pred_of(l_pred, r_pred) {
                            self.sub_unify_pred(l_pred, r_pred)?;
                            continue;
                        }
                    }
                }
                Ok(())
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
    fn sub_unify(&self, maybe_sub: &Type, maybe_super: &Type) -> TyCheckResult<()> {
        log!(info "trying {}sub_unify:\nmaybe_sub: {maybe_sub}\nmaybe_super: {maybe_super}", self.undoable.map_or("", |_| "undoable_"));
        set_recursion_limit!(
            panic,
            "recursion limit exceed: sub_unify({maybe_sub}, {maybe_super})",
            128
        );
        // In this case, there is no new information to be gained
        if maybe_sub == &Type::Never
            || maybe_super == &Type::Obj
            || maybe_super.addr_eq(maybe_sub)
            || (maybe_sub.has_no_unbound_var()
                && maybe_super.has_no_unbound_var()
                && maybe_sub == maybe_super)
        {
            log!(info "no-op:\nmaybe_sub: {maybe_sub}\nmaybe_super: {maybe_super}");
            return Ok(());
        }
        // API definition was failed and inspection is useless after this
        if maybe_sub == &Type::Failure || maybe_super == &Type::Failure {
            log!(info "no-op:\nmaybe_sub: {maybe_sub}\nmaybe_super: {maybe_super}");
            return Ok(());
        }
        self.occur(maybe_sub, maybe_super)
            .inspect_err(|_e| log!(err "occur error: {maybe_sub} / {maybe_super}"))?;
        let maybe_sub_is_sub = self.ctx.subtype_of(maybe_sub, maybe_super);
        if !maybe_sub_is_sub {
            log!(err "{maybe_sub} !<: {maybe_super}");
            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                self.ctx.cfg.input.clone(),
                line!() as usize,
                self.loc.loc(),
                self.ctx.caused_by(),
                self.param_name.as_ref().unwrap_or(&Str::ever("_")),
                None,
                maybe_super,
                maybe_sub,
                self.ctx.get_candidates(maybe_sub),
                self.ctx
                    .get_simple_type_mismatch_hint(maybe_super, maybe_sub),
            )));
        } else if maybe_sub.has_no_unbound_var() && maybe_super.has_no_unbound_var() {
            log!(info "no-op:\nmaybe_sub: {maybe_sub}\nmaybe_super: {maybe_super}");
            return Ok(());
        }
        match (maybe_sub, maybe_super) {
            (FreeVar(sub_fv), _) if sub_fv.is_linked() => {
                self.sub_unify(&sub_fv.unwrap_linked(), maybe_super)?;
            }
            (_, FreeVar(super_fv)) if super_fv.is_linked() => {
                self.sub_unify(maybe_sub, &super_fv.unwrap_linked())?;
            }
            // lfv's sup can be shrunk (take min), rfv's sub can be expanded (take union)
            // lfvのsupは縮小可能(minを取る)、rfvのsubは拡大可能(unionを取る)
            // sub_unify(?T[0](:> Never, <: Int), ?U[1](:> Never, <: Nat)): (/* ?U[1] --> ?T[0](:> Never, <: Nat))
            // sub_unify(?T[1](:> Never, <: Nat), ?U[0](:> Never, <: Int)): (/* ?T[1] --> ?U[0](:> Never, <: Nat))
            // sub_unify(?T[0](:> Never, <: Str), ?U[1](:> Never, <: Int)): (?T[0](:> Never, <: Str and Int) --> Error!)
            // sub_unify(?T[0](:> Int, <: Add()), ?U[1](:> Never, <: Mul())): (?T[0](:> Int, <: Add() and Mul()))
            // sub_unify(?T[0](:> Str, <: Obj), ?U[1](:> Int, <: Obj)): (/* ?U[1] --> ?T[0](:> Str or Int) */)
            (FreeVar(sub_fv), FreeVar(super_fv))
                if sub_fv.constraint_is_sandwiched() && super_fv.constraint_is_sandwiched() =>
            {
                if !self.change_generalized
                    && (sub_fv.is_generalized() || super_fv.is_generalized())
                {
                    log!(info "generalized:\nmaybe_sub: {maybe_sub}\nmaybe_super: {maybe_super}");
                    return Ok(());
                }
                let (lsub, lsup) = sub_fv.get_subsup().unwrap();
                let (rsub, rsup) = super_fv.get_subsup().unwrap();
                // sub: ?T(:> ?U)
                // sup: ?U
                // => ?T == ?U
                if &lsub == maybe_super {
                    maybe_sub.link(maybe_super, self.undoable);
                    return Ok(());
                } else if &rsup == maybe_sub {
                    maybe_super.link(maybe_sub, self.undoable);
                    return Ok(());
                }
                // ?T(<: Add(?T))
                // ?U(:> {1, 2}, <: Add(?U)) ==> {1, 2}
                super_fv.dummy_link();
                sub_fv.dummy_link();
                if lsub.qual_name() == rsub.qual_name() {
                    for (lps, rps) in lsub.typarams().iter().zip(rsub.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, false).inspect_err(|_e| {
                            super_fv.undo();
                            sub_fv.undo();
                        })?;
                    }
                }
                // lsup: Add(?X(:> Int)), rsup: Add(?Y(:> Nat))
                //   => lsup: Add(?X(:> Int)), rsup: Add((?X(:> Int)))
                if lsup.qual_name() == rsup.qual_name() {
                    for (lps, rps) in lsup.typarams().iter().zip(rsup.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, false).inspect_err(|_e| {
                            super_fv.undo();
                            sub_fv.undo();
                        })?;
                    }
                }
                super_fv.undo();
                sub_fv.undo();
                let sup_intersec = self.ctx.intersection(&lsup, &rsup);
                if sup_intersec == Type::Never {
                    return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_super,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                    )));
                }
                let sub_union = self.ctx.union(&lsub, &rsub);
                if lsub.union_size().max(rsub.union_size()) < sub_union.union_size() {
                    let (l, r) = sub_union.union_pair().unwrap_or((lsub, rsub.clone()));
                    let unified = self.unify(&l, &r);
                    if unified.is_none() {
                        let maybe_sub = self.ctx.readable_type(maybe_sub.clone());
                        let union = self.ctx.readable_type(sub_union);
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
                if !(sup_intersec.is_recursive() && rsup.is_recursive()) {
                    self.sub_unify(&sup_intersec, &rsup)?;
                }
                self.sub_unify(&rsub, &sub_union)?;
                // self.sub_unify(&intersec, &lsup, loc, param_name)?;
                // self.sub_unify(&lsub, &union, loc, param_name)?;
                match sub_fv
                    .level()
                    .unwrap_or(GENERIC_LEVEL)
                    .cmp(&super_fv.level().unwrap_or(GENERIC_LEVEL))
                {
                    std::cmp::Ordering::Less => {
                        if super_fv.level().unwrap_or(GENERIC_LEVEL) == GENERIC_LEVEL {
                            maybe_super.update_tyvar(sub_union, sup_intersec, self.undoable, false);
                            maybe_sub.link(maybe_super, self.undoable);
                        } else {
                            maybe_sub.update_tyvar(sub_union, sup_intersec, self.undoable, false);
                            maybe_super.link(maybe_sub, self.undoable);
                        }
                    }
                    std::cmp::Ordering::Greater => {
                        if sub_fv.level().unwrap_or(GENERIC_LEVEL) == GENERIC_LEVEL {
                            maybe_sub.update_tyvar(sub_union, sup_intersec, self.undoable, false);
                            maybe_super.link(maybe_sub, self.undoable);
                        } else {
                            maybe_super.update_tyvar(sub_union, sup_intersec, self.undoable, false);
                            maybe_sub.link(maybe_super, self.undoable);
                        }
                    }
                    std::cmp::Ordering::Equal => {
                        // choose named one
                        if super_fv.is_named_unbound() {
                            maybe_super.update_tyvar(sub_union, sup_intersec, self.undoable, false);
                            maybe_sub.link(maybe_super, self.undoable);
                        } else {
                            maybe_sub.update_tyvar(sub_union, sup_intersec, self.undoable, false);
                            maybe_super.link(maybe_sub, self.undoable);
                        }
                    }
                }
            }
            (FreeVar(sub_fv), FreeVar(super_fv))
                if sub_fv.constraint_is_sandwiched() && super_fv.constraint_is_typeof() =>
            {
                if !self.change_generalized
                    && (sub_fv.is_generalized() || super_fv.is_generalized())
                {
                    log!(info "generalized:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_super}");
                    return Ok(());
                }
                let (lsub, lsup) = sub_fv.get_subsup().unwrap();
                // sub: ?T(:> ?U(: {Str, Int}))
                // sup: ?U(: {Str, Int})
                // => ?T == ?U
                if &lsub == maybe_super {
                    maybe_sub.link(maybe_super, self.undoable);
                    return Ok(());
                }
                let rty = super_fv.get_type().unwrap();
                let Some(rtys) = rty.refinement_values() else {
                    todo!("{rty}");
                };
                // sub: ?T(:> Nat)
                // sup: ?U(: {Str, Int})
                // => ?T(:> Nat, <: Int)
                for tp in rtys {
                    let Ok(ty) = self.ctx.convert_tp_into_type(tp.clone()) else {
                        todo!("{tp}");
                    };
                    if self.ctx.subtype_of(&lsub, &ty) {
                        let intersec = self.ctx.intersection(&lsup, &ty);
                        maybe_sub.update_super(intersec, self.undoable, true);
                        return Ok(());
                    }
                }
                // REVIEW: unreachable?
            }
            (
                Bounded {
                    sub: lsub,
                    sup: lsuper,
                },
                FreeVar(super_fv),
            ) if super_fv.constraint_is_sandwiched() => {
                if !self.change_generalized && super_fv.is_generalized() {
                    log!(info "generalized:\nmaybe_sub: {maybe_sub}\nmaybe_super: {maybe_super}");
                    return Ok(());
                }
                let (rsub, rsuper) = super_fv.get_subsup().unwrap();
                // ?T(<: Add(?T))
                // ?U(:> {1, 2}, <: Add(?U)) ==> {1, 2}
                super_fv.dummy_link();
                if lsub.qual_name() == rsub.qual_name() {
                    for (lps, rps) in lsub.typarams().iter().zip(rsub.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, false)
                            .inspect_err(|_e| super_fv.undo())?;
                    }
                }
                // lsup: Add(?X(:> Int)), rsup: Add(?Y(:> Nat))
                //   => lsup: Add(?X(:> Int)), rsup: Add((?X(:> Int)))
                if lsuper.qual_name() == rsuper.qual_name() {
                    for (lps, rps) in lsuper.typarams().iter().zip(rsuper.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, false)
                            .inspect_err(|_e| super_fv.undo())?;
                    }
                }
                super_fv.undo();
                let intersec = self.ctx.intersection(lsuper, &rsuper);
                if intersec == Type::Never {
                    return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_super,
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
                self.sub_unify(&intersec, &rsuper)?;
                self.sub_unify(&rsub, &union)?;
                // self.sub_unify(&intersec, &lsup, loc, param_name)?;
                // self.sub_unify(&lsub, &union, loc, param_name)?;
                maybe_super.update_tyvar(union, intersec, self.undoable, false);
            }
            // TODO: Preferentially compare same-structure types (e.g. K(?T) <: K(?U))
            (And(ltys, _), And(rtys, _)) => {
                let mut ltys_ = ltys.clone();
                let mut rtys_ = rtys.clone();
                // Show and EqHash and T <: Eq and Show and Ord
                // => EqHash and T <: Eq and Ord
                for lty in ltys.iter() {
                    if let Some(idx) = rtys_.iter().position(|r| r == lty) {
                        rtys_.remove(idx);
                        let idx = ltys_.iter().position(|l| l == lty).unwrap();
                        ltys_.remove(idx);
                    }
                }
                // EqHash and T <: Eq and Ord
                for lty in ltys_.iter() {
                    // lty: EqHash
                    // rty: Eq, Ord
                    for rty in rtys_.iter() {
                        if self.ctx.subtype_of(lty, rty) {
                            self.sub_unify(lty, rty)?;
                            continue;
                        }
                    }
                }
            }
            // TODO: Preferentially compare same-structure types (e.g. K(?T) <: K(?U))
            // Nat or Str or NoneType <: NoneType or ?T or Int
            // => Str <: ?T
            // (Int or ?T) <: (?U or Int)
            // OK: (Int <: Int); (?T <: ?U)
            // NG: (Int <: ?U); (?T <: Int)
            (Or(ltys), Or(rtys)) => {
                let mut ltys_ = ltys.clone();
                let mut rtys_ = rtys.clone();
                // Nat or T or Str <: Str or Int or NoneType
                // => Nat or T <: Int or NoneType
                for lty in ltys {
                    if rtys_.linear_remove(lty) {
                        ltys_.linear_remove(lty);
                    }
                }
                // Nat or T <: Int or NoneType
                for lty in ltys_.iter() {
                    // lty: Nat
                    // rty: Int, NoneType
                    for rty in rtys_.iter() {
                        if self.ctx.subtype_of(lty, rty) {
                            self.sub_unify(lty, rty)?;
                            continue;
                        }
                    }
                }
            }
            // NG: Nat <: ?T or Int ==> Nat or Int (?T = Nat)
            // OK: Nat <: ?T or Int ==> ?T or Int
            (sub, Or(tys))
                if !sub.is_unbound_var()
                    && tys
                        .iter()
                        .any(|ty| !ty.is_unbound_var() && self.ctx.subtype_of(sub, ty)) => {}
            // e.g. Structural({ .method = (self: T) -> Int })/T
            (Structural(sub), FreeVar(sup_fv))
                if sup_fv.is_unbound() && sub.contains_tvar(sup_fv) => {}
            (_, FreeVar(sup_fv)) if !self.change_generalized && sup_fv.is_generalized() => {}
            (_, FreeVar(super_fv)) if super_fv.is_unbound() => {
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
                // * sub_unify(K(Int, 1), ?T(:> K(?A, ?N))) ==> ?A(:> Int), ?N == 1
                if let Type::Refinement(refine) = maybe_sub {
                    if refine.t.addr_eq(maybe_super) {
                        return Ok(());
                    }
                }
                if let Some((sub, mut supe)) = super_fv.get_subsup() {
                    if !supe.is_recursive() {
                        self.sub_unify(maybe_sub, &supe)?;
                    }
                    let mut new_sub = self.ctx.union(maybe_sub, &sub);
                    if !sub.is_recursive()
                        && maybe_sub.qual_name() == sub.qual_name()
                        && new_sub.has_unbound_var()
                    {
                        let list = UndoableLinkedList::new();
                        if self
                            .ctx
                            .undoable_sub_unify(maybe_sub, &sub, &(), &list, None)
                            .is_ok()
                            && !maybe_sub.is_recursive()
                            && !sub.is_recursive()
                        {
                            drop(list);
                            self.sub_unify(maybe_sub, &sub)?;
                        }
                    }
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
                            new_sub = unified;
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
                    if supe.contains_union(&new_sub) {
                        maybe_super.link(&new_sub, self.undoable); // Bool <: ?T <: Bool or Y ==> ?T == Bool
                    } else {
                        maybe_super.update_tyvar(
                            new_sub,
                            mem::take(&mut supe),
                            self.undoable,
                            true,
                        );
                    }
                }
                // sub_unify(Nat, ?T(: Type)): (/* ?T(:> Nat) */)
                else if let Some(ty) = super_fv.get_type() {
                    if self.ctx.supertype_of(&Type, &ty) {
                        let constr = Constraint::new_supertype_of(maybe_sub.clone());
                        maybe_super.update_constraint(constr, self.undoable, true);
                    } else {
                        // ?T: GenericDict
                        // todo!("{maybe_sub} <: {maybe_sup}")
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
                        self.sub_unify(sub_ty, &sup_ty)
                            .inspect_err(|_e| sub_fv.undo())?;
                    } else if !self.ctx.subtype_of(&sub, &Never) {
                        sub_fv.undo();
                        let sub_hash = get_hash(maybe_sub);
                        maybe_sub.coerce(self.undoable);
                        if get_hash(maybe_sub) != sub_hash {
                            return self.sub_unify(maybe_sub, maybe_super);
                        }
                    } else {
                        // e.g. ?T / Structural({ .method = (self: ?T) -> Int })
                        let constr = Constraint::new_sandwiched(
                            sub.clone(),
                            self.ctx.intersection(&sup, maybe_super),
                        );
                        sub_fv.undo();
                        maybe_sub.update_constraint(constr, None, false);
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
                if let Some((mut sub, supe)) = sub_fv.get_subsup() {
                    if supe.is_structural() {
                        return Ok(());
                    }
                    let sub = mem::take(&mut sub);
                    // min(L, R) != L and R
                    let new_super = if let Some(new_sup) = self.ctx.min(&supe, maybe_super).either()
                    {
                        new_sup.clone()
                    } else {
                        self.ctx.intersection(&supe, maybe_super)
                    };
                    if !maybe_sub.is_recursive() && (&sub != maybe_sub || &new_super != maybe_super)
                    {
                        self.sub_unify(&sub, &new_super)?;
                    }
                    // ?T(:> Int, <: Int) ==> ?T == Int
                    // ?T(:> List(Int, 3), <: List(?T, ?N)) ==> ?T == List(Int, 3)
                    // ?T(:> List(Int, 3), <: Indexable(?K, ?V)) ==> ?T(:> List(Int, 3), <: Indexable(0..2, Int))
                    if !sub.is_refinement()
                        && new_super.qual_name() == sub.qual_name()
                        && !new_super.is_unbound_var()
                        && !sub.is_unbound_var()
                    {
                        maybe_sub.link(&sub, self.undoable);
                    } else {
                        maybe_sub.update_tyvar(sub, new_super, self.undoable, true);
                    }
                }
                // sub_unify(?T(: Type), Int): (?T(<: Int))
                else if let Some(ty) = sub_fv.get_type() {
                    if self.ctx.supertype_of(&Type, &ty) {
                        let constr = Constraint::new_subtype_of(maybe_super.clone());
                        maybe_sub.update_constraint(constr, self.undoable, true);
                    } else {
                        // ?T: GenericDict
                        // todo!("{maybe_sub} <: {maybe_sup}")
                    }
                }
            }
            (Record(sub_rec), Record(super_rec)) => {
                for (k, l) in sub_rec.iter() {
                    if let Some(r) = super_rec.get(k) {
                        self.sub_unify(l, r)?;
                    } else {
                        return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            maybe_sub,
                            maybe_super,
                            self.loc.loc(),
                            self.ctx.caused_by(),
                        )));
                    }
                }
            }
            (NamedTuple(sub_tup), NamedTuple(super_tup)) => {
                for ((_, lt), (_, rt)) in sub_tup.iter().zip(super_tup.iter()) {
                    self.sub_unify(lt, rt)?;
                }
            }
            (Subr(sub_subr), Subr(super_subr)) => {
                // (Int, *Int) -> ... <: (T, U, V) -> ...
                if let Some(sub_var) = sub_subr.var_params.as_deref() {
                    sub_subr
                        .non_default_params
                        .iter()
                        .chain(repeat(sub_var))
                        .zip(super_subr.non_default_params.iter())
                        .try_for_each(|(sub, sup)| {
                            // contravariant
                            self.sub_unify(sup.typ(), sub.typ())
                        })?;
                } else {
                    // (self: Self, Int) -> ... <: T -> ...
                    let sub_params = if !super_subr.is_method() && sub_subr.is_method() {
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
                        .zip(super_subr.non_default_params.iter())
                        .try_for_each(|(sub, sup)| {
                            // contravariant
                            self.sub_unify(sup.typ(), sub.typ())
                        })?;
                }
                sub_subr
                    .var_params
                    .iter()
                    .zip(super_subr.var_params.iter())
                    .try_for_each(|(sub, sup)| {
                        // contravariant
                        self.sub_unify(sup.typ(), sub.typ())
                    })?;
                for super_pt in super_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == super_pt.name())
                    {
                        // contravariant
                        self.sub_unify(super_pt.typ(), sub_pt.typ())?;
                    } else if let Some(sub_pt) = sub_subr.kw_var_params.as_ref() {
                        self.sub_unify(super_pt.typ(), sub_pt.typ())?;
                    } else {
                        let param_name = super_pt.name().map_or("_", |s| &s[..]);
                        let similar_param = erg_common::levenshtein::get_similar_name(
                            super_subr
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
                self.sub_unify(&sub_subr.return_t, &super_subr.return_t)?;
            }
            (Quantified(sub_subr), Subr(super_subr)) => {
                let Ok(sub_subr) = <&SubrType>::try_from(sub_subr.as_ref()) else {
                    unreachable!()
                };
                sub_subr
                    .non_default_params
                    .iter()
                    .zip(super_subr.non_default_params.iter())
                    .try_for_each(|(sub, sup)| {
                        if !self.change_generalized && sub.typ().is_generalized() {
                            Ok(())
                        }
                        // contravariant
                        else {
                            self.sub_unify(sup.typ(), sub.typ())
                        }
                    })?;
                for super_pt in super_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == super_pt.name())
                    {
                        if !self.change_generalized && sub_pt.typ().is_generalized() {
                            continue;
                        }
                        // contravariant
                        self.sub_unify(super_pt.typ(), sub_pt.typ())?;
                    } else if let Some(sub_pt) = sub_subr.kw_var_params.as_ref() {
                        self.sub_unify(super_pt.typ(), sub_pt.typ())?;
                    } else {
                        todo!("{maybe_sub} <: {maybe_super}")
                    }
                }
                if !sub_subr.return_t.is_generalized() {
                    // covariant
                    self.sub_unify(&sub_subr.return_t, &super_subr.return_t)?;
                }
            }
            (Subr(sub_subr), Quantified(super_subr)) => {
                let Ok(super_subr) = <&SubrType>::try_from(super_subr.as_ref()) else {
                    unreachable!()
                };
                sub_subr
                    .non_default_params
                    .iter()
                    .zip(super_subr.non_default_params.iter())
                    .try_for_each(|(sub, sup)| {
                        // contravariant
                        if !self.change_generalized && sup.typ().is_generalized() {
                            Ok(())
                        } else {
                            self.sub_unify(sup.typ(), sub.typ())
                        }
                    })?;
                for super_pt in super_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == super_pt.name())
                    {
                        // contravariant
                        if !self.change_generalized && super_pt.typ().is_generalized() {
                            continue;
                        }
                        self.sub_unify(super_pt.typ(), sub_pt.typ())?;
                    } else if let Some(sub_pt) = sub_subr.kw_var_params.as_ref() {
                        self.sub_unify(super_pt.typ(), sub_pt.typ())?;
                    } else {
                        todo!("{maybe_sub} <: {maybe_super}")
                    }
                }
                if !super_subr.return_t.is_generalized() {
                    // covariant
                    self.sub_unify(&sub_subr.return_t, &super_subr.return_t)?;
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
                    self.nominal_sub_unify(maybe_sub, maybe_super)?;
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
            (sub, Structural(supe)) => {
                let sub_fields = self.ctx.fields(sub);
                for (sup_field, sup_ty) in self.ctx.fields(supe) {
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
                            self.ctx.get_no_attr_hint(sub, &sup_field.symbol),
                        )));
                    }
                }
            }
            // (X or Y) <: Z is valid when X <: Z and Y <: Z
            (Or(tys), _) => {
                for ty in tys {
                    self.sub_unify(ty, maybe_super)?;
                }
            }
            // X <: (Y and Z) is valid when X <: Y and X <: Z
            (_, And(tys, _)) => {
                for ty in tys {
                    self.sub_unify(maybe_sub, ty)?;
                }
            }
            // (X and Y) <: Z is valid when X <: Z or Y <: Z
            (And(tys, _), _) => {
                for ty in tys {
                    if self.ctx.subtype_of(ty, maybe_super) {
                        return self.sub_unify(ty, maybe_super);
                    }
                }
                self.sub_unify(tys.iter().next().unwrap(), maybe_super)?;
            }
            // X <: (Y or Z) is valid when X <: Y or X <: Z
            (_, Or(tys)) => {
                for ty in tys {
                    if self.ctx.subtype_of(maybe_sub, ty) {
                        return self.sub_unify(maybe_sub, ty);
                    }
                }
                self.sub_unify(maybe_sub, tys.iter().next().unwrap())?;
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
                    if maybe_super != &evaled {
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
                        self.sub_unify(&evaled, maybe_super)?;
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
                    if maybe_super != &evaled {
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
                        self.sub_unify(&evaled, maybe_super)?;
                    }
                }
            }
            // TODO: Judgment for any number of preds
            (Refinement(sub), Refinement(supe)) => {
                // {I: Int or Str | I == 0} <: {I: Int}
                if self.ctx.subtype_of(&sub.t, &supe.t) {
                    self.sub_unify(&sub.t, &supe.t)?;
                }
                if supe.pred.as_ref() == &Predicate::TRUE {
                    self.sub_unify(&sub.t, &supe.t)?;
                    return Ok(());
                }
                self.sub_unify_pred(&sub.pred, &supe.pred)?;
            }
            // {Int} <: Obj -> Int
            (Refinement(_), Subr(_) | Quantified(_))
                if maybe_sub.is_singleton_refinement_type() => {}
            // {I: Int | I >= 1} <: Nat == {I: Int | I >= 0}
            (Refinement(_), sup) => {
                let sup = sup.clone().into_refinement();
                self.sub_unify(maybe_sub, &Type::Refinement(sup))?;
            }
            (sub, Refinement(_)) => {
                if let Some(sub) = sub.to_singleton() {
                    self.sub_unify(&Type::Refinement(sub), maybe_super)?;
                } else {
                    let sub = sub.clone().into_refinement();
                    self.sub_unify(&Type::Refinement(sub), maybe_super)?;
                }
            }
            (Subr(_) | Record(_), Type) => {}
            (Guard(_), _) | (_, Guard(_)) => {}
            // REVIEW: correct?
            (Poly { name, .. }, Type) if &name[..] == "List" || &name[..] == "Tuple" => {}
            (Poly { .. }, _) => {
                if maybe_sub.has_no_qvar() && maybe_super.has_no_qvar() {
                    return Ok(());
                }
                self.nominal_sub_unify(maybe_sub, maybe_super)?;
            }
            (_, Poly { .. }) => {
                self.nominal_sub_unify(maybe_sub, maybe_super)?;
            }
            (Subr(_), Mono(name)) if &name[..] == "Subroutine" => {}
            _ => {
                return type_feature_error!(
                    self.ctx,
                    self.loc.loc(),
                    &format!(
                        "{maybe_sub} can be a subtype of {maybe_super}, but failed to semi-unify"
                    )
                )
            }
        }
        log!(info "sub_unified:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_super}");
        Ok(())
    }

    /// e.g. `maybe_sub: Vec, maybe_sup: Iterable T (Vec <: Iterable Int, T <: Int)`
    ///
    /// TODO: Current implementation is inefficient because coercion is performed twice with `subtype_of` in `sub_unify`
    fn nominal_sub_unify(&self, maybe_sub: &Type, maybe_super: &Type) -> TyCheckResult<()> {
        debug_assert_ne!(maybe_sub.qual_name(), maybe_super.qual_name());
        if (maybe_sub.is_dict() || maybe_sub.is_dict_mut())
            && (maybe_super.is_dict() || maybe_super.is_dict_mut())
        {
            let sub_dict = maybe_sub.typarams().into_iter().next().unwrap();
            let super_dict = maybe_super.typarams().into_iter().next().unwrap();
            return self.sub_unify_tp(&sub_dict, &super_dict, None, false);
        }
        if let Some(sub_ctx) = self.ctx.get_nominal_type_ctx(maybe_sub) {
            let sub_def_t = &sub_ctx.typ;
            // e.g.
            // maybe_sub: Zip(Int, Str)
            // sub_def_t: Zip(T, U) ==> Zip(Int, Str)
            // super_traits: [Iterable((T, U)), ...] ==> [Iterable((Int, Str)), ...]
            let _sub_substituter =
                Substituter::substitute_typarams(self.ctx, sub_def_t, maybe_sub)?;
            let sups = if self.ctx.is_class(maybe_super) || self.ctx.is_trait(maybe_sub) {
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
                if sup_of_sub.qual_name() == maybe_super.qual_name()
                    && self.ctx.subtype_of(sup_of_sub, maybe_super)
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
            let super_params = maybe_super.typarams();
            'l: for sup_of_sub in compatibles {
                let _substituter = Substituter::substitute_self(sup_of_sub, maybe_sub, self.ctx);
                let _substituter2 =
                    if let Some((class, _)) = sub_ctx.get_trait_impl_types(sup_of_sub) {
                        Substituter::substitute_typarams(self.ctx, class, maybe_sub)?
                    } else {
                        None
                    };
                let sub_instance = self.ctx.instantiate_def_type(sup_of_sub)?;
                let sub_params = sub_instance.typarams();
                let variances = self
                    .ctx
                    .get_nominal_type_ctx(&sub_instance)
                    .map(|ctx| ctx.type_params_variance().into_iter().map(Some).collect())
                    .unwrap_or(vec![None; super_params.len()]);
                let list = UndoableLinkedList::new();
                for (l_maybe_sub, r_maybe_sup) in sub_params.iter().zip(super_params.iter()) {
                    list.push_tp(l_maybe_sub);
                    list.push_tp(r_maybe_sup);
                }
                // debug_power_assert!(variances.len(), >=, sup_params.len(), "{sub_instance} / {maybe_sup}");
                let unifier = Unifier::new(self.ctx, self.loc, Some(&list), false, None);
                for ((l_maybe_sub, r_maybe_sup), variance) in sub_params
                    .iter()
                    .zip(super_params.iter())
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
                    .zip(super_params.iter())
                    .zip(variances.into_iter().chain(repeat(None)))
                {
                    self.sub_unify_tp(l_maybe_sub, r_maybe_sup, variance, false)?;
                }
                return Ok(());
            }
            log!(err "no compatible supertype found: {maybe_sub} <: {maybe_super}");
        }
        Err(TyCheckErrors::from(TyCheckError::unification_error(
            self.ctx.cfg.input.clone(),
            line!() as usize,
            maybe_sub,
            maybe_super,
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
    /// unify(K(1), K(2)) == None
    /// unify(Int, ?U(<: Int) and ?T(<: Int)) == Some(?U and ?T)
    /// ```
    fn unify(&self, lhs: &Type, rhs: &Type) -> Option<Type> {
        match (lhs, rhs) {
            (Never, other) | (other, Never) => {
                return Some(other.clone());
            }
            (Or(tys), other) | (other, Or(tys)) => {
                let mut unified = Never;
                for ty in tys {
                    if let Some(t) = self.unify(ty, other) {
                        unified = self.ctx.union(&unified, &t);
                    }
                }
                if unified != Never {
                    return Some(unified);
                } else {
                    return None;
                }
            }
            (And(tys, _), other) | (other, And(tys, _)) => {
                let mut unified = Obj;
                for ty in tys {
                    if let Some(t) = self.unify(ty, other) {
                        unified = self.ctx.intersection(&unified, &t);
                    }
                }
                if unified != Obj && unified != Never {
                    return Some(unified);
                } else {
                    return None;
                }
            }
            (FreeVar(fv), _) if fv.is_linked() => return self.unify(&fv.unwrap_linked(), rhs),
            (_, FreeVar(fv)) if fv.is_linked() => return self.unify(lhs, &fv.unwrap_linked()),
            // TODO: unify(?T, ?U) ?
            (FreeVar(_), FreeVar(_)) => {}
            (FreeVar(fv), _) if fv.constraint_is_sandwiched() => {
                let sub = fv.get_sub()?;
                return self.unify(&sub, rhs);
            }
            (_, FreeVar(fv)) if fv.constraint_is_sandwiched() => {
                let sub = fv.get_sub()?;
                return self.unify(lhs, &sub);
            }
            (Refinement(lhs), Refinement(rhs)) => {
                if let Some(_union) = self.unify(&lhs.t, &rhs.t) {
                    return Some(self.ctx.union_refinement(lhs, rhs).into());
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
            ) if ln == rn && (lhs.is_dict() || lhs.is_dict_mut()) => {
                let Ok(ValueObj::Dict(l_dict)) = self.ctx.convert_tp_into_value(lps[0].clone())
                else {
                    return None;
                };
                let Ok(ValueObj::Dict(r_dict)) = self.ctx.convert_tp_into_value(rps[0].clone())
                else {
                    return None;
                };
                if l_dict.len() == 1 && r_dict.len() == 1 {
                    let l_key = self
                        .ctx
                        .convert_value_into_type(l_dict.keys().next()?.clone())
                        .ok()?;
                    let r_key = self
                        .ctx
                        .convert_value_into_type(r_dict.keys().next()?.clone())
                        .ok()?;
                    let l_value = self
                        .ctx
                        .convert_value_into_type(l_dict.values().next()?.clone())
                        .ok()?;
                    let r_value = self
                        .ctx
                        .convert_value_into_type(r_dict.values().next()?.clone())
                        .ok()?;
                    let unified_key = self.unify(&l_key, &r_key)?;
                    let unified_value = self.unify(&l_value, &r_value)?;
                    let unified_dict = TyParam::t(dict! { unified_key => unified_value }.into());
                    return Some(poly(ln.clone(), vec![unified_dict]));
                }
            }
            _ => {}
        }
        let l_sups = self.ctx.get_super_classes(lhs)?;
        let r_sups = self.ctx.get_super_classes(rhs)?;
        for l_sup in l_sups {
            if l_sup == Obj || self.ctx.is_trait(&l_sup) {
                continue;
            }
            for r_sup in r_sups.clone() {
                if r_sup == Obj || self.ctx.is_trait(&r_sup) {
                    continue;
                }
                let Ok(l_substituter) = Substituter::substitute_typarams(self.ctx, &l_sup, lhs)
                else {
                    continue;
                };
                let mut tv_cache = TyVarCache::new(self.ctx.level, self.ctx);
                let detached_l_sup = self.ctx.detach(l_sup.clone(), &mut tv_cache);
                drop(l_substituter);
                let Ok(r_substituter) = Substituter::substitute_typarams(self.ctx, &r_sup, rhs)
                else {
                    continue;
                };
                let mut tv_cache = TyVarCache::new(self.ctx.level, self.ctx);
                let detached_r_sup = self.ctx.detach(r_sup.clone(), &mut tv_cache);
                drop(r_substituter);
                if let Some(t) = self.ctx.max(&detached_l_sup, &detached_r_sup).either() {
                    for l_tp in l_sup.typarams() {
                        if l_tp.has_qvar() && t.contains_tp(&l_tp) {
                            return None;
                        }
                    }
                    for r_tp in r_sup.typarams() {
                        if r_tp.has_qvar() && t.contains_tp(&r_tp) {
                            return None;
                        }
                    }
                    debug_assert!(t.has_no_qvar(), "{t} has qvar");
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
        maybe_super: &Type,
        loc: &impl Locational,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, None, false, param_name.cloned());
        unifier.sub_unify(maybe_sub, maybe_super)
    }

    pub(crate) fn sub_unify_with_coercion(
        &self,
        maybe_sub: &Type,
        maybe_super: &Type,
        loc: &impl Locational,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, None, false, param_name.cloned());
        unifier.sub_unify(maybe_sub, maybe_super).or_else(|err| {
            log!(err "{err}");
            // don't coerce to Never
            if maybe_sub.get_sub().is_some_and(|sub| sub == Never) {
                return Err(err);
            }
            maybe_sub.coerce(unifier.undoable);
            // maybe_sup.coerce(unifier.undoable);
            let new_sub = self
                .eval_t_params(maybe_sub.clone(), self.level, loc)
                .map_err(|(_, errs)| errs)?;
            if new_sub != Never && &new_sub != maybe_sub {
                maybe_sub.link(&new_sub, unifier.undoable);
            }
            let new_super = self
                .eval_t_params(maybe_super.clone(), self.level, loc)
                .map_err(|(_, errs)| errs)?;
            unifier.sub_unify(&new_sub, &new_super)
        })
    }

    /// This will rewrite generalized type variables.
    pub(crate) fn force_sub_unify(
        &self,
        maybe_sub: &Type,
        maybe_super: &Type,
        loc: &impl Locational,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, None, true, param_name.cloned());
        unifier.sub_unify(maybe_sub, maybe_super)
    }

    pub(crate) fn undoable_sub_unify(
        &self,
        maybe_sub: &Type,
        maybe_super: &Type,
        loc: &impl Locational,
        list: &UndoableLinkedList,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        let unifier = Unifier::new(self, loc, Some(list), false, param_name.cloned());
        unifier.sub_unify(maybe_sub, maybe_super)
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
