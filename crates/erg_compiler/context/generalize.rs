use std::mem;

use erg_common::consts::DEBUG_MODE;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{dict, fn_name, set};
#[allow(unused_imports)]
use erg_common::{fmt_vec, log};

use crate::ty::constructors::*;
use crate::ty::free::{CanbeFree, Constraint, Free, HasLevel};
use crate::ty::typaram::{TyParam, TyParamLambda};
use crate::ty::value::ValueObj;
use crate::ty::{HasType, Predicate, SubrType, Type};

use crate::context::{Context, Variance};
use crate::error::{TyCheckError, TyCheckErrors, TyCheckResult};
use crate::{feature_error, hir};

use Type::*;
use Variance::*;

pub struct Generalizer {
    level: usize,
    variance: Variance,
    qnames: Set<Str>,
    structural_inner: bool,
}

impl Generalizer {
    pub fn new(level: usize) -> Self {
        Self {
            level,
            variance: Covariant,
            qnames: set! {},
            structural_inner: false,
        }
    }

    fn generalize_tp(&mut self, free: TyParam, uninit: bool) -> TyParam {
        match free {
            TyParam::Type(t) => TyParam::t(self.generalize_t(*t, uninit)),
            TyParam::FreeVar(fv) if fv.is_generalized() => TyParam::FreeVar(fv),
            TyParam::FreeVar(fv) if fv.is_linked() => {
                self.generalize_tp(fv.crack().clone(), uninit)
            }
            // TODO: Polymorphic generalization
            TyParam::FreeVar(fv) if fv.level() > Some(self.level) => {
                let constr = self.generalize_constraint(&fv);
                fv.update_constraint(constr, true);
                fv.generalize();
                TyParam::FreeVar(fv)
            }
            TyParam::Array(tps) => TyParam::Array(
                tps.into_iter()
                    .map(|tp| self.generalize_tp(tp, uninit))
                    .collect(),
            ),
            TyParam::Tuple(tps) => TyParam::Tuple(
                tps.into_iter()
                    .map(|tp| self.generalize_tp(tp, uninit))
                    .collect(),
            ),
            TyParam::Dict(tps) => TyParam::Dict(
                tps.into_iter()
                    .map(|(k, v)| (self.generalize_tp(k, uninit), self.generalize_tp(v, uninit)))
                    .collect(),
            ),
            TyParam::Record(rec) => TyParam::Record(
                rec.into_iter()
                    .map(|(field, tp)| (field, self.generalize_tp(tp, uninit)))
                    .collect(),
            ),
            TyParam::Lambda(lambda) => {
                let nd_params = lambda
                    .nd_params
                    .into_iter()
                    .map(|pt| pt.map_type(|t| self.generalize_t(t, uninit)))
                    .collect::<Vec<_>>();
                let var_params = lambda
                    .var_params
                    .map(|pt| pt.map_type(|t| self.generalize_t(t, uninit)));
                let d_params = lambda
                    .d_params
                    .into_iter()
                    .map(|pt| pt.map_type(|t| self.generalize_t(t, uninit)))
                    .collect::<Vec<_>>();
                let body = lambda
                    .body
                    .into_iter()
                    .map(|tp| self.generalize_tp(tp, uninit))
                    .collect();
                TyParam::Lambda(TyParamLambda::new(
                    lambda.const_,
                    nd_params,
                    var_params,
                    d_params,
                    body,
                ))
            }
            TyParam::FreeVar(_) => free,
            TyParam::Proj { obj, attr } => {
                let obj = self.generalize_tp(*obj, uninit);
                TyParam::proj(obj, attr)
            }
            TyParam::Erased(t) => TyParam::erased(self.generalize_t(*t, uninit)),
            TyParam::App { name, args } => {
                let args = args
                    .into_iter()
                    .map(|tp| self.generalize_tp(tp, uninit))
                    .collect();
                TyParam::App { name, args }
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let lhs = self.generalize_tp(*lhs, uninit);
                let rhs = self.generalize_tp(*rhs, uninit);
                TyParam::bin(op, lhs, rhs)
            }
            TyParam::UnaryOp { op, val } => {
                let val = self.generalize_tp(*val, uninit);
                TyParam::unary(op, val)
            }
            other if other.has_no_unbound_var() => other,
            other => todo!("{other}"),
        }
    }

