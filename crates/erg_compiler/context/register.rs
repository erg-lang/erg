use std::fmt;
use std::io::BufRead;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use erg_common::env::erg_pystd_path;
use erg_common::erg_util::BUILTIN_ERG_MODS;
use erg_common::levenshtein::get_similar_name;
use erg_common::python_util::BUILTIN_PYTHON_MODS;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::triple::Triple;
use erg_common::Str;
use erg_common::{enum_unwrap, get_hash, log, set};

use ast::{ConstIdentifier, Decorator, DefId, Identifier, OperationKind, SimpleTypeSpec, VarName};
use erg_parser::ast::{self, PreDeclTypeSpec};

use crate::ty::constructors::{
    free_var, func, func0, func1, proc, ref_, ref_mut, unknown_len_array_t, v_enum,
};
use crate::ty::free::{Constraint, FreeKind, HasLevel};
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{HasType, ParamTy, SubrType, Type, Visibility};

use crate::build_hir::HIRBuilder;
use crate::context::{
    ClassDefType, Context, ContextKind, DefaultInfo, MethodInfo, RegistrationMode, TraitImpl,
};
use crate::error::readable_name;
use crate::error::{
    CompileError, CompileErrors, CompileResult, TyCheckError, TyCheckErrors, TyCheckResult,
};
use crate::hir::Literal;
use crate::varinfo::{AbsLocation, Mutability, VarInfo, VarKind};
use crate::{feature_error, hir};
use Mutability::*;
use RegistrationMode::*;

use super::instantiate::TyVarCache;
use super::instantiate_spec::ParamKind;

pub fn valid_mod_name(name: &str) -> bool {
    !name.is_empty() && name.trim() == name
}

/// format:
/// ```python
/// #[pylyzer] succeed foo.py 1234567890
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PylyzerStatus {
    pub succeed: bool,
    pub file: PathBuf,
    pub timestamp: SystemTime,
}

impl fmt::Display for PylyzerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "##[pylyzer] {} {} {}",
            if self.succeed { "succeed" } else { "failed" },
            self.file.display(),
            self.timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        )
    }
}

impl std::str::FromStr for PylyzerStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split_whitespace();
        let pylyzer = iter.next().ok_or("no pylyzer")?;
        if pylyzer != "##[pylyzer]" {
            return Err("not pylyzer".to_string());
        }
        let succeed = iter.next().ok_or("no succeed")?;
        let succeed = succeed == "succeed";
        let file = iter.next().ok_or("no file")?;
        let file = PathBuf::from(file);
        let timestamp = iter.next().ok_or("no timestamp")?;
        let timestamp = SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(
                timestamp
                    .parse()
                    .map_err(|e| format!("timestamp parse error: {e}"))?,
            ))
            .ok_or("timestamp overflow")?;
        Ok(PylyzerStatus {
            succeed,
            file,
            timestamp,
        })
    }
}

