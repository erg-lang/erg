// (type) getters & validators
use std::option::Option; // conflicting to Type::Option
use std::path::{Path, PathBuf};

use erg_common::config::{ErgConfig, Input};
use erg_common::env::{erg_pystd_path, erg_std_path};
use erg_common::error::{ErrorCore, ErrorKind, Location, SubMessage};
use erg_common::levenshtein::get_similar_name;
use erg_common::set::Set;
use erg_common::traits::{Locational, NoTypeDisplay, Stream};
use erg_common::vis::Visibility;
use erg_common::{enum_unwrap, fmt_option, fmt_slice, log, set, switch_lang};
use erg_common::{option_enum_unwrap, Str};
use Type::*;

use ast::VarName;
use erg_parser::ast::{self, Identifier};
use erg_parser::token::Token;

use crate::ty::constructors::{anon, free_var, func, mono, poly, proc, proj, ref_, subr_t};
use crate::ty::free::Constraint;
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{HasType, ParamTy, SubrKind, SubrType, Type};

use crate::context::instantiate::ConstTemplate;
use crate::context::{Context, RegistrationMode, TyVarCache, TypeRelationInstance, Variance};
use crate::error::{
    binop_to_dname, readable_name, unaryop_to_dname, SingleTyCheckResult, TyCheckError,
    TyCheckErrors, TyCheckResult,
};
use crate::varinfo::{Mutability, VarInfo, VarKind};
use crate::AccessKind;
use crate::{feature_error, hir};
use RegistrationMode::*;
use Visibility::*;

use super::{ContextKind, MethodInfo};