    /// see doc/LANG/compiler/inference.md#一般化 for details
    /// ```python
    /// generalize_t(?T) == 'T: Type
    /// generalize_t(?T(<: Nat) -> ?T) == |'T <: Nat| 'T -> 'T
    /// generalize_t(?T(<: Add(?T(<: Eq(?T(<: ...)))) -> ?T) == |'T <: Add('T)| 'T -> 'T
    /// generalize_t(?T(<: TraitX) -> Int) == TraitX -> Int // 戻り値に現れないなら量化しない
    /// ```
    fn generalize_t(&mut self, free_type: Type, uninit: bool) -> Type {
        match free_type {
            FreeVar(fv) if fv.is_linked() => self.generalize_t(fv.unsafe_crack().clone(), uninit),
            FreeVar(fv) if fv.is_generalized() => Type::FreeVar(fv),
            // TODO: Polymorphic generalization
            FreeVar(fv) if fv.level().unwrap() > self.level => {
                fv.generalize();
                if uninit {
                    return Type::FreeVar(fv);
                }
                if let Some((sub, sup)) = fv.get_subsup() {
                    // |Int <: T <: Int| T -> T ==> Int -> Int
                    if sub == sup {
                        let t = self.generalize_t(sub, uninit);
                        let res = FreeVar(fv);
                        res.link(&t);
                        res
                    } else if sup != Obj
                        && !self.qnames.contains(&fv.unbound_name().unwrap())
                        && self.variance == Contravariant
                    {
                        // |T <: Bool| T -> Int ==> Bool -> Int
                        self.generalize_t(sup, uninit)
                    } else if sub != Never
                        && !self.qnames.contains(&fv.unbound_name().unwrap())
                        && self.variance == Covariant
                    {
                        // |T :> Int| X -> T ==> X -> Int
                        self.generalize_t(sub, uninit)
                    } else {
                        fv.update_constraint(self.generalize_constraint(&fv), true);
                        Type::FreeVar(fv)
                    }
                } else {
                    // ?S(: Str) => 'S
                    fv.update_constraint(self.generalize_constraint(&fv), true);
                    Type::FreeVar(fv)
                }
            }
            Subr(mut subr) => {
                self.variance = Contravariant;
                let qnames = subr.essential_qnames();
                self.qnames.extend(qnames.clone());
                subr.non_default_params.iter_mut().for_each(|nd_param| {
                    *nd_param.typ_mut() = self.generalize_t(mem::take(nd_param.typ_mut()), uninit);
                });
                if let Some(var_args) = &mut subr.var_params {
                    *var_args.typ_mut() = self.generalize_t(mem::take(var_args.typ_mut()), uninit);
                }
                subr.default_params.iter_mut().for_each(|d_param| {
                    *d_param.typ_mut() = self.generalize_t(mem::take(d_param.typ_mut()), uninit);
                });
                self.variance = Covariant;
                let return_t = self.generalize_t(*subr.return_t, uninit);
                self.qnames = self.qnames.difference(&qnames);
                subr_t(
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|x| *x),
                    subr.default_params,
                    return_t,
                )
            }
            Record(rec) => {
                let fields = rec
                    .into_iter()
                    .map(|(name, t)| (name, self.generalize_t(t, uninit)))
                    .collect();
                Type::Record(fields)
            }
            Callable { .. } => todo!(),
            Ref(t) => ref_(self.generalize_t(*t, uninit)),
            RefMut { before, after } => {
                let after = after.map(|aft| self.generalize_t(*aft, uninit));
                ref_mut(self.generalize_t(*before, uninit), after)
            }
            Refinement(refine) => {
                let t = self.generalize_t(*refine.t, uninit);
                let pred = self.generalize_pred(*refine.pred, uninit);
                refinement(refine.var, t, pred)
            }
            Poly { name, mut params } => {
                let params = params
                    .iter_mut()
                    .map(|p| self.generalize_tp(mem::take(p), uninit))
                    .collect::<Vec<_>>();
                poly(name, params)
            }
            Proj { lhs, rhs } => {
                let lhs = self.generalize_t(*lhs, uninit);
                proj(lhs, rhs)
            }
            ProjCall {
                lhs,
                attr_name,
                mut args,
            } => {
                let lhs = self.generalize_tp(*lhs, uninit);
                for arg in args.iter_mut() {
                    *arg = self.generalize_tp(mem::take(arg), uninit);
                }
                proj_call(lhs, attr_name, args)
            }
            And(l, r) => {
                let l = self.generalize_t(*l, uninit);
                let r = self.generalize_t(*r, uninit);
                // not `self.intersection` because types are generalized
                and(l, r)
            }
            Or(l, r) => {
                let l = self.generalize_t(*l, uninit);
                let r = self.generalize_t(*r, uninit);
                // not `self.union` because types are generalized
                or(l, r)
            }
            Not(l) => not(self.generalize_t(*l, uninit)),
            Structural(ty) => {
                if self.structural_inner {
                    ty.structuralize()
                } else {
                    if ty.is_recursive() {
                        self.structural_inner = true;
                    }
                    let res = self.generalize_t(*ty, uninit).structuralize();
                    self.structural_inner = false;
                    res
                }
            }
            // REVIEW: その他何でもそのまま通していいのか?
            other => other,
        }
    }

    fn generalize_constraint<T: CanbeFree>(&mut self, fv: &Free<T>) -> Constraint {
        if let Some((sub, sup)) = fv.get_subsup() {
            let sub = self.generalize_t(sub, true);
            let sup = self.generalize_t(sup, true);
            Constraint::new_sandwiched(sub, sup)
        } else if let Some(ty) = fv.get_type() {
            let t = self.generalize_t(ty, true);
            Constraint::new_type_of(t)
        } else {
            unreachable!()
        }
    }

    fn generalize_pred(&mut self, pred: Predicate, uninit: bool) -> Predicate {
        match pred {
            Predicate::Const(_) => pred,
            Predicate::Value(ValueObj::Type(mut typ)) => {
                *typ.typ_mut() = self.generalize_t(mem::take(typ.typ_mut()), uninit);
                Predicate::Value(ValueObj::Type(typ))
            }
            Predicate::Value(_) => pred,
            Predicate::Equal { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, uninit);
                Predicate::eq(lhs, rhs)
            }
            Predicate::GreaterEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, uninit);
                Predicate::ge(lhs, rhs)
            }
            Predicate::LessEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, uninit);
                Predicate::le(lhs, rhs)
            }
            Predicate::NotEqual { lhs, rhs } => {
                let rhs = self.generalize_tp(rhs, uninit);
                Predicate::ne(lhs, rhs)
            }
            Predicate::And(lhs, rhs) => {
                let lhs = self.generalize_pred(*lhs, uninit);
                let rhs = self.generalize_pred(*rhs, uninit);
                Predicate::and(lhs, rhs)
            }
            Predicate::Or(lhs, rhs) => {
                let lhs = self.generalize_pred(*lhs, uninit);
                let rhs = self.generalize_pred(*rhs, uninit);
                Predicate::or(lhs, rhs)
            }
            Predicate::Not(pred) => {
                let pred = self.generalize_pred(*pred, uninit);
                !pred
            }
        }
    }
}

