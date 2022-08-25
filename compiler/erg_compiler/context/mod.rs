//! Defines `Context`.
//! `Context` is used for type inference and type checking.
pub mod compare;
pub mod instantiate;
pub mod test;
pub mod tyvar;

use std::cmp::Ordering;
use std::fmt;
use std::mem;
use std::option::Option; // conflicting to Type::Option

use erg_common::color::{GREEN, RED};
use erg_common::dict::Dict;
use erg_common::error::{ErrorCore, Location};
use erg_common::impl_display_from_debug;
use erg_common::levenshtein::levenshtein;
use erg_common::set::Set;
use erg_common::traits::{HasType, Locational, Stream};
use erg_common::ty::{
    Constraint, HasLevel, ParamTy, Predicate, SubrKind, SubrType, TyBound, TyParam, Type,
};
use erg_common::value::{Field, ValueObj, Visibility};
use erg_common::Str;
use erg_common::{enum_unwrap, fmt_option, fmt_slice, fn_name, get_hash, log, set};
use Type::*;

use ast::{DefId, VarName};
use erg_parser::ast;
use erg_parser::token::Token;

use crate::context::instantiate::{ConstTemplate, TyVarContext};
use crate::error::readable_name;
use crate::error::{binop_to_dname, unaryop_to_dname, TyCheckError, TyCheckErrors, TyCheckResult};
use crate::eval::Evaluator;
use crate::hir;
use crate::varinfo::{Mutability, ParamIdx, VarInfo, VarKind};
use Mutability::*;
use Visibility::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitInstancePair {
    pub sub_type: Type,
    pub sup_trait: Type,
}

impl std::fmt::Display for TraitInstancePair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TraitInstancePair{{{} <: {}}}",
            self.sub_type, self.sup_trait
        )
    }
}

impl TraitInstancePair {
    pub const fn new(sub_type: Type, sup_trait: Type) -> Self {
        TraitInstancePair {
            sub_type,
            sup_trait,
        }
    }
}

/// ```
/// # use erg_common::ty::{Type, TyParam};
/// # use erg_compiler::context::TyParamIdx;
///
/// let r = Type::mono_q("R");
/// let o = Type::mono_q("O");
/// let search_from = Type::poly("Add", vec![TyParam::t(r.clone()), TyParam::t(o.clone())]);
/// assert_eq!(TyParamIdx::search(&search_from, &o), Some(TyParamIdx::Nth(1)));
/// let i = Type::mono_q("I");
/// let f = Type::poly("F", vec![TyParam::t(o.clone()), TyParam::t(i.clone())]);
/// let search_from = Type::poly("Add", vec![TyParam::t(r), TyParam::t(f)]);
/// assert_eq!(TyParamIdx::search(&search_from, &o), Some(TyParamIdx::nested(1, TyParamIdx::Nth(0))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyParamIdx {
    Nth(usize),
    Nested { idx: usize, inner: Box<TyParamIdx> },
}

impl TyParamIdx {
    pub fn search(search_from: &Type, target: &Type) -> Option<Self> {
        match search_from {
            Type::Poly { params, .. } => {
                for (i, tp) in params.iter().enumerate() {
                    match tp {
                        TyParam::Type(t) if t.as_ref() == target => return Some(Self::Nth(i)),
                        TyParam::Type(t) if t.is_monomorphic() => {}
                        TyParam::Type(inner) => {
                            if let Some(inner) = Self::search(&inner, target) {
                                return Some(Self::nested(i, inner));
                            }
                        }
                        other => todo!("{other:?}"),
                    }
                }
                None
            }
            _ => todo!(),
        }
    }

    /// ```erg
    /// Nested(Nth(1), 0).select(F(X, G(Y, Z))) == Y
    /// ```
    pub fn select(self, from: &Type) -> Type {
        match self {
            Self::Nth(n) => {
                let tps = from.typarams();
                let tp = tps.iter().nth(n).unwrap();
                match tp {
                    TyParam::Type(t) => *t.clone(),
                    _ => todo!(),
                }
            }
            Self::Nested { .. } => todo!(),
        }
    }

