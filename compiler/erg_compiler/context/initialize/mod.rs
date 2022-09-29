//! defines type information for builtin objects (in `Context`)
//!
//! 組み込みオブジェクトの型情報を(Contextに)定義
pub mod const_func;
pub mod py_mods;

use std::path::PathBuf;

use erg_common::config::ErgConfig;
use erg_common::error::Location;
use erg_common::vis::Visibility;
use erg_common::Str;
use erg_common::{set, unique_in_place};

use erg_type::typaram::TyParam;
use erg_type::value::ValueObj;
use erg_type::Type;
use erg_type::{constructors::*, BuiltinConstSubr, ConstSubr};
use ParamSpec as PS;
use Type::*;

use erg_parser::ast::VarName;

use crate::context::initialize::const_func::*;
use crate::context::instantiate::{ConstTemplate, TyVarContext};
use crate::context::{ClassDefType, Context, ContextKind, DefaultInfo, ParamSpec, TraitInstance};
use crate::mod_cache::SharedModuleCache;
use crate::varinfo::{Mutability, VarInfo, VarKind};
use DefaultInfo::*;
use Mutability::*;
use VarKind::*;
use Visibility::*;

impl Context {
    fn register_builtin_decl(&mut self, name: &'static str, t: Type, vis: Visibility) {
        let name = VarName::from_static(name);
        if self.decls.get(&name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.decls
                .insert(name, VarInfo::new(t, Immutable, vis, Builtin, None));
        }
    }

    fn register_builtin_impl(
        &mut self,
        name: &'static str,
        t: Type,
        muty: Mutability,
        vis: Visibility,
    ) {
        let name = VarName::from_static(name);
        if self.locals.get(&name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.locals
                .insert(name, VarInfo::new(t, muty, vis, Builtin, None));
        }
    }

    fn register_builtin_const(&mut self, name: &str, obj: ValueObj) {
        if self.rec_get_const_obj(name).is_some() {
            panic!("already registered: {name}");
        } else {
            // TODO: not all value objects are comparable
            let vi = VarInfo::new(v_enum(set! {obj.clone()}), Const, Private, Builtin, None);
            self.consts.insert(VarName::from_str(Str::rc(name)), obj);
            self.locals.insert(VarName::from_str(Str::rc(name)), vi);
        }
    }

    fn register_const_param_defaults(&mut self, name: &'static str, params: Vec<ConstTemplate>) {
        if self.const_param_defaults.get(name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.const_param_defaults.insert(Str::ever(name), params);
        }
    }

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

    fn register_builtin_type(&mut self, t: Type, ctx: Self, muty: Mutability) {
        if t.typarams_len().is_none() {
            self.register_mono_type(t, ctx, muty);
        } else {
            self.register_poly_type(t, ctx, muty);
        }
    }

    fn register_mono_type(&mut self, t: Type, ctx: Self, muty: Mutability) {
        // FIXME: recursive search
        if self.mono_types.contains_key(&t.name()) {
            panic!("{} has already been registered", t.name());
        } else if self.rec_get_const_obj(&t.name()).is_some() {
            panic!("{} has already been registered as const", t.name());
        } else {
            let name = VarName::from_str(t.name());
            self.locals.insert(
                name.clone(),
                VarInfo::new(Type, muty, Private, Builtin, None),
            );
            self.consts
                .insert(name.clone(), ValueObj::builtin_t(t.clone()));
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls.get_mut(&impl_trait.name()) {
                    impls.push(TraitInstance::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls.insert(
                        impl_trait.name(),
                        vec![TraitInstance::new(t.clone(), impl_trait.clone())],
                    );
                }
            }
            self.mono_types.insert(name, (t, ctx));
        }
    }

    // FIXME: MethodDefsと再代入は違う
    fn register_poly_type(&mut self, t: Type, ctx: Self, muty: Mutability) {
        let mut tv_ctx = TyVarContext::new(self.level, ctx.type_params_bounds(), self);
        let t = self
            .instantiate_t(t, &mut tv_ctx, Location::Unknown)
            .unwrap();
        // FIXME: panic
        if let Some((_, root_ctx)) = self.poly_types.get_mut(&t.name()) {
            root_ctx.methods_list.push((ClassDefType::Simple(t), ctx));
        } else {
            let name = VarName::from_str(t.name());
            self.locals.insert(
                name.clone(),
                VarInfo::new(Type, muty, Private, Builtin, None),
            );
            self.consts
                .insert(name.clone(), ValueObj::builtin_t(t.clone()));
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls.get_mut(&impl_trait.name()) {
                    impls.push(TraitInstance::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls.insert(
                        impl_trait.name(),
                        vec![TraitInstance::new(t.clone(), impl_trait.clone())],
                    );
                }
            }
            self.poly_types.insert(name, (t, ctx));
        }
    }

    fn register_builtin_patch(&mut self, name: &'static str, ctx: Self, muty: Mutability) {
        if self.patches.contains_key(name) {
            panic!("{} has already been registered", name);
        } else {
            let name = VarName::from_static(name);
            self.locals.insert(
                name.clone(),
                VarInfo::new(Type, muty, Private, Builtin, None),
            );
            for method_name in ctx.locals.keys() {
                if let Some(patches) = self.method_impl_patches.get_mut(method_name) {
                    patches.push(name.clone());
                } else {
                    self.method_impl_patches
                        .insert(method_name.clone(), vec![name.clone()]);
                }
            }
            self.patches.insert(name, ctx);
        }
    }