pub struct Dereferencer<'c, 'q, 'l, L: Locational> {
    ctx: &'c Context,
    variance: Variance,
    coerce: bool,
    stash: Variance,
    qnames: &'q Set<Str>,
    loc: &'l L,
}

impl<'c, 'q, 'l, L: Locational> Dereferencer<'c, 'q, 'l, L> {
    pub fn new(
        ctx: &'c Context,
        variance: Variance,
        coerce: bool,
        qnames: &'q Set<Str>,
        loc: &'l L,
    ) -> Self {
        Self {
            ctx,
            variance,
            coerce,
            stash: Variance::Invariant,
            qnames,
            loc,
        }
    }

    pub fn simple(ctx: &'c Context, qnames: &'q Set<Str>, loc: &'l L) -> Self {
        Self::new(ctx, Variance::Covariant, true, qnames, loc)
    }

    fn push_variance(&mut self, variance: Variance) {
        self.stash = self.variance;
        self.variance = variance; // self.variance * variance;
    }

    fn pop_variance(&mut self) {
        self.variance = self.stash;
    }

    pub(crate) fn deref_tp(&mut self, tp: TyParam) -> TyCheckResult<TyParam> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let inner = fv.unwrap_linked();
                self.deref_tp(inner)
            }
            TyParam::FreeVar(fv)
                if fv.is_generalized() && self.qnames.contains(&fv.unbound_name().unwrap()) =>
            {
                Ok(TyParam::FreeVar(fv))
            }
            // REVIEW:
            TyParam::FreeVar(_) if self.ctx.level == 0 => {
                Ok(TyParam::erased(self.ctx.get_tp_t(&tp).unwrap_or(Type::Obj)))
            }
            TyParam::Type(t) => Ok(TyParam::t(self.deref_tyvar(*t)?)),
            TyParam::Erased(t) => Ok(TyParam::erased(self.deref_tyvar(*t)?)),
            TyParam::App { name, mut args } => {
                for param in args.iter_mut() {
                    *param = self.deref_tp(mem::take(param))?;
                }
                Ok(TyParam::App { name, args })
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let lhs = self.deref_tp(*lhs)?;
                let rhs = self.deref_tp(*rhs)?;
                Ok(TyParam::BinOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                })
            }
            TyParam::UnaryOp { op, val } => {
                let val = self.deref_tp(*val)?;
                Ok(TyParam::UnaryOp {
                    op,
                    val: Box::new(val),
                })
            }
            TyParam::Array(tps) => {
                let mut new_tps = vec![];
                for tp in tps {
                    new_tps.push(self.deref_tp(tp)?);
                }
                Ok(TyParam::Array(new_tps))
            }
            TyParam::Tuple(tps) => {
                let mut new_tps = vec![];
                for tp in tps {
                    new_tps.push(self.deref_tp(tp)?);
                }
                Ok(TyParam::Tuple(new_tps))
            }
            TyParam::Dict(dic) => {
                let mut new_dic = dict! {};
                for (k, v) in dic.into_iter() {
                    let k = self.deref_tp(k)?;
                    let v = self.deref_tp(v)?;
                    new_dic
                        .entry(k)
                        .and_modify(|old_v| {
                            *old_v = self.ctx.union_tp(&mem::take(old_v), &v).unwrap();
                        })
                        .or_insert(v);
                }
                Ok(TyParam::Dict(new_dic))
            }
            TyParam::Set(set) => {
                let mut new_set = set! {};
                for v in set.into_iter() {
                    new_set.insert(self.deref_tp(v)?);
                }
                Ok(TyParam::Set(new_set))
            }
            TyParam::Record(rec) => {
                let mut new_rec = dict! {};
                for (field, tp) in rec.into_iter() {
                    new_rec.insert(field, self.deref_tp(tp)?);
                }
                Ok(TyParam::Record(new_rec))
            }
            TyParam::Lambda(lambda) => {
                let nd_params = lambda
                    .nd_params
                    .into_iter()
                    .map(|pt| pt.try_map_type(|t| self.deref_tyvar(t)))
                    .collect::<TyCheckResult<_>>()?;
                let var_params = lambda
                    .var_params
                    .map(|pt| pt.try_map_type(|t| self.deref_tyvar(t)))
                    .transpose()?;
                let d_params = lambda
                    .d_params
                    .into_iter()
                    .map(|pt| pt.try_map_type(|t| self.deref_tyvar(t)))
                    .collect::<TyCheckResult<_>>()?;
                let body = lambda
                    .body
                    .into_iter()
                    .map(|tp| self.deref_tp(tp))
                    .collect::<TyCheckResult<Vec<_>>>()?;
                Ok(TyParam::Lambda(TyParamLambda::new(
                    lambda.const_,
                    nd_params,
                    var_params,
                    d_params,
                    body,
                )))
            }
            TyParam::Proj { obj, attr } => {
                let obj = self.deref_tp(*obj)?;
                Ok(TyParam::Proj {
                    obj: Box::new(obj),
                    attr,
                })
            }
            TyParam::Failure if self.ctx.level == 0 => Err(TyCheckErrors::from(
                TyCheckError::dummy_infer_error(self.ctx.cfg.input.clone(), fn_name!(), line!()),
            )),
            t => Ok(t),
        }
    }

    fn deref_constraint(&mut self, constraint: Constraint) -> TyCheckResult<Constraint> {
        match constraint {
            Constraint::Sandwiched { sub, sup } => Ok(Constraint::new_sandwiched(
                self.deref_tyvar(sub)?,
                self.deref_tyvar(sup)?,
            )),
            Constraint::TypeOf(t) => Ok(Constraint::new_type_of(self.deref_tyvar(t)?)),
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
    pub(crate) fn deref_tyvar(&mut self, t: Type) -> TyCheckResult<Type> {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.unwrap_linked();
                self.deref_tyvar(t)
            }
            Type::FreeVar(fv)
                if fv.is_generalized() && self.qnames.contains(&fv.unbound_name().unwrap()) =>
            {
                Ok(Type::FreeVar(fv))
            }
            // ?T(:> Nat, <: Int)[n] ==> Nat (self.level <= n)
            // ?T(:> Nat, <: Sub ?U(:> {1}))[n] ==> Nat
            // ?T(<: Int, :> Add(?T)) ==> Int
            // ?T(:> Nat, <: Sub(Str)) ==> Error!
            // ?T(:> {1, "a"}, <: Eq(?T(:> {1, "a"}, ...)) ==> Error!
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub_t, super_t) = fv.get_subsup().unwrap();
                if self.ctx.level <= fv.level().unwrap() {
                    // we need to force linking to avoid infinite loop
                    // e.g. fv == ?T(<: Int, :> Add(?T))
                    //      fv == ?T(:> ?T.Output, <: Add(Int))
                    let fv_t = Type::FreeVar(fv.clone());
                    match (sub_t.contains(&fv_t), super_t.contains(&fv_t)) {
                        // REVIEW: to prevent infinite recursion, but this may cause a nonsense error
                        (true, true) => {
                            fv.dummy_link();
                        }
                        (true, false) => {
                            fv_t.undoable_link(&super_t);
                        }
                        (false, true | false) => {
                            fv_t.undoable_link(&sub_t);
                        }
                    }
                    let res = self.validate_subsup(sub_t, super_t);
                    fv.undo();
                    match res {
                        Ok(ty) => {
                            // TODO: T(:> Nat <: Int) -> T(:> Nat, <: Int) ==> Int -> Nat
                            // fv.link(&ty);
                            Ok(ty)
                        }
                        Err(errs) => {
                            Type::FreeVar(fv).link(&Never);
                            Err(errs)
                        }
                    }
                } else {
                    // no dereference at this point
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::FreeVar(fv) if fv.is_unbound() => {
                if self.ctx.level == 0 {
                    #[allow(clippy::single_match)]
                    match &*fv.crack_constraint() {
                        Constraint::TypeOf(_) => {
                            return Err(TyCheckErrors::from(TyCheckError::dummy_infer_error(
                                self.ctx.cfg.input.clone(),
                                fn_name!(),
                                line!(),
                            )));
                        }
                        _ => {}
                    }
                    Ok(Type::FreeVar(fv))
                } else {
                    let new_constraint = fv.crack_constraint().clone();
                    let new_constraint = self.deref_constraint(new_constraint)?;
                    fv.update_constraint(new_constraint, true);
                    Ok(Type::FreeVar(fv))
                }
            }
            Type::Poly { name, mut params } => {
                let typ = poly(&name, params.clone());
                let (_, ctx) = self.ctx.get_nominal_type_ctx(&typ).ok_or_else(|| {
                    TyCheckError::type_not_found(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                        &typ,
                    )
                })?;
                let variances = ctx.type_params_variance();
                for (param, variance) in params.iter_mut().zip(variances.into_iter()) {
                    self.push_variance(variance);
                    *param = self.deref_tp(mem::take(param))?;
                    self.pop_variance();
                }
                Ok(Type::Poly { name, params })
            }
            Type::Subr(mut subr) => {
                for param in subr.non_default_params.iter_mut() {
                    self.push_variance(Contravariant);
                    *param.typ_mut() = self.deref_tyvar(mem::take(param.typ_mut()))?;
                    self.pop_variance();
                }
                if let Some(var_args) = &mut subr.var_params {
                    self.push_variance(Contravariant);
                    *var_args.typ_mut() = self.deref_tyvar(mem::take(var_args.typ_mut()))?;
                    self.pop_variance();
                }
                for d_param in subr.default_params.iter_mut() {
                    self.push_variance(Contravariant);
                    *d_param.typ_mut() = self.deref_tyvar(mem::take(d_param.typ_mut()))?;
                    self.pop_variance();
                }
                self.push_variance(Covariant);
                subr.return_t = Box::new(self.deref_tyvar(mem::take(&mut subr.return_t))?);
                self.pop_variance();
                Ok(Type::Subr(subr))
            }
            Type::Callable {
                mut param_ts,
                return_t,
            } => {
                for param_t in param_ts.iter_mut() {
                    *param_t = self.deref_tyvar(mem::take(param_t))?;
                }
                let return_t = self.deref_tyvar(*return_t)?;
                Ok(callable(param_ts, return_t))
            }
            Type::Quantified(subr) => self.eliminate_needless_quant(*subr),
            Type::Ref(t) => {
                let t = self.deref_tyvar(*t)?;
                Ok(ref_(t))
            }
            Type::RefMut { before, after } => {
                let before = self.deref_tyvar(*before)?;
                let after = if let Some(after) = after {
                    Some(self.deref_tyvar(*after)?)
                } else {
                    None
                };
                Ok(ref_mut(before, after))
            }
            Type::Record(mut rec) => {
                for (_, field) in rec.iter_mut() {
                    *field = self.deref_tyvar(mem::take(field))?;
                }
                Ok(Type::Record(rec))
            }
            Type::Refinement(refine) => {
                let t = self.deref_tyvar(*refine.t)?;
                // TODO: deref_predicate
                Ok(refinement(refine.var, t, *refine.pred))
            }
            Type::And(l, r) => {
                let l = self.deref_tyvar(*l)?;
                let r = self.deref_tyvar(*r)?;
                Ok(self.ctx.intersection(&l, &r))
            }
            Type::Or(l, r) => {
                let l = self.deref_tyvar(*l)?;
                let r = self.deref_tyvar(*r)?;
                Ok(self.ctx.union(&l, &r))
            }
            Type::Not(ty) => {
                let ty = self.deref_tyvar(*ty)?;
                Ok(self.ctx.complement(&ty))
            }
            Type::Proj { lhs, rhs } => {
                let proj = self
                    .ctx
                    .eval_proj(*lhs.clone(), rhs.clone(), self.ctx.level, self.loc)
                    .or_else(|_| {
                        let lhs = self.deref_tyvar(*lhs)?;
                        self.ctx.eval_proj(lhs, rhs, self.ctx.level, self.loc)
                    })
                    .unwrap_or(Failure);
                Ok(proj)
            }
            Type::ProjCall {
                lhs,
                attr_name,
                args,
            } => {
                let lhs = self.deref_tp(*lhs)?;
                let mut new_args = vec![];
                for arg in args.into_iter() {
                    new_args.push(self.deref_tp(arg)?);
                }
                let proj = self
                    .ctx
                    .eval_proj_call(lhs, attr_name, new_args, self.ctx.level, self.loc)
                    .unwrap_or(Failure);
                Ok(proj)
            }
            Type::Structural(inner) => {
                let inner = self.deref_tyvar(*inner)?;
                Ok(inner.structuralize())
            }
            t => Ok(t),
        }
    }

    fn validate_subsup(&mut self, sub_t: Type, super_t: Type) -> TyCheckResult<Type> {
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
                let (_, ctx) = self.ctx.get_nominal_type_ctx(&typ).ok_or_else(|| {
                    TyCheckError::type_not_found(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        self.loc.loc(),
                        self.ctx.caused_by(),
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
                    self.ctx
                        .sub_unify_tp(&lp, &rp, Some(variance), self.loc, false)?;
                    let param = if variance == Covariant { lp } else { rp };
                    tps.push(param);
                }
                Ok(poly(rn, tps))
            }
            (sub_t, super_t) => self.validate_simple_subsup(sub_t, super_t),
        }
    }

    fn validate_simple_subsup(&mut self, sub_t: Type, super_t: Type) -> TyCheckResult<Type> {
        if self.ctx.is_trait(&super_t) {
            self.ctx
                .check_trait_impl(&sub_t, &super_t, self.qnames, self.loc)?;
        }
        let is_subtype = self.ctx.subtype_of(&sub_t, &super_t);
        let sub_t = if DEBUG_MODE {
            sub_t
        } else {
            self.deref_tyvar(sub_t)?
        };
        let super_t = if DEBUG_MODE {
            super_t
        } else {
            self.deref_tyvar(super_t)?
        };
        if sub_t == super_t {
            Ok(sub_t)
        }
        // REVIEW: Even if type constraints can be satisfied, implementation may not exist
        else if is_subtype {
            match self.variance {
                // ?T(<: Sup) --> Sup (Sup != Obj), because completion will not work if Never is selected.
                // ?T(:> Never, <: Obj) --> Never
                // ?T(:> Never, <: Int) --> Never..Int == Int
                Variance::Covariant if self.coerce => {
                    if sub_t != Never || super_t == Obj {
                        Ok(sub_t)
                    } else {
                        Ok(bounded(sub_t, super_t))
                    }
                }
                Variance::Contravariant if self.coerce => Ok(super_t),
                Variance::Covariant | Variance::Contravariant => Ok(bounded(sub_t, super_t)),
                Variance::Invariant => {
                    // need to check if sub_t == super_t (sub_t <: super_t is already checked)
                    if self.ctx.supertype_of(&sub_t, &super_t) {
                        Ok(sub_t)
                    } else {
                        Err(TyCheckErrors::from(TyCheckError::invariant_error(
                            self.ctx.cfg.input.clone(),
                            line!() as usize,
                            &sub_t,
                            &super_t,
                            self.loc.loc(),
                            self.ctx.caused_by(),
                        )))
                    }
                }
            }
        } else {
            Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                self.ctx.cfg.input.clone(),
                line!() as usize,
                &sub_t,
                &super_t,
                self.loc.loc(),
                self.ctx.caused_by(),
            )))
        }
    }

    // here ?T can be eliminated
    //     ?T -> Int
    //     ?T, ?U -> K(?U)
    //     Int -> ?T
    // here ?T cannot be eliminated
    //     ?T -> ?T
    //     ?T -> K(?T)
    //     ?T -> ?U(:> ?T)
    fn eliminate_needless_quant(&mut self, subr: Type) -> TyCheckResult<Type> {
        let Ok(mut subr) = SubrType::try_from(subr) else { unreachable!() };
        let essential_qnames = subr.essential_qnames();
        let mut _self = Dereferencer::new(
            self.ctx,
            self.variance,
            self.coerce,
            &essential_qnames,
            self.loc,
        );
        for param in subr.non_default_params.iter_mut() {
            _self.push_variance(Contravariant);
            *param.typ_mut() = _self.deref_tyvar(mem::take(param.typ_mut()))?;
            _self.pop_variance();
        }
        if let Some(var_args) = &mut subr.var_params {
            _self.push_variance(Contravariant);
            *var_args.typ_mut() = _self.deref_tyvar(mem::take(var_args.typ_mut()))?;
            _self.pop_variance();
        }
        for d_param in subr.default_params.iter_mut() {
            _self.push_variance(Contravariant);
            *d_param.typ_mut() = _self.deref_tyvar(mem::take(d_param.typ_mut()))?;
            _self.pop_variance();
        }
        _self.push_variance(Covariant);
        subr.return_t = Box::new(_self.deref_tyvar(mem::take(&mut subr.return_t))?);
        _self.pop_variance();
        let subr = Type::Subr(subr);
        if subr.has_qvar() {
            Ok(subr.quantify())
        } else {
            Ok(subr)
        }
    }
}

