use std::fmt;
use std::mem;
use std::option::Option; // conflicting to Type::Option

use erg_common::astr::AtomicStr;
use erg_common::dict::Dict;
use erg_common::error::Location;
#[allow(unused)]
use erg_common::log;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{assume_unreachable, enum_unwrap, set, try_map_mut};

use ast::{
    ParamSignature, ParamTySpec, PreDeclTypeSpec, SimpleTypeSpec, TypeBoundSpec, TypeBoundSpecs,
    TypeSpec,
};
use erg_parser::ast;
use erg_parser::token::TokenKind;

use crate::ty::constructors::*;
use crate::ty::free::{Constraint, Cyclicity, FreeTyVar};
use crate::ty::typaram::{IntervalOp, TyParam, TyParamOrdering};
use crate::ty::value::ValueObj;
use crate::ty::{HasType, ParamTy, Predicate, SubrKind, TyBound, Type};
use TyParamOrdering::*;
use Type::*;

use crate::context::{Context, RegistrationMode};
use crate::error::{SingleTyCheckResult, TyCheckError, TyCheckErrors, TyCheckResult};
use crate::hir;
use crate::AccessKind;
use RegistrationMode::*;

/// Context for instantiating a quantified type
/// 量化型をインスタンス化するための文脈
/// e.g. Array -> [("T": ?T(: Type)), ("N": ?N(: Nat))]
/// FIXME: current implementation is wrong
/// It will not work unless the type variable is used with the same name as the definition.
#[derive(Debug, Clone)]
pub struct TyVarInstContext {
    level: usize,
    pub(crate) tyvar_instances: Dict<Str, Type>,
    pub(crate) typaram_instances: Dict<Str, TyParam>,
}

impl fmt::Display for TyVarInstContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "TyVarInstContext {{ tyvar_instances: {}, typaram_instances: {} }}",
            self.tyvar_instances, self.typaram_instances,
        )
    }
}

impl TyVarInstContext {
    pub fn new(level: usize, bounds: Set<TyBound>, ctx: &Context) -> Self {
        let mut self_ = Self {
            level,
            tyvar_instances: Dict::new(),
            typaram_instances: Dict::new(),
        };
        for bound in bounds.into_iter() {
            self_.instantiate_bound(bound, ctx);
        }
        self_
    }

    pub fn concat(self, other: Self) -> Self {
        Self {
            level: self.level.min(other.level), // REVIEW
            tyvar_instances: self.tyvar_instances.concat(other.tyvar_instances),
            typaram_instances: self.typaram_instances.concat(other.typaram_instances),
        }
    }

    fn instantiate_const_template(
        &mut self,
        var_name: &str,
        _callee_name: &Str,
        ct: &ConstTemplate,
    ) -> TyParam {
        match ct {
            ConstTemplate::Obj(o) => match o {
                ValueObj::Type(t) if t.typ().is_mono_q() => {
                    if &t.typ().qual_name()[..] == "Self" {
                        let constraint = Constraint::new_type_of(Type);
                        let t = named_free_var(Str::rc(var_name), self.level, constraint);
                        TyParam::t(t)
                    } else {
                        todo!()
                    }
                }
                ValueObj::Type(t) => TyParam::t(t.typ().clone()),
                v => TyParam::Value(v.clone()),
            },
            ConstTemplate::App { .. } => {
                todo!()
            }
        }
    }

    fn instantiate_poly(
        &mut self,
        tvar_name: Str,
        name: &Str,
        params: Vec<TyParam>,
        ctx: &Context,
    ) -> Type {
        if let Some(temp_defaults) = ctx.rec_get_const_param_defaults(name) {
            let ctx = ctx
                .get_nominal_type_ctx(&poly(name.clone(), params.clone()))
                .unwrap_or_else(|| panic!("{} not found", name));
            let defined_params_len = ctx.params.len();
            let given_params_len = params.len();
            if defined_params_len < given_params_len {
                panic!()
            }
            let inst_non_defaults = self.instantiate_params(params);
            let mut inst_defaults = vec![];
            for template in temp_defaults
                .iter()
                .take(defined_params_len - given_params_len)
            {
                let tp = self.instantiate_const_template(&tvar_name, name, template);
                self.push_or_init_typaram(&tp.tvar_name().unwrap(), &tp);
                inst_defaults.push(tp);
            }
            poly(name, [inst_non_defaults, inst_defaults].concat())
        } else {
            poly(name, self.instantiate_params(params))
        }
    }

    fn instantiate_params(&mut self, params: Vec<TyParam>) -> Vec<TyParam> {
        params
            .into_iter()
            .map(|p| {
                if let Some(name) = p.tvar_name() {
                    let tp = self.instantiate_qtp(p);
                    self.push_or_init_typaram(&name, &tp);
                    tp
                } else {
                    p
                }
            })
            .collect()
    }

