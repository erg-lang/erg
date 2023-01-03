//! Defines `Context`.
//!
//! `Context` is used for type inference and type checking.
#![allow(clippy::result_unit_err)]
pub mod cache;
pub mod compare;
pub mod eval;
pub mod generalize;
pub mod hint;
pub mod initialize;
pub mod inquire;
pub mod instantiate;
pub mod register;
pub mod test;
pub mod unify;

use std::fmt;
use std::mem;
use std::option::Option; // conflicting to Type::Option
use std::path::Path;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::impl_display_from_debug;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::Str;
use erg_common::{fn_name, get_hash, log};

use crate::ty::value::ValueObj;
use crate::ty::{Predicate, Type};
use erg_parser::ast::DefKind;
use Type::*;

use ast::{DefId, VarName};
use erg_parser::ast;
use erg_parser::token::Token;

use crate::context::instantiate::{ConstTemplate, TyVarCache};
use crate::error::{TyCheckError, TyCheckErrors};
use crate::mod_cache::SharedModuleCache;
use crate::varinfo::{Mutability, VarInfo, VarKind};
use Visibility::*;

/// For implementing LSP or other IDE features
pub trait ContextProvider {
    fn dir(&self) -> Vec<(&VarName, &VarInfo)>;
    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context>;
    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)>;
}

const BUILTINS: &Str = &Str::ever("<builtins>");

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitImpl {
    pub sub_type: Type,
    pub sup_trait: Type,
}

impl std::fmt::Display for TraitImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TraitImpl{{{} <: {}}}", self.sub_type, self.sup_trait)
    }
}

impl TraitImpl {
    pub const fn new(sub_type: Type, sup_trait: Type) -> Self {
        Self {
            sub_type,
            sup_trait,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClassDefType {
    Simple(Type),
    ImplTrait { class: Type, impl_trait: Type },
}

impl std::fmt::Display for ClassDefType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassDefType::Simple(ty) => write!(f, "{ty}"),
            ClassDefType::ImplTrait { class, impl_trait } => {
                write!(f, "{class}|<: {impl_trait}|")
            }
        }
    }
}