impl Context {
    pub const TOP_LEVEL: usize = 1;

    /// Quantification occurs only once in function types.
    /// Therefore, this method is called only once at the top level, and `generalize_t_inner` is called inside.
    pub(crate) fn generalize_t(&self, free_type: Type) -> Type {
        let mut generalizer = Generalizer::new(self.level);
        let maybe_unbound_t = generalizer.generalize_t(free_type, false);
        if maybe_unbound_t.is_subr() && maybe_unbound_t.has_qvar() {
            maybe_unbound_t.quantify()
        } else {
            maybe_unbound_t
        }
    }

    pub fn readable_type(&self, t: Type) -> Type {
        let qnames = set! {};
        let mut dereferencer = Dereferencer::new(self, Covariant, false, &qnames, &());
        dereferencer.deref_tyvar(t.clone()).unwrap_or(t)
    }

    pub(crate) fn coerce(&self, t: Type, t_loc: &impl Locational) -> TyCheckResult<Type> {
        let qnames = set! {};
        let mut dereferencer = Dereferencer::new(self, Covariant, true, &qnames, t_loc);
        dereferencer.deref_tyvar(t)
    }

    pub(crate) fn coerce_tp(&self, tp: TyParam, t_loc: &impl Locational) -> TyCheckResult<TyParam> {
        let qnames = set! {};
        let mut dereferencer = Dereferencer::new(self, Covariant, true, &qnames, t_loc);
        dereferencer.deref_tp(tp)
    }

