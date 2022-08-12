//! Defines `Context`.
//! `Context` is used for type inference and type checking.
use std::fmt;
use std::mem;
use std::option::Option; // conflicting to Type::Option

use erg_common::Str;
use erg_common::ty::Constraint;
use erg_common::ty::RefinementType;
use erg_common::ty::fresh_varname;
use erg_common::{fn_name, get_hash, log, assume_unreachable, set, try_map, fmt_slice, enum_unwrap};
use erg_common::dict::Dict;
use erg_common::set::Set;
use erg_common::error::{Location, ErrorCore};
use erg_common::value::ValueObj;
use erg_common::levenshtein::levenshtein;
use erg_common::traits::{HasType, Locational, Stream};
use erg_common::ty::{
    Type, TyParam, TyParamOrdering, TyBound, ConstObj,
    IntervalOp, FreeKind, HasLevel, SubrKind, SubrType, ParamTy, Predicate,
};
use TyParamOrdering::*;
use Type::*;
use Predicate as Pred;
use ValueObj::{Inf, NegInf};

use erg_parser::ast;
use ast::{VarName, DefId, TypeSpec, ParamTySpec, PreDeclTypeSpec, SimpleTypeSpec, TypeBoundSpec, TypeBoundSpecs, ParamSig};
use erg_parser::token::{Token, TokenKind};

use crate::hir;
use crate::eval::{Evaluator};
use crate::error::{TyCheckError, TyCheckErrors, TyCheckResult, binop_to_dname, unaryop_to_dname};
use crate::varinfo::{VarInfo, Mutability, Visibility, VarKind, ParamId};
use Mutability::*;
use Visibility::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultInfo {
    NonDefault,
    WithDefault,
}

impl DefaultInfo {
    pub const fn has_default(&self) -> bool { matches!(self, DefaultInfo::WithDefault) }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamSpec {
    pub(crate) name: Option<&'static str>,
    pub(crate) t: Type,
    pub default_info: DefaultInfo,
}

impl ParamSpec {
    pub const fn new(name: Option<&'static str>, t: Type, default: DefaultInfo) -> Self {
        Self { name, t, default_info: default }
    }

    pub const fn named(name: &'static str, t: Type, default: DefaultInfo) -> Self {
        Self::new(Some(name), t, default)
    }

    pub const fn named_nd(name: &'static str, t: Type) -> Self {
        Self::new(Some(name), t, DefaultInfo::NonDefault)
    }

