//! Defines `Context`.
//!
//! `Context` is used for type inference and type checking.
#![allow(clippy::result_unit_err)]
#![allow(clippy::assigning_clones)]
pub mod compare;
pub mod eval;
pub mod generalize;
pub mod hint;
pub mod initialize;
pub mod inquire;
pub mod instantiate;
pub mod instantiate_spec;
pub mod register;
pub mod test;
pub mod unify;

use std::fmt;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::option::Option; // conflicting to Type::Option
use std::path::Path;

use erg_common::config::ErgConfig;
use erg_common::consts::DEBUG_MODE;
use erg_common::consts::ERG_MODE;
use erg_common::consts::PYTHON_MODE;
use erg_common::dict::Dict;
use erg_common::error::Location;
use erg_common::impl_display_from_debug;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{fmt_option, fn_name, get_hash, log};

use ast::{DefId, DefKind, VarName};
use erg_parser::ast;
use erg_parser::ast::Def;
use erg_parser::ast::Signature;
use erg_parser::token::Token;

use crate::context::instantiate::TyVarCache;
use crate::context::instantiate_spec::ConstTemplate;
use crate::error::{TyCheckError, TyCheckErrors};
use crate::hir::Identifier;
use crate::module::SharedModuleGraph;
use crate::module::{
    SharedCompilerResource, SharedModuleCache, SharedModuleIndex, SharedPromises, SharedTraitImpls,
};
use crate::ty::value::ValueObj;
use crate::ty::GuardType;
use crate::ty::ParamTy;
use crate::ty::{Predicate, Type, Visibility, VisibilityModifier};
use crate::varinfo::{AbsLocation, Mutability, VarInfo, VarKind};
use Type::*;

/// For implementing LSP or other IDE features
pub trait ContextProvider {
    fn dir(&self) -> Dict<&VarName, &VarInfo>;
    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context>;
    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)>;
    fn has(&self, name: &str) -> bool {
        self.get_var_info(name).is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlKind {
    If,
    While,
    For,
    Match,
    Try,
    With,
    Discard,
    Assert,
}

impl fmt::Display for ControlKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::If => write!(f, "if"),
            Self::While => write!(f, "while"),
            Self::For => write!(f, "for"),
            Self::Match => write!(f, "match"),
            Self::Try => write!(f, "try"),
            Self::With => write!(f, "with"),
            Self::Discard => write!(f, "discard"),
            Self::Assert => write!(f, "assert"),
        }
    }
}

impl TryFrom<&str> for ControlKind {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "if" | "if!" => Ok(ControlKind::If),
            "while!" => Ok(ControlKind::While),
            "while" if PYTHON_MODE => Ok(ControlKind::While),
            "for" | "for!" => Ok(ControlKind::For),
            "match" | "match!" => Ok(ControlKind::Match),
            "try" | "try!" => Ok(ControlKind::Try),
            "with" | "with!" => Ok(ControlKind::With),
            "discard" => Ok(ControlKind::Discard),
            "assert" => Ok(ControlKind::Assert),
            _ => Err(()),
        }
    }
}

impl ControlKind {
    pub const fn is_if(&self) -> bool {
        matches!(self, Self::If)
    }
    /// if | if! | while!
    pub const fn is_conditional(&self) -> bool {
        matches!(self, Self::If | Self::While)
    }
    pub const fn makes_scope(&self) -> bool {
        !matches!(self, Self::Assert)
    }
}

#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub sub_type: Type,
    pub sup_trait: Type,
    pub declared_in: Option<NormalizedPathBuf>,
}

impl PartialEq for TraitImpl {
    fn eq(&self, other: &Self) -> bool {
        self.sub_type == other.sub_type && self.sup_trait == other.sup_trait
    }
}

impl Eq for TraitImpl {}

impl std::hash::Hash for TraitImpl {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sub_type.hash(state);
        self.sup_trait.hash(state);
    }
}

impl std::fmt::Display for TraitImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TraitImpl{{{} <: {}}}", self.sub_type, self.sup_trait)
    }
}

