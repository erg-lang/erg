use std::fmt;
use std::mem;
use std::option::Option; // conflicting to Type::Option

use erg_common::consts::DEBUG_MODE;
use erg_common::dict::Dict;
use erg_common::enum_unwrap;
#[allow(unused)]
use erg_common::log;
use erg_common::set::Set;
use erg_common::traits::Locational;
use erg_common::Str;
use erg_parser::ast::VarName;

use crate::ty::constructors::*;
use crate::ty::free::{Constraint, FreeTyParam, FreeTyVar, HasLevel, GENERIC_LEVEL};
use crate::ty::typaram::{TyParam, TyParamLambda};
use crate::ty::ConstSubr;
use crate::ty::ValueObj;
use crate::ty::{HasType, Predicate, Type};
use crate::{type_feature_error, unreachable_error};
use Type::*;

use crate::context::{Context, VarInfo};
use crate::error::{TyCheckError, TyCheckErrors, TyCheckResult};
use crate::hir;

/// Context for instantiating a quantified type
/// For example, cloning each type variable of quantified type `?T -> ?T` would result in `?1 -> ?2`.
/// To avoid this, an environment to store type variables is needed, which is `TyVarCache`.
/// 量化型をインスタンス化するための文脈
/// e.g. Array -> [("T": ?T(: Type)), ("N": ?N(: Nat))]
/// FIXME: current implementation is wrong
/// It will not work unless the type variable is used with the same name as the definition.
#[derive(Debug, Clone)]
pub struct TyVarCache {
    _level: usize,
    pub(crate) already_appeared: Set<Str>,
    pub(crate) tyvar_instances: Dict<VarName, Type>,
    pub(crate) typaram_instances: Dict<VarName, TyParam>,
    pub(crate) var_infos: Dict<VarName, VarInfo>,
    pub(crate) structural_inner: bool,
}

impl fmt::Display for TyVarCache {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "TyVarContext {{ tyvar_instances: {}, typaram_instances: {}, var_infos: {} }}",
            self.tyvar_instances, self.typaram_instances, self.var_infos
        )
    }
}

