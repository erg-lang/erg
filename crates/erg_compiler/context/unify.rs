//! provides type variable related operations
use std::mem;
use std::option::Option;

use erg_common::fn_name;
use erg_common::traits::Locational;
use erg_common::Str;
#[allow(unused_imports)]
use erg_common::{fmt_vec, log};

use crate::context::instantiate::TyVarCache;
use crate::ty::constructors::*;
use crate::ty::free::{Constraint, FreeKind, HasLevel};
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{Predicate, SubrType, Type};

use crate::context::{Context, Variance};
use crate::error::{SingleTyCheckResult, TyCheckError, TyCheckErrors, TyCheckResult};
use crate::{feature_error, type_feature_error};

use Predicate as Pred;
use Type::*;
use ValueObj::{Inf, NegInf};

impl Context {
    /// occur(?T, ?T) ==> Error
    /// occur(X -> ?T, ?T) ==> Error
    /// occur(?T, ?T -> X) ==> Error
    /// occur(?T, Option(?T)) ==> Error
    /// occur(?T, ?T.Output) ==> OK
    pub(crate) fn occur(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        match (maybe_sub, maybe_sup) {
            (Type::FreeVar(fv), _) if fv.is_linked() => self.occur(&fv.crack(), maybe_sup, loc),
            (_, Type::FreeVar(fv)) if fv.is_linked() => self.occur(maybe_sub, &fv.crack(), loc),
            (Type::FreeVar(sub), Type::FreeVar(sup)) => {
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
            (Type::Subr(subr), Type::FreeVar(fv)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur(default_t, maybe_sup, loc)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur(var_params.typ(), maybe_sup, loc)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur(non_default_t, maybe_sup, loc)?;
                }
                self.occur(&subr.return_t, maybe_sup, loc)?;
                Ok(())
            }
            (Type::FreeVar(fv), Type::Subr(subr)) if fv.is_unbound() => {
                for default_t in subr.default_params.iter().map(|pt| pt.typ()) {
                    self.occur(maybe_sub, default_t, loc)?;
                }
                if let Some(var_params) = subr.var_params.as_ref() {
                    self.occur(maybe_sub, var_params.typ(), loc)?;
                }
                for non_default_t in subr.non_default_params.iter().map(|pt| pt.typ()) {
                    self.occur(maybe_sub, non_default_t, loc)?;
                }
                self.occur(maybe_sub, &subr.return_t, loc)?;
                Ok(())
            }
            (Type::Subr(lhs), Type::Subr(rhs)) => {
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
            (Type::Poly { params, .. }, Type::FreeVar(fv)) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| {
                    if let TyParam::Type(t) = tp {
                        Some(t)
                    } else {
                        None
                    }
                }) {
                    self.occur(param, maybe_sup, loc)?;
                }
                Ok(())
            }
            (Type::FreeVar(fv), Type::Poly { params, .. }) if fv.is_unbound() => {
                for param in params.iter().filter_map(|tp| {
                    if let TyParam::Type(t) = tp {
                        Some(t)
                    } else {
                        None
                    }
                }) {
                    self.occur(maybe_sub, param, loc)?;
                }
                Ok(())
            }
            (lhs, Type::Or(l, r)) | (lhs, Type::And(l, r)) => {
                self.occur(lhs, l, loc)?;
                self.occur(lhs, r, loc)
            }
            (Type::Or(l, r), rhs) | (Type::And(l, r), rhs) => {
                self.occur(l, rhs, loc)?;
                self.occur(r, rhs, loc)
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
                let l = self
                    .convert_tp_into_ty(l.clone())
                    .unwrap_or_else(|_| todo!("{l} cannot be a type"));
                self.sub_unify(&l, r, loc, None)?;
                Ok(())
            }
            (TyParam::Type(l), r) => {
                let r = self
                    .convert_tp_into_ty(r.clone())
                    .unwrap_or_else(|_| todo!("{r} cannot be a type"));
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
            (l, r) => panic!("type-parameter unification failed:\nl:{l}\nr: {r}"),
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
            (l, r) => panic!("type-parameter re-unification failed:\nl: {l}\nr: {r}"),
        }
    }

