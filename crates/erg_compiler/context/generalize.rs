use std::mem;

use erg_common::consts::DEBUG_MODE;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{dict, fn_name, get_hash, set};
#[allow(unused_imports)]
use erg_common::{fmt_vec, log};

use crate::hir::GuardClause;
use crate::module::GeneralizationResult;
use crate::ty::constructors::*;
use crate::ty::free::{CanbeFree, Constraint, Free, HasLevel};
use crate::ty::typaram::{TyParam, TyParamLambda};
use crate::ty::value::ValueObj;
use crate::ty::{HasType, Predicate, SubrType, Type};

use crate::context::{Context, Variance};
use crate::error::{TyCheckError, TyCheckErrors, TyCheckResult};
use crate::{feature_error, hir, mono_type_pattern, mono_value_pattern, unreachable_error};

use Type::*;
use Variance::*;

use super::eval::{Substituter, UndoableLinkedList};

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
            TyParam::Value(val) => TyParam::Value(
                val.map_t(&mut |t| self.generalize_t(t, uninit))
                    .map_tp(&mut |tp| self.generalize_tp(tp, uninit)),
            ),
            TyParam::FreeVar(fv) if fv.is_generalized() => TyParam::FreeVar(fv),
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let tp = fv.crack().clone();
                self.generalize_tp(tp, uninit)
            }
            // TODO: Polymorphic generalization
            TyParam::FreeVar(fv) if fv.level() > Some(self.level) => {
                let constr = self.generalize_constraint(&fv);
                fv.update_constraint(constr, true);
                fv.generalize();
                TyParam::FreeVar(fv)
            }
            TyParam::List(tps) => TyParam::List(
                tps.into_iter()
                    .map(|tp| self.generalize_tp(tp, uninit))
                    .collect(),
            ),
            TyParam::UnsizedList(tp) => {
                TyParam::UnsizedList(Box::new(self.generalize_tp(*tp, uninit)))
            }
            TyParam::Tuple(tps) => TyParam::Tuple(
                tps.into_iter()
                    .map(|tp| self.generalize_tp(tp, uninit))
                    .collect(),
            ),
            TyParam::Set(set) => TyParam::Set(
                set.into_iter()
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
            TyParam::DataClass { name, fields } => {
                let fields = fields
                    .into_iter()
                    .map(|(field, tp)| (field, self.generalize_tp(tp, uninit)))
                    .collect();
                TyParam::DataClass { name, fields }
            }
            TyParam::Lambda(lambda) => {
                let nd_params = lambda
                    .nd_params
                    .into_iter()
                    .map(|pt| pt.map_type(&mut |t| self.generalize_t(t, uninit)))
                    .collect::<Vec<_>>();
                let var_params = lambda
                    .var_params
                    .map(|pt| pt.map_type(&mut |t| self.generalize_t(t, uninit)));
                let d_params = lambda
                    .d_params
                    .into_iter()
                    .map(|pt| pt.map_type(&mut |t| self.generalize_t(t, uninit)))
                    .collect::<Vec<_>>();
                let kw_var_params = lambda
                    .kw_var_params
                    .map(|pt| pt.map_type(&mut |t| self.generalize_t(t, uninit)));
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
                    kw_var_params,
                    body,
                ))
            }
            TyParam::FreeVar(_) => free,
            TyParam::Proj { obj, attr } => {
                let obj = self.generalize_tp(*obj, uninit);
                TyParam::proj(obj, attr)
            }
            TyParam::ProjCall { obj, attr, args } => {
                let obj = self.generalize_tp(*obj, uninit);
                let args = args
                    .into_iter()
                    .map(|tp| self.generalize_tp(tp, uninit))
                    .collect();
                TyParam::proj_call(obj, attr, args)
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
            TyParam::Mono(_) | TyParam::Failure => free,
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
            FreeVar(fv) if fv.is_linked() => self.generalize_t(fv.unwrap_linked(), uninit),
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
                        res.set_level(1);
                        res.destructive_link(&t);
                        res.generalize();
                        res
                    } else if sup != Obj
                        && self.variance == Contravariant
                        && !self.qnames.contains(&fv.unbound_name().unwrap())
                    {
                        // |T <: Bool| T -> Int ==> Bool -> Int
                        self.generalize_t(sup, uninit)
                    } else if sub != Never
                        && self.variance == Covariant
                        && !self.qnames.contains(&fv.unbound_name().unwrap())
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
            FreeVar(_) => free_type,
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
                    if let Some(default) = d_param.default_typ_mut() {
                        *default = self.generalize_t(mem::take(default), uninit);
                    }
                });
                self.variance = Covariant;
                let return_t = self.generalize_t(*subr.return_t, uninit);
                self.qnames = self.qnames.difference(&qnames);
                subr_t(
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|x| *x),
                    subr.default_params,
                    subr.kw_var_params.map(|x| *x),
                    return_t,
                )
            }
            Quantified(quant) => {
                log!(err "{quant}");
                quant.quantify()
            }
            Record(rec) => {
                let fields = rec
                    .into_iter()
                    .map(|(name, t)| (name, self.generalize_t(t, uninit)))
                    .collect();
                Type::Record(fields)
            }
            NamedTuple(rec) => {
                let fields = rec
                    .into_iter()
                    .map(|(name, t)| (name, self.generalize_t(t, uninit)))
                    .collect();
                Type::NamedTuple(fields)
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
            And(ands) => {
                // not `self.intersection` because types are generalized
                let ands = ands
                    .into_iter()
                    .map(|t| self.generalize_t(t, uninit))
                    .collect();
                Type::checked_and(ands)
            }
            Or(ors) => {
                // not `self.union` because types are generalized
                let ors = ors
                    .into_iter()
                    .map(|t| self.generalize_t(t, uninit))
                    .collect();
                Type::checked_or(ors)
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
            Guard(grd) => {
                let to = self.generalize_t(*grd.to, uninit);
                guard(grd.namespace, grd.target, to)
            }
            Bounded { sub, sup } => {
                let sub = self.generalize_t(*sub, uninit);
                let sup = self.generalize_t(*sup, uninit);
                bounded(sub, sup)
            }
            Int | Nat | Float | Ratio | Complex | Bool | Str | Never | Obj | Type | Error
            | Code | Frame | NoneType | Inf | NegInf | NotImplementedType | Ellipsis
            | ClassType | TraitType | Patch | Failure | Uninited | Mono(_) => free_type,
        }
    }

    fn generalize_constraint<T: CanbeFree + Send + Clone>(&mut self, fv: &Free<T>) -> Constraint {
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
            Predicate::Const(_) | Predicate::Failure => pred,
            Predicate::Value(val) => {
                Predicate::Value(val.map_t(&mut |t| self.generalize_t(t, uninit)))
            }
            Predicate::Call {
                receiver,
                name,
                args,
            } => {
                let receiver = self.generalize_tp(receiver, uninit);
                let mut new_args = vec![];
                for arg in args.into_iter() {
                    new_args.push(self.generalize_tp(arg, uninit));
                }
                Predicate::call(receiver, name, new_args)
            }
            Predicate::Attr { receiver, name } => {
                let receiver = self.generalize_tp(receiver, uninit);
                Predicate::attr(receiver, name)
            }
            Predicate::GeneralEqual { lhs, rhs } => {
                let lhs = self.generalize_pred(*lhs, uninit);
                let rhs = self.generalize_pred(*rhs, uninit);
                Predicate::general_eq(lhs, rhs)
            }
            Predicate::GeneralGreaterEqual { lhs, rhs } => {
                let lhs = self.generalize_pred(*lhs, uninit);
                let rhs = self.generalize_pred(*rhs, uninit);
                Predicate::general_ge(lhs, rhs)
            }
            Predicate::GeneralLessEqual { lhs, rhs } => {
                let lhs = self.generalize_pred(*lhs, uninit);
                let rhs = self.generalize_pred(*rhs, uninit);
                Predicate::general_le(lhs, rhs)
            }
            Predicate::GeneralNotEqual { lhs, rhs } => {
                let lhs = self.generalize_pred(*lhs, uninit);
                let rhs = self.generalize_pred(*rhs, uninit);
                Predicate::general_ne(lhs, rhs)
            }
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
    /// This is basically the same as `ctx.level`, but can be changed
    level: usize,
    coerce: bool,
    variance_stack: Vec<Variance>,
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
            level: ctx.level,
            coerce,
            variance_stack: vec![Invariant, variance],
            qnames,
            loc,
        }
    }

    pub fn simple(ctx: &'c Context, qnames: &'q Set<Str>, loc: &'l L) -> Self {
        Self::new(ctx, Variance::Covariant, true, qnames, loc)
    }

    pub fn set_level(&mut self, level: usize) {
        self.level = level;
    }

    fn push_variance(&mut self, variance: Variance) {
        self.variance_stack.push(variance);
    }

    fn pop_variance(&mut self) {
        self.variance_stack.pop();
    }

    fn current_variance(&self) -> Variance {
        *self.variance_stack.last().unwrap()
    }

    fn deref_value(&mut self, val: ValueObj) -> TyCheckResult<ValueObj> {
        match val {
            ValueObj::Type(mut t) => {
                t.try_map_t(&mut |t| self.deref_tyvar(t.clone()))?;
                Ok(ValueObj::Type(t))
            }
            ValueObj::List(vs) => {
                let mut new_vs = vec![];
                for v in vs.iter() {
                    new_vs.push(self.deref_value(v.clone())?);
                }
                Ok(ValueObj::List(new_vs.into()))
            }
            ValueObj::Tuple(vs) => {
                let mut new_vs = vec![];
                for v in vs.iter() {
                    new_vs.push(self.deref_value(v.clone())?);
                }
                Ok(ValueObj::Tuple(new_vs.into()))
            }
            ValueObj::Dict(dic) => {
                let mut new_dic = dict! {};
                for (k, v) in dic.into_iter() {
                    let k = self.deref_value(k)?;
                    let v = self.deref_value(v)?;
                    new_dic.insert(k, v);
                }
                Ok(ValueObj::Dict(new_dic))
            }
            ValueObj::Set(set) => {
                let mut new_set = set! {};
                for v in set.into_iter() {
                    new_set.insert(self.deref_value(v)?);
                }
                Ok(ValueObj::Set(new_set))
            }
            ValueObj::Record(rec) => {
                let mut new_rec = dict! {};
                for (field, v) in rec.into_iter() {
                    new_rec.insert(field, self.deref_value(v)?);
                }
                Ok(ValueObj::Record(new_rec))
            }
            ValueObj::DataClass { name, fields } => {
                let mut new_fields = dict! {};
                for (field, v) in fields.into_iter() {
                    new_fields.insert(field, self.deref_value(v)?);
                }
                Ok(ValueObj::DataClass {
                    name,
                    fields: new_fields,
                })
            }
            ValueObj::UnsizedList(v) => Ok(ValueObj::UnsizedList(Box::new(self.deref_value(*v)?))),
            ValueObj::Subr(subr) => Ok(ValueObj::Subr(subr)),
            mono_value_pattern!() => Ok(val),
        }
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
            TyParam::FreeVar(_) if self.level == 0 => {
                let t = self.ctx.get_tp_t(&tp).unwrap_or(Type::Obj);
                Ok(TyParam::erased(self.deref_tyvar(t)?))
            }
            TyParam::FreeVar(fv) if fv.get_type().is_some() => {
                let t = self.deref_tyvar(fv.get_type().unwrap())?;
                fv.update_type(t);
                Ok(TyParam::FreeVar(fv))
            }
            TyParam::FreeVar(_) => Ok(tp),
            TyParam::Type(t) => Ok(TyParam::t(self.deref_tyvar(*t)?)),
            TyParam::Value(val) => self.deref_value(val).map(TyParam::Value),
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
            TyParam::List(tps) => {
                let mut new_tps = vec![];
                for tp in tps {
                    new_tps.push(self.deref_tp(tp)?);
                }
                Ok(TyParam::List(new_tps))
            }
            TyParam::UnsizedList(tp) => Ok(TyParam::UnsizedList(Box::new(self.deref_tp(*tp)?))),
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
                            if let Some(union) = self.ctx.union_tp(&mem::take(old_v), &v) {
                                *old_v = union;
                            }
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
            TyParam::DataClass { name, fields } => {
                let mut new_fields = dict! {};
                for (field, tp) in fields.into_iter() {
                    new_fields.insert(field, self.deref_tp(tp)?);
                }
                Ok(TyParam::DataClass {
                    name,
                    fields: new_fields,
                })
            }
            TyParam::Lambda(lambda) => {
                let nd_params = lambda
                    .nd_params
                    .into_iter()
                    .map(|pt| pt.try_map_type(&mut |t| self.deref_tyvar(t)))
                    .collect::<TyCheckResult<_>>()?;
                let var_params = lambda
                    .var_params
                    .map(|pt| pt.try_map_type(&mut |t| self.deref_tyvar(t)))
                    .transpose()?;
                let d_params = lambda
                    .d_params
                    .into_iter()
                    .map(|pt| pt.try_map_type(&mut |t| self.deref_tyvar(t)))
                    .collect::<TyCheckResult<_>>()?;
                let kw_var_params = lambda
                    .kw_var_params
                    .map(|pt| pt.try_map_type(&mut |t| self.deref_tyvar(t)))
                    .transpose()?;
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
                    kw_var_params,
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
            TyParam::ProjCall { obj, attr, args } => {
                let obj = self.deref_tp(*obj)?;
                let mut new_args = vec![];
                for arg in args.into_iter() {
                    new_args.push(self.deref_tp(arg)?);
                }
                Ok(TyParam::ProjCall {
                    obj: Box::new(obj),
                    attr,
                    args: new_args,
                })
            }
            TyParam::Failure if self.level == 0 => Err(TyCheckErrors::from(
                TyCheckError::dummy_infer_error(self.ctx.cfg.input.clone(), fn_name!(), line!()),
            )),
            TyParam::Mono(_) | TyParam::Failure => Ok(tp),
        }
    }

    fn deref_pred(&mut self, pred: Predicate) -> TyCheckResult<Predicate> {
        match pred {
            Predicate::Equal { lhs, rhs } => {
                let rhs = self.deref_tp(rhs)?;
                Ok(Predicate::eq(lhs, rhs))
            }
            Predicate::GreaterEqual { lhs, rhs } => {
                let rhs = self.deref_tp(rhs)?;
                Ok(Predicate::ge(lhs, rhs))
            }
            Predicate::LessEqual { lhs, rhs } => {
                let rhs = self.deref_tp(rhs)?;
                Ok(Predicate::le(lhs, rhs))
            }
            Predicate::NotEqual { lhs, rhs } => {
                let rhs = self.deref_tp(rhs)?;
                Ok(Predicate::ne(lhs, rhs))
            }
            Predicate::GeneralEqual { lhs, rhs } => {
                let lhs = self.deref_pred(*lhs)?;
                let rhs = self.deref_pred(*rhs)?;
                match (lhs, rhs) {
                    (Predicate::Value(lhs), Predicate::Value(rhs)) => {
                        Ok(Predicate::Value(ValueObj::Bool(lhs == rhs)))
                    }
                    (lhs, rhs) => Ok(Predicate::general_eq(lhs, rhs)),
                }
            }
            Predicate::GeneralNotEqual { lhs, rhs } => {
                let lhs = self.deref_pred(*lhs)?;
                let rhs = self.deref_pred(*rhs)?;
                match (lhs, rhs) {
                    (Predicate::Value(lhs), Predicate::Value(rhs)) => {
                        Ok(Predicate::Value(ValueObj::Bool(lhs != rhs)))
                    }
                    (lhs, rhs) => Ok(Predicate::general_ne(lhs, rhs)),
                }
            }
            Predicate::GeneralGreaterEqual { lhs, rhs } => {
                let lhs = self.deref_pred(*lhs)?;
                let rhs = self.deref_pred(*rhs)?;
                match (lhs, rhs) {
                    (Predicate::Value(lhs), Predicate::Value(rhs)) => {
                        let Some(ValueObj::Bool(res)) = lhs.try_ge(rhs) else {
                            // TODO:
                            return Err(TyCheckErrors::from(TyCheckError::dummy_infer_error(
                                self.ctx.cfg.input.clone(),
                                fn_name!(),
                                line!(),
                            )));
                        };
                        Ok(Predicate::Value(ValueObj::Bool(res)))
                    }
                    (lhs, rhs) => Ok(Predicate::general_ge(lhs, rhs)),
                }
            }
            Predicate::GeneralLessEqual { lhs, rhs } => {
                let lhs = self.deref_pred(*lhs)?;
                let rhs = self.deref_pred(*rhs)?;
                match (lhs, rhs) {
                    (Predicate::Value(lhs), Predicate::Value(rhs)) => {
                        let Some(ValueObj::Bool(res)) = lhs.try_le(rhs) else {
                            return Err(TyCheckErrors::from(TyCheckError::dummy_infer_error(
                                self.ctx.cfg.input.clone(),
                                fn_name!(),
                                line!(),
                            )));
                        };
                        Ok(Predicate::Value(ValueObj::Bool(res)))
                    }
                    (lhs, rhs) => Ok(Predicate::general_le(lhs, rhs)),
                }
            }
            Predicate::Call {
                receiver,
                name,
                args,
            } => {
                let Ok(receiver) = self.deref_tp(receiver.clone()) else {
                    return Ok(Predicate::call(receiver, name, args));
                };
                let mut new_args = vec![];
                for arg in args.into_iter() {
                    let Ok(arg) = self.deref_tp(arg) else {
                        return Ok(Predicate::call(receiver, name, new_args));
                    };
                    new_args.push(arg);
                }
                let evaled = if let Some(name) = &name {
                    self.ctx
                        .eval_proj_call(receiver.clone(), name.clone(), new_args.clone(), &())
                } else {
                    self.ctx.eval_call(receiver.clone(), new_args.clone(), &())
                };
                match evaled {
                    Ok(TyParam::Value(value)) => Ok(Predicate::Value(value)),
                    _ => Ok(Predicate::call(receiver, name, new_args)),
                }
            }
            Predicate::And(lhs, rhs) => {
                let lhs = self.deref_pred(*lhs)?;
                let rhs = self.deref_pred(*rhs)?;
                Ok(Predicate::and(lhs, rhs))
            }
            Predicate::Or(lhs, rhs) => {
                let lhs = self.deref_pred(*lhs)?;
                let rhs = self.deref_pred(*rhs)?;
                Ok(Predicate::or(lhs, rhs))
            }
            Predicate::Not(pred) => {
                let pred = self.deref_pred(*pred)?;
                Ok(!pred)
            }
            Predicate::Attr { receiver, name } => {
                let receiver = self.deref_tp(receiver)?;
                Ok(Predicate::attr(receiver, name))
            }
            Predicate::Value(v) => self.deref_value(v).map(Predicate::Value),
            Predicate::Const(_) | Predicate::Failure => Ok(pred),
        }
    }

    fn deref_constraint(&mut self, constraint: Constraint) -> TyCheckResult<Constraint> {
        match constraint {
            Constraint::Sandwiched { sub, sup } => Ok(Constraint::new_sandwiched(
                self.deref_tyvar(sub)?,
                self.deref_tyvar(sup)?,
            )),
            Constraint::TypeOf(t) => Ok(Constraint::new_type_of(self.deref_tyvar(t)?)),
            _ => unreachable_error!(TyCheckErrors, TyCheckError, self.ctx),
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
            FreeVar(fv) if fv.is_linked() => {
                let t = fv.unwrap_linked();
                self.deref_tyvar(t)
            }
            FreeVar(mut fv)
                if fv.is_generalized() && self.qnames.contains(&fv.unbound_name().unwrap()) =>
            {
                fv.update_init();
                Ok(Type::FreeVar(fv))
            }
            // ?T(:> Nat, <: Int)[n] ==> Nat (self.level <= n)
            // ?T(:> Nat, <: Sub ?U(:> {1}))[n] ==> Nat
            // ?T(<: Int, :> Add(?T)) ==> Int
            // ?T(:> Nat, <: Sub(Str)) ==> Error!
            // ?T(:> {1, "a"}, <: Eq(?T(:> {1, "a"}, ...)) ==> Error!
            FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub_t, super_t) = fv.get_subsup().unwrap();
                if self.level <= fv.level().unwrap() {
                    // we need to force linking to avoid infinite loop
                    // e.g. fv == ?T(<: Int, :> Add(?T))
                    //      fv == ?T(:> ?T.Output, <: Add(Int))
                    let list = UndoableLinkedList::new();
                    let fv_t = Type::FreeVar(fv.clone());
                    let dummy = match (sub_t.contains_type(&fv_t), super_t.contains_type(&fv_t)) {
                        // REVIEW: to prevent infinite recursion, but this may cause a nonsense error
                        (true, true) => {
                            fv.dummy_link();
                            true
                        }
                        (true, false) => {
                            fv_t.undoable_link(&super_t, &list);
                            false
                        }
                        (false, true | false) => {
                            fv_t.undoable_link(&sub_t, &list);
                            false
                        }
                    };
                    let res = self.validate_subsup(sub_t, super_t, &fv);
                    if dummy {
                        fv.undo();
                    } else {
                        drop(list);
                    }
                    match res {
                        Ok(ty) => {
                            // TODO: T(:> Nat <: Int) -> T(:> Nat, <: Int) ==> Int -> Nat
                            // Type::FreeVar(fv).destructive_link(&ty);
                            Ok(ty)
                        }
                        Err(errs) => {
                            if !fv.is_generalized() {
                                Type::FreeVar(fv).destructive_link(&Never);
                            }
                            Err(errs)
                        }
                    }
                } else {
                    // no dereference at this point
                    Ok(Type::FreeVar(fv))
                }
            }
            FreeVar(fv) if fv.get_type().is_some() => {
                let ty = fv.get_type().unwrap();
                if self.level <= fv.level().unwrap() {
                    // T: {Int, Str} => Int or Str
                    if let Some(tys) = ty.refinement_values() {
                        let mut union = Never;
                        for tp in tys {
                            if let Ok(ty) = self.ctx.convert_tp_into_type(tp.clone()) {
                                union = self.ctx.union(&union, &ty);
                            }
                        }
                        return Ok(union);
                    }
                    Ok(Type::FreeVar(fv))
                } else {
                    Ok(Type::FreeVar(fv))
                }
            }
            FreeVar(fv) if fv.is_unbound() => {
                if self.level == 0 {
                    match &*fv.crack_constraint() {
                        Constraint::TypeOf(t) if !t.is_type() => {
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
            FreeVar(_) => Ok(t),
            Poly { name, mut params } => {
                let typ = poly(&name, params.clone());
                let ctx = self.ctx.get_nominal_type_ctx(&typ).ok_or_else(|| {
                    TyCheckError::type_not_found(
                        self.ctx.cfg.input.clone(),
                        line!() as usize,
                        self.loc.loc(),
                        self.ctx.caused_by(),
                        &typ,
                    )
                })?;
                let mut errs = TyCheckErrors::empty();
                let variances = ctx.type_params_variance();
                for (param, variance) in params
                    .iter_mut()
                    .zip(variances.into_iter().chain(std::iter::repeat(Invariant)))
                {
                    self.push_variance(variance);
                    match self.deref_tp(mem::take(param)) {
                        Ok(t) => *param = t,
                        Err(es) => errs.extend(es),
                    }
                    self.pop_variance();
                }
                if errs.is_empty() {
                    Ok(Type::Poly { name, params })
                } else {
                    Err(errs)
                }
            }
            Subr(mut subr) => {
                let mut errs = TyCheckErrors::empty();
                for param in subr.non_default_params.iter_mut() {
                    self.push_variance(Contravariant);
                    match self.deref_tyvar(mem::take(param.typ_mut())) {
                        Ok(t) => *param.typ_mut() = t,
                        Err(es) => errs.extend(es),
                    }
                    self.pop_variance();
                }
                if let Some(var_args) = &mut subr.var_params {
                    self.push_variance(Contravariant);
                    match self.deref_tyvar(mem::take(var_args.typ_mut())) {
                        Ok(t) => *var_args.typ_mut() = t,
                        Err(es) => errs.extend(es),
                    }
                    self.pop_variance();
                }
                for d_param in subr.default_params.iter_mut() {
                    self.push_variance(Contravariant);
                    match self.deref_tyvar(mem::take(d_param.typ_mut())) {
                        Ok(t) => *d_param.typ_mut() = t,
                        Err(es) => errs.extend(es),
                    }
                    if let Some(default) = d_param.default_typ_mut() {
                        match self.deref_tyvar(mem::take(default)) {
                            Ok(t) => *default = t,
                            Err(es) => errs.extend(es),
                        }
                    }
                    self.pop_variance();
                }
                self.push_variance(Covariant);
                match self.deref_tyvar(mem::take(&mut subr.return_t)) {
                    Ok(t) => *subr.return_t = t,
                    Err(es) => errs.extend(es),
                }
                self.pop_variance();
                if errs.is_empty() {
                    Ok(Type::Subr(subr))
                } else {
                    Err(errs)
                }
            }
            Callable {
                mut param_ts,
                return_t,
            } => {
                for param_t in param_ts.iter_mut() {
                    *param_t = self.deref_tyvar(mem::take(param_t))?;
                }
                let return_t = self.deref_tyvar(*return_t)?;
                Ok(callable(param_ts, return_t))
            }
            Quantified(subr) => self.eliminate_needless_quant(*subr),
            Ref(t) => {
                let t = self.deref_tyvar(*t)?;
                Ok(ref_(t))
            }
            RefMut { before, after } => {
                let before = self.deref_tyvar(*before)?;
                let after = if let Some(after) = after {
                    Some(self.deref_tyvar(*after)?)
                } else {
                    None
                };
                Ok(ref_mut(before, after))
            }
            Record(mut rec) => {
                for (_, field) in rec.iter_mut() {
                    *field = self.deref_tyvar(mem::take(field))?;
                }
                Ok(Type::Record(rec))
            }
            NamedTuple(mut rec) => {
                for (_, t) in rec.iter_mut() {
                    *t = self.deref_tyvar(mem::take(t))?;
                }
                Ok(Type::NamedTuple(rec))
            }
            Refinement(refine) => {
                let t = self.deref_tyvar(*refine.t)?;
                let pred = self.deref_pred(*refine.pred)?;
                Ok(refinement(refine.var, t, pred))
            }
            And(ands) => {
                let mut new_ands = vec![];
                for t in ands.into_iter() {
                    new_ands.push(self.deref_tyvar(t)?);
                }
                Ok(new_ands
                    .into_iter()
                    .fold(Type::Obj, |acc, t| self.ctx.intersection(&acc, &t)))
            }
            Or(ors) => {
                let mut new_ors = vec![];
                for t in ors.into_iter() {
                    new_ors.push(self.deref_tyvar(t)?);
                }
                Ok(new_ors
                    .into_iter()
                    .fold(Type::Never, |acc, t| self.ctx.union(&acc, &t)))
            }
            Not(ty) => {
                let ty = self.deref_tyvar(*ty)?;
                Ok(self.ctx.complement(&ty))
            }
            Proj { lhs, rhs } => {
                let proj = self
                    .ctx
                    .eval_proj(*lhs.clone(), rhs.clone(), self.level, self.loc)
                    .or_else(|_| {
                        let lhs = self.deref_tyvar(*lhs)?;
                        self.ctx.eval_proj(lhs, rhs, self.level, self.loc)
                    })
                    .unwrap_or(Failure);
                Ok(proj)
            }
            ProjCall {
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
                    .eval_proj_call_t(lhs, attr_name, new_args, self.level, self.loc)
                    .unwrap_or(Failure);
                Ok(proj)
            }
            Structural(inner) => {
                let inner = self.deref_tyvar(*inner)?;
                Ok(inner.structuralize())
            }
            Guard(grd) => {
                let to = self.deref_tyvar(*grd.to)?;
                Ok(guard(grd.namespace, grd.target, to))
            }
            Bounded { sub, sup } => {
                let sub = self.deref_tyvar(*sub)?;
                let sup = self.deref_tyvar(*sup)?;
                Ok(bounded(sub, sup))
            }
            mono_type_pattern!() => Ok(t),
        }
    }

    fn validate_subsup(
        &mut self,
        sub_t: Type,
        super_t: Type,
        fv: &Free<Type>,
    ) -> TyCheckResult<Type> {
        // TODO: Subr, ...
        match (sub_t, super_t) {
            /*(sub_t @ Type::Refinement(_), super_t @ Type::Refinement(_)) => {
                self.validate_simple_subsup(sub_t, super_t)
            }
            (Type::Refinement(refine), super_t) => {
                self.validate_simple_subsup(*refine.t, super_t)
            }*/
            // See tests\should_err\subtyping.er:8~13
            (
                Poly {
                    name: ln,
                    params: lps,
                },
                Poly {
                    name: rn,
                    params: rps,
                },
            ) if ln == rn => {
                let typ = poly(ln, lps.clone());
                let ctx = self.ctx.get_nominal_type_ctx(&typ).ok_or_else(|| {
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
                    .zip(variances.into_iter().chain(std::iter::repeat(Invariant)))
                {
                    self.ctx
                        .sub_unify_tp(&lp, &rp, Some(variance), self.loc, false)?;
                    let param = if variance == Covariant { lp } else { rp };
                    tps.push(param);
                }
                Ok(poly(rn, tps))
            }
            (sub_t, super_t) => self.validate_simple_subsup(sub_t, super_t, fv),
        }
    }

    fn validate_simple_subsup(
        &mut self,
        sub_t: Type,
        super_t: Type,
        fv: &Free<Type>,
    ) -> TyCheckResult<Type> {
        let opt_res = self.ctx.shared().gen_cache.get(fv);
        if opt_res.is_none() && self.ctx.is_class(&sub_t) && self.ctx.is_trait(&super_t) {
            self.ctx
                .check_trait_impl(&sub_t, &super_t, self.qnames, self.loc)?;
        }
        let is_subtype = opt_res.map(|res| res.is_subtype).unwrap_or_else(|| {
            let is_subtype = self.ctx.subtype_of(&sub_t, &super_t); // PERF NOTE: bottleneck
            let res = GeneralizationResult {
                is_subtype,
                impl_trait: true,
            };
            self.ctx.shared().gen_cache.insert(fv.clone(), res);
            is_subtype
        });
        let sub_t = self.deref_tyvar(sub_t)?;
        let super_t = self.deref_tyvar(super_t)?;
        if sub_t == super_t {
            Ok(sub_t)
        } else if is_subtype {
            match self.current_variance() {
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
        let Ok(mut subr) = SubrType::try_from(subr) else {
            unreachable!()
        };
        let essential_qnames = subr.essential_qnames();
        let mut _self = Dereferencer::new(
            self.ctx,
            self.current_variance(),
            self.coerce,
            &essential_qnames,
            self.loc,
        );
        for param in subr.non_default_params.iter_mut() {
            _self.push_variance(Contravariant);
            *param.typ_mut() = _self
                .deref_tyvar(mem::take(param.typ_mut()))
                .inspect_err(|_e| _self.pop_variance())?;
            _self.pop_variance();
        }
        if let Some(var_args) = &mut subr.var_params {
            _self.push_variance(Contravariant);
            *var_args.typ_mut() = _self
                .deref_tyvar(mem::take(var_args.typ_mut()))
                .inspect_err(|_e| _self.pop_variance())?;
            _self.pop_variance();
        }
        for d_param in subr.default_params.iter_mut() {
            _self.push_variance(Contravariant);
            *d_param.typ_mut() = _self
                .deref_tyvar(mem::take(d_param.typ_mut()))
                .inspect_err(|_e| {
                    _self.pop_variance();
                })?;
            if let Some(default) = d_param.default_typ_mut() {
                *default = _self
                    .deref_tyvar(mem::take(default))
                    .inspect_err(|_e| _self.pop_variance())?;
            }
            _self.pop_variance();
        }
        _self.push_variance(Covariant);
        *subr.return_t = _self
            .deref_tyvar(mem::take(&mut subr.return_t))
            .inspect_err(|_e| _self.pop_variance())?;
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
        dereferencer.set_level(0);
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

    /// Check if a trait implementation exists for a polymorphic class.
    /// This is needed because the trait implementation spec can contain projection types.
    /// e.g. `Tuple(Ts) <: Container(Ts.union())`
    fn poly_class_trait_impl_exists(&self, class: &Type, trait_: &Type) -> bool {
        let class_hash = get_hash(&class);
        let trait_hash = get_hash(&trait_);
        for imp in self.get_trait_impls(trait_).into_iter() {
            let _sub_subs = Substituter::substitute_typarams(self, &imp.sub_type, class).ok();
            let _sup_subs = Substituter::substitute_typarams(self, &imp.sup_trait, trait_).ok();
            if self.supertype_of(&imp.sub_type, class) && self.supertype_of(&imp.sup_trait, trait_)
            {
                if class_hash != get_hash(&class) {
                    class.undo();
                }
                if trait_hash != get_hash(&trait_) {
                    trait_.undo();
                }
                return true;
            }
            if class_hash != get_hash(&class) {
                class.undo();
            }
            if trait_hash != get_hash(&trait_) {
                trait_.undo();
            }
        }
        false
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
        for methods in methods_list.iter_mut() {
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
        if let Some(kw_var) = &mut params.kw_var_params {
            kw_var.vi.t.generalize();
            let t = mem::take(&mut kw_var.vi.t);
            let mut dereferencer =
                Dereferencer::new(self, Contravariant, false, qnames, kw_var.as_ref());
            kw_var.vi.t = dereferencer.deref_tyvar(t)?;
        }
        for guard in params.guards.iter_mut() {
            match guard {
                GuardClause::Bind(def) => {
                    self.resolve_def_t(def, qnames)?;
                }
                GuardClause::Condition(cond) => {
                    self.resolve_expr_t(cond, qnames)?;
                }
            }
        }
        Ok(())
    }

    /// Resolution should start at a deeper level.
    /// For example, if it is a lambda function, the body should be checked before the signature.
    /// However, a binop call error, etc., is more important then binop operands.
    fn resolve_expr_t(&self, expr: &mut hir::Expr, qnames: &Set<Str>) -> TyCheckResult<()> {
        match expr {
            hir::Expr::Literal(_) => Ok(()),
            hir::Expr::Accessor(acc) => {
                let t = mem::take(acc.ref_mut_t().unwrap());
                let mut dereferencer = Dereferencer::simple(self, qnames, acc);
                *acc.ref_mut_t().unwrap() = dereferencer.deref_tyvar(t)?;
                if let hir::Accessor::Attr(attr) = acc {
                    self.resolve_expr_t(&mut attr.obj, qnames)?;
                }
                Ok(())
            }
            hir::Expr::List(list) => match list {
                hir::List::Normal(lis) => {
                    for elem in lis.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr, qnames)?;
                    }
                    let t = mem::take(&mut lis.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, lis);
                    lis.t = dereferencer.deref_tyvar(t)?;
                    Ok(())
                }
                hir::List::WithLength(lis) => {
                    self.resolve_expr_t(&mut lis.elem, qnames)?;
                    if let Some(len) = &mut lis.len {
                        self.resolve_expr_t(len, qnames)?;
                    }
                    let t = mem::take(&mut lis.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, lis);
                    lis.t = dereferencer.deref_tyvar(t)?;
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
                    for elem in tup.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr, qnames)?;
                    }
                    let t = mem::take(&mut tup.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, tup);
                    tup.t = dereferencer.deref_tyvar(t)?;
                    Ok(())
                }
            },
            hir::Expr::Set(set) => match set {
                hir::Set::Normal(st) => {
                    for elem in st.elems.pos_args.iter_mut() {
                        self.resolve_expr_t(&mut elem.expr, qnames)?;
                    }
                    let t = mem::take(&mut st.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, st);
                    st.t = dereferencer.deref_tyvar(t)?;
                    Ok(())
                }
                hir::Set::WithLength(st) => {
                    self.resolve_expr_t(&mut st.elem, qnames)?;
                    self.resolve_expr_t(&mut st.len, qnames)?;
                    let t = mem::take(&mut st.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, st);
                    st.t = dereferencer.deref_tyvar(t)?;
                    Ok(())
                }
            },
            hir::Expr::Dict(dict) => match dict {
                hir::Dict::Normal(dic) => {
                    for kv in dic.kvs.iter_mut() {
                        self.resolve_expr_t(&mut kv.key, qnames)?;
                        self.resolve_expr_t(&mut kv.value, qnames)?;
                    }
                    let t = mem::take(&mut dic.t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, dic);
                    dic.t = dereferencer.deref_tyvar(t)?;
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
                for attr in record.attrs.iter_mut() {
                    let t = mem::take(attr.sig.ref_mut_t().unwrap());
                    let mut dereferencer = Dereferencer::simple(self, qnames, &attr.sig);
                    let t = dereferencer.deref_tyvar(t)?;
                    *attr.sig.ref_mut_t().unwrap() = t;
                    for chunk in attr.body.block.iter_mut() {
                        self.resolve_expr_t(chunk, qnames)?;
                    }
                }
                let t = mem::take(&mut record.t);
                let mut dereferencer = Dereferencer::simple(self, qnames, record);
                record.t = dereferencer.deref_tyvar(t)?;
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
                for arg in call.args.pos_args.iter_mut() {
                    self.resolve_expr_t(&mut arg.expr, qnames)?;
                }
                if let Some(var_args) = &mut call.args.var_args {
                    self.resolve_expr_t(&mut var_args.expr, qnames)?;
                }
                for arg in call.args.kw_args.iter_mut() {
                    self.resolve_expr_t(&mut arg.expr, qnames)?;
                }
                if let Some(kw_var) = &mut call.args.kw_var {
                    self.resolve_expr_t(&mut kw_var.expr, qnames)?;
                }
                self.resolve_expr_t(&mut call.obj, qnames)?;
                if let Some(t) = call.signature_mut_t() {
                    let t = mem::take(t);
                    let mut dereferencer = Dereferencer::simple(self, qnames, call);
                    *call.signature_mut_t().unwrap() = dereferencer.deref_tyvar(t)?;
                }
                Ok(())
            }
            hir::Expr::Def(def) => self.resolve_def_t(def, qnames),
            hir::Expr::Lambda(lambda) => {
                let qnames = if let Type::Quantified(quant) = lambda.ref_t() {
                    let Ok(subr) = <&SubrType>::try_from(quant.as_ref()) else {
                        unreachable!()
                    };
                    subr.essential_qnames()
                } else {
                    qnames.clone()
                };
                let mut errs = TyCheckErrors::empty();
                for chunk in lambda.body.iter_mut() {
                    if let Err(es) = self.resolve_expr_t(chunk, &qnames) {
                        errs.extend(es);
                    }
                }
                if let Err(es) = self.resolve_params_t(&mut lambda.params, &qnames) {
                    errs.extend(es);
                }
                let t = mem::take(&mut lambda.t);
                let mut dereferencer = Dereferencer::simple(self, &qnames, lambda);
                match dereferencer.deref_tyvar(t) {
                    Ok(t) => lambda.t = t,
                    Err(es) => errs.extend(es),
                }
                if !errs.is_empty() {
                    Err(errs)
                } else {
                    Ok(())
                }
            }
            hir::Expr::ClassDef(class_def) => {
                for def in class_def.all_methods_mut() {
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
            hir::Expr::Import(_) => unreachable_error!(TyCheckErrors, TyCheckError, self),
        }
    }

    fn resolve_def_t(&self, def: &mut hir::Def, qnames: &Set<Str>) -> TyCheckResult<()> {
        let qnames = if let Type::Quantified(quant) = def.sig.ref_t() {
            // double quantification is not allowed
            let Ok(subr) = <&SubrType>::try_from(quant.as_ref()) else {
                unreachable!()
            };
            subr.essential_qnames()
        } else {
            qnames.clone()
        };
        let t = mem::take(def.sig.ref_mut_t().unwrap());
        let mut dereferencer = Dereferencer::simple(self, &qnames, &def.sig);
        *def.sig.ref_mut_t().unwrap() = dereferencer.deref_tyvar(t)?;
        if let Some(params) = def.sig.params_mut() {
            self.resolve_params_t(params, &qnames)?;
        }
        for chunk in def.body.block.iter_mut() {
            self.resolve_expr_t(chunk, &qnames)?;
        }
        Ok(())
    }

    /// ```erg
    /// squash_tyvar(?1 or ?2) == ?1(== ?2)
    /// squash_tyvar(?T or ?U) == ?T or ?U
    /// squash_tyvar(?T or NoneType) == ?T or Nonetype
    /// ```
    pub(crate) fn squash_tyvar(&self, typ: Type) -> Type {
        match typ {
            Or(tys) => {
                let new_tys = tys
                    .into_iter()
                    .map(|t| self.squash_tyvar(t))
                    .collect::<Vec<_>>();
                let mut union = Never;
                // REVIEW:
                if new_tys.iter().all(|t| t.is_unnamed_unbound_var()) {
                    for ty in new_tys.iter() {
                        if union == Never {
                            union = ty.clone();
                            continue;
                        }
                        match (self.subtype_of(&union, ty), self.subtype_of(&union, ty)) {
                            (true, true) | (true, false) => {
                                let _ = self.sub_unify(&union, ty, &(), None);
                            }
                            (false, true) => {
                                let _ = self.sub_unify(ty, &union, &(), None);
                            }
                            _ => {}
                        }
                    }
                }
                new_tys
                    .into_iter()
                    .fold(Never, |acc, t| self.union(&acc, &t))
            }
            FreeVar(ref fv) if fv.constraint_is_sandwiched() => {
                let (sub_t, super_t) = fv.get_subsup().unwrap();
                let sub_t = self.squash_tyvar(sub_t);
                let super_t = self.squash_tyvar(super_t);
                typ.update_tyvar(sub_t, super_t, None, false);
                typ
            }
            other => other,
        }
    }
}