    /// see std/prelude.er
    /// All type boundaries are defined in each subroutine
    /// `push_subtype_bound`, etc. are used for type boundary determination in user-defined APIs
    // 型境界はすべて各サブルーチンで定義する
    // push_subtype_boundなどはユーザー定義APIの型境界決定のために使用する
    fn init_builtin_traits(&mut self) {
        let unpack = Self::builtin_mono_trait("Unpack", 2);
        let inheritable_type = Self::builtin_mono_trait("InheritableType", 2);
        let named = Self::builtin_mono_trait("Named", 2);
        let mut mutable = Self::builtin_mono_trait("Mutable", 2);
        let proj = mono_proj(mono_q("Self"), "ImmutType");
        let f_t = func(vec![param_t("old", proj.clone())], None, vec![], proj);
        let t = pr1_met(ref_mut(mono_q("Self"), None), f_t, NoneType);
        let t = quant(
            t,
            set! { subtypeof(mono_q("Self"), builtin_mono("Immutizable")) },
        );
        mutable.register_builtin_decl("update!", t, Public);
        // REVIEW: Immutatable?
        let mut immutizable = Self::builtin_mono_trait("Immutizable", 2);
        immutizable.register_superclass(builtin_mono("Mutable"), &mutable);
        immutizable.register_builtin_decl("ImmutType", Type, Public);
        // REVIEW: Mutatable?
        let mut mutizable = Self::builtin_mono_trait("Mutizable", 2);
        mutizable.register_builtin_decl("MutType!", Type, Public);
        let pathlike = Self::builtin_mono_trait("PathLike", 2);
        let mut in_ = Self::builtin_poly_trait("In", vec![PS::t("T", NonDefault)], 2);
        let params = vec![PS::t("T", NonDefault)];
        let input = Self::builtin_poly_trait("Input", params.clone(), 2);
        let output = Self::builtin_poly_trait("Output", params, 2);
        in_.register_superclass(builtin_poly("Input", vec![ty_tp(mono_q("T"))]), &input);
        let op_t = fn1_met(mono_q("T"), mono_q("I"), Bool);
        let op_t = quant(
            op_t,
            set! { static_instance("T", Type), subtypeof(mono_q("I"), builtin_poly("In", vec![ty_tp(mono_q("T"))])) },
        );
        in_.register_builtin_decl("__in__", op_t, Public);
        // Erg does not have a trait equivalent to `PartialEq` in Rust
        // This means, Erg's `Float` cannot be compared with other `Float`
        // use `l - r < EPSILON` to check if two floats are almost equal
        let mut eq = Self::builtin_poly_trait("Eq", vec![PS::t("R", WithDefault)], 2);
        eq.register_superclass(builtin_poly("Output", vec![ty_tp(mono_q("R"))]), &output);
        // __eq__: |Self <: Eq()| Self.(Self) -> Bool
        let op_t = fn1_met(mono_q("Self"), mono_q("R"), Bool);
        let op_t = quant(
            op_t,
            set! {
                subtypeof(mono_q("Self"), builtin_poly("Eq", vec![ty_tp(mono_q("R"))])),
                static_instance("R", Type)
            },
        );
        eq.register_builtin_decl("__eq__", op_t, Public);
        let mut partial_ord =
            Self::builtin_poly_trait("PartialOrd", vec![PS::t("R", WithDefault)], 2);
        partial_ord.register_superclass(builtin_poly("Eq", vec![ty_tp(mono_q("R"))]), &eq);
        let op_t = fn1_met(
            mono_q("Self"),
            mono_q("R"),
            option(builtin_mono("Ordering")),
        );
        let op_t = quant(
            op_t,
            set! {
                subtypeof(mono_q("Self"), builtin_poly("PartialOrd", vec![ty_tp(mono_q("R"))])),
                static_instance("R", Type)
            },
        );
        partial_ord.register_builtin_decl("__partial_cmp__", op_t, Public);
        let mut ord = Self::builtin_mono_trait("Ord", 2);
        ord.register_superclass(builtin_poly("Eq", vec![ty_tp(builtin_mono("Self"))]), &eq);
        ord.register_superclass(
            builtin_poly("PartialOrd", vec![ty_tp(builtin_mono("Self"))]),
            &partial_ord,
        );
        // FIXME: poly trait
        let num = Self::builtin_mono_trait("Num", 2);
        /* vec![
            poly("Add", vec![]),
            poly("Sub", vec![]),
            poly("Mul", vec![]),
        ], */
        let mut seq = Self::builtin_poly_trait("Seq", vec![PS::t("T", NonDefault)], 2);
        seq.register_superclass(builtin_poly("Output", vec![ty_tp(mono_q("T"))]), &output);
        let self_t = mono_q("Self");
        let t = fn0_met(self_t.clone(), Nat);
        let t = quant(
            t,
            set! {subtypeof(self_t.clone(), builtin_poly("Seq", vec![TyParam::erased(Type)]))},
        );
        seq.register_builtin_decl("len", t, Public);
        let t = fn1_met(self_t.clone(), Nat, mono_q("T"));
        let t = quant(
            t,
            set! {subtypeof(self_t, builtin_poly("Seq", vec![ty_tp(mono_q("T"))])), static_instance("T", Type)},
        );
        // Seq.get: |Self <: Seq(T)| Self.(Nat) -> T
        seq.register_builtin_decl("get", t, Public);
        let r = mono_q("R");
        let r_bound = static_instance("R", Type);
        let params = vec![PS::t("R", WithDefault)];
        let ty_params = vec![ty_tp(mono_q("R"))];
        let mut add = Self::builtin_poly_trait("Add", params.clone(), 2);
        // Rについて共変(__add__の型とは関係ない)
        add.register_superclass(builtin_poly("Output", vec![ty_tp(mono_q("R"))]), &output);
        let self_bound = subtypeof(mono_q("Self"), builtin_poly("Add", ty_params.clone()));
        let op_t = fn1_met(
            mono_q("Self"),
            r.clone(),
            mono_proj(mono_q("Self"), "Output"),
        );
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        add.register_builtin_decl("__add__", op_t, Public);
        add.register_builtin_decl("Output", Type, Public);
        let mut sub = Self::builtin_poly_trait("Sub", params.clone(), 2);
        sub.register_superclass(builtin_poly("Output", vec![ty_tp(mono_q("R"))]), &output);
        let op_t = fn1_met(
            mono_q("Self"),
            r.clone(),
            mono_proj(mono_q("Self"), "Output"),
        );
        let self_bound = subtypeof(mono_q("Self"), builtin_poly("Sub", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        sub.register_builtin_decl("__sub__", op_t, Public);
        sub.register_builtin_decl("Output", Type, Public);
        let mut mul = Self::builtin_poly_trait("Mul", params.clone(), 2);
        mul.register_superclass(builtin_poly("Output", vec![ty_tp(mono_q("R"))]), &output);
        let op_t = fn1_met(
            mono_q("Self"),
            r.clone(),
            mono_proj(mono_q("Self"), "Output"),
        );
        let self_bound = subtypeof(mono_q("Self"), builtin_poly("Mul", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        mul.register_builtin_decl("__mul__", op_t, Public);
        mul.register_builtin_decl("Output", Type, Public);
        let mut div = Self::builtin_poly_trait("Div", params, 2);
        div.register_superclass(builtin_poly("Output", vec![ty_tp(mono_q("R"))]), &output);
        let op_t = fn1_met(mono_q("Self"), r, mono_proj(mono_q("Self"), "Output"));
        let self_bound = subtypeof(mono_q("Self"), builtin_poly("Div", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound, self_bound});
        div.register_builtin_decl("__div__", op_t, Public);
        div.register_builtin_decl("Output", Type, Public);
        self.register_builtin_type(builtin_mono("Unpack"), unpack, Const);
        self.register_builtin_type(builtin_mono("InheritableType"), inheritable_type, Const);
        self.register_builtin_type(builtin_mono("Named"), named, Const);
        self.register_builtin_type(builtin_mono("Mutable"), mutable, Const);
        self.register_builtin_type(builtin_mono("Immutizable"), immutizable, Const);
        self.register_builtin_type(builtin_mono("Mutizable"), mutizable, Const);
        self.register_builtin_type(builtin_mono("PathLike"), pathlike, Const);
        self.register_builtin_type(
            builtin_poly("Input", vec![ty_tp(mono_q("T"))]),
            input,
            Const,
        );
        self.register_builtin_type(
            builtin_poly("Output", vec![ty_tp(mono_q("T"))]),
            output,
            Const,
        );
        self.register_builtin_type(builtin_poly("In", vec![ty_tp(mono_q("T"))]), in_, Const);
        self.register_builtin_type(builtin_poly("Eq", vec![ty_tp(mono_q("R"))]), eq, Const);
        self.register_builtin_type(
            builtin_poly("PartialOrd", vec![ty_tp(mono_q("R"))]),
            partial_ord,
            Const,
        );
        self.register_builtin_type(builtin_mono("Ord"), ord, Const);
        self.register_builtin_type(builtin_mono("Num"), num, Const);
        self.register_builtin_type(builtin_poly("Seq", vec![ty_tp(mono_q("T"))]), seq, Const);
        self.register_builtin_type(builtin_poly("Add", ty_params.clone()), add, Const);
        self.register_builtin_type(builtin_poly("Sub", ty_params.clone()), sub, Const);
        self.register_builtin_type(builtin_poly("Mul", ty_params.clone()), mul, Const);
        self.register_builtin_type(builtin_poly("Div", ty_params), div, Const);
        self.register_const_param_defaults(
            "Eq",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(mono_q("Self")))],
        );
        self.register_const_param_defaults(
            "PartialOrd",
            vec![ConstTemplate::app("Self", vec![], vec![])],
        );
        self.register_const_param_defaults(
            "Add",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(mono_q("Self")))],
        );
        self.register_const_param_defaults(
            "Sub",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(mono_q("Self")))],
        );
        self.register_const_param_defaults(
            "Mul",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(mono_q("Self")))],
        );
        self.register_const_param_defaults(
            "Div",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(mono_q("Self")))],
        );
    }

    fn init_builtin_classes(&mut self) {
        let mut obj = Self::builtin_mono_class("Obj", 2);
        let t = fn0_met(mono_q("Self"), mono_q("Self"));
        let t = quant(t, set! {subtypeof(mono_q("Self"), builtin_mono("Obj"))});
        obj.register_builtin_impl("clone", t, Const, Public);
        obj.register_builtin_impl("__module__", Str, Const, Public);
        obj.register_builtin_impl("__sizeof__", fn0_met(Obj, Nat), Const, Public);
        obj.register_builtin_impl("__repr__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_builtin_impl("__str__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_builtin_impl("__dict__", fn0_met(Obj, dict(Str, Obj)), Immutable, Public);
        obj.register_builtin_impl(
            "__bytes__",
            fn0_met(Obj, builtin_mono("Bytes")),
            Immutable,
            Public,
        );
        let mut obj_in = Self::builtin_methods("In", 2);
        obj_in.register_builtin_impl("__in__", fn1_met(Obj, Type, Bool), Const, Public);
        obj.register_trait(Obj, builtin_poly("Eq", vec![ty_tp(Type)]), obj_in);
        let mut obj_mutizable = Self::builtin_methods("Mutizable", 1);
        obj_mutizable.register_builtin_const("MutType!", ValueObj::builtin_t(builtin_mono("Obj!")));
        obj.register_trait(Obj, builtin_mono("Mutizable"), obj_mutizable);
        let mut float = Self::builtin_mono_class("Float", 2);
        float.register_superclass(Obj, &obj);
        // TODO: support multi platform
        float.register_builtin_const("EPSILON", ValueObj::Float(2.220446049250313e-16));
        float.register_builtin_impl("Real", Float, Const, Public);
        float.register_builtin_impl("Imag", Float, Const, Public);
        float.register_marker_trait(builtin_mono("Num"));
        float.register_marker_trait(builtin_mono("Ord"));
        let mut float_partial_ord = Self::builtin_methods("PartialOrd", 2);
        float_partial_ord.register_builtin_impl(
            "__cmp__",
            fn1_met(Float, Float, builtin_mono("Ordering")),
            Const,
            Public,
        );
        float.register_trait(
            Float,
            builtin_poly("PartialOrd", vec![ty_tp(Float)]),
            float_partial_ord,
        );
        // Float doesn't have an `Eq` implementation
        let op_t = fn1_met(Float, Float, Float);
        let mut float_add = Self::builtin_methods("Add", 2);
        float_add.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        float_add.register_builtin_const("Output", ValueObj::builtin_t(Float));
        float.register_trait(Float, builtin_poly("Add", vec![ty_tp(Float)]), float_add);
        let mut float_sub = Self::builtin_methods("Sub", 2);
        float_sub.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        float_sub.register_builtin_const("Output", ValueObj::builtin_t(Float));
        float.register_trait(Float, builtin_poly("Sub", vec![ty_tp(Float)]), float_sub);
        let mut float_mul = Self::builtin_methods("Mul", 2);
        float_mul.register_builtin_impl("__mul__", op_t.clone(), Const, Public);
        float_mul.register_builtin_const("Output", ValueObj::builtin_t(Float));
        float_mul.register_builtin_const("PowOutput", ValueObj::builtin_t(Float));
        float.register_trait(Float, builtin_poly("Mul", vec![ty_tp(Float)]), float_mul);
        let mut float_div = Self::builtin_methods("Div", 2);
        float_div.register_builtin_impl("__div__", op_t, Const, Public);
        float_div.register_builtin_const("Output", ValueObj::builtin_t(Float));
        float_div.register_builtin_const("ModOutput", ValueObj::builtin_t(Float));
        float.register_trait(Float, builtin_poly("Div", vec![ty_tp(Float)]), float_div);
        let mut float_mutizable = Self::builtin_methods("Mutizable", 2);
        float_mutizable
            .register_builtin_const("MutType!", ValueObj::builtin_t(builtin_mono("Float!")));
        float.register_trait(Float, builtin_mono("Mutizable"), float_mutizable);
        // TODO: Int, Nat, Boolの継承元をRatioにする(今はFloat)
        let mut ratio = Self::builtin_mono_class("Ratio", 2);
        ratio.register_superclass(Obj, &obj);
        ratio.register_builtin_impl("Real", Ratio, Const, Public);
        ratio.register_builtin_impl("Imag", Ratio, Const, Public);
        ratio.register_marker_trait(builtin_mono("Num"));
        ratio.register_marker_trait(builtin_mono("Ord"));
        let mut ratio_partial_ord = Self::builtin_methods("PartialOrd", 2);
        ratio_partial_ord.register_builtin_impl(
            "__cmp__",
            fn1_met(Ratio, Ratio, builtin_mono("Ordering")),
            Const,
            Public,
        );
        ratio.register_trait(
            Ratio,
            builtin_poly("PartialOrd", vec![ty_tp(Ratio)]),
            ratio_partial_ord,
        );
        let mut ratio_eq = Self::builtin_methods("Eq", 2);
        ratio_eq.register_builtin_impl("__eq__", fn1_met(Ratio, Ratio, Bool), Const, Public);
        ratio.register_trait(Ratio, builtin_poly("Eq", vec![ty_tp(Ratio)]), ratio_eq);
        let op_t = fn1_met(Ratio, Ratio, Ratio);
        let mut ratio_add = Self::builtin_methods("Add", 2);
        ratio_add.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        ratio_add.register_builtin_const("Output", ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, builtin_poly("Add", vec![ty_tp(Ratio)]), ratio_add);
        let mut ratio_sub = Self::builtin_methods("Sub", 2);
        ratio_sub.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        ratio_sub.register_builtin_const("Output", ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, builtin_poly("Sub", vec![ty_tp(Ratio)]), ratio_sub);
        let mut ratio_mul = Self::builtin_methods("Mul", 2);
        ratio_mul.register_builtin_impl("__mul__", op_t.clone(), Const, Public);
        ratio_mul.register_builtin_const("Output", ValueObj::builtin_t(Ratio));
        ratio_mul.register_builtin_const("PowOutput", ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, builtin_poly("Mul", vec![ty_tp(Ratio)]), ratio_mul);
        let mut ratio_div = Self::builtin_methods("Div", 2);
        ratio_div.register_builtin_impl("__div__", op_t, Const, Public);
        ratio_div.register_builtin_const("Output", ValueObj::builtin_t(Ratio));
        ratio_div.register_builtin_const("ModOutput", ValueObj::builtin_t(Ratio));
        ratio.register_trait(Ratio, builtin_poly("Div", vec![ty_tp(Ratio)]), ratio_div);
        let mut ratio_mutizable = Self::builtin_methods("Mutizable", 2);
        ratio_mutizable
            .register_builtin_const("MutType!", ValueObj::builtin_t(builtin_mono("Ratio!")));
        ratio.register_trait(Ratio, builtin_mono("Mutizable"), ratio_mutizable);
        let mut int = Self::builtin_mono_class("Int", 2);
        int.register_superclass(Float, &float); // TODO: Float -> Ratio
        int.register_superclass(Obj, &obj);
        int.register_marker_trait(builtin_mono("Num"));
        int.register_marker_trait(builtin_mono("Ord"));
        int.register_marker_trait(builtin_poly("Eq", vec![ty_tp(Int)]));
        // class("Rational"),
        // class("Integral"),
        int.register_builtin_impl("abs", fn0_met(Int, Nat), Immutable, Public);
        let mut int_partial_ord = Self::builtin_methods("PartialOrd", 2);
        int_partial_ord.register_builtin_impl(
            "__partial_cmp__",
            fn1_met(Int, Int, option(builtin_mono("Ordering"))),
            Const,
            Public,
        );
        int.register_trait(
            Int,
            builtin_poly("PartialOrd", vec![ty_tp(Int)]),
            int_partial_ord,
        );
        let mut int_eq = Self::builtin_methods("Eq", 2);
        int_eq.register_builtin_impl("__eq__", fn1_met(Int, Int, Bool), Const, Public);
        int.register_trait(Int, builtin_poly("Eq", vec![ty_tp(Int)]), int_eq);
        // __div__ is not included in Int (cast to Ratio)
        let op_t = fn1_met(Int, Int, Int);
        let mut int_add = Self::builtin_methods("Add", 2);
        int_add.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        int_add.register_builtin_const("Output", ValueObj::builtin_t(Int));
        int.register_trait(Int, builtin_poly("Add", vec![ty_tp(Int)]), int_add);
        let mut int_sub = Self::builtin_methods("Sub", 2);
        int_sub.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        int_sub.register_builtin_const("Output", ValueObj::builtin_t(Int));
        int.register_trait(Int, builtin_poly("Sub", vec![ty_tp(Int)]), int_sub);
        let mut int_mul = Self::builtin_methods("Mul", 2);
        int_mul.register_builtin_impl("__mul__", op_t, Const, Public);
        int_mul.register_builtin_const("Output", ValueObj::builtin_t(Int));
        int_mul.register_builtin_const("PowOutput", ValueObj::builtin_t(Nat));
        int.register_trait(Int, builtin_poly("Mul", vec![ty_tp(Int)]), int_mul);
        let mut int_mutizable = Self::builtin_methods("Mutizable", 2);
        int_mutizable.register_builtin_const("MutType!", ValueObj::builtin_t(builtin_mono("Int!")));
        int.register_trait(Int, builtin_mono("Mutizable"), int_mutizable);
        int.register_builtin_impl("Real", Int, Const, Public);
        int.register_builtin_impl("Imag", Int, Const, Public);
        let mut nat = Self::builtin_mono_class("Nat", 10);
        nat.register_superclass(Int, &int);
        nat.register_superclass(Float, &float); // TODO: Float -> Ratio
        nat.register_superclass(Obj, &obj);
        // class("Rational"),
        // class("Integral"),
        nat.register_builtin_impl(
            "times!",
            pr_met(
                Nat,
                vec![param_t("p", nd_proc(vec![], None, NoneType))],
                None,
                vec![],
                NoneType,
            ),
            Immutable,
            Public,
        );
        nat.register_marker_trait(builtin_mono("Num"));
        nat.register_marker_trait(builtin_mono("Ord"));
        let mut nat_eq = Self::builtin_methods("Eq", 2);
        nat_eq.register_builtin_impl("__eq__", fn1_met(Nat, Nat, Bool), Const, Public);
        nat.register_trait(Nat, builtin_poly("Eq", vec![ty_tp(Nat)]), nat_eq);
        let mut nat_partial_ord = Self::builtin_methods("PartialOrd", 2);
        nat_partial_ord.register_builtin_impl(
            "__cmp__",
            fn1_met(Nat, Nat, builtin_mono("Ordering")),
            Const,
            Public,
        );
        nat.register_trait(
            Nat,
            builtin_poly("PartialOrd", vec![ty_tp(Nat)]),
            nat_partial_ord,
        );
        // __sub__, __div__ is not included in Nat (cast to Int/ Ratio)
        let op_t = fn1_met(Nat, Nat, Nat);
        let mut nat_add = Self::builtin_methods("Add", 2);
        nat_add.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        nat_add.register_builtin_const("Output", ValueObj::builtin_t(Nat));
        nat.register_trait(Nat, builtin_poly("Add", vec![ty_tp(Nat)]), nat_add);
        let mut nat_mul = Self::builtin_methods("Mul", 2);
        nat_mul.register_builtin_impl("__mul__", op_t, Const, Public);
        nat_mul.register_builtin_const("Output", ValueObj::builtin_t(Nat));
        nat.register_trait(Nat, builtin_poly("Mul", vec![ty_tp(Nat)]), nat_mul);
        let mut nat_mutizable = Self::builtin_methods("Mutizable", 2);
        nat_mutizable.register_builtin_const("MutType!", ValueObj::builtin_t(builtin_mono("Nat!")));
        nat.register_trait(Nat, builtin_mono("Mutizable"), nat_mutizable);
        nat.register_builtin_impl("Real", Nat, Const, Public);
        nat.register_builtin_impl("Imag", Nat, Const, Public);
        let mut bool_ = Self::builtin_mono_class("Bool", 10);
        bool_.register_superclass(Nat, &nat);
        bool_.register_superclass(Int, &int);
        bool_.register_superclass(Float, &float); // TODO: Float -> Ratio
        bool_.register_superclass(Obj, &obj);
        // class("Rational"),
        // class("Integral"),
        // TODO: And, Or trait
        bool_.register_builtin_impl("__and__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_builtin_impl("__or__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_marker_trait(builtin_mono("Num"));
        bool_.register_marker_trait(builtin_mono("Ord"));
        let mut bool_partial_ord = Self::builtin_methods("PartialOrd", 2);
        bool_partial_ord.register_builtin_impl(
            "__cmp__",
            fn1_met(Bool, Bool, builtin_mono("Ordering")),
            Const,
            Public,
        );
        bool_.register_trait(
            Bool,
            builtin_poly("PartialOrd", vec![ty_tp(Bool)]),
            bool_partial_ord,
        );
        let mut bool_eq = Self::builtin_methods("Eq", 2);
        bool_eq.register_builtin_impl("__eq__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_trait(Bool, builtin_poly("Eq", vec![ty_tp(Bool)]), bool_eq);
        let mut bool_add = Self::builtin_methods("Add", 2);
        bool_add.register_builtin_impl("__add__", fn1_met(Bool, Bool, Int), Const, Public);
        bool_add.register_builtin_const("Output", ValueObj::builtin_t(Nat));
        bool_.register_trait(Bool, builtin_poly("Add", vec![ty_tp(Bool)]), bool_add);
        let mut bool_mutizable = Self::builtin_methods("Mutizable", 2);
        bool_mutizable
            .register_builtin_const("MutType!", ValueObj::builtin_t(builtin_mono("Bool!")));
        bool_.register_trait(Bool, builtin_mono("Mutizable"), bool_mutizable);
        let mut str_ = Self::builtin_mono_class("Str", 10);
        str_.register_superclass(Obj, &obj);
        str_.register_marker_trait(builtin_mono("Ord"));
        str_.register_marker_trait(builtin_mono("PathLike"));
        str_.register_builtin_impl(
            "replace",
            fn_met(
                Str,
                vec![param_t("pat", Str), param_t("into", Str)],
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
                vec![param_t("encoding", Str), param_t("errors", Str)],
                builtin_mono("Bytes"),
            ),
            Immutable,
            Public,
        );
        let mut str_eq = Self::builtin_methods("Eq", 2);
        str_eq.register_builtin_impl("__eq__", fn1_met(Str, Str, Bool), Const, Public);
        str_.register_trait(Str, builtin_poly("Eq", vec![ty_tp(Str)]), str_eq);
        let mut str_seq = Self::builtin_methods("Seq", 2);
        str_seq.register_builtin_impl("len", fn0_met(Str, Nat), Const, Public);
        str_seq.register_builtin_impl("get", fn1_met(Str, Nat, Str), Const, Public);
        str_.register_trait(Str, builtin_poly("Seq", vec![ty_tp(Str)]), str_seq);
        let mut str_add = Self::builtin_methods("Add", 2);
        str_add.register_builtin_impl("__add__", fn1_met(Str, Str, Str), Const, Public);
        str_add.register_builtin_const("Output", ValueObj::builtin_t(Str));
        str_.register_trait(Str, builtin_poly("Add", vec![ty_tp(Str)]), str_add);
        let mut str_mul = Self::builtin_methods("Mul", 2);
        str_mul.register_builtin_impl("__mul__", fn1_met(Str, Nat, Str), Const, Public);
        str_mul.register_builtin_const("Output", ValueObj::builtin_t(Str));
        str_.register_trait(Str, builtin_poly("Mul", vec![ty_tp(Nat)]), str_mul);
        let mut str_mutizable = Self::builtin_methods("Mutizable", 2);
        str_mutizable.register_builtin_const("MutType!", ValueObj::builtin_t(builtin_mono("Str!")));
        str_.register_trait(Str, builtin_mono("Mutizable"), str_mutizable);
        let mut type_ = Self::builtin_mono_class("Type", 2);
        type_.register_superclass(Obj, &obj);
        type_.register_builtin_impl("mro", array(Type, TyParam::erased(Nat)), Immutable, Public);
        type_.register_marker_trait(builtin_mono("Named"));
        let mut type_eq = Self::builtin_methods("Eq", 2);
        type_eq.register_builtin_impl("__eq__", fn1_met(Type, Type, Bool), Const, Public);
        type_.register_trait(Type, builtin_poly("Eq", vec![ty_tp(Type)]), type_eq);
        let mut class_type = Self::builtin_mono_class("ClassType", 2);
        class_type.register_superclass(Type, &type_);
        class_type.register_superclass(Obj, &obj);
        class_type.register_marker_trait(builtin_mono("Named"));
        let mut class_eq = Self::builtin_methods("Eq", 2);
        class_eq.register_builtin_impl("__eq__", fn1_met(Class, Class, Bool), Const, Public);
        class_type.register_trait(Class, builtin_poly("Eq", vec![ty_tp(Class)]), class_eq);
        let g_module_t = builtin_mono("GenericModule");
        let mut generic_module = Self::builtin_mono_class("GenericModule", 2);
        generic_module.register_superclass(Obj, &obj);
        generic_module.register_marker_trait(builtin_mono("Named"));
        let mut generic_module_eq = Self::builtin_methods("Eq", 2);
        generic_module_eq.register_builtin_impl(
            "__eq__",
            fn1_met(g_module_t.clone(), g_module_t.clone(), Bool),
            Const,
            Public,
        );
        generic_module.register_trait(
            g_module_t.clone(),
            builtin_poly("Eq", vec![ty_tp(g_module_t.clone())]),
            generic_module_eq,
        );
        let module_t = module(mono_q_tp("Path"));
        let mut module = Self::builtin_poly_class("Module", vec![PS::named_nd("Path", Str)], 2);
        module.register_superclass(g_module_t.clone(), &generic_module);
        let mut array_ =
            Self::builtin_poly_class("Array", vec![PS::t_nd("T"), PS::named_nd("N", Nat)], 10);
        array_.register_superclass(Obj, &obj);
        array_.register_marker_trait(builtin_poly("Output", vec![ty_tp(mono_q("T"))]));
        let n = mono_q_tp("N");
        let m = mono_q_tp("M");
        let array_t = array(mono_q("T"), n.clone());
        let t = fn_met(
            array_t.clone(),
            vec![param_t("rhs", array(mono_q("T"), m.clone()))],
            None,
            vec![],
            array(mono_q("T"), n + m),
        );
        let t = quant(
            t,
            set! {static_instance("N", Nat), static_instance("M", Nat)},
        );
        array_.register_builtin_impl("concat", t, Immutable, Public);
        let mut_type = ValueObj::builtin_t(builtin_poly(
            "Array!",
            vec![TyParam::t(mono_q("T")), TyParam::mono_q("N").mutate()],
        ));
        // [T; N].MutType! = [T; !N] (neither [T!; N] nor [T; N]!)
        array_.register_builtin_const("MutType!", mut_type);
        let mut array_eq = Self::builtin_methods("Eq", 2);
        array_eq.register_builtin_impl(
            "__eq__",
            fn1_met(array_t.clone(), array_t.clone(), Bool),
            Const,
            Public,
        );
        array_.register_trait(
            array_t.clone(),
            builtin_poly("Eq", vec![ty_tp(array_t)]),
            array_eq,
        );
        array_.register_marker_trait(builtin_mono("Mutizable"));
        array_.register_marker_trait(builtin_poly("Seq", vec![ty_tp(mono_q("T"))]));
        // TODO: make Tuple6, Tuple7, ... etc.
        let mut tuple_ = Self::builtin_mono_class("Tuple", 2);
        tuple_.register_superclass(Obj, &obj);
        let mut tuple_eq = Self::builtin_methods("Eq", 2);
        tuple_eq.register_builtin_impl(
            "__eq__",
            fn1_met(builtin_mono("Tuple"), builtin_mono("Tuple"), Bool),
            Const,
            Public,
        );
        tuple_.register_trait(
            builtin_mono("Tuple"),
            builtin_poly("Eq", vec![ty_tp(builtin_mono("Tuple"))]),
            tuple_eq,
        );
        let mut tuple1 = Self::builtin_poly_class("Tuple1", vec![PS::t_nd("A")], 2);
        tuple1.register_superclass(builtin_mono("Tuple"), &tuple_);
        tuple1.register_superclass(Obj, &obj);
        let mut tuple1_eq = Self::builtin_methods("Eq", 2);
        tuple1_eq.register_builtin_impl(
            "__eq__",
            fn1_met(
                builtin_poly("Tuple1", vec![ty_tp(mono_q("A"))]),
                builtin_poly("Tuple1", vec![ty_tp(mono_q("A"))]),
                Bool,
            ),
            Const,
            Public,
        );
        tuple1.register_trait(
            builtin_poly("Tuple1", vec![ty_tp(mono_q("A"))]),
            builtin_poly(
                "Eq",
                vec![ty_tp(builtin_poly("Tuple1", vec![ty_tp(mono_q("A"))]))],
            ),
            tuple1_eq,
        );
        let mut tuple2 = Self::builtin_poly_class("Tuple2", vec![PS::t_nd("A"), PS::t_nd("B")], 2);
        tuple2.register_superclass(builtin_mono("Tuple"), &tuple_);
        tuple2.register_superclass(Obj, &obj);
        let mut tuple2_eq = Self::builtin_methods("Eq", 2);
        tuple2_eq.register_builtin_impl(
            "__eq__",
            fn1_met(
                builtin_poly("Tuple2", vec![ty_tp(mono_q("A")), ty_tp(mono_q("B"))]),
                builtin_poly("Tuple2", vec![ty_tp(mono_q("A")), ty_tp(mono_q("B"))]),
                Bool,
            ),
            Const,
            Public,
        );
        tuple2.register_trait(
            builtin_poly("Tuple2", vec![ty_tp(mono_q("A")), ty_tp(mono_q("B"))]),
            builtin_poly(
                "Eq",
                vec![ty_tp(builtin_poly(
                    "Tuple2",
                    vec![ty_tp(mono_q("A")), ty_tp(mono_q("B"))],
                ))],
            ),
            tuple2_eq,
        );
        let mut tuple3 = Self::builtin_poly_class(
            "Tuple3",
            vec![PS::t_nd("A"), PS::t_nd("B"), PS::t_nd("C")],
            2,
        );
        tuple3.register_superclass(builtin_mono("Tuple"), &tuple_);
        tuple3.register_superclass(Obj, &obj);
        let mut tuple3_eq = Self::builtin_methods("Eq", 2);
        tuple3_eq.register_builtin_impl(
            "__eq__",
            fn1_met(
                builtin_poly(
                    "Tuple3",
                    vec![ty_tp(mono_q("A")), ty_tp(mono_q("B")), ty_tp(mono_q("C"))],
                ),
                builtin_poly(
                    "Tuple3",
                    vec![ty_tp(mono_q("A")), ty_tp(mono_q("B")), ty_tp(mono_q("C"))],
                ),
                Bool,
            ),
            Const,
            Public,
        );
        tuple3.register_trait(
            builtin_poly(
                "Tuple3",
                vec![ty_tp(mono_q("A")), ty_tp(mono_q("B")), ty_tp(mono_q("C"))],
            ),
            builtin_poly(
                "Eq",
                vec![ty_tp(builtin_poly(
                    "Tuple3",
                    vec![ty_tp(mono_q("A")), ty_tp(mono_q("B")), ty_tp(mono_q("C"))],
                ))],
            ),
            tuple3_eq,
        );
        let mut tuple4 = Self::builtin_poly_class(
            "Tuple4",
            vec![PS::t_nd("A"), PS::t_nd("B"), PS::t_nd("C"), PS::t_nd("D")],
            2,
        );
        tuple4.register_superclass(builtin_mono("Tuple"), &tuple_);
        tuple4.register_superclass(Obj, &obj);
        let mut tuple4_eq = Self::builtin_methods("Eq", 2);
        tuple4_eq.register_builtin_impl(
            "__eq__",
            fn1_met(
                builtin_poly(
                    "Tuple4",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                    ],
                ),
                builtin_poly(
                    "Tuple4",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                    ],
                ),
                Bool,
            ),
            Const,
            Public,
        );
        tuple4.register_trait(
            builtin_poly(
                "Tuple4",
                vec![
                    ty_tp(mono_q("A")),
                    ty_tp(mono_q("B")),
                    ty_tp(mono_q("C")),
                    ty_tp(mono_q("D")),
                ],
            ),
            builtin_poly(
                "Eq",
                vec![ty_tp(builtin_poly(
                    "Tuple4",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                    ],
                ))],
            ),
            tuple4_eq,
        );
        let mut tuple5 = Self::builtin_poly_class(
            "Tuple5",
            vec![
                PS::t_nd("A"),
                PS::t_nd("B"),
                PS::t_nd("C"),
                PS::t_nd("D"),
                PS::t_nd("E"),
            ],
            2,
        );
        tuple5.register_superclass(builtin_mono("Tuple"), &tuple_);
        tuple5.register_superclass(Obj, &obj);
        let mut tuple5_eq = Self::builtin_methods("Eq", 2);
        tuple5_eq.register_builtin_impl(
            "__eq__",
            fn1_met(
                builtin_poly(
                    "Tuple5",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                    ],
                ),
                builtin_poly(
                    "Tuple5",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                    ],
                ),
                Bool,
            ),
            Const,
            Public,
        );
        tuple5.register_trait(
            builtin_poly(
                "Tuple5",
                vec![
                    ty_tp(mono_q("A")),
                    ty_tp(mono_q("B")),
                    ty_tp(mono_q("C")),
                    ty_tp(mono_q("D")),
                    ty_tp(mono_q("E")),
                ],
            ),
            builtin_poly(
                "Eq",
                vec![ty_tp(builtin_poly(
                    "Tuple5",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                    ],
                ))],
            ),
            tuple5_eq,
        );
        let mut tuple6 = Self::builtin_poly_class(
            "Tuple6",
            vec![
                PS::t_nd("A"),
                PS::t_nd("B"),
                PS::t_nd("C"),
                PS::t_nd("D"),
                PS::t_nd("E"),
                PS::t_nd("F"),
            ],
            2,
        );
        tuple6.register_superclass(builtin_mono("Tuple"), &tuple_);
        tuple6.register_superclass(Obj, &obj);
        let mut tuple6_eq = Self::builtin_methods("Eq", 2);
        tuple6_eq.register_builtin_impl(
            "__eq__",
            fn1_met(
                builtin_poly(
                    "Tuple6",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                    ],
                ),
                builtin_poly(
                    "Tuple6",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                    ],
                ),
                Bool,
            ),
            Const,
            Public,
        );
        tuple6.register_trait(
            builtin_poly(
                "Tuple6",
                vec![
                    ty_tp(mono_q("A")),
                    ty_tp(mono_q("B")),
                    ty_tp(mono_q("C")),
                    ty_tp(mono_q("D")),
                    ty_tp(mono_q("E")),
                    ty_tp(mono_q("F")),
                ],
            ),
            builtin_poly(
                "Eq",
                vec![ty_tp(builtin_poly(
                    "Tuple6",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                    ],
                ))],
            ),
            tuple6_eq,
        );
        let mut tuple7 = Self::builtin_poly_class(
            "Tuple7",
            vec![
                PS::t_nd("A"),
                PS::t_nd("B"),
                PS::t_nd("C"),
                PS::t_nd("D"),
                PS::t_nd("E"),
                PS::t_nd("F"),
                PS::t_nd("G"),
            ],
            2,
        );
        tuple7.register_superclass(builtin_mono("Tuple"), &tuple_);
        tuple7.register_superclass(Obj, &obj);
        let mut tuple7_eq = Self::builtin_methods("Eq", 2);
        tuple7_eq.register_builtin_impl(
            "__eq__",
            fn1_met(
                builtin_poly(
                    "Tuple7",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                        ty_tp(mono_q("G")),
                    ],
                ),
                builtin_poly(
                    "Tuple7",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                        ty_tp(mono_q("G")),
                    ],
                ),
                Bool,
            ),
            Const,
            Public,
        );
        tuple7.register_trait(
            builtin_poly(
                "Tuple7",
                vec![
                    ty_tp(mono_q("A")),
                    ty_tp(mono_q("B")),
                    ty_tp(mono_q("C")),
                    ty_tp(mono_q("D")),
                    ty_tp(mono_q("E")),
                    ty_tp(mono_q("F")),
                    ty_tp(mono_q("G")),
                ],
            ),
            builtin_poly(
                "Eq",
                vec![ty_tp(builtin_poly(
                    "Tuple7",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                        ty_tp(mono_q("G")),
                    ],
                ))],
            ),
            tuple7_eq,
        );
        let mut tuple8 = Self::builtin_poly_class(
            "Tuple8",
            vec![
                PS::t_nd("A"),
                PS::t_nd("B"),
                PS::t_nd("C"),
                PS::t_nd("D"),
                PS::t_nd("E"),
                PS::t_nd("F"),
                PS::t_nd("G"),
                PS::t_nd("H"),
            ],
            2,
        );
        tuple8.register_superclass(builtin_mono("Tuple"), &tuple_);
        tuple8.register_superclass(Obj, &obj);
        let mut tuple8_eq = Self::builtin_methods("Eq", 2);
        tuple8_eq.register_builtin_impl(
            "__eq__",
            fn1_met(
                builtin_poly(
                    "Tuple8",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                        ty_tp(mono_q("G")),
                        ty_tp(mono_q("H")),
                    ],
                ),
                builtin_poly(
                    "Tuple8",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                        ty_tp(mono_q("G")),
                        ty_tp(mono_q("H")),
                    ],
                ),
                Bool,
            ),
            Const,
            Public,
        );
        tuple8.register_trait(
            builtin_poly(
                "Tuple8",
                vec![
                    ty_tp(mono_q("A")),
                    ty_tp(mono_q("B")),
                    ty_tp(mono_q("C")),
                    ty_tp(mono_q("D")),
                    ty_tp(mono_q("E")),
                    ty_tp(mono_q("F")),
                    ty_tp(mono_q("G")),
                    ty_tp(mono_q("H")),
                ],
            ),
            builtin_poly(
                "Eq",
                vec![ty_tp(builtin_poly(
                    "Tuple8",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                        ty_tp(mono_q("F")),
                        ty_tp(mono_q("G")),
                        ty_tp(mono_q("H")),
                    ],
                ))],
            ),
            tuple8_eq,
        );
        let mut record = Self::builtin_mono_class("Record", 2);
        record.register_superclass(Obj, &obj);
        let mut record_type = Self::builtin_mono_class("RecordType", 2);
        record_type.register_superclass(builtin_mono("Record"), &record);
        record_type.register_superclass(builtin_mono("Type"), &type_);
        record_type.register_superclass(Obj, &obj);
        let mut float_mut = Self::builtin_mono_class("Float!", 2);
        float_mut.register_superclass(Float, &float);
        float_mut.register_superclass(Obj, &obj);
        let mut float_mut_mutable = Self::builtin_methods("Mutable", 2);
        float_mut_mutable.register_builtin_const("ImmutType", ValueObj::builtin_t(Float));
        let f_t = param_t("f", func(vec![param_t("old", Float)], None, vec![], Float));
        let t = pr_met(
            ref_mut(builtin_mono("Float!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        float_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        float_mut.register_trait(
            builtin_mono("Float!"),
            builtin_mono("Mutable"),
            float_mut_mutable,
        );
        let mut ratio_mut = Self::builtin_mono_class("Ratio!", 2);
        ratio_mut.register_superclass(Ratio, &ratio);
        ratio_mut.register_superclass(Obj, &obj);
        let mut ratio_mut_mutable = Self::builtin_methods("Mutable", 2);
        ratio_mut_mutable.register_builtin_const("ImmutType", ValueObj::builtin_t(Ratio));
        let f_t = param_t(
            "f",
            func(
                vec![param_t("old", builtin_mono("Ratio"))],
                None,
                vec![],
                builtin_mono("Ratio"),
            ),
        );
        let t = pr_met(
            ref_mut(builtin_mono("Ratio!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        ratio_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        ratio_mut.register_trait(
            builtin_mono("Ratio!"),
            builtin_mono("Mutable"),
            ratio_mut_mutable,
        );
        let mut int_mut = Self::builtin_mono_class("Int!", 2);
        int_mut.register_superclass(Int, &int);
        int_mut.register_superclass(builtin_mono("Float!"), &float_mut);
        int_mut.register_superclass(Obj, &obj);
        let mut int_mut_mutable = Self::builtin_methods("Mutable", 2);
        int_mut_mutable.register_builtin_const("ImmutType", ValueObj::builtin_t(Int));
        let f_t = param_t("f", func(vec![param_t("old", Int)], None, vec![], Int));
        let t = pr_met(
            ref_mut(builtin_mono("Int!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        int_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        int_mut.register_trait(
            builtin_mono("Int!"),
            builtin_mono("Mutable"),
            int_mut_mutable,
        );
        let mut nat_mut = Self::builtin_mono_class("Nat!", 2);
        nat_mut.register_superclass(Nat, &nat);
        nat_mut.register_superclass(builtin_mono("Int!"), &int_mut);
        nat_mut.register_superclass(builtin_mono("Float!"), &float_mut);
        nat_mut.register_superclass(Obj, &obj);
        let mut nat_mut_mutable = Self::builtin_methods("Mutable", 2);
        nat_mut_mutable.register_builtin_const("ImmutType", ValueObj::builtin_t(Nat));
        let f_t = param_t("f", func(vec![param_t("old", Nat)], None, vec![], Nat));
        let t = pr_met(
            ref_mut(builtin_mono("Nat!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        nat_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        nat_mut.register_trait(
            builtin_mono("Nat!"),
            builtin_mono("Mutable"),
            nat_mut_mutable,
        );
        let mut bool_mut = Self::builtin_mono_class("Bool!", 2);
        bool_mut.register_superclass(Bool, &bool_);
        bool_mut.register_superclass(builtin_mono("Nat!"), &nat_mut);
        bool_mut.register_superclass(builtin_mono("Int!"), &int_mut);
        bool_mut.register_superclass(builtin_mono("Float!"), &float_mut);
        bool_mut.register_superclass(Obj, &obj);
        let mut bool_mut_mutable = Self::builtin_methods("Mutable", 2);
        bool_mut_mutable.register_builtin_const("ImmutType", ValueObj::builtin_t(Bool));
        let f_t = param_t("f", func(vec![param_t("old", Bool)], None, vec![], Bool));
        let t = pr_met(
            ref_mut(builtin_mono("Bool!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        bool_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        bool_mut.register_trait(
            builtin_mono("Bool!"),
            builtin_mono("Mutable"),
            bool_mut_mutable,
        );
        let mut str_mut = Self::builtin_mono_class("Str!", 2);
        str_mut.register_superclass(Str, &str_);
        str_mut.register_superclass(Obj, &obj);
        let mut str_mut_mutable = Self::builtin_methods("Mutable", 2);
        str_mut_mutable.register_builtin_const("ImmutType", ValueObj::builtin_t(Str));
        let f_t = param_t("f", func(vec![param_t("old", Str)], None, vec![], Str));
        let t = pr_met(
            ref_mut(builtin_mono("Str!"), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        str_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        str_mut.register_trait(
            builtin_mono("Str!"),
            builtin_mono("Mutable"),
            str_mut_mutable,
        );
        let file_mut = Self::builtin_mono_class("File!", 2);
        let array_t = builtin_poly("Array", vec![ty_tp(mono_q("T")), mono_q_tp("N")]);
        let array_mut_t = builtin_poly("Array!", vec![ty_tp(mono_q("T")), mono_q_tp("N")]);
        let mut array_mut_ = Self::builtin_poly_class(
            "Array!",
            vec![PS::t_nd("T"), PS::named_nd("N", builtin_mono("Nat!"))],
            2,
        );
        array_mut_.register_superclass(array_t.clone(), &array_);
        array_mut_.register_superclass(Obj, &obj);
        let t = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(builtin_poly(
                    "Array!",
                    vec![ty_tp(mono_q("T")), mono_q_tp("N") + value(1)],
                )),
            ),
            vec![param_t("elem", mono_q("T"))],
            None,
            vec![],
            NoneType,
        );
        let t = quant(
            t,
            set! {static_instance("T", Type), static_instance("N", builtin_mono("Nat!"))},
        );
        array_mut_.register_builtin_impl("push!", t, Immutable, Public);
        let t = pr_met(
            array_mut_t.clone(),
            vec![param_t(
                "f",
                nd_func(vec![anon(mono_q("T"))], None, mono_q("T")),
            )],
            None,
            vec![],
            NoneType,
        );
        let t = quant(
            t,
            set! {static_instance("T", Type), static_instance("N", builtin_mono("Nat!"))},
        );
        array_mut_.register_builtin_impl("strict_map!", t, Immutable, Public);
        let f_t = param_t(
            "f",
            func(
                vec![param_t("old", array_t.clone())],
                None,
                vec![],
                array_t.clone(),
            ),
        );
        let t = pr_met(
            ref_mut(array_mut_t.clone(), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        let mut array_mut_mutable = Self::builtin_methods("Mutable", 2);
        array_mut_mutable.register_builtin_impl("update!", t, Immutable, Public);
        array_mut_.register_trait(
            array_mut_t.clone(),
            builtin_mono("Mutable"),
            array_mut_mutable,
        );
        let range_t = builtin_poly("Range", vec![TyParam::t(mono_q("T"))]);
        let mut range = Self::builtin_poly_class("Range", vec![PS::t_nd("T")], 2);
        range.register_superclass(Obj, &obj);
        range.register_marker_trait(builtin_poly("Output", vec![ty_tp(mono_q("T"))]));
        let mut range_eq = Self::builtin_methods("Eq", 2);
        range_eq.register_builtin_impl(
            "__eq__",
            fn1_met(range_t.clone(), range_t.clone(), Bool),
            Const,
            Public,
        );
        range.register_trait(
            range_t.clone(),
            builtin_poly("Eq", vec![ty_tp(range_t.clone())]),
            range_eq,
        );
        let mut proc = Self::builtin_mono_class("Proc", 2);
        proc.register_superclass(Obj, &obj);
        // TODO: lambda
        proc.register_marker_trait(builtin_mono("Named"));
        let mut func = Self::builtin_mono_class("Func", 2);
        func.register_superclass(builtin_mono("Proc"), &proc);
        func.register_superclass(Obj, &obj);
        // TODO: lambda
        func.register_marker_trait(builtin_mono("Named"));
        let mut qfunc = Self::builtin_mono_class("QuantifiedFunc", 2);
        qfunc.register_superclass(builtin_mono("Func"), &func);
        qfunc.register_superclass(Obj, &obj);
        self.register_builtin_type(Obj, obj, Const);
        // self.register_type(mono("Record"), vec![], record, Const);
        self.register_builtin_type(Int, int, Const);
        self.register_builtin_type(Nat, nat, Const);
        self.register_builtin_type(Float, float, Const);
        self.register_builtin_type(Ratio, ratio, Const);
        self.register_builtin_type(Bool, bool_, Const);
        self.register_builtin_type(Str, str_, Const);
        self.register_builtin_type(Type, type_, Const);
        self.register_builtin_type(Class, class_type, Const);
        self.register_builtin_type(g_module_t, generic_module, Const);
        self.register_builtin_type(module_t, module, Const);
        self.register_builtin_type(array_t, array_, Const);
        self.register_builtin_type(tuple(vec![mono_q("A")]), tuple1, Const);
        self.register_builtin_type(tuple(vec![mono_q("A"), mono_q("B")]), tuple2, Const);
        self.register_builtin_type(
            tuple(vec![mono_q("A"), mono_q("B"), mono_q("C")]),
            tuple3,
            Const,
        );
        self.register_builtin_type(
            tuple(vec![mono_q("A"), mono_q("B"), mono_q("C"), mono_q("D")]),
            tuple4,
            Const,
        );
        self.register_builtin_type(
            tuple(vec![
                mono_q("A"),
                mono_q("B"),
                mono_q("C"),
                mono_q("D"),
                mono_q("E"),
            ]),
            tuple5,
            Const,
        );
        self.register_builtin_type(
            tuple(vec![
                mono_q("A"),
                mono_q("B"),
                mono_q("C"),
                mono_q("D"),
                mono_q("E"),
                mono_q("F"),
            ]),
            tuple6,
            Const,
        );
        self.register_builtin_type(
            tuple(vec![
                mono_q("A"),
                mono_q("B"),
                mono_q("C"),
                mono_q("D"),
                mono_q("E"),
                mono_q("F"),
                mono_q("G"),
            ]),
            tuple7,
            Const,
        );
        self.register_builtin_type(
            tuple(vec![
                mono_q("A"),
                mono_q("B"),
                mono_q("C"),
                mono_q("D"),
                mono_q("E"),
                mono_q("F"),
                mono_q("G"),
                mono_q("H"),
            ]),
            tuple8,
            Const,
        );
        self.register_builtin_type(builtin_mono("Record"), record, Const);
        self.register_builtin_type(builtin_mono("RecordType"), record_type, Const);
        self.register_builtin_type(builtin_mono("Int!"), int_mut, Const);
        self.register_builtin_type(builtin_mono("Nat!"), nat_mut, Const);
        self.register_builtin_type(builtin_mono("Float!"), float_mut, Const);
        self.register_builtin_type(builtin_mono("Ratio!"), ratio_mut, Const);
        self.register_builtin_type(builtin_mono("Bool!"), bool_mut, Const);
        self.register_builtin_type(builtin_mono("Str!"), str_mut, Const);
        self.register_builtin_type(builtin_mono("File!"), file_mut, Const);
        self.register_builtin_type(array_mut_t, array_mut_, Const);
        self.register_builtin_type(range_t, range, Const);
        self.register_builtin_type(builtin_mono("Tuple"), tuple_, Const);
        self.register_builtin_type(builtin_mono("Proc"), proc, Const);
        self.register_builtin_type(builtin_mono("Func"), func, Const);
        self.register_builtin_type(builtin_mono("QuantifiedFunc"), qfunc, Const);
    }

    fn init_builtin_funcs(&mut self) {
        let t_abs = nd_func(vec![param_t("n", builtin_mono("Num"))], None, Nat);
        let t_assert = func(
            vec![param_t("condition", Bool)],
            None,
            vec![param_t("err_message", Str)],
            NoneType,
        );
        let t_classof = nd_func(vec![param_t("old", Obj)], None, option(Class));
        let t_compile = nd_func(vec![param_t("src", Str)], None, Code);
        let t_cond = nd_func(
            vec![
                param_t("condition", Bool),
                param_t("then", mono_q("T")),
                param_t("else", mono_q("T")),
            ],
            None,
            mono_q("T"),
        );
        let t_cond = quant(t_cond, set! {static_instance("T", Type)});
        let t_discard = nd_func(vec![param_t("old", Obj)], None, NoneType);
        // FIXME: quantify
        let t_if = func(
            vec![
                param_t("cond", Bool),
                param_t("then", nd_func(vec![], None, mono_q("T"))),
            ],
            None,
            vec![param_t("else", nd_func(vec![], None, mono_q("T")))],
            option(mono_q("T")),
        );
        let t_if = quant(t_if, set! {static_instance("T", Type)});
        let t_import = nd_func(
            vec![anon(tp_enum(Str, set! {mono_q_tp("Path")}))],
            None,
            module(mono_q_tp("Path")),
        );
        let t_import = quant(t_import, set! {static_instance("Path", Str)});
        let t_log = func(
            vec![],
            Some(param_t("objects", ref_(Obj))),
            vec![
                param_t("sep", Str),
                param_t("end", Str),
                param_t("file", builtin_mono("Write")),
                param_t("flush", Bool),
            ],
            NoneType,
        );
        let t_pyimport = nd_func(
            vec![anon(tp_enum(Str, set! {mono_q_tp("Path")}))],
            None,
            module(mono_q_tp("Path")),
        );
        let t_pyimport = quant(t_pyimport, set! {static_instance("Path", Str)});
        let t_quit = func(vec![], None, vec![param_t("code", Int)], NoneType);
        self.register_builtin_impl("abs", t_abs, Immutable, Private);
        self.register_builtin_impl("assert", t_assert, Const, Private); // assert casting に悪影響が出る可能性があるため、Constとしておく
        self.register_builtin_impl("classof", t_classof, Immutable, Private);
        self.register_builtin_impl("compile", t_compile, Immutable, Private);
        self.register_builtin_impl("cond", t_cond, Immutable, Private);
        self.register_builtin_impl("discard", t_discard, Immutable, Private);
        self.register_builtin_impl("if", t_if, Immutable, Private);
        self.register_builtin_impl("log", t_log, Immutable, Private);
        self.register_builtin_impl("import", t_import, Immutable, Private);
        if cfg!(feature = "debug") {
            self.register_builtin_impl("py", t_pyimport.clone(), Immutable, Private);
        }
        self.register_builtin_impl("pyimport", t_pyimport, Immutable, Private);
        self.register_builtin_impl("quit", t_quit, Immutable, Private);
    }

    fn init_builtin_const_funcs(&mut self) {
        let class_t = func(
            vec![param_t("Requirement", Type)],
            None,
            vec![param_t("Impl", Type)],
            Class,
        );
        let class = ConstSubr::Builtin(BuiltinConstSubr::new("Class", class_func, class_t, None));
        self.register_builtin_const("Class", ValueObj::Subr(class));
        let inherit_t = func(
            vec![param_t("Super", Class)],
            None,
            vec![param_t("Impl", Type), param_t("Additional", Type)],
            Class,
        );
        let inherit = ConstSubr::Builtin(BuiltinConstSubr::new(
            "Inherit",
            inherit_func,
            inherit_t,
            None,
        ));
        self.register_builtin_const("Inherit", ValueObj::Subr(inherit));
        let trait_t = func(
            vec![param_t("Requirement", Type)],
            None,
            vec![param_t("Impl", Type)],
            Trait,
        );
        let trait_ = ConstSubr::Builtin(BuiltinConstSubr::new("Trait", trait_func, trait_t, None));
        self.register_builtin_const("Trait", ValueObj::Subr(trait_));
        let subsume_t = func(
            vec![param_t("Super", Trait)],
            None,
            vec![param_t("Impl", Type), param_t("Additional", Type)],
            Trait,
        );
        let subsume = ConstSubr::Builtin(BuiltinConstSubr::new(
            "Subsume",
            subsume_func,
            subsume_t,
            None,
        ));
        self.register_builtin_const("Subsume", ValueObj::Subr(subsume));
        // decorators
        let inheritable_t = func1(Class, Class);
        let inheritable = ConstSubr::Builtin(BuiltinConstSubr::new(
            "Inheritable",
            inheritable_func,
            inheritable_t,
            None,
        ));
        self.register_builtin_const("Inheritable", ValueObj::Subr(inheritable));
    }

    fn init_builtin_procs(&mut self) {
        let t_dir = proc(
            vec![param_t("obj", ref_(Obj))],
            None,
            vec![],
            array(Str, TyParam::erased(Nat)),
        );
        let t_print = proc(
            vec![],
            Some(param_t("objects", ref_(Obj))),
            vec![
                param_t("sep", Str),
                param_t("end", Str),
                param_t("file", builtin_mono("Write")),
                param_t("flush", Bool),
            ],
            NoneType,
        );
        let t_id = nd_func(vec![param_t("old", Obj)], None, Nat);
        let t_input = proc(vec![], None, vec![param_t("msg", Str)], Str);
        let t_if = proc(
            vec![
                param_t("cond", Bool),
                param_t("then", nd_proc(vec![], None, mono_q("T"))),
            ],
            None,
            vec![param_t("else", nd_proc(vec![], None, mono_q("T")))],
            option(mono_q("T")),
        );
        let t_if = quant(t_if, set! {static_instance("T", Type)});
        let t_for = nd_proc(
            vec![
                param_t("iter", iter(mono_q("T"))),
                param_t("p", nd_proc(vec![anon(mono_q("T"))], None, NoneType)),
            ],
            None,
            NoneType,
        );
        let t_for = quant(t_for, set! {static_instance("T", Type)});
        let t_while = nd_proc(
            vec![
                param_t("cond", builtin_mono("Bool!")),
                param_t("p", nd_proc(vec![], None, NoneType)),
            ],
            None,
            NoneType,
        );
        let t_open = proc(
            vec![param_t("file", mono_q("P"))],
            None,
            vec![
                param_t("mode", Str),
                param_t("buffering", Int),
                param_t("encoding", or(Str, NoneType)),
                param_t("errors", or(Str, NoneType)),
                param_t("newline", or(Str, NoneType)),
                param_t("closefd", Bool),
                // param_t("opener", option),
            ],
            builtin_mono("File!"),
        );
        let t_open = quant(
            t_open,
            set! {subtypeof(mono_q("P"), builtin_mono("PathLike"))},
        );
        // TODO: T <: With
        let t_with = nd_proc(
            vec![
                param_t("obj", mono_q("T")),
                param_t("p!", nd_proc(vec![anon(mono_q("T"))], None, mono_q("U"))),
            ],
            None,
            mono_q("U"),
        );
        let t_with = quant(
            t_with,
            set! {static_instance("T", Type), static_instance("U", Type)},
        );
        self.register_builtin_impl("dir!", t_dir, Immutable, Private);
        self.register_builtin_impl("print!", t_print, Immutable, Private);
        self.register_builtin_impl("id!", t_id, Immutable, Private);
        self.register_builtin_impl("input!", t_input, Immutable, Private);
        self.register_builtin_impl("if!", t_if, Immutable, Private);
        self.register_builtin_impl("for!", t_for, Immutable, Private);
        self.register_builtin_impl("while!", t_while, Immutable, Private);
        self.register_builtin_impl("open!", t_open, Immutable, Private);
        self.register_builtin_impl("with!", t_with, Immutable, Private);
    }

    fn init_builtin_operators(&mut self) {
        /* binary */
        let l = mono_q("L");
        let r = mono_q("R");
        let params = vec![ty_tp(mono_q("R"))];
        let op_t = nd_func(
            vec![param_t("lhs", l.clone()), param_t("rhs", r.clone())],
            None,
            mono_proj(mono_q("L"), "Output"),
        );
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), builtin_poly("Add", params.clone()))
            },
        );
        self.register_builtin_impl("__add__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "Output"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), builtin_poly("Sub", params.clone()))
            },
        );
        self.register_builtin_impl("__sub__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "Output"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), builtin_poly("Mul", params.clone()))
            },
        );
        self.register_builtin_impl("__mul__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "Output"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), builtin_poly("Div", params.clone()))
            },
        );
        self.register_builtin_impl("__div__", op_t, Const, Private);
        let m = mono_q("M");
        let op_t = bin_op(m.clone(), m.clone(), mono_proj(m.clone(), "PowOutput"));
        let op_t = quant(op_t, set! {subtypeof(m, builtin_poly("Mul", vec![]))});
        // TODO: add bound: M == M.Output
        self.register_builtin_impl("__pow__", op_t, Const, Private);
        let d = mono_q("D");
        let op_t = bin_op(d.clone(), d.clone(), mono_proj(d.clone(), "ModOutput"));
        let op_t = quant(op_t, set! {subtypeof(d, builtin_poly("Div", vec![]))});
        self.register_builtin_impl("__mod__", op_t, Const, Private);
        let e = mono_q("E");
        let op_t = bin_op(e.clone(), e.clone(), Bool);
        let op_t = quant(op_t, set! {subtypeof(e, builtin_poly("Eq", vec![]))});
        self.register_builtin_impl("__eq__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__ne__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r, Bool);
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l, builtin_poly("PartialOrd", params))
            },
        );
        self.register_builtin_impl("__lt__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__le__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__gt__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__ge__", op_t, Const, Private);
        self.register_builtin_impl("__and__", bin_op(Bool, Bool, Bool), Const, Private);
        self.register_builtin_impl("__or__", bin_op(Bool, Bool, Bool), Const, Private);
        let t = mono_q("T");
        let op_t = bin_op(t.clone(), t.clone(), range(t.clone()));
        let op_t = quant(op_t, set! {subtypeof(t, builtin_mono("Ord"))});
        self.register_builtin_decl("__rng__", op_t.clone(), Private);
        self.register_builtin_decl("__lorng__", op_t.clone(), Private);
        self.register_builtin_decl("__rorng__", op_t.clone(), Private);
        self.register_builtin_decl("__orng__", op_t, Private);
        // TODO: use existential type: |T: Type| (T, In(T)) -> Bool
        let op_t = bin_op(mono_q("T"), mono_q("I"), Bool);
        let op_t = quant(
            op_t,
            set! { static_instance("T", Type), subtypeof(mono_q("I"), builtin_poly("In", vec![ty_tp(mono_q("T"))])) },
        );
        self.register_builtin_impl("__in__", op_t, Const, Private);
        /* unary */
        // TODO: Boolの+/-は警告を出したい
        let op_t = func1(mono_q("T"), mono_proj(mono_q("T"), "MutType!"));
        let op_t = quant(
            op_t,
            set! {subtypeof(mono_q("T"), builtin_mono("Mutizable"))},
        );
        self.register_builtin_impl("__mutate__", op_t, Const, Private);
        let n = mono_q("N");
        let op_t = func1(n.clone(), n.clone());
        let op_t = quant(op_t, set! {subtypeof(n, builtin_mono("Num"))});
        self.register_builtin_decl("__pos__", op_t.clone(), Private);
        self.register_builtin_decl("__neg__", op_t, Private);
    }

    fn init_builtin_patches(&mut self) {
        let m = mono_q_tp("M");
        let n = mono_q_tp("N");
        let o = mono_q_tp("O");
        let p = mono_q_tp("P");
        let params = vec![
            PS::named_nd("M", Int),
            PS::named_nd("N", Int),
            PS::named_nd("O", Int),
            PS::named_nd("P", Int),
        ];
        // Interval is a bounding patch connecting M..N and (Add(O..P, M+O..N..P), Sub(O..P, M-P..N-O))
        let mut interval = Self::builtin_poly_patch("Interval", Type::from(&m..=&n), params, 2);
        let op_t = fn1_met(
            Type::from(&m..=&n),
            Type::from(&o..=&p),
            Type::from(m.clone() + o.clone()..=n.clone() + p.clone()),
        );
        let mut interval_add = Self::builtin_methods("Add", 2);
        interval_add.register_builtin_impl("__add__", op_t, Const, Public);
        interval_add.register_builtin_const(
            "Output",
            ValueObj::builtin_t(Type::from(m.clone() + o.clone()..=n.clone() + p.clone())),
        );
        interval.register_trait(
            Type::from(&m..=&n),
            builtin_poly("Add", vec![TyParam::from(&o..=&p)]),
            interval_add,
        );
        let mut interval_sub = Self::builtin_methods("Sub", 2);
        let op_t = fn1_met(
            Type::from(&m..=&n),
            Type::from(&o..=&p),
            Type::from(m.clone() - p.clone()..=n.clone() - o.clone()),
        );
        interval_sub.register_builtin_impl("__sub__", op_t, Const, Public);
        interval_sub.register_builtin_const(
            "Output",
            ValueObj::builtin_t(Type::from(m.clone() - p.clone()..=n.clone() - o.clone())),
        );
        interval.register_trait(
            Type::from(&m..=&n),
            builtin_poly("Sub", vec![TyParam::from(&o..=&p)]),
            interval_sub,
        );
        self.register_builtin_patch("Interval", interval, Const);
        // eq.register_impl("__ne__", op_t,         Const, Public);
        // ord.register_impl("__le__", op_t.clone(), Const, Public);
        // ord.register_impl("__gt__", op_t.clone(), Const, Public);
        // ord.register_impl("__ge__", op_t,         Const, Public);
    }

    pub(crate) fn init_builtins(mod_cache: &SharedModuleCache) {
        // TODO: capacityを正確に把握する
        let mut ctx = Context::builtin_module("<builtins>", 40);
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
