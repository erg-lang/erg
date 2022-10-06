use std::option::Option;
use std::path::PathBuf;

use erg_common::config::{ErgConfig, Input};
use erg_common::levenshtein::get_similar_name;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::Str;
use erg_common::{enum_unwrap, get_hash, log, set};
use erg_type::free::HasLevel;

use ast::{DefId, Identifier, VarName};
use erg_parser::ast;

use erg_type::constructors::{func, func1, proc, ref_, ref_mut, v_enum};
use erg_type::value::{GenTypeObj, TypeKind, TypeObj, ValueObj};
use erg_type::{ParamTy, SubrType, Type};

use crate::build_hir::HIRBuilder;
use crate::context::{
    ClassDefType, Context, ContextKind, DefaultInfo, MethodType, RegistrationMode, TraitInstance,
};
use crate::error::readable_name;
use crate::error::{
    CompileResult, SingleTyCheckResult, TyCheckError, TyCheckErrors, TyCheckResult,
};
use crate::hir;
use crate::hir::Literal;
use crate::mod_cache::SharedModuleCache;
use crate::varinfo::{Mutability, ParamIdx, VarInfo, VarKind};
use Mutability::*;
use RegistrationMode::*;
use Visibility::*;

use super::instantiate::TyVarContext;
use super::OperationKind;

impl Context {
    /// If it is a constant that is defined, there must be no variable of the same name defined across all scopes
    pub(crate) fn registered_info(
        &self,
        name: &str,
        is_const: bool,
    ) -> Option<(&VarName, &VarInfo)> {
        if let Some((name, vi)) = self.params.iter().find(|(maybe_name, _)| {
            maybe_name
                .as_ref()
                .map(|n| &n.inspect()[..] == name)
                .unwrap_or(false)
        }) {
            return Some((name.as_ref().unwrap(), vi));
        } else if let Some((name, vi)) = self.locals.get_key_value(name) {
            return Some((name, vi));
        }
        if is_const {
            let outer = self.get_outer().or_else(|| self.get_builtins())?;
            outer.registered_info(name, is_const)
        } else {
            None
        }
    }

