//! provides type variable related operations
use std::mem;
use std::option::Option;

use erg_common::fresh::fresh_varname;
use erg_common::traits::Locational;
use erg_common::Str;
#[allow(unused_imports)]
use erg_common::{fmt_vec, fn_name, log};

use crate::ty::constructors::*;
use crate::ty::free::{Constraint, FreeKind, HasLevel, GENERIC_LEVEL};
use crate::ty::typaram::{OpKind, TyParam};
use crate::ty::value::ValueObj;
use crate::ty::{Predicate, SubrType, Type};

use crate::context::{Context, Variance};
use crate::error::{TyCheckError, TyCheckErrors, TyCheckResult};
use crate::{feature_error, type_feature_error};

use Predicate as Pred;
use Type::*;
use ValueObj::{Inf, NegInf};

impl Context {
    /// ```erg
    /// occur(?T, ?T) ==> OK
    /// occur(X -> ?T, ?T) ==> Error
    /// occur(X -> ?T, X -> ?T) ==> OK
    /// occur(?T, ?T -> X) ==> Error
    /// occur(?T, Option(?T)) ==> Error
    /// occur(?T or ?U, ?T) ==> OK
    /// occur(?T or Int, Int or ?T) ==> OK
    /// occur(?T(<: Str) or ?U(<: Int), ?T(<: Str)) ==> Error
    /// occur(?T, ?T.Output) ==> OK
    /// ```
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
            (Or(l, r), Or(l2, r2)) | (And(l, r), And(l2, r2)) => self
                .occur(l, l2, loc)
                .and(self.occur(r, r2, loc))
                .or(self.occur(l, r2, loc).and(self.occur(r, l2, loc))),
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
            (TyParam::Type(sub), TyParam::Type(sup)) => self.sub_unify(sub, sup, loc, None),
            (TyParam::Value(ValueObj::Type(sub)), TyParam::Type(sup)) => {
                self.sub_unify(sub.typ(), sup, loc, None)
            }
            (TyParam::Type(sub), TyParam::Value(ValueObj::Type(sup))) => {
                self.sub_unify(sub, sup.typ(), loc, None)
            }
            (TyParam::Value(ValueObj::Type(sub)), TyParam::Value(ValueObj::Type(sup))) => {
                self.sub_unify(sub.typ(), sup.typ(), loc, None)
            }
            (TyParam::FreeVar(sub_fv), TyParam::FreeVar(sup_fv))
                if sub_fv.is_unbound() && sup_fv.is_unbound() =>
            {
                if sub_fv.level().unwrap() > sup_fv.level().unwrap() {
                    if !sub_fv.is_generalized() {
                        sub_fv.link(maybe_sup);
                    }
                } else if !sup_fv.is_generalized() {
                    sup_fv.link(maybe_sub);
                }
                Ok(())
            }
            (TyParam::FreeVar(sub_fv), sup_tp) => {
                match &*sub_fv.borrow() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.sub_unify_tp(l, sup_tp, _variance, loc, allow_divergence);
                    }
                    FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {}
                } // &fv is dropped
                let fv_t = sub_fv.constraint().unwrap().get_type().unwrap().clone(); // lfvを参照しないよいにcloneする(あとでborrow_mutするため)
                let tp_t = self.get_tp_t(sup_tp)?;
                if self.supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if sub_fv.level() < Some(self.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t);
                        if self.is_sub_constraint_of(&sub_fv.constraint().unwrap(), &new_constraint)
                            || sub_fv.constraint().unwrap().get_type() == Some(&Type)
                        {
                            sub_fv.update_constraint(new_constraint, false);
                        }
                    } else {
                        sub_fv.link(sup_tp);
                    }
                    Ok(())
                } else if allow_divergence
                    && (self.eq_tp(sup_tp, &TyParam::value(Inf))
                        || self.eq_tp(sup_tp, &TyParam::value(NegInf)))
                    && self.subtype_of(&fv_t, &mono("Num"))
                {
                    sub_fv.link(sup_tp);
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    )))
                }
            }
            (sub_tp, TyParam::FreeVar(sup_fv)) => {
                match &*sup_fv.borrow() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.sub_unify_tp(l, sub_tp, _variance, loc, allow_divergence);
                    }
                    FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {}
                } // &fv is dropped
                let fv_t = sup_fv.constraint().unwrap().get_type().unwrap().clone(); // fvを参照しないよいにcloneする(あとでborrow_mutするため)
                let tp_t = self.get_tp_t(sub_tp)?;
                if self.supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if sup_fv.level() < Some(self.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t);
                        if self.is_sub_constraint_of(&sup_fv.constraint().unwrap(), &new_constraint)
                            || sup_fv.constraint().unwrap().get_type() == Some(&Type)
                        {
                            sup_fv.update_constraint(new_constraint, false);
                        }
                    } else {
                        sup_fv.link(sub_tp);
                    }
                    Ok(())
                } else if allow_divergence
                    && (self.eq_tp(sub_tp, &TyParam::value(Inf))
                        || self.eq_tp(sub_tp, &TyParam::value(NegInf)))
                    && self.subtype_of(&fv_t, &mono("Num"))
                {
                    sup_fv.link(sub_tp);
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
            (sub, TyParam::Erased(t)) => {
                let sub_t = self.get_tp_t(sub)?;
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
            (sub, TyParam::Type(sup)) => {
                let l = self.convert_tp_into_type(sub.clone()).map_err(|_| {
                    TyCheckError::tp_to_type_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        sub,
                        loc.loc(),
                        self.caused_by(),
                    )
                })?;
                self.sub_unify(&l, sup, loc, None)?;
                Ok(())
            }
            (TyParam::Type(sub), sup) => {
                let r = self.convert_tp_into_type(sup.clone()).map_err(|_| {
                    TyCheckError::tp_to_type_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        sup,
                        loc.loc(),
                        self.caused_by(),
                    )
                })?;
                self.sub_unify(sub, &r, loc, None)?;
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
        self.occur(maybe_sub, maybe_sup, loc).map_err(|err| {
            log!(err "occur error: {maybe_sub} / {maybe_sup}");
            err
        })?;
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
                sup_fv.dummy_link();
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
                if intersec == Type::Never {
                    return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        loc.loc(),
                        self.caused_by(),
                    )));
                }
                let union = self.union(&lsub, &rsub);
                if lsub.union_size().max(rsub.union_size()) < union.union_size() {
                    let (l, r) = union.union_pair().unwrap_or((lsub, rsub));
                    let unified = self.unify(&l, &r);
                    if unified.is_none() {
                        let maybe_sub = self.readable_type(maybe_sub.clone());
                        let union = self.readable_type(union);
                        return Err(TyCheckErrors::from(TyCheckError::implicit_widening_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            loc.loc(),
                            self.caused_by(),
                            &maybe_sub,
                            &union,
                        )));
                    }
                }
                let new_constraint = Constraint::new_sandwiched(union, intersec);
                match sub_fv
                    .level()
                    .unwrap_or(GENERIC_LEVEL)
                    .cmp(&sup_fv.level().unwrap_or(GENERIC_LEVEL))
                {
                    std::cmp::Ordering::Less => {
                        sub_fv.update_constraint(new_constraint, false);
                        sup_fv.link(maybe_sub);
                    }
                    std::cmp::Ordering::Greater => {
                        sup_fv.update_constraint(new_constraint, false);
                        sub_fv.link(maybe_sup);
                    }
                    std::cmp::Ordering::Equal => {
                        // choose named one
                        if sup_fv.is_named_unbound() {
                            sup_fv.update_constraint(new_constraint, false);
                            sub_fv.link(maybe_sup);
                        } else {
                            sub_fv.update_constraint(new_constraint, false);
                            sup_fv.link(maybe_sub);
                        }
                    }
                }
                Ok(())
            }
            // (Int or ?T) <: (?U or Int)
            // OK: (Int <: Int); (?T <: ?U)
            // NG: (Int <: ?U); (?T <: Int)
            (Or(l1, r1), Or(l2, r2)) | (And(l1, r1), And(l2, r2)) => {
                if self.subtype_of(l1, l2) && self.subtype_of(r1, r2) {
                    let (l_sup, r_sup) = if self.subtype_of(l1, r2)
                        && !l1.is_unbound_var()
                        && !r2.is_unbound_var()
                    {
                        (r2, l2)
                    } else {
                        (l2, r2)
                    };
                    self.sub_unify(l1, l_sup, loc, param_name)?;
                    self.sub_unify(r1, r_sup, loc, param_name)
                } else {
                    self.sub_unify(l1, r2, loc, param_name)?;
                    self.sub_unify(r1, l2, loc, param_name)
                }
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
            (_, FreeVar(sup_fv)) if sup_fv.is_generalized() => Ok(()),
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
                    if maybe_sub.union_size().max(sub.union_size()) < new_sub.union_size()
                        && new_sub.union_types().iter().any(|t| !t.is_unbound_var())
                    {
                        let (l, r) = new_sub.union_pair().unwrap_or((maybe_sub.clone(), sub));
                        let unified = self.unify(&l, &r);
                        if unified.is_none() {
                            let maybe_sub = self.readable_type(maybe_sub.clone());
                            let new_sub = self.readable_type(new_sub);
                            return Err(TyCheckErrors::from(
                                TyCheckError::implicit_widening_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    loc.loc(),
                                    self.caused_by(),
                                    &maybe_sub,
                                    &new_sub,
                                ),
                            ));
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
            (FreeVar(sub_fv), Ref(sup)) if sub_fv.is_unbound() => {
                self.sub_unify(maybe_sub, sup, loc, param_name)
            }
            (FreeVar(sub_fv), _) if sub_fv.is_generalized() => Ok(()),
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
                    let new_sup = if let Some(new_sup) = self.min(&sup, maybe_sup).either() {
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
            (Record(sub_rec), Record(sup_rec)) => {
                for (k, l) in sub_rec.iter() {
                    if let Some(r) = sup_rec.get(k) {
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
            (Subr(sub_subr), Subr(sup_subr)) => {
                sub_subr
                    .non_default_params
                    .iter()
                    .zip(sup_subr.non_default_params.iter())
                    .try_for_each(|(sub, sup)| {
                        // contravariant
                        self.sub_unify(sup.typ(), sub.typ(), loc, param_name)
                    })?;
                sub_subr
                    .var_params
                    .iter()
                    .zip(sup_subr.var_params.iter())
                    .try_for_each(|(sub, sup)| {
                        // contravariant
                        self.sub_unify(sup.typ(), sub.typ(), loc, param_name)
                    })?;
                for sup_pt in sup_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == sup_pt.name())
                    {
                        // contravariant
                        self.sub_unify(sup_pt.typ(), sub_pt.typ(), loc, param_name)?;
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
                self.sub_unify(&sub_subr.return_t, &sup_subr.return_t, loc, param_name)?;
                Ok(())
            }
            (Quantified(sub_subr), Subr(sup_subr)) => {
                let Ok(sub_subr) = <&SubrType>::try_from(sub_subr.as_ref()) else { unreachable!() };
                sub_subr
                    .non_default_params
                    .iter()
                    .zip(sup_subr.non_default_params.iter())
                    .try_for_each(|(sub, sup)| {
                        if sub.typ().is_generalized() {
                            Ok(())
                        }
                        // contravariant
                        else {
                            self.sub_unify(sup.typ(), sub.typ(), loc, param_name)
                        }
                    })?;
                for sup_pt in sup_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == sup_pt.name())
                    {
                        if sup_pt.typ().is_generalized() {
                            continue;
                        }
                        // contravariant
                        self.sub_unify(sup_pt.typ(), sub_pt.typ(), loc, param_name)?;
                    } else {
                        todo!()
                    }
                }
                // covariant
                self.sub_unify(&sub_subr.return_t, &sup_subr.return_t, loc, param_name)?;
                Ok(())
            }
            (Subr(sub_subr), Quantified(sup_subr)) => {
                let Ok(sup_subr) = <&SubrType>::try_from(sup_subr.as_ref()) else { unreachable!() };
                sub_subr
                    .non_default_params
                    .iter()
                    .zip(sup_subr.non_default_params.iter())
                    .try_for_each(|(sub, sup)| {
                        // contravariant
                        if sup.typ().is_generalized() {
                            Ok(())
                        } else {
                            self.sub_unify(sup.typ(), sub.typ(), loc, param_name)
                        }
                    })?;
                for sup_pt in sup_subr.default_params.iter() {
                    if let Some(sub_pt) = sub_subr
                        .default_params
                        .iter()
                        .find(|sub_pt| sub_pt.name() == sup_pt.name())
                    {
                        // contravariant
                        if sup_pt.typ().is_generalized() {
                            continue;
                        }
                        self.sub_unify(sup_pt.typ(), sub_pt.typ(), loc, param_name)?;
                    } else {
                        todo!()
                    }
                }
                // covariant
                self.sub_unify(&sub_subr.return_t, &sup_subr.return_t, loc, param_name)?;
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
            (Structural(sub), Structural(sup)) => self.sub_unify(sub, sup, loc, param_name),
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
            (And(l, r), _) => {
                if self.subtype_of(l, maybe_sup) {
                    self.sub_unify(l, maybe_sup, loc, param_name)
                } else {
                    self.sub_unify(r, maybe_sup, loc, param_name)
                }
            }
            // X <: (Y or Z) is valid when X <: Y or X <: Z
            (_, Or(l, r)) => {
                if self.subtype_of(maybe_sub, l) {
                    self.sub_unify(maybe_sub, l, loc, param_name)
                } else {
                    self.sub_unify(maybe_sub, r, loc, param_name)
                }
            }
            (Ref(sub), Ref(sup)) => self.sub_unify(sub, sup, loc, param_name),
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
            (
                _,
                Poly {
                    params: sup_params, ..
                },
            ) => self.nominal_sub_unify(maybe_sub, maybe_sup, sup_params, loc),
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
        debug_assert_ne!(maybe_sub.qual_name(), maybe_sup.qual_name());
        if let Some((sub_def_t, sub_ctx)) = self.get_nominal_type_ctx(maybe_sub) {
            // e.g.
            // maybe_sub: Zip(Int, Str)
            // sub_def_t: Zip(T, U) ==> Zip(Int, Str)
            // super_traits: [Iterable((T, U)), ...] ==> [Iterable((Int, Str)), ...]
            self.substitute_typarams(sub_def_t, maybe_sub)
                .map_err(|errs| {
                    Self::undo_substitute_typarams(sub_def_t);
                    errs
                })?;
            let sups = if self.is_class(maybe_sup) {
                sub_ctx.super_classes.iter()
            } else {
                sub_ctx.super_traits.iter()
            };
            for sup_ty in sups {
                let sub_instance = self.instantiate_def_type(sup_ty)?;
                if self.supertype_of(maybe_sup, sup_ty) {
                    for (l_maybe_sub, r_maybe_sup) in
                        sub_instance.typarams().iter().zip(sup_params.iter())
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
    /// unify(Int, Nat) == Some(Int)
    /// unify(Int, Str) == None
    /// unify({1.2}, Nat) == Some(Float)
    /// unify(Nat, Int!) == Some(Int)
    /// unify(Eq, Int) == None
    /// ```
    pub(crate) fn unify(&self, lhs: &Type, rhs: &Type) -> Option<Type> {
        let l_sups = self._get_super_classes(lhs)?;
        let r_sups = self._get_super_classes(rhs)?;
        for l_sup in l_sups {
            if self.supertype_of(&l_sup, &Obj) {
                continue;
            }
            for r_sup in r_sups.clone() {
                if self.supertype_of(&r_sup, &Obj) {
                    continue;
                }
                if let Some(t) = self.max(&l_sup, &r_sup).either() {
                    return Some(t.clone());
                }
            }
        }
        None
    }
}
