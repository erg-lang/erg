//! defines type information for builtin objects (in `Context`)
//!
//! 組み込みオブジェクトの型情報を(Contextに)定義
#![allow(non_snake_case)]

mod classes;
pub mod const_func;
mod funcs;
mod patches;
mod procs;
mod traits;

use std::path::PathBuf;

use erg_common::config::ErgConfig;
use erg_common::dict;
// use erg_common::error::Location;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::vis::Visibility;
use erg_common::Str;
use erg_common::{set, unique_in_place};

use crate::ty::free::Constraint;
use crate::ty::value::ValueObj;
use crate::ty::Type;
use crate::ty::{constructors::*, BuiltinConstSubr, ConstSubr, Predicate};
use erg_common::fresh::fresh_varname;
use ParamSpec as PS;
use Type::*;

use erg_parser::ast::VarName;

use crate::context::initialize::const_func::*;
use crate::context::instantiate::ConstTemplate;
use crate::context::{
    ClassDefType, Context, ContextKind, MethodInfo, ParamSpec, TypeRelationInstance,
};
use crate::mod_cache::SharedModuleCache;
use crate::varinfo::{Mutability, VarInfo, VarKind};
use Mutability::*;
use VarKind::*;
use Visibility::*;

const NUM: &str = "Num";

const UNPACK: &str = "Unpack";
const INHERITABLE_TYPE: &str = "InheritableType";
const NAMED: &str = "Named";
const MUTABLE: &str = "Mutable";
const SELF: &str = "Self";
const IMMUTIZABLE: &str = "Immutizable";
const IMMUT_TYPE: &str = "ImmutType";
const PROC_UPDATE: &str = "update!";
const MUTIZABLE: &str = "Mutizable";
const MUTABLE_MUT_TYPE: &str = "MutType!";
const PATH_LIKE: &str = "PathLike";
const MUTABLE_READABLE: &str = "Readable!";
const READABLE: &str = "Readable";
const FUNC_READ: &str = "read";
const PROC_READ: &str = "read!";
const WRITABLE: &str = "Writable";
const MUTABLE_WRITABLE: &str = "Writable!";
const FUNC_WRITE: &str = "write";
const PROC_WRITE: &str = "write!";
const FILE_LIKE: &str = "FileLike";
const MUTABLE_FILE_LIKE: &str = "FileLike!";
const SHOW: &str = "Show";
const INPUT: &str = "Input";
const OUTPUT: &str = "Output";
const IN: &str = "In";
const EQ: &str = "Eq";
const ORD: &str = "Ord";
const TO_STR: &str = "to_str";
const ORDERING: &str = "Ordering";
const SEQ: &str = "Seq";
const FUNC_LEN: &str = "len";
const FUNC_GET: &str = "get";
const ITERABLE: &str = "Iterable";
const FUNC_ITER: &str = "iter";
const ITER: &str = "Iter";
const ADD: &str = "Add";
const SUB: &str = "Sub";
const MUL: &str = "Mul";
const DIV: &str = "Div";
const FLOOR_DIV: &str = "FloorDiv";
const OP_IN: &str = "__in__";
const OP_EQ: &str = "__eq__";
const OP_CMP: &str = "__cmp__";
const OP_ADD: &str = "__add__";
const OP_SUB: &str = "__sub__";
const OP_MUL: &str = "__mul__";
const OP_DIV: &str = "__div__";
const OP_FLOOR_DIV: &str = "__floordiv__";

const NAME: &str = "__name__";
const FUNDAMENTAL_STR: &str = "__str__";
const FUNDAMENTAL_ITER: &str = "__iter__";

const LICENSE: &str = "license";
const CREDITS: &str = "credits";
const COPYRIGHT: &str = "copyright";
const TRUE: &str = "True";
const FALSE: &str = "False";
const NONE: &str = "None";
const NOT_IMPLEMENTED: &str = "NotImplemented";
const ELLIPSIS: &str = "Ellipsis";
const SITEBUILTINS_PRINTER: &str = "_sitebuiltins._Printer";

