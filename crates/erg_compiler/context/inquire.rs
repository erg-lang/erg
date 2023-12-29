// (type) getters & validators
use std::option::Option; // conflicting to Type::Option
use std::path::{Path, PathBuf};

use erg_common::consts::{DEBUG_MODE, ERG_MODE, PYTHON_MODE};
use erg_common::error::{ErrorCore, Location, SubMessage};
use erg_common::io::Input;
use erg_common::levenshtein;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::set::Set;
use erg_common::traits::{Locational, NoTypeDisplay, Stream};
use erg_common::triple::Triple;
use erg_common::Str;
use erg_common::{
    dict, fmt_option, fmt_slice, get_hash, log, option_enum_unwrap, set, switch_lang,
};

use erg_parser::ast::{self, Identifier, VarName};
use erg_parser::token::Token;

use crate::ty::constructors::{anon, fn_met, free_var, func, mono, poly, proc, proj, ref_, subr_t};
use crate::ty::free::{Constraint, FreeTyParam, FreeTyVar};
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{
    Field, GuardType, HasType, ParamTy, Predicate, RefinementType, SubrKind, SubrType, Type,
    Visibility,
};
use Type::*;

use crate::context::instantiate_spec::ConstTemplate;
use crate::context::{Context, RegistrationMode, TraitImpl, TyVarCache, Variance};
use crate::error::{
    binop_to_dname, ordinal_num, readable_name, unaryop_to_dname, FailableOption,
    SingleTyCheckResult, TyCheckError, TyCheckErrors, TyCheckResult,
};
use crate::varinfo::{AbsLocation, Mutability, VarInfo, VarKind};
use crate::{feature_error, hir};
use crate::{unreachable_error, AccessKind};
use RegistrationMode::*;

use super::eval::UndoableLinkedList;
use super::instantiate_spec::ParamKind;
use super::{ContextKind, MethodPair, TypeContext};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubstituteResult {
    Ok,
    __Call__(Type),
    Coerced(Type),
}

impl Context {
    pub(crate) fn mod_registered(&self, path: &NormalizedPathBuf) -> bool {
        (self.shared.is_some() && self.promises().is_registered(path)) || self.mod_cached(path)
    }

    pub(crate) fn mod_cached(&self, path: &Path) -> bool {
        self.mod_cache().get(path).is_some() || self.py_mod_cache().get(path).is_some()
    }

    /// Get the context of the module. If it was in analysis, wait until analysis is complete and join the thread.
    /// If you only want to know if the module is registered, use `mod_registered`.
    pub(crate) fn get_mod_with_path(&self, path: &Path) -> Option<&Context> {
        if self.module_path() == path {
            return self.get_module();
        }
        let path = NormalizedPathBuf::from(path);
        if let Some(ctx) = self.get_module_from_stack(&path) {
            return Some(ctx);
        }
        if self.shared.is_some() && self.promises().is_registered(&path) && !self.mod_cached(&path)
        {
            let _result = self.promises().join(&path);
        }
        self.opt_mod_cache()?
            .raw_ref_ctx(&path)
            .or_else(|| self.opt_py_mod_cache()?.raw_ref_ctx(&path))
            .map(|mod_ctx| &mod_ctx.context)
    }

    pub(crate) fn get_current_scope_non_param(&self, name: &VarName) -> Option<&VarInfo> {
        #[cfg(feature = "py_compat")]
        let search_name = self
            .erg_to_py_names
            .get(name.inspect())
            .unwrap_or(name.inspect());
        #[cfg(not(feature = "py_compat"))]
        let search_name = name.inspect();
        self.locals
            .get(search_name)
            .or_else(|| self.decls.get(search_name))
            .or_else(|| {
                for methods in self.methods_list.iter() {
                    if let Some(vi) = methods.get_current_scope_non_param(name) {
                        return Some(vi);
                    }
                }
                None
            })
            .or_else(|| {
                self.tv_cache
                    .as_ref()
                    .and_then(|tv_cache| tv_cache.var_infos.get(name))
            })
    }

    pub(crate) fn get_current_scope_var(&self, name: &VarName) -> Option<&VarInfo> {
        #[cfg(feature = "py_compat")]
        let search_name = self
            .erg_to_py_names
            .get(name.inspect())
            .unwrap_or(name.inspect());
        #[cfg(not(feature = "py_compat"))]
        let search_name = name.inspect();
        self.locals
            .get(search_name)
            .or_else(|| self.decls.get(search_name))
            .or_else(|| {
                self.params
                    .iter()
                    .find(|(opt_name, _)| {
                        opt_name
                            .as_ref()
                            .map(|n| n.inspect() == search_name)
                            .unwrap_or(false)
                    })
                    .map(|(_, vi)| vi)
            })
            .or_else(|| {
                for methods in self.methods_list.iter() {
                    if let Some(vi) = methods.get_current_scope_var(name) {
                        return Some(vi);
                    }
                }
                None
            })
            .or_else(|| {
                self.tv_cache
                    .as_ref()
                    .and_then(|tv_cache| tv_cache.var_infos.get(name))
            })
    }

    pub(crate) fn get_mut_current_scope_var(&mut self, name: &VarName) -> Option<&mut VarInfo> {
        #[cfg(feature = "py_compat")]
        let search_name = self
            .erg_to_py_names
            .get(name.inspect())
            .unwrap_or(name.inspect());
        #[cfg(not(feature = "py_compat"))]
        let search_name = name.inspect();
        self.locals
            .get_mut(search_name)
            .or_else(|| self.decls.get_mut(search_name))
            .or_else(|| {
                self.params
                    .iter_mut()
                    .find(|(opt_name, _)| {
                        opt_name
                            .as_ref()
                            .map(|n| n.inspect() == search_name)
                            .unwrap_or(false)
                    })
                    .map(|(_, vi)| vi)
            })
            .or_else(|| {
                for methods in self.methods_list.iter_mut() {
                    if let Some(vi) = methods.get_mut_current_scope_var(name) {
                        return Some(vi);
                    }
                }
                None
            })
    }

    pub(crate) fn get_var_kv(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        self.locals
            .get_key_value(name)
            .or_else(|| self.get_param_kv(name))
            .or_else(|| self.decls.get_key_value(name))
            .or_else(|| self.future_defined_locals.get_key_value(name))
            .or_else(|| self.get_outer().and_then(|ctx| ctx.get_var_kv(name)))
    }

    fn get_param_kv(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        self.params
            .iter()
            .find(|(opt_name, _)| {
                opt_name
                    .as_ref()
                    .map(|n| &n.inspect()[..] == name)
                    .unwrap_or(false)
            })
            .map(|(opt_name, vi)| (opt_name.as_ref().unwrap(), vi))
    }

    pub fn get_method_kv(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        self.get_var_kv(name)
            .or_else(|| {
                for methods in self.methods_list.iter() {
                    if let Some(vi) = methods.get_method_kv(name) {
                        return Some(vi);
                    }
                }
                None
            })
            .or_else(|| self.get_outer().and_then(|ctx| ctx.get_method_kv(name)))
    }

