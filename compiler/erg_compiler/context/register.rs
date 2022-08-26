use std::option::Option; // conflicting to Type::Option

use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::Str;
use erg_common::{enum_unwrap, get_hash, log, set};
use erg_type::free::HasLevel;

use ast::{DefId, VarName};
use erg_parser::ast;

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
    fn registered(&self, name: &Str, recursive: bool) -> bool {
        if self.params.iter().any(|(maybe_name, _)| {
            maybe_name
                .as_ref()
                .map(|n| n.inspect() == name)
                .unwrap_or(false)
        }) || self.locals.contains_key(name)
        {
            return true;
        }
        if recursive {
            if let Some(outer) = &self.outer {
                outer.registered(name, recursive)
            } else {
                false
            }
        } else {
            false
        }
    }

    pub(crate) fn declare_var(
        &mut self,
        sig: &ast::VarSignature,
        opt_t: Option<Type>,
        id: Option<DefId>,
    ) -> TyCheckResult<()> {
        self.declare_var_pat(sig, opt_t, id)
    }

    fn declare_var_pat(
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
                    let sig_t = self.instantiate_var_sig_t(sig, opt_t, PreRegister)?;
                    self.decls
                        .insert(ident.name.clone(), VarInfo::new(sig_t, muty, vis, kind));
                    Ok(())
                }
            }
            ast::VarPattern::Array(a) => {
                if let Some(opt_ts) = opt_t.and_then(|t| t.non_default_params().cloned()) {
                    for (elem, p) in a.iter().zip(opt_ts.into_iter()) {
                        self.declare_var_pat(elem, Some(p.ty), None)?;
                    }
                } else {
                    for elem in a.iter() {
                        self.declare_var_pat(elem, None, None)?;
                    }
                }
                Ok(())
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

    pub(crate) fn assign_var(
        &mut self,
        sig: &ast::VarSignature,
        id: DefId,
        body_t: &Type,
    ) -> TyCheckResult<()> {
        self.assign_var_sig(sig, body_t, id)
    }

    fn assign_var_sig(
        &mut self,
        sig: &ast::VarSignature,
        body_t: &Type,
        id: DefId,
    ) -> TyCheckResult<()> {
        self.validate_var_sig_t(sig, body_t, Normal)?;
        let muty = Mutability::from(&sig.inspect().unwrap()[..]);
        let generalized = self.generalize_t(body_t.clone());
        match &sig.pat {
            ast::VarPattern::Discard(_token) => Ok(()),
            ast::VarPattern::Ident(ident) => {
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
                    self.params.push((Some(ident.name.clone()), vi));
                    Ok(())
                }
            }
            ast::VarPattern::Array(arr) => {
                for (elem, inf) in arr.iter().zip(generalized.inner_ts().iter()) {
                    let id = DefId(get_hash(&(&self.name, elem)));
                    self.assign_var_sig(elem, inf, id)?;
                }
                Ok(())
            }
            ast::VarPattern::Tuple(_) => todo!(),
            ast::VarPattern::Record { .. } => todo!(),
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
            ast::ParamPattern::Array(arr) => {
                let mut array_nth = 0;
                let array_outer = if let Some(outer) = outer {
                    ParamIdx::nested(outer, nth)
                } else {
                    ParamIdx::Nth(nth)
                };
                if let Some(decl_t) = opt_decl_t {
                    for (elem, p) in arr
                        .elems
                        .non_defaults
                        .iter()
                        .zip(decl_t.ty.non_default_params().unwrap())
                    {
                        self.assign_param(elem, Some(array_outer.clone()), array_nth, Some(p))?;
                        array_nth += 1;
                    }
                    for (elem, p) in arr
                        .elems
                        .defaults
                        .iter()
                        .zip(decl_t.ty.default_params().unwrap())
                    {
                        self.assign_param(elem, Some(array_outer.clone()), array_nth, Some(p))?;
                        array_nth += 1;
                    }
                } else {
                    for elem in arr.elems.non_defaults.iter() {
                        self.assign_param(elem, Some(array_outer.clone()), array_nth, None)?;
                        array_nth += 1;
                    }
                    for elem in arr.elems.defaults.iter() {
                        self.assign_param(elem, Some(array_outer.clone()), array_nth, None)?;
                        array_nth += 1;
                    }
                }
                Ok(())
            }
            ast::ParamPattern::Lit(_) => Ok(()),
            _ => todo!(),
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
                .chain(
                    params
                        .defaults
                        .iter()
                        .zip(decl_subr_t.default_params.iter()),
                )
                .enumerate()
            {
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
        let default_params = t.default_params().unwrap();
        if let Some(spec_ret_t) = t.return_t() {
            self.sub_unify(body_t, spec_ret_t, None, Some(sig.loc()))
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
                Type::proc(
                    non_default_params.clone(),
                    default_params.clone(),
                    body_t.clone(),
                )
            } else {
                Type::func(
                    non_default_params.clone(),
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
            log!("Registered {}::{name}: {}", self.name, &vi.t);
            self.params.push((Some(name.clone()), vi));
            Ok(())
        }
    }

    // 再帰サブルーチン/型の推論を可能にするため、予め登録しておく
    pub(crate) fn preregister(&mut self, block: &[ast::Expr]) -> TyCheckResult<()> {
        for expr in block.iter() {
            if let ast::Expr::Def(def) = expr {
                let id = Some(def.body.id);
                let eval_body_t = || {
                    self.eval
                        .eval_const_block(&def.body.block, self)
                        .map(|c| Type::enum_t(set![c]))
                };
                match &def.sig {
                    ast::Signature::Subr(sig) => {
                        let opt_ret_t = if let Some(spec) = sig.return_t_spec.as_ref() {
                            Some(self.instantiate_typespec(spec, PreRegister)?)
                        } else {
                            eval_body_t()
                        };
                        self.declare_sub(sig, opt_ret_t, id)?;
                    }
                    ast::Signature::Var(sig) if sig.is_const() => {
                        let t = if let Some(spec) = sig.t_spec.as_ref() {
                            Some(self.instantiate_typespec(spec, PreRegister)?)
                        } else {
                            eval_body_t()
                        };
                        self.declare_var(sig, t, id)?;
                    }
                    _ => {}
                }
            }
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
                if self.rec_subtype_of(&lit.data.class(), &Str) {
                    let name = enum_unwrap!(lit.data.clone(), ValueObj::Str);
                    match &name[..] {
                        "math" => {
                            self.mods.insert(var_name.clone(), Self::init_py_math_mod());
                        }
                        "random" => {
                            self.mods
                                .insert(var_name.clone(), Self::init_py_random_mod());
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