const TY_T: &str = "T";
const TY_I: &str = "I";
const TY_R: &str = "R";

impl Context {
    fn register_builtin_decl(&mut self, name: &'static str, t: Type, vis: Visibility) {
        if cfg!(feature = "debug") {
            if let Type::Subr(subr) = &t {
                if subr.has_qvar() {
                    panic!("not quantified subr: {subr}");
                }
            }
        }
        let impl_of = if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
            Some(tr.clone())
        } else {
            None
        };
        let name = VarName::from_static(name);
        if self.decls.get(&name).is_some() {
            panic!("already registered: {} {name}", self.name);
        } else {
            self.decls.insert(
                name,
                VarInfo::new(t, Immutable, vis, Builtin, None, impl_of, None),
            );
        }
    }

    fn register_builtin_py_decl(
        &mut self,
        name: &'static str,
        t: Type,
        vis: Visibility,
        py_name: Option<&'static str>,
    ) {
        if cfg!(feature = "debug") {
            if let Type::Subr(subr) = &t {
                if subr.has_qvar() {
                    panic!("not quantified subr: {subr}");
                }
            }
        }
        let impl_of = if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
            Some(tr.clone())
        } else {
            None
        };
        let name = if cfg!(feature = "py_compatible") {
            if let Some(py_name) = py_name {
                VarName::from_static(py_name)
            } else {
                VarName::from_static(name)
            }
        } else {
            VarName::from_static(name)
        };
        let vi = VarInfo::new(
            t,
            Immutable,
            vis,
            Builtin,
            None,
            impl_of,
            py_name.map(Str::ever),
        );
        if let Some(_vi) = self.decls.get(&name) {
            if _vi != &vi {
                panic!("already registered: {} {name}", self.name);
            }
        } else {
            self.decls.insert(name, vi);
        }
    }

    fn register_builtin_impl(
        &mut self,
        name: &'static str,
        t: Type,
        muty: Mutability,
        vis: Visibility,
    ) {
        if cfg!(feature = "debug") {
            if let Type::Subr(subr) = &t {
                if subr.has_qvar() {
                    panic!("not quantified subr: {subr}");
                }
            }
        }
        let impl_of = if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
            Some(tr.clone())
        } else {
            None
        };
        let name = VarName::from_static(name);
        let vi = VarInfo::new(t, muty, vis, Builtin, None, impl_of, None);
        if self.locals.get(&name).is_some() {
            panic!("already registered: {} {name}", self.name);
        } else {
            self.locals.insert(name, vi);
        }
    }

    fn register_builtin_py_impl(
        &mut self,
        name: &'static str,
        t: Type,
        muty: Mutability,
        vis: Visibility,
        py_name: Option<&'static str>,
    ) {
        if cfg!(feature = "debug") {
            if let Type::Subr(subr) = &t {
                if subr.has_qvar() {
                    panic!("not quantified subr: {subr}");
                }
            }
        }
        let impl_of = if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
            Some(tr.clone())
        } else {
            None
        };
        let name = if cfg!(feature = "py_compatible") {
            if let Some(py_name) = py_name {
                VarName::from_static(py_name)
            } else {
                VarName::from_static(name)
            }
        } else {
            VarName::from_static(name)
        };
        let vi = VarInfo::new(t, muty, vis, Builtin, None, impl_of, py_name.map(Str::ever));
        if let Some(_vi) = self.locals.get(&name) {
            if _vi != &vi {
                panic!("already registered: {} {name}", self.name);
            }
        } else {
            self.locals.insert(name, vi);
        }
    }

    fn register_builtin_const(&mut self, name: &str, vis: Visibility, obj: ValueObj) {
        if self.rec_get_const_obj(name).is_some() {
            panic!("already registered: {} {name}", self.name);
        } else {
            let impl_of = if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
                Some(tr.clone())
            } else {
                None
            };
            // TODO: not all value objects are comparable
            let vi = VarInfo::new(
                v_enum(set! {obj.clone()}),
                Const,
                vis,
                Builtin,
                None,
                impl_of,
                None,
            );
            self.consts.insert(VarName::from_str(Str::rc(name)), obj);
            self.locals.insert(VarName::from_str(Str::rc(name)), vi);
        }
    }

    fn register_const_param_defaults(&mut self, name: &'static str, params: Vec<ConstTemplate>) {
        if self.const_param_defaults.get(name).is_some() {
            panic!("already registered: {} {name}", self.name);
        } else {
            self.const_param_defaults.insert(Str::ever(name), params);
        }
    }

    /// FIXME: トレイトの汎化型を指定するのにも使っているので、この名前は適当でない
    pub(crate) fn register_superclass(&mut self, sup: Type, sup_ctx: &Context) {
        self.super_classes.push(sup);
        self.super_classes.extend(sup_ctx.super_classes.clone());
        self.super_traits.extend(sup_ctx.super_traits.clone());
        unique_in_place(&mut self.super_classes);
        unique_in_place(&mut self.super_traits);
    }

    pub(crate) fn register_supertrait(&mut self, sup: Type, sup_ctx: &Context) {
        self.super_traits.push(sup);
        self.super_traits.extend(sup_ctx.super_traits.clone());
        unique_in_place(&mut self.super_traits);
    }

    fn register_builtin_type(
        &mut self,
        t: Type,
        ctx: Self,
        vis: Visibility,
        muty: Mutability,
        py_name: Option<&'static str>,
    ) {
        if t.typarams_len().is_none() {
            self.register_mono_type(t, ctx, vis, muty, py_name);
        } else {
            self.register_poly_type(t, ctx, vis, muty, py_name);
        }
    }

    fn register_mono_type(
        &mut self,
        t: Type,
        ctx: Self,
        vis: Visibility,
        muty: Mutability,
        py_name: Option<&'static str>,
    ) {
        if self.rec_get_mono_type(&t.local_name()).is_some() {
            panic!("{} has already been registered", t.local_name());
        } else if self.rec_get_const_obj(&t.local_name()).is_some() {
            panic!("{} has already been registered as const", t.local_name());
        } else {
            let name = VarName::from_str(t.local_name());
            let meta_t = match ctx.kind {
                ContextKind::Class => Type::ClassType,
                ContextKind::Trait => Type::TraitType,
                _ => Type::Type,
            };
            self.locals.insert(
                name.clone(),
                VarInfo::new(
                    meta_t,
                    muty,
                    vis,
                    Builtin,
                    None,
                    None,
                    py_name.map(Str::ever),
                ),
            );
            self.consts
                .insert(name.clone(), ValueObj::builtin_t(t.clone()));
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls.get_mut(&impl_trait.qual_name()) {
                    impls.insert(TypeRelationInstance::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls.insert(
                        impl_trait.qual_name(),
                        set![TypeRelationInstance::new(t.clone(), impl_trait.clone())],
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
            self.mono_types.insert(name, (t, ctx));
        }
    }

    // FIXME: MethodDefsと再代入は違う
    fn register_poly_type(
        &mut self,
        t: Type,
        ctx: Self,
        vis: Visibility,
        muty: Mutability,
        py_name: Option<&'static str>,
    ) {
        // FIXME: panic
        if let Some((_, root_ctx)) = self.poly_types.get_mut(&t.local_name()) {
            root_ctx.methods_list.push((ClassDefType::Simple(t), ctx));
        } else {
            let name = VarName::from_str(t.local_name());
            let meta_t = match ctx.kind {
                ContextKind::Class => Type::ClassType,
                ContextKind::Trait => Type::TraitType,
                _ => Type::Type,
            };
            if !cfg!(feature = "py_compatible") {
                self.locals.insert(
                    name.clone(),
                    VarInfo::new(
                        meta_t,
                        muty,
                        vis,
                        Builtin,
                        None,
                        None,
                        py_name.map(Str::ever),
                    ),
                );
            }
            self.consts
                .insert(name.clone(), ValueObj::builtin_t(t.clone()));
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls.get_mut(&impl_trait.qual_name()) {
                    impls.insert(TypeRelationInstance::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls.insert(
                        impl_trait.qual_name(),
                        set![TypeRelationInstance::new(t.clone(), impl_trait.clone())],
                    );
                }
            }
            for (trait_method, vi) in ctx.decls.iter() {
                if let Some(traits) = self.method_to_traits.get_mut(trait_method.inspect()) {
                    traits.push(MethodInfo::new(t.clone(), vi.clone()));
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
            self.poly_types.insert(name, (t, ctx));
        }
    }

    fn register_builtin_patch(
        &mut self,
        name: &'static str,
        ctx: Self,
        vis: Visibility,
        muty: Mutability,
    ) {
        if self.patches.contains_key(name) {
            panic!("{} has already been registered", name);
        } else {
            let name = VarName::from_static(name);
            self.locals.insert(
                name.clone(),
                VarInfo::new(Patch, muty, vis, Builtin, None, None, None),
            );
            for method_name in ctx.locals.keys() {
                if let Some(patches) = self.method_impl_patches.get_mut(method_name) {
                    patches.push(name.clone());
                } else {
                    self.method_impl_patches
                        .insert(method_name.clone(), vec![name.clone()]);
                }
            }
            if let ContextKind::GluePatch(tr_inst) = &ctx.kind {
                if let Some(impls) = self.trait_impls.get_mut(&tr_inst.sup_trait.qual_name()) {
                    impls.insert(tr_inst.clone());
                } else {
                    self.trait_impls
                        .insert(tr_inst.sup_trait.qual_name(), set![tr_inst.clone()]);
                }
            }
            self.patches.insert(name, ctx);
        }
    }

    fn init_builtin_consts(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        // TODO: this is not a const, but a special property
        self.register_builtin_py_impl(NAME, Str, Immutable, vis, Some(NAME));
        self.register_builtin_py_impl(
            LICENSE,
            mono(SITEBUILTINS_PRINTER),
            Immutable,
            vis,
            Some(LICENSE),
        );
        self.register_builtin_py_impl(
            CREDITS,
            mono(SITEBUILTINS_PRINTER),
            Immutable,
            vis,
            Some(CREDITS),
        );
        self.register_builtin_py_impl(
            COPYRIGHT,
            mono(SITEBUILTINS_PRINTER),
            Immutable,
            vis,
            Some(COPYRIGHT),
        );
        self.register_builtin_py_impl(TRUE, Bool, Const, Private, Some(TRUE));
        self.register_builtin_py_impl(FALSE, Bool, Const, Private, Some(FALSE));
        self.register_builtin_py_impl(NONE, NoneType, Const, Private, Some(NONE));
        self.register_builtin_py_impl(
            NOT_IMPLEMENTED,
            NotImplementedType,
            Const,
            Private,
            Some(NOT_IMPLEMENTED),
        );
        self.register_builtin_py_impl(ELLIPSIS, Ellipsis, Const, Private, Some(ELLIPSIS));
    }

    pub(crate) fn init_builtins(cfg: ErgConfig, mod_cache: &SharedModuleCache) {
        let mut ctx = Context::builtin_module("<builtins>", cfg, 100);
        ctx.init_builtin_consts();
        ctx.init_builtin_funcs();
        ctx.init_builtin_const_funcs();
        ctx.init_builtin_procs();
        ctx.init_builtin_operators();
        ctx.init_builtin_traits();
        ctx.init_builtin_classes();
        ctx.init_builtin_patches();
        mod_cache.register(PathBuf::from("<builtins>"), None, ctx);
    }

    pub fn new_module<S: Into<Str>>(
        name: S,
        cfg: ErgConfig,
        mod_cache: SharedModuleCache,
        py_mod_cache: SharedModuleCache,
    ) -> Self {
        Context::new(
            name.into(),
            cfg,
            ContextKind::Module,
            vec![],
            None,
            Some(mod_cache),
            Some(py_mod_cache),
            Context::TOP_LEVEL,
        )
    }
}