    pub const fn t(name: &'static str, default: DefaultInfo) -> Self { Self::new(Some(name), Type, default) }

    pub const fn t_nd(name: &'static str) -> Self { Self::new(Some(name), Type, DefaultInfo::NonDefault) }
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

/// Context for instantiating a quantified type
/// 量化型をインスタンス化するための文脈
#[derive(Debug, Clone)]
pub struct TyVarContext {
    level: usize,
    pub(crate) tyvar_instances: Dict<Str, Type>,
    pub(crate) typaram_instances: Dict<Str, TyParam>,
}

impl TyVarContext {
    pub fn new(level: usize, bounds: Set<TyBound>) -> Self {
        let mut self_ = Self{ level, tyvar_instances: Dict::new(), typaram_instances: Dict::new() };
        for bound in bounds.into_iter() {
            self_.instantiate_bound(bound);
        }
        self_
    }

    fn instantiate_bound(&mut self, bound: TyBound) {
        match bound {
            TyBound::Subtype{ sub, sup } => {
                let sup = match sup {
                    Type::Poly{ name, params } => {
                        let sup = Type::poly(name, params.into_iter().map(|p| self.instantiate_tp(p)).collect());
                        sup
                    },
                    Type::MonoProj{ lhs, rhs } => {
                        Type::mono_proj(self.instantiate_t(*lhs), rhs)
                    }
                    sup => sup,
                };
                let constraint = Constraint::SubtypeOf(sup);
                self.push_tyvar(Str::rc(sub.name()), Type::free_var(self.level, constraint));
            },
            TyBound::Supertype{ sup, sub } => {
                let sub = match sub {
                    Type::Poly{ name, params } => {
                        let sub = Type::poly(name, params.into_iter().map(|p| self.instantiate_tp(p)).collect());
                        sub
                    },
                    Type::MonoProj{ lhs, rhs } => {
                        Type::mono_proj(self.instantiate_t(*lhs), rhs)
                    }
                    sub => sub,
                };
                let constraint = Constraint::SupertypeOf(sub);
                self.push_tyvar(Str::rc(sup.name()), Type::free_var(self.level, constraint));
            },
            TyBound::Sandwiched{ sub, mid, sup } => {
                let sub = match sub {
                    Type::Poly{ name, params } => {
                        let sub = Type::poly(name, params.into_iter().map(|p| self.instantiate_tp(p)).collect());
                        sub
                    },
                    Type::MonoProj{ lhs, rhs } => {
                        Type::mono_proj(self.instantiate_t(*lhs), rhs)
                    }
                    sub => sub,
                };
                let sup = match sup {
                    Type::Poly{ name, params } => {
                        let sup = Type::poly(name, params.into_iter().map(|p| self.instantiate_tp(p)).collect());
                        sup
                    },
                    Type::MonoProj{ lhs, rhs } => {
                        Type::mono_proj(self.instantiate_t(*lhs), rhs)
                    }
                    sup => sup,
                };
                let constraint = Constraint::Sandwiched{sub, sup};
                self.push_tyvar(Str::rc(mid.name()), Type::free_var(self.level, constraint));
            },
            TyBound::Instance{ name, t } => {
                let t = match t {
                    Type::Poly{ name, params } => {
                        let t = Type::poly(name, params.into_iter().map(|p| self.instantiate_tp(p)).collect());
                        t
                    },
                    t => t,
                };
                // TODO: type-like types
                if &t == &Type {
                    let constraint = Constraint::TypeOf(t);
                    self.push_tyvar(name.clone(), Type::named_free_var(name, self.level, constraint));
                } else {
                    self.push_typaram(name.clone(), TyParam::named_free_var(name, self.level, t));
                }
            },
        }
    }

    fn _instantiate_pred(&self, _pred: Predicate) -> Predicate {
        todo!()
    }

    pub(crate) fn instantiate_t(&self, quantified: Type) -> Type {
        match quantified {
            Type::MonoQVar(n) => {
                if let Some(t) = self.get_tyvar(&n) {
                    return t.clone()
                } else if let Some(t) = self.get_typaram(&n) {
                    if let TyParam::Type(t) = t { return *t.clone() }
                    else { todo!() }
                } else { todo!() }
            },
            other => todo!("{other}"),
        }
    }

    fn instantiate_tp(&self, quantified: TyParam) -> TyParam {
        match quantified {
            TyParam::MonoQVar(n) => {
                if let Some(t) = self.get_typaram(&n) {
                    return t.clone()
                } else if let Some(t) = self.get_tyvar(&n) {
                    return TyParam::t(t.clone())
                } else { todo!() }
            },
            TyParam::UnaryOp{ op, val } => {
                let res = self.instantiate_tp(*val);
                TyParam::unary(op, res)
            },
            TyParam::BinOp{ op, lhs, rhs } => {
                let lhs = self.instantiate_tp(*lhs);
                let rhs = self.instantiate_tp(*rhs);
                TyParam::bin(op, lhs, rhs)
            },
            p @ TyParam::ConstObj(_) => p,
            other => todo!("{other}"),
        }
    }

    pub(crate) fn push_tyvar(&mut self, name: Str, t: Type) {
        self.tyvar_instances.insert(name, t);
    }

    pub(crate) fn push_typaram(&mut self, name: Str, t: TyParam) {
        self.typaram_instances.insert(name, t);
    }

    pub(crate) fn get_tyvar(&self, name: &str) -> Option<&Type> {
        self.tyvar_instances.get(name)
    }

    pub(crate) fn get_typaram(&self, name: &str) -> Option<&TyParam> {
        self.typaram_instances.get(name)
    }
}

/// Represents the context of the current scope
#[derive(Debug)]
pub struct Context {
    pub(crate) name: Str,
    pub(crate) kind: ContextKind,
    // Type bounds & Predicates (if the context kind is Subroutine)
    // ユーザー定義APIでのみ使う
    pub(crate) bounds: Vec<TyBound>,
    pub(crate) preds: Vec<Predicate>,
    // for looking up the parent scope
    pub(crate) outer: Option<Box<Context>>,
    // patchによってsuper class/traitになったものはここに含まれない
    pub(crate) super_classes: Vec<Type>, // if self is a patch, means patch classes
    pub(crate) super_traits: Vec<Type>, // if self is not a trait, means implemented traits
    // K: メソッド名, V: それを実装するパッチたち
    // 提供メソッドはスコープごとに実装を切り替えることができる
    pub(crate) _method_impl_patches: Dict<VarName, Vec<VarName>>,
    // .0: 関係付けるパッチ(glue patch), .1: サブタイプになる型, .2: スーパータイプになるトレイト
    // 一つの型ペアを接着パッチは同時に一つまでしか存在しないが、付け替えは可能
    pub(crate) glue_patch_and_types: Vec<(VarName, Type, Type)>,
    // stores declared names (not initialized)
    pub(crate) decls: Dict<VarName, VarInfo>,
    // stores defined names
    // 型の一致はHashMapでは判定できないため、keyはVarName
    pub(crate) impls: Dict<VarName, VarInfo>,
    pub(crate) consts: Dict<Str, ConstObj>,
    pub(crate) unnamed_params: Vec<VarInfo>,
    // FIXME: Compilerが持つ
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
        Self::new("<dummy>".into(), ContextKind::Dummy, vec![], None, vec![], vec![], Self::TOP_LEVEL)
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("name", &self.name)
            .field("bounds", &self.bounds)
            .field("preds", &self.preds)
            .field("decls", &self.decls)
            .field("impls", &self.impls)
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
        level: usize
    ) -> Self {
        Self::with_capacity(name, kind, params, outer, super_classes, super_traits, 0, level)
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
        let mut impls = Dict::with_capacity(capacity);
        let mut unnamed_params = Vec::new();
        for (pos, param) in params.into_iter().enumerate() {
            let id = DefId(get_hash(&(&name, &param)));
            if let Some(name) = param.name {
                let param_id = if param.default_info.has_default() {
                    ParamId::var_default(name.into(), pos)
                } else { ParamId::var_non_default(name.into(), pos) };
                let kind = VarKind::parameter(id, param_id);
                // TODO: is_const { Const } else { Immutable }
                let vi = VarInfo::new(param.t, Immutable, Private, kind);
                impls.insert(VarName::new(Token::static_symbol(name)), vi);
            } else {
                let param_id = if param.default_info.has_default() {
                    ParamId::PatWithDefault(pos)
                } else { ParamId::PatNonDefault(pos) };
                let kind = VarKind::parameter(id, param_id);
                let vi = VarInfo::new(param.t, Immutable, Private, kind);
                unnamed_params.push(vi);
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
            _method_impl_patches: Dict::default(),
            glue_patch_and_types: Vec::default(),
            decls: Dict::default(),
            impls,
            consts: Dict::default(),
            unnamed_params,
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
        level: usize
    ) -> Self {
        Self::with_capacity(name, kind, vec![], outer, super_classes, super_traits, 0, level)
    }

    #[inline]
    pub fn poly(
        name: Str,
        kind: ContextKind,
        params: Vec<ParamSpec>,
        outer: Option<Context>,
        super_classes: Vec<Type>,
        super_traits: Vec<Type>,
        level: usize
    ) -> Self {
        Self::with_capacity(name, kind, params, outer, super_classes, super_traits, 0, level)
    }

    pub fn poly_trait<S: Into<Str>>(name: S, params: Vec<ParamSpec>, supers: Vec<Type>, level: usize) -> Self {
        let name = name.into();
        Self::poly(name, ContextKind::Trait, params, None, vec![], supers, level)
    }

    pub fn poly_class<S: Into<Str>>(name: S, params: Vec<ParamSpec>, super_classes: Vec<Type>, impl_traits: Vec<Type>, level: usize) -> Self {
        let name = name.into();
        Self::poly(name, ContextKind::Class, params, None, super_classes, impl_traits, level)
    }

    #[inline]
    pub fn mono_trait<S: Into<Str>>(name: S, supers: Vec<Type>, level: usize) -> Self {
        Self::poly_trait(name, vec![], supers, level)
    }

    #[inline]
    pub fn mono_class<S: Into<Str>>(name: S, super_classes: Vec<Type>, super_traits: Vec<Type>, level: usize) -> Self {
        Self::poly_class(name, vec![], super_classes, super_traits, level)
    }

    #[inline]
    pub fn poly_patch<S: Into<Str>>(name: S, params: Vec<ParamSpec>, patch_classes: Vec<Type>, impl_traits: Vec<Type>, level: usize) -> Self {
        Self::poly(name.into(), ContextKind::Trait, params, None, patch_classes, impl_traits, level)
    }

    #[inline]
    pub fn module(name: Str, capacity: usize) -> Self {
        Self::with_capacity(name, ContextKind::Module, vec![], None, vec![], vec![], capacity, Self::TOP_LEVEL)
    }

    #[inline]
    pub fn caused_by(&self) -> Str { self.name.clone() }

    fn registered<Q: std::hash::Hash + Eq>(&self, name: &Q, recursive: bool) -> bool
    where VarName: std::borrow::Borrow<Q> {
        if self.impls.contains_key(name) { return true }
        if recursive {
            if let Some(outer) = &self.outer {
                outer.registered(name, recursive)
            } else { false }
        } else { false }
    }
}

// setters
impl Context {
    pub(crate) fn declare_var(&mut self, sig: &ast::VarSignature, opt_t: Option<Type>, id: Option<DefId>) -> TyCheckResult<()> {
        self.declare_var_pat(sig, opt_t, id)
    }

    fn declare_var_pat(&mut self, sig: &ast::VarSignature, opt_t: Option<Type>, id: Option<DefId>) -> TyCheckResult<()> {
        let vis = Private; // TODO:
        let muty = Mutability::from(&sig.inspect().unwrap()[..]);
        match &sig.pat {
            ast::VarPattern::VarName(v) => {
                if sig.t_spec.is_none() && opt_t.is_none() {
                    Err(TyCheckError::no_type_spec_error(sig.loc(), self.caused_by(), v.inspect()))
                } else {
                    if self.registered(v, v.inspect().is_uppercase()) {
                        return Err(TyCheckError::duplicate_decl_error(sig.loc(), self.caused_by(), v.inspect()))
                    }
                    let kind = id.map(|id| VarKind::Defined(id)).unwrap_or(VarKind::Declared);
                    let sig_t = self.instantiate_var_sig_t(sig, opt_t, PreRegister)?;
                    self.decls.insert(v.clone(), VarInfo::new(sig_t, muty, vis, kind));
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

    pub(crate) fn declare_sub(&mut self, sig: &ast::SubrSignature, opt_ret_t: Option<Type>, id: Option<DefId>) -> TyCheckResult<()> {
        let name = sig.name.inspect();
        let muty = Mutability::from(&name[..]);
        let kind = id.map(|id| VarKind::Defined(id)).unwrap_or(VarKind::Declared);
        if self.registered(name, name.is_uppercase()) {
            return Err(TyCheckError::duplicate_decl_error(sig.loc(), self.caused_by(), name))
        }
        let t = self.instantiate_sub_sig_t(sig, opt_ret_t, PreRegister)?;
        let vi = VarInfo::new(t, muty, Private, kind);
        if let Some(_decl) = self.decls.remove(name) {
            return Err(TyCheckError::duplicate_decl_error(sig.loc(), self.caused_by(), name))
        } else {
            self.decls.insert(sig.name.clone(), vi);
        }
        Ok(())
    }

    pub(crate) fn assign_var(&mut self, sig: &ast::VarSignature, id: DefId, body_t: &Type) -> TyCheckResult<()> {
        self.assign_var_sig(sig, body_t, id)
    }

    fn assign_var_sig(&mut self, sig: &ast::VarSignature, body_t: &Type,  id: DefId) -> TyCheckResult<()> {
        self.validate_var_sig_t(sig, body_t, Normal)?;
        let vis = Private; // TODO:
        let muty = Mutability::from(&sig.inspect().unwrap()[..]);
        let (generalized, bounds) = self.generalize_t(body_t.clone());
        let generalized = if !bounds.is_empty() {
            if self.rec_supertype_of(&Type::CallableCommon, &generalized) {
                Type::quantified(generalized, bounds)
            } else { panic!() }
        } else { generalized };
        match &sig.pat {
            ast::VarPattern::Discard(_token) => Ok(()),
            ast::VarPattern::VarName(v) => {
                if self.registered(v, v.inspect().is_uppercase()) {
                    Err(TyCheckError::reassign_error(v.loc(), self.caused_by(), v.inspect()))
                } else {
                    if let Some(_) = self.decls.remove(v.inspect()) {
                        // something to do?
                    }
                    let vi = VarInfo::new(generalized, muty, vis, VarKind::Defined(id));
                    self.impls.insert(v.clone(), vi);
                    Ok(())
                }
            }
            ast::VarPattern::SelfDot(_) => todo!(),
            ast::VarPattern::Array(arr) => {
                for (elem, inf) in arr.iter().zip(generalized.inner_ts().iter()) {
                    let id = DefId(get_hash(&(&self.name, elem)));
                    self.assign_var_sig(elem, inf, id)?;
                }
                Ok(())
            }
            ast::VarPattern::Tuple(_) => todo!(),
            ast::VarPattern::Record{ .. } => todo!(),
        }
    }

    /// 宣言が既にある場合、opt_decl_tに宣言の型を渡す
    fn assign_non_default_param(&mut self, sig: &ast::NonDefaultParamSignature, opt_param_pos: Option<usize>, opt_decl_t: Option<&ParamTy>) -> TyCheckResult<()> {
        match &sig.pat {
            ast::ParamPattern::Discard(_token) => Ok(()),
            ast::ParamPattern::VarName(v) => {
                if self.registered(v, v.inspect().is_uppercase()) {
                    Err(TyCheckError::reassign_error(v.loc(), self.caused_by(), v.inspect()))
                } else { // ok, impl not found
                    let spec_t = self.instantiate_param_sig_t(sig, opt_decl_t, Normal)?;
                    let param_id = if let Some(param_pos) = opt_param_pos {
                        ParamId::var_non_default(v.inspect().into(), param_pos)
                    } else { ParamId::Embedded(v.inspect().into()) };
                    let kind = VarKind::parameter(DefId(get_hash(&(&self.name, v))), param_id);
                    self.impls.insert(v.clone(), VarInfo::new(spec_t, Immutable, Private, kind));
                    Ok(())
                }
            }
            ast::ParamPattern::Array(arr) => {
                if let Some(decl_t) = opt_decl_t {
                    for (elem, p) in arr.elems.non_defaults.iter().zip(decl_t.ty.non_default_params().unwrap()) {
                        self.assign_non_default_param(elem, None, Some(p))?;
                    }
                    for (elem, p) in arr.elems.defaults.iter().zip(decl_t.ty.default_params().unwrap()) {
                        self.assign_default_param(elem, None, Some(p))?;
                    }
                } else {
                    for elem in arr.elems.non_defaults.iter() {
                        self.assign_non_default_param(elem, None, None)?;
                    }
                    for elem in arr.elems.defaults.iter() {
                        self.assign_default_param(elem, None, None)?;
                    }
                }
                Ok(())
            }
            ast::ParamPattern::Lit(_) => Ok(()),
            _ => todo!(),
        }
    }

    fn assign_default_param(&mut self, sig: &ast::DefaultParamSignature, opt_param_pos: Option<usize>, opt_decl_t: Option<&ParamTy>) -> TyCheckResult<()> {
        match &sig.pat {
            ast::ParamPattern::Discard(_token) => Ok(()),
            ast::ParamPattern::VarName(v) => {
                if self.registered(v, v.inspect().is_uppercase()) {
                    Err(TyCheckError::reassign_error(v.loc(), self.caused_by(), v.inspect()))
                } else { // ok, impl not found
                    let spec_t = self.instantiate_param_sig_t(sig, opt_decl_t, Normal)?;
                    let param_id = if let Some(param_pos) = opt_param_pos {
                        ParamId::var_default(v.inspect().into(), param_pos)
                    } else { ParamId::Embedded(v.inspect().into()) };
                    let kind = VarKind::parameter(DefId(get_hash(&(&self.name, v))), param_id);
                    self.impls.insert(v.clone(), VarInfo::new(spec_t, Immutable, Private, kind));
                    Ok(())
                }
            }
            ast::ParamPattern::Array(arr) => {
                if let Some(decl_t) = opt_decl_t {
                    for (elem, p) in arr.elems.non_defaults.iter().zip(decl_t.ty.non_default_params().unwrap()) {
                        self.assign_non_default_param(elem, None, Some(p))?;
                    }
                    for (elem, p) in arr.elems.defaults.iter().zip(decl_t.ty.default_params().unwrap()) {
                        self.assign_default_param(elem, None, Some(p))?;
                    }
                } else {
                    for elem in arr.elems.non_defaults.iter() {
                        self.assign_non_default_param(elem, None, None)?;
                    }
                    for elem in arr.elems.defaults.iter() {
                        self.assign_default_param(elem, None, None)?;
                    }
                }
                Ok(())
            }
            ast::ParamPattern::Lit(_) => Ok(()),
            _ => todo!(),
        }
    }

    pub(crate) fn assign_params(&mut self, params: &ast::Params, opt_decl_subr_t: Option<SubrType>) -> TyCheckResult<()> {
        if let Some(decl_subr_t) = opt_decl_subr_t {
            for (pos, (sig, pt)) in params.non_defaults.iter().zip(decl_subr_t.non_default_params.iter()).enumerate() {
                self.assign_non_default_param(sig, Some(pos), Some(pt))?;
            }
            for (pos, (sig, pt)) in params.defaults.iter().zip(decl_subr_t.default_params.iter()).enumerate() {
                self.assign_default_param(sig, Some(pos), Some(pt))?;
            }
        } else {
            for (pos, sig) in params.non_defaults.iter().enumerate() {
                self.assign_non_default_param(sig, Some(pos), None)?;
            }
            for (pos, sig) in params.defaults.iter().enumerate() {
                self.assign_default_param(sig, Some(pos), None)?;
            }
        }
        Ok(())
    }

    /// ## Errors
    /// * TypeError: if `return_t` != typeof `body`
    /// * AssignError: if `name` has already been registered
    pub(crate) fn assign_subr(&mut self, sig: &ast::SubrSignature, id: DefId, body_t: &Type) -> TyCheckResult<()> {
        let muty = if sig.name.is_const() { Mutability::Const } else { Mutability::Immutable };
        let name = &sig.name;
        // FIXME: constでない関数
        let t = self.get_current_scope_local_var(&name.inspect())
            .map(|v| &v.t)
            .unwrap();
        let non_default_params = t.non_default_params().unwrap();
        let default_params = t.default_params().unwrap();
        if let Some(spec_ret_t) = t.return_t() {
            self.unify(spec_ret_t, body_t, Some(sig.loc()), None).map_err(|e| {
                TyCheckError::return_type_error(e.core.loc, e.caused_by, name.inspect(), spec_ret_t, body_t)
            })?;
        }
        if self.registered(name, name.inspect().is_uppercase()) {
            Err(TyCheckError::reassign_error(name.loc(), self.caused_by(), name.inspect()))
        } else {
            let sub_t = if sig.name.is_procedural() {
                Type::proc(non_default_params.clone(), default_params.clone(), body_t.clone())
            } else {
                Type::func(non_default_params.clone(), default_params.clone(), body_t.clone())
            };
            sub_t.lift();
            let (generalized, bounds) = self.generalize_t(sub_t);
            let found_t = if !bounds.is_empty() {
                if self.rec_supertype_of(&Type::CallableCommon, &generalized) {
                    Type::quantified(generalized, bounds)
                } else { panic!() }
            } else { generalized };
            if let Some(mut vi) = self.decls.remove(name) {
                if vi.t.has_unbound_var() {
                    vi.t.lift();
                    let (generalized, bounds) = self.generalize_t(vi.t.clone());
                    let generalized = if !bounds.is_empty() {
                        if self.rec_supertype_of(&Type::CallableCommon, &generalized) {
                            Type::quantified(generalized, bounds)
                        } else { panic!() }
                    } else { generalized };
                    vi.t = generalized;
                }
                self.decls.insert(name.clone(), vi);
            }
            if let Some(vi) = self.decls.remove(name) {
                if !self.rec_supertype_of(&vi.t, &found_t) {
                    return Err(TyCheckError::violate_decl_error(
                        sig.loc(),
                        self.caused_by(),
                        name.inspect(),
                        &vi.t,
                        &found_t,
                    ))
                }
            }
            // TODO: visibility
            let vi = VarInfo::new(found_t, muty, Private, VarKind::Defined(id));
            log!("Registered {}::{name}: {}", self.name, &vi.t);
            self.impls.insert(name.clone(), vi);
            Ok(())
        }
    }

    pub(crate) fn import_mod(&mut self, var_name: &VarName, mod_name: &hir::Expr) -> TyCheckResult<()> {
        match mod_name {
            hir::Expr::Lit(lit) => {
                if self.rec_subtype_of(&lit.data.class(), &Str) {
                    let name = enum_unwrap!(lit.data.clone(), ValueObj::Str);
                    match &name[..] {
                        "math" => { self.mods.insert(var_name.clone(), Self::init_py_math_mod()); },
                        "random" => { self.mods.insert(var_name.clone(), Self::init_py_random_mod()); },
                        other => todo!("importing {other}"),
                    }
                } else {
                    return Err(TyCheckError::type_mismatch_error(
                        mod_name.loc(),
                        self.caused_by(),
                        "import::name",
                        &Str,
                        mod_name.ref_t()
                    ))
                }
            },
            _ => {
                return Err(TyCheckError::feature_error(mod_name.loc(), "non-literal importing", self.caused_by()))
            },
        }
        Ok(())
    }

    pub(crate) fn _push_subtype_bound(&mut self, sub: Type, sup: Type) {
        self.bounds.push(TyBound::subtype(sub, sup));
    }

    pub(crate) fn _push_instance_bound(&mut self, name: Str, t: Type) {
        self.bounds.push(TyBound::instance(name, t));
    }
}

// type variable related operations
impl Context {
    pub const TOP_LEVEL: usize = 1;
    // HACK: see doc/compiler/inference.md for details
    pub const GENERIC_LEVEL: usize = usize::MAX;

    /// 型を非依存化する
    fn _independentise<'a>(_t: Type, _ts: &[Type]) -> Type {
        todo!()
    }

    fn _generalize_tp(&self, free: TyParam) -> (TyParam, Set<TyBound>) {
        match free {
            TyParam::FreeVar(v) if v.is_linked() => {
                let bounds: Set<TyBound>;
                if let FreeKind::Linked(tp) = &mut *v.borrow_mut() {
                    (*tp, bounds) = self._generalize_tp(tp.clone());
                } else { assume_unreachable!() }
                (TyParam::FreeVar(v), bounds)
            },
            // TODO: Polymorphic generalization
            TyParam::FreeVar(fv) if fv.level() > Some(self.level)  => {
                match &*fv.borrow() {
                    FreeKind::Unbound{ id, constraint, .. } => {
                        let name = id.to_string();
                        let bound = match constraint {
                            Constraint::SubtypeOf(sup) => TyBound::subtype(Type::mono(name.clone()), sup.clone()),
                            Constraint::SupertypeOf(sub) => TyBound::supertype(Type::mono(name.clone()), sub.clone()),
                            Constraint::Sandwiched{ sub, sup } => TyBound::sandwiched(sub.clone(), Type::mono(name.clone()), sup.clone()),
                            Constraint::TypeOf(t) => TyBound::instance(Str::rc(&name[..]), t.clone()),
                        };
                        (TyParam::mono_q(&name), set!{bound})
                    },
                    FreeKind::NamedUnbound{ name, constraint, .. } => {
                        let bound = match constraint {
                            Constraint::SubtypeOf(sup) => TyBound::subtype(Type::mono(name.clone()), sup.clone()),
                            Constraint::SupertypeOf(sub) => TyBound::supertype(Type::mono(name.clone()), sub.clone()),
                            Constraint::Sandwiched{ sub, sup } => TyBound::sandwiched(sub.clone(), Type::mono(name.clone()), sup.clone()),
                            Constraint::TypeOf(t) => TyBound::instance(Str::rc(&name[..]), t.clone()),
                        };
                        (TyParam::mono_q(name), set!{bound})
                    }
                    _ => assume_unreachable!(),
                }
            },
            other if other.has_no_unbound_var() => (other, set!{}),
            other => todo!("{other}"),
        }
    }

    /// see doc/LANG/compiler/inference.md#一般化 for details
    /// ```
    /// generalize_t(?T) == 'T: Type
    /// generalize_t(?T(<: Nat) -> ?T) == |'T <: Nat| 'T -> 'T
    /// generalize_t(?T(<: Nat) -> Int) == Nat -> Int // 戻り値に現れないなら量化しない
    /// ```
    fn generalize_t(&self, free: Type) -> (Type, Set<TyBound>) {
        match free {
            FreeVar(v) if v.is_linked() => {
                let bounds: Set<TyBound>;
                if let FreeKind::Linked(t) = &mut *v.borrow_mut() {
                    (*t, bounds) = self.generalize_t(t.clone());
                } else { assume_unreachable!() }
                (Type::FreeVar(v), bounds)
            },
            // TODO: Polymorphic generalization
            FreeVar(fv) if fv.level() > Some(self.level)  => {
                match &*fv.borrow() {
                    FreeKind::Unbound{ id, constraint, .. } => {
                        let name = id.to_string();
                        let bound = match constraint {
                            Constraint::SubtypeOf(sup) => TyBound::subtype(Type::mono(name.clone()), sup.clone()),
                            Constraint::SupertypeOf(sub) => TyBound::supertype(Type::mono(name.clone()), sub.clone()),
                            Constraint::Sandwiched{ sub, sup } =>
                                TyBound::sandwiched(sub.clone(), Type::mono(name.clone()), sup.clone()),
                            Constraint::TypeOf(t) => TyBound::instance(Str::rc(&name[..]), t.clone()),
                        };
                        (Type::mono_q(&name), set!{bound})
                    },
                    FreeKind::NamedUnbound{ name, constraint, .. } => {
                        let bound = match constraint {
                            Constraint::SubtypeOf(sup) => TyBound::subtype(Type::mono(name.clone()), sup.clone()),
                            Constraint::SupertypeOf(sub) => TyBound::supertype(Type::mono(name.clone()), sub.clone()),
                            Constraint::Sandwiched{ sub, sup } =>
                                TyBound::sandwiched(sub.clone(), Type::mono(name.clone()), sup.clone()),
                            Constraint::TypeOf(t) => TyBound::instance(Str::rc(&name[..]), t.clone()),
                        };
                        (Type::mono_q(name), set!{bound})
                    }
                    _ => assume_unreachable!(),
                }
            },
            Subr(mut subr) => {
                let mut bounds = set!{};
                let kind = match subr.kind {
                    SubrKind::FuncMethod(self_t) => {
                        let (t, bs) = self.generalize_t(*self_t);
                        bounds.merge(bs);
                        SubrKind::fn_met(t)
                    },
                    SubrKind::ProcMethod { before, after } => {
                        let (before, bs) = self.generalize_t(*before);
                        bounds.merge(bs);
                        if let Some(after) = after {
                            let (after, bs) = self.generalize_t(*after);
                            bounds.merge(bs);
                            SubrKind::pr_met(before, Some(after))
                        } else {
                            SubrKind::pr_met(before, None)
                        }
                    },
                    other => other,
                };
                subr.non_default_params.iter_mut()
                    .for_each(|p| {
                        let (t, bs) = self.generalize_t(mem::take(&mut p.ty));
                        p.ty = t;
                        bounds.merge(bs);
                    });
                subr.default_params.iter_mut()
                    .for_each(|p| {
                        let (t, bs) = self.generalize_t(mem::take(&mut p.ty));
                        p.ty = t;
                        bounds.merge(bs);
                    });
                let (return_t, bs) = self.generalize_t(*subr.return_t);
                bounds.merge(bs);
                (Type::subr(kind, subr.non_default_params, subr.default_params, return_t), bounds)
            },
            // REVIEW: その他何でもそのまま通していいのか?
            other => (other, set!{}),
        }
    }

    pub(crate) fn bounds(&self) -> Set<TyBound> {
        self.impls.iter()
        .filter(|(_, vi)| vi.kind.is_parameter())
        .map(|(name, vi)| TyBound::instance(name.inspect().clone(), vi.t.clone()))
        .collect()
    }

    fn instantiate_tp(quantified: TyParam, tv_ctx: TyVarContext) -> (TyParam, TyVarContext) {
        match quantified {
            TyParam::MonoQVar(n) => {
                if let Some(t) = tv_ctx.get_typaram(&n) {
                    (t.clone(), tv_ctx)
                } else if let Some(_t) = tv_ctx.get_tyvar(&n) {
                    todo!()
                } else {
                    panic!("type parameter {n} is not defined")
                }
            },
            TyParam::UnaryOp{ op, val } => {
                let (res, tv_ctx) = Self::instantiate_tp(*val, tv_ctx);
                (TyParam::unary(op, res), tv_ctx)
            },
            TyParam::BinOp{ op, lhs, rhs } => {
                let (lhs, tv_ctx) = Self::instantiate_tp(*lhs, tv_ctx);
                let (rhs, tv_ctx) = Self::instantiate_tp(*rhs, tv_ctx);
                (TyParam::bin(op, lhs, rhs), tv_ctx)
            },
            TyParam::Type(t) => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                (TyParam::t(t), tv_ctx)
            },
            p @ (TyParam::ConstObj(_) | TyParam::Mono(_)) => (p, tv_ctx),
            other => todo!("{other}"),
        }
    }

    /// 'T -> ?T (quantified to free)
    pub(crate) fn instantiate_t(quantified: Type, mut tv_ctx: TyVarContext) -> (Type, TyVarContext) {
        match quantified {
            MonoQVar(n) => {
                if let Some(t) = tv_ctx.get_tyvar(&n) {
                    (t.clone(), tv_ctx)
                } else if let Some(_t) = tv_ctx.get_typaram(&n) {
                    todo!()
                } else {
                    panic!("the type variable {n} is not defined")
                }
            },
            PolyQVar{ name, mut params } => {
                for param in params.iter_mut() {
                    (*param, tv_ctx) = Self::instantiate_tp(mem::take(param), tv_ctx);
                }
                (Type::poly_q(name, params), tv_ctx)
            },
            Refinement(mut refine) => {
                refine.preds = refine.preds.into_iter().map(|mut pred| {
                    for tp in pred.typarams_mut() {
                        (*tp, tv_ctx) = Self::instantiate_tp(mem::take(tp), tv_ctx.clone());
                    }
                    pred
                }).collect();
                (Type::Refinement(refine), tv_ctx)
            },
            Subr(mut subr) => {
                let kind = match subr.kind {
                    SubrKind::FuncMethod(self_t)  => {
                        let (res, _tv_ctx) = Self::instantiate_t(*self_t, tv_ctx);
                        tv_ctx = _tv_ctx;
                        SubrKind::FuncMethod(Box::new(res))
                    }
                    SubrKind::ProcMethod{ before, after } => {
                        let (before, _tv_ctx) = Self::instantiate_t(*before, tv_ctx);
                        let (after, _tv_ctx) = if let Some(after) = after {
                            let (after, _tv_ctx) = Self::instantiate_t(*after, _tv_ctx);
                            (Some(after), _tv_ctx)
                        } else {
                            (None, _tv_ctx)
                        };
                        tv_ctx = _tv_ctx;
                        SubrKind::pr_met(before, after)
                    }
                    other => other,
                };
                for p in subr.non_default_params.iter_mut() {
                    (p.ty, tv_ctx) = Self::instantiate_t(mem::take(&mut p.ty), tv_ctx);
                }
                for p in subr.default_params.iter_mut() {
                    (p.ty, tv_ctx) = Self::instantiate_t(mem::take(&mut p.ty), tv_ctx);
                }
                let (return_t, tv_ctx) = Self::instantiate_t(*subr.return_t, tv_ctx);
                (Type::subr(kind, subr.non_default_params, subr.default_params, return_t), tv_ctx)
            },
            Type::Array{ t, len } => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                let (len, tv_ctx) = Self::instantiate_tp(len, tv_ctx);
                (Type::array(t, len), tv_ctx)
            },
            Type::Dict{ k, v } => {
                let (k, tv_ctx) = Self::instantiate_t(*k, tv_ctx);
                let (v, tv_ctx) = Self::instantiate_t(*v, tv_ctx);
                (Type::dict(k, v), tv_ctx)
            },
            Tuple(mut ts) => {
                for t in ts.iter_mut() {
                    (*t, tv_ctx) = Self::instantiate_t(mem::take(t), tv_ctx);
                }
                (Type::Tuple(ts), tv_ctx)
            },
            Record(mut dict) => {
                for v in dict.values_mut() {
                    (*v, tv_ctx) = Self::instantiate_t(mem::take(v), tv_ctx);
                }
                (Type::Record(dict), tv_ctx)
            },
            Range(t) => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                (Type::range(t), tv_ctx)
            },
            Iter(t) => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                (Type::iter(t), tv_ctx)
            },
            Option(t) => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                (Type::option(t), tv_ctx)
            },
            OptionMut(t) => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                (Type::option_mut(t), tv_ctx)
            },
            Ref(t) => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                (Type::refer(t), tv_ctx)
            },
            RefMut(t) => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                (Type::ref_mut(t), tv_ctx)
            },
            VarArgs(t) => {
                let (t, tv_ctx) = Self::instantiate_t(*t, tv_ctx);
                (Type::var_args(t), tv_ctx)
            }
            MonoProj{ lhs, rhs } => {
                let (lhs, tv_ctx) = Self::instantiate_t(*lhs, tv_ctx);
                (Type::mono_proj(lhs, rhs), tv_ctx)
            }
            Poly{ name, mut params } => {
                for param in params.iter_mut() {
                    (*param, tv_ctx) = Self::instantiate_tp(mem::take(param), tv_ctx);
                }
                (Type::poly(name, params), tv_ctx)
            }
            other if other.is_monomorphic() => (other, tv_ctx),
            other => todo!("{other}"),
        }
    }