    pub fn nested(idx: usize, inner: Self) -> Self {
        Self::Nested {
            idx,
            inner: Box::new(inner),
        }
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
    pub(crate) name: Option<&'static str>, // TODO: nested
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextKind {
    Func,
    Proc,
    Tuple,
    Record,
    Class,
    Trait,
    StructuralTrait,
    Patch,
    StructuralPatch,
    Module,
    Instant,
    Dummy,
}

/// 記号表に登録されているモードを表す
/// Preregister: サブルーチンまたは定数式、前方参照できる
/// Normal: 前方参照できない
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationMode {
    PreRegister,
    Normal,
}

use RegistrationMode::*;

/// Represents the context of the current scope
///
/// Recursive functions/methods are highlighted with the prefix `rec_`, as performance may be significantly degraded.
#[derive(Debug)]
pub struct Context {
    pub(crate) name: Str,
    pub(crate) kind: ContextKind,
    // Type bounds & Predicates (if the context kind is Subroutine)
    // ユーザー定義APIでのみ使う
    pub(crate) bounds: Vec<TyBound>,
    pub(crate) preds: Vec<Predicate>,
    /// for looking up the parent scope
    pub(crate) outer: Option<Box<Context>>,
    // e.g. { "Add": [ConstObjTemplate::App("Self", vec![])])
    pub(crate) const_param_defaults: Dict<Str, Vec<ConstTemplate>>,
    // Superclasses/supertraits by a patch are not included here
    // patchによってsuper class/traitになったものはここに含まれない
    pub(crate) super_classes: Vec<Type>, // if self is a patch, means patch classes
    pub(crate) super_traits: Vec<Type>,  // if self is not a trait, means implemented traits
    /// K: method name, V: impl patch
    /// Provided methods can switch implementations on a scope-by-scope basis
    /// K: メソッド名, V: それを実装するパッチたち
    /// 提供メソッドはスコープごとに実装を切り替えることができる
    pub(crate) method_impl_patches: Dict<VarName, Vec<VarName>>,
    /// K: name of a trait, V: (type, monomorphised trait that the type implements)
    /// K: トレイトの名前, V: (型, その型が実装する単相化トレイト)
    /// e.g. { "Named": [(Type, Named), (Func, Named), ...], "Add": [(Nat, Add(Nat)), (Int, Add(Int)), ...], ... }
    pub(crate) trait_impls: Dict<Str, Vec<TraitInstancePair>>,
    /// .0: glue patch, .1: type as subtype, .2: trait as supertype
    /// .0: 関係付けるパッチ(glue patch), .1: サブタイプになる型, .2: スーパータイプになるトレイト
    /// 一つの型ペアを接着パッチは同時に一つまでしか存在しないが、付け替えは可能
    pub(crate) glue_patch_and_types: Vec<(VarName, TraitInstancePair)>,
    /// stores declared names (not initialized)
    pub(crate) decls: Dict<VarName, VarInfo>,
    // stores defined names
    // 型の一致はHashMapでは判定できないため、keyはVarNameとして1つずつ見ていく
    /// ```erg
    /// f [x, y], z = ...
    /// ```
    /// => params: vec![(None, [T; 2]), (Some("z"), U)]
    /// => locals: {"x": T, "y": T}
    pub(crate) params: Vec<(Option<VarName>, VarInfo)>,
    pub(crate) locals: Dict<VarName, VarInfo>,
    pub(crate) consts: Dict<VarName, ValueObj>,
    pub(crate) eval: Evaluator,
    // stores user-defined type context
    pub(crate) types: Dict<Type, Context>,
    pub(crate) patches: Dict<VarName, Context>,
    pub(crate) mods: Dict<VarName, Context>,
    pub(crate) _nlocals: usize, // necessary for CodeObj.nlocals
    pub(crate) level: usize,
}

impl Default for Context {
    #[inline]
    fn default() -> Self {
        Self::new(
            "<dummy>".into(),
            ContextKind::Dummy,
            vec![],
            None,
            vec![],
            vec![],
            Self::TOP_LEVEL,
        )
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("name", &self.name)
            .field("bounds", &self.bounds)
            .field("preds", &self.preds)
            .field("params", &self.params)
            .field("decls", &self.decls)
            .field("locals", &self.params)
            .field("consts", &self.consts)
            .field("eval", &self.eval)
            .field("types", &self.types)
            .field("patches", &self.patches)
            .field("mods", &self.mods)
            .finish()
    }
}

impl Context {
    #[inline]
    pub fn new(
        name: Str,
        kind: ContextKind,
        params: Vec<ParamSpec>,
        outer: Option<Context>,
        super_classes: Vec<Type>,
        super_traits: Vec<Type>,
        level: usize,
    ) -> Self {
        Self::with_capacity(
            name,
            kind,
            params,
            outer,
            super_classes,
            super_traits,
            0,
            level,
        )
    }

    pub fn with_capacity(
        name: Str,
        kind: ContextKind,
        params: Vec<ParamSpec>,
        outer: Option<Context>,
        super_classes: Vec<Type>,
        super_traits: Vec<Type>,
        capacity: usize,
        level: usize,
    ) -> Self {
        let mut params_ = Vec::new();
        for (idx, param) in params.into_iter().enumerate() {
            let id = DefId(get_hash(&(&name, &param)));
            if let Some(name) = param.name {
                let idx = ParamIdx::Nth(idx);
                let kind = VarKind::parameter(id, idx, param.default_info);
                // TODO: is_const { Const } else { Immutable }
                let vi = VarInfo::new(param.t, Immutable, Private, kind);
                params_.push((Some(VarName::new(Token::static_symbol(name))), vi));
            } else {
                let idx = ParamIdx::Nth(idx);
                let kind = VarKind::parameter(id, idx, param.default_info);
                let vi = VarInfo::new(param.t, Immutable, Private, kind);
                params_.push((None, vi));
            }
        }
        Self {
            name,
            kind,
            bounds: vec![],
            preds: vec![],
            outer: outer.map(Box::new),
            super_classes,
            super_traits,
            const_param_defaults: Dict::default(),
            method_impl_patches: Dict::default(),
            trait_impls: Dict::default(),
            glue_patch_and_types: Vec::default(),
            params: params_,
            decls: Dict::default(),
            locals: Dict::with_capacity(capacity),
            consts: Dict::default(),
            eval: Evaluator::default(),
            types: Dict::default(),
            mods: Dict::default(),
            patches: Dict::default(),
            _nlocals: 0,
            level,
        }
    }

    #[inline]
    pub fn mono(
        name: Str,
        kind: ContextKind,
        outer: Option<Context>,
        super_classes: Vec<Type>,
        super_traits: Vec<Type>,
        level: usize,
    ) -> Self {
        Self::with_capacity(
            name,
            kind,
            vec![],
            outer,
            super_classes,
            super_traits,
            0,
            level,
        )
    }

    #[inline]
    pub fn poly(
        name: Str,
        kind: ContextKind,
        params: Vec<ParamSpec>,
        outer: Option<Context>,
        super_classes: Vec<Type>,
        super_traits: Vec<Type>,
        level: usize,
    ) -> Self {
        Self::with_capacity(
            name,
            kind,
            params,
            outer,
            super_classes,
            super_traits,
            0,
            level,
        )
    }

    pub fn poly_trait<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        supers: Vec<Type>,
        level: usize,
    ) -> Self {
        let name = name.into();
        Self::poly(
            name,
            ContextKind::Trait,
            params,
            None,
            vec![],
            supers,
            level,
        )
    }

    pub fn poly_class<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        super_classes: Vec<Type>,
        impl_traits: Vec<Type>,
        level: usize,
    ) -> Self {
        let name = name.into();
        Self::poly(
            name,
            ContextKind::Class,
            params,
            None,
            super_classes,
            impl_traits,
            level,
        )
    }

