use std::fmt;
use std::io::BufRead;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use erg_common::config::ErgMode;
use erg_common::consts::PYTHON_MODE;
use erg_common::env::erg_pystd_path;
use erg_common::erg_util::BUILTIN_ERG_MODS;
use erg_common::levenshtein::get_similar_name;
use erg_common::python_util::BUILTIN_PYTHON_MODS;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::triple::Triple;
use erg_common::{get_hash, log, set};
use erg_common::{unique_in_place, Str};

use ast::{
    ConstIdentifier, Decorator, DefId, Identifier, OperationKind, PolyTypeSpec, PreDeclTypeSpec,
    VarName,
};
use erg_parser::ast;

use crate::ty::constructors::{
    free_var, func, func0, func1, proc, ref_, ref_mut, unknown_len_array_t, v_enum,
};
use crate::ty::free::{Constraint, HasLevel};
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{
    GuardType, HasType, ParamTy, SubrType, Type, Variable, Visibility, VisibilityModifier,
};

use crate::build_hir::HIRBuilder;
use crate::context::{
    ClassDefType, Context, ContextKind, DefaultInfo, MethodPair, RegistrationMode, TraitImpl,
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
    !name.is_empty() && !name.starts_with('/') && name.trim() == name
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckStatus {
    Succeed,
    Failed,
    Ongoing,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckStatus::Succeed => write!(f, "succeed"),
            CheckStatus::Failed => write!(f, "failed"),
            CheckStatus::Ongoing => write!(f, "ongoing"),
        }
    }
}

impl std::str::FromStr for CheckStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "succeed" => Ok(CheckStatus::Succeed),
            "failed" => Ok(CheckStatus::Failed),
            "ongoing" => Ok(CheckStatus::Ongoing),
            _ => Err(format!("invalid status: {s}")),
        }
    }
}

/// format:
/// ```python
/// #[pylyzer] succeed foo.py 1234567890
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PylyzerStatus {
    pub status: CheckStatus,
    pub file: PathBuf,
    pub timestamp: SystemTime,
    pub hash: u64,
}