    fn instantiate(&self, quantified: Type, callee: &hir::Expr) -> TyCheckResult<Type> {
        match quantified {
            Quantified(quant) => {
                let tv_ctx = TyVarContext::new(self.level, quant.bounds);
                let (t, _) = Self::instantiate_t(*quant.unbound_callable, tv_ctx);
                match &t {
                    Type::Subr(subr) => {
                        match (subr.kind.self_t(), callee.receiver_t()) {
                            (Some(l), Some(r)) => {
                                self.unify(l, r, None, Some(callee.loc()))?;
                            },
                            // if callee is a Module object or some named one
                            (None, Some(r)) if self.rec_subtype_of(r, &Type::mono("Named")) => {},
                            (None, None) => {},
                            (l, r) => todo!("{l:?}, {r:?}"),
                        }
                    }
                    _ => unreachable!(),
                }
                Ok(t)
            },
            // rank-1制限により、通常の型(rank-0型)の内側に量化型は存在しない
            other => Ok(other),
        }
    }

    /// e.g.
    /// ```
    /// substitute_call(instance: ((?T, ?U) -> ?T), [Int, Str], []) => instance: (Int, Str) -> Int
    /// substitute_call(instance: ((?T, Int) -> ?T), [Int, Nat], []) => instance: (Int, Int) -> Str
    /// substitute_call(instance: ((?M(: Nat)..?N(: Nat)) -> ?M+?N), [1..2], []) => instance: (1..2) -> {3}
    /// substitute_call(instance: ((?L(: Add(?R, ?O)), ?R) -> ?O), [1, 2], []) => instance: (Nat, Nat) -> Nat
    /// ```
    fn substitute_call(&self, callee: &hir::Expr, instance: &Type, pos_args: &[hir::PosArg], kw_args: &[hir::KwArg]) -> TyCheckResult<()> {
        match instance {
            Type::Subr(subr) => {
                let params_len =
                    subr.non_default_params.len()
                    + subr.default_params.len();
                if params_len < pos_args.len() + kw_args.len() {
                    return Err(TyCheckError::too_many_args_error(
                        callee.loc(),
                        &callee.to_string(),
                        self.caused_by(),
                        params_len,
                        pos_args.len(),
                        kw_args.len(),
                    ))
                }
                let mut passed_params = set!{};
                let params = subr.non_default_params.iter().chain(subr.default_params.iter());
                for (param_ty, pos_arg) in params.clone().zip(pos_args) {
                    self.sub_unify(pos_arg.expr.ref_t(), &param_ty.ty, None, Some(pos_arg.loc())).map_err(|e| {
                        // REVIEW:
                        let name = callee.var_full_name().unwrap_or("".to_string());
                        let name = name + "::" + param_ty.name.as_ref().map(|s| &s[..]).unwrap_or("");
                        TyCheckError::type_mismatch_error(
                            e.core.loc,
                            e.caused_by,
                            &name[..],
                            &param_ty.ty,
                            pos_arg.expr.ref_t()
                        )
                    })?;
                    if let Some(name) = &param_ty.name {
                        if passed_params.contains(name) {
                            return Err(TyCheckError::multiple_args_error(
                                callee.loc(),
                                &callee.to_string(),
                                self.caused_by(),
                                name,
                            ))
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
                        self.sub_unify(kw_arg.expr.ref_t(), param_ty, None, Some(kw_arg.loc()))?;
                    } else {
                        return Err(TyCheckError::unexpected_kw_arg_error(
                            kw_arg.keyword.loc(),
                            &callee.to_string(),
                            self.caused_by(),
                            kw_arg.keyword.inspect()
                        ))
                    }
                }
                Ok(())
            },
            other => todo!("{other}"),
        }
    }

    // FIXME:
    fn eliminate_linked_tp(tp: TyParam) -> TyCheckResult<TyParam> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => Ok(fv.unwrap()),
            TyParam::Type(t) => Ok(TyParam::t(Self::eliminate_linked_vars(*t)?)),
            TyParam::App{ name, mut args } => {
                for param in args.iter_mut() {
                    *param = Self::eliminate_linked_tp(mem::take(param))?;
                }
                Ok(TyParam::App{ name, args })
            }
            t => Ok(t),
        }
    }

    // FIXME:
    fn eliminate_linked_vars(t: Type) -> TyCheckResult<Type> {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => Ok(fv.unwrap()),
            // 未連携型変数のチェックはモジュール全体の型検査が終わった後にやる
            // Type::FreeVar(_) =>
            //    Err(TyCheckError::checker_bug(0, Location::Unknown, fn_name!(), line!())),
            Type::Poly{ name, mut params } => {
                for param in params.iter_mut() {
                    *param = Self::eliminate_linked_tp(mem::take(param))?;
                }
                Ok(Type::poly(name, params))
            },
            Type::Array{ mut t, mut len } => {
                let t = Self::eliminate_linked_vars(mem::take(&mut t))?;
                let len = Self::eliminate_linked_tp(mem::take(&mut len))?;
                Ok(Type::array(t, len))
            },
            Type::Subr(mut subr) => {
                match &mut subr.kind {
                    SubrKind::FuncMethod(t) => {
                        *t = Box::new(Self::eliminate_linked_vars(mem::take(t))?);
                    },
                    SubrKind::ProcMethod{ before, after } => {
                        *before = Box::new(Self::eliminate_linked_vars(mem::take(before))?);
                        if let Some(after) = after {
                            *after = Box::new(Self::eliminate_linked_vars(mem::take(after))?);
                        }
                    },
                    _ => {},
                }
                let params = subr.non_default_params.iter_mut().chain(subr.default_params.iter_mut());
                for param in params {
                    param.ty = Self::eliminate_linked_vars(mem::take(&mut param.ty))?;
                }
                subr.return_t = Box::new(Self::eliminate_linked_vars(mem::take(&mut subr.return_t))?);
                Ok(Type::Subr(subr))
            }
            t => Ok(t),
        }
    }

    /// 可変依存型の変更を伝搬させる
    fn propagate(&self, t: &Type, callee: &hir::Expr) -> TyCheckResult<()> {
        match t {
            Type::Subr(subr) => {
                match &subr.kind {
                    SubrKind::ProcMethod{ before: _, after: Some(after) } => {
                        let receiver_t = callee.receiver_t().unwrap();
                        self.reunify(receiver_t, after, Some(callee.loc()), None)?;
                    },
                    _ => {},
                }
            },
            _ => {},
        }
        Ok(())
    }

    fn _occur(&self, _t: Type) -> TyCheckResult<Type> { todo!() }

    /// allow_divergence = trueにすると、Num型変数と±Infの単一化を許す
    pub(crate) fn unify_tp(&self, l: &TyParam, r: &TyParam, bounds: Option<&Set<TyBound>>, allow_divergence: bool) -> TyCheckResult<()> {
        if l.has_no_unbound_var() && r.has_no_unbound_var() && l.rec_eq(r) { return Ok(()) }
        match (l, r) {
            (TyParam::Type(l), TyParam::Type(r)) =>
                self.unify(&l, &r, None, None),
            (
                ltp @ TyParam::FreeVar(lfv),
                rtp @ TyParam::FreeVar(rfv),
            ) if lfv.is_unbound() && rfv.is_unbound() => {
                if lfv.level().unwrap() > rfv.level().unwrap() { lfv.link(rtp); }
                else { rfv.link(ltp); }
                Ok(())
            },
            (TyParam::FreeVar(fv), tp)
            | (tp, TyParam::FreeVar(fv)) => {
                match &*fv.borrow() {
                    FreeKind::Linked(l) => { return self.unify_tp(l, tp, bounds, allow_divergence) },
                    FreeKind::Unbound{ .. } | FreeKind::NamedUnbound{ .. } => {},
                } // &fv is dropped
                let fv_t = fv.borrow()
                    .constraint()
                    .unwrap().typ()
                    .unwrap().clone(); // fvを参照しないよいにcloneする(あとでborrow_mutするため)
                let tp_t = self.eval.get_tp_t(tp, bounds, &self)?;
                if self.rec_supertype_of(&fv_t, &tp_t) {
                    // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                    if fv.level() < Some(self.level) {
                        let new_constraint = Constraint::SubtypeOf(tp_t.clone());
                        if self.is_sub_constraint_of(fv.borrow().constraint().unwrap(), &new_constraint)
                        || fv.borrow().constraint().unwrap().typ() == Some(&Type) {
                            fv.update_constraint(new_constraint);
                        }
                    } else {
                        fv.link(tp);
                    }
                    Ok(())
                } else {
                    if allow_divergence
                    && (
                        self.eq_tp(&tp, &TyParam::value(Inf), None)
                        || self.eq_tp(&tp, &TyParam::value(NegInf), None)
                    ) && self.rec_subtype_of(&fv_t, &Type::mono("Num")) {
                        fv.link(tp);
                        Ok(())
                    } else {
                        Err(TyCheckError::unreachable(fn_name!(), line!()))
                    }
                }
            },
            (   TyParam::UnaryOp{ op: lop, val: lval },
                TyParam::UnaryOp{ op: rop, val: rval }
            ) if lop == rop => {
                self.unify_tp(lval, rval, bounds, allow_divergence)
            },
            (
                TyParam::BinOp{ op: lop, lhs, rhs },
                TyParam::BinOp{ op: rop, lhs: lhs2, rhs: rhs2 }
            ) if lop == rop => {
                self.unify_tp(lhs, lhs2, bounds, allow_divergence)?;
                self.unify_tp(rhs, rhs2, bounds, allow_divergence)
            },
            (l, r) => panic!("type-parameter unification failed:\nl:{l}\nr: {r}"),
        }
    }

    fn reunify_tp(&self, before: &TyParam, after: &TyParam, bounds: Option<&Set<TyBound>>) -> TyCheckResult<()> {
        match (before, after) {
            (TyParam::ConstObj(ConstObj::MutValue(l)),TyParam::ConstObj(ConstObj::Value(r))) => {
                *l.borrow_mut() = r.clone();
                Ok(())
            },
            (TyParam::ConstObj(ConstObj::MutValue(l)),TyParam::ConstObj(ConstObj::MutValue(r))) => {
                *l.borrow_mut() = r.borrow().clone();
                Ok(())
            },
            (TyParam::Type(l), TyParam::Type(r)) =>
                self.reunify(&l, &r, None, None),
            (   TyParam::UnaryOp{ op: lop, val: lval },
                TyParam::UnaryOp{ op: rop, val: rval }
            ) if lop == rop => {
                self.reunify_tp(lval, rval, bounds)
            },
            (
                TyParam::BinOp{ op: lop, lhs, rhs },
                TyParam::BinOp{ op: rop, lhs: lhs2, rhs: rhs2 }
            ) if lop == rop => {
                self.reunify_tp(lhs, lhs2, bounds)?;
                self.reunify_tp(rhs, rhs2, bounds)
            },
            (l, r) if self.eq_tp(l, r, None) => Ok(()),
            (l, r) => panic!("type-parameter re-unification failed:\nl: {l}\nr: {r}"),
        }
    }

    /// predは正規化されているとする
    fn unify_pred(&self, l_pred: &Predicate, r_pred: &Predicate) -> TyCheckResult<()> {
        match (l_pred, r_pred) {
            (Pred::Value(_), Pred::Value(_))
            | (Pred::Const(_), Pred::Const(_)) => Ok(()),
            (Pred::Equal{ rhs, .. }, Pred::Equal{ rhs: rhs2, .. })
            | (Pred::GreaterEqual{ rhs, .. }, Pred::GreaterEqual{ rhs: rhs2, .. })
            | (Pred::LessEqual{ rhs, .. }, Pred::LessEqual{ rhs: rhs2, .. })
            | (Pred::NotEqual{ rhs, .. }, Pred::NotEqual{ rhs: rhs2, .. }) =>
                self.unify_tp(rhs, rhs2, None, false),
            (Pred::And(l1, r1), Pred::And(l2, r2))
            | (Pred::Or(l1, r1), Pred::Or(l2, r2))
            | (Pred::Not(l1, r1), Pred::Not(l2, r2)) =>
                match (self.unify_pred(l1, l2), self.unify_pred(r1, r2)) {
                    (Ok(()), Ok(())) => Ok(()),
                    (Ok(()), Err(e)) | (Err(e), Ok(()))
                    | (Err(e), Err(_)) => Err(e),
                }
            // unify({I >= 0}, {I >= ?M and I <= ?N}): ?M => 0, ?N => Inf
            (Pred::GreaterEqual{ rhs, .. }, Pred::And(l , r))
            | (Predicate::And(l, r), Pred::GreaterEqual{ rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual{ rhs: ge_rhs, .. }, Pred::LessEqual{ rhs: le_rhs, .. })
                | (Pred::LessEqual{ rhs: le_rhs, .. }, Pred::GreaterEqual{ rhs: ge_rhs, .. }) => {
                    self.unify_tp(rhs, ge_rhs, None, false)?;
                    self.unify_tp(le_rhs, &TyParam::value(Inf), None, true)
                },
                _ => Err(TyCheckError::pred_unification_error(l_pred, r_pred, self.caused_by())),
            },
            (Pred::LessEqual{ rhs, .. }, Pred::And(l , r))
            | (Pred::And(l, r), Pred::LessEqual{ rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual{ rhs: ge_rhs, .. }, Pred::LessEqual{ rhs: le_rhs, .. })
                | (Pred::LessEqual{ rhs: le_rhs, .. }, Pred::GreaterEqual{ rhs: ge_rhs, .. }) => {
                    self.unify_tp(rhs, le_rhs, None, false)?;
                    self.unify_tp(ge_rhs, &TyParam::value(NegInf), None, true)
                },
                _ => Err(TyCheckError::pred_unification_error(l_pred, r_pred, self.caused_by())),
            },
            (Pred::Equal{ rhs, .. }, Pred::And(l , r))
            | (Pred::And(l, r), Pred::Equal{ rhs, .. }) => match (l.as_ref(), r.as_ref()) {
                (Pred::GreaterEqual{ rhs: ge_rhs, .. }, Pred::LessEqual{ rhs: le_rhs, .. })
                | (Pred::LessEqual{ rhs: le_rhs, .. }, Pred::GreaterEqual{ rhs: ge_rhs, .. }) => {
                    self.unify_tp(rhs, le_rhs, None, false)?;
                    self.unify_tp(rhs, ge_rhs, None, false)
                },
                _ => Err(TyCheckError::pred_unification_error(l_pred, r_pred, self.caused_by())),
            },
            _ => Err(TyCheckError::pred_unification_error(l_pred, r_pred, self.caused_by())),
        }
    }

    /// By default, all type variables are instances of Class ('T: Nominal)
    /// So `unify(?T, Int); unify(?T, Bool)` will causes an error
    /// To bypass the constraint, you need to specify `'T: Structural` in the type bounds
    pub(crate) fn unify(&self, lhs_t: &Type, rhs_t: &Type, lhs_loc: Option<Location>, rhs_loc: Option<Location>) -> TyCheckResult<()> {
        if lhs_t.has_no_unbound_var()
        && rhs_t.has_no_unbound_var()
        && self.rec_supertype_of(lhs_t, rhs_t) { return Ok(()) }
        match (lhs_t, rhs_t) {
            // unify(?T[2], ?U[3]): ?U[3] => ?T[2]
            // bind the higher level var to lower one
            (lt @ Type::FreeVar(lfv), rt @ Type::FreeVar(rfv))
            if lfv.is_unbound() && rfv.is_unbound() => {
                if lfv.level().unwrap() > rfv.level().unwrap() { lfv.link(rt); }
                else { rfv.link(lt); }
                Ok(())
            },
            // unify(?L(<: Add(?R, ?O)), Nat): (?R => Nat, ?O => Nat, ?L => Nat)
            // unify(?A(<: Mutate), [?T; 0]): (?A => [?T; 0])
            (Type::FreeVar(fv), t)
            | (t, Type::FreeVar(fv)) => {
                match &mut *fv.borrow_mut() {
                    FreeKind::Linked(l) => { return self.unify(l, t, lhs_loc, rhs_loc) },
                    FreeKind::Unbound{ lev, constraint, .. }
                    | FreeKind::NamedUnbound{ lev, constraint, .. } => {
                        t.update_level(*lev);
                        // TODO: constraint.type_of()
                        if let Some(sup) = constraint.super_type_mut() {
                            // 下のような場合は制約を弱化する
                            // unify(?T(<: Nat), Int): (?T(<: Int))
                            if self.rec_subtype_of(sup, t) {
                                *sup = t.clone();
                            } else {
                                self.sub_unify(t, sup, rhs_loc, lhs_loc)?;
                            }
                        }
                    },
                } // &fv is dropped
                let new_constraint = Constraint::SubtypeOf(t.clone());
                // 外部未連携型変数の場合、linkしないで制約を弱めるだけにする(see compiler/inference.md)
                // fv == ?T(: Type)の場合は?T(<: U)にする
                if fv.level() < Some(self.level) {
                    if self.is_sub_constraint_of(fv.borrow().constraint().unwrap(), &new_constraint)
                    || fv.borrow().constraint().unwrap().typ() == Some(&Type) {
                        fv.update_constraint(new_constraint);
                    }
                } else {
                    fv.link(t);
                }
                Ok(())
            },
            (Type::Refinement(l), Type::Refinement(r)) => {
                if !self.supertype_of(&l.t, &r.t, None) && !self.supertype_of(&r.t, &l.t, None) {
                    return Err(TyCheckError::unification_error(lhs_t, rhs_t, lhs_loc, rhs_loc, self.caused_by()))
                }
                // FIXME: 正規化する
                for l_pred in l.preds.iter() {
                    for r_pred in r.preds.iter() {
                        self.unify_pred(l_pred, r_pred)?;
                    }
                }
                Ok(())
            },
            (Type::Refinement(_), r) => {
                let rhs_t = self.into_refinement(r.clone());
                self.unify(lhs_t, &Type::Refinement(rhs_t), lhs_loc, rhs_loc)
            },
            (l, Type::Refinement(_)) => {
                let lhs_t = self.into_refinement(l.clone());
                self.unify(&Type::Refinement(lhs_t), rhs_t, lhs_loc, rhs_loc)
            },
            (Type::Subr(ls), Type::Subr(rs)) if ls.kind.same_kind_as(&rs.kind) => {
                if let (Some(l), Some(r)) = (ls.kind.self_t(), rs.kind.self_t()) {
                    self.unify(l, r, lhs_loc, rhs_loc)?;
                }
                for (l, r) in ls.non_default_params.iter().zip(rs.non_default_params.iter()) {
                    self.unify(&l.ty, &r.ty, lhs_loc, rhs_loc)?;
                }
                self.unify(&ls.return_t, &rs.return_t, lhs_loc, rhs_loc)
            },
            (Range(l), Range(r))
            | (Iter(l), Iter(r))
            | (Type::Ref(l), Type::Ref(r))
            | (Type::RefMut(l), Type::RefMut(r))
            | (Type::Option(l), Type::Option(r))
            | (OptionMut(l), OptionMut(r))
            | (VarArgs(l), VarArgs(r)) => self.unify(l, r, lhs_loc, rhs_loc),
            // REVIEW:
            (Type::Ref(l), r)
            | (Type::RefMut(l), r) => self.unify(l, r, lhs_loc, rhs_loc),
            (l, Type::Ref(r))
            | (l, Type::RefMut(r)) => self.unify(l, r, lhs_loc, rhs_loc),
            (Type::Poly{ name: ln, params: lps }, Type::Poly{ name: rn, params: rps }) => {
                if ln != rn { return Err(TyCheckError::unification_error(lhs_t, rhs_t, lhs_loc, rhs_loc, self.caused_by())) }
                for (l, r) in lps.iter().zip(rps.iter()) {
                    self.unify_tp(l, r, None, false)?;
                }
                Ok(())
            },
            (Type::Poly{ name: _, params: _ }, _r) => {
                todo!()
            },
            (l, r) =>
                Err(TyCheckError::unification_error(l, r, lhs_loc, rhs_loc, self.caused_by())),
        }
    }

    /// T: Array(Int, !0), U: Array(Int, !1)
    /// reunify(T, U):
    /// T: Array(Int, !1), U: Array(Int, !1)
    pub(crate) fn reunify(&self, before_t: &Type, after_t: &Type, bef_loc: Option<Location>, aft_loc: Option<Location>) -> TyCheckResult<()> {
        match (before_t, after_t) {
            (Type::FreeVar(fv), r) if fv.is_linked() =>
                self.reunify(&fv.crack(), r, bef_loc, aft_loc),
            (l, Type::FreeVar(fv)) if fv.is_linked() =>
                self.reunify(l, &fv.crack(), bef_loc, aft_loc),
            (Type::Range(l), Type::Range(r))
            | (Type::Iter(l), Type::Iter(r))
            | (Type::Ref(l), Type::Ref(r))
            | (Type::RefMut(l), Type::RefMut(r))
            | (Type::Option(l), Type::Option(r))
            | (Type::OptionMut(l), Type::OptionMut(r))
            | (Type::VarArgs(l), Type::VarArgs(r)) => self.reunify(l, r, bef_loc, aft_loc),
            // REVIEW:
            (Type::Ref(l), r)
            | (Type::RefMut(l), r) => self.reunify(l, r, bef_loc, aft_loc),
            (l, Type::Ref(r))
            | (l, Type::RefMut(r)) => self.reunify(l, r, bef_loc, aft_loc),
            (Type::Poly{ name: ln, params: lps }, Type::Poly{ name: rn, params: rps }) => {
                if ln != rn {
                    let before_t = Type::poly(ln.clone(), lps.clone());
                    return Err(TyCheckError::re_unification_error(&before_t, after_t, bef_loc, aft_loc, self.caused_by()))
                }
                for (l, r) in lps.iter().zip(rps.iter()) {
                    self.reunify_tp(l, r, None)?;
                }
                Ok(())
            },
            (l, r) if self.same_type_of(l, r, None) => Ok(()),
            (l, r) =>
                Err(TyCheckError::re_unification_error(l, r, bef_loc, aft_loc, self.caused_by())),
        }
    }

    /// Assuming that `sub` is a subtype of `sup`, fill in the type variable to satisfy the assumption
    /// ```
    /// sub_unify({I: Int | I == 0}, ?T(<: Ord)): (/* OK */)
    /// sub_unify(Int, ?T(:> Nat)): (?T :> Int)
    /// sub_unify(Nat, ?T(:> Int)): (/* OK */)
    /// sub_unify(Nat, Add(?R, ?O)): (?R => Nat, ?O => Nat)
    /// sub_unify([?T; 0], Mutate): (/* OK */)
    /// ```
    fn sub_unify(&self, maybe_sub: &Type, maybe_sup: &Type, sub_loc: Option<Location>, sup_loc: Option<Location>) -> TyCheckResult<()> {
        erg_common::fmt_dbg!(maybe_sub, maybe_sup,);
        let maybe_sub_is_sub = self.rec_subtype_of(maybe_sub, maybe_sup);
        if maybe_sub.has_no_unbound_var()
        && maybe_sup.has_no_unbound_var()
        && maybe_sub_is_sub { return Ok(()) }
        if !maybe_sub_is_sub {
            let loc = sub_loc.or(sup_loc).unwrap_or(Location::Unknown);
            return Err(TyCheckError::type_mismatch_error(
                loc,
                self.caused_by(),
                "<???>",
                maybe_sup,
                maybe_sub,
            ))
        }
        match (maybe_sub, maybe_sup) {
            (l, Type::FreeVar(fv)) if fv.is_unbound() => {
                match &mut *fv.borrow_mut() {
                    FreeKind::NamedUnbound{ constraint, .. }
                    | FreeKind::Unbound{ constraint, .. } => match constraint {
                        // sub_unify(Nat, ?T(:> Int)): (/* OK */)
                        // sub_unify(Int, ?T(:> Nat)): (?T :> Int)
                        Constraint::SupertypeOf(sub) if self.rec_supertype_of(l, sub) => {
                            *constraint = Constraint::SupertypeOf(l.clone());
                        },
                        // sub_unify(Nat, ?T(<: Int)): (/* OK */)
                        // sub_unify(Int, ?T(<: Nat)): Error!
                        Constraint::SubtypeOf(sup) if self.rec_supertype_of(l, sup) => {
                            return Err(TyCheckError::subtyping_error(l, sup, sub_loc, sup_loc, self.caused_by()))
                        },
                        // sub_unify(Nat, (Ratio :> ?T :> Int)): (/* OK */)
                        // sub_unify(Int, (Ratio :> ?T :> Nat)): (Ratio :> ?T :> Int)
                        Constraint::Sandwiched{ sub, sup } if self.rec_supertype_of(l, sub) => {
                            *constraint = Constraint::Sandwiched{ sub: l.clone(), sup: mem::take(sup) };
                        },
                        _ => {}
                    },
                    _ => {},
                }
                return Ok(())
            },
            (Type::FreeVar(fv), _r) if fv.is_linked() => todo!(),
            (l @ Refinement(_), r @ Refinement(_)) => {
                return self.unify(l ,r, sub_loc, sup_loc)
            },
            _ => {}
        }
        let mut opt_smallest = None;
        for ctx in self.get_sorted_supertypes(maybe_sub) {
            let instances = ctx.super_classes.iter()
                .chain(ctx.super_traits.iter())
                .filter(|t| self.supertype_of(maybe_sup, t, None));
            // instanceが複数ある場合、経験的に最も小さい型を選ぶのが良い
            // これでうまくいかない場合は型指定してもらう(REVIEW: もっと良い方法があるか?)
            if let Some(t) = self.smallest_ref_t(instances) {
                opt_smallest = if let Some(small) = opt_smallest { self.min(small, t) } else { Some(t) };
            }
        }
        let glue_patch_and_types = self.rec_get_glue_patch_and_types();
        let patch_instances = glue_patch_and_types.iter()
            .filter_map(|(patch_name, l, r)| {
                let patch = self.rec_get_patch(patch_name).unwrap();
                let bounds = patch.bounds();
                if self.supertype_of(l, maybe_sub, Some(&bounds))
                && self.supertype_of(r, maybe_sup, Some(&bounds)) {
                    let tv_ctx = TyVarContext::new(self.level, bounds);
                    let (l, _) = Self::instantiate_t(l.clone(), tv_ctx.clone());
                    let (r, _) = Self::instantiate_t(r.clone(), tv_ctx);
                    Some((l, r))
                } else { None }
            });
        let opt_smallest_pair = self.smallest_pair(patch_instances);
        match (opt_smallest, opt_smallest_pair) {
            (Some(smallest), Some((l, r))) => {
                if self.min(smallest, &r) == Some(&r) {
                    self.unify(maybe_sub, &l, sub_loc, None)?;
                    self.unify(maybe_sup, &r, sup_loc, None)
                } else {
                    self.unify(maybe_sup, smallest, sup_loc, None)
                }
            },
            (Some(smallest), None) => {
                self.unify(maybe_sup, smallest, sup_loc, None)
            },
            (None, Some((l, r))) => {
                self.unify(maybe_sub, &l, sub_loc, None)?;
                self.unify(maybe_sup, &r, sup_loc, None)?;
                Ok(())
            },
            (None, None) => {
                log!("{maybe_sub}, {maybe_sup}");
                todo!()
            }
        }
    }
}

