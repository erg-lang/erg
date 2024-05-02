use std::option::Option; // conflicting to Type::Option

use erg_common::levenshtein::get_similar_name;
#[allow(unused)]
use erg_common::log;
use erg_common::traits::{Locational, Stream};
use erg_common::{assume_unreachable, dict, failable_map_mut, fn_name, set, Str};

use ast::{
    NonDefaultParamSignature, ParamTySpec, PreDeclTypeSpec, TypeBoundSpec, TypeBoundSpecs, TypeSpec,
};
use erg_parser::ast::{
    self, ConstApp, ConstArgs, ConstExpr, ConstList, ConstSet, Identifier, VarName,
    VisModifierSpec, VisRestriction,
};
use erg_parser::token::TokenKind;
use erg_parser::Parser;

use crate::ty::free::{CanbeFree, Constraint, HasLevel};
use crate::ty::typaram::{IntervalOp, OpKind, TyParam, TyParamLambda, TyParamOrdering};
use crate::ty::value::ValueObj;
use crate::ty::{
    constructors::*, CastTarget, GuardType, Predicate, RefinementType, VisibilityModifier,
};
use crate::ty::{Field, HasType, ParamTy, SubrKind, SubrType, Type};
use crate::type_feature_error;
use crate::varinfo::{AbsLocation, VarInfo};
use TyParamOrdering::*;
use Type::*;

use crate::context::instantiate::TyVarCache;
use crate::context::{Context, DefaultInfo, RegistrationMode};
use crate::error::{Failable, TyCheckError, TyCheckErrors, TyCheckResult};
use crate::AccessKind;
use RegistrationMode::*;

