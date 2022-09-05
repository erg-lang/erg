//! provides type variable related operations
use std::mem;
use std::option::Option;

use erg_common::error::Location;
use erg_common::set::Set;
use erg_common::traits::Stream;
use erg_common::Str;
use erg_common::{assume_unreachable, fn_name, set};

use erg_type::constructors::*;
use erg_type::free::{Constraint, Cyclicity, FreeKind, HasLevel};
use erg_type::typaram::TyParam;
use erg_type::value::ValueObj;
use erg_type::{HasType, Predicate, SubrKind, TyBound, Type};

use crate::context::{Context, Variance};
use crate::error::{TyCheckError, TyCheckResult};
use crate::hir;

use Predicate as Pred;
use Type::*;
use ValueObj::{Inf, NegInf};

impl Context {
    pub const TOP_LEVEL: usize = 1;
    // HACK: see doc/compiler/inference.md for details
    pub const GENERIC_LEVEL: usize = usize::MAX;

    /// 型を非依存化する
    fn _independentise<'a>(_t: Type, _ts: &[Type]) -> Type {
        todo!()
    }

    fn generalize_tp(
        &self,
        free: TyParam,
        bounds: &mut Set<TyBound>,
        lazy_inits: &mut Set<Str>,
    ) -> TyParam {
        match free {
            TyParam::Type(t) => TyParam::t(self.generalize_t_inner(*t, bounds, lazy_inits)),
            TyParam::FreeVar(v) if v.is_linked() => {
                if let FreeKind::Linked(tp) = &mut *v.borrow_mut() {
                    *tp = self.generalize_tp(tp.clone(), bounds, lazy_inits);
                } else {
                    assume_unreachable!()
                }
                TyParam::FreeVar(v)
            }
            // TODO: Polymorphic generalization
            TyParam::FreeVar(fv) if fv.level() > Some(self.level) => match &*fv.borrow() {
                FreeKind::Unbound { id, constraint, .. } => {
                    let name = id.to_string();
                    self.generalize_constraint(&name, constraint, bounds, lazy_inits);
                    TyParam::mono_q(name)
                }
                FreeKind::NamedUnbound {
                    name, constraint, ..
                } => {
                    self.generalize_constraint(name, constraint, bounds, lazy_inits);
                    TyParam::mono_q(name)
                }
                _ => assume_unreachable!(),
            },
            other if other.has_no_unbound_var() => other,
            other => todo!("{other}"),
        }
    }

    pub(crate) fn generalize_t(&self, free_type: Type) -> Type {
        let mut bounds = set! {};
        let mut lazy_inits = set! {};
        let maybe_unbound_t = self.generalize_t_inner(free_type, &mut bounds, &mut lazy_inits);
        // NOTE: ?T(<: TraitX) -> Intなどは単なるTraitX -> Intとなる
        if bounds.is_empty() {
            maybe_unbound_t
        } else {
            quant(maybe_unbound_t, bounds)
        }
    }

    /// see doc/LANG/compiler/inference.md#一般化 for details
    /// ```erg
    /// generalize_t(?T) == 'T: Type
    /// generalize_t(?T(<: Nat) -> ?T) == |'T <: Nat| 'T -> 'T
    /// generalize_t(?T(<: Eq(?T(<: Eq(?T(<: ...)))) -> ?T) == |'T <: Eq('T)| 'T -> 'T
    /// generalize_t(?T(<: TraitX) -> Int) == TraitX -> Int // 戻り値に現れないなら量化しない
    /// ```
    fn generalize_t_inner(
        &self,
        free_type: Type,
        bounds: &mut Set<TyBound>,
        lazy_inits: &mut Set<Str>,
    ) -> Type {
        match free_type {
            FreeVar(v) if v.is_linked() => {
                if let FreeKind::Linked(t) = &mut *v.borrow_mut() {
                    *t = self.generalize_t_inner(t.clone(), bounds, lazy_inits);
                } else {
                    assume_unreachable!()
                }
                Type::FreeVar(v)
            }
            // TODO: Polymorphic generalization
            FreeVar(fv) if fv.level().unwrap() > self.level => match &*fv.borrow() {
                FreeKind::Unbound { id, constraint, .. } => {
                    let name = id.to_string();
                    self.generalize_constraint(&name, constraint, bounds, lazy_inits);
                    mono_q(name)
                }
                FreeKind::NamedUnbound {
                    name, constraint, ..
                } => {
                    self.generalize_constraint(name, constraint, bounds, lazy_inits);
                    mono_q(name)
                }
                _ => assume_unreachable!(),
            },
            Subr(mut subr) => {
                let kind = match subr.kind {
                    SubrKind::FuncMethod(self_t) => {
                        let t = self.generalize_t_inner(*self_t, bounds, lazy_inits);
                        SubrKind::fn_met(t)
                    }
                    SubrKind::ProcMethod { before, after } => {
                        let before = self.generalize_t_inner(*before, bounds, lazy_inits);
                        if let Some(after) = after {
                            let after = self.generalize_t_inner(*after, bounds, lazy_inits);
                            SubrKind::pr_met(before, Some(after))
                        } else {
                            SubrKind::pr_met(before, None)
                        }
                    }
                    other => other,
                };
                subr.non_default_params.iter_mut().for_each(|nd_param| {
                    *nd_param.typ_mut() =
                        self.generalize_t_inner(mem::take(nd_param.typ_mut()), bounds, lazy_inits);
                });
                if let Some(var_args) = &mut subr.var_params {
                    *var_args.typ_mut() =
                        self.generalize_t_inner(mem::take(var_args.typ_mut()), bounds, lazy_inits);
                }
                subr.default_params.iter_mut().for_each(|d_param| {
                    *d_param.typ_mut() =
                        self.generalize_t_inner(mem::take(d_param.typ_mut()), bounds, lazy_inits);
                });
                let return_t = self.generalize_t_inner(*subr.return_t, bounds, lazy_inits);
                subr_t(
                    kind,
                    subr.non_default_params,
                    subr.var_params.map(|x| *x),
                    subr.default_params,
                    return_t,
                )
            }
            Callable { .. } => todo!(),
            Ref(t) => ref_(self.generalize_t_inner(*t, bounds, lazy_inits)),
            RefMut(t) => ref_mut(self.generalize_t_inner(*t, bounds, lazy_inits)),
            Poly { name, mut params } => {
                let params = params
                    .iter_mut()
                    .map(|p| self.generalize_tp(mem::take(p), bounds, lazy_inits))
                    .collect::<Vec<_>>();
                poly(name, params)
            }
            // REVIEW: その他何でもそのまま通していいのか?
            other => other,
        }
    }

    fn generalize_constraint<S: Into<Str>>(
        &self,
        name: S,
        constraint: &Constraint,
        bounds: &mut Set<TyBound>,
        lazy_inits: &mut Set<Str>,
    ) {
        let name = name.into();
        //  Quantify types with type boundaries only at the top level
        // トップレベルでのみ、型境界付きで量化する
        if !lazy_inits.contains(&name[..]) {
            lazy_inits.insert(name.clone());
            match constraint {
                Constraint::Sandwiched { sub, sup, .. } => {
                    let sub = self.generalize_t_inner(sub.clone(), bounds, lazy_inits);
                    let sup = self.generalize_t_inner(sup.clone(), bounds, lazy_inits);
                    // let bs = sub_bs.concat(sup_bs);
                    bounds.insert(TyBound::sandwiched(sub, mono_q(name.clone()), sup));
                }
                Constraint::TypeOf(t) => {
                    let t = self.generalize_t_inner(t.clone(), bounds, lazy_inits);
                    bounds.insert(TyBound::instance(Str::rc(&name[..]), t));
                }
                Constraint::Uninited => unreachable!(),
            }
        }
    }

    fn deref_tp(&self, tp: TyParam) -> TyCheckResult<TyParam> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let inner = fv.unwrap_linked();
                self.deref_tp(inner)
            }
            TyParam::FreeVar(_fv) if self.level == 0 => {
                Err(TyCheckError::dummy_infer_error(fn_name!(), line!()))
            }
            TyParam::Type(t) => Ok(TyParam::t(self.deref_tyvar(*t)?)),
            TyParam::App { name, mut args } => {
                for param in args.iter_mut() {
                    *param = self.deref_tp(mem::take(param))?;
                }
                Ok(TyParam::App { name, args })
            }
            TyParam::BinOp { .. } => todo!(),
            TyParam::UnaryOp { .. } => todo!(),
            TyParam::Array(_) | TyParam::Tuple(_) => todo!(),
            TyParam::MonoProj { .. }
            | TyParam::MonoQVar(_)
            | TyParam::PolyQVar { .. }
            | TyParam::Failure
                if self.level == 0 =>
            {
                Err(TyCheckError::dummy_infer_error(fn_name!(), line!()))
            }
            t => Ok(t),
        }
    }

    fn deref_constraint(&self, constraint: Constraint) -> TyCheckResult<Constraint> {
        match constraint {
            Constraint::Sandwiched {
                sub,
                sup,
                cyclicity: cyclic,
            } => {
                if cyclic.is_cyclic() {
                    return Err(TyCheckError::dummy_infer_error(fn_name!(), line!()));
                }
                Ok(Constraint::new_sandwiched(
                    self.deref_tyvar(sub)?,
                    self.deref_tyvar(sup)?,
                    cyclic,
                ))
            }
            Constraint::TypeOf(t) => Ok(Constraint::new_type_of(self.deref_tyvar(t)?)),
            _ => unreachable!(),
        }
    }

    /// e.g.
    /// ```erg
    /// deref_tyvar(?T(:> Never, <: Int)[n]): ?T => Int (if self.level <= n)
    /// deref_tyvar((Int)): (Int) => Int
    /// ```
    pub(crate) fn deref_tyvar(&self, t: Type) -> TyCheckResult<Type> {
        match t {
            // ?T(:> Nat, <: Int)[n] => Nat (self.level <= n)
            // ?T(:> Nat, <: Sub ?U(:> {1}))[n] => Nat
            // ?T(:> Never, <: Nat)[n] => Nat
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let constraint = fv.crack_constraint();
                let (sub_t, super_t) = constraint.get_sub_sup().unwrap();
                if self.rec_same_type_of(sub_t, super_t) {
                    self.unify(sub_t, super_t, None, None)?;
                    let t = if sub_t == &Never {
                        super_t.clone()
                    } else {
                        sub_t.clone()
                    };
                    drop(constraint);
                    fv.link(&t);
                    self.deref_tyvar(Type::FreeVar(fv))
                } else if self.level <= fv.level().unwrap() {
                    let new_t = if sub_t == &Never {
                        super_t.clone()
                    } else {
                        sub_t.clone()
                    };
                    drop(constraint);
                    fv.link(&new_t);
                    self.deref_tyvar(Type::FreeVar(fv))
                } else {
                    drop(constraint);
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::FreeVar(fv) if fv.is_unbound() => {
                if self.level == 0 {
                    match &*fv.crack_constraint() {
                        Constraint::TypeOf(_) => {
                            Err(TyCheckError::dummy_infer_error(fn_name!(), line!()))
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let new_constraint = fv.crack_constraint().clone();
                    let new_constraint = self.deref_constraint(new_constraint)?;
                    fv.update_constraint(new_constraint);
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.unwrap_linked();
                self.deref_tyvar(t)
            }
            Type::Poly { name, mut params } => {
                for param in params.iter_mut() {
                    *param = self.deref_tp(mem::take(param))?;
                }
                let t = Type::Poly { name, params };
                self.resolve_trait(t)
            }
            Type::Subr(mut subr) => {
                match &mut subr.kind {
                    SubrKind::FuncMethod(t) => {
                        *t = Box::new(self.deref_tyvar(mem::take(t))?);
                    }
                    SubrKind::ProcMethod { before, after } => {
                        *before = Box::new(self.deref_tyvar(mem::take(before))?);
                        if let Some(after) = after {
                            *after = Box::new(self.deref_tyvar(mem::take(after))?);
                        }
                    }
                    _ => {}
                }
                for param in subr.non_default_params.iter_mut() {
                    *param.typ_mut() = self.deref_tyvar(mem::take(param.typ_mut()))?;
                }
                if let Some(var_args) = &mut subr.var_params {
                    *var_args.typ_mut() = self.deref_tyvar(mem::take(var_args.typ_mut()))?;
                }
                for d_param in subr.default_params.iter_mut() {
                    *d_param.typ_mut() = self.deref_tyvar(mem::take(d_param.typ_mut()))?;
                }
                subr.return_t = Box::new(self.deref_tyvar(mem::take(&mut subr.return_t))?);
                Ok(Type::Subr(subr))
            }
            Type::Ref(t) => {
                let t = self.deref_tyvar(*t)?;
                Ok(ref_(t))
            }
            Type::RefMut(t) => {
                let t = self.deref_tyvar(*t)?;
                Ok(ref_mut(t))
            }
            Type::Callable { .. } => todo!(),
            Type::Record(mut rec) => {
                for (_, field) in rec.iter_mut() {
                    *field = self.deref_tyvar(mem::take(field))?;
                }
                Ok(Type::Record(rec))
            }
            Type::Refinement(refine) => {
                let t = self.deref_tyvar(*refine.t)?;
                // TODO: deref_predicate
                Ok(refinement(refine.var, t, refine.preds))
            }
            t => Ok(t),
        }
    }

    pub(crate) fn deref_toplevel(&mut self, mut hir: hir::HIR) -> TyCheckResult<hir::HIR> {
        self.level = 0;
        for chunk in hir.module.iter_mut() {
            self.deref_expr_t(chunk).map_err(|e| e)?;
        }
        Ok(hir)
    }

    fn deref_expr_t(&self, expr: &mut hir::Expr) -> TyCheckResult<()> {
        match expr {
            hir::Expr::Lit(_) => Ok(()),
            hir::Expr::Accessor(acc) => {
                let t = acc.ref_mut_t();
                *t = self.deref_tyvar(mem::take(t))?;
                match acc {
                    hir::Accessor::Attr(attr) => {
                        self.deref_expr_t(&mut attr.obj)?;
                    }
                    hir::Accessor::TupleAttr(attr) => {
                        self.deref_expr_t(&mut attr.obj)?;
                    }
                    hir::Accessor::Subscr(subscr) => {
                        self.deref_expr_t(&mut subscr.obj)?;
                        self.deref_expr_t(&mut subscr.index)?;
                    }
                    hir::Accessor::Local(_) | hir::Accessor::Public(_) => {}
                }
                Ok(())
            }
            hir::Expr::Array(array) => match array {
                hir::Array::Normal(arr) => {
                    arr.t = self.deref_tyvar(mem::take(&mut arr.t))?;
                    for elem in arr.elems.pos_args.iter_mut() {
                        self.deref_expr_t(&mut elem.expr)?;
                    }
                    Ok(())
                }
                _ => todo!(),
            },
            hir::Expr::Tuple(tuple) => match tuple {
                hir::Tuple::Normal(tup) => {
                    for elem in tup.elems.pos_args.iter_mut() {
                        self.deref_expr_t(&mut elem.expr)?;
                    }
                    Ok(())
                }
            },
            hir::Expr::Dict(_dict) => {
                todo!()
            }
            hir::Expr::Record(record) => {
                for attr in record.attrs.iter_mut() {
                    match &mut attr.sig {
                        hir::Signature::Var(var) => {
                            var.t = self.deref_tyvar(mem::take(&mut var.t))?;
                        }
                        hir::Signature::Subr(subr) => {
                            subr.t = self.deref_tyvar(mem::take(&mut subr.t))?;
                        }
                    }
                    for chunk in attr.body.block.iter_mut() {
                        self.deref_expr_t(chunk)?;
                    }
                }
                Ok(())
            }
            hir::Expr::BinOp(binop) => {
                let t = binop.signature_mut_t().unwrap();
                *t = self.deref_tyvar(mem::take(t))?;
                self.deref_expr_t(&mut binop.lhs)?;
                self.deref_expr_t(&mut binop.rhs)?;
                Ok(())
            }
            hir::Expr::UnaryOp(unaryop) => {
                let t = unaryop.signature_mut_t().unwrap();
                *t = self.deref_tyvar(mem::take(t))?;
                self.deref_expr_t(&mut unaryop.expr)?;
                Ok(())
            }
            hir::Expr::Call(call) => {
                let t = call.signature_mut_t().unwrap();
                *t = self.deref_tyvar(mem::take(t))?;
                for arg in call.args.pos_args.iter_mut() {
                    self.deref_expr_t(&mut arg.expr)?;
                }
                for arg in call.args.kw_args.iter_mut() {
                    self.deref_expr_t(&mut arg.expr)?;
                }
                Ok(())
            }
            hir::Expr::Decl(decl) => {
                decl.t = self.deref_tyvar(mem::take(&mut decl.t))?;
                Ok(())
            }
            hir::Expr::Def(def) => {
                match &mut def.sig {
                    hir::Signature::Var(var) => {
                        var.t = self.deref_tyvar(mem::take(&mut var.t))?;
                    }
                    hir::Signature::Subr(subr) => {
                        subr.t = self.deref_tyvar(mem::take(&mut subr.t))?;
                    }
                }
                for chunk in def.body.block.iter_mut() {
                    self.deref_expr_t(chunk)?;
                }
                Ok(())
            }
            hir::Expr::Lambda(lambda) => {
                lambda.t = self.deref_tyvar(mem::take(&mut lambda.t))?;
                for chunk in lambda.body.iter_mut() {
                    self.deref_expr_t(chunk)?;
                }
                Ok(())
            }
            hir::Expr::MethodDefs(method_defs) => {
                for def in method_defs.defs.iter_mut() {
                    match &mut def.sig {
                        hir::Signature::Var(var) => {
                            var.t = self.deref_tyvar(mem::take(&mut var.t))?;
                        }
                        hir::Signature::Subr(subr) => {
                            subr.t = self.deref_tyvar(mem::take(&mut subr.t))?;
                        }
                    }
                    for chunk in def.body.block.iter_mut() {
                        self.deref_expr_t(chunk)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn _occur(&self, _t: Type) -> TyCheckResult<Type> {
        todo!()
    }

    /// allow_divergence = trueにすると、Num型変数と±Infの単一化を許す
    pub(crate) fn unify_tp(
        &self,
        lhs: &TyParam,
        rhs: &TyParam,
        lhs_variance: Option<&Vec<Variance>>,
        allow_divergence: bool,
    ) -> TyCheckResult<()> {
        if lhs.has_no_unbound_var() && rhs.has_no_unbound_var() && lhs == rhs {
            return Ok(());
        }
        match (lhs, rhs) {
            (TyParam::Type(l), TyParam::Type(r)) => self.unify(l, r, None, None),
            (ltp @ TyParam::FreeVar(lfv), rtp @ TyParam::FreeVar(rfv))
                if lfv.is_unbound() && rfv.is_unbound() =>
            {
                if lfv.level().unwrap() > rfv.level().unwrap() {
                    lfv.link(rtp);
                } else {
                    rfv.link(ltp);
                }
                Ok(())
            }
            (TyParam::FreeVar(fv), tp) | (tp, TyParam::FreeVar(fv)) => {
                match &*fv.borrow() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.unify_tp(l, tp, lhs_variance, allow_divergence);
                    }
                    FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {}
                } // &fv is dropped
                let fv_t = fv
                    .borrow()
                    .constraint()
                    .unwrap()
                    .get_type()
                    .unwrap()
                    .clone(); // fvを参照しないよいにcloneする(あとでborrow_mutするため)
                let tp_t = self.get_tp_t(tp)?;
                if self.rec_supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if fv.level() < Some(self.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t, Cyclicity::Not);
                        if self.is_sub_constraint_of(
                            fv.borrow().constraint().unwrap(),
                            &new_constraint,
                        ) || fv.borrow().constraint().unwrap().get_type() == Some(&Type)
                        {
                            fv.update_constraint(new_constraint);
                        }
                    } else {
                        fv.link(tp);
                    }
                    Ok(())
                } else if allow_divergence
                    && (self.eq_tp(tp, &TyParam::value(Inf))
                        || self.eq_tp(tp, &TyParam::value(NegInf)))
                    && self.rec_subtype_of(&fv_t, &mono("Num"))
                {
                    fv.link(tp);
                    Ok(())
                } else {
                    Err(TyCheckError::unreachable(fn_name!(), line!()))
                }
            }
            (TyParam::UnaryOp { op: lop, val: lval }, TyParam::UnaryOp { op: rop, val: rval })
                if lop == rop =>
            {
                self.unify_tp(lval, rval, lhs_variance, allow_divergence)
            }
            (
                TyParam::BinOp { op: lop, lhs, rhs },
                TyParam::BinOp {
                    op: rop,
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if lop == rop => {
                self.unify_tp(lhs, lhs2, lhs_variance, allow_divergence)?;
                self.unify_tp(rhs, rhs2, lhs_variance, allow_divergence)
            }
            (l, r) => panic!("type-parameter unification failed:\nl:{l}\nr: {r}"),
        }
    }

    fn reunify_tp(&self, before: &TyParam, after: &TyParam) -> TyCheckResult<()> {
        match (before, after) {
            (TyParam::Value(ValueObj::Mut(l)), TyParam::Value(ValueObj::Mut(r))) => {
                *l.borrow_mut() = r.borrow().clone();
                Ok(())
            }
            (TyParam::Value(ValueObj::Mut(l)), TyParam::Value(r)) => {
                *l.borrow_mut() = r.clone();
                Ok(())
            }
            (TyParam::Type(l), TyParam::Type(r)) => self.reunify(l, r, None, None),
            (TyParam::UnaryOp { op: lop, val: lval }, TyParam::UnaryOp { op: rop, val: rval })
                if lop == rop =>
            {
                self.reunify_tp(lval, rval)
            }
            (
                TyParam::BinOp { op: lop, lhs, rhs },
                TyParam::BinOp {
                    op: rop,
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if lop == rop => {
                self.reunify_tp(lhs, lhs2)?;
                self.reunify_tp(rhs, rhs2)
            }
            (l, r) if self.eq_tp(l, r) => Ok(()),
            (l, r) => panic!("type-parameter re-unification failed:\nl: {l}\nr: {r}"),
        }
    }

    /// predは正規化されているとする
    fn unify_pred(&self, l_pred: &Predicate, r_pred: &Predicate) -> TyCheckResult<()> {
        match (l_pred, r_pred) {
            (Pred::Value(_), Pred::Value(_)) | (Pred::Const(_), Pred::Const(_)) => Ok(()),
            (Pred::Equal { rhs, .. }, Pred::Equal { rhs: rhs2, .. })
            | (Pred::GreaterEqual { rhs, .. }, Pred::GreaterEqual { rhs: rhs2, .. })
            | (Pred::LessEqual { rhs, .. }, Pred::LessEqual { rhs: rhs2, .. })
            | (Pred::NotEqual { rhs, .. }, Pred::NotEqual { rhs: rhs2, .. }) => {
                self.unify_tp(rhs, rhs2, None, false)
            }
            (Pred::And(l1, r1), Pred::And(l2, r2))
            | (Pred::Or(l1, r1), Pred::Or(l2, r2))
            | (Pred::Not(l1, r1), Pred::Not(l2, r2)) => {
                match (self.unify_pred(l1, l2), self.unify_pred(r1, r2)) {
                    (Ok(()), Ok(())) => Ok(()),
                    (Ok(()), Err(e)) | (Err(e), Ok(())) | (Err(e), Err(_)) => Err(e),
                }
            }
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
                        self.unify_tp(rhs, ge_rhs, None, false)?;
                        self.unify_tp(le_rhs, &TyParam::value(Inf), None, true)
                    }
                    _ => Err(TyCheckError::pred_unification_error(
                        line!() as usize,
                        l_pred,
                        r_pred,
                        self.caused_by(),
                    )),
                }
            }
            (Pred::LessEqual { rhs, .. }, Pred::And(l, r))
            | (Pred::And(l, r), Pred::LessEqual { rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual { rhs: ge_rhs, .. }, Pred::LessEqual { rhs: le_rhs, .. })
                | (Pred::LessEqual { rhs: le_rhs, .. }, Pred::GreaterEqual { rhs: ge_rhs, .. }) => {
                    self.unify_tp(rhs, le_rhs, None, false)?;
                    self.unify_tp(ge_rhs, &TyParam::value(NegInf), None, true)
                }
                _ => Err(TyCheckError::pred_unification_error(
                    line!() as usize,
                    l_pred,
                    r_pred,
                    self.caused_by(),
                )),
            },
            (Pred::Equal { rhs, .. }, Pred::And(l, r))
            | (Pred::And(l, r), Pred::Equal { rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual { rhs: ge_rhs, .. }, Pred::LessEqual { rhs: le_rhs, .. })
                | (Pred::LessEqual { rhs: le_rhs, .. }, Pred::GreaterEqual { rhs: ge_rhs, .. }) => {
                    self.unify_tp(rhs, le_rhs, None, false)?;
                    self.unify_tp(rhs, ge_rhs, None, false)
                }
                _ => Err(TyCheckError::pred_unification_error(
                    line!() as usize,
                    l_pred,
                    r_pred,
                    self.caused_by(),
                )),
            },
            _ => Err(TyCheckError::pred_unification_error(
                line!() as usize,
                l_pred,
                r_pred,
                self.caused_by(),
            )),
        }
    }

    /// By default, all type variables are instances of Class ('T: Nominal)
    /// So `unify(?T, Int); unify(?T, Bool)` will causes an error
    /// To bypass the constraint, you need to specify `'T: Structural` in the type bounds
    pub(crate) fn unify(
        &self,
        lhs_t: &Type,
        rhs_t: &Type,
        lhs_loc: Option<Location>,
        rhs_loc: Option<Location>,
    ) -> TyCheckResult<()> {
        if lhs_t.has_no_unbound_var()
            && rhs_t.has_no_unbound_var()
            && self.rec_supertype_of(lhs_t, rhs_t)
        {
            return Ok(());
        }
        match (lhs_t, rhs_t) {
            // unify(?T[2], ?U[3]): ?U[3] => ?T[2]
            // bind the higher level var to lower one
            (lt @ Type::FreeVar(lfv), rt @ Type::FreeVar(rfv))
                if lfv.is_unbound() && rfv.is_unbound() =>
            {
                if lfv.constraint_is_typeof() && !rfv.constraint_is_typeof() {
                    lfv.update_constraint(rfv.crack_constraint().clone());
                } else if rfv.constraint_is_typeof() && !lfv.constraint_is_typeof() {
                    rfv.update_constraint(lfv.crack_constraint().clone());
                }
                if lfv.level().unwrap() > rfv.level().unwrap() {
                    lfv.link(rt);
                } else {
                    rfv.link(lt);
                }
                Ok(())
            }
            // unify(?L(<: Add(?R, ?O)), Nat): (?R => Nat, ?O => Nat, ?L => Nat)
            // unify(?A(<: Mutate), [?T; 0]): (?A => [?T; 0])
            (Type::FreeVar(fv), t) | (t, Type::FreeVar(fv)) => {
                match &mut *fv.borrow_mut() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.unify(l, t, lhs_loc, rhs_loc);
                    }
                    FreeKind::Unbound {
                        lev, constraint, ..
                    }
                    | FreeKind::NamedUnbound {
                        lev, constraint, ..
                    } => {
                        t.update_level(*lev);
                        // TODO: constraint.type_of()
                        if let Some(sup) = constraint.get_super_mut() {
                            // 下のような場合は制約を弱化する
                            // unify(?T(<: Nat), Int): (?T(<: Int))
                            if self.rec_subtype_of(sup, t) {
                                *sup = t.clone();
                            } else {
                                self.sub_unify(t, sup, rhs_loc, lhs_loc, None)?;
                            }
                        }
                    }
                } // &fv is dropped
                let new_constraint = Constraint::new_subtype_of(t.clone(), fv.cyclicity());
                // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                // fv == ?T(: Type)の場合は?T(<: U)にする
                if fv.level() < Some(self.level) {
                    if self.is_sub_constraint_of(fv.borrow().constraint().unwrap(), &new_constraint)
                        || fv.borrow().constraint().unwrap().get_type() == Some(&Type)
                    {
                        fv.update_constraint(new_constraint);
                    }
                } else {
                    fv.link(t);
                }
                Ok(())
            }
            (Type::Refinement(l), Type::Refinement(r)) => {
                if !self.structural_supertype_of(&l.t, &r.t)
                    && !self.structural_supertype_of(&r.t, &l.t)
                {
                    return Err(TyCheckError::unification_error(
                        line!() as usize,
                        lhs_t,
                        rhs_t,
                        lhs_loc,
                        rhs_loc,
                        self.caused_by(),
                    ));
                }
                // FIXME: 正規化する
                for l_pred in l.preds.iter() {
                    for r_pred in r.preds.iter() {
                        self.unify_pred(l_pred, r_pred)?;
                    }
                }
                Ok(())
            }
            (Type::Refinement(_), r) => {
                let rhs_t = self.into_refinement(r.clone());
                self.unify(lhs_t, &Type::Refinement(rhs_t), lhs_loc, rhs_loc)
            }
            (l, Type::Refinement(_)) => {
                let lhs_t = self.into_refinement(l.clone());
                self.unify(&Type::Refinement(lhs_t), rhs_t, lhs_loc, rhs_loc)
            }
            (Type::Subr(ls), Type::Subr(rs)) if ls.kind.same_kind_as(&rs.kind) => {
                if let (Some(l), Some(r)) = (ls.kind.self_t(), rs.kind.self_t()) {
                    self.unify(l, r, lhs_loc, rhs_loc)?;
                }
                for (l, r) in ls
                    .non_default_params
                    .iter()
                    .zip(rs.non_default_params.iter())
                {
                    self.unify(l.typ(), r.typ(), lhs_loc, rhs_loc)?;
                }
                for (l, r) in ls.var_params.as_ref().zip(rs.var_params.as_ref()) {
                    self.unify(l.typ(), r.typ(), lhs_loc, rhs_loc)?;
                }
                for lpt in ls.default_params.iter() {
                    if let Some(rpt) = rs
                        .default_params
                        .iter()
                        .find(|rpt| rpt.name() == lpt.name())
                    {
                        self.unify(lpt.typ(), rpt.typ(), lhs_loc, rhs_loc)?;
                    } else {
                        todo!()
                    }
                }
                self.unify(&ls.return_t, &rs.return_t, lhs_loc, rhs_loc)
            }
            (Type::Ref(l), Type::Ref(r)) | (Type::RefMut(l), Type::RefMut(r)) => {
                self.unify(l, r, lhs_loc, rhs_loc)
            }
            // REVIEW:
            (Type::Ref(l), r) | (Type::RefMut(l), r) => self.unify(l, r, lhs_loc, rhs_loc),
            (l, Type::Ref(r)) | (l, Type::RefMut(r)) => self.unify(l, r, lhs_loc, rhs_loc),
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
                    return Err(TyCheckError::unification_error(
                        line!() as usize,
                        lhs_t,
                        rhs_t,
                        lhs_loc,
                        rhs_loc,
                        self.caused_by(),
                    ));
                }
                for (l, r) in lps.iter().zip(rps.iter()) {
                    self.unify_tp(l, r, None, false)?;
                }
                Ok(())
            }
            (Type::Poly { name: _, params: _ }, _r) => {
                todo!()
            }
            (l, r) => Err(TyCheckError::unification_error(
                line!() as usize,
                l,
                r,
                lhs_loc,
                rhs_loc,
                self.caused_by(),
            )),
        }
    }

    /// T: Array(Int, !0), U: Array(Int, !1)
    /// reunify(T, U):
    /// T: Array(Int, !1), U: Array(Int, !1)
    pub(crate) fn reunify(
        &self,
        before_t: &Type,
        after_t: &Type,
        bef_loc: Option<Location>,
        aft_loc: Option<Location>,
    ) -> TyCheckResult<()> {
        match (before_t, after_t) {
            (Type::FreeVar(fv), r) if fv.is_linked() => {
                self.reunify(&fv.crack(), r, bef_loc, aft_loc)
            }
            (l, Type::FreeVar(fv)) if fv.is_linked() => {
                self.reunify(l, &fv.crack(), bef_loc, aft_loc)
            }
            (Type::Ref(l), Type::Ref(r)) | (Type::RefMut(l), Type::RefMut(r)) => {
                self.reunify(l, r, bef_loc, aft_loc)
            }
            // REVIEW:
            (Type::Ref(l), r) | (Type::RefMut(l), r) => self.reunify(l, r, bef_loc, aft_loc),
            (l, Type::Ref(r)) | (l, Type::RefMut(r)) => self.reunify(l, r, bef_loc, aft_loc),
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
                        line!() as usize,
                        &before_t,
                        after_t,
                        bef_loc,
                        aft_loc,
                        self.caused_by(),
                    ));
                }
                for (l, r) in lps.iter().zip(rps.iter()) {
                    self.reunify_tp(l, r)?;
                }
                Ok(())
            }
            (l, r) if self.rec_same_type_of(l, r) => Ok(()),
            (l, r) => Err(TyCheckError::re_unification_error(
                line!() as usize,
                l,
                r,
                bef_loc,
                aft_loc,
                self.caused_by(),
            )),
        }
    }

    /// Assuming that `sub` is a subtype of `sup`, fill in the type variable to satisfy the assumption
    ///
    /// When comparing arguments and parameter, the left side (`sub`) is the argument (found) and the right side (`sup`) is the parameter (expected)
    ///
    /// The parameter type must be a supertype of the argument type
    /// ```erg
    /// sub_unify({I: Int | I == 0}, ?T(<: Ord)): (/* OK */)
    /// sub_unify(Int, ?T(:> Nat)): (?T :> Int)
    /// sub_unify(Nat, ?T(:> Int)): (/* OK */)
    /// sub_unify(Nat, Add(?R)): (?R => Nat, Nat.AddO => Nat)
    /// sub_unify([?T; 0], Mutate): (/* OK */)
    /// ```
    pub(crate) fn sub_unify(
        &self,
        maybe_sub: &Type,
        maybe_sup: &Type,
        sub_loc: Option<Location>,
        sup_loc: Option<Location>,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        erg_common::log!(info "trying sub_unify:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
        // In this case, there is no new information to be gained
        // この場合、特に新しく得られる情報はない
        if maybe_sub == &Type::Never || maybe_sup == &Type::Obj || maybe_sup == maybe_sub {
            return Ok(());
        }
        let maybe_sub_is_sub = self.rec_subtype_of(maybe_sub, maybe_sup);
        if maybe_sub.has_no_unbound_var() && maybe_sup.has_no_unbound_var() && maybe_sub_is_sub {
            return Ok(());
        }
        if !maybe_sub_is_sub {
            let loc = sub_loc.or(sup_loc).unwrap_or(Location::Unknown);
            return Err(TyCheckError::type_mismatch_error(
                line!() as usize,
                loc,
                self.caused_by(),
                param_name.unwrap_or(&Str::ever("_")),
                maybe_sup,
                maybe_sub,
                self.get_type_mismatch_hint(maybe_sup, maybe_sub),
            ));
        }
        match (maybe_sub, maybe_sup) {
            (Type::FreeVar(lfv), _) if lfv.is_linked() =>
                self.sub_unify(&lfv.crack(), maybe_sup, sub_loc, sup_loc, param_name),
            (_, Type::FreeVar(rfv)) if rfv.is_linked() =>
                self.sub_unify(maybe_sub, &rfv.crack(), sub_loc, sup_loc, param_name),
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
                let (lsub, lsup) = lfv.get_bound_types().unwrap();
                let l_cyc = lfv.cyclicity();
                let (rsub, rsup) = rfv.get_bound_types().unwrap();
                let r_cyc = rfv.cyclicity();
                let cyclicity = l_cyc.combine(r_cyc);
                let intersec = self.rec_intersection(&lsup, &rsup);
                let new_constraint = if intersec != Type::Never {
                    Constraint::new_sandwiched(self.rec_union(&lsub, &rsub), intersec, cyclicity)
                } else {
                    return Err(TyCheckError::subtyping_error(
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        sub_loc,
                        sup_loc,
                        self.caused_by(),
                    ));
                };
                if lfv.level().unwrap() <= rfv.level().unwrap() {
                    lfv.update_constraint(new_constraint);
                    rfv.link(maybe_sub);
                } else {
                    rfv.update_constraint(new_constraint);
                    lfv.link(maybe_sup);
                }
                return Ok(())
            }
            (_, Type::FreeVar(rfv)) if rfv.is_unbound() => {
                // NOTE: cannot `borrow_mut` because of cycle reference
                let rfv_ref = unsafe { rfv.as_ptr().as_mut().unwrap() };
                match rfv_ref {
                    FreeKind::NamedUnbound { constraint, .. }
                    | FreeKind::Unbound { constraint, .. } => match constraint {
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
                        Constraint::Sandwiched { sub, sup, cyclicity } => {
                            let judge = match cyclicity {
                                Cyclicity::Super => self.cyclic_supertype_of(rfv, maybe_sub),
                                Cyclicity::Not => self.rec_supertype_of(sup, maybe_sub),
                                _ => todo!(),
                            };
                            if !judge {
                                return Err(TyCheckError::subtyping_error(
                                    line!() as usize,
                                    maybe_sub,
                                    sup, // TODO: this?
                                    sub_loc,
                                    sup_loc,
                                    self.caused_by(),
                                ));
                            }
                            if let Some(new_sub) = self.rec_max(maybe_sub, sub) {
                                *constraint =
                                    Constraint::new_sandwiched(new_sub.clone(), mem::take(sup), *cyclicity);
                            } else {
                                let new_sub = self.rec_union(maybe_sub, sub);
                                *constraint = Constraint::new_sandwiched(new_sub, mem::take(sup), *cyclicity);
                            }
                        }
                        // sub_unify(Nat, ?T(: Type)): (/* ?T(:> Nat) */)
                        Constraint::TypeOf(ty) => {
                            if self.rec_supertype_of(&Type, ty) {
                                *constraint = Constraint::new_supertype_of(maybe_sub.clone(), Cyclicity::Not);
                            } else {
                                todo!()
                            }
                        }
                        _ => unreachable!(),
                    },
                    _ => {}
                }
                return Ok(());
            }
            (Type::FreeVar(lfv), _) if lfv.is_unbound() => {
                let lfv_ref = &mut *lfv.borrow_mut();
                match lfv_ref {
                    FreeKind::NamedUnbound { constraint, .. }
                    | FreeKind::Unbound { constraint, .. } => match constraint {
                        // sub !<: r => Error
                        // * sub_unify(?T(:> Int,   <: _), Nat): (/* Error */)
                        // * sub_unify(?T(:> Nat,   <: _), Str): (/* Error */)
                        // sup !:> r => Error
                        // * sub_unify(?T(:> _, <: Str), Int): (/* Error */)
                        // * sub_unify(?T(:> _, <: Int), Nat): (/* Error */)
                        // sub <: r, sup :> r => sup = min(sup, r) if min exists
                        // * sub_unify(?T(:> Never, <: Nat), Int): (/* OK */)
                        // * sub_unify(?T(:> Nat,   <: Obj), Int): (?T(:> Nat,   <: Int))
                        // sup = union(sup, r) if min does not exist
                        // * sub_unify(?T(:> Never, <: {1}), {0}): (?T(:> Never, <: {0, 1}))
                        Constraint::Sandwiched { sub, sup, cyclicity } => {
                            if !self.rec_subtype_of(sub, maybe_sup) || !self.rec_supertype_of(sup, maybe_sup) {
                                return Err(TyCheckError::subtyping_error(
                                    line!() as usize,
                                    sub,
                                    maybe_sup,
                                    sub_loc,
                                    sup_loc,
                                    self.caused_by(),
                                ));
                            }
                            if let Some(new_sup) = self.rec_min(sup, maybe_sup) {
                                *constraint =
                                    Constraint::new_sandwiched(mem::take(sub), new_sup.clone(), *cyclicity);
                            } else {
                                let new_sup = self.rec_union(sup, maybe_sup);
                                *constraint = Constraint::new_sandwiched(mem::take(sub), new_sup, *cyclicity);
                            }
                        }
                        // sub_unify(?T(: Type), Int): (?T(<: Int))
                        Constraint::TypeOf(ty) => {
                            if self.rec_supertype_of(&Type, ty) {
                                *constraint = Constraint::new_subtype_of(maybe_sup.clone(), Cyclicity::Not);
                            } else {
                                todo!()
                            }
                        }
                        _ => unreachable!(),
                    },
                    _ => {}
                }
                return Ok(());
            }
            (Type::FreeVar(_fv), _r) => todo!(),
            (Type::Record(lrec), Type::Record(rrec)) => {
                for (k, l) in lrec.iter() {
                    if let Some(r) = rrec.get(k) {
                        self.sub_unify(l, r, sub_loc, sup_loc, param_name)?;
                    } else {
                        return Err(TyCheckError::subtyping_error(
                            line!() as usize,
                            maybe_sub,
                            maybe_sup,
                            sub_loc,
                            sup_loc,
                            self.caused_by(),
                        ));
                    }
                }
                return Ok(());
            }
            (Type::Subr(lsub), Type::Subr(rsub)) => {
                for lpt in lsub.default_params.iter() {
                    if let Some(rpt) = rsub.default_params.iter().find(|rpt| rpt.name() == lpt.name()) {
                        self.unify(lpt.typ(), rpt.typ(), sub_loc, sup_loc)?;
                    } else { todo!() }
                }
                lsub.non_default_params.iter().zip(rsub.non_default_params.iter()).try_for_each(
                    |(l, r)| self.unify(l.typ(), r.typ(), sub_loc, sup_loc),
                )?;
                self.unify(&lsub.return_t, &rsub.return_t, sub_loc, sup_loc)?;
                return Ok(());
            }
            (Type::MonoProj { .. }, _) => todo!(),
            (_, Type::MonoProj { .. }) => todo!(),
            (Refinement(_), Refinement(_)) => todo!(),
            _ => todo!("{maybe_sub} can be a subtype of {maybe_sup}, but failed to semi-unify (or existential types are not supported)"),
        }
    }
}