// (type) getters & validators
impl Context {
    fn validate_var_sig_t(&self, sig: &ast::VarSignature, body_t: &Type, mode: RegistrationMode) -> TyCheckResult<()> {
        let spec_t = self.instantiate_var_sig_t(sig, None, mode)?;
        match &sig.pat {
            ast::VarPattern::Discard(token) => {
                if self.unify(&spec_t, body_t, None, Some(sig.loc())).is_err() {
                    return Err(TyCheckError::type_mismatch_error(
                        token.loc(), self.caused_by(), "_", &spec_t, body_t,
                    ))
                }
            },
            ast::VarPattern::VarName(n) => {
                if self.unify(&spec_t, body_t, None, Some(sig.loc())).is_err() {
                    return Err(TyCheckError::type_mismatch_error(
                        n.loc(), self.caused_by(), n.inspect(), &spec_t, body_t,
                    ))
                }
            }
            ast::VarPattern::Array(a) => {
                for (elem, inf_elem_t) in a.iter().zip(body_t.inner_ts().iter()) {
                    self.validate_var_sig_t(elem, inf_elem_t, mode)?;
                }
            },
            _ => todo!(),
        }
        Ok(())
    }

    pub(crate) fn instantiate_var_sig_t(&self, sig: &ast::VarSignature, opt_t: Option<Type>, mode: RegistrationMode) -> TyCheckResult<Type> {
        let ty = if let Some(s) = sig.t_spec.as_ref() {
            self.instantiate_typespec(s, mode)?
        } else { Type::free_var(self.level, Constraint::TypeOf(Type)) };
        if let Some(t) = opt_t {
            self.unify(&ty, &t, sig.t_spec.as_ref().map(|s| s.loc()), None)?;
        }
        Ok(ty)
    }