    pub fn get_singular_ctxs_by_hir_expr(
        &self,
        obj: &hir::Expr,
        namespace: &Context,
    ) -> SingleTyCheckResult<Vec<&Context>> {
        match obj {
            hir::Expr::Accessor(hir::Accessor::Ident(ident)) => {
                // e.g. ident.t: {Int}
                if let Ok(refine) = <&RefinementType>::try_from(ident.ref_t()) {
                    if let Predicate::Equal { rhs, .. } = refine.pred.as_ref() {
                        if let Ok(t) = <&Type>::try_from(rhs) {
                            if let Some(ctxs) = self.get_nominal_super_type_ctxs(t) {
                                return Ok(ctxs.into_iter().map(|ctx| &ctx.ctx).collect());
                            }
                        }
                    }
                }
                self.get_singular_ctxs_by_ident(&ident.raw, namespace)
            }
            hir::Expr::Accessor(hir::Accessor::Attr(attr)) => {
                // REVIEW: 両方singularとは限らない?
                let mut ctxs = vec![];
                for ctx in self.get_singular_ctxs_by_hir_expr(&attr.obj, namespace)? {
                    ctxs.extend(ctx.get_singular_ctxs_by_ident(&attr.ident.raw, namespace)?);
                }
                Ok(ctxs)
            }
            hir::Expr::TypeAsc(tasc) => self.get_singular_ctxs_by_hir_expr(&tasc.expr, namespace),
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

    pub(crate) fn get_singular_ctxs_by_ident(
        &self,
        ident: &ast::Identifier,
        namespace: &Context,
    ) -> SingleTyCheckResult<Vec<&Context>> {
        self.get_mod(ident.inspect())
            .map(|ctx| vec![ctx])
            .or_else(|| {
                let ctx = self.get_type_ctx(ident.inspect())?;
                self.get_nominal_super_type_ctxs(&ctx.typ)
                    .map(|ctxs| ctxs.into_iter().map(|ctx| &ctx.ctx).collect())
            })
            .or_else(|| self.rec_get_patch(ident.inspect()).map(|ctx| vec![ctx]))
            .ok_or_else(|| {
                let (similar_info, similar_name) =
                    self.get_similar_name_and_info(ident.inspect()).unzip();
                TyCheckError::detailed_no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    ident.loc(),
                    namespace.name.to_string(),
                    ident.inspect(),
                    similar_name,
                    similar_info,
                )
            })
    }

    #[allow(unused)]
    pub(crate) fn get_mut_singular_ctxs_by_ident(
        &mut self,
        ident: &ast::Identifier,
        namespace: &Str,
    ) -> SingleTyCheckResult<&mut Context> {
        self.get_mut_singular_ctxs_and_t_by_ident(ident, namespace)
            .map(|ctx| &mut ctx.ctx)
    }

    pub(crate) fn get_mut_singular_ctxs_and_t_by_ident(
        &mut self,
        ident: &ast::Identifier,
        namespace: &Str,
    ) -> SingleTyCheckResult<&mut TypeContext> {
        let err = TyCheckError::no_var_error(
            self.cfg.input.clone(),
            line!() as usize,
            ident.loc(),
            namespace.into(),
            ident.inspect(),
            self.get_similar_name(ident.inspect()),
        );
        self.rec_get_mut_type(ident.inspect()).ok_or(err)
    }

    pub(crate) fn get_singular_ctxs(
        &self,
        obj: &ast::Expr,
        namespace: &Context,
    ) -> SingleTyCheckResult<Vec<&Context>> {
        match obj {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) => {
                self.get_singular_ctxs_by_ident(ident, namespace)
            }
            ast::Expr::Accessor(ast::Accessor::Attr(attr)) => {
                let local_attr = ast::Expr::Accessor(ast::Accessor::Ident(attr.ident.clone()));
                let mut ctxs = vec![];
                // REVIEW: 両方singularとは限らない?
                for ctx in self.get_singular_ctxs(&attr.obj, namespace)? {
                    ctxs.extend(ctx.get_singular_ctxs(&local_attr, namespace)?);
                }
                Ok(ctxs)
            }
            ast::Expr::Accessor(ast::Accessor::TypeApp(tapp)) => {
                self.get_singular_ctxs(&tapp.obj, namespace)
            }
            ast::Expr::Call(call) => self.get_singular_ctxs(&call.obj, namespace),
            ast::Expr::TypeAscription(tasc) => self.get_singular_ctxs(&tasc.expr, namespace),
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

    pub(crate) fn get_mut_singular_ctx_and_t(
        &mut self,
        obj: &ast::Expr,
        namespace: &Str,
    ) -> SingleTyCheckResult<&mut TypeContext> {
        match obj {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) => {
                self.get_mut_singular_ctxs_and_t_by_ident(ident, namespace)
            }
            ast::Expr::Accessor(ast::Accessor::Attr(attr)) => {
                // REVIEW: 両方singularとは限らない?
                let ctx = self.get_mut_singular_ctx(&attr.obj, namespace)?;
                let attr = ast::Expr::Accessor(ast::Accessor::Ident(attr.ident.clone()));
                ctx.get_mut_singular_ctx_and_t(&attr, namespace)
            }
            ast::Expr::Accessor(ast::Accessor::TypeApp(tapp)) => {
                self.get_mut_singular_ctx_and_t(&tapp.obj, namespace)
            }
            ast::Expr::Call(call) => self.get_mut_singular_ctx_and_t(&call.obj, namespace),
            ast::Expr::TypeAscription(tasc) => {
                self.get_mut_singular_ctx_and_t(&tasc.expr, namespace)
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
        self.get_mut_singular_ctx_and_t(obj, namespace)
            .map(|ctx| &mut ctx.ctx)
    }

    fn get_match_call_t(
        &self,
        kind: SubrKind,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> FailableOption<VarInfo> {
        let mut errs = TyCheckErrors::empty();
        if !kw_args.is_empty() {
            // TODO: this error desc is not good
            return Err((
                None,
                TyCheckErrors::from(TyCheckError::default_param_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    kw_args[0].loc(),
                    self.caused_by(),
                    "match",
                )),
            ));
        }
        for pos_arg in pos_args.iter().skip(1) {
            let t = pos_arg.expr.ref_t();
            // Allow only anonymous functions to be passed as match arguments (for aesthetic reasons)
            if !matches!(&pos_arg.expr, hir::Expr::Lambda(_)) {
                return Err((
                    None,
                    TyCheckErrors::from(TyCheckError::type_mismatch_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        pos_arg.loc(),
                        self.caused_by(),
                        "match",
                        None,
                        &mono("LambdaFunc"),
                        t,
                        self.get_candidates(t),
                        self.get_simple_type_mismatch_hint(&mono("LambdaFunc"), t),
                    )),
                ));
            }
        }
        let match_target_expr_t = pos_args[0].expr.ref_t();
        // Never or T => T
        let mut union_pat_t = Type::Never;
        let mut arm_ts = vec![];
        for (i, pos_arg) in pos_args.iter().skip(1).enumerate() {
            let lambda = erg_common::enum_unwrap!(&pos_arg.expr, hir::Expr::Lambda); // already checked
            if !lambda.params.defaults.is_empty() {
                return Err((
                    None,
                    TyCheckErrors::from(TyCheckError::default_param_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        pos_args[i + 1].loc(),
                        self.caused_by(),
                        "match",
                    )),
                ));
            }
            if lambda.params.len() != 1 {
                return Err((
                    None,
                    TyCheckErrors::from(TyCheckError::param_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        pos_args[i + 1].loc(),
                        self.caused_by(),
                        1,
                        lambda.params.len(),
                    )),
                ));
            }
            let mut dummy_tv_cache = TyVarCache::new(self.level, self);
            let rhs = match self.instantiate_param_sig_t(
                &lambda.params.non_defaults[0].raw,
                None,
                &mut dummy_tv_cache,
                Normal,
                ParamKind::NonDefault,
                false,
            ) {
                Ok(ty) => ty,
                Err((ty, es)) => {
                    errs.extend(es);
                    ty
                }
            };
            if lambda.params.non_defaults[0].raw.t_spec.is_none() && rhs.is_free_var() {
                rhs.link(&Obj, None);
            }
            union_pat_t = self.union(&union_pat_t, &rhs);
            arm_ts.push(rhs);
        }
        // NG: expr_t: Nat, union_pat_t: {1, 2}
        // OK: expr_t: Int, union_pat_t: {1} or 'T
        if let Err(err) = self.sub_unify(match_target_expr_t, &union_pat_t, &pos_args[0], None) {
            if cfg!(feature = "debug") {
                eprintln!("match error: {err}");
            }
            errs.push(TyCheckError::match_error(
                self.cfg.input.clone(),
                line!() as usize,
                pos_args[0].loc(),
                self.caused_by(),
                match_target_expr_t,
                &union_pat_t,
                arm_ts,
            ));
            return Err((None, errs));
        }
        let branch_ts = pos_args
            .iter()
            .skip(1)
            .map(|a| ParamTy::Pos(a.expr.ref_t().clone()))
            .collect::<Vec<_>>();
        let Some(mut return_t) = branch_ts
            .first()
            .and_then(|branch| branch.typ().return_t().cloned())
        else {
            errs.push(TyCheckError::args_missing_error(
                self.cfg.input.clone(),
                line!() as usize,
                pos_args[0].loc(),
                "match",
                self.caused_by(),
                vec![Str::ever("obj")],
            ));
            return Err((None, errs));
        };
        for arg_t in branch_ts.iter().skip(1) {
            return_t = self.union(&return_t, arg_t.typ().return_t().unwrap_or(&Type::Never));
        }
        let param_ty = ParamTy::Pos(match_target_expr_t.clone());
        let param_ts = [vec![param_ty], branch_ts.to_vec()].concat();
        let t = if kind.is_func() {
            func(param_ts, None, vec![], None, return_t)
        } else {
            proc(param_ts, None, vec![], None, return_t)
        };
        let vi = VarInfo {
            t,
            ..VarInfo::default()
        };
        if errs.is_empty() {
            Ok(vi)
        } else {
            Err((Some(vi), errs))
        }
    }

