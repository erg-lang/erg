//! provides type variable related operations
use std::mem;
use std::option::Option;

use erg_common::error::Location;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{assume_unreachable, fn_name, log};

use crate::ty::constructors::*;
use crate::ty::free::{Constraint, FreeKind, HasLevel};
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{HasType, Predicate, Type};

// use crate::context::eval::SubstContext;
use crate::context::{Context, Variance};
use crate::error::{SingleTyCheckResult, TyCheckError, TyCheckErrors, TyCheckResult};
use crate::hir;

use Predicate as Pred;
use Type::*;
use ValueObj::{Inf, NegInf};
use Variance::*;

impl Context {
    pub const TOP_LEVEL: usize = 1;

    /// 型を非依存化する
    fn _independentise(_t: Type, _ts: &[Type]) -> Type {
        todo!()
    }

    fn generalize_tp(&self, free: TyParam, variance: Variance) -> TyParam {
        match free {
            TyParam::Type(t) => TyParam::t(self.generalize_t_inner(*t, variance)),
            TyParam::FreeVar(v) if v.is_linked() => {
                if let FreeKind::Linked(tp) = &mut *v.borrow_mut() {
                    *tp = self.generalize_tp(tp.clone(), variance);
                } else {
                    assume_unreachable!()
                }
                TyParam::FreeVar(v)
            }
            // TODO: Polymorphic generalization
            TyParam::FreeVar(fv) if fv.level() > Some(self.level) => {
                self.generalize_constraint(&fv.crack_constraint(), variance);
                fv.generalize();
                TyParam::FreeVar(fv)
            }
            TyParam::FreeVar(_) => free,
            other if other.has_no_unbound_var() => other,
            other => todo!("{other}"),
        }
    }

    pub(crate) fn generalize_t(&self, free_type: Type) -> Type {
        if cfg!(feature = "debug") && free_type.has_qvar() {
            panic!("{free_type} has qvars")
        }
        let maybe_unbound_t = self.generalize_t_inner(free_type, Covariant);
        if maybe_unbound_t.has_qvar() {
            // NOTE: `?T(<: TraitX) -> Int` should be `TraitX -> Int`
            // However, the current Erg cannot handle existential types, so it quantifies anyway
            /*if !maybe_unbound_t.return_t().unwrap().has_qvar() {
                let mut tv_ctx = TyVarInstContext::new(self.level, bounds.clone(), self);
                let inst = Self::instantiate_t(
                    maybe_unbound_t,
                    &mut tv_ctx,
                    Location::Unknown,
                )
                .unwrap();
                inst.lift();
                self.deref_tyvar(inst, Location::Unknown).unwrap()
            } else { */
            maybe_unbound_t.quantify()
            // }
        } else {
            maybe_unbound_t
        }
    }