    pub(crate) fn trait_impl_exists(&self, class: &Type, trait_: &Type) -> bool {
        // `Never` implements any trait
        if self.subtype_of(class, &Type::Never) {
            return true;
        }
        if class.is_monomorphic() {
            self.mono_class_trait_impl_exist(class, trait_)
        } else {
            self.poly_class_trait_impl_exists(class, trait_)
        }
    }

    fn mono_class_trait_impl_exist(&self, class: &Type, trait_: &Type) -> bool {
        let mut super_exists = false;
        for imp in self.get_trait_impls(trait_).into_iter() {
            if self.supertype_of(&imp.sub_type, class) && self.supertype_of(&imp.sup_trait, trait_)
            {
                super_exists = true;
                break;
            }
        }
        super_exists
    }

    fn poly_class_trait_impl_exists(&self, class: &Type, trait_: &Type) -> bool {
        let mut super_exists = false;
        for imp in self.get_trait_impls(trait_).into_iter() {
            self.substitute_typarams(&imp.sub_type, class).unwrap_or(());
            self.substitute_typarams(&imp.sup_trait, trait_)
                .unwrap_or(());
            if self.supertype_of(&imp.sub_type, class) && self.supertype_of(&imp.sup_trait, trait_)
            {
                super_exists = true;
                Self::undo_substitute_typarams(&imp.sub_type);
                Self::undo_substitute_typarams(&imp.sup_trait);
                break;
            }
            Self::undo_substitute_typarams(&imp.sub_type);
            Self::undo_substitute_typarams(&imp.sup_trait);
        }
        super_exists
    }