    fn instantiate_bound_type(&mut self, mid: &Type, sub_or_sup: Type, ctx: &Context) -> Type {
        match sub_or_sup {
            Type::Poly { name, params } => {
                self.instantiate_poly(mid.qual_name(), &name, params, ctx)
            }
            Type::Proj { lhs, rhs } => {
                let lhs = if lhs.has_qvar() {
                    self.instantiate_qvar(*lhs)
                } else {
                    *lhs
                };
                proj(lhs, rhs)
            }
            Type::Ref(t) if t.has_qvar() => ref_(self.instantiate_qvar(*t)),
            Type::RefMut { before, after } => {
                let before = if before.has_qvar() {
                    self.instantiate_qvar(*before)
                } else {
                    *before
                };
                let after = after.map(|t| {
                    if t.has_qvar() {
                        self.instantiate_qvar(*t)
                    } else {
                        *t
                    }
                });
                ref_mut(before, after)
            }
            Type::And(l, r) => {
                let l = if l.has_qvar() {
                    self.instantiate_qvar(*l)
                } else {
                    *l
                };
                let r = if r.has_qvar() {
                    self.instantiate_qvar(*r)
                } else {
                    *r
                };
                and(l, r)
            }
            Type::Or(l, r) => {
                let l = if l.has_qvar() {
                    self.instantiate_qvar(*l)
                } else {
                    *l
                };
                let r = if r.has_qvar() {
                    self.instantiate_qvar(*r)
                } else {
                    *r
                };
                or(l, r)
            }
            Type::Not(l, r) => {
                let l = if l.has_qvar() {
                    self.instantiate_qvar(*l)
                } else {
                    *l
                };
                let r = if r.has_qvar() {
                    self.instantiate_qvar(*r)
                } else {
                    *r
                };
                not(l, r)
            }
            Type::MonoQVar(_) => self.instantiate_qvar(sub_or_sup),
            other => other,
        }
    }

    fn instantiate_bound(&mut self, bound: TyBound, ctx: &Context) {
        match bound {
            TyBound::Sandwiched { sub, mid, sup } => {
                let sub_instance = self.instantiate_bound_type(&mid, sub, ctx);
                let sup_instance = self.instantiate_bound_type(&mid, sup, ctx);
                let name = mid.qual_name();
                let constraint =
                    Constraint::new_sandwiched(sub_instance, sup_instance, Cyclicity::Not);
                self.push_or_init_tyvar(
                    &name,
                    &named_free_var(name.clone(), self.level, constraint),
                );
            }
            TyBound::Instance { name, t } => {
                let t = match t {
                    Type::FreeVar(fv) if fv.is_linked() => todo!(),
                    Type::Poly { name, params } => {
                        self.instantiate_poly(name.clone(), &name, params, ctx)
                    }
                    t => t,
                };
                let constraint = Constraint::new_type_of(t.clone());
                if t.is_type() {
                    if let Some(tv) = self.tyvar_instances.get(&name) {
                        tv.update_constraint(constraint);
                    } else if let Some(tp) = self.typaram_instances.get(&name) {
                        tp.update_constraint(constraint);
                    } else {
                        self.push_or_init_tyvar(
                            &name,
                            &named_free_var(name.clone(), self.level, constraint),
                        );
                    }
                } else if let Some(tp) = self.typaram_instances.get(&name) {
                    tp.update_constraint(constraint);
                } else {
                    self.push_or_init_typaram(
                        &name,
                        &TyParam::named_free_var(name.clone(), self.level, t),
                    );
                }
            }
        }
    }

    fn _instantiate_pred(&self, _pred: Predicate) -> Predicate {
        todo!()
    }

    fn instantiate_qvar(&mut self, quantified: Type) -> Type {
        match quantified {
            Type::MonoQVar(n) => {
                if let Some(t) = self.get_tyvar(&n) {
                    t.clone()
                } else if let Some(t) = self.get_typaram(&n) {
                    if let TyParam::Type(t) = t {
                        *t.clone()
                    } else {
                        todo!()
                    }
                } else {
                    let tv = named_free_var(n.clone(), self.level, Constraint::Uninited);
                    self.push_or_init_tyvar(&n, &tv);
                    tv
                }
            }
            other => todo!("{other}"),
        }
    }

    pub(crate) fn get_qvar(&self, quantified: Type) -> Option<Type> {
        match quantified {
            Type::MonoQVar(n) => {
                if let Some(t) = self.get_tyvar(&n) {
                    Some(t.clone())
                } else if let Some(t) = self.get_typaram(&n) {
                    if let TyParam::Type(t) = t {
                        Some(*t.clone())
                    } else {
                        todo!()
                    }
                } else {
                    None
                }
            }
            other => todo!("{other}"),
        }
    }

    fn instantiate_qtp(&mut self, quantified: TyParam) -> TyParam {
        match quantified {
            TyParam::MonoQVar(n) => {
                if let Some(t) = self.get_typaram(&n) {
                    t.clone()
                } else if let Some(t) = self.get_tyvar(&n) {
                    TyParam::t(t.clone())
                } else {
                    let tp = TyParam::named_free_var(n.clone(), self.level, Type::Uninited);
                    self.push_or_init_typaram(&n, &tp);
                    tp
                }
            }
            TyParam::Type(t) => {
                if let Some(n) = t.as_ref().tvar_name() {
                    if let Some(t) = self.get_typaram(&n) {
                        t.clone()
                    } else if let Some(t) = self.get_tyvar(&n) {
                        TyParam::t(t.clone())
                    } else {
                        let tv = named_free_var(n.clone(), self.level, Constraint::Uninited);
                        self.push_or_init_tyvar(&n, &tv);
                        TyParam::t(tv)
                    }
                } else {
                    unreachable!("{t}")
                }
            }
            TyParam::UnaryOp { op, val } => {
                let res = self.instantiate_qtp(*val);
                TyParam::unary(op, res)
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let lhs = self.instantiate_qtp(*lhs);
                let rhs = self.instantiate_qtp(*rhs);
                TyParam::bin(op, lhs, rhs)
            }
            TyParam::App { .. } => todo!(),
            p @ TyParam::Value(_) => p,
            other => todo!("{other}"),
        }
    }