impl fmt::Display for PylyzerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "##[pylyzer] {} {} {} {}",
            self.status,
            self.file.display(),
            self.timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            self.hash,
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
        let status = iter.next().ok_or("no succeed")?;
        let status = status.parse()?;
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
        let hash = iter.next().ok_or("no hash")?;
        let hash = hash.parse().map_err(|e| format!("hash parse error: {e}"))?;
        Ok(PylyzerStatus {
            status,
            file,
            timestamp,
            hash,
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
        let sig_t =
            self.instantiate_var_sig_t(sig.t_spec.as_ref().map(|ts| &ts.t_spec), PreRegister)?;
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
        let ident = if PYTHON_MODE && py_name.is_some() {
            let mut symbol = ident.name.clone().into_token();
            symbol.content = py_name.clone().unwrap();
            Identifier::new(ident.vis.clone(), VarName::new(symbol))
        } else {
            ident.clone()
        };
        let vis = self.instantiate_vis_modifier(&ident.vis)?;
        // already defined as const
        if sig.is_const() {
            let mut vi = self.decls.remove(ident.inspect()).unwrap_or_else(|| {
                VarInfo::new(
                    body_t.clone(),
                    Mutability::Const,
                    Visibility::new(vis, self.name.clone()),
                    VarKind::Declared,
                    None,
                    self.impl_of(),
                    py_name.clone(),
                    self.absolutize(ident.name.loc()),
                )
            });
            if vi.py_name.is_none() {
                vi.py_name = py_name;
            }
            self.locals.insert(ident.name.clone(), vi.clone());
            if let Ok(value) = self.convert_singular_type_into_value(vi.t.clone()) {
                self.consts.insert(ident.name.clone(), value);
            }
            return Ok(vi);
        }
        let muty = Mutability::from(&ident.inspect()[..]);
        let opt_vi = self
            .decls
            .remove(ident.inspect())
            .or_else(|| self.future_defined_locals.remove(ident.inspect()));
        let py_name = opt_vi
            .as_ref()
            .and_then(|vi| vi.py_name.clone())
            .or(py_name);
        let kind = if id.0 == 0 {
            VarKind::Declared
        } else {
            VarKind::Defined(id)
        };
        let t = sig.t_spec.as_ref().map_or(body_t.clone(), |ts| {
            if ts.ascription_kind().is_force_cast() {
                self.instantiate_typespec(&ts.t_spec)
                    .unwrap_or(body_t.clone())
            } else {
                body_t.clone()
            }
        });
        let vi = VarInfo::new(
            t,
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
        if let Ok(value) = self.convert_singular_type_into_value(vi.t.clone()) {
            self.consts.insert(ident.name.clone(), value);
        }
        Ok(vi)
    }

    fn type_self_param(
        &self,
        pat: &ast::ParamPattern,
        name: &VarName,
        spec_t: &Type,
        errs: &mut TyCheckErrors,
    ) {
        if let Some(self_t) = self.rec_get_self_t() {
            let self_t = match pat {
                ast::ParamPattern::Ref(_) => ref_(self_t),
                ast::ParamPattern::RefMut(_) => ref_mut(self_t, None),
                _ => self_t,
            };
            if let Err(es) = self.sub_unify(spec_t, &self_t, name, Some(name.inspect())) {
                errs.extend(es);
            }
        } else {
            log!(err "self_t is None");
        }
    }

    /// TODO: sig should be immutable
    /// 宣言が既にある場合、opt_decl_tに宣言の型を渡す
    fn assign_param(
        &mut self,
        sig: &mut hir::NonDefaultParamSignature,
        opt_decl_t: Option<&ParamTy>,
        kind: ParamKind,
    ) -> TyCheckResult<()> {
        let vis = if PYTHON_MODE {
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
                        self.type_self_param(&sig.raw.pat, name, &spec_t, &mut errs);
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
                        self.type_self_param(&sig.raw.pat, name, &spec_t, &mut errs);
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
                        self.type_self_param(&sig.raw.pat, name, &spec_t, &mut errs);
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

    pub(crate) fn assign_bounds(&mut self, tv_cache: &TyVarCache) {
        for tyvar in tv_cache.tyvar_instances.keys() {
            let vi =
                VarInfo::nd_parameter(Type::Type, self.absolutize(tyvar.loc()), self.name.clone());
            self.locals.insert(tyvar.clone(), vi);
        }
        for (typaram, tp) in tv_cache.typaram_instances.iter() {
            let t = self.get_tp_t(tp).unwrap_or(Type::Obj);
            let vi = VarInfo::nd_parameter(t, self.absolutize(typaram.loc()), self.name.clone());
            self.locals.insert(typaram.clone(), vi);
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
        body_loc: &impl Locational,
    ) -> Result<VarInfo, (TyCheckErrors, VarInfo)> {
        let mut errs = TyCheckErrors::empty();
        // already defined as const
        if sig.ident.is_const() {
            let vi = self.decls.remove(sig.ident.inspect()).unwrap();
            self.locals.insert(sig.ident.name.clone(), vi.clone());
            return Ok(vi);
        }
        let vis = match self.instantiate_vis_modifier(&sig.ident.vis) {
            Ok(vis) => vis,
            Err(es) => {
                errs.extend(es);
                VisibilityModifier::Private
            }
        };
        let muty = if sig.ident.is_const() {
            Mutability::Const
        } else {
            Mutability::Immutable
        };
        let name = &sig.ident.name;
        // FIXME: constでない関数
        let t = self.get_current_scope_var(name).map(|vi| &vi.t).unwrap();
        debug_assert!(t.is_subr(), "{t} is not subr");
        let empty = vec![];
        let non_default_params = t.non_default_params().unwrap_or(&empty);
        let var_args = t.var_params();
        let default_params = t.default_params().unwrap_or(&empty);
        if let Some(spec_ret_t) = t.return_t() {
            let unify_result = if let Some(t_spec) = sig.return_t_spec.as_ref() {
                self.sub_unify(body_t, spec_ret_t, t_spec, None)
            } else {
                self.sub_unify(body_t, spec_ret_t, body_loc, None)
            };
            if let Err(unify_errs) = unify_result {
                let es = TyCheckErrors::new(
                    unify_errs
                        .into_iter()
                        .map(|e| {
                            let expect = if cfg!(feature = "debug") {
                                spec_ret_t.clone()
                            } else {
                                self.readable_type(spec_ret_t.clone())
                            };
                            let found = if cfg!(feature = "debug") {
                                body_t.clone()
                            } else {
                                self.readable_type(body_t.clone())
                            };
                            TyCheckError::return_type_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                e.core.get_loc_with_fallback(),
                                e.caused_by,
                                readable_name(name.inspect()),
                                &expect,
                                &found,
                                e.core.get_hint().map(|s| s.to_string()),
                            )
                        })
                        .collect(),
                );
                errs.extend(es);
            }
        }
        // NOTE: not `body_t.clone()` because the body may contain `return`
        let return_t = t.return_t().unwrap().clone();
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
                errs.push(err);
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
        if errs.is_empty() {
            Ok(vi)
        } else {
            Err((errs, vi))
        }
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
                        total_errs.extend(errs);
                    }
                    if def.def_kind().is_import() {
                        if let Err(errs) = self.pre_import(def) {
                            total_errs.extend(errs);
                        }
                    }
                }
                ast::Expr::ClassDef(class_def) => {
                    if let Err(errs) = self.preregister_def(&class_def.def) {
                        total_errs.extend(errs);
                    }
                }
                ast::Expr::PatchDef(patch_def) => {
                    if let Err(errs) = self.preregister_def(&patch_def.def) {
                        total_errs.extend(errs);
                    }
                }
                ast::Expr::Dummy(dummy) => {
                    if let Err(errs) = self.preregister(&dummy.exprs) {
                        total_errs.extend(errs);
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
    fn pre_import(&mut self, def: &ast::Def) -> TyCheckResult<()> {
        let Some(ast::Expr::Call(call)) = def.body.block.first() else { unreachable!() };
        let Some(ast::Expr::Literal(mod_name)) = call.args.get_left_or_key("Path") else {
            return Ok(());
        };
        let Ok(mod_name) = hir::Literal::try_from(mod_name.token.clone()) else {
            return Ok(());
        };
        let res = self.import_mod(call.additional_operation().unwrap(), &mod_name);
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
        let Some(ident) = def.sig.ident() else { return Ok(()) };
        let Some((_, vi)) = self.get_var_info(ident.inspect()) else {
            return Ok(());
        };
        if let Some(fv) = vi.t.as_free() {
            fv.link(&typ);
        }
        res.map(|_| ())
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
                            .instantiate_typespec_full(
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
                    let kind = ContextKind::from(def);
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
                            .instantiate_typespec_full(
                                &spec.t_spec,
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

    pub(crate) fn register_marker_trait(&mut self, ctx: &Self, trait_: Type) -> CompileResult<()> {
        let (_, trait_ctx) = ctx.get_nominal_type_ctx(&trait_).ok_or_else(|| {
            CompileError::type_not_found(
                self.cfg.input.clone(),
                line!() as usize,
                ().loc(),
                self.caused_by(),
                &trait_,
            )
        })?;
        // self.register_supertrait(trait_, ctx);
        let traits = trait_ctx.super_traits.clone();
        self.super_traits.push(trait_);
        self.super_traits.extend(traits);
        unique_in_place(&mut self.super_traits);
        Ok(())
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
                        let meta_t = gen.meta_type();
                        self.register_type_alias(ident, gen.into_typ(), meta_t)
                    }
                    TypeObj::Generated(gen) => self.register_gen_type(ident, gen),
                    TypeObj::Builtin { t, meta_t } => self.register_type_alias(ident, t, meta_t),
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
                            TypeObj::Builtin {
                                t: Type::Record(rec),
                                ..
                            } => {
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
                        let (_, sup_ctx) = self.get_nominal_type_ctx(&sup).ok_or_else(|| {
                            TyCheckErrors::from(TyCheckError::type_not_found(
                                self.cfg.input.clone(),
                                line!() as usize,
                                ident.loc(),
                                self.caused_by(),
                                &sup,
                            ))
                        })?;
                        ctx.register_superclass(sup, sup_ctx);
                    }
                    let mut methods =
                        Self::methods(None, self.cfg.clone(), self.shared.clone(), 2, self.level);
                    if let Some(sup) = gen.base_or_sup() {
                        let param_t = match sup {
                            TypeObj::Builtin { t, .. } => Some(t),
                            TypeObj::Generated(t) => t.base_or_sup().map(|t| t.typ()),
                        };
                        // `Super.Requirement := {x = Int}` and `Self.Additional := {y = Int}`
                        // => `Self.Requirement := {x = Int; y = Int}`
                        let param_t = if let Some(additional) = gen.additional() {
                            if let TypeObj::Builtin {
                                t: Type::Record(rec),
                                ..
                            } = additional
                            {
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
                            param_t
                                .map(|t| self.intersection(t, additional.typ()))
                                .or(Some(additional.typ().clone()))
                        } else {
                            param_t.cloned()
                        };
                        let new_t = if let Some(t) = param_t {
                            func1(t, gen.typ().clone())
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
                    let Some(TypeObj::Builtin{ t: Type::Record(req), .. }) = gen.base_or_sup() else { todo!("{gen}") };
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
                    let additional = if let Some(TypeObj::Builtin {
                        t: Type::Record(additional),
                        ..
                    }) = gen.additional()
                    {
                        Some(additional)
                    } else {
                        None
                    };
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
                    let Some(TypeObj::Builtin{ t: base, .. }) = gen.base_or_sup() else { todo!("{gen}") };
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

    pub(crate) fn register_type_alias(
        &mut self,
        ident: &Identifier,
        t: Type,
        meta_t: Type,
    ) -> CompileResult<()> {
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
            let val = ValueObj::Type(TypeObj::Builtin { t, meta_t });
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
            let val = ValueObj::Type(TypeObj::Generated(gen));
            let meta_t = v_enum(set! { val.clone() });
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
            self.consts.insert(name.clone(), val);
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
                    types.push(MethodPair::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_traits.insert(
                        trait_method.inspect().clone(),
                        vec![MethodPair::new(t.clone(), vi.clone())],
                    );
                }
            }
            for (class_method, vi) in ctx.locals.iter() {
                if let Some(types) = self.method_to_classes.get_mut(class_method.inspect()) {
                    types.push(MethodPair::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_classes.insert(
                        class_method.inspect().clone(),
                        vec![MethodPair::new(t.clone(), vi.clone())],
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
                    types.push(MethodPair::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_traits.insert(
                        trait_method.inspect().clone(),
                        vec![MethodPair::new(t.clone(), vi.clone())],
                    );
                }
            }
            for (class_method, vi) in ctx.locals.iter() {
                if let Some(types) = self.method_to_classes.get_mut(class_method.inspect()) {
                    types.push(MethodPair::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_classes.insert(
                        class_method.inspect().clone(),
                        vec![MethodPair::new(t.clone(), vi.clone())],
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

    fn import_err(&self, __name__: &Str, loc: &impl Locational) -> TyCheckErrors {
        let mod_cache = self.mod_cache();
        let py_mod_cache = self.py_mod_cache();
        TyCheckErrors::from(TyCheckError::import_error(
            self.cfg.input.clone(),
            line!() as usize,
            format!("module {__name__} not found"),
            loc.loc(),
            self.caused_by(),
            self.similar_builtin_erg_mod_name(__name__)
                .or_else(|| mod_cache.get_similar_name(__name__)),
            self.similar_builtin_py_mod_name(__name__)
                .or_else(|| py_mod_cache.get_similar_name(__name__)),
        ))
    }

    fn import_erg_mod(&self, __name__: &Str, loc: &impl Locational) -> CompileResult<PathBuf> {
        let mod_cache = self.mod_cache();
        let path = match self.cfg.input.resolve_real_path(Path::new(&__name__[..])) {
            Some(path) => path,
            None => {
                return Err(self.import_err(__name__, loc));
            }
        };
        // module itself
        if self.cfg.input.path() == Some(path.as_path()) {
            return Ok(path);
        }
        if let Some(referrer) = self.cfg.input.path() {
            let graph = &self.shared.as_ref().unwrap().graph;
            graph.inc_ref(referrer, path.clone());
        }
        if mod_cache.get(&path).is_some() {
            return Ok(path);
        }
        let mut cfg = self.cfg.inherit(path.clone());
        let src = cfg
            .input
            .try_read()
            .map_err(|_| self.import_err(__name__, loc))?;
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
        let meta = std::fs::metadata(&status.file).ok()?;
        let dummy_hash = meta.len();
        if status.hash != dummy_hash {
            None
        } else {
            Some(status)
        }
    }

    fn get_decl_path(&self, __name__: &Str, loc: &impl Locational) -> CompileResult<PathBuf> {
        match self.cfg.input.resolve_decl_path(Path::new(&__name__[..])) {
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
        if let Ok(path) = self.cfg.input.resolve_py(Path::new(&__name__[..])) {
            let (out, err) = if self.cfg.mode == ErgMode::LanguageServer || self.cfg.quiet_repl {
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
                if let Some(path) = self.cfg.input.resolve_decl_path(Path::new(&__name__[..])) {
                    return Ok(path);
                }
            }
        }
        Err(())
    }

    fn import_py_mod(&self, __name__: &Str, loc: &impl Locational) -> CompileResult<PathBuf> {
        let py_mod_cache = self.py_mod_cache();
        let path = self.get_decl_path(__name__, loc)?;
        // module itself
        if self.cfg.input.path() == Some(path.as_path()) {
            return Ok(path);
        }
        if let Some(referrer) = self.cfg.input.path() {
            let graph = &self.shared.as_ref().unwrap().graph;
            graph.inc_ref(referrer, path.clone());
        }
        if py_mod_cache.get(&path).is_some() {
            return Ok(path);
        }
        let mut cfg = self.cfg.inherit(path.clone());
        let src = cfg
            .input
            .try_read()
            .map_err(|_| self.import_err(__name__, loc))?;
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
        let is_const = self
            .rec_get_var_info(&ident.raw, crate::AccessKind::Name, &self.cfg.input, self)
            .map_ok_or(false, |vi| vi.muty.is_const());
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
        guard: GuardType,
        overwritten: &mut Vec<(VarName, VarInfo)>,
    ) -> TyCheckResult<()> {
        if let Variable::Var(name, _) = &guard.var {
            let vi = if let Some((name, vi)) = self.locals.remove_entry(name) {
                overwritten.push((name, vi.clone()));
                vi
            } else if let Some((n, vi)) = self.get_var_kv(name) {
                overwritten.push((n.clone(), vi.clone()));
                vi.clone()
            } else {
                VarInfo::nd_parameter(
                    *guard.to.clone(),
                    self.absolutize(().loc()),
                    self.name.clone(),
                )
            };
            match self.recover_typarams(&vi.t, &guard) {
                Ok(t) => {
                    self.locals
                        .insert(VarName::from_str(name.clone()), VarInfo { t, ..vi });
                }
                Err(errs) => {
                    self.locals.insert(VarName::from_str(name.clone()), vi);
                    return Err(errs);
                }
            }
        } /* else {
              return Err(TyCheckErrors::from(TyCheckError::feature_error(
                  self.cfg.input.clone(),
                  guard.var.loc(),
                  &format!("casting {}", guard.var),
                  self.caused_by(),
              )));
          } */
        Ok(())
    }

    pub(crate) fn inc_ref<L: Locational>(&self, vi: &VarInfo, name: &L, namespace: &Context) {
        if let Some(index) = self.opt_index() {
            index.inc_ref(vi, namespace.absolutize(name.loc()));
        }
    }

    pub(crate) fn inc_ref_acc(&self, acc: &ast::Accessor, namespace: &Context) -> bool {
        match acc {
            ast::Accessor::Ident(ident) => self.inc_ref_local(ident, namespace),
            ast::Accessor::Attr(attr) => {
                self.inc_ref_expr(&attr.obj, namespace);
                if let Ok(ctxs) = self.get_singular_ctxs(&attr.obj, self) {
                    for ctx in ctxs {
                        if ctx.inc_ref_local(&attr.ident, namespace) {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(crate) fn inc_ref_predecl_typespec(
        &self,
        predecl: &PreDeclTypeSpec,
        namespace: &Context,
    ) -> bool {
        match predecl {
            PreDeclTypeSpec::Mono(mono) => self.inc_ref_mono_typespec(mono, namespace),
            PreDeclTypeSpec::Poly(poly) => self.inc_ref_poly_typespec(poly, namespace),
            PreDeclTypeSpec::Attr { namespace: obj, t } => {
                self.inc_ref_expr(obj, namespace);
                if let Ok(ctxs) = self.get_singular_ctxs(obj, self) {
                    for ctx in ctxs {
                        if ctx.inc_ref_mono_typespec(t, namespace) {
                            return true;
                        }
                    }
                }
                false
            }
            // TODO:
            _ => false,
        }
    }

    fn inc_ref_mono_typespec(&self, ident: &Identifier, namespace: &Context) -> bool {
        if let Triple::Ok(vi) = self.rec_get_var_info(
            ident,
            crate::compile::AccessKind::Name,
            &self.cfg.input,
            self,
        ) {
            self.inc_ref(&vi, &ident.name, namespace);
            true
        } else {
            false
        }
    }

    /// TODO: params
    fn inc_ref_poly_typespec(&self, poly: &PolyTypeSpec, namespace: &Context) -> bool {
        self.inc_ref_acc(&poly.acc.clone().downgrade(), namespace)
    }

    fn inc_ref_local(&self, local: &ConstIdentifier, namespace: &Context) -> bool {
        if let Triple::Ok(vi) = self.rec_get_var_info(
            local,
            crate::compile::AccessKind::Name,
            &self.cfg.input,
            self,
        ) {
            self.inc_ref(&vi, &local.name, namespace);
            true
        } else {
            &local.inspect()[..] == "module" || &local.inspect()[..] == "global"
        }
    }

    fn inc_ref_expr(&self, expr: &ast::Expr, namespace: &Context) -> bool {
        #[allow(clippy::single_match)]
        match expr {
            ast::Expr::Accessor(acc) => self.inc_ref_acc(acc, namespace),
            // TODO:
            _ => false,
        }
    }
}