impl Context {
    pub(crate) fn validate_var_sig_t(
        &self,
        ident: &ast::Identifier,
        t_spec: Option<&ast::TypeSpec>,
        body_t: &Type,
        mode: RegistrationMode,
    ) -> TyCheckResult<()> {
        let spec_t = self.instantiate_var_sig_t(t_spec, None, mode)?;
        if self.sub_unify(body_t, &spec_t, ident.loc(), None).is_err() {
            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                None,
                &spec_t,
                body_t,
                self.get_candidates(body_t),
                Self::get_simple_type_mismatch_hint(&spec_t, body_t),
            )));
        }
        Ok(())
    }

    pub(crate) fn get_current_scope_var(&self, name: &str) -> Option<&VarInfo> {
        self.locals
            .get(name)
            .or_else(|| self.decls.get(name))
            .or_else(|| {
                self.params
                    .iter()
                    .find(|(opt_name, _)| {
                        opt_name
                            .as_ref()
                            .map(|n| &n.inspect()[..] == name)
                            .unwrap_or(false)
                    })
                    .map(|(_, vi)| vi)
            })
            .or_else(|| {
                for (_, methods) in self.methods_list.iter() {
                    if let Some(vi) = methods.get_current_scope_var(name) {
                        return Some(vi);
                    }
                }
                None
            })
    }

    pub(crate) fn get_mut_current_scope_var(&mut self, name: &str) -> Option<&mut VarInfo> {
        self.locals
            .get_mut(name)
            .or_else(|| self.decls.get_mut(name))
            .or_else(|| {
                self.params
                    .iter_mut()
                    .find(|(opt_name, _)| {
                        opt_name
                            .as_ref()
                            .map(|n| &n.inspect()[..] == name)
                            .unwrap_or(false)
                    })
                    .map(|(_, vi)| vi)
            })
            .or_else(|| {
                for (_, methods) in self.methods_list.iter_mut() {
                    if let Some(vi) = methods.get_mut_current_scope_var(name) {
                        return Some(vi);
                    }
                }
                None
            })
    }

    pub(crate) fn get_local_kv(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.locals.get_key_value(name)
    }

    pub(crate) fn get_singular_ctx_by_hir_expr(
        &self,
        obj: &hir::Expr,
        namespace: &Str,
    ) -> SingleTyCheckResult<&Context> {
        match obj {
            hir::Expr::Accessor(hir::Accessor::Ident(ident)) => {
                self.get_singular_ctx_by_ident(&ident.clone().downcast(), namespace)
            }
            hir::Expr::Accessor(hir::Accessor::Attr(attr)) => {
                // REVIEW: 両方singularとは限らない?
                let ctx = self.get_singular_ctx_by_hir_expr(&attr.obj, namespace)?;
                let attr = hir::Expr::Accessor(hir::Accessor::Ident(attr.ident.clone()));
                ctx.get_singular_ctx_by_hir_expr(&attr, namespace)
            }
            // TODO: change error
            _ => Err(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                obj.loc(),
                self.caused_by(),
                &obj.to_string(),
                None,
            )),
        }
    }

    pub(crate) fn get_singular_ctx_by_ident(
        &self,
        ident: &ast::Identifier,
        namespace: &Str,
    ) -> SingleTyCheckResult<&Context> {
        self.get_mod(ident.inspect())
            .or_else(|| self.rec_get_type(ident.inspect()).map(|(_, ctx)| ctx))
            .or_else(|| self.rec_get_patch(ident.inspect()))
            .ok_or_else(|| {
                TyCheckError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    ident.loc(),
                    namespace.into(),
                    ident.inspect(),
                    self.get_similar_name(ident.inspect()),
                )
            })
    }

    pub(crate) fn get_mut_singular_ctx_by_ident(
        &mut self,
        ident: &ast::Identifier,
        namespace: &Str,
    ) -> SingleTyCheckResult<&mut Context> {
        let err = TyCheckError::no_var_error(
            self.cfg.input.clone(),
            line!() as usize,
            ident.loc(),
            namespace.into(),
            ident.inspect(),
            self.get_similar_name(ident.inspect()),
        );
        self.get_mut_type(ident.inspect())
            .map(|(_, ctx)| ctx)
            .ok_or(err)
    }

    pub(crate) fn get_singular_ctx(
        &self,
        obj: &ast::Expr,
        namespace: &Str,
    ) -> SingleTyCheckResult<&Context> {
        match obj {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) => {
                self.get_singular_ctx_by_ident(ident, namespace)
            }
            ast::Expr::Accessor(ast::Accessor::Attr(attr)) => {
                // REVIEW: 両方singularとは限らない?
                let ctx = self.get_singular_ctx(&attr.obj, namespace)?;
                let attr = ast::Expr::Accessor(ast::Accessor::Ident(attr.ident.clone()));
                ctx.get_singular_ctx(&attr, namespace)
            }
            _ => Err(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                obj.loc(),
                self.caused_by(),
                &obj.to_string(),
                None,
            )),
        }
    }

    pub(crate) fn get_mut_singular_ctx(
        &mut self,
        obj: &ast::Expr,
        namespace: &Str,
    ) -> SingleTyCheckResult<&mut Context> {
        match obj {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) => {
                self.get_mut_singular_ctx_by_ident(ident, namespace)
            }
            ast::Expr::Accessor(ast::Accessor::Attr(attr)) => {
                // REVIEW: 両方singularとは限らない?
                let ctx = self.get_mut_singular_ctx(&attr.obj, namespace)?;
                let attr = ast::Expr::Accessor(ast::Accessor::Ident(attr.ident.clone()));
                ctx.get_mut_singular_ctx(&attr, namespace)
            }
            _ => Err(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                obj.loc(),
                self.caused_by(),
                &obj.to_string(),
                None,
            )),
        }
    }

    fn get_match_call_t(
        &self,
        kind: SubrKind,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<VarInfo> {
        if !kw_args.is_empty() {
            // TODO: this error desc is not good
            return Err(TyCheckErrors::from(TyCheckError::default_param_error(
                self.cfg.input.clone(),
                line!() as usize,
                kw_args[0].loc(),
                self.caused_by(),
                "match",
            )));
        }
        for pos_arg in pos_args.iter().skip(1) {
            let t = pos_arg.expr.ref_t();
            // Allow only anonymous functions to be passed as match arguments (for aesthetic reasons)
            if !matches!(&pos_arg.expr, hir::Expr::Lambda(_)) {
                return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    pos_arg.loc(),
                    self.caused_by(),
                    "match",
                    None,
                    &mono("LambdaFunc"),
                    t,
                    self.get_candidates(t),
                    Self::get_simple_type_mismatch_hint(&mono("LambdaFunc"), t),
                )));
            }
        }
        let match_target_expr_t = pos_args[0].expr.ref_t();
        // Never or T => T
        let mut union_pat_t = Type::Never;
        for (i, pos_arg) in pos_args.iter().skip(1).enumerate() {
            let lambda = erg_common::enum_unwrap!(&pos_arg.expr, hir::Expr::Lambda); // already checked
            if !lambda.params.defaults.is_empty() {
                return Err(TyCheckErrors::from(TyCheckError::default_param_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    pos_args[i + 1].loc(),
                    self.caused_by(),
                    "match",
                )));
            }
            if lambda.params.len() != 1 {
                return Err(TyCheckErrors::from(TyCheckError::param_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    pos_args[i + 1].loc(),
                    self.caused_by(),
                    1,
                    lambda.params.len(),
                )));
            }
            let mut dummy_tv_cache = TyVarCache::new(self.level, self);
            let rhs = self.instantiate_param_sig_t(
                &lambda.params.non_defaults[0],
                None,
                &mut dummy_tv_cache,
                Normal,
            )?;
            union_pat_t = self.union(&union_pat_t, &rhs);
        }
        // NG: expr_t: Nat, union_pat_t: {1, 2}
        // OK: expr_t: Int, union_pat_t: {1} or 'T
        if self
            .sub_unify(match_target_expr_t, &union_pat_t, pos_args[0].loc(), None)
            .is_err()
        {
            return Err(TyCheckErrors::from(TyCheckError::match_error(
                self.cfg.input.clone(),
                line!() as usize,
                pos_args[0].loc(),
                self.caused_by(),
                match_target_expr_t,
            )));
        }
        let branch_ts = pos_args
            .iter()
            .skip(1)
            .map(|a| ParamTy::anonymous(a.expr.ref_t().clone()))
            .collect::<Vec<_>>();
        let mut return_t = branch_ts[0]
            .typ()
            .return_t()
            .unwrap_or_else(|| todo!("{}", branch_ts[0]))
            .clone();
        for arg_t in branch_ts.iter().skip(1) {
            return_t = self.union(&return_t, arg_t.typ().return_t().unwrap());
        }
        let param_ty = ParamTy::anonymous(match_target_expr_t.clone());
        let param_ts = [vec![param_ty], branch_ts.to_vec()].concat();
        let t = if kind.is_func() {
            func(param_ts, None, vec![], return_t)
        } else {
            proc(param_ts, None, vec![], return_t)
        };
        Ok(VarInfo {
            t,
            ..VarInfo::default()
        })
    }

    pub(crate) fn rec_get_var_info(
        &self,
        ident: &Identifier,
        acc_kind: AccessKind,
        input: &Input,
        namespace: &Str,
    ) -> SingleTyCheckResult<VarInfo> {
        if let Some(vi) = self.get_current_scope_var(&ident.inspect()[..]) {
            match self.validate_visibility(ident, vi, input, namespace) {
                Ok(()) => {
                    return Ok(vi.clone());
                }
                Err(err) => {
                    if !acc_kind.is_local() {
                        return Err(err);
                    }
                }
            }
        } else if let Some((name, _vi)) = self
            .future_defined_locals
            .get_key_value(&ident.inspect()[..])
        {
            return Err(TyCheckError::access_before_def_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                namespace.into(),
                ident.inspect(),
                name.ln_begin().unwrap_or(0),
                self.get_similar_name(ident.inspect()),
            ));
        } else if let Some((name, _vi)) = self.deleted_locals.get_key_value(&ident.inspect()[..]) {
            return Err(TyCheckError::access_deleted_var_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                namespace.into(),
                ident.inspect(),
                name.ln_begin().unwrap_or(0),
                self.get_similar_name(ident.inspect()),
            ));
        }
        if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
            match parent.rec_get_var_info(ident, acc_kind, input, namespace) {
                Ok(vi) => Ok(vi),
                Err(err) if err.core.kind == ErrorKind::DummyError => {
                    Err(TyCheckError::no_var_error(
                        input.clone(),
                        line!() as usize,
                        ident.loc(),
                        namespace.into(),
                        ident.inspect(),
                        self.get_similar_name(ident.inspect()),
                    ))
                }
                Err(err) => Err(err),
            }
        } else {
            Err(TyCheckError::dummy(
                self.cfg.input.clone(),
                line!() as usize,
            ))
        }
    }

    pub(crate) fn rec_get_decl_info(
        &self,
        ident: &Identifier,
        acc_kind: AccessKind,
        input: &Input,
        namespace: &Str,
    ) -> SingleTyCheckResult<VarInfo> {
        if let Some(vi) = self.decls.get(&ident.inspect()[..]) {
            match self.validate_visibility(ident, vi, input, namespace) {
                Ok(()) => {
                    return Ok(vi.clone());
                }
                Err(err) => {
                    if !acc_kind.is_local() {
                        return Err(err);
                    }
                }
            }
        }
        if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
            return parent.rec_get_decl_info(ident, acc_kind, input, namespace);
        }
        Err(TyCheckError::no_var_error(
            input.clone(),
            line!() as usize,
            ident.loc(),
            namespace.into(),
            ident.inspect(),
            self.get_similar_name(ident.inspect()),
        ))
    }

    pub(crate) fn rec_get_attr_info(
        &self,
        obj: &hir::Expr,
        ident: &Identifier,
        input: &Input,
        namespace: &Str,
    ) -> SingleTyCheckResult<VarInfo> {
        let self_t = obj.t();
        let name = ident.name.token();
        match self.get_attr_info_from_attributive(&self_t, ident, namespace) {
            Ok(vi) => {
                return Ok(vi);
            }
            Err(e) if e.core.kind == ErrorKind::DummyError => {}
            Err(e) => {
                return Err(e);
            }
        }
        if let Ok(singular_ctx) = self.get_singular_ctx_by_hir_expr(obj, namespace) {
            match singular_ctx.rec_get_var_info(ident, AccessKind::Attr, input, namespace) {
                Ok(vi) => {
                    return Ok(vi);
                }
                Err(e) if e.core.kind == ErrorKind::NameError => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }
        match self.get_attr_from_nominal_t(obj, ident, input, namespace) {
            Ok(vi) => {
                return Ok(vi);
            }
            Err(e) if e.core.kind == ErrorKind::DummyError => {}
            Err(e) => {
                return Err(e);
            }
        }
        for patch in self.find_patches_of(obj.ref_t()) {
            if let Some(vi) = patch
                .locals
                .get(ident.inspect())
                .or_else(|| patch.decls.get(ident.inspect()))
            {
                self.validate_visibility(ident, vi, input, namespace)?;
                return Ok(vi.clone());
            }
            for (_, methods_ctx) in patch.methods_list.iter() {
                if let Some(vi) = methods_ctx
                    .locals
                    .get(ident.inspect())
                    .or_else(|| methods_ctx.decls.get(ident.inspect()))
                {
                    self.validate_visibility(ident, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
            parent.rec_get_attr_info(obj, ident, input, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                input.clone(),
                line!() as usize,
                name.loc(),
                namespace.into(),
                &self_t,
                name.inspect(),
                self.get_similar_attr(&self_t, name.inspect()),
            ))
        }
    }

    fn get_attr_from_nominal_t(
        &self,
        obj: &hir::Expr,
        ident: &Identifier,
        input: &Input,
        namespace: &Str,
    ) -> SingleTyCheckResult<VarInfo> {
        let self_t = obj.t();
        if let Some(sups) = self.get_nominal_super_type_ctxs(&self_t) {
            for ctx in sups {
                match ctx.rec_get_var_info(ident, AccessKind::Attr, input, namespace) {
                    Ok(t) => {
                        return Ok(t);
                    }
                    Err(e) if e.core.kind == ErrorKind::NameError => {}
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
        let coerced = self
            .deref_tyvar(obj.t(), Variance::Covariant, Location::Unknown)
            .map_err(|mut es| es.remove(0))?;
        if obj.ref_t() != &coerced {
            for ctx in self.get_nominal_super_type_ctxs(&coerced).ok_or_else(|| {
                TyCheckError::type_not_found(
                    self.cfg.input.clone(),
                    line!() as usize,
                    obj.loc(),
                    self.caused_by(),
                    &coerced,
                )
            })? {
                match ctx.rec_get_var_info(ident, AccessKind::Attr, input, namespace) {
                    Ok(t) => {
                        self.coerce(obj.ref_t());
                        return Ok(t);
                    }
                    Err(e) if e.core.kind == ErrorKind::NameError => {}
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
        Err(TyCheckError::dummy(input.clone(), line!() as usize))
    }

    /// get type from given attributive type (Record).
    /// not ModuleType or ClassType etc.
    fn get_attr_info_from_attributive(
        &self,
        t: &Type,
        ident: &Identifier,
        namespace: &Str,
    ) -> SingleTyCheckResult<VarInfo> {
        match t {
            // (obj: Never).foo: Never
            Type::Never => Ok(VarInfo::ILLEGAL.clone()),
            Type::FreeVar(fv) if fv.is_linked() => {
                self.get_attr_info_from_attributive(&fv.crack(), ident, namespace)
            }
            Type::FreeVar(fv) /* if fv.is_unbound() */ => {
                let sup = fv.get_super().unwrap();
                self.get_attr_info_from_attributive(&sup, ident, namespace)
            }
            Type::Ref(t) => self.get_attr_info_from_attributive(t, ident, namespace),
            Type::RefMut { before, .. } => {
                self.get_attr_info_from_attributive(before, ident, namespace)
            }
            Type::Refinement(refine) => {
                self.get_attr_info_from_attributive(&refine.t, ident, namespace)
            }
            Type::Record(record) => {
                if let Some(attr_t) = record.get(ident.inspect()) {
                    let muty = Mutability::from(&ident.inspect()[..]);
                    let vi = VarInfo::new(
                        attr_t.clone(),
                        muty,
                        Public,
                        VarKind::Builtin,
                        None,
                        None,
                        None,
                    );
                    Ok(vi)
                } else {
                    let t = Type::Record(record.clone());
                    Err(TyCheckError::no_attr_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        ident.loc(),
                        namespace.into(),
                        &t,
                        ident.inspect(),
                        self.get_similar_attr(&t, ident.inspect()),
                    ))
                }
            }
            other => {
                if let Some(v) = self.rec_get_const_obj(&other.local_name()) {
                    match v {
                        ValueObj::Type(TypeObj::Generated(gen)) => self
                            .get_gen_t_require_attr_t(gen, &ident.inspect()[..])
                            .map(|attr_t| {
                                let muty = Mutability::from(&ident.inspect()[..]);
                                VarInfo::new(
                                    attr_t.clone(),
                                    muty,
                                    Public,
                                    VarKind::Builtin,
                                    None,
                                    None,
                                    None,
                                )
                            })
                            .ok_or_else(|| {
                                TyCheckError::dummy(self.cfg.input.clone(), line!() as usize)
                            }),
                        ValueObj::Type(TypeObj::Builtin(_t)) => {
                            // FIXME:
                            Err(TyCheckError::dummy(
                                self.cfg.input.clone(),
                                line!() as usize,
                            ))
                        }
                        _other => Err(TyCheckError::dummy(
                            self.cfg.input.clone(),
                            line!() as usize,
                        )),
                    }
                } else {
                    Err(TyCheckError::dummy(
                        self.cfg.input.clone(),
                        line!() as usize,
                    ))
                }
            }
        }
    }

    // returns callee's type, not the return type
    fn search_callee_info(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        input: &Input,
        namespace: &Str,
    ) -> SingleTyCheckResult<VarInfo> {
        if obj.ref_t() == Type::FAILURE {
            // (...Obj) -> Failure
            return Ok(VarInfo {
                t: Type::Subr(SubrType::new(
                    SubrKind::Func,
                    vec![],
                    Some(ParamTy::pos(None, ref_(Obj))),
                    vec![],
                    Failure,
                )),
                ..VarInfo::default()
            });
        }
        if let Some(attr_name) = attr_name.as_ref() {
            for ctx in self
                .get_nominal_super_type_ctxs(obj.ref_t())
                .ok_or_else(|| {
                    TyCheckError::type_not_found(
                        self.cfg.input.clone(),
                        line!() as usize,
                        obj.loc(),
                        self.caused_by(),
                        obj.ref_t(),
                    )
                })?
            {
                if let Some(vi) = ctx
                    .locals
                    .get(attr_name.inspect())
                    .or_else(|| ctx.decls.get(attr_name.inspect()))
                {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
                for (_, methods_ctx) in ctx.methods_list.iter() {
                    if let Some(vi) = methods_ctx
                        .locals
                        .get(attr_name.inspect())
                        .or_else(|| methods_ctx.decls.get(attr_name.inspect()))
                    {
                        self.validate_visibility(attr_name, vi, input, namespace)?;
                        return Ok(vi.clone());
                    }
                }
            }
            if let Ok(singular_ctx) = self.get_singular_ctx_by_hir_expr(obj, namespace) {
                if let Some(vi) = singular_ctx
                    .locals
                    .get(attr_name.inspect())
                    .or_else(|| singular_ctx.decls.get(attr_name.inspect()))
                {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
                for (_, method_ctx) in singular_ctx.methods_list.iter() {
                    if let Some(vi) = method_ctx
                        .locals
                        .get(attr_name.inspect())
                        .or_else(|| method_ctx.decls.get(attr_name.inspect()))
                    {
                        self.validate_visibility(attr_name, vi, input, namespace)?;
                        return Ok(vi.clone());
                    }
                }
                return Err(TyCheckError::singular_no_attr_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    attr_name.loc(),
                    namespace.into(),
                    obj.qual_name().unwrap_or("?"),
                    obj.ref_t(),
                    attr_name.inspect(),
                    self.get_similar_attr_from_singular(obj, attr_name.inspect()),
                ));
            }
            match self.get_method_type_by_name(attr_name) {
                Ok(method) => {
                    self.sub_unify(obj.ref_t(), &method.definition_type, obj.loc(), None)
                        // HACK: change this func's return type to TyCheckResult<Type>
                        .map_err(|mut errs| errs.remove(0))?;
                    return Ok(method.method_type.clone());
                }
                Err(err) if err.core.kind == ErrorKind::TypeError => {
                    return Err(err);
                }
                _ => {}
            }
            for patch in self.find_patches_of(obj.ref_t()) {
                if let Some(vi) = patch
                    .locals
                    .get(attr_name.inspect())
                    .or_else(|| patch.decls.get(attr_name.inspect()))
                {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
                for (_, methods_ctx) in patch.methods_list.iter() {
                    if let Some(vi) = methods_ctx
                        .locals
                        .get(attr_name.inspect())
                        .or_else(|| methods_ctx.decls.get(attr_name.inspect()))
                    {
                        self.validate_visibility(attr_name, vi, input, namespace)?;
                        return Ok(vi.clone());
                    }
                }
            }
            Err(TyCheckError::no_attr_error(
                self.cfg.input.clone(),
                line!() as usize,
                attr_name.loc(),
                namespace.into(),
                obj.ref_t(),
                attr_name.inspect(),
                self.get_similar_attr(obj.ref_t(), attr_name.inspect()),
            ))
        } else {
            Ok(VarInfo {
                t: obj.t(),
                ..VarInfo::default()
            })
        }
    }

    fn validate_visibility(
        &self,
        ident: &Identifier,
        vi: &VarInfo,
        input: &Input,
        namespace: &str,
    ) -> SingleTyCheckResult<()> {
        if ident.vis() != vi.vis {
            Err(TyCheckError::visibility_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                vi.vis,
            ))
        // check if the private variable is loaded from the other scope
        } else if vi.vis.is_private()
            && &self.name[..] != "<builtins>"
            && &self.name[..] != namespace
            && !namespace.contains(&self.name[..])
        {
            log!(err "{namespace}/{}", self.name);
            Err(TyCheckError::visibility_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                Private,
            ))
        } else {
            Ok(())
        }
    }

    // HACK: dname.loc()はダミーLocationしか返さないので、エラーならop.loc()で上書きする
    fn append_loc_info(&self, e: TyCheckError, loc: Location) -> TyCheckError {
        if e.core.loc == Location::Unknown {
            let mut sub_msges = Vec::new();
            for sub_msg in e.core.sub_messages {
                sub_msges.push(SubMessage::ambiguous_new(loc, sub_msg.msg, sub_msg.hint));
            }
            let core = ErrorCore::new(
                sub_msges,
                e.core.main_message,
                e.core.errno,
                e.core.kind,
                e.core.loc,
            );
            TyCheckError::new(core, self.cfg.input.clone(), e.caused_by)
        } else {
            e
        }
    }

    pub(crate) fn get_binop_t(
        &self,
        op: &Token,
        args: &[hir::PosArg],
        input: &Input,
        namespace: &Str,
    ) -> TyCheckResult<VarInfo> {
        erg_common::debug_power_assert!(args.len() == 2);
        let cont = binop_to_dname(op.inspect());
        let symbol = Token::from_str(op.kind, cont);
        let t = self.rec_get_var_info(
            &Identifier::new(None, VarName::new(symbol.clone())),
            AccessKind::Name,
            input,
            namespace,
        )?;
        let op = hir::Expr::Accessor(hir::Accessor::private(symbol, t));
        self.get_call_t(&op, &None, args, &[], input, namespace)
            .map_err(|errs| {
                let op_ident = enum_unwrap!(op, hir::Expr::Accessor:(hir::Accessor::Ident:(_)));
                let vi = op_ident.vi.clone();
                let lhs = args[0].expr.clone();
                let rhs = args[1].expr.clone();
                let bin = hir::BinOp::new(op_ident.name.into_token(), lhs, rhs, vi);
                let errs = errs
                    .into_iter()
                    .map(|e| self.append_loc_info(e, bin.loc()))
                    .collect();
                TyCheckErrors::new(errs)
            })
    }

    pub(crate) fn get_unaryop_t(
        &self,
        op: &Token,
        args: &[hir::PosArg],
        input: &Input,
        namespace: &Str,
    ) -> TyCheckResult<VarInfo> {
        erg_common::debug_power_assert!(args.len() == 1);
        let cont = unaryop_to_dname(op.inspect());
        let symbol = Token::from_str(op.kind, cont);
        let vi = self.rec_get_var_info(
            &Identifier::new(None, VarName::new(symbol.clone())),
            AccessKind::Name,
            input,
            namespace,
        )?;
        let op = hir::Expr::Accessor(hir::Accessor::private(symbol, vi));
        self.get_call_t(&op, &None, args, &[], input, namespace)
            .map_err(|errs| {
                let op_ident = enum_unwrap!(op, hir::Expr::Accessor:(hir::Accessor::Ident:(_)));
                let vi = op_ident.vi.clone();
                let expr = args[0].expr.clone();
                let unary = hir::UnaryOp::new(op_ident.name.into_token(), expr, vi);
                let errs = errs
                    .into_iter()
                    .map(|e| self.append_loc_info(e, unary.loc()))
                    .collect();
                TyCheckErrors::new(errs)
            })
    }

    /// 可変依存型の変更を伝搬させる
    fn propagate(&self, t: &Type, callee: &hir::Expr) -> TyCheckResult<()> {
        if let Type::Subr(subr) = t {
            if let Some(after) = subr.self_t().and_then(|self_t| {
                if let RefMut { after, .. } = self_t {
                    after.as_ref()
                } else {
                    None
                }
            }) {
                self.reunify(callee.ref_t(), after, callee.loc())?;
            }
        }
        Ok(())
    }

    /// if `obj` has `__call__` method, then the return value is `Some(call_instance)`
    ///
    /// e.g.
    /// ```python
    /// substitute_call(instance: ((?T, ?U) -> ?T), [Int, Str], []) => instance: (Int, Str) -> Int
    /// substitute_call(instance: ((?T, Int) -> ?T), [Int, Nat], []) => instance: (Int, Int) -> Str
    /// substitute_call(instance: ((?M(: Nat)..?N(: Nat)) -> ?M+?N), [1..2], []) => instance: (1..2) -> {3}
    /// substitute_call(instance: ((?L(: Add(?R, ?O)), ?R) -> ?O), [1, 2], []) => instance: (Nat, Nat) -> Nat
    /// substitute_call(instance: ((Failure, ?T) -> ?T), [Int, Int]) => instance: (Failure, Int) -> Int
    /// ```
    fn substitute_call(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        instance: &Type,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<Option<Type>> {
        match instance {
            Type::FreeVar(fv) if fv.is_linked() => {
                self.substitute_call(obj, attr_name, &fv.crack(), pos_args, kw_args)
            }
            Type::FreeVar(fv) => {
                if let Some(attr_name) = attr_name {
                    feature_error!(TyCheckErrors, TyCheckError, self, attr_name.loc(), "")
                } else {
                    let is_procedural = obj
                        .show_acc()
                        .map(|acc| acc.ends_with('!'))
                        .unwrap_or(false);
                    let kind = if is_procedural {
                        SubrKind::Proc
                    } else {
                        SubrKind::Func
                    };
                    let ret_t = free_var(self.level, Constraint::new_type_of(Type));
                    let non_default_params = pos_args.iter().map(|a| anon(a.expr.t())).collect();
                    let subr_t = subr_t(kind, non_default_params, None, vec![], ret_t);
                    fv.link(&subr_t);
                    Ok(None)
                }
            }
            Type::Refinement(refine) => {
                self.substitute_call(obj, attr_name, &refine.t, pos_args, kw_args)
            }
            Type::Subr(subr) => {
                let mut errs = TyCheckErrors::empty();
                let is_method = subr.self_t().is_some();
                let callee = if let Some(ident) = attr_name {
                    if is_method {
                        obj.clone()
                    } else {
                        let attr = hir::Attribute::new(
                            obj.clone(),
                            hir::Identifier::bare(ident.dot.clone(), ident.name.clone()),
                        );
                        hir::Expr::Accessor(hir::Accessor::Attr(attr))
                    }
                } else {
                    obj.clone()
                };
                let params_len = subr.non_default_params.len() + subr.default_params.len();
                if (params_len < pos_args.len() || params_len < pos_args.len() + kw_args.len())
                    && subr.var_params.is_none()
                {
                    return Err(self.gen_too_many_args_error(&callee, subr, pos_args, kw_args));
                }
                let mut passed_params = set! {};
                let non_default_params = if is_method {
                    let mut non_default_params = subr.non_default_params.iter();
                    let self_pt = non_default_params.next().unwrap();
                    if let Err(mut es) =
                        self.sub_unify(obj.ref_t(), self_pt.typ(), obj.loc(), self_pt.name())
                    {
                        errs.append(&mut es);
                    }
                    non_default_params
                } else {
                    subr.non_default_params.iter()
                };
                let non_default_params_len = non_default_params.len();
                let mut nth = 1;
                if pos_args.len() >= non_default_params_len {
                    let (non_default_args, var_args) = pos_args.split_at(non_default_params_len);
                    for (nd_arg, nd_param) in non_default_args.iter().zip(non_default_params) {
                        if let Err(mut es) = self.substitute_pos_arg(
                            &callee,
                            attr_name,
                            &nd_arg.expr,
                            nth,
                            nd_param,
                            &mut passed_params,
                        ) {
                            errs.append(&mut es);
                        }
                        nth += 1;
                    }
                    if let Some(var_param) = subr.var_params.as_ref() {
                        for var_arg in var_args.iter() {
                            if let Err(mut es) = self.substitute_var_arg(
                                &callee,
                                attr_name,
                                &var_arg.expr,
                                nth,
                                var_param,
                            ) {
                                errs.append(&mut es);
                            }
                            nth += 1;
                        }
                    } else {
                        for (arg, pt) in var_args.iter().zip(subr.default_params.iter()) {
                            if let Err(mut es) = self.substitute_pos_arg(
                                &callee,
                                attr_name,
                                &arg.expr,
                                nth,
                                pt,
                                &mut passed_params,
                            ) {
                                errs.append(&mut es);
                            }
                            nth += 1;
                        }
                    }
                    for kw_arg in kw_args.iter() {
                        if let Err(mut es) = self.substitute_kw_arg(
                            &callee,
                            attr_name,
                            kw_arg,
                            nth,
                            subr,
                            &mut passed_params,
                        ) {
                            errs.append(&mut es);
                        }
                        nth += 1;
                    }
                    for not_passed in subr
                        .default_params
                        .iter()
                        .filter(|pt| !passed_params.contains(pt.name().unwrap()))
                    {
                        if let ParamTy::KwWithDefault { ty, default, .. } = &not_passed {
                            if let Err(mut es) =
                                self.sub_unify(default, ty, obj.loc(), not_passed.name())
                            {
                                errs.append(&mut es);
                            }
                        }
                    }
                } else {
                    // pos_args.len() < non_default_params_len
                    let mut params = non_default_params.chain(subr.default_params.iter());
                    for pos_arg in pos_args.iter() {
                        if let Err(mut es) = self.substitute_pos_arg(
                            &callee,
                            attr_name,
                            &pos_arg.expr,
                            nth,
                            params.next().unwrap(),
                            &mut passed_params,
                        ) {
                            errs.append(&mut es);
                        }
                        nth += 1;
                    }
                    for kw_arg in kw_args.iter() {
                        if let Err(mut es) = self.substitute_kw_arg(
                            &callee,
                            attr_name,
                            kw_arg,
                            nth,
                            subr,
                            &mut passed_params,
                        ) {
                            errs.append(&mut es);
                        }
                        nth += 1;
                    }
                    let missing_params = subr
                        .non_default_params
                        .iter()
                        .map(|pt| pt.name().cloned().unwrap_or(Str::ever("_")))
                        .filter(|pt| !passed_params.contains(pt))
                        .collect::<Vec<_>>();
                    if !missing_params.is_empty() {
                        return Err(TyCheckErrors::from(TyCheckError::args_missing_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            callee.loc(),
                            &callee.to_string(),
                            self.caused_by(),
                            missing_params,
                        )));
                    }
                }
                if errs.is_empty() {
                    Ok(None)
                } else {
                    Err(errs)
                }
            }
            other => {
                if let Ok(typ_ctx) = self.get_singular_ctx_by_hir_expr(obj, &self.name) {
                    if let Some(call_vi) = typ_ctx.get_current_scope_var("__call__") {
                        let mut dummy = TyVarCache::new(self.level, self);
                        let instance =
                            self.instantiate_t_inner(call_vi.t.clone(), &mut dummy, obj.loc())?;
                        self.substitute_call(obj, attr_name, &instance, pos_args, kw_args)?;
                        return Ok(Some(instance));
                    }
                }
                let hint = if other == &ClassType {
                    Some(switch_lang! {
                        "japanese" => format!("インスタンスを生成したい場合は、{}.newを使用してください", obj.to_string_notype()),
                        "simplified_chinese" => format!("如果要生成实例，请使用 {}.new", obj.to_string_notype()),
                        "traditional_chinese" => format!("如果要生成實例，請使用 {}.new", obj.to_string_notype()),
                        "english" => format!("If you want to generate an instance, use {}.new", obj.to_string_notype()),
                    })
                } else {
                    None
                };
                if let Some(attr_name) = attr_name {
                    Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        Location::concat(obj, attr_name),
                        self.caused_by(),
                        &(obj.to_string() + &attr_name.to_string()),
                        None,
                        &mono("Callable"),
                        other,
                        self.get_candidates(other),
                        hint,
                    )))
                } else {
                    Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        obj.loc(),
                        self.caused_by(),
                        &obj.to_string(),
                        None,
                        &mono("Callable"),
                        other,
                        self.get_candidates(other),
                        hint,
                    )))
                }
            }
        }
    }

    fn gen_too_many_args_error(
        &self,
        callee: &hir::Expr,
        subr_ty: &SubrType,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckErrors {
        let mut unknown_args = vec![];
        let mut passed_args: Vec<&hir::KwArg> = vec![];
        let mut duplicated_args = vec![];
        for kw_arg in kw_args.iter() {
            if subr_ty
                .non_default_params
                .iter()
                .all(|pt| pt.name() != Some(kw_arg.keyword.inspect()))
                && subr_ty
                    .var_params
                    .as_ref()
                    .map(|pt| pt.name() != Some(kw_arg.keyword.inspect()))
                    .unwrap_or(true)
                && subr_ty
                    .default_params
                    .iter()
                    .all(|pt| pt.name() != Some(kw_arg.keyword.inspect()))
            {
                unknown_args.push(kw_arg);
            }
            if passed_args.iter().any(|a| a.keyword == kw_arg.keyword) {
                duplicated_args.push(kw_arg);
            } else {
                passed_args.push(kw_arg);
            }
        }
        if unknown_args.is_empty() && duplicated_args.is_empty() {
            let params_len = subr_ty.non_default_params.len() + subr_ty.default_params.len();
            TyCheckErrors::from(TyCheckError::too_many_args_error(
                self.cfg.input.clone(),
                line!() as usize,
                callee.loc(),
                &callee.to_string(),
                self.caused_by(),
                params_len,
                pos_args.len(),
                kw_args.len(),
            ))
        } else {
            let unknown_arg_errors = unknown_args.into_iter().map(|arg| {
                TyCheckError::unexpected_kw_arg_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    arg.loc(),
                    &callee.to_string(),
                    self.caused_by(),
                    arg.keyword.inspect(),
                )
            });
            let duplicated_arg_errors = duplicated_args.into_iter().map(|arg| {
                TyCheckError::multiple_args_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    arg.loc(),
                    &callee.to_string(),
                    self.caused_by(),
                    arg.keyword.inspect(),
                )
            });
            unknown_arg_errors.chain(duplicated_arg_errors).collect()
        }
    }

    fn substitute_pos_arg(
        &self,
        callee: &hir::Expr,
        attr_name: &Option<Identifier>,
        arg: &hir::Expr,
        nth: usize,
        param: &ParamTy,
        passed_params: &mut Set<Str>,
    ) -> TyCheckResult<()> {
        let arg_t = arg.ref_t();
        let param_t = &param.typ();
        if let Some(name) = param.name() {
            if passed_params.contains(name) {
                return Err(TyCheckErrors::from(TyCheckError::multiple_args_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    callee.loc(),
                    &callee.to_string(),
                    self.caused_by(),
                    name,
                )));
            } else {
                passed_params.insert(name.clone());
            }
        }
        self.sub_unify(arg_t, param_t, arg.loc(), param.name())
            .map_err(|errs| {
                log!(err "semi-unification failed with {callee}\n{arg_t} !<: {param_t}");
                let name = if let Some(attr) = attr_name {
                    format!("{callee}{attr}")
                } else {
                    callee.show_acc().unwrap_or_default()
                };
                let name = name + "::" + param.name().map(|s| readable_name(&s[..])).unwrap_or("");
                let mut hint = Self::get_call_type_mismatch_hint(
                    callee.ref_t(),
                    attr_name.as_ref().map(|i| &i.inspect()[..]),
                    nth,
                    param_t,
                    arg_t,
                );
                TyCheckErrors::new(
                    errs.into_iter()
                        .map(|e| {
                            TyCheckError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                e.core.loc,
                                e.caused_by,
                                &name[..],
                                Some(nth),
                                param_t,
                                arg_t,
                                self.get_candidates(arg_t),
                                std::mem::take(&mut hint),
                            )
                        })
                        .collect(),
                )
            })?;
        Ok(())
    }

    fn substitute_var_arg(
        &self,
        callee: &hir::Expr,
        attr_name: &Option<Identifier>,
        arg: &hir::Expr,
        nth: usize,
        param: &ParamTy,
    ) -> TyCheckResult<()> {
        let arg_t = arg.ref_t();
        let param_t = &param.typ();
        self.sub_unify(arg_t, param_t, arg.loc(), param.name())
            .map_err(|errs| {
                log!(err "semi-unification failed with {callee}\n{arg_t} !<: {param_t}");
                let name = if let Some(attr) = attr_name {
                    format!("{callee}{attr}")
                } else {
                    callee.show_acc().unwrap_or_default()
                };
                let name = name + "::" + param.name().map(|s| readable_name(&s[..])).unwrap_or("");
                TyCheckErrors::new(
                    errs.into_iter()
                        .map(|e| {
                            TyCheckError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                e.core.loc,
                                e.caused_by,
                                &name[..],
                                Some(nth),
                                param_t,
                                arg_t,
                                self.get_candidates(arg_t),
                                Self::get_simple_type_mismatch_hint(param_t, arg_t),
                            )
                        })
                        .collect(),
                )
            })
    }

    fn substitute_kw_arg(
        &self,
        callee: &hir::Expr,
        attr_name: &Option<Identifier>,
        arg: &hir::KwArg,
        nth: usize,
        subr_ty: &SubrType,
        passed_params: &mut Set<Str>,
    ) -> TyCheckResult<()> {
        let arg_t = arg.expr.ref_t();
        let kw_name = arg.keyword.inspect();
        if passed_params.contains(&kw_name[..]) {
            return Err(TyCheckErrors::from(TyCheckError::multiple_args_error(
                self.cfg.input.clone(),
                line!() as usize,
                callee.loc(),
                &callee.to_string(),
                self.caused_by(),
                arg.keyword.inspect(),
            )));
        }
        if let Some(pt) = subr_ty
            .non_default_params
            .iter()
            .chain(subr_ty.default_params.iter())
            .find(|pt| pt.name().as_ref() == Some(&kw_name))
        {
            passed_params.insert(kw_name.clone());
            self.sub_unify(arg_t, pt.typ(), arg.loc(), Some(kw_name))
                .map_err(|errs| {
                    log!(err "semi-unification failed with {callee}\n{arg_t} !<: {}", pt.typ());
                    let name = if let Some(attr) = attr_name {
                        format!("{callee}{attr}")
                    } else {
                        callee.show_acc().unwrap_or_default()
                    };
                    let name = name + "::" + readable_name(kw_name);
                    TyCheckErrors::new(
                        errs.into_iter()
                            .map(|e| {
                                TyCheckError::type_mismatch_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    e.core.loc,
                                    e.caused_by,
                                    &name[..],
                                    Some(nth),
                                    pt.typ(),
                                    arg_t,
                                    self.get_candidates(arg_t),
                                    Self::get_simple_type_mismatch_hint(pt.typ(), arg_t),
                                )
                            })
                            .collect(),
                    )
                })?;
        } else {
            return Err(TyCheckErrors::from(TyCheckError::unexpected_kw_arg_error(
                self.cfg.input.clone(),
                line!() as usize,
                arg.keyword.loc(),
                &callee.to_string(),
                self.caused_by(),
                kw_name,
            )));
        }
        Ok(())
    }

    pub(crate) fn get_call_t(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        input: &Input,
        namespace: &Str,
    ) -> TyCheckResult<VarInfo> {
        if let hir::Expr::Accessor(hir::Accessor::Ident(local)) = obj {
            if local.vis().is_private() {
                match &local.inspect()[..] {
                    "match" => {
                        return self.get_match_call_t(SubrKind::Func, pos_args, kw_args);
                    }
                    "match!" => {
                        return self.get_match_call_t(SubrKind::Proc, pos_args, kw_args);
                    }
                    _ => {}
                }
            }
        }
        let found = self.search_callee_info(obj, attr_name, input, namespace)?;
        log!(
            "Found:\ncallee: {obj}{}\nfound: {found}",
            fmt_option!(pre ".", attr_name.as_ref().map(|ident| &ident.name))
        );
        let instance = self.instantiate(found.t, obj)?;
        log!(
            "Instantiated:\ninstance: {instance}\npos_args: ({})\nkw_args: ({})",
            fmt_slice(pos_args),
            fmt_slice(kw_args)
        );
        let res = self.substitute_call(obj, attr_name, &instance, pos_args, kw_args)?;
        let instance = if let Some(__call__) = res {
            __call__
        } else {
            instance
        };
        log!(info "Substituted:\ninstance: {instance}");
        let res = self.eval_t_params(instance, self.level, obj.loc())?;
        log!(info "Params evaluated:\nres: {res}\n");
        self.propagate(&res, obj)?;
        log!(info "Propagated:\nres: {res}\n");
        let res = VarInfo { t: res, ..found };
        Ok(res)
    }

    pub(crate) fn get_const_local(
        &self,
        name: &Token,
        namespace: &Str,
    ) -> SingleTyCheckResult<ValueObj> {
        if let Some(obj) = self.consts.get(name.inspect()) {
            Ok(obj.clone())
        } else {
            /*if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
                return parent.get_const_local(name, namespace);
            }*/
            Err(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                namespace.into(),
                name.inspect(),
                self.get_similar_name(name.inspect()),
            ))
        }
    }

    pub(crate) fn _get_const_attr(
        &self,
        obj: &hir::Expr,
        name: &Token,
        namespace: &Str,
    ) -> SingleTyCheckResult<ValueObj> {
        let self_t = obj.ref_t();
        for ctx in self.get_nominal_super_type_ctxs(self_t).ok_or_else(|| {
            TyCheckError::type_not_found(
                self.cfg.input.clone(),
                line!() as usize,
                obj.loc(),
                self.caused_by(),
                self_t,
            )
        })? {
            if let Ok(t) = ctx.get_const_local(name, namespace) {
                return Ok(t);
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
            parent._get_const_attr(obj, name, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                namespace.into(),
                self_t,
                name.inspect(),
                self.get_similar_attr(self_t, name.inspect()),
            ))
        }
    }

    pub(crate) fn get_similar_name(&self, name: &str) -> Option<&str> {
        get_similar_name(
            self.dir().into_iter().map(|(vn, _)| &vn.inspect()[..]),
            name,
        )
    }

    pub(crate) fn get_similar_attr_from_singular<'a>(
        &'a self,
        obj: &hir::Expr,
        name: &str,
    ) -> Option<&'a str> {
        if let Ok(ctx) = self.get_singular_ctx_by_hir_expr(obj, &self.name) {
            if let Some(name) = ctx.get_similar_name(name) {
                return Some(name);
            }
        }
        None
    }

    pub(crate) fn get_similar_attr<'a>(&'a self, self_t: &'a Type, name: &str) -> Option<&'a str> {
        for ctx in self.get_nominal_super_type_ctxs(self_t)? {
            if let Some(name) = ctx.get_similar_name(name) {
                return Some(name);
            }
        }
        None
    }

    // selfが示す型が、各パラメータTypeに対してどのような変性Varianceを持つかを返す
    // 特に指定されない型に対してはInvariant
    // e.g. K(T, U) = Class(..., Impl: F(T) and Output(U) and Input(T))
    // -> K.variance() == vec![Contravariant, Covariant]
    // TODO: support keyword arguments
    pub(crate) fn type_params_variance(&self) -> Vec<Variance> {
        let in_inout = |t: &Type, name: &VarName| {
            (&t.qual_name()[..] == "Input" || &t.qual_name()[..] == "Output")
                && t.typarams()
                    .first()
                    .map(|inner| inner.qual_name().as_ref() == Some(name.inspect()))
                    .unwrap_or(false)
        };
        self.params
            .iter()
            .map(|(opt_name, _)| {
                if let Some(name) = opt_name {
                    // トレイトの変性を調べるときはsuper_classesも見る必要がある
                    if let Some(t) = self
                        .super_traits
                        .iter()
                        .chain(self.super_classes.iter())
                        .find(|t| in_inout(t, name))
                    {
                        match &t.qual_name()[..] {
                            "Output" => Variance::Covariant,
                            "Input" => Variance::Contravariant,
                            _ => unreachable!(),
                        }
                    } else {
                        Variance::Invariant
                    }
                } else {
                    Variance::Invariant
                }
            })
            .collect()
    }

    /// Perform types linearization.
    /// TODO: Current implementation may be very inefficient.
    ///
    /// C3 linearization requires prior knowledge of inter-type dependencies, and cannot be used for Erg structural subtype linearization
    ///
    /// Algorithm:
    /// ```python
    /// [Int, Str, Nat, Never, Obj, Str!, Module]
    /// => [], [Int, Str, Nat, Never, Obj, Str!, Module]
    /// => [[Int]], [Str, Nat, Never, Obj, Str!, Module]
    /// # 1. If related, put them in the same array; if not, put them in different arrays.
    /// => [[Int], [Str]], [Nat, Never, Obj, Str!, Module]
    /// => ...
    /// => [[Int, Nat, Never, Obj]], [Str, Str!], [Module]]
    /// # 2. Then, perform sorting on the arrays
    /// => [[Never, Nat, Int, Obj], [Str!, Str], [Module]]
    /// # 3. Concatenate the arrays
    /// => [Never, Nat, Int, Obj, Str!, Str, Module]
    /// # 4. From the left, "slide" types as far as it can.
    /// => [Never, Nat, Int, Str!, Str, Module, Obj]
    /// ```
    pub fn sort_types<'a>(&self, types: impl Iterator<Item = &'a Type>) -> Vec<&'a Type> {
        let mut buffers: Vec<Vec<&Type>> = vec![];
        for t in types {
            let mut found = false;
            for buf in buffers.iter_mut() {
                if buf.iter().all(|buf_inner| self.related(buf_inner, t)) {
                    found = true;
                    buf.push(t);
                    break;
                }
            }
            if !found {
                buffers.push(vec![t]);
            }
        }
        for buf in buffers.iter_mut() {
            // this unwrap should be safe
            buf.sort_by(|lhs, rhs| self.cmp_t(lhs, rhs).try_into().unwrap());
        }
        let mut concatenated = buffers.into_iter().flatten().collect::<Vec<_>>();
        let mut idx = 0;
        let len = concatenated.len();
        while let Some(maybe_sup) = concatenated.get(idx) {
            if let Some(pos) = concatenated
                .iter()
                .take(len - idx - 1)
                .rposition(|t| self.supertype_of(maybe_sup, t))
            {
                let sup = concatenated.remove(idx);
                concatenated.insert(pos, sup); // not `pos + 1` because the element was removed at idx
            }
            idx += 1;
        }
        concatenated
    }

    pub(crate) fn get_nominal_super_type_ctxs<'a>(&'a self, t: &Type) -> Option<Vec<&'a Context>> {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => self.get_nominal_super_type_ctxs(&fv.crack()),
            Type::FreeVar(fv) => {
                if let Some(sup) = fv.get_super() {
                    self.get_nominal_super_type_ctxs(&sup)
                } else {
                    self.get_nominal_super_type_ctxs(&Type)
                }
            }
            Type::And(l, r) => {
                match (
                    self.get_nominal_super_type_ctxs(l),
                    self.get_nominal_super_type_ctxs(r),
                ) {
                    // TODO: sort
                    (Some(l), Some(r)) => Some([l, r].concat()),
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    (None, None) => None,
                }
            }
            // TODO
            Type::Or(l, r) => match (l.as_ref(), r.as_ref()) {
                (Type::FreeVar(l), Type::FreeVar(r)) if l.is_unbound() && r.is_unbound() => {
                    let (_lsub, lsup) = l.get_subsup().unwrap();
                    let (_rsub, rsup) = r.get_subsup().unwrap();
                    self.get_nominal_super_type_ctxs(&self.union(&lsup, &rsup))
                }
                (Type::Refinement(l), Type::Refinement(r)) if l.t == r.t => {
                    self.get_nominal_super_type_ctxs(&l.t)
                }
                _ => None,
            },
            _ => self
                .get_simple_nominal_super_type_ctxs(t)
                .map(|ctxs| ctxs.collect()),
        }
    }

    /// include `t` itself
    fn get_simple_nominal_super_type_ctxs<'a>(
        &'a self,
        t: &Type,
    ) -> Option<impl Iterator<Item = &'a Context>> {
        let (_, ctx) = self.get_nominal_type_ctx(t)?;
        let sups = ctx
            .super_classes
            .iter()
            .chain(ctx.super_traits.iter())
            .map(|sup| {
                self.get_nominal_type_ctx(sup)
                    .unwrap_or_else(|| todo!("compiler bug: {sup} not found"))
                    .1
            });
        Some(vec![ctx].into_iter().chain(sups))
    }

    pub(crate) fn _get_super_traits(&self, typ: &Type) -> Option<impl Iterator<Item = Type>> {
        self.get_nominal_type_ctx(typ)
            .map(|(_, ctx)| ctx.super_traits.clone().into_iter())
    }

    /// if `typ` is a refinement type, include the base type (refine.t)
    pub(crate) fn _get_super_classes(&self, typ: &Type) -> Option<impl Iterator<Item = Type>> {
        self.get_nominal_type_ctx(typ).map(|(_, ctx)| {
            let super_classes = ctx.super_classes.clone();
            let derefined = typ.derefine();
            if typ != &derefined {
                vec![derefined].into_iter().chain(super_classes)
            } else {
                vec![].into_iter().chain(super_classes)
            }
        })
    }

    // TODO: Never
    pub(crate) fn get_nominal_type_ctx<'a>(
        &'a self,
        typ: &Type,
    ) -> Option<(&'a Type, &'a Context)> {
        match typ {
            Type::FreeVar(fv) if fv.is_linked() => {
                if let Some(res) = self.get_nominal_type_ctx(&fv.crack()) {
                    return Some(res);
                }
            }
            Type::FreeVar(fv) => {
                let sup = fv.get_super().unwrap();
                if let Some(res) = self.get_nominal_type_ctx(&sup) {
                    return Some(res);
                }
            }
            Type::Refinement(refine) => {
                if let Some(res) = self.get_nominal_type_ctx(&refine.t) {
                    return Some(res);
                }
            }
            Type::Quantified(_) => {
                if let Some((t, ctx)) = self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_get_mono_type("QuantifiedFunc")
                {
                    return Some((t, ctx));
                }
            }
            Type::Subr(subr) => match subr.kind {
                SubrKind::Func => {
                    if let Some((t, ctx)) = self
                        .get_builtins()
                        .unwrap_or(self)
                        .rec_get_mono_type("Func")
                    {
                        return Some((t, ctx));
                    }
                }
                SubrKind::Proc => {
                    if let Some((t, ctx)) = self
                        .get_builtins()
                        .unwrap_or(self)
                        .rec_get_mono_type("Proc")
                    {
                        return Some((t, ctx));
                    }
                }
            },
            Type::Mono(name) => {
                if let Some((t, ctx)) = self.rec_get_mono_type(&typ.local_name()) {
                    return Some((t, ctx));
                }
                // e.g. http.client.Response -> http.client
                let mut namespaces = name.split_with(&[".", "::"]);
                if namespaces.len() < 2 {
                    return None;
                }
                let type_name = namespaces.pop().unwrap(); // Response
                let path = Path::new(namespaces.remove(0));
                let mut path = Self::resolve_path(&self.cfg, path)?;
                for p in namespaces.into_iter() {
                    path = self.push_path(path, Path::new(p));
                }
                if let Some(ctx) = self
                    .mod_cache
                    .as_ref()
                    .and_then(|cache| cache.ref_ctx(path.as_path()))
                    .or_else(|| {
                        self.py_mod_cache
                            .as_ref()
                            .and_then(|cache| cache.ref_ctx(path.as_path()))
                    })
                {
                    if let Some((t, ctx)) = ctx.rec_get_mono_type(type_name) {
                        return Some((t, ctx));
                    }
                }
            }
            Type::Poly { name, .. } => {
                if let Some((t, ctx)) = self.rec_get_poly_type(&typ.local_name()) {
                    return Some((t, ctx));
                }
                // NOTE: This needs to be changed if we want to be able to define classes/traits outside of the top level
                let mut namespaces = name.split_with(&[".", "::"]);
                if namespaces.len() < 2 {
                    return None;
                }
                let type_name = namespaces.pop().unwrap(); // Response
                let path = Path::new(namespaces.remove(0));
                let mut path = Self::resolve_path(&self.cfg, path)?;
                for p in namespaces.into_iter() {
                    path = self.push_path(path, Path::new(p));
                }
                if let Some(ctx) = self
                    .mod_cache
                    .as_ref()
                    .and_then(|cache| cache.ref_ctx(path.as_path()))
                    .or_else(|| {
                        self.py_mod_cache
                            .as_ref()
                            .and_then(|cache| cache.ref_ctx(path.as_path()))
                    })
                {
                    if let Some((t, ctx)) = ctx.rec_get_poly_type(type_name) {
                        return Some((t, ctx));
                    }
                }
            }
            Type::Record(rec) if rec.values().all(|attr| self.supertype_of(&Type, attr)) => {
                return self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_get_mono_type("RecordType");
            }
            Type::Record(_) => {
                return self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_get_mono_type("Record");
            }
            Type::Or(_l, _r) => {
                if let Some(ctx) = self.get_nominal_type_ctx(&poly("Or", vec![])) {
                    return Some(ctx);
                }
            }
            // FIXME: `F()`などの場合、実際は引数が省略されていてもmonomorphicになる
            other if other.is_monomorphic() => {
                if let Some((t, ctx)) = self.rec_get_mono_type(&other.local_name()) {
                    return Some((t, ctx));
                }
            }
            Type::Ref(t) | Type::RefMut { before: t, .. } => {
                if let Some(res) = self.get_nominal_type_ctx(t) {
                    return Some(res);
                }
            }
            other => {
                log!("{other} has no nominal definition");
            }
        }
        None
    }

    // TODO: Never
    pub(crate) fn get_mut_nominal_type_ctx<'a>(
        &'a mut self,
        typ: &Type,
    ) -> Option<(&'a Type, &'a mut Context)> {
        match typ {
            Type::FreeVar(fv) if fv.is_linked() => {
                if let Some(res) = self.get_mut_nominal_type_ctx(&fv.crack()) {
                    return Some(res);
                }
            }
            Type::FreeVar(fv) => {
                let sup = fv.get_super().unwrap();
                if let Some(res) = self.get_mut_nominal_type_ctx(&sup) {
                    return Some(res);
                }
            }
            Type::Refinement(refine) => {
                if let Some(res) = self.get_mut_nominal_type_ctx(&refine.t) {
                    return Some(res);
                }
            }
            Type::Quantified(_) => {
                if let Some(res) = self.get_mut_nominal_type_ctx(&mono("QuantifiedFunc")) {
                    return Some(res);
                }
            }
            Type::Mono(_) => {
                if let Some((t, ctx)) = self.rec_get_mut_mono_type(&typ.local_name()) {
                    return Some((t, ctx));
                }
            }
            Type::Poly { .. } => {
                if let Some((t, ctx)) = self.rec_get_mut_poly_type(&typ.local_name()) {
                    return Some((t, ctx));
                }
            }
            // FIXME: `F()`などの場合、実際は引数が省略されていてもmonomorphicになる
            other if other.is_monomorphic() => {
                if let Some((t, ctx)) = self.rec_get_mut_mono_type(&other.local_name()) {
                    return Some((t, ctx));
                }
            }
            Type::Ref(t) | Type::RefMut { before: t, .. } => {
                if let Some(res) = self.get_mut_nominal_type_ctx(t) {
                    return Some(res);
                }
            }
            other => {
                log!("{other} has no nominal definition");
            }
        }
        None
    }

    pub(crate) fn get_trait_impls(&self, t: &Type) -> Set<TypeRelationInstance> {
        match t {
            // And(Add, Sub) == intersection({Int <: Add(Int), Bool <: Add(Bool) ...}, {Int <: Sub(Int), ...})
            // == {Int <: Add(Int) and Sub(Int), ...}
            Type::And(l, r) => {
                let l_impls = self.get_trait_impls(l);
                let l_base = Set::from_iter(l_impls.iter().map(|ti| &ti.sub_type));
                let r_impls = self.get_trait_impls(r);
                let r_base = Set::from_iter(r_impls.iter().map(|ti| &ti.sub_type));
                let bases = l_base.intersection(&r_base);
                let mut isec = set! {};
                for base in bases.into_iter() {
                    let lti = l_impls.iter().find(|ti| &ti.sub_type == base).unwrap();
                    let rti = r_impls.iter().find(|ti| &ti.sub_type == base).unwrap();
                    let sup_trait = self.intersection(&lti.sup_trait, &rti.sup_trait);
                    isec.insert(TypeRelationInstance::new(lti.sub_type.clone(), sup_trait));
                }
                isec
            }
            Type::Or(l, r) => {
                let l_impls = self.get_trait_impls(l);
                let r_impls = self.get_trait_impls(r);
                // FIXME:
                l_impls.union(&r_impls)
            }
            _ => self.get_simple_trait_impls(t),
        }
    }

    pub(crate) fn get_simple_trait_impls(&self, t: &Type) -> Set<TypeRelationInstance> {
        let current = if let Some(impls) = self.trait_impls.get(&t.qual_name()) {
            impls.clone()
        } else {
            set! {}
        };
        if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            current.union(&outer.get_simple_trait_impls(t))
        } else {
            current
        }
    }

    pub(crate) fn all_patches(&self) -> Vec<&Context> {
        if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            [outer.all_patches(), self.patches.values().collect()].concat()
        } else {
            self.patches.values().collect()
        }
    }

    pub(crate) fn resolve_path(cfg: &ErgConfig, path: &Path) -> Option<PathBuf> {
        Self::resolve_real_path(cfg, path).or_else(|| Self::resolve_decl_path(cfg, path))
    }

    pub(crate) fn resolve_real_path(cfg: &ErgConfig, path: &Path) -> Option<PathBuf> {
        if let Ok(path) = cfg.input.local_resolve(path) {
            Some(path)
        } else if let Ok(path) = erg_std_path()
            .join(format!("{}.er", path.display()))
            .canonicalize()
        {
            Some(path)
        } else if let Ok(path) = erg_std_path()
            .join(format!("{}", path.display()))
            .join("__init__.er")
            .canonicalize()
        {
            Some(path)
        } else {
            None
        }
    }

    pub(crate) fn resolve_decl_path(cfg: &ErgConfig, path: &Path) -> Option<PathBuf> {
        if let Ok(path) = cfg.input.local_resolve(path) {
            Some(path)
        } else if let Ok(path) = erg_pystd_path()
            .join(format!("{}.d.er", path.display()))
            .canonicalize()
        {
            Some(path)
        } else if let Ok(path) = erg_pystd_path()
            .join(format!("{}.d", path.display()))
            .join("__init__.d.er")
            .canonicalize()
        {
            Some(path)
        } else {
            None
        }
    }

    pub(crate) fn push_path(&self, mut path: PathBuf, add: &Path) -> PathBuf {
        path.pop(); // __init__.d.er
        if let Ok(path) = path.join(add).canonicalize() {
            path
        } else if let Ok(path) = path.join(format!("{}.d.er", add.display())).canonicalize() {
            path
        } else if let Ok(path) = path
            .join(format!("{}.d", add.display()))
            .join("__init__.d.er")
            .canonicalize()
        {
            path
        } else {
            todo!("{} {}", path.display(), add.display())
        }
    }

    // FIXME: 現在の実装だとimportしたモジュールはどこからでも見れる
    pub(crate) fn get_mod(&self, name: &str) -> Option<&Context> {
        let t = self.get_var_info(name).map(|(_, vi)| vi.t.clone())?;
        if t.is_module() {
            let path =
                option_enum_unwrap!(t.typarams().remove(0), TyParam::Value:(ValueObj::Str:(_)))?;
            let path = Self::resolve_path(&self.cfg, Path::new(&path[..]))?;
            self.mod_cache
                .as_ref()
                .and_then(|cache| cache.ref_ctx(&path))
                .or_else(|| {
                    self.py_mod_cache
                        .as_ref()
                        .and_then(|cache| cache.ref_ctx(&path))
                })
        } else {
            None
        }
    }

    // rec_get_const_localとは違い、位置情報を持たないしエラーとならない
    pub(crate) fn rec_get_const_obj(&self, name: &str) -> Option<&ValueObj> {
        if let Some(val) = self.consts.get(name) {
            return Some(val);
        }
        for (_, ctx) in self.methods_list.iter() {
            if let Some(val) = ctx.consts.get(name) {
                return Some(val);
            }
        }
        if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_const_obj(name)
        } else {
            None
        }
    }

    pub(crate) fn _rec_get_const_param_defaults(&self, name: &str) -> Option<&Vec<ConstTemplate>> {
        if let Some(impls) = self.const_param_defaults.get(name) {
            Some(impls)
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer._rec_get_const_param_defaults(name)
        } else {
            None
        }
    }

    // TODO: poly type
    pub(crate) fn rec_get_self_t(&self) -> Option<Type> {
        if self.kind.is_method_def() || self.kind.is_type() {
            Some(mono(self.name.clone()))
        } else if let ContextKind::PatchMethodDefs(t) = &self.kind {
            Some(t.clone())
        } else if let Some(outer) = self.get_outer() {
            outer.rec_get_self_t()
        } else {
            None
        }
    }

    pub(crate) fn rec_get_mono_type(&self, name: &str) -> Option<(&Type, &Context)> {
        if let Some((t, ctx)) = self.mono_types.get(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_mono_type(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_get_poly_type(&self, name: &str) -> Option<(&Type, &Context)> {
        if let Some((t, ctx)) = self.poly_types.get(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_poly_type(name)
        } else {
            None
        }
    }

    fn rec_get_mut_mono_type(&mut self, name: &str) -> Option<(&mut Type, &mut Context)> {
        if let Some((t, ctx)) = self.mono_types.get_mut(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.outer.as_mut() {
            // builtins cannot be got as mutable
            outer.rec_get_mut_mono_type(name)
        } else {
            None
        }
    }

    fn rec_get_mut_poly_type(&mut self, name: &str) -> Option<(&mut Type, &mut Context)> {
        if let Some((t, ctx)) = self.poly_types.get_mut(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.outer.as_mut() {
            outer.rec_get_mut_poly_type(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_get_type(&self, name: &str) -> Option<(&Type, &Context)> {
        if let Some((t, ctx)) = self.mono_types.get(name) {
            Some((t, ctx))
        } else if let Some((t, ctx)) = self.poly_types.get(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_type(name)
        } else {
            None
        }
    }

    pub(crate) fn get_mut_type(&mut self, name: &str) -> Option<(&Type, &mut Context)> {
        if let Some((t, ctx)) = self.mono_types.get_mut(name) {
            Some((t, ctx))
        } else if let Some((t, ctx)) = self.poly_types.get_mut(name) {
            Some((t, ctx))
        } else {
            None
        }
    }

    pub(crate) fn rec_get_patch(&self, name: &str) -> Option<&Context> {
        if let Some(ctx) = self.patches.get(name) {
            Some(ctx)
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_patch(name)
        } else {
            None
        }
    }

    fn get_method_type_by_name(&self, name: &Identifier) -> SingleTyCheckResult<&MethodInfo> {
        // TODO: min_by
        if let Some(candidates) = self.method_to_traits.get(name.inspect()) {
            let first_method_type = &candidates.first().unwrap().method_type;
            if candidates
                .iter()
                .skip(1)
                .all(|t| &t.method_type == first_method_type)
            {
                return Ok(&candidates[0]);
            } else {
                return Err(TyCheckError::ambiguous_type_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    name,
                    &candidates
                        .iter()
                        .map(|t| t.definition_type.clone())
                        .collect::<Vec<_>>(),
                    self.caused_by(),
                ));
            }
        }
        if let Some(candidates) = self.method_to_classes.get(name.inspect()) {
            let first_method_type = &candidates.first().unwrap().method_type;
            if candidates
                .iter()
                .skip(1)
                .all(|t| &t.method_type == first_method_type)
            {
                return Ok(&candidates[0]);
            } else {
                return Err(TyCheckError::ambiguous_type_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    name,
                    &candidates
                        .iter()
                        .map(|t| t.definition_type.clone())
                        .collect::<Vec<_>>(),
                    self.caused_by(),
                ));
            }
        }
        if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.get_method_type_by_name(name)
        } else {
            Err(TyCheckError::no_attr_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                self.caused_by(),
                &Type::Failure,
                name.inspect(),
                None,
            ))
        }
    }

    fn get_gen_t_require_attr_t<'a>(&'a self, gen: &'a GenTypeObj, attr: &str) -> Option<&'a Type> {
        match gen.require_or_sup().unwrap().typ() {
            Type::Record(rec) => {
                if let Some(t) = rec.get(attr) {
                    return Some(t);
                }
            }
            other => {
                let obj = self.rec_get_const_obj(&other.local_name());
                let obj = enum_unwrap!(obj, Some:(ValueObj::Type:(TypeObj::Generated:(_))));
                if let Some(t) = self.get_gen_t_require_attr_t(obj, attr) {
                    return Some(t);
                }
            }
        }
        if let Some(additional) = gen.additional() {
            if let Type::Record(gen) = additional.typ() {
                if let Some(t) = gen.get(attr) {
                    return Some(t);
                }
            }
        }
        None
    }

    // TODO: params, polymorphic types
    pub(crate) fn get_candidates(&self, t: &Type) -> Option<Set<Type>> {
        match t {
            Type::Proj { lhs, rhs } => Some(self.get_proj_candidates(lhs, rhs)),
            Type::Subr(subr) => {
                let candidates = self.get_candidates(&subr.return_t)?;
                Some(
                    candidates
                        .into_iter()
                        .map(|ret_t| {
                            let subr = SubrType::new(
                                subr.kind,
                                subr.non_default_params.clone(),
                                subr.var_params.as_ref().map(|p| *p.clone()),
                                subr.default_params.clone(),
                                ret_t,
                            );
                            Type::Subr(subr)
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }

    fn get_proj_candidates(&self, lhs: &Type, rhs: &Str) -> Set<Type> {
        #[allow(clippy::single_match)]
        match lhs {
            Type::FreeVar(fv) => {
                if let Some(sup) = fv.get_super() {
                    let insts = self.get_trait_impls(&sup);
                    let candidates = insts.into_iter().filter_map(move |inst| {
                        if self.supertype_of(&inst.sup_trait, &sup) {
                            self.eval_t_params(
                                proj(inst.sub_type, rhs),
                                self.level,
                                Location::Unknown,
                            )
                            .ok()
                        } else {
                            None
                        }
                    });
                    return candidates.collect();
                }
            }
            _ => {}
        }
        set! {}
    }

    pub(crate) fn is_class(&self, typ: &Type) -> bool {
        match typ {
            Type::And(_l, _r) => false,
            Type::Never => true,
            Type::FreeVar(fv) if fv.is_linked() => self.is_class(&fv.crack()),
            Type::FreeVar(_) => false,
            Type::Or(l, r) => self.is_class(l) && self.is_class(r),
            Type::Proj { lhs, rhs } => self
                .get_proj_candidates(lhs, rhs)
                .iter()
                .all(|t| self.is_class(t)),
            Type::Refinement(refine) => self.is_class(&refine.t),
            Type::Ref(t) | Type::RefMut { before: t, .. } => self.is_class(t),
            _ => {
                if let Some((_, ctx)) = self.get_nominal_type_ctx(typ) {
                    ctx.kind.is_class()
                } else {
                    // TODO: unknown types
                    false
                }
            }
        }
    }

    pub(crate) fn is_trait(&self, typ: &Type) -> bool {
        match typ {
            Type::Never => false,
            Type::FreeVar(fv) if fv.is_linked() => self.is_class(&fv.crack()),
            Type::FreeVar(_) => false,
            Type::And(l, r) | Type::Or(l, r) => self.is_trait(l) && self.is_trait(r),
            Type::Proj { lhs, rhs } => self
                .get_proj_candidates(lhs, rhs)
                .iter()
                .all(|t| self.is_trait(t)),
            Type::Refinement(refine) => self.is_trait(&refine.t),
            Type::Ref(t) | Type::RefMut { before: t, .. } => self.is_trait(t),
            _ => {
                if let Some((_, ctx)) = self.get_nominal_type_ctx(typ) {
                    ctx.kind.is_trait()
                } else {
                    false
                }
            }
        }
    }

    // TODO:
    /// Int.meta_type() == ClassType (<: Type)
    /// Show.meta_type() == TraitType (<: Type)
    /// [Int; 3].meta_type() == [ClassType; 3] (<: Type)
    pub fn meta_type(&self, typ: &Type) -> Type {
        match typ {
            Type::Poly { name, params } => poly(
                name.clone(),
                params
                    .iter()
                    .map(|tp| {
                        if let Ok(t) = self.convert_tp_into_ty(tp.clone()) {
                            TyParam::t(self.meta_type(&t))
                        } else {
                            tp.clone()
                        }
                    })
                    .collect(),
            ),
            _ => Type,
        }
    }
}