pub fn token_kind_to_op_kind(kind: TokenKind) -> Option<OpKind> {
    match kind {
        TokenKind::Plus => Some(OpKind::Add),
        TokenKind::Minus => Some(OpKind::Sub),
        TokenKind::Star => Some(OpKind::Mul),
        TokenKind::Slash => Some(OpKind::Div),
        TokenKind::FloorDiv => Some(OpKind::FloorDiv),
        TokenKind::Mod => Some(OpKind::Mod),
        TokenKind::Pow => Some(OpKind::Pow),
        TokenKind::PrePlus => Some(OpKind::Pos),
        TokenKind::PreMinus => Some(OpKind::Neg),
        TokenKind::PreBitNot => Some(OpKind::Invert),
        TokenKind::DblEq => Some(OpKind::Eq),
        TokenKind::NotEq => Some(OpKind::Ne),
        TokenKind::Less => Some(OpKind::Lt),
        TokenKind::LessEq => Some(OpKind::Le),
        TokenKind::Gre => Some(OpKind::Gt),
        TokenKind::GreEq => Some(OpKind::Ge),
        TokenKind::AndOp => Some(OpKind::And),
        TokenKind::OrOp => Some(OpKind::Or),
        TokenKind::BitAnd => Some(OpKind::BitAnd),
        TokenKind::BitOr => Some(OpKind::BitOr),
        TokenKind::BitXor => Some(OpKind::BitXor),
        TokenKind::Shl => Some(OpKind::Shl),
        TokenKind::Shr => Some(OpKind::Shr),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamKind {
    NonDefault,
    Default(Type),
    VarParams,
    KwVarParams,
}

impl ParamKind {
    pub const fn is_var_params(&self) -> bool {
        matches!(self, ParamKind::VarParams)
    }
    pub const fn is_kw_var_params(&self) -> bool {
        matches!(self, ParamKind::KwVarParams)
    }
    pub const fn is_default(&self) -> bool {
        matches!(self, ParamKind::Default(_))
    }
    pub const fn default_info(&self) -> DefaultInfo {
        match self {
            ParamKind::Default(_) => DefaultInfo::WithDefault,
            _ => DefaultInfo::NonDefault,
        }
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
    pub(crate) fn instantiate_ty_bound(
        &self,
        bound: &TypeBoundSpec,
        tv_cache: &mut TyVarCache,
        mode: RegistrationMode,
    ) -> TyCheckResult<()> {
        // REVIEW: 型境界の左辺に来れるのは型変数だけか?
        // TODO: 高階型変数
        match bound {
            TypeBoundSpec::Omitted(name) => {
                // TODO: other than type `Type`
                let constr = Constraint::new_type_of(Type);
                let tv = named_free_var(name.inspect().clone(), self.level, constr);
                tv_cache.push_or_init_tyvar(name, &tv, self)
            }
            TypeBoundSpec::NonDefault { lhs, spec } => {
                let mut errs = TyCheckErrors::empty();
                let constr = match spec.op.kind {
                    TokenKind::SubtypeOf => {
                        let sup = match self.instantiate_typespec_full(
                            &spec.t_spec,
                            None,
                            tv_cache,
                            mode,
                            true,
                        ) {
                            Ok(sup) => sup,
                            Err((sup, es)) => {
                                errs.extend(es);
                                sup
                            }
                        };
                        Constraint::new_subtype_of(sup)
                    }
                    TokenKind::SupertypeOf => {
                        let sub = match self.instantiate_typespec_full(
                            &spec.t_spec,
                            None,
                            tv_cache,
                            mode,
                            true,
                        ) {
                            Ok(sub) => sub,
                            Err((sub, es)) => {
                                errs.extend(es);
                                sub
                            }
                        };
                        Constraint::new_supertype_of(sub)
                    }
                    TokenKind::Colon => {
                        let t = match self.instantiate_typespec_full(
                            &spec.t_spec,
                            None,
                            tv_cache,
                            mode,
                            true,
                        ) {
                            Ok(t) => t,
                            Err((t, es)) => {
                                errs.extend(es);
                                t
                            }
                        };
                        Constraint::new_type_of(t)
                    }
                    _ => unreachable!(),
                };
                if constr.get_sub_sup().is_none() {
                    let tp = TyParam::named_free_var(lhs.inspect().clone(), self.level, constr);
                    tv_cache.push_or_init_typaram(lhs, &tp, self)?;
                } else {
                    let tv = named_free_var(lhs.inspect().clone(), self.level, constr);
                    tv_cache.push_or_init_tyvar(lhs, &tv, self)?;
                }
                if errs.is_empty() {
                    Ok(())
                } else {
                    Err(errs)
                }
            }
            TypeBoundSpec::WithDefault { .. } => type_feature_error!(
                self,
                bound.loc(),
                "type boundary specification with default"
            ),
        }
    }

    pub(crate) fn instantiate_ty_bounds(
        &self,
        bounds: &TypeBoundSpecs,
        mode: RegistrationMode,
    ) -> Failable<TyVarCache> {
        let mut errs = TyCheckErrors::empty();
        let mut tv_cache = TyVarCache::new(self.level, self);
        for bound in bounds.iter() {
            if let Err(es) = self.instantiate_ty_bound(bound, &mut tv_cache, mode) {
                errs.extend(es);
            }
        }
        for tv in tv_cache.tyvar_instances.values() {
            if tv.constraint().map(|c| c.is_uninited()).unwrap_or(false) {
                errs.push(TyCheckError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    bounds.loc(),
                    self.caused_by(),
                    &tv.local_name(),
                    self.get_similar_name(&tv.local_name()),
                ));
            }
        }
        for tp in tv_cache.typaram_instances.values() {
            if tp.constraint().map(|c| c.is_uninited()).unwrap_or(false) {
                errs.push(TyCheckError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    bounds.loc(),
                    self.caused_by(),
                    &tp.to_string(),
                    self.get_similar_name(&tp.to_string()),
                ));
            }
        }
        if errs.is_empty() {
            Ok(tv_cache)
        } else {
            Err((tv_cache, errs))
        }
    }

    pub(crate) fn instantiate_var_sig_t(
        &self,
        t_spec: Option<&TypeSpec>,
        mode: RegistrationMode,
    ) -> Failable<Type> {
        let mut tmp_tv_cache = TyVarCache::new(self.level, self);
        let spec_t = if let Some(t_spec) = t_spec {
            self.instantiate_typespec_full(t_spec, None, &mut tmp_tv_cache, mode, false)?
        } else {
            free_var(self.level, Constraint::new_type_of(Type))
        };
        Ok(spec_t)
    }

    pub(crate) fn instantiate_sub_sig_t(
        &self,
        sig: &ast::SubrSignature,
        mode: RegistrationMode,
    ) -> Failable<Type> {
        let mut errs = TyCheckErrors::empty();
        // -> Result<Type, (Type, TyCheckErrors)> {
        let opt_decl_sig_t = match self
            .rec_get_decl_info(&sig.ident, AccessKind::Name, &self.cfg.input, self)
            .ok()
            .map(|vi| vi.t)
        {
            Some(Type::Subr(subr)) => Some(subr),
            Some(Type::FreeVar(fv)) if fv.is_unbound() => return Ok(Type::FreeVar(fv)),
            Some(other) => {
                let err = TyCheckError::unreachable(self.cfg.input.clone(), fn_name!(), line!());
                return Err((other, TyCheckErrors::from(err)));
            }
            None => None,
        };
        let mut tmp_tv_cache = match self.instantiate_ty_bounds(&sig.bounds, PreRegister) {
            Ok(tv_cache) => tv_cache,
            Err((tv_cache, es)) => {
                errs.extend(es);
                tv_cache
            }
        };
        let mut non_defaults = vec![];
        for (n, param) in sig.params.non_defaults.iter().enumerate() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .and_then(|subr| subr.non_default_params.get(n));
            match self.instantiate_param_ty(
                param,
                opt_decl_t,
                &mut tmp_tv_cache,
                mode,
                ParamKind::NonDefault,
                false,
            ) {
                Ok(pt) => non_defaults.push(pt),
                Err((pt, es)) => {
                    errs.extend(es);
                    non_defaults.push(pt);
                }
            }
        }
        let var_params = if let Some(var_args) = sig.params.var_params.as_ref() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .and_then(|subr| subr.var_params.as_ref().map(|v| v.as_ref()));
            let pt = match self.instantiate_param_ty(
                var_args,
                opt_decl_t,
                &mut tmp_tv_cache,
                mode,
                ParamKind::VarParams,
                false,
            ) {
                Ok(pt) => pt,
                Err((pt, es)) => {
                    errs.extend(es);
                    pt
                }
            };
            Some(pt)
        } else {
            None
        };
        let mut defaults = vec![];
        for (n, p) in sig.params.defaults.iter().enumerate() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .and_then(|subr| subr.default_params.get(n));
            // NOTE: We constrain this type variable later (in `ASTLowerer::lower_params`).
            let default_t = free_var(self.level, Constraint::new_type_of(Type::Type));
            match self.instantiate_param_ty(
                &p.sig,
                opt_decl_t,
                &mut tmp_tv_cache,
                mode,
                ParamKind::Default(default_t),
                false,
            ) {
                Ok(pt) => defaults.push(pt),
                Err((pt, es)) => {
                    errs.extend(es);
                    defaults.push(pt);
                }
            }
        }
        let kw_var_params = if let Some(kw_var_args) = sig.params.kw_var_params.as_ref() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .and_then(|subr| subr.kw_var_params.as_ref().map(|v| v.as_ref()));
            let pt = match self.instantiate_param_ty(
                kw_var_args,
                opt_decl_t,
                &mut tmp_tv_cache,
                mode,
                ParamKind::KwVarParams,
                false,
            ) {
                Ok(pt) => pt,
                Err((pt, es)) => {
                    errs.extend(es);
                    pt
                }
            };
            Some(pt)
        } else {
            None
        };
        let spec_return_t = if let Some(t_spec) = sig.return_t_spec.as_ref() {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .map(|subr| ParamTy::Pos(subr.return_t.as_ref().clone()));
            match self.instantiate_typespec_full(
                &t_spec.t_spec,
                opt_decl_t.as_ref(),
                &mut tmp_tv_cache,
                mode,
                false,
            ) {
                Ok(ty) => {
                    let params = non_defaults
                        .iter()
                        .chain(&var_params)
                        .chain(&defaults)
                        .chain(&kw_var_params)
                        .filter_map(|pt| pt.name());
                    self.recover_guard(ty, params)
                }
                Err((ty, es)) => {
                    errs.extend(es);
                    ty
                }
            }
        } else {
            // preregisterならouter scopeで型宣言(see inference.md)
            let level = if mode.is_preregister() {
                self.level
            } else {
                self.level + 1
            };
            free_var(level, Constraint::new_type_of(Type))
        };
        // tmp_tv_cache.warn_isolated_vars(self);
        let typ = if sig.ident.is_procedural() {
            proc(
                non_defaults,
                var_params,
                defaults,
                kw_var_params,
                spec_return_t,
            )
        } else {
            func(
                non_defaults,
                var_params,
                defaults,
                kw_var_params,
                spec_return_t,
            )
        };
        if errs.is_empty() {
            Ok(typ)
        } else {
            Err((typ, errs))
        }
    }

    /// spec_t == Noneかつリテラル推論が不可能なら型変数を発行する
    pub(crate) fn instantiate_param_sig_t(
        &self,
        sig: &NonDefaultParamSignature,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_cache: &mut TyVarCache,
        mode: RegistrationMode,
        kind: ParamKind,
        not_found_is_qvar: bool,
    ) -> Failable<Type> {
        let mut errs = TyCheckErrors::empty();
        let gen_free_t = || {
            let level = if mode.is_preregister() {
                self.level
            } else {
                self.level + 1
            };
            free_var(level, Constraint::new_type_of(Type))
        };
        let spec_t = if let Some(spec_with_op) = &sig.t_spec {
            match self.instantiate_typespec_full(
                &spec_with_op.t_spec,
                opt_decl_t,
                tmp_tv_cache,
                mode,
                not_found_is_qvar,
            ) {
                Ok(t) => t,
                Err((t, es)) => {
                    // prevent double error reporting
                    if mode.is_normal() {
                        errs.extend(es);
                    }
                    t
                }
            }
        } else {
            match &sig.pat {
                ast::ParamPattern::Lit(lit) => {
                    let lit = self.eval_lit(lit).map_err(|errs| (Type::Failure, errs))?;
                    v_enum(set![lit])
                }
                ast::ParamPattern::Discard(_) => Type::Obj,
                ast::ParamPattern::Ref(_) => ref_(gen_free_t()),
                ast::ParamPattern::RefMut(_) => ref_mut(gen_free_t(), None),
                // ast::ParamPattern::VarName(name) if &name.inspect()[..] == "_" => Type::Obj,
                // TODO: List<Lit>
                _ => gen_free_t(),
            }
        };
        if let Some(decl_pt) = opt_decl_t {
            if kind.is_var_params() {
                let spec_t = unknown_len_list_t(spec_t.clone());
                if let Err(es) = self.sub_unify(
                    decl_pt.typ(),
                    &spec_t,
                    &sig.t_spec.as_ref().ok_or(sig),
                    None,
                ) {
                    return Err((spec_t, errs.concat(es)));
                }
            } else if let Err(es) = self.sub_unify(
                decl_pt.typ(),
                &spec_t,
                &sig.t_spec.as_ref().ok_or(sig),
                None,
            ) {
                return Err((spec_t, errs.concat(es)));
            }
        }
        if errs.is_empty() {
            Ok(spec_t)
        } else {
            Err((spec_t, errs))
        }
    }

    /// Given the type `T -> U`, if `T` is a known type, then this is a function type that takes `T` and returns `U`.
    /// If the type `T` is not defined, then `T` is considered a constant parameter.
    /// FIXME: The type bounds are processed regardless of the order in the specification, but in the current implementation, undefined type may be considered a constant parameter.
    pub(crate) fn instantiate_param_ty(
        &self,
        sig: &NonDefaultParamSignature,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_cache: &mut TyVarCache,
        mode: RegistrationMode,
        kind: ParamKind,
        not_found_is_qvar: bool,
    ) -> Failable<ParamTy> {
        if let Some(value) = sig
            .name()
            .and_then(|name| self.get_const_local(name.token(), &self.name).ok())
        {
            return Ok(ParamTy::Pos(v_enum(set! { value })));
        } else if let Some((tp, _vi)) = sig
            .name()
            .and_then(|name| self.get_tp_from_tv_cache(name.inspect(), tmp_tv_cache))
        {
            match tp {
                TyParam::Type(t) => return Ok(ParamTy::Pos(*t)),
                other => {
                    let (t, errs) = match self.get_tp_t(&other) {
                        Ok(t) => (t, TyCheckErrors::empty()),
                        Err(errs) => (Type::Failure, errs),
                    };
                    let pt = ParamTy::Pos(tp_enum(t, set! { other }));
                    if errs.is_empty() {
                        return Ok(pt);
                    } else {
                        return Err((pt, errs));
                    }
                }
            }
        }
        let (t, errs) = match self.instantiate_param_sig_t(
            sig,
            opt_decl_t,
            tmp_tv_cache,
            mode,
            kind.clone(),
            not_found_is_qvar,
        ) {
            Ok(t) => (t, TyCheckErrors::empty()),
            Err((t, errs)) => (t, errs),
        };
        let pt = match (sig.inspect(), kind) {
            (Some(name), ParamKind::Default(default_t)) => {
                ParamTy::kw_default(name.clone(), t, default_t)
            }
            (Some(name), _) => ParamTy::kw(name.clone(), t),
            (None, _) => ParamTy::Pos(t),
        };
        if errs.is_empty() {
            Ok(pt)
        } else {
            Err((pt, errs))
        }
    }

    pub(crate) fn instantiate_predecl_t(
        &self,
        predecl: &PreDeclTypeSpec,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> Failable<Type> {
        self.inc_ref_predecl_typespec(predecl, self, tmp_tv_cache);
        match predecl {
            ast::PreDeclTypeSpec::Mono(simple) => self
                .instantiate_mono_t(simple, opt_decl_t, tmp_tv_cache, not_found_is_qvar)
                .map_err(|errs| (Type::Failure, errs)),
            ast::PreDeclTypeSpec::Poly(poly) => match &poly.acc {
                ast::ConstAccessor::Local(local) => self.instantiate_local_poly_t(
                    local,
                    &poly.args,
                    opt_decl_t,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ),
                ast::ConstAccessor::Attr(attr) => {
                    let ctxs = self
                        .get_singular_ctxs(&attr.obj.clone().downgrade(), self)
                        .map_err(|errs| (Type::Failure, errs.into()))?;
                    for ctx in ctxs {
                        if let Ok(typ) = ctx.instantiate_local_poly_t(
                            &attr.name,
                            &poly.args,
                            opt_decl_t,
                            tmp_tv_cache,
                            not_found_is_qvar,
                        ) {
                            return Ok(typ);
                        }
                    }
                    Err((
                        Type::Failure,
                        TyCheckErrors::from(TyCheckError::no_var_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            attr.loc(),
                            self.caused_by(),
                            attr.name.inspect(),
                            self.get_similar_name(attr.name.inspect()),
                        )),
                    ))
                }
                _ => type_feature_error!(self, poly.loc(), &format!("instantiating type {poly}"))
                    .map_err(|errs| (Type::Failure, errs)),
            },
            ast::PreDeclTypeSpec::Attr { namespace, t } => {
                if let Ok(receiver) = Parser::validate_const_expr(namespace.as_ref().clone()) {
                    if let Ok(receiver_t) = self.instantiate_const_expr_as_type(
                        &receiver,
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        return self
                            .eval_proj(receiver_t, t.inspect().clone(), self.level, predecl)
                            .map_err(|errs| (Type::Failure, errs));
                    }
                }
                let ctxs = self
                    .get_singular_ctxs(namespace.as_ref(), self)
                    .map_err(|errs| (Type::Failure, errs.into()))?;
                for ctx in ctxs {
                    if let Some(ctx) = ctx.rec_local_get_type(t.inspect()) {
                        // TODO: visibility check
                        return Ok(ctx.typ.clone());
                    }
                }
                Err((
                    Type::Failure,
                    TyCheckErrors::from(TyCheckError::no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        t.loc(),
                        self.caused_by(),
                        t.inspect(),
                        self.get_similar_name(t.inspect()),
                    )),
                ))
            }
            other => type_feature_error!(self, other.loc(), &format!("instantiating type {other}"))
                .map_err(|errs| (Type::Failure, errs)),
        }
    }

    /// `opt_decl_pt` is a fallback, but it is used only if its type is not `Failure`.
    pub(crate) fn instantiate_mono_t(
        &self,
        ident: &Identifier,
        opt_decl_pt: Option<&ParamTy>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> TyCheckResult<Type> {
        match &ident.inspect()[..] {
            "_" | "Obj" => Ok(Type::Obj),
            "Nat" => Ok(Type::Nat),
            "Int" => Ok(Type::Int),
            "Ratio" => Ok(Type::Ratio),
            "Float" => Ok(Type::Float),
            "Str" => Ok(Type::Str),
            "Bool" => Ok(Type::Bool),
            "NoneType" => Ok(Type::NoneType),
            "Ellipsis" => Ok(Type::Ellipsis),
            "NotImplementedType" => Ok(Type::NotImplementedType),
            "Inf" => Ok(Type::Inf),
            "NegInf" => Ok(Type::NegInf),
            "Never" => Ok(Type::Never),
            "ClassType" => Ok(Type::ClassType),
            "TraitType" => Ok(Type::TraitType),
            "Type" => Ok(Type::Type),
            "Self" => self.rec_get_self_t().ok_or_else(|| {
                TyCheckErrors::from(TyCheckError::self_type_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    ident.loc(),
                    self.caused_by(),
                ))
            }),
            "True" | "False" | "None" => Err(TyCheckErrors::from(TyCheckError::not_a_type_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            ))),
            other => {
                if let Some((TyParam::Type(t), vi)) = self.get_tp_from_tv_cache(other, tmp_tv_cache)
                {
                    self.inc_ref(ident.inspect(), vi, ident, self);
                    return Ok(*t);
                }
                if let Some(typ) = self
                    .consts
                    .get(ident.inspect())
                    .and_then(|v| self.convert_value_into_type(v.clone()).ok())
                {
                    if let Some((_, vi)) = self.get_var_info(ident.inspect()) {
                        self.inc_ref(ident.inspect(), vi, ident, self);
                    }
                    return Ok(typ);
                }
                if let Some(outer) = &self.outer {
                    if let Ok(t) = outer.instantiate_mono_t(
                        ident,
                        opt_decl_pt,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        return Ok(t);
                    }
                }
                if let Some(ctx) = self.get_type_ctx(ident.inspect()) {
                    if let Some((_, vi)) = self.get_var_info(ident.inspect()) {
                        self.inc_ref(ident.inspect(), vi, ident, self);
                    }
                    Ok(ctx.typ.clone())
                } else if not_found_is_qvar {
                    let tyvar = named_free_var(Str::rc(other), self.level, Constraint::Uninited);
                    tmp_tv_cache.push_or_init_tyvar(&ident.name, &tyvar, self)?;
                    Ok(tyvar)
                } else if let Some(decl_t) =
                    opt_decl_pt.and_then(|decl| (!decl.typ().is_failure()).then_some(decl))
                {
                    Ok(decl_t.typ().clone())
                } else {
                    Err(TyCheckErrors::from(TyCheckError::no_type_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        ident.loc(),
                        self.caused_by(),
                        other,
                        self.get_similar_name(other),
                    )))
                }
            }
        }
    }

    fn instantiate_local_poly_t(
        &self,
        name: &Identifier,
        args: &ConstArgs,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> Failable<Type> {
        let mut errs = TyCheckErrors::empty();
        match name.inspect().trim_start_matches([':', '.']) {
            "List" => {
                let ctx = &self
                    .get_nominal_type_ctx(&list_t(Type::Obj, TyParam::Failure))
                    .unwrap()
                    .ctx;
                // TODO: kw
                let mut pos_args = args.pos_args();
                if let Some(first) = pos_args.next() {
                    let t = match self.instantiate_const_expr_as_type(
                        &first.expr,
                        Some((ctx, 0)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(t) => t,
                        Err((t, es)) => {
                            errs.extend(es);
                            t
                        }
                    };
                    let len = if let Some(len) = pos_args.next() {
                        match self.instantiate_const_expr(
                            &len.expr,
                            Some((ctx, 1)),
                            tmp_tv_cache,
                            not_found_is_qvar,
                        ) {
                            Ok(len) => len,
                            Err((len, es)) => {
                                errs.extend(es);
                                len
                            }
                        }
                    } else {
                        TyParam::erased(Nat)
                    };
                    if errs.is_empty() {
                        Ok(list_t(t, len))
                    } else {
                        Err((list_t(t, len), errs))
                    }
                } else {
                    Ok(mono("GenericList"))
                }
            }
            "Ref" => {
                let mut pos_args = args.pos_args();
                let Some(first) = pos_args.next() else {
                    return Err((
                        Failure,
                        TyCheckErrors::from(TyCheckError::args_missing_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            args.loc(),
                            "Ref",
                            self.caused_by(),
                            vec![Str::from("T")],
                        )),
                    ));
                };
                let t = match self.instantiate_const_expr_as_type(
                    &first.expr,
                    None,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                if errs.is_empty() {
                    Ok(ref_(t))
                } else {
                    Err((ref_(t), errs))
                }
            }
            "RefMut" => {
                // TODO after
                let mut pos_args = args.pos_args();
                let Some(first) = pos_args.next() else {
                    return Err((
                        Failure,
                        TyCheckErrors::from(TyCheckError::args_missing_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            args.loc(),
                            "RefMut",
                            self.caused_by(),
                            vec![Str::from("T")],
                        )),
                    ));
                };
                let t = match self.instantiate_const_expr_as_type(
                    &first.expr,
                    None,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                if errs.is_empty() {
                    Ok(ref_mut(t, None))
                } else {
                    Err((ref_mut(t, None), errs))
                }
            }
            "Structural" => {
                let mut pos_args = args.pos_args();
                let Some(first) = pos_args.next() else {
                    return Err((
                        Failure,
                        TyCheckErrors::from(TyCheckError::args_missing_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            args.loc(),
                            "Structural",
                            self.caused_by(),
                            vec![Str::from("Type")],
                        )),
                    ));
                };
                let t = match self.instantiate_const_expr_as_type(
                    &first.expr,
                    None,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                if errs.is_empty() {
                    Ok(t.structuralize())
                } else {
                    Err((t.structuralize(), errs))
                }
            }
            "NamedTuple" => {
                let mut pose_args = args.pos_args();
                let Some(first) = pose_args.next() else {
                    return Err((
                        Failure,
                        TyCheckErrors::from(TyCheckError::args_missing_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            args.loc(),
                            "NamedTuple",
                            self.caused_by(),
                            vec![Str::from("Fields")],
                        )),
                    ));
                };
                let ConstExpr::Record(fields) = &first.expr else {
                    return Err((
                        Failure,
                        TyCheckErrors::from(TyCheckError::type_mismatch_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            first.expr.loc(),
                            self.caused_by(),
                            "NamedTuple",
                            None,
                            &mono("Record"),
                            &self.instantiate_const_expr_as_type(
                                &first.expr,
                                None,
                                tmp_tv_cache,
                                not_found_is_qvar,
                            )?,
                            None,
                            None,
                        )),
                    ));
                };
                let mut ts = vec![];
                for def in fields.attrs.iter() {
                    let t = match self.instantiate_const_expr_as_type(
                        &def.body.block[0],
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(t) => t,
                        Err((t, es)) => {
                            errs.extend(es);
                            t
                        }
                    };
                    let vis = match self.instantiate_vis_modifier(&def.ident.vis) {
                        Ok(vis) => vis,
                        Err(es) => {
                            errs.extend(es);
                            VisibilityModifier::Public
                        }
                    };
                    ts.push((Field::new(vis, def.ident.inspect().clone()), t));
                }
                if errs.is_empty() {
                    Ok(Type::NamedTuple(ts))
                } else {
                    Err((Type::NamedTuple(ts), errs))
                }
            }
            other => {
                let Some(ctx) = self.get_type_ctx(other).or_else(|| {
                    self.consts
                        .get(other)
                        .and_then(|v| self.convert_value_into_type(v.clone()).ok())
                        .and_then(|typ| self.get_nominal_type_ctx(&typ))
                }) else {
                    if let Some(outer) = &self.outer {
                        if let Ok(t) = outer.instantiate_local_poly_t(
                            name,
                            args,
                            opt_decl_t,
                            tmp_tv_cache,
                            not_found_is_qvar,
                        ) {
                            return Ok(t);
                        }
                    }
                    if let Some(decl_t) = opt_decl_t {
                        return Ok(decl_t.typ().clone());
                    }
                    return Err((
                        Failure,
                        TyCheckErrors::from(TyCheckError::no_type_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            name.loc(),
                            self.caused_by(),
                            other,
                            self.get_similar_name(other),
                        )),
                    ));
                };
                let mut errs = TyCheckErrors::empty();
                let mut new_params = vec![];
                for ((i, arg), (name, param_vi)) in
                    args.pos_args().enumerate().zip(ctx.params.iter())
                {
                    match self.instantiate_arg(
                        &arg.expr,
                        param_vi,
                        name.as_ref(),
                        ctx,
                        i,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(tp) => new_params.push(tp),
                        Err((tp, es)) => {
                            errs.extend(es);
                            new_params.push(tp);
                        }
                    }
                }
                let mut missing_args = vec![];
                // Fill kw params
                for (_, param_vi) in ctx.params.iter().skip(args.pos_args.len()) {
                    new_params.push(TyParam::erased(param_vi.t.clone()));
                }
                let mut passed_kw_args = set! {};
                for (i, (name, param_vi)) in ctx.params.iter().skip(args.pos_args.len()).enumerate()
                {
                    if let Some(idx) = name.as_ref().and_then(|name| {
                        args.kw_args
                            .iter()
                            .position(|arg| arg.keyword.inspect() == name.inspect())
                    }) {
                        let kw_arg = &args.kw_args[idx];
                        let already_passed =
                            !passed_kw_args.insert(kw_arg.keyword.inspect().clone());
                        if already_passed {
                            errs.push(TyCheckError::multiple_args_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                kw_arg.loc(),
                                other,
                                self.caused_by(),
                                name.as_ref().map_or("_", |n| &n.inspect()[..]),
                            ));
                        }
                        let tp = match self.instantiate_arg(
                            &kw_arg.expr,
                            param_vi,
                            name.as_ref(),
                            ctx,
                            i,
                            tmp_tv_cache,
                            not_found_is_qvar,
                        ) {
                            Ok(tp) => tp,
                            Err((tp, es)) => {
                                errs.extend(es);
                                tp
                            }
                        };
                        if let Some(old) = new_params.get_mut(args.pos_args.len() + idx) {
                            *old = tp;
                        } else {
                            log!(err "{tp} / {} / {idx}", args.pos_args.len());
                            // TODO: too many kw args
                        }
                    } else if !param_vi.kind.has_default() {
                        missing_args
                            .push(name.as_ref().map_or("_".into(), |n| n.inspect().clone()));
                    }
                }
                if !missing_args.is_empty() {
                    errs.push(TyCheckError::args_missing_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        args.loc(),
                        other,
                        self.caused_by(),
                        missing_args,
                    ));
                }
                if ctx.params.len() < args.pos_args.len() + args.kw_args.len() {
                    errs.push(TyCheckError::too_many_args_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        args.loc(),
                        other,
                        self.caused_by(),
                        ctx.params.len(),
                        args.pos_args.len(),
                        args.kw_args.len(),
                    ));
                }
                let param_names = ctx
                    .params
                    .iter()
                    .filter_map(|(n, _)| n.as_ref().map(|n| &n.inspect()[..]))
                    .collect::<Vec<_>>();
                for unexpected in args
                    .kw_args
                    .iter()
                    .filter(|kw| !passed_kw_args.contains(&kw.keyword.inspect()[..]))
                {
                    let kw = unexpected.keyword.inspect();
                    errs.push(TyCheckError::unexpected_kw_arg_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        unexpected.loc(),
                        other,
                        self.caused_by(),
                        unexpected.keyword.inspect(),
                        get_similar_name(param_names.iter(), kw).copied(),
                    ));
                }
                // FIXME: non-builtin
                let t = poly(ctx.typ.qual_name(), new_params);
                if errs.is_empty() {
                    Ok(t)
                } else {
                    Err((t, errs))
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn instantiate_arg(
        &self,
        arg: &ConstExpr,
        param_vi: &VarInfo,
        name: Option<&VarName>,
        ctx: &Context,
        i: usize,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> Failable<TyParam> {
        let mut errs = TyCheckErrors::empty();
        let param =
            self.instantiate_const_expr(arg, Some((ctx, i)), tmp_tv_cache, not_found_is_qvar);
        let param = param.unwrap_or_else(|(tp, e)| {
            errs.extend(e);
            if not_found_is_qvar {
                let name = arg.to_string();
                // FIXME: handle `::` as a right way
                let name = Str::rc(name.trim_start_matches("::"));
                let tp = TyParam::named_free_var(name.clone(), self.level, Constraint::Uninited);
                let varname = VarName::from_str(name);
                if let Err(es) = tmp_tv_cache.push_or_init_typaram(&varname, &tp, self) {
                    errs.extend(es);
                }
                tp
            } else {
                tp
            }
        });
        let arg_t = self
            .get_tp_t(&param)
            .map_err(|err| {
                log!(err "{param}: {err}");
                err
            })
            .unwrap_or(Obj);
        if self.subtype_of(&arg_t, &param_vi.t) {
            if errs.is_empty() {
                Ok(param)
            } else {
                Err((param, errs))
            }
        } else {
            let tp = TyParam::erased(param_vi.t.clone());
            let err = TyCheckError::type_mismatch_error(
                self.cfg.input.clone(),
                line!() as usize,
                arg.loc(),
                self.caused_by(),
                name.as_ref().map_or("", |n| &n.inspect()[..]),
                Some(i),
                &param_vi.t,
                &arg_t,
                None,
                None,
            );
            errs.push(err);
            Err((tp, errs))
        }
    }

    fn instantiate_acc(
        &self,
        acc: &ast::ConstAccessor,
        erased_idx: Option<(&Context, usize)>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> Failable<TyParam> {
        self.inc_ref_acc(&acc.clone().downgrade(), self, tmp_tv_cache);
        match acc {
            ast::ConstAccessor::Attr(attr) => match self.instantiate_const_expr(
                &attr.obj,
                erased_idx,
                tmp_tv_cache,
                not_found_is_qvar,
            ) {
                Ok(obj) => Ok(obj.proj(attr.name.inspect())),
                Err((_obj, errs)) => {
                    if let Ok(ctxs) = self.get_singular_ctxs(&attr.obj.clone().downgrade(), self) {
                        for ctx in ctxs {
                            if let Some(value) = ctx.rec_get_const_obj(attr.name.inspect()) {
                                return Ok(TyParam::Value(value.clone()));
                            }
                        }
                    }
                    Err((TyParam::Failure, errs))
                }
            },
            ast::ConstAccessor::Local(local) => self
                .instantiate_local(local, erased_idx, tmp_tv_cache, local, not_found_is_qvar)
                .map_err(|errs| (TyParam::Failure, errs)),
            other => type_feature_error!(
                self,
                other.loc(),
                &format!("instantiating const expression {other}")
            )
            .map_err(|errs| (TyParam::Failure, errs)),
        }
    }

    fn instantiate_local(
        &self,
        name: &Identifier,
        erased_idx: Option<(&Context, usize)>,
        tmp_tv_cache: &mut TyVarCache,
        loc: &impl Locational,
        not_found_is_qvar: bool,
    ) -> TyCheckResult<TyParam> {
        if &name.inspect()[..] == "_" {
            let t = if let Some((ctx, i)) = erased_idx {
                let param = ctx.params.get(i).ok_or_else(|| {
                    TyCheckErrors::from(TyCheckError::too_many_args_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        loc.loc(),
                        &ctx.name,
                        self.caused_by(),
                        ctx.params.len(),
                        i,
                        0,
                    ))
                })?;
                param.1.t.clone()
            } else {
                Type::Uninited
            };
            return Ok(TyParam::erased(t));
        }
        if let Some((tp, _vi)) = self.get_tp_from_tv_cache(name.inspect(), tmp_tv_cache) {
            return Ok(tp);
        }
        if let Some(value) = self.rec_get_const_obj(name.inspect()) {
            return Ok(TyParam::Value(value.clone()));
        }
        if not_found_is_qvar {
            let tyvar = named_free_var(name.inspect().clone(), self.level, Constraint::Uninited);
            tmp_tv_cache.push_or_init_tyvar(&name.name, &tyvar, self)?;
            return Ok(TyParam::t(tyvar));
        }
        if name.is_const() {
            if let Some((_, vi)) = self.get_var_info(name.inspect()) {
                self.inc_ref(name.inspect(), vi, name, self);
                return Ok(TyParam::mono(name.inspect()));
            }
        }
        Err(TyCheckErrors::from(TyCheckError::no_var_error(
            self.cfg.input.clone(),
            line!() as usize,
            loc.loc(),
            self.caused_by(),
            name.inspect(),
            self.get_similar_name(name.inspect()),
        )))
    }

    fn instantiate_app(
        &self,
        app: &ConstApp,
        erased_idx: Option<(&Context, usize)>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> Failable<TyParam> {
        let mut errs = TyCheckErrors::empty();
        // this operation may fail but if is OK, it is recoverable
        match self.instantiate_const_expr(&app.obj, erased_idx, tmp_tv_cache, not_found_is_qvar) {
            Ok(obj) => {
                let ctx = self
                    .get_singular_ctxs(&app.obj.clone().downgrade(), self)
                    .ok()
                    .and_then(|ctxs| ctxs.first().copied())
                    .or_else(|| {
                        let typ = self.get_tp_t(&obj).ok()?;
                        self.get_nominal_type_ctx(&typ).map(|ctx| &ctx.ctx)
                    })
                    .unwrap_or(self);
                let mut args = vec![];
                for (i, arg) in app.args.pos_args().enumerate() {
                    let arg_t = match self.instantiate_const_expr(
                        &arg.expr,
                        Some((ctx, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(arg_t) => arg_t,
                        Err((arg_t, es)) => {
                            errs.extend(es);
                            arg_t
                        }
                    };
                    args.push(arg_t);
                }
                let tp = if let Some(attr_name) = app.attr_name.as_ref() {
                    obj.proj_call(attr_name.inspect().clone(), args)
                } else if ctx.kind.is_type() && !ctx.params.is_empty() {
                    TyParam::t(poly(ctx.name.clone(), args))
                } else {
                    let ast::ConstExpr::Accessor(ast::ConstAccessor::Local(ident)) =
                        app.obj.as_ref()
                    else {
                        return type_feature_error!(self, app.loc(), "instantiating const callee")
                            .map_err(|es| {
                                errs.extend(es);
                                (TyParam::Failure, errs)
                            });
                    };
                    TyParam::app(ident.inspect().clone(), args)
                };
                if errs.is_empty() {
                    Ok(tp)
                } else {
                    Err((tp, errs))
                }
            }
            Err((_tp, es)) => {
                let Some(attr_name) = app.attr_name.as_ref() else {
                    errs.extend(es);
                    return Err((TyParam::Failure, errs));
                };
                // recover process (`es` are cleared)
                let acc = app.obj.clone().attr(attr_name.clone());
                let attr =
                    self.instantiate_acc(&acc, erased_idx, tmp_tv_cache, not_found_is_qvar)?;
                let ctxs = self
                    .get_singular_ctxs(&acc.downgrade().into(), self)
                    .map_err(|es| (TyParam::Failure, es.into()))?;
                let ctx = ctxs.first().copied().unwrap_or(self);
                let mut args = vec![];
                for (i, arg) in app.args.pos_args().enumerate() {
                    let arg_t = match self.instantiate_const_expr(
                        &arg.expr,
                        Some((ctx, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(arg_t) => arg_t,
                        Err((arg_t, es)) => {
                            errs.extend(es);
                            arg_t
                        }
                    };
                    args.push(arg_t);
                }
                let app = TyParam::app(attr.qual_name().unwrap(), args);
                if errs.is_empty() {
                    Ok(app)
                } else {
                    Err((app, errs))
                }
            }
        }
    }

    /// erased_index:
    /// e.g. `instantiate_const_expr(List(Str, _), Some((self, 1))) => List(Str, _: Nat)`
    pub(crate) fn instantiate_const_expr(
        &self,
        expr: &ast::ConstExpr,
        erased_idx: Option<(&Context, usize)>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> Failable<TyParam> {
        let mut errs = TyCheckErrors::empty();
        if let Ok(value) = self.eval_const_expr(&expr.clone().downgrade()) {
            return Ok(TyParam::Value(value));
        }
        match expr {
            ast::ConstExpr::Lit(lit) => {
                let v = self.eval_lit(lit).map_err(|e| (TyParam::Failure, e))?;
                Ok(TyParam::Value(v))
            }
            ast::ConstExpr::Accessor(acc) => {
                self.instantiate_acc(acc, erased_idx, tmp_tv_cache, not_found_is_qvar)
            }
            ast::ConstExpr::App(app) => {
                self.instantiate_app(app, erased_idx, tmp_tv_cache, not_found_is_qvar)
            }
            ast::ConstExpr::List(ConstList::Normal(list)) => {
                let mut tp_lis = vec![];
                for (i, elem) in list.elems.pos_args().enumerate() {
                    let el = match self.instantiate_const_expr(
                        &elem.expr,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(el) => el,
                        Err((el, es)) => {
                            errs.extend(es);
                            el
                        }
                    };
                    tp_lis.push(el);
                }
                let list = TyParam::List(tp_lis);
                if errs.is_empty() {
                    Ok(list)
                } else {
                    Err((list, errs))
                }
            }
            ast::ConstExpr::List(ConstList::WithLength(lis)) => {
                let elem = self.instantiate_const_expr(
                    &lis.elem,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                let length = self.instantiate_const_expr(
                    &lis.length,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                if length.is_erased() {
                    if let Ok(elem_t) = self.instantiate_tp_as_type(elem, lis) {
                        return Ok(TyParam::t(unknown_len_list_t(elem_t)));
                    }
                }
                type_feature_error!(
                    self,
                    lis.loc(),
                    &format!("instantiating const expression {expr}")
                )
                .map_err(|es| {
                    errs.extend(es);
                    (TyParam::Failure, errs)
                })
            }
            ast::ConstExpr::Set(ConstSet::Normal(set)) => {
                let mut tp_set = set! {};
                for (i, elem) in set.elems.pos_args().enumerate() {
                    let el = match self.instantiate_const_expr(
                        &elem.expr,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(el) => el,
                        Err((el, es)) => {
                            errs.extend(es);
                            el
                        }
                    };
                    tp_set.insert(el);
                }
                let set = TyParam::Set(tp_set);
                if errs.is_empty() {
                    Ok(set)
                } else {
                    Err((set, errs))
                }
            }
            ast::ConstExpr::Set(ConstSet::Comprehension(set)) => {
                if set.layout.is_none() && set.generators.len() == 1 && set.guard.is_some() {
                    let (ident, expr) = set.generators.first().unwrap();
                    let iter = self.instantiate_const_expr(
                        expr,
                        erased_idx,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    let pred = match self
                        .instantiate_pred_from_expr(set.guard.as_ref().unwrap(), tmp_tv_cache)
                    {
                        Ok(pred) => pred,
                        Err((pred, es)) => {
                            errs.extend(es);
                            pred
                        }
                    };
                    if let Ok(t) = self.instantiate_tp_as_type(iter, set) {
                        let tp = TyParam::t(refinement(ident.inspect().clone(), t, pred));
                        if errs.is_empty() {
                            return Ok(tp);
                        } else {
                            return Err((tp, errs));
                        }
                    }
                }
                type_feature_error!(
                    self,
                    set.loc(),
                    &format!("instantiating const expression {expr}")
                )
                .map_err(|es| (TyParam::Failure, es))
            }
            ast::ConstExpr::Dict(dict) => {
                let mut tp_dict = dict! {};
                for (i, elem) in dict.kvs.iter().enumerate() {
                    let key = match self.instantiate_const_expr(
                        &elem.key,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(key) => key,
                        Err((key, es)) => {
                            errs.extend(es);
                            key
                        }
                    };
                    let val = match self.instantiate_const_expr(
                        &elem.value,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(val) => val,
                        Err((val, es)) => {
                            errs.extend(es);
                            val
                        }
                    };
                    tp_dict.insert(key, val);
                }
                let dict = TyParam::Dict(tp_dict);
                if errs.is_empty() {
                    Ok(dict)
                } else {
                    Err((dict, errs))
                }
            }
            ast::ConstExpr::Tuple(tuple) => {
                let mut tp_tuple = vec![];
                for (i, elem) in tuple.elems.pos_args().enumerate() {
                    let el = match self.instantiate_const_expr(
                        &elem.expr,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(el) => el,
                        Err((el, es)) => {
                            errs.extend(es);
                            el
                        }
                    };
                    tp_tuple.push(el);
                }
                let tuple = TyParam::Tuple(tp_tuple);
                if errs.is_empty() {
                    Ok(tuple)
                } else {
                    Err((tuple, errs))
                }
            }
            ast::ConstExpr::Record(rec) => {
                let mut tp_rec = dict! {};
                for attr in rec.attrs.iter() {
                    let field = match self.instantiate_field(&attr.ident) {
                        Ok(field) => field,
                        Err((field, es)) => {
                            errs.extend(es);
                            field
                        }
                    };
                    let val = match self.instantiate_const_expr(
                        attr.body.block.first().unwrap(),
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(val) => val,
                        Err((val, es)) => {
                            errs.extend(es);
                            val
                        }
                    };
                    tp_rec.insert(field, val);
                }
                let tp_rec = TyParam::Record(tp_rec);
                if errs.is_empty() {
                    Ok(tp_rec)
                } else {
                    Err((tp_rec, errs))
                }
            }
            ast::ConstExpr::Lambda(lambda) => {
                let mut errs = TyCheckErrors::empty();
                let _tmp_tv_cache = match self
                    .instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal)
                {
                    Ok(tv_cache) => tv_cache,
                    Err((tv_cache, es)) => {
                        errs.extend(es);
                        tv_cache
                    }
                };
                // Since there are type variables and other variables that can be constrained within closures,
                // they are `merge`d once and then `purge`d of type variables that are only used internally after instantiation.
                tmp_tv_cache.merge(&_tmp_tv_cache);
                let mut nd_params = Vec::with_capacity(lambda.sig.params.non_defaults.len());
                for sig in lambda.sig.params.non_defaults.iter() {
                    let pt = match self.instantiate_param_ty(
                        sig,
                        None,
                        tmp_tv_cache,
                        RegistrationMode::Normal,
                        ParamKind::NonDefault,
                        not_found_is_qvar,
                    ) {
                        Ok(pt) => pt,
                        Err((pt, es)) => {
                            errs.extend(es);
                            pt
                        }
                    };
                    nd_params.push(pt);
                }
                let var_params = if let Some(p) = lambda.sig.params.var_params.as_ref() {
                    let pt = match self.instantiate_param_ty(
                        p,
                        None,
                        tmp_tv_cache,
                        RegistrationMode::Normal,
                        ParamKind::VarParams,
                        not_found_is_qvar,
                    ) {
                        Ok(pt) => pt,
                        Err((pt, es)) => {
                            errs.extend(es);
                            pt
                        }
                    };
                    Some(pt)
                } else {
                    None
                };
                let mut d_params = Vec::with_capacity(lambda.sig.params.defaults.len());
                for sig in lambda.sig.params.defaults.iter() {
                    let expr = match self.eval_const_expr(&sig.default_val) {
                        Ok(val) => val,
                        Err((val, es)) => {
                            errs.extend(es);
                            val
                        }
                    };
                    let pt = match self.instantiate_param_ty(
                        &sig.sig,
                        None,
                        tmp_tv_cache,
                        RegistrationMode::Normal,
                        ParamKind::Default(expr.t()),
                        not_found_is_qvar,
                    ) {
                        Ok(pt) => pt,
                        Err((pt, es)) => {
                            errs.extend(es);
                            pt
                        }
                    };
                    d_params.push(pt);
                }
                let kw_var_params = if let Some(p) = lambda.sig.params.kw_var_params.as_ref() {
                    let pt = match self.instantiate_param_ty(
                        p,
                        None,
                        tmp_tv_cache,
                        RegistrationMode::Normal,
                        ParamKind::KwVarParams,
                        not_found_is_qvar,
                    ) {
                        Ok(pt) => pt,
                        Err((pt, es)) => {
                            errs.extend(es);
                            pt
                        }
                    };
                    Some(pt)
                } else {
                    None
                };
                let mut lambda_ctx = Context::instant(
                    Str::ever("<lambda>"),
                    self.cfg.clone(),
                    0,
                    self.shared.clone(),
                    self.clone(),
                );
                for non_default in nd_params.iter() {
                    let name = non_default
                        .name()
                        .map(|name| VarName::from_str(name.clone()));
                    let vi = VarInfo::nd_parameter(
                        non_default.typ().clone(),
                        AbsLocation::unknown(),
                        lambda_ctx.name.clone(),
                    );
                    lambda_ctx.params.push((name, vi));
                }
                if let Some(var_param) = var_params.as_ref() {
                    let name = var_param.name().map(|name| VarName::from_str(name.clone()));
                    let vi = VarInfo::nd_parameter(
                        var_param.typ().clone(),
                        AbsLocation::unknown(),
                        lambda_ctx.name.clone(),
                    );
                    lambda_ctx.params.push((name, vi));
                }
                for default in d_params.iter() {
                    let name = default.name().map(|name| VarName::from_str(name.clone()));
                    let vi = VarInfo::d_parameter(
                        default.typ().clone(),
                        AbsLocation::unknown(),
                        lambda_ctx.name.clone(),
                    );
                    lambda_ctx.params.push((name, vi));
                }
                let mut body = vec![];
                for expr in lambda.body.iter() {
                    let param = lambda_ctx.instantiate_const_expr(
                        expr,
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    body.push(param);
                }
                tmp_tv_cache.purge(&_tmp_tv_cache);
                let lambda = TyParam::Lambda(TyParamLambda::new(
                    lambda.clone(),
                    nd_params,
                    var_params,
                    d_params,
                    kw_var_params,
                    body,
                ));
                if errs.is_empty() {
                    Ok(lambda)
                } else {
                    Err((lambda, errs))
                }
            }
            ast::ConstExpr::BinOp(bin) => {
                let Some(op) = token_kind_to_op_kind(bin.op.kind) else {
                    return type_feature_error!(
                        self,
                        bin.loc(),
                        &format!("instantiating const expression {bin}")
                    )
                    .map_err(|es| {
                        errs.extend(es);
                        (TyParam::Failure, errs)
                    });
                };
                let lhs = match self.instantiate_const_expr(
                    &bin.lhs,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ) {
                    Ok(lhs) => lhs,
                    Err((lhs, es)) => {
                        errs.extend(es);
                        lhs
                    }
                };
                let rhs = match self.instantiate_const_expr(
                    &bin.rhs,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ) {
                    Ok(rhs) => rhs,
                    Err((rhs, es)) => {
                        errs.extend(es);
                        rhs
                    }
                };
                if errs.is_empty() {
                    Ok(TyParam::bin(op, lhs, rhs))
                } else {
                    Err((TyParam::bin(op, lhs, rhs), errs))
                }
            }
            ast::ConstExpr::UnaryOp(unary) => {
                let Some(op) = token_kind_to_op_kind(unary.op.kind) else {
                    return type_feature_error!(
                        self,
                        unary.loc(),
                        &format!("instantiating const expression {unary}")
                    )
                    .map_err(|es| {
                        errs.extend(es);
                        (TyParam::Failure, errs)
                    });
                };
                let val = match self.instantiate_const_expr(
                    &unary.expr,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ) {
                    Ok(val) => val,
                    Err((val, es)) => {
                        errs.extend(es);
                        val
                    }
                };
                if errs.is_empty() {
                    Ok(TyParam::unary(op, val))
                } else {
                    Err((TyParam::unary(op, val), errs))
                }
            }
            ast::ConstExpr::TypeAsc(tasc) => {
                let tp = self.instantiate_const_expr(
                    &tasc.expr,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                let spec_t = match self.instantiate_typespec_full(
                    &tasc.t_spec.t_spec,
                    None,
                    tmp_tv_cache,
                    RegistrationMode::Normal,
                    false,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                let tp_t = match self.get_tp_t(&tp) {
                    Ok(t) => t,
                    Err(es) => {
                        errs.extend(es);
                        return Err((tp, errs));
                    }
                };
                if self.subtype_of(&tp_t, &spec_t) {
                    Ok(tp)
                } else {
                    let err = TyCheckError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &tp_t,
                        &spec_t,
                        tasc.loc(),
                        self.caused_by(),
                    );
                    errs.push(err);
                    Err((tp, errs))
                }
            }
            other => type_feature_error!(
                self,
                other.loc(),
                &format!("instantiating const expression {other}")
            )
            .map_err(|es| (TyParam::Failure, es)),
        }
    }

    pub(crate) fn instantiate_const_expr_as_type(
        &self,
        expr: &ast::ConstExpr,
        erased_idx: Option<(&Context, usize)>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> Failable<Type> {
        match self.instantiate_const_expr(expr, erased_idx, tmp_tv_cache, not_found_is_qvar) {
            Ok(tp) => self.instantiate_tp_as_type(tp, expr),
            Err((tp, mut errs)) => match self.instantiate_tp_as_type(tp, expr) {
                Ok(t) => Err((t, errs)),
                Err((t, es)) => {
                    errs.extend(es);
                    Err((t, errs))
                }
            },
        }
    }

    fn instantiate_tp_as_type(&self, tp: TyParam, loc: &impl Locational) -> Failable<Type> {
        let mut errs = TyCheckErrors::empty();
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let tp = fv.crack().clone();
                self.instantiate_tp_as_type(tp, loc)
            }
            TyParam::Mono(name) => Ok(mono(name)),
            TyParam::Proj { obj, attr } => {
                let obj = self.instantiate_tp_as_type(*obj, loc)?;
                Ok(proj(obj, attr))
            }
            TyParam::App { name, args } => Ok(poly(name, args)),
            TyParam::Type(t) => Ok(*t),
            TyParam::Value(value) => self.convert_value_into_type(value).or_else(|value| {
                type_feature_error!(self, loc.loc(), &format!("instantiate `{value}` as type"))
                    .map_err(|e| (Type::Failure, e))
            }),
            TyParam::List(lis) => {
                let len = TyParam::value(lis.len());
                let mut union = Type::Never;
                for tp in lis {
                    let t = match self.instantiate_tp_as_type(tp, loc) {
                        Ok(t) => t,
                        Err((t, es)) => {
                            errs.extend(es);
                            t
                        }
                    };
                    union = self.union(&union, &t);
                }
                if errs.is_empty() {
                    Ok(list_t(union, len))
                } else {
                    Err((list_t(union, len), errs))
                }
            }
            TyParam::Set(set) => {
                let t = set
                    .iter()
                    .next()
                    .and_then(|tp| self.get_tp_t(tp).ok())
                    .unwrap_or(Type::Never);
                Ok(tp_enum(t, set))
            }
            TyParam::Tuple(ts) => {
                let mut tps = vec![];
                for tp in ts {
                    let t = match self.instantiate_tp_as_type(tp, loc) {
                        Ok(t) => t,
                        Err((t, es)) => {
                            errs.extend(es);
                            t
                        }
                    };
                    tps.push(t);
                }
                if errs.is_empty() {
                    Ok(tuple_t(tps))
                } else {
                    Err((tuple_t(tps), errs))
                }
            }
            TyParam::Record(rec) => {
                let mut rec_t = dict! {};
                for (field, tp) in rec {
                    let t = match self.instantiate_tp_as_type(tp, loc) {
                        Ok(t) => t,
                        Err((t, es)) => {
                            errs.extend(es);
                            t
                        }
                    };
                    rec_t.insert(field, t);
                }
                if errs.is_empty() {
                    Ok(Type::Record(rec_t))
                } else {
                    Err((Type::Record(rec_t), errs))
                }
            }
            TyParam::Lambda(lambda) => {
                let return_t = self
                    .instantiate_tp_as_type(lambda.body.last().unwrap().clone(), &lambda.const_)?;
                let subr = SubrType::new(
                    SubrKind::from(lambda.const_.op.kind),
                    lambda.nd_params,
                    lambda.var_params,
                    lambda.d_params,
                    lambda.kw_var_params,
                    return_t,
                );
                Ok(Type::Subr(subr))
            }
            TyParam::BinOp { op, lhs, rhs } => match op {
                OpKind::And => {
                    let lhs = self.instantiate_tp_as_type(*lhs, loc)?;
                    let rhs = self.instantiate_tp_as_type(*rhs, loc)?;
                    Ok(lhs & rhs)
                }
                OpKind::Or => {
                    let lhs = self.instantiate_tp_as_type(*lhs, loc)?;
                    let rhs = self.instantiate_tp_as_type(*rhs, loc)?;
                    Ok(lhs | rhs)
                }
                _ => type_feature_error!(
                    self,
                    loc.loc(),
                    &format!("instantiate `{lhs} {op} {rhs}` as type")
                )
                .map_err(|es| {
                    errs.extend(es);
                    (Type::Failure, errs)
                }),
            },
            other =>
            {
                #[allow(clippy::bind_instead_of_map)]
                self.convert_tp_into_type(other).or_else(|tp| {
                    type_feature_error!(self, loc.loc(), &format!("instantiate `{tp}` as type"))
                        .map_err(|e| (Type::Failure, e))
                })
            }
        }
    }

    fn instantiate_func_param_spec(
        &self,
        p: &ParamTySpec,
        opt_decl_t: Option<&ParamTy>,
        default_t: Option<&TypeSpec>,
        tmp_tv_cache: &mut TyVarCache,
        mode: RegistrationMode,
        not_found_is_qvar: bool,
    ) -> Failable<ParamTy> {
        let mut errs = TyCheckErrors::empty();
        let t = match self.instantiate_typespec_full(
            &p.ty,
            opt_decl_t,
            tmp_tv_cache,
            mode,
            not_found_is_qvar,
        ) {
            Ok(t) => t,
            Err((t, es)) => {
                errs.extend(es);
                t
            }
        };
        let pt = if let Some(default_t) = default_t {
            let default = match self.instantiate_typespec_full(
                default_t,
                opt_decl_t,
                tmp_tv_cache,
                mode,
                not_found_is_qvar,
            ) {
                Ok(t) => t,
                Err((t, es)) => {
                    errs.extend(es);
                    t
                }
            };
            ParamTy::kw_default(p.name.as_ref().unwrap().inspect().to_owned(), t, default)
        } else {
            ParamTy::pos_or_kw(p.name.as_ref().map(|t| t.inspect().to_owned()), t)
        };
        if errs.is_empty() {
            Ok(pt)
        } else {
            Err((pt, errs))
        }
    }

    fn instantiate_pred_from_expr(
        &self,
        expr: &ast::ConstExpr,
        tmp_tv_cache: &mut TyVarCache,
    ) -> Failable<Predicate> {
        let mut errs = TyCheckErrors::empty();
        let pred = match expr {
            ast::ConstExpr::Lit(lit) => {
                let value = self.eval_lit(lit).map_err(|e| (Predicate::Failure, e))?;
                Predicate::Value(value)
            }
            ast::ConstExpr::Accessor(ast::ConstAccessor::Local(local)) => {
                self.inc_ref_local(local, self, tmp_tv_cache);
                Predicate::Const(local.inspect().clone())
            }
            ast::ConstExpr::Accessor(ast::ConstAccessor::Attr(attr)) => {
                let obj = match self.instantiate_const_expr(&attr.obj, None, tmp_tv_cache, false) {
                    Ok(obj) => obj,
                    Err((obj, es)) => {
                        errs.extend(es);
                        obj
                    }
                };
                Predicate::attr(obj, attr.name.inspect().clone())
            }
            ast::ConstExpr::App(app) => {
                let receiver =
                    match self.instantiate_const_expr(&app.obj, None, tmp_tv_cache, false) {
                        Ok(obj) => obj,
                        Err((obj, es)) => {
                            errs.extend(es);
                            obj
                        }
                    };
                let name = app.attr_name.as_ref().map(|n| n.inspect().to_owned());
                let mut args = vec![];
                for arg in app.args.pos_args() {
                    let arg =
                        match self.instantiate_const_expr(&arg.expr, None, tmp_tv_cache, false) {
                            Ok(arg) => arg,
                            Err((arg, es)) => {
                                errs.extend(es);
                                arg
                            }
                        };
                    args.push(arg);
                }
                Predicate::Call {
                    receiver,
                    name,
                    args,
                }
            }
            ast::ConstExpr::BinOp(bin) => {
                let lhs = match self.instantiate_pred_from_expr(&bin.lhs, tmp_tv_cache) {
                    Ok(lhs) => lhs,
                    Err((lhs, es)) => {
                        errs.extend(es);
                        lhs
                    }
                };
                let rhs = match self.instantiate_pred_from_expr(&bin.rhs, tmp_tv_cache) {
                    Ok(rhs) => rhs,
                    Err((rhs, es)) => {
                        errs.extend(es);
                        rhs
                    }
                };
                match bin.op.kind {
                    TokenKind::DblEq
                    | TokenKind::NotEq
                    | TokenKind::Less
                    | TokenKind::LessEq
                    | TokenKind::Gre
                    | TokenKind::GreEq => {
                        let var = match lhs {
                            Predicate::Const(var) => var,
                            other if bin.op.kind == TokenKind::DblEq => {
                                return Ok(Predicate::general_eq(other, rhs));
                            }
                            other if bin.op.kind == TokenKind::NotEq => {
                                return Ok(Predicate::general_ne(other, rhs));
                            }
                            other if bin.op.kind == TokenKind::GreEq => {
                                return Ok(Predicate::general_ge(other, rhs));
                            }
                            other if bin.op.kind == TokenKind::LessEq => {
                                return Ok(Predicate::general_le(other, rhs));
                            }
                            _ => {
                                return type_feature_error!(
                                    self,
                                    bin.loc(),
                                    &format!("instantiating predicate `{expr}`")
                                )
                                .map_err(|es| {
                                    errs.extend(es);
                                    (Predicate::Failure, errs)
                                });
                            }
                        };
                        let rhs = match rhs {
                            Predicate::Value(value) => TyParam::Value(value),
                            Predicate::Const(var) => TyParam::Mono(var),
                            other if bin.op.kind == TokenKind::DblEq => {
                                return Ok(Predicate::general_eq(Predicate::Const(var), other));
                            }
                            other if bin.op.kind == TokenKind::NotEq => {
                                return Ok(Predicate::general_ne(Predicate::Const(var), other));
                            }
                            other if bin.op.kind == TokenKind::GreEq => {
                                return Ok(Predicate::general_ge(Predicate::Const(var), other));
                            }
                            other if bin.op.kind == TokenKind::LessEq => {
                                return Ok(Predicate::general_le(Predicate::Const(var), other));
                            }
                            _ => {
                                return type_feature_error!(
                                    self,
                                    bin.loc(),
                                    &format!("instantiating predicate `{expr}`")
                                )
                                .map_err(|es| {
                                    errs.extend(es);
                                    (Predicate::Failure, errs)
                                });
                            }
                        };
                        match bin.op.kind {
                            TokenKind::DblEq => Predicate::eq(var, rhs),
                            TokenKind::NotEq => Predicate::ne(var, rhs),
                            TokenKind::Less => Predicate::lt(var, rhs),
                            TokenKind::LessEq => Predicate::le(var, rhs),
                            TokenKind::Gre => Predicate::gt(var, rhs),
                            TokenKind::GreEq => Predicate::ge(var, rhs),
                            _ => unreachable!(),
                        }
                    }
                    TokenKind::OrOp => lhs | rhs,
                    TokenKind::AndOp => lhs & rhs,
                    _ => {
                        return type_feature_error!(
                            self,
                            bin.loc(),
                            &format!("instantiating predicate `{expr}`")
                        )
                        .map_err(|e| {
                            errs.extend(e);
                            (Predicate::Failure, errs)
                        })
                    }
                }
            }
            ast::ConstExpr::UnaryOp(unop) => {
                let pred = match self.instantiate_pred_from_expr(&unop.expr, tmp_tv_cache) {
                    Ok(pred) => pred,
                    Err((pred, es)) => {
                        errs.extend(es);
                        pred
                    }
                };
                match unop.op.kind {
                    TokenKind::PreBitNot => !pred,
                    _ => {
                        return type_feature_error!(
                            self,
                            unop.loc(),
                            &format!("instantiating predicate `{expr}`")
                        )
                        .map_err(|e| {
                            errs.extend(e);
                            (Predicate::Failure, errs)
                        })
                    }
                }
            }
            _ => {
                return type_feature_error!(
                    self,
                    expr.loc(),
                    &format!("instantiating predicate `{expr}`")
                )
                .map_err(|e| {
                    errs.extend(e);
                    (Predicate::Failure, errs)
                })
            }
        };
        if errs.is_empty() {
            Ok(pred)
        } else {
            Err((pred, errs))
        }
    }

    #[inline]
    fn recover_guard<'a>(&self, return_t: Type, mut params: impl Iterator<Item = &'a Str>) -> Type {
        match return_t {
            Type::Guard(GuardType {
                namespace,
                target: CastTarget::Expr(expr),
                to,
            }) => {
                let target = if let Some(nth) = params.position(|p| Some(p) == expr.get_name()) {
                    CastTarget::arg(nth, expr.get_name().unwrap().clone(), ().loc())
                } else {
                    CastTarget::Expr(expr)
                };
                Type::Guard(GuardType {
                    namespace,
                    target,
                    to,
                })
            }
            _ => return_t,
        }
    }

    // FIXME: opt_decl_t must be disassembled for each polymorphic type
    pub(crate) fn instantiate_typespec_full(
        &self,
        t_spec: &TypeSpec,
        opt_decl_t: Option<&ParamTy>,
        tmp_tv_cache: &mut TyVarCache,
        mode: RegistrationMode,
        not_found_is_qvar: bool,
    ) -> Failable<Type> {
        let mut errs = TyCheckErrors::empty();
        match t_spec {
            TypeSpec::Infer(_) => Ok(free_var(self.level, Constraint::new_type_of(Type))),
            TypeSpec::PreDeclTy(predecl) => Ok(self.instantiate_predecl_t(
                predecl,
                opt_decl_t,
                tmp_tv_cache,
                not_found_is_qvar,
            )?),
            TypeSpec::And(lhs, rhs) => {
                let lhs = match self.instantiate_typespec_full(
                    lhs,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                let rhs = match self.instantiate_typespec_full(
                    rhs,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                if errs.is_empty() {
                    Ok(self.intersection(&lhs, &rhs))
                } else {
                    Err((self.intersection(&lhs, &rhs), errs))
                }
            }
            TypeSpec::Or(lhs, rhs) => Ok(self.union(
                &self.instantiate_typespec_full(
                    lhs,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                )?,
                &self.instantiate_typespec_full(
                    rhs,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                )?,
            )),
            TypeSpec::Not(ty) => Ok(self.complement(&self.instantiate_typespec_full(
                ty,
                opt_decl_t,
                tmp_tv_cache,
                mode,
                not_found_is_qvar,
            )?)),
            TypeSpec::List(lis) => {
                let elem_t = match self.instantiate_typespec_full(
                    &lis.ty,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                let mut len = match self.instantiate_const_expr(
                    &lis.len,
                    None,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ) {
                    Ok(len) => len,
                    Err((len, es)) => {
                        errs.extend(es);
                        len
                    }
                };
                if let TyParam::Erased(t) = &mut len {
                    *t.as_mut() = Type::Nat;
                }
                if errs.is_empty() {
                    Ok(list_t(elem_t, len))
                } else {
                    Err((list_t(elem_t, len), errs))
                }
            }
            TypeSpec::SetWithLen(set) => {
                let elem_t = self.instantiate_typespec_full(
                    &set.ty,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                )?;
                let mut len = match self.instantiate_const_expr(
                    &set.len,
                    None,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ) {
                    Ok(len) => len,
                    Err((len, es)) => {
                        errs.extend(es);
                        len
                    }
                };
                if let TyParam::Erased(t) = &mut len {
                    *t.as_mut() = Type::Nat;
                }
                if errs.is_empty() {
                    Ok(set_t(elem_t, len))
                } else {
                    Err((set_t(elem_t, len), errs))
                }
            }
            TypeSpec::Tuple(tup) => {
                let mut inst_tys = vec![];
                for spec in tup.tys.iter() {
                    match self.instantiate_typespec_full(
                        spec,
                        opt_decl_t,
                        tmp_tv_cache,
                        mode,
                        not_found_is_qvar,
                    ) {
                        Ok(t) => inst_tys.push(t),
                        Err((t, es)) => {
                            errs.extend(es);
                            inst_tys.push(t);
                        }
                    }
                }
                if errs.is_empty() {
                    Ok(tuple_t(inst_tys))
                } else {
                    Err((tuple_t(inst_tys), errs))
                }
            }
            TypeSpec::Dict(dict) => {
                let mut inst_tys = dict! {};
                for (k, v) in dict.kvs.iter() {
                    let k = match self.instantiate_typespec_full(
                        k,
                        opt_decl_t,
                        tmp_tv_cache,
                        mode,
                        not_found_is_qvar,
                    ) {
                        Ok(k) => k,
                        Err((k, es)) => {
                            errs.extend(es);
                            k
                        }
                    };
                    let v = match self.instantiate_typespec_full(
                        v,
                        opt_decl_t,
                        tmp_tv_cache,
                        mode,
                        not_found_is_qvar,
                    ) {
                        Ok(v) => v,
                        Err((v, es)) => {
                            errs.extend(es);
                            v
                        }
                    };
                    inst_tys.insert(k, v);
                }
                if errs.is_empty() {
                    Ok(dict_t(inst_tys.into()))
                } else {
                    Err((dict_t(inst_tys.into()), errs))
                }
            }
            TypeSpec::Record(rec) => {
                let mut inst_tys = dict! {};
                for (k, v) in rec.attrs.iter() {
                    let v = match self.instantiate_typespec_full(
                        v,
                        opt_decl_t,
                        tmp_tv_cache,
                        mode,
                        not_found_is_qvar,
                    ) {
                        Ok(v) => v,
                        Err((v, es)) => {
                            errs.extend(es);
                            v
                        }
                    };
                    let field = match self.instantiate_field(k) {
                        Ok(field) => field,
                        Err((field, es)) => {
                            errs.extend(es);
                            field
                        }
                    };
                    inst_tys.insert(field, v);
                }
                if errs.is_empty() {
                    Ok(Type::Record(inst_tys))
                } else {
                    Err((Type::Record(inst_tys), errs))
                }
            }
            // TODO: エラー処理(リテラルでない)はパーサーにやらせる
            TypeSpec::Enum(set) => {
                let mut new_set = set! {};
                // guard type (e.g. {x in Int})
                if set.pos_args.len() == 1 {
                    let expr = &set.pos_args().next().unwrap().expr;
                    match expr {
                        ConstExpr::BinOp(bin) if bin.op.is(TokenKind::InOp) => {
                            if let Ok(to) = self.instantiate_const_expr_as_type(
                                &bin.rhs,
                                None,
                                tmp_tv_cache,
                                not_found_is_qvar,
                            ) {
                                let target = CastTarget::expr(bin.lhs.clone().downgrade());
                                return Ok(Type::Guard(GuardType::new(
                                    self.name.clone(),
                                    target,
                                    to,
                                )));
                            }
                        }
                        _ => {}
                    }
                }
                for arg in set.pos_args() {
                    match self.instantiate_const_expr(
                        &arg.expr,
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        Ok(tp) => new_set.insert(tp),
                        Err((tp, es)) => {
                            errs.extend(es);
                            new_set.insert(tp)
                        }
                    };
                }
                let ty = new_set.iter().fold(Type::Never, |t, tp| {
                    self.union(&t, &self.get_tp_t(tp).unwrap_or(Obj).derefine())
                });
                if errs.is_empty() {
                    Ok(tp_enum(ty, new_set))
                } else {
                    Err((tp_enum(ty, new_set), errs))
                }
            }
            TypeSpec::Interval { op, lhs, rhs } => {
                let mut errs = TyCheckErrors::empty();
                let op = match op.kind {
                    TokenKind::Closed => IntervalOp::Closed,
                    TokenKind::LeftOpen => IntervalOp::LeftOpen,
                    TokenKind::RightOpen => IntervalOp::RightOpen,
                    TokenKind::Open => IntervalOp::Open,
                    _ => assume_unreachable!(),
                };
                let l =
                    match self.instantiate_const_expr(lhs, None, tmp_tv_cache, not_found_is_qvar) {
                        Ok(tp) => tp,
                        Err((tp, es)) => {
                            errs.extend(es);
                            tp
                        }
                    };
                let l = match self.eval_tp(l) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                let r =
                    match self.instantiate_const_expr(rhs, None, tmp_tv_cache, not_found_is_qvar) {
                        Ok(tp) => tp,
                        Err((tp, es)) => {
                            errs.extend(es);
                            tp
                        }
                    };
                let r = match self.eval_tp(r) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                if let Some(Greater) = self.try_cmp(&l, &r) {
                    panic!("{l}..{r} is not a valid interval type (should be lhs <= rhs)")
                }
                let l_t = self.get_tp_t(&l).unwrap_or(Obj).derefine();
                let r_t = self.get_tp_t(&r).unwrap_or(Obj).derefine();
                let t = self.union(&l_t, &r_t);
                let int = interval(op, t, l, r);
                if errs.is_empty() {
                    Ok(int)
                } else {
                    Err((int, errs))
                }
            }
            TypeSpec::Subr(subr) => {
                let mut errs = TyCheckErrors::empty();
                let mut inner_tv_ctx = if !subr.bounds.is_empty() {
                    let tv_cache = match self.instantiate_ty_bounds(&subr.bounds, mode) {
                        Ok(tv_cache) => tv_cache,
                        Err((tv_cache, es)) => {
                            errs.extend(es);
                            tv_cache
                        }
                    };
                    Some(tv_cache)
                } else {
                    None
                };
                if let Some(inner) = &mut inner_tv_ctx {
                    inner.merge(tmp_tv_cache);
                }
                let tmp_tv_ctx = if let Some(inner) = &mut inner_tv_ctx {
                    inner
                } else {
                    tmp_tv_cache
                };
                let non_defaults = match failable_map_mut(subr.non_defaults.iter(), |p| {
                    self.instantiate_func_param_spec(
                        p,
                        opt_decl_t,
                        None,
                        tmp_tv_ctx,
                        mode,
                        not_found_is_qvar,
                    )
                }) {
                    Ok(v) => v,
                    Err((v, es)) => {
                        for e in es {
                            errs.extend(e);
                        }
                        v
                    }
                };
                let var_params = match subr.var_params.as_ref().map(|p| {
                    self.instantiate_func_param_spec(
                        p,
                        opt_decl_t,
                        None,
                        tmp_tv_ctx,
                        mode,
                        not_found_is_qvar,
                    )
                }) {
                    Some(Ok(pt)) => Some(pt),
                    Some(Err((pt, es))) => {
                        errs.extend(es);
                        Some(pt)
                    }
                    None => None,
                };
                let defaults = failable_map_mut(subr.defaults.iter(), |p| {
                    self.instantiate_func_param_spec(
                        &p.param,
                        opt_decl_t,
                        Some(&p.default),
                        tmp_tv_ctx,
                        mode,
                        not_found_is_qvar,
                    )
                })
                .unwrap_or_else(|(pts, es)| {
                    for e in es {
                        errs.extend(e);
                    }
                    pts
                })
                .into_iter()
                .collect();
                let kw_var_params = match subr.kw_var_params.as_ref().map(|p| {
                    self.instantiate_func_param_spec(
                        p,
                        opt_decl_t,
                        None,
                        tmp_tv_ctx,
                        mode,
                        not_found_is_qvar,
                    )
                }) {
                    Some(Ok(pt)) => Some(pt),
                    Some(Err((pt, es))) => {
                        errs.extend(es);
                        Some(pt)
                    }
                    None => None,
                };
                let return_t = match self.instantiate_typespec_full(
                    &subr.return_t,
                    opt_decl_t,
                    tmp_tv_ctx,
                    mode,
                    not_found_is_qvar,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                let params = non_defaults
                    .iter()
                    .chain(&var_params)
                    .chain(&defaults)
                    .chain(&kw_var_params)
                    .filter_map(|pt| pt.name());
                let return_t = self.recover_guard(return_t, params);
                // no quantification at this point (in `generalize_t`)
                let subr = subr_t(
                    SubrKind::from(subr.arrow.kind),
                    non_defaults,
                    var_params,
                    defaults,
                    kw_var_params,
                    return_t,
                );
                if errs.is_empty() {
                    Ok(subr)
                } else {
                    Err((subr, errs))
                }
            }
            TypeSpec::TypeApp { spec, args } => type_feature_error!(
                self,
                t_spec.loc(),
                &format!("instantiating type spec {spec}{args}")
            )
            .map_err(|es| {
                errs.extend(es);
                (Type::Failure, errs)
            }),
            TypeSpec::Refinement(refine) => {
                let t = match self.instantiate_typespec_full(
                    &refine.typ,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                ) {
                    Ok(t) => t,
                    Err((t, es)) => {
                        errs.extend(es);
                        t
                    }
                };
                let name = VarName::new(refine.var.clone());
                tmp_tv_cache.push_refine_var(&name, t.clone(), self);
                let pred = match self.instantiate_pred_from_expr(&refine.pred, tmp_tv_cache) {
                    Ok(pred) => pred,
                    Err((pred, es)) => {
                        errs.extend(es);
                        pred
                    }
                };
                tmp_tv_cache.remove(name.inspect());
                let refine =
                    Type::Refinement(RefinementType::new(refine.var.inspect().clone(), t, pred));
                if errs.is_empty() {
                    Ok(refine)
                } else {
                    Err((refine, errs))
                }
            }
        }
    }

    pub(crate) fn instantiate_typespec(&self, t_spec: &ast::TypeSpec) -> Failable<Type> {
        let mut dummy_tv_cache = TyVarCache::new(self.level, self);
        self.instantiate_typespec_with_tv_cache(t_spec, &mut dummy_tv_cache)
    }

    pub(crate) fn instantiate_typespec_with_tv_cache(
        &self,
        t_spec: &ast::TypeSpec,
        tv_cache: &mut TyVarCache,
    ) -> Failable<Type> {
        let (t, errs) = match self.instantiate_typespec_full(
            t_spec,
            None,
            tv_cache,
            RegistrationMode::Normal,
            false,
        ) {
            Ok(t) => (t, TyCheckErrors::empty()),
            Err((t, es)) => (t, es),
        };
        t.lift();
        let t = self.generalize_t(t);
        if errs.is_empty() {
            Ok(t)
        } else {
            Err((t, errs))
        }
    }

    pub(crate) fn instantiate_field(&self, ident: &Identifier) -> Failable<Field> {
        match self.instantiate_vis_modifier(&ident.vis) {
            Ok(vis) => Ok(Field::new(vis, ident.inspect().clone())),
            Err(errs) => {
                let field = Field::new(VisibilityModifier::Public, ident.inspect().clone());
                Err((field, errs))
            }
        }
    }

    pub(crate) fn instantiate_vis_modifier(
        &self,
        spec: &VisModifierSpec,
    ) -> TyCheckResult<VisibilityModifier> {
        match spec {
            VisModifierSpec::Auto => Err(TyCheckErrors::from(TyCheckError::unreachable(
                self.cfg.input.clone(),
                fn_name!(),
                line!(),
            ))),
            VisModifierSpec::Private | VisModifierSpec::ExplicitPrivate(_) => {
                Ok(VisibilityModifier::Private)
            }
            VisModifierSpec::Public(_) => Ok(VisibilityModifier::Public),
            VisModifierSpec::Restricted(rest) => match &rest {
                VisRestriction::Namespaces(namespace) => {
                    let mut namespaces = set! {};
                    for ns in namespace.iter() {
                        let ast::Accessor::Ident(ident) = ns else {
                            return type_feature_error!(self, ns.loc(), "namespace accessors");
                        };
                        let vi = self
                            .rec_get_var_info(ident, AccessKind::Name, &self.cfg.input, self)
                            .none_or_result(|| {
                                TyCheckError::no_var_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    ident.loc(),
                                    self.caused_by(),
                                    ident.inspect(),
                                    None,
                                )
                            })?;
                        let name = Str::from(format!(
                            "{}{}{}",
                            vi.vis.def_namespace,
                            vi.vis.modifier.display_as_accessor(),
                            ident.inspect()
                        ));
                        namespaces.insert(name);
                    }
                    Ok(VisibilityModifier::Restricted(namespaces))
                }
                VisRestriction::SubtypeOf(typ) => {
                    let t = self.instantiate_typespec(typ).map_err(|(_, es)| es)?;
                    Ok(VisibilityModifier::SubtypeRestricted(t))
                }
            },
        }
    }

    pub(crate) fn expr_to_type(&self, expr: ast::Expr) -> Failable<Type> {
        let t_spec = Parser::expr_to_type_spec(expr).map_err(|err| {
            let err = TyCheckError::new(err.into(), self.cfg.input.clone(), self.caused_by());
            (Type::Failure, TyCheckErrors::from(err))
        })?;
        self.instantiate_typespec(&t_spec)
    }

    pub(crate) fn expr_to_value(&self, expr: ast::Expr) -> Failable<ValueObj> {
        let const_expr = Parser::validate_const_expr(expr).map_err(|err| {
            let err = TyCheckError::new(err.into(), self.cfg.input.clone(), self.caused_by());
            (ValueObj::Failure, TyCheckErrors::from(err))
        })?;
        let mut dummy = TyVarCache::new(self.level, self);
        match self.instantiate_const_expr(&const_expr, None, &mut dummy, false) {
            Ok(tp) => ValueObj::try_from(tp).map_err(|_| {
                let err = TyCheckError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    const_expr.loc(),
                    self.caused_by(),
                );
                (ValueObj::Failure, TyCheckErrors::from(err))
            }),
            Err((tp, mut errs)) => match ValueObj::try_from(tp) {
                Ok(value) => Err((value, errs)),
                Err(_) => {
                    let err = TyCheckError::not_const_expr(
                        self.cfg.input.clone(),
                        line!() as usize,
                        const_expr.loc(),
                        self.caused_by(),
                    );
                    errs.push(err);
                    Err((ValueObj::Failure, errs))
                }
            },
        }
    }

    pub(crate) fn expr_to_tp(&self, expr: ast::Expr) -> Failable<TyParam> {
        let const_expr = Parser::validate_const_expr(expr).map_err(|err| {
            let err = TyCheckError::new(err.into(), self.cfg.input.clone(), self.caused_by());
            (TyParam::Failure, TyCheckErrors::from(err))
        })?;
        let mut dummy = TyVarCache::new(self.level, self);
        self.instantiate_const_expr(&const_expr, None, &mut dummy, false)
    }
}