impl ClassDefType {
    pub const fn impl_trait(class: Type, impl_trait: Type) -> Self {
        ClassDefType::ImplTrait { class, impl_trait }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultInfo {
    NonDefault,
    WithDefault,
}

impl_display_from_debug!(DefaultInfo);

impl DefaultInfo {
    pub const fn has_default(&self) -> bool {
        matches!(self, DefaultInfo::WithDefault)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variance {
    /// Output(T)
    Covariant, // 共変
    /// Input(T)
    Contravariant, // 反変
    #[default]
    Invariant, // 不変
}

impl_display_from_debug!(Variance);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamSpec {
    pub(crate) name: Option<&'static str>,
    // TODO: `:` or `<:`
    pub(crate) t: Type,
    pub default_info: DefaultInfo,
}

impl ParamSpec {
    pub const fn new(name: Option<&'static str>, t: Type, default: DefaultInfo) -> Self {
        Self {
            name,
            t,
            default_info: default,
        }
    }

    pub const fn named(name: &'static str, t: Type, default: DefaultInfo) -> Self {
        Self::new(Some(name), t, default)
    }

    pub const fn named_nd(name: &'static str, t: Type) -> Self {
        Self::new(Some(name), t, DefaultInfo::NonDefault)
    }

    pub const fn t(name: &'static str, default: DefaultInfo) -> Self {
        Self::new(Some(name), Type, default)
    }

    pub const fn t_nd(name: &'static str) -> Self {
        Self::new(Some(name), Type, DefaultInfo::NonDefault)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContextKind {
    Func,
    Proc,
    Class,
    MethodDefs(Option<Type>), // Type: trait implemented
    PatchMethodDefs(Type),
    Trait,
    StructuralTrait,
    Patch(Type),
    StructuralPatch(Type),
    GluePatch(TraitImpl), // TODO: deprecate (integrate into Patch)
    Module,
    Instant,
    Dummy,
}

impl From<DefKind> for ContextKind {
    fn from(kind: DefKind) -> Self {
        match kind {
            DefKind::Class | DefKind::Inherit => Self::Class,
            DefKind::Trait | DefKind::Subsume => Self::Trait,
            DefKind::StructuralTrait => Self::StructuralTrait,
            DefKind::ErgImport | DefKind::PyImport => Self::Module,
            DefKind::Other => Self::Instant,
        }
    }
}

impl ContextKind {
    pub const fn is_method_def(&self) -> bool {
        matches!(self, Self::MethodDefs(_))
    }

    pub const fn is_type(&self) -> bool {
        matches!(self, Self::Class | Self::Trait | Self::StructuralTrait)
    }

    pub fn is_class(&self) -> bool {
        matches!(self, Self::Class)
    }

    pub fn is_trait(&self) -> bool {
        matches!(self, Self::Trait | Self::StructuralTrait)
    }

    pub fn is_patch(&self) -> bool {
        matches!(self, Self::Patch(_) | Self::GluePatch(_))
    }
}

/// Indicates the mode registered in the Context
/// Preregister: subroutine or constant expression, can be forward referenced
/// Normal: Cannot be forward referenced
/// 環境に登録されているモードを表す
/// Preregister: サブルーチンまたは定数式、前方参照できる
/// Normal: 前方参照できない
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationMode {
    PreRegister,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextInfo {
    mod_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodInfo {
    definition_type: Type,
    method_type: VarInfo,
}

impl fmt::Display for MethodInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ def: {} info: {} }}",
            self.definition_type, self.method_type
        )
    }
}

impl MethodInfo {
    pub const fn new(definition_type: Type, method_type: VarInfo) -> Self {
        Self {
            definition_type,
            method_type,
        }
    }
}

/// Represents the context of the current scope
///
/// Recursive functions/methods are highlighted with the prefix `rec_`, as performance may be significantly degraded.
#[derive(Debug, Clone)]
pub struct Context {
    pub name: Str,
    pub kind: ContextKind,
    pub(crate) cfg: ErgConfig,
    pub(crate) preds: Vec<Predicate>,
    /// for looking up the parent scope
    pub(crate) outer: Option<Box<Context>>,
    // e.g. { "Add": [ConstObjTemplate::App("Self", vec![])])
    pub(crate) const_param_defaults: Dict<Str, Vec<ConstTemplate>>,
    // Superclasses/supertraits by a patch are not included here
    // patchによってsuper class/traitになったものはここに含まれない
    pub(crate) super_classes: Vec<Type>, // if self is a patch, means patch classes
    pub(crate) super_traits: Vec<Type>,  // if self is not a trait, means implemented traits
    // method definitions, if the context is a type
    // specializations are included and needs to be separated out
    pub(crate) methods_list: Vec<(ClassDefType, Context)>,
    // K: method name, V: types defines the method
    // If it is declared in a trait, it takes precedence over the class.
    pub(crate) method_to_traits: Dict<Str, Vec<MethodInfo>>,
    pub(crate) method_to_classes: Dict<Str, Vec<MethodInfo>>,
    /// K: method name, V: impl patch
    /// Provided methods can switch implementations on a scope-by-scope basis
    /// K: メソッド名, V: それを実装するパッチたち
    /// 提供メソッドはスコープごとに実装を切り替えることができる
    pub(crate) method_impl_patches: Dict<VarName, Vec<VarName>>,
    /// K: name of a trait, V: (type, monomorphised trait that the type implements)
    /// K: トレイトの名前, V: (型, その型が実装する単相化トレイト)
    /// e.g. { "Named": [(Type, Named), (Func, Named), ...], "Add": [(Nat, Add(Nat)), (Int, Add(Int)), ...], ... }
    pub(crate) trait_impls: Dict<Str, Set<TraitImpl>>,
    /// stores declared names (not initialized)
    pub(crate) decls: Dict<VarName, VarInfo>,
    /// for error reporting
    pub(crate) future_defined_locals: Dict<VarName, VarInfo>,
    pub(crate) deleted_locals: Dict<VarName, VarInfo>,
    // stores defined names
    // 型の一致はHashMapでは判定できないため、keyはVarNameとして1つずつ見ていく
    /// ```python
    /// f [x, y], z = ...
    /// ```
    /// => params: vec![(None, [T; 2]), (Some("z"), U)]
    /// => locals: {"x": T, "y": T}
    /// TODO: impl params desugaring and replace to `Dict`
    pub(crate) params: Vec<(Option<VarName>, VarInfo)>,
    pub(crate) locals: Dict<VarName, VarInfo>,
    pub(crate) consts: Dict<VarName, ValueObj>,
    // {"Nat": ctx, "Int": ctx, ...}
    pub(crate) mono_types: Dict<VarName, (Type, Context)>,
    // Implementation Contexts for Polymorphic Types
    // Vec<TyParam> are specialization parameters
    // e.g. {"Array": [(Array(Nat), ctx), (Array(Int), ctx), (Array(Str), ctx), (Array(Obj), ctx), (Array('T), ctx)], ...}
    pub(crate) poly_types: Dict<VarName, (Type, Context)>,
    // patches can be accessed like normal records
    // but when used as a fallback to a type, values are traversed instead of accessing by keys
    pub(crate) patches: Dict<VarName, Context>,
    pub(crate) mod_cache: Option<SharedModuleCache>,
    pub(crate) py_mod_cache: Option<SharedModuleCache>,
    pub(crate) tv_cache: Option<TyVarCache>,
    pub(crate) higher_order_caller: Vec<Str>,
    pub(crate) level: usize,
}

impl Default for Context {
    #[inline]
    fn default() -> Self {
        Self::default_with_name("<dummy>")
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("name", &self.name)
            .field("preds", &self.preds)
            .field("params", &self.params)
            .field("decls", &self.decls)
            .field("locals", &self.params)
            .field("consts", &self.consts)
            .field("mono_types", &self.mono_types)
            .field("poly_types", &self.poly_types)
            .field("patches", &self.patches)
            // .field("mod_cache", &self.mod_cache)
            .finish()
    }
}

impl ContextProvider for Context {
    fn dir(&self) -> Vec<(&VarName, &VarInfo)> {
        let mut vars: Vec<_> = self
            .locals
            .iter()
            .chain(
                self.params
                    .iter()
                    .filter_map(|(k, v)| k.as_ref().map(|k| (k, v))),
            )
            .chain(self.methods_list.iter().flat_map(|(_, ctx)| ctx.dir()))
            .collect();
        for sup in self.super_classes.iter() {
            if let Some((_, sup_ctx)) = self.get_nominal_type_ctx(sup) {
                vars.extend(sup_ctx.type_dir());
            }
        }
        if let Some(outer) = self.get_outer() {
            vars.extend(outer.dir());
        } else if let Some(builtins) = self.get_builtins() {
            vars.extend(builtins.locals.iter());
        }
        vars
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.get_mod(receiver_name)
            .or_else(|| self.rec_get_type(receiver_name).map(|(_, ctx)| ctx))
            .or_else(|| {
                let (_, vi) = self.get_var_info(receiver_name)?;
                self.get_nominal_type_ctx(&vi.t).map(|(_, ctx)| ctx)
            })
    }

    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        if let Some(info) = self.get_local_kv(name) {
            Some(info)
        } else {
            if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
                return parent.get_var_info(name);
            }
            None
        }
    }
}

impl Context {
    pub fn dir(&self) -> Vec<(&VarName, &VarInfo)> {
        ContextProvider::dir(self)
    }