    pub(crate) fn instantiate_sub_sig_t(&self, sig: &ast::SubrSignature, opt_ret_t: Option<Type>, mode: RegistrationMode) -> TyCheckResult<Type> {
        let non_defaults = sig.params.non_defaults.iter()
            .map(|p| ParamTy::new(
                p.inspect().cloned(),
                self.instantiate_param_sig_t(p, None, mode).unwrap()
            )).collect::<Vec<_>>();
        let defaults = sig.params.defaults.iter()
            .map(|p| ParamTy::new(
                p.inspect().cloned(), self.instantiate_param_sig_t(p, None, mode).unwrap()
            )).collect::<Vec<_>>();
        let return_t = if let Some(s) = sig.return_t_spec.as_ref() {
            self.instantiate_typespec(s, mode)?
        } else {
            // preregisterならouter scopeで型宣言(see inference.md)
            let level = if mode == PreRegister { self.level } else { self.level+1 };
            Type::free_var(level, Constraint::TypeOf(Type))
        };
        if let Some(ret_t) = opt_ret_t {
            self.unify(&return_t, &ret_t, sig.return_t_spec.as_ref().map(|s| s.loc()), None)?;
        }
        Ok(
            if sig.name.is_procedural() { Type::proc(non_defaults, defaults, return_t) }
            else { Type::func(non_defaults, defaults, return_t) }
        )
    }

    /// spec_t == Noneかつリテラル推論が不可能なら型変数を発行する
    pub(crate) fn instantiate_param_sig_t(&self, sig: impl ParamSig, opt_decl_t: Option<&ParamTy>, mode: RegistrationMode) -> TyCheckResult<Type> {
        let t = if let Some(spec) = sig.t_spec() {
            self.instantiate_typespec(spec, mode)?
        } else {
            match sig.pat() {
                ast::ParamPattern::Lit(lit) => Type::enum_t(set![self.eval.eval_const_lit(lit)]),
                // TODO: Array<Lit>
                _ => {
                    let level = if mode == PreRegister { self.level } else { self.level+1 };
                    Type::free_var(level, Constraint::TypeOf(Type))
                },
            }
        };
        if let Some(decl_t) = opt_decl_t {
            self.unify(&t, &decl_t.ty, sig.t_spec().map(|s| s.loc()), None)?;
        }
        Ok(t)
    }

    pub(crate) fn instantiate_predecl_t(&self, _predecl: &PreDeclTypeSpec) -> TyCheckResult<Type> {
        match _predecl {
            ast::PreDeclTypeSpec::Simple(simple) => self.instantiate_simple_t(simple),
            _ => todo!(),
        }
    }

    pub(crate) fn instantiate_simple_t(&self, simple: &SimpleTypeSpec) -> TyCheckResult<Type> {
        match &simple.name.inspect()[..] {
            "Nat" => Ok(Type::Nat),
            "Nat!" => Ok(Type::NatMut),
            "Int" => Ok(Type::Int),
            "Int!" => Ok(Type::IntMut),
            "Ratio" => Ok(Type::Ratio),
            "Ratio!" => Ok(Type::RatioMut),
            "Float" => Ok(Type::Float),
            "Float!" => Ok(Type::FloatMut),
            "Str" => Ok(Type::Str),
            "Str!" => Ok(Type::StrMut),
            "Bool" => Ok(Type::Bool),
            "Bool!" => Ok(Type::BoolMut),
            "None" => Ok(Type::NoneType),
            "Ellipsis" => Ok(Type::Ellipsis),
            "NotImplemented" => Ok(Type::NotImplemented),
            "Inf" => Ok(Type::Inf),
            "Obj" => Ok(Type::Obj),
            "Obj!" => Ok(Type::ObjMut),
            "Array" => {
                // TODO: kw
                let mut args = simple.args.pos_args();
                if let Some(first) = args.next() {
                    let t = self.instantiate_const_expr_as_type(&first.expr)?;
                    let len = args.next().unwrap();
                    let len = self.instantiate_const_expr(&len.expr);
                    Ok(Type::array(t, len))
                } else {
                    Ok(Type::ArrayCommon)
                }
            }
            other if simple.args.is_empty() => Ok(Type::mono(Str::rc(other))),
            other => {
                // FIXME: kw args
                let params = simple.args.pos_args().map(|arg| {
                    match &arg.expr {
                        ast::ConstExpr::Lit(lit) => TyParam::ConstObj(ConstObj::Value(ValueObj::from(lit))),
                        _ => { todo!() }
                    }
                });
                Ok(Type::poly(Str::rc(other), params.collect()))
            },
        }
    }

    pub(crate) fn instantiate_const_expr(&self, expr: &ast::ConstExpr) -> TyParam {
        match expr {
            ast::ConstExpr::Lit(lit) => TyParam::ConstObj(ConstObj::Value(ValueObj::from(&lit.token))),
            ast::ConstExpr::Accessor(ast::ConstAccessor::Local(name)) => TyParam::Mono(name.inspect().clone()),
            _ => todo!(),
        }
    }

    pub(crate) fn instantiate_const_expr_as_type(&self, expr: &ast::ConstExpr) -> TyCheckResult<Type> {
        match expr {
            ast::ConstExpr::Accessor(ast::ConstAccessor::Local(name)) => Ok(Type::mono(name.inspect())),
            _ => todo!(),
        }
    }

    fn instantiate_func_param_spec(&self, p: &ParamTySpec, mode: RegistrationMode) -> TyCheckResult<ParamTy> {
        let t = self.instantiate_typespec(&p.ty, mode)?;
        Ok(ParamTy::new(p.name.as_ref().map(|t| t.inspect().to_owned()), t))
    }

