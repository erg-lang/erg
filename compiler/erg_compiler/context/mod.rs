//! Defines `Context`.
//! `Context` is used for type inference and type checking.
pub mod compare;
pub mod inquire;
pub mod instantiate;
pub mod register;
pub mod test;
pub mod tyvar;

use std::fmt;
use std::mem;
use std::option::Option; // conflicting to Type::Option

use erg_common::dict::Dict;
use erg_common::error::Location;
use erg_common::impl_display_from_debug;
use erg_common::traits::{Locational, Stream};
use erg_common::ty::{Predicate, TyBound, Type};
use erg_common::typaram::TyParam;
use erg_common::value::{ValueObj, Visibility};
use erg_common::Str;
use erg_common::{fn_name, get_hash, log};
use Type::*;

use ast::{DefId, VarName};
use erg_parser::ast;
use erg_parser::token::Token;

use crate::context::instantiate::ConstTemplate;
use crate::error::{TyCheckError, TyCheckErrors, TyCheckResult};
use crate::eval::Evaluator;
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
