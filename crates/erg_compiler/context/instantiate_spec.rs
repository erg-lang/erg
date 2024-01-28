use std::option::Option; // conflicting to Type::Option

#[allow(unused)]
use erg_common::log;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{assume_unreachable, dict, set, try_map_mut};

use ast::{
    NonDefaultParamSignature, ParamTySpec, PreDeclTypeSpec, TypeBoundSpec, TypeBoundSpecs, TypeSpec,
};
use erg_parser::ast::{
    self, ConstApp, ConstArgs, ConstArray, ConstExpr, ConstSet, Identifier, VarName,
    VisModifierSpec, VisRestriction,
};
use erg_parser::token::TokenKind;
use erg_parser::Parser;

use crate::ty::free::{CanbeFree, Constraint, HasLevel};
use crate::ty::typaram::{IntervalOp, OpKind, TyParam, TyParamLambda, TyParamOrdering};
use crate::ty::value::ValueObj;
use crate::ty::{constructors::*, Predicate, RefinementType, VisibilityModifier};
use crate::ty::{Field, HasType, ParamTy, SubrKind, SubrType, Type};
use crate::type_feature_error;
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
                tv_cache.push_or_init_tyvar(name, &tv, self);
                Ok(())
            }
            TypeBoundSpec::NonDefault { lhs, spec } => {
                let constr = match spec.op.kind {
                    TokenKind::SubtypeOf => Constraint::new_subtype_of(
                        self.instantiate_typespec_full(&spec.t_spec, None, tv_cache, mode, true)?,
                    ),
                    TokenKind::SupertypeOf => Constraint::new_supertype_of(
                        self.instantiate_typespec_full(&spec.t_spec, None, tv_cache, mode, true)?,
                    ),
                    TokenKind::Colon => Constraint::new_type_of(self.instantiate_typespec_full(
                        &spec.t_spec,
                        None,
                        tv_cache,
                        mode,
                        true,
                    )?),
                    _ => unreachable!(),
                };
                if constr.get_sub_sup().is_none() {
                    let tp = TyParam::named_free_var(lhs.inspect().clone(), self.level, constr);
                    tv_cache.push_or_init_typaram(lhs, &tp, self);
                } else {
                    let tv = named_free_var(lhs.inspect().clone(), self.level, constr);
                    tv_cache.push_or_init_tyvar(lhs, &tv, self);
                }
                Ok(())
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
    ) -> TyCheckResult<TyVarCache> {
        let mut tv_cache = TyVarCache::new(self.level, self);
        for bound in bounds.iter() {
            self.instantiate_ty_bound(bound, &mut tv_cache, mode)?;
        }
        for tv in tv_cache.tyvar_instances.values() {
            if tv.constraint().map(|c| c.is_uninited()).unwrap_or(false) {
                return Err(TyCheckErrors::from(TyCheckError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    bounds.loc(),
                    self.caused_by(),
                    &tv.local_name(),
                    self.get_similar_name(&tv.local_name()),
                )));
            }
        }
        for tp in tv_cache.typaram_instances.values() {
            if tp.constraint().map(|c| c.is_uninited()).unwrap_or(false) {
                return Err(TyCheckErrors::from(TyCheckError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    bounds.loc(),
                    self.caused_by(),
                    &tp.to_string(),
                    self.get_similar_name(&tp.to_string()),
                )));
            }
        }
        Ok(tv_cache)
    }

    pub(crate) fn instantiate_var_sig_t(
        &self,
        t_spec: Option<&TypeSpec>,
        mode: RegistrationMode,
    ) -> TyCheckResult<Type> {
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
        default_ts: Vec<Type>,
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
                let err = TyCheckError::unreachable(
                    self.cfg.input.clone(),
                    "instantiate_sub_sig_t",
                    line!(),
                );
                return Err((other, TyCheckErrors::from(err)));
            }
            None => None,
        };
        let mut tmp_tv_cache = self
            .instantiate_ty_bounds(&sig.bounds, PreRegister)
            .map_err(|errs| (Type::Failure, errs))?;
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
        let var_args = if let Some(var_args) = sig.params.var_params.as_ref() {
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
        for ((n, p), default_t) in sig.params.defaults.iter().enumerate().zip(default_ts) {
            let opt_decl_t = opt_decl_sig_t
                .as_ref()
                .and_then(|subr| subr.default_params.get(n));
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
        let kw_var_args = if let Some(kw_var_args) = sig.params.kw_var_params.as_ref() {
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
                Ok(ty) => ty,
                Err(es) => {
                    errs.extend(es);
                    Type::Failure
                }
            }
        } else {
            // preregisterならouter scopeで型宣言(see inference.md)
            let level = if mode == PreRegister {
                self.level
            } else {
                self.level + 1
            };
            free_var(level, Constraint::new_type_of(Type))
        };
        // tmp_tv_cache.warn_isolated_vars(self);
        let typ = if sig.ident.is_procedural() {
            proc(non_defaults, var_args, defaults, kw_var_args, spec_return_t)
        } else {
            func(non_defaults, var_args, defaults, kw_var_args, spec_return_t)
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
        let gen_free_t = || {
            let level = if mode == PreRegister {
                self.level
            } else {
                self.level + 1
            };
            free_var(level, Constraint::new_type_of(Type))
        };
        let spec_t = if let Some(spec_with_op) = &sig.t_spec {
            self.instantiate_typespec_full(
                &spec_with_op.t_spec,
                opt_decl_t,
                tmp_tv_cache,
                mode,
                not_found_is_qvar,
            )
            .map_err(|errs| (Type::Failure, errs))?
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
                // TODO: Array<Lit>
                _ => gen_free_t(),
            }
        };
        if let Some(decl_pt) = opt_decl_t {
            if kind.is_var_params() {
                let spec_t = unknown_len_array_t(spec_t.clone());
                self.sub_unify(
                    decl_pt.typ(),
                    &spec_t,
                    &sig.t_spec.as_ref().ok_or(sig),
                    None,
                )
                .map_err(|errs| (spec_t, errs))?;
            } else {
                self.sub_unify(
                    decl_pt.typ(),
                    &spec_t,
                    &sig.t_spec.as_ref().ok_or(sig),
                    None,
                )
                .map_err(|errs| (spec_t.clone(), errs))?;
            }
        }
        Ok(spec_t)
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
    ) -> TyCheckResult<Type> {
        self.inc_ref_predecl_typespec(predecl, self, tmp_tv_cache);
        match predecl {
            ast::PreDeclTypeSpec::Mono(simple) => {
                self.instantiate_mono_t(simple, opt_decl_t, tmp_tv_cache, not_found_is_qvar)
            }
            ast::PreDeclTypeSpec::Poly(poly) => match &poly.acc {
                ast::ConstAccessor::Local(local) => self.instantiate_local_poly_t(
                    local,
                    &poly.args,
                    opt_decl_t,
                    tmp_tv_cache,
                    not_found_is_qvar,
                ),
                ast::ConstAccessor::Attr(attr) => {
                    let ctxs = self.get_singular_ctxs(&attr.obj.clone().downgrade(), self)?;
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
                    Err(TyCheckErrors::from(TyCheckError::no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        attr.loc(),
                        self.caused_by(),
                        attr.name.inspect(),
                        self.get_similar_name(attr.name.inspect()),
                    )))
                }
                _ => type_feature_error!(self, poly.loc(), &format!("instantiating type {poly}")),
            },
            ast::PreDeclTypeSpec::Attr { namespace, t } => {
                if let Ok(receiver) = Parser::validate_const_expr(namespace.as_ref().clone()) {
                    if let Ok(receiver_t) = self.instantiate_const_expr_as_type(
                        &receiver,
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    ) {
                        return self.eval_proj(
                            receiver_t,
                            t.inspect().clone(),
                            self.level,
                            predecl,
                        );
                    }
                }
                let ctxs = self.get_singular_ctxs(namespace.as_ref(), self)?;
                for ctx in ctxs {
                    if let Some(ctx) = ctx.rec_local_get_type(t.inspect()) {
                        // TODO: visibility check
                        return Ok(ctx.typ.clone());
                    }
                }
                Err(TyCheckErrors::from(TyCheckError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    t.loc(),
                    self.caused_by(),
                    t.inspect(),
                    self.get_similar_name(t.inspect()),
                )))
            }
            other => type_feature_error!(self, other.loc(), &format!("instantiating type {other}")),
        }
    }

    pub(crate) fn instantiate_mono_t(
        &self,
        ident: &Identifier,
        opt_decl_t: Option<&ParamTy>,
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
                if let Some(outer) = &self.outer {
                    if let Ok(t) =
                        outer.instantiate_mono_t(ident, opt_decl_t, tmp_tv_cache, not_found_is_qvar)
                    {
                        return Ok(t);
                    }
                }
                if let Some(typ) = self
                    .consts
                    .get(ident.inspect())
                    .and_then(|v| self.convert_value_into_type(v.clone()).ok())
                {
                    if let Some((_, vi)) = self.get_var_info(ident.inspect()) {
                        self.inc_ref(ident.inspect(), vi, ident, self);
                    }
                    Ok(typ)
                } else if let Some(ctx) = self.get_type_ctx(ident.inspect()) {
                    if let Some((_, vi)) = self.get_var_info(ident.inspect()) {
                        self.inc_ref(ident.inspect(), vi, ident, self);
                    }
                    Ok(ctx.typ.clone())
                } else if not_found_is_qvar {
                    let tyvar = named_free_var(Str::rc(other), self.level, Constraint::Uninited);
                    tmp_tv_cache.push_or_init_tyvar(&ident.name, &tyvar, self);
                    Ok(tyvar)
                } else if let Some(decl_t) = opt_decl_t {
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
        _opt_decl_t: Option<&ParamTy>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> TyCheckResult<Type> {
        match name.inspect().trim_start_matches([':', '.']) {
            "Array" => {
                let ctx = &self
                    .get_nominal_type_ctx(&array_t(Type::Obj, TyParam::Failure))
                    .unwrap()
                    .ctx;
                // TODO: kw
                let mut pos_args = args.pos_args();
                if let Some(first) = pos_args.next() {
                    let t = self.instantiate_const_expr_as_type(
                        &first.expr,
                        Some((ctx, 0)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    let len = if let Some(len) = pos_args.next() {
                        self.instantiate_const_expr(
                            &len.expr,
                            Some((ctx, 1)),
                            tmp_tv_cache,
                            not_found_is_qvar,
                        )?
                    } else {
                        TyParam::erased(Nat)
                    };
                    Ok(array_t(t, len))
                } else {
                    Ok(mono("GenericArray"))
                }
            }
            "Ref" => {
                let mut pos_args = args.pos_args();
                let Some(first) = pos_args.next() else {
                    return Err(TyCheckErrors::from(TyCheckError::args_missing_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        args.loc(),
                        "Ref",
                        self.caused_by(),
                        vec![Str::from("T")],
                    )));
                };
                let t = self.instantiate_const_expr_as_type(
                    &first.expr,
                    None,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                Ok(ref_(t))
            }
            "RefMut" => {
                // TODO after
                let mut pos_args = args.pos_args();
                let Some(first) = pos_args.next() else {
                    return Err(TyCheckErrors::from(TyCheckError::args_missing_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        args.loc(),
                        "RefMut",
                        self.caused_by(),
                        vec![Str::from("T")],
                    )));
                };
                let t = self.instantiate_const_expr_as_type(
                    &first.expr,
                    None,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                Ok(ref_mut(t, None))
            }
            "Structural" => {
                let mut pos_args = args.pos_args();
                let Some(first) = pos_args.next() else {
                    return Err(TyCheckErrors::from(TyCheckError::args_missing_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        args.loc(),
                        "Structural",
                        self.caused_by(),
                        vec![Str::from("Type")],
                    )));
                };
                let t = self.instantiate_const_expr_as_type(
                    &first.expr,
                    None,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                Ok(t.structuralize())
            }
            "NamedTuple" => {
                let mut pose_args = args.pos_args();
                let Some(first) = pose_args.next() else {
                    return Err(TyCheckErrors::from(TyCheckError::args_missing_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        args.loc(),
                        "NamedTuple",
                        self.caused_by(),
                        vec![Str::from("Fields")],
                    )));
                };
                let ConstExpr::Record(fields) = &first.expr else {
                    return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
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
                    )));
                };
                let mut ts = vec![];
                for def in fields.attrs.iter() {
                    let t = self.instantiate_const_expr_as_type(
                        &def.body.block[0],
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    let vis = self.instantiate_vis_modifier(&def.ident.vis)?;
                    ts.push((Field::new(vis, def.ident.inspect().clone()), t));
                }
                Ok(Type::NamedTuple(ts))
            }
            other => {
                let Some(ctx) = self.get_type_ctx(&Str::rc(other)) else {
                    return Err(TyCheckErrors::from(TyCheckError::no_type_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        name.loc(),
                        self.caused_by(),
                        other,
                        self.get_similar_name(other),
                    )));
                };
                // FIXME: kw args
                let mut new_params = vec![];
                for ((i, arg), (name, param_vi)) in
                    args.pos_args().enumerate().zip(ctx.params.iter())
                {
                    let param = self.instantiate_const_expr(
                        &arg.expr,
                        Some((ctx, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    );
                    let param = param.or_else(|e| {
                        if not_found_is_qvar {
                            let name = arg.expr.to_string();
                            // FIXME: handle `::` as a right way
                            let name = Str::rc(name.trim_start_matches("::"));
                            let tp = TyParam::named_free_var(
                                name.clone(),
                                self.level,
                                Constraint::Uninited,
                            );
                            let varname = VarName::from_str(name);
                            tmp_tv_cache.push_or_init_typaram(&varname, &tp, self);
                            Ok(tp)
                        } else {
                            Err(e)
                        }
                    })?;
                    let arg_t = self
                        .get_tp_t(&param)
                        .map_err(|err| {
                            log!(err "{err}");
                            err
                        })
                        .unwrap_or(Obj);
                    if self.subtype_of(&arg_t, &param_vi.t) {
                        new_params.push(param);
                    } else {
                        return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            arg.expr.loc(),
                            self.caused_by(),
                            name.as_ref().map_or("", |n| &n.inspect()[..]),
                            Some(i),
                            &param_vi.t,
                            &arg_t,
                            None,
                            None,
                        )));
                    }
                }
                // FIXME: non-builtin
                Ok(poly(ctx.typ.qual_name(), new_params))
            }
        }
    }

    fn instantiate_acc(
        &self,
        acc: &ast::ConstAccessor,
        erased_idx: Option<(&Context, usize)>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> TyCheckResult<TyParam> {
        self.inc_ref_acc(&acc.clone().downgrade(), self, tmp_tv_cache);
        match acc {
            ast::ConstAccessor::Attr(attr) => match self.instantiate_const_expr(
                &attr.obj,
                erased_idx,
                tmp_tv_cache,
                not_found_is_qvar,
            ) {
                Ok(obj) => Ok(obj.proj(attr.name.inspect())),
                Err(errs) => {
                    if let Ok(ctxs) = self.get_singular_ctxs(&attr.obj.clone().downgrade(), self) {
                        for ctx in ctxs {
                            if let Some(value) = ctx.rec_get_const_obj(attr.name.inspect()) {
                                return Ok(TyParam::Value(value.clone()));
                            }
                        }
                    }
                    Err(errs)
                }
            },
            ast::ConstAccessor::Local(local) => {
                self.instantiate_local(local, erased_idx, tmp_tv_cache, local, not_found_is_qvar)
            }
            other => type_feature_error!(
                self,
                other.loc(),
                &format!("instantiating const expression {other}")
            ),
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
            tmp_tv_cache.push_or_init_tyvar(&name.name, &tyvar, self);
            return Ok(TyParam::t(tyvar));
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
    ) -> TyCheckResult<TyParam> {
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
                    let arg_t = self.instantiate_const_expr(
                        &arg.expr,
                        Some((ctx, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    args.push(arg_t);
                }
                if let Some(attr_name) = app.attr_name.as_ref() {
                    Ok(obj.proj_call(attr_name.inspect().clone(), args))
                } else if ctx.kind.is_type() && !ctx.params.is_empty() {
                    Ok(TyParam::t(poly(ctx.name.clone(), args)))
                } else {
                    let ast::ConstExpr::Accessor(ast::ConstAccessor::Local(ident)) =
                        app.obj.as_ref()
                    else {
                        return type_feature_error!(self, app.loc(), "instantiating const callee");
                    };
                    Ok(TyParam::app(ident.inspect().clone(), args))
                }
            }
            Err(errs) => {
                let Some(attr_name) = app.attr_name.as_ref() else {
                    return Err(errs);
                };
                let acc = app.obj.clone().attr(attr_name.clone());
                let attr =
                    self.instantiate_acc(&acc, erased_idx, tmp_tv_cache, not_found_is_qvar)?;
                let ctxs = self.get_singular_ctxs(&acc.downgrade().into(), self)?;
                let ctx = ctxs.first().copied().unwrap_or(self);
                let mut args = vec![];
                for (i, arg) in app.args.pos_args().enumerate() {
                    let arg_t = self.instantiate_const_expr(
                        &arg.expr,
                        Some((ctx, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    args.push(arg_t);
                }
                Ok(TyParam::app(attr.qual_name().unwrap(), args))
            }
        }
    }

    /// erased_index:
    /// e.g. `instantiate_const_expr(Array(Str, _), Some((self, 1))) => Array(Str, _: Nat)`
    pub(crate) fn instantiate_const_expr(
        &self,
        expr: &ast::ConstExpr,
        erased_idx: Option<(&Context, usize)>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> TyCheckResult<TyParam> {
        if let Ok(value) = self.eval_const_expr(&expr.clone().downgrade()) {
            return Ok(TyParam::Value(value));
        }
        match expr {
            ast::ConstExpr::Lit(lit) => Ok(TyParam::Value(self.eval_lit(lit)?)),
            ast::ConstExpr::Accessor(acc) => {
                self.instantiate_acc(acc, erased_idx, tmp_tv_cache, not_found_is_qvar)
            }
            ast::ConstExpr::App(app) => {
                self.instantiate_app(app, erased_idx, tmp_tv_cache, not_found_is_qvar)
            }
            ast::ConstExpr::Array(ConstArray::Normal(array)) => {
                let mut tp_arr = vec![];
                for (i, elem) in array.elems.pos_args().enumerate() {
                    let el = self.instantiate_const_expr(
                        &elem.expr,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    tp_arr.push(el);
                }
                Ok(TyParam::Array(tp_arr))
            }
            ast::ConstExpr::Array(ConstArray::WithLength(arr)) => {
                let elem = self.instantiate_const_expr(
                    &arr.elem,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                let length = self.instantiate_const_expr(
                    &arr.length,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                if length.is_erased() {
                    if let Ok(elem_t) = self.instantiate_tp_as_type(elem, arr) {
                        return Ok(TyParam::t(unknown_len_array_t(elem_t)));
                    }
                }
                type_feature_error!(
                    self,
                    arr.loc(),
                    &format!("instantiating const expression {expr}")
                )
            }
            ast::ConstExpr::Set(ConstSet::Normal(set)) => {
                let mut tp_set = set! {};
                for (i, elem) in set.elems.pos_args().enumerate() {
                    let el = self.instantiate_const_expr(
                        &elem.expr,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    tp_set.insert(el);
                }
                Ok(TyParam::Set(tp_set))
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
                    let pred =
                        self.instantiate_pred_from_expr(set.guard.as_ref().unwrap(), tmp_tv_cache)?;
                    if let Ok(t) = self.instantiate_tp_as_type(iter, set) {
                        return Ok(TyParam::t(refinement(ident.inspect().clone(), t, pred)));
                    }
                }
                type_feature_error!(
                    self,
                    set.loc(),
                    &format!("instantiating const expression {expr}")
                )
            }
            ast::ConstExpr::Dict(dict) => {
                let mut tp_dict = dict! {};
                for (i, elem) in dict.kvs.iter().enumerate() {
                    let key = self.instantiate_const_expr(
                        &elem.key,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    let val = self.instantiate_const_expr(
                        &elem.value,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    tp_dict.insert(key, val);
                }
                Ok(TyParam::Dict(tp_dict))
            }
            ast::ConstExpr::Tuple(tuple) => {
                let mut tp_tuple = vec![];
                for (i, elem) in tuple.elems.pos_args().enumerate() {
                    let el = self.instantiate_const_expr(
                        &elem.expr,
                        Some((self, i)),
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    tp_tuple.push(el);
                }
                Ok(TyParam::Tuple(tp_tuple))
            }
            ast::ConstExpr::Record(rec) => {
                let mut tp_rec = dict! {};
                for attr in rec.attrs.iter() {
                    let field = self.instantiate_field(&attr.ident)?;
                    let val = self.instantiate_const_expr(
                        attr.body.block.get(0).unwrap(),
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?;
                    tp_rec.insert(field, val);
                }
                Ok(TyParam::Record(tp_rec))
            }
            ast::ConstExpr::Lambda(lambda) => {
                let _tmp_tv_cache =
                    self.instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal)?;
                // Since there are type variables and other variables that can be constrained within closures,
                // they are `merge`d once and then `purge`d of type variables that are only used internally after instantiation.
                tmp_tv_cache.merge(&_tmp_tv_cache);
                let mut nd_params = Vec::with_capacity(lambda.sig.params.non_defaults.len());
                for sig in lambda.sig.params.non_defaults.iter() {
                    let pt = self
                        .instantiate_param_ty(
                            sig,
                            None,
                            tmp_tv_cache,
                            RegistrationMode::Normal,
                            ParamKind::NonDefault,
                            not_found_is_qvar,
                        )
                        // TODO: continue
                        .map_err(|(_, errs)| errs)?;
                    nd_params.push(pt);
                }
                let var_params = if let Some(p) = lambda.sig.params.var_params.as_ref() {
                    let pt = self
                        .instantiate_param_ty(
                            p,
                            None,
                            tmp_tv_cache,
                            RegistrationMode::Normal,
                            ParamKind::VarParams,
                            not_found_is_qvar,
                        )
                        .map_err(|(_, errs)| errs)?;
                    Some(pt)
                } else {
                    None
                };
                let mut d_params = Vec::with_capacity(lambda.sig.params.defaults.len());
                for sig in lambda.sig.params.defaults.iter() {
                    let expr = self.eval_const_expr(&sig.default_val)?;
                    let pt = self
                        .instantiate_param_ty(
                            &sig.sig,
                            None,
                            tmp_tv_cache,
                            RegistrationMode::Normal,
                            ParamKind::Default(expr.t()),
                            not_found_is_qvar,
                        )
                        .map_err(|(_, errs)| errs)?;
                    d_params.push(pt);
                }
                let kw_var_params = if let Some(p) = lambda.sig.params.kw_var_params.as_ref() {
                    let pt = self
                        .instantiate_param_ty(
                            p,
                            None,
                            tmp_tv_cache,
                            RegistrationMode::Normal,
                            ParamKind::KwVarParams,
                            not_found_is_qvar,
                        )
                        .map_err(|(_, errs)| errs)?;
                    Some(pt)
                } else {
                    None
                };
                let mut body = vec![];
                for expr in lambda.body.iter() {
                    let param =
                        self.instantiate_const_expr(expr, None, tmp_tv_cache, not_found_is_qvar)?;
                    body.push(param);
                }
                tmp_tv_cache.purge(&_tmp_tv_cache);
                Ok(TyParam::Lambda(TyParamLambda::new(
                    lambda.clone(),
                    nd_params,
                    var_params,
                    d_params,
                    kw_var_params,
                    body,
                )))
            }
            ast::ConstExpr::BinOp(bin) => {
                let Some(op) = token_kind_to_op_kind(bin.op.kind) else {
                    return type_feature_error!(
                        self,
                        bin.loc(),
                        &format!("instantiating const expression {bin}")
                    );
                };
                let lhs = self.instantiate_const_expr(
                    &bin.lhs,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                let rhs = self.instantiate_const_expr(
                    &bin.rhs,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                Ok(TyParam::bin(op, lhs, rhs))
            }
            ast::ConstExpr::UnaryOp(unary) => {
                let Some(op) = token_kind_to_op_kind(unary.op.kind) else {
                    return type_feature_error!(
                        self,
                        unary.loc(),
                        &format!("instantiating const expression {unary}")
                    );
                };
                let val = self.instantiate_const_expr(
                    &unary.expr,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                Ok(TyParam::unary(op, val))
            }
            ast::ConstExpr::TypeAsc(tasc) => {
                let tp = self.instantiate_const_expr(
                    &tasc.expr,
                    erased_idx,
                    tmp_tv_cache,
                    not_found_is_qvar,
                )?;
                let spec_t = self.instantiate_typespec_full(
                    &tasc.t_spec.t_spec,
                    None,
                    tmp_tv_cache,
                    RegistrationMode::Normal,
                    false,
                )?;
                if self.subtype_of(&self.get_tp_t(&tp)?, &spec_t) {
                    Ok(tp)
                } else {
                    Err(TyCheckErrors::from(TyCheckError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &self.get_tp_t(&tp)?,
                        &spec_t,
                        tasc.loc(),
                        self.caused_by(),
                    )))
                }
            }
            other => type_feature_error!(
                self,
                other.loc(),
                &format!("instantiating const expression {other}")
            ),
        }
    }

    pub(crate) fn instantiate_const_expr_as_type(
        &self,
        expr: &ast::ConstExpr,
        erased_idx: Option<(&Context, usize)>,
        tmp_tv_cache: &mut TyVarCache,
        not_found_is_qvar: bool,
    ) -> TyCheckResult<Type> {
        let tp = self.instantiate_const_expr(expr, erased_idx, tmp_tv_cache, not_found_is_qvar)?;
        self.instantiate_tp_as_type(tp, expr)
    }

    fn instantiate_tp_as_type(&self, tp: TyParam, loc: &impl Locational) -> TyCheckResult<Type> {
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
            #[allow(clippy::bind_instead_of_map)]
            TyParam::Value(value) => self.convert_value_into_type(value).or_else(|value| {
                type_feature_error!(self, loc.loc(), &format!("instantiate `{value}` as type"))
            }),
            TyParam::Array(arr) => {
                let len = TyParam::value(arr.len());
                let mut union = Type::Never;
                for tp in arr {
                    let t = self.instantiate_tp_as_type(tp, loc)?;
                    union = self.union(&union, &t);
                }
                Ok(array_t(union, len))
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
                    let t = self.instantiate_tp_as_type(tp, loc)?;
                    tps.push(t);
                }
                Ok(tuple_t(tps))
            }
            TyParam::Record(rec) => {
                let mut rec_t = dict! {};
                for (field, tp) in rec {
                    let t = self.instantiate_tp_as_type(tp, loc)?;
                    rec_t.insert(field, t);
                }
                Ok(Type::Record(rec_t))
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
                ),
            },
            other =>
            {
                #[allow(clippy::bind_instead_of_map)]
                self.convert_tp_into_type(other).or_else(|tp| {
                    type_feature_error!(self, loc.loc(), &format!("instantiate `{tp}` as type"))
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
    ) -> TyCheckResult<ParamTy> {
        let t = self.instantiate_typespec_full(
            &p.ty,
            opt_decl_t,
            tmp_tv_cache,
            mode,
            not_found_is_qvar,
        )?;
        if let Some(default_t) = default_t {
            Ok(ParamTy::kw_default(
                p.name.as_ref().unwrap().inspect().to_owned(),
                t,
                self.instantiate_typespec_full(
                    default_t,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                )?,
            ))
        } else {
            Ok(ParamTy::pos_or_kw(
                p.name.as_ref().map(|t| t.inspect().to_owned()),
                t,
            ))
        }
    }

    fn instantiate_pred_from_expr(
        &self,
        expr: &ast::ConstExpr,
        tmp_tv_cache: &mut TyVarCache,
    ) -> TyCheckResult<Predicate> {
        match expr {
            ast::ConstExpr::Lit(lit) => {
                let value = self.eval_lit(lit)?;
                Ok(Predicate::Value(value))
            }
            ast::ConstExpr::Accessor(ast::ConstAccessor::Local(local)) => {
                self.inc_ref_local(local, self, tmp_tv_cache);
                Ok(Predicate::Const(local.inspect().clone()))
            }
            ast::ConstExpr::App(app) => {
                let receiver = self.instantiate_const_expr(&app.obj, None, tmp_tv_cache, false)?;
                let name = app.attr_name.as_ref().map(|n| n.inspect().to_owned());
                let mut args = vec![];
                for arg in app.args.pos_args() {
                    let arg = self.instantiate_const_expr(&arg.expr, None, tmp_tv_cache, false)?;
                    args.push(arg);
                }
                Ok(Predicate::Call {
                    receiver,
                    name,
                    args,
                })
            }
            ast::ConstExpr::BinOp(bin) => {
                let lhs = self.instantiate_pred_from_expr(&bin.lhs, tmp_tv_cache)?;
                let rhs = self.instantiate_pred_from_expr(&bin.rhs, tmp_tv_cache)?;
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
                                );
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
                                );
                            }
                        };
                        let pred = match bin.op.kind {
                            TokenKind::DblEq => Predicate::eq(var, rhs),
                            TokenKind::NotEq => Predicate::ne(var, rhs),
                            TokenKind::Less => Predicate::lt(var, rhs),
                            TokenKind::LessEq => Predicate::le(var, rhs),
                            TokenKind::Gre => Predicate::gt(var, rhs),
                            TokenKind::GreEq => Predicate::ge(var, rhs),
                            _ => unreachable!(),
                        };
                        Ok(pred)
                    }
                    TokenKind::OrOp => Ok(lhs | rhs),
                    TokenKind::AndOp => Ok(lhs & rhs),
                    _ => type_feature_error!(
                        self,
                        bin.loc(),
                        &format!("instantiating predicate `{expr}`")
                    ),
                }
            }
            ast::ConstExpr::UnaryOp(unop) => {
                let pred = self.instantiate_pred_from_expr(&unop.expr, tmp_tv_cache)?;
                match unop.op.kind {
                    TokenKind::PreBitNot => Ok(!pred),
                    _ => type_feature_error!(
                        self,
                        unop.loc(),
                        &format!("instantiating predicate `{expr}`")
                    ),
                }
            }
            _ => type_feature_error!(
                self,
                expr.loc(),
                &format!("instantiating predicate `{expr}`")
            ),
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
    ) -> TyCheckResult<Type> {
        match t_spec {
            TypeSpec::Infer(_) => Ok(free_var(self.level, Constraint::new_type_of(Type))),
            TypeSpec::PreDeclTy(predecl) => Ok(self.instantiate_predecl_t(
                predecl,
                opt_decl_t,
                tmp_tv_cache,
                not_found_is_qvar,
            )?),
            TypeSpec::And(lhs, rhs) => Ok(self.intersection(
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
            TypeSpec::Array(arr) => {
                let elem_t = self.instantiate_typespec_full(
                    &arr.ty,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                )?;
                let mut len =
                    self.instantiate_const_expr(&arr.len, None, tmp_tv_cache, not_found_is_qvar)?;
                if let TyParam::Erased(t) = &mut len {
                    *t.as_mut() = Type::Nat;
                }
                Ok(array_t(elem_t, len))
            }
            TypeSpec::SetWithLen(set) => {
                let elem_t = self.instantiate_typespec_full(
                    &set.ty,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                )?;
                let mut len =
                    self.instantiate_const_expr(&set.len, None, tmp_tv_cache, not_found_is_qvar)?;
                if let TyParam::Erased(t) = &mut len {
                    *t.as_mut() = Type::Nat;
                }
                Ok(set_t(elem_t, len))
            }
            TypeSpec::Tuple(tup) => {
                let mut inst_tys = vec![];
                for spec in tup.tys.iter() {
                    inst_tys.push(self.instantiate_typespec_full(
                        spec,
                        opt_decl_t,
                        tmp_tv_cache,
                        mode,
                        not_found_is_qvar,
                    )?);
                }
                Ok(tuple_t(inst_tys))
            }
            TypeSpec::Dict(dict) => {
                let mut inst_tys = dict! {};
                for (k, v) in dict.kvs.iter() {
                    inst_tys.insert(
                        self.instantiate_typespec_full(
                            k,
                            opt_decl_t,
                            tmp_tv_cache,
                            mode,
                            not_found_is_qvar,
                        )?,
                        self.instantiate_typespec_full(
                            v,
                            opt_decl_t,
                            tmp_tv_cache,
                            mode,
                            not_found_is_qvar,
                        )?,
                    );
                }
                Ok(dict_t(inst_tys.into()))
            }
            TypeSpec::Record(rec) => {
                let mut inst_tys = dict! {};
                for (k, v) in rec.attrs.iter() {
                    inst_tys.insert(
                        self.instantiate_field(k)?,
                        self.instantiate_typespec_full(
                            v,
                            opt_decl_t,
                            tmp_tv_cache,
                            mode,
                            not_found_is_qvar,
                        )?,
                    );
                }
                Ok(Type::Record(inst_tys))
            }
            // TODO: エラー処理(リテラルでない)はパーサーにやらせる
            TypeSpec::Enum(set) => {
                let mut new_set = set! {};
                for arg in set.pos_args() {
                    new_set.insert(self.instantiate_const_expr(
                        &arg.expr,
                        None,
                        tmp_tv_cache,
                        not_found_is_qvar,
                    )?);
                }
                let ty = new_set.iter().fold(Type::Never, |t, tp| {
                    self.union(&t, &self.get_tp_t(tp).unwrap().derefine())
                });
                Ok(tp_enum(ty, new_set))
            }
            TypeSpec::Interval { op, lhs, rhs } => {
                let op = match op.kind {
                    TokenKind::Closed => IntervalOp::Closed,
                    TokenKind::LeftOpen => IntervalOp::LeftOpen,
                    TokenKind::RightOpen => IntervalOp::RightOpen,
                    TokenKind::Open => IntervalOp::Open,
                    _ => assume_unreachable!(),
                };
                let l = self.instantiate_const_expr(lhs, None, tmp_tv_cache, not_found_is_qvar)?;
                let l = self.eval_tp(l)?;
                let r = self.instantiate_const_expr(rhs, None, tmp_tv_cache, not_found_is_qvar)?;
                let r = self.eval_tp(r)?;
                if let Some(Greater) = self.try_cmp(&l, &r) {
                    panic!("{l}..{r} is not a valid interval type (should be lhs <= rhs)")
                }
                Ok(int_interval(op, l, r))
            }
            TypeSpec::Subr(subr) => {
                let mut inner_tv_ctx = if !subr.bounds.is_empty() {
                    let tv_cache = self.instantiate_ty_bounds(&subr.bounds, mode)?;
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
                let non_defaults = try_map_mut(subr.non_defaults.iter(), |p| {
                    self.instantiate_func_param_spec(
                        p,
                        opt_decl_t,
                        None,
                        tmp_tv_ctx,
                        mode,
                        not_found_is_qvar,
                    )
                })?;
                let var_params = subr
                    .var_params
                    .as_ref()
                    .map(|p| {
                        self.instantiate_func_param_spec(
                            p,
                            opt_decl_t,
                            None,
                            tmp_tv_ctx,
                            mode,
                            not_found_is_qvar,
                        )
                    })
                    .transpose()?;
                let defaults = try_map_mut(subr.defaults.iter(), |p| {
                    self.instantiate_func_param_spec(
                        &p.param,
                        opt_decl_t,
                        Some(&p.default),
                        tmp_tv_ctx,
                        mode,
                        not_found_is_qvar,
                    )
                })?
                .into_iter()
                .collect();
                let kw_var_params = subr
                    .kw_var_params
                    .as_ref()
                    .map(|p| {
                        self.instantiate_func_param_spec(
                            p,
                            opt_decl_t,
                            None,
                            tmp_tv_ctx,
                            mode,
                            not_found_is_qvar,
                        )
                    })
                    .transpose()?;
                let return_t = self.instantiate_typespec_full(
                    &subr.return_t,
                    opt_decl_t,
                    tmp_tv_ctx,
                    mode,
                    not_found_is_qvar,
                )?;
                // no quantification at this point (in `generalize_t`)
                Ok(subr_t(
                    SubrKind::from(subr.arrow.kind),
                    non_defaults,
                    var_params,
                    defaults,
                    kw_var_params,
                    return_t,
                ))
            }
            TypeSpec::TypeApp { spec, args } => {
                type_feature_error!(
                    self,
                    t_spec.loc(),
                    &format!("instantiating type spec {spec}{args}")
                )
            }
            TypeSpec::Refinement(refine) => {
                let t = self.instantiate_typespec_full(
                    &refine.typ,
                    opt_decl_t,
                    tmp_tv_cache,
                    mode,
                    not_found_is_qvar,
                )?;
                let name = VarName::new(refine.var.clone());
                tmp_tv_cache.push_refine_var(&name, t.clone(), self);
                let pred = self
                    .instantiate_pred_from_expr(&refine.pred, tmp_tv_cache)
                    .map_err(|err| {
                        tmp_tv_cache.remove(name.inspect());
                        err
                    })?;
                tmp_tv_cache.remove(name.inspect());
                let refine =
                    Type::Refinement(RefinementType::new(refine.var.inspect().clone(), t, pred));
                Ok(refine)
            }
        }
    }

    pub(crate) fn instantiate_typespec(&self, t_spec: &ast::TypeSpec) -> TyCheckResult<Type> {
        let mut dummy_tv_cache = TyVarCache::new(self.level, self);
        self.instantiate_typespec_with_tv_cache(t_spec, &mut dummy_tv_cache)
    }

    pub(crate) fn instantiate_typespec_with_tv_cache(
        &self,
        t_spec: &ast::TypeSpec,
        tv_cache: &mut TyVarCache,
    ) -> TyCheckResult<Type> {
        let t = self.instantiate_typespec_full(
            t_spec,
            None,
            tv_cache,
            RegistrationMode::Normal,
            false,
        )?;
        t.lift();
        let t = self.generalize_t(t);
        Ok(t)
    }

    pub(crate) fn instantiate_field(&self, ident: &Identifier) -> TyCheckResult<Field> {
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        Ok(Field::new(vis, ident.inspect().clone()))
    }

    pub(crate) fn instantiate_vis_modifier(
        &self,
        spec: &VisModifierSpec,
    ) -> TyCheckResult<VisibilityModifier> {
        match spec {
            VisModifierSpec::Auto => unreachable!(),
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
                    let t = self.instantiate_typespec(typ)?;
                    Ok(VisibilityModifier::SubtypeRestricted(t))
                }
            },
        }
    }

    pub(crate) fn expr_to_type(&self, expr: ast::Expr) -> Option<Type> {
        let t_spec = Parser::expr_to_type_spec(expr).ok()?;
        self.instantiate_typespec(&t_spec).ok()
    }

    pub(crate) fn expr_to_value(&self, expr: ast::Expr) -> Option<ValueObj> {
        let const_expr = Parser::validate_const_expr(expr).ok()?;
        let mut dummy = TyVarCache::new(self.level, self);
        self.instantiate_const_expr(&const_expr, None, &mut dummy, false)
            .ok()
            .and_then(|tp| ValueObj::try_from(tp).ok())
    }

    pub(crate) fn expr_to_tp(&self, expr: ast::Expr) -> Option<TyParam> {
        let const_expr = Parser::validate_const_expr(expr).ok()?;
        let mut dummy = TyVarCache::new(self.level, self);
        self.instantiate_const_expr(&const_expr, None, &mut dummy, false)
            .ok()
    }
}