    /// see doc/LANG/compiler/inference.md#一般化 for details
    /// ```python
    /// generalize_t(?T) == 'T: Type
    /// generalize_t(?T(<: Nat) -> ?T) == |'T <: Nat| 'T -> 'T
    /// generalize_t(?T(<: Eq(?T(<: Eq(?T(<: ...)))) -> ?T) == |'T <: Eq('T)| 'T -> 'T
    /// generalize_t(?T(<: TraitX) -> Int) == TraitX -> Int // 戻り値に現れないなら量化しない
    /// ```
    fn generalize_t_inner(&self, free_type: Type, variance: Variance) -> Type {
        log!(err "{free_type}");
        match free_type {
            FreeVar(v) if v.is_linked() => {
                if let FreeKind::Linked(t) = &mut *v.borrow_mut() {
                    *t = self.generalize_t_inner(t.clone(), variance);
                } else {
                    assume_unreachable!()
                }
                Type::FreeVar(v)
            }
            // TODO: Polymorphic generalization
            FreeVar(fv) if fv.level().unwrap() > self.level => {
                let constr = fv.constraint().unwrap();
                // |Int <: T <: Int| T -> T ==> Int -> Int
                let (l, r) = constr.get_sub_sup().unwrap();
                if self.same_type_of(l, r) {
                    fv.forced_link(l);
                    FreeVar(fv)
                } else if r != &Obj && self.is_class(r) && variance == Contravariant {
                    // |T <: Bool| T -> Int ==> Bool -> Int
                    r.clone()
                } else if l != &Never && self.is_class(l) && variance == Covariant {
                    // |T :> Int| X -> T ==> X -> Int
                    l.clone()
                } else {
                    log!(err "{fv}");
                    self.generalize_constraint(&fv.crack_constraint(), variance);
                    fv.generalize();
                    log!(err "{fv}");
                    Type::FreeVar(fv)
                }
            }
            Subr(mut subr) => {
                subr.non_default_params.iter_mut().for_each(|nd_param| {
                    *nd_param.typ_mut() =
                        self.generalize_t_inner(mem::take(nd_param.typ_mut()), Contravariant);
                });
                if let Some(var_args) = &mut subr.var_params {
                    *var_args.typ_mut() =
                        self.generalize_t_inner(mem::take(var_args.typ_mut()), Contravariant);
                }
                subr.default_params.iter_mut().for_each(|d_param| {
                    *d_param.typ_mut() =
                        self.generalize_t_inner(mem::take(d_param.typ_mut()), Contravariant);
                });
                let return_t = self.generalize_t_inner(*subr.return_t, Covariant);
                subr_t(
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|x| *x),
                    subr.default_params,
                    return_t,
                )
            }
            Callable { .. } => todo!(),
            Ref(t) => ref_(self.generalize_t_inner(*t, variance)),
            RefMut { before, after } => {
                let after = after.map(|aft| self.generalize_t_inner(*aft, variance));
                ref_mut(self.generalize_t_inner(*before, variance), after)
            }
            Refinement(refine) => {
                let t = self.generalize_t_inner(*refine.t, variance);
                let preds = refine
                    .preds
                    .into_iter()
                    .map(|pred| self.generalize_pred(pred, variance))
                    .collect();
                refinement(refine.var, t, preds)
            }
            Poly { name, mut params } => {
                let params = params
                    .iter_mut()
                    .map(|p| self.generalize_tp(mem::take(p), variance))
                    .collect::<Vec<_>>();
                poly(name, params)
            }
            Proj { lhs, rhs } => {
                let lhs = self.generalize_t_inner(*lhs, variance);
                proj(lhs, rhs)
            }
            ProjCall {
                lhs,
                attr_name,
                mut args,
            } => {
                let lhs = self.generalize_tp(*lhs, variance);
                for arg in args.iter_mut() {
                    *arg = self.generalize_tp(mem::take(arg), variance);
                }
                proj_call(lhs, attr_name, args)
            }
            And(l, r) => {
                let l = self.generalize_t_inner(*l, variance);
                let r = self.generalize_t_inner(*r, variance);
                // not `self.intersection` because types are generalized
                and(l, r)
            }
            Or(l, r) => {
                let l = self.generalize_t_inner(*l, variance);
                let r = self.generalize_t_inner(*r, variance);
                // not `self.union` because types are generalized
                or(l, r)
            }
            Not(l, r) => {
                let l = self.generalize_t_inner(*l, variance);
                let r = self.generalize_t_inner(*r, variance);
                not(l, r)
            }
            // REVIEW: その他何でもそのまま通していいのか?
            other => other,
        }
    }

    fn generalize_constraint(&self, constraint: &Constraint, variance: Variance) -> Constraint {
        match constraint {
            Constraint::Sandwiched { sub, sup, .. } => {
                let sub = self.generalize_t_inner(sub.clone(), variance);
                let sup = self.generalize_t_inner(sup.clone(), variance);
                Constraint::new_sandwiched(sub, sup)
            }
            Constraint::TypeOf(t) => {
                let t = self.generalize_t_inner(t.clone(), variance);
                Constraint::new_type_of(t)
            }
            Constraint::Uninited => unreachable!(),
        }
    }

    fn generalize_pred(&self, pred: Predicate, variance: Variance) -> Predicate {
        match pred {
            Predicate::Const(_) => pred,
            Predicate::Equal { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, variance);
                Predicate::eq(lhs, rhs)
            }
            Predicate::GreaterEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, variance);
                Predicate::ge(lhs, rhs)
            }
            Predicate::LessEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, variance);
                Predicate::le(lhs, rhs)
            }
            Predicate::NotEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, variance);
                Predicate::ne(lhs, rhs)
            }
            other => todo!("{other}"),
        }
    }

    pub(crate) fn deref_tp(
        &self,
        tp: TyParam,
        variance: Variance,
        loc: Location,
    ) -> SingleTyCheckResult<TyParam> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let inner = fv.unwrap_linked();
                self.deref_tp(inner, variance, loc)
            }
            TyParam::FreeVar(_fv) if self.level == 0 => Err(TyCheckError::dummy_infer_error(
                self.cfg.input.clone(),
                fn_name!(),
                line!(),
            )),
            TyParam::Type(t) => Ok(TyParam::t(self.deref_tyvar(*t, variance, loc)?)),
            TyParam::App { name, mut args } => {
                for param in args.iter_mut() {
                    *param = self.deref_tp(mem::take(param), variance, loc)?;
                }
                Ok(TyParam::App { name, args })
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let lhs = self.deref_tp(*lhs, variance, loc)?;
                let rhs = self.deref_tp(*rhs, variance, loc)?;
                Ok(TyParam::BinOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                })
            }
            TyParam::UnaryOp { op, val } => {
                let val = self.deref_tp(*val, variance, loc)?;
                Ok(TyParam::UnaryOp {
                    op,
                    val: Box::new(val),
                })
            }
            TyParam::Array(tps) => {
                let mut new_tps = vec![];
                for tp in tps {
                    new_tps.push(self.deref_tp(tp, variance, loc)?);
                }
                Ok(TyParam::Array(new_tps))
            }
            TyParam::Tuple(tps) => {
                let mut new_tps = vec![];
                for tp in tps {
                    new_tps.push(self.deref_tp(tp, variance, loc)?);
                }
                Ok(TyParam::Tuple(new_tps))
            }
            TyParam::Proj { .. } | TyParam::Failure if self.level == 0 => Err(
                TyCheckError::dummy_infer_error(self.cfg.input.clone(), fn_name!(), line!()),
            ),
            t => Ok(t),
        }
    }

    fn deref_constraint(
        &self,
        constraint: Constraint,
        variance: Variance,
        loc: Location,
    ) -> SingleTyCheckResult<Constraint> {
        match constraint {
            Constraint::Sandwiched { sub, sup } => {
                /*if cyclic.is_cyclic() {
                    return Err(TyCheckError::dummy_infer_error(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    ));
                }*/
                Ok(Constraint::new_sandwiched(
                    self.deref_tyvar(sub, variance, loc)?,
                    self.deref_tyvar(sup, variance, loc)?,
                ))
            }
            Constraint::TypeOf(t) => {
                Ok(Constraint::new_type_of(self.deref_tyvar(t, variance, loc)?))
            }
            _ => unreachable!(),
        }
    }

    /// e.g.
    /// ```python
    // ?T(:> Nat, <: Int)[n] ==> Nat (self.level <= n)
    // ?T(:> Nat, <: Sub(?U(:> {1}))) ==> Nat
    // ?T(:> Nat, <: Sub(?U(:> {1}))) -> ?U ==> |U: Type, T <: Sub(U)| T -> U
    // ?T(:> Nat, <: Sub(Str)) ==> Error!
    // ?T(:> {1, "a"}, <: Eq(?T(:> {1, "a"}, ...)) ==> Error!
    // ```
    pub(crate) fn deref_tyvar(
        &self,
        t: Type,
        variance: Variance,
        loc: Location,
    ) -> SingleTyCheckResult<Type> {
        match t {
            // ?T(:> Nat, <: Int)[n] ==> Nat (self.level <= n)
            // ?T(:> Nat, <: Sub ?U(:> {1}))[n] ==> Nat
            // ?T(:> Nat, <: Sub(Str)) ==> Error!
            // ?T(:> {1, "a"}, <: Eq(?T(:> {1, "a"}, ...)) ==> Error!
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub_t, super_t) = fv.get_subsup().unwrap();
                if self.level <= fv.level().unwrap() {
                    /*if fv.cyclicity().is_super_cyclic() {
                        fv.forced_link(&sub_t);
                    }*/
                    if self.is_trait(&super_t) {
                        self.check_trait_impl(&sub_t, &super_t, loc)?;
                    }
                    // REVIEW: Even if type constraints can be satisfied, implementation may not exist
                    if self.subtype_of(&sub_t, &super_t) {
                        match variance {
                            Variance::Covariant => Ok(sub_t),
                            Variance::Contravariant => Ok(super_t),
                            Variance::Invariant => {
                                if self.supertype_of(&sub_t, &super_t) {
                                    Ok(sub_t)
                                } else {
                                    Err(TyCheckError::subtyping_error(
                                        self.cfg.input.clone(),
                                        line!() as usize,
                                        &self.deref_tyvar(sub_t, variance, loc)?,
                                        &self.deref_tyvar(super_t, variance, loc)?,
                                        loc,
                                        self.caused_by(),
                                    ))
                                }
                            }
                        }
                    } else {
                        let sub_t = if cfg!(feature = "debug") {
                            sub_t
                        } else {
                            self.deref_tyvar(sub_t, variance, loc)?
                        };
                        let super_t = if cfg!(feature = "debug") {
                            super_t
                        } else {
                            self.deref_tyvar(super_t, variance, loc)?
                        };
                        Err(TyCheckError::subtyping_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            &sub_t,
                            &super_t,
                            loc,
                            self.caused_by(),
                        ))
                    }
                } else {
                    // no dereference at this point
                    // drop(constraint);
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::FreeVar(fv) if fv.is_unbound() => {
                if self.level == 0 {
                    match &*fv.crack_constraint() {
                        Constraint::TypeOf(_) => Err(TyCheckError::dummy_infer_error(
                            self.cfg.input.clone(),
                            fn_name!(),
                            line!(),
                        )),
                        _ => unreachable!(),
                    }
                } else {
                    let new_constraint = fv.crack_constraint().clone();
                    let new_constraint = self.deref_constraint(new_constraint, variance, loc)?;
                    fv.update_constraint(new_constraint);
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.unwrap_linked();
                self.deref_tyvar(t, variance, loc)
            }
            Type::Poly { name, mut params } => {
                let typ = poly(&name, params.clone());
                let ctx = self.get_nominal_type_ctx(&typ).unwrap();
                let variances = ctx.type_params_variance();
                for (param, variance) in params.iter_mut().zip(variances.into_iter()) {
                    *param = self.deref_tp(mem::take(param), variance, loc)?;
                }
                Ok(Type::Poly { name, params })
            }
            Type::Subr(mut subr) => {
                for param in subr.non_default_params.iter_mut() {
                    *param.typ_mut() =
                        self.deref_tyvar(mem::take(param.typ_mut()), Contravariant, loc)?;
                }
                if let Some(var_args) = &mut subr.var_params {
                    *var_args.typ_mut() =
                        self.deref_tyvar(mem::take(var_args.typ_mut()), Contravariant, loc)?;
                }
                for d_param in subr.default_params.iter_mut() {
                    *d_param.typ_mut() =
                        self.deref_tyvar(mem::take(d_param.typ_mut()), Contravariant, loc)?;
                }
                subr.return_t =
                    Box::new(self.deref_tyvar(mem::take(&mut subr.return_t), Covariant, loc)?);
                Ok(Type::Subr(subr))
            }
            Type::Ref(t) => {
                let t = self.deref_tyvar(*t, variance, loc)?;
                Ok(ref_(t))
            }
            Type::RefMut { before, after } => {
                let before = self.deref_tyvar(*before, variance, loc)?;
                let after = if let Some(after) = after {
                    Some(self.deref_tyvar(*after, variance, loc)?)
                } else {
                    None
                };
                Ok(ref_mut(before, after))
            }
            Type::Callable { .. } => todo!(),
            Type::Record(mut rec) => {
                for (_, field) in rec.iter_mut() {
                    *field = self.deref_tyvar(mem::take(field), variance, loc)?;
                }
                Ok(Type::Record(rec))
            }
            // |X <: T <: X| X -> X ==> T -> T
            /*Type::Quantified(quant) => {
                let mut replace_list = vec![];
                let mut new_bounds = set!{};
                for bound in quant.bounds.into_iter() {
                    if let Some((sub, mid, sup)) = bound.get_types() {
                        if self.subtype_of(sub, sup) && self.supertype_of(sub, sup) {
                            replace_list.push((mid, sub));
                        } else {
                            new_bounds.insert(bound);
                        }
                    } else {
                        new_bounds.insert(bound);
                    }
                }
                Ok(())
            }*/
            Type::Refinement(refine) => {
                let t = self.deref_tyvar(*refine.t, variance, loc)?;
                // TODO: deref_predicate
                Ok(refinement(refine.var, t, refine.preds))
            }
            Type::And(l, r) => {
                let l = self.deref_tyvar(*l, variance, loc)?;
                let r = self.deref_tyvar(*r, variance, loc)?;
                Ok(self.intersection(&l, &r))
            }
            Type::Or(l, r) => {
                let l = self.deref_tyvar(*l, variance, loc)?;
                let r = self.deref_tyvar(*r, variance, loc)?;
                Ok(self.union(&l, &r))
            }
            Type::Not(l, r) => {
                let l = self.deref_tyvar(*l, variance, loc)?;
                let r = self.deref_tyvar(*r, variance, loc)?;
                // TODO: complement
                Ok(not(l, r))
            }
            t => Ok(t),
        }
    }

    pub(crate) fn trait_impl_exists(&self, class: &Type, trait_: &Type) -> bool {
        if class.is_monomorphic() {
            self.mono_class_trait_impl_exist(class, trait_)
        } else {
            self.poly_class_trait_impl_exists(class, trait_)
        }
    }

    fn mono_class_trait_impl_exist(&self, class: &Type, trait_: &Type) -> bool {
        let mut super_exists = false;
        for inst in self.get_trait_impls(trait_).into_iter() {
            if self.supertype_of(&inst.sub_type, class)
                && self.supertype_of(&inst.sup_trait, trait_)
            {
                super_exists = true;
                break;
            }
        }
        super_exists
    }

    fn poly_class_trait_impl_exists(&self, class: &Type, trait_: &Type) -> bool {
        let mut super_exists = false;
        /*let subst_ctx = if self.get_nominal_type_ctx(class).is_some() {
            SubstContext::new(class, self, Location::Unknown)
        } else {
            return false;
        };*/
        for inst in self.get_trait_impls(trait_).into_iter() {
            if self.supertype_of(&inst.sub_type, class)
                && self.supertype_of(&inst.sup_trait, trait_)
            {
                super_exists = true;
                break;
            }
        }
        super_exists
    }

    fn check_trait_impl(
        &self,
        class: &Type,
        trait_: &Type,
        loc: Location,
    ) -> SingleTyCheckResult<()> {
        if !self.trait_impl_exists(class, trait_) {
            Err(TyCheckError::no_trait_impl_error(
                self.cfg.input.clone(),
                line!() as usize,
                class,
                trait_,
                loc,
                self.caused_by(),
                None,
            ))
        } else {
            Ok(())
        }
    }

    /// Fix type variables at their lower bound
    pub(crate) fn coerce(&self, t: &Type) {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => {
                self.coerce(&fv.crack());
            }
            Type::FreeVar(fv) if fv.is_unbound() => {
                let (sub, _sup) = fv.get_subsup().unwrap();
                fv.link(&sub);
            }
            Type::And(l, r) | Type::Or(l, r) | Type::Not(l, r) => {
                self.coerce(l);
                self.coerce(r);
            }
            _ => {}
        }
    }

    pub(crate) fn coerce_tp(&self, tp: &TyParam) {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                self.coerce_tp(&fv.crack());
            }
            TyParam::Type(t) => self.coerce(t),
            _ => {}
        }
    }

    /// Check if all types are resolvable (if traits, check if an implementation exists)
    /// And replace them if resolvable
    pub(crate) fn resolve(
        &mut self,
        mut hir: hir::HIR,
    ) -> Result<hir::HIR, (hir::HIR, TyCheckErrors)> {
        self.level = 0;
        let mut errs = TyCheckErrors::empty();
        for chunk in hir.module.iter_mut() {
            if let Err(err) = self.resolve_expr_t(chunk) {
                errs.push(err);
            }
        }
        if errs.is_empty() {
            Ok(hir)
        } else {
            Err((hir, errs))
        }
    }

    fn resolve_expr_t(&self, expr: &mut hir::Expr) -> SingleTyCheckResult<()> {
        match expr {
            hir::Expr::Lit(_) => Ok(()),
            hir::Expr::Accessor(acc) => {
                let loc = acc.loc();
                let t = acc.ref_mut_t();
                *t = self.deref_tyvar(mem::take(t), Covariant, loc)?;
                if let hir::Accessor::Attr(attr) = acc {
                    self.resolve_expr_t(&mut attr.obj)?;
                }
                Ok(())
            }
            hir::Expr::Array(array) => match array {
                hir::Array::Normal(arr) => {
                    let loc = arr.loc();
                    arr.t = self.deref_tyvar(mem::take(&mut arr.t), Covariant, loc)?;
                    for elem in arr.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr)?;
                    }
                    Ok(())
                }
                hir::Array::WithLength(arr) => {
                    let loc = arr.loc();
                    arr.t = self.deref_tyvar(mem::take(&mut arr.t), Covariant, loc)?;
                    self.resolve_expr_t(&mut arr.elem)?;
                    self.resolve_expr_t(&mut arr.len)?;
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
            hir::Expr::Set(set) => match set {
                hir::Set::Normal(st) => {
                    let loc = st.loc();
                    st.t = self.deref_tyvar(mem::take(&mut st.t), Covariant, loc)?;
                    for elem in st.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr)?;
                    }
                    Ok(())
                }
                hir::Set::WithLength(st) => {
                    let loc = st.loc();
                    st.t = self.deref_tyvar(mem::take(&mut st.t), Covariant, loc)?;
                    self.resolve_expr_t(&mut st.elem)?;
                    self.resolve_expr_t(&mut st.len)?;
                    Ok(())
                }
            },
            hir::Expr::Dict(dict) => match dict {
                hir::Dict::Normal(dic) => {
                    let loc = dic.loc();
                    dic.t = self.deref_tyvar(mem::take(&mut dic.t), Covariant, loc)?;
                    for kv in dic.kvs.iter_mut() {
                        self.resolve_expr_t(&mut kv.key)?;
                        self.resolve_expr_t(&mut kv.value)?;
                    }
                    Ok(())
                }
                other => todo!("{other}"),
            },
            hir::Expr::Record(record) => {
                for attr in record.attrs.iter_mut() {
                    match &mut attr.sig {
                        hir::Signature::Var(var) => {
                            *var.ref_mut_t() =
                                self.deref_tyvar(mem::take(var.ref_mut_t()), Covariant, var.loc())?;
                        }
                        hir::Signature::Subr(subr) => {
                            subr.t =
                                self.deref_tyvar(mem::take(&mut subr.t), Covariant, subr.loc())?;
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
                *t = self.deref_tyvar(mem::take(t), Covariant, loc)?;
                self.resolve_expr_t(&mut binop.lhs)?;
                self.resolve_expr_t(&mut binop.rhs)?;
                Ok(())
            }
            hir::Expr::UnaryOp(unaryop) => {
                let loc = unaryop.loc();
                let t = unaryop.signature_mut_t().unwrap();
                *t = self.deref_tyvar(mem::take(t), Covariant, loc)?;
                self.resolve_expr_t(&mut unaryop.expr)?;
                Ok(())
            }
            hir::Expr::Call(call) => {
                let loc = call.loc();
                if let Some(t) = call.signature_mut_t() {
                    *t = self.deref_tyvar(mem::take(t), Covariant, loc)?;
                }
                self.resolve_expr_t(&mut call.obj)?;
                for arg in call.args.pos_args.iter_mut() {
                    self.resolve_expr_t(&mut arg.expr)?;
                }
                if let Some(var_args) = &mut call.args.var_args {
                    self.resolve_expr_t(&mut var_args.expr)?;
                }
                for arg in call.args.kw_args.iter_mut() {
                    self.resolve_expr_t(&mut arg.expr)?;
                }
                Ok(())
            }
            hir::Expr::Def(def) => {
                // It is not possible to further dereference the quantified type.
                // TODO: However, it is possible that there are external type variables within the quantified type.
                if !def.sig.ref_t().is_quantified() {
                    match &mut def.sig {
                        hir::Signature::Var(var) => {
                            *var.ref_mut_t() =
                                self.deref_tyvar(mem::take(var.ref_mut_t()), Covariant, var.loc())?;
                        }
                        hir::Signature::Subr(subr) => {
                            subr.t =
                                self.deref_tyvar(mem::take(&mut subr.t), Covariant, subr.loc())?;
                        }
                    }
                    for chunk in def.body.block.iter_mut() {
                        self.resolve_expr_t(chunk)?;
                    }
                }
                Ok(())
            }
            hir::Expr::Lambda(lambda) => {
                lambda.t = self.deref_tyvar(mem::take(&mut lambda.t), Covariant, lambda.loc())?;
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
            hir::Expr::Import(_) => unreachable!(),
        }
    }

    // occur(X -> ?T, ?T) ==> Error
    // occur(?T, ?T -> X) ==> Error
    // occur(?T, Option(?T)) ==> Error
    fn occur(&self, maybe_sub: &Type, maybe_sup: &Type, loc: Location) -> TyCheckResult<()> {
        match (maybe_sub, maybe_sup) {
            (Type::FreeVar(sub), Type::FreeVar(sup)) => {
                if sub.is_unbound() && sup.is_unbound() && sub == sup {
                    Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        loc,
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
            _ => Ok(()),
        }
    }

    /// allow_divergence = trueにすると、Num型変数と±Infの単一化を許す
    pub(crate) fn sub_unify_tp(
        &self,
        maybe_sub: &TyParam,
        maybe_sup: &TyParam,
        variance: Option<Variance>,
        loc: Location,
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
                    lfv.link(maybe_sup);
                } else {
                    rfv.link(maybe_sub);
                }
                Ok(())
            }
            (TyParam::FreeVar(fv), tp) => {
                match &*fv.borrow() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.sub_unify_tp(l, tp, variance, loc, allow_divergence);
                    }
                    FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {}
                } // &fv is dropped
                let fv_t = fv.constraint().unwrap().get_type().unwrap().clone(); // fvを参照しないよいにcloneする(あとでborrow_mutするため)
                let tp_t = self.get_tp_t(tp)?;
                if self.supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if fv.level() < Some(self.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t);
                        if self.is_sub_constraint_of(&fv.constraint().unwrap(), &new_constraint)
                            || fv.constraint().unwrap().get_type() == Some(&Type)
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
                    && self.subtype_of(&fv_t, &mono("Num"))
                {
                    fv.link(tp);
                    Ok(())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    )))
                }
            }
            (tp, TyParam::FreeVar(fv)) => {
                match &*fv.borrow() {
                    FreeKind::Linked(l) | FreeKind::UndoableLinked { t: l, .. } => {
                        return self.sub_unify_tp(l, tp, variance, loc, allow_divergence);
                    }
                    FreeKind::Unbound { .. } | FreeKind::NamedUnbound { .. } => {}
                } // &fv is dropped
                let fv_t = fv.constraint().unwrap().get_type().unwrap().clone(); // fvを参照しないよいにcloneする(あとでborrow_mutするため)
                let tp_t = self.get_tp_t(tp)?;
                if self.supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if fv.level() < Some(self.level) {
                        let new_constraint = Constraint::new_subtype_of(tp_t);
                        if self.is_sub_constraint_of(&fv.constraint().unwrap(), &new_constraint)
                            || fv.constraint().unwrap().get_type() == Some(&Type)
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
                    && self.subtype_of(&fv_t, &mono("Num"))
                {
                    fv.link(tp);
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
                self.sub_unify_tp(lval, rval, variance, loc, allow_divergence)
            }
            (
                TyParam::BinOp { op: lop, lhs, rhs },
                TyParam::BinOp {
                    op: rop,
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if lop == rop => {
                self.sub_unify_tp(lhs, lhs2, variance, loc, allow_divergence)?;
                self.sub_unify_tp(rhs, rhs2, variance, loc, allow_divergence)
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
                        loc,
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
            (l, r) => panic!("type-parameter unification failed:\nl:{l}\nr: {r}"),
        }
    }

    fn reunify_tp(
        &self,
        before: &TyParam,
        after: &TyParam,
        loc: Location,
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
            /*(TyParam::Value(ValueObj::Mut(l)), TyParam::Erased(_)) => {
                *l.borrow_mut() = after.clone();
                Ok(())
            }*/
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
            (l, r) if self.eq_tp(l, r) => Ok(()),
            (l, r) => panic!("type-parameter re-unification failed:\nl: {l}\nr: {r}"),
        }
    }

    /// predは正規化されているとする
    fn sub_unify_pred(
        &self,
        l_pred: &Predicate,
        r_pred: &Predicate,
        loc: Location,
    ) -> TyCheckResult<()> {
        match (l_pred, r_pred) {
            (Pred::Value(_), Pred::Value(_)) | (Pred::Const(_), Pred::Const(_)) => Ok(()),
            (Pred::Equal { rhs, .. }, Pred::Equal { rhs: rhs2, .. })
            | (Pred::GreaterEqual { rhs, .. }, Pred::GreaterEqual { rhs: rhs2, .. })
            | (Pred::LessEqual { rhs, .. }, Pred::LessEqual { rhs: rhs2, .. })
            | (Pred::NotEqual { rhs, .. }, Pred::NotEqual { rhs: rhs2, .. }) => {
                self.sub_unify_tp(rhs, rhs2, None, loc, false)
            }
            (Pred::And(l1, r1), Pred::And(l2, r2))
            | (Pred::Or(l1, r1), Pred::Or(l2, r2))
            | (Pred::Not(l1, r1), Pred::Not(l2, r2)) => {
                match (
                    self.sub_unify_pred(l1, l2, loc),
                    self.sub_unify_pred(r1, r2, loc),
                ) {
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
        loc: Location,
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
                        loc,
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
                loc,
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
        loc: Location,
        param_name: Option<&Str>,
    ) -> TyCheckResult<()> {
        log!(info "trying sub_unify:\nmaybe_sub: {maybe_sub}\nmaybe_sup: {maybe_sup}");
        // In this case, there is no new information to be gained
        // この場合、特に新しく得られる情報はない
        if maybe_sub == &Type::Never || maybe_sup == &Type::Obj || maybe_sup == maybe_sub {
            return Ok(());
        }
        self.occur(maybe_sub, maybe_sup, loc)?;
        let maybe_sub_is_sub = self.subtype_of(maybe_sub, maybe_sup);
        if maybe_sub.has_no_unbound_var() && maybe_sup.has_no_unbound_var() && maybe_sub_is_sub {
            return Ok(());
        }
        if !maybe_sub_is_sub {
            log!(err "{maybe_sub} !<: {maybe_sup}");
            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                self.cfg.input.clone(),
                line!() as usize,
                loc,
                self.caused_by(),
                param_name.unwrap_or(&Str::ever("_")),
                maybe_sup,
                maybe_sub,
                self.get_candidates(maybe_sub),
                self.get_type_mismatch_hint(maybe_sup, maybe_sub),
            )));
        }
        match (maybe_sub, maybe_sup) {
            (Type::FreeVar(fv), _) if fv.is_quanted() => todo!("{maybe_sub}, {maybe_sup}"),
            (_, Type::FreeVar(fv)) if fv.is_quanted() => todo!("{maybe_sub}, {maybe_sup}"),
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
                let (lsub, lsup) = lfv.get_subsup().unwrap();
                let (rsub, rsup) = rfv.get_subsup().unwrap();
                let intersec = self.intersection(&lsup, &rsup);
                let new_constraint = if intersec != Type::Never {
                    Constraint::new_sandwiched(self.union(&lsub, &rsub), intersec)
                } else {
                    return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        loc,
                        self.caused_by(),
                    )));
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
            (Type::FreeVar(lfv), _) if lfv.is_linked() =>
                self.sub_unify(&lfv.crack(), maybe_sup, loc, param_name),
            (_, Type::FreeVar(rfv)) if rfv.is_linked() =>
                self.sub_unify(maybe_sub, &rfv.crack(), loc, param_name),
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
                        // * sub_unify(Bool,  ?T(<: Bool or Y)): (?T == Bool)
                        Constraint::Sandwiched { sub, sup } => {
                            /*if let Some(new_sub) = self.max(maybe_sub, sub) {
                                *constraint =
                                    Constraint::new_sandwiched(new_sub.clone(), mem::take(sup), *cyclicity);
                            } else {*/
                            let new_sub = self.union(maybe_sub, sub);
                            if sup.contains_union(&new_sub) {
                                rfv.link(&new_sub); // Bool <: ?T <: Bool or Y ==> ?T == Bool
                            } else {
                                *constraint = Constraint::new_sandwiched(new_sub, mem::take(sup));
                            }
                            // }
                        }
                        // sub_unify(Nat, ?T(: Type)): (/* ?T(:> Nat) */)
                        Constraint::TypeOf(ty) => {
                            if self.supertype_of(&Type, ty) {
                                *constraint = Constraint::new_supertype_of(maybe_sub.clone());
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
                        Constraint::Sandwiched { sub, sup } => {
                            // REVIEW: correct?
                            if let Some(new_sup) = self.min(sup, maybe_sup) {
                                *constraint =
                                    Constraint::new_sandwiched(mem::take(sub), new_sup.clone());
                            } else {
                                let new_sup = self.union(sup, maybe_sup);
                                *constraint = Constraint::new_sandwiched(mem::take(sub), new_sup);
                            }
                        }
                        // sub_unify(?T(: Type), Int): (?T(<: Int))
                        Constraint::TypeOf(ty) => {
                            if self.supertype_of(&Type, ty) {
                                *constraint = Constraint::new_subtype_of(maybe_sup.clone());
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
                        self.sub_unify(l, r, loc, param_name)?;
                    } else {
                        return Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            maybe_sub,
                            maybe_sup,
                            loc,
                            self.caused_by(),
                        )));
                    }
                }
                Ok(())
            }
            (Type::Subr(lsub), Type::Subr(rsub)) => {
                for lpt in lsub.default_params.iter() {
                    if let Some(rpt) = rsub.default_params.iter().find(|rpt| rpt.name() == lpt.name()) {
                        // contravariant
                        self.sub_unify(rpt.typ(), lpt.typ(), loc, param_name)?;
                    } else { todo!() }
                }
                lsub.non_default_params.iter().zip(rsub.non_default_params.iter()).try_for_each(
                    // contravariant
                    |(l, r)| self.sub_unify(r.typ(), l.typ(), loc, param_name),
                )?;
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
                if ln != rn {
                    if let Some(sub_ctx) = self.get_nominal_type_ctx(maybe_sub) {
                        // let subst_ctx = SubstContext::new(maybe_sub, self, loc);
                        for sup_trait in sub_ctx.super_traits.iter() {
                            /*let sup_trait = if sup_trait.has_qvar() {
                                subst_ctx.substitute(sup_trait.clone())?
                            } else {
                                sup_trait.clone()
                            };*/
                            if self.supertype_of(maybe_sup, sup_trait) {
                                for (l_maybe_sub, r_maybe_sup) in sup_trait.typarams().iter().zip(rps.iter()) {
                                    self.sub_unify_tp(l_maybe_sub, r_maybe_sup, None, loc, false)?;
                                }
                                return Ok(());
                            }
                        }
                    }
                    return Err(TyCheckErrors::from(TyCheckError::unification_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        maybe_sub,
                        maybe_sup,
                        loc,
                        self.caused_by(),
                    )));
                }
                for (l_maybe_sub, r_maybe_sup) in lps.iter().zip(rps.iter()) {
                    self.sub_unify_tp(l_maybe_sub, r_maybe_sup, None, loc, false)?;
                }
                Ok(())
            }
            (Type::And(l, r), _)
            | (Type::Or(l, r), _)
            | (Type::Not(l, r), _) => {
                self.sub_unify(l, maybe_sup, loc, param_name)?;
                self.sub_unify(r, maybe_sup, loc, param_name)?;
                Ok(())
            }
            (_, Type::And(l, r))
            | (_, Type::Or(l, r))
            | (_, Type::Not(l, r)) => {
                self.sub_unify(maybe_sub, l, loc, param_name)?;
                self.sub_unify(maybe_sub, r, loc, param_name)?;
                Ok(())
            }
            (_, Type::Ref(t)) => {
                self.sub_unify(maybe_sub, t, loc, param_name)?;
                Ok(())
            }
            (_, Type::RefMut{ before, .. }) => {
                self.sub_unify(maybe_sub, before, loc, param_name)?;
                Ok(())
            }
            (Type::Proj { .. }, _) => todo!(),
            (_, Type::Proj { .. }) => todo!(),
            (Refinement(l), Refinement(r)) => {
                if l.preds.len() == 1 && r.preds.len() == 1 {
                    let l_first = l.preds.iter().next().unwrap();
                    let r_first = r.preds.iter().next().unwrap();
                    self.sub_unify_pred(l_first, r_first, loc)?;
                    return Ok(());
                }
                todo!("{l}, {r}")
            },
            (Type::Subr(_) | Type::Record(_), Type) => Ok(()),
            // REVIEW: correct?
            (Type::Poly{ name, .. }, Type) if &name[..] == "Array" || &name[..] == "Tuple" => Ok(()),
            _ => todo!("{maybe_sub} can be a subtype of {maybe_sup}, but failed to semi-unify (or existential types are not supported)"),
        }
    }
}