const UBAR: &Str = &Str::ever("_");

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

    fn pre_define_var(&mut self, sig: &ast::VarSignature, id: Option<DefId>) -> TyCheckResult<()> {
        let muty = Mutability::from(&sig.inspect().unwrap_or(UBAR)[..]);
        let ident = match &sig.pat {
            ast::VarPattern::Ident(ident) => ident,
            ast::VarPattern::Discard(_) => {
                return Ok(());
            }
            other => unreachable!("{other}"),
        };
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        let kind = id.map_or(VarKind::Declared, VarKind::Defined);
        let sig_t = self.instantiate_var_sig_t(sig.t_spec.as_ref(), PreRegister)?;
        let py_name = if let ContextKind::PatchMethodDefs(_base) = &self.kind {
            Some(Str::from(format!("::{}{}", self.name, ident)))
        } else {
            None
        };
        if let Some(_decl) = self.decls.remove(&ident.name) {
            Err(TyCheckErrors::from(TyCheckError::duplicate_decl_error(
                self.cfg.input.clone(),
                line!() as usize,
                sig.loc(),
                self.caused_by(),
                ident.name.inspect(),
            )))
        } else {
            let vi = VarInfo::new(
                sig_t,
                muty,
                Visibility::new(vis, self.name.clone()),
                kind,
                None,
                self.impl_of(),
                py_name,
                self.absolutize(ident.name.loc()),
            );
            self.index().register(&vi);
            self.future_defined_locals.insert(ident.name.clone(), vi);
            Ok(())
        }
    }

    pub(crate) fn declare_sub(
        &mut self,
        sig: &ast::SubrSignature,
        id: Option<DefId>,
    ) -> TyCheckResult<()> {
        let name = sig.ident.inspect();
        let vis = self.instantiate_vis_modifier(&sig.ident.vis)?;
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
        let default_ts =
            vec![free_var(self.level, Constraint::new_type_of(Type::Type)); sig.params.len()];
        let (errs, t) = match self.instantiate_sub_sig_t(sig, default_ts, PreRegister) {
            Ok(t) => (TyCheckErrors::empty(), t),
            Err((errs, t)) => (errs, t),
        };
        let py_name = if let ContextKind::PatchMethodDefs(_base) = &self.kind {
            Some(Str::from(format!("::{}{}", self.name, sig.ident)))
        } else {
            None
        };
        let vi = VarInfo::new(
            t,
            muty,
            Visibility::new(vis, self.name.clone()),
            kind,
            Some(comptime_decos),
            self.impl_of(),
            py_name,
            self.absolutize(sig.ident.name.loc()),
        );
        self.index().register(&vi);
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
            if errs.is_empty() {
                Ok(())
            } else {
                Err(errs)
            }
        }
    }

    /// already validated
    pub(crate) fn assign_var_sig(
        &mut self,
        sig: &ast::VarSignature,
        body_t: &Type,
        id: DefId,
        py_name: Option<Str>,
    ) -> TyCheckResult<VarInfo> {
        let ident = match &sig.pat {
            ast::VarPattern::Ident(ident) => ident,
            ast::VarPattern::Discard(_) => {
                return Ok(VarInfo {
                    t: body_t.clone(),
                    ..VarInfo::const_default_private()
                });
            }
            _ => unreachable!(),
        };
        if let Some(py_name) = &py_name {
            self.erg_to_py_names
                .insert(ident.inspect().clone(), py_name.clone());
        }
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        // already defined as const
        if sig.is_const() {
            let vi = self.decls.remove(ident.inspect()).unwrap_or_else(|| {
                VarInfo::new(
                    body_t.clone(),
                    Mutability::Const,
                    Visibility::new(vis, self.name.clone()),
                    VarKind::Declared,
                    None,
                    self.impl_of(),
                    py_name,
                    self.absolutize(ident.name.loc()),
                )
            });
            self.locals.insert(ident.name.clone(), vi.clone());
            return Ok(vi);
        }
        let muty = Mutability::from(&ident.inspect()[..]);
        let py_name = if let Some(vi) = self
            .decls
            .remove(ident.inspect())
            .or_else(|| self.future_defined_locals.remove(ident.inspect()))
        {
            vi.py_name
        } else {
            py_name
        };
        let kind = if id.0 == 0 {
            VarKind::Declared
        } else {
            VarKind::Defined(id)
        };
        let vi = VarInfo::new(
            body_t.clone(),
            muty,
            Visibility::new(vis, self.name.clone()),
            kind,
            None,
            self.impl_of(),
            py_name,
            self.absolutize(ident.name.loc()),
        );
        log!(info "Registered {}{}: {}", self.name, ident, vi);
        self.locals.insert(ident.name.clone(), vi.clone());
        Ok(vi)
    }

    /// TODO: sig should be immutable
    /// 宣言が既にある場合、opt_decl_tに宣言の型を渡す
    fn assign_param(
        &mut self,
        sig: &mut hir::NonDefaultParamSignature,
        opt_decl_t: Option<&ParamTy>,
        kind: ParamKind,
    ) -> TyCheckResult<()> {
        let vis = if cfg!(feature = "py_compatible") {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::private(self.name.clone())
        };
        let default = kind.default_info();
        let is_var_params = kind.is_var_params();
        match &sig.raw.pat {
            // Literal patterns will be desugared to discard patterns
            ast::ParamPattern::Lit(_) => unreachable!(),
            ast::ParamPattern::Discard(token) => {
                let (spec_t, errs) = match self.instantiate_param_sig_t(
                    &sig.raw,
                    opt_decl_t,
                    &mut TyVarCache::new(self.level, self),
                    Normal,
                    kind,
                ) {
                    Ok(ty) => (ty, TyCheckErrors::empty()),
                    Err(errs) => (Type::Failure, errs),
                };
                let def_id = DefId(get_hash(&(&self.name, "_")));
                let kind = VarKind::parameter(def_id, is_var_params, DefaultInfo::NonDefault);
                let vi = VarInfo::new(
                    spec_t,
                    Immutable,
                    vis,
                    kind,
                    None,
                    None,
                    None,
                    self.absolutize(token.loc()),
                );
                sig.vi = vi.clone();
                self.params.push((Some(VarName::from_static("_")), vi));
                if errs.is_empty() {
                    Ok(())
                } else {
                    Err(errs)
                }
            }
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
                    let mut dummy_tv_cache = TyVarCache::new(self.level, self);
                    let (spec_t, mut errs) = match self.instantiate_param_sig_t(
                        &sig.raw,
                        opt_decl_t,
                        &mut dummy_tv_cache,
                        Normal,
                        kind,
                    ) {
                        Ok(ty) => (ty, TyCheckErrors::empty()),
                        Err(errs) => (Type::Failure, errs),
                    };
                    let spec_t = if is_var_params {
                        unknown_len_array_t(spec_t)
                    } else {
                        spec_t
                    };
                    if &name.inspect()[..] == "self" {
                        if let Some(self_t) = self.rec_get_self_t() {
                            if let Err(es) =
                                self.sub_unify(&spec_t, &self_t, name, Some(name.inspect()))
                            {
                                errs.extend(es);
                            }
                        } else {
                            log!(err "self_t is None");
                        }
                    }
                    let def_id = DefId(get_hash(&(&self.name, name)));
                    let kind = VarKind::parameter(def_id, is_var_params, default);
                    let muty = Mutability::from(&name.inspect()[..]);
                    let vi = VarInfo::new(
                        spec_t,
                        muty,
                        vis,
                        kind,
                        None,
                        None,
                        None,
                        self.absolutize(name.loc()),
                    );
                    self.index().register(&vi);
                    sig.vi = vi.clone();
                    self.params.push((Some(name.clone()), vi));
                    if errs.is_empty() {
                        Ok(())
                    } else {
                        Err(errs)
                    }
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
                    let mut dummy_tv_cache = TyVarCache::new(self.level, self);
                    let (spec_t, mut errs) = match self.instantiate_param_sig_t(
                        &sig.raw,
                        opt_decl_t,
                        &mut dummy_tv_cache,
                        Normal,
                        kind,
                    ) {
                        Ok(ty) => (ty, TyCheckErrors::empty()),
                        Err(errs) => (Type::Failure, errs),
                    };
                    if &name.inspect()[..] == "self" {
                        if let Some(self_t) = self.rec_get_self_t() {
                            if let Err(es) =
                                self.sub_unify(&spec_t, &ref_(self_t), name, Some(name.inspect()))
                            {
                                errs.extend(es);
                            }
                        } else {
                            log!(err "self_t is None");
                        }
                    }
                    let kind = VarKind::parameter(
                        DefId(get_hash(&(&self.name, name))),
                        is_var_params,
                        default,
                    );
                    let vi = VarInfo::new(
                        spec_t,
                        Immutable,
                        vis,
                        kind,
                        None,
                        None,
                        None,
                        self.absolutize(name.loc()),
                    );
                    sig.vi = vi.clone();
                    self.params.push((Some(name.clone()), vi));
                    if errs.is_empty() {
                        Ok(())
                    } else {
                        Err(errs)
                    }
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
                    let mut dummy_tv_cache = TyVarCache::new(self.level, self);
                    let (spec_t, mut errs) = match self.instantiate_param_sig_t(
                        &sig.raw,
                        opt_decl_t,
                        &mut dummy_tv_cache,
                        Normal,
                        kind,
                    ) {
                        Ok(ty) => (ty, TyCheckErrors::empty()),
                        Err(errs) => (Type::Failure, errs),
                    };
                    if &name.inspect()[..] == "self" {
                        if let Some(self_t) = self.rec_get_self_t() {
                            if let Err(es) = self.sub_unify(
                                &spec_t,
                                &ref_mut(self_t, None),
                                name,
                                Some(name.inspect()),
                            ) {
                                errs.extend(es);
                            }
                        } else {
                            log!(err "self_t is None");
                        }
                    }
                    let kind = VarKind::parameter(
                        DefId(get_hash(&(&self.name, name))),
                        is_var_params,
                        default,
                    );
                    let vi = VarInfo::new(
                        spec_t,
                        Immutable,
                        vis,
                        kind,
                        None,
                        None,
                        None,
                        self.absolutize(name.loc()),
                    );
                    sig.vi = vi.clone();
                    self.params.push((Some(name.clone()), vi));
                    if errs.is_empty() {
                        Ok(())
                    } else {
                        Err(errs)
                    }
                }
            }
            other => {
                log!(err "{other}");
                unreachable!("{other}")
            }
        }
    }

    pub(crate) fn assign_params(
        &mut self,
        params: &mut hir::Params,
        opt_decl_subr_t: Option<SubrType>,
    ) -> TyCheckResult<()> {
        let mut errs = TyCheckErrors::empty();
        if let Some(decl_subr_t) = opt_decl_subr_t {
            debug_assert_eq!(
                params.non_defaults.len(),
                decl_subr_t.non_default_params.len()
            );
            debug_assert_eq!(params.defaults.len(), decl_subr_t.default_params.len());
            for (non_default, pt) in params
                .non_defaults
                .iter_mut()
                .zip(decl_subr_t.non_default_params.iter())
            {
                if let Err(es) = self.assign_param(non_default, Some(pt), ParamKind::NonDefault) {
                    errs.extend(es);
                }
            }
            if let Some(var_params) = &mut params.var_params {
                if let Some(pt) = &decl_subr_t.var_params {
                    let pt = pt.clone().map_type(unknown_len_array_t);
                    if let Err(es) = self.assign_param(var_params, Some(&pt), ParamKind::VarParams)
                    {
                        errs.extend(es);
                    }
                } else if let Err(es) = self.assign_param(var_params, None, ParamKind::VarParams) {
                    errs.extend(es);
                }
            }
            for (default, pt) in params
                .defaults
                .iter_mut()
                .zip(decl_subr_t.default_params.iter())
            {
                if let Err(es) = self.assign_param(
                    &mut default.sig,
                    Some(pt),
                    ParamKind::Default(default.default_val.t()),
                ) {
                    errs.extend(es);
                }
            }
        } else {
            for non_default in params.non_defaults.iter_mut() {
                if let Err(es) = self.assign_param(non_default, None, ParamKind::NonDefault) {
                    errs.extend(es);
                }
            }
            if let Some(var_params) = &mut params.var_params {
                if let Err(es) = self.assign_param(var_params, None, ParamKind::VarParams) {
                    errs.extend(es);
                }
            }
            for default in params.defaults.iter_mut() {
                if let Err(es) = self.assign_param(
                    &mut default.sig,
                    None,
                    ParamKind::Default(default.default_val.t()),
                ) {
                    errs.extend(es);
                }
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }

    /// ## Errors
    /// * TypeError: if `return_t` != typeof `body`
    /// * AssignError: if `name` has already been registered
    pub(crate) fn assign_subr(
        &mut self,
        sig: &ast::SubrSignature,
        id: DefId,
        body_t: &Type,
    ) -> TyCheckResult<VarInfo> {
        // already defined as const
        if sig.ident.is_const() {
            let vi = self.decls.remove(sig.ident.inspect()).unwrap();
            self.locals.insert(sig.ident.name.clone(), vi.clone());
            return Ok(vi);
        }
        let vis = self.instantiate_vis_modifier(&sig.ident.vis)?;
        let muty = if sig.ident.is_const() {
            Mutability::Const
        } else {
            Mutability::Immutable
        };
        let name = &sig.ident.name;
        // FIXME: constでない関数
        let t = self.get_current_scope_var(name).map(|vi| &vi.t).unwrap();
        let non_default_params = t.non_default_params().unwrap();
        let var_args = t.var_params();
        let default_params = t.default_params().unwrap();
        let mut errs = if let Some(spec_ret_t) = t.return_t() {
            let return_t_loc = sig.return_t_spec.as_ref().ok_or(sig);
            self.sub_unify(body_t, spec_ret_t, &return_t_loc, None)
                .map_err(|errs| {
                    TyCheckErrors::new(
                        errs.into_iter()
                            .map(|e| {
                                TyCheckError::return_type_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    e.core.get_loc_with_fallback(),
                                    e.caused_by,
                                    readable_name(name.inspect()),
                                    spec_ret_t,
                                    body_t,
                                )
                            })
                            .collect(),
                    )
                })
        } else {
            Ok(())
        };
        let return_t = if errs.is_err() {
            Type::Failure
        } else {
            // NOTE: not `body_t.clone()` because the body may contain `return`
            t.return_t().unwrap().clone()
        };
        let sub_t = if sig.ident.is_procedural() {
            proc(
                non_default_params.clone(),
                var_args.cloned(),
                default_params.clone(),
                return_t,
            )
        } else {
            func(
                non_default_params.clone(),
                var_args.cloned(),
                default_params.clone(),
                return_t,
            )
        };
        sub_t.lift();
        let found_t = self.generalize_t(sub_t);
        // let found_t = self.eliminate_needless_quant(found_t, crate::context::Variance::Covariant, sig)?;
        let py_name = if let Some(vi) = self.decls.remove(name) {
            if !self.supertype_of(&vi.t, &found_t) {
                let err = TyCheckError::violate_decl_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    sig.ident.loc(),
                    self.caused_by(),
                    name.inspect(),
                    &vi.t,
                    &found_t,
                );
                match errs {
                    Ok(()) => {
                        errs = Err(TyCheckErrors::from(err));
                    }
                    Err(ref mut es) => {
                        es.push(err);
                    }
                }
            }
            vi.py_name
        } else {
            None
        };
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
            Visibility::new(vis, self.name.clone()),
            VarKind::Defined(id),
            Some(comptime_decos),
            self.impl_of(),
            py_name,
            self.absolutize(name.loc()),
        );
        let vis = if vi.vis.is_private() { "::" } else { "." };
        log!(info "Registered {}{}{name}: {}", self.name, vis, &vi.t);
        self.locals.insert(name.clone(), vi.clone());
        errs?;
        Ok(vi)
    }

    pub(crate) fn fake_subr_assign(
        &mut self,
        ident: &Identifier,
        decorators: &Set<Decorator>,
        failure_t: Type,
    ) -> TyCheckResult<()> {
        // already defined as const
        if ident.is_const() {
            if let Some(vi) = self.decls.remove(ident.inspect()) {
                self.locals.insert(ident.name.clone(), vi);
            } else {
                log!(err "not found: {}", ident.name);
                return Ok(());
            }
        }
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        let muty = if ident.is_const() {
            Mutability::Const
        } else {
            Mutability::Immutable
        };
        let name = &ident.name;
        self.decls.remove(name);
        let comptime_decos = decorators
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
            Visibility::new(vis, self.name.clone()),
            VarKind::DoesNotExist,
            Some(comptime_decos),
            self.impl_of(),
            None,
            self.absolutize(name.loc()),
        );
        log!(info "Registered {}::{name}: {}", self.name, &vi.t);
        self.locals.insert(name.clone(), vi);
        Ok(())
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
                    if def.def_kind().is_import() {
                        self.pre_import(def);
                    }
                }
                ast::Expr::ClassDef(class_def) => {
                    if let Err(errs) = self.preregister_def(&class_def.def) {
                        total_errs.extend(errs.into_iter());
                    }
                }
                ast::Expr::PatchDef(patch_def) => {
                    if let Err(errs) = self.preregister_def(&patch_def.def) {
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

    /// HACK: The constant expression evaluator can evaluate attributes when the type of the receiver is known.
    /// import/pyimport is not a constant function, but specially assumes that the type of the module is known in the eval phase.
    fn pre_import(&mut self, def: &ast::Def) {
        let Some(ast::Expr::Call(call)) = def.body.block.first() else { unreachable!() };
        let Some(ast::Expr::Literal(mod_name)) = call.args.get_left_or_key("Path") else {
            return;
        };
        let Ok(mod_name) = hir::Literal::try_from(mod_name.token.clone()) else {
            return;
        };
        let _ = self.import_mod(call.additional_operation().unwrap(), &mod_name);
        let arg = TyParam::Value(ValueObj::Str(
            mod_name.token.content.replace('\"', "").into(),
        ));
        let typ = if def.def_kind().is_erg_import() {
            Type::Poly {
                name: Str::ever("Module"),
                params: vec![arg],
            }
        } else {
            Type::Poly {
                name: Str::ever("PyModule"),
                params: vec![arg],
            }
        };
        let Some(ident) = def.sig.ident() else { return  };
        let Some((_, vi)) = self.get_var_info(ident.inspect()) else {
            return;
        };
        if let Type::FreeVar(fv) = &vi.t {
            fv.link(&typ);
        }
    }

    pub(crate) fn preregister_def(&mut self, def: &ast::Def) -> TyCheckResult<()> {
        let id = Some(def.body.id);
        let __name__ = def.sig.ident().map(|i| i.inspect()).unwrap_or(UBAR);
        match &def.sig {
            ast::Signature::Subr(sig) => {
                if sig.is_const() {
                    let tv_cache = self.instantiate_ty_bounds(&sig.bounds, PreRegister)?;
                    let vis = self.instantiate_vis_modifier(sig.vis())?;
                    self.grow(__name__, ContextKind::Proc, vis, Some(tv_cache));
                    let (obj, const_t) = match self.eval_const_block(&def.body.block) {
                        Ok(obj) => (obj.clone(), v_enum(set! {obj})),
                        Err(errs) => {
                            self.pop();
                            return Err(errs);
                        }
                    };
                    if let Some(spec) = sig.return_t_spec.as_ref() {
                        let mut dummy_tv_cache = TyVarCache::new(self.level, self);
                        let spec_t = self
                            .instantiate_typespec(
                                spec,
                                None,
                                &mut dummy_tv_cache,
                                PreRegister,
                                false,
                            )
                            .map_err(|errs| {
                                self.pop();
                                errs
                            })?;
                        self.sub_unify(&const_t, &spec_t, &def.body, None)
                            .map_err(|errs| {
                                self.pop();
                                errs
                            })?;
                    }
                    self.pop();
                    self.register_gen_const(
                        def.sig.ident().unwrap(),
                        obj,
                        def.def_kind().is_other(),
                    )?;
                } else {
                    self.declare_sub(sig, id)?;
                }
            }
            ast::Signature::Var(sig) => {
                if sig.is_const() {
                    let kind = ContextKind::from(def.def_kind());
                    let vis = self.instantiate_vis_modifier(sig.vis())?;
                    self.grow(__name__, kind, vis, None);
                    let (obj, const_t) = match self.eval_const_block(&def.body.block) {
                        Ok(obj) => (obj.clone(), v_enum(set! {obj})),
                        Err(errs) => {
                            self.pop();
                            return Err(errs);
                        }
                    };
                    if let Some(spec) = sig.t_spec.as_ref() {
                        let mut dummy_tv_cache = TyVarCache::new(self.level, self);
                        let spec_t = self
                            .instantiate_typespec(
                                spec,
                                None,
                                &mut dummy_tv_cache,
                                PreRegister,
                                false,
                            )
                            .map_err(|errs| {
                                self.pop();
                                errs
                            })?;
                        self.sub_unify(&const_t, &spec_t, &def.body, None)
                            .map_err(|errs| {
                                self.pop();
                                errs
                            })?;
                    }
                    self.pop();
                    if let Some(ident) = sig.ident() {
                        self.register_gen_const(ident, obj, def.def_kind().is_other())?;
                    }
                } else {
                    self.pre_define_var(sig, id)?;
                }
            }
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
        py_name: Option<Str>,
    ) -> CompileResult<()> {
        let name = VarName::from_static(name);
        if self.locals.get(&name).is_some() {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                self.caused_by(),
                name.inspect(),
            )))
        } else {
            let vi = VarInfo::new(
                t,
                muty,
                vis,
                VarKind::Auto,
                None,
                self.impl_of(),
                py_name,
                AbsLocation::unknown(),
            );
            self.locals.insert(name, vi);
            Ok(())
        }
    }

    /// e.g. ::__new__
    fn register_fixed_auto_impl(
        &mut self,
        name: &'static str,
        t: Type,
        muty: Mutability,
        vis: Visibility,
        py_name: Option<Str>,
    ) -> CompileResult<()> {
        let name = VarName::from_static(name);
        if self.locals.get(&name).is_some() {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                self.caused_by(),
                name.inspect(),
            )))
        } else {
            self.locals.insert(
                name,
                VarInfo::new(
                    t,
                    muty,
                    vis,
                    VarKind::FixedAuto,
                    None,
                    self.impl_of(),
                    py_name,
                    AbsLocation::unknown(),
                ),
            );
            Ok(())
        }
    }

    fn _register_gen_decl(
        &mut self,
        name: VarName,
        t: Type,
        vis: Visibility,
        impl_of: Option<Type>,
        py_name: Option<Str>,
    ) -> CompileResult<()> {
        if self.decls.get(&name).is_some() {
            Err(CompileErrors::from(CompileError::duplicate_decl_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                self.caused_by(),
                name.inspect(),
            )))
        } else {
            let vi = VarInfo::new(
                t,
                Immutable,
                vis,
                VarKind::Declared,
                None,
                impl_of,
                py_name,
                self.absolutize(name.loc()),
            );
            self.decls.insert(name, vi);
            Ok(())
        }
    }

    fn _register_gen_impl(
        &mut self,
        name: VarName,
        t: Type,
        muty: Mutability,
        vis: Visibility,
        impl_of: Option<Type>,
        py_name: Option<Str>,
    ) -> CompileResult<()> {
        if self.locals.get(&name).is_some() {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                self.caused_by(),
                name.inspect(),
            )))
        } else {
            let id = DefId(get_hash(&(&self.name, &name)));
            let vi = VarInfo::new(
                t,
                muty,
                vis,
                VarKind::Defined(id),
                None,
                impl_of,
                py_name,
                self.absolutize(name.loc()),
            );
            self.locals.insert(name, vi);
            Ok(())
        }
    }

    pub(crate) fn register_trait(&mut self, class: Type, methods: Self) {
        let trait_ = if let ContextKind::MethodDefs(Some(tr)) = &methods.kind {
            tr.clone()
        } else {
            unreachable!()
        };
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
        alias: bool,
    ) -> CompileResult<()> {
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        if self.rec_get_const_obj(ident.inspect()).is_some() && vis.is_private() {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            )))
        } else {
            match obj {
                ValueObj::Type(t) => match t {
                    TypeObj::Generated(gen) if alias => {
                        self.register_type_alias(ident, gen.into_typ())
                    }
                    TypeObj::Generated(gen) => self.register_gen_type(ident, gen),
                    TypeObj::Builtin(t) => self.register_type_alias(ident, t),
                },
                // TODO: not all value objects are comparable
                other => {
                    let id = DefId(get_hash(ident));
                    let vi = VarInfo::new(
                        v_enum(set! {other.clone()}),
                        Const,
                        Visibility::new(vis, self.name.clone()),
                        VarKind::Defined(id),
                        None,
                        self.impl_of(),
                        None,
                        self.absolutize(ident.name.loc()),
                    );
                    self.index().register(&vi);
                    self.decls.insert(ident.name.clone(), vi);
                    self.consts.insert(ident.name.clone(), other);
                    Ok(())
                }
            }
        }
    }

    pub(crate) fn register_gen_type(
        &mut self,
        ident: &Identifier,
        gen: GenTypeObj,
    ) -> CompileResult<()> {
        match gen {
            GenTypeObj::Class(_) => {
                if gen.typ().is_monomorphic() {
                    // let super_traits = gen.impls.iter().map(|to| to.typ().clone()).collect();
                    let mut ctx = Self::mono_class(
                        gen.typ().qual_name(),
                        self.cfg.clone(),
                        self.shared.clone(),
                        2,
                        self.level,
                    );
                    let mut methods =
                        Self::methods(None, self.cfg.clone(), self.shared.clone(), 2, self.level);
                    let new_t = if let Some(base) = gen.base_or_sup() {
                        match base {
                            TypeObj::Builtin(Type::Record(rec)) => {
                                for (field, t) in rec.iter() {
                                    let varname = VarName::from_str(field.symbol.clone());
                                    let vi = VarInfo::instance_attr(
                                        field.clone(),
                                        t.clone(),
                                        self.impl_of(),
                                        ctx.name.clone(),
                                    );
                                    ctx.decls.insert(varname, vi);
                                }
                            }
                            other => {
                                methods.register_fixed_auto_impl(
                                    "base",
                                    other.typ().clone(),
                                    Immutable,
                                    Visibility::BUILTIN_PRIVATE,
                                    None,
                                )?;
                            }
                        }
                        func1(base.typ().clone(), gen.typ().clone())
                    } else {
                        func0(gen.typ().clone())
                    };
                    methods.register_fixed_auto_impl(
                        "__new__",
                        new_t.clone(),
                        Immutable,
                        Visibility::BUILTIN_PRIVATE,
                        Some("__call__".into()),
                    )?;
                    // 必要なら、ユーザーが独自に上書きする
                    // users can override this if necessary
                    methods.register_auto_impl(
                        "new",
                        new_t,
                        Immutable,
                        Visibility::BUILTIN_PUBLIC,
                        None,
                    )?;
                    ctx.methods_list
                        .push((ClassDefType::Simple(gen.typ().clone()), methods));
                    self.register_gen_mono_type(ident, gen, ctx, Const)
                } else {
                    feature_error!(
                        CompileErrors,
                        CompileError,
                        self,
                        ident.loc(),
                        "polymorphic class definition"
                    )
                }
            }
            GenTypeObj::Subclass(_) => {
                if gen.typ().is_monomorphic() {
                    let super_classes = vec![gen.base_or_sup().unwrap().typ().clone()];
                    // let super_traits = gen.impls.iter().map(|to| to.typ().clone()).collect();
                    let mut ctx = Self::mono_class(
                        gen.typ().qual_name(),
                        self.cfg.clone(),
                        self.shared.clone(),
                        2,
                        self.level,
                    );
                    for sup in super_classes.into_iter() {
                        let (_, sup_ctx) = self
                            .get_nominal_type_ctx(&sup)
                            .unwrap_or_else(|| todo!("{sup} not found"));
                        ctx.register_superclass(sup, sup_ctx);
                    }
                    let mut methods =
                        Self::methods(None, self.cfg.clone(), self.shared.clone(), 2, self.level);
                    if let Some(sup) =
                        self.rec_get_const_obj(&gen.base_or_sup().unwrap().typ().local_name())
                    {
                        let ValueObj::Type(sup) = sup else {
                            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                ident.loc(),
                                self.caused_by(),
                                "",
                                Some(1),
                                &Type::Type,
                                &sup.class(),
                                None,
                                None
                            )));
                        };
                        let param_t = match sup {
                            TypeObj::Builtin(t) => t,
                            TypeObj::Generated(t) => {
                                if let Some(t) = t.base_or_sup() {
                                    t.typ()
                                } else {
                                    return Err(TyCheckErrors::from(TyCheckError::param_error(
                                        self.cfg.input.clone(),
                                        line!() as usize,
                                        ident.loc(),
                                        self.caused_by(),
                                        1,
                                        0,
                                    )));
                                }
                            }
                        };
                        // `Super.Requirement := {x = Int}` and `Self.Additional := {y = Int}`
                        // => `Self.Requirement := {x = Int; y = Int}`
                        let param_t = if let Some(additional) = gen.additional() {
                            if let TypeObj::Builtin(Type::Record(rec)) = additional {
                                for (field, t) in rec.iter() {
                                    let varname = VarName::from_str(field.symbol.clone());
                                    let vi = VarInfo::instance_attr(
                                        field.clone(),
                                        t.clone(),
                                        self.impl_of(),
                                        ctx.name.clone(),
                                    );
                                    ctx.decls.insert(varname, vi);
                                }
                            }
                            self.intersection(param_t, additional.typ())
                        } else {
                            param_t.clone()
                        };
                        let new_t = func1(param_t, gen.typ().clone());
                        methods.register_fixed_auto_impl(
                            "__new__",
                            new_t.clone(),
                            Immutable,
                            Visibility::BUILTIN_PRIVATE,
                            Some("__call__".into()),
                        )?;
                        // 必要なら、ユーザーが独自に上書きする
                        methods.register_auto_impl(
                            "new",
                            new_t,
                            Immutable,
                            Visibility::BUILTIN_PUBLIC,
                            None,
                        )?;
                        ctx.methods_list
                            .push((ClassDefType::Simple(gen.typ().clone()), methods));
                        self.register_gen_mono_type(ident, gen, ctx, Const)
                    } else {
                        let class_name = gen.base_or_sup().unwrap().typ().local_name();
                        Err(CompileErrors::from(CompileError::no_type_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            ident.loc(),
                            self.caused_by(),
                            &class_name,
                            self.get_similar_name(&class_name),
                        )))
                    }
                } else {
                    feature_error!(
                        CompileErrors,
                        CompileError,
                        self,
                        ident.loc(),
                        "polymorphic class definition"
                    )
                }
            }
            GenTypeObj::Trait(_) => {
                if gen.typ().is_monomorphic() {
                    let mut ctx = Self::mono_trait(
                        gen.typ().qual_name(),
                        self.cfg.clone(),
                        self.shared.clone(),
                        2,
                        self.level,
                    );
                    let Some(TypeObj::Builtin(Type::Record(req))) = gen.base_or_sup() else { todo!("{gen}") };
                    for (field, t) in req.iter() {
                        let vi = VarInfo::instance_attr(
                            field.clone(),
                            t.clone(),
                            self.impl_of(),
                            ctx.name.clone(),
                        );
                        ctx.decls
                            .insert(VarName::from_str(field.symbol.clone()), vi);
                    }
                    self.register_gen_mono_type(ident, gen, ctx, Const)
                } else {
                    feature_error!(
                        CompileErrors,
                        CompileError,
                        self,
                        ident.loc(),
                        "polymorphic trait definition"
                    )
                }
            }
            GenTypeObj::Subtrait(_) => {
                if gen.typ().is_monomorphic() {
                    let super_classes = vec![gen.base_or_sup().unwrap().typ().clone()];
                    // let super_traits = gen.impls.iter().map(|to| to.typ().clone()).collect();
                    let mut ctx = Self::mono_trait(
                        gen.typ().qual_name(),
                        self.cfg.clone(),
                        self.shared.clone(),
                        2,
                        self.level,
                    );
                    let additional = gen.additional().map(
                        |additional| enum_unwrap!(additional, TypeObj::Builtin:(Type::Record:(_))),
                    );
                    if let Some(additional) = additional {
                        for (field, t) in additional.iter() {
                            let vi = VarInfo::instance_attr(
                                field.clone(),
                                t.clone(),
                                self.impl_of(),
                                ctx.name.clone(),
                            );
                            ctx.decls
                                .insert(VarName::from_str(field.symbol.clone()), vi);
                        }
                    }
                    for sup in super_classes.into_iter() {
                        if let Some((_, sup_ctx)) = self.get_nominal_type_ctx(&sup) {
                            ctx.register_supertrait(sup, sup_ctx);
                        } else {
                            log!(err "{sup} not found");
                        }
                    }
                    self.register_gen_mono_type(ident, gen, ctx, Const)
                } else {
                    feature_error!(
                        CompileErrors,
                        CompileError,
                        self,
                        ident.loc(),
                        "polymorphic trait definition"
                    )
                }
            }
            GenTypeObj::Patch(_) => {
                if gen.typ().is_monomorphic() {
                    let Some(TypeObj::Builtin(base)) = gen.base_or_sup() else { todo!("{gen}") };
                    let ctx = Self::mono_patch(
                        gen.typ().qual_name(),
                        base.clone(),
                        self.cfg.clone(),
                        self.shared.clone(),
                        2,
                        self.level,
                    );
                    self.register_gen_mono_patch(ident, gen, ctx, Const)
                } else {
                    feature_error!(
                        CompileErrors,
                        CompileError,
                        self,
                        ident.loc(),
                        "polymorphic patch definition"
                    )
                }
            }
            other => feature_error!(
                CompileErrors,
                CompileError,
                self,
                ident.loc(),
                &format!("{other} definition")
            ),
        }
    }

    pub(crate) fn register_type_alias(&mut self, ident: &Identifier, t: Type) -> CompileResult<()> {
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        if self.mono_types.contains_key(ident.inspect()) {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            )))
        } else if self.rec_get_const_obj(ident.inspect()).is_some() && vis.is_private() {
            // TODO: display where defined
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            )))
        } else {
            let name = &ident.name;
            let muty = Mutability::from(&ident.inspect()[..]);
            let id = DefId(get_hash(&(&self.name, &name)));
            let val = ValueObj::Type(TypeObj::Builtin(t));
            let vi = VarInfo::new(
                v_enum(set! { val.clone() }),
                muty,
                Visibility::new(vis, self.name.clone()),
                VarKind::Defined(id),
                None,
                self.impl_of(),
                None,
                self.absolutize(name.loc()),
            );
            self.index().register(&vi);
            self.decls.insert(name.clone(), vi);
            self.consts.insert(name.clone(), val);
            Ok(())
        }
    }

    fn register_gen_mono_type(
        &mut self,
        ident: &Identifier,
        gen: GenTypeObj,
        ctx: Self,
        muty: Mutability,
    ) -> CompileResult<()> {
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        // FIXME: recursive search
        if self.mono_types.contains_key(ident.inspect()) {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            )))
        } else if self.rec_get_const_obj(ident.inspect()).is_some() && vis.is_private() {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            )))
        } else {
            let t = gen.typ().clone();
            let meta_t = gen.meta_type();
            let name = &ident.name;
            let id = DefId(get_hash(&(&self.name, &name)));
            let vi = VarInfo::new(
                meta_t,
                muty,
                Visibility::new(vis, self.name.clone()),
                VarKind::Defined(id),
                None,
                self.impl_of(),
                None,
                self.absolutize(name.loc()),
            );
            self.index().register(&vi);
            self.decls.insert(name.clone(), vi);
            self.consts
                .insert(name.clone(), ValueObj::Type(TypeObj::Generated(gen)));
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls().get_mut(&impl_trait.qual_name()) {
                    impls.insert(TraitImpl::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls().register(
                        impl_trait.qual_name(),
                        set![TraitImpl::new(t.clone(), impl_trait.clone())],
                    );
                }
            }
            for (trait_method, vi) in ctx.decls.iter() {
                if let Some(types) = self.method_to_traits.get_mut(trait_method.inspect()) {
                    types.push(MethodInfo::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_traits.insert(
                        trait_method.inspect().clone(),
                        vec![MethodInfo::new(t.clone(), vi.clone())],
                    );
                }
            }
            for (class_method, vi) in ctx.locals.iter() {
                if let Some(types) = self.method_to_classes.get_mut(class_method.inspect()) {
                    types.push(MethodInfo::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_classes.insert(
                        class_method.inspect().clone(),
                        vec![MethodInfo::new(t.clone(), vi.clone())],
                    );
                }
            }
            self.mono_types.insert(name.clone(), (t, ctx));
            Ok(())
        }
    }

    fn register_gen_mono_patch(
        &mut self,
        ident: &Identifier,
        gen: GenTypeObj,
        ctx: Self,
        muty: Mutability,
    ) -> CompileResult<()> {
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        // FIXME: recursive search
        if self.patches.contains_key(ident.inspect()) {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            )))
        } else if self.rec_get_const_obj(ident.inspect()).is_some() && vis.is_private() {
            Err(CompileErrors::from(CompileError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
            )))
        } else {
            let t = gen.typ().clone();
            let meta_t = gen.meta_type();
            let name = &ident.name;
            let id = DefId(get_hash(&(&self.name, &name)));
            self.decls.insert(
                name.clone(),
                VarInfo::new(
                    meta_t,
                    muty,
                    Visibility::new(vis, self.name.clone()),
                    VarKind::Defined(id),
                    None,
                    self.impl_of(),
                    None,
                    self.absolutize(name.loc()),
                ),
            );
            self.consts
                .insert(name.clone(), ValueObj::Type(TypeObj::Generated(gen)));
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls().get_mut(&impl_trait.qual_name()) {
                    impls.insert(TraitImpl::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls().register(
                        impl_trait.qual_name(),
                        set![TraitImpl::new(t.clone(), impl_trait.clone())],
                    );
                }
            }
            for (trait_method, vi) in ctx.decls.iter() {
                if let Some(types) = self.method_to_traits.get_mut(trait_method.inspect()) {
                    types.push(MethodInfo::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_traits.insert(
                        trait_method.inspect().clone(),
                        vec![MethodInfo::new(t.clone(), vi.clone())],
                    );
                }
            }
            for (class_method, vi) in ctx.locals.iter() {
                if let Some(types) = self.method_to_classes.get_mut(class_method.inspect()) {
                    types.push(MethodInfo::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_classes.insert(
                        class_method.inspect().clone(),
                        vec![MethodInfo::new(t.clone(), vi.clone())],
                    );
                }
            }
            self.patches.insert(name.clone(), ctx);
            Ok(())
        }
    }

    pub(crate) fn import_mod(
        &mut self,
        kind: OperationKind,
        mod_name: &Literal,
    ) -> CompileResult<PathBuf> {
        let ValueObj::Str(__name__) = &mod_name.value else {
            let name = if kind.is_erg_import() { "import" } else { "pyimport" };
            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                self.cfg.input.clone(),
                line!() as usize,
                mod_name.loc(),
                self.caused_by(),
                name,
                Some(1),
                &Type::Str,
                &mod_name.t(),
                None,
                None,
            )));
        };
        if !valid_mod_name(__name__) {
            return Err(TyCheckErrors::from(TyCheckError::syntax_error(
                self.cfg.input.clone(),
                line!() as usize,
                mod_name.loc(),
                self.caused_by(),
                format!("{__name__} is not a valid module name"),
                None,
            )));
        }
        if kind.is_erg_import() {
            self.import_erg_mod(__name__, mod_name)
        } else {
            self.import_py_mod(__name__, mod_name)
        }
    }

    fn import_erg_mod(&self, __name__: &Str, loc: &impl Locational) -> CompileResult<PathBuf> {
        let mod_cache = self.mod_cache();
        let py_mod_cache = self.py_mod_cache();
        let path = match Self::resolve_real_path(&self.cfg, Path::new(&__name__[..])) {
            Some(path) => path,
            None => {
                let err = TyCheckErrors::from(TyCheckError::import_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    format!("module {__name__} not found"),
                    loc.loc(),
                    self.caused_by(),
                    self.similar_builtin_erg_mod_name(__name__)
                        .or_else(|| mod_cache.get_similar_name(__name__)),
                    self.similar_builtin_py_mod_name(__name__)
                        .or_else(|| py_mod_cache.get_similar_name(__name__)),
                ));
                return Err(err);
            }
        };
        if let Some(referrer) = self.cfg.input.path() {
            let graph = &self.shared.as_ref().unwrap().graph;
            graph.inc_ref(referrer, path.clone());
        }
        if mod_cache.get(&path).is_some() {
            return Ok(path);
        }
        let mut cfg = self.cfg.inherit(path.clone());
        let src = cfg.input.read();
        let mut builder =
            HIRBuilder::new_with_cache(cfg, __name__, self.shared.as_ref().unwrap().clone());
        match builder.build(src, "exec") {
            Ok(artifact) => {
                mod_cache.register(
                    path.clone(),
                    Some(artifact.object),
                    builder.pop_mod_ctx().unwrap(),
                );
            }
            Err(artifact) => {
                if let Some(hir) = artifact.object {
                    mod_cache.register(path, Some(hir), builder.pop_mod_ctx().unwrap());
                }
                return Err(artifact.errors);
            }
        }
        Ok(path)
    }

    fn similar_builtin_py_mod_name(&self, name: &Str) -> Option<Str> {
        get_similar_name(BUILTIN_PYTHON_MODS.into_iter(), name).map(Str::rc)
    }

    fn similar_builtin_erg_mod_name(&self, name: &Str) -> Option<Str> {
        get_similar_name(BUILTIN_ERG_MODS.into_iter(), name).map(Str::rc)
    }

    fn is_pystd_main_module(&self, path: &Path) -> bool {
        let mut path = PathBuf::from(path);
        if path.ends_with("__init__.d.er") {
            path.pop();
            path.pop();
        } else {
            path.pop();
        }
        let pystd_path = erg_pystd_path();
        path == pystd_path
    }

    /// e.g. http.d/client.d.er -> http.client
    /// math.d.er -> math
    fn mod_name(&self, path: &Path) -> Str {
        let mut name = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_end_matches(".d.er")
            .to_string();
        for parent in path.components().rev().skip(1) {
            let parent = parent.as_os_str().to_str().unwrap();
            if parent.ends_with(".d") {
                name = parent.trim_end_matches(".d").to_string() + "." + &name;
            } else {
                break;
            }
        }
        Str::from(name)
    }

    fn can_reuse(path: &Path) -> Option<PylyzerStatus> {
        let file = std::fs::File::open(path).ok()?;
        let mut line = "".to_string();
        std::io::BufReader::new(file).read_line(&mut line).ok()?;
        let status = line.parse::<PylyzerStatus>().ok()?;
        if status.timestamp < std::fs::metadata(&status.file).ok()?.modified().ok()? {
            None
        } else {
            Some(status)
        }
    }

    fn get_path(&self, __name__: &Str, loc: &impl Locational) -> CompileResult<PathBuf> {
        match Self::resolve_decl_path(&self.cfg, Path::new(&__name__[..])) {
            Some(path) => {
                if Self::can_reuse(&path).is_none() {
                    let _ = self.try_gen_py_decl_file(__name__);
                }
                if self.is_pystd_main_module(path.as_path())
                    && !BUILTIN_PYTHON_MODS.contains(&&__name__[..])
                {
                    let err = TyCheckError::module_env_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        __name__,
                        loc.loc(),
                        self.caused_by(),
                    );
                    return Err(TyCheckErrors::from(err));
                }
                Ok(path)
            }
            None => {
                if let Ok(path) = self.try_gen_py_decl_file(__name__) {
                    return Ok(path);
                }
                let err = TyCheckError::import_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    format!("module {__name__} not found"),
                    loc.loc(),
                    self.caused_by(),
                    self.similar_builtin_erg_mod_name(__name__)
                        .or_else(|| self.mod_cache().get_similar_name(__name__)),
                    self.similar_builtin_py_mod_name(__name__)
                        .or_else(|| self.py_mod_cache().get_similar_name(__name__)),
                );
                Err(TyCheckErrors::from(err))
            }
        }
    }

    fn try_gen_py_decl_file(&self, __name__: &Str) -> Result<PathBuf, ()> {
        if let Ok(path) = self.cfg.input.local_py_resolve(Path::new(&__name__[..])) {
            let (out, err) = if self.cfg.quiet_repl {
                (Stdio::null(), Stdio::null())
            } else {
                (Stdio::inherit(), Stdio::inherit())
            };
            // pylyzer is a static analysis tool for Python (https://github.com/mtshiba/pylyzer).
            // It can convert a Python script to an Erg AST for code analysis.
            // There is also an option to output the analysis result as `d.er`. Use this if the system have pylyzer installed.
            // A type definition file may be generated even if not all type checks succeed.
            if let Ok(_status) = Command::new("pylyzer")
                .arg("--dump-decl")
                .arg(path.to_str().unwrap())
                .stdout(out)
                .stderr(err)
                .spawn()
                .and_then(|mut child| child.wait())
            {
                if let Some(path) = Self::resolve_decl_path(&self.cfg, Path::new(&__name__[..])) {
                    return Ok(path);
                }
            }
        }
        Err(())
    }

    fn import_py_mod(&self, __name__: &Str, loc: &impl Locational) -> CompileResult<PathBuf> {
        let py_mod_cache = self.py_mod_cache();
        let path = self.get_path(__name__, loc)?;
        if let Some(referrer) = self.cfg.input.path() {
            let graph = &self.shared.as_ref().unwrap().graph;
            graph.inc_ref(referrer, path.clone());
        }
        if py_mod_cache.get(&path).is_some() {
            return Ok(path);
        }
        let mut cfg = self.cfg.inherit(path.clone());
        let src = cfg.input.read();
        let mut builder = HIRBuilder::new_with_cache(
            cfg,
            self.mod_name(&path),
            self.shared.as_ref().unwrap().clone(),
        );
        match builder.build(src, "declare") {
            Ok(artifact) => {
                let ctx = builder.pop_mod_ctx().unwrap();
                py_mod_cache.register(path.clone(), Some(artifact.object), ctx);
            }
            Err(artifact) => {
                if let Some(hir) = artifact.object {
                    py_mod_cache.register(path, Some(hir), builder.pop_mod_ctx().unwrap());
                }
                return Err(artifact.errors);
            }
        }
        Ok(path)
    }

    pub fn del(&mut self, ident: &hir::Identifier) -> CompileResult<()> {
        let is_const = self.rec_get_const_obj(ident.inspect()).is_some();
        let is_builtin = self
            .get_builtins()
            .unwrap()
            .get_var_kv(ident.inspect())
            .is_some();
        if is_const || is_builtin {
            Err(TyCheckErrors::from(TyCheckError::del_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident,
                is_const,
                self.caused_by(),
            )))
        } else if self.locals.get(ident.inspect()).is_some() {
            let vi = self.locals.remove(ident.inspect()).unwrap();
            self.deleted_locals.insert(ident.raw.name.clone(), vi);
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

    pub(crate) fn cast(
        &mut self,
        type_spec: ast::TypeSpec,
        call: &mut hir::Call,
    ) -> TyCheckResult<()> {
        let mut dummy_tv_cache = TyVarCache::new(self.level, self);
        let cast_to = self.instantiate_typespec(
            &type_spec,
            None,
            &mut dummy_tv_cache,
            RegistrationMode::Normal,
            false,
        )?;
        let Some(hir::Expr::BinOp(hir::BinOp { lhs, .. })) = call.args.get_mut_left_or_key("pred") else { todo!("{}", call.args) };
        match (
            self.supertype_of(lhs.ref_t(), &cast_to),
            self.subtype_of(lhs.ref_t(), &cast_to),
        ) {
            // assert 1 in {1}
            (true, true) => Ok(()),
            // assert x in Int (x: Nat)
            (false, true) => Ok(()), // TODO: warn (needless)
            // assert x in Nat (x: Int)
            (true, false) => {
                if let hir::Expr::Accessor(ref acc) = lhs.as_ref() {
                    self.change_var_type(acc, cast_to.clone())?;
                }
                match lhs.ref_t() {
                    Type::FreeVar(fv) if fv.is_linked() => {
                        let constraint = Constraint::new_subtype_of(cast_to);
                        fv.replace(FreeKind::new_unbound(self.level, constraint));
                    }
                    Type::FreeVar(fv) => {
                        let new_constraint = Constraint::new_subtype_of(cast_to);
                        fv.update_constraint(new_constraint, false);
                    }
                    _ => {
                        *lhs.ref_mut_t() = cast_to;
                    }
                }
                Ok(())
            }
            // assert x in Str (x: Int)
            (false, false) => Err(TyCheckErrors::from(TyCheckError::invalid_type_cast_error(
                self.cfg.input.clone(),
                line!() as usize,
                lhs.loc(),
                self.caused_by(),
                &lhs.to_string(),
                &cast_to,
                None,
            ))),
        }
    }

    fn change_var_type(&mut self, acc: &hir::Accessor, t: Type) -> TyCheckResult<()> {
        #[allow(clippy::single_match)]
        match acc {
            hir::Accessor::Ident(ident) => {
                if let Some(vi) = self.get_mut_current_scope_var(&ident.raw.name) {
                    vi.t = t;
                } else {
                    return Err(TyCheckErrors::from(TyCheckError::feature_error(
                        self.cfg.input.clone(),
                        acc.loc(),
                        &format!("casting {acc}"),
                        self.caused_by(),
                    )));
                }
            }
            _ => {
                // TODO: support other accessors
            }
        }
        Ok(())
    }

    fn inc_ref_acc(&self, acc: &ast::Accessor, namespace: &Context) {
        match acc {
            ast::Accessor::Ident(ident) => self.inc_ref_local(ident, namespace),
            ast::Accessor::Attr(attr) => {
                self.inc_ref_expr(&attr.obj, namespace);
                if let Ok(ctxs) = self.get_singular_ctxs(&attr.obj, self) {
                    if let Some(first) = ctxs.first() {
                        first.inc_ref_local(&attr.ident, namespace);
                    }
                }
            }
            _ => {}
        }
    }

    fn inc_ref_expr(&self, expr: &ast::Expr, namespace: &Context) {
        #[allow(clippy::single_match)]
        match expr {
            ast::Expr::Accessor(acc) => self.inc_ref_acc(acc, namespace),
            // TODO:
            _ => {}
        }
    }

    pub(crate) fn inc_ref_predecl_typespec(&self, predecl: &PreDeclTypeSpec, namespace: &Context) {
        match predecl {
            PreDeclTypeSpec::Attr { namespace: obj, t } => {
                self.inc_ref_expr(obj, namespace);
                if let Ok(ctxs) = self.get_singular_ctxs(obj, self) {
                    if let Some(first) = ctxs.first() {
                        first.inc_ref_simple_typespec(t, namespace);
                    }
                }
            }
            PreDeclTypeSpec::Simple(simple) => self.inc_ref_simple_typespec(simple, namespace),
            // TODO:
            _ => {}
        }
    }

    pub(crate) fn inc_ref_simple_typespec(&self, simple: &SimpleTypeSpec, namespace: &Context) {
        if let Triple::Ok(vi) = self.rec_get_var_info(
            &simple.ident,
            crate::compile::AccessKind::Name,
            &self.cfg.input,
            self,
        ) {
            self.inc_ref(&vi, &simple.ident.name, namespace);
        }
    }

    pub(crate) fn inc_ref_local(&self, local: &ConstIdentifier, namespace: &Context) {
        if let Triple::Ok(vi) = self.rec_get_var_info(
            local,
            crate::compile::AccessKind::Name,
            &self.cfg.input,
            self,
        ) {
            self.inc_ref(&vi, &local.name, namespace);
        }
    }

    pub fn inc_ref<L: Locational>(&self, vi: &VarInfo, name: &L, namespace: &Context) {
        self.index().inc_ref(vi, namespace.absolutize(name.loc()));
    }
}
