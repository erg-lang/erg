//! defines type information for builtin objects (in `Context`)
//!
//! 組み込みオブジェクトの型情報を(Contextに)定義
pub mod const_func;
pub mod py_mods;

use erg_common::set;
use erg_common::vis::Visibility;
use erg_common::Str;

use erg_type::typaram::TyParam;
use erg_type::value::ValueObj;
use erg_type::Type;
use erg_type::{constructors::*, BuiltinConstSubr, ConstSubr};
use ParamSpec as PS;
use Type::*;

use erg_parser::ast::VarName;

use crate::context::initialize::const_func::{class_func, inherit_func, inheritable_func};
use crate::context::instantiate::{ConstTemplate, TyVarContext};
use crate::context::{Context, ContextKind, DefaultInfo, ParamSpec, TraitInstance};
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
                .insert(name, VarInfo::new(t, Immutable, vis, Builtin));
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
                .insert(name, VarInfo::new(t, muty, vis, Builtin));
        }
    }

    fn register_builtin_const(&mut self, name: &str, obj: ValueObj) {
        if self.rec_get_const_obj(name).is_some() {
            panic!("already registered: {name}");
        } else {
            let vi = VarInfo::new(enum_t(set! {obj.clone()}), Const, Private, Builtin);
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
            self.locals
                .insert(name.clone(), VarInfo::new(Type, muty, Private, Builtin));
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
        let t = Self::instantiate_t(t, &mut tv_ctx);
        // FIXME: panic
        if let Some((_, root_ctx)) = self.poly_types.get_mut(&t.name()) {
            root_ctx.method_defs.push((t, ctx));
        } else {
            let name = VarName::from_str(t.name());
            self.locals
                .insert(name.clone(), VarInfo::new(Type, muty, Private, Builtin));
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
            self.locals
                .insert(name.clone(), VarInfo::new(Type, muty, Private, Builtin));
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
        let unpack = Self::mono_trait("Unpack", vec![], Self::TOP_LEVEL);
        let named = Self::mono_trait("Named", vec![], Self::TOP_LEVEL);
        let mut mutable = Self::mono_trait("Mutable", vec![], Self::TOP_LEVEL);
        let proj = mono_proj(mono_q("Self"), "ImmutType");
        let f_t = func(vec![param_t("old", proj.clone())], None, vec![], proj);
        let t = pr1_met(mono_q("Self"), None, f_t, NoneType);
        let t = quant(t, set! { subtypeof(mono_q("Self"), mono("Immutizable")) });
        mutable.register_builtin_decl("update!", t, Public);
        let mut immutizable =
            Self::mono_trait("Immutizable", vec![mono("Mutable")], Self::TOP_LEVEL);
        immutizable.register_builtin_decl("ImmutType", Type, Public);
        let mut mutizable = Self::mono_trait("Mutizable", vec![], Self::TOP_LEVEL);
        mutizable.register_builtin_decl("MutType!", Type, Public);
        let mut in_ = Self::poly_trait(
            "In",
            vec![PS::t("T", NonDefault)],
            vec![poly("Input", vec![ty_tp(mono_q("T"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("T"), mono_q("I"), Bool);
        let op_t = quant(
            op_t,
            set! { static_instance("T", Type), subtypeof(mono_q("I"), poly("In", vec![ty_tp(mono_q("T"))])) },
        );
        in_.register_builtin_decl("__in__", op_t, Public);
        // Erg does not have a trait equivalent to `PartialEq` in Rust
        // This means, Erg's `Float` cannot be compared with other `Float`
        // use `l - r < EPSILON` to check if two floats are almost equal
        let mut eq = Self::poly_trait(
            "Eq",
            vec![PS::t("R", WithDefault)],
            vec![poly("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        // __eq__: |Self <: Eq()| Self.(Self) -> Bool
        let op_t = fn1_met(mono_q("Self"), mono_q("R"), Bool);
        let op_t = quant(
            op_t,
            set! {
                subtypeof(mono_q("Self"), poly("Eq", vec![ty_tp(mono_q("R"))])),
                static_instance("R", Type)
            },
        );
        eq.register_builtin_decl("__eq__", op_t.clone(), Public);
        let mut partial_ord = Self::poly_trait(
            "PartialOrd",
            vec![PS::t("R", WithDefault)],
            vec![poly("PartialEq", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("Self"), mono_q("R"), Bool);
        let op_t = quant(
            op_t,
            set! {
                subtypeof(mono_q("Self"), poly("PartialOrd", vec![ty_tp(mono_q("R"))])),
                static_instance("R", Type)
            },
        );
        partial_ord.register_builtin_decl("__lt__", op_t.clone(), Public);
        let ord = Self::mono_trait(
            "Ord",
            vec![poly("Eq", vec![]), poly("PartialOrd", vec![])],
            Self::TOP_LEVEL,
        );
        let num = Self::mono_trait(
            "Num",
            vec![
                poly("Add", vec![]),
                poly("Sub", vec![]),
                poly("Mul", vec![]),
            ],
            Self::TOP_LEVEL,
        );
        let mut seq = Self::poly_trait(
            "Seq",
            vec![PS::t("T", NonDefault)],
            vec![poly("Output", vec![ty_tp(mono_q("T"))])],
            Self::TOP_LEVEL,
        );
        let self_t = mono_q("Self");
        let t = fn0_met(self_t.clone(), Nat);
        let t = quant(
            t,
            set! {subtypeof(self_t.clone(), poly("Seq", vec![TyParam::erased(Type)]))},
        );
        seq.register_builtin_decl("__len__", t, Public);
        let t = fn1_met(self_t.clone(), Nat, mono_q("T"));
        let t = quant(
            t,
            set! {subtypeof(self_t, poly("Seq", vec![ty_tp(mono_q("T"))])), static_instance("T", Type)},
        );
        // Seq.get: |Self <: Seq(T)| Self.(Nat) -> T
        seq.register_builtin_decl("get", t, Public);
        let params = vec![PS::t("T", NonDefault)];
        let input = Self::poly_trait("Input", params.clone(), vec![], Self::TOP_LEVEL);
        let output = Self::poly_trait("Output", params.clone(), vec![], Self::TOP_LEVEL);
        let r = mono_q("R");
        let r_bound = static_instance("R", Type);
        let params = vec![PS::t("R", WithDefault)];
        let ty_params = vec![ty_tp(mono_q("R"))];
        let mut add = Self::poly_trait(
            "Add",
            params.clone(),
            vec![poly("Output", vec![ty_tp(mono_q("R"))])], // Rについて共変(__add__の型とは関係ない)
            Self::TOP_LEVEL,
        );
        let self_bound = subtypeof(mono_q("Self"), poly("Add", ty_params.clone()));
        let op_t = fn1_met(mono_q("Self"), r.clone(), mono_proj(mono_q("Self"), "AddO"));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        add.register_builtin_decl("__add__", op_t, Public);
        add.register_builtin_decl("AddO", Type, Public);
        let mut sub = Self::poly_trait(
            "Sub",
            params.clone(),
            vec![poly("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("Self"), r.clone(), mono_proj(mono_q("Self"), "SubO"));
        let self_bound = subtypeof(mono_q("Self"), poly("Sub", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        sub.register_builtin_decl("__sub__", op_t, Public);
        sub.register_builtin_decl("SubO", Type, Public);
        let mut mul = Self::poly_trait(
            "Mul",
            params.clone(),
            vec![poly("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("Self"), r.clone(), mono_proj(mono_q("Self"), "MulO"));
        let self_bound = subtypeof(mono_q("Self"), poly("Mul", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        mul.register_builtin_decl("__mul__", op_t, Public);
        mul.register_builtin_decl("MulO", Type, Public);
        let mut div = Self::poly_trait(
            "Div",
            params.clone(),
            vec![poly("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("Self"), r, mono_proj(mono_q("Self"), "DivO"));
        let self_bound = subtypeof(mono_q("Self"), poly("Div", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        div.register_builtin_decl("__div__", op_t, Public);
        div.register_builtin_decl("DivO", Type, Public);
        self.register_builtin_type(mono("Unpack"), unpack, Const);
        self.register_builtin_type(mono("Named"), named, Const);
        self.register_builtin_type(mono("Mutable"), mutable, Const);
        self.register_builtin_type(mono("Immutizable"), immutizable, Const);
        self.register_builtin_type(mono("Mutizable"), mutizable, Const);
        self.register_builtin_type(poly("In", vec![ty_tp(mono_q("T"))]), in_, Const);
        self.register_builtin_type(poly("Eq", vec![ty_tp(mono_q("R"))]), eq, Const);
        self.register_builtin_type(
            poly("PartialOrd", vec![ty_tp(mono_q("R"))]),
            partial_ord,
            Const,
        );
        self.register_builtin_type(mono("Ord"), ord, Const);
        self.register_builtin_type(mono("Num"), num, Const);
        self.register_builtin_type(poly("Seq", vec![ty_tp(mono_q("T"))]), seq, Const);
        self.register_builtin_type(poly("Input", vec![ty_tp(mono_q("T"))]), input, Const);
        self.register_builtin_type(poly("Output", vec![ty_tp(mono_q("T"))]), output, Const);
        self.register_builtin_type(poly("Add", ty_params.clone()), add, Const);
        self.register_builtin_type(poly("Sub", ty_params.clone()), sub, Const);
        self.register_builtin_type(poly("Mul", ty_params.clone()), mul, Const);
        self.register_builtin_type(poly("Div", ty_params), div, Const);
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
        let mut obj = Self::mono_class("Obj", vec![], vec![], Self::TOP_LEVEL);
        let t = fn0_met(mono_q("Self"), mono_q("Self"));
        let t = quant(t, set! {subtypeof(mono_q("Self"), mono("Obj"))});
        obj.register_builtin_impl("clone", t, Const, Public);
        obj.register_builtin_impl("__module__", Str, Const, Public);
        obj.register_builtin_impl("__sizeof__", fn0_met(Obj, Nat), Const, Public);
        obj.register_builtin_impl("__repr__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_builtin_impl("__str__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_builtin_impl("__dict__", fn0_met(Obj, dict(Str, Obj)), Immutable, Public);
        obj.register_builtin_impl("__bytes__", fn0_met(Obj, mono("Bytes")), Immutable, Public);
        obj.register_builtin_const("MutType!", ValueObj::builtin_t(mono("Obj!")));
        // let mut record = Self::mono_trait("Record", vec![Obj], Self::TOP_LEVEL);
        // let mut class = Self::mono_class("Class", vec![Type, Obj], Self::TOP_LEVEL);
        let mut int = Self::mono_class(
            "Int",
            vec![Ratio, Obj],
            vec![
                mono("Ord"),
                poly("Eq", vec![ty_tp(Int)]),
                poly("Add", vec![ty_tp(Int)]),
                poly("Sub", vec![ty_tp(Int)]),
                poly("Mul", vec![ty_tp(Int)]),
                poly("Div", vec![ty_tp(Int)]),
                mono("Num"),
                // class("Rational"),
                // class("Integral"),
                mono("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        int.register_builtin_impl("abs", fn0_met(Int, Nat), Immutable, Public);
        // __div__ is not included in Int (cast to Float)
        let op_t = fn1_met(Int, Int, Int);
        int.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        int.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        int.register_builtin_impl("__mul__", op_t, Const, Public);
        int.register_builtin_const("AddO", ValueObj::builtin_t(Int));
        int.register_builtin_const("SubO", ValueObj::builtin_t(Int));
        int.register_builtin_const("MulO", ValueObj::builtin_t(Int));
        int.register_builtin_const("DivO", ValueObj::builtin_t(Ratio));
        int.register_builtin_const("MutType!", ValueObj::builtin_t(mono("Int!")));
        int.register_builtin_impl("Real", Int, Const, Public);
        int.register_builtin_impl("Imag", Int, Const, Public);
        let mut nat = Self::mono_class(
            "Nat",
            vec![Int, Ratio, Obj],
            vec![
                mono("Ord"),
                poly("Eq", vec![ty_tp(Nat)]),
                poly("Add", vec![ty_tp(Nat)]),
                poly("Sub", vec![ty_tp(Nat)]),
                poly("Mul", vec![ty_tp(Nat)]),
                poly("Div", vec![ty_tp(Nat)]),
                mono("Num"),
                // class("Rational"),
                // class("Integral"),
                mono("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        // __sub__, __div__ is not included in Nat (cast to Int)
        let op_t = fn1_met(Nat, Nat, Nat);
        nat.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        nat.register_builtin_impl("__mul__", op_t, Const, Public);
        nat.register_builtin_impl(
            "times!",
            pr_met(
                Nat,
                None,
                vec![param_t("p", nd_proc(vec![], None, NoneType))],
                None,
                vec![],
                NoneType,
            ),
            Immutable,
            Public,
        );
        nat.register_builtin_const("AddO", ValueObj::builtin_t(Nat));
        nat.register_builtin_const("SubO", ValueObj::builtin_t(Int));
        nat.register_builtin_const("MulO", ValueObj::builtin_t(Nat));
        nat.register_builtin_const("DivO", ValueObj::builtin_t(Ratio));
        nat.register_builtin_const("MutType!", ValueObj::builtin_t(mono("Nat!")));
        nat.register_builtin_impl("Real", Nat, Const, Public);
        nat.register_builtin_impl("Imag", Nat, Const, Public);
        let mut float = Self::mono_class(
            "Float",
            vec![Obj],
            vec![
                mono("Num"),
                // class("Eq"), // Float doesn't have an Eq implementation
                mono("Ord"),
                poly("Add", vec![ty_tp(Float)]),
                poly("Sub", vec![ty_tp(Float)]),
                poly("Mul", vec![ty_tp(Float)]),
                poly("Div", vec![ty_tp(Float)]),
                mono("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(Float, Float, Float);
        float.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        float.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        float.register_builtin_impl("__mul__", op_t.clone(), Const, Public);
        float.register_builtin_impl("__div__", op_t, Const, Public);
        float.register_builtin_const("AddO", ValueObj::builtin_t(Float));
        float.register_builtin_const("SubO", ValueObj::builtin_t(Float));
        float.register_builtin_const("MulO", ValueObj::builtin_t(Float));
        float.register_builtin_const("DivO", ValueObj::builtin_t(Float));
        float.register_builtin_const("MutType!", ValueObj::builtin_t(mono("Float!")));
        float.register_builtin_impl("Real", Float, Const, Public);
        float.register_builtin_impl("Imag", Float, Const, Public);
        let mut ratio = Self::mono_class(
            "Ratio",
            vec![Obj],
            vec![
                mono("Num"),
                poly("Eq", vec![ty_tp(Ratio)]),
                mono("Ord"),
                poly("Add", vec![ty_tp(Ratio)]),
                poly("Sub", vec![ty_tp(Ratio)]),
                poly("Mul", vec![ty_tp(Ratio)]),
                poly("Div", vec![ty_tp(Ratio)]),
                mono("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(Ratio, Ratio, Ratio);
        ratio.register_builtin_impl("__add__", op_t.clone(), Const, Public);
        ratio.register_builtin_impl("__sub__", op_t.clone(), Const, Public);
        ratio.register_builtin_impl("__mul__", op_t.clone(), Const, Public);
        ratio.register_builtin_impl("__div__", op_t, Const, Public);
        ratio.register_builtin_const("AddO", ValueObj::builtin_t(Ratio));
        ratio.register_builtin_const("SubO", ValueObj::builtin_t(Ratio));
        ratio.register_builtin_const("MulO", ValueObj::builtin_t(Ratio));
        ratio.register_builtin_const("DivO", ValueObj::builtin_t(Ratio));
        ratio.register_builtin_const("MutType!", ValueObj::builtin_t(mono("Ratio!")));
        ratio.register_builtin_impl("Real", Ratio, Const, Public);
        ratio.register_builtin_impl("Imag", Ratio, Const, Public);
        let mut bool_ = Self::mono_class(
            "Bool",
            vec![Nat, Int, Ratio, Obj],
            vec![
                mono("Num"),
                // class("Rational"),
                // class("Integral"),
                poly("Eq", vec![ty_tp(Bool)]),
                poly("Add", vec![ty_tp(Bool)]),
                mono("Ord"),
                // mono("SelfAdd"),
                // mono("SelfSub"),
                // mono("SelfMul"),
                // mono("SelfDiv"),
                mono("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        bool_.register_builtin_impl("__and__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_builtin_impl("__or__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_builtin_const("MutType!", ValueObj::builtin_t(mono("Bool!")));
        let mut str_ = Self::mono_class(
            "Str",
            vec![Obj],
            vec![
                poly("Eq", vec![ty_tp(Str)]),
                mono("Ord"),
                mono("Mutizable"),
                poly("Seq", vec![ty_tp(Str)]),
                poly("Add", vec![ty_tp(Str)]),
                poly("Mul", vec![ty_tp(Nat)]),
            ],
            Self::TOP_LEVEL,
        );
        str_.register_builtin_impl("__add__", fn1_met(Str, Str, Str), Const, Public);
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
                mono("Bytes"),
            ),
            Immutable,
            Public,
        );
        str_.register_builtin_const("AddO", ValueObj::builtin_t(Str));
        str_.register_builtin_const("MulO", ValueObj::builtin_t(Str));
        str_.register_builtin_const("MutType!", ValueObj::builtin_t(mono("Str!")));
        let mut type_ = Self::mono_class(
            "Type",
            vec![Obj],
            vec![
                poly("Eq", vec![ty_tp(Type)]),
                poly("In", vec![ty_tp(Obj)]), // x in Type
                mono("Named"),
            ],
            Self::TOP_LEVEL,
        );
        type_.register_builtin_impl("mro", array(Type, TyParam::erased(Nat)), Immutable, Public);
        let module = Self::mono_class(
            "Module",
            vec![Obj],
            vec![poly("Eq", vec![ty_tp(Module)]), mono("Named")],
            Self::TOP_LEVEL,
        );
        let mut array_ = Self::poly_class(
            "Array",
            vec![PS::t_nd("T"), PS::named_nd("N", Nat)],
            vec![Obj],
            vec![
                poly(
                    "Eq",
                    vec![ty_tp(poly(
                        "Array",
                        vec![ty_tp(mono_q("T")), mono_q_tp("N")],
                    ))],
                ),
                mono("Mutizable"),
                poly("Seq", vec![ty_tp(mono_q("T"))]),
                poly("Output", vec![ty_tp(mono_q("T"))]),
            ],
            Self::TOP_LEVEL,
        );
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
        let n = mono_q_tp("N");
        let array_inner = mono_q("T");
        let array_t = array(array_inner.clone(), n.clone());
        let proj_t = mono_proj(array_inner.clone(), "ImmutType");
        let t = fn_met(
            array_t.clone(),
            vec![param_t(
                "f",
                nd_func(vec![anon(proj_t.clone())], None, proj_t),
            )],
            None,
            vec![],
            NoneType,
        );
        let t = quant(
            t,
            set! {static_instance("N", Nat), static_instance("T", mono("Mutable"))},
        );
        array_.register_builtin_impl("map!", t, Immutable, Public);
        let mut_type = ValueObj::builtin_t(poly(
            "Array!",
            vec![TyParam::t(mono_q("T")), TyParam::mono_q("N").mutate()],
        ));
        // [T; N].MutType! = [T; !N] (neither [T!; N] nor [T; N]!)
        array_.register_builtin_const("MutType!", mut_type);
        let mut int_mut = Self::mono_class(
            "Int!",
            vec![Int, mono("Ratio!"), Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        // TODO: make Tuple6, Tuple7, ... etc.
        let tuple_ = Self::mono_class(
            "Tuple",
            vec![Obj],
            vec![poly("Eq", vec![ty_tp(mono("Tuple"))])],
            Self::TOP_LEVEL,
        );
        let tuple1 = Self::poly_class(
            "Tuple1",
            vec![PS::t_nd("A")],
            vec![mono("Tuple"), Obj],
            vec![poly(
                "Eq",
                vec![ty_tp(poly("Tuple1", vec![ty_tp(mono_q("A"))]))],
            )],
            Self::TOP_LEVEL,
        );
        let tuple2 = Self::poly_class(
            "Tuple2",
            vec![PS::t_nd("A"), PS::t_nd("B")],
            vec![mono("Tuple"), Obj],
            vec![poly(
                "Eq",
                vec![ty_tp(poly(
                    "Tuple2",
                    vec![ty_tp(mono_q("A")), ty_tp(mono_q("B"))],
                ))],
            )],
            Self::TOP_LEVEL,
        );
        let tuple3 = Self::poly_class(
            "Tuple3",
            vec![PS::t_nd("A"), PS::t_nd("B"), PS::t_nd("C")],
            vec![mono("Tuple"), Obj],
            vec![poly(
                "Eq",
                vec![ty_tp(poly(
                    "Tuple3",
                    vec![ty_tp(mono_q("A")), ty_tp(mono_q("B")), ty_tp(mono_q("C"))],
                ))],
            )],
            Self::TOP_LEVEL,
        );
        let tuple4 = Self::poly_class(
            "Tuple4",
            vec![PS::t_nd("A"), PS::t_nd("B"), PS::t_nd("C"), PS::t_nd("D")],
            vec![mono("Tuple"), Obj],
            vec![poly(
                "Eq",
                vec![ty_tp(poly(
                    "Tuple4",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                    ],
                ))],
            )],
            Self::TOP_LEVEL,
        );
        let tuple5 = Self::poly_class(
            "Tuple5",
            vec![
                PS::t_nd("A"),
                PS::t_nd("B"),
                PS::t_nd("C"),
                PS::t_nd("D"),
                PS::t_nd("E"),
            ],
            vec![mono("Tuple"), Obj],
            vec![poly(
                "Eq",
                vec![ty_tp(poly(
                    "Tuple5",
                    vec![
                        ty_tp(mono_q("A")),
                        ty_tp(mono_q("B")),
                        ty_tp(mono_q("C")),
                        ty_tp(mono_q("D")),
                        ty_tp(mono_q("E")),
                    ],
                ))],
            )],
            Self::TOP_LEVEL,
        );
        let tuple6 = Self::poly_class(
            "Tuple6",
            vec![
                PS::t_nd("A"),
                PS::t_nd("B"),
                PS::t_nd("C"),
                PS::t_nd("D"),
                PS::t_nd("E"),
                PS::t_nd("F"),
            ],
            vec![mono("Tuple"), Obj],
            vec![poly(
                "Eq",
                vec![ty_tp(poly(
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
            )],
            Self::TOP_LEVEL,
        );
        let tuple7 = Self::poly_class(
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
            vec![mono("Tuple"), Obj],
            vec![poly(
                "Eq",
                vec![ty_tp(poly(
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
            )],
            Self::TOP_LEVEL,
        );
        let tuple8 = Self::poly_class(
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
            vec![mono("Tuple"), Obj],
            vec![poly(
                "Eq",
                vec![ty_tp(poly(
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
            )],
            Self::TOP_LEVEL,
        );
        int_mut.register_builtin_const("ImmutType", ValueObj::builtin_t(Int));
        let f_t = param_t("f", func(vec![param_t("old", Int)], None, vec![], Int));
        let t = pr_met(mono("Int!"), None, vec![f_t], None, vec![], NoneType);
        int_mut.register_builtin_impl("update!", t, Immutable, Public);
        let mut nat_mut = Self::mono_class(
            "Nat!",
            vec![Nat, mono("Int!"), mono("Ratio!"), Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        nat_mut.register_builtin_const("ImmutType", ValueObj::builtin_t(Nat));
        let f_t = param_t("f", func(vec![param_t("old", Nat)], None, vec![], Nat));
        let t = pr_met(mono("Nat!"), None, vec![f_t], None, vec![], NoneType);
        nat_mut.register_builtin_impl("update!", t, Immutable, Public);
        let mut float_mut = Self::mono_class(
            "Float!",
            vec![Float, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        float_mut.register_builtin_const("ImmutType", ValueObj::builtin_t(Float));
        let f_t = param_t("f", func(vec![param_t("old", Float)], None, vec![], Float));
        let t = pr_met(mono("Float!"), None, vec![f_t], None, vec![], NoneType);
        float_mut.register_builtin_impl("update!", t, Immutable, Public);
        let mut ratio_mut = Self::mono_class(
            "Ratio!",
            vec![Ratio, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        ratio_mut.register_builtin_const("ImmutType", ValueObj::builtin_t(Ratio));
        let f_t = param_t(
            "f",
            func(
                vec![param_t("old", mono("Ratio"))],
                None,
                vec![],
                mono("Ratio"),
            ),
        );
        let t = pr_met(mono("Ratio!"), None, vec![f_t], None, vec![], NoneType);
        ratio_mut.register_builtin_impl("update!", t, Immutable, Public);
        let mut bool_mut = Self::mono_class(
            "Bool!",
            vec![Bool, mono("Nat!"), mono("Int!"), mono("Ratio!"), Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        bool_mut.register_builtin_const("ImmutType", ValueObj::builtin_t(Bool));
        let f_t = param_t("f", func(vec![param_t("old", Bool)], None, vec![], Bool));
        let t = pr_met(mono("Bool!"), None, vec![f_t], None, vec![], NoneType);
        bool_mut.register_builtin_impl("update!", t, Immutable, Public);
        let mut str_mut = Self::mono_class(
            "Str!",
            vec![Str, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        str_mut.register_builtin_const("ImmutType", ValueObj::builtin_t(Str));
        let f_t = param_t("f", func(vec![param_t("old", Str)], None, vec![], Str));
        let t = pr_met(mono("Str!"), None, vec![f_t], None, vec![], NoneType);
        str_mut.register_builtin_impl("update!", t, Immutable, Public);
        let array_mut_t = poly("Array!", vec![ty_tp(mono_q("T")), mono_q_tp("N")]);
        let mut array_mut = Self::poly_class(
            "Array!",
            vec![PS::t_nd("T"), PS::named_nd("N", mono("Nat!"))],
            vec![poly("Range", vec![ty_tp(mono_q("T")), mono_q_tp("N")]), Obj],
            vec![mono("Mutizable"), poly("Seq", vec![ty_tp(mono_q("T"))])],
            Self::TOP_LEVEL,
        );
        let t = pr_met(
            ref_mut(array_mut_t.clone()),
            Some(ref_mut(poly(
                "Array!",
                vec![ty_tp(mono_q("T")), mono_q_tp("N") + value(1)],
            ))),
            vec![param_t("elem", mono_q("T"))],
            None,
            vec![],
            NoneType,
        );
        let t = quant(
            t,
            set! {static_instance("T", Type), static_instance("N", mono("Nat!"))},
        );
        array_mut.register_builtin_impl("push!", t, Immutable, Public);
        let f_t = param_t(
            "f",
            func(
                vec![param_t("old", array_t.clone())],
                None,
                vec![],
                array_t.clone(),
            ),
        );
        let t = pr_met(array_mut_t.clone(), None, vec![f_t], None, vec![], NoneType);
        array_mut.register_builtin_impl("update!", t, Immutable, Public);
        let range_t = poly("Range", vec![TyParam::t(mono_q("T"))]);
        let range = Self::poly_class(
            "Range",
            vec![PS::t_nd("T")],
            vec![Obj],
            vec![
                poly("Eq", vec![ty_tp(poly("Range", vec![ty_tp(mono_q("T"))]))]),
                mono("Mutizable"),
                poly("Seq", vec![ty_tp(mono_q("T"))]),
                poly("Output", vec![ty_tp(mono_q("T"))]),
            ],
            Self::TOP_LEVEL,
        );
        let func = Self::mono_class("Function", vec![Obj], vec![mono("Named")], Self::TOP_LEVEL);
        let qfunc = Self::mono_class(
            "QuantifiedFunction",
            vec![mono("Function"), Obj],
            vec![],
            Self::TOP_LEVEL,
        );
        self.register_builtin_type(Obj, obj, Const);
        // self.register_type(mono("Record"), vec![], record, Const);
        // self.register_type(mono("Class"), vec![], class, Const);
        self.register_builtin_type(Int, int, Const);
        self.register_builtin_type(Nat, nat, Const);
        self.register_builtin_type(Float, float, Const);
        self.register_builtin_type(Ratio, ratio, Const);
        self.register_builtin_type(Bool, bool_, Const);
        self.register_builtin_type(Str, str_, Const);
        self.register_builtin_type(Type, type_, Const);
        self.register_builtin_type(Module, module, Const);
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
        self.register_builtin_type(mono("Int!"), int_mut, Const);
        self.register_builtin_type(mono("Nat!"), nat_mut, Const);
        self.register_builtin_type(mono("Float!"), float_mut, Const);
        self.register_builtin_type(mono("Ratio!"), ratio_mut, Const);
        self.register_builtin_type(mono("Bool!"), bool_mut, Const);
        self.register_builtin_type(mono("Str!"), str_mut, Const);
        self.register_builtin_type(array_mut_t, array_mut, Const);
        self.register_builtin_type(range_t, range, Const);
        self.register_builtin_type(mono("Tuple"), tuple_, Const);
        self.register_builtin_type(mono("Function"), func, Const);
        self.register_builtin_type(mono("QuantifiedFunction"), qfunc, Const);
    }

    fn init_builtin_funcs(&mut self) {
        let t_abs = nd_func(vec![param_t("n", mono("Num"))], None, Nat);
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
        let t_id = nd_func(vec![param_t("old", Obj)], None, Nat);
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
        let t_import = nd_func(vec![param_t("path", Str)], None, Module);
        let t_log = func(
            vec![],
            Some(param_t("objects", ref_(Obj))),
            vec![
                param_t("sep", Str),
                param_t("end", Str),
                param_t("file", mono("Write")),
                param_t("flush", Bool),
            ],
            NoneType,
        );
        let t_pyimport = nd_func(vec![param_t("path", Str)], None, Module);
        let t_quit = func(vec![], None, vec![param_t("code", Int)], NoneType);
        self.register_builtin_impl("abs", t_abs, Const, Private);
        self.register_builtin_impl("assert", t_assert, Const, Private);
        self.register_builtin_impl("classof", t_classof, Const, Private);
        self.register_builtin_impl("compile", t_compile, Const, Private);
        self.register_builtin_impl("cond", t_cond, Const, Private);
        self.register_builtin_impl("discard", t_discard, Const, Private);
        self.register_builtin_impl("id", t_id, Const, Private);
        self.register_builtin_impl("if", t_if, Const, Private);
        self.register_builtin_impl("log", t_log, Const, Private);
        self.register_builtin_impl("import", t_import, Const, Private);
        if cfg!(feature = "debug") {
            self.register_builtin_impl("py", t_pyimport.clone(), Const, Private);
        }
        self.register_builtin_impl("pyimport", t_pyimport, Const, Private);
        self.register_builtin_impl("quit", t_quit, Const, Private);
    }

    fn init_builtin_const_funcs(&mut self) {
        let class_t = func(
            vec![param_t("Requirement", Type)],
            None,
            vec![param_t("Impl", Type)],
            Type,
        );
        let class = ConstSubr::Builtin(BuiltinConstSubr::new(class_func, class_t));
        self.register_builtin_const("Class", ValueObj::Subr(class));
        let inherit_t = func(
            vec![param_t("Super", Type)],
            None,
            vec![param_t("Impl", Type), param_t("Additional", Type)],
            Type,
        );
        let inherit = ConstSubr::Builtin(BuiltinConstSubr::new(inherit_func, inherit_t));
        self.register_builtin_const("Inherit", ValueObj::Subr(inherit));
        // decorators
        let inheritable =
            ConstSubr::Builtin(BuiltinConstSubr::new(inheritable_func, func1(Type, Type)));
        self.register_builtin_const("Inheritable", ValueObj::Subr(inheritable));
    }

    fn init_builtin_procs(&mut self) {
        let t_print = proc(
            vec![],
            Some(param_t("objects", ref_(Obj))),
            vec![
                param_t("sep", Str),
                param_t("end", Str),
                param_t("file", mono("Write")),
                param_t("flush", Bool),
            ],
            NoneType,
        );
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
                param_t("cond", mono("Bool!")),
                param_t("p", nd_proc(vec![], None, NoneType)),
            ],
            None,
            NoneType,
        );
        self.register_builtin_impl("print!", t_print, Const, Private);
        self.register_builtin_impl("input!", t_input, Const, Private);
        self.register_builtin_impl("if!", t_if, Const, Private);
        self.register_builtin_impl("for!", t_for, Const, Private);
        self.register_builtin_impl("while!", t_while, Const, Private);
    }

    fn init_builtin_operators(&mut self) {
        /* binary */
        let l = mono_q("L");
        let r = mono_q("R");
        let params = vec![ty_tp(mono_q("R"))];
        let op_t = nd_func(
            vec![param_t("lhs", l.clone()), param_t("rhs", r.clone())],
            None,
            mono_proj(mono_q("L"), "AddO"),
        );
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly("Add", params.clone()))
            },
        );
        self.register_builtin_impl("__add__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "SubO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly("Sub", params.clone()))
            },
        );
        self.register_builtin_impl("__sub__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "MulO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly("Mul", params.clone()))
            },
        );
        self.register_builtin_impl("__mul__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "DivO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l, poly("Mul", params.clone()))
            },
        );
        self.register_builtin_impl("__div__", op_t, Const, Private);
        let m = mono_q("M");
        let op_t = bin_op(m.clone(), m.clone(), m.clone());
        let op_t = quant(op_t, set! {subtypeof(m, poly("Mul", vec![]))});
        // TODO: add bound: M == MulO
        self.register_builtin_impl("__pow__", op_t, Const, Private);
        let d = mono_q("D");
        let op_t = bin_op(d.clone(), d.clone(), d.clone());
        let op_t = quant(op_t, set! {subtypeof(d, poly("Div", vec![]))});
        self.register_builtin_impl("__mod__", op_t, Const, Private);
        let e = mono_q("E");
        let op_t = bin_op(e.clone(), e.clone(), Bool);
        let op_t = quant(op_t, set! {subtypeof(e, poly("Eq", vec![]))});
        self.register_builtin_impl("__eq__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__ne__", op_t, Const, Private);
        let o = mono_q("O");
        let op_t = bin_op(o.clone(), o.clone(), Bool);
        let op_t = quant(op_t, set! {subtypeof(o, mono("Ord"))});
        self.register_builtin_impl("__lt__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__le__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__gt__", op_t.clone(), Const, Private);
        self.register_builtin_impl("__ge__", op_t, Const, Private);
        self.register_builtin_impl("__and__", bin_op(Bool, Bool, Bool), Const, Private);
        self.register_builtin_impl("__or__", bin_op(Bool, Bool, Bool), Const, Private);
        let t = mono_q("T");
        let op_t = bin_op(t.clone(), t.clone(), range(t.clone()));
        let op_t = quant(op_t, set! {subtypeof(t.clone(), mono("Ord"))});
        self.register_builtin_decl("__rng__", op_t.clone(), Private);
        self.register_builtin_decl("__lorng__", op_t.clone(), Private);
        self.register_builtin_decl("__rorng__", op_t.clone(), Private);
        self.register_builtin_decl("__orng__", op_t, Private);
        // TODO: use existential type: |T: Type| (T, In(T)) -> Bool
        let op_t = bin_op(mono_q("T"), mono_q("I"), Bool);
        let op_t = quant(
            op_t,
            set! { static_instance("T", Type), subtypeof(mono_q("I"), poly("In", vec![ty_tp(mono_q("T"))])) },
        );
        self.register_builtin_impl("__in__", op_t, Const, Private);
        /* unary */
        // TODO: Boolの+/-は警告を出したい
        let op_t = func1(mono_q("T"), mono_proj(mono_q("T"), "MutType!"));
        let op_t = quant(op_t, set! {subtypeof(mono_q("T"), mono("Mutizable"))});
        self.register_builtin_impl("__mutate__", op_t, Const, Private);
        let n = mono_q("N");
        let op_t = func1(n.clone(), n.clone());
        let op_t = quant(op_t, set! {subtypeof(n, mono("Num"))});
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
        let mut interval = Self::poly_patch(
            "Interval",
            params,
            vec![Type::from(&m..=&n)],
            vec![
                poly("Add", vec![TyParam::from(&o..=&p)]),
                poly("Sub", vec![TyParam::from(&o..=&p)]),
            ],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(
            Type::from(&m..=&n),
            Type::from(&o..=&p),
            Type::from(m.clone() + o.clone()..=n.clone() + p.clone()),
        );
        interval.register_builtin_impl("__add__", op_t, Const, Public);
        let op_t = fn1_met(
            Type::from(&m..=&n),
            Type::from(&o..=&p),
            Type::from(m.clone() - p.clone()..=n.clone() - o.clone()),
        );
        interval.register_builtin_impl("__sub__", op_t, Const, Public);
        interval.register_builtin_const(
            "AddO",
            ValueObj::builtin_t(Type::from(m.clone() + o.clone()..=n.clone() + p.clone())),
        );
        interval.register_builtin_const("SubO", ValueObj::builtin_t(Type::from(m - p..=n - o)));
        self.register_builtin_patch("Interval", interval, Const);
        // eq.register_impl("__ne__", op_t,         Const, Public);
        // ord.register_impl("__le__", op_t.clone(), Const, Public);
        // ord.register_impl("__gt__", op_t.clone(), Const, Public);
        // ord.register_impl("__ge__", op_t,         Const, Public);
    }

    pub(crate) fn init_builtins() -> Self {
        // TODO: capacityを正確に把握する
        let mut ctx = Context::module("<builtins>".into(), 40);
        ctx.init_builtin_funcs();
        ctx.init_builtin_const_funcs();
        ctx.init_builtin_procs();
        ctx.init_builtin_operators();
        ctx.init_builtin_traits();
        ctx.init_builtin_classes();
        ctx.init_builtin_patches();
        ctx
    }

    pub fn new_main_module() -> Self {
        Context::new(
            "<module>".into(),
            ContextKind::Module,
            vec![],
            Some(Context::init_builtins()),
            vec![],
            vec![],
            Context::TOP_LEVEL,
        )
    }
}