    fn get_qtp(&self, quantified: TyParam) -> Option<TyParam> {
        match quantified {
            TyParam::MonoQVar(n) => {
                if let Some(t) = self.get_typaram(&n) {
                    Some(t.clone())
                } else {
                    self.get_tyvar(&n).map(|t| TyParam::t(t.clone()))
                }
            }
            TyParam::Type(t) => {
                if let Some(n) = t.as_ref().tvar_name() {
                    if let Some(t) = self.get_typaram(&n) {
                        Some(t.clone())
                    } else {
                        self.get_tyvar(&n).map(|t| TyParam::t(t.clone()))
                    }
                } else {
                    unreachable!("{t}")
                }
            }
            TyParam::UnaryOp { op, val } => {
                let res = self.get_qtp(*val)?;
                Some(TyParam::unary(op, res))
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let lhs = self.get_qtp(*lhs)?;
                let rhs = self.get_qtp(*rhs)?;
                Some(TyParam::bin(op, lhs, rhs))
            }
            TyParam::App { .. } => todo!(),
            p @ TyParam::Value(_) => Some(p),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn push_or_init_tyvar(&mut self, name: &Str, tv: &Type) {
        if let Some(inst) = self.tyvar_instances.get(name) {
            // T<tv> <: Eq(T<inst>)
            // T<inst> is uninitialized
            // T<inst>.link(T<tv>);
            // T <: Eq(T <: Eq(T <: ...))
            if let Type::FreeVar(fv_inst) = inst {
                self.check_cyclicity_and_link(name, fv_inst, tv);
            } else {
                todo!()
            }
        } else if let Some(inst) = self.typaram_instances.get(name) {
            if let TyParam::Type(inst) = inst {
                if let Type::FreeVar(fv_inst) = inst.as_ref() {
                    self.check_cyclicity_and_link(name, fv_inst, tv);
                } else {
                    todo!()
                }
            } else {
                todo!()
            }
        }
        self.tyvar_instances.insert(name.clone(), tv.clone());
    }

    fn check_cyclicity_and_link(&self, name: &str, fv_inst: &FreeTyVar, tv: &Type) {
        let (sub, sup) = enum_unwrap!(tv, Type::FreeVar).get_bound_types().unwrap();
        let new_cyclicity = match (sup.contains_tvar(name), sub.contains_tvar(name)) {
            (true, true) => Cyclicity::Both,
            // T <: Super
            (true, _) => Cyclicity::Super,
            // T :> Sub
            (false, true) => Cyclicity::Sub,
            _ => Cyclicity::Not,
        };
        fv_inst.link(tv);
        tv.update_cyclicity(new_cyclicity);
    }

    pub(crate) fn push_or_init_typaram(&mut self, name: &Str, tp: &TyParam) {
        // FIXME:
        if let Some(_tp) = self.typaram_instances.get(name) {
            return;
        }
        if let Some(_t) = self.tyvar_instances.get(name) {
            return;
        }
        self.typaram_instances.insert(name.clone(), tp.clone());
    }

    pub(crate) fn get_tyvar(&self, name: &str) -> Option<&Type> {
        self.tyvar_instances.get(name).or_else(|| {
            self.typaram_instances.get(name).map(|t| {
                if let TyParam::Type(t) = t {
                    t.as_ref()
                } else {
                    todo!("{t}")
                }
            })
        })
    }

    pub(crate) fn get_typaram(&self, name: &str) -> Option<&TyParam> {
        self.typaram_instances.get(name)
    }
}

/// TODO: this struct will be removed when const functions are implemented.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstTemplate {
    Obj(ValueObj),
    App {
        name: Str,
        non_default_args: Vec<Type>,
        default_args: Vec<ConstTemplate>,
    },
}

impl ConstTemplate {
    pub const fn app(
        name: &'static str,
        non_default_args: Vec<Type>,
        default_args: Vec<ConstTemplate>,
    ) -> Self {
        ConstTemplate::App {
            name: Str::ever(name),
            non_default_args,
            default_args,
        }
    }
}

impl Context {
    pub(crate) fn instantiate_var_sig_t(
        &self,
        t_spec: Option<&TypeSpec>,
        opt_eval_t: Option<Type>,
        mode: RegistrationMode,
    ) -> TyCheckResult<Type> {
        let spec_t = if let Some(s) = t_spec {
            self.instantiate_typespec(s, None, None, mode)?
        } else {
            free_var(self.level, Constraint::new_type_of(Type))
        };
        if let Some(eval_t) = opt_eval_t {
            self.sub_unify(
                &eval_t,
                &spec_t,
                t_spec.map(|s| s.loc()).unwrap_or(Location::Unknown),
                None,
            )?;
        }
        Ok(spec_t)
    }