    /// predは正規化されているとする
    fn sub_unify_pred(
        &self,
        l_pred: &Predicate,
        r_pred: &Predicate,
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        match (l_pred, r_pred) {
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
            // unify({I >= 0}, {I >= ?M and I <= ?N}): ?M => 0, ?N => Inf
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
                        l_pred,
                        r_pred,
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
                    l_pred,
                    r_pred,
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
                    l_pred,
                    r_pred,
                    self.caused_by(),
                ))),
            },
            _ => Err(TyCheckErrors::from(TyCheckError::pred_unification_error(
                self.cfg.input.clone(),
                line!() as usize,
                l_pred,
                r_pred,
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
            (Type::FreeVar(fv), r) if fv.is_linked() => self.reunify(&fv.crack(), r, loc),
            (l, Type::FreeVar(fv)) if fv.is_linked() => self.reunify(l, &fv.crack(), loc),
            (Type::Ref(l), Type::Ref(r)) => self.reunify(l, r, loc),
            (
                Type::RefMut {
                    before: lbefore,
                    after: lafter,
                },
                Type::RefMut {
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
            (Type::Ref(l), r) => self.reunify(l, r, loc),
            // REVIEW:
            (Type::RefMut { before, .. }, r) => self.reunify(before, r, loc),
            (l, Type::Ref(r)) => self.reunify(l, r, loc),
            (l, Type::RefMut { before, .. }) => self.reunify(l, before, loc),
            (
                Type::Poly {
                    name: ln,
                    params: lps,
                },
                Type::Poly {
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
            (Type::FreeVar(lfv), _) if lfv.is_linked() => {
                self.sub_unify(&lfv.crack(), maybe_sup, loc, param_name)
            }
            (_, Type::FreeVar(rfv)) if rfv.is_linked() => {
                self.sub_unify(maybe_sub, &rfv.crack(), loc, param_name)
            }
            // lfv's sup can be shrunk (take min), rfv's sub can be expanded (take union)
            // lfvのsupは縮小可能(minを取る)、rfvのsubは拡大可能(unionを取る)
            // sub_unify(?T[0](:> Never, <: Int), ?U[1](:> Never, <: Nat)): (/* ?U[1] --> ?T[0](:> Never, <: Nat))
            // sub_unify(?T[1](:> Never, <: Nat), ?U[0](:> Never, <: Int)): (/* ?T[1] --> ?U[0](:> Never, <: Nat))
            // sub_unify(?T[0](:> Never, <: Str), ?U[1](:> Never, <: Int)): (?T[0](:> Never, <: Str and Int) --> Error!)
            // sub_unify(?T[0](:> Int, <: Add()), ?U[1](:> Never, <: Mul())): (?T[0](:> Int, <: Add() and Mul()))
            // sub_unify(?T[0](:> Str, <: Obj), ?U[1](:> Int, <: Obj)): (/* ?U[1] --> ?T[0](:> Str or Int) */)
            (Type::FreeVar(lfv), Type::FreeVar(rfv))
                if lfv.constraint_is_sandwiched() && rfv.constraint_is_sandwiched() =>
            {
                if lfv.is_generalized() || rfv.is_generalized() {
                    return Ok(());
                }
                let (lsub, lsup) = lfv.get_subsup().unwrap();
                let (rsub, rsup) = rfv.get_subsup().unwrap();
                // ?T(<: Add(?T))
                // ?U(:> {1, 2}, <: Add(?U)) ==> {1, 2}
                rfv.forced_undoable_link(&rsub);
                if lsub.qual_name() == rsub.qual_name() {
                    for (lps, rps) in lsub.typarams().iter().zip(rsub.typarams().iter()) {
                        self.sub_unify_tp(lps, rps, None, loc, false)
                            .map_err(|errs| {
                                rfv.undo();
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
                                rfv.undo();
                                errs
                            })?;
                    }
                }
                rfv.undo();
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
                if lfv.level().unwrap() <= rfv.level().unwrap() {
                    lfv.update_constraint(new_constraint, false);
                    rfv.link(maybe_sub);
                } else {
                    rfv.update_constraint(new_constraint, false);
                    lfv.link(maybe_sup);
                }
                Ok(())
            }
            // NG: Nat <: ?T or Int ==> Nat or Int (?T = Nat)
            // OK: Nat <: ?T or Int ==> ?T or Int
            (sub, Type::Or(l, r))
                if l.is_unbound_var()
                    && !sub.is_unbound_var()
                    && !r.is_unbound_var()
                    && self.subtype_of(sub, r) =>
            {
                Ok(())
            }
            (sub, Type::Or(l, r))
                if r.is_unbound_var()
                    && !sub.is_unbound_var()
                    && !l.is_unbound_var()
                    && self.subtype_of(sub, l) =>
            {
                Ok(())
            }
            // e.g. Structural({ .method = (self: T) -> Int })/T
            (Type::Structural(sub), Type::FreeVar(fv))
                if fv.is_unbound() && sub.contains_tvar(fv) =>
            {
                Ok(())
            }
            (_, Type::FreeVar(rfv)) if rfv.is_unbound() => {
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
                if let Some((sub, mut sup)) = rfv.get_subsup() {
                    if sup.is_structural() {
                        self.sub_unify(maybe_sub, &sup, loc, param_name)?;
                    }
                    let new_sub = self.union(maybe_sub, &sub);
                    if sup.contains_union(&new_sub) {
                        rfv.link(&new_sub); // Bool <: ?T <: Bool or Y ==> ?T == Bool
                    } else {
                        let constr = Constraint::new_sandwiched(new_sub, mem::take(&mut sup));
                        rfv.update_constraint(constr, true);
                    }
                }
                // sub_unify(Nat, ?T(: Type)): (/* ?T(:> Nat) */)
                else if let Some(ty) = rfv.get_type() {
                    if self.supertype_of(&Type, &ty) {
                        let constr = Constraint::new_supertype_of(maybe_sub.clone());
                        rfv.update_constraint(constr, true);
                    } else {
                        todo!()
                    }
                }
                Ok(())
            }
            (Type::FreeVar(fv), Type::Structural(sup)) if fv.is_unbound() => {
                let sub_fields = self.fields(maybe_sub);
                for (sup_field, sup_ty) in self.fields(sup) {
                    if let Some((_, sub_ty)) = sub_fields.get_key_value(&sup_field) {
                        self.sub_unify(sub_ty, &sup_ty, loc, param_name)?;
                    } else if !self.subtype_of(&fv.get_sub().unwrap(), &Never) {
                        maybe_sub.coerce();
                        return self.sub_unify(maybe_sub, maybe_sup, loc, param_name);
                    } else {
                        // e.g. ?T / Structural({ .method = (self: ?T) -> Int })
                        fv.update_super(|sup| self.intersection(&sup, maybe_sup));
                    }
                }
                Ok(())
            }
            (Type::FreeVar(lfv), Ref(t)) if lfv.is_unbound() => {
                self.sub_unify(maybe_sub, t, loc, param_name)
            }
            (Type::FreeVar(lfv), _) if lfv.is_unbound() => {
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
                if let Some((mut sub, sup)) = lfv.get_subsup() {
                    if sup.is_structural() {
                        return Ok(());
                    }
                    if let Some(new_sup) = self.min(&sup, maybe_sup) {
                        let constr =
                            Constraint::new_sandwiched(mem::take(&mut sub), new_sup.clone());
                        lfv.update_constraint(constr, true);
                    } else {
                        let new_sup = self.intersection(&sup, maybe_sup);
                        let constr = Constraint::new_sandwiched(mem::take(&mut sub), new_sup);
                        lfv.update_constraint(constr, true);
                    }
                }
                // sub_unify(?T(: Type), Int): (?T(<: Int))
                else if let Some(ty) = lfv.get_type() {
                    if self.supertype_of(&Type, &ty) {
                        let constr = Constraint::new_subtype_of(maybe_sup.clone());
                        lfv.update_constraint(constr, true);
                    } else {
                        todo!()
                    }
                }
                Ok(())
            }
            (Type::Record(lrec), Type::Record(rrec)) => {
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
            (Type::Subr(lsub), Type::Subr(rsub)) => {
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
            (Type::Quantified(lsub), Type::Subr(rsub)) => {
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
            (Type::Subr(lsub), Type::Quantified(rsub)) => {
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
                Type::Poly {
                    name: ln,
                    params: lps,
                },
                Type::Poly {
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
            (Type::Structural(l), Type::Structural(r)) => self.sub_unify(l, r, loc, param_name),
            (sub, Type::Structural(sup)) => {
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
                Type::Poly {
                    params: sup_params, ..
                },
            ) => self.nominal_sub_unify(maybe_sub, maybe_sup, sup_params, loc),
            (Type::Or(l1, r1), Type::Or(l2, r2)) | (Type::And(l1, r1), Type::And(l2, r2)) => {
                if self.subtype_of(l1, l2) && self.subtype_of(r1, r2) {
                    self.sub_unify(l1, l2, loc, param_name)?;
                    self.sub_unify(r1, r2, loc, param_name)
                } else {
                    self.sub_unify(l1, r2, loc, param_name)?;
                    self.sub_unify(r1, l2, loc, param_name)
                }
            }
            // (X or Y) <: Z is valid when X <: Z and Y <: Z
            (Type::Or(l, r), _) => {
                self.sub_unify(l, maybe_sup, loc, param_name)?;
                self.sub_unify(r, maybe_sup, loc, param_name)
            }
            // X <: (Y and Z) is valid when X <: Y and X <: Z
            (_, Type::And(l, r)) => {
                self.sub_unify(maybe_sub, l, loc, param_name)?;
                self.sub_unify(maybe_sub, r, loc, param_name)
            }
            // (X and Y) <: Z is valid when X <: Z or Y <: Z
            (Type::And(l, r), _) => self
                .sub_unify(l, maybe_sup, loc, param_name)
                .or_else(|_e| self.sub_unify(r, maybe_sup, loc, param_name)),
            // X <: (Y or Z) is valid when X <: Y or X <: Z
            (_, Type::Or(l, r)) => self
                .sub_unify(maybe_sub, l, loc, param_name)
                .or_else(|_e| self.sub_unify(maybe_sub, r, loc, param_name)),
            (Type::Ref(l), Type::Ref(r)) => self.sub_unify(l, r, loc, param_name),
            (_, Type::Ref(t)) => self.sub_unify(maybe_sub, t, loc, param_name),
            (Type::RefMut { before: l, .. }, Type::RefMut { before: r, .. }) => {
                self.sub_unify(l, r, loc, param_name)
            }
            (_, Type::RefMut { before, .. }) => self.sub_unify(maybe_sub, before, loc, param_name),
            (_, Type::Proj { lhs, rhs }) => {
                if let Ok(evaled) = self.eval_proj(*lhs.clone(), rhs.clone(), self.level, loc) {
                    if maybe_sup != &evaled {
                        self.sub_unify(maybe_sub, &evaled, loc, param_name)?;
                    }
                }
                Ok(())
            }
            (Type::Proj { lhs, rhs }, _) => {
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
            (Type::Refinement(_), sup) => {
                let sup = sup.clone().into_refinement();
                self.sub_unify(maybe_sub, &Type::Refinement(sup), loc, param_name)
            }
            (sub, Type::Refinement(_)) => {
                let sub = sub.clone().into_refinement();
                self.sub_unify(&Type::Refinement(sub), maybe_sup, loc, param_name)
            }
            (Type::Subr(_) | Type::Record(_), Type) => Ok(()),
            // REVIEW: correct?
            (Type::Poly { name, .. }, Type) if &name[..] == "Array" || &name[..] == "Tuple" => {
                Ok(())
            }
            (Type::Poly { .. }, _) => self.nominal_sub_unify(maybe_sub, maybe_sup, &[], loc),
            (Type::Subr(_), Mono(name)) if &name[..] == "GenericCallable" => Ok(()),
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
            let mut tv_cache = TyVarCache::new(self.level, self);
            let _sub_def_instance =
                self.instantiate_t_inner(sub_def_t.clone(), &mut tv_cache, loc)?;
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
                let sub_trait_instance =
                    self.instantiate_t_inner(sup_trait.clone(), &mut tv_cache, loc)?;
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
}