    pub(crate) fn instantiate_typespec(&self, spec: &TypeSpec, mode: RegistrationMode) -> TyCheckResult<Type> {
        match spec {
            TypeSpec::PreDeclTy(predecl) => self.instantiate_predecl_t(predecl),
            // TODO: Flatten
            TypeSpec::And(lhs, rhs) =>
                Ok(Type::And(vec![self.instantiate_typespec(lhs, mode)?, self.instantiate_typespec(rhs, mode)?])),
            TypeSpec::Not(lhs, rhs) =>
                Ok(Type::Not(vec![self.instantiate_typespec(lhs, mode)?, self.instantiate_typespec(rhs, mode)?])),
            TypeSpec::Or(lhs, rhs) =>
                Ok(Type::Or(vec![self.instantiate_typespec(lhs, mode)?, self.instantiate_typespec(rhs, mode)?])),
            TypeSpec::Array { .. } => todo!(),
            // FIXME: unwrap
            TypeSpec::Tuple(tys) =>
                Ok(Type::Tuple(tys.iter().map(|spec| self.instantiate_typespec(spec, mode).unwrap()).collect())),
            // TODO: エラー処理(リテラルでない、ダブりがある)はパーサーにやらせる
            TypeSpec::Enum(set) => Ok(Type::enum_t(
                set.pos_args().map(|arg| if let ast::ConstExpr::Lit(lit) = &arg.expr {
                    ValueObj::from(lit)
                } else { todo!() }).collect::<Set<_>>()
            )),
            TypeSpec::Interval{ op, lhs, rhs } => {
                let op = match op.kind {
                    TokenKind::Closed => IntervalOp::Closed,
                    TokenKind::LeftOpen => IntervalOp::LeftOpen,
                    TokenKind::RightOpen => IntervalOp::RightOpen,
                    TokenKind::Open => IntervalOp::Open,
                    _ => assume_unreachable!(),
                };
                let l = self.instantiate_const_expr(lhs);
                let l = self.eval.eval_tp(&l, self)?;
                let r = self.instantiate_const_expr(rhs);
                let r = self.eval.eval_tp(&r, self)?;
                if let Some(Greater) = self.try_cmp(&l, &r, None) {
                    panic!("{l}..{r} is not a valid interval type (should be lhs <= rhs)")
                }
                Ok(Type::int_interval(op, l, r))
            },
            TypeSpec::Subr(subr) => {
                let non_defaults = try_map(subr.non_defaults.iter(), |p| self.instantiate_func_param_spec(p, mode))?;
                let defaults = try_map(subr.defaults.iter(), |p| self.instantiate_func_param_spec(p, mode))?;
                let return_t = self.instantiate_typespec(&subr.return_t, mode)?;
                Ok(Type::subr(subr.kind.clone(), non_defaults, defaults, return_t))
            },
        }
    }

    pub(crate) fn instantiate_ty_bound(&self, bound: &TypeBoundSpec, mode: RegistrationMode) -> TyCheckResult<TyBound> {
        // REVIEW: 型境界の左辺に来れるのは型変数だけか?
        // TODO: 高階型変数
        match bound {
            TypeBoundSpec::Subtype{ sub, sup } =>
                Ok(TyBound::subtype(Type::mono_q(sub.inspect().clone()), self.instantiate_typespec(sup, mode)?)),
            TypeBoundSpec::Instance{ name, ty } =>
                Ok(TyBound::instance(name.inspect().clone(), self.instantiate_typespec(ty, mode)?)),
        }
    }

    pub(crate) fn instantiate_ty_bounds(&self, bounds: &TypeBoundSpecs, mode: RegistrationMode) -> TyCheckResult<Set<TyBound>> {
        let mut new_bounds = set!{};
        for bound in bounds.iter() {
            new_bounds.insert(self.instantiate_ty_bound(bound, mode)?);
        }
        Ok(new_bounds)
    }

    pub(crate) fn get_current_scope_local_var(&self, name: &str) -> Option<&VarInfo> {
        self.impls.get(name).or_else(|| self.decls.get(name))
    }

    fn get_context(&self, obj: &hir::Expr, kind: Option<ContextKind>, namespace: &Str) -> TyCheckResult<&Context> {
        match obj {
            hir::Expr::Accessor(hir::Accessor::Local(name)) => {
                if kind == Some(ContextKind::Module) {
                    if let Some(ctx) = self.rec_get_mod(name.inspect()) {
                        Ok(ctx)
                    } else {
                        Err(TyCheckError::no_var_error(
                            obj.loc(),
                            namespace.clone(),
                            name.inspect(),
                            self.get_similar_name(name.inspect()),
                        ))
                    }
                } else { todo!() }
            },
            _ => todo!(),
        }
    }

    fn get_match_call_t(&self, pos_args: &[hir::PosArg], kw_args: &[hir::KwArg]) -> TyCheckResult<Type> {
        if !kw_args.is_empty() { todo!() }
        for pos_arg in pos_args.iter().skip(1) {
            let t = pos_arg.expr.ref_t();
            if !matches!(&pos_arg.expr, hir::Expr::Lambda(_)) {
                return Err(TyCheckError::type_mismatch_error(
                    pos_arg.loc(),
                    self.caused_by(),
                    "match",
                    &Type::mono("LambdaFunc"),
                    &t,
                ))
            }
        }
        let expr_t = pos_args[0].expr.ref_t();
        // Never or T => T
        let mut union_pat_t = Type::Never;
        for (i, a) in pos_args.iter().skip(1).enumerate() {
            let lambda = erg_common::enum_unwrap!(&a.expr, hir::Expr::Lambda);
            if !lambda.params.defaults.is_empty() { todo!() }
            if lambda.params.len() != 1 {
                return Err(TyCheckError::argument_error(
                    pos_args[i+1].loc(),
                    self.caused_by(),
                    1,
                    pos_args[i+1].expr.ref_t().typaram_len(),
                ))
            }
            let rhs = self.instantiate_param_sig_t(&lambda.params.non_defaults[0], None, Normal)?;
            union_pat_t = self.union(&union_pat_t, &rhs);
        }
        // NG: expr_t: Nat, union_pat_t: {1, 2}
        // OK: expr_t: Int, union_pat_t: {1} | 'T
        if expr_t.has_no_unbound_var()
            && self.supertype_of(&expr_t, &union_pat_t, None)
            && !self.supertype_of(&union_pat_t, &expr_t, None) {
            return Err(TyCheckError::match_error(
                pos_args[0].loc(),
                self.caused_by(),
                &expr_t,
            ))
        }
        let branch_ts = pos_args.iter().skip(1)
            .map(|a| ParamTy::anonymous(a.expr.ref_t().clone())).collect::<Vec<_>>();
        let mut return_t = branch_ts[0].ty.return_t().unwrap().clone();
        for arg_t in branch_ts.iter().skip(1) {
            return_t = self.union(&return_t, arg_t.ty.return_t().unwrap());
        }
        let expr_t = if expr_t.has_unbound_var() { union_pat_t } else { expr_t.clone() };
        let param_ts = [
            vec![ParamTy::anonymous(expr_t)],
            branch_ts.iter().map(|pt| pt.clone()).collect()
        ].concat();
        let t = Type::func(param_ts, vec![], return_t);
        Ok(t)
    }

    pub(crate) fn get_local_uniq_obj_name(&self, name: &Token) -> Option<Str> {
        // TODO: types, functions, patches
        if let Some(ctx) = self.rec_get_mod(name.inspect()) {
                return Some(ctx.name.clone())
        }
        None
    }

    pub(crate) fn get_local_t(&self, name: &Token, namespace: &Str) -> TyCheckResult<Type> {
        if let Some(vi) = self.impls.get(name.inspect())
            .or_else(|| self.decls.get(name.inspect())) {
                Ok(vi.t())
        } else {
            if let Some(parent) = self.outer.as_ref() {
                return parent.get_local_t(name, namespace)
            }
            Err(TyCheckError::no_var_error(
                name.loc(),
                namespace.clone(),
                name.inspect(),
                self.get_similar_name(name.inspect()),
            ))
        }
    }

    pub(crate) fn get_attr_t(&self, obj: &hir::Expr, name: &Token, namespace: &Str) -> TyCheckResult<Type> {
        let self_t = obj.t();
        match self_t {
            ASTOmitted => panic!(),
            Type => todo!(),
            Type::Record(_) => todo!(),
            Module => {
                let mod_ctx = self.get_context(obj, Some(ContextKind::Module), namespace)?;
                let t = mod_ctx.get_local_t(name, namespace)?;
                return Ok(t)
            },
            _ => {}
        }
        for ctx in self.get_sorted_supertypes(&self_t) {
            if let Ok(t) = ctx.get_local_t(name, namespace) {
                return Ok(t)
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.outer.as_ref() {
            parent.get_attr_t(obj, name, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                name.loc(),
                namespace.clone(),
                &self_t,
                name.inspect(),
                self.get_similar_attr(&self_t, name.inspect()),
            ))
        }
    }

    /// 戻り値ではなく、call全体の型を返す
    /// objは現時点ではAccessorのみ対応
    /// 受け入れるobj(Accessor)はcheckしてないハリボテ
    fn search_call_t(&self, callee: &hir::Expr, namespace: &Str) -> TyCheckResult<Type> {
        match callee {
            hir::Expr::Accessor(hir::Accessor::Local(local)) => {
                self.get_local_t(&local.name, namespace)
            },
            hir::Expr::Accessor(hir::Accessor::Attr(attr)) => {
                self.get_attr_t(&attr.obj, &attr.name, namespace)
            }
            _ => todo!(),
        }
    }

    pub(crate) fn get_binop_t(&self, op: &Token, args: &[hir::PosArg], namespace: &Str) -> TyCheckResult<Type> {
        erg_common::debug_power_assert!(args.len() == 2);
        let symbol = Token::symbol(binop_to_dname(op.inspect()));
        let mut op = hir::Expr::Accessor(hir::Accessor::local(symbol, Type::ASTOmitted));
        self.get_call_t(&mut op, args, &[], namespace).map_err(|e| {
            // HACK: dname.loc()はダミーLocationしか返さないので、エラーならop.loc()で上書きする
            let core = ErrorCore::new(e.core.errno, e.core.kind, op.loc(), e.core.desc, e.core.hint);
            TyCheckError::new(core, e.caused_by)
        })
    }

    pub(crate) fn get_unaryop_t(&self, op: &Token, args: &[hir::PosArg], namespace: &Str) -> TyCheckResult<Type> {
        erg_common::debug_power_assert!(args.len() == 1);
        let symbol = Token::symbol(unaryop_to_dname(op.inspect()));
        let mut op = hir::Expr::Accessor(hir::Accessor::local(symbol, Type::ASTOmitted));
        self.get_call_t(&mut op, args, &[], namespace).map_err(|e| {
            let core = ErrorCore::new(e.core.errno, e.core.kind, op.loc(), e.core.desc, e.core.hint);
            TyCheckError::new(core, e.caused_by)
        })
    }

    pub(crate) fn get_call_t(
        &self,
        callee: &hir::Expr,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        namespace: &Str
    ) -> TyCheckResult<Type> {
        match callee {
            hir::Expr::Accessor(hir::Accessor::Local(local)) if &local.inspect()[..] == "match" => {
                return self.get_match_call_t(pos_args, kw_args)
            }
            _ => {}
        }
        let found = self.search_call_t(callee, namespace)?;
        log!("Found:\ncallee: {callee}\nfound: {found}");
        let instance = self.instantiate(found, callee)?;
        log!("Instantiated:\ninstance: {instance}\npos_args: ({})\nkw_args: ({})", fmt_slice(pos_args), fmt_slice(kw_args));
        self.substitute_call(callee, &instance, pos_args, kw_args)?;
        log!("Substituted:\ninstance: {instance}");
        let res = self.eval.eval_t(instance, &self, self.level)?;
        log!("Evaluated:\nres: {res}\n");
        let res = Self::eliminate_linked_vars(res)?;
        log!("Eliminated:\nres: {res}\n");
        self.propagate(&res, callee)?;
        log!("Propagated:\nres: {res}\n");
        Ok(res)
    }

    pub(crate) fn rec_supertype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        if self.supertype_of(lhs, rhs, None) { return true }
        for sup_rhs in self.get_sorted_supertypes(rhs) {
            let bounds = sup_rhs.bounds();
            if sup_rhs.super_classes.iter().any(|sup| self.supertype_of(lhs, sup, Some(&bounds)))
            || sup_rhs.super_traits.iter().any(|sup| self.supertype_of(lhs, sup, Some(&bounds))) { return true }
        }
        for (patch_name, sub, sup) in self.glue_patch_and_types.iter() {
            let patch = self.rec_get_patch(patch_name).unwrap();
            let bounds = patch.bounds();
            if self.supertype_of(sub, rhs, Some(&bounds))
            && self.supertype_of(sup, lhs, Some(&bounds)) { return true }
        }
        if let Some(outer) = &self.outer {
            if outer.rec_supertype_of(lhs, rhs) { return true }
        }
        false
    }

