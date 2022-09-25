//! provides type variable related operations
use std::mem;
use std::option::Option;

use erg_common::error::Location;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{assume_unreachable, fn_name, log, set};

use erg_type::constructors::*;
use erg_type::free::{Constraint, Cyclicity, FreeKind};
use erg_type::typaram::TyParam;
use erg_type::value::ValueObj;
use erg_type::{HasType, Predicate, TyBound, Type};

use crate::context::{Context, Variance};
// use crate::context::instantiate::TyVarContext;
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
    fn _independentise(_t: Type, _ts: &[Type]) -> Type {
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
        if bounds.is_empty() {
            maybe_unbound_t
        } else {
            // NOTE: `?T(<: TraitX) -> Int` should be `TraitX -> Int`
            // However, the current Erg cannot handle existential types, so it quantifies anyway
            /*if !maybe_unbound_t.return_t().unwrap().has_qvar() {
                let mut tv_ctx = TyVarContext::new(self.level, bounds.clone(), self);
                let inst = Self::instantiate_t(
                    maybe_unbound_t,
                    &mut tv_ctx,
                    Location::Unknown,
                )
                .unwrap();
                inst.lift();
                self.deref_tyvar(inst, Location::Unknown).unwrap()
            } else { */
            quant(maybe_unbound_t, bounds)
            // }
        }
    }

    /// see doc/LANG/compiler/inference.md#一般化 for details
    /// ```python
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
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|x| *x),
                    subr.default_params,
                    return_t,
                )
            }
            Callable { .. } => todo!(),
            Ref(t) => ref_(self.generalize_t_inner(*t, bounds, lazy_inits)),
            RefMut { before, after } => {
                let after = after.map(|aft| self.generalize_t_inner(*aft, bounds, lazy_inits));
                ref_mut(self.generalize_t_inner(*before, bounds, lazy_inits), after)
            }
            Poly { name, mut params } => {
                let params = params
                    .iter_mut()
                    .map(|p| self.generalize_tp(mem::take(p), bounds, lazy_inits))
                    .collect::<Vec<_>>();
                poly(name, params)
            }
            MonoProj { lhs, rhs } => {
                let lhs = self.generalize_t_inner(*lhs, bounds, lazy_inits);
                mono_proj(lhs, rhs)
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
                    bounds.insert(TyBound::sandwiched(sub, mono_q(name), sup));
                }
                Constraint::TypeOf(t) => {
                    let t = self.generalize_t_inner(t.clone(), bounds, lazy_inits);
                    bounds.insert(TyBound::instance(Str::rc(&name[..]), t));
                }
                Constraint::Uninited => unreachable!(),
            }
        }
    }

    fn deref_tp(&self, tp: TyParam, loc: Location) -> TyCheckResult<TyParam> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let inner = fv.unwrap_linked();
                self.deref_tp(inner, loc)
            }
            TyParam::FreeVar(_fv) if self.level == 0 => {
                Err(TyCheckError::dummy_infer_error(fn_name!(), line!()))
            }
            TyParam::Type(t) => Ok(TyParam::t(self.deref_tyvar(*t, loc)?)),
            TyParam::App { name, mut args } => {
                for param in args.iter_mut() {
                    *param = self.deref_tp(mem::take(param), loc)?;
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

    fn deref_constraint(&self, constraint: Constraint, loc: Location) -> TyCheckResult<Constraint> {
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
                    self.deref_tyvar(sub, loc)?,
                    self.deref_tyvar(sup, loc)?,
                    cyclic,
                ))
            }
            Constraint::TypeOf(t) => Ok(Constraint::new_type_of(self.deref_tyvar(t, loc)?)),
            _ => unreachable!(),
        }
    }

    /// e.g.
    /// ```python
    // ?T(:> Nat, <: Int)[n] ==> Nat (self.level <= n)
    // ?T(:> Nat, <: Sub(?U(:> {1}))) ==> Nat
    // ?T(:> Nat, <: Sub(Str)) ==> Error!
    // ?T(:> {1, "a"}, <: Eq(?T(:> {1, "a"}, ...)) ==> Error!
    // ```
    pub(crate) fn deref_tyvar(&self, t: Type, loc: Location) -> TyCheckResult<Type> {
        match t {
            // ?T(:> Nat, <: Int)[n] ==> Nat (self.level <= n)
            // ?T(:> Nat, <: Sub ?U(:> {1}))[n] ==> Nat
            // ?T(:> Nat, <: Sub(Str)) ==> Error!
            // ?T(:> {1, "a"}, <: Eq(?T(:> {1, "a"}, ...)) ==> Error!
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let constraint = fv.crack_constraint();
                let (sub_t, super_t) = constraint.get_sub_sup().unwrap();
                if self.level <= fv.level().unwrap() {
                    // REVIEW: Even if type constraints can be satisfied, implementation may not exist
                    if self.subtype_of(sub_t, super_t) {
                        Ok(sub_t.clone())
                    } else {
                        Err(TyCheckError::subtyping_error(
                            line!() as usize,
                            &self.deref_tyvar(sub_t.clone(), loc)?,
                            &self.deref_tyvar(super_t.clone(), loc)?,
                            None,
                            Some(loc),
                            self.caused_by(),
                        ))
                    }
                } else {
                    // no dereference at this point
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
                    let new_constraint = self.deref_constraint(new_constraint, loc)?;
                    fv.update_constraint(new_constraint);
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.unwrap_linked();
                self.deref_tyvar(t, loc)
            }
            Type::Poly { name, mut params } => {
                for param in params.iter_mut() {
                    *param = self.deref_tp(mem::take(param), loc)?;
                }
                Ok(Type::Poly { name, params })
            }
            Type::Subr(mut subr) => {
                for param in subr.non_default_params.iter_mut() {
                    *param.typ_mut() = self.deref_tyvar(mem::take(param.typ_mut()), loc)?;
                }
                if let Some(var_args) = &mut subr.var_params {
                    *var_args.typ_mut() = self.deref_tyvar(mem::take(var_args.typ_mut()), loc)?;
                }
                for d_param in subr.default_params.iter_mut() {
                    *d_param.typ_mut() = self.deref_tyvar(mem::take(d_param.typ_mut()), loc)?;
                }
                subr.return_t = Box::new(self.deref_tyvar(mem::take(&mut subr.return_t), loc)?);
                Ok(Type::Subr(subr))
            }
            Type::Ref(t) => {
                let t = self.deref_tyvar(*t, loc)?;
                Ok(ref_(t))
            }
            Type::RefMut { before, after } => {
                let before = self.deref_tyvar(*before, loc)?;
                let after = if let Some(after) = after {
                    Some(self.deref_tyvar(*after, loc)?)
                } else {
                    None
                };
                Ok(ref_mut(before, after))
            }
            Type::Callable { .. } => todo!(),
            Type::Record(mut rec) => {
                for (_, field) in rec.iter_mut() {
                    *field = self.deref_tyvar(mem::take(field), loc)?;
                }
                Ok(Type::Record(rec))
            }
            Type::Refinement(refine) => {
                let t = self.deref_tyvar(*refine.t, loc)?;
                // TODO: deref_predicate
                Ok(refinement(refine.var, t, refine.preds))
            }
            t => Ok(t),
        }
    }

    /// Check if all types are resolvable (if traits, check if an implementation exists)
    /// And replace them if resolvable
    pub(crate) fn resolve(&mut self, mut hir: hir::HIR) -> TyCheckResult<hir::HIR> {
        self.level = 0;
        for chunk in hir.module.iter_mut() {
            self.resolve_expr_t(chunk)?;
        }
        Ok(hir)
    }

    fn resolve_expr_t(&self, expr: &mut hir::Expr) -> TyCheckResult<()> {
        match expr {
            hir::Expr::Lit(_) => Ok(()),
            hir::Expr::Accessor(acc) => {
                let loc = acc.loc();
                let t = acc.ref_mut_t();
                *t = self.deref_tyvar(mem::take(t), loc)?;
                match acc {
                    hir::Accessor::Attr(attr) => {
                        self.resolve_expr_t(&mut attr.obj)?;
                    }
                    hir::Accessor::TupleAttr(attr) => {
                        self.resolve_expr_t(&mut attr.obj)?;
                    }
                    hir::Accessor::Subscr(subscr) => {
                        self.resolve_expr_t(&mut subscr.obj)?;
                        self.resolve_expr_t(&mut subscr.index)?;
                    }
                    hir::Accessor::Ident(_) => {}
                }
                Ok(())
            }
            hir::Expr::Array(array) => match array {
                hir::Array::Normal(arr) => {
                    let loc = arr.loc();
                    arr.t = self.deref_tyvar(mem::take(&mut arr.t), loc)?;
                    for elem in arr.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr)?;
                    }
                    Ok(())
                }
                _ => todo!(),
            },
            hir::Expr::Tuple(tuple) => match tuple {
                hir::Tuple::Normal(tup) => {
                    for elem in tup.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr)?;
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
                            var.t = self.deref_tyvar(mem::take(&mut var.t), var.loc())?;
                        }
                        hir::Signature::Subr(subr) => {
                            subr.t = self.deref_tyvar(mem::take(&mut subr.t), subr.loc())?;
                        }
                    }
                    for chunk in attr.body.block.iter_mut() {
                        self.resolve_expr_t(chunk)?;
                    }
                }
                Ok(())
            }
            hir::Expr::BinOp(binop) => {
                let loc = binop.loc();
                let t = binop.signature_mut_t().unwrap();
                *t = self.deref_tyvar(mem::take(t), loc)?;
                self.resolve_expr_t(&mut binop.lhs)?;
                self.resolve_expr_t(&mut binop.rhs)?;
                Ok(())
            }
            hir::Expr::UnaryOp(unaryop) => {
                let loc = unaryop.loc();
                let t = unaryop.signature_mut_t().unwrap();
                *t = self.deref_tyvar(mem::take(t), loc)?;
                self.resolve_expr_t(&mut unaryop.expr)?;
                Ok(())
            }
            hir::Expr::Call(call) => {
                let loc = call.loc();
                let t = call.signature_mut_t().unwrap();
                *t = self.deref_tyvar(mem::take(t), loc)?;
                for arg in call.args.pos_args.iter_mut() {
                    self.resolve_expr_t(&mut arg.expr)?;
                }
                for arg in call.args.kw_args.iter_mut() {
                    self.resolve_expr_t(&mut arg.expr)?;
                }
                Ok(())
            }
            hir::Expr::Decl(decl) => {
                decl.t = self.deref_tyvar(mem::take(&mut decl.t), decl.loc())?;
                Ok(())
            }
            hir::Expr::Def(def) => {
                match &mut def.sig {
                    hir::Signature::Var(var) => {
                        var.t = self.deref_tyvar(mem::take(&mut var.t), var.loc())?;
                    }
                    hir::Signature::Subr(subr) => {
                        subr.t = self.deref_tyvar(mem::take(&mut subr.t), subr.loc())?;
                    }
                }
                for chunk in def.body.block.iter_mut() {
                    self.resolve_expr_t(chunk)?;
                }
                Ok(())
            }
            hir::Expr::Lambda(lambda) => {
                lambda.t = self.deref_tyvar(mem::take(&mut lambda.t), lambda.loc())?;
                for chunk in lambda.body.iter_mut() {
                    self.resolve_expr_t(chunk)?;
                }
                Ok(())
            }
            hir::Expr::ClassDef(class_def) => {
                for def in class_def.methods.iter_mut() {
                    self.resolve_expr_t(def)?;
                }
                Ok(())
            }
            hir::Expr::AttrDef(attr_def) => {
                // REVIEW: attr_def.attr is not dereferenced
                for chunk in attr_def.block.iter_mut() {
                    self.resolve_expr_t(chunk)?;
                }
                Ok(())
            }
            hir::Expr::TypeAsc(tasc) => self.resolve_expr_t(&mut tasc.expr),
            hir::Expr::Code(chunks) | hir::Expr::Compound(chunks) => {
                for chunk in chunks.iter_mut() {
                    self.resolve_expr_t(chunk)?;
                }
                Ok(())
            }
        }
    }

    fn _occur(&self, _t: Type) -> TyCheckResult<Type> {
        todo!()
    }

    /// allow_divergence = trueにすると、Num型変数と±Infの単一化を許す
    pub(crate) fn sub_unify_tp(
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
            (TyParam::Type(l), TyParam::Type(r)) => self.sub_unify(l, r, None, None, None),
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
                        return self.sub_unify_tp(l, tp, lhs_variance, allow_divergence);
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
                if self.supertype_of(&fv_t, &tp_t) {
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
                    && self.subtype_of(&fv_t, &builtin_mono("Num"))
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
                self.sub_unify_tp(lval, rval, lhs_variance, allow_divergence)
            }
            (
                TyParam::BinOp { op: lop, lhs, rhs },
                TyParam::BinOp {
                    op: rop,
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if lop == rop => {
                self.sub_unify_tp(lhs, lhs2, lhs_variance, allow_divergence)?;
                self.sub_unify_tp(rhs, rhs2, lhs_variance, allow_divergence)
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
    fn _sub_unify_pred(&self, l_pred: &Predicate, r_pred: &Predicate) -> TyCheckResult<()> {
        match (l_pred, r_pred) {
            (Pred::Value(_), Pred::Value(_)) | (Pred::Const(_), Pred::Const(_)) => Ok(()),
            (Pred::Equal { rhs, .. }, Pred::Equal { rhs: rhs2, .. })
            | (Pred::GreaterEqual { rhs, .. }, Pred::GreaterEqual { rhs: rhs2, .. })
            | (Pred::LessEqual { rhs, .. }, Pred::LessEqual { rhs: rhs2, .. })
            | (Pred::NotEqual { rhs, .. }, Pred::NotEqual { rhs: rhs2, .. }) => {
                self.sub_unify_tp(rhs, rhs2, None, false)
            }
            (Pred::And(l1, r1), Pred::And(l2, r2))
            | (Pred::Or(l1, r1), Pred::Or(l2, r2))
            | (Pred::Not(l1, r1), Pred::Not(l2, r2)) => {
                match (self._sub_unify_pred(l1, l2), self._sub_unify_pred(r1, r2)) {
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
                        self.sub_unify_tp(rhs, ge_rhs, None, false)?;
                        self.sub_unify_tp(le_rhs, &TyParam::value(Inf), None, true)
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
                    self.sub_unify_tp(rhs, le_rhs, None, false)?;
                    self.sub_unify_tp(ge_rhs, &TyParam::value(NegInf), None, true)
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
                    self.sub_unify_tp(rhs, le_rhs, None, false)?;
                    self.sub_unify_tp(rhs, ge_rhs, None, false)
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
            (Type::Ref(l), Type::Ref(r)) => self.reunify(l, r, bef_loc, aft_loc),
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
                self.reunify(lbefore, rbefore, bef_loc, aft_loc)?;
                match (lafter, rafter) {
                    (Some(lafter), Some(rafter)) => {
                        self.reunify(lafter, rafter, bef_loc, aft_loc)?;
                    }
                    (None, None) => {}
                    _ => todo!(),
                }
                Ok(())
            }
            (Type::Ref(l), r) => self.reunify(l, r, bef_loc, aft_loc),
            // REVIEW:
            (Type::RefMut { before, .. }, r) => self.reunify(before, r, bef_loc, aft_loc),
            (l, Type::Ref(r)) => self.reunify(l, r, bef_loc, aft_loc),
            (l, Type::RefMut { before, .. }) => self.reunify(l, before, bef_loc, aft_loc),
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
            (l, r) if self.same_type_of(l, r) => Ok(()),
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
        sub_loc: Option<Location>,
        sup_loc: Option<Location>,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        log!(info "trying sub_unify:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
        // In this case, there is no new information to be gained
        // この場合、特に新しく得られる情報はない
        if maybe_sub == &Type::Never || maybe_sup == &Type::Obj || maybe_sup == maybe_sub {
            return Ok(());
        }
        let maybe_sub_is_sub = self.subtype_of(maybe_sub, maybe_sup);
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
                self.get_candidates(maybe_sub),
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
                let intersec = self.intersection(&lsup, &rsup);
                let new_constraint = if intersec != Type::Never {
                    Constraint::new_sandwiched(self.union(&lsub, &rsub), intersec, cyclicity)
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
                Ok(())
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
                                Cyclicity::Not => self.supertype_of(sup, maybe_sub),
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
                            if let Some(new_sub) = self.max(maybe_sub, sub) {
                                *constraint =
                                    Constraint::new_sandwiched(new_sub.clone(), mem::take(sup), *cyclicity);
                            } else {
                                let new_sub = self.union(maybe_sub, sub);
                                *constraint = Constraint::new_sandwiched(new_sub, mem::take(sup), *cyclicity);
                            }
                        }
                        // sub_unify(Nat, ?T(: Type)): (/* ?T(:> Nat) */)
                        Constraint::TypeOf(ty) => {
                            if self.supertype_of(&Type, ty) {
                                *constraint = Constraint::new_supertype_of(maybe_sub.clone(), Cyclicity::Not);
                            } else {
                                todo!()
                            }
                        }
                        _ => unreachable!(),
                    },
                    _ => {}
                }
                Ok(())
            }
            (Type::FreeVar(lfv), _) if lfv.is_unbound() => {
                match &mut *lfv.borrow_mut() {
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
                            if !self.subtype_of(sub, maybe_sup) || !self.supertype_of(sup, maybe_sup) {
                                return Err(TyCheckError::subtyping_error(
                                    line!() as usize,
                                    sub,
                                    maybe_sup,
                                    sub_loc,
                                    sup_loc,
                                    self.caused_by(),
                                ));
                            }
                            if let Some(new_sup) = self.min(sup, maybe_sup) {
                                *constraint =
                                    Constraint::new_sandwiched(mem::take(sub), new_sup.clone(), *cyclicity);
                            } else {
                                let new_sup = self.union(sup, maybe_sup);
                                *constraint = Constraint::new_sandwiched(mem::take(sub), new_sup, *cyclicity);
                            }
                        }
                        // sub_unify(?T(: Type), Int): (?T(<: Int))
                        Constraint::TypeOf(ty) => {
                            if self.supertype_of(&Type, ty) {
                                *constraint = Constraint::new_subtype_of(maybe_sup.clone(), Cyclicity::Not);
                            } else {
                                todo!()
                            }
                        }
                        _ => unreachable!(),
                    },
                    _ => {}
                }
                Ok(())
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
                Ok(())
            }
            (Type::Subr(lsub), Type::Subr(rsub)) => {
                for lpt in lsub.default_params.iter() {
                    if let Some(rpt) = rsub.default_params.iter().find(|rpt| rpt.name() == lpt.name()) {
                        // contravariant
                        self.sub_unify(rpt.typ(), lpt.typ(), sup_loc, sub_loc, param_name)?;
                    } else { todo!() }
                }
                lsub.non_default_params.iter().zip(rsub.non_default_params.iter()).try_for_each(
                    // contravariant
                    |(l, r)| self.sub_unify(r.typ(), l.typ(), sup_loc, sub_loc, param_name),
                )?;
                // covariant
                self.sub_unify(&lsub.return_t, &rsub.return_t, sub_loc, sup_loc, param_name)?;
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
                if ln != rn {
                    return Err(TyCheckError::unification_error(
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        sub_loc,
                        sup_loc,
                        self.caused_by(),
                    ));
                }
                for (l, r) in lps.iter().zip(rps.iter()) {
                    self.sub_unify_tp(l, r, None, false)?;
                }
                Ok(())
            }
            (_, Type::Ref(t)) => {
                self.sub_unify(maybe_sub, t, sub_loc, sup_loc, param_name)?;
                Ok(())
            }
            (_, Type::RefMut{ before, .. }) => {
                self.sub_unify(maybe_sub, before, sub_loc, sup_loc, param_name)?;
                Ok(())
            }
            (Type::MonoProj { .. }, _) => todo!(),
            (_, Type::MonoProj { .. }) => todo!(),
            (Refinement(_), Refinement(_)) => todo!(),
            (Type::Subr(_) | Type::Record(_), Type) => Ok(()),
            // TODO Tuple2, ...
            (Type::Poly{ name, .. }, Type) if &name[..] == "Array" || &name[..] == "Tuple" => Ok(()),
            _ => todo!("{maybe_sub} can be a subtype of {maybe_sup}, but failed to semi-unify (or existential types are not supported)"),
        }
    }
}