    pub(crate) fn instantiate_sub_sig_t(
        &self,
        sig: &ast::SubrSignature,
        mode: RegistrationMode,
    ) -> TyCheckResult<Type> {
        // -> Result<Type, (Type, TyCheckErrors)> {
        let opt_decl_sig_t = self
            .rec_get_decl_info(&sig.ident, AccessKind::Name, &self.cfg.input, &self.name)
            .ok()
            .map(|vi| enum_unwrap!(vi.t, Type::Subr));
        let bounds = self.instantiate_ty_bounds(&sig.bounds, PreRegister)?;
        let tv_ctx = TyVarInstContext::new(self.level, bounds, self);
        let mut non_defaults = vec![];
        for (n, p) in sig.params.non_defaults.iter().enumerate() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .and_then(|subr| subr.non_default_params.get(n));
            non_defaults.push(self.instantiate_param_ty(p, opt_decl_t, Some(&tv_ctx), mode)?);
        }
        let var_args = if let Some(var_args) = sig.params.var_args.as_ref() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .and_then(|subr| subr.var_params.as_ref().map(|v| v.as_ref()));
            Some(self.instantiate_param_ty(var_args, opt_decl_t, Some(&tv_ctx), mode)?)
        } else {
            None
        };
        let mut defaults = vec![];
        for (n, p) in sig.params.defaults.iter().enumerate() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .and_then(|subr| subr.default_params.get(n));
            defaults.push(self.instantiate_param_ty(p, opt_decl_t, Some(&tv_ctx), mode)?);
        }
        let spec_return_t = if let Some(s) = sig.return_t_spec.as_ref() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .map(|subr| ParamTy::anonymous(subr.return_t.as_ref().clone()));
            self.instantiate_typespec(s, opt_decl_t.as_ref(), Some(&tv_ctx), mode)?
        } else {
            // preregisterならouter scopeで型宣言(see inference.md)
            let level = if mode == PreRegister {
                self.level
            } else {
                self.level + 1
            };
            free_var(level, Constraint::new_type_of(Type))
        };
        Ok(if sig.ident.is_procedural() {
            proc(non_defaults, var_args, defaults, spec_return_t)
        } else {
            func(non_defaults, var_args, defaults, spec_return_t)
        })
    }

    /// spec_t == Noneかつリテラル推論が不可能なら型変数を発行する
    pub(crate) fn instantiate_param_sig_t(
        &self,
        sig: &ParamSignature,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_ctx: Option<&TyVarInstContext>,
        mode: RegistrationMode,
    ) -> TyCheckResult<Type> {
        let spec_t = if let Some(spec_with_op) = &sig.t_spec {
            self.instantiate_typespec(&spec_with_op.t_spec, opt_decl_t, tmp_tv_ctx, mode)?
        } else {
            match &sig.pat {
                ast::ParamPattern::Lit(lit) => v_enum(set![self.eval_lit(lit)?]),
                // TODO: Array<Lit>
                _ => {
                    let level = if mode == PreRegister {
                        self.level
                    } else {
                        self.level + 1
                    };
                    free_var(level, Constraint::new_type_of(Type))
                }
            }
        };
        if let Some(decl_pt) = opt_decl_t {
            self.sub_unify(
                decl_pt.typ(),
                &spec_t,
                sig.t_spec
                    .as_ref()
                    .map(|s| s.loc())
                    .unwrap_or_else(|| sig.loc()),
                None,
            )?;
        }
        Ok(spec_t)
    }

    pub(crate) fn instantiate_param_ty(
        &self,
        sig: &ParamSignature,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_ctx: Option<&TyVarInstContext>,
        mode: RegistrationMode,
    ) -> TyCheckResult<ParamTy> {
        let t = self.instantiate_param_sig_t(sig, opt_decl_t, tmp_tv_ctx, mode)?;
        match (sig.inspect(), &sig.opt_default_val) {
            (Some(name), Some(default)) => {
                let default = self.instantiate_const_expr(default)?;
                Ok(ParamTy::kw_default(
                    name.clone(),
                    t,
                    self.get_tp_t(&default)?,
                ))
            }
            (Some(name), None) => Ok(ParamTy::kw(name.clone(), t)),
            (None, None) => Ok(ParamTy::anonymous(t)),
            _ => unreachable!(),
        }
    }

    pub(crate) fn instantiate_predecl_t(
        &self,
        predecl: &PreDeclTypeSpec,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_ctx: Option<&TyVarInstContext>,
    ) -> TyCheckResult<Type> {
        match predecl {
            ast::PreDeclTypeSpec::Simple(simple) => {
                self.instantiate_simple_t(simple, opt_decl_t, tmp_tv_ctx)
            }
            _ => todo!(),
        }
    }

    pub(crate) fn instantiate_simple_t(
        &self,
        simple: &SimpleTypeSpec,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_ctx: Option<&TyVarInstContext>,
    ) -> TyCheckResult<Type> {
        match &simple.name.inspect()[..] {
            "_" | "Obj" => Ok(Type::Obj),
            "Nat" => Ok(Type::Nat),
            "Int" => Ok(Type::Int),
            "Ratio" => Ok(Type::Ratio),
            "Float" => Ok(Type::Float),
            "Str" => Ok(Type::Str),
            "Bool" => Ok(Type::Bool),
            "NoneType" => Ok(Type::NoneType),
            "Ellipsis" => Ok(Type::Ellipsis),
            "NotImplemented" => Ok(Type::NotImplemented),
            "Inf" => Ok(Type::Inf),
            "NegInf" => Ok(Type::NegInf),
            "Array" => {
                // TODO: kw
                let mut args = simple.args.pos_args();
                if let Some(first) = args.next() {
                    let t = self.instantiate_const_expr_as_type(&first.expr)?;
                    let len = args.next().unwrap();
                    let len = self.instantiate_const_expr(&len.expr)?;
                    Ok(array_t(t, len))
                } else {
                    Ok(mono("GenericArray"))
                }
            }
            other if simple.args.is_empty() => {
                if let Some(tmp_tv_ctx) = tmp_tv_ctx {
                    if let Ok(t) =
                        self.instantiate_t(mono_q(Str::rc(other)), tmp_tv_ctx, simple.loc())
                    {
                        return Ok(t);
                    }
                }
                if let Some(tv_ctx) = &self.tv_ctx {
                    if let Some(t) = tv_ctx.get_qvar(MonoQVar(Str::rc(other))) {
                        return Ok(t);
                    }
                }
                if let Some(outer) = &self.outer {
                    if let Ok(t) = outer.instantiate_simple_t(simple, opt_decl_t, None) {
                        return Ok(t);
                    }
                }
                if let Some(decl_t) = opt_decl_t {
                    return Ok(decl_t.typ().clone());
                }
                if let Some((typ, _)) = self.rec_get_type(other) {
                    Ok(typ.clone())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        simple.loc(),
                        self.caused_by(),
                        other,
                        self.get_similar_name(other),
                    )))
                }
            }
            other => {
                // FIXME: kw args
                let mut new_params = vec![];
                for arg in simple.args.pos_args() {
                    match &arg.expr {
                        ast::ConstExpr::Lit(lit) => {
                            new_params.push(TyParam::Value(self.eval_lit(lit)?));
                        }
                        ast::ConstExpr::Accessor(ast::ConstAccessor::Local(name)) => {
                            if let Some((typ, _)) = self.rec_get_type(name.inspect()) {
                                new_params.push(TyParam::t(typ.clone()));
                            } else {
                                return Err(TyCheckErrors::from(TyCheckError::no_var_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    name.loc(),
                                    self.caused_by(),
                                    name.inspect(),
                                    self.get_similar_name(name.inspect()),
                                )));
                            }
                        }
                        other => {
                            todo!("instantiating {other}")
                        }
                    }
                }
                // FIXME: non-builtin
                Ok(poly(Str::rc(other), new_params))
            }
        }
    }

    pub(crate) fn instantiate_const_expr(&self, expr: &ast::ConstExpr) -> TyCheckResult<TyParam> {
        match expr {
            ast::ConstExpr::Lit(lit) => Ok(TyParam::Value(self.eval_lit(lit)?)),
            ast::ConstExpr::Accessor(ast::ConstAccessor::Local(name)) => {
                if &name.inspect()[..] == "_" {
                    Ok(TyParam::erased(Type::Uninited))
                } else {
                    Ok(TyParam::Mono(name.inspect().clone()))
                }
            }
            _ => todo!(),
        }
    }

    pub(crate) fn instantiate_const_expr_as_type(
        &self,
        expr: &ast::ConstExpr,
    ) -> SingleTyCheckResult<Type> {
        match expr {
            ast::ConstExpr::Accessor(ast::ConstAccessor::Local(name)) => Ok(mono(name.inspect())),
            _ => todo!(),
        }
    }

    fn instantiate_func_param_spec(
        &self,
        p: &ParamTySpec,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_ctx: Option<&TyVarInstContext>,
        mode: RegistrationMode,
    ) -> TyCheckResult<ParamTy> {
        let t = self.instantiate_typespec(&p.ty, opt_decl_t, tmp_tv_ctx, mode)?;
        Ok(ParamTy::pos(
            p.name.as_ref().map(|t| t.inspect().to_owned()),
            t,
        ))
    }

    pub(crate) fn instantiate_typespec(
        &self,
        spec: &TypeSpec,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_ctx: Option<&TyVarInstContext>,
        mode: RegistrationMode,
    ) -> TyCheckResult<Type> {
        match spec {
            TypeSpec::PreDeclTy(predecl) => {
                Ok(self.instantiate_predecl_t(predecl, opt_decl_t, tmp_tv_ctx)?)
            }
            TypeSpec::And(lhs, rhs) => Ok(self.union(
                &self.instantiate_typespec(lhs, opt_decl_t, tmp_tv_ctx, mode)?,
                &self.instantiate_typespec(rhs, opt_decl_t, tmp_tv_ctx, mode)?,
            )),
            TypeSpec::Or(lhs, rhs) => Ok(self.intersection(
                &self.instantiate_typespec(lhs, opt_decl_t, tmp_tv_ctx, mode)?,
                &self.instantiate_typespec(rhs, opt_decl_t, tmp_tv_ctx, mode)?,
            )),
            TypeSpec::Not(lhs, rhs) => Ok(not(
                self.instantiate_typespec(lhs, opt_decl_t, tmp_tv_ctx, mode)?,
                self.instantiate_typespec(rhs, opt_decl_t, tmp_tv_ctx, mode)?,
            )),
            TypeSpec::Array(arr) => {
                let elem_t = self.instantiate_typespec(&arr.ty, opt_decl_t, tmp_tv_ctx, mode)?;
                let mut len = self.instantiate_const_expr(&arr.len)?;
                if let TyParam::Erased(t) = &mut len {
                    *t.as_mut() = Type::Nat;
                }
                Ok(array_t(elem_t, len))
            }
            TypeSpec::Set(set) => {
                let elem_t = self.instantiate_typespec(&set.ty, opt_decl_t, tmp_tv_ctx, mode)?;
                let mut len = self.instantiate_const_expr(&set.len)?;
                if let TyParam::Erased(t) = &mut len {
                    *t.as_mut() = Type::Nat;
                }
                Ok(set_t(elem_t, len))
            }
            TypeSpec::Tuple(tys) => {
                let mut inst_tys = vec![];
                for spec in tys {
                    inst_tys.push(self.instantiate_typespec(spec, opt_decl_t, tmp_tv_ctx, mode)?);
                }
                Ok(tuple_t(inst_tys))
            }
            // TODO: エラー処理(リテラルでない)はパーサーにやらせる
            TypeSpec::Enum(set) => {
                let mut new_set = set! {};
                for arg in set.pos_args() {
                    if let ast::ConstExpr::Lit(lit) = &arg.expr {
                        new_set.insert(self.eval_lit(lit)?);
                    } else {
                        todo!()
                    }
                }
                Ok(v_enum(new_set))
            }
            TypeSpec::Interval { op, lhs, rhs } => {
                let op = match op.kind {
                    TokenKind::Closed => IntervalOp::Closed,
                    TokenKind::LeftOpen => IntervalOp::LeftOpen,
                    TokenKind::RightOpen => IntervalOp::RightOpen,
                    TokenKind::Open => IntervalOp::Open,
                    _ => assume_unreachable!(),
                };
                let l = self.instantiate_const_expr(lhs)?;
                let l = self.eval_tp(&l)?;
                let r = self.instantiate_const_expr(rhs)?;
                let r = self.eval_tp(&r)?;
                if let Some(Greater) = self.try_cmp(&l, &r) {
                    panic!("{l}..{r} is not a valid interval type (should be lhs <= rhs)")
                }
                Ok(int_interval(op, l, r))
            }
            TypeSpec::Subr(subr) => {
                let non_defaults = try_map_mut(subr.non_defaults.iter(), |p| {
                    self.instantiate_func_param_spec(p, opt_decl_t, tmp_tv_ctx, mode)
                })?;
                let var_args = subr
                    .var_args
                    .as_ref()
                    .map(|p| self.instantiate_func_param_spec(p, opt_decl_t, tmp_tv_ctx, mode))
                    .transpose()?;
                let defaults = try_map_mut(subr.defaults.iter(), |p| {
                    self.instantiate_func_param_spec(p, opt_decl_t, tmp_tv_ctx, mode)
                })?
                .into_iter()
                .collect();
                let return_t =
                    self.instantiate_typespec(&subr.return_t, opt_decl_t, tmp_tv_ctx, mode)?;
                Ok(subr_t(
                    SubrKind::from(subr.arrow.kind),
                    non_defaults,
                    var_args,
                    defaults,
                    return_t,
                ))
            }
            TypeSpec::TypeApp { spec, args } => {
                todo!("{spec}{args}")
            }
        }
    }

    pub(crate) fn instantiate_ty_bound(
        &self,
        bound: &TypeBoundSpec,
        mode: RegistrationMode,
    ) -> TyCheckResult<TyBound> {
        // REVIEW: 型境界の左辺に来れるのは型変数だけか?
        // TODO: 高階型変数
        match bound {
            TypeBoundSpec::NonDefault { lhs, spec } => {
                let bound = match spec.op.kind {
                    TokenKind::SubtypeOf => TyBound::subtype_of(
                        mono_q(lhs.inspect().clone()),
                        self.instantiate_typespec(&spec.t_spec, None, None, mode)?,
                    ),
                    TokenKind::SupertypeOf => todo!(),
                    TokenKind::Colon => TyBound::instance(
                        lhs.inspect().clone(),
                        self.instantiate_typespec(&spec.t_spec, None, None, mode)?,
                    ),
                    _ => unreachable!(),
                };
                Ok(bound)
            }
            TypeBoundSpec::WithDefault { .. } => todo!(),
        }
    }

    pub(crate) fn instantiate_ty_bounds(
        &self,
        bounds: &TypeBoundSpecs,
        mode: RegistrationMode,
    ) -> TyCheckResult<Set<TyBound>> {
        let mut new_bounds = set! {};
        for bound in bounds.iter() {
            new_bounds.insert(self.instantiate_ty_bound(bound, mode)?);
        }
        Ok(new_bounds)
    }

    fn instantiate_tp(
        &self,
        quantified: TyParam,
        tmp_tv_ctx: &TyVarInstContext,
        loc: Location,
    ) -> TyCheckResult<TyParam> {
        match quantified {
            TyParam::MonoQVar(n) => {
                if let Some(tp) = tmp_tv_ctx.get_typaram(&n) {
                    Ok(tp.clone())
                } else if let Some(t) = tmp_tv_ctx.get_tyvar(&n) {
                    Ok(TyParam::t(t.clone()))
                } else if let Some(tv_ctx) = &self.tv_ctx {
                    tv_ctx.get_qtp(TyParam::MonoQVar(n.clone())).ok_or_else(|| {
                        TyCheckErrors::from(TyCheckError::tyvar_not_defined_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            &n,
                            loc,
                            AtomicStr::ever("?"),
                        ))
                    })
                } else if let Some(outer) = &self.outer {
                    outer.instantiate_tp(TyParam::MonoQVar(n), tmp_tv_ctx, loc)
                } else {
                    Err(TyCheckErrors::from(TyCheckError::tyvar_not_defined_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &n,
                        loc,
                        AtomicStr::ever("?"),
                    )))
                }
            }
            TyParam::UnaryOp { op, val } => {
                let res = self.instantiate_tp(*val, tmp_tv_ctx, loc)?;
                Ok(TyParam::unary(op, res))
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let lhs = self.instantiate_tp(*lhs, tmp_tv_ctx, loc)?;
                let rhs = self.instantiate_tp(*rhs, tmp_tv_ctx, loc)?;
                Ok(TyParam::bin(op, lhs, rhs))
            }
            TyParam::Type(t) => {
                // Int
                /*if t.is_monomorphic() {
                    Ok(TyParam::Type(t))
                }*/
                // 'T -> ?T
                if t.is_mono_q() {
                    let t = self.instantiate_t(*t, tmp_tv_ctx, loc)?;
                    Ok(TyParam::t(t))
                }
                // K('U) -> K(?U)
                else {
                    let ctx = self.get_nominal_type_ctx(&t).unwrap();
                    let tv_ctx = TyVarInstContext::new(self.level, ctx.bounds(), self);
                    let t = self.instantiate_t(*t, &tv_ctx, loc)?;
                    Ok(TyParam::t(t))
                }
            }
            TyParam::FreeVar(fv) if fv.is_linked() => {
                self.instantiate_tp(fv.crack().clone(), tmp_tv_ctx, loc)
            }
            p @ (TyParam::Value(_)
            | TyParam::Mono(_)
            | TyParam::FreeVar(_)
            | TyParam::Erased(_)) => Ok(p),
            other => todo!("{other}"),
        }
    }

    /// 'T -> ?T (quantified to free)
    pub(crate) fn instantiate_t(
        &self,
        unbound: Type,
        tmp_tv_ctx: &TyVarInstContext,
        loc: Location,
    ) -> TyCheckResult<Type> {
        match unbound {
            MonoQVar(n) => {
                if let Some(t) = tmp_tv_ctx.get_tyvar(&n) {
                    Ok(t.clone())
                } else if let Some(tp) = tmp_tv_ctx.get_typaram(&n) {
                    if let TyParam::Type(t) = tp {
                        Ok(*t.clone())
                    } else {
                        todo!(
                            "typaram_insts: {}\ntyvar_insts:{}\n{tp}",
                            tmp_tv_ctx.typaram_instances,
                            tmp_tv_ctx.tyvar_instances,
                        )
                    }
                } else if let Some(tv_ctx) = &self.tv_ctx {
                    tv_ctx.get_qvar(MonoQVar(n.clone())).ok_or_else(|| {
                        TyCheckErrors::from(TyCheckError::tyvar_not_defined_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            &n,
                            loc,
                            AtomicStr::ever("?"),
                        ))
                    })
                } else {
                    Err(TyCheckErrors::from(TyCheckError::tyvar_not_defined_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &n,
                        loc,
                        AtomicStr::ever("?"),
                    )))
                }
            }
            PolyQVar { name, mut params } => {
                for param in params.iter_mut() {
                    *param = self.instantiate_tp(mem::take(param), tmp_tv_ctx, loc)?;
                }
                Ok(poly_q(name, params))
            }
            Refinement(mut refine) => {
                refine.t = Box::new(self.instantiate_t(*refine.t, tmp_tv_ctx, loc)?);
                let mut new_preds = set! {};
                for mut pred in refine.preds.into_iter() {
                    for tp in pred.typarams_mut() {
                        *tp = self.instantiate_tp(mem::take(tp), tmp_tv_ctx, loc)?;
                    }
                    new_preds.insert(pred);
                }
                refine.preds = new_preds;
                Ok(Type::Refinement(refine))
            }
            Subr(mut subr) => {
                for pt in subr.non_default_params.iter_mut() {
                    *pt.typ_mut() = self.instantiate_t(mem::take(pt.typ_mut()), tmp_tv_ctx, loc)?;
                }
                if let Some(var_args) = subr.var_params.as_mut() {
                    *var_args.typ_mut() =
                        self.instantiate_t(mem::take(var_args.typ_mut()), tmp_tv_ctx, loc)?;
                }
                for pt in subr.default_params.iter_mut() {
                    *pt.typ_mut() = self.instantiate_t(mem::take(pt.typ_mut()), tmp_tv_ctx, loc)?;
                }
                let return_t = self.instantiate_t(*subr.return_t, tmp_tv_ctx, loc)?;
                let res = subr_t(
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|p| *p),
                    subr.default_params,
                    return_t,
                );
                Ok(res)
            }
            Record(mut dict) => {
                for v in dict.values_mut() {
                    *v = self.instantiate_t(mem::take(v), tmp_tv_ctx, loc)?;
                }
                Ok(Type::Record(dict))
            }
            Ref(t) => {
                let t = self.instantiate_t(*t, tmp_tv_ctx, loc)?;
                Ok(ref_(t))
            }
            RefMut { before, after } => {
                let before = self.instantiate_t(*before, tmp_tv_ctx, loc)?;
                let after = after
                    .map(|aft| self.instantiate_t(*aft, tmp_tv_ctx, loc))
                    .transpose()?;
                Ok(ref_mut(before, after))
            }
            Proj { lhs, rhs } => {
                let lhs = self.instantiate_t(*lhs, tmp_tv_ctx, loc)?;
                Ok(proj(lhs, rhs))
            }
            ProjCall {
                lhs,
                attr_name,
                mut args,
            } => {
                let lhs = self.instantiate_tp(*lhs, tmp_tv_ctx, loc)?;
                for arg in args.iter_mut() {
                    *arg = self.instantiate_tp(mem::take(arg), tmp_tv_ctx, loc)?;
                }
                Ok(proj_call(lhs, attr_name, args))
            }
            Poly { name, mut params } => {
                for param in params.iter_mut() {
                    *param = self.instantiate_tp(mem::take(param), tmp_tv_ctx, loc)?;
                }
                Ok(poly(name, params))
            }
            Quantified(_) => {
                panic!("a quantified type should not be instantiated, instantiate the inner type")
            }
            FreeVar(fv) if fv.is_linked() => {
                self.instantiate_t(fv.crack().clone(), tmp_tv_ctx, loc)
            }
            FreeVar(fv) => {
                let (sub, sup) = fv.get_bound_types().unwrap();
                let sub = self.instantiate_t(sub, tmp_tv_ctx, loc)?;
                let sup = self.instantiate_t(sup, tmp_tv_ctx, loc)?;
                let new_constraint = Constraint::new_sandwiched(sub, sup, fv.cyclicity());
                fv.update_constraint(new_constraint);
                Ok(FreeVar(fv))
            }
            And(l, r) => {
                let l = self.instantiate_t(*l, tmp_tv_ctx, loc)?;
                let r = self.instantiate_t(*r, tmp_tv_ctx, loc)?;
                Ok(self.intersection(&l, &r))
            }
            Or(l, r) => {
                let l = self.instantiate_t(*l, tmp_tv_ctx, loc)?;
                let r = self.instantiate_t(*r, tmp_tv_ctx, loc)?;
                Ok(self.union(&l, &r))
            }
            Not(l, r) => {
                let l = self.instantiate_t(*l, tmp_tv_ctx, loc)?;
                let r = self.instantiate_t(*r, tmp_tv_ctx, loc)?;
                Ok(not(l, r))
            }
            other if other.is_monomorphic() => Ok(other),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn instantiate(&self, quantified: Type, callee: &hir::Expr) -> TyCheckResult<Type> {
        match quantified {
            Quantified(quant) => {
                let tmp_tv_ctx = TyVarInstContext::new(self.level, quant.bounds, self);
                let t = self.instantiate_t(*quant.unbound_callable, &tmp_tv_ctx, callee.loc())?;
                match &t {
                    Type::Subr(subr) => {
                        if let Some(self_t) = subr.self_t() {
                            self.sub_unify(
                                callee.ref_t(),
                                self_t,
                                callee.loc(),
                                Some(&Str::ever("self")),
                            )?;
                        }
                    }
                    _ => unreachable!(),
                }
                if cfg!(feature = "debug") && t.has_qvar() {
                    panic!("{t} has qvar")
                }
                Ok(t)
            }
            // HACK: {op: |T|(T -> T) | op == F} => ?T -> ?T
            Refinement(refine) if refine.t.is_quantified() => {
                let quant = enum_unwrap!(*refine.t, Type::Quantified);
                let tmp_tv_ctx = TyVarInstContext::new(self.level, quant.bounds, self);
                let t = self.instantiate_t(*quant.unbound_callable, &tmp_tv_ctx, callee.loc())?;
                match &t {
                    Type::Subr(subr) => {
                        if let Some(self_t) = subr.self_t() {
                            self.sub_unify(
                                callee.ref_t(),
                                self_t,
                                callee.loc(),
                                Some(&Str::ever("self")),
                            )?;
                        }
                    }
                    _ => unreachable!(),
                }
                Ok(t)
            }
            // rank-1制限により、通常の型(rank-0型)の内側に量化型は存在しない
            other => Ok(other),
        }
    }
}
