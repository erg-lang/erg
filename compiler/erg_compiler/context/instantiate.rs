use std::fmt;
use std::mem;
use std::option::Option; // conflicting to Type::Option

use erg_common::dict::Dict;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{assume_unreachable, enum_unwrap, set, try_map};

use ast::{
    ParamSignature, ParamTySpec, PreDeclTypeSpec, SimpleTypeSpec, TypeBoundSpec, TypeBoundSpecs,
    TypeSpec,
};
use erg_parser::ast;
use erg_parser::token::TokenKind;

use erg_type::constructors::*;
use erg_type::free::{Constraint, Cyclicity, FreeTyVar};
use erg_type::typaram::{IntervalOp, TyParam, TyParamOrdering};
use erg_type::value::ValueObj;
use erg_type::{HasType, ParamTy, Predicate, SubrKind, TyBound, Type};
use TyParamOrdering::*;
use Type::*;

use crate::context::eval::eval_lit;
use crate::context::{Context, RegistrationMode};
use crate::error::TyCheckResult;
use crate::hir;
use RegistrationMode::*;

/// Context for instantiating a quantified type
/// 量化型をインスタンス化するための文脈
#[derive(Debug, Clone)]
pub struct TyVarContext {
    level: usize,
    pub(crate) tyvar_instances: Dict<Str, Type>,
    pub(crate) typaram_instances: Dict<Str, TyParam>,
}

impl fmt::Display for TyVarContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "TyVarContext {{ tyvar_instances: {}, typaram_instances: {} }}",
            self.tyvar_instances, self.typaram_instances,
        )
    }
}

impl TyVarContext {
    pub fn new(level: usize, bounds: Set<TyBound>, ctx: &Context) -> Self {
        let mut self_ = Self {
            level,
            tyvar_instances: Dict::new(),
            typaram_instances: Dict::new(),
        };
        // TODO: this is valid but cause a crash: T <: Ord T
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
                    if &t.typ().name()[..] == "Self" {
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
            let (_, ctx) = ctx
                .rec_get_nominal_type_ctx(&poly(name.clone(), params.clone()))
                .unwrap_or_else(|| panic!("{} not found", name));
            let defined_params_len = ctx.params.len();
            let given_params_len = params.len();
            if defined_params_len < given_params_len {
                panic!()
            }
            let inst_non_defaults = params
                .into_iter()
                .map(|tp| {
                    let name = tp.tvar_name().unwrap();
                    let tp = self.instantiate_qtp(tp);
                    self.push_or_init_typaram(&name, &tp);
                    tp
                })
                .collect();
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
            poly(
                name,
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
                    .collect(),
            )
        }
    }