    #[inline]
    pub fn mono_trait<S: Into<Str>>(name: S, supers: Vec<Type>, level: usize) -> Self {
        Self::poly_trait(name, vec![], supers, level)
    }

    #[inline]
    pub fn mono_class<S: Into<Str>>(
        name: S,
        super_classes: Vec<Type>,
        super_traits: Vec<Type>,
        level: usize,
    ) -> Self {
        Self::poly_class(name, vec![], super_classes, super_traits, level)
    }

    #[inline]
    pub fn poly_patch<S: Into<Str>>(
        name: S,
        params: Vec<ParamSpec>,
        patch_classes: Vec<Type>,
        impl_traits: Vec<Type>,
        level: usize,
    ) -> Self {
        Self::poly(
            name.into(),
            ContextKind::Trait,
            params,
            None,
            patch_classes,
            impl_traits,
            level,
        )
    }

    #[inline]
    pub fn module(name: Str, capacity: usize) -> Self {
        Self::with_capacity(
            name,
            ContextKind::Module,
            vec![],
            None,
            vec![],
            vec![],
            capacity,
            Self::TOP_LEVEL,
        )
    }

    #[inline]
    pub fn caused_by(&self) -> Str {
        self.name.clone()
    }

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

    pub(crate) fn grow(
        &mut self,
        name: &str,
        kind: ContextKind,
        vis: Visibility,
    ) -> TyCheckResult<()> {
        let name = if vis.is_public() {
            format!("{parent}.{name}", parent = self.name)
        } else {
            format!("{parent}::{name}", parent = self.name)
        };
        log!("{}: current namespace: {name}", fn_name!());
        self.outer = Some(Box::new(mem::take(self)));
        self.name = name.into();
        self.kind = kind;
        Ok(())
    }

    pub(crate) fn pop(&mut self) -> Result<(), TyCheckErrors> {
        let mut uninited_errs = TyCheckErrors::empty();
        for (name, vi) in self.decls.iter() {
            uninited_errs.push(TyCheckError::uninitialized_error(
                line!() as usize,
                name.loc(),
                self.caused_by(),
                name.inspect(),
                &vi.t,
            ));
        }
        if let Some(parent) = &mut self.outer {
            *self = mem::take(parent);
            log!("{}: current namespace: {}", fn_name!(), self.name);
            if !uninited_errs.is_empty() {
                Err(uninited_errs)
            } else {
                Ok(())
            }
        } else {
            Err(TyCheckErrors::from(TyCheckError::checker_bug(
                0,
                Location::Unknown,
                fn_name!(),
                line!(),
            )))
        }
    }
}