impl TraitImpl {
    pub const fn new(
        sub_type: Type,
        sup_trait: Type,
        declared_in: Option<NormalizedPathBuf>,
    ) -> Self {
        Self {
            sub_type,
            sup_trait,
            declared_in,
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

    pub fn get_class(&self) -> &Type {
        match self {
            ClassDefType::Simple(class) => class,
            ClassDefType::ImplTrait { class, .. } => class,
        }
    }

    pub fn get_impl_trait(&self) -> Option<&Type> {
        match self {
            ClassDefType::Simple(_) => None,
            ClassDefType::ImplTrait { impl_trait, .. } => Some(impl_trait),
        }
    }

    pub fn is_class_of(&self, t: &Type) -> bool {
        match self {
            ClassDefType::Simple(class) => class == t,
            ClassDefType::ImplTrait { class, .. } => class == t,
        }
    }

    pub fn is_impl_of(&self, trait_: &Type) -> bool {
        match self {
            ClassDefType::ImplTrait { impl_trait, .. } => impl_trait == trait_,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MethodContext {
    pub id: DefId,
    pub typ: ClassDefType,
    pub ctx: Context,
}

impl Deref for MethodContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl DerefMut for MethodContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

impl MethodContext {
    pub const fn new(id: DefId, typ: ClassDefType, ctx: Context) -> Self {
        Self { id, typ, ctx }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    pub typ: Type,
    pub ctx: Context,
}

impl Deref for TypeContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl DerefMut for TypeContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

impl TypeContext {
    pub const fn new(typ: Type, ctx: Context) -> Self {
        Self { typ, ctx }
    }

    /// contains both class attributes and instance attributes
    /// see also: `get_class_attr`
    pub(crate) fn get_class_member<'c>(
        &'c self,
        name: &VarName,
        ctx: &'c Context,
    ) -> Option<&'c VarInfo> {
        self.get_current_scope_attr(name).or_else(|| {
            let sups = ctx.get_nominal_super_type_ctxs(&self.typ)?;
            for sup in sups {
                if let Some(vi) = sup.get_current_scope_attr(name) {
                    return Some(vi);
                }
            }
            None
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultInfo {
    NonDefault, // var-args should be non-default
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
    /// Output(T), 共変, +
    Covariant,
    /// Input(T), 反変, -
    Contravariant,
    /// 不変, 0
    #[default]
    Invariant,
}

impl_display_from_debug!(Variance);

impl std::ops::Mul for Variance {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Variance::Covariant, Variance::Covariant)
            | (Variance::Contravariant, Variance::Contravariant) => Variance::Covariant,
            (Variance::Covariant, Variance::Contravariant)
            | (Variance::Contravariant, Variance::Covariant) => Variance::Contravariant,
            _ => Variance::Invariant,
        }
    }
}

impl Variance {
    pub const fn is_invariant(&self) -> bool {
        matches!(self, Variance::Invariant)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamSpec {
    pub(crate) name: Option<Str>,
    // TODO: `:` or `<:`
    pub(crate) t: Type,
    pub is_var_params: bool,
    pub default_info: DefaultInfo,
    loc: AbsLocation,
}

impl ParamSpec {
    pub fn new<S: Into<Str>>(
        name: Option<S>,
        t: Type,
        is_var_params: bool,
        default: DefaultInfo,
        loc: AbsLocation,
    ) -> Self {
        Self {
            name: name.map(|s| s.into()),
            t,
            is_var_params,
            default_info: default,
            loc,
        }
    }

    pub fn named<S: Into<Str>>(
        name: S,
        t: Type,
        is_var_params: bool,
        default: DefaultInfo,
    ) -> Self {
        Self::new(
            Some(name),
            t,
            is_var_params,
            default,
            AbsLocation::unknown(),
        )
    }

    pub fn named_nd<S: Into<Str>>(name: S, t: Type) -> Self {
        Self::new(
            Some(name),
            t,
            false,
            DefaultInfo::NonDefault,
            AbsLocation::unknown(),
        )
    }

    pub fn default<S: Into<Str>>(name: S, t: Type) -> Self {
        Self::new(
            Some(name),
            t,
            false,
            DefaultInfo::WithDefault,
            AbsLocation::unknown(),
        )
    }

    pub fn t<S: Into<Str>>(name: S, is_var_params: bool, default: DefaultInfo) -> Self {
        Self::new(
            Some(name),
            Type,
            is_var_params,
            default,
            AbsLocation::unknown(),
        )
    }

    pub fn t_nd<S: Into<Str>>(name: S) -> Self {
        Self::new(
            Some(name),
            Type,
            false,
            DefaultInfo::NonDefault,
            AbsLocation::unknown(),
        )
    }

    pub fn has_default(&self) -> bool {
        self.default_info.has_default()
    }
}

impl From<&ParamSpec> for ParamTy {
    fn from(param: &ParamSpec) -> Self {
        if let Some(name) = &param.name {
            ParamTy::kw(name.clone(), param.t.clone())
        } else {
            ParamTy::Pos(param.t.clone())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContextKind {
    LambdaFunc(Option<ControlKind>),
    LambdaProc(Option<ControlKind>),
    Func,
    Proc,
    Class,
    MethodDefs(Option<Type>), // Type: trait implemented
    PatchMethodDefs(Type),
    Trait,
    StructuralTrait,
    Patch(Type),
    StructuralPatch(Type),
    // use Box to reduce the size of enum
    GluePatch(Box<TraitImpl>), // TODO: deprecate (integrate into Patch)
    Module,
    Instant,
    Record,
    Dummy,
}

impl From<&Def> for ContextKind {
    fn from(def: &Def) -> Self {
        match def.def_kind() {
            DefKind::Class | DefKind::Inherit => Self::Class,
            DefKind::Trait | DefKind::Subsume => Self::Trait,
            DefKind::StructuralTrait => Self::StructuralTrait,
            DefKind::ErgImport | DefKind::PyImport | DefKind::RsImport | DefKind::InlineModule => {
                Self::Module
            }
            DefKind::Other => {
                if let Signature::Subr(subr) = &def.sig {
                    if subr.ident.is_procedural() {
                        Self::Proc
                    } else {
                        Self::Func
                    }
                } else {
                    Self::Instant
                }
            }
            // FIXME: Patch(Type),
            DefKind::Patch => Self::Instant,
        }
    }
}

impl fmt::Display for ContextKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LambdaFunc(kind) => write!(f, "LambdaFunc({})", fmt_option!(kind)),
            Self::LambdaProc(kind) => write!(f, "LambdaProc({})", fmt_option!(kind)),
            Self::Func => write!(f, "Func"),
            Self::Proc => write!(f, "Proc"),
            Self::Class => write!(f, "Class"),
            Self::MethodDefs(trait_) => write!(f, "MethodDefs({})", fmt_option!(trait_)),
            Self::PatchMethodDefs(type_) => write!(f, "PatchMethodDefs({type_})"),
            Self::Trait => write!(f, "Trait"),
            Self::StructuralTrait => write!(f, "StructuralTrait"),
            Self::Patch(type_) => write!(f, "Patch({type_})"),
            Self::StructuralPatch(type_) => write!(f, "StructuralPatch({type_})"),
            Self::GluePatch(type_) => write!(f, "GluePatch({type_})"),
            Self::Module => write!(f, "Module"),
            Self::Instant => write!(f, "Instant"),
            Self::Record => write!(f, "Record"),
            Self::Dummy => write!(f, "Dummy"),
        }
    }
}

impl ContextKind {
    pub const fn is_method_def(&self) -> bool {
        matches!(self, Self::MethodDefs(_))
    }

    pub const fn is_trait_impl(&self) -> bool {
        matches!(self, Self::MethodDefs(Some(_)))
    }

    pub const fn is_type(&self) -> bool {
        matches!(self, Self::Class | Self::Trait | Self::StructuralTrait)
    }

    pub const fn is_subr(&self) -> bool {
        matches!(
            self,
            Self::Func | Self::Proc | Self::LambdaFunc(_) | Self::LambdaProc(_)
        )
    }

    pub const fn is_class(&self) -> bool {
        matches!(self, Self::Class)
    }

    pub const fn is_trait(&self) -> bool {
        matches!(self, Self::Trait | Self::StructuralTrait)
    }

    pub const fn is_patch(&self) -> bool {
        matches!(self, Self::Patch(_) | Self::GluePatch(_))
    }

    pub const fn is_module(&self) -> bool {
        matches!(self, Self::Module)
    }

    pub const fn is_instant(&self) -> bool {
        matches!(self, Self::Instant)
    }

    pub const fn control_kind(&self) -> Option<ControlKind> {
        match self {
            Self::LambdaFunc(kind) | Self::LambdaProc(kind) => *kind,
            _ => None,
        }
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

impl RegistrationMode {
    pub const fn is_preregister(&self) -> bool {
        matches!(self, Self::PreRegister)
    }
    pub const fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodPair {
    pub definition_type: Type,
    pub method_info: VarInfo,
}

impl fmt::Display for MethodPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ def: {} info: {} }}",
            self.definition_type, self.method_info
        )
    }
}

impl MethodPair {
    pub const fn new(definition_type: Type, method_info: VarInfo) -> Self {
        Self {
            definition_type,
            method_info,
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
    pub(crate) methods_list: Vec<MethodContext>,
    // K: method name, V: types defines the method
    // If it is declared in a trait, it takes precedence over the class.
    pub(crate) method_to_traits: Dict<Str, Vec<MethodPair>>,
    pub(crate) method_to_classes: Dict<Str, Vec<MethodPair>>,
    /// K: method name, V: impl patch
    /// Provided methods can switch implementations on a scope-by-scope basis
    /// K: メソッド名, V: それを実装するパッチたち
    /// 提供メソッドはスコープごとに実装を切り替えることができる
    pub(crate) method_impl_patches: Dict<VarName, Vec<VarName>>,
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
    pub(crate) params_spec: Vec<ParamSpec>,
    pub(crate) locals: Dict<VarName, VarInfo>,
    pub(crate) consts: Dict<VarName, ValueObj>,
    // {"Nat": ctx, "Int": ctx, ...}
    pub(crate) mono_types: Dict<VarName, TypeContext>,
    // Implementation Contexts for Polymorphic Types
    // Vec<TyParam> are specialization parameters
    // e.g. {"List": [(List(Nat), ctx), (List(Int), ctx), (List(Str), ctx), (List(Obj), ctx), (List('T), ctx)], ...}
    pub(crate) poly_types: Dict<VarName, TypeContext>,
    // patches can be accessed like normal records
    // but when used as a fallback to a type, values are traversed instead of accessing by keys
    pub(crate) patches: Dict<VarName, Context>,
    pub(crate) shared: Option<SharedCompilerResource>,
    pub(crate) tv_cache: Option<TyVarCache>,
    pub(crate) higher_order_caller: Vec<Str>,
    pub(crate) guards: Vec<GuardType>,
    pub(crate) erg_to_py_names: Dict<Str, Str>,
    pub(crate) captured_names: Vec<Identifier>,
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
    fn dir(&self) -> Dict<&VarName, &VarInfo> {
        let mut vars = self.type_dir(self);
        if let Some(outer) = self.get_outer_scope() {
            vars.guaranteed_extend(outer.dir());
        } else if let Some(builtins) = self.get_builtins() {
            vars.guaranteed_extend(builtins.locals.iter());
        }
        vars
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.get_mod(receiver_name)
            .or_else(|| self.rec_local_get_type(receiver_name).map(|ctx| &ctx.ctx))
            .or_else(|| {
                let (_, vi) = self.get_var_info(receiver_name)?;
                self.get_nominal_type_ctx(&vi.t).map(|ctx| &ctx.ctx)
            })
    }

    // this is internally recursive
    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.get_var_kv(name).or_else(|| {
            self.get_builtins()
                .and_then(|builtin| builtin.get_var_kv(name))
        })
    }
}

impl Context {
    pub fn dir(&self) -> Dict<&VarName, &VarInfo> {
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
                    .flat_map(|t| self.get_nominal_type_ctx(t).map(|ctx| &ctx.ctx)),
            );
        }
        ctxs
    }

    pub fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        ContextProvider::get_var_info(self, name)
    }

    pub fn has(&self, name: &str) -> bool {
        ContextProvider::has(self, name)
    }

    /// NOTE: Actually, this can be used for searching variables, attributes, etc.
    pub fn get_type_info(&self, typ: &Type) -> Option<(&VarName, &VarInfo)> {
        let namespace = typ.namespace();
        let ctx = self.get_namespace(&namespace)?;
        ctx.get_var_info(&typ.local_name())
    }

    pub fn unregister(&mut self, name: &str) -> Option<VarInfo> {
        self.mono_types.remove(name);
        self.poly_types.remove(name);
        self.patches.remove(name);
        self.erg_to_py_names.remove(name);
        self.locals
            .remove(name)
            .or_else(|| self.locals.remove(name))
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
        shared: Option<SharedCompilerResource>,
        level: usize,
    ) -> Self {
        Self::with_capacity(name, cfg, kind, params, outer, shared, 0, level)
    }

    pub fn default_with_name(name: &'static str) -> Self {
        Self::new(
            name.into(),
            ErgConfig::default(),
            ContextKind::Dummy,
            vec![],
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
        shared: Option<SharedCompilerResource>,
        capacity: usize,
        level: usize,
    ) -> Self {
        let mut params_ = Vec::new();
        for param in params.clone().into_iter() {
            let id = DefId(get_hash(&(&name, &param)));
            if let Some(name) = param.name {
                let var_kind = VarKind::parameter(id, param.is_var_params, param.default_info);
                let muty = Mutability::from(&name[..]);
                let vi = VarInfo::new(
                    param.t,
                    muty,
                    Visibility::private(&name),
                    var_kind,
                    None,
                    kind.clone(),
                    None,
                    param.loc,
                );
                params_.push((Some(VarName::new(Token::symbol(&name))), vi));
            } else {
                let var_kind = VarKind::parameter(id, param.is_var_params, param.default_info);
                let muty = Mutability::Immutable;
                let vi = VarInfo::new(
                    param.t,
                    muty,
                    Visibility::private(name.clone()),
                    var_kind,
                    None,
                    kind.clone(),
                    None,
                    param.loc,
                );
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
            params: params_,
            params_spec: params,
            decls: Dict::default(),
            future_defined_locals: Dict::default(),
            deleted_locals: Dict::default(),
            locals: Dict::with_capacity(capacity),
            consts: Dict::default(),
            mono_types: Dict::default(),
            poly_types: Dict::default(),
            shared,
            tv_cache: None,
            patches: Dict::default(),
            higher_order_caller: vec![],
            guards: vec![],
            erg_to_py_names: Dict::default(),
            captured_names: vec![],
            level,
        }
    }

    #[inline]
    pub fn mono(
        name: Str,
        cfg: ErgConfig,
        kind: ContextKind,
        outer: Option<Context>,
        shared: Option<SharedCompilerResource>,
        level: usize,
    ) -> Self {
        Self::new(name, cfg, kind, vec![], outer, shared, level)
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn poly(
        name: Str,
        cfg: ErgConfig,
        kind: ContextKind,
        params: Vec<ParamSpec>,
        outer: Option<Context>,
        shared: Option<SharedCompilerResource>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::with_capacity(name, cfg, kind, params, outer, shared, capacity, level)
    }

    pub fn poly_trait<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        cfg: ErgConfig,
        shared: Option<SharedCompilerResource>,
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
            shared,
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
            capacity,
            Self::TOP_LEVEL,
        )
    }

    pub fn poly_class<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        cfg: ErgConfig,
        shared: Option<SharedCompilerResource>,
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
            shared,
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
        shared: Option<SharedCompilerResource>,
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
            shared,
            capacity,
            level,
        )
    }

    #[inline]
    pub fn mono_trait<S: Into<Str>>(
        name: S,
        cfg: ErgConfig,
        shared: Option<SharedCompilerResource>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::poly_trait(name, vec![], cfg, shared, capacity, level)
    }

    #[inline]
    pub fn builtin_mono_trait<S: Into<Str>>(name: S, capacity: usize) -> Self {
        Self::mono_trait(name, ErgConfig::default(), None, capacity, Self::TOP_LEVEL)
    }

    #[inline]
    pub fn mono_class<S: Into<Str>>(
        name: S,
        cfg: ErgConfig,
        shared: Option<SharedCompilerResource>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::poly_class(name, vec![], cfg, shared, capacity, level)
    }

    #[inline]
    pub fn builtin_mono_class<S: Into<Str>>(name: S, capacity: usize) -> Self {
        Self::mono_class(name, ErgConfig::default(), None, capacity, Self::TOP_LEVEL)
    }

    #[inline]
    pub fn mono_patch<S: Into<Str>>(
        name: S,
        base: Type,
        cfg: ErgConfig,
        shared: Option<SharedCompilerResource>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::poly_patch(name, base, vec![], cfg, shared, capacity, level)
    }

    #[inline]
    pub fn methods(
        impl_trait: Option<Type>,
        cfg: ErgConfig,
        shared: Option<SharedCompilerResource>,
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
            shared,
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
        shared: Option<SharedCompilerResource>,
        capacity: usize,
        level: usize,
    ) -> Self {
        Self::poly(
            name.into(),
            cfg,
            ContextKind::GluePatch(Box::new(TraitImpl::new(base, impls, None))),
            params,
            None,
            shared,
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
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[inline]
    pub fn builtin_patch<S: Into<Str>>(
        name: S,
        impls: Type,
        params: Vec<ParamSpec>,
        capacity: usize,
    ) -> Self {
        Self::poly(
            name.into(),
            ErgConfig::default(),
            ContextKind::Patch(impls),
            params,
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
        shared: Option<SharedCompilerResource>,
        capacity: usize,
    ) -> Self {
        Self::with_capacity(
            name,
            cfg,
            ContextKind::Module,
            vec![],
            None,
            shared,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[inline]
    pub fn builtin_module<S: Into<Str>>(
        name: S,
        cfg: ErgConfig,
        shared: SharedCompilerResource,
        capacity: usize,
    ) -> Self {
        Self::module(name.into(), cfg, Some(shared), capacity)
    }

    #[inline]
    pub fn instant(
        name: Str,
        cfg: ErgConfig,
        capacity: usize,
        shared: Option<SharedCompilerResource>,
        outer: Context,
    ) -> Self {
        Self::with_capacity(
            name,
            cfg,
            ContextKind::Instant,
            vec![],
            Some(outer),
            shared,
            capacity,
            Self::TOP_LEVEL,
        )
    }

    pub(crate) fn module_path(&self) -> &Path {
        self.cfg.input.path()
    }

    pub(crate) fn absolutize(&self, loc: Location) -> AbsLocation {
        AbsLocation::new(Some(NormalizedPathBuf::from(self.module_path())), loc)
    }

    #[inline]
    pub fn caused_by(&self) -> String {
        String::from(&self.name[..])
    }

    /// use `get_outer_scope` for getting the outer scope
    pub(crate) fn get_outer(&self) -> Option<&Context> {
        self.outer.as_ref().map(|x| x.as_ref())
    }

    /// If both `self` and `outer` are modules, returns `None` because the outer namespace is different from the current context
    pub(crate) fn get_outer_scope(&self) -> Option<&Context> {
        let outer = self.get_outer()?;
        if outer.name != "<builtins>" && outer.kind.is_module() && self.kind.is_module() {
            None
        } else {
            Some(outer)
        }
    }

    pub(crate) fn get_outer_scope_or_builtins(&self) -> Option<&Context> {
        self.get_outer_scope().or_else(|| self.get_builtins())
    }

    pub(crate) fn get_mut_outer(&mut self) -> Option<&mut Context> {
        self.outer.as_mut().map(|x| x.as_mut())
    }

    pub(crate) fn impl_of(&self) -> Option<Type> {
        if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
            Some(tr.clone())
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        let default_locals = if ERG_MODE { 4 } else { 3 };
        self.super_classes.is_empty()
            && self.super_traits.is_empty()
            && self.methods_list.is_empty()
            && self.const_param_defaults.is_empty()
            && self.method_to_traits.is_empty()
            && self.method_to_classes.is_empty()
            && self.method_impl_patches.is_empty()
            && self.params.is_empty()
            && self.params_spec.is_empty()
            && self.decls.is_empty()
            && self.future_defined_locals.is_empty()
            && self.deleted_locals.is_empty()
            && self.consts.is_empty()
            && self.mono_types.is_empty()
            && self.poly_types.is_empty()
            && self.patches.is_empty()
            && self.locals.len() <= default_locals
    }

    pub(crate) fn path(&self) -> Str {
        // NOTE: maybe this need to be changed if we want to support nested classes/traits
        if self.kind == ContextKind::Module {
            self.name.replace(".__init__", "").into()
        } else if let Some(outer) = self.get_outer() {
            outer.path()
        } else {
            self.name.replace(".__init__", "").into()
        }
    }

    /// Returns None if self is `<builtins>`.
    /// This avoids infinite loops.
    pub(crate) fn get_builtins(&self) -> Option<&Context> {
        // builtins中で定義した型等はmod_cacheがNoneになっている
        if &self.path()[..] != "<builtins>" {
            self.shared
                .as_ref()
                .map(|shared| shared.mod_cache.raw_ref_builtins_ctx().unwrap())
                .map(|mod_ctx| &mod_ctx.context)
        } else {
            None
        }
    }

    pub(crate) fn get_module(&self) -> Option<&Context> {
        self.get_outer()
            .and_then(|outer| {
                if outer.kind == ContextKind::Module {
                    Some(outer)
                } else {
                    outer.get_module()
                }
            })
            .or_else(|| {
                if self.kind == ContextKind::Module {
                    Some(self)
                } else {
                    None
                }
            })
    }

    pub(crate) fn get_module_from_stack(&self, path: &NormalizedPathBuf) -> Option<&Context> {
        if self.kind == ContextKind::Module && &NormalizedPathBuf::from(self.module_path()) == path
        {
            return Some(self);
        }
        self.get_outer()
            .and_then(|outer| outer.get_module_from_stack(path))
    }

    /// This method is intended to be called __only__ in the top-level module.
    /// `.cfg` is not initialized and is used around.
    pub fn initialize(&mut self) {
        let mut shared = mem::take(&mut self.shared);
        if let Some(mod_cache) = shared.as_mut().map(|s| &mut s.mod_cache) {
            mod_cache.initialize();
        }
        if let Some(py_mod_cache) = shared.as_mut().map(|s| &mut s.py_mod_cache) {
            py_mod_cache.initialize();
        }
        *self = Self::new(
            self.name.clone(),
            self.cfg.clone(),
            self.kind.clone(),
            vec![],
            None,
            shared,
            self.level,
        );
    }

    pub(crate) fn grow(
        &mut self,
        name: &str,
        kind: ContextKind,
        vis: VisibilityModifier,
        tv_cache: Option<TyVarCache>,
    ) {
        let name = if kind.is_module() {
            name.into()
        } else if vis.is_public() {
            format!("{parent}.{name}", parent = self.name)
        } else {
            format!("{parent}::{name}", parent = self.name)
        };
        log!(info "{}: current namespace: {name}", fn_name!());
        self.outer = Some(Box::new(mem::take(self)));
        // self.level += 1;
        self.cfg = self.get_outer().unwrap().cfg.clone();
        self.shared = self.get_outer().unwrap().shared.clone();
        self.higher_order_caller = self.get_outer().unwrap().higher_order_caller.clone();
        self.tv_cache = tv_cache;
        self.name = name.into();
        self.kind = kind;
    }

    pub(crate) fn replace(&mut self, new: Self) {
        let old = mem::take(self);
        *self = new;
        self.outer = Some(Box::new(old));
        self.cfg = self.get_outer().unwrap().cfg.clone();
        self.shared = self.get_outer().unwrap().shared.clone();
        self.higher_order_caller = self.get_outer().unwrap().higher_order_caller.clone();
    }

    pub(crate) fn clear_invalid_vars(&mut self) {
        self.locals.retain(|_, v| v.t != Failure);
        self.decls.retain(|_, v| v.t != Failure);
        self.params.retain(|(_, v)| v.t != Failure);
    }

    /// Note that the popped context is detached and `outer == None`.
    pub fn pop(&mut self) -> Context {
        self.check_types();
        if let Some(parent) = self.outer.take() {
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
        self.check_types();
        if self.outer.is_some() {
            log!(err "not in the top-level context");
            if self.kind.is_module() {
                Some(self.pop())
            } else {
                None
            }
        } else {
            log!(info "{}: current namespace: <builtins>", fn_name!());
            // toplevel
            Some(mem::take(self))
        }
    }

    pub(crate) fn check_decls_and_pop(&mut self) -> (Context, TyCheckErrors) {
        match self.check_decls() {
            Ok(_) => (self.pop(), TyCheckErrors::empty()),
            Err(errs) => (self.pop(), errs),
        }
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

    /// enumerates all the variables/methods in the current context & super contexts.
    pub(crate) fn type_dir<'t>(&'t self, namespace: &'t Context) -> Dict<&VarName, &VarInfo> {
        let mut attrs = self.locals.iter().collect::<Dict<_, _>>();
        attrs.guaranteed_extend(
            self.params
                .iter()
                .filter_map(|(k, v)| k.as_ref().map(|k| (k, v))),
        );
        attrs.guaranteed_extend(self.decls.iter());
        attrs.guaranteed_extend(
            self.methods_list
                .iter()
                .flat_map(|ctx| ctx.type_dir(namespace)),
        );
        if let Some(tv_cache) = &self.tv_cache {
            attrs.guaranteed_extend(tv_cache.var_infos.iter());
        }
        for sup in self.super_classes.iter() {
            if let Some(sup_ctx) = namespace.get_nominal_type_ctx(sup) {
                if sup_ctx.name == self.name {
                    continue;
                }
                attrs.guaranteed_extend(sup_ctx.type_dir(namespace));
            }
        }
        for sup in self.super_traits.iter() {
            if let Some(sup_ctx) = namespace.get_nominal_type_ctx(sup) {
                if sup_ctx.name == self.name {
                    continue;
                }
                attrs.guaranteed_extend(sup_ctx.type_impl_dir(namespace));
            }
        }
        attrs
    }

    fn type_impl_dir<'t>(&'t self, namespace: &'t Context) -> Dict<&VarName, &VarInfo> {
        let mut attrs = self.locals.iter().collect::<Dict<_, _>>();
        attrs.guaranteed_extend(
            self.methods_list
                .iter()
                .flat_map(|ctx| ctx.type_impl_dir(namespace)),
        );
        for sup in self.super_classes.iter().chain(self.super_traits.iter()) {
            if let Some(sup_ctx) = namespace.get_nominal_type_ctx(sup) {
                if sup_ctx.name == self.name {
                    continue;
                }
                attrs.guaranteed_extend(sup_ctx.type_impl_dir(namespace));
            }
        }
        attrs
    }

    pub fn local_dir(&self) -> Dict<&VarName, &VarInfo> {
        self.type_dir(self)
    }

    pub(crate) fn opt_mod_cache(&self) -> Option<&SharedModuleCache> {
        self.shared.as_ref().map(|s| &s.mod_cache)
    }

    pub(crate) fn mod_cache(&self) -> &SharedModuleCache {
        &self.shared().mod_cache
    }

    pub(crate) fn opt_py_mod_cache(&self) -> Option<&SharedModuleCache> {
        self.shared.as_ref().map(|s| &s.py_mod_cache)
    }

    pub(crate) fn py_mod_cache(&self) -> &SharedModuleCache {
        &self.shared().py_mod_cache
    }

    pub(crate) fn opt_index(&self) -> Option<&SharedModuleIndex> {
        self.shared.as_ref().map(|s| &s.index)
    }

    pub fn index(&self) -> &SharedModuleIndex {
        &self.shared().index
    }

    pub fn graph(&self) -> &SharedModuleGraph {
        &self.shared().graph
    }

    pub fn trait_impls(&self) -> &SharedTraitImpls {
        &self.shared().trait_impls
    }

    pub fn shared(&self) -> &SharedCompilerResource {
        self.shared.as_ref().unwrap()
    }

    pub fn promises(&self) -> &SharedPromises {
        &self.shared().promises
    }

    pub fn current_control_flow(&self) -> Option<ControlKind> {
        self.higher_order_caller
            .last()
            .and_then(|name| ControlKind::try_from(&name[..]).ok())
    }

    pub fn control_kind(&self) -> Option<ControlKind> {
        for caller in self.higher_order_caller.iter().rev() {
            if let Ok(control) = ControlKind::try_from(&caller[..]) {
                return Some(control);
            }
        }
        None
    }

    /// Context of the function that actually creates the scope.
    /// Control flow function blocks do not create actual scopes.
    pub fn current_true_function_ctx(&self) -> Option<&Context> {
        if self.kind.is_subr() && self.current_control_flow().is_none() {
            Some(self)
        } else if let Some(outer) = self.get_outer_scope() {
            outer.current_true_function_ctx()
        } else {
            None
        }
    }

    pub(crate) fn check_types(&self) {
        if DEBUG_MODE {
            for (_, ctx) in self.poly_types.iter() {
                if ctx.typ.has_undoable_linked_var() {
                    panic!("{} has undoable linked vars", ctx.typ);
                }
                ctx.check_types();
            }
            for methods in self.methods_list.iter() {
                if methods.typ.get_class().has_undoable_linked_var() {
                    panic!("{} has undoable linked vars", methods.typ);
                }
                if methods
                    .typ
                    .get_impl_trait()
                    .is_some_and(|t| t.has_undoable_linked_var())
                {
                    panic!("{} has undoable linked vars", methods.typ);
                }
                methods.check_types();
            }
            if let Some(outer) = self.get_outer() {
                outer.check_types();
            }
        }
    }

    pub(crate) fn get_tv_ctx(&self, ident: &ast::Identifier, args: &ast::Args) -> TyVarCache {
        let mut tv_ctx = TyVarCache::new(self.level, self);
        if let Some(ctx) = self.get_type_ctx(ident.inspect()) {
            let arg_ts = ctx.params.iter().map(|(_, vi)| &vi.t);
            for ((tp, arg), arg_t) in ctx.typ.typarams().iter().zip(args.pos_args()).zip(arg_ts) {
                let tp = self.detach_tp(tp.clone(), &mut tv_ctx);
                if let ast::Expr::Accessor(ast::Accessor::Ident(ident)) = &arg.expr {
                    if self.subtype_of(arg_t, &Type::Type) {
                        if let Ok(tv) = self.convert_tp_into_type(tp.clone()) {
                            let _ = tv_ctx.push_or_init_tyvar(&ident.name, &tv, self);
                            continue;
                        }
                    }
                    let _ = tv_ctx.push_or_init_typaram(&ident.name, &tp, self);
                }
            }
        }
        tv_ctx
    }
}

#[derive(Debug, Clone, Default)]
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

    pub fn get_top_cfg(&self) -> ErgConfig {
        self.context.cfg.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.context.is_empty()
    }
}