    pub(crate) fn rec_get_var_info(
        &self,
        ident: &Identifier,
        acc_kind: AccessKind,
        input: &Input,
        namespace: &Context,
    ) -> Triple<VarInfo, TyCheckError> {
        if ident.inspect() == "Self" {
            if let Some(self_t) = self.rec_get_self_t() {
                return self.rec_get_var_info(
                    &Identifier::auto(self_t.local_name()),
                    acc_kind,
                    input,
                    namespace,
                );
            }
        }
        if let Some(vi) = self.get_current_scope_var(&ident.name) {
            match self.validate_visibility(ident, vi, input, namespace) {
                Ok(()) if acc_kind.matches(vi) => {
                    return Triple::Ok(vi.clone());
                }
                Err(err) => {
                    if !acc_kind.is_local() {
                        return Triple::Err(err);
                    }
                }
                _ => {}
            }
        } else if let Some((name, _vi)) = self
            .future_defined_locals
            .get_key_value(&ident.inspect()[..])
        {
            return Triple::Err(TyCheckError::access_before_def_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                namespace.name.to_string(),
                ident.inspect(),
                name.ln_begin().unwrap_or(0),
                self.get_similar_name(ident.inspect()),
            ));
        } else if let Some((name, _vi)) = self.deleted_locals.get_key_value(&ident.inspect()[..]) {
            return Triple::Err(TyCheckError::access_deleted_var_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                namespace.name.to_string(),
                ident.inspect(),
                name.ln_begin().unwrap_or(0),
                self.get_similar_name(ident.inspect()),
            ));
        }
        for method_ctx in self.methods_list.iter() {
            match method_ctx.rec_get_var_info(ident, acc_kind, input, namespace) {
                Triple::Ok(vi) => {
                    return Triple::Ok(vi);
                }
                Triple::Err(e) => {
                    return Triple::Err(e);
                }
                Triple::None => {}
            }
        }
        if acc_kind.is_local() {
            if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
                return parent.rec_get_var_info(ident, acc_kind, input, namespace);
            }
        }
        Triple::None
    }

    pub(crate) fn rec_get_mut_var_info(
        &mut self,
        ident: &Identifier,
        acc_kind: AccessKind,
    ) -> Option<&mut VarInfo> {
        if let Some(vi) = self.get_current_scope_var(&ident.name) {
            match self.validate_visibility(ident, vi, &self.cfg.input, self) {
                Ok(()) if acc_kind.matches(vi) => {
                    let vi = self.get_mut_current_scope_var(&ident.name).unwrap();
                    return Some(vi);
                }
                Err(_err) => {
                    if !acc_kind.is_local() {
                        return None;
                    }
                }
                _ => {}
            }
        }
        if acc_kind.is_local() {
            if let Some(parent) = self.get_mut_outer() {
                return parent.rec_get_mut_var_info(ident, acc_kind);
            }
        }
        None
    }

    pub(crate) fn rec_get_decl_info(
        &self,
        ident: &Identifier,
        acc_kind: AccessKind,
        input: &Input,
        namespace: &Context,
    ) -> Triple<VarInfo, TyCheckError> {
        if let Some(vi) = self
            .decls
            .get(&ident.inspect()[..])
            .or_else(|| self.future_defined_locals.get(&ident.inspect()[..]))
        {
            match self.validate_visibility(ident, vi, input, namespace) {
                Ok(()) if acc_kind.matches(vi) => {
                    return Triple::Ok(vi.clone());
                }
                Err(err) => {
                    if !acc_kind.is_local() {
                        return Triple::Err(err);
                    }
                }
                _ => {}
            }
        }
        if acc_kind.is_local() {
            if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
                return parent.rec_get_decl_info(ident, acc_kind, input, namespace);
            }
        }
        Triple::None
    }

    pub(crate) fn get_attr_info(
        &self,
        obj: &hir::Expr,
        ident: &Identifier,
        input: &Input,
        namespace: &Context,
        expect: Option<&Type>,
    ) -> Triple<VarInfo, TyCheckError> {
        // get_attr_info(?T, aaa) == None
        // => ?T(<: Structural({ .aaa = ?U }))
        if PYTHON_MODE
            && obj
                .var_info()
                .is_some_and(|vi| vi.is_ambiguously_typed_parameter())
        {
            let constraint = expect.map_or(Constraint::new_type_of(Type), |t| {
                Constraint::new_subtype_of(t.clone())
            });
            let t = free_var(self.level, constraint);
            if let Some(fv) = obj.ref_t().as_free() {
                if fv.get_sub().is_some() {
                    let sup = fv.get_super().unwrap();
                    let vis = self.instantiate_vis_modifier(&ident.vis).unwrap();
                    let structural = Type::Record(
                        dict! { Field::new(vis, ident.inspect().clone()) => t.clone() },
                    )
                    .structuralize();
                    let intersection = self.intersection(&sup, &structural);
                    if intersection != Never {
                        fv.update_super(|_| intersection);
                    }
                }
            }
            let muty = Mutability::from(&ident.inspect()[..]);
            let vi = VarInfo::new(
                t,
                muty,
                Visibility::DUMMY_PUBLIC,
                VarKind::Builtin,
                None,
                ContextKind::Dummy,
                None,
                AbsLocation::unknown(),
            );
            return Triple::Ok(vi);
        }
        let self_t = obj.t();
        match self.get_attr_info_from_attributive(&self_t, ident, namespace) {
            Triple::Ok(vi) => {
                return Triple::Ok(vi);
            }
            Triple::Err(e) => {
                return Triple::Err(e);
            }
            _ => {}
        }
        // class/module attr
        if let Ok(singular_ctxs) = self.get_singular_ctxs_by_hir_expr(obj, namespace) {
            for ctx in singular_ctxs {
                match ctx.rec_get_var_info(ident, AccessKind::UnboundAttr, input, namespace) {
                    Triple::Ok(vi) => {
                        return Triple::Ok(vi);
                    }
                    Triple::Err(e) => {
                        return Triple::Err(e);
                    }
                    Triple::None => {}
                }
            }
        }
        // bound method/instance attr
        match self.get_bound_attr_from_nominal_t(obj, ident, input, namespace) {
            Triple::Ok(vi) => {
                if let Some(self_t) = vi.t.self_t() {
                    let list = UndoableLinkedList::new();
                    if let Err(errs) = self
                        .undoable_sub_unify(obj.ref_t(), self_t, obj, &list, Some(&"self".into()))
                        .map_err(|mut e| e.remove(0))
                    {
                        return Triple::Err(errs);
                    }
                    drop(list);
                    self.sub_unify(obj.ref_t(), self_t, obj, Some(&"self".into()))
                        .unwrap();
                }
                return Triple::Ok(vi);
            }
            Triple::Err(e) => {
                return Triple::Err(e);
            }
            _ => {}
        }
        for patch in self.find_patches_of(obj.ref_t()) {
            if let Some(vi) = patch.get_current_scope_non_param(&ident.name) {
                return match self.validate_visibility(ident, vi, input, namespace) {
                    Ok(_) => Triple::Ok(vi.clone()),
                    Err(e) => Triple::Err(e),
                };
            }
            for methods_ctx in patch.methods_list.iter() {
                if let Some(vi) = methods_ctx.get_current_scope_non_param(&ident.name) {
                    return match self.validate_visibility(ident, vi, input, namespace) {
                        Ok(_) => Triple::Ok(vi.clone()),
                        Err(e) => Triple::Err(e),
                    };
                }
            }
        }
        // REVIEW: get by name > coercion?
        match self.get_attr_type_by_name(obj, ident, namespace) {
            Triple::Ok(method) => {
                let list = UndoableLinkedList::new();
                if self
                    .undoable_sub_unify(obj.ref_t(), &method.definition_type, obj, &list, None)
                    .is_ok()
                {
                    drop(list);
                    self.sub_unify(obj.ref_t(), &method.definition_type, obj, None)
                        .unwrap();
                    return Triple::Ok(method.method_info.clone());
                }
            }
            Triple::Err(err) if ERG_MODE => {
                return Triple::Err(err);
            }
            _ => {}
        }
        self.fallback_get_attr_info(obj, ident, input, namespace, expect)
    }

    fn fallback_get_attr_info(
        &self,
        obj: &hir::Expr,
        ident: &Identifier,
        input: &Input,
        namespace: &Context,
        expect: Option<&Type>,
    ) -> Triple<VarInfo, TyCheckError> {
        if let Ok(coerced) = self.coerce(obj.t(), &obj) {
            if &coerced != obj.ref_t() {
                let hash = get_hash(obj.ref_t());
                let list = UndoableLinkedList::new();
                obj.ref_t().undoable_coerce(&list);
                if hash == get_hash(obj.ref_t()) {
                    return Triple::None;
                }
                if let Triple::Ok(vi) = self.get_attr_info(obj, ident, input, namespace, expect) {
                    drop(list);
                    obj.ref_t().coerce(None);
                    return Triple::Ok(vi);
                }
            }
        }
        Triple::None
    }

    fn get_bound_attr_from_nominal_t(
        &self,
        obj: &hir::Expr,
        ident: &Identifier,
        input: &Input,
        namespace: &Context,
    ) -> Triple<VarInfo, TyCheckError> {
        let self_t = obj.t();
        if let Some(sups) = self.get_nominal_super_type_ctxs(&self_t) {
            for ctx in sups {
                match ctx.rec_get_var_info(ident, AccessKind::BoundAttr, input, namespace) {
                    Triple::Ok(vi) => {
                        return Triple::Ok(vi);
                    }
                    Triple::Err(e) => {
                        return Triple::Err(e);
                    }
                    _ => {}
                }
                // if self is a methods context
                if let Some(ctx) = self.get_same_name_context(&ctx.name) {
                    match ctx.rec_get_var_info(ident, AccessKind::BoundAttr, input, namespace) {
                        Triple::Ok(vi) => {
                            return Triple::Ok(vi);
                        }
                        Triple::Err(e) => {
                            return Triple::Err(e);
                        }
                        _ => {}
                    }
                }
            }
        }
        let coerced = match self.coerce(obj.t(), &()).map_err(|mut es| es.remove(0)) {
            Ok(t) => t,
            Err(e) => {
                return Triple::Err(e);
            }
        };
        if obj.ref_t() != &coerced {
            let ctxs = match self.get_nominal_super_type_ctxs(&coerced).ok_or_else(|| {
                TyCheckError::type_not_found(
                    self.cfg.input.clone(),
                    line!() as usize,
                    obj.loc(),
                    self.caused_by(),
                    &coerced,
                )
            }) {
                Ok(ctxs) => ctxs,
                Err(e) => {
                    return Triple::Err(e);
                }
            };
            for ctx in ctxs {
                match ctx.rec_get_var_info(ident, AccessKind::BoundAttr, input, namespace) {
                    Triple::Ok(vi) => {
                        obj.ref_t().destructive_coerce();
                        return Triple::Ok(vi);
                    }
                    Triple::Err(e) => {
                        return Triple::Err(e);
                    }
                    _ => {}
                }
                if let Some(ctx) = self.get_same_name_context(&ctx.name) {
                    match ctx.rec_get_var_info(ident, AccessKind::BoundAttr, input, namespace) {
                        Triple::Ok(vi) => {
                            return Triple::Ok(vi);
                        }
                        Triple::Err(e) => {
                            return Triple::Err(e);
                        }
                        _ => {}
                    }
                }
            }
        }
        Triple::None
    }

    /// get type from given attributive type (Record).
    /// not ModuleType or ClassType etc.
    /// if `t == Failure`, returns `VarInfo::ILLEGAL`
    fn get_attr_info_from_attributive(
        &self,
        t: &Type,
        ident: &Identifier,
        namespace: &Context,
    ) -> Triple<VarInfo, TyCheckError> {
        match t {
            // (obj: Failure).foo: Failure
            Type::Failure => Triple::Ok(VarInfo::ILLEGAL),
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
                if let Some((field, attr_t)) = record.get_key_value(ident.inspect()) {
                    let muty = Mutability::from(&ident.inspect()[..]);
                    let vi = VarInfo::new(
                        attr_t.clone(),
                        muty,
                        Visibility::new(field.vis.clone(), Str::ever("<dummy>")),
                        VarKind::Builtin,
                        None,
                        ContextKind::Record,
                        None,
                        AbsLocation::unknown(),
                    );
                    if let Err(err) = self.validate_visibility(ident, &vi, &self.cfg.input, namespace) {
                        return Triple::Err(err);
                    }
                    Triple::Ok(vi)
                } else {
                    Triple::None
                }
            }
            Type::NamedTuple(tuple) => {
                if let Some((field, attr_t)) = tuple.iter().find(|(f, _)| &f.symbol == ident.inspect()) {
                    let muty = Mutability::from(&ident.inspect()[..]);
                    let vi = VarInfo::new(
                        attr_t.clone(),
                        muty,
                        Visibility::new(field.vis.clone(), Str::ever("<dummy>")),
                        VarKind::Builtin,
                        None,
                        ContextKind::Record,
                        None,
                        AbsLocation::unknown(),
                    );
                    if let Err(err) = self.validate_visibility(ident, &vi, &self.cfg.input, namespace) {
                        return Triple::Err(err);
                    }
                    Triple::Ok(vi)
                } else {
                    Triple::None
                }
            }
            Type::Structural(t) => self.get_attr_info_from_attributive(t, ident, namespace),
            _other => Triple::None,
        }
    }

    // returns callee's type, not the return type
    fn search_callee_info(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        input: &Input,
        namespace: &Context,
    ) -> SingleTyCheckResult<VarInfo> {
        if obj.ref_t() == Type::FAILURE {
            // (...Obj) -> Failure
            return Ok(VarInfo {
                t: Type::Subr(SubrType::new(
                    SubrKind::Func,
                    vec![],
                    Some(ParamTy::Pos(ref_(Obj))),
                    vec![],
                    Some(ParamTy::Pos(ref_(Obj))),
                    Failure,
                )),
                ..VarInfo::default()
            });
        }
        if let Some(attr_name) = attr_name.as_ref() {
            let mut vi =
                self.search_method_info(obj, attr_name, pos_args, kw_args, input, namespace)?;
            vi.t = self.resolve_overload(obj, vi.t, pos_args, kw_args, attr_name)?;
            Ok(vi)
        } else {
            let t = self.resolve_overload(obj, obj.t(), pos_args, kw_args, obj)?;
            Ok(VarInfo {
                t,
                ..VarInfo::default()
            })
        }
    }

    fn search_callee_info_without_args(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        input: &Input,
        namespace: &Context,
    ) -> SingleTyCheckResult<VarInfo> {
        if obj.ref_t() == Type::FAILURE {
            // (...Obj) -> Failure
            return Ok(VarInfo {
                t: Type::Subr(SubrType::new(
                    SubrKind::Func,
                    vec![],
                    Some(ParamTy::Pos(ref_(Obj))),
                    vec![],
                    Some(ParamTy::Pos(ref_(Obj))),
                    Failure,
                )),
                ..VarInfo::default()
            });
        }
        if let Some(attr_name) = attr_name.as_ref() {
            self.search_method_info_without_args(obj, attr_name, input, namespace)
        } else {
            Ok(VarInfo {
                t: obj.t(),
                ..VarInfo::default()
            })
        }
    }

    fn resolve_overload(
        &self,
        obj: &hir::Expr,
        instance: Type,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        loc: &impl Locational,
    ) -> SingleTyCheckResult<Type> {
        let intersecs = instance.intersection_types();
        if intersecs.len() == 1 {
            Ok(instance)
        } else {
            let mut input_t = subr_t(
                SubrKind::Proc,
                pos_args
                    .iter()
                    .map(|pos| ParamTy::Pos(pos.expr.t()))
                    .collect(),
                None,
                kw_args
                    .iter()
                    .map(|kw| ParamTy::kw(kw.keyword.content.clone(), kw.expr.t()))
                    .collect(),
                None,
                Obj,
            );
            for ty in intersecs.iter() {
                match (ty.is_method(), input_t.is_method()) {
                    (true, false) => {
                        let Type::Subr(sub) = &mut input_t else {
                            unreachable!()
                        };
                        sub.non_default_params
                            .insert(0, ParamTy::kw(Str::ever("self"), obj.t()));
                    }
                    (false, true) => {
                        let Type::Subr(sub) = &mut input_t else {
                            unreachable!()
                        };
                        sub.non_default_params.remove(0);
                    }
                    _ => {}
                }
                if self.subtype_of(ty, &input_t) {
                    return Ok(ty.clone());
                }
            }
            let Type::Subr(subr_t) = input_t else {
                unreachable!()
            };
            Err(TyCheckError::overload_error(
                self.cfg.input.clone(),
                line!() as usize,
                loc.loc(),
                self.caused_by(),
                subr_t.non_default_params,
                subr_t.default_params,
                intersecs.iter(),
            ))
        }
    }

    pub(crate) fn get_same_name_context(&self, name: &str) -> Option<&Context> {
        if &self.name[..] == name {
            Some(self)
        } else {
            self.get_outer().and_then(|p| p.get_same_name_context(name))
        }
    }

    // Note that the method may be static.
    fn search_method_info(
        &self,
        obj: &hir::Expr,
        attr_name: &Identifier,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        input: &Input,
        namespace: &Context,
    ) -> SingleTyCheckResult<VarInfo> {
        // search_method_info(?T, aaa, pos_args: [1, 2]) == None
        // => ?T(<: Structural({ .aaa = (self: ?T, ?U, ?V) -> ?W }))
        if PYTHON_MODE
            && obj
                .var_info()
                .is_some_and(|vi| vi.is_ambiguously_typed_parameter())
        {
            let nd_params = pos_args
                .iter()
                .map(|_| ParamTy::Pos(free_var(self.level, Constraint::new_type_of(Type))))
                .collect::<Vec<_>>();
            let d_params = kw_args
                .iter()
                .map(|arg| {
                    ParamTy::kw(
                        arg.keyword.inspect().clone(),
                        free_var(self.level, Constraint::new_type_of(Type)),
                    )
                })
                .collect::<Vec<_>>();
            let return_t = free_var(self.level, Constraint::new_type_of(Type));
            let subr_t = fn_met(obj.t(), nd_params, None, d_params, None, return_t);
            if let Some(fv) = obj.ref_t().as_free() {
                if fv.get_sub().is_some() {
                    let sup = fv.get_super().unwrap();
                    let vis = self.instantiate_vis_modifier(&attr_name.vis).unwrap();
                    let structural = Type::Record(
                        dict! { Field::new(vis, attr_name.inspect().clone()) => subr_t.clone() },
                    )
                    .structuralize();
                    let intersection = self.intersection(&sup, &structural);
                    if intersection != Never {
                        fv.update_super(|_| intersection);
                    }
                }
            }
            let muty = Mutability::from(&attr_name.inspect()[..]);
            let vi = VarInfo::new(
                subr_t,
                muty,
                Visibility::DUMMY_PUBLIC,
                VarKind::Builtin,
                None,
                ContextKind::Dummy,
                None,
                AbsLocation::unknown(),
            );
            return Ok(vi);
        }
        match self.get_attr_info_from_attributive(obj.ref_t(), attr_name, namespace) {
            Triple::Ok(vi) => {
                return Ok(vi);
            }
            Triple::Err(e) => {
                return Err(e);
            }
            _ => {}
        }
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
            if let Some(vi) = ctx.get_current_scope_non_param(&attr_name.name) {
                self.validate_visibility(attr_name, vi, input, namespace)?;
                return Ok(vi.clone());
            }
            for methods_ctx in ctx.methods_list.iter() {
                if let Some(vi) = methods_ctx.get_current_scope_non_param(&attr_name.name) {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
            }
            if let Some(ctx) = self.get_same_name_context(&ctx.name) {
                match ctx.rec_get_var_info(attr_name, AccessKind::BoundAttr, input, namespace) {
                    Triple::Ok(t) => {
                        return Ok(t);
                    }
                    Triple::Err(e) => {
                        return Err(e);
                    }
                    Triple::None => {}
                }
            }
        }
        if let Ok(singular_ctxs) = self.get_singular_ctxs_by_hir_expr(obj, namespace) {
            for ctx in singular_ctxs {
                if let Some(vi) = ctx.get_current_scope_non_param(&attr_name.name) {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
                for method_ctx in ctx.methods_list.iter() {
                    if let Some(vi) = method_ctx.get_current_scope_non_param(&attr_name.name) {
                        self.validate_visibility(attr_name, vi, input, namespace)?;
                        return Ok(vi.clone());
                    }
                }
            }
            return Err(TyCheckError::singular_no_attr_error(
                self.cfg.input.clone(),
                line!() as usize,
                attr_name.loc(),
                namespace.name.to_string(),
                obj.qual_name().as_deref().unwrap_or("?"),
                obj.ref_t(),
                attr_name.inspect(),
                self.get_similar_attr_from_singular(obj, attr_name.inspect()),
            ));
        }
        match self.get_attr_type_by_name(obj, attr_name, namespace) {
            Triple::Ok(method) => {
                let def_t = self.instantiate_def_type(&method.definition_type).unwrap();
                let list = UndoableLinkedList::new();
                self.undoable_sub_unify(obj.ref_t(), &def_t, obj, &list, None)
                    // HACK: change this func's return type to TyCheckResult<Type>
                    .map_err(|mut errs| errs.remove(0))?;
                drop(list);
                self.sub_unify(obj.ref_t(), &def_t, obj, None).unwrap();
                return Ok(method.method_info.clone());
            }
            Triple::Err(err) => {
                return Err(err);
            }
            _ => {}
        }
        for patch in self.find_patches_of(obj.ref_t()) {
            if let Some(vi) = patch.get_current_scope_non_param(&attr_name.name) {
                self.validate_visibility(attr_name, vi, input, namespace)?;
                return Ok(vi.clone());
            }
            for methods_ctx in patch.methods_list.iter() {
                if let Some(vi) = methods_ctx.get_current_scope_non_param(&attr_name.name) {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
            }
        }
        let coerced = self
            .coerce(obj.t(), &())
            .map_err(|mut errs| errs.remove(0))?;
        if &coerced != obj.ref_t() {
            let hash = get_hash(obj.ref_t());
            obj.ref_t().destructive_coerce();
            if get_hash(obj.ref_t()) != hash {
                return self
                    .search_method_info(obj, attr_name, pos_args, kw_args, input, namespace);
            }
        }
        Err(TyCheckError::no_attr_error(
            self.cfg.input.clone(),
            line!() as usize,
            attr_name.loc(),
            namespace.name.to_string(),
            obj.ref_t(),
            attr_name.inspect(),
            self.get_similar_attr(obj.ref_t(), attr_name.inspect()),
        ))
    }

    fn search_method_info_without_args(
        &self,
        obj: &hir::Expr,
        attr_name: &Identifier,
        input: &Input,
        namespace: &Context,
    ) -> SingleTyCheckResult<VarInfo> {
        match self.get_attr_info_from_attributive(obj.ref_t(), attr_name, namespace) {
            Triple::Ok(vi) => {
                return Ok(vi);
            }
            Triple::Err(e) => {
                return Err(e);
            }
            _ => {}
        }
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
            if let Some(vi) = ctx.get_current_scope_non_param(&attr_name.name) {
                self.validate_visibility(attr_name, vi, input, namespace)?;
                return Ok(vi.clone());
            }
            for methods_ctx in ctx.methods_list.iter() {
                if let Some(vi) = methods_ctx.get_current_scope_non_param(&attr_name.name) {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
            }
            if let Some(ctx) = self.get_same_name_context(&ctx.name) {
                match ctx.rec_get_var_info(attr_name, AccessKind::BoundAttr, input, namespace) {
                    Triple::Ok(vi) => {
                        return Ok(vi);
                    }
                    Triple::Err(e) => {
                        return Err(e);
                    }
                    Triple::None => {}
                }
            }
        }
        if let Ok(singular_ctxs) = self.get_singular_ctxs_by_hir_expr(obj, namespace) {
            for ctx in singular_ctxs {
                if let Some(vi) = ctx.get_current_scope_non_param(&attr_name.name) {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
                for method_ctx in ctx.methods_list.iter() {
                    if let Some(vi) = method_ctx.get_current_scope_non_param(&attr_name.name) {
                        self.validate_visibility(attr_name, vi, input, namespace)?;
                        return Ok(vi.clone());
                    }
                }
            }
            return Err(TyCheckError::singular_no_attr_error(
                self.cfg.input.clone(),
                line!() as usize,
                attr_name.loc(),
                namespace.name.to_string(),
                obj.qual_name().as_deref().unwrap_or("?"),
                obj.ref_t(),
                attr_name.inspect(),
                self.get_similar_attr_from_singular(obj, attr_name.inspect()),
            ));
        }
        match self.get_attr_type_by_name(obj, attr_name, namespace) {
            Triple::Ok(method) => {
                let def_t = self.instantiate_def_type(&method.definition_type).unwrap();
                let list = UndoableLinkedList::new();
                self.undoable_sub_unify(obj.ref_t(), &def_t, obj, &list, None)
                    // HACK: change this func's return type to TyCheckResult<Type>
                    .map_err(|mut errs| errs.remove(0))?;
                drop(list);
                self.sub_unify(obj.ref_t(), &def_t, obj, None).unwrap();
                return Ok(method.method_info.clone());
            }
            Triple::Err(err) => {
                return Err(err);
            }
            _ => {}
        }
        for patch in self.find_patches_of(obj.ref_t()) {
            if let Some(vi) = patch.get_current_scope_non_param(&attr_name.name) {
                self.validate_visibility(attr_name, vi, input, namespace)?;
                return Ok(vi.clone());
            }
            for methods_ctx in patch.methods_list.iter() {
                if let Some(vi) = methods_ctx.get_current_scope_non_param(&attr_name.name) {
                    self.validate_visibility(attr_name, vi, input, namespace)?;
                    return Ok(vi.clone());
                }
            }
        }
        let coerced = self
            .coerce(obj.t(), &())
            .map_err(|mut errs| errs.remove(0))?;
        if &coerced != obj.ref_t() {
            let hash = get_hash(obj.ref_t());
            obj.ref_t().destructive_coerce();
            if get_hash(obj.ref_t()) != hash {
                return self.search_method_info_without_args(obj, attr_name, input, namespace);
            }
        }
        Err(TyCheckError::no_attr_error(
            self.cfg.input.clone(),
            line!() as usize,
            attr_name.loc(),
            namespace.name.to_string(),
            obj.ref_t(),
            attr_name.inspect(),
            self.get_similar_attr(obj.ref_t(), attr_name.inspect()),
        ))
    }

    pub(crate) fn validate_visibility(
        &self,
        ident: &Identifier,
        vi: &VarInfo,
        input: &Input,
        namespace: &Context,
    ) -> SingleTyCheckResult<()> {
        if vi.vis.compatible(&ident.acc_kind(), namespace) {
            Ok(())
        } else {
            Err(TyCheckError::visibility_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                vi.vis.clone(),
            ))
        }
    }

    // HACK: dname.loc()はダミーLocationしか返さないので、エラーならop.loc()で上書きする
    fn append_loc_info(&self, e: TyCheckError, loc: Location) -> TyCheckError {
        if e.core.loc == Location::Unknown {
            let mut sub_msgs = Vec::new();
            for sub_msg in e.core.sub_messages {
                sub_msgs.push(SubMessage::ambiguous_new(loc, sub_msg.msg, sub_msg.hint));
            }
            let core = ErrorCore::new(
                sub_msgs,
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
        namespace: &Context,
    ) -> TyCheckResult<VarInfo> {
        erg_common::debug_power_assert!(args.len() == 2);
        let Some(cont) = binop_to_dname(op.inspect()) else {
            return Err(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                op.loc(),
                namespace.caused_by(),
                op.inspect(),
                None,
            )
            .into());
        };
        // not a `Token::from_str(op.kind, cont)` because ops are defined as symbols
        let symbol = Token::symbol_with_loc(Str::rc(cont), Location::concat(&args[0], &args[1]));
        let ident = Identifier::private_from_token(symbol.clone());
        let t = self
            .rec_get_var_info(&ident, AccessKind::Name, input, namespace)
            .none_or_result(|| {
                let (similar_info, similar_name) =
                    namespace.get_similar_name_and_info(ident.inspect()).unzip();
                TyCheckError::detailed_no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    ident.loc(),
                    namespace.caused_by(),
                    ident.inspect(),
                    similar_name,
                    similar_info,
                )
            })?;
        let op = hir::Expr::Accessor(hir::Accessor::private(symbol, t));
        self.get_call_t(&op, &None, args, &[], input, namespace)
            .map_err(|(_, errs)| {
                let hir::Expr::Accessor(hir::Accessor::Ident(op_ident)) = op else {
                    return errs;
                };
                let vi = op_ident.vi.clone();
                let lhs = args[0].expr.clone();
                let rhs = args[1].expr.clone();
                let bin = hir::BinOp::new(op_ident.raw.name.into_token(), lhs, rhs, vi);
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
        namespace: &Context,
    ) -> TyCheckResult<VarInfo> {
        erg_common::debug_power_assert!(args.len() == 1);
        let Some(cont) = unaryop_to_dname(op.inspect()) else {
            return Err(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                op.loc(),
                namespace.caused_by(),
                op.inspect(),
                None,
            )
            .into());
        };
        let symbol = Token::symbol(cont);
        let ident = Identifier::private_from_token(symbol.clone());
        let vi = self
            .rec_get_var_info(&ident, AccessKind::Name, input, namespace)
            .none_or_result(|| {
                let (similar_info, similar_name) =
                    namespace.get_similar_name_and_info(ident.inspect()).unzip();
                TyCheckError::detailed_no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    ident.loc(),
                    namespace.caused_by(),
                    ident.inspect(),
                    similar_name,
                    similar_info,
                )
            })?;
        let op = hir::Expr::Accessor(hir::Accessor::private(symbol, vi));
        self.get_call_t(&op, &None, args, &[], input, namespace)
            .map_err(|(_, errs)| {
                let hir::Expr::Accessor(hir::Accessor::Ident(op_ident)) = op else {
                    return errs;
                };
                let vi = op_ident.vi.clone();
                let expr = args[0].expr.clone();
                let unary = hir::UnaryOp::new(op_ident.raw.name.into_token(), expr, vi);
                let errs = errs
                    .into_iter()
                    .map(|e| self.append_loc_info(e, unary.loc()))
                    .collect();
                TyCheckErrors::new(errs)
            })
    }

    /// Propagate mutable dependent types changes
    /// 可変依存型の変更を伝搬させる
    /// ```erg
    /// v = ![] # Γ: { v: [Int; 0]! }
    /// v.push! 1 # v: [Int; 0]! ~> [Int; 1]!; Γ: { v: [Int; 1]! }
    /// v # : [Int; 1]!
    /// ```
    pub(crate) fn propagate(
        &mut self,
        subr_t: &mut Type,
        receiver: &hir::Expr,
    ) -> TyCheckResult<()> {
        if let Type::Subr(subr) = subr_t {
            if let Some(self_t) = subr.mut_self_t() {
                log!(info "Propagating:\n {self_t}");
                if let RefMut {
                    after: Some(after), ..
                } = self_t
                {
                    log!(info "~> {after}\n");
                    *self_t = *after.clone();
                    if let hir::Expr::Accessor(hir::Accessor::Ident(ident)) = receiver {
                        if let Some(vi) = self.rec_get_mut_var_info(&ident.raw, AccessKind::Name) {
                            vi.t = self_t.clone();
                        }
                    }
                }
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn not_callable_error(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        other: &Type,
        hint: Option<String>,
    ) -> TyCheckErrors {
        let (loc, name) = if let Some(attr_name) = attr_name {
            (
                Location::concat(obj, attr_name),
                (obj.to_string() + &attr_name.to_string()),
            )
        } else {
            (obj.loc(), obj.to_string())
        };
        let other = self.readable_type(other.clone());
        TyCheckErrors::from(TyCheckError::type_mismatch_error(
            self.cfg.input.clone(),
            line!() as usize,
            loc,
            self.caused_by(),
            &name,
            None,
            &mono("Callable"),
            &other,
            self.get_candidates(&other),
            hint,
        ))
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
    /// ↓ don't substitute `Int` to `self`
    /// substitute_call(obj: Int, instance: ((self: Int, other: Int) -> Int), [1, 2]) => instance: (Int, Int) -> Int
    /// ```
    fn substitute_call(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        instance: &Type,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<SubstituteResult> {
        match instance {
            Type::FreeVar(fv) if fv.is_linked() => {
                self.substitute_call(obj, attr_name, &fv.crack(), pos_args, kw_args)
            }
            Type::FreeVar(fv) => {
                if let Some(sub) = fv.get_sub() {
                    if !self.subtype_of(&sub, &mono("GenericCallable")) {
                        return Err(self.not_callable_error(obj, attr_name, instance, None));
                    }
                    if sub != Never {
                        let hash = get_hash(instance);
                        instance.destructive_coerce();
                        if instance.is_quantified_subr() {
                            let instance = self.instantiate(instance.clone(), obj)?;
                            self.substitute_call(obj, attr_name, &instance, pos_args, kw_args)?;
                            return Ok(SubstituteResult::Coerced(instance));
                        } else if get_hash(instance) != hash {
                            return self
                                .substitute_call(obj, attr_name, instance, pos_args, kw_args);
                        }
                    }
                }
                if let Some(attr_name) = attr_name {
                    feature_error!(
                        TyCheckErrors,
                        TyCheckError,
                        self,
                        attr_name.loc(),
                        "substitute_call for methods/type-var"
                    )
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
                    let subr_t = subr_t(kind, non_default_params, None, vec![], None, ret_t);
                    self.occur(&subr_t, instance, obj)?;
                    instance.destructive_link(&subr_t);
                    Ok(SubstituteResult::Ok)
                }
            }
            Type::Refinement(refine) => {
                self.substitute_call(obj, attr_name, &refine.t, pos_args, kw_args)
            }
            // instance must be instantiated
            Type::Quantified(_) => unreachable_error!(TyCheckErrors, TyCheckError, self),
            Type::Subr(subr) => {
                let mut errs = TyCheckErrors::empty();
                // method: obj: 1, subr: (self: Int, other: Int) -> Int
                // non-method: obj: Int, subr: (self: Int, other: Int) -> Int
                // FIXME: staticmethod
                let is_method = subr
                    .self_t()
                    .map_or(false, |self_t| self.subtype_of(obj.ref_t(), self_t));
                let callee = if let Some(ident) = attr_name {
                    if is_method {
                        obj.clone()
                    } else {
                        let attr =
                            hir::Attribute::new(obj.clone(), hir::Identifier::bare(ident.clone()));
                        hir::Expr::Accessor(hir::Accessor::Attr(attr))
                    }
                } else {
                    obj.clone()
                };
                let params_len = if is_method {
                    subr.non_default_params.len().saturating_sub(1) + subr.default_params.len()
                } else {
                    subr.non_default_params.len() + subr.default_params.len()
                };
                if (params_len < pos_args.len() || params_len < pos_args.len() + kw_args.len())
                    && subr.is_no_var()
                {
                    return Err(
                        self.gen_too_many_args_error(&callee, subr, is_method, pos_args, kw_args)
                    );
                }
                let mut passed_params = set! {};
                let non_default_params = if is_method {
                    let mut non_default_params = subr.non_default_params.iter();
                    let self_pt = non_default_params.next().unwrap();
                    if let Err(mut es) =
                        self.sub_unify(obj.ref_t(), self_pt.typ(), obj, self_pt.name())
                    {
                        errs.append(&mut es);
                    }
                    passed_params.insert("self".into());
                    non_default_params
                } else {
                    subr.non_default_params.iter()
                };
                let non_default_params_len = non_default_params.len();
                if pos_args.len() >= non_default_params_len {
                    let (non_default_args, var_args) = pos_args.split_at(non_default_params_len);
                    let mut args = non_default_args
                        .iter()
                        .zip(non_default_params)
                        .enumerate()
                        .collect::<Vec<_>>();
                    // TODO: remove `obj.local_name() != Some("__contains__")`
                    if obj.local_name() != Some("__contains__")
                        && !subr.essential_qnames().is_empty()
                    {
                        args.sort_by(|(_, (l, _)), (_, (r, _))| {
                            l.expr.complexity().cmp(&r.expr.complexity())
                        });
                    }
                    for (i, (nd_arg, nd_param)) in args {
                        if let Err(mut es) = self.substitute_pos_arg(
                            &callee,
                            attr_name,
                            &nd_arg.expr,
                            i + 1,
                            nd_param,
                            &mut passed_params,
                        ) {
                            errs.append(&mut es);
                        }
                    }
                    let mut nth = 1 + non_default_params_len;
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
                            if let Err(mut es) = self.sub_unify(default, ty, obj, not_passed.name())
                            {
                                errs.append(&mut es);
                            }
                        }
                    }
                } else {
                    let mut nth = 1;
                    // pos_args.len() < non_default_params_len
                    // don't use `zip`
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
                        .enumerate()
                        .filter(|(_, pt)| {
                            pt.name().map_or(true, |name| !passed_params.contains(name))
                        })
                        .map(|(i, pt)| {
                            let n = if is_method { i } else { i + 1 };
                            let nth = format!("({} param)", ordinal_num(n));
                            pt.name()
                                .map_or(nth.clone(), |name| format!("{name} {nth}"))
                                .into()
                        })
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
                    /*if subr.has_qvar() {
                        panic!("{subr} has qvar");
                    }*/
                    Ok(SubstituteResult::Ok)
                } else {
                    Err(errs)
                }
            }
            Type::Failure => Ok(SubstituteResult::Ok),
            _ => self.substitute_dunder_call(obj, attr_name, instance, pos_args, kw_args),
        }
    }

    fn substitute_dunder_call(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        instance: &Type,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<SubstituteResult> {
        let ctxs = self
            .get_singular_ctxs_by_hir_expr(obj, self)
            .ok()
            .unwrap_or_default();
        let one = attr_name
            .as_ref()
            .map(|attr| {
                ctxs.iter()
                    .flat_map(|ctx| {
                        ctx.get_singular_ctxs_by_ident(attr, self)
                            .ok()
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or(ctxs);
        let two = obj
            .qual_name()
            .map(|name| {
                self.get_same_name_context(&name)
                    .map_or(vec![], |ctx| vec![ctx])
            })
            .unwrap_or_default();
        let fallbacks = one.into_iter().chain(two);
        for typ_ctx in fallbacks {
            if let Some(call_vi) = typ_ctx.get_current_scope_var(&VarName::from_static("__call__"))
            {
                let instance = self.instantiate_def_type(&call_vi.t)?;
                self.substitute_call(obj, attr_name, &instance, pos_args, kw_args)?;
                return Ok(SubstituteResult::__Call__(instance));
            }
        }
        let hint = if self.subtype_of(instance, &ClassType) {
            let acc = attr_name.as_ref().map_or(obj.to_string_notype(), |attr| {
                obj.to_string_notype() + &attr.to_string()
            });
            Some(switch_lang! {
                "japanese" => format!("インスタンスを生成したい場合は、{acc}.newを使用してください"),
                "simplified_chinese" => format!("如果要生成实例，请使用 {acc}.new"),
                "traditional_chinese" => format!("如果要生成實例，請使用 {acc}.new"),
                "english" => format!("If you want to generate an instance, use {acc}.new"),
            })
        } else {
            None
        };
        Err(self.not_callable_error(obj, attr_name, instance, hint))
    }

    fn gen_too_many_args_error(
        &self,
        callee: &hir::Expr,
        subr_ty: &SubrType,
        is_method: bool,
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
            let params_len = if is_method {
                subr_ty.non_default_params.len().saturating_sub(1) + subr_ty.default_params.len()
            } else {
                subr_ty.non_default_params.len() + subr_ty.default_params.len()
            };
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
                let similar =
                    levenshtein::get_similar_name(subr_ty.param_names(), arg.keyword.inspect());
                TyCheckError::unexpected_kw_arg_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    arg.loc(),
                    &callee.to_string(),
                    self.caused_by(),
                    arg.keyword.inspect(),
                    similar,
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
        let param_t = param.typ();
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
        } else {
            passed_params.insert(Str::from(format!("({} param)", ordinal_num(nth))));
        }
        self.sub_unify(arg_t, param_t, arg, param.name())
            .map_err(|errs| {
                log!(err "semi-unification failed with {callee}\n{arg_t} !<: {param_t}");
                let name = if let Some(attr) = attr_name {
                    format!("{callee}{attr}")
                } else {
                    callee.show_acc().unwrap_or_default()
                };
                let name = name + "::" + param.name().map(|s| readable_name(&s[..])).unwrap_or("");
                let mut hint = self.get_call_type_mismatch_hint(
                    callee.ref_t(),
                    attr_name.as_ref().map(|i| &i.inspect()[..]),
                    nth,
                    param_t,
                    arg_t,
                );
                let param_t = self.readable_type(param_t.clone());
                let arg_t = self.readable_type(arg_t.clone());
                TyCheckErrors::new(
                    errs.into_iter()
                        .map(|e| {
                            log!("err: {e}");
                            TyCheckError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                e.core.loc,
                                e.caused_by,
                                &name[..],
                                Some(nth),
                                &param_t,
                                &arg_t,
                                self.get_candidates(&arg_t),
                                e.core
                                    .get_hint()
                                    .map(|s| s.to_string())
                                    .or(std::mem::take(&mut hint)),
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
        let param_t = param.typ();
        self.sub_unify(arg_t, param_t, arg, param.name())
            .map_err(|errs| {
                log!(err "semi-unification failed with {callee}\n{arg_t} !<: {param_t}");
                let name = if let Some(attr) = attr_name {
                    format!("{callee}{attr}")
                } else {
                    callee.show_acc().unwrap_or_default()
                };
                let name = name + "::" + param.name().map(|s| readable_name(&s[..])).unwrap_or("");
                let hint = self.get_simple_type_mismatch_hint(param_t, arg_t);
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
                                hint.clone(),
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
            let param_t = pt.typ();
            passed_params.insert(kw_name.clone());
            self.sub_unify(arg_t, param_t, arg, Some(kw_name))
                .map_err(|errs| {
                    log!(err "semi-unification failed with {callee}\n{arg_t} !<: {}", pt.typ());
                    let name = if let Some(attr) = attr_name {
                        format!("{callee}{attr}")
                    } else {
                        callee.show_acc().unwrap_or_default()
                    };
                    let name = name + "::" + readable_name(kw_name);
                    let hint = self.get_simple_type_mismatch_hint(param_t, arg_t);
                    let param_t = self.readable_type(param_t.clone());
                    let arg_t = self.readable_type(arg_t.clone());
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
                                    &param_t,
                                    &arg_t,
                                    self.get_candidates(&arg_t),
                                    hint.clone(),
                                )
                            })
                            .collect(),
                    )
                })?;
        } else if let Some(kw_var) = subr_ty.kw_var_params.as_deref() {
            self.sub_unify(arg_t, kw_var.typ(), arg, Some(kw_name))?;
        } else {
            let similar =
                levenshtein::get_similar_name(subr_ty.param_names(), arg.keyword.inspect());
            return Err(TyCheckErrors::from(TyCheckError::unexpected_kw_arg_error(
                self.cfg.input.clone(),
                line!() as usize,
                arg.keyword.loc(),
                &callee.to_string(),
                self.caused_by(),
                kw_name,
                similar,
            )));
        }
        Ok(())
    }

    pub(crate) fn get_call_t_without_args(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        input: &Input,
        expected_return: Option<&Type>,
        namespace: &Context,
    ) -> FailableOption<VarInfo> {
        let found = self
            .search_callee_info_without_args(obj, attr_name, input, namespace)
            .map_err(|err| (None, TyCheckErrors::from(err)))?;
        log!(
            "Found:\ncallee: {obj}{}\nfound: {found}",
            fmt_option!(pre ".", attr_name.as_ref().map(|ident| &ident.name))
        );
        let instance = self
            .instantiate(found.t.clone(), obj)
            .map_err(|errs| (Some(found.clone()), errs))?;
        log!("Instantiated:\ninstance: {instance}");
        debug_assert!(
            !instance.is_quantified_subr(),
            "{instance} is quantified subr"
        );
        log!(info "Substituted:\ninstance: {instance}");
        debug_assert!(
            instance.is_type() || instance.has_no_qvar(),
            "{instance} has qvar (obj: {obj}, attr: {}",
            fmt_option!(attr_name)
        );
        if let Some((expected, instance)) = expected_return.zip(instance.return_t()) {
            let _res = self.sub_unify(instance, expected, obj, None);
        }
        let res = VarInfo {
            t: instance,
            ..found
        };
        Ok(res)
    }

    pub(crate) fn get_call_t(
        &self,
        obj: &hir::Expr,
        attr_name: &Option<Identifier>,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        input: &Input,
        namespace: &Context,
    ) -> FailableOption<VarInfo> {
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
        let found = self
            .search_callee_info(obj, attr_name, pos_args, kw_args, input, namespace)
            .map_err(|err| (None, TyCheckErrors::from(err)))?;
        log!(
            "Found:\ncallee: {obj}{}\nfound: {found}",
            fmt_option!(pre ".", attr_name.as_ref().map(|ident| &ident.name))
        );
        let instance = self
            .instantiate(found.t.clone(), obj)
            .map_err(|errs| (Some(found.clone()), errs))?;
        log!(
            "Instantiated:\ninstance: {instance}\npos_args: ({})\nkw_args: ({})",
            fmt_slice(pos_args),
            fmt_slice(kw_args)
        );
        let instance = match self
            .substitute_call(obj, attr_name, &instance, pos_args, kw_args)
            .map_err(|errs| {
                (
                    Some(VarInfo {
                        t: instance.clone(),
                        ..found.clone()
                    }),
                    errs,
                )
            })? {
            SubstituteResult::Ok => instance,
            SubstituteResult::__Call__(__call__) => __call__,
            SubstituteResult::Coerced(coerced) => coerced,
        };
        debug_assert!(
            !instance.is_quantified_subr(),
            "{instance} is quantified subr"
        );
        log!(info "Substituted:\ninstance: {instance}");
        let res = self
            .eval_t_params(instance, self.level, obj)
            .map_err(|(t, errs)| {
                log!(err "failed to eval: {t}");
                (Some(VarInfo { t, ..found.clone() }), errs)
            })?;
        debug_assert!(res.has_no_qvar(), "{res} has qvar");
        log!(info "Params evaluated:\nres: {res}\n");
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
        levenshtein::get_similar_name(
            self.dir().into_iter().map(|(vn, _)| &vn.inspect()[..]),
            name,
        )
    }

    pub(crate) fn get_similar_name_and_info(&self, name: &str) -> Option<(&VarInfo, &str)> {
        levenshtein::get_similar_name_and_some(
            self.dir()
                .into_iter()
                .map(|(vn, vi)| (vi, &vn.inspect()[..])),
            name,
        )
    }

    pub(crate) fn get_similar_attr_from_singular<'a>(
        &'a self,
        obj: &hir::Expr,
        name: &str,
    ) -> Option<&'a str> {
        if let Ok(ctxs) = self.get_singular_ctxs_by_hir_expr(obj, self) {
            for ctx in ctxs {
                if let Some(name) = ctx.get_similar_name(name) {
                    return Some(name);
                }
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

    pub(crate) fn get_similar_attr_and_info<'a>(
        &'a self,
        self_t: &'a Type,
        name: &str,
    ) -> Option<(&'a VarInfo, &'a str)> {
        for ctx in self.get_nominal_super_type_ctxs(self_t)? {
            if let Some((vi, name)) = ctx.get_similar_name_and_info(name) {
                return Some((vi, name));
            }
        }
        None
    }

    // Returns what kind of variance the type has for each parameter Type.
    // Invariant for types not specified
    // selfが示す型が、各パラメータTypeに対してどのような変性Varianceを持つかを返す
    // 特に指定されない型に対してはInvariant
    // e.g. K(T, U) = Class(..., Impl: F(T) and Output(U) and Input(T))
    // -> K.variance() == vec![Contravariant, Covariant]
    // TODO: support keyword arguments
    pub(crate) fn type_params_variance(&self) -> Vec<Variance> {
        let match_tp_name = |tp: &TyParam, name: &VarName| -> bool {
            if let Ok(free) = <&FreeTyParam>::try_from(tp) {
                if let Some(prev) = free.get_previous() {
                    return prev.unbound_name().as_ref() == Some(name.inspect());
                }
            } else if let Ok(free) = <&FreeTyVar>::try_from(tp) {
                if let Some(prev) = free.get_previous() {
                    return prev.unbound_name().as_ref() == Some(name.inspect());
                }
            }
            tp.qual_name().as_ref() == Some(name.inspect())
        };
        let in_inout = |t: &Type, name: &VarName| {
            (&t.qual_name()[..] == "Input" || &t.qual_name()[..] == "Output")
                && t.typarams()
                    .first()
                    .map(|inner| match_tp_name(inner, name))
                    .unwrap_or(false)
        };
        self.params
            .iter()
            .map(|(opt_name, _)| {
                if let Some(name) = opt_name {
                    // トレイトの変性を調べるときはsuper_classesも見る必要がある
                    if let Some(variance_trait) = self
                        .super_traits
                        .iter()
                        .chain(self.super_classes.iter())
                        .find(|t| in_inout(t, name))
                    {
                        match &variance_trait.qual_name()[..] {
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

    /// Returns the smallest type among the iterators of a given type.
    /// If there is no subtype relationship, returns `None`.
    /// ```erg
    /// min_type([Int, Int]) == Int
    /// min_type([Int, Nat]) == Nat
    /// min_type([Int, Str]) == None
    /// min_type([Int, Str, Nat]) == None
    /// ```
    pub fn min_type<'a>(&self, types: impl Iterator<Item = &'a Type>) -> Option<&'a Type> {
        let mut opt_min = None;
        for t in types {
            if let Some(min) = opt_min {
                if self.subtype_of(min, t) {
                    continue;
                } else if self.subtype_of(t, min) {
                    opt_min = Some(t);
                } else {
                    return None;
                }
            } else {
                opt_min = Some(t);
            }
        }
        opt_min
    }

    /// Returns the largest type among the iterators of a given type.
    /// If there is no subtype relationship, returns `None`.
    /// ```erg
    /// max_type([Int, Int]) == Int
    /// max_type([Int, Nat]) == Int
    /// max_type([Int, Str]) == None
    /// max_type([Int, Str, Nat]) == None
    /// ```
    pub fn max_type<'a>(&self, types: impl Iterator<Item = &'a Type>) -> Option<&'a Type> {
        let mut opt_max = None;
        for t in types {
            if let Some(max) = opt_max {
                if self.supertype_of(max, t) {
                    continue;
                } else if self.supertype_of(t, max) {
                    opt_max = Some(t);
                } else {
                    return None;
                }
            } else {
                opt_max = Some(t);
            }
        }
        opt_max
    }

    pub fn get_nominal_super_type_ctxs<'a>(&'a self, t: &Type) -> Option<Vec<&'a TypeContext>> {
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
                _ => self.get_nominal_type_ctx(&Obj).map(|ctx| vec![ctx]),
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
    ) -> Option<impl Iterator<Item = &'a TypeContext>> {
        let ctx = self.get_nominal_type_ctx(t)?;
        let sups = ctx.super_classes.iter().chain(ctx.super_traits.iter());
        let mut sup_ctxs = vec![];
        for sup in sups {
            if let Some(ctx) = self.get_nominal_type_ctx(sup) {
                sup_ctxs.push(ctx);
            } else if DEBUG_MODE {
                todo!("no ctx ({} / {}) for {sup}", self.name, self.kind);
            }
        }
        Some(vec![ctx].into_iter().chain(sup_ctxs))
    }

    pub(crate) fn _get_super_traits(&self, typ: &Type) -> Option<impl Iterator<Item = Type>> {
        self.get_nominal_type_ctx(typ)
            .map(|ctx| ctx.super_traits.clone().into_iter())
    }

    /// include `typ` itself.
    /// if `typ` is a refinement type, include the base type (refine.t)
    pub(crate) fn get_super_classes(
        &self,
        typ: &Type,
    ) -> Option<impl Iterator<Item = Type> + Clone> {
        self.get_nominal_type_ctx(typ).map(|ctx| {
            let super_classes = ctx.super_classes.clone();
            let derefined = typ.derefine();
            if typ != &derefined {
                vec![ctx.typ.clone(), derefined]
                    .into_iter()
                    .chain(super_classes)
            } else {
                vec![ctx.typ.clone()].into_iter().chain(super_classes)
            }
        })
    }

    pub(crate) fn _get_super_types(
        &self,
        typ: &Type,
    ) -> Option<impl Iterator<Item = Type> + Clone> {
        self.get_nominal_type_ctx(typ).map(|ctx| {
            let super_classes = ctx.super_classes.clone();
            let super_traits = ctx.super_traits.clone();
            let derefined = typ.derefine();
            if typ != &derefined {
                vec![ctx.typ.clone(), derefined]
                    .into_iter()
                    .chain(super_classes)
                    .chain(super_traits)
            } else {
                vec![ctx.typ.clone()]
                    .into_iter()
                    .chain(super_classes)
                    .chain(super_traits)
            }
        })
    }

    // TODO: Never
    pub(crate) fn get_nominal_type_ctx<'a>(&'a self, typ: &Type) -> Option<&'a TypeContext> {
        match typ {
            Type::FreeVar(fv) if fv.is_linked() => {
                if let Some(res) = self.get_nominal_type_ctx(&fv.crack()) {
                    return Some(res);
                }
            }
            Type::FreeVar(fv) => {
                let sup = fv.get_super()?;
                if let Some(res) = self.get_nominal_type_ctx(&sup) {
                    return Some(res);
                }
            }
            Type::Refinement(refine) => {
                if let Predicate::Equal {
                    rhs: TyParam::Value(ValueObj::Type(typ)),
                    ..
                } = refine.pred.as_ref()
                {
                    if let Some(res) = self.get_nominal_type_ctx(typ.typ()) {
                        return Some(res);
                    }
                }
                if let Some(res) = self.get_nominal_type_ctx(&refine.t) {
                    return Some(res);
                }
            }
            Type::Quantified(_) => {
                if let Some(ctx) = self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_local_get_mono_type("QuantifiedFunc")
                {
                    return Some(ctx);
                }
            }
            Type::Subr(subr) => match subr.kind {
                SubrKind::Func => {
                    if let Some(ctx) = self
                        .get_builtins()
                        .unwrap_or(self)
                        .rec_local_get_mono_type("Func")
                    {
                        return Some(ctx);
                    }
                }
                SubrKind::Proc => {
                    if let Some(ctx) = self
                        .get_builtins()
                        .unwrap_or(self)
                        .rec_local_get_mono_type("Proc")
                    {
                        return Some(ctx);
                    }
                }
            },
            Type::Mono(name) => {
                return self.get_mono_type(name);
            }
            Type::Poly { name, .. } => {
                return self.get_poly_type(name);
            }
            /*Type::Record(rec) if rec.values().all(|attr| self.supertype_of(&Type, attr)) => {
                return self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_local_get_mono_type("RecordType");
            }*/
            Type::Record(_) => {
                return self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_local_get_mono_type("Record");
            }
            Type::NamedTuple(_) => {
                return self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_local_get_mono_type("GenericNamedTuple");
            }
            Type::Or(_l, _r) => {
                if let Some(ctx) = self.get_nominal_type_ctx(&poly("Or", vec![])) {
                    return Some(ctx);
                }
            }
            // FIXME: `F()`などの場合、実際は引数が省略されていてもmonomorphicになる
            other if other.is_monomorphic() => {
                if let Some(ctx) = self.rec_local_get_mono_type(&other.local_name()) {
                    return Some(ctx);
                }
            }
            Type::Ref(t) | Type::RefMut { before: t, .. } => {
                if let Some(res) = self.get_nominal_type_ctx(t) {
                    return Some(res);
                }
            }
            Type::Bounded { sup, .. } => {
                if let Some(res) = self.get_nominal_type_ctx(sup) {
                    return Some(res);
                }
            }
            Type::Proj { lhs, rhs } => {
                if let Ok(typ) = self.eval_proj(*lhs.clone(), rhs.clone(), self.level, &()) {
                    return self.get_nominal_type_ctx(&typ);
                }
            }
            Type::ProjCall {
                lhs,
                attr_name,
                args,
            } => {
                if let Ok(typ) = self.eval_proj_call_t(
                    *lhs.clone(),
                    attr_name.clone(),
                    args.clone(),
                    self.level,
                    &(),
                ) {
                    return self.get_nominal_type_ctx(&typ);
                }
            }
            other => {
                log!("{other} has no nominal definition");
            }
        }
        None
    }

    /// It is currently not possible to get the type defined in another module
    // TODO: Never
    pub(crate) fn get_mut_nominal_type_ctx<'a>(
        &'a mut self,
        typ: &Type,
    ) -> Option<&'a mut TypeContext> {
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
                if let Some(ctx) = self.rec_get_mut_mono_type(&typ.local_name()) {
                    return Some(ctx);
                }
            }
            Type::Poly { .. } => {
                if let Some(ctx) = self.rec_get_mut_poly_type(&typ.local_name()) {
                    return Some(ctx);
                }
            }
            // FIXME: `F()`などの場合、実際は引数が省略されていてもmonomorphicになる
            other if other.is_monomorphic() => {
                if let Some(ctx) = self.rec_get_mut_mono_type(&other.local_name()) {
                    return Some(ctx);
                }
            }
            Type::Ref(t) | Type::RefMut { before: t, .. } => {
                if let Some(res) = self.get_mut_nominal_type_ctx(t) {
                    return Some(res);
                }
            }
            Type::Bounded { sup, .. } => {
                if let Some(res) = self.get_mut_nominal_type_ctx(sup) {
                    return Some(res);
                }
            }
            other => {
                log!("{other} has no nominal definition");
            }
        }
        None
    }

    pub(crate) fn get_trait_impls(&self, trait_: &Type) -> Set<TraitImpl> {
        match trait_ {
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
                    isec.insert(TraitImpl::new(lti.sub_type.clone(), sup_trait));
                }
                isec
            }
            Type::Or(l, r) => {
                let l_impls = self.get_trait_impls(l);
                let r_impls = self.get_trait_impls(r);
                // FIXME:
                l_impls.union(&r_impls)
            }
            _ => self.get_simple_trait_impls(trait_),
        }
    }

    pub(crate) fn get_simple_trait_impls(&self, trait_: &Type) -> Set<TraitImpl> {
        let current = if let Some(impls) = self.trait_impls().get(&trait_.qual_name()) {
            impls.clone()
        } else {
            set! {}
        };
        if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            current.union(&outer.get_simple_trait_impls(trait_))
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

    /// name: Identifier.inspect()
    // FIXME: 現在の実装だとimportしたモジュールはどこからでも見れる
    pub(crate) fn get_mod(&self, name: &str) -> Option<&Context> {
        if name == "module" && ERG_MODE {
            self.get_module()
        } else if name == "global" {
            self.get_builtins()
        } else {
            let t = self.get_var_info(name).map(|(_, vi)| &vi.t)?;
            self.get_mod_with_t(t)
        }
    }

    pub fn get_mod_with_t(&self, mod_t: &Type) -> Option<&Context> {
        self.get_mod_with_path(&mod_t.module_path()?)
    }

    // rec_get_const_localとは違い、位置情報を持たないしエラーとならない
    pub(crate) fn rec_get_const_obj(&self, name: &str) -> Option<&ValueObj> {
        if name.split('.').count() > 1 {
            let typ = Type::Mono(Str::rc(name));
            let namespace = self.get_namespace(&typ.namespace())?;
            return namespace.rec_get_const_obj(&typ.local_name());
        }
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        if name == "Self" {
            if let Some(ty) = self.rec_get_self_t() {
                return self.rec_get_const_obj(&ty.local_name());
            }
        }
        if let Some(val) = self.consts.get(name) {
            return Some(val);
        }
        for ctx in self.methods_list.iter() {
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

    pub(crate) fn gen_type(&self, ident: &ast::Identifier) -> Type {
        let vis = ident.vis.display_as_accessor();
        mono(format!("{}{vis}{}", self.name, ident.inspect()))
    }

    pub(crate) fn get_namespace_path(&self, namespace: &Str) -> Option<PathBuf> {
        // get the true name
        let namespace = if let Some((_, vi)) = self.get_var_info(namespace) {
            if let Some(path) = vi.t.module_path() {
                return Some(path);
            } else {
                namespace.clone()
            }
        } else {
            namespace.clone()
        };
        let mut namespaces = namespace.split_with(&[".", "::"]);
        let mut str_namespace = namespaces.first().map(|n| n.to_string())?;
        namespaces.remove(0);
        while str_namespace.is_empty() || str_namespace.ends_with('.') {
            if namespaces.is_empty() {
                break;
            }
            str_namespace.push('.');
            str_namespace.push_str(namespaces.remove(0));
        }
        let path = Path::new(&str_namespace);
        let mut path = self.cfg.input.resolve_path(path)?;
        for p in namespaces.into_iter() {
            path = Input::try_push_path(path, Path::new(p)).ok()?;
        }
        Some(path) // NG: path.canonicalize().ok()
    }

    pub(crate) fn get_namespace(&self, namespace: &Str) -> Option<&Context> {
        if &namespace[..] == "global" {
            return self.get_builtins();
        } else if &namespace[..] == "module" || namespace.is_empty() {
            return self.get_module();
        }
        self.get_mod_with_path(self.get_namespace_path(namespace)?.as_path())
    }

    pub(crate) fn get_mono_type(&self, name: &Str) -> Option<&TypeContext> {
        if let Some(ctx) = self.rec_local_get_mono_type(name) {
            return Some(ctx);
        }
        let typ = Type::Mono(Str::rc(name));
        if self.name.starts_with(&typ.namespace()[..]) {
            if let Some(ctx) = self.rec_local_get_mono_type(&typ.local_name()) {
                return Some(ctx);
            }
        }
        if let Some(ctx) = self.get_namespace(&typ.namespace()) {
            if let Some(ctx) = ctx.rec_local_get_mono_type(&typ.local_name()) {
                return Some(ctx);
            }
        }
        None
    }

    /// you should use `get_mono_type` instead of this
    pub(crate) fn rec_local_get_mono_type(&self, name: &str) -> Option<&TypeContext> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        if let Some(ctx) = self.mono_types.get(name) {
            Some(ctx)
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_local_get_mono_type(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_local_get_poly_type(&self, name: &str) -> Option<&TypeContext> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        if let Some(ctx) = self.poly_types.get(name) {
            Some(ctx)
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_local_get_poly_type(name)
        } else {
            None
        }
    }

    pub(crate) fn get_poly_type(&self, name: &Str) -> Option<&TypeContext> {
        if let Some(ctx) = self.rec_local_get_poly_type(name) {
            return Some(ctx);
        }
        let typ = Type::Mono(Str::rc(name));
        if self.name.starts_with(&typ.namespace()[..]) {
            if let Some(ctx) = self.rec_local_get_poly_type(&typ.local_name()) {
                return Some(ctx);
            }
        }
        if let Some(ctx) = self.get_namespace(&typ.namespace()) {
            if let Some(ctx) = ctx.rec_local_get_poly_type(&typ.local_name()) {
                return Some(ctx);
            }
        }
        None
    }

    fn rec_get_mut_mono_type(&mut self, name: &str) -> Option<&mut TypeContext> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        if let Some(ctx) = self.mono_types.get_mut(name) {
            Some(ctx)
        } else if let Some(outer) = self.outer.as_mut() {
            // builtins cannot be got as mutable
            outer.rec_get_mut_mono_type(name)
        } else {
            None
        }
    }

    fn rec_get_mut_poly_type(&mut self, name: &str) -> Option<&mut TypeContext> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        if let Some(ctx) = self.poly_types.get_mut(name) {
            Some(ctx)
        } else if let Some(outer) = self.outer.as_mut() {
            outer.rec_get_mut_poly_type(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_get_mut_type(&mut self, name: &str) -> Option<&mut TypeContext> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        if let Some(ctx) = self.mono_types.get_mut(name) {
            Some(ctx)
        } else if let Some(ctx) = self.poly_types.get_mut(name) {
            Some(ctx)
        } else if let Some(outer) = self.outer.as_mut() {
            outer.rec_get_mut_type(name)
        } else {
            None
        }
    }

    pub(crate) fn get_type_ctx(&self, name: &str) -> Option<&TypeContext> {
        if let Some(ctx) = self.rec_local_get_type(name) {
            return Some(ctx);
        }
        let typ = Type::Mono(Str::rc(name));
        if self.name.starts_with(&typ.namespace()[..]) {
            if let Some(ctx) = self.rec_local_get_type(&typ.local_name()) {
                return Some(ctx);
            }
        }
        if let Some(ctx) = self.get_namespace(&typ.namespace()) {
            if let Some(ctx) = ctx.rec_local_get_type(&typ.local_name()) {
                return Some(ctx);
            }
        }
        None
    }

    pub fn get_type_info_by_str(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.get_type_ctx(name)
            .and_then(|ctx| self.get_type_info(&ctx.typ))
    }

    /// you should use `get_type` instead of this
    pub(crate) fn rec_local_get_type(&self, name: &str) -> Option<&TypeContext> {
        #[cfg(feature = "py_compat")]
        let name = self.erg_to_py_names.get(name).map_or(name, |s| &s[..]);
        if let Some(ctx) = self.mono_types.get(name) {
            Some(ctx)
        } else if let Some(ctx) = self.poly_types.get(name) {
            Some(ctx)
        } else if let Some(value) = self.consts.get(name) {
            value
                .as_type(self)
                .and_then(|typ_obj| self.get_nominal_type_ctx(typ_obj.typ()))
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_local_get_type(name)
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

    pub(crate) fn rec_get_guards(&self) -> Vec<&GuardType> {
        if let Some(outer) = self.get_outer() {
            [self.guards.iter().collect(), outer.rec_get_guards()].concat()
        } else {
            self.guards.iter().collect()
        }
    }

    // TODO: `Override` decorator should also be used
    /// e.g.
    /// ```erg
    /// [Int -> Bool, Float -> Bool] => true
    /// [Int -> Bool, (Float, Str) -> Bool] => false
    /// [Int -> Bool, Int -> Str] => false
    /// [] => true
    /// ```
    fn same_shape<'t>(&self, mut candidates: impl Iterator<Item = &'t Type>) -> bool {
        let Some(first) = candidates.next() else {
            return true;
        };
        for cand in candidates {
            if cand
                .return_t()
                .zip(first.return_t())
                .map_or(true, |(a, b)| a != b)
            {
                return false;
            }
            if cand
                .non_default_params()
                .zip(first.non_default_params())
                .map_or(true, |(a, b)| a.len() != b.len())
            {
                return false;
            }
            if cand.var_params().is_some() != first.var_params().is_some() {
                return false;
            }
            if cand
                .default_params()
                .zip(first.default_params())
                .map_or(true, |(a, b)| {
                    a.len() != b.len() || a.iter().zip(b.iter()).any(|(a, b)| a.name() != b.name())
                })
            {
                return false;
            }
        }
        true
    }

    fn get_attr_type<'m>(
        &self,
        obj: &hir::Expr,
        attr: &Identifier,
        candidates: &'m [MethodPair],
        namespace: &Context,
    ) -> Triple<&'m MethodPair, TyCheckError> {
        if candidates.first().is_none() {
            return Triple::None;
        }
        let matches = candidates
            .iter()
            .filter(|mp| self.supertype_of(&mp.definition_type, obj.ref_t()))
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            let method_pair = matches[0];
            if method_pair
                .method_info
                .vis
                .compatible(&attr.acc_kind(), self)
            {
                return Triple::Ok(method_pair);
            }
        }
        if self.same_shape(candidates.iter().map(|mp| &mp.method_info.t)) {
            // if all methods have the same return type, the minimum type (has biggest param types) is selected
            // e.g. [Float -> Bool, Int -> Bool] => Float -> Bool
            // REVIEW: should [Int -> Bool, Str -> Bool] => (Str or Int) -> Bool?
            if let Some(min) = self.min_type(candidates.iter().map(|mp| &mp.method_info.t)) {
                let min_pair = candidates
                    .iter()
                    .find(|mp| &mp.method_info.t == min)
                    .unwrap();
                if min_pair.method_info.vis.compatible(&attr.acc_kind(), self) {
                    return Triple::Ok(min_pair);
                }
            }
        }
        Triple::Err(TyCheckError::ambiguous_method_error(
            namespace.cfg.input.clone(),
            line!() as usize,
            obj,
            attr,
            &candidates
                .iter()
                .map(|mp| mp.definition_type.clone())
                .collect::<Vec<_>>(),
            namespace.caused_by(),
        ))
    }

    /// Infer the receiver type from the attribute name.
    /// Returns an error if multiple candidates are found. If nothing is found, returns None.
    fn get_attr_type_by_name(
        &self,
        receiver: &hir::Expr,
        attr: &Identifier,
        namespace: &Context,
    ) -> Triple<&MethodPair, TyCheckError> {
        if let Some(candidates) = self.method_to_traits.get(attr.inspect()) {
            return self.get_attr_type(receiver, attr, candidates, namespace);
        }
        if let Some(candidates) = self.method_to_classes.get(attr.inspect()) {
            return self.get_attr_type(receiver, attr, candidates, namespace);
        }
        if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.get_attr_type_by_name(receiver, attr, namespace)
        } else {
            Triple::None
        }
    }

    fn _get_gen_t_require_attr_t<'a>(
        &'a self,
        gen: &'a GenTypeObj,
        attr: &str,
    ) -> Option<&'a Type> {
        match gen.base_or_sup().map(|req_sup| req_sup.typ()) {
            Some(Type::Record(rec)) => {
                if let Some(t) = rec.get(attr) {
                    return Some(t);
                }
            }
            Some(other) => {
                let obj = self.rec_get_const_obj(&other.local_name());
                let obj = option_enum_unwrap!(obj, Some:(ValueObj::Type:(TypeObj::Generated:(_))))?;
                if let Some(t) = self._get_gen_t_require_attr_t(obj, attr) {
                    return Some(t);
                }
            }
            None => {}
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
                                subr.var_params.as_deref().cloned(),
                                subr.default_params.clone(),
                                subr.kw_var_params.as_deref().cloned(),
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
        match lhs {
            Type::FreeVar(fv) => {
                if let Some(sup) = fv.get_super() {
                    if self.is_trait(&sup) {
                        self.get_trait_proj_candidates(&sup, rhs)
                    } else {
                        self.eval_proj(sup, rhs.clone(), self.level, &())
                            .map_or(set! {}, |t| set! {t})
                    }
                } else {
                    set! {}
                }
            }
            Type::Failure | Type::Never => set! { lhs.clone() },
            _ => set! {},
        }
    }

    fn get_trait_proj_candidates(&self, trait_: &Type, rhs: &Str) -> Set<Type> {
        let impls = self.get_trait_impls(trait_);
        let candidates = impls.into_iter().filter_map(move |imp| {
            if self.supertype_of(&imp.sup_trait, trait_) {
                self.eval_t_params(proj(imp.sub_type, rhs), self.level, &())
                    .ok()
            } else {
                None
            }
        });
        candidates.collect()
    }

    pub fn is_class(&self, typ: &Type) -> bool {
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
                if let Some(ctx) = self.get_nominal_type_ctx(typ) {
                    ctx.kind.is_class()
                } else {
                    // TODO: unknown types
                    false
                }
            }
        }
    }

    pub fn is_trait(&self, typ: &Type) -> bool {
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
                if let Some(ctx) = self.get_nominal_type_ctx(typ) {
                    ctx.kind.is_trait()
                } else {
                    false
                }
            }
        }
    }

    // TODO:
    /// ```erg
    /// Int.meta_type() == ClassType (<: Type)
    /// Show.meta_type() == TraitType (<: Type)
    /// [Int; 3].meta_type() == [ClassType; 3] (<: Type)
    /// Indexable(T).meta_type() == TraitType (<: Type)
    /// NamedTuple({ .x = Int; .y = Str }).meta_type() == NamedTuple({ .x = ClassType; .y = ClassType })
    /// ```
    pub fn meta_type(&self, typ: &Type) -> Type {
        match typ {
            Type::Poly { name, params } if &name[..] == "Array" || &name[..] == "Set" => poly(
                name.clone(),
                params
                    .iter()
                    .map(|tp| {
                        if let Ok(t) = self.convert_tp_into_type(tp.clone()) {
                            TyParam::t(self.meta_type(&t))
                        } else {
                            tp.clone()
                        }
                    })
                    .collect(),
            ),
            NamedTuple(tuple) => NamedTuple(
                tuple
                    .iter()
                    .map(|(name, tp)| (name.clone(), self.meta_type(tp)))
                    .collect(),
            ),
            Record(rec) => Record(
                rec.iter()
                    .map(|(name, tp)| (name.clone(), self.meta_type(tp)))
                    .collect(),
            ),
            _ => Type,
        }
    }

    pub(crate) fn get_tp_from_tv_cache<'v>(
        &'v self,
        name: &str,
        tmp_tv_cache: &'v TyVarCache,
    ) -> Option<(TyParam, &'v VarInfo)> {
        if let Some(tp) = tmp_tv_cache.get_typaram(name) {
            Some((tp.clone(), &tmp_tv_cache.var_infos[name]))
        } else if let Some(t) = tmp_tv_cache.get_tyvar(name) {
            Some((TyParam::t(t.clone()), &tmp_tv_cache.var_infos[name]))
        } else if let Some(tv_ctx) = &self.tv_cache {
            if let Some(t) = tv_ctx.get_tyvar(name) {
                Some((TyParam::t(t.clone()), &tv_ctx.var_infos[name]))
            } else {
                tv_ctx
                    .get_typaram(name)
                    .cloned()
                    .map(|tp| (tp, &tv_ctx.var_infos[name]))
            }
        } else {
            None
        }
    }

    /// ```erg
    /// recover_typarams(Int, Nat) == Nat
    /// recover_typarams(Array!(Int, _), Array(Nat, 2)) == Array!(Nat, 2)
    /// ```
    /// ```erg
    /// # REVIEW: should be?
    /// recover_typarams(Nat or Str, Int) == Nat
    /// ```
    pub(crate) fn recover_typarams(&self, base: &Type, guard: &GuardType) -> TyCheckResult<Type> {
        let intersec = self.intersection(&guard.to, base);
        let is_never =
            self.subtype_of(&intersec, &Type::Never) && guard.to.as_ref() != &Type::Never;
        if !is_never {
            return Ok(intersec);
        }
        if guard.to.is_monomorphic() {
            if self.related(base, &guard.to) {
                return Ok(*guard.to.clone());
            } else {
                return Err(TyCheckErrors::from(TyCheckError::invalid_type_cast_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    guard.target.loc(),
                    self.caused_by(),
                    &guard.target.to_string(),
                    base,
                    &guard.to,
                    None,
                )));
            }
        }
        // Array(Nat, 2) !<: Array!(Int, _)
        let base_def_t = self
            .get_nominal_type_ctx(base)
            .map(|ctx| &ctx.typ)
            .unwrap_or(&Type::Obj);
        let assert_def_t = self
            .get_nominal_type_ctx(&guard.to)
            .map(|ctx| &ctx.typ)
            .unwrap_or(&Type::Obj);
        if self.related(base_def_t, assert_def_t) {
            // FIXME: Vec(_), Array(Int, 2) -> Vec(2)
            let casted = poly(base.qual_name(), guard.to.typarams());
            Ok(casted)
        } else {
            Err(TyCheckErrors::from(TyCheckError::invalid_type_cast_error(
                self.cfg.input.clone(),
                line!() as usize,
                guard.target.loc(),
                self.caused_by(),
                &guard.target.to_string(),
                base,
                &guard.to,
                None,
            )))
        }
    }

    pub(crate) fn get_instance_attr(&self, name: &str) -> Option<&VarInfo> {
        if let Some(vi) = self.locals.get(name) {
            if vi.kind.is_instance_attr() {
                return Some(vi);
            }
        }
        if let Some(vi) = self.decls.get(name) {
            if vi.kind.is_instance_attr() {
                return Some(vi);
            }
        }
        if self.kind.is_method_def() {
            self.get_nominal_type_ctx(&mono(&self.name))
                .and_then(|ctx| ctx.get_instance_attr(name))
        } else {
            self.methods_list.iter().find_map(|ctx| {
                if ctx.kind.is_trait_impl() {
                    None
                } else {
                    ctx.get_instance_attr(name)
                }
            })
        }
    }

    /// does not remove instance attribute declarations
    pub(crate) fn remove_class_attr(&mut self, name: &str) -> Option<(VarName, VarInfo)> {
        if let Some((k, v)) = self.locals.remove_entry(name) {
            if v.kind.is_instance_attr() {
                self.locals.insert(k, v);
            } else {
                return Some((k, v));
            }
        } else if let Some((k, v)) = self.decls.remove_entry(name) {
            if v.kind.is_instance_attr() {
                self.decls.insert(k, v);
            } else {
                return Some((k, v));
            }
        }
        if self.kind.is_method_def() {
            self.get_mut_nominal_type_ctx(&mono(&self.name))
                .and_then(|ctx| ctx.remove_class_attr(name))
        } else {
            self.methods_list.iter_mut().find_map(|ctx| {
                if ctx.kind.is_trait_impl() {
                    None
                } else {
                    ctx.remove_class_attr(name)
                }
            })
        }
    }
}