// setters
impl Context {
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

// (type) getters & validators
impl Context {
    fn validate_var_sig_t(
        &self,
        sig: &ast::VarSignature,
        body_t: &Type,
        mode: RegistrationMode,
    ) -> TyCheckResult<()> {
        let spec_t = self.instantiate_var_sig_t(sig, None, mode)?;
        match &sig.pat {
            ast::VarPattern::Discard(token) => {
                if self
                    .sub_unify(body_t, &spec_t, None, Some(sig.loc()))
                    .is_err()
                {
                    return Err(TyCheckError::type_mismatch_error(
                        line!() as usize,
                        token.loc(),
                        self.caused_by(),
                        "_",
                        &spec_t,
                        body_t,
                    ));
                }
            }
            ast::VarPattern::Ident(ident) => {
                if self
                    .sub_unify(body_t, &spec_t, None, Some(sig.loc()))
                    .is_err()
                {
                    return Err(TyCheckError::type_mismatch_error(
                        line!() as usize,
                        ident.loc(),
                        self.caused_by(),
                        ident.inspect(),
                        &spec_t,
                        body_t,
                    ));
                }
            }
            ast::VarPattern::Array(a) => {
                for (elem, inf_elem_t) in a.iter().zip(body_t.inner_ts().iter()) {
                    self.validate_var_sig_t(elem, inf_elem_t, mode)?;
                }
            }
            _ => todo!(),
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
    }

    fn get_context(
        &self,
        obj: &hir::Expr,
        kind: Option<ContextKind>,
        namespace: &Str,
    ) -> TyCheckResult<&Context> {
        match obj {
            hir::Expr::Accessor(hir::Accessor::Local(name)) => {
                if kind == Some(ContextKind::Module) {
                    if let Some(ctx) = self.rec_get_mod(name.inspect()) {
                        Ok(ctx)
                    } else {
                        Err(TyCheckError::no_var_error(
                            line!() as usize,
                            obj.loc(),
                            namespace.clone(),
                            name.inspect(),
                            self.get_similar_name(name.inspect()),
                        ))
                    }
                } else {
                    todo!()
                }
            }
            _ => todo!(),
        }
    }

    fn get_match_call_t(
        &self,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<Type> {
        if !kw_args.is_empty() {
            todo!()
        }
        for pos_arg in pos_args.iter().skip(1) {
            let t = pos_arg.expr.ref_t();
            // Allow only anonymous functions to be passed as match arguments (for aesthetic reasons)
            if !matches!(&pos_arg.expr, hir::Expr::Lambda(_)) {
                return Err(TyCheckError::type_mismatch_error(
                    line!() as usize,
                    pos_arg.loc(),
                    self.caused_by(),
                    "match",
                    &Type::mono("LambdaFunc"),
                    t,
                ));
            }
        }
        let match_target_expr_t = pos_args[0].expr.ref_t();
        // Never or T => T
        let mut union_pat_t = Type::Never;
        for (i, pos_arg) in pos_args.iter().skip(1).enumerate() {
            let lambda = erg_common::enum_unwrap!(&pos_arg.expr, hir::Expr::Lambda);
            if !lambda.params.defaults.is_empty() {
                todo!()
            }
            // TODO: If the first argument of the match is a tuple?
            if lambda.params.len() != 1 {
                return Err(TyCheckError::argument_error(
                    line!() as usize,
                    pos_args[i + 1].loc(),
                    self.caused_by(),
                    1,
                    pos_args[i + 1].expr.signature_t().unwrap().typarams_len(),
                ));
            }
            let rhs = self.instantiate_param_sig_t(&lambda.params.non_defaults[0], None, Normal)?;
            union_pat_t = self.rec_union(&union_pat_t, &rhs);
        }
        // NG: expr_t: Nat, union_pat_t: {1, 2}
        // OK: expr_t: Int, union_pat_t: {1} or 'T
        if self
            .sub_unify(match_target_expr_t, &union_pat_t, None, None)
            .is_err()
        {
            return Err(TyCheckError::match_error(
                line!() as usize,
                pos_args[0].loc(),
                self.caused_by(),
                match_target_expr_t,
            ));
        }
        let branch_ts = pos_args
            .iter()
            .skip(1)
            .map(|a| ParamTy::anonymous(a.expr.ref_t().clone()))
            .collect::<Vec<_>>();
        let mut return_t = branch_ts[0].ty.return_t().unwrap().clone();
        for arg_t in branch_ts.iter().skip(1) {
            return_t = self.rec_union(&return_t, arg_t.ty.return_t().unwrap());
        }
        let param_ty = ParamTy::anonymous(match_target_expr_t.clone());
        let param_ts = [vec![param_ty], branch_ts.to_vec()].concat();
        let t = Type::func(param_ts, vec![], return_t);
        Ok(t)
    }

    pub(crate) fn get_local_uniq_obj_name(&self, name: &Token) -> Option<Str> {
        // TODO: types, functions, patches
        if let Some(ctx) = self.rec_get_mod(name.inspect()) {
            return Some(ctx.name.clone());
        }
        None
    }

    pub(crate) fn rec_get_var_t(
        &self,
        name: &Token,
        vis: Visibility,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        if let Some(vi) = self.get_current_scope_var(&name.inspect()[..]) {
            if vi.vis == vis {
                Ok(vi.t())
            } else {
                Err(TyCheckError::visibility_error(
                    line!() as usize,
                    name.loc(),
                    namespace.clone(),
                    name.inspect(),
                    vi.vis,
                ))
            }
        } else {
            if let Some(parent) = self.outer.as_ref() {
                return parent.rec_get_var_t(name, vis, namespace);
            }
            Err(TyCheckError::no_var_error(
                line!() as usize,
                name.loc(),
                namespace.clone(),
                name.inspect(),
                self.get_similar_name(name.inspect()),
            ))
        }
    }

    pub(crate) fn rec_get_attr_t(
        &self,
        obj: &hir::Expr,
        name: &Token,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        let self_t = obj.t();
        match self_t {
            Type => todo!(),
            Type::Record(rec) => {
                // REVIEW: `rec.get(name.inspect())` returns None (Borrow<Str> is implemented for Field). Why?
                if let Some(attr) = rec.get(&Field::new(Public, name.inspect().clone())) {
                    return Ok(attr.clone());
                } else {
                    let t = Type::Record(rec);
                    return Err(TyCheckError::no_attr_error(
                        line!() as usize,
                        name.loc(),
                        namespace.clone(),
                        &t,
                        name.inspect(),
                        self.get_similar_attr(&t, name.inspect()),
                    ));
                }
            }
            Module => {
                let mod_ctx = self.get_context(obj, Some(ContextKind::Module), namespace)?;
                let t = mod_ctx.rec_get_var_t(name, Public, namespace)?;
                return Ok(t);
            }
            _ => {}
        }
        for ctx in self.rec_sorted_sup_type_ctxs(&self_t) {
            if let Ok(t) = ctx.rec_get_var_t(name, Public, namespace) {
                return Ok(t);
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.outer.as_ref() {
            parent.rec_get_attr_t(obj, name, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                line!() as usize,
                name.loc(),
                namespace.clone(),
                &self_t,
                name.inspect(),
                self.get_similar_attr(&self_t, name.inspect()),
            ))
        }
    }

    /// 戻り値ではなく、call全体の型を返す
    fn search_callee_t(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Token>,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        if let Some(method_name) = method_name.as_ref() {
            for ctx in self.rec_sorted_sup_type_ctxs(obj.ref_t()) {
                if let Some(vi) = ctx.locals.get(method_name.inspect()) {
                    return Ok(vi.t());
                } else if let Some(vi) = ctx.decls.get(method_name.inspect()) {
                    return Ok(vi.t());
                }
            }
            Err(TyCheckError::no_attr_error(
                line!() as usize,
                method_name.loc(),
                namespace.clone(),
                obj.ref_t(),
                method_name.inspect(),
                self.get_similar_attr(obj.ref_t(), method_name.inspect()),
            ))
        } else {
            Ok(obj.t())
        }
    }

    pub(crate) fn get_binop_t(
        &self,
        op: &Token,
        args: &[hir::PosArg],
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        erg_common::debug_power_assert!(args.len() == 2);
        let cont = binop_to_dname(op.inspect());
        let symbol = Token::new(op.kind, Str::rc(cont), op.lineno, op.col_begin);
        let t = self.rec_get_var_t(&symbol, Private, namespace)?;
        let op = hir::Expr::Accessor(hir::Accessor::local(symbol, t));
        self.get_call_t(&op, &None, args, &[], namespace)
            .map_err(|e| {
                let op = enum_unwrap!(op, hir::Expr::Accessor:(hir::Accessor::Local:(_)));
                let lhs = args[0].expr.clone();
                let rhs = args[1].expr.clone();
                let bin = hir::BinOp::new(op.name, lhs, rhs, op.t);
                // HACK: dname.loc()はダミーLocationしか返さないので、エラーならop.loc()で上書きする
                let core = ErrorCore::new(
                    e.core.errno,
                    e.core.kind,
                    bin.loc(),
                    e.core.desc,
                    e.core.hint,
                );
                TyCheckError::new(core, e.caused_by)
            })
    }

    pub(crate) fn get_unaryop_t(
        &self,
        op: &Token,
        args: &[hir::PosArg],
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        erg_common::debug_power_assert!(args.len() == 1);
        let cont = unaryop_to_dname(op.inspect());
        let symbol = Token::new(op.kind, Str::rc(cont), op.lineno, op.col_begin);
        let t = self.rec_get_var_t(&symbol, Private, namespace)?;
        let op = hir::Expr::Accessor(hir::Accessor::local(symbol, t));
        self.get_call_t(&op, &None, args, &[], namespace)
            .map_err(|e| {
                let op = enum_unwrap!(op, hir::Expr::Accessor:(hir::Accessor::Local:(_)));
                let expr = args[0].expr.clone();
                let unary = hir::UnaryOp::new(op.name, expr, op.t);
                let core = ErrorCore::new(
                    e.core.errno,
                    e.core.kind,
                    unary.loc(),
                    e.core.desc,
                    e.core.hint,
                );
                TyCheckError::new(core, e.caused_by)
            })
    }

    /// 可変依存型の変更を伝搬させる
    fn propagate(&self, t: &Type, callee: &hir::Expr) -> TyCheckResult<()> {
        if let Type::Subr(SubrType {
            kind: SubrKind::ProcMethod {
                after: Some(after), ..
            },
            ..
        }) = t
        {
            let receiver_t = callee.receiver_t().unwrap();
            self.reunify(receiver_t, after, Some(callee.loc()), None)?;
        }
        Ok(())
    }

    /// Replace monomorphised trait with concrete type
    /// Just return input if the type is already concrete (or there is still a type variable that cannot be resolved)
    /// 単相化されたトレイトを具体的な型に置換する
    /// 既に具体的な型である(か、まだ型変数があり解決できない)場合はそのまま返す
    /// ```erg
    /// instantiate_trait(Add(Int)) => Ok(Int)
    /// instantiate_trait(Array(Add(Int), 2)) => Ok(Array(Int, 2))
    /// instantiate_trait(Array(Int, 2)) => Ok(Array(Int, 2))
    /// instantiate_trait(Int) => Ok(Int)
    /// ```
    fn resolve_trait(&self, maybe_trait: Type) -> TyCheckResult<Type> {
        match maybe_trait {
            Type::FreeVar(fv) if fv.is_linked() => {
                let inner = fv.crack().clone();
                let t = self.resolve_trait(inner)?;
                fv.link(&t);
                Ok(Type::FreeVar(fv))
            }
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = enum_unwrap!(
                    fv.crack_constraint().clone(),
                    Constraint::Sandwiched { sub, sup }
                );
                let (new_sub, new_sup) = (self.resolve_trait(sub)?, self.resolve_trait(sup)?);
                let new_constraint = Constraint::sandwiched(new_sub, new_sup);
                fv.update_constraint(new_constraint);
                Ok(Type::FreeVar(fv))
            }
            Type::Poly { name, params } if params.iter().all(|tp| tp.has_no_unbound_var()) => {
                let t_name = name.clone();
                let t_params = params.clone();
                let maybe_trait = Type::Poly { name, params };
                let mut min = Type::Obj;
                for pair in self.rec_get_trait_impls(&t_name) {
                    if self.rec_supertype_of(&pair.sup_trait, &maybe_trait) {
                        let new_min = self.rec_min(&min, &pair.sub_type).unwrap_or(&min).clone();
                        min = new_min;
                    }
                }
                if min == Type::Obj {
                    // may be `Array(Add(Int), 2)`, etc.
                    let mut new_params = Vec::with_capacity(t_params.len());
                    for param in t_params.into_iter() {
                        match param {
                            TyParam::Type(t) => {
                                let new_t = self.resolve_trait(*t)?;
                                new_params.push(TyParam::t(new_t));
                            }
                            other => {
                                new_params.push(other);
                            }
                        }
                    }
                    Ok(Type::poly(t_name, new_params))
                } else {
                    Ok(min)
                }
            }
            Type::Subr(subr) => {
                let mut new_non_default_params = Vec::with_capacity(subr.non_default_params.len());
                for param in subr.non_default_params.into_iter() {
                    let t = self.resolve_trait(param.ty)?;
                    new_non_default_params.push(ParamTy::new(param.name, t));
                }
                let mut new_default_params = Vec::with_capacity(subr.default_params.len());
                for param in subr.default_params.into_iter() {
                    let t = self.resolve_trait(param.ty)?;
                    new_default_params.push(ParamTy::new(param.name, t));
                }
                let new_return_t = self.resolve_trait(*subr.return_t)?;
                let t = Type::subr(
                    subr.kind, // TODO: resolve self
                    new_non_default_params,
                    new_default_params,
                    new_return_t,
                );
                Ok(t)
            }
            Type::MonoProj { lhs, rhs } => {
                let new_lhs = self.resolve_trait(*lhs)?;
                Ok(Type::mono_proj(new_lhs, rhs))
            }
            Type::Refinement(refine) => {
                let new_t = self.resolve_trait(*refine.t)?;
                Ok(Type::refinement(refine.var, new_t, refine.preds))
            }
            Type::Ref(t) => {
                let new_t = self.resolve_trait(*t)?;
                Ok(Type::ref_(new_t))
            }
            Type::RefMut(t) => {
                let new_t = self.resolve_trait(*t)?;
                Ok(Type::ref_mut(new_t))
            }
            Type::VarArgs(t) => {
                let new_t = self.resolve_trait(*t)?;
                Ok(Type::var_args(new_t))
            }
            Type::Callable { .. } => todo!(),
            Type::And(_, _) | Type::Or(_, _) | Type::Not(_, _) => todo!(),
            other => Ok(other),
        }
    }

    /// e.g.
    /// ```erg
    /// substitute_call(instance: ((?T, ?U) -> ?T), [Int, Str], []) => instance: (Int, Str) -> Int
    /// substitute_call(instance: ((?T, Int) -> ?T), [Int, Nat], []) => instance: (Int, Int) -> Str
    /// substitute_call(instance: ((?M(: Nat)..?N(: Nat)) -> ?M+?N), [1..2], []) => instance: (1..2) -> {3}
    /// substitute_call(instance: ((?L(: Add(?R, ?O)), ?R) -> ?O), [1, 2], []) => instance: (Nat, Nat) -> Nat
    /// ```
    fn substitute_call(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Token>,
        instance: &Type,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<()> {
        match instance {
            Type::Subr(subr) => {
                let callee = if let Some(name) = method_name {
                    let attr = hir::Attribute::new(obj.clone(), name.clone(), Type::Ellipsis);
                    let acc = hir::Expr::Accessor(hir::Accessor::Attr(attr));
                    acc
                } else {
                    obj.clone()
                };
                let params_len = subr.non_default_params.len() + subr.default_params.len();
                if params_len < pos_args.len() + kw_args.len() {
                    return Err(TyCheckError::too_many_args_error(
                        line!() as usize,
                        callee.loc(),
                        &callee.to_string(),
                        self.caused_by(),
                        params_len,
                        pos_args.len(),
                        kw_args.len(),
                    ));
                }
                let mut passed_params = set! {};
                let params = subr
                    .non_default_params
                    .iter()
                    .chain(subr.default_params.iter());
                for (param_ty, pos_arg) in params.clone().zip(pos_args) {
                    let arg_t = pos_arg.expr.ref_t();
                    let param_t = &param_ty.ty;
                    self.sub_unify(arg_t, param_t, Some(pos_arg.loc()), None)
                        .map_err(|e| {
                            log!("{RED}semi-unification failed with {callee} ({arg_t} <:? {param_t})");
                            log!("errno: {}{GREEN}", e.core.errno);
                            // REVIEW:
                            let name = callee.var_full_name().unwrap_or_else(|| "".to_string());
                            let name = name
                                + "::"
                                + param_ty
                                    .name
                                    .as_ref()
                                    .map(|s| readable_name(&s[..]))
                                    .unwrap_or("");
                            TyCheckError::type_mismatch_error(
                                line!() as usize,
                                e.core.loc,
                                e.caused_by,
                                &name[..],
                                param_t,
                                arg_t,
                            )
                        })?;
                    if let Some(name) = &param_ty.name {
                        if passed_params.contains(name) {
                            return Err(TyCheckError::multiple_args_error(
                                line!() as usize,
                                callee.loc(),
                                &callee.to_string(),
                                self.caused_by(),
                                name,
                            ));
                        } else {
                            passed_params.insert(name);
                        }
                    }
                }
                let param_ts = {
                    let mut param_ts = Dict::new();
                    for param_ty in params {
                        if let Some(name) = &param_ty.name {
                            param_ts.insert(name, &param_ty.ty);
                        }
                    }
                    param_ts
                };
                for kw_arg in kw_args.iter() {
                    if let Some(param_ty) = param_ts.get(kw_arg.keyword.inspect()) {
                        self.sub_unify(kw_arg.expr.ref_t(), param_ty, Some(kw_arg.loc()), None)?;
                    } else {
                        return Err(TyCheckError::unexpected_kw_arg_error(
                            line!() as usize,
                            kw_arg.keyword.loc(),
                            &callee.to_string(),
                            self.caused_by(),
                            kw_arg.keyword.inspect(),
                        ));
                    }
                }
                Ok(())
            }
            other => todo!("{other}"),
        }
    }

    pub(crate) fn get_call_t(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Token>,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        match obj {
            hir::Expr::Accessor(hir::Accessor::Local(local)) if &local.inspect()[..] == "match" => {
                return self.get_match_call_t(pos_args, kw_args)
            }
            _ => {}
        }
        let found = self.search_callee_t(obj, method_name, namespace)?;
        log!(
            "Found:\ncallee: {obj}{}\nfound: {found}",
            fmt_option!(pre ".", method_name.as_ref().map(|t| &t.content))
        );
        let instance = self.instantiate(found, obj)?;
        log!(
            "Instantiated:\ninstance: {instance}\npos_args: ({})\nkw_args: ({})",
            fmt_slice(pos_args),
            fmt_slice(kw_args)
        );
        self.substitute_call(obj, method_name, &instance, pos_args, kw_args)?;
        log!("Substituted:\ninstance: {instance}");
        let res = self.eval.eval_t_params(instance, &self, self.level)?;
        log!("Params evaluated:\nres: {res}\n");
        self.propagate(&res, obj)?;
        log!("Propagated:\nres: {res}\n");
        let res = self.resolve_trait(res)?;
        log!("Trait resolved:\nres: {res}\n");
        Ok(res)
    }

    pub(crate) fn get_local(&self, name: &Token, namespace: &Str) -> TyCheckResult<ValueObj> {
        if let Some(obj) = self.consts.get(name.inspect()) {
            Ok(obj.clone())
        } else {
            if let Some(parent) = self.outer.as_ref() {
                return parent.get_local(name, namespace);
            }
            Err(TyCheckError::no_var_error(
                line!() as usize,
                name.loc(),
                namespace.clone(),
                name.inspect(),
                self.get_similar_name(name.inspect()),
            ))
        }
    }

    pub(crate) fn _get_attr(
        &self,
        obj: &hir::Expr,
        name: &Token,
        namespace: &Str,
    ) -> TyCheckResult<ValueObj> {
        let self_t = obj.t();
        for ctx in self.sorted_sup_type_ctxs(&self_t) {
            if let Ok(t) = ctx.get_local(name, namespace) {
                return Ok(t);
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.outer.as_ref() {
            parent._get_attr(obj, name, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                line!() as usize,
                name.loc(),
                namespace.clone(),
                &self_t,
                name.inspect(),
                self.get_similar_attr(&self_t, name.inspect()),
            ))
        }
    }

    pub(crate) fn get_similar_name(&self, name: &str) -> Option<&Str> {
        let name = readable_name(name);
        if name.len() <= 1 {
            return None;
        }
        // TODO: add `.decls`
        let most_similar_name = self
            .params
            .iter()
            .filter_map(|(opt_name, _)| opt_name.as_ref())
            .chain(self.locals.keys())
            .min_by_key(|v| levenshtein(readable_name(v.inspect()), name))?
            .inspect();
        let len = most_similar_name.len();
        if levenshtein(most_similar_name, name) >= len / 2 {
            let outer = self.outer.as_ref()?;
            outer.get_similar_name(name)
        } else {
            Some(most_similar_name)
        }
    }

    pub(crate) fn get_similar_attr<'a>(&'a self, self_t: &'a Type, name: &str) -> Option<&'a Str> {
        for ctx in self.rec_sorted_sup_type_ctxs(self_t) {
            if let Some(name) = ctx.get_similar_name(name) {
                return Some(name);
            }
        }
        None
    }

    pub(crate) fn type_params_bounds(&self) -> Set<TyBound> {
        self.params
            .iter()
            .filter(|(opt_name, vi)| vi.kind.is_parameter() && opt_name.is_some())
            .map(|(name, vi)| {
                TyBound::instance(name.as_ref().unwrap().inspect().clone(), vi.t.clone())
            })
            .collect()
    }

    // selfが示す型が、各パラメータTypeに対してどのような変性Varianceを持つかを返す
    // 特に指定されない型に対してはInvariant
    // e.g. K(T, U) = Class(..., Impl: F(T) and Output(U) and Input(T))
    // -> K.variance() == vec![Contravariant, Covariant]
    // TODO: support keyword arguments
    pub(crate) fn type_params_variance(&self) -> Vec<Variance> {
        self.params
            .iter()
            .map(|(opt_name, _)| {
                if let Some(name) = opt_name {
                    if let Some(t) = self.super_traits.iter().find(|t| {
                        (&t.name()[..] == "Input" || &t.name()[..] == "Output")
                            && t.inner_ts()
                                .first()
                                .map(|t| &t.name() == name.inspect())
                                .unwrap_or(false)
                    }) {
                        match &t.name()[..] {
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
    /// ```erg
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

    // TODO: unify with type_sort
    fn sort_type_ctxs<'a>(
        &self,
        type_and_ctxs: impl Iterator<Item = (&'a Type, &'a Context)>,
    ) -> Vec<(&'a Type, &'a Context)> {
        let mut buffers: Vec<Vec<(&Type, &Context)>> = vec![];
        for t_ctx in type_and_ctxs {
            let mut found = false;
            for buf in buffers.iter_mut() {
                if buf
                    .iter()
                    .all(|(buf_inner, _)| self.related(buf_inner, t_ctx.0))
                {
                    found = true;
                    buf.push(t_ctx);
                    break;
                }
            }
            if !found {
                buffers.push(vec![t_ctx]);
            }
        }
        for buf in buffers.iter_mut() {
            // this unwrap should be safe
            buf.sort_by(|(lhs, _), (rhs, _)| self.cmp_t(lhs, rhs).try_into().unwrap());
        }
        let mut concatenated = buffers.into_iter().flatten().collect::<Vec<_>>();
        let mut idx = 0;
        let len = concatenated.len();
        while let Some((maybe_sup, _)) = concatenated.get(idx) {
            if let Some(pos) = concatenated
                .iter()
                .take(len - idx - 1)
                .rposition(|(t, _)| self.supertype_of(maybe_sup, t))
            {
                let sup = concatenated.remove(idx);
                concatenated.insert(pos, sup); // not `pos + 1` because the element was removed at idx
            }
            idx += 1;
        }
        concatenated
    }

    fn sort_type_pairs(
        &self,
        type_and_traits: impl Iterator<Item = TraitInstancePair>,
    ) -> Vec<TraitInstancePair> {
        let mut buffers: Vec<Vec<TraitInstancePair>> = vec![];
        for t_trait in type_and_traits {
            let mut found = false;
            for buf in buffers.iter_mut() {
                if buf
                    .iter()
                    .all(|pair| self.related(&pair.sup_trait, &t_trait.sub_type))
                {
                    found = true;
                    buf.push(t_trait.clone());
                    break;
                }
            }
            if !found {
                buffers.push(vec![t_trait]);
            }
        }
        for buf in buffers.iter_mut() {
            // this unwrap should be safe
            buf.sort_by(|lhs, rhs| {
                self.cmp_t(&lhs.sup_trait, &rhs.sup_trait)
                    .try_into()
                    .unwrap()
            });
        }
        let mut concatenated = buffers.into_iter().flatten().collect::<Vec<_>>();
        let mut idx = 0;
        let len = concatenated.len();
        while let Some(pair) = concatenated.get(idx) {
            if let Some(pos) = concatenated
                .iter()
                .take(len - idx - 1)
                .rposition(|p| self.supertype_of(&pair.sup_trait, &p.sup_trait))
            {
                let sup = concatenated.remove(idx);
                concatenated.insert(pos, sup); // not `pos + 1` because the element was removed at idx
            }
            idx += 1;
        }
        concatenated
    }

    pub(crate) fn rec_sorted_sup_type_ctxs<'a>(
        &'a self,
        t: &'a Type,
    ) -> impl Iterator<Item = &'a Context> {
        let i = self.sorted_sup_type_ctxs(t);
        if i.size_hint().1 == Some(0) {
            if let Some(outer) = &self.outer {
                return outer.sorted_sup_type_ctxs(t);
            }
        }
        i
    }

    /// Return `Context`s equal to or greater than `t`
    /// tと一致ないしそれよりも大きい型のContextを返す
    fn sorted_sup_type_ctxs<'a>(&'a self, t: &'a Type) -> impl Iterator<Item = &'a Context> {
        log!("{t}");
        let mut ctxs = self._sup_type_ctxs(t).collect::<Vec<_>>();
        log!("{t}");
        // Avoid heavy sorting as much as possible for efficiency
        let mut cheap_sort_succeed = true;
        ctxs.sort_by(|(lhs, _), (rhs, _)| match self.cmp_t(lhs, rhs).try_into() {
            Ok(ord) => ord,
            Err(_) => {
                cheap_sort_succeed = false;
                Ordering::Equal
            }
        });
        let sorted = if cheap_sort_succeed {
            ctxs
        } else {
            self.sort_type_ctxs(ctxs.into_iter())
        };
        sorted.into_iter().map(|(_, ctx)| ctx)
    }

    fn _just_type_ctxs<'a>(&'a self, t: &'a Type) -> Option<(&'a Type, &'a Context)> {
        self.types.iter().find(move |(maybe_sup, ctx)| {
            let maybe_sup_inst = if maybe_sup.has_qvar() {
                let bounds = ctx.type_params_bounds();
                let mut tv_ctx = TyVarContext::new(self.level, bounds, self);
                Self::instantiate_t((*maybe_sup).clone(), &mut tv_ctx)
            } else {
                (*maybe_sup).clone()
            };
            self.same_type_of(&maybe_sup_inst, t)
        })
    }

    /// this method is for `sorted_type_ctxs` only
    fn _sup_type_ctxs<'a>(&'a self, t: &'a Type) -> impl Iterator<Item = (&'a Type, &'a Context)> {
        log!("{t}");
        self.types.iter().filter_map(move |(maybe_sup, ctx)| {
            let maybe_sup_inst = if maybe_sup.has_qvar() {
                let bounds = ctx.type_params_bounds();
                let mut tv_ctx = TyVarContext::new(self.level, bounds, self);
                Self::instantiate_t(maybe_sup.clone(), &mut tv_ctx)
            } else {
                maybe_sup.clone()
            };
            log!("{maybe_sup}, {t}");
            if self.supertype_of(&maybe_sup_inst, t) {
                Some((maybe_sup, ctx))
            } else {
                None
            }
        })
    }

    fn rec_get_trait_impls(&self, name: &Str) -> Vec<TraitInstancePair> {
        let current = if let Some(impls) = self.trait_impls.get(name) {
            impls.clone()
        } else {
            vec![]
        };
        if let Some(outer) = &self.outer {
            [current, outer.rec_get_trait_impls(name)].concat()
        } else {
            current
        }
    }

    fn rec_get_glue_patch_and_types(&self) -> Vec<(VarName, TraitInstancePair)> {
        if let Some(outer) = &self.outer {
            [
                &self.glue_patch_and_types[..],
                &outer.rec_get_glue_patch_and_types(),
            ]
            .concat()
        } else {
            self.glue_patch_and_types.clone()
        }
    }

    fn rec_get_patch(&self, name: &VarName) -> Option<&Context> {
        if let Some(patch) = self.patches.get(name) {
            Some(patch)
        } else if let Some(outer) = &self.outer {
            outer.rec_get_patch(name)
        } else {
            None
        }
    }

    fn rec_get_mod(&self, name: &str) -> Option<&Context> {
        if let Some(mod_) = self.mods.get(name) {
            Some(mod_)
        } else if let Some(outer) = &self.outer {
            outer.rec_get_mod(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_get_const_obj(&self, name: &str) -> Option<&ValueObj> {
        if let Some(val) = self.consts.get(name) {
            Some(val)
        } else if let Some(outer) = &self.outer {
            outer.rec_get_const_obj(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_type_ctx_by_name<'a>(&'a self, t_name: &'a str) -> Option<&'a Context> {
        if let Some((_, ctx)) = self.types.iter().find(|(t, _ctx)| &t.name()[..] == t_name) {
            return Some(ctx);
        }
        if let Some(outer) = &self.outer {
            outer.rec_type_ctx_by_name(t_name)
        } else {
            None
        }
    }

    pub(crate) fn rec_get_const_param_defaults(&self, name: &str) -> Option<&Vec<ConstTemplate>> {
        if let Some(impls) = self.const_param_defaults.get(name) {
            return Some(impls);
        }
        if let Some(outer) = &self.outer {
            outer.rec_get_const_param_defaults(name)
        } else {
            None
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
}
