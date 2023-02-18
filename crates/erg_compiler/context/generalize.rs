use std::mem;

use erg_common::traits::{Locational, Stream};
use erg_common::{assume_unreachable, fn_name};
use erg_common::{dict, set};
#[allow(unused_imports)]
use erg_common::{fmt_vec, log};

use crate::ty::constructors::*;
use crate::ty::free::{Constraint, FreeKind, HasLevel};
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{HasType, Predicate, Type};

use crate::context::{Context, Variance};
use crate::error::{TyCheckError, TyCheckErrors, TyCheckResult};
use crate::{feature_error, hir};

use Type::*;
use Variance::*;

impl Context {
    pub const TOP_LEVEL: usize = 1;

    fn generalize_tp(&self, free: TyParam, variance: Variance, uninit: bool) -> TyParam {
        match free {
            TyParam::Type(t) => TyParam::t(self.generalize_t_inner(*t, variance, uninit)),
            TyParam::FreeVar(fv) if fv.is_generalized() => TyParam::FreeVar(fv),
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let fv_mut = unsafe { fv.as_ptr().as_mut().unwrap() };
                if let FreeKind::Linked(tp) = fv_mut {
                    *tp = self.generalize_tp(tp.clone(), variance, uninit);
                } else {
                    assume_unreachable!()
                }
                TyParam::FreeVar(fv)
            }
            // TODO: Polymorphic generalization
            TyParam::FreeVar(fv) if fv.level() > Some(self.level) => {
                let constr = self.generalize_constraint(&fv.crack_constraint(), variance);
                fv.update_constraint(constr, true);
                fv.generalize();
                TyParam::FreeVar(fv)
            }
            TyParam::Array(tps) => TyParam::Array(
                tps.into_iter()
                    .map(|tp| self.generalize_tp(tp, variance, uninit))
                    .collect(),
            ),
            TyParam::Tuple(tps) => TyParam::Tuple(
                tps.into_iter()
                    .map(|tp| self.generalize_tp(tp, variance, uninit))
                    .collect(),
            ),
            TyParam::Dict(tps) => TyParam::Dict(
                tps.into_iter()
                    .map(|(k, v)| {
                        (
                            self.generalize_tp(k, variance, uninit),
                            self.generalize_tp(v, variance, uninit),
                        )
                    })
                    .collect(),
            ),
            TyParam::FreeVar(_) => free,
            other if other.has_no_unbound_var() => other,
            other => todo!("{other}"),
        }
    }

    pub(crate) fn generalize_t(&self, free_type: Type) -> Type {
        if cfg!(feature = "debug") && free_type.has_qvar() {
            panic!("{free_type} has qvars")
        }
        let maybe_unbound_t = self.generalize_t_inner(free_type, Covariant, false);
        if maybe_unbound_t.has_qvar() {
            // NOTE: `?T(<: TraitX) -> Int` should be `TraitX -> Int`
            // However, the current Erg cannot handle existential types, so it quantifies anyway
            maybe_unbound_t.quantify()
        } else {
            maybe_unbound_t
        }
    }

    /// see doc/LANG/compiler/inference.md#一般化 for details
    /// ```python
    /// generalize_t(?T) == 'T: Type
    /// generalize_t(?T(<: Nat) -> ?T) == |'T <: Nat| 'T -> 'T
    /// generalize_t(?T(<: Add(?T(<: Eq(?T(<: ...)))) -> ?T) == |'T <: Add('T)| 'T -> 'T
    /// generalize_t(?T(<: TraitX) -> Int) == TraitX -> Int // 戻り値に現れないなら量化しない
    /// ```
    fn generalize_t_inner(&self, free_type: Type, variance: Variance, uninit: bool) -> Type {
        match free_type {
            FreeVar(fv) if fv.is_linked() => {
                self.generalize_t_inner(fv.crack().clone(), variance, uninit)
                /*let fv_mut = unsafe { fv.as_ptr().as_mut().unwrap() };
                if let FreeKind::Linked(t) = fv_mut {
                    *t = self.generalize_t_inner(t.clone(), variance, uninit);
                } else {
                    assume_unreachable!()
                }
                Type::FreeVar(fv)*/
            }
            FreeVar(fv) if fv.is_generalized() => Type::FreeVar(fv),
            // TODO: Polymorphic generalization
            FreeVar(fv) if fv.level().unwrap() > self.level => {
                if uninit {
                    // use crate::ty::free::GENERIC_LEVEL;
                    // return named_free_var(fv.unbound_name().unwrap(), GENERIC_LEVEL, Constraint::Uninited);
                    fv.generalize();
                    return Type::FreeVar(fv);
                }
                let constr = fv.constraint().unwrap();
                if let Some((l, r)) = constr.get_sub_sup() {
                    // |Int <: T <: Int| T -> T ==> Int -> Int
                    if l == r {
                        fv.forced_link(l);
                        FreeVar(fv)
                    } else if r != &Obj && self.is_class(r) && variance == Contravariant {
                        // |T <: Bool| T -> Int ==> Bool -> Int
                        r.clone()
                    } else if l != &Never && self.is_class(l) && variance == Covariant {
                        // |T :> Int| X -> T ==> X -> Int
                        l.clone()
                    } else {
                        fv.update_constraint(
                            self.generalize_constraint(&fv.crack_constraint(), variance),
                            true,
                        );
                        fv.generalize();
                        Type::FreeVar(fv)
                    }
                } else {
                    // ?S(: Str) => 'S
                    fv.update_constraint(
                        self.generalize_constraint(&fv.crack_constraint(), variance),
                        true,
                    );
                    fv.generalize();
                    Type::FreeVar(fv)
                }
            }
            Subr(mut subr) => {
                subr.non_default_params.iter_mut().for_each(|nd_param| {
                    *nd_param.typ_mut() = self.generalize_t_inner(
                        mem::take(nd_param.typ_mut()),
                        Contravariant,
                        uninit,
                    );
                });
                if let Some(var_args) = &mut subr.var_params {
                    *var_args.typ_mut() = self.generalize_t_inner(
                        mem::take(var_args.typ_mut()),
                        Contravariant,
                        uninit,
                    );
                }
                subr.default_params.iter_mut().for_each(|d_param| {
                    *d_param.typ_mut() = self.generalize_t_inner(
                        mem::take(d_param.typ_mut()),
                        Contravariant,
                        uninit,
                    );
                });
                let return_t = self.generalize_t_inner(*subr.return_t, Covariant, uninit);
                subr_t(
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|x| *x),
                    subr.default_params,
                    return_t,
                )
            }
            Callable { .. } => todo!(),
            Ref(t) => ref_(self.generalize_t_inner(*t, variance, uninit)),
            RefMut { before, after } => {
                let after = after.map(|aft| self.generalize_t_inner(*aft, variance, uninit));
                ref_mut(self.generalize_t_inner(*before, variance, uninit), after)
            }
            Refinement(refine) => {
                let t = self.generalize_t_inner(*refine.t, variance, uninit);
                let preds = refine
                    .preds
                    .into_iter()
                    .map(|pred| self.generalize_pred(pred, variance, uninit))
                    .collect();
                refinement(refine.var, t, preds)
            }
            Poly { name, mut params } => {
                let params = params
                    .iter_mut()
                    .map(|p| self.generalize_tp(mem::take(p), variance, uninit))
                    .collect::<Vec<_>>();
                poly(name, params)
            }
            Proj { lhs, rhs } => {
                let lhs = self.generalize_t_inner(*lhs, variance, uninit);
                proj(lhs, rhs)
            }
            ProjCall {
                lhs,
                attr_name,
                mut args,
            } => {
                let lhs = self.generalize_tp(*lhs, variance, uninit);
                for arg in args.iter_mut() {
                    *arg = self.generalize_tp(mem::take(arg), variance, uninit);
                }
                proj_call(lhs, attr_name, args)
            }
            And(l, r) => {
                let l = self.generalize_t_inner(*l, variance, uninit);
                let r = self.generalize_t_inner(*r, variance, uninit);
                // not `self.intersection` because types are generalized
                and(l, r)
            }
            Or(l, r) => {
                let l = self.generalize_t_inner(*l, variance, uninit);
                let r = self.generalize_t_inner(*r, variance, uninit);
                // not `self.union` because types are generalized
                or(l, r)
            }
            Not(l) => not(self.generalize_t_inner(*l, variance, uninit)),
            // REVIEW: その他何でもそのまま通していいのか?
            other => other,
        }
    }

    fn generalize_constraint(&self, constraint: &Constraint, variance: Variance) -> Constraint {
        match constraint {
            Constraint::Sandwiched { sub, sup, .. } => {
                let sub = self.generalize_t_inner(sub.clone(), variance, true);
                let sup = self.generalize_t_inner(sup.clone(), variance, true);
                Constraint::new_sandwiched(sub, sup)
            }
            Constraint::TypeOf(t) => {
                let t = self.generalize_t_inner(t.clone(), variance, true);
                Constraint::new_type_of(t)
            }
            Constraint::Uninited => unreachable!(),
        }
    }

    fn generalize_pred(&self, pred: Predicate, variance: Variance, uninit: bool) -> Predicate {
        match pred {
            Predicate::Const(_) => pred,
            Predicate::Value(ValueObj::Type(mut typ)) => {
                *typ.typ_mut() =
                    self.generalize_t_inner(mem::take(typ.typ_mut()), variance, uninit);
                Predicate::Value(ValueObj::Type(typ))
            }
            Predicate::Value(_) => pred,
            Predicate::Equal { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, variance, uninit);
                Predicate::eq(lhs, rhs)
            }
            Predicate::GreaterEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, variance, uninit);
                Predicate::ge(lhs, rhs)
            }
            Predicate::LessEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, variance, uninit);
                Predicate::le(lhs, rhs)
            }
            Predicate::NotEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, variance, uninit);
                Predicate::ne(lhs, rhs)
            }
            Predicate::And(lhs, rhs) => {
                let lhs = self.generalize_pred(*lhs, variance, uninit);
                let rhs = self.generalize_pred(*rhs, variance, uninit);
                Predicate::and(lhs, rhs)
            }
            Predicate::Or(lhs, rhs) => {
                let lhs = self.generalize_pred(*lhs, variance, uninit);
                let rhs = self.generalize_pred(*rhs, variance, uninit);
                Predicate::or(lhs, rhs)
            }
            Predicate::Not(pred) => {
                let pred = self.generalize_pred(*pred, variance, uninit);
                Predicate::not(pred)
            }
        }
    }

    pub(crate) fn deref_tp(
        &self,
        tp: TyParam,
        variance: Variance,
        loc: &impl Locational,
    ) -> TyCheckResult<TyParam> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let inner = fv.unwrap_linked();
                self.deref_tp(inner, variance, loc)
            }
            TyParam::FreeVar(_fv) if self.level == 0 => Err(TyCheckErrors::from(
                TyCheckError::dummy_infer_error(self.cfg.input.clone(), fn_name!(), line!()),
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
            TyParam::Dict(dic) => {
                let mut new_dic = dict! {};
                for (k, v) in dic.into_iter() {
                    new_dic.insert(
                        self.deref_tp(k, variance, loc)?,
                        self.deref_tp(v, variance, loc)?,
                    );
                }
                Ok(TyParam::Dict(new_dic))
            }
            TyParam::Set(set) => {
                let mut new_set = set! {};
                for v in set.into_iter() {
                    new_set.insert(self.deref_tp(v, variance, loc)?);
                }
                Ok(TyParam::Set(new_set))
            }
            TyParam::Proj { .. } | TyParam::Failure if self.level == 0 => Err(TyCheckErrors::from(
                TyCheckError::dummy_infer_error(self.cfg.input.clone(), fn_name!(), line!()),
            )),
            t => Ok(t),
        }
    }

    fn deref_constraint(
        &self,
        constraint: Constraint,
        variance: Variance,
        loc: &impl Locational,
    ) -> TyCheckResult<Constraint> {
        match constraint {
            Constraint::Sandwiched { sub, sup } => Ok(Constraint::new_sandwiched(
                self.deref_tyvar(sub, variance, loc)?,
                self.deref_tyvar(sup, variance, loc)?,
            )),
            Constraint::TypeOf(t) => {
                Ok(Constraint::new_type_of(self.deref_tyvar(t, variance, loc)?))
            }
            _ => unreachable!(),
        }
    }

    fn validate_subsup(
        &self,
        sub_t: Type,
        super_t: Type,
        variance: Variance,
        loc: &impl Locational,
    ) -> TyCheckResult<Type> {
        // TODO: Subr, ...
        match (sub_t, super_t) {
            // See tests\should_err\subtyping.er:8~13
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
                let typ = poly(ln, lps.clone());
                let (_, ctx) = self.get_nominal_type_ctx(&typ).ok_or_else(|| {
                    TyCheckError::type_not_found(
                        self.cfg.input.clone(),
                        line!() as usize,
                        loc.loc(),
                        self.caused_by(),
                        &typ,
                    )
                })?;
                let variances = ctx.type_params_variance();
                let mut tps = vec![];
                for ((lp, rp), variance) in lps
                    .into_iter()
                    .zip(rps.into_iter())
                    .zip(variances.into_iter())
                {
                    self.sub_unify_tp(&lp, &rp, Some(variance), loc, false)?;
                    let param = if variance == Covariant { lp } else { rp };
                    tps.push(param);
                }
                Ok(poly(rn, tps))
            }
            (sub_t, super_t) => self.validate_simple_subsup(sub_t, super_t, variance, loc),
        }
    }

    fn validate_simple_subsup(
        &self,
        sub_t: Type,
        super_t: Type,
        variance: Variance,
        loc: &impl Locational,
    ) -> TyCheckResult<Type> {
        if self.is_trait(&super_t) {
            self.check_trait_impl(&sub_t, &super_t, loc)?;
        }
        // REVIEW: Even if type constraints can be satisfied, implementation may not exist
        if self.subtype_of(&sub_t, &super_t) {
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
            match variance {
                Variance::Covariant => Ok(sub_t),
                Variance::Contravariant => Ok(super_t),
                Variance::Invariant => {
                    // need to check if sub_t == super_t
                    if self.supertype_of(&sub_t, &super_t) {
                        Ok(sub_t)
                    } else {
                        Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            &sub_t,
                            &super_t,
                            loc.loc(),
                            self.caused_by(),
                        )))
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
            Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                self.cfg.input.clone(),
                line!() as usize,
                &sub_t,
                &super_t,
                loc.loc(),
                self.caused_by(),
            )))
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
        loc: &impl Locational,
    ) -> TyCheckResult<Type> {
        match t {
            // ?T(:> Nat, <: Int)[n] ==> Nat (self.level <= n)
            // ?T(:> Nat, <: Sub ?U(:> {1}))[n] ==> Nat
            // ?T(<: Int, :> Add(?T)) ==> Int
            // ?T(:> Nat, <: Sub(Str)) ==> Error!
            // ?T(:> {1, "a"}, <: Eq(?T(:> {1, "a"}, ...)) ==> Error!
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub_t, super_t) = fv.get_subsup().unwrap();
                if self.level <= fv.level().unwrap() {
                    // if fv == ?T(<: Int, :> Add(?T)), deref_tyvar(super_t) will cause infinite loop
                    // so we need to force linking
                    fv.forced_undoable_link(&sub_t);
                    let res = self.validate_subsup(sub_t, super_t, variance, loc);
                    fv.undo();
                    res
                } else {
                    // no dereference at this point
                    // drop(constraint);
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::FreeVar(fv) if fv.is_unbound() => {
                if self.level == 0 {
                    #[allow(clippy::single_match)]
                    match &*fv.crack_constraint() {
                        Constraint::TypeOf(_) => {
                            return Err(TyCheckErrors::from(TyCheckError::dummy_infer_error(
                                self.cfg.input.clone(),
                                fn_name!(),
                                line!(),
                            )));
                        }
                        _ => {}
                    }
                    Ok(Type::FreeVar(fv))
                } else {
                    let new_constraint = fv.crack_constraint().clone();
                    let new_constraint = self.deref_constraint(new_constraint, variance, loc)?;
                    fv.update_constraint(new_constraint, true);
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.unwrap_linked();
                self.deref_tyvar(t, variance, loc)
            }
            Type::Poly { name, mut params } => {
                let typ = poly(&name, params.clone());
                let (_, ctx) = self.get_nominal_type_ctx(&typ).ok_or_else(|| {
                    TyCheckError::type_not_found(
                        self.cfg.input.clone(),
                        line!() as usize,
                        loc.loc(),
                        self.caused_by(),
                        &typ,
                    )
                })?;
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
            Type::Quantified(subr)
                if subr.return_t().map(|ret| !ret.has_qvar()).unwrap_or(false) =>
            {
                let subr = self.deref_tyvar(*subr, variance, loc)?;
                Ok(subr)
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
            // Type::Callable { .. } => todo!(),
            Type::Record(mut rec) => {
                for (_, field) in rec.iter_mut() {
                    *field = self.deref_tyvar(mem::take(field), variance, loc)?;
                }
                Ok(Type::Record(rec))
            }
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
            Type::Not(ty) => {
                let ty = self.deref_tyvar(*ty, variance, loc)?;
                Ok(self.complement(&ty))
            }
            Type::Proj { lhs, rhs } => {
                let lhs = self.deref_tyvar(*lhs, variance, loc)?;
                self.eval_proj(lhs, rhs, self.level, loc)
            }
            Type::ProjCall {
                lhs,
                attr_name,
                args,
            } => {
                let lhs = self.deref_tp(*lhs, variance, loc)?;
                let mut new_args = vec![];
                for arg in args.into_iter() {
                    new_args.push(self.deref_tp(arg, variance, loc)?);
                }
                self.eval_proj_call(lhs, attr_name, new_args, self.level, loc)
            }
            t => Ok(t),
        }
    }

    pub fn readable_type(&self, t: Type, is_parameter: bool) -> Type {
        let variance = if is_parameter {
            Contravariant
        } else {
            Covariant
        };
        self.deref_tyvar(t.clone(), variance, &()).unwrap_or(t)
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
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        if !self.trait_impl_exists(class, trait_) {
            let class = if cfg!(feature = "debug") {
                class.clone()
            } else {
                self.deref_tyvar(class.clone(), Variance::Covariant, loc)?
            };
            let trait_ = if cfg!(feature = "debug") {
                trait_.clone()
            } else {
                self.deref_tyvar(trait_.clone(), Variance::Covariant, loc)?
            };
            Err(TyCheckErrors::from(TyCheckError::no_trait_impl_error(
                self.cfg.input.clone(),
                line!() as usize,
                &class,
                &trait_,
                loc.loc(),
                self.caused_by(),
                self.get_simple_type_mismatch_hint(&trait_, &class),
            )))
        } else {
            Ok(())
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
            if let Err(es) = self.resolve_expr_t(chunk) {
                errs.extend(es);
            }
        }
        self.resolve_ctx_vars();
        if errs.is_empty() {
            Ok(hir)
        } else {
            Err((hir, errs))
        }
    }

    fn resolve_ctx_vars(&mut self) {
        let mut locals = mem::take(&mut self.locals);
        let mut params = mem::take(&mut self.params);
        let mut methods_list = mem::take(&mut self.methods_list);
        for (name, vi) in locals.iter_mut() {
            if let Ok(t) = self.deref_tyvar(mem::take(&mut vi.t), Variance::Covariant, name) {
                vi.t = t;
            }
        }
        for (name, vi) in params.iter_mut() {
            if let Ok(t) = self.deref_tyvar(mem::take(&mut vi.t), Variance::Covariant, name) {
                vi.t = t;
            }
        }
        for (_, methods) in methods_list.iter_mut() {
            methods.resolve_ctx_vars();
        }
        self.locals = locals;
        self.params = params;
        self.methods_list = methods_list;
    }

    fn resolve_params_t(&self, params: &mut hir::Params) -> TyCheckResult<()> {
        for param in params.non_defaults.iter_mut() {
            param.vi.t = self.deref_tyvar(mem::take(&mut param.vi.t), Contravariant, param)?;
        }
        if let Some(var_params) = &mut params.var_params {
            var_params.vi.t = self.deref_tyvar(
                mem::take(&mut var_params.vi.t),
                Contravariant,
                var_params.as_ref(),
            )?;
        }
        for param in params.defaults.iter_mut() {
            param.sig.vi.t =
                self.deref_tyvar(mem::take(&mut param.sig.vi.t), Contravariant, param)?;
            self.resolve_expr_t(&mut param.default_val)?;
        }
        Ok(())
    }

    fn resolve_expr_t(&self, expr: &mut hir::Expr) -> TyCheckResult<()> {
        match expr {
            hir::Expr::Lit(_) => Ok(()),
            hir::Expr::Accessor(acc) => {
                let variance = if acc.var_info().kind.is_parameter() {
                    Contravariant
                } else {
                    Covariant
                };
                let t = mem::take(acc.ref_mut_t());
                *acc.ref_mut_t() = self.deref_tyvar(t, variance, acc)?;
                if let hir::Accessor::Attr(attr) = acc {
                    self.resolve_expr_t(&mut attr.obj)?;
                }
                Ok(())
            }
            hir::Expr::Array(array) => match array {
                hir::Array::Normal(arr) => {
                    arr.t = self.deref_tyvar(mem::take(&mut arr.t), Covariant, arr)?;
                    for elem in arr.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr)?;
                    }
                    Ok(())
                }
                hir::Array::WithLength(arr) => {
                    arr.t = self.deref_tyvar(mem::take(&mut arr.t), Covariant, arr)?;
                    self.resolve_expr_t(&mut arr.elem)?;
                    self.resolve_expr_t(&mut arr.len)?;
                    Ok(())
                }
                other => feature_error!(
                    TyCheckErrors,
                    TyCheckError,
                    self,
                    other.loc(),
                    "resolve types of array comprehension"
                ),
            },
            hir::Expr::Tuple(tuple) => match tuple {
                hir::Tuple::Normal(tup) => {
                    tup.t = self.deref_tyvar(mem::take(&mut tup.t), Covariant, tup)?;
                    for elem in tup.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr)?;
                    }
                    Ok(())
                }
            },
            hir::Expr::Set(set) => match set {
                hir::Set::Normal(st) => {
                    st.t = self.deref_tyvar(mem::take(&mut st.t), Covariant, st)?;
                    for elem in st.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr)?;
                    }
                    Ok(())
                }
                hir::Set::WithLength(st) => {
                    st.t = self.deref_tyvar(mem::take(&mut st.t), Covariant, st)?;
                    self.resolve_expr_t(&mut st.elem)?;
                    self.resolve_expr_t(&mut st.len)?;
                    Ok(())
                }
            },
            hir::Expr::Dict(dict) => match dict {
                hir::Dict::Normal(dic) => {
                    dic.t = self.deref_tyvar(mem::take(&mut dic.t), Covariant, dic)?;
                    for kv in dic.kvs.iter_mut() {
                        self.resolve_expr_t(&mut kv.key)?;
                        self.resolve_expr_t(&mut kv.value)?;
                    }
                    Ok(())
                }
                other => feature_error!(
                    TyCheckErrors,
                    TyCheckError,
                    self,
                    other.loc(),
                    "resolve types of dict comprehension"
                ),
            },
            hir::Expr::Record(record) => {
                record.t = self.deref_tyvar(mem::take(&mut record.t), Covariant, record)?;
                for attr in record.attrs.iter_mut() {
                    match &mut attr.sig {
                        hir::Signature::Var(var) => {
                            *var.ref_mut_t() =
                                self.deref_tyvar(mem::take(var.ref_mut_t()), Covariant, var)?;
                        }
                        hir::Signature::Subr(subr) => {
                            *subr.ref_mut_t() =
                                self.deref_tyvar(mem::take(subr.ref_mut_t()), Covariant, subr)?;
                        }
                    }
                    for chunk in attr.body.block.iter_mut() {
                        self.resolve_expr_t(chunk)?;
                    }
                }
                Ok(())
            }
            hir::Expr::BinOp(binop) => {
                let t = mem::take(binop.signature_mut_t().unwrap());
                *binop.signature_mut_t().unwrap() = self.deref_tyvar(t, Covariant, binop)?;
                self.resolve_expr_t(&mut binop.lhs)?;
                self.resolve_expr_t(&mut binop.rhs)?;
                Ok(())
            }
            hir::Expr::UnaryOp(unaryop) => {
                let t = mem::take(unaryop.signature_mut_t().unwrap());
                *unaryop.signature_mut_t().unwrap() = self.deref_tyvar(t, Covariant, unaryop)?;
                self.resolve_expr_t(&mut unaryop.expr)?;
                Ok(())
            }
            hir::Expr::Call(call) => {
                if let Some(t) = call.signature_mut_t() {
                    let t = mem::take(t);
                    *call.signature_mut_t().unwrap() = self.deref_tyvar(t, Covariant, call)?;
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
                *def.sig.ref_mut_t() =
                    self.deref_tyvar(mem::take(def.sig.ref_mut_t()), Covariant, &def.sig)?;
                if let Some(params) = def.sig.params_mut() {
                    self.resolve_params_t(params)?;
                }
                for chunk in def.body.block.iter_mut() {
                    self.resolve_expr_t(chunk)?;
                }
                Ok(())
            }
            hir::Expr::Lambda(lambda) => {
                lambda.t = self.deref_tyvar(mem::take(&mut lambda.t), Covariant, lambda)?;
                self.resolve_params_t(&mut lambda.params)?;
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
            hir::Expr::PatchDef(patch_def) => {
                for def in patch_def.methods.iter_mut() {
                    self.resolve_expr_t(def)?;
                }
                Ok(())
            }
            hir::Expr::ReDef(redef) => {
                // REVIEW: redef.attr is not dereferenced
                for chunk in redef.block.iter_mut() {
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
            hir::Expr::Dummy(chunks) => {
                for chunk in chunks.iter_mut() {
                    self.resolve_expr_t(chunk)?;
                }
                Ok(())
            }
            hir::Expr::Import(_) => unreachable!(),
        }
    }
}