    fn check_trait_impl(
        &self,
        class: &Type,
        trait_: &Type,
        qnames: &Set<Str>,
        loc: &impl Locational,
    ) -> TyCheckResult<()> {
        if !self.trait_impl_exists(class, trait_) {
            let mut dereferencer = Dereferencer::new(self, Variance::Covariant, false, qnames, loc);
            let class = if DEBUG_MODE {
                class.clone()
            } else {
                dereferencer.deref_tyvar(class.clone())?
            };
            let trait_ = if DEBUG_MODE {
                trait_.clone()
            } else {
                dereferencer.deref_tyvar(trait_.clone())?
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
            if let Err(es) = self.resolve_expr_t(chunk, &set! {}) {
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
            let qnames = set! {};
            let mut derferencer = Dereferencer::simple(self, &qnames, name);
            if let Ok(t) = derferencer.deref_tyvar(mem::take(&mut vi.t)) {
                vi.t = t;
            }
        }
        for (name, vi) in params.iter_mut() {
            let qnames = set! {};
            let mut derferencer = Dereferencer::simple(self, &qnames, name);
            if let Ok(t) = derferencer.deref_tyvar(mem::take(&mut vi.t)) {
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

    fn resolve_params_t(&self, params: &mut hir::Params, qnames: &Set<Str>) -> TyCheckResult<()> {
        for param in params.non_defaults.iter_mut() {
            // generalization should work properly for the subroutine type, but may not work for the parameters' own types
            // HACK: so generalize them manually
            param.vi.t.generalize();
            let t = mem::take(&mut param.vi.t);
            let mut dereferencer = Dereferencer::new(self, Contravariant, false, qnames, param);
            param.vi.t = dereferencer.deref_tyvar(t)?;
        }
        if let Some(var_params) = &mut params.var_params {
            var_params.vi.t.generalize();
            let t = mem::take(&mut var_params.vi.t);
            let mut dereferencer =
                Dereferencer::new(self, Contravariant, false, qnames, var_params.as_ref());
            var_params.vi.t = dereferencer.deref_tyvar(t)?;
        }
        for param in params.defaults.iter_mut() {
            param.sig.vi.t.generalize();
            let t = mem::take(&mut param.sig.vi.t);
            let mut dereferencer = Dereferencer::new(self, Contravariant, false, qnames, param);
            param.sig.vi.t = dereferencer.deref_tyvar(t)?;
            self.resolve_expr_t(&mut param.default_val, qnames)?;
        }
        Ok(())
    }

    fn resolve_expr_t(&self, expr: &mut hir::Expr, qnames: &Set<Str>) -> TyCheckResult<()> {
        match expr {
            hir::Expr::Lit(_) => Ok(()),
            hir::Expr::Accessor(acc) => {
                if acc
                    .ref_t()
                    .unbound_name()
                    .map_or(false, |name| !qnames.contains(&name))
                {
                    let t = mem::take(acc.ref_mut_t());
                    let mut dereferencer = Dereferencer::simple(self, qnames, acc);
                    *acc.ref_mut_t() = dereferencer.deref_tyvar(t)?;
                }
                if let hir::Accessor::Attr(attr) = acc {
                    self.resolve_expr_t(&mut attr.obj, qnames)?;
                }
                Ok(())
            }
            hir::Expr::Array(array) => match array {
                hir::Array::Normal(arr) => {
                    let t = mem::take(&mut arr.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, arr);
                    arr.t = dereferencer.deref_tyvar(t)?;
                    for elem in arr.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr, qnames)?;
                    }
                    Ok(())
                }
                hir::Array::WithLength(arr) => {
                    let t = mem::take(&mut arr.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, arr);
                    arr.t = dereferencer.deref_tyvar(t)?;
                    self.resolve_expr_t(&mut arr.elem, qnames)?;
                    self.resolve_expr_t(&mut arr.len, qnames)?;
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
                    let t = mem::take(&mut tup.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, tup);
                    tup.t = dereferencer.deref_tyvar(t)?;
                    for elem in tup.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr, qnames)?;
                    }
                    Ok(())
                }
            },
            hir::Expr::Set(set) => match set {
                hir::Set::Normal(st) => {
                    let t = mem::take(&mut st.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, st);
                    st.t = dereferencer.deref_tyvar(t)?;
                    for elem in st.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr, qnames)?;
                    }
                    Ok(())
                }
                hir::Set::WithLength(st) => {
                    let t = mem::take(&mut st.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, st);
                    st.t = dereferencer.deref_tyvar(t)?;
                    self.resolve_expr_t(&mut st.elem, qnames)?;
                    self.resolve_expr_t(&mut st.len, qnames)?;
                    Ok(())
                }
            },
            hir::Expr::Dict(dict) => match dict {
                hir::Dict::Normal(dic) => {
                    let t = mem::take(&mut dic.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, dic);
                    dic.t = dereferencer.deref_tyvar(t)?;
                    for kv in dic.kvs.iter_mut() {
                        self.resolve_expr_t(&mut kv.key, qnames)?;
                        self.resolve_expr_t(&mut kv.value, qnames)?;
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
                let t = mem::take(&mut record.t);
                let mut dereferencer = Dereferencer::simple(self, qnames, record);
                record.t = dereferencer.deref_tyvar(t)?;
                for attr in record.attrs.iter_mut() {
                    let t = mem::take(attr.sig.ref_mut_t());
                    let mut dereferencer = Dereferencer::simple(self, qnames, &attr.sig);
                    let t = dereferencer.deref_tyvar(t)?;
                    *attr.sig.ref_mut_t() = t;
                    for chunk in attr.body.block.iter_mut() {
                        self.resolve_expr_t(chunk, qnames)?;
                    }
                }
                Ok(())
            }
            hir::Expr::BinOp(binop) => {
                let t = mem::take(binop.signature_mut_t().unwrap());
                let mut dereferencer = Dereferencer::simple(self, qnames, binop);
                *binop.signature_mut_t().unwrap() = dereferencer.deref_tyvar(t)?;
                self.resolve_expr_t(&mut binop.lhs, qnames)?;
                self.resolve_expr_t(&mut binop.rhs, qnames)?;
                Ok(())
            }
            hir::Expr::UnaryOp(unaryop) => {
                let t = mem::take(unaryop.signature_mut_t().unwrap());
                let mut dereferencer = Dereferencer::simple(self, qnames, unaryop);
                *unaryop.signature_mut_t().unwrap() = dereferencer.deref_tyvar(t)?;
                self.resolve_expr_t(&mut unaryop.expr, qnames)?;
                Ok(())
            }
            hir::Expr::Call(call) => {
                if let Some(t) = call.signature_mut_t() {
                    let t = mem::take(t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, call);
                    *call.signature_mut_t().unwrap() = dereferencer.deref_tyvar(t)?;
                }
                self.resolve_expr_t(&mut call.obj, qnames)?;
                for arg in call.args.pos_args.iter_mut() {
                    self.resolve_expr_t(&mut arg.expr, qnames)?;
                }
                if let Some(var_args) = &mut call.args.var_args {
                    self.resolve_expr_t(&mut var_args.expr, qnames)?;
                }
                for arg in call.args.kw_args.iter_mut() {
                    self.resolve_expr_t(&mut arg.expr, qnames)?;
                }
                Ok(())
            }
            hir::Expr::Def(def) => {
                let qnames = if let Type::Quantified(quant) = def.sig.ref_t() {
                    // double quantification is not allowed
                    let Ok(subr) = <&SubrType>::try_from(quant.as_ref()) else { unreachable!() };
                    subr.essential_qnames()
                } else {
                    qnames.clone()
                };
                let t = mem::take(def.sig.ref_mut_t());
                let mut dereferencer = Dereferencer::simple(self, &qnames, &def.sig);
                *def.sig.ref_mut_t() = dereferencer.deref_tyvar(t)?;
                if let Some(params) = def.sig.params_mut() {
                    self.resolve_params_t(params, &qnames)?;
                }
                for chunk in def.body.block.iter_mut() {
                    self.resolve_expr_t(chunk, &qnames)?;
                }
                Ok(())
            }
            hir::Expr::Lambda(lambda) => {
                let qnames = if let Type::Quantified(quant) = lambda.ref_t() {
                    let Ok(subr) = <&SubrType>::try_from(quant.as_ref()) else { unreachable!() };
                    subr.essential_qnames()
                } else {
                    qnames.clone()
                };
                let t = mem::take(&mut lambda.t);
                let mut dereferencer = Dereferencer::simple(self, &qnames, lambda);
                lambda.t = dereferencer.deref_tyvar(t)?;
                self.resolve_params_t(&mut lambda.params, &qnames)?;
                for chunk in lambda.body.iter_mut() {
                    self.resolve_expr_t(chunk, &qnames)?;
                }
                Ok(())
            }
            hir::Expr::ClassDef(class_def) => {
                for def in class_def.methods.iter_mut() {
                    self.resolve_expr_t(def, qnames)?;
                }
                Ok(())
            }
            hir::Expr::PatchDef(patch_def) => {
                for def in patch_def.methods.iter_mut() {
                    self.resolve_expr_t(def, qnames)?;
                }
                Ok(())
            }
            hir::Expr::ReDef(redef) => {
                // REVIEW: redef.attr is not dereferenced
                for chunk in redef.block.iter_mut() {
                    self.resolve_expr_t(chunk, qnames)?;
                }
                Ok(())
            }
            hir::Expr::TypeAsc(tasc) => self.resolve_expr_t(&mut tasc.expr, qnames),
            hir::Expr::Code(chunks) | hir::Expr::Compound(chunks) => {
                for chunk in chunks.iter_mut() {
                    self.resolve_expr_t(chunk, qnames)?;
                }
                Ok(())
            }
            hir::Expr::Dummy(chunks) => {
                for chunk in chunks.iter_mut() {
                    self.resolve_expr_t(chunk, qnames)?;
                }
                Ok(())
            }
            hir::Expr::Import(_) => unreachable!(),
        }
    }

    /// ```erg
    /// squash_tyvar(?1 or ?2) == ?1(== ?2)
    /// squash_tyvar(?T or ?U) == ?T or ?U
    /// squash_tyvar(?T or NoneType) == ?T or Nonetype
    /// ```
    pub(crate) fn squash_tyvar(&self, typ: Type) -> Type {
        match typ {
            Type::Or(l, r) => {
                let l = self.squash_tyvar(*l);
                let r = self.squash_tyvar(*r);
                if l.is_unnamed_unbound_var() && r.is_unnamed_unbound_var() {
                    match (self.subtype_of(&l, &r), self.subtype_of(&r, &l)) {
                        (true, true) | (true, false) => {
                            let _ = self.sub_unify(&l, &r, &(), None);
                        }
                        (false, true) => {
                            let _ = self.sub_unify(&r, &l, &(), None);
                        }
                        _ => {}
                    }
                }
                self.union(&l, &r)
            }
            other => other,
        }
    }
}
