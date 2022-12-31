//! defines type information for builtin objects (in `Context`)
//!
//! 組み込みオブジェクトの型情報を(Contextに)定義
#![allow(non_snake_case)]

pub mod const_func;

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
use crate::ty::typaram::TyParam;
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
    ClassDefType, Context, ContextKind, DefaultInfo, MethodInfo, ParamSpec, TypeRelationInstance,
};
use crate::mod_cache::SharedModuleCache;
use crate::varinfo::{Mutability, VarInfo, VarKind};
use DefaultInfo::*;
use Mutability::*;
use VarKind::*;
use Visibility::*;

const NUM: &str = "Num";

const NAME: &str = "__name__";
const LICENSE: &str = "license";
const CREDITS: &str = "credits";
const COPYRIGHT: &str = "copyright";
const TRUE: &str = "True";
const FALSE: &str = "False";
const NONE: &str = "None";
const NOT_IMPLEMENTED: &str = "NotImplemented";
const ELLIPSIS: &str = "Ellipsis";
const SITEBUILTINS_PRINTER: &str = "_sitebuiltins._Printer";

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

    /// see std/prelude.er
    /// All type boundaries are defined in each subroutine
    /// `push_subtype_bound`, etc. are used for type boundary determination in user-defined APIs
    // 型境界はすべて各サブルーチンで定義する
    // push_subtype_boundなどはユーザー定義APIの型境界決定のために使用する
    fn init_builtin_traits(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let unpack = Self::builtin_mono_trait("Unpack", 2);
        let inheritable_type = Self::builtin_mono_trait("InheritableType", 2);
        let named = Self::builtin_mono_trait("Named", 2);
        let mut mutable = Self::builtin_mono_trait("Mutable", 2);
        let Slf = mono_q("Self", subtypeof(mono("Immutizable")));
        let immut_t = proj(Slf.clone(), "ImmutType");
        let f_t = func(vec![kw("old", immut_t.clone())], None, vec![], immut_t);
        let t = pr1_met(ref_mut(Slf, None), f_t, NoneType).quantify();
        mutable.register_builtin_decl("update!", t, Public);
        // REVIEW: Immutatable?
        let mut immutizable = Self::builtin_mono_trait("Immutizable", 2);
        immutizable.register_superclass(mono("Mutable"), &mutable);
        immutizable.register_builtin_decl("ImmutType", Type, Public);
        // REVIEW: Mutatable?
        let mut mutizable = Self::builtin_mono_trait("Mutizable", 2);
        mutizable.register_builtin_decl("MutType!", Type, Public);
        let pathlike = Self::builtin_mono_trait("PathLike", 2);
        /* Readable */
        let mut readable = Self::builtin_mono_trait("Readable!", 2);
        let Slf = mono_q("Self", subtypeof(mono("Readable!")));
        let t_read = pr_met(ref_mut(Slf, None), vec![], None, vec![kw("n", Int)], Str).quantify();
        readable.register_builtin_py_decl("read!", t_read, Public, Some("read"));
        /* Writable */
        let mut writable = Self::builtin_mono_trait("Writable!", 2);
        let Slf = mono_q("Self", subtypeof(mono("Writable!")));
        let t_write = pr1_kw_met(ref_mut(Slf, None), kw("s", Str), Nat).quantify();
        writable.register_builtin_py_decl("write!", t_write, Public, Some("write"));
        // TODO: Add required methods
        let mut filelike = Self::builtin_mono_trait("FileLike", 2);
        filelike.register_superclass(mono("Readable"), &readable);
        let mut filelike_mut = Self::builtin_mono_trait("FileLike!", 2);
        filelike_mut.register_superclass(mono("FileLike"), &filelike);
        filelike_mut.register_superclass(mono("Writable!"), &writable);
        /* Show */
        let mut show = Self::builtin_mono_trait("Show", 2);
        let Slf = mono_q("Self", subtypeof(mono("Show")));
        let t_show = fn0_met(ref_(Slf), Str).quantify();
        show.register_builtin_py_decl("to_str", t_show, Public, Some("__str__"));
        /* In */
        let mut in_ = Self::builtin_poly_trait("In", vec![PS::t("T", NonDefault)], 2);
        let params = vec![PS::t("T", NonDefault)];
        let input = Self::builtin_poly_trait("Input", params.clone(), 2);
        let output = Self::builtin_poly_trait("Output", params, 2);
        let T = mono_q("T", instanceof(Type));
        let I = mono_q("I", subtypeof(poly("In", vec![ty_tp(T.clone())])));
        in_.register_superclass(poly("Input", vec![ty_tp(T.clone())]), &input);
        let op_t = fn1_met(T.clone(), I, Bool).quantify();
        in_.register_builtin_decl("__in__", op_t, Public);
        /* Eq */
        // Erg does not have a trait equivalent to `PartialEq` in Rust
        // This means, Erg's `Float` cannot be compared with other `Float`
        // use `l - r < EPSILON` to check if two floats are almost equal
        let mut eq = Self::builtin_mono_trait("Eq", 2);
        let Slf = mono_q("Self", subtypeof(mono("Eq")));
        // __eq__: |Self <: Eq| (self: Self, other: Self) -> Bool
        let op_t = fn1_met(Slf.clone(), Slf, Bool).quantify();
        eq.register_builtin_decl("__eq__", op_t, Public);
        /* Ord */
        let mut ord = Self::builtin_mono_trait("Ord", 2);
        ord.register_superclass(mono("Eq"), &eq);
        let Slf = mono_q("Self", subtypeof(mono("Ord")));
        let op_t = fn1_met(Slf.clone(), Slf, or(mono("Ordering"), NoneType)).quantify();
        ord.register_builtin_decl("__cmp__", op_t, Public);
        // FIXME: poly trait
        /* Num */
        let num = Self::builtin_mono_trait(NUM, 2);
        /* vec![
            poly("Add", vec![]),
            poly("Sub", vec![]),
            poly("Mul", vec![]),
        ], */
        /* Seq */
        let mut seq = Self::builtin_poly_trait("Seq", vec![PS::t("T", NonDefault)], 2);
        seq.register_superclass(poly("Output", vec![ty_tp(T.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Seq", vec![TyParam::erased(Type)])));
        let t = fn0_met(Slf.clone(), Nat).quantify();
        seq.register_builtin_decl("len", t, Public);
        let t = fn1_met(Slf, Nat, T.clone()).quantify();
        // Seq.get: |Self <: Seq(T)| Self.(Nat) -> T
        seq.register_builtin_decl("get", t, Public);
        /* Iterable */
        let mut iterable = Self::builtin_poly_trait("Iterable", vec![PS::t("T", NonDefault)], 2);
        iterable.register_superclass(poly("Output", vec![ty_tp(T.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Iterable", vec![ty_tp(T.clone())])));
        let t = fn0_met(Slf.clone(), proj(Slf, "Iter")).quantify();
        iterable.register_builtin_py_decl("iter", t, Public, Some("__iter__"));
        iterable.register_builtin_decl("Iter", Type, Public);
        let R = mono_q("R", instanceof(Type));
        let params = vec![PS::t("R", WithDefault)];
        let ty_params = vec![ty_tp(R.clone())];
        /* Num */
        let mut add = Self::builtin_poly_trait("Add", params.clone(), 2);
        // Covariant with `R` (independent of the type of __add__)
        add.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Add", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, "Output")).quantify();
        add.register_builtin_decl("__add__", op_t, Public);
        add.register_builtin_decl("Output", Type, Public);
        /* Sub */
        let mut sub = Self::builtin_poly_trait("Sub", params.clone(), 2);
        sub.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Sub", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, "Output")).quantify();
        sub.register_builtin_decl("__sub__", op_t, Public);
        sub.register_builtin_decl("Output", Type, Public);
        /* Mul */
        let mut mul = Self::builtin_poly_trait("Mul", params.clone(), 2);
        mul.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Mul", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, "Output")).quantify();
        mul.register_builtin_decl("__mul__", op_t, Public);
        mul.register_builtin_decl("Output", Type, Public);
        /* Div */
        let mut div = Self::builtin_poly_trait("Div", params.clone(), 2);
        div.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Div", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, "Output")).quantify();
        div.register_builtin_decl("__div__", op_t, Public);
        div.register_builtin_decl("Output", Type, Public);
        /* FloorDiv */
        let mut floor_div = Self::builtin_poly_trait("FloorDiv", params, 2);
        floor_div.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("FloorDiv", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R, proj(Slf.clone(), "Output")).quantify();
        floor_div.register_builtin_decl("__floordiv__", op_t, Public);
        floor_div.register_builtin_decl("Output", Type, Public);
        self.register_builtin_type(mono("Unpack"), unpack, vis, Const, None);
        self.register_builtin_type(
            mono("InheritableType"),
            inheritable_type,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(mono("Named"), named, vis, Const, None);
        self.register_builtin_type(mono("Mutable"), mutable, vis, Const, None);
        self.register_builtin_type(mono("Immutizable"), immutizable, vis, Const, None);
        self.register_builtin_type(mono("Mutizable"), mutizable, vis, Const, None);
        self.register_builtin_type(mono("PathLike"), pathlike, vis, Const, None);
        self.register_builtin_type(
            mono("Readable!"),
            readable,
            Private,
            Const,
            Some("Readable"),
        );
        self.register_builtin_type(
            mono("Writable!"),
            writable,
            Private,
            Const,
            Some("Writable"),
        );
        self.register_builtin_type(mono("FileLike"), filelike, vis, Const, None);
        self.register_builtin_type(mono("FileLike!"), filelike_mut, vis, Const, None);
        self.register_builtin_type(mono("Show"), show, vis, Const, None);
        self.register_builtin_type(
            poly("Input", vec![ty_tp(T.clone())]),
            input,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(
            poly("Output", vec![ty_tp(T.clone())]),
            output,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(
            poly("In", vec![ty_tp(T.clone())]),
            in_,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(mono("Eq"), eq, vis, Const, None);
        self.register_builtin_type(mono("Ord"), ord, vis, Const, None);
        self.register_builtin_type(mono(NUM), num, vis, Const, None);
        self.register_builtin_type(
            poly("Seq", vec![ty_tp(T.clone())]),
            seq,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(
            poly("Iterable", vec![ty_tp(T)]),
            iterable,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(poly("Add", ty_params.clone()), add, vis, Const, None);
        self.register_builtin_type(poly("Sub", ty_params.clone()), sub, vis, Const, None);
        self.register_builtin_type(poly("Mul", ty_params.clone()), mul, vis, Const, None);
        self.register_builtin_type(poly("Div", ty_params.clone()), div, vis, Const, None);
        self.register_builtin_type(poly("FloorDiv", ty_params), floor_div, vis, Const, None);
        self.register_const_param_defaults(
            "Add",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            "Sub",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            "Mul",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            "Div",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            "FloorDiv",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf))],
        );
    }

    fn init_builtin_classes(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let T = mono_q("T", instanceof(Type));
        let U = mono_q("U", instanceof(Type));
        let L = mono_q("L", instanceof(Type));
        let R = mono_q("R", instanceof(Type));
        let N = mono_q_tp("N", instanceof(Nat));
        let M = mono_q_tp("M", instanceof(Nat));
        let never = Self::builtin_mono_class("Never", 1);
        /* Obj */
        let mut obj = Self::builtin_mono_class("Obj", 2);
        let Slf = mono_q("Self", subtypeof(Obj));
        let t = fn0_met(Slf.clone(), Slf).quantify();
        obj.register_builtin_impl("clone", t, Const, Public);
        obj.register_builtin_impl("__module__", Str, Const, Public);
        obj.register_builtin_impl("__sizeof__", fn0_met(Obj, Nat), Const, Public);
        obj.register_builtin_impl("__repr__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_builtin_impl("__str__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_builtin_impl(
            "__dict__",
            fn0_met(Obj, dict! {Str => Obj}.into()),
            Immutable,
            Public,
        );
        obj.register_builtin_impl("__bytes__", fn0_met(Obj, mono("Bytes")), Immutable, Public);
        let mut obj_in = Self::builtin_methods(Some(poly("In", vec![ty_tp(Type)])), 2);
        obj_in.register_builtin_impl("__in__", fn1_met(Obj, Type, Bool), Const, Public);
        obj.register_trait(Obj, obj_in);
        let mut obj_mutizable = Self::builtin_methods(Some(mono("Mutizable")), 1);
        obj_mutizable.register_builtin_const("MutType!", Public, ValueObj::builtin_t(mono("Obj!")));
        obj.register_trait(Obj, obj_mutizable);
        // Obj does not implement Eq

        /* Float */
        let mut float = Self::builtin_mono_class("Float", 2);
        float.register_superclass(Obj, &obj);
        // TODO: support multi platform
        float.register_builtin_const("EPSILON", Public, ValueObj::Float(2.220446049250313e-16));
        float.register_builtin_py_impl("Real", Float, Const, Public, Some("real"));
        float.register_builtin_py_impl("Imag", Float, Const, Public, Some("imag"));
        float.register_builtin_py_impl(
            "conjugate",
            fn0_met(Float, Float),
            Const,
            Public,
            Some("conjugate"),
        );
        float.register_builtin_py_impl(
            "is_integer",
            fn0_met(Float, Bool),
            Const,
            Public,
            Some("is_integer"),
        );
        float.register_builtin_py_impl("hex", fn0_met(Float, Str), Const, Public, Some("hex"));
        float.register_builtin_py_impl(
            "fromhex",
            nd_func(vec![kw("s", Str)], None, Float),
            Const,
            Public,
            Some("fromhex"),
        );
        float.register_marker_trait(mono(NUM));
        float.register_marker_trait(mono("Ord"));
        let mut float_ord = Self::builtin_methods(Some(mono("Ord")), 2);
        float_ord.register_builtin_impl(
            "__cmp__",
            fn1_met(Float, Float, mono("Ordering")),
            Const,
            Public,
        );
        float.register_trait(Float, float_ord);
        // Float doesn't have an `Eq` implementation
        let op_t = fn1_met(Float, Float, Float);
        let mut float_add = Self::builtin_methods(Some(poly("Add", vec![ty_tp(Float)])), 2);
        float_add.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        float_add.register_builtin_const("Output", Public, ValueObj::builtin_t(Float));
        float.register_trait(Float, float_add);
        let mut float_sub = Self::builtin_methods(Some(poly("Sub", vec![ty_tp(Float)])), 2);
        float_sub.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        float_sub.register_builtin_const("Output", Public, ValueObj::builtin_t(Float));
        float.register_trait(Float, float_sub);
        let mut float_mul = Self::builtin_methods(Some(poly("Mul", vec![ty_tp(Float)])), 2);
        float_mul.register_builtin_impl("__mul__", op_t.clone(), Const, Public);
        float_mul.register_builtin_const("Output", Public, ValueObj::builtin_t(Float));
        float_mul.register_builtin_const("PowOutput", Public, ValueObj::builtin_t(Float));
        float.register_trait(Float, float_mul);
        let mut float_div = Self::builtin_methods(Some(poly("Div", vec![ty_tp(Float)])), 2);
        float_div.register_builtin_impl("__div__", op_t.clone(), Const, Public);
        float_div.register_builtin_const("Output", Public, ValueObj::builtin_t(Float));
        float_div.register_builtin_const("ModOutput", Public, ValueObj::builtin_t(Float));
        float.register_trait(Float, float_div);
        let mut float_floordiv =
            Self::builtin_methods(Some(poly("FloorDiv", vec![ty_tp(Float)])), 2);
        float_floordiv.register_builtin_impl("__floordiv__", op_t, Const, Public);
        float_floordiv.register_builtin_const("Output", Public, ValueObj::builtin_t(Float));
        float.register_trait(Float, float_floordiv);
        let mut float_mutizable = Self::builtin_methods(Some(mono("Mutizable")), 2);
        float_mutizable.register_builtin_const(
            "MutType!",
            Public,
            ValueObj::builtin_t(mono("Float!")),
        );
        float.register_trait(Float, float_mutizable);
        let mut float_show = Self::builtin_methods(Some(mono("Show")), 1);
        let t = fn0_met(Float, Str);
        float_show.register_builtin_py_impl("to_str", t, Immutable, Public, Some("__str__"));
        float.register_trait(Float, float_show);

        /* Ratio */
        // TODO: Int, Nat, Boolの継承元をRatioにする(今はFloat)
        let mut ratio = Self::builtin_mono_class("Ratio", 2);
        ratio.register_superclass(Obj, &obj);
        ratio.register_builtin_py_impl("Real", Ratio, Const, Public, Some("real"));
        ratio.register_builtin_py_impl("Imag", Ratio, Const, Public, Some("imag"));
        ratio.register_marker_trait(mono(NUM));
        ratio.register_marker_trait(mono("Ord"));
        let mut ratio_ord = Self::builtin_methods(Some(mono("Ord")), 2);
        ratio_ord.register_builtin_impl(
            "__cmp__",
            fn1_met(Ratio, Ratio, mono("Ordering")),
            Const,
            Public,
        );
        ratio.register_trait(Ratio, ratio_ord);
        let mut ratio_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        ratio_eq.register_builtin_impl("__eq__", fn1_met(Ratio, Ratio, Bool), Const, Public);
        ratio.register_trait(Ratio, ratio_eq);
        let op_t = fn1_met(Ratio, Ratio, Ratio);
        let mut ratio_add = Self::builtin_methods(Some(poly("Add", vec![ty_tp(Ratio)])), 2);
        ratio_add.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        ratio_add.register_builtin_const("Output", Public, ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, ratio_add);
        let mut ratio_sub = Self::builtin_methods(Some(poly("Sub", vec![ty_tp(Ratio)])), 2);
        ratio_sub.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        ratio_sub.register_builtin_const("Output", Public, ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, ratio_sub);
        let mut ratio_mul = Self::builtin_methods(Some(poly("Mul", vec![ty_tp(Ratio)])), 2);
        ratio_mul.register_builtin_impl("__mul__", op_t.clone(), Const, Public);
        ratio_mul.register_builtin_const("Output", Public, ValueObj::builtin_t(Ratio));
        ratio_mul.register_builtin_const("PowOutput", Public, ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, ratio_mul);
        let mut ratio_div = Self::builtin_methods(Some(poly("Div", vec![ty_tp(Ratio)])), 2);
        ratio_div.register_builtin_impl("__div__", op_t.clone(), Const, Public);
        ratio_div.register_builtin_const("Output", Public, ValueObj::builtin_t(Ratio));
        ratio_div.register_builtin_const("ModOutput", Public, ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, ratio_div);
        let mut ratio_floordiv =
            Self::builtin_methods(Some(poly("FloorDiv", vec![ty_tp(Ratio)])), 2);
        ratio_floordiv.register_builtin_impl("__floordiv__", op_t, Const, Public);
        ratio_floordiv.register_builtin_const("Output", Public, ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, ratio_floordiv);
        let mut ratio_mutizable = Self::builtin_methods(Some(mono("Mutizable")), 2);
        ratio_mutizable.register_builtin_const(
            "MutType!",
            Public,
            ValueObj::builtin_t(mono("Ratio!")),
        );
        ratio.register_trait(Ratio, ratio_mutizable);
        let mut ratio_show = Self::builtin_methods(Some(mono("Show")), 1);
        let t = fn0_met(Ratio, Str);
        ratio_show.register_builtin_impl("to_str", t, Immutable, Public);
        ratio.register_trait(Ratio, ratio_show);

        /* Int */
        let mut int = Self::builtin_mono_class("Int", 2);
        int.register_superclass(Float, &float); // TODO: Float -> Ratio
        int.register_marker_trait(mono(NUM));
        // class("Rational"),
        // class("Integral"),
        int.register_builtin_py_impl("abs", fn0_met(Int, Nat), Immutable, Public, Some("__abs__"));
        int.register_builtin_py_impl("succ", fn0_met(Int, Int), Immutable, Public, Some("succ"));
        int.register_builtin_py_impl("pred", fn0_met(Int, Int), Immutable, Public, Some("pred"));
        int.register_builtin_py_impl(
            "bit_length",
            fn0_met(Int, Nat),
            Immutable,
            Public,
            Some("bit_length"),
        );
        int.register_builtin_py_impl(
            "bit_count",
            fn0_met(Int, Nat),
            Immutable,
            Public,
            Some("bit_count"),
        );
        let t_from_bytes = func(
            vec![kw(
                "bytes",
                or(
                    mono("Bytes"),
                    array_t(Type::from(value(0)..=value(255)), TyParam::erased(Nat)),
                ),
            )],
            None,
            vec![kw(
                "byteorder",
                v_enum(set! {ValueObj::Str("big".into()), ValueObj::Str("little".into())}),
            )],
            Int,
        );
        int.register_builtin_py_impl(
            "from_bytes",
            t_from_bytes,
            Const,
            Public,
            Some("from_bytes"),
        );
        let t_to_bytes = func(
            vec![kw("self", Int)],
            None,
            vec![
                kw("length", Nat),
                kw(
                    "byteorder",
                    v_enum(set! {ValueObj::Str("big".into()), ValueObj::Str("little".into())}),
                ),
            ],
            mono("Bytes"),
        );
        int.register_builtin_py_impl("to_bytes", t_to_bytes, Immutable, Public, Some("to_bytes"));
        let mut int_ord = Self::builtin_methods(Some(mono("Ord")), 2);
        int_ord.register_builtin_impl(
            "__partial_cmp__",
            fn1_met(Int, Int, or(mono("Ordering"), NoneType)),
            Const,
            Public,
        );
        int.register_trait(Int, int_ord);
        let mut int_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        int_eq.register_builtin_impl("__eq__", fn1_met(Int, Int, Bool), Const, Public);
        int.register_trait(Int, int_eq);
        // __div__ is not included in Int (cast to Ratio)
        let op_t = fn1_met(Int, Int, Int);
        let mut int_add = Self::builtin_methods(Some(poly("Add", vec![ty_tp(Int)])), 2);
        int_add.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        int_add.register_builtin_const("Output", Public, ValueObj::builtin_t(Int));
        int.register_trait(Int, int_add);
        let mut int_sub = Self::builtin_methods(Some(poly("Sub", vec![ty_tp(Int)])), 2);
        int_sub.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        int_sub.register_builtin_const("Output", Public, ValueObj::builtin_t(Int));
        int.register_trait(Int, int_sub);
        let mut int_mul = Self::builtin_methods(Some(poly("Mul", vec![ty_tp(Int)])), 2);
        int_mul.register_builtin_impl("__mul__", op_t.clone(), Const, Public);
        int_mul.register_builtin_const("Output", Public, ValueObj::builtin_t(Int));
        int_mul.register_builtin_const("PowOutput", Public, ValueObj::builtin_t(Nat));
        int.register_trait(Int, int_mul);
        let mut int_floordiv = Self::builtin_methods(Some(poly("FloorDiv", vec![ty_tp(Int)])), 2);
        int_floordiv.register_builtin_impl("__floordiv__", op_t, Const, Public);
        int_floordiv.register_builtin_const("Output", Public, ValueObj::builtin_t(Int));
        int.register_trait(Int, int_floordiv);
        let mut int_mutizable = Self::builtin_methods(Some(mono("Mutizable")), 2);
        int_mutizable.register_builtin_const("MutType!", Public, ValueObj::builtin_t(mono("Int!")));
        int.register_trait(Int, int_mutizable);
        let mut int_show = Self::builtin_methods(Some(mono("Show")), 1);
        let t = fn0_met(Int, Str);
        int_show.register_builtin_py_impl("to_str", t, Immutable, Public, Some("__str__"));
        int.register_trait(Int, int_show);
        int.register_builtin_py_impl("Real", Int, Const, Public, Some("real"));
        int.register_builtin_py_impl("Imag", Int, Const, Public, Some("imag"));

        /* Nat */
        let mut nat = Self::builtin_mono_class("Nat", 10);
        nat.register_superclass(Int, &int);
        // class("Rational"),
        // class("Integral"),
        nat.register_builtin_py_impl(
            "times!",
            pr_met(
                Nat,
                vec![kw("proc!", nd_proc(vec![], None, NoneType))],
                None,
                vec![],
                NoneType,
            ),
            Immutable,
            Public,
            Some("times"),
        );
        nat.register_marker_trait(mono(NUM));
        let mut nat_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        nat_eq.register_builtin_impl("__eq__", fn1_met(Nat, Nat, Bool), Const, Public);
        nat.register_trait(Nat, nat_eq);
        let mut nat_ord = Self::builtin_methods(Some(mono("Ord")), 2);
        nat_ord.register_builtin_impl(
            "__cmp__",
            fn1_met(Nat, Nat, mono("Ordering")),
            Const,
            Public,
        );
        nat.register_trait(Nat, nat_ord);
        // __sub__, __div__ is not included in Nat (cast to Int/ Ratio)
        let op_t = fn1_met(Nat, Nat, Nat);
        let mut nat_add = Self::builtin_methods(Some(poly("Add", vec![ty_tp(Nat)])), 2);
        nat_add.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        nat_add.register_builtin_const("Output", Public, ValueObj::builtin_t(Nat));
        nat.register_trait(Nat, nat_add);
        let mut nat_mul = Self::builtin_methods(Some(poly("Mul", vec![ty_tp(Nat)])), 2);
        nat_mul.register_builtin_impl("__mul__", op_t.clone(), Const, Public);
        nat_mul.register_builtin_const("Output", Public, ValueObj::builtin_t(Nat));
        nat.register_trait(Nat, nat_mul);
        let mut nat_floordiv = Self::builtin_methods(Some(poly("FloorDiv", vec![ty_tp(Nat)])), 2);
        nat_floordiv.register_builtin_impl("__floordiv__", op_t, Const, Public);
        nat_floordiv.register_builtin_const("Output", Public, ValueObj::builtin_t(Nat));
        nat.register_trait(Nat, nat_floordiv);
        let mut nat_mutizable = Self::builtin_methods(Some(mono("Mutizable")), 2);
        nat_mutizable.register_builtin_const("MutType!", Public, ValueObj::builtin_t(mono("Nat!")));
        nat.register_trait(Nat, nat_mutizable);
        nat.register_builtin_impl("Real", Nat, Const, Public);
        nat.register_builtin_impl("Imag", Nat, Const, Public);

        /* Bool */
        let mut bool_ = Self::builtin_mono_class("Bool", 10);
        bool_.register_superclass(Nat, &nat);
        // class("Rational"),
        // class("Integral"),
        bool_.register_builtin_impl("__and__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_builtin_impl("__or__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_marker_trait(mono(NUM));
        let mut bool_ord = Self::builtin_methods(Some(mono("Ord")), 2);
        bool_ord.register_builtin_impl(
            "__cmp__",
            fn1_met(Bool, Bool, mono("Ordering")),
            Const,
            Public,
        );
        bool_.register_trait(Bool, bool_ord);
        let mut bool_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        bool_eq.register_builtin_impl("__eq__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_trait(Bool, bool_eq);
        let mut bool_mutizable = Self::builtin_methods(Some(mono("Mutizable")), 2);
        bool_mutizable.register_builtin_const(
            "MutType!",
            Public,
            ValueObj::builtin_t(mono("Bool!")),
        );
        bool_.register_trait(Bool, bool_mutizable);
        let mut bool_show = Self::builtin_methods(Some(mono("Show")), 1);
        bool_show.register_builtin_impl("to_str", fn0_met(Bool, Str), Immutable, Public);
        bool_.register_trait(Bool, bool_show);
        /* Str */
        let mut str_ = Self::builtin_mono_class("Str", 10);
        str_.register_superclass(Obj, &obj);
        str_.register_marker_trait(mono("Ord"));
        str_.register_marker_trait(mono("PathLike"));
        str_.register_builtin_impl(
            "replace",
            fn_met(
                Str,
                vec![kw("pat", Str), kw("into", Str)],
                None,
                vec![],
                Str,
            ),
            Immutable,
            Public,
        );
        str_.register_builtin_impl(
            "encode",
            fn_met(
                Str,
                vec![],
                None,
                vec![kw("encoding", Str), kw("errors", Str)],
                mono("Bytes"),
            ),
            Immutable,
            Public,
        );
        str_.register_builtin_impl(
            "format",
            fn_met(Str, vec![], Some(kw("args", Obj)), vec![], Str),
            Immutable,
            Public,
        );
        str_.register_builtin_impl(
            "lower",
            fn_met(Str, vec![], None, vec![], Str),
            Immutable,
            Public,
        );
        str_.register_builtin_impl(
            "upper",
            fn_met(Str, vec![], None, vec![], Str),
            Immutable,
            Public,
        );
        str_.register_builtin_impl(
            "to_int",
            fn_met(Str, vec![], None, vec![], or(Int, NoneType)),
            Immutable,
            Public,
        );
        let str_getitem_t = fn1_kw_met(Str, kw("idx", Nat), Str);
        str_.register_builtin_impl("__getitem__", str_getitem_t, Immutable, Public);
        let mut str_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        str_eq.register_builtin_impl("__eq__", fn1_met(Str, Str, Bool), Const, Public);
        str_.register_trait(Str, str_eq);
        let mut str_seq = Self::builtin_methods(Some(poly("Seq", vec![ty_tp(Str)])), 2);
        str_seq.register_builtin_impl("len", fn0_met(Str, Nat), Const, Public);
        str_seq.register_builtin_impl("get", fn1_met(Str, Nat, Str), Const, Public);
        str_.register_trait(Str, str_seq);
        let mut str_add = Self::builtin_methods(Some(poly("Add", vec![ty_tp(Str)])), 2);
        str_add.register_builtin_impl("__add__", fn1_met(Str, Str, Str), Const, Public);
        str_add.register_builtin_const("Output", Public, ValueObj::builtin_t(Str));
        str_.register_trait(Str, str_add);
        let mut str_mul = Self::builtin_methods(Some(poly("Mul", vec![ty_tp(Nat)])), 2);
        str_mul.register_builtin_impl("__mul__", fn1_met(Str, Nat, Str), Const, Public);
        str_mul.register_builtin_const("Output", Public, ValueObj::builtin_t(Str));
        str_.register_trait(Str, str_mul);
        let mut str_mutizable = Self::builtin_methods(Some(mono("Mutizable")), 2);
        str_mutizable.register_builtin_const("MutType!", Public, ValueObj::builtin_t(mono("Str!")));
        str_.register_trait(Str, str_mutizable);
        let mut str_show = Self::builtin_methods(Some(mono("Show")), 1);
        str_show.register_builtin_impl("to_str", fn0_met(Str, Str), Immutable, Public);
        str_.register_trait(Str, str_show);
        let mut str_iterable = Self::builtin_methods(Some(poly("Iterable", vec![ty_tp(Str)])), 2);
        str_iterable.register_builtin_py_impl(
            "iter",
            fn0_met(Str, mono("StrIterator")),
            Immutable,
            Public,
            Some("__iter__"),
        );
        str_iterable.register_builtin_const(
            "Iterator",
            vis,
            ValueObj::builtin_t(mono("StrIterator")),
        );
        str_.register_trait(Str, str_iterable);
        /* NoneType */
        let mut nonetype = Self::builtin_mono_class("NoneType", 10);
        nonetype.register_superclass(Obj, &obj);
        let mut nonetype_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        nonetype_eq.register_builtin_impl(
            "__eq__",
            fn1_met(NoneType, NoneType, Bool),
            Const,
            Public,
        );
        nonetype.register_trait(NoneType, nonetype_eq);
        let mut nonetype_show = Self::builtin_methods(Some(mono("Show")), 1);
        nonetype_show.register_builtin_impl("to_str", fn0_met(NoneType, Str), Immutable, Public);
        nonetype.register_trait(NoneType, nonetype_show);
        /* Type */
        let mut type_ = Self::builtin_mono_class("Type", 2);
        type_.register_superclass(Obj, &obj);
        type_.register_builtin_impl(
            "mro",
            array_t(Type, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        type_.register_marker_trait(mono("Named"));
        let mut type_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        type_eq.register_builtin_impl("__eq__", fn1_met(Type, Type, Bool), Const, Public);
        type_.register_trait(Type, type_eq);
        let mut class_type = Self::builtin_mono_class("ClassType", 2);
        class_type.register_superclass(Type, &type_);
        class_type.register_marker_trait(mono("Named"));
        let mut class_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        class_eq.register_builtin_impl(
            "__eq__",
            fn1_met(ClassType, ClassType, Bool),
            Const,
            Public,
        );
        class_type.register_trait(ClassType, class_eq);
        let mut trait_type = Self::builtin_mono_class("TraitType", 2);
        trait_type.register_superclass(Type, &type_);
        trait_type.register_marker_trait(mono("Named"));
        let mut trait_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        trait_eq.register_builtin_impl(
            "__eq__",
            fn1_met(TraitType, TraitType, Bool),
            Const,
            Public,
        );
        trait_type.register_trait(TraitType, trait_eq);
        let mut code = Self::builtin_mono_class("Code", 10);
        code.register_superclass(Obj, &obj);
        code.register_builtin_impl("co_argcount", Nat, Immutable, Public);
        code.register_builtin_impl(
            "co_varnames",
            array_t(Str, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        code.register_builtin_impl(
            "co_consts",
            array_t(Obj, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        code.register_builtin_impl(
            "co_names",
            array_t(Str, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        code.register_builtin_impl(
            "co_freevars",
            array_t(Str, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        code.register_builtin_impl(
            "co_cellvars",
            array_t(Str, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        code.register_builtin_impl("co_filename", Str, Immutable, Public);
        code.register_builtin_impl("co_name", Str, Immutable, Public);
        code.register_builtin_impl("co_firstlineno", Nat, Immutable, Public);
        code.register_builtin_impl("co_stacksize", Nat, Immutable, Public);
        code.register_builtin_impl("co_flags", Nat, Immutable, Public);
        code.register_builtin_impl("co_code", mono("Bytes"), Immutable, Public);
        code.register_builtin_impl("co_lnotab", mono("Bytes"), Immutable, Public);
        code.register_builtin_impl("co_nlocals", Nat, Immutable, Public);
        code.register_builtin_impl("co_kwonlyargcount", Nat, Immutable, Public);
        code.register_builtin_impl("co_posonlyargcount", Nat, Immutable, Public);
        let mut code_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        code_eq.register_builtin_impl("__eq__", fn1_met(Code, Code, Bool), Const, Public);
        code.register_trait(Code, code_eq);
        let g_module_t = mono("GenericModule");
        let mut generic_module = Self::builtin_mono_class("GenericModule", 2);
        generic_module.register_superclass(Obj, &obj);
        generic_module.register_marker_trait(mono("Named"));
        let mut generic_module_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        generic_module_eq.register_builtin_impl(
            "__eq__",
            fn1_met(g_module_t.clone(), g_module_t.clone(), Bool),
            Const,
            Public,
        );
        generic_module.register_trait(g_module_t.clone(), generic_module_eq);
        let Path = mono_q_tp("Path", instanceof(Str));
        let module_t = module(Path.clone());
        let py_module_t = py_module(Path);
        let mut module = Self::builtin_poly_class("Module", vec![PS::named_nd("Path", Str)], 2);
        module.register_superclass(g_module_t.clone(), &generic_module);
        let mut py_module =
            Self::builtin_poly_class("PyModule", vec![PS::named_nd("Path", Str)], 2);
        if !cfg!(feature = "py_compatible") {
            py_module.register_superclass(g_module_t.clone(), &generic_module);
        }
        /* Array */
        let mut array_ =
            Self::builtin_poly_class("Array", vec![PS::t_nd("T"), PS::named_nd("N", Nat)], 10);
        array_.register_superclass(Obj, &obj);
        array_.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        let arr_t = array_t(T.clone(), N.clone());
        let t = fn_met(
            arr_t.clone(),
            vec![kw("rhs", array_t(T.clone(), M.clone()))],
            None,
            vec![],
            array_t(T.clone(), N.clone() + M.clone()),
        )
        .quantify();
        array_.register_builtin_py_impl("concat", t.clone(), Immutable, Public, Some("__add__"));
        let t_count = fn_met(arr_t.clone(), vec![kw("x", T.clone())], None, vec![], Nat).quantify();
        array_.register_builtin_py_impl("count", t_count, Immutable, Public, Some("count"));
        // Array(T, N)|<: Add(Array(T, M))|.
        //     Output = Array(T, N + M)
        //     __add__: (self: Array(T, N), other: Array(T, M)) -> Array(T, N + M) = Array.concat
        let mut array_add = Self::builtin_methods(
            Some(poly("Add", vec![ty_tp(array_t(T.clone(), M.clone()))])),
            2,
        );
        array_add.register_builtin_impl("__add__", t, Immutable, Public);
        let out_t = array_t(T.clone(), N.clone() + M.clone());
        array_add.register_builtin_const("Output", Public, ValueObj::builtin_t(out_t));
        array_.register_trait(arr_t.clone(), array_add);
        let t = fn_met(
            arr_t.clone(),
            vec![kw("elem", T.clone())],
            None,
            vec![],
            array_t(T.clone(), N.clone() + value(1usize)),
        )
        .quantify();
        array_.register_builtin_impl("push", t, Immutable, Public);
        // [T; N].MutType! = [T; !N] (neither [T!; N] nor [T; N]!)
        let mut_type = ValueObj::builtin_t(poly(
            "Array!",
            vec![TyParam::t(T.clone()), N.clone().mutate()],
        ));
        array_.register_builtin_const("MutType!", Public, mut_type);
        let var = Str::from(fresh_varname());
        let input = refinement(
            var.clone(),
            Nat,
            set! { Predicate::le(var, N.clone() - value(1usize)) },
        );
        // __getitem__: |T, N|(self: [T; N], _: {I: Nat | I <= N}) -> T
        let array_getitem_t =
            fn1_kw_met(array_t(T.clone(), N.clone()), anon(input), T.clone()).quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            "__getitem__",
            __array_getitem__,
            array_getitem_t,
            None,
        )));
        array_.register_builtin_const("__getitem__", Public, get_item);
        let mut array_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        array_eq.register_builtin_impl(
            "__eq__",
            fn1_met(arr_t.clone(), arr_t.clone(), Bool).quantify(),
            Const,
            Public,
        );
        array_.register_trait(arr_t.clone(), array_eq);
        array_.register_marker_trait(mono("Mutizable"));
        array_.register_marker_trait(poly("Seq", vec![ty_tp(T.clone())]));
        let mut array_show = Self::builtin_methods(Some(mono("Show")), 1);
        array_show.register_builtin_py_impl(
            "to_str",
            fn0_met(arr_t.clone(), Str).quantify(),
            Immutable,
            Public,
            Some("__str__"),
        );
        array_.register_trait(arr_t.clone(), array_show);
        let mut array_iterable =
            Self::builtin_methods(Some(poly("Iterable", vec![ty_tp(T.clone())])), 2);
        let array_iter = poly("ArrayIterator", vec![ty_tp(T.clone())]);
        let t = fn0_met(array_t(T.clone(), TyParam::erased(Nat)), array_iter.clone()).quantify();
        array_iterable.register_builtin_py_impl("iter", t, Immutable, Public, Some("__iter__"));
        array_iterable.register_builtin_const("Iterator", vis, ValueObj::builtin_t(array_iter));
        array_.register_trait(arr_t.clone(), array_iterable);
        /* Set */
        let mut set_ =
            Self::builtin_poly_class("Set", vec![PS::t_nd("T"), PS::named_nd("N", Nat)], 10);
        let set_t = set_t(T.clone(), N.clone());
        set_.register_superclass(Obj, &obj);
        set_.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        let t = fn_met(
            set_t.clone(),
            vec![kw("rhs", array_t(T.clone(), M.clone()))],
            None,
            vec![],
            array_t(T.clone(), N.clone() + M),
        )
        .quantify();
        set_.register_builtin_impl("concat", t, Immutable, Public);
        let mut_type = ValueObj::builtin_t(poly(
            "Set!",
            vec![TyParam::t(T.clone()), N.clone().mutate()],
        ));
        set_.register_builtin_const("MutType!", Public, mut_type);
        let mut set_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        set_eq.register_builtin_impl(
            "__eq__",
            fn1_met(set_t.clone(), set_t.clone(), Bool).quantify(),
            Const,
            Public,
        );
        set_.register_trait(set_t.clone(), set_eq);
        set_.register_marker_trait(mono("Mutizable"));
        set_.register_marker_trait(poly("Seq", vec![ty_tp(T.clone())]));
        let mut set_show = Self::builtin_methods(Some(mono("Show")), 1);
        set_show.register_builtin_impl(
            "to_str",
            fn0_met(set_t.clone(), Str).quantify(),
            Immutable,
            Public,
        );
        set_.register_trait(set_t.clone(), set_show);
        let g_dict_t = mono("GenericDict");
        let mut generic_dict = Self::builtin_mono_class("GenericDict", 2);
        generic_dict.register_superclass(Obj, &obj);
        let mut generic_dict_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        generic_dict_eq.register_builtin_impl(
            "__eq__",
            fn1_met(g_dict_t.clone(), g_dict_t.clone(), Bool).quantify(),
            Const,
            Public,
        );
        generic_dict.register_trait(g_dict_t.clone(), generic_dict_eq);
        let D = mono_q_tp("D", instanceof(mono("GenericDict")));
        // .get: _: T -> T or None
        let dict_get_t = fn1_met(g_dict_t.clone(), T.clone(), or(T.clone(), NoneType)).quantify();
        generic_dict.register_builtin_impl("get", dict_get_t, Immutable, Public);
        let dict_t = poly("Dict", vec![D.clone()]);
        let mut dict_ =
            // TODO: D <: GenericDict
            Self::builtin_poly_class("Dict", vec![PS::named_nd("D", mono("GenericDict"))], 10);
        dict_.register_superclass(g_dict_t.clone(), &generic_dict);
        dict_.register_marker_trait(poly("Output", vec![D.clone()]));
        // __getitem__: _: T -> D[T]
        let dict_getitem_t = fn1_met(
            dict_t.clone(),
            T.clone(),
            proj_call(D, "__getitem__", vec![ty_tp(T.clone())]),
        )
        .quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            "__getitem__",
            __dict_getitem__,
            dict_getitem_t,
            None,
        )));
        dict_.register_builtin_const("__getitem__", Public, get_item);
        /* Bytes */
        let mut bytes = Self::builtin_mono_class("Bytes", 2);
        bytes.register_superclass(Obj, &obj);
        let decode_t = pr_met(
            mono("Bytes"),
            vec![],
            None,
            vec![kw("encoding", Str), kw("errors", Str)],
            Str,
        );
        bytes.register_builtin_impl("decode", decode_t, Immutable, Public);
        /* GenericTuple */
        let mut generic_tuple = Self::builtin_mono_class("GenericTuple", 1);
        generic_tuple.register_superclass(Obj, &obj);
        let mut tuple_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        tuple_eq.register_builtin_impl(
            "__eq__",
            fn1_met(mono("GenericTuple"), mono("GenericTuple"), Bool),
            Const,
            Public,
        );
        generic_tuple.register_trait(mono("GenericTuple"), tuple_eq);
        let Ts = mono_q_tp("Ts", instanceof(array_t(Type, N.clone())));
        // Ts <: GenericArray
        let _tuple_t = poly("Tuple", vec![Ts.clone()]);
        let mut tuple_ = Self::builtin_poly_class(
            "Tuple",
            vec![PS::named_nd("Ts", array_t(Type, N.clone()))],
            2,
        );
        tuple_.register_superclass(mono("GenericTuple"), &generic_tuple);
        tuple_.register_marker_trait(poly("Output", vec![Ts.clone()]));
        // __Tuple_getitem__: (self: Tuple(Ts), _: {N}) -> Ts[N]
        let return_t = proj_call(Ts, "__getitem__", vec![N.clone()]);
        let tuple_getitem_t =
            fn1_met(_tuple_t.clone(), tp_enum(Nat, set! {N}), return_t).quantify();
        tuple_.register_builtin_py_impl(
            "__Tuple_getitem__",
            tuple_getitem_t.clone(),
            Const,
            Public,
            Some("__getitem__"),
        );
        // `__Tuple_getitem__` and `__getitem__` are the same thing
        // but `x.0` => `x__Tuple_getitem__(0)` determines that `x` is a tuple, which is better for type inference.
        tuple_.register_builtin_py_impl(
            "__getitem__",
            tuple_getitem_t,
            Const,
            Public,
            Some("__getitem__"),
        );
        /* record */
        let mut record = Self::builtin_mono_class("Record", 2);
        record.register_superclass(Obj, &obj);
        /* Or (true or type) */
        let or_t = poly("Or", vec![ty_tp(L), ty_tp(R)]);
        let mut or = Self::builtin_poly_class("Or", vec![PS::t_nd("L"), PS::t_nd("R")], 2);
        or.register_superclass(Obj, &obj);
        /* Iterators */
        let mut str_iterator = Self::builtin_mono_class("StrIterator", 1);
        str_iterator.register_superclass(Obj, &obj);
        str_iterator.register_marker_trait(poly("Iterable", vec![ty_tp(Str)]));
        str_iterator.register_marker_trait(poly("Output", vec![ty_tp(Str)]));
        let mut array_iterator = Self::builtin_poly_class("ArrayIterator", vec![PS::t_nd("T")], 1);
        array_iterator.register_superclass(Obj, &obj);
        array_iterator.register_marker_trait(poly("Iterable", vec![ty_tp(T.clone())]));
        array_iterator.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        let mut range_iterator = Self::builtin_poly_class("RangeIterator", vec![PS::t_nd("T")], 1);
        range_iterator.register_superclass(Obj, &obj);
        range_iterator.register_marker_trait(poly("Iterable", vec![ty_tp(T.clone())]));
        range_iterator.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        /* Enumerate */
        let mut enumerate = Self::builtin_poly_class("Enumerate", vec![PS::t_nd("T")], 2);
        enumerate.register_superclass(Obj, &obj);
        enumerate.register_marker_trait(poly("Iterable", vec![ty_tp(T.clone())]));
        enumerate.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        /* Filter */
        let mut filter = Self::builtin_poly_class("Filter", vec![PS::t_nd("T")], 2);
        filter.register_superclass(Obj, &obj);
        filter.register_marker_trait(poly("Iterable", vec![ty_tp(T.clone())]));
        filter.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        /* Map */
        let mut map = Self::builtin_poly_class("Map", vec![PS::t_nd("T")], 2);
        map.register_superclass(Obj, &obj);
        map.register_marker_trait(poly("Iterable", vec![ty_tp(T.clone())]));
        map.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        /* Revered */
        let mut reversed = Self::builtin_poly_class("Reversed", vec![PS::t_nd("T")], 2);
        reversed.register_superclass(Obj, &obj);
        reversed.register_marker_trait(poly("Iterable", vec![ty_tp(T.clone())]));
        reversed.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        /* Zip */
        let mut zip = Self::builtin_poly_class("Zip", vec![PS::t_nd("T"), PS::t_nd("U")], 2);
        zip.register_superclass(Obj, &obj);
        zip.register_marker_trait(poly(
            "Iterable",
            vec![ty_tp(tuple_t(vec![T.clone(), U.clone()]))],
        ));
        zip.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        zip.register_marker_trait(poly("Output", vec![ty_tp(U.clone())]));
        let mut obj_mut = Self::builtin_mono_class("Obj!", 2);
        obj_mut.register_superclass(Obj, &obj);
        let mut obj_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        obj_mut_mutable.register_builtin_const("ImmutType", Public, ValueObj::builtin_t(Obj));
        let f_t = kw("func", func(vec![kw("old", Int)], None, vec![], Int));
        let t = pr_met(
            ref_mut(mono("Obj!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        obj_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        obj_mut.register_trait(mono("Obj!"), obj_mut_mutable);
        /* Float! */
        let mut float_mut = Self::builtin_mono_class("Float!", 2);
        float_mut.register_superclass(Float, &float);
        let mut float_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        float_mut_mutable.register_builtin_const("ImmutType", Public, ValueObj::builtin_t(Float));
        let f_t = kw("func", func(vec![kw("old", Float)], None, vec![], Float));
        let t = pr_met(
            ref_mut(mono("Float!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        float_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        float_mut.register_trait(mono("Float!"), float_mut_mutable);
        /* Ratio! */
        let mut ratio_mut = Self::builtin_mono_class("Ratio!", 2);
        ratio_mut.register_superclass(Ratio, &ratio);
        let mut ratio_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        ratio_mut_mutable.register_builtin_const("ImmutType", Public, ValueObj::builtin_t(Ratio));
        let f_t = kw("func", func(vec![kw("old", Ratio)], None, vec![], Ratio));
        let t = pr_met(
            ref_mut(mono("Ratio!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        ratio_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        ratio_mut.register_trait(mono("Ratio!"), ratio_mut_mutable);
        /* Int! */
        let mut int_mut = Self::builtin_mono_class("Int!", 2);
        int_mut.register_superclass(Int, &int);
        int_mut.register_superclass(mono("Float!"), &float_mut);
        let t = pr_met(mono("Int!"), vec![], None, vec![kw("i", Int)], NoneType);
        int_mut.register_builtin_py_impl("inc!", t.clone(), Immutable, Public, Some("inc"));
        int_mut.register_builtin_py_impl("dec!", t, Immutable, Public, Some("dec"));
        let mut int_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        int_mut_mutable.register_builtin_const("ImmutType", Public, ValueObj::builtin_t(Int));
        let f_t = kw("func", func(vec![kw("old", Int)], None, vec![], Int));
        let t = pr_met(
            ref_mut(mono("Int!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        int_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        int_mut.register_trait(mono("Int!"), int_mut_mutable);
        let mut nat_mut = Self::builtin_mono_class("Nat!", 2);
        nat_mut.register_superclass(Nat, &nat);
        nat_mut.register_superclass(mono("Int!"), &int_mut);
        /* Nat! */
        let mut nat_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        nat_mut_mutable.register_builtin_const("ImmutType", Public, ValueObj::builtin_t(Nat));
        let f_t = kw("func", func(vec![kw("old", Nat)], None, vec![], Nat));
        let t = pr_met(
            ref_mut(mono("Nat!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        nat_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        nat_mut.register_trait(mono("Nat!"), nat_mut_mutable);
        /* Bool! */
        let mut bool_mut = Self::builtin_mono_class("Bool!", 2);
        bool_mut.register_superclass(Bool, &bool_);
        bool_mut.register_superclass(mono("Nat!"), &nat_mut);
        let mut bool_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        bool_mut_mutable.register_builtin_const("ImmutType", Public, ValueObj::builtin_t(Bool));
        let f_t = kw("func", func(vec![kw("old", Bool)], None, vec![], Bool));
        let t = pr_met(
            ref_mut(mono("Bool!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        bool_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        bool_mut.register_trait(mono("Bool!"), bool_mut_mutable);
        /* Str! */
        let mut str_mut = Self::builtin_mono_class("Str!", 2);
        str_mut.register_superclass(Str, &nonetype);
        let mut str_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        str_mut_mutable.register_builtin_const("ImmutType", Public, ValueObj::builtin_t(Str));
        let f_t = kw("func", func(vec![kw("old", Str)], None, vec![], Str));
        let t = pr_met(
            ref_mut(mono("Str!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        str_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        str_mut.register_trait(mono("Str!"), str_mut_mutable);
        /* File! */
        let mut file_mut = Self::builtin_mono_class("File!", 2);
        let mut file_mut_readable = Self::builtin_methods(Some(mono("Readable!")), 1);
        file_mut_readable.register_builtin_py_impl(
            "read!",
            pr_met(
                ref_mut(mono("File!"), None),
                vec![],
                None,
                vec![kw("n", Int)],
                Str,
            ),
            Immutable,
            Public,
            Some("read"),
        );
        file_mut.register_trait(mono("File!"), file_mut_readable);
        let mut file_mut_writable = Self::builtin_methods(Some(mono("Writable!")), 1);
        file_mut_writable.register_builtin_py_impl(
            "write!",
            pr1_kw_met(ref_mut(mono("File!"), None), kw("s", Str), Nat),
            Immutable,
            Public,
            Some("write"),
        );
        file_mut.register_trait(mono("File!"), file_mut_writable);
        file_mut.register_marker_trait(mono("FileLike"));
        file_mut.register_marker_trait(mono("FileLike!"));
        /* Array! */
        let N_MUT = mono_q_tp("N", instanceof(mono("Nat!")));
        let array_mut_t = poly("Array!", vec![ty_tp(T.clone()), N_MUT.clone()]);
        let mut array_mut_ = Self::builtin_poly_class(
            "Array!",
            vec![PS::t_nd("T"), PS::named_nd("N", mono("Nat!"))],
            2,
        );
        array_mut_.register_superclass(arr_t.clone(), &array_);
        let t = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    "Array!",
                    vec![ty_tp(T.clone()), N_MUT.clone() + value(1usize)],
                )),
            ),
            vec![kw("elem", T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_builtin_py_impl("push!", t, Immutable, Public, Some("append"));
        let t_extend = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    "Array!",
                    vec![ty_tp(T.clone()), TyParam::erased(mono("Nat!"))],
                )),
            ),
            vec![kw("iterable", poly("Iterable", vec![ty_tp(T.clone())]))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_builtin_py_impl("extend!", t_extend, Immutable, Public, Some("extend"));
        let t_insert = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    "Array!",
                    vec![ty_tp(T.clone()), N_MUT.clone() + value(1usize)],
                )),
            ),
            vec![kw("index", Nat), kw("elem", T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_builtin_py_impl("insert!", t_insert, Immutable, Public, Some("insert"));
        let t_remove = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    "Array!",
                    vec![ty_tp(T.clone()), N_MUT.clone() - value(1usize)],
                )),
            ),
            vec![kw("x", T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_builtin_py_impl("remove!", t_remove, Immutable, Public, Some("remove"));
        let t_pop = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    "Array!",
                    vec![ty_tp(T.clone()), N_MUT.clone() - value(1usize)],
                )),
            ),
            vec![],
            None,
            vec![kw("index", Nat)],
            T.clone(),
        )
        .quantify();
        array_mut_.register_builtin_py_impl("pop!", t_pop, Immutable, Public, Some("pop"));
        let t_clear = pr0_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly("Array!", vec![ty_tp(T.clone()), value(0usize)])),
            ),
            NoneType,
        )
        .quantify();
        array_mut_.register_builtin_py_impl("clear!", t_clear, Immutable, Public, Some("clear"));
        let t_sort = pr_met(
            ref_mut(array_mut_t.clone(), None),
            vec![],
            None,
            vec![kw(
                "key",
                func(vec![kw("x", T.clone())], None, vec![], mono("Ord")),
            )],
            NoneType,
        )
        .quantify();
        array_mut_.register_builtin_py_impl("sort!", t_sort, Immutable, Public, Some("sort"));
        let t_reverse = pr0_met(ref_mut(array_mut_t.clone(), None), NoneType).quantify();
        array_mut_.register_builtin_py_impl(
            "reverse!",
            t_reverse,
            Immutable,
            Public,
            Some("reverse"),
        );
        let t = pr_met(
            array_mut_t.clone(),
            vec![kw("func", nd_func(vec![anon(T.clone())], None, T.clone()))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_builtin_impl("strict_map!", t, Immutable, Public);
        let f_t = kw(
            "func",
            func(vec![kw("old", arr_t.clone())], None, vec![], arr_t.clone()),
        );
        let t = pr_met(
            ref_mut(array_mut_t.clone(), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        let mut array_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        array_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        array_mut_.register_trait(array_mut_t.clone(), array_mut_mutable);
        /* Set! */
        let set_mut_t = poly("Set!", vec![ty_tp(T.clone()), N_MUT]);
        let mut set_mut_ = Self::builtin_poly_class(
            "Set!",
            vec![PS::t_nd("T"), PS::named_nd("N", mono("Nat!"))],
            2,
        );
        set_mut_.register_superclass(set_t.clone(), &set_);
        // `add!` will erase N
        let t = pr_met(
            ref_mut(
                set_mut_t.clone(),
                Some(poly(
                    "Set!",
                    vec![ty_tp(T.clone()), TyParam::erased(mono("Nat!"))],
                )),
            ),
            vec![kw("elem", T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        set_mut_.register_builtin_py_impl("add!", t, Immutable, Public, Some("add"));
        let t = pr_met(
            set_mut_t.clone(),
            vec![kw("func", nd_func(vec![anon(T.clone())], None, T.clone()))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        set_mut_.register_builtin_impl("strict_map!", t, Immutable, Public);
        let f_t = kw(
            "func",
            func(vec![kw("old", set_t.clone())], None, vec![], set_t.clone()),
        );
        let t = pr_met(
            ref_mut(set_mut_t.clone(), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        let mut set_mut_mutable = Self::builtin_methods(Some(mono("Mutable")), 2);
        set_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        set_mut_.register_trait(set_mut_t.clone(), set_mut_mutable);
        /* Range */
        let range_t = poly("Range", vec![TyParam::t(T.clone())]);
        let mut range = Self::builtin_poly_class("Range", vec![PS::t_nd("T")], 2);
        // range.register_superclass(Obj, &obj);
        range.register_superclass(Type, &type_);
        range.register_marker_trait(poly("Output", vec![ty_tp(T.clone())]));
        range.register_marker_trait(poly("Seq", vec![ty_tp(T.clone())]));
        let mut range_eq = Self::builtin_methods(Some(mono("Eq")), 2);
        range_eq.register_builtin_impl(
            "__eq__",
            fn1_met(range_t.clone(), range_t.clone(), Bool).quantify(),
            Const,
            Public,
        );
        range.register_trait(range_t.clone(), range_eq);
        let mut range_iterable =
            Self::builtin_methods(Some(poly("Iterable", vec![ty_tp(T.clone())])), 2);
        let range_iter = poly("RangeIterator", vec![ty_tp(T.clone())]);
        range_iterable.register_builtin_py_impl(
            "iter",
            fn0_met(range_t.clone(), range_iter.clone()).quantify(),
            Immutable,
            Public,
            Some("__iter__"),
        );
        range_iterable.register_builtin_const("Iterator", vis, ValueObj::builtin_t(range_iter));
        range.register_trait(range_t.clone(), range_iterable);
        let range_getitem_t = fn1_kw_met(range_t.clone(), anon(T.clone()), T.clone()).quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            "__getitem__",
            __range_getitem__,
            range_getitem_t,
            None,
        )));
        range.register_builtin_const("__getitem__", Public, get_item);
        let mut g_callable = Self::builtin_mono_class("GenericCallable", 2);
        g_callable.register_superclass(Obj, &obj);
        let t_return = fn1_met(mono("GenericCallable"), Obj, Never).quantify();
        g_callable.register_builtin_impl("return", t_return, Immutable, Public);
        let mut g_generator = Self::builtin_mono_class("GenericGenerator", 2);
        g_generator.register_superclass(mono("GenericCallable"), &g_callable);
        let t_yield = fn1_met(mono("GenericGenerator"), Obj, Never).quantify();
        g_generator.register_builtin_impl("yield", t_yield, Immutable, Public);
        /* Proc */
        let mut proc = Self::builtin_mono_class("Proc", 2);
        proc.register_superclass(mono("GenericCallable"), &g_callable);
        let mut named_proc = Self::builtin_mono_class("NamedProc", 2);
        named_proc.register_superclass(mono("Proc"), &proc);
        named_proc.register_marker_trait(mono("Named"));
        /* Func */
        let mut func = Self::builtin_mono_class("Func", 2);
        func.register_superclass(mono("Proc"), &proc);
        let mut named_func = Self::builtin_mono_class("NamedFunc", 2);
        named_func.register_superclass(mono("Func"), &func);
        named_func.register_marker_trait(mono("Named"));
        let mut quant = Self::builtin_mono_class("Quantified", 2);
        quant.register_superclass(mono("Proc"), &proc);
        let mut qfunc = Self::builtin_mono_class("QuantifiedFunc", 2);
        qfunc.register_superclass(mono("Func"), &func);
        self.register_builtin_type(Never, never, vis, Const, Some("Never"));
        self.register_builtin_type(Obj, obj, vis, Const, Some("object"));
        // self.register_type(mono("Record"), vec![], record, Private, Const);
        self.register_builtin_type(Int, int, vis, Const, Some("int"));
        self.register_builtin_type(Nat, nat, vis, Const, Some("Nat"));
        self.register_builtin_type(Float, float, vis, Const, Some("float"));
        self.register_builtin_type(Ratio, ratio, vis, Const, Some("Ratio"));
        let name = if cfg!(feature = "py_compatible") {
            "bool"
        } else {
            "Bool"
        };
        self.register_builtin_type(Bool, bool_, vis, Const, Some(name));
        let name = if cfg!(feature = "py_compatible") {
            "str"
        } else {
            "Str"
        };
        self.register_builtin_type(Str, str_, vis, Const, Some(name));
        self.register_builtin_type(NoneType, nonetype, vis, Const, Some("NoneType"));
        self.register_builtin_type(Type, type_, vis, Const, Some("type"));
        self.register_builtin_type(ClassType, class_type, vis, Const, Some("ClassType"));
        self.register_builtin_type(TraitType, trait_type, vis, Const, Some("TraitType"));
        self.register_builtin_type(Code, code, vis, Const, Some("CodeType"));
        self.register_builtin_type(
            g_module_t,
            generic_module,
            Private,
            Const,
            Some("ModuleType"),
        );
        self.register_builtin_type(py_module_t, py_module, vis, Const, Some("ModuleType"));
        self.register_builtin_type(arr_t, array_, vis, Const, Some("list"));
        self.register_builtin_type(set_t, set_, vis, Const, Some("set"));
        self.register_builtin_type(g_dict_t, generic_dict, vis, Const, Some("dict"));
        self.register_builtin_type(dict_t, dict_, vis, Const, Some("dict"));
        self.register_builtin_type(mono("Bytes"), bytes, vis, Const, Some("bytes"));
        self.register_builtin_type(
            mono("GenericTuple"),
            generic_tuple,
            Private,
            Const,
            Some("tuple"),
        );
        self.register_builtin_type(_tuple_t, tuple_, vis, Const, Some("tuple"));
        self.register_builtin_type(mono("Record"), record, vis, Const, Some("Record"));
        self.register_builtin_type(or_t, or, vis, Const, Some("Union"));
        self.register_builtin_type(
            mono("StrIterator"),
            str_iterator,
            Private,
            Const,
            Some("str_iterator"),
        );
        self.register_builtin_type(
            poly("ArrayIterator", vec![ty_tp(T.clone())]),
            array_iterator,
            Private,
            Const,
            Some("array_iterator"),
        );
        self.register_builtin_type(
            poly("RangeIterator", vec![ty_tp(T.clone())]),
            range_iterator,
            Private,
            Const,
            Some("RangeIterator"),
        );
        self.register_builtin_type(
            poly("Enumerate", vec![ty_tp(T.clone())]),
            enumerate,
            Private,
            Const,
            Some("enumerate"),
        );
        self.register_builtin_type(
            poly("Filter", vec![ty_tp(T.clone())]),
            filter,
            Private,
            Const,
            Some("filter"),
        );
        self.register_builtin_type(
            poly("Map", vec![ty_tp(T.clone())]),
            map,
            Private,
            Const,
            Some("map"),
        );
        self.register_builtin_type(
            poly("Revered", vec![ty_tp(T.clone())]),
            reversed,
            Private,
            Const,
            Some("reversed"),
        );
        self.register_builtin_type(
            poly("Zip", vec![ty_tp(T), ty_tp(U)]),
            zip,
            Private,
            Const,
            Some("zip"),
        );
        self.register_builtin_type(mono("File!"), file_mut, vis, Const, Some("File"));
        self.register_builtin_type(array_mut_t, array_mut_, vis, Const, Some("list"));
        self.register_builtin_type(set_mut_t, set_mut_, vis, Const, Some("set"));
        self.register_builtin_type(
            mono("GenericCallable"),
            g_callable,
            vis,
            Const,
            Some("Callable"),
        );
        self.register_builtin_type(
            mono("GenericGenerator"),
            g_generator,
            vis,
            Const,
            Some("Generator"),
        );
        self.register_builtin_type(mono("Proc"), proc, vis, Const, Some("Proc"));
        self.register_builtin_type(mono("Func"), func, vis, Const, Some("Func"));
        self.register_builtin_type(range_t, range, vis, Const, Some("range"));
        if !cfg!(feature = "py_compatible") {
            self.register_builtin_type(module_t, module, vis, Const, Some("ModuleType"));
            self.register_builtin_type(mono("Obj!"), obj_mut, vis, Const, Some("object"));
            self.register_builtin_type(mono("Int!"), int_mut, vis, Const, Some("int"));
            self.register_builtin_type(mono("Nat!"), nat_mut, vis, Const, Some("Nat"));
            self.register_builtin_type(mono("Float!"), float_mut, vis, Const, Some("float"));
            self.register_builtin_type(mono("Ratio!"), ratio_mut, vis, Const, Some("Ratio"));
            self.register_builtin_type(mono("Bool!"), bool_mut, vis, Const, Some("Bool"));
            self.register_builtin_type(mono("Str!"), str_mut, vis, Const, Some("Str"));
            self.register_builtin_type(
                mono("NamedProc"),
                named_proc,
                Private,
                Const,
                Some("NamedProc"),
            );
            self.register_builtin_type(
                mono("NamedFunc"),
                named_func,
                Private,
                Const,
                Some("NamedFunc"),
            );
            self.register_builtin_type(
                mono("Quantified"),
                quant,
                Private,
                Const,
                Some("Quantified"),
            );
            self.register_builtin_type(
                mono("QuantifiedFunc"),
                qfunc,
                Private,
                Const,
                Some("QuantifiedFunc"),
            );
        }
    }

    fn init_builtin_funcs(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let T = mono_q("T", instanceof(Type));
        let U = mono_q("U", instanceof(Type));
        let Path = mono_q_tp("Path", instanceof(Str));
        let t_abs = nd_func(vec![kw("n", mono(NUM))], None, Nat);
        let t_all = func(
            vec![kw("iterable", poly("Iterable", vec![ty_tp(Bool)]))],
            None,
            vec![],
            Bool,
        );
        let t_any = func(
            vec![kw("iterable", poly("Iterable", vec![ty_tp(Bool)]))],
            None,
            vec![],
            Bool,
        );
        let t_ascii = nd_func(vec![kw("object", Obj)], None, Str);
        let t_assert = func(
            vec![kw("condition", Bool)],
            None,
            vec![kw("err_message", Str)],
            NoneType,
        );
        let t_bin = nd_func(vec![kw("n", Int)], None, Str);
        let t_bytes = nd_func(
            vec![kw("str", Str), kw("encoding", Str)],
            None,
            mono("Bytes"),
        );
        let t_chr = nd_func(
            vec![kw("i", Type::from(value(0usize)..=value(1_114_111usize)))],
            None,
            Str,
        );
        let t_classof = nd_func(vec![kw("old", Obj)], None, ClassType);
        let t_compile = nd_func(vec![kw("src", Str)], None, Code);
        let t_cond = nd_func(
            vec![
                kw("condition", Bool),
                kw("then", T.clone()),
                kw("else", T.clone()),
            ],
            None,
            T.clone(),
        )
        .quantify();
        let t_discard = nd_func(vec![kw("obj", Obj)], None, NoneType);
        let t_enumerate = func(
            vec![kw("iterable", poly("Iterable", vec![ty_tp(T.clone())]))],
            None,
            vec![kw("start", Int)],
            poly("Enumerate", vec![ty_tp(T.clone())]),
        )
        .quantify();
        let t_if = func(
            vec![
                kw("cond", Bool),
                kw("then", nd_func(vec![], None, T.clone())),
            ],
            None,
            vec![kw_default(
                "else",
                nd_func(vec![], None, U.clone()),
                nd_func(vec![], None, NoneType),
            )],
            or(T.clone(), U.clone()),
        )
        .quantify();
        let t_int = nd_func(vec![kw("obj", Obj)], None, or(Int, NoneType));
        let t_import = nd_func(
            vec![anon(tp_enum(Str, set! {Path.clone()}))],
            None,
            module(Path.clone()),
        )
        .quantify();
        let t_isinstance = nd_func(
            vec![
                kw("object", Obj),
                kw("classinfo", ClassType), // TODO: => ClassInfo
            ],
            None,
            Bool,
        );
        let t_issubclass = nd_func(
            vec![
                kw("subclass", ClassType),
                kw("classinfo", ClassType), // TODO: => ClassInfo
            ],
            None,
            Bool,
        );
        let I = mono_q("I", subtypeof(poly("Iterable", vec![ty_tp(T.clone())])));
        let t_iter = nd_func(vec![kw("object", I.clone())], None, proj(I, "Iterator")).quantify();
        let t_len = nd_func(
            vec![kw("s", poly("Seq", vec![TyParam::erased(Type)]))],
            None,
            Nat,
        );
        let t_log = func(
            vec![],
            Some(kw("objects", ref_(Obj))),
            vec![
                kw("sep", Str),
                kw("end", Str),
                kw("file", mono("Write")),
                kw("flush", Bool),
            ],
            NoneType,
        );
        let t_map = nd_func(
            vec![
                kw("proc!", nd_proc(vec![anon(T.clone())], None, T.clone())),
                kw("iterable", poly("Iterable", vec![ty_tp(T.clone())])),
            ],
            None,
            poly("Map", vec![ty_tp(T.clone())]),
        )
        .quantify();
        let O = mono_q("O", subtypeof(mono("Ord")));
        // TODO: iterable should be non-empty
        let t_max = nd_func(
            vec![kw("iterable", poly("Iterable", vec![ty_tp(O.clone())]))],
            None,
            O.clone(),
        )
        .quantify();
        let t_min = nd_func(
            vec![kw("iterable", poly("Iterable", vec![ty_tp(O.clone())]))],
            None,
            O,
        )
        .quantify();
        let t_nat = nd_func(vec![kw("obj", Obj)], None, or(Nat, NoneType));
        // e.g. not(b: Bool!): Bool!
        let B = mono_q("B", subtypeof(Bool));
        let t_not = nd_func(vec![kw("b", B.clone())], None, B).quantify();
        let t_oct = nd_func(vec![kw("x", Int)], None, Str);
        let t_ord = nd_func(vec![kw("c", Str)], None, Nat);
        let t_panic = nd_func(vec![kw("err_message", Str)], None, Never);
        let M = mono_q("M", Constraint::Uninited);
        let M = mono_q("M", subtypeof(poly("Mul", vec![ty_tp(M)])));
        // TODO: mod
        let t_pow = nd_func(
            vec![kw("base", M.clone()), kw("exp", M.clone())],
            None,
            proj(M, "Output"),
        )
        .quantify();
        let t_pyimport = nd_func(
            vec![anon(tp_enum(Str, set! {Path.clone()}))],
            None,
            py_module(Path),
        )
        .quantify();
        let t_pycompile = nd_func(
            vec![kw("src", Str), kw("filename", Str), kw("mode", Str)],
            None,
            Code,
        );
        let t_quit = func(vec![], None, vec![kw("code", Int)], Never);
        let t_exit = t_quit.clone();
        let t_repr = nd_func(vec![kw("object", Obj)], None, Str);
        let t_reversed = nd_func(
            vec![kw("seq", poly("Seq", vec![ty_tp(T.clone())]))],
            None,
            poly("Reversed", vec![ty_tp(T.clone())]),
        )
        .quantify();
        let t_round = nd_func(vec![kw("number", Float)], None, Int);
        let t_sorted = nd_func(
            vec![kw("iterable", poly("Iterable", vec![ty_tp(T.clone())]))],
            None,
            array_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let t_str = nd_func(vec![kw("object", Obj)], None, Str);
        let A = mono_q("A", Constraint::Uninited);
        let A = mono_q("A", subtypeof(poly("Add", vec![ty_tp(A)])));
        let t_sum = func(
            vec![kw("iterable", poly("Iterable", vec![ty_tp(A.clone())]))],
            None,
            vec![kw_default("start", or(A.clone(), Int), Int)],
            A,
        )
        .quantify();
        let t_unreachable = nd_func(vec![], None, Never);
        let t_zip = nd_func(
            vec![
                kw("iterable1", poly("Iterable", vec![ty_tp(T.clone())])),
                kw("iterable2", poly("Iterable", vec![ty_tp(U.clone())])),
            ],
            None,
            poly("Zip", vec![ty_tp(T.clone()), ty_tp(U.clone())]),
        )
        .quantify();
        self.register_builtin_py_impl("abs", t_abs, Immutable, vis, Some("abs"));
        self.register_builtin_py_impl("all", t_all, Immutable, vis, Some("all"));
        self.register_builtin_py_impl("any", t_any, Immutable, vis, Some("any"));
        self.register_builtin_py_impl("ascii", t_ascii, Immutable, vis, Some("ascii"));
        // Leave as `Const`, as it may negatively affect assert casting.
        self.register_builtin_impl("assert", t_assert, Const, vis);
        self.register_builtin_py_impl("bin", t_bin, Immutable, vis, Some("bin"));
        self.register_builtin_py_impl("bytes", t_bytes, Immutable, vis, Some("bytes"));
        self.register_builtin_py_impl("chr", t_chr, Immutable, vis, Some("chr"));
        self.register_builtin_py_impl("classof", t_classof, Immutable, vis, Some("type"));
        self.register_builtin_py_impl("compile", t_compile, Immutable, vis, Some("compile"));
        self.register_builtin_impl("cond", t_cond, Immutable, vis);
        self.register_builtin_py_impl("enumerate", t_enumerate, Immutable, vis, Some("enumerate"));
        self.register_builtin_py_impl("exit", t_exit, Immutable, vis, Some("exit"));
        self.register_builtin_py_impl(
            "isinstance",
            t_isinstance,
            Immutable,
            vis,
            Some("isinstance"),
        );
        self.register_builtin_py_impl(
            "issubclass",
            t_issubclass,
            Immutable,
            vis,
            Some("issubclass"),
        );
        self.register_builtin_py_impl("iter", t_iter, Immutable, vis, Some("iter"));
        self.register_builtin_py_impl("len", t_len, Immutable, vis, Some("len"));
        self.register_builtin_py_impl("map", t_map, Immutable, vis, Some("map"));
        self.register_builtin_py_impl("max", t_max, Immutable, vis, Some("max"));
        self.register_builtin_py_impl("min", t_min, Immutable, vis, Some("min"));
        self.register_builtin_py_impl("not", t_not, Immutable, vis, None); // `not` is not a function in Python
        self.register_builtin_py_impl("oct", t_oct, Immutable, vis, Some("oct"));
        self.register_builtin_py_impl("ord", t_ord, Immutable, vis, Some("ord"));
        self.register_builtin_py_impl("pow", t_pow, Immutable, vis, Some("pow"));
        self.register_builtin_py_impl(
            "pyimport",
            t_pyimport.clone(),
            Immutable,
            vis,
            Some("__import__"),
        );
        self.register_builtin_py_impl("quit", t_quit, Immutable, vis, Some("quit"));
        self.register_builtin_py_impl("repr", t_repr, Immutable, vis, Some("repr"));
        self.register_builtin_py_impl("reversed", t_reversed, Immutable, vis, Some("reversed"));
        self.register_builtin_py_impl("round", t_round, Immutable, vis, Some("round"));
        self.register_builtin_py_impl("sorted", t_sorted, Immutable, vis, Some("sorted"));
        self.register_builtin_py_impl("str", t_str, Immutable, vis, Some("str"));
        self.register_builtin_py_impl("sum", t_sum, Immutable, vis, Some("sum"));
        self.register_builtin_py_impl("zip", t_zip, Immutable, vis, Some("zip"));
        let name = if cfg!(feature = "py_compatible") {
            "int"
        } else {
            "int__"
        };
        self.register_builtin_py_impl("int", t_int, Immutable, vis, Some(name));
        if !cfg!(feature = "py_compatible") {
            self.register_builtin_py_impl("if", t_if, Immutable, vis, Some("if__"));
            self.register_builtin_py_impl("discard", t_discard, Immutable, vis, Some("discard__"));
            self.register_builtin_py_impl("import", t_import, Immutable, vis, Some("__import__"));
            self.register_builtin_py_impl("log", t_log, Immutable, vis, Some("print"));
            self.register_builtin_py_impl("nat", t_nat, Immutable, vis, Some("nat__"));
            self.register_builtin_py_impl("panic", t_panic, Immutable, vis, Some("quit"));
            if cfg!(feature = "debug") {
                self.register_builtin_py_impl("py", t_pyimport, Immutable, vis, Some("__import__"));
            }
            self.register_builtin_py_impl(
                "pycompile",
                t_pycompile,
                Immutable,
                vis,
                Some("compile"),
            );
            // TODO: original implementation
            self.register_builtin_py_impl(
                "unreachable",
                t_unreachable,
                Immutable,
                vis,
                Some("exit"),
            );
        } else {
            let t_range = func(
                vec![kw("stop", or(Int, NoneType))],
                None,
                vec![
                    kw("start", or(Int, NoneType)),
                    kw("step", or(Int, NoneType)),
                ],
                poly("Range", vec![ty_tp(Int)]),
            );
            self.register_builtin_py_impl("range", t_range, Immutable, vis, Some("range"));
            let t_list = func(
                vec![],
                None,
                vec![kw("iterable", poly("Iterable", vec![ty_tp(T.clone())]))],
                poly("Array", vec![ty_tp(T.clone()), TyParam::erased(Nat)]),
            );
            self.register_builtin_py_impl("list", t_list, Immutable, vis, Some("list"));
            let t_dict = func(
                vec![],
                None,
                vec![kw(
                    "iterable",
                    poly("Iterable", vec![ty_tp(tuple_t(vec![T.clone(), U.clone()]))]),
                )],
                dict! { T => U }.into(),
            );
            self.register_builtin_py_impl("dict", t_dict, Immutable, vis, Some("dict"));
        }
    }

    fn init_builtin_const_funcs(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let class_t = func(
            vec![],
            None,
            vec![kw("Requirement", or(Type, Ellipsis)), kw("Impl", Type)],
            ClassType,
        );
        let class = ConstSubr::Builtin(BuiltinConstSubr::new("Class", class_func, class_t, None));
        self.register_builtin_const("Class", vis, ValueObj::Subr(class));
        let inherit_t = func(
            vec![kw("Super", ClassType)],
            None,
            vec![kw("Impl", Type), kw("Additional", Type)],
            ClassType,
        );
        let inherit = ConstSubr::Builtin(BuiltinConstSubr::new(
            "Inherit",
            inherit_func,
            inherit_t,
            None,
        ));
        self.register_builtin_const("Inherit", vis, ValueObj::Subr(inherit));
        let trait_t = func(
            vec![kw("Requirement", Type)],
            None,
            vec![kw("Impl", Type)],
            TraitType,
        );
        let trait_ = ConstSubr::Builtin(BuiltinConstSubr::new("Trait", trait_func, trait_t, None));
        self.register_builtin_const("Trait", vis, ValueObj::Subr(trait_));
        let subsume_t = func(
            vec![kw("Super", TraitType)],
            None,
            vec![kw("Impl", Type), kw("Additional", Type)],
            TraitType,
        );
        let subsume = ConstSubr::Builtin(BuiltinConstSubr::new(
            "Subsume",
            subsume_func,
            subsume_t,
            None,
        ));
        self.register_builtin_const("Subsume", vis, ValueObj::Subr(subsume));
        // decorators
        let inheritable_t = func1(ClassType, ClassType);
        let inheritable = ConstSubr::Builtin(BuiltinConstSubr::new(
            "Inheritable",
            inheritable_func,
            inheritable_t,
            None,
        ));
        self.register_builtin_const("Inheritable", vis, ValueObj::Subr(inheritable));
        // TODO: register Del function object
        let t_del = nd_func(vec![kw("obj", Obj)], None, NoneType);
        self.register_builtin_impl("Del", t_del, Immutable, vis);
        let patch_t = func(
            vec![kw("Requirement", Type)],
            None,
            vec![kw("Impl", Type)],
            TraitType,
        );
        let patch = ConstSubr::Builtin(BuiltinConstSubr::new("Patch", patch_func, patch_t, None));
        self.register_builtin_const("Patch", vis, ValueObj::Subr(patch));
    }

    fn init_builtin_procs(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let T = mono_q("T", instanceof(Type));
        let U = mono_q("U", instanceof(Type));
        let t_dir = proc(
            vec![kw("obj", ref_(Obj))],
            None,
            vec![],
            array_t(Str, TyParam::erased(Nat)),
        );
        let t_print = proc(
            vec![],
            Some(kw("objects", ref_(Obj))),
            vec![
                kw("sep", Str),
                kw("end", Str),
                kw("file", mono("Write")),
                kw("flush", Bool),
            ],
            NoneType,
        );
        let t_id = nd_func(vec![kw("old", Obj)], None, Nat);
        let t_input = proc(vec![], None, vec![kw("msg", Str)], Str);
        let t_if = proc(
            vec![
                kw("cond", Bool),
                kw("then", nd_proc(vec![], None, T.clone())),
            ],
            None,
            vec![kw_default(
                "else",
                nd_proc(vec![], None, U.clone()),
                nd_proc(vec![], None, NoneType),
            )],
            or(T.clone(), U.clone()),
        )
        .quantify();
        let t_for = nd_proc(
            vec![
                kw("iterable", poly("Iterable", vec![ty_tp(T.clone())])),
                kw("proc!", nd_proc(vec![anon(T.clone())], None, NoneType)),
            ],
            None,
            NoneType,
        )
        .quantify();
        let t_globals = proc(vec![], None, vec![], dict! { Str => Obj }.into());
        let t_locals = proc(vec![], None, vec![], dict! { Str => Obj }.into());
        let t_next = nd_proc(
            vec![kw(
                "iterable",
                ref_mut(poly("Iterable", vec![ty_tp(T.clone())]), None),
            )],
            None,
            T.clone(),
        )
        .quantify();
        let t_cond = if cfg!(feature = "py_compatible") {
            Bool
        } else {
            // not Bool! type because `cond` may be the result of evaluation of a mutable object's method returns Bool.
            nd_proc(vec![], None, Bool)
        };
        let t_while = nd_proc(
            vec![
                kw("cond!", t_cond),
                kw("proc!", nd_proc(vec![], None, NoneType)),
            ],
            None,
            NoneType,
        );
        let P = mono_q("P", subtypeof(mono("PathLike")));
        let t_open = proc(
            vec![kw("file", P)],
            None,
            vec![
                kw("mode", Str),
                kw("buffering", Int),
                kw("encoding", or(Str, NoneType)),
                kw("errors", or(Str, NoneType)),
                kw("newline", or(Str, NoneType)),
                kw("closefd", Bool),
                // param_t("opener", option),
            ],
            mono("File!"),
        )
        .quantify();
        // TODO: T <: With
        let t_with = nd_proc(
            vec![
                kw("obj", T.clone()),
                kw("proc!", nd_proc(vec![anon(T)], None, U.clone())),
            ],
            None,
            U,
        )
        .quantify();
        self.register_builtin_py_impl("dir!", t_dir, Immutable, vis, Some("dir"));
        self.register_builtin_py_impl("print!", t_print, Immutable, vis, Some("print"));
        self.register_builtin_py_impl("id!", t_id, Immutable, vis, Some("id"));
        self.register_builtin_py_impl("input!", t_input, Immutable, vis, Some("input"));
        self.register_builtin_py_impl("globals!", t_globals, Immutable, vis, Some("globals"));
        self.register_builtin_py_impl("locals!", t_locals, Immutable, vis, Some("locals"));
        self.register_builtin_py_impl("next!", t_next, Immutable, vis, Some("next"));
        self.register_builtin_py_impl("open!", t_open, Immutable, vis, Some("open"));
        let name = if cfg!(feature = "py_compatible") {
            "if"
        } else {
            "if__"
        };
        self.register_builtin_py_impl("if!", t_if, Immutable, vis, Some(name));
        let name = if cfg!(feature = "py_compatible") {
            "for"
        } else {
            "for__"
        };
        self.register_builtin_py_impl("for!", t_for, Immutable, vis, Some(name));
        let name = if cfg!(feature = "py_compatible") {
            "while"
        } else {
            "while__"
        };
        self.register_builtin_py_impl("while!", t_while, Immutable, vis, Some(name));
        let name = if cfg!(feature = "py_compatible") {
            "with"
        } else {
            "with__"
        };
        self.register_builtin_py_impl("with!", t_with, Immutable, vis, Some(name));
    }

    fn init_builtin_operators(&mut self) {
        /* binary */
        let R = mono_q("R", instanceof(Type));
        let params = vec![ty_tp(R.clone())];
        let L = mono_q("L", subtypeof(poly("Add", params.clone())));
        let op_t = nd_func(
            vec![kw("lhs", L.clone()), kw("rhs", R.clone())],
            None,
            proj(L, "Output"),
        )
        .quantify();
        self.register_builtin_impl("__add__", op_t, Const, Private);
        let L = mono_q("L", subtypeof(poly("Sub", params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, "Output")).quantify();
        self.register_builtin_impl("__sub__", op_t, Const, Private);
        let L = mono_q("L", subtypeof(poly("Mul", params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, "Output")).quantify();
        self.register_builtin_impl("__mul__", op_t, Const, Private);
        let L = mono_q("L", subtypeof(poly("Div", params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, "Output")).quantify();
        self.register_builtin_impl("__div__", op_t, Const, Private);
        let L = mono_q("L", subtypeof(poly("FloorDiv", params)));
        let op_t = bin_op(L.clone(), R, proj(L, "Output")).quantify();
        self.register_builtin_impl("__floordiv__", op_t, Const, Private);
        let P = mono_q("P", Constraint::Uninited);
        let P = mono_q("P", subtypeof(poly("Mul", vec![ty_tp(P)])));
        let op_t = bin_op(P.clone(), P.clone(), proj(P, "PowOutput")).quantify();
        // TODO: add bound: M == M.Output
        self.register_builtin_impl("__pow__", op_t, Const, Private);
        let M = mono_q("M", Constraint::Uninited);
        let M = mono_q("M", subtypeof(poly("Div", vec![ty_tp(M)])));
        let op_t = bin_op(M.clone(), M.clone(), proj(M, "ModOutput")).quantify();
        self.register_builtin_impl("__mod__", op_t, Const, Private);
        let op_t = nd_proc(vec![kw("lhs", Obj), kw("rhs", Obj)], None, Bool);
        self.register_builtin_impl("__is__!", op_t.clone(), Const, Private);
        self.register_builtin_impl("__isnot__!", op_t, Const, Private);
        let E = mono_q("E", subtypeof(mono("Eq")));
        let op_t = bin_op(E.clone(), E, Bool).quantify();
        self.register_builtin_impl("__eq__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__ne__", op_t, Const, Private);
        let O = mono_q("O", subtypeof(mono("Ord")));
        let op_t = bin_op(O.clone(), O.clone(), Bool).quantify();
        self.register_builtin_impl("__lt__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__le__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__gt__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__ge__", op_t, Const, Private);
        let BT = mono_q("BT", subtypeof(or(Bool, Type)));
        let op_t = bin_op(BT.clone(), BT.clone(), BT).quantify();
        self.register_builtin_impl("__and__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__or__", op_t, Const, Private);
        let op_t = bin_op(O.clone(), O.clone(), range(O)).quantify();
        self.register_builtin_decl("__rng__", op_t.clone(), Private);
        self.register_builtin_decl("__lorng__", op_t.clone(), Private);
        self.register_builtin_decl("__rorng__", op_t.clone(), Private);
        self.register_builtin_decl("__orng__", op_t, Private);
        // TODO: use existential type: |T: Type| (T, In(T)) -> Bool
        let T = mono_q("T", instanceof(Type));
        let I = mono_q("I", subtypeof(poly("In", vec![ty_tp(T.clone())])));
        let op_t = bin_op(I, T, Bool).quantify();
        self.register_builtin_impl("__in__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__notin__", op_t, Const, Private);
        /* unary */
        // TODO: +/- Bool would like to be warned
        let M = mono_q("M", subtypeof(mono("Mutizable")));
        let op_t = func1(M.clone(), proj(M, "MutType!")).quantify();
        self.register_builtin_impl("__mutate__", op_t, Const, Private);
        let N = mono_q("N", subtypeof(mono(NUM)));
        let op_t = func1(N.clone(), N).quantify();
        self.register_builtin_decl("__pos__", op_t.clone(), Private);
        self.register_builtin_decl("__neg__", op_t, Private);
    }

    fn init_builtin_patches(&mut self) {
        let m = mono_q_tp("M", instanceof(Int));
        let n = mono_q_tp("N", instanceof(Int));
        let o = mono_q_tp("O", instanceof(Int));
        let p = mono_q_tp("P", instanceof(Int));
        let params = vec![
            PS::named_nd("M", Int),
            PS::named_nd("N", Int),
            PS::named_nd("O", Int),
            PS::named_nd("P", Int),
        ];
        let class = Type::from(&m..=&n);
        let impls = poly("Add", vec![TyParam::from(&o..=&p)]);
        // Interval is a bounding patch connecting M..N and (Add(O..P, M+O..N..P), Sub(O..P, M-P..N-O))
        let mut interval =
            Self::builtin_poly_glue_patch("Interval", class.clone(), impls.clone(), params, 2);
        let op_t = fn1_met(
            class.clone(),
            Type::from(&o..=&p),
            Type::from(m.clone() + o.clone()..=n.clone() + p.clone()),
        )
        .quantify();
        let mut interval_add = Self::builtin_methods(Some(impls), 2);
        interval_add.register_builtin_impl("__add__", op_t, Const, Public);
        interval_add.register_builtin_const(
            "Output",
            Public,
            ValueObj::builtin_t(Type::from(m.clone() + o.clone()..=n.clone() + p.clone())),
        );
        interval.register_trait(class.clone(), interval_add);
        let mut interval_sub =
            Self::builtin_methods(Some(poly("Sub", vec![TyParam::from(&o..=&p)])), 2);
        let op_t = fn1_met(
            class.clone(),
            Type::from(&o..=&p),
            Type::from(m.clone() - p.clone()..=n.clone() - o.clone()),
        )
        .quantify();
        interval_sub.register_builtin_impl("__sub__", op_t, Const, Public);
        interval_sub.register_builtin_const(
            "Output",
            Public,
            ValueObj::builtin_t(Type::from(m - p..=n - o)),
        );
        interval.register_trait(class, interval_sub);
        self.register_builtin_patch("Interval", interval, Private, Const);
        // eq.register_impl("__ne__", op_t,         Const, Public);
        // ord.register_impl("__le__", op_t.clone(), Const, Public);
        // ord.register_impl("__gt__", op_t.clone(), Const, Public);
        // ord.register_impl("__ge__", op_t,         Const, Public);
        let E = mono_q("E", subtypeof(mono("Eq")));
        let base = or(E, NoneType);
        let impls = mono("Eq");
        let params = vec![PS::named_nd("E", Type)];
        let mut option_eq =
            Self::builtin_poly_glue_patch("OptionEq", base.clone(), impls.clone(), params, 1);
        let mut option_eq_impl = Self::builtin_methods(Some(impls), 1);
        let op_t = fn1_met(base.clone(), base.clone(), Bool).quantify();
        option_eq_impl.register_builtin_impl("__eq__", op_t, Const, Public);
        option_eq.register_trait(base, option_eq_impl);
        self.register_builtin_patch("OptionEq", option_eq, Private, Const);
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