    pub(crate) fn rec_subtype_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.rec_supertype_of(rhs, lhs)
    }

    pub(crate) fn _rec_same_type_of(&self, lhs: &Type, rhs: &Type) -> bool {
        self.rec_supertype_of(lhs, rhs) && self.rec_subtype_of(lhs, rhs)
    }

    fn eq_tp(&self, lhs: &TyParam, rhs: &TyParam, bounds: Option<&Set<TyBound>>) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(lhs), TyParam::Type(rhs)) => { return self.same_type_of(lhs, rhs, bounds) },
            (TyParam::Mono(l), TyParam::Mono(r)) => {
                if let (Some((l, _)), Some((r, _))) = (
                    self.types.iter().find(|(t, _)| t.name() == &l[..]),
                    self.types.iter().find(|(t, _)| t.name() == &r[..]),
                ) { return self.supertype_of(l, r, None) || self.subtype_of(l, r, None) }
            },
            (TyParam::MonoQVar(name), other)
            | (other, TyParam::MonoQVar(name)) => {
                if let Some(bs) = bounds {
                    if let Some(bound) = bs.iter().find(|b| b.mentions_as_instance(name)) {
                        let other_t = self.type_of(other, bounds);
                        return self.supertype_of(bound.t(), &other_t, bounds)
                    } else { todo!() } // subtyping
                }
            },
            (
                TyParam::App{ name: ln, args: largs },
                TyParam::App{ name: rn, args: rargs },
            ) => {
                return ln == rn
                    && largs.len() == rargs.len()
                    && largs.iter().zip(rargs.iter()).all(|(l, r)| self.eq_tp(l, r, bounds))
            },
            (TyParam::FreeVar(fv), other)
            | (other, TyParam::FreeVar(fv)) => {
                match &*fv.borrow() {
                    FreeKind::Linked(tp) => { return self.eq_tp(tp, other, bounds) },
                    FreeKind::Unbound{ constraint, .. }
                    | FreeKind::NamedUnbound{ constraint, .. }=> {
                        let t = constraint.typ().unwrap();
                        let other_t = self.type_of(other, bounds);
                        return self.supertype_of(&t, &other_t, bounds)
                    }
                }
            },
            (l, r) if l == r => { return true },
            _ => {},
        }
        self.eval.shallow_eq_tp(lhs, rhs, &self)
    }

    /// lhs :> rhs?
    /// ```
    /// assert supertype_of(Int, Nat) # i: Int = 1 as Nat
    /// assert supertype_of(Bool, Bool)
    /// ```
    /// TODO: Inputs/Outputs trait
    /// 単一化、評価等はここでは行わない、スーパータイプになる可能性があるかだけ判定する
    /// ので、lhsが(未連携)型変数の場合は単一化せずにtrueを返す
    pub(crate) fn supertype_of(&self, lhs: &Type, rhs: &Type, bounds: Option<&Set<TyBound>>) -> bool {
        if lhs.rec_eq(rhs) { return true }
        match (lhs, rhs) {
            // FIXME: Obj/Neverはクラス、Top/Bottomは構造型
            (Obj, _) | (_, Never) => true,
            (_, Obj) | (Never, _) => false,
            (Float | Ratio | Int | Nat | Bool, Bool)
            | (Float | Ratio | Int | Nat, Nat)
            | (Float | Ratio | Int, Int)
            | (Float | Ratio, Ratio)
            | (Float, Float) => true,
            (FuncCommon, Subr(SubrType{ kind: SubrKind::Func, .. }))
            | (ProcCommon, Subr(SubrType{ kind: SubrKind::Proc, .. }))
            | (FuncMethodCommon, Subr(SubrType{ kind: SubrKind::FuncMethod(_), .. }))
            | (ProcMethodCommon, Subr(SubrType{ kind: SubrKind::ProcMethod{ .. }, .. }))
            | (ArrayCommon, Type::Array{ .. })
            | (DictCommon, Type::Dict{ .. }) => true,
            (CallableCommon, Subr(_) | FuncCommon | ProcCommon | FuncMethodCommon | ProcMethodCommon) => true,
            (Subr(ls), Subr(rs))
                if ls.kind.same_kind_as(&rs.kind)
                && (ls.kind == SubrKind::Func || ls.kind == SubrKind::Proc) => {
                // () -> Never <: () -> Int <: () -> Object
                // (Object) -> Int <: (Int) -> Int <: (Never) -> Int
                ls.non_default_params.len() == rs.non_default_params.len()
                && ls.default_params.len() == rs.default_params.len()
                && self.supertype_of(&ls.return_t, &rs.return_t, bounds) // covariant
                && ls.non_default_params.iter()
                    .zip(rs.non_default_params.iter())
                    .all(|(l, r)| self.subtype_of(&l.ty, &r.ty, bounds))
                && ls.default_params.iter()
                    .zip(rs.default_params.iter())
                    .all(|(l, r)| self.subtype_of(&l.ty, &r.ty, bounds)) // contravariant
            }
            (Type::Array{ t: lhs, len: llen }, Type::Array{ t: rhs, len: rlen }) => {
                self.eq_tp(llen, rlen, bounds)
                && self.supertype_of(lhs, rhs, bounds)
            }
            (Tuple(lhs), Tuple(rhs)) => {
                lhs.len() == rhs.len()
                && lhs.iter()
                    .zip(rhs.iter())
                    .all(|(l, r)| self.supertype_of(l, r, bounds))
            }
            // RefMut, OptionMutは非変
            (Range(lhs), Range(rhs))
            | (Iter(lhs), Iter(rhs))
            | (Ref(lhs), Ref(rhs))
            | (Option(lhs), Option(rhs))
            | (VarArgs(lhs), VarArgs(rhs)) => self.supertype_of(lhs, rhs, bounds),
            // 型変数の場合は、上位型になり得るならtrue、(型制約上)なり得ないならfalse
            // 可能性に応じてその後の型判断を下すので、ここで型制約は課さない
            (FreeVar(v), rhs) => {
                match &*v.borrow() {
                    FreeKind::Linked(t) => self.supertype_of(t, rhs, bounds),
                    FreeKind::Unbound { constraint, .. }
                    | FreeKind::NamedUnbound{ constraint, .. } => match constraint {
                        // `(?T <: Int) :> Nat` can be true, `(?T <: Nat) :> Int` is false
                        Constraint::SubtypeOf(sup) => self.supertype_of(sup, rhs, bounds),
                        // `(?T :> X) :> Y` is true,
                        Constraint::SupertypeOf(_) => true,
                        // `(Nat <: ?T <: Ratio) :> Nat` can be true
                        Constraint::Sandwiched{ sup, .. } => self.supertype_of(sup, rhs, bounds),
                        // (?v: Type, rhs): OK
                        // (?v: Nat, rhs): Something wrong
                        // Class <: Type, but Nat <!: Type (Nat: Type)
                        Constraint::TypeOf(t) =>
                            if self.supertype_of(&Type, t, bounds) { true } else { panic!() },
                    },
                }
            }
            (lhs, FreeVar(v)) => {
                match &*v.borrow() {
                    FreeKind::Linked(t) => self.supertype_of(lhs, t, bounds),
                    FreeKind::Unbound { constraint, .. }
                    | FreeKind::NamedUnbound{ constraint, .. } => match constraint {
                        // `Nat :> (?T <: Int)` can be true => `X :> (?T <: Y)` can be true
                        Constraint::SubtypeOf(_sup) => true,
                        // `Int :> (?T :> Nat)` can be true, `Nat :> (?T :> Int)` is false
                        Constraint::SupertypeOf(sub) => self.supertype_of(lhs, sub, bounds),
                        // `Int :> (Nat <: ?T <: Ratio)` can be true, `Nat :> (Int <: ?T <: Ratio)` is false
                        Constraint::Sandwiched{ sub, .. } => self.supertype_of(lhs, sub, bounds),
                        Constraint::TypeOf(t) =>
                            if self.supertype_of(&Type, t, bounds) { true } else { panic!() },
                    },
                }
            }
            // (MonoQuantVar(_), _) | (_, MonoQuantVar(_)) => true,
            // REVIEW: maybe this is incomplete
            // ({I: Int | I >= 0} :> {N: Int | N >= 0}) == true,
            // ({I: Int | I >= 0} :> {I: Int | I >= 1}) == true,
            // ({I: Int | I >= 0} :> {N: Nat | N >= 1}) == true,
            // ({I: Int | I > 1 or I < -1} :> {I: Int | I >= 0}) == false,
            (Refinement(l), Refinement(r)) => {
                if !self.supertype_of(&l.t, &r.t, bounds) { return false }
                let mut r_preds_clone = r.preds.clone();
                for l_pred in l.preds.iter() {
                    for r_pred in r.preds.iter() {
                        if l_pred.subject().unwrap_or("") == &l.var[..]
                        && r_pred.subject().unwrap_or("") == &r.var[..]
                        && self.is_super_pred_of(l_pred, r_pred, bounds) {
                            r_preds_clone.remove(r_pred);
                        }
                    }
                }
                r_preds_clone.is_empty()
            },
            (Nat, re @ Refinement(_)) => {
                let nat = Type::Refinement(self.into_refinement(Nat));
                self.supertype_of(&nat, re, bounds)
            }
            (re @ Refinement(_), Nat) => {
                let nat = Type::Refinement(self.into_refinement(Nat));
                self.supertype_of(re, &nat, bounds)
            }
            // Int :> {I: Int | ...} == true, Real :> {I: Int | ...} == false, Int :> {I: Str| ...} == false
            (l, Refinement(r)) => {
                self.supertype_of(l, &r.t, bounds)
            },
            // ({I: Int | True} :> Int) == true, ({N: Nat | ...} :> Int) == false, ({I: Int | I >= 0} :> Int) == false
            (Refinement(l), r) => {
                if l.preds.iter().any(|p| p.mentions(&l.var) && p.can_be_false()) {
                    return false
                }
                self.supertype_of(&l.t, r, bounds)
            },
            (Quantified(l), Quantified(r)) => {
                // REVIEW: maybe this should be `unreachable`
                if bounds.is_some() { panic!("Nested quantification") }
                else {
                    // TODO: bounds同士の評価
                    self.supertype_of(l.unbound_callable.as_ref(), r.unbound_callable.as_ref(), Some(&l.bounds))
                }
            },
            (Quantified(q), r) => {
                // REVIEW: maybe this should be `unreachable`
                if bounds.is_some() { panic!("Nested quantification") }
                else { self.supertype_of(q.unbound_callable.as_ref(), r, Some(&q.bounds)) }
            },
            (lhs, Or(tys)) => tys.iter().all(|t| self.supertype_of(lhs, t, bounds)),
            (And(tys), rhs) => tys.iter().all(|t| self.supertype_of(t, rhs, bounds)),
            (VarArgs(lhs), rhs) => self.supertype_of(lhs, rhs, bounds),
            // TはすべてのRef(T)のメソッドを持つので、Ref(T)のサブタイプ
            (Ref(lhs), rhs)
            | (RefMut(lhs), rhs) => self.supertype_of(lhs, rhs, bounds),
            // TODO: Consider variance
            (Poly{ name: ln, params: lp  }, Poly{ name: rn, params: rp }) => {
                ln == rn
                && lp.len() == rp.len()
                && lp.iter()
                    .zip(rp.iter())
                    .all(|(l, r)| self.eq_tp(l, r, bounds))
            },
            (MonoQVar(name), r) => {
                if let Some(bs) = bounds {
                    if let Some(bound) = bs.iter().find(|b| b.mentions_as_subtype(name)) {
                        self.supertype_of(bound.t(), r, bounds)
                    } else if let Some(bound) = bs.iter().find(|b| b.mentions_as_instance(name)) {
                        if self.same_type_of(bound.t(), &Type::Type, bounds) { true } else { todo!()}
                    } else { panic!("Unbound type variable: {name}") }
                } else { panic!("No quantification") }
            },
            (_l, MonoQVar(_name)) => todo!(),
            (PolyQVar{ .. }, _r) => todo!(),
            (_l, PolyQVar{ .. }) => todo!(),
            (_l, _r) => false,
        }
    }

    /// lhs <: rhs?
    pub(crate) fn subtype_of(&self, lhs: &Type, rhs: &Type, bounds: Option<&Set<TyBound>>) -> bool {
        self.supertype_of(rhs, lhs, bounds)
    }

    pub(crate) fn same_type_of(&self, lhs: &Type, rhs: &Type, bounds: Option<&Set<TyBound>>) -> bool {
        self.supertype_of(lhs, rhs, bounds) && self.subtype_of(lhs, rhs, bounds)
    }

    fn try_cmp(&self, l: &TyParam, r: &TyParam, bounds: Option<&Set<TyBound>>) -> Option<TyParamOrdering> {
        match (l, r) {
            (TyParam::ConstObj(l), TyParam::ConstObj(r)) =>
                l.try_cmp(r).map(Into::into),
            // TODO: 型を見て判断する
            (TyParam::BinOp{ op, lhs, rhs }, r) => {
                if let Ok(l) = self.eval.eval_bin_tp(*op, lhs, rhs) {
                    self.try_cmp(&l, r, bounds)
                } else { Some(Any) }
            },
            (TyParam::FreeVar(fv), p) if fv.is_linked() => {
                self.try_cmp(&*fv.crack(), p, bounds)
            }
            (p, TyParam::FreeVar(fv)) if fv.is_linked() => {
                self.try_cmp(p, &*fv.crack(), bounds)
            }
            (
                l @ (TyParam::FreeVar(_) | TyParam::Erased(_) | TyParam::MonoQVar(_)),
                r @ (TyParam::FreeVar(_) | TyParam::Erased(_) | TyParam::MonoQVar(_)),
            ) /* if v.is_unbound() */ => {
                let l_t = self.eval.get_tp_t(l, bounds, self).unwrap();
                let r_t = self.eval.get_tp_t(r, bounds, self).unwrap();
                if self.rec_supertype_of(&l_t, &r_t) || self.rec_subtype_of(&l_t, &r_t) {
                    Some(Any)
                } else { Some(NotEqual) }
            },
            // Intervalとしてのl..rはl<=rであることが前提となっている
            // try_cmp((n: 1..10), 1) -> Some(GreaterEqual)
            // try_cmp((n: 0..2), 1) -> Some(Any)
            // try_cmp((n: 2.._), 1) -> Some(Greater)
            // try_cmp((n: -1.._), 1) -> Some(Any)
            (l @ (TyParam::Erased(_) | TyParam::FreeVar(_) | TyParam::MonoQVar(_)), p) => {
                let t = self.eval.get_tp_t(l, bounds, &self).unwrap();
                let inf = self.inf(&t);
                let sup = self.sup(&t);
                if let (Some(inf), Some(sup)) = (inf, sup) {
                    // (n: Int, 1) -> (-inf..inf, 1) -> (cmp(-inf, 1), cmp(inf, 1)) -> (Less, Greater) -> Any
                    // (n: 5..10, 2) -> (cmp(5..10, 2), cmp(5..10, 2)) -> (Greater, Greater) -> Greater
                    match (
                        self.try_cmp(&inf, p, bounds).unwrap(),
                        self.try_cmp(&sup, p, bounds).unwrap()
                    ) {
                        (Less, Less) => Some(Less),
                        (Less, Equal) => Some(LessEqual),
                        (Less, LessEqual) => Some(LessEqual),
                        (Less, NotEqual) => Some(NotEqual),
                        (Less, Greater | GreaterEqual | Any) => Some(Any),
                        (Equal, Less) => assume_unreachable!(),
                        (Equal, Equal) => Some(Equal),
                        (Equal, Greater) => Some(GreaterEqual),
                        (Equal, LessEqual) => Some(Equal),
                        (Equal, NotEqual) => Some(GreaterEqual),
                        (Equal, GreaterEqual | Any) => Some(GreaterEqual),
                        (Greater, Less) => assume_unreachable!(),
                        (Greater, Equal) => assume_unreachable!(),
                        (Greater, Greater | NotEqual | GreaterEqual | Any) => Some(Greater),
                        (Greater, LessEqual) => assume_unreachable!(),
                        (LessEqual, Less) => assume_unreachable!(),
                        (LessEqual, Equal | LessEqual) => Some(LessEqual),
                        (LessEqual, Greater | NotEqual | GreaterEqual | Any) => Some(Any),
                        (NotEqual, Less) => Some(Less),
                        (NotEqual, Equal | LessEqual) => Some(LessEqual),
                        (NotEqual, Greater | GreaterEqual | Any) => Some(Any),
                        (NotEqual, NotEqual) => Some(NotEqual),
                        (GreaterEqual, Less) => assume_unreachable!(),
                        (GreaterEqual, Equal | LessEqual) => Some(Equal),
                        (GreaterEqual, Greater | NotEqual | GreaterEqual | Any) => Some(GreaterEqual),
                        (Any, Less) => Some(Less),
                        (Any, Equal | LessEqual) => Some(LessEqual),
                        (Any, Greater | NotEqual | GreaterEqual | Any) => Some(Any),
                    }
                } else { None }
            }
            (l, r @ (TyParam::Erased(_) | TyParam::MonoQVar(_) | TyParam::FreeVar(_))) =>
                self.try_cmp(r, l, bounds).map(|ord| ord.reverse()),
            (_l, _r) => {
                erg_common::fmt_dbg!(_l, _r,);
                None
            },
        }
    }

    fn into_refinement(&self, t: Type) -> RefinementType {
        match t {
            Nat => {
                let var = Str::from(fresh_varname());
                RefinementType::new(var.clone(), Int, set!{Predicate::ge(var, TyParam::value(0))})
            }
            Refinement(r) => r,
            t => {
                let var = Str::from(fresh_varname());
                RefinementType::new(var.clone(), t, set!{})
            }
        }
    }

    /// 和集合(A or B)を返す
    fn union(&self, lhs: &Type, rhs: &Type) -> Type {
        match (self.rec_supertype_of(lhs, rhs), self.rec_subtype_of(lhs, rhs)) {
            (true, true) => return lhs.clone(), // lhs = rhs
            (true, false) => return lhs.clone(), // lhs :> rhs
            (false, true) => return rhs.clone(),
            (false, false) => {},
        }
        match (lhs, rhs) {
            (Refinement(l), Refinement(r)) =>
                Type::Refinement(self.union_refinement(l, r)),
            (Or(ts), t)
            | (t, Or(ts)) => Or([vec![t.clone()], ts.clone()].concat()),
            (t, Type::Never) | (Type::Never, t) => t.clone(),
            (t, Refinement(r))
            | (Refinement(r), t) => {
                let t = self.into_refinement(t.clone());
                Type::Refinement(self.union_refinement(&t, r))
            },
            (l, r) => Type::Or(vec![l.clone(), r.clone()]),
        }
    }

    fn union_refinement(&self, lhs: &RefinementType, rhs: &RefinementType) -> RefinementType {
        if !self.supertype_of(&lhs.t, &rhs.t, None) && !self.subtype_of(&lhs.t, &rhs.t, None) {
            log!("{lhs}\n{rhs}");
            todo!()
        } else {
            let name = lhs.var.clone();
            let rhs_preds = rhs.preds.iter().map(|p| p.clone().change_subject_name(name.clone())).collect();
            // FIXME: predの包含関係も考慮する
            RefinementType::new(
                lhs.var.clone(),
                *lhs.t.clone(),
                lhs.preds.clone().concat(rhs_preds),
            )
        }
    }

    /// see doc/LANG/compiler/refinement_subtyping.md
    /// ```
    /// assert is_super_pred({I >= 0}, {I == 0})
    /// assert is_super_pred({T >= 0}, {I == 0})
    /// assert !is_super_pred({I < 0}, {I == 0})
    /// ```
    fn is_super_pred_of(&self, lhs: &Predicate, rhs: &Predicate, bounds: Option<&Set<TyBound>>) -> bool {
        match (lhs, rhs) {
            (Pred::LessEqual{ rhs, .. }, _) if !rhs.has_upper_bound() => true,
            (Pred::GreaterEqual{ rhs, .. }, _) if !rhs.has_lower_bound() => true,
            (
                Pred::Equal{ .. },
                Pred::GreaterEqual{ .. } | Pred::LessEqual{ .. } | Pred::NotEqual{ .. }
            )
            | (Pred::LessEqual{ .. }, Pred::GreaterEqual{ .. })
            | (Pred::GreaterEqual{ .. }, Pred::LessEqual{ .. })
            | (Pred::NotEqual{ .. }, Pred::Equal{ .. }) => false,
            (Pred::Equal{ rhs, .. }, Pred::Equal{ rhs: rhs2, .. })
            | (Pred::NotEqual{ rhs, .. }, Pred::NotEqual{ rhs: rhs2, .. }) =>
                self.try_cmp(rhs, rhs2, bounds).unwrap().is_eq(),
            // {T >= 0} :> {T >= 1}, {T >= 0} :> {T == 1}
            (
                Pred::GreaterEqual{ rhs, .. },
                Pred::GreaterEqual{ rhs: rhs2, .. } | Pred::Equal{ rhs: rhs2, .. },
            ) => self.try_cmp(rhs, rhs2, bounds).unwrap().is_le(),
            (
                Pred::LessEqual{ rhs, .. },
                Pred::LessEqual{ rhs: rhs2, .. } | Pred::Equal{ rhs: rhs2, .. },
            ) => self.try_cmp(rhs, rhs2, bounds).unwrap().is_ge(),
            (lhs @ (Pred::GreaterEqual{ .. } | Pred::LessEqual { .. }), Pred::And(l, r)) =>
                self.is_super_pred_of(lhs, l, bounds) || self.is_super_pred_of(lhs, r, bounds),
            (lhs, Pred::Or(l, r)) =>
                self.is_super_pred_of(lhs, l, bounds) && self.is_super_pred_of(lhs, r, bounds),
            (Pred::Or(l, r), rhs @ (Pred::GreaterEqual{ .. } | Pred::LessEqual{ .. })) =>
                self.is_super_pred_of(l, rhs, bounds) || self.is_super_pred_of(r, rhs, bounds),
            (Pred::And(l, r), rhs) =>
                self.is_super_pred_of(l, rhs, bounds) && self.is_super_pred_of(r, rhs, bounds),
            (lhs, rhs) => todo!("{lhs}/{rhs}"),
        }
    }

    fn is_sub_constraint_of(&self, l: &Constraint, r: &Constraint) -> bool {
        match (l, r) {
            // |T <: Nat| <: |T <: Int|
            // |I: Nat| <: |I: Int|
            (Constraint::SubtypeOf(lhs), Constraint::SubtypeOf(rhs))
            | (Constraint::TypeOf(lhs), Constraint::TypeOf(rhs)) =>
                self.rec_subtype_of(lhs, rhs),
            // |Int <: T| <: |Nat <: T|
            (Constraint::SupertypeOf(lhs), Constraint::SupertypeOf(rhs)) =>
                self.rec_supertype_of(lhs, rhs),
            (Constraint::SubtypeOf(_), Constraint::TypeOf(Type)) => true,
            // |Int <: T <: Ratio| <: |Nat <: T <: Complex|
            (
                Constraint::Sandwiched{ sub: lsub, sup: lsup },
                Constraint::Sandwiched{ sub: rsub, sup: rsup },
            ) => self.rec_supertype_of(lsub, rsub) && self.rec_subtype_of(lsup, rsup),
            _ => false,
        }
    }

    #[inline]
    fn type_of(&self, p: &TyParam, bounds: Option<&Set<TyBound>>) -> Type {
        self.eval.get_tp_t(p, bounds, &self).unwrap()
    }

    // sup/inf({±∞}) = ±∞ではあるが、Inf/NegInfにはOrdを実装しない
    fn sup(&self, t: &Type) -> Option<TyParam> {
        match t {
            Int | Nat | Float => Some(TyParam::value(Inf)),
            Refinement(refine) => {
                let mut maybe_max = None;
                for pred in refine.preds.iter() {
                    match pred {
                        Pred::LessEqual{ lhs, rhs } |
                        Pred::Equal{ lhs, rhs } if lhs == &refine.var => {
                            if let Some(max) = &maybe_max {
                                if self.try_cmp(rhs, &max, None).unwrap() == Greater {
                                    maybe_max = Some(rhs.clone());
                                }
                            } else {
                                maybe_max = Some(rhs.clone());
                            }
                        },
                        _ => {}
                    }
                }
                maybe_max
            },
            _other => None,
        }
    }

    fn inf(&self, t: &Type) -> Option<TyParam> {
        match t {
            Int | Float => Some(TyParam::value(-Inf)),
            Nat => Some(TyParam::value(0usize)),
            Refinement(refine) => {
                let mut maybe_min = None;
                for pred in refine.preds.iter() {
                    match pred {
                        Predicate::GreaterEqual{ lhs, rhs } |
                        Predicate::Equal{ lhs, rhs } if lhs == &refine.var => {
                            if let Some(min) = &maybe_min {
                                if self.try_cmp(rhs, &min, None).unwrap() == Less {
                                    maybe_min = Some(rhs.clone());
                                }
                            } else {
                                maybe_min = Some(rhs.clone());
                            }
                        },
                        _ => {}
                    }
                }
                maybe_min
            },
            _other => None,
        }
    }

    /// lhsとrhsが包含関係にあるとき小さいほうを返す
    /// 関係なければNoneを返す
    fn min<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> Option<&'t Type> {
        // 同じならどちらを返しても良い
        match (self.rec_supertype_of(lhs, rhs), self.rec_subtype_of(lhs, rhs)) {
            (true, true) | (true, false) => Some(rhs),
            (false, true) => Some(lhs),
            _ => None,
        }
    }

    fn cmp_t<'t>(&self, lhs: &'t Type, rhs: &'t Type) -> TyParamOrdering {
        match self.min(lhs, rhs) {
            Some(l) if l == lhs => TyParamOrdering::Less,
            Some(_) => TyParamOrdering::Greater,
            None => todo!(),
        }
    }

    // TODO:
    fn smallest_pair<I: Iterator<Item=(Type, Type)>>(&self, ts: I) -> Option<(Type, Type)> {
        ts.min_by(|(_, lhs), (_, rhs)| self.cmp_t(lhs, rhs).try_into().unwrap())
    }

    fn smallest_ref_t<'t, I: Iterator<Item=&'t Type>>(&self, ts: I) -> Option<&'t Type> {
        ts.min_by(|lhs, rhs| self.cmp_t(lhs, rhs).try_into().unwrap())
    }

    pub(crate) fn get_local(&self, name: &Token, namespace: &Str) -> TyCheckResult<ConstObj> {
        if let Some(obj) = self.consts.get(name.inspect()) {
                Ok(obj.clone())
        } else {
            if let Some(parent) = self.outer.as_ref() {
                return parent.get_local(name, namespace)
            }
            Err(TyCheckError::no_var_error(
                name.loc(),
                namespace.clone(),
                name.inspect(),
                self.get_similar_name(name.inspect()),
            ))
        }
    }

    pub(crate) fn _get_attr(&self, obj: &hir::Expr, name: &Token, namespace: &Str) -> TyCheckResult<ConstObj> {
        let self_t = obj.t();
        for ctx in self.get_sorted_supertypes(&self_t) {
            if let Ok(t) = ctx.get_local(name, namespace) {
                return Ok(t)
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.outer.as_ref() {
            parent._get_attr(obj, name, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                name.loc(),
                namespace.clone(),
                &self_t,
                name.inspect(),
                self.get_similar_attr(&self_t, name.inspect()),
            ))
        }
    }

    pub(crate) fn get_similar_name(&self, name: &str) -> Option<&Str> {
        if name.len() <= 1 { return None }
        let most_similar_name = self.impls.keys().min_by_key(|v| levenshtein(v.inspect(), name))?.inspect();
        let len = most_similar_name.len();
        if levenshtein(most_similar_name, name) >= len/2 {
            let outer = self.outer.as_ref()?;
            outer.get_similar_name(name)
        }
        else { Some(most_similar_name) }
    }

    pub(crate) fn get_similar_attr<'a>(&'a self, self_t: &'a Type, name: &str) -> Option<&'a Str> {
        for ctx in self.get_sorted_supertypes(self_t) {
            if let Some(name) = ctx.get_similar_name(name) { return Some(name) }
        }
        None
    }
}