impl TyVarCache {
    pub fn new(level: usize, _ctx: &Context) -> Self {
        Self {
            _level: level,
            already_appeared: Set::new(),
            tyvar_instances: Dict::new(),
            typaram_instances: Dict::new(),
            var_infos: Dict::new(),
            structural_inner: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tyvar_instances.is_empty() && self.typaram_instances.is_empty()
    }

    pub fn merge(&mut self, outer: &Self) {
        for (name, ty) in outer.tyvar_instances.iter() {
            if self.tyvar_instances.contains_key(name) {
                continue;
            } else {
                self.tyvar_instances.insert(name.clone(), ty.clone());
            }
        }
        for (name, ty) in outer.typaram_instances.iter() {
            if self.typaram_instances.contains_key(name) {
                continue;
            } else {
                self.typaram_instances.insert(name.clone(), ty.clone());
            }
        }
    }

    pub fn purge(&mut self, other: &Self) {
        for name in other.tyvar_instances.keys() {
            self.tyvar_instances.remove(name);
        }
        for name in other.typaram_instances.keys() {
            self.typaram_instances.remove(name);
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.tyvar_instances.remove(name);
        self.typaram_instances.remove(name);
        self.var_infos.remove(name);
    }

    /// Warn when a type does not need to be a type variable, such as `|T| T -> Int` (it should be `Obj -> Int`).
    ///
    /// TODO: This warning is currently disabled because it raises a false warning in cases like `|T|(x: T) -> (y: T) -> (x, y)`.
    pub fn warn_isolated_vars(&self, ctx: &Context) {
        for (name, vi) in self.var_infos.iter() {
            let refs = &ctx.index().get_refs(&vi.def_loc).unwrap().referrers;
            if refs.len() == 1 {
                let warn = TyCheckError::unnecessary_tyvar_warning(
                    ctx.cfg.input.clone(),
                    line!() as usize,
                    vi.def_loc.loc,
                    name.inspect(),
                    ctx.caused_by(),
                );
                ctx.shared().warns.push(warn);
            }
        }
    }

    fn instantiate_constraint(
        &mut self,
        constr: Constraint,
        ctx: &Context,
        loc: &impl Locational,
    ) -> TyCheckResult<Constraint> {
        match constr {
            Constraint::Sandwiched { sub, sup } => Ok(Constraint::new_sandwiched(
                ctx.instantiate_t_inner(sub, self, loc)?,
                ctx.instantiate_t_inner(sup, self, loc)?,
            )),
            Constraint::TypeOf(t) => Ok(Constraint::new_type_of(
                ctx.instantiate_t_inner(t, self, loc)?,
            )),
            Constraint::Uninited => Ok(Constraint::Uninited),
        }
    }

    /// Some of the quantified types are circulating.
    /// e.g.
    /// ```erg
    /// add: |T <: Add(T(<: Add(T(<: ...))))|(T, T) -> T.Output
    /// ```
    /// `T` in `Add` should be instantiated as `Constraint::Uninited`.
    /// And with the outer `T`, the Compiler will overwrite the inner `T`'s constraint.
    /// ```erg
    /// T <: Add(?T(: Uninited))
    /// ↓
    /// ?T <: Add(?T(<: Add(?T(<: ...))))
    /// ```
    /// After the instantiation:
    /// ```erg
    /// add: (?T(<: Add(?T)), ?T(<: ...)) -> ?T(<: ...).Output
    /// ```
    /// Therefore, it is necessary to register the type variables that appear inside.
    pub(crate) fn push_appeared(&mut self, name: Str) {
        self.already_appeared.insert(name);
    }

    pub(crate) fn push_or_init_tyvar(
        &mut self,
        name: &VarName,
        tv: &Type,
        ctx: &Context,
    ) -> TyCheckResult<()> {
        if name.inspect() == "_" {
            return Ok(());
        }
        if let Some(inst) = self.tyvar_instances.get(name) {
            self.update_tyvar(inst, tv, ctx);
        } else if let Some(inst) = self.typaram_instances.get(name) {
            if let Ok(inst) = <&Type>::try_from(inst) {
                self.update_tyvar(inst, tv, ctx);
            } else if let TyParam::FreeVar(_fv) = inst {
                inst.destructive_link(&TyParam::t(tv.clone()));
            } else {
                unreachable!()
            }
        } else {
            self.tyvar_instances.insert(name.clone(), tv.clone());
            let vi = VarInfo::type_var(Type::Type, ctx.absolutize(name.loc()), ctx.name.clone());
            ctx.index().register(name.inspect().clone(), &vi);
            self.var_infos.insert(name.clone(), vi);
        }
        Ok(())
    }

    pub(crate) fn dummy_push_or_init_tyvar(&mut self, name: &VarName, tv: &Type, ctx: &Context) {
        if name.inspect() == "_" {
            return;
        }
        if let Some(inst) = self.tyvar_instances.get(name) {
            self.update_tyvar(inst, tv, ctx);
        } else if let Some(inst) = self.typaram_instances.get(name) {
            if let Ok(inst) = <&Type>::try_from(inst) {
                self.update_tyvar(inst, tv, ctx);
            } else if let TyParam::FreeVar(_fv) = inst {
                inst.destructive_link(&TyParam::t(tv.clone()));
            } else {
                unreachable!()
            }
        } else {
            self.tyvar_instances.insert(name.clone(), tv.clone());
        }
    }

    fn update_tyvar(&self, inst: &Type, tv: &Type, ctx: &Context) {
        // T<tv> <: Eq(T<inst>)
        // T<inst> is uninitialized
        // T<inst>.link(T<tv>);
        // T <: Eq(T <: Eq(T <: ...))
        let Ok(free_inst) = <&FreeTyVar>::try_from(inst) else {
            if DEBUG_MODE {
                todo!("{inst}");
            }
            return;
        };
        if free_inst.constraint_is_uninited() {
            inst.destructive_link(tv);
        } else {
            // inst: ?T(<: Int) => old_sub: Never, old_sup: Int
            // tv: ?T(:> Nat) => new_sub: Nat, new_sup: Obj
            // => ?T(:> Nat, <: Int)
            // inst: ?T(:> Str)
            // tv: ?T(:> Nat)
            // => ?T(:> Nat or Str)
            let (old_sub, old_sup) = free_inst.get_subsup().unwrap();
            let Ok(tv) = <&FreeTyVar>::try_from(tv) else {
                if DEBUG_MODE {
                    todo!("{tv}");
                }
                return;
            };
            let (new_sub, new_sup) = tv.get_subsup().unwrap();
            let new_constraint = Constraint::new_sandwiched(
                ctx.union(&old_sub, &new_sub),
                ctx.intersection(&old_sup, &new_sup),
            );
            free_inst.update_constraint(new_constraint, true);
        }
    }

    pub(crate) fn push_or_init_typaram(
        &mut self,
        name: &VarName,
        tp: &TyParam,
        ctx: &Context,
    ) -> TyCheckResult<()> {
        if name.inspect() == "_" {
            return Ok(());
        }
        if ctx.rec_get_const_obj(name.inspect()).is_some() {
            return Err(TyCheckError::reassign_error(
                ctx.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                ctx.caused_by(),
                name.inspect(),
            )
            .into());
        }
        // FIXME:
        if let Some(inst) = self.typaram_instances.get(name) {
            self.update_typaram(inst, tp, ctx);
        } else if let Some(inst) = self.tyvar_instances.get(name) {
            if let Ok(tv) = <&Type>::try_from(tp) {
                self.update_tyvar(inst, tv, ctx);
            } else {
                unreachable!("{name} / {inst} / {tp}");
            }
        } else {
            let t = ctx.get_tp_t(tp).unwrap_or(Type::Obj);
            let vi = VarInfo::type_var(t, ctx.absolutize(name.loc()), ctx.name.clone());
            ctx.index().register(name.inspect().clone(), &vi);
            self.var_infos.insert(name.clone(), vi);
            self.typaram_instances.insert(name.clone(), tp.clone());
        }
        Ok(())
    }

    pub(crate) fn push_refine_var(&mut self, name: &VarName, t: Type, ctx: &Context) {
        if name.inspect() == "_" {
            return;
        }
        let vi = VarInfo::type_var(t, ctx.absolutize(name.loc()), ctx.name.clone());
        ctx.index().register(name.inspect().clone(), &vi);
        self.var_infos.insert(name.clone(), vi);
        let tp = TyParam::mono(name.inspect());
        self.typaram_instances.insert(name.clone(), tp);
    }

    pub(crate) fn dummy_push_or_init_typaram(
        &mut self,
        name: &VarName,
        tp: &TyParam,
        ctx: &Context,
    ) {
        if name.inspect() == "_" {
            return;
        }
        // FIXME:
        if let Some(inst) = self.typaram_instances.get(name) {
            self.update_typaram(inst, tp, ctx);
        } else if let Some(inst) = self.tyvar_instances.get(name) {
            if let Ok(tv) = <&Type>::try_from(tp) {
                self.update_tyvar(inst, tv, ctx);
            } else {
                unreachable!()
            }
        } else {
            self.typaram_instances.insert(name.clone(), tp.clone());
        }
    }

    fn update_typaram(&self, inst: &TyParam, tp: &TyParam, ctx: &Context) {
        if inst.level() == Some(GENERIC_LEVEL) {
            log!(err "{inst} is fixed");
            return;
        }
        let Ok(free_inst) = <&FreeTyParam>::try_from(inst) else {
            if let (Ok(inst), Ok(t)) = (<&Type>::try_from(inst), <&Type>::try_from(tp)) {
                return self.update_tyvar(inst, t, ctx);
            } else {
                if DEBUG_MODE {
                    todo!("{inst}");
                }
                return;
            }
        };
        if free_inst.constraint_is_uninited() {
            inst.destructive_link(tp);
        } else {
            let old_type = free_inst.get_type().unwrap();
            let Ok(tv) = <&FreeTyParam>::try_from(tp) else {
                if DEBUG_MODE {
                    todo!("{tp}");
                }
                return;
            };
            let new_type = tv.get_type().unwrap();
            let new_constraint = Constraint::new_type_of(ctx.intersection(&old_type, &new_type));
            free_inst.update_constraint(new_constraint, true);
        }
    }

    pub(crate) fn appeared(&self, name: &Str) -> bool {
        self.already_appeared.contains(name)
    }

    pub(crate) fn get_tyvar(&self, name: &str) -> Option<&Type> {
        self.tyvar_instances.get(name).or_else(|| {
            self.typaram_instances.get(name).and_then(|tp| {
                <&Type>::try_from(tp)
                    .map_err(|_| {
                        log!(err "cannot convert {tp} into a type");
                    })
                    .ok()
            })
        })
    }

    pub(crate) fn get_typaram(&self, name: &str) -> Option<&TyParam> {
        self.typaram_instances.get(name)
    }
}

impl Context {
    fn instantiate_tp(
        &self,
        quantified: TyParam,
        tmp_tv_cache: &mut TyVarCache,
        loc: &impl Locational,
    ) -> TyCheckResult<TyParam> {
        match quantified {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let tp = fv.crack().clone();
                self.instantiate_tp(tp, tmp_tv_cache, loc)
            }
            TyParam::FreeVar(fv) if fv.is_generalized() => {
                let (name, constr) = (fv.unbound_name().unwrap(), fv.constraint().unwrap());
                if let Some(tp) = tmp_tv_cache.get_typaram(&name) {
                    let tp = tp.clone();
                    if let TyParam::FreeVar(fv) = &tp {
                        if fv
                            .constraint()
                            .map(|cons| cons.is_uninited())
                            .unwrap_or(false)
                        {
                            let new_constr =
                                tmp_tv_cache.instantiate_constraint(constr, self, loc)?;
                            fv.update_constraint(new_constr, true);
                        }
                    }
                    Ok(tp)
                } else if let Some(t) = tmp_tv_cache.get_tyvar(&name) {
                    let t = t.clone();
                    if let Some(fv) = t.as_free() {
                        if fv
                            .constraint()
                            .map(|cons| cons.is_uninited())
                            .unwrap_or(false)
                        {
                            let new_constr =
                                tmp_tv_cache.instantiate_constraint(constr, self, loc)?;
                            fv.update_constraint(new_constr, true);
                        }
                    }
                    Ok(TyParam::t(t))
                } else {
                    let varname = VarName::from_str(name.clone());
                    if tmp_tv_cache.appeared(&name) {
                        let tp =
                            TyParam::named_free_var(name.clone(), self.level, Constraint::Uninited);
                        tmp_tv_cache.dummy_push_or_init_typaram(&varname, &tp, self);
                        return Ok(tp);
                    }
                    if let Some(tv_cache) = &self.tv_cache {
                        if let Some(tp) = tv_cache.get_typaram(&name) {
                            return Ok(tp.clone());
                        } else if let Some(t) = tv_cache.get_tyvar(&name) {
                            return Ok(TyParam::t(t.clone()));
                        }
                    }
                    tmp_tv_cache.push_appeared(name.clone());
                    let constr = tmp_tv_cache.instantiate_constraint(constr, self, loc)?;
                    let tp = TyParam::named_free_var(name.clone(), self.level, constr);
                    tmp_tv_cache.dummy_push_or_init_typaram(&varname, &tp, self);
                    Ok(tp)
                }
            }
            TyParam::Dict(dict) => {
                let dict = dict
                    .into_iter()
                    .map(|(k, v)| {
                        let k = self.instantiate_tp(k, tmp_tv_cache, loc)?;
                        let v = self.instantiate_tp(v, tmp_tv_cache, loc)?;
                        Ok((k, v))
                    })
                    .collect::<TyCheckResult<_>>()?;
                Ok(TyParam::Dict(dict))
            }
            TyParam::Array(arr) => {
                let arr = arr
                    .into_iter()
                    .map(|v| self.instantiate_tp(v, tmp_tv_cache, loc))
                    .collect::<TyCheckResult<_>>()?;
                Ok(TyParam::Array(arr))
            }
            TyParam::Set(set) => {
                let set = set
                    .into_iter()
                    .map(|v| self.instantiate_tp(v, tmp_tv_cache, loc))
                    .collect::<TyCheckResult<_>>()?;
                Ok(TyParam::Set(set))
            }
            TyParam::Tuple(tup) => {
                let tup = tup
                    .into_iter()
                    .map(|v| self.instantiate_tp(v, tmp_tv_cache, loc))
                    .collect::<TyCheckResult<_>>()?;
                Ok(TyParam::Tuple(tup))
            }
            TyParam::Record(rec) => {
                let rec = rec
                    .into_iter()
                    .map(|(k, v)| {
                        let v = self.instantiate_tp(v, tmp_tv_cache, loc)?;
                        Ok((k, v))
                    })
                    .collect::<TyCheckResult<_>>()?;
                Ok(TyParam::Record(rec))
            }
            TyParam::DataClass { name, fields } => {
                let fields = fields
                    .into_iter()
                    .map(|(k, v)| {
                        let v = self.instantiate_tp(v, tmp_tv_cache, loc)?;
                        Ok((k, v))
                    })
                    .collect::<TyCheckResult<_>>()?;
                Ok(TyParam::DataClass { name, fields })
            }
            TyParam::Lambda(lambda) => {
                let nd_params = lambda
                    .nd_params
                    .into_iter()
                    .map(|pt| pt.try_map_type(|t| self.instantiate_t_inner(t, tmp_tv_cache, loc)))
                    .collect::<TyCheckResult<_>>()?;
                let var_params = lambda
                    .var_params
                    .map(|pt| pt.try_map_type(|t| self.instantiate_t_inner(t, tmp_tv_cache, loc)))
                    .transpose()?;
                let d_params = lambda
                    .d_params
                    .into_iter()
                    .map(|pt| pt.try_map_type(|t| self.instantiate_t_inner(t, tmp_tv_cache, loc)))
                    .collect::<TyCheckResult<_>>()?;
                let kw_var_params = lambda
                    .kw_var_params
                    .map(|pt| pt.try_map_type(|t| self.instantiate_t_inner(t, tmp_tv_cache, loc)))
                    .transpose()?;
                let body = lambda
                    .body
                    .into_iter()
                    .map(|v| self.instantiate_tp(v, tmp_tv_cache, loc))
                    .collect::<TyCheckResult<_>>()?;
                Ok(TyParam::Lambda(TyParamLambda::new(
                    lambda.const_,
                    nd_params,
                    var_params,
                    d_params,
                    kw_var_params,
                    body,
                )))
            }
            TyParam::UnaryOp { op, val } => {
                let res = self.instantiate_tp(*val, tmp_tv_cache, loc)?;
                Ok(TyParam::unary(op, res))
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let lhs = self.instantiate_tp(*lhs, tmp_tv_cache, loc)?;
                let rhs = self.instantiate_tp(*rhs, tmp_tv_cache, loc)?;
                Ok(TyParam::bin(op, lhs, rhs))
            }
            TyParam::App { name, args } => {
                let mut new_args = Vec::with_capacity(args.len());
                for arg in args {
                    new_args.push(self.instantiate_tp(arg, tmp_tv_cache, loc)?);
                }
                Ok(TyParam::app(name, new_args))
            }
            TyParam::ProjCall { obj, attr, args } => {
                let obj = self.instantiate_tp(*obj, tmp_tv_cache, loc)?;
                let mut new_args = Vec::with_capacity(args.len());
                for arg in args {
                    new_args.push(self.instantiate_tp(arg, tmp_tv_cache, loc)?);
                }
                Ok(TyParam::proj_call(obj, attr, new_args))
            }
            TyParam::Proj { obj, attr } => {
                let obj = self.instantiate_tp(*obj, tmp_tv_cache, loc)?;
                Ok(TyParam::proj(obj, attr))
            }
            TyParam::Type(t) => {
                let t = self.instantiate_t_inner(*t, tmp_tv_cache, loc)?;
                Ok(TyParam::t(t))
            }
            TyParam::Value(ValueObj::Type(t)) => {
                let t = self.instantiate_t_inner(t.into_typ(), tmp_tv_cache, loc)?;
                Ok(TyParam::t(t))
            }
            p @ (TyParam::Value(_)
            | TyParam::Mono(_)
            | TyParam::FreeVar(_)
            | TyParam::Erased(_)) => Ok(p),
            other => {
                type_feature_error!(
                    self,
                    loc.loc(),
                    &format!("instantiating type-parameter {other}")
                )
            }
        }
    }

    fn instantiate_pred(
        &self,
        pred: Predicate,
        tmp_tv_cache: &mut TyVarCache,
        loc: &impl Locational,
    ) -> TyCheckResult<Predicate> {
        match pred {
            Predicate::And(l, r) => {
                let l = self.instantiate_pred(*l, tmp_tv_cache, loc)?;
                let r = self.instantiate_pred(*r, tmp_tv_cache, loc)?;
                Ok(Predicate::and(l, r))
            }
            Predicate::Or(l, r) => {
                let l = self.instantiate_pred(*l, tmp_tv_cache, loc)?;
                let r = self.instantiate_pred(*r, tmp_tv_cache, loc)?;
                Ok(Predicate::or(l, r))
            }
            Predicate::Not(pred) => {
                let pred = self.instantiate_pred(*pred, tmp_tv_cache, loc)?;
                Ok(Predicate::invert(pred))
            }
            Predicate::Equal { lhs, rhs } => {
                let rhs = self.instantiate_tp(rhs, tmp_tv_cache, loc)?;
                Ok(Predicate::eq(lhs, rhs))
            }
            Predicate::NotEqual { lhs, rhs } => {
                let rhs = self.instantiate_tp(rhs, tmp_tv_cache, loc)?;
                Ok(Predicate::ne(lhs, rhs))
            }
            Predicate::GreaterEqual { lhs, rhs } => {
                let rhs = self.instantiate_tp(rhs, tmp_tv_cache, loc)?;
                Ok(Predicate::ge(lhs, rhs))
            }
            Predicate::LessEqual { lhs, rhs } => {
                let rhs = self.instantiate_tp(rhs, tmp_tv_cache, loc)?;
                Ok(Predicate::le(lhs, rhs))
            }
            Predicate::Value(value) => {
                let value = self.instantiate_value(value, tmp_tv_cache, loc)?;
                Ok(Predicate::Value(value))
            }
            Predicate::Call {
                receiver,
                name,
                args,
            } => {
                let receiver = self.instantiate_tp(receiver, tmp_tv_cache, loc)?;
                let mut new_args = Vec::with_capacity(args.len());
                for arg in args {
                    new_args.push(self.instantiate_tp(arg, tmp_tv_cache, loc)?);
                }
                Ok(Predicate::call(receiver, name, new_args))
            }
            Predicate::GeneralEqual { lhs, rhs } => {
                let lhs = self.instantiate_pred(*lhs, tmp_tv_cache, loc)?;
                let rhs = self.instantiate_pred(*rhs, tmp_tv_cache, loc)?;
                Ok(Predicate::general_eq(lhs, rhs))
            }
            Predicate::GeneralGreaterEqual { lhs, rhs } => {
                let lhs = self.instantiate_pred(*lhs, tmp_tv_cache, loc)?;
                let rhs = self.instantiate_pred(*rhs, tmp_tv_cache, loc)?;
                Ok(Predicate::general_ge(lhs, rhs))
            }
            Predicate::GeneralLessEqual { lhs, rhs } => {
                let lhs = self.instantiate_pred(*lhs, tmp_tv_cache, loc)?;
                let rhs = self.instantiate_pred(*rhs, tmp_tv_cache, loc)?;
                Ok(Predicate::general_le(lhs, rhs))
            }
            Predicate::GeneralNotEqual { lhs, rhs } => {
                let lhs = self.instantiate_pred(*lhs, tmp_tv_cache, loc)?;
                let rhs = self.instantiate_pred(*rhs, tmp_tv_cache, loc)?;
                Ok(Predicate::general_ne(lhs, rhs))
            }
            _ => Ok(pred),
        }
    }

    fn instantiate_value(
        &self,
        value: ValueObj,
        tmp_tv_cache: &mut TyVarCache,
        loc: &impl Locational,
    ) -> TyCheckResult<ValueObj> {
        match value {
            ValueObj::Type(mut typ) => {
                let t = mem::take(typ.typ_mut());
                let t = self.instantiate_t_inner(t, tmp_tv_cache, loc)?;
                *typ.typ_mut() = t;
                Ok(ValueObj::Type(typ))
            }
            ValueObj::Subr(subr) => match subr {
                ConstSubr::Builtin(mut builtin) => {
                    let t = mem::take(&mut builtin.sig_t);
                    let t = self.instantiate_t_inner(t, tmp_tv_cache, loc)?;
                    builtin.sig_t = t;
                    Ok(ValueObj::Subr(ConstSubr::Builtin(builtin)))
                }
                ConstSubr::Gen(mut gen) => {
                    let t = mem::take(&mut gen.sig_t);
                    let t = self.instantiate_t_inner(t, tmp_tv_cache, loc)?;
                    gen.sig_t = t;
                    Ok(ValueObj::Subr(ConstSubr::Gen(gen)))
                }
                ConstSubr::User(mut user) => {
                    let t = mem::take(&mut user.sig_t);
                    let t = self.instantiate_t_inner(t, tmp_tv_cache, loc)?;
                    user.sig_t = t;
                    Ok(ValueObj::Subr(ConstSubr::User(user)))
                }
            },
            ValueObj::Array(arr) => {
                let mut new = vec![];
                for v in arr.iter().cloned() {
                    new.push(self.instantiate_value(v, tmp_tv_cache, loc)?);
                }
                Ok(ValueObj::Array(new.into()))
            }
            ValueObj::Tuple(tup) => {
                let mut new = vec![];
                for v in tup.iter().cloned() {
                    new.push(self.instantiate_value(v, tmp_tv_cache, loc)?);
                }
                Ok(ValueObj::Tuple(new.into()))
            }
            ValueObj::Set(set) => {
                let mut new = Set::new();
                for v in set.into_iter() {
                    let v = self.instantiate_value(v, tmp_tv_cache, loc)?;
                    new.insert(v);
                }
                Ok(ValueObj::Set(new))
            }
            ValueObj::Dict(dict) => {
                let mut new = Dict::new();
                for (k, v) in dict.into_iter() {
                    let k = self.instantiate_value(k, tmp_tv_cache, loc)?;
                    let v = self.instantiate_value(v, tmp_tv_cache, loc)?;
                    new.insert(k, v);
                }
                Ok(ValueObj::Dict(new))
            }
            ValueObj::Record(rec) => {
                let mut new = Dict::new();
                for (k, v) in rec.into_iter() {
                    let v = self.instantiate_value(v, tmp_tv_cache, loc)?;
                    new.insert(k, v);
                }
                Ok(ValueObj::Record(new))
            }
            ValueObj::DataClass { name, fields } => {
                let mut new = Dict::new();
                for (k, v) in fields.into_iter() {
                    let v = self.instantiate_value(v, tmp_tv_cache, loc)?;
                    new.insert(k, v);
                }
                Ok(ValueObj::DataClass { name, fields: new })
            }
            _ => Ok(value),
        }
    }

    /// 'T -> ?T (quantified to free)
    fn instantiate_t_inner(
        &self,
        unbound: Type,
        tmp_tv_cache: &mut TyVarCache,
        loc: &impl Locational,
    ) -> TyCheckResult<Type> {
        match unbound {
            FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.instantiate_t_inner(t, tmp_tv_cache, loc)
            }
            FreeVar(fv) if fv.is_generalized() => {
                let (name, constr) = (fv.unbound_name().unwrap(), fv.constraint().unwrap());
                if let Some(t) = tmp_tv_cache.get_tyvar(&name) {
                    let t = t.clone();
                    Ok(t)
                } else if let Some(tp) = tmp_tv_cache.get_typaram(&name) {
                    if let TyParam::Type(t) = tp {
                        let t = *t.clone();
                        Ok(t)
                    } else {
                        todo!(
                            "typaram_insts: {}\ntyvar_insts:{}\n{tp}",
                            tmp_tv_cache.typaram_instances,
                            tmp_tv_cache.tyvar_instances,
                        )
                    }
                } else {
                    let varname = VarName::from_str(name.clone());
                    if tmp_tv_cache.appeared(&name) {
                        let tyvar = named_free_var(name.clone(), self.level, Constraint::Uninited);
                        tmp_tv_cache.dummy_push_or_init_tyvar(&varname, &tyvar, self);
                        return Ok(tyvar);
                    }
                    if let Some(tv_ctx) = &self.tv_cache {
                        if let Some(t) = tv_ctx.get_tyvar(&name) {
                            return Ok(t.clone());
                        } else if let Some(tp) = tv_ctx.get_typaram(&name) {
                            if let TyParam::Type(t) = tp {
                                return Ok(*t.clone());
                            } else {
                                todo!(
                                    "typaram_insts: {}\ntyvar_insts:{}\n{tp}",
                                    tmp_tv_cache.typaram_instances,
                                    tmp_tv_cache.tyvar_instances,
                                )
                            }
                        }
                    }
                    tmp_tv_cache.push_appeared(name.clone());
                    let constr = tmp_tv_cache.instantiate_constraint(constr, self, loc)?;
                    let tyvar = named_free_var(name.clone(), self.level, constr);
                    tmp_tv_cache.dummy_push_or_init_tyvar(&varname, &tyvar, self);
                    Ok(tyvar)
                }
            }
            Refinement(mut refine) => {
                refine.t = Box::new(self.instantiate_t_inner(*refine.t, tmp_tv_cache, loc)?);
                refine.pred = Box::new(self.instantiate_pred(*refine.pred, tmp_tv_cache, loc)?);
                Ok(Type::Refinement(refine))
            }
            Subr(mut subr) => {
                for pt in subr.non_default_params.iter_mut() {
                    *pt.typ_mut() =
                        self.instantiate_t_inner(mem::take(pt.typ_mut()), tmp_tv_cache, loc)?;
                }
                if let Some(var) = subr.var_params.as_mut() {
                    *var.typ_mut() =
                        self.instantiate_t_inner(mem::take(var.typ_mut()), tmp_tv_cache, loc)?;
                }
                for pt in subr.default_params.iter_mut() {
                    *pt.typ_mut() =
                        self.instantiate_t_inner(mem::take(pt.typ_mut()), tmp_tv_cache, loc)?;
                    if let Some(default) = pt.default_typ_mut() {
                        *default =
                            self.instantiate_t_inner(mem::take(default), tmp_tv_cache, loc)?;
                    }
                }
                if let Some(kw_var) = subr.kw_var_params.as_mut() {
                    *kw_var.typ_mut() =
                        self.instantiate_t_inner(mem::take(kw_var.typ_mut()), tmp_tv_cache, loc)?;
                    if let Some(default) = kw_var.default_typ_mut() {
                        *default =
                            self.instantiate_t_inner(mem::take(default), tmp_tv_cache, loc)?;
                    }
                }
                let return_t = self.instantiate_t_inner(*subr.return_t, tmp_tv_cache, loc)?;
                let res = subr_t(
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|p| *p),
                    subr.default_params,
                    subr.kw_var_params.map(|p| *p),
                    return_t,
                );
                Ok(res)
            }
            Quantified(subr) => self.instantiate_t_inner(*subr, tmp_tv_cache, loc),
            Record(mut dict) => {
                for v in dict.values_mut() {
                    *v = self.instantiate_t_inner(mem::take(v), tmp_tv_cache, loc)?;
                }
                Ok(Type::Record(dict))
            }
            NamedTuple(mut tup) => {
                for (_, v) in tup.iter_mut() {
                    *v = self.instantiate_t_inner(mem::take(v), tmp_tv_cache, loc)?;
                }
                Ok(Type::NamedTuple(tup))
            }
            Ref(t) => {
                let t = self.instantiate_t_inner(*t, tmp_tv_cache, loc)?;
                Ok(ref_(t))
            }
            RefMut { before, after } => {
                let before = self.instantiate_t_inner(*before, tmp_tv_cache, loc)?;
                let after = after
                    .map(|aft| self.instantiate_t_inner(*aft, tmp_tv_cache, loc))
                    .transpose()?;
                Ok(ref_mut(before, after))
            }
            Proj { lhs, rhs } => {
                let lhs = self.instantiate_t_inner(*lhs, tmp_tv_cache, loc)?;
                Ok(proj(lhs, rhs))
            }
            ProjCall {
                lhs,
                attr_name,
                mut args,
            } => {
                let lhs = self.instantiate_tp(*lhs, tmp_tv_cache, loc)?;
                for arg in args.iter_mut() {
                    *arg = self.instantiate_tp(mem::take(arg), tmp_tv_cache, loc)?;
                }
                Ok(proj_call(lhs, attr_name, args))
            }
            Poly { name, mut params } => {
                for param in params.iter_mut() {
                    *param = self.instantiate_tp(mem::take(param), tmp_tv_cache, loc)?;
                }
                Ok(poly(name, params))
            }
            Structural(t) => {
                // avoid infinite recursion
                if tmp_tv_cache.structural_inner {
                    Ok(t.structuralize())
                } else {
                    if t.is_recursive() {
                        tmp_tv_cache.structural_inner = true;
                    }
                    let t = self.instantiate_t_inner(*t, tmp_tv_cache, loc)?;
                    Ok(t.structuralize())
                }
            }
            FreeVar(fv) => {
                if let Some((sub, sup)) = fv.get_subsup() {
                    let sub = if sub.is_recursive() {
                        sub
                    } else {
                        self.instantiate_t_inner(sub, tmp_tv_cache, loc)?
                    };
                    let sup = if sup.is_recursive() {
                        sup
                    } else {
                        self.instantiate_t_inner(sup, tmp_tv_cache, loc)?
                    };
                    let new_constraint = Constraint::new_sandwiched(sub, sup);
                    fv.update_constraint(new_constraint, true);
                } else if let Some(ty) = fv.get_type() {
                    let ty = self.instantiate_t_inner(ty, tmp_tv_cache, loc)?;
                    let new_constraint = Constraint::new_type_of(ty);
                    fv.update_constraint(new_constraint, true);
                }
                Ok(FreeVar(fv))
            }
            And(l, r) => {
                let l = self.instantiate_t_inner(*l, tmp_tv_cache, loc)?;
                let r = self.instantiate_t_inner(*r, tmp_tv_cache, loc)?;
                Ok(self.intersection(&l, &r))
            }
            Or(l, r) => {
                let l = self.instantiate_t_inner(*l, tmp_tv_cache, loc)?;
                let r = self.instantiate_t_inner(*r, tmp_tv_cache, loc)?;
                Ok(self.union(&l, &r))
            }
            Not(ty) => {
                let ty = self.instantiate_t_inner(*ty, tmp_tv_cache, loc)?;
                Ok(self.complement(&ty))
            }
            other if other.is_monomorphic() => Ok(other),
            other => type_feature_error!(self, loc.loc(), &format!("instantiating type {other}")),
        }
    }

    pub(crate) fn instantiate(&self, quantified: Type, callee: &hir::Expr) -> TyCheckResult<Type> {
        match quantified {
            FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.instantiate(t, callee)
            }
            And(lhs, rhs) => {
                let lhs = self.instantiate(*lhs, callee)?;
                let rhs = self.instantiate(*rhs, callee)?;
                Ok(lhs & rhs)
            }
            Quantified(quant) => {
                let mut tmp_tv_cache = TyVarCache::new(self.level, self);
                let ty = self.instantiate_t_inner(*quant, &mut tmp_tv_cache, callee)?;
                if let Some(self_t) = ty.self_t() {
                    self.sub_unify(callee.ref_t(), self_t, callee, Some(&Str::ever("self")))?;
                }
                if cfg!(feature = "debug") && ty.has_qvar() {
                    panic!("{ty} has qvar")
                }
                Ok(ty)
            }
            // HACK: {op: |T|(T -> T) | op == F} => ?T -> ?T
            Refinement(refine) if refine.t.is_quantified_subr() => {
                let t = self.instantiate(*refine.t, callee)?;
                match &t {
                    Type::Subr(subr) => {
                        if let Some(self_t) = subr.self_t() {
                            self.sub_unify(
                                callee.ref_t(),
                                self_t,
                                callee,
                                Some(&Str::ever("self")),
                            )?;
                        }
                    }
                    Type::And(l, r) => {
                        if let Some(self_t) = l.self_t() {
                            self.sub_unify(
                                callee.ref_t(),
                                self_t,
                                callee,
                                Some(&Str::ever("self")),
                            )?;
                        }
                        if let Some(self_t) = r.self_t() {
                            self.sub_unify(
                                callee.ref_t(),
                                self_t,
                                callee,
                                Some(&Str::ever("self")),
                            )?;
                        }
                    }
                    other => unreachable!("{other}"),
                }
                Ok(t)
            }
            Subr(subr) if subr.has_qvar() => {
                log!(err "{subr} has qvar");
                self.instantiate(Type::Subr(subr).quantify(), callee)
            }
            // rank-1制限により、通常の型(rank-0型)の内側に量化型は存在しない
            other => Ok(other),
        }
    }

    pub(crate) fn instantiate_dummy(&self, quantified: Type) -> TyCheckResult<Type> {
        match quantified {
            FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.instantiate_dummy(t)
            }
            And(lhs, rhs) => {
                let lhs = self.instantiate_dummy(*lhs)?;
                let rhs = self.instantiate_dummy(*rhs)?;
                Ok(lhs & rhs)
            }
            Quantified(quant) => {
                let mut tmp_tv_cache = TyVarCache::new(self.level, self);
                let ty = self.instantiate_t_inner(*quant, &mut tmp_tv_cache, &())?;
                if cfg!(feature = "debug") && ty.has_qvar() {
                    panic!("{ty} has qvar")
                }
                Ok(ty)
            }
            Refinement(refine) if refine.t.is_quantified_subr() => {
                let quant = enum_unwrap!(*refine.t, Type::Quantified);
                let mut tmp_tv_cache = TyVarCache::new(self.level, self);
                self.instantiate_t_inner(*quant, &mut tmp_tv_cache, &())
            }
            _other => unreachable_error!(TyCheckErrors, TyCheckError, self),
        }
    }

    pub(crate) fn instantiate_def_type(&self, typ: &Type) -> TyCheckResult<Type> {
        let mut tv_cache = TyVarCache::new(self.level, self);
        self.instantiate_t_inner(typ.clone(), &mut tv_cache, &())
    }
}
