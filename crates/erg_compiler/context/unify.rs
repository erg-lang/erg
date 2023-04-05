//! provides type variable related operations
use std::mem;
use std::option::Option;

use erg_common::fresh::fresh_varname;
use erg_common::traits::Locational;
use erg_common::Str;
#[allow(unused_imports)]
use erg_common::{fmt_vec, log};
use erg_common::{fn_name, switch_lang};

use crate::ty::constructors::*;
use crate::ty::free::{Constraint, FreeKind, HasLevel, GENERIC_LEVEL};
use crate::ty::typaram::{OpKind, TyParam};
use crate::ty::value::ValueObj;
use crate::ty::{Predicate, SubrType, Type};

use crate::context::{Context, Variance};
use crate::error::{SingleTyCheckResult, TyCheckError, TyCheckErrors, TyCheckResult};
use crate::{feature_error, type_feature_error};

use Predicate as Pred;
use Type::*;
use ValueObj::{Inf, NegInf};

impl Context {
    /// occur(?T, ?T) ==> OK
    /// occur(X -> ?T, ?T) ==> Error
    /// occur(X -> ?T, X -> ?T) ==> OK
    /// occur(?T, ?T -> X) ==> Error
    /// occur(?T, Option(?T)) ==> Error
    /// occur(?T, ?T.Output) ==> OK
    pub(crate) fn occur(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        if maybe_sub == maybe_sup {
            return Ok(());
        }
        match (maybe_sub, maybe_sup) {
            (FreeVar(fv), _) if fv.is_linked() => self.occur(&fv.crack(), maybe_sup, loc),
            (_, FreeVar(fv)) if fv.is_linked() => self.occur(maybe_sub, &fv.crack(), loc),
            (Subr(subr), FreeVar(fv)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(default_t, maybe_sup, loc)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(var_params.typ(), maybe_sup, loc)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(non_default_t, maybe_sup, loc)?;
                }
                self.occur_inner(&subr.return_t, maybe_sup, loc)?;
                Ok(())
            }
            (FreeVar(fv), Subr(subr)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(maybe_sub, default_t, loc)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(maybe_sub, var_params.typ(), loc)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(maybe_sub, non_default_t, loc)?;
                }
                self.occur_inner(maybe_sub, &subr.return_t, loc)?;
                Ok(())
            }
            (Subr(lhs), Subr(rhs)) => {
                for (lhs, rhs) in lhs
                    .default_params
                    .iter()
                    .map(|pt| pt.typ())
                    .zip(rhs.default_params.iter().map(|pt| pt.typ()))
                {
                    self.occur(lhs, rhs, loc)?;
                }
                if let Some(lhs) = lhs.var_params.as_ref() {
                    if let Some(rhs) = rhs.var_params.as_ref() {
                        self.occur(lhs.typ(), rhs.typ(), loc)?;
                    }
                }
                for (lhs, rhs) in lhs
                    .non_default_params
                    .iter()
                    .map(|pt| pt.typ())
                    .zip(rhs.non_default_params.iter().map(|pt| pt.typ()))
                {
                    self.occur(lhs, rhs, loc)?;
                }
                self.occur(&lhs.return_t, &rhs.return_t, loc)?;
                Ok(())
            }
            (Poly { params, .. }, FreeVar(fv)) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| {
                    if let TyParam::Type(t) = tp {
                        Some(t)
                    } else {
                        None
                    }
                }) {
                    self.occur_inner(param, maybe_sup, loc)?;
                }
                Ok(())
            }
            (FreeVar(fv), Poly { params, .. }) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| {
                    if let TyParam::Type(t) = tp {
                        Some(t)
                    } else {
                        None
                    }
                }) {
                    self.occur_inner(maybe_sub, param, loc)?;
                }
                Ok(())
            }
            (lhs, Or(l, r)) | (lhs, And(l, r)) => {
                self.occur_inner(lhs, l, loc)?;
                self.occur_inner(lhs, r, loc)
            }
            (Or(l, r), rhs) | (And(l, r), rhs) => {
                self.occur_inner(l, rhs, loc)?;
                self.occur_inner(r, rhs, loc)
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn occur_inner(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        match (maybe_sub, maybe_sup) {
            (FreeVar(fv), _) if fv.is_linked() => self.occur_inner(&fv.crack(), maybe_sup, loc),
            (_, FreeVar(fv)) if fv.is_linked() => self.occur_inner(maybe_sub, &fv.crack(), loc),
            (FreeVar(sub), FreeVar(sup)) => {
                if sub.is_unbound() && sup.is_unbound() && sub == sup {
                    Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        loc.loc(),
                        self.caused_by(),
                    )))
                } else {
                    Ok(())
                }
            }
            (Subr(subr), FreeVar(fv)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(default_t, maybe_sup, loc)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(var_params.typ(), maybe_sup, loc)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(non_default_t, maybe_sup, loc)?;
                }
                self.occur_inner(&subr.return_t, maybe_sup, loc)?;
                Ok(())
            }
            (FreeVar(fv), Subr(subr)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(maybe_sub, default_t, loc)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur_inner(maybe_sub, var_params.typ(), loc)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur_inner(maybe_sub, non_default_t, loc)?;
                }
                self.occur_inner(maybe_sub, &subr.return_t, loc)?;
                Ok(())
            }
            (Subr(lhs), Subr(rhs)) => {
                for (lhs, rhs) in lhs
                    .default_params
                    .iter()
                    .map(|pt| pt.typ())
                    .zip(rhs.default_params.iter().map(|pt| pt.typ()))
                {
                    self.occur_inner(lhs, rhs, loc)?;
                }
                if let Some(lhs) = lhs.var_params.as_ref() {
                    if let Some(rhs) = rhs.var_params.as_ref() {
                        self.occur_inner(lhs.typ(), rhs.typ(), loc)?;
                    }
                }
                for (lhs, rhs) in lhs
                    .non_default_params
                    .iter()
                    .map(|pt| pt.typ())
                    .zip(rhs.non_default_params.iter().map(|pt| pt.typ()))
                {
                    self.occur_inner(lhs, rhs, loc)?;
                }
                self.occur_inner(&lhs.return_t, &rhs.return_t, loc)?;
                Ok(())
            }
            (Poly { params, .. }, FreeVar(fv)) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| {
                    if let TyParam::Type(t) = tp {
                        Some(t)
                    } else {
                        None
                    }
                }) {
                    self.occur_inner(param, maybe_sup, loc)?;
                }
                Ok(())
            }
            (FreeVar(fv), Poly { params, .. }) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| {
                    if let TyParam::Type(t) = tp {
                        Some(t)
                    } else {
                        None
                    }
                }) {
                    self.occur_inner(maybe_sub, param, loc)?;
                }
                Ok(())
            }
            (lhs, Or(l, r)) | (lhs, And(l, r)) => {
                self.occur_inner(lhs, l, loc)?;
                self.occur_inner(lhs, r, loc)
            }
            (Or(l, r), rhs) | (And(l, r), rhs) => {
                self.occur_inner(l, rhs, loc)?;
                self.occur_inner(r, rhs, loc)
            }
            _ => Ok(()),
        }
    }

    /// allow_divergence = trueにすると、Num型変数と±Infの単一化を許す
    pub(crate) fn sub_unify_tp(
        &self,
        maybe_sub: &TyParam,
        maybe_sup: &TyParam,
        _variance: Option<Variance>,
        loc: &impl Locational,
        allow_divergence: bool,
    ) -> TyCheckResult<()> {
        if maybe_sub.has_no_unbound_var()
            && maybe_sup.has_no_unbound_var()
            && maybe_sub == maybe_sup
        {
            return Ok(());
        }
        match (maybe_sub, maybe_sup) {
            (TyParam::Type(maybe_sub), TyParam::Type(maybe_sup)) => {
                self.sub_unify(maybe_sub, maybe_sup, loc, None)
            }
            (TyParam::FreeVar(lfv), TyParam::FreeVar(rfv))
                if lfv.is_unbound() && rfv.is_unbound() =>
            {
                if lfv.level().unwrap() > rfv.level().unwrap() {
                    if !lfv.is_generalized() {
                        lfv.link(maybe_sup);
                    }
                } else if !rfv.is_generalized() {
                    rfv.link(maybe_sub);
                }
                Ok(())
            }
            (TyParam::FreeVar(lfv), tp) => {
                match &*lfv.borrow() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.sub_unify_tp(l, tp, _variance, loc, allow_divergence);
                    }
                    FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {}
                } // &fv is dropped
                let fv_t = lfv.constraint().unwrap().get_type().unwrap().clone(); // lfvを参照しないよいにcloneする(あとでborrow_mutするため)
                let tp_t = self.get_tp_t(tp)?;
                if self.supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if lfv.level() < Some(self.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t);
                        if self.is_sub_constraint_of(&lfv.constraint().unwrap(), &new_constraint)
                            || lfv.constraint().unwrap().get_type() == Some(&Type)
                        {
                            lfv.update_constraint(new_constraint, false);
                        }
                    } else {
                        lfv.link(tp);
                    }
                    Ok(())
                } else if allow_divergence
                    && (self.eq_tp(tp, &TyParam::value(Inf))
                        || self.eq_tp(tp, &TyParam::value(NegInf)))
                    && self.subtype_of(&fv_t, &mono("Num"))
                {
                    lfv.link(tp);
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    )))
                }
            }
            (tp, TyParam::FreeVar(rfv)) => {
                match &*rfv.borrow() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.sub_unify_tp(l, tp, _variance, loc, allow_divergence);
                    }
                    FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {}
                } // &fv is dropped
                let fv_t = rfv.constraint().unwrap().get_type().unwrap().clone(); // fvを参照しないよいにcloneする(あとでborrow_mutするため)
                let tp_t = self.get_tp_t(tp)?;
                if self.supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if rfv.level() < Some(self.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t);
                        if self.is_sub_constraint_of(&rfv.constraint().unwrap(), &new_constraint)
                            || rfv.constraint().unwrap().get_type() == Some(&Type)
                        {
                            rfv.update_constraint(new_constraint, false);
                        }
                    } else {
                        rfv.link(tp);
                    }
                    Ok(())
                } else if allow_divergence
                    && (self.eq_tp(tp, &TyParam::value(Inf))
                        || self.eq_tp(tp, &TyParam::value(NegInf)))
                    && self.subtype_of(&fv_t, &mono("Num"))
                {
                    rfv.link(tp);
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    )))
                }
            }
            (TyParam::UnaryOp { op: lop, val: lval }, TyParam::UnaryOp { op: rop, val: rval })
                if lop == rop =>
            {
                self.sub_unify_tp(lval, rval, _variance, loc, allow_divergence)
            }
            (
                TyParam::BinOp { op: lop, lhs, rhs },
                TyParam::BinOp {
                    op: rop,
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if lop == rop => {
                self.sub_unify_tp(lhs, lhs2, _variance, loc, allow_divergence)?;
                self.sub_unify_tp(rhs, rhs2, _variance, loc, allow_divergence)
            }
            (TyParam::Lambda(_l), TyParam::Lambda(_r)) => {
                todo!("{_l}/{_r}")
            }
            (l, TyParam::Erased(t)) => {
                let sub_t = self.get_tp_t(l)?;
                if self.subtype_of(&sub_t, t) {
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &sub_t,
                        t,
                        loc.loc(),
                        self.caused_by(),
                    )))
                }
            }
            (l, TyParam::Type(r)) => {
                let l = self.convert_tp_into_ty(l.clone()).map_err(|_| {
                    TyCheckError::tp_to_type_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        l,
                        loc.loc(),
                        self.caused_by(),
                    )
                })?;
                self.sub_unify(&l, r, loc, None)?;
                Ok(())
            }
            (TyParam::Type(l), r) => {
                let r = self.convert_tp_into_ty(r.clone()).map_err(|_| {
                    TyCheckError::tp_to_type_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        r,
                        loc.loc(),
                        self.caused_by(),
                    )
                })?;
                self.sub_unify(l, &r, loc, None)?;
                Ok(())
            }
            (TyParam::Array(ls), TyParam::Array(rs)) | (TyParam::Tuple(ls), TyParam::Tuple(rs)) => {
                for (l, r) in ls.iter().zip(rs.iter()) {
                    self.sub_unify_tp(l, r, _variance, loc, allow_divergence)?;
                }
                Ok(())
            }
            (TyParam::Dict(ls), TyParam::Dict(rs)) => {
                for (lk, lv) in ls.iter() {
                    if let Some(rv) = rs.get(lk) {
                        self.sub_unify_tp(lv, rv, _variance, loc, allow_divergence)?;
                    } else {
                        // TODO:
                        return Err(TyCheckErrors::from(TyCheckError::unreachable(
                            self.cfg.input.clone(),
                            fn_name!(),
                            line!(),
                        )));
                    }
                }
                Ok(())
            }
            (l, r) => type_feature_error!(self, loc.loc(), &format!("unifying {l} and {r}")),
        }
    }

    fn reunify_tp(
        &self,
        before: &TyParam,
        after: &TyParam,
        loc: &impl Locational,
    ) -> SingleTyCheckResult<()> {
        match (before, after) {
            (TyParam::Value(ValueObj::Mut(l)), TyParam::Value(ValueObj::Mut(r))) => {
                *l.borrow_mut() = r.borrow().clone();
                Ok(())
            }
            (TyParam::Value(ValueObj::Mut(l)), TyParam::Value(r)) => {
                *l.borrow_mut() = r.clone();
                Ok(())
            }
            (TyParam::Type(l), TyParam::Type(r)) => self.reunify(l, r, loc),
            (TyParam::UnaryOp { op: lop, val: lval }, TyParam::UnaryOp { op: rop, val: rval })
                if lop == rop =>
            {
                self.reunify_tp(lval, rval, loc)
            }
            (
                TyParam::BinOp { op: lop, lhs, rhs },
                TyParam::BinOp {
                    op: rop,
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if lop == rop => {
                self.reunify_tp(lhs, lhs2, loc)?;
                self.reunify_tp(rhs, rhs2, loc)
            }
            (TyParam::Lambda(_l), TyParam::Lambda(_r)) => {
                todo!("{_l}/{_r}")
            }
            (l, r) if self.eq_tp(l, r) => Ok(()),
            (l, r) => {
                type_feature_error!(error self, loc.loc(), &format!("re-unifying {l} and {r}"))
            }
        }
    }

    /// predは正規化されているとする
    fn sub_unify_pred(
        &self,
        sub_pred: &Predicate,
        sup_pred: &Predicate,
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        match (sub_pred, sup_pred) {
            (Pred::Value(_), Pred::Value(_)) | (Pred::Const(_), Pred::Const(_)) => Ok(()),
            (Pred::Equal { rhs, .. }, Pred::Equal { rhs: rhs2, .. })
            | (Pred::GreaterEqual { rhs, .. }, Pred::GreaterEqual { rhs: rhs2, .. })
            | (Pred::LessEqual { rhs, .. }, Pred::LessEqual { rhs: rhs2, .. })
            | (Pred::NotEqual { rhs, .. }, Pred::NotEqual { rhs: rhs2, .. }) => {
                self.sub_unify_tp(rhs, rhs2, None, loc, false)
            }
            (Pred::And(l1, r1), Pred::And(l2, r2)) | (Pred::Or(l1, r1), Pred::Or(l2, r2)) => {
                match (
                    self.sub_unify_pred(l1, l2, loc),
                    self.sub_unify_pred(r1, r2, loc),
                ) {
                    (Ok(()), Ok(())) => Ok(()),
                    (Ok(()), Err(e)) | (Err(e), Ok(())) | (Err(e), Err(_)) => Err(e),
                }
            }
            (Pred::Not(l), Pred::Not(r)) => self.sub_unify_pred(r, l, loc),
            // sub_unify_pred(I == M, I <= ?N(: Nat)) ==> ?N(: M..)
            (Pred::Equal { rhs, .. }, Pred::LessEqual { rhs: rhs2, .. }) => {
                self.coerce_greater_than(rhs2, rhs, loc)
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
                        self.sub_unify_tp(rhs, ge_rhs, None, loc, false)?;
                        self.sub_unify_tp(le_rhs, &TyParam::value(Inf), None, loc, true)
                    }
                    _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        sub_pred,
                        sup_pred,
                        loc.loc(),
                        self.caused_by(),
                    ))),
                }
            }
            (Pred::LessEqual { rhs, .. }, Pred::And(l, r))
            | (Pred::And(l, r), Pred::LessEqual { rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual { rhs: ge_rhs, .. }, Pred::LessEqual { rhs: le_rhs, .. })
                | (Pred::LessEqual { rhs: le_rhs, .. }, Pred::GreaterEqual { rhs: ge_rhs, .. }) => {
                    self.sub_unify_tp(rhs, le_rhs, None, loc, false)?;
                    self.sub_unify_tp(ge_rhs, &TyParam::value(NegInf), None, loc, true)
                }
                _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    sub_pred,
                    sup_pred,
                    loc.loc(),
                    self.caused_by(),
                ))),
            },
            (Pred::Equal { rhs, .. }, Pred::And(l, r))
            | (Pred::And(l, r), Pred::Equal { rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual { rhs: ge_rhs, .. }, Pred::LessEqual { rhs: le_rhs, .. })
                | (Pred::LessEqual { rhs: le_rhs, .. }, Pred::GreaterEqual { rhs: ge_rhs, .. }) => {
                    self.sub_unify_tp(rhs, le_rhs, None, loc, false)?;
                    self.sub_unify_tp(rhs, ge_rhs, None, loc, false)
                }
                _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    sub_pred,
                    sup_pred,
                    loc.loc(),
                    self.caused_by(),
                ))),
            },
            _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                self.cfg.input.clone(),
                line!() as usize,
                sub_pred,
                sup_pred,
                loc.loc(),
                self.caused_by(),
            ))),
        }
    }

    fn coerce_greater_than(
        &self,
        target: &TyParam,
        value: &TyParam,
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        match target {
            TyParam::FreeVar(fv) => {
                if let Ok(evaled) = self.eval_tp(value.clone()) {
                    let pred = Predicate::ge(fresh_varname().into(), evaled);
                    let new_type = self.type_from_pred(pred);
                    let new_constr = Constraint::new_type_of(Type::from(new_type));
                    fv.update_constraint(new_constr, false);
                }
                Ok(())
            }
            TyParam::BinOp {
                op: OpKind::Sub,
                lhs,
                rhs,
            } => {
                let value = TyParam::bin(OpKind::Add, value.clone(), *rhs.clone());
                self.coerce_greater_than(lhs, &value, loc)
            }
            _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                self.cfg.input.clone(),
                line!() as usize,
                &Pred::eq("_".into(), value.clone()),
                &Pred::le("_".into(), target.clone()),
                loc.loc(),
                self.caused_by(),
            ))),
        }
    }

    /// T: Array(Int, !0), U: Array(Int, !1)
    /// reunify(T, U):
    /// T: Array(Int, !1), U: Array(Int, !1)
    pub(crate) fn reunify(
        &self,
        before_t: &Type,
        after_t: &Type,
        loc: &impl Locational,
    ) -> SingleTyCheckResult<()> {
        match (before_t, after_t) {
            (FreeVar(fv), r) if fv.is_linked() => self.reunify(&fv.crack(), r, loc),
            (l, FreeVar(fv)) if fv.is_linked() => self.reunify(l, &fv.crack(), loc),
            (Ref(l), Ref(r)) => self.reunify(l, r, loc),
            (
                RefMut {
                    before: lbefore,
                    after: lafter,
                },
                RefMut {
                    before: rbefore,
                    after: rafter,
                },
            ) => {
                self.reunify(lbefore, rbefore, loc)?;
                match (lafter, rafter) {
                    (Some(lafter), Some(rafter)) => {
                        self.reunify(lafter, rafter, loc)?;
                    }
                    (None, None) => {}
                    _ => todo!(),
                }
                Ok(())
            }
            (Ref(l), r) => self.reunify(l, r, loc),
            // REVIEW:
            (RefMut { before, .. }, r) => self.reunify(before, r, loc),
            (l, Ref(r)) => self.reunify(l, r, loc),
            (l, RefMut { before, .. }) => self.reunify(l, before, loc),
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
                if ln != rn {
                    let before_t = poly(ln.clone(), lps.clone());
                    return Err(TyCheckError::re_unification_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &before_t,
                        after_t,
                        loc.loc(),
                        self.caused_by(),
                    ));
                }
                for (l, r) in lps.iter().zip(rps.iter()) {
                    self.reunify_tp(l, r, loc)?;
                }
                Ok(())
            }
            (l, r) if self.same_type_of(l, r) => Ok(()),
            (l, r) => Err(TyCheckError::re_unification_error(
                self.cfg.input.clone(),
                line!() as usize,
                l,
                r,
                loc.loc(),
                self.caused_by(),
            )),
        }
    }

    /// Assuming that `sub` is a subtype of `sup`, fill in the type variable to satisfy the assumption
    ///
    /// When comparing arguments and parameter, the left side (`sub`) is the argument (found) and the right side (`sup`) is the parameter (expected)
    ///
    /// The parameter type must be a supertype of the argument type
    /// ```python
    /// sub_unify({I: Int | I == 0}, ?T(<: Ord)): (/* OK */)
    /// sub_unify(Int, ?T(:> Nat)): (?T :> Int)
    /// sub_unify(Nat, ?T(:> Int)): (/* OK */)
    /// sub_unify(Nat, Add(?R)): (?R => Nat, Nat.Output => Nat)
    /// sub_unify([?T; 0], Mutate): (/* OK */)
    /// ```
    pub(crate) fn sub_unify(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        log!(info "trying sub_unify:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
        // In this case, there is no new information to be gained
        // この場合、特に新しく得られる情報はない
        if maybe_sub == &Type::Never || maybe_sup == &Type::Obj || maybe_sup == maybe_sub {
            return Ok(());
        }
        // API definition was failed and inspection is useless after this
        if maybe_sub == &Type::Failure || maybe_sup == &Type::Failure {
            return Ok(());
        }
        self.occur(maybe_sub, maybe_sup, loc)?;
        let maybe_sub_is_sub = self.subtype_of(maybe_sub, maybe_sup);
        if !maybe_sub_is_sub {
            log!(err "{maybe_sub} !<: {maybe_sup}");
            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                self.cfg.input.clone(),
                line!() as usize,
                loc.loc(),
                self.caused_by(),
                param_name.unwrap_or(&Str::ever("_")),
                None,
                maybe_sup,
                maybe_sub,
                self.get_candidates(maybe_sub),
                self.get_simple_type_mismatch_hint(maybe_sup, maybe_sub),
            )));
        } else if maybe_sub.has_no_unbound_var() && maybe_sup.has_no_unbound_var() {
            return Ok(());
        }
        match (maybe_sub, maybe_sup) {
            (FreeVar(sub_fv), _) if sub_fv.is_linked() => {
                self.sub_unify(&sub_fv.crack(), maybe_sup, loc, param_name)
            }
            (_, FreeVar(sup_fv)) if sup_fv.is_linked() => {
                self.sub_unify(maybe_sub, &sup_fv.crack(), loc, param_name)
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
                if sub_fv.is_generalized() || sup_fv.is_generalized() {
                    return Ok(());
                }
                let (lsub, lsup) = sub_fv.get_subsup().unwrap();
                let (rsub, rsup) = sup_fv.get_subsup().unwrap();
                // ?T(<: Add(?T))
                // ?U(:> {1, 2}, <: Add(?U)) ==> {1, 2}
                sup_fv.forced_undoable_link(&rsub);
                if lsub.qual_name() == rsub.qual_name() {
                    for (lps, rps) in lsub.typarams().iter().zip(rsub.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, loc, false)
                            .map_err(|errs| {
                                sup_fv.undo();
                                errs
                            })?;
                    }
                }
                // lsup: Add(?X(:> Int)), rsup: Add(?Y(:> Nat))
                //   => lsup: Add(?X(:> Int)), rsup: Add((?X(:> Int)))
                if lsup.qual_name() == rsup.qual_name() {
                    for (lps, rps) in lsup.typarams().iter().zip(rsup.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, loc, false)
                            .map_err(|errs| {
                                sup_fv.undo();
                                errs
                            })?;
                    }
                }
                sup_fv.undo();
                let intersec = self.intersection(&lsup, &rsup);
                let new_constraint = if intersec != Type::Never {
                    Constraint::new_sandwiched(self.union(&lsub, &rsub), intersec)
                } else {
                    return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        loc.loc(),
                        self.caused_by(),
                    )));
                };
                if sub_fv.level().unwrap_or(GENERIC_LEVEL)
                    <= sup_fv.level().unwrap_or(GENERIC_LEVEL)
                {
                    sub_fv.update_constraint(new_constraint, false);
                    sup_fv.link(maybe_sub);
                } else {
                    sup_fv.update_constraint(new_constraint, false);
                    sub_fv.link(maybe_sup);
                }
                Ok(())
            }
            // NG: Nat <: ?T or Int ==> Nat or Int (?T = Nat)
            // OK: Nat <: ?T or Int ==> ?T or Int
            (sub, Or(l, r))
                if l.is_unbound_var()
                    && !sub.is_unbound_var()
                    && !r.is_unbound_var()
                    && self.subtype_of(sub, r) =>
            {
                Ok(())
            }
            (sub, Or(l, r))
                if r.is_unbound_var()
                    && !sub.is_unbound_var()
                    && !l.is_unbound_var()
                    && self.subtype_of(sub, l) =>
            {
                Ok(())
            }
            // e.g. Structural({ .method = (self: T) -> Int })/T
            (Structural(sub), FreeVar(sup_fv))
                if sup_fv.is_unbound() && sub.contains_tvar(sup_fv) =>
            {
                Ok(())
            }
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
                if let Some((sub, mut sup)) = sup_fv.get_subsup() {
                    if sup.is_structural() {
                        self.sub_unify(maybe_sub, &sup, loc, param_name)?;
                    }
                    let new_sub = self.union(maybe_sub, &sub);
                    // Expanding to an Or-type is prohibited by default
                    // This increases the quality of error reporting
                    // (Try commenting out this part and run tests/should_err/subtyping.er to see the error report changes on lines 29-30)
                    if !maybe_sub.is_union_type() && !sub.is_union_type() && new_sub.is_union_type()
                    {
                        let (l, r) = new_sub.union_pair().unwrap();
                        if self.unify(&l, &r).is_none() {
                            let hint = switch_lang!(
                                "japanese" => format!("{maybe_sub}から{new_sub}への暗黙の型拡大はデフォルトでは禁止されています。明示的に型指定してください"),
                                "simplified_chinese" => format!("隐式扩展{maybe_sub}到{new_sub}被默认禁止。请明确指定类型。"),
                                "traditional_chinese" => format!("隱式擴展{maybe_sub}到{new_sub}被默認禁止。請明確指定類型。"),
                                "english" => format!("Implicitly widening {maybe_sub} to {new_sub} is prohibited by default. Consider specifying the type explicitly."),
                            );
                            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                loc.loc(),
                                self.caused_by(),
                                "",
                                None,
                                maybe_sub,
                                maybe_sup,
                                None,
                                Some(hint),
                            )));
                        }
                    }
                    if sup.contains_union(&new_sub) {
                        sup_fv.link(&new_sub); // Bool <: ?T <: Bool or Y ==> ?T == Bool
                    } else {
                        let constr = Constraint::new_sandwiched(new_sub, mem::take(&mut sup));
                        sup_fv.update_constraint(constr, true);
                    }
                }
                // sub_unify(Nat, ?T(: Type)): (/* ?T(:> Nat) */)
                else if let Some(ty) = sup_fv.get_type() {
                    if self.supertype_of(&Type, &ty) {
                        let constr = Constraint::new_supertype_of(maybe_sub.clone());
                        sup_fv.update_constraint(constr, true);
                    } else {
                        todo!()
                    }
                }
                Ok(())
            }
            (FreeVar(sub_fv), Structural(sup)) if sub_fv.is_unbound() => {
                let sub_fields = self.fields(maybe_sub);
                for (sup_field, sup_ty) in self.fields(sup) {
                    if let Some((_, sub_ty)) = sub_fields.get_key_value(&sup_field) {
                        self.sub_unify(sub_ty, &sup_ty, loc, param_name)?;
                    } else if !self.subtype_of(&sub_fv.get_sub().unwrap(), &Never) {
                        maybe_sub.coerce();
                        return self.sub_unify(maybe_sub, maybe_sup, loc, param_name);
                    } else {
                        // e.g. ?T / Structural({ .method = (self: ?T) -> Int })
                        sub_fv.update_super(|sup| self.intersection(&sup, maybe_sup));
                    }
                }
                Ok(())
            }
            (FreeVar(sub_fv), Ref(t)) if sub_fv.is_unbound() => {
                self.sub_unify(maybe_sub, t, loc, param_name)
            }
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
                    let new_sup = if let Some(new_sup) = self.min(&sup, maybe_sup) {
                        new_sup.clone()
                    } else {
                        self.intersection(&sup, maybe_sup)
                    };
                    // ?T(:> Int, <: Int) ==> ?T == Int
                    // ?T(:> Array(Int, 3), <: Array(?T, ?N)) ==> ?T == Array(Int, 3)
                    if !sub.is_refinement()
                        && new_sup.qual_name() == sub.qual_name()
                        && !new_sup.is_unbound_var()
                        && !sub.is_unbound_var()
                    {
                        self.sub_unify(&sub, &new_sup, loc, param_name)?;
                        sub_fv.link(&sub);
                    } else {
                        let constr = Constraint::new_sandwiched(sub, new_sup);
                        sub_fv.update_constraint(constr, true);
                    }
                }
                // sub_unify(?T(: Type), Int): (?T(<: Int))
                else if let Some(ty) = sub_fv.get_type() {
                    if self.supertype_of(&Type, &ty) {
                        let constr = Constraint::new_subtype_of(maybe_sup.clone());
                        sub_fv.update_constraint(constr, true);
                    } else {
                        todo!()
                    }
                }
                Ok(())
            }
            (Record(lrec), Record(rrec)) => {
                for (k, l) in lrec.iter() {
                    if let Some(r) = rrec.get(k) {
                        self.sub_unify(l, r, loc, param_name)?;
                    } else {
                        return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            maybe_sub,
                            maybe_sup,
                            loc.loc(),
                            self.caused_by(),
                        )));
                    }
                }
                Ok(())
            }
            (Subr(lsub), Subr(rsub)) => {
                lsub.non_default_params
                    .iter()
                    .zip(rsub.non_default_params.iter())
                    .try_for_each(|(l, r)| {
                        // contravariant
                        self.sub_unify(r.typ(), l.typ(), loc, param_name)
                    })?;
                lsub.var_params
                    .iter()
                    .zip(rsub.var_params.iter())
                    .try_for_each(|(l, r)| {
                        // contravariant
                        self.sub_unify(r.typ(), l.typ(), loc, param_name)
                    })?;
                for lpt in lsub.default_params.iter() {
                    if let Some(rpt) = rsub
                        .default_params
                        .iter()
                        .find(|rpt| rpt.name() == lpt.name())
                    {
                        // contravariant
                        self.sub_unify(rpt.typ(), lpt.typ(), loc, param_name)?;
                    } else {
                        let param_name = lpt.name().map_or("_", |s| &s[..]);
                        let similar_param = erg_common::levenshtein::get_similar_name(
                            rsub.default_params
                                .iter()
                                .map(|pt| pt.name().map_or("_", |s| &s[..])),
                            param_name,
                        );
                        return Err(TyCheckErrors::from(
                            TyCheckError::default_param_not_found_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                loc.loc(),
                                self.caused_by(),
                                param_name,
                                similar_param,
                            ),
                        ));
                    }
                }
                // covariant
                self.sub_unify(&lsub.return_t, &rsub.return_t, loc, param_name)?;
                Ok(())
            }
            (Quantified(lsub), Subr(rsub)) => {
                let Ok(lsub) = <&SubrType>::try_from(lsub.as_ref()) else { unreachable!() };
                for lpt in lsub.default_params.iter() {
                    if let Some(rpt) = rsub
                        .default_params
                        .iter()
                        .find(|rpt| rpt.name() == lpt.name())
                    {
                        if lpt.typ().is_generalized() {
                            continue;
                        }
                        // contravariant
                        self.sub_unify(rpt.typ(), lpt.typ(), loc, param_name)?;
                    } else {
                        todo!()
                    }
                }
                lsub.non_default_params
                    .iter()
                    .zip(rsub.non_default_params.iter())
                    .try_for_each(|(l, r)| {
                        if l.typ().is_generalized() {
                            Ok(())
                        }
                        // contravariant
                        else {
                            self.sub_unify(r.typ(), l.typ(), loc, param_name)
                        }
                    })?;
                // covariant
                self.sub_unify(&lsub.return_t, &rsub.return_t, loc, param_name)?;
                Ok(())
            }
            (Subr(lsub), Quantified(rsub)) => {
                let Ok(rsub) = <&SubrType>::try_from(rsub.as_ref()) else { unreachable!() };
                for lpt in lsub.default_params.iter() {
                    if let Some(rpt) = rsub
                        .default_params
                        .iter()
                        .find(|rpt| rpt.name() == lpt.name())
                    {
                        // contravariant
                        if rpt.typ().is_generalized() {
                            continue;
                        }
                        self.sub_unify(rpt.typ(), lpt.typ(), loc, param_name)?;
                    } else {
                        todo!()
                    }
                }
                lsub.non_default_params
                    .iter()
                    .zip(rsub.non_default_params.iter())
                    .try_for_each(|(l, r)| {
                        // contravariant
                        if r.typ().is_generalized() {
                            Ok(())
                        } else {
                            self.sub_unify(r.typ(), l.typ(), loc, param_name)
                        }
                    })?;
                // covariant
                self.sub_unify(&lsub.return_t, &rsub.return_t, loc, param_name)?;
                Ok(())
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
                //      Array(Str) <: Iterable(Str)
                //      Zip(T, U) <: Iterable(Tuple([T, U]))
                if ln != rn {
                    self.nominal_sub_unify(maybe_sub, maybe_sup, rps, loc)
                } else {
                    for (l_maybe_sub, r_maybe_sup) in lps.iter().zip(rps.iter()) {
                        self.sub_unify_tp(l_maybe_sub, r_maybe_sup, None, loc, false)?;
                    }
                    Ok(())
                }
            }
            (Structural(l), Structural(r)) => self.sub_unify(l, r, loc, param_name),
            (sub, Structural(sup)) => {
                let sub_fields = self.fields(sub);
                for (sup_field, sup_ty) in self.fields(sup) {
                    if let Some((_, sub_ty)) = sub_fields.get_key_value(&sup_field) {
                        self.sub_unify(sub_ty, &sup_ty, loc, param_name)?;
                    } else {
                        return Err(TyCheckErrors::from(TyCheckError::no_attr_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            loc.loc(),
                            self.caused_by(),
                            sub,
                            &sup_field.symbol,
                            self.get_similar_attr(sub, &sup_field.symbol),
                        )));
                    }
                }
                Ok(())
            }
            (
                _,
                Poly {
                    params: sup_params, ..
                },
            ) => self.nominal_sub_unify(maybe_sub, maybe_sup, sup_params, loc),
            (Or(l1, r1), Or(l2, r2)) | (Type::And(l1, r1), Type::And(l2, r2)) => {
                if self.subtype_of(l1, l2) && self.subtype_of(r1, r2) {
                    self.sub_unify(l1, l2, loc, param_name)?;
                    self.sub_unify(r1, r2, loc, param_name)
                } else {
                    self.sub_unify(l1, r2, loc, param_name)?;
                    self.sub_unify(r1, l2, loc, param_name)
                }
            }
            // (X or Y) <: Z is valid when X <: Z and Y <: Z
            (Or(l, r), _) => {
                self.sub_unify(l, maybe_sup, loc, param_name)?;
                self.sub_unify(r, maybe_sup, loc, param_name)
            }
            // X <: (Y and Z) is valid when X <: Y and X <: Z
            (_, And(l, r)) => {
                self.sub_unify(maybe_sub, l, loc, param_name)?;
                self.sub_unify(maybe_sub, r, loc, param_name)
            }
            // (X and Y) <: Z is valid when X <: Z or Y <: Z
            (And(l, r), _) => self
                .sub_unify(l, maybe_sup, loc, param_name)
                .or_else(|_e| self.sub_unify(r, maybe_sup, loc, param_name)),
            // X <: (Y or Z) is valid when X <: Y or X <: Z
            (_, Or(l, r)) => self
                .sub_unify(maybe_sub, l, loc, param_name)
                .or_else(|_e| self.sub_unify(maybe_sub, r, loc, param_name)),
            (Ref(l), Ref(r)) => self.sub_unify(l, r, loc, param_name),
            (_, Ref(t)) => self.sub_unify(maybe_sub, t, loc, param_name),
            (RefMut { before: l, .. }, RefMut { before: r, .. }) => {
                self.sub_unify(l, r, loc, param_name)
            }
            (_, RefMut { before, .. }) => self.sub_unify(maybe_sub, before, loc, param_name),
            (_, Proj { lhs, rhs }) => {
                if let Ok(evaled) = self.eval_proj(*lhs.clone(), rhs.clone(), self.level, loc) {
                    if maybe_sup != &evaled {
                        self.sub_unify(maybe_sub, &evaled, loc, param_name)?;
                    }
                }
                Ok(())
            }
            (Proj { lhs, rhs }, _) => {
                if let Ok(evaled) = self.eval_proj(*lhs.clone(), rhs.clone(), self.level, loc) {
                    if maybe_sub != &evaled {
                        self.sub_unify(&evaled, maybe_sup, loc, param_name)?;
                    }
                }
                Ok(())
            }
            // TODO: Judgment for any number of preds
            (Refinement(sub), Refinement(sup)) => {
                // {I: Int or Str | I == 0} <: {I: Int}
                if self.subtype_of(&sub.t, &sup.t) {
                    self.sub_unify(&sub.t, &sup.t, loc, param_name)?;
                }
                if sup.pred.as_ref() == &Predicate::TRUE {
                    self.sub_unify(&sub.t, &sup.t, loc, param_name)?;
                    return Ok(());
                }
                self.sub_unify_pred(&sub.pred, &sup.pred, loc)
            }
            // {I: Int | I >= 1} <: Nat == {I: Int | I >= 0}
            (Refinement(_), sup) => {
                let sup = sup.clone().into_refinement();
                self.sub_unify(maybe_sub, &Type::Refinement(sup), loc, param_name)
            }
            (sub, Refinement(_)) => {
                let sub = sub.clone().into_refinement();
                self.sub_unify(&Type::Refinement(sub), maybe_sup, loc, param_name)
            }
            (Subr(_) | Record(_), Type) => Ok(()),
            // REVIEW: correct?
            (Poly { name, .. }, Type) if &name[..] == "Array" || &name[..] == "Tuple" => Ok(()),
            (Poly { .. }, _) => self.nominal_sub_unify(maybe_sub, maybe_sup, &[], loc),
            (Subr(_), Mono(name)) if &name[..] == "GenericCallable" => Ok(()),
            _ => type_feature_error!(
                self,
                loc.loc(),
                &format!("{maybe_sub} can be a subtype of {maybe_sup}, but failed to semi-unify")
            ),
        }
    }

    // TODO: Current implementation is inefficient because coercion is performed twice with `subtype_of` in `sub_unify`
    fn nominal_sub_unify(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        sup_params: &[TyParam],
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        if let Some((sub_def_t, sub_ctx)) = self.get_nominal_type_ctx(maybe_sub) {
            let _sub_def_instance = self.instantiate_def_type(sub_def_t)?;
            // e.g.
            // maybe_sub: Zip(Int, Str)
            // sub_def_t: Zip(T, U) ==> Zip(Int, Str)
            // super_traits: [Iterable((T, U)), ...] ==> [Iterable((Int, Str)), ...]
            self.substitute_typarams(sub_def_t, maybe_sub)
                .map_err(|errs| {
                    Self::undo_substitute_typarams(sub_def_t);
                    errs
                })?;
            for sup_trait in sub_ctx.super_traits.iter() {
                let sub_trait_instance = self.instantiate_def_type(sup_trait)?;
                if self.supertype_of(maybe_sup, sup_trait) {
                    for (l_maybe_sub, r_maybe_sup) in
                        sub_trait_instance.typarams().iter().zip(sup_params.iter())
                    {
                        self.sub_unify_tp(l_maybe_sub, r_maybe_sup, None, loc, false)
                            .map_err(|errs| {
                                Self::undo_substitute_typarams(sub_def_t);
                                errs
                            })?;
                    }
                    Self::undo_substitute_typarams(sub_def_t);
                    return Ok(());
                }
            }
            Self::undo_substitute_typarams(sub_def_t);
        }
        Err(TyCheckErrors::from(TyCheckError::unification_error(
            self.cfg.input.clone(),
            line!() as usize,
            maybe_sub,
            maybe_sup,
            loc.loc(),
            self.caused_by(),
        )))
    }

    /// Unify two types into a single type based on the subtype relation.
    ///
    /// Error if they can't unify without upcasting both types (derefining is allowed) or using the Or type
    /// ```erg
    /// unify(Int, Nat) == Ok(Int)
    /// unify(Int, Str) == Err
    /// unify({1.2}, Nat) == Ok(Float)
    /// unify(Eq, Int) == Ok(Eq)
    /// unify(Eq, Float) == Err
    /// ```
    pub fn unify(&self, lhs: &Type, rhs: &Type) -> Option<Type> {
        let lhs = lhs.derefine();
        let rhs = rhs.derefine();
        self.max(&lhs, &rhs).cloned()
    }
}