impl Context {
    pub(crate) fn grow(&mut self, name: &str, kind: ContextKind, vis: Visibility) -> TyCheckResult<()> {
        let name = if vis.is_public() {
            format!("{parent}.{name}", parent = self.name)
        } else { format!("{parent}::{name}", parent = self.name) };
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
                name.loc(),
                self.caused_by(),
                name.inspect(),
                &vi.t,
            ));
        }
        if let Some(parent) = &mut self.outer {
            *self = mem::take(parent);
            log!("{}: current namespace: {}", fn_name!(), self.name);
            if !uninited_errs.is_empty() { Err(uninited_errs) }
            else { Ok(()) }
        } else {
            Err(TyCheckErrors::from(TyCheckError::checker_bug(0, Location::Unknown, fn_name!(), line!())))
        }
    }

    pub(crate) fn get_sorted_supertypes<'a>(&'a self, t: &'a Type) -> impl Iterator<Item=&'a Context> {
        let mut ctxs = self._rec_get_supertypes(t).collect::<Vec<_>>();
        ctxs.sort_by(|(lhs, _), (rhs, _)| self.cmp_t(lhs, rhs).try_into().unwrap());
        ctxs.into_iter().map(|(_, ctx)| ctx)
    }

    /// this method is for `get_sorted_supertypes` only
    fn _rec_get_supertypes<'a>(&'a self, t: &'a Type) -> impl Iterator<Item=(&'a Type, &'a Context)> {
        let i = self._get_supertypes(t);
        if i.size_hint().1 == Some(0) {
            if let Some(outer) = &self.outer {
                return outer._rec_get_supertypes(t)
            }
        }
        i
    }

    /// this method is for `rec_get_supertypes` only
    fn _get_supertypes<'a>(&'a self, t: &'a Type) -> impl Iterator<Item=(&'a Type, &'a Context)> {
        self.types.iter()
            .filter_map(|(maybe_sup, ctx)| {
                let bounds = ctx.bounds();
                if self.supertype_of(maybe_sup, t, Some(&bounds)) { Some((maybe_sup, ctx)) } else { None }
            })
    }

    fn rec_get_glue_patch_and_types(&self) -> Vec<(VarName, Type, Type)> {
        if let Some(outer) = &self.outer {
            [&self.glue_patch_and_types[..], &outer.rec_get_glue_patch_and_types()].concat()
        } else {
            self.glue_patch_and_types.clone()
        }
    }

    fn rec_get_patch(&self, name: &VarName) -> Option<&Context> {
        if let Some(patch) = self.patches.get(name) {
            return Some(patch)
        } else {
            if let Some(outer) = &self.outer {
                return outer.rec_get_patch(name)
            }
        }
        None
    }

    fn rec_get_mod(&self, name: &str) -> Option<&Context> {
        if let Some(mod_) = self.mods.get(name) {
            return Some(mod_)
        } else {
            if let Some(outer) = &self.outer {
                return outer.rec_get_mod(name)
            }
        }
        None
    }

    // 再帰サブルーチン/型の推論を可能にするため、予め登録しておく
    pub(crate) fn preregister(&mut self, block: &Vec<ast::Expr>) -> TyCheckResult<()> {
        for expr in block.iter() {
            match expr {
                ast::Expr::Def(def) => {
                    let id = Some(def.body.id);
                    let eval_body_t = || {
                        self.eval.eval_const_block(&def.body.block, &self)
                            .map(|c| Type::enum_t(set![c]))
                    };
                    match &def.sig {
                        ast::Signature::Subr(sig) => {
                            let opt_ret_t = if let Some(spec) = sig.return_t_spec.as_ref() {
                                Some(self.instantiate_typespec(spec, PreRegister)?)
                            } else { eval_body_t() };
                            self.declare_sub(&sig, opt_ret_t, id)?;
                        }
                        ast::Signature::Var(sig) if sig.is_const() => {
                            let t = if let Some(spec) = sig.t_spec.as_ref() {
                                Some(self.instantiate_typespec(spec, PreRegister)?)
                            } else { eval_body_t() };
                            self.declare_var(&sig, t, id)?;
                        }
                        _ => {}
                    }
                },
                _ => {},
            }
        }
        Ok(())
    }
}