    fn instantiate_bound(&mut self, bound: TyBound, ctx: &Context) {
        match bound {
            TyBound::Sandwiched { sub, mid, sup } => {
                let sub_instance = match sub {
                    Type::Poly { name, params } => {
                        self.instantiate_poly(mid.name(), &name, params, ctx)
                    }
                    Type::MonoProj { lhs, rhs } => mono_proj(self.instantiate_qvar(*lhs), rhs),
                    sub => sub,
                };
                let sup_instance = match sup {
                    Type::Poly { name, params } => {
                        self.instantiate_poly(mid.name(), &name, params, ctx)
                    }
                    Type::MonoProj { lhs, rhs } => mono_proj(self.instantiate_qvar(*lhs), rhs),
                    sup => sup,
                };
                let name = mid.name();
                let constraint =
                    Constraint::new_sandwiched(sub_instance, sup_instance, Cyclicity::Not);
                self.push_or_init_tyvar(
                    &name,
                    &named_free_var(name.clone(), self.level, constraint),
                );
            }
            TyBound::Instance { name, t } => {
                let t = match t {
                    Type::Poly { name, params } => {
                        self.instantiate_poly(name.clone(), &name, params, ctx)
                    }
                    t => t,
                };
                let constraint = Constraint::new_type_of(t.clone());
                // TODO: type-like types
                if t == Type {
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

    pub(crate) fn instantiate_qvar(&mut self, quantified: Type) -> Type {
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
                if let Type::MonoQVar(n) = *t {
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
                    todo!("{t}")
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
        if self.tyvar_instances.get(name).is_some() || self.typaram_instances.get(name).is_some() {
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
                    todo!()
                }
            })
        })
    }

    pub(crate) fn get_typaram(&self, name: &str) -> Option<&TyParam> {
        self.typaram_instances.get(name)
    }
}

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
            self.instantiate_typespec(s, mode)?
        } else {
            free_var(self.level, Constraint::new_type_of(Type))
        };
        if let Some(eval_t) = opt_eval_t {
            self.sub_unify(&eval_t, &spec_t, None, t_spec.map(|s| s.loc()), None)?;
        }
        Ok(spec_t)
    }

    pub(crate) fn instantiate_sub_sig_t(
        &self,
        sig: &ast::SubrSignature,
        eval_ret_t: Option<Type>,
        mode: RegistrationMode,
    ) -> TyCheckResult<Type> {
        let non_defaults = sig
            .params
            .non_defaults
            .iter()
            .map(|p| {
                ParamTy::pos(
                    p.inspect().cloned(),
                    self.instantiate_param_sig_t(p, None, mode).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let var_args = if let Some(var_args) = sig.params.var_args.as_ref() {
            let va_t = self.instantiate_param_sig_t(var_args, None, mode)?;
            Some(ParamTy::pos(var_args.inspect().cloned(), va_t))
        } else {
            None
        };
        let defaults = sig
            .params
            .defaults
            .iter()
            .map(|p| {
                ParamTy::kw(
                    p.inspect().unwrap().clone(),
                    self.instantiate_param_sig_t(p, None, mode).unwrap(),
                )
            })
            .collect();
        let spec_return_t = if let Some(s) = sig.return_t_spec.as_ref() {
            self.instantiate_typespec(s, mode)?
        } else {
            // preregisterならouter scopeで型宣言(see inference.md)
            let level = if mode == PreRegister {
                self.level
            } else {
                self.level + 1
            };
            free_var(level, Constraint::new_type_of(Type))
        };
        if let Some(eval_ret_t) = eval_ret_t {
            self.sub_unify(
                &eval_ret_t,
                &spec_return_t,
                None,
                sig.return_t_spec.as_ref().map(|s| s.loc()),
                None,
            )?;
        }
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
        mode: RegistrationMode,
    ) -> TyCheckResult<Type> {
        let spec_t = if let Some(spec) = &sig.t_spec {
            self.instantiate_typespec(spec, mode)?
        } else {
            match &sig.pat {
                ast::ParamPattern::Lit(lit) => enum_t(set![eval_lit(lit)]),
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
                None,
                sig.t_spec.as_ref().map(|s| s.loc()),
                None,
            )?;
        }
        Ok(spec_t)
    }

    pub(crate) fn instantiate_predecl_t(&self, _predecl: &PreDeclTypeSpec) -> TyCheckResult<Type> {
        match _predecl {
            ast::PreDeclTypeSpec::Simple(simple) => self.instantiate_simple_t(simple),
            _ => todo!(),
        }
    }

    pub(crate) fn instantiate_simple_t(&self, simple: &SimpleTypeSpec) -> TyCheckResult<Type> {
        match &simple.name.inspect()[..] {
            "Nat" => Ok(Type::Nat),
            "Int" => Ok(Type::Int),
            "Ratio" => Ok(Type::Ratio),
            "Float" => Ok(Type::Float),
            "Str" => Ok(Type::Str),
            "Bool" => Ok(Type::Bool),
            "None" => Ok(Type::NoneType),
            "Ellipsis" => Ok(Type::Ellipsis),
            "NotImplemented" => Ok(Type::NotImplemented),
            "Inf" => Ok(Type::Inf),
            "Obj" => Ok(Type::Obj),
            "Array" => {
                // TODO: kw
                let mut args = simple.args.pos_args();
                if let Some(first) = args.next() {
                    let t = self.instantiate_const_expr_as_type(&first.expr)?;
                    let len = args.next().unwrap();
                    let len = self.instantiate_const_expr(&len.expr);
                    Ok(array(t, len))
                } else {
                    Ok(mono("GenericArray"))
                }
            }
            other if simple.args.is_empty() => Ok(mono(Str::rc(other))),
            other => {
                // FIXME: kw args
                let params = simple.args.pos_args().map(|arg| match &arg.expr {
                    ast::ConstExpr::Lit(lit) => TyParam::Value(eval_lit(lit)),
                    _ => {
                        todo!()
                    }
                });
                // FIXME: if type is a trait
                Ok(poly(Str::rc(other), params.collect()))
            }
        }
    }

    pub(crate) fn instantiate_const_expr(&self, expr: &ast::ConstExpr) -> TyParam {
        match expr {
            ast::ConstExpr::Lit(lit) => TyParam::Value(eval_lit(lit)),
            ast::ConstExpr::Accessor(ast::ConstAccessor::Local(name)) => {
                TyParam::Mono(name.inspect().clone())
            }
            _ => todo!(),
        }
    }

    pub(crate) fn instantiate_const_expr_as_type(
        &self,
        expr: &ast::ConstExpr,
    ) -> TyCheckResult<Type> {
        match expr {
            ast::ConstExpr::Accessor(ast::ConstAccessor::Local(name)) => Ok(mono(name.inspect())),
            _ => todo!(),
        }
    }

    fn instantiate_func_param_spec(
        &self,
        p: &ParamTySpec,
        mode: RegistrationMode,
    ) -> TyCheckResult<ParamTy> {
        let t = self.instantiate_typespec(&p.ty, mode)?;
        Ok(ParamTy::pos(
            p.name.as_ref().map(|t| t.inspect().to_owned()),
            t,
        ))
    }

    pub(crate) fn instantiate_typespec(
        &self,
        spec: &TypeSpec,
        mode: RegistrationMode,
    ) -> TyCheckResult<Type> {
        match spec {
            TypeSpec::PreDeclTy(predecl) => self.instantiate_predecl_t(predecl),
            // TODO: Flatten
            TypeSpec::And(lhs, rhs) => Ok(and(
                self.instantiate_typespec(lhs, mode)?,
                self.instantiate_typespec(rhs, mode)?,
            )),
            TypeSpec::Not(lhs, rhs) => Ok(not(
                self.instantiate_typespec(lhs, mode)?,
                self.instantiate_typespec(rhs, mode)?,
            )),
            TypeSpec::Or(lhs, rhs) => Ok(or(
                self.instantiate_typespec(lhs, mode)?,
                self.instantiate_typespec(rhs, mode)?,
            )),
            TypeSpec::Array { .. } => todo!(),
            // FIXME: unwrap
            TypeSpec::Tuple(tys) => Ok(tuple(
                tys.iter()
                    .map(|spec| self.instantiate_typespec(spec, mode).unwrap())
                    .collect(),
            )),
            // TODO: エラー処理(リテラルでない、ダブりがある)はパーサーにやらせる
            TypeSpec::Enum(set) => Ok(enum_t(
                set.pos_args()
                    .map(|arg| {
                        if let ast::ConstExpr::Lit(lit) = &arg.expr {
                            eval_lit(lit)
                        } else {
                            todo!()
                        }
                    })
                    .collect::<Set<_>>(),
            )),
            TypeSpec::Interval { op, lhs, rhs } => {
                let op = match op.kind {
                    TokenKind::Closed => IntervalOp::Closed,
                    TokenKind::LeftOpen => IntervalOp::LeftOpen,
                    TokenKind::RightOpen => IntervalOp::RightOpen,
                    TokenKind::Open => IntervalOp::Open,
                    _ => assume_unreachable!(),
                };
                let l = self.instantiate_const_expr(lhs);
                let l = self.eval_tp(&l)?;
                let r = self.instantiate_const_expr(rhs);
                let r = self.eval_tp(&r)?;
                if let Some(Greater) = self.rec_try_cmp(&l, &r) {
                    panic!("{l}..{r} is not a valid interval type (should be lhs <= rhs)")
                }
                Ok(int_interval(op, l, r))
            }
            TypeSpec::Subr(subr) => {
                let non_defaults = try_map(subr.non_defaults.iter(), |p| {
                    self.instantiate_func_param_spec(p, mode)
                })?;
                let var_args = subr
                    .var_args
                    .as_ref()
                    .map(|p| self.instantiate_func_param_spec(p, mode))
                    .transpose()?;
                let defaults = try_map(subr.defaults.iter(), |p| {
                    self.instantiate_func_param_spec(p, mode)
                })?
                .into_iter()
                .collect();
                let return_t = self.instantiate_typespec(&subr.return_t, mode)?;
                Ok(subr_t(
                    if subr.arrow.is(TokenKind::FuncArrow) {
                        SubrKind::Func
                    } else {
                        SubrKind::Proc
                    },
                    non_defaults,
                    var_args,
                    defaults,
                    return_t,
                ))
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
            TypeBoundSpec::Subtype { sub, sup } => Ok(TyBound::subtype_of(
                mono_q(sub.inspect().clone()),
                self.instantiate_typespec(sup, mode)?,
            )),
            TypeBoundSpec::Instance { name, ty } => Ok(TyBound::instance(
                name.inspect().clone(),
                self.instantiate_typespec(ty, mode)?,
            )),
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

    fn instantiate_tp(quantified: TyParam, tv_ctx: &mut TyVarContext) -> TyParam {
        match quantified {
            TyParam::MonoQVar(n) => {
                if let Some(tp) = tv_ctx.get_typaram(&n) {
                    tp.clone()
                } else if let Some(t) = tv_ctx.get_tyvar(&n) {
                    TyParam::t(t.clone())
                } else {
                    panic!("type parameter {n} is not defined")
                }
            }
            TyParam::UnaryOp { op, val } => {
                let res = Self::instantiate_tp(*val, tv_ctx);
                TyParam::unary(op, res)
            }
            TyParam::BinOp { op, lhs, rhs } => {
                let lhs = Self::instantiate_tp(*lhs, tv_ctx);
                let rhs = Self::instantiate_tp(*rhs, tv_ctx);
                TyParam::bin(op, lhs, rhs)
            }
            TyParam::Type(t) => {
                let t = Self::instantiate_t(*t, tv_ctx);
                TyParam::t(t)
            }
            p @ (TyParam::Value(_) | TyParam::Mono(_) | TyParam::FreeVar(_)) => p,
            other => todo!("{other}"),
        }
    }

    /// 'T -> ?T (quantified to free)
    pub(crate) fn instantiate_t(unbound: Type, tv_ctx: &mut TyVarContext) -> Type {
        match unbound {
            MonoQVar(n) => {
                if let Some(t) = tv_ctx.get_tyvar(&n) {
                    t.clone()
                } else if let Some(tp) = tv_ctx.get_typaram(&n) {
                    if let TyParam::Type(t) = tp {
                        *t.clone()
                    } else {
                        todo!(
                            "typaram_insts: {}\ntyvar_insts:{}\n{tp}",
                            tv_ctx.typaram_instances,
                            tv_ctx.tyvar_instances,
                        )
                    }
                } else {
                    panic!("the type variable {n} is not defined")
                }
            }
            PolyQVar { name, mut params } => {
                for param in params.iter_mut() {
                    *param = Self::instantiate_tp(mem::take(param), tv_ctx);
                }
                poly_q(name, params)
            }
            Refinement(mut refine) => {
                refine.preds = refine
                    .preds
                    .into_iter()
                    .map(|mut pred| {
                        for tp in pred.typarams_mut() {
                            *tp = Self::instantiate_tp(mem::take(tp), tv_ctx);
                        }
                        pred
                    })
                    .collect();
                Type::Refinement(refine)
            }
            Subr(mut subr) => {
                for pt in subr.non_default_params.iter_mut() {
                    *pt.typ_mut() = Self::instantiate_t(mem::take(pt.typ_mut()), tv_ctx);
                }
                if let Some(var_args) = subr.var_params.as_mut() {
                    *var_args.typ_mut() =
                        Self::instantiate_t(mem::take(var_args.typ_mut()), tv_ctx);
                }
                for pt in subr.default_params.iter_mut() {
                    *pt.typ_mut() = Self::instantiate_t(mem::take(pt.typ_mut()), tv_ctx);
                }
                let return_t = Self::instantiate_t(*subr.return_t, tv_ctx);
                subr_t(
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|p| *p),
                    subr.default_params,
                    return_t,
                )
            }
            Record(mut dict) => {
                for v in dict.values_mut() {
                    *v = Self::instantiate_t(mem::take(v), tv_ctx);
                }
                Type::Record(dict)
            }
            Ref(t) => {
                let t = Self::instantiate_t(*t, tv_ctx);
                ref_(t)
            }
            RefMut { before, after } => {
                let before = Self::instantiate_t(*before, tv_ctx);
                let after = after.map(|aft| Self::instantiate_t(*aft, tv_ctx));
                ref_mut(before, after)
            }
            MonoProj { lhs, rhs } => {
                let lhs = Self::instantiate_t(*lhs, tv_ctx);
                mono_proj(lhs, rhs)
            }
            Poly { name, mut params } => {
                for param in params.iter_mut() {
                    *param = Self::instantiate_tp(mem::take(param), tv_ctx);
                }
                poly(name, params)
            }
            Quantified(_) => {
                panic!("a quantified type should not be instantiated, instantiate the inner type")
            }
            other if other.is_monomorphic() => other,
            other => todo!("{other}"),
        }
    }

    pub(crate) fn instantiate(&self, quantified: Type, callee: &hir::Expr) -> TyCheckResult<Type> {
        match quantified {
            Quantified(quant) => {
                let mut tv_ctx = TyVarContext::new(self.level, quant.bounds, self);
                let t = Self::instantiate_t(*quant.unbound_callable, &mut tv_ctx);
                match &t {
                    Type::Subr(subr) => {
                        if let Some(self_t) = subr.self_t() {
                            self.sub_unify(
                                callee.ref_t(),
                                self_t,
                                None,
                                Some(callee.loc()),
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