    pub fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        ContextProvider::get_receiver_ctx(self, receiver_name)
    }

    pub fn get_receiver_ctxs(&self, receiver_name: &str) -> Vec<&Context> {
        let mut ctxs = vec![];
        if let Some(receiver_ctx) = self.get_receiver_ctx(receiver_name) {
            ctxs.push(receiver_ctx);
            ctxs.extend(
                receiver_ctx
                    .super_classes
                    .iter()
                    .flat_map(|t| self.get_nominal_type_ctx(t).map(|(_, ctx)| ctx)),
            );
        }
        ctxs
    }

    pub fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        ContextProvider::get_var_info(self, name)
    }
}

impl Context {
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn new(
        name: Str,
        cfg: ErgConfig,
        kind: ContextKind,
        params: Vec<ParamSpec>,
        outer: Option<Context>,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        level: usize,
    ) -> Self {
        Self::with_capacity(
            name,
            cfg,
            kind,
            params,
            outer,
            mod_cache,
            py_mod_cache,
            0,
            level,
        )
    }

    pub fn default_with_name(name: &'static str) -> Self {
        Self::new(
            name.into(),
            ErgConfig::default(),
            ContextKind::Dummy,
            vec![],
            None,
            None,
            None,
            Self::TOP_LEVEL,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_capacity(
        name: Str,
        cfg: ErgConfig,
        kind: ContextKind,
        params: Vec<ParamSpec>,
        outer: Option<Context>,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        let mut params_ = Vec::new();
        for param in params.into_iter() {
            let id = DefId(get_hash(&(&name, &param)));
            if let Some(name) = param.name {
                let kind = VarKind::parameter(id, param.default_info);
                let muty = Mutability::from(name);
                let vi = VarInfo::new(param.t, muty, Private, kind, None, None, None);
                params_.push((Some(VarName::new(Token::static_symbol(name))), vi));
            } else {
                let kind = VarKind::parameter(id, param.default_info);
                let muty = Mutability::Immutable;
                let vi = VarInfo::new(param.t, muty, Private, kind, None, None, None);
                params_.push((None, vi));
            }
        }
        Self {
            name,
            cfg,
            kind,
            preds: vec![],
            outer: outer.map(Box::new),
            super_classes: vec![],
            super_traits: vec![],
            methods_list: vec![],
            const_param_defaults: Dict::default(),
            method_to_traits: Dict::default(),
            method_to_classes: Dict::default(),
            method_impl_patches: Dict::default(),
            trait_impls: Dict::default(),
            params: params_,
            decls: Dict::default(),
            future_defined_locals: Dict::default(),
            deleted_locals: Dict::default(),
            locals: Dict::with_capacity(capacity),
            consts: Dict::default(),
            mono_types: Dict::default(),
            poly_types: Dict::default(),
            mod_cache,
            py_mod_cache,
            tv_cache: None,
            patches: Dict::default(),
            higher_order_caller: vec![],
            level,
        }
    }

    #[inline]
    pub fn mono(
        name: Str,
        cfg: ErgConfig,
        kind: ContextKind,
        outer: Option<Context>,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        level: usize,
    ) -> Self {
        Self::new(
            name,
            cfg,
            kind,
            vec![],
            outer,
            mod_cache,
            py_mod_cache,
            level,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn poly(
        name: Str,
        cfg: ErgConfig,
        kind: ContextKind,
        params: Vec<ParamSpec>,
        outer: Option<Context>,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::with_capacity(
            name,
            cfg,
            kind,
            params,
            outer,
            mod_cache,
            py_mod_cache,
            capacity,
            level,
        )
    }

    pub fn poly_trait<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        let name = name.into();
        Self::poly(
            name,
            cfg,
            ContextKind::Trait,
            params,
            None,
            mod_cache,
            py_mod_cache,
            capacity,
            level,
        )
    }

    #[inline]
    pub fn builtin_poly_trait<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        capacity: usize,
    ) -> Self {
        Self::poly_trait(
            name,
            params,
            ErgConfig::default(),
            None,
            None,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    pub fn poly_class<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        let name = name.into();
        Self::poly(
            name,
            cfg,
            ContextKind::Class,
            params,
            None,
            mod_cache,
            py_mod_cache,
            capacity,
            level,
        )
    }

    #[inline]
    pub fn builtin_poly_class<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        capacity: usize,
    ) -> Self {
        Self::poly_class(
            name,
            params,
            ErgConfig::default(),
            None,
            None,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn poly_patch<S: Into<Str>>(
        name: S,
        base: Type,
        params: Vec<ParamSpec>,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        let name = name.into();
        Self::poly(
            name,
            cfg,
            ContextKind::Patch(base),
            params,
            None,
            mod_cache,
            py_mod_cache,
            capacity,
            level,
        )
    }

    #[inline]
    pub fn mono_trait<S: Into<Str>>(
        name: S,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::poly_trait(name, vec![], cfg, mod_cache, py_mod_cache, capacity, level)
    }

    #[inline]
    pub fn builtin_mono_trait<S: Into<Str>>(name: S, capacity: usize) -> Self {
        Self::mono_trait(
            name,
            ErgConfig::default(),
            None,
            None,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[inline]
    pub fn mono_class<S: Into<Str>>(
        name: S,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::poly_class(name, vec![], cfg, mod_cache, py_mod_cache, capacity, level)
    }

    #[inline]
    pub fn builtin_mono_class<S: Into<Str>>(name: S, capacity: usize) -> Self {
        Self::mono_class(
            name,
            ErgConfig::default(),
            None,
            None,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[inline]
    pub fn mono_patch<S: Into<Str>>(
        name: S,
        base: Type,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::poly_patch(
            name,
            base,
            vec![],
            cfg,
            mod_cache,
            py_mod_cache,
            capacity,
            level,
        )
    }

    #[inline]
    pub fn methods(
        impl_trait: Option<Type>,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        let name = if let Some(tr) = &impl_trait {
            tr.local_name()
        } else {
            Str::ever("Methods")
        };
        Self::with_capacity(
            name,
            cfg,
            ContextKind::MethodDefs(impl_trait),
            vec![],
            None,
            mod_cache,
            py_mod_cache,
            capacity,
            level,
        )
    }

    #[inline]
    pub fn builtin_methods(impl_trait: Option<Type>, capacity: usize) -> Self {
        Self::methods(
            impl_trait,
            ErgConfig::default(),
            None,
            None,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn poly_glue_patch<S: Into<Str>>(
        name: S,
        base: Type,
        impls: Type,
        params: Vec<ParamSpec>,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::poly(
            name.into(),
            cfg,
            ContextKind::GluePatch(TraitImpl::new(base, impls)),
            params,
            None,
            mod_cache,
            py_mod_cache,
            capacity,
            level,
        )
    }

    #[inline]
    pub fn builtin_poly_glue_patch<S: Into<Str>>(
        name: S,
        base: Type,
        impls: Type,
        params: Vec<ParamSpec>,
        capacity: usize,
    ) -> Self {
        Self::poly_glue_patch(
            name,
            base,
            impls,
            params,
            ErgConfig::default(),
            None,
            None,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[inline]
    pub fn module(
        name: Str,
        cfg: ErgConfig,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        capacity: usize,
    ) -> Self {
        Self::with_capacity(
            name,
            cfg,
            ContextKind::Module,
            vec![],
            None,
            mod_cache,
            py_mod_cache,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[inline]
    pub fn builtin_module<S: Into<Str>>(name: S, cfg: ErgConfig, capacity: usize) -> Self {
        Self::module(name.into(), cfg, None, None, capacity)
    }

    #[inline]
    pub fn instant(
        name: Str,
        cfg: ErgConfig,
        capacity: usize,
        mod_cache: Option<SharedModuleCache>,
        py_mod_cache: Option<SharedModuleCache>,
        outer: Context,
    ) -> Self {
        Self::with_capacity(
            name,
            cfg,
            ContextKind::Instant,
            vec![],
            Some(outer),
            mod_cache,
            py_mod_cache,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[inline]
    pub fn caused_by(&self) -> String {
        String::from(&self.name[..])
    }

    pub(crate) fn get_outer(&self) -> Option<&Context> {
        self.outer.as_ref().map(|x| x.as_ref())
    }

    pub(crate) fn impl_of(&self) -> Option<Type> {
        if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
            Some(tr.clone())
        } else {
            None
        }
    }

    pub(crate) fn path(&self) -> Str {
        // NOTE: this need to be changed if we want to support nested classes/traits
        if let Some(outer) = self.get_outer() {
            outer.path()
        } else if self.kind == ContextKind::Module {
            self.name.clone()
        } else {
            BUILTINS.clone()
        }
    }

    /// Returns None if self is `<builtins>`.
    /// This avoids infinite loops.
    pub(crate) fn get_builtins(&self) -> Option<&Context> {
        // builtins中で定義した型等はmod_cacheがNoneになっている
        if self.kind != ContextKind::Module || &self.path()[..] != "<builtins>" {
            self.mod_cache
                .as_ref()
                .map(|cache| cache.ref_ctx(Path::new("<builtins>")).unwrap())
                .map(|mod_ctx| &mod_ctx.context)
        } else {
            None
        }
    }

    /// This method is intended to be called __only__ in the top-level module.
    /// `.cfg` is not initialized and is used around.
    pub fn initialize(&mut self) {
        let mut mod_cache = mem::take(&mut self.mod_cache);
        if let Some(mod_cache) = &mut mod_cache {
            mod_cache.initialize();
        }
        let mut py_mod_cache = mem::take(&mut self.py_mod_cache);
        if let Some(py_mod_cache) = &mut py_mod_cache {
            py_mod_cache.initialize();
        }
        *self = Self::new(
            self.name.clone(),
            self.cfg.clone(),
            self.kind.clone(),
            vec![],
            None,
            mod_cache,
            py_mod_cache,
            self.level,
        );
    }

    pub(crate) fn grow(
        &mut self,
        name: &str,
        kind: ContextKind,
        vis: Visibility,
        tv_cache: Option<TyVarCache>,
    ) {
        let name = if vis.is_public() {
            format!("{parent}.{name}", parent = self.name)
        } else {
            format!("{parent}::{name}", parent = self.name)
        };
        log!(info "{}: current namespace: {name}", fn_name!());
        self.outer = Some(Box::new(mem::take(self)));
        self.cfg = self.get_outer().unwrap().cfg.clone();
        self.mod_cache = self.get_outer().unwrap().mod_cache.clone();
        self.py_mod_cache = self.get_outer().unwrap().py_mod_cache.clone();
        self.tv_cache = tv_cache;
        self.name = name.into();
        self.kind = kind;
    }

    pub(crate) fn clear_invalid_vars(&mut self) {
        self.locals.retain(|_, v| v.t != Failure);
        self.decls.retain(|_, v| v.t != Failure);
        self.params.retain(|(_, v)| v.t != Failure);
    }

    pub fn pop(&mut self) -> Context {
        if let Some(parent) = self.outer.as_mut() {
            let parent = mem::take(parent);
            let ctx = mem::take(self);
            *self = *parent;
            log!(info "{}: current namespace: {}", fn_name!(), self.name);
            ctx
        } else {
            panic!("cannot pop the top-level context (or use `pop_mod`)");
        }
    }

    /// unlike `pop`, `outer` must be `None`.
    pub fn pop_mod(&mut self) -> Option<Context> {
        if self.outer.is_some() {
            log!(err "not in the top-level context");
            None
        } else {
            log!(info "{}: current namespace: <builtins>", fn_name!());
            // toplevel
            Some(mem::take(self))
        }
    }

    pub(crate) fn check_decls_and_pop(&mut self) -> Result<Context, TyCheckErrors> {
        self.check_decls().map_err(|errs| {
            self.pop();
            errs
        })?;
        Ok(self.pop())
    }

    pub(crate) fn check_decls(&mut self) -> Result<(), TyCheckErrors> {
        let mut uninited_errs = TyCheckErrors::empty();
        for (name, vi) in self.decls.iter() {
            uninited_errs.push(TyCheckError::uninitialized_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                self.caused_by(),
                name.inspect(),
                &vi.t,
            ));
        }
        if !uninited_errs.is_empty() {
            Err(uninited_errs)
        } else {
            Ok(())
        }
    }

    fn type_dir(&self) -> Vec<(&VarName, &VarInfo)> {
        self.locals
            .iter()
            .chain(self.methods_list.iter().flat_map(|(_, ctx)| ctx.dir()))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ModuleContext {
    pub context: Context,
    pub scope: Dict<Str, Context>,
}

impl ModuleContext {
    pub const fn new(toplevel: Context, scope: Dict<Str, Context>) -> Self {
        Self {
            context: toplevel,
            scope,
        }
    }
}
