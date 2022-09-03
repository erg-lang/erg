use std::option::Option; // conflicting to Type::Option

use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::Str;
use erg_common::{enum_unwrap, get_hash, log, set};
use erg_type::free::HasLevel;

use ast::{DefId, VarName};
use erg_parser::ast;

use erg_type::constructors::{enum_t, func, proc};
use erg_type::value::ValueObj;
use erg_type::{HasType, ParamTy, SubrType, TyBound, Type};
use Type::*;

use crate::context::{Context, DefaultInfo, RegistrationMode};
use crate::error::readable_name;
use crate::error::{TyCheckError, TyCheckResult};
use crate::hir;
use crate::varinfo::{Mutability, ParamIdx, VarInfo, VarKind};
use Mutability::*;
use RegistrationMode::*;
use Visibility::*;

impl Context {
    /// If it is a constant that is defined, there must be no variable of the same name defined across all scopes
    fn registered(&self, name: &Str, is_const: bool) -> bool {
        if self.params.iter().any(|(maybe_name, _)| {
            maybe_name
                .as_ref()
                .map(|n| n.inspect() == name)
                .unwrap_or(false)
        }) || self.locals.contains_key(name)
        {
            return true;
        }
        if is_const {
            if let Some(outer) = &self.outer {
                outer.registered(name, is_const)
            } else {
                false
            }
        } else {
            false
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
                if sig.t_spec.is_none() && opt_t.is_none() {
                    Err(TyCheckError::no_type_spec_error(
                        line!() as usize,
                        sig.loc(),
                        self.caused_by(),
                        ident.inspect(),
                    ))
                } else {
                    if self.registered(ident.inspect(), ident.is_const()) {
                        return Err(TyCheckError::duplicate_decl_error(
                            line!() as usize,
                            sig.loc(),
                            self.caused_by(),
                            ident.inspect(),
                        ));
                    }
                    let vis = ident.vis();
                    let kind = id.map_or(VarKind::Declared, VarKind::Defined);
                    let sig_t =
                        self.instantiate_var_sig_t(sig.t_spec.as_ref(), opt_t, PreRegister)?;
                    self.decls
                        .insert(ident.name.clone(), VarInfo::new(sig_t, muty, vis, kind));
                    Ok(())
                }
            }
            _ => todo!(),
        }
    }

    pub(crate) fn declare_sub(
        &mut self,
        sig: &ast::SubrSignature,
        opt_ret_t: Option<Type>,
        id: Option<DefId>,
    ) -> TyCheckResult<()> {
        let name = sig.ident.inspect();
        let vis = sig.ident.vis();
        let muty = Mutability::from(&name[..]);
        let kind = id.map_or(VarKind::Declared, VarKind::Defined);
        if self.registered(name, sig.is_const()) {
            return Err(TyCheckError::duplicate_decl_error(
                line!() as usize,
                sig.loc(),
                self.caused_by(),
                name,
            ));
        }
        let t = self.instantiate_sub_sig_t(sig, opt_ret_t, PreRegister)?;
        let vi = VarInfo::new(t, muty, vis, kind);
        if let Some(_decl) = self.decls.remove(name) {
            return Err(TyCheckError::duplicate_decl_error(
                line!() as usize,
                sig.loc(),
                self.caused_by(),
                name,
            ));
        } else {
            self.decls.insert(sig.ident.name.clone(), vi);
        }
        Ok(())
    }

    pub(crate) fn assign_var_sig(
        &mut self,
        sig: &ast::VarSignature,
        body_t: &Type,
        id: DefId,
    ) -> TyCheckResult<()> {
        // already defined as const
        if sig.is_const() {
            return Ok(());
        }
        let ident = match &sig.pat {
            ast::VarPattern::Ident(ident) => ident,
            _ => todo!(),
        };
        self.validate_var_sig_t(ident, sig.t_spec.as_ref(), body_t, Normal)?;
        let muty = Mutability::from(&ident.inspect()[..]);
        let generalized = self.generalize_t(body_t.clone());
        if self.registered(ident.inspect(), ident.is_const()) {
            Err(TyCheckError::reassign_error(
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            ))
        } else {
            if self.decls.remove(ident.inspect()).is_some() {
                // something to do?
            }
            let vis = ident.vis();
            let vi = VarInfo::new(generalized, muty, vis, VarKind::Defined(id));
            self.locals.insert(ident.name.clone(), vi);
            Ok(())
        }
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
            ast::ParamPattern::Discard(_token) => Ok(()),
            ast::ParamPattern::VarName(v) => {
                if self.registered(v.inspect(), v.inspect().is_uppercase()) {
                    Err(TyCheckError::reassign_error(
                        line!() as usize,
                        v.loc(),
                        self.caused_by(),
                        v.inspect(),
                    ))
                } else {
                    // ok, not defined
                    let spec_t = self.instantiate_param_sig_t(sig, opt_decl_t, Normal)?;
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
                    let kind = VarKind::parameter(DefId(get_hash(&(&self.name, v))), idx, default);
                    self.params.push((
                        Some(v.clone()),
                        VarInfo::new(spec_t, Immutable, Private, kind),
                    ));
                    Ok(())
                }
            }
            ast::ParamPattern::Lit(_) => Ok(()),
            _ => unreachable!(),
        }
    }

    pub(crate) fn assign_params(
        &mut self,
        params: &ast::Params,
        opt_decl_subr_t: Option<SubrType>,
    ) -> TyCheckResult<()> {
        if let Some(decl_subr_t) = opt_decl_subr_t {
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
    ) -> TyCheckResult<()> {
        // already defined as const
        if sig.is_const() {
            return Ok(());
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
            self.sub_unify(body_t, spec_ret_t, None, Some(sig.loc()), None)
                .map_err(|e| {
                    TyCheckError::return_type_error(
                        line!() as usize,
                        e.core.loc,
                        e.caused_by,
                        readable_name(name.inspect()),
                        spec_ret_t,
                        body_t,
                    )
                })?;
        }
        if self.registered(name.inspect(), name.inspect().is_uppercase()) {
            Err(TyCheckError::reassign_error(
                line!() as usize,
                name.loc(),
                self.caused_by(),
                name.inspect(),
            ))
        } else {
            let sub_t = if sig.ident.is_procedural() {
                proc(
                    non_default_params.clone(),
                    var_args.as_ref().map(|v| *(*v).clone()),
                    default_params.clone(),
                    body_t.clone(),
                )
            } else {
                func(
                    non_default_params.clone(),
                    var_args.as_ref().map(|v| *(*v).clone()),
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
                if !self.rec_supertype_of(&vi.t, &found_t) {
                    return Err(TyCheckError::violate_decl_error(
                        line!() as usize,
                        sig.loc(),
                        self.caused_by(),
                        name.inspect(),
                        &vi.t,
                        &found_t,
                    ));
                }
            }
            // TODO: visibility
            let vi = VarInfo::new(found_t, muty, Private, VarKind::Defined(id));
            log!(info "Registered {}::{name}: {}", self.name, &vi.t);
            self.params.push((Some(name.clone()), vi));
            Ok(())
        }
    }

    // To allow forward references and recursive definitions
    pub(crate) fn preregister(&mut self, block: &ast::Block) -> TyCheckResult<()> {
        for expr in block.iter() {
            if let ast::Expr::Def(def) = expr {
                self.preregister_def(def)?;
            }
        }
        Ok(())
    }

    pub(crate) fn preregister_def(&mut self, def: &ast::Def) -> TyCheckResult<()> {
        let id = Some(def.body.id);
        let __name__ = def.sig.ident().map(|i| i.inspect());
        match &def.sig {
            ast::Signature::Subr(sig) => {
                if sig.is_const() {
                    let (obj, const_t) = match self.eval_const_block(&def.body.block, __name__) {
                        Ok(obj) => (obj.clone(), enum_t(set! {obj})),
                        Err(e) => {
                            return Err(e);
                        }
                    };
                    if let Some(spec) = sig.return_t_spec.as_ref() {
                        let spec_t = self.instantiate_typespec(spec, PreRegister)?;
                        self.sub_unify(&const_t, &spec_t, Some(def.body.loc()), None, None)?;
                    }
                    self.register_const(__name__.unwrap(), obj);
                } else {
                    let opt_ret_t = if let Some(spec) = sig.return_t_spec.as_ref() {
                        let spec_t = self.instantiate_typespec(spec, PreRegister)?;
                        Some(spec_t)
                    } else {
                        None
                    };
                    self.declare_sub(sig, opt_ret_t, id)?;
                }
            }
            ast::Signature::Var(sig) if sig.is_const() => {
                let (obj, const_t) = match self.eval_const_block(&def.body.block, __name__) {
                    Ok(obj) => (obj.clone(), enum_t(set! {obj})),
                    Err(e) => {
                        return Err(e);
                    }
                };
                if let Some(spec) = sig.t_spec.as_ref() {
                    let spec_t = self.instantiate_typespec(spec, PreRegister)?;
                    self.sub_unify(&const_t, &spec_t, Some(def.body.loc()), None, None)?;
                }
                self.register_const(__name__.unwrap(), obj);
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn import_mod(
        &mut self,
        var_name: &VarName,
        mod_name: &hir::Expr,
    ) -> TyCheckResult<()> {
        match mod_name {
            hir::Expr::Lit(lit) => {
                if self.rec_subtype_of(&lit.value.class(), &Str) {
                    let name = enum_unwrap!(lit.value.clone(), ValueObj::Str);
                    match &name[..] {
                        "importlib" => {
                            self.mods
                                .insert(var_name.clone(), Self::init_py_importlib_mod());
                        }
                        "io" => {
                            self.mods.insert(var_name.clone(), Self::init_py_io_mod());
                        }
                        "math" => {
                            self.mods.insert(var_name.clone(), Self::init_py_math_mod());
                        }
                        "random" => {
                            self.mods
                                .insert(var_name.clone(), Self::init_py_random_mod());
                        }
                        "socket" => {
                            self.mods
                                .insert(var_name.clone(), Self::init_py_socket_mod());
                        }
                        "sys" => {
                            self.mods.insert(var_name.clone(), Self::init_py_sys_mod());
                        }
                        "time" => {
                            self.mods.insert(var_name.clone(), Self::init_py_time_mod());
                        }
                        other => todo!("importing {other}"),
                    }
                } else {
                    return Err(TyCheckError::type_mismatch_error(
                        line!() as usize,
                        mod_name.loc(),
                        self.caused_by(),
                        "import::name",
                        &Str,
                        mod_name.ref_t(),
                        self.get_type_mismatch_hint(&Str, mod_name.ref_t()),
                    ));
                }
            }
            _ => {
                return Err(TyCheckError::feature_error(
                    line!() as usize,
                    mod_name.loc(),
                    "non-literal importing",
                    self.caused_by(),
                ))
            }
        }
        Ok(())
    }

    pub(crate) fn _push_subtype_bound(&mut self, sub: Type, sup: Type) {
        self.bounds.push(TyBound::subtype_of(sub, sup));
    }

    pub(crate) fn _push_instance_bound(&mut self, name: Str, t: Type) {
        self.bounds.push(TyBound::instance(name, t));
    }
}