    fn _declare_var(
        &mut self,
        sig: &ast::VarSignature,
        opt_t: Option<Type>,
        id: Option<DefId>,
    ) -> TyCheckResult<()> {
        let muty = Mutability::from(&sig.inspect().unwrap()[..]);
        match &sig.pat {
            ast::VarPattern::Ident(ident) => {
                let vis = ident.vis();
                let kind = id.map_or(VarKind::Declared, VarKind::Defined);
                let sig_t = self.instantiate_var_sig_t(sig.t_spec.as_ref(), opt_t, PreRegister)?;
                if let Some(_decl) = self.decls.remove(&ident.name) {
                    Err(TyCheckErrors::from(TyCheckError::duplicate_decl_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        sig.loc(),
                        self.caused_by(),
                        ident.name.inspect(),
                    )))
                } else {
                    self.decls.insert(
                        ident.name.clone(),
                        VarInfo::new(sig_t, muty, vis, kind, None),
                    );
                    Ok(())
                }
            }
            _ => todo!(),
        }
    }

    pub(crate) fn declare_sub(
        &mut self,
        sig: &ast::SubrSignature,
        id: Option<DefId>,
    ) -> TyCheckResult<()> {
        let name = sig.ident.inspect();
        let vis = sig.ident.vis();
        let muty = Mutability::from(&name[..]);
        let kind = id.map_or(VarKind::Declared, VarKind::Defined);
        let comptime_decos = sig
            .decorators
            .iter()
            .filter_map(|deco| match &deco.0 {
                ast::Expr::Accessor(ast::Accessor::Ident(local)) if local.is_const() => {
                    Some(local.inspect().clone())
                }
                _ => None,
            })
            .collect::<Set<_>>();
        let t = self.instantiate_sub_sig_t(sig, PreRegister).map_err(|e| {
            let vi = VarInfo::new(
                Type::Failure,
                muty,
                vis,
                kind.clone(),
                Some(comptime_decos.clone()),
            );
            self.decls.insert(sig.ident.name.clone(), vi);
            e
        })?;
        let vi = VarInfo::new(t, muty, vis, kind, Some(comptime_decos));
        if let Some(_decl) = self.decls.remove(name) {
            Err(TyCheckErrors::from(TyCheckError::duplicate_decl_error(
                self.cfg.input.clone(),
                line!() as usize,
                sig.loc(),
                self.caused_by(),
                name,
            )))
        } else {
            self.decls.insert(sig.ident.name.clone(), vi);
            Ok(())
        }
    }

    pub(crate) fn assign_var_sig(
        &mut self,
        sig: &ast::VarSignature,
        body_t: &Type,
        id: DefId,
    ) -> TyCheckResult<()> {
        let ident = match &sig.pat {
            ast::VarPattern::Ident(ident) => ident,
            _ => todo!(),
        };
        // already defined as const
        if sig.is_const() {
            let vi = self.decls.remove(ident.inspect()).unwrap();
            self.locals.insert(ident.name.clone(), vi);
            return Ok(());
        }
        self.validate_var_sig_t(ident, sig.t_spec.as_ref(), body_t, Normal)?;
        let muty = Mutability::from(&ident.inspect()[..]);
        let generalized = self.generalize_t(body_t.clone());
        self.decls.remove(ident.inspect());
        let vis = ident.vis();
        let vi = VarInfo::new(generalized, muty, vis, VarKind::Defined(id), None);
        self.locals.insert(ident.name.clone(), vi);
        Ok(())
    }

    /// 宣言が既にある場合、opt_decl_tに宣言の型を渡す
    fn assign_param(
        &mut self,
        sig: &ast::ParamSignature,
        outer: Option<ParamIdx>,
        nth: usize,
        opt_decl_t: Option<&ParamTy>,
    ) -> TyCheckResult<()> {
        match &sig.pat {
            ast::ParamPattern::Lit(_) => Ok(()),
            ast::ParamPattern::Discard(_token) => Ok(()),
            ast::ParamPattern::VarName(name) => {
                if self
                    .registered_info(name.inspect(), name.is_const())
                    .is_some()
                {
                    Err(TyCheckErrors::from(TyCheckError::reassign_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        name.loc(),
                        self.caused_by(),
                        name.inspect(),
                    )))
                } else {
                    // ok, not defined
                    let spec_t = self.instantiate_param_sig_t(sig, opt_decl_t, None, Normal)?;
                    if &name.inspect()[..] == "self" {
                        let self_t = self.rec_get_self_t().unwrap();
                        self.sub_unify(&spec_t, &self_t, name.loc(), Some(name.inspect()))?;
                    }
                    let idx = if let Some(outer) = outer {
                        ParamIdx::nested(outer, nth)
                    } else {
                        ParamIdx::Nth(nth)
                    };
                    let default = if sig.opt_default_val.is_some() {
                        DefaultInfo::WithDefault
                    } else {
                        DefaultInfo::NonDefault
                    };
                    let kind =
                        VarKind::parameter(DefId(get_hash(&(&self.name, name))), idx, default);
                    let muty = Mutability::from(&name.inspect()[..]);
                    self.params.push((
                        Some(name.clone()),
                        VarInfo::new(spec_t, muty, Private, kind, None),
                    ));
                    Ok(())
                }
            }
            ast::ParamPattern::Ref(name) => {
                if self
                    .registered_info(name.inspect(), name.is_const())
                    .is_some()
                {
                    Err(TyCheckErrors::from(TyCheckError::reassign_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        name.loc(),
                        self.caused_by(),
                        name.inspect(),
                    )))
                } else {
                    // ok, not defined
                    let spec_t = self.instantiate_param_sig_t(sig, opt_decl_t, None, Normal)?;
                    if &name.inspect()[..] == "self" {
                        let self_t = self.rec_get_self_t().unwrap();
                        self.sub_unify(&spec_t, &self_t, name.loc(), Some(name.inspect()))?;
                    }
                    let spec_t = ref_(spec_t);
                    let idx = if let Some(outer) = outer {
                        ParamIdx::nested(outer, nth)
                    } else {
                        ParamIdx::Nth(nth)
                    };
                    let default = if sig.opt_default_val.is_some() {
                        DefaultInfo::WithDefault
                    } else {
                        DefaultInfo::NonDefault
                    };
                    let kind =
                        VarKind::parameter(DefId(get_hash(&(&self.name, name))), idx, default);
                    self.params.push((
                        Some(name.clone()),
                        VarInfo::new(spec_t, Immutable, Private, kind, None),
                    ));
                    Ok(())
                }
            }
            ast::ParamPattern::RefMut(name) => {
                if self
                    .registered_info(name.inspect(), name.is_const())
                    .is_some()
                {
                    Err(TyCheckErrors::from(TyCheckError::reassign_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        name.loc(),
                        self.caused_by(),
                        name.inspect(),
                    )))
                } else {
                    // ok, not defined
                    let spec_t = self.instantiate_param_sig_t(sig, opt_decl_t, None, Normal)?;
                    if &name.inspect()[..] == "self" {
                        let self_t = self.rec_get_self_t().unwrap();
                        self.sub_unify(&spec_t, &self_t, name.loc(), Some(name.inspect()))?;
                    }
                    let spec_t = ref_mut(spec_t.clone(), Some(spec_t));
                    let idx = if let Some(outer) = outer {
                        ParamIdx::nested(outer, nth)
                    } else {
                        ParamIdx::Nth(nth)
                    };
                    let default = if sig.opt_default_val.is_some() {
                        DefaultInfo::WithDefault
                    } else {
                        DefaultInfo::NonDefault
                    };
                    let kind =
                        VarKind::parameter(DefId(get_hash(&(&self.name, name))), idx, default);
                    self.params.push((
                        Some(name.clone()),
                        VarInfo::new(spec_t, Immutable, Private, kind, None),
                    ));
                    Ok(())
                }
            }
            other => {
                log!(err "{other}");
                unreachable!()
            }
        }
    }

    pub(crate) fn assign_params(
        &mut self,
        params: &ast::Params,
        opt_decl_subr_t: Option<SubrType>,
    ) -> TyCheckResult<()> {
        if let Some(decl_subr_t) = opt_decl_subr_t {
            assert_eq!(
                params.non_defaults.len(),
                decl_subr_t.non_default_params.len()
            );
            assert_eq!(params.defaults.len(), decl_subr_t.default_params.len());
            for (nth, (sig, pt)) in params
                .non_defaults
                .iter()
                .zip(decl_subr_t.non_default_params.iter())
                .enumerate()
            {
                self.assign_param(sig, None, nth, Some(pt))?;
            }
            for (nth, (sig, pt)) in params
                .defaults
                .iter()
                .zip(decl_subr_t.default_params.iter())
                .enumerate()
            {
                // TODO: .clone()
                self.assign_param(sig, None, nth, Some(pt))?;
            }
        } else {
            for (nth, sig) in params
                .non_defaults
                .iter()
                .chain(params.defaults.iter())
                .enumerate()
            {
                self.assign_param(sig, None, nth, None)?;
            }
        }
        Ok(())
    }

    /// ## Errors
    /// * TypeError: if `return_t` != typeof `body`
    /// * AssignError: if `name` has already been registered
    pub(crate) fn assign_subr(
        &mut self,
        sig: &ast::SubrSignature,
        id: DefId,
        body_t: &Type,
    ) -> TyCheckResult<Type> {
        // already defined as const
        if sig.is_const() {
            let vi = self.decls.remove(sig.ident.inspect()).unwrap();
            let t = vi.t.clone();
            self.locals.insert(sig.ident.name.clone(), vi);
            return Ok(t);
        }
        let muty = if sig.ident.is_const() {
            Mutability::Const
        } else {
            Mutability::Immutable
        };
        let name = &sig.ident.name;
        // FIXME: constでない関数
        let t = self
            .get_current_scope_var(name.inspect())
            .map(|v| &v.t)
            .unwrap();
        let non_default_params = t.non_default_params().unwrap();
        let var_args = t.var_args();
        let default_params = t.default_params().unwrap();
        if let Some(spec_ret_t) = t.return_t() {
            self.sub_unify(body_t, spec_ret_t, sig.loc(), None)
                .map_err(|errs| {
                    TyCheckErrors::new(
                        errs.into_iter()
                            .map(|e| {
                                TyCheckError::return_type_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    e.core.loc,
                                    e.caused_by,
                                    readable_name(name.inspect()),
                                    spec_ret_t,
                                    body_t,
                                )
                            })
                            .collect(),
                    )
                })?;
        }
        let sub_t = if sig.ident.is_procedural() {
            proc(
                non_default_params.clone(),
                var_args.cloned(),
                default_params.clone(),
                body_t.clone(),
            )
        } else {
            func(
                non_default_params.clone(),
                var_args.cloned(),
                default_params.clone(),
                body_t.clone(),
            )
        };
        sub_t.lift();
        let found_t = self.generalize_t(sub_t);
        if let Some(mut vi) = self.decls.remove(name) {
            if vi.t.has_unbound_var() {
                vi.t.lift();
                vi.t = self.generalize_t(vi.t.clone());
            }
            self.decls.insert(name.clone(), vi);
        }
        if let Some(vi) = self.decls.remove(name) {
            if !self.supertype_of(&vi.t, &found_t) {
                return Err(TyCheckErrors::from(TyCheckError::violate_decl_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    sig.loc(),
                    self.caused_by(),
                    name.inspect(),
                    &vi.t,
                    &found_t,
                )));
            }
        }
        let comptime_decos = sig
            .decorators
            .iter()
            .filter_map(|deco| match &deco.0 {
                ast::Expr::Accessor(ast::Accessor::Ident(local)) if local.is_const() => {
                    Some(local.inspect().clone())
                }
                _ => None,
            })
            .collect();
        let vi = VarInfo::new(
            found_t,
            muty,
            sig.ident.vis(),
            VarKind::Defined(id),
            Some(comptime_decos),
        );
        let t = vi.t.clone();
        log!(info "Registered {}::{name}: {}", self.name, t);
        self.locals.insert(name.clone(), vi);
        Ok(t)
    }

    pub(crate) fn fake_subr_assign(&mut self, sig: &ast::SubrSignature, failure_t: Type) {
        // already defined as const
        if sig.is_const() {
            let vi = self.decls.remove(sig.ident.inspect()).unwrap();
            self.locals.insert(sig.ident.name.clone(), vi);
        }
        let muty = if sig.ident.is_const() {
            Mutability::Const
        } else {
            Mutability::Immutable
        };
        let name = &sig.ident.name;
        self.decls.remove(name);
        let comptime_decos = sig
            .decorators
            .iter()
            .filter_map(|deco| match &deco.0 {
                ast::Expr::Accessor(ast::Accessor::Ident(local)) if local.is_const() => {
                    Some(local.inspect().clone())
                }
                _ => None,
            })
            .collect();
        let vi = VarInfo::new(
            failure_t,
            muty,
            sig.ident.vis(),
            VarKind::DoesNotExist,
            Some(comptime_decos),
        );
        log!(info "Registered {}::{name}: {}", self.name, &vi.t);
        self.locals.insert(name.clone(), vi);
    }

    // To allow forward references and recursive definitions
    pub(crate) fn preregister(&mut self, block: &ast::Block) -> TyCheckResult<()> {
        let mut total_errs = TyCheckErrors::empty();
        for expr in block.iter() {
            match expr {
                ast::Expr::Def(def) => {
                    if let Err(errs) = self.preregister_def(def) {
                        total_errs.extend(errs.into_iter());
                    }
                }
                ast::Expr::ClassDef(class_def) => {
                    if let Err(errs) = self.preregister_def(&class_def.def) {
                        total_errs.extend(errs.into_iter());
                    }
                }
                _ => {}
            }
        }
        if total_errs.is_empty() {
            Ok(())
        } else {
            Err(total_errs)
        }
    }

    pub(crate) fn preregister_def(&mut self, def: &ast::Def) -> TyCheckResult<()> {
        let id = Some(def.body.id);
        let __name__ = def.sig.ident().unwrap().inspect();
        match &def.sig {
            ast::Signature::Subr(sig) => {
                if sig.is_const() {
                    let bounds = self.instantiate_ty_bounds(&sig.bounds, PreRegister)?;
                    let tv_ctx = TyVarContext::new(self.level, bounds, self);
                    let vis = def.sig.vis();
                    self.grow(__name__, ContextKind::Proc, vis, Some(tv_ctx))?;
                    let (obj, const_t) =
                        match self.eval_const_block(&def.body.block, Some(__name__)) {
                            Ok(obj) => (obj.clone(), v_enum(set! {obj})),
                            Err(e) => {
                                self.pop();
                                return Err(e);
                            }
                        };
                    if let Some(spec) = sig.return_t_spec.as_ref() {
                        let spec_t = self.instantiate_typespec(spec, None, None, PreRegister)?;
                        self.sub_unify(&const_t, &spec_t, def.body.loc(), None)?;
                    }
                    self.pop();
                    self.register_gen_const(def.sig.ident().unwrap(), obj)?;
                } else {
                    self.declare_sub(sig, id)?;
                }
            }
            ast::Signature::Var(sig) if sig.is_const() => {
                let (obj, const_t) = match self.eval_const_block(&def.body.block, Some(__name__)) {
                    Ok(obj) => (obj.clone(), v_enum(set! {obj})),
                    Err(e) => {
                        return Err(e);
                    }
                };
                if let Some(spec) = sig.t_spec.as_ref() {
                    let spec_t = self.instantiate_typespec(spec, None, None, PreRegister)?;
                    self.sub_unify(&const_t, &spec_t, def.body.loc(), None)?;
                }
                self.register_gen_const(sig.ident().unwrap(), obj)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// e.g. .new
    fn register_auto_impl(
        &mut self,
        name: &'static str,
        t: Type,
        muty: Mutability,
        vis: Visibility,
    ) {
        let name = VarName::from_static(name);
        if self.locals.get(&name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.locals
                .insert(name, VarInfo::new(t, muty, vis, VarKind::Auto, None));
        }
    }

    /// e.g. ::__new__
    fn register_fixed_auto_impl(
        &mut self,
        name: &'static str,
        t: Type,
        muty: Mutability,
        vis: Visibility,
    ) {
        let name = VarName::from_static(name);
        if self.locals.get(&name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.locals
                .insert(name, VarInfo::new(t, muty, vis, VarKind::FixedAuto, None));
        }
    }

    fn _register_gen_decl(&mut self, name: VarName, t: Type, vis: Visibility) {
        if self.decls.get(&name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.decls.insert(
                name,
                VarInfo::new(t, Immutable, vis, VarKind::Declared, None),
            );
        }
    }

    fn _register_gen_impl(&mut self, name: VarName, t: Type, muty: Mutability, vis: Visibility) {
        if self.locals.get(&name).is_some() {
            panic!("already registered: {name}");
        } else {
            let id = DefId(get_hash(&(&self.name, &name)));
            self.locals
                .insert(name, VarInfo::new(t, muty, vis, VarKind::Defined(id), None));
        }
    }

    pub(crate) fn register_trait(&mut self, class: Type, trait_: Type, methods: Self) {
        self.super_traits.push(trait_.clone());
        self.methods_list
            .push((ClassDefType::impl_trait(class, trait_), methods));
    }

    pub(crate) fn register_marker_trait(&mut self, trait_: Type) {
        self.super_traits.push(trait_);
    }

    pub(crate) fn register_gen_const(
        &mut self,
        ident: &Identifier,
        obj: ValueObj,
    ) -> SingleTyCheckResult<()> {
        if self.rec_get_const_obj(ident.inspect()).is_some() {
            Err(TyCheckError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            ))
        } else {
            match obj {
                ValueObj::Type(t) => match t {
                    TypeObj::Generated(gen) => {
                        self.register_gen_type(ident, gen);
                    }
                    TypeObj::Builtin(_t) => panic!("aliasing bug"),
                },
                // TODO: not all value objects are comparable
                other => {
                    let id = DefId(get_hash(ident));
                    let vi = VarInfo::new(
                        v_enum(set! {other.clone()}),
                        Const,
                        ident.vis(),
                        VarKind::Defined(id),
                        None,
                    );
                    self.consts.insert(ident.name.clone(), other);
                    self.decls.insert(ident.name.clone(), vi);
                }
            }
            Ok(())
        }
    }

    pub(crate) fn register_gen_type(&mut self, ident: &Identifier, gen: GenTypeObj) {
        match gen.kind {
            TypeKind::Class => {
                if gen.t.is_monomorphic() {
                    // let super_traits = gen.impls.iter().map(|to| to.typ().clone()).collect();
                    let mut ctx = Self::mono_class(
                        gen.t.name(),
                        self.cfg.clone(),
                        self.mod_cache.clone(),
                        self.py_mod_cache.clone(),
                        2,
                        self.level,
                    );
                    let mut methods = Self::methods(
                        gen.t.name(),
                        self.cfg.clone(),
                        self.mod_cache.clone(),
                        self.py_mod_cache.clone(),
                        2,
                        self.level,
                    );
                    let require = gen.require_or_sup.typ().clone();
                    let new_t = func1(require, gen.t.clone());
                    methods.register_fixed_auto_impl("__new__", new_t.clone(), Immutable, Private);
                    // 必要なら、ユーザーが独自に上書きする
                    methods.register_auto_impl("new", new_t, Immutable, Public);
                    ctx.methods_list
                        .push((ClassDefType::Simple(gen.t.clone()), methods));
                    self.register_gen_mono_type(ident, gen, ctx, Const);
                } else {
                    todo!("polymorphic type definition is not supported yet");
                }
            }
            TypeKind::Subclass => {
                if gen.t.is_monomorphic() {
                    let super_classes = vec![gen.require_or_sup.typ().clone()];
                    // let super_traits = gen.impls.iter().map(|to| to.typ().clone()).collect();
                    let mut ctx = Self::mono_class(
                        gen.t.name(),
                        self.cfg.clone(),
                        self.mod_cache.clone(),
                        self.py_mod_cache.clone(),
                        2,
                        self.level,
                    );
                    for sup in super_classes.into_iter() {
                        let (_, sup_ctx) = self.get_nominal_type_ctx(&sup).unwrap();
                        ctx.register_superclass(sup, sup_ctx);
                    }
                    let mut methods = Self::methods(
                        gen.t.name(),
                        self.cfg.clone(),
                        self.mod_cache.clone(),
                        self.py_mod_cache.clone(),
                        2,
                        self.level,
                    );
                    if let Some(sup) = self.rec_get_const_obj(&gen.require_or_sup.typ().name()) {
                        let sup = enum_unwrap!(sup, ValueObj::Type);
                        let param_t = match sup {
                            TypeObj::Builtin(t) => t,
                            TypeObj::Generated(t) => t.require_or_sup.as_ref().typ(),
                        };
                        // `Super.Requirement := {x = Int}` and `Self.Additional := {y = Int}`
                        // => `Self.Requirement := {x = Int; y = Int}`
                        let param_t = if let Some(additional) = &gen.additional {
                            self.intersection(param_t, additional.typ())
                        } else {
                            param_t.clone()
                        };
                        let new_t = func1(param_t, gen.t.clone());
                        methods.register_fixed_auto_impl(
                            "__new__",
                            new_t.clone(),
                            Immutable,
                            Private,
                        );
                        // 必要なら、ユーザーが独自に上書きする
                        methods.register_auto_impl("new", new_t, Immutable, Public);
                        ctx.methods_list
                            .push((ClassDefType::Simple(gen.t.clone()), methods));
                        self.register_gen_mono_type(ident, gen, ctx, Const);
                    } else {
                        todo!("super class not found")
                    }
                } else {
                    todo!("polymorphic type definition is not supported yet");
                }
            }
            TypeKind::Trait => {
                if gen.t.is_monomorphic() {
                    let mut ctx = Self::mono_trait(
                        gen.t.name(),
                        self.cfg.clone(),
                        self.mod_cache.clone(),
                        self.py_mod_cache.clone(),
                        2,
                        self.level,
                    );
                    let require = enum_unwrap!(gen.require_or_sup.as_ref(), TypeObj::Builtin:(Type::Record:(_)));
                    for (field, t) in require.iter() {
                        let muty = if field.is_const() {
                            Mutability::Const
                        } else {
                            Mutability::Immutable
                        };
                        let vi = VarInfo::new(t.clone(), muty, field.vis, VarKind::Declared, None);
                        ctx.decls
                            .insert(VarName::from_str(field.symbol.clone()), vi);
                    }
                    self.register_gen_mono_type(ident, gen, ctx, Const);
                } else {
                    todo!("polymorphic type definition is not supported yet");
                }
            }
            TypeKind::Subtrait => {
                if gen.t.is_monomorphic() {
                    let super_classes = vec![gen.require_or_sup.typ().clone()];
                    // let super_traits = gen.impls.iter().map(|to| to.typ().clone()).collect();
                    let mut ctx = Self::mono_trait(
                        gen.t.name(),
                        self.cfg.clone(),
                        self.mod_cache.clone(),
                        self.py_mod_cache.clone(),
                        2,
                        self.level,
                    );
                    let additional = gen.additional.as_ref().map(|additional| enum_unwrap!(additional.as_ref(), TypeObj::Builtin:(Type::Record:(_))));
                    if let Some(additional) = additional {
                        for (field, t) in additional.iter() {
                            let muty = if field.is_const() {
                                Mutability::Const
                            } else {
                                Mutability::Immutable
                            };
                            let vi =
                                VarInfo::new(t.clone(), muty, field.vis, VarKind::Declared, None);
                            ctx.decls
                                .insert(VarName::from_str(field.symbol.clone()), vi);
                        }
                    }
                    for sup in super_classes.into_iter() {
                        let (_, sup_ctx) = self.get_nominal_type_ctx(&sup).unwrap();
                        ctx.register_supertrait(sup, sup_ctx);
                    }
                    self.register_gen_mono_type(ident, gen, ctx, Const);
                } else {
                    todo!("polymorphic type definition is not supported yet");
                }
            }
            other => todo!("{other:?}"),
        }
    }

    fn register_gen_mono_type(
        &mut self,
        ident: &Identifier,
        gen: GenTypeObj,
        ctx: Self,
        muty: Mutability,
    ) {
        // FIXME: not panic but error
        // FIXME: recursive search
        if self.mono_types.contains_key(&gen.t.name()) {
            panic!("{} has already been registered", gen.t.name());
        } else if self.rec_get_const_obj(&gen.t.name()).is_some() {
            panic!("{} has already been registered as const", gen.t.name());
        } else {
            let t = gen.t.clone();
            let meta_t = gen.meta_type();
            let name = &ident.name;
            let id = DefId(get_hash(&(&self.name, &name)));
            self.decls.insert(
                name.clone(),
                VarInfo::new(meta_t, muty, ident.vis(), VarKind::Defined(id), None),
            );
            self.consts
                .insert(name.clone(), ValueObj::Type(TypeObj::Generated(gen)));
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls.get_mut(&impl_trait.name()) {
                    impls.insert(TraitInstance::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls.insert(
                        impl_trait.name(),
                        set![TraitInstance::new(t.clone(), impl_trait.clone())],
                    );
                }
            }
            for (trait_method, vi) in ctx.decls.iter() {
                if let Some(types) = self.method_to_traits.get_mut(trait_method.inspect()) {
                    types.push(MethodType::new(t.clone(), vi.t.clone()));
                } else {
                    self.method_to_traits.insert(
                        trait_method.inspect().clone(),
                        vec![MethodType::new(t.clone(), vi.t.clone())],
                    );
                }
            }
            for (class_method, vi) in ctx.locals.iter() {
                if let Some(types) = self.method_to_classes.get_mut(class_method.inspect()) {
                    types.push(MethodType::new(t.clone(), vi.t.clone()));
                } else {
                    self.method_to_classes.insert(
                        class_method.inspect().clone(),
                        vec![MethodType::new(t.clone(), vi.t.clone())],
                    );
                }
            }
            self.mono_types.insert(name.clone(), (t, ctx));
        }
    }

    pub(crate) fn import_mod(
        &mut self,
        kind: OperationKind,
        mod_name: &Literal,
    ) -> CompileResult<PathBuf> {
        if kind.is_erg_import() {
            self.import_erg_mod(mod_name)
        } else {
            self.import_py_mod(mod_name)
        }
    }

    fn import_erg_mod(&mut self, mod_name: &Literal) -> CompileResult<PathBuf> {
        let __name__ = enum_unwrap!(mod_name.value.clone(), ValueObj::Str);
        let mod_cache = self.mod_cache.as_ref().unwrap();
        let py_mod_cache = self.py_mod_cache.as_ref().unwrap();
        #[allow(clippy::match_single_binding)]
        match &__name__[..] {
            // TODO: erg builtin modules
            _ => self.import_user_erg_mod(__name__, mod_name, mod_cache, py_mod_cache),
        }
    }

    fn import_user_erg_mod(
        &self,
        __name__: Str,
        mod_name: &Literal,
        mod_cache: &SharedModuleCache,
        py_mod_cache: &SharedModuleCache,
    ) -> CompileResult<PathBuf> {
        let mut dir = if let Input::File(mut path) = self.cfg.input.clone() {
            path.pop();
            path
        } else {
            PathBuf::new()
        };
        dir.push(format!("{__name__}.er"));
        let path = match dir.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                let err = TyCheckErrors::from(TyCheckError::import_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    err.to_string(),
                    mod_name.loc(),
                    self.caused_by(),
                    self.mod_cache.as_ref().unwrap().get_similar_name(&__name__),
                    self.similar_builtin_py_mod_name(&__name__).or_else(|| {
                        self.py_mod_cache
                            .as_ref()
                            .unwrap()
                            .get_similar_name(&__name__)
                    }),
                ));
                return Err(err);
            }
        };
        if mod_cache.get(&path).is_some() {
            return Ok(path);
        }
        let cfg = ErgConfig::with_module_path(path.clone());
        let src = cfg.input.read();
        let mut builder =
            HIRBuilder::new_with_cache(cfg, __name__, mod_cache.clone(), py_mod_cache.clone());
        match builder.build(src, "exec") {
            Ok(hir) => {
                mod_cache.register(path.clone(), Some(hir), builder.pop_ctx());
            }
            Err((maybe_hir, errs)) => {
                if let Some(hir) = maybe_hir {
                    mod_cache.register(path, Some(hir), builder.pop_ctx());
                }
                return Err(errs);
            }
        }
        Ok(path)
    }

    fn import_py_mod(&mut self, mod_name: &Literal) -> CompileResult<PathBuf> {
        let __name__ = enum_unwrap!(mod_name.value.clone(), ValueObj::Str);
        let py_mod_cache = self.py_mod_cache.as_ref().unwrap();
        let builtin_path = PathBuf::from(&format!("<builtins>.{__name__}"));
        if py_mod_cache.get(&builtin_path).is_some() {
            return Ok(builtin_path);
        }
        match &__name__[..] {
            "importlib" => {
                py_mod_cache.register(builtin_path.clone(), None, Self::init_py_importlib_mod());
                Ok(builtin_path)
            }
            "io" => {
                py_mod_cache.register(builtin_path.clone(), None, Self::init_py_io_mod());
                Ok(builtin_path)
            }
            "math" => {
                py_mod_cache.register(builtin_path.clone(), None, Self::init_py_math_mod());
                Ok(builtin_path)
            }
            "random" => {
                py_mod_cache.register(builtin_path.clone(), None, Self::init_py_random_mod());
                Ok(builtin_path)
            }
            "socket" => {
                py_mod_cache.register(builtin_path.clone(), None, Self::init_py_socket_mod());
                Ok(builtin_path)
            }
            "sys" => {
                py_mod_cache.register(builtin_path.clone(), None, Self::init_py_sys_mod());
                Ok(builtin_path)
            }
            "time" => {
                py_mod_cache.register(builtin_path.clone(), None, Self::init_py_time_mod());
                Ok(builtin_path)
            }
            "urllib" => {
                py_mod_cache.register(builtin_path.clone(), None, Self::init_py_urllib_mod());
                Ok(builtin_path)
            }
            _ => self.import_user_py_mod(mod_name),
        }
    }

    fn similar_builtin_py_mod_name(&self, name: &Str) -> Option<Str> {
        get_similar_name(
            ["importlib", "io", "math", "random", "socket", "sys", "time"].into_iter(),
            name,
        )
        .map(Str::rc)
    }

    fn import_user_py_mod(&self, mod_name: &Literal) -> CompileResult<PathBuf> {
        let __name__ = enum_unwrap!(mod_name.value.clone(), ValueObj::Str);
        let py_mod_cache = self.py_mod_cache.as_ref().unwrap();
        let mut dir = if let Input::File(mut path) = self.cfg.input.clone() {
            path.pop();
            path
        } else {
            PathBuf::new()
        };
        dir.push(format!("{__name__}.d.er"));
        let path = match dir.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                let err = TyCheckError::import_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    err.to_string(),
                    mod_name.loc(),
                    self.caused_by(),
                    self.mod_cache.as_ref().unwrap().get_similar_name(&__name__),
                    self.similar_builtin_py_mod_name(&__name__)
                        .or_else(|| py_mod_cache.get_similar_name(&__name__)),
                );
                return Err(TyCheckErrors::from(err));
            }
        };
        if py_mod_cache.get(&path).is_some() {
            return Ok(path);
        }
        let cfg = ErgConfig::with_module_path(path.clone());
        let src = cfg.input.read();
        let mut builder =
            HIRBuilder::new_with_cache(cfg, __name__, py_mod_cache.clone(), py_mod_cache.clone());
        match builder.build(src, "declare") {
            Ok(hir) => {
                py_mod_cache.register(path.clone(), Some(hir), builder.pop_ctx());
            }
            Err((maybe_hir, errs)) => {
                if let Some(hir) = maybe_hir {
                    py_mod_cache.register(path, Some(hir), builder.pop_ctx());
                }
                return Err(errs);
            }
        }
        Ok(path)
    }

    pub fn del(&mut self, ident: &hir::Identifier) -> CompileResult<()> {
        if self.rec_get_const_obj(ident.inspect()).is_some()
            || self
                .get_builtins()
                .unwrap()
                .get_local_kv(ident.inspect())
                .is_some()
        {
            Err(TyCheckErrors::from(TyCheckError::del_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident,
                self.caused_by(),
            )))
        } else if self.locals.get(ident.inspect()).is_some() {
            self.locals.remove(ident.inspect());
            Ok(())
        } else {
            Err(TyCheckErrors::from(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                self.get_similar_name(ident.inspect()),
            )))
        }
    }
}
