//! defines type information for builtin objects (in `Context`)
//!
//! 組み込みオブジェクトの型情報を(Contextに)定義
use erg_common::set;
use erg_common::vis::Visibility;
use erg_common::Str;

use erg_type::constructors::*;
use erg_type::typaram::TyParam;
use erg_type::value::ValueObj;
use erg_type::Type;
use ParamSpec as PS;
use Type::*;

use erg_parser::ast::VarName;

use crate::context::instantiate::{ConstTemplate, TyVarContext};
use crate::context::{Context, ContextKind, DefaultInfo, ParamSpec, TraitInstance};
use crate::varinfo::{Mutability, VarInfo, VarKind};
use DefaultInfo::*;
use Mutability::*;
use VarKind::*;
use Visibility::*;

impl Context {
    fn register_decl(&mut self, name: &'static str, t: Type, vis: Visibility) {
        let name = VarName::from_static(name);
        if self.decls.get(&name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.decls
                .insert(name, VarInfo::new(t, Immutable, vis, Builtin));
        }
    }

    fn register_impl(&mut self, name: &'static str, t: Type, muty: Mutability, vis: Visibility) {
        let name = VarName::from_static(name);
        if self.locals.get(&name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.locals
                .insert(name, VarInfo::new(t, muty, vis, Builtin));
        }
    }

    fn register_const(&mut self, name: &'static str, obj: ValueObj) {
        if self.consts.get(name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.consts.insert(VarName::from_static(name), obj);
        }
    }

    fn register_const_param_defaults(&mut self, name: &'static str, params: Vec<ConstTemplate>) {
        if self.const_param_defaults.get(name).is_some() {
            panic!("already registered: {name}");
        } else {
            self.const_param_defaults.insert(Str::ever(name), params);
        }
    }

    fn register_type(&mut self, t: Type, ctx: Self, muty: Mutability) {
        if t.typarams_len().is_none() {
            self.register_mono_type(t, ctx, muty);
        } else {
            if t.is_class() {
                self.register_poly_class(t, ctx, muty);
            } else if t.is_trait() {
                self.register_poly_trait(t, ctx, muty);
            } else {
                todo!()
            }
        }
    }

    fn register_mono_type(&mut self, t: Type, ctx: Self, muty: Mutability) {
        if self.mono_types.contains_key(&t.name()) {
            panic!("{} has already been registered", t.name());
        } else {
            let name = VarName::from_str(t.name());
            self.locals
                .insert(name.clone(), VarInfo::new(Type, muty, Private, Builtin));
            self.consts.insert(name.clone(), ValueObj::t(t.clone()));
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

    fn register_poly_class(&mut self, t: Type, ctx: Self, muty: Mutability) {
        let mut tv_ctx = TyVarContext::new(self.level, ctx.type_params_bounds(), self);
        let t = Self::instantiate_t(t, &mut tv_ctx);
        if let Some((_, root_ctx)) = self.poly_classes.get_mut(&t.name()) {
            root_ctx.specializations.push((t, ctx));
        } else {
            let name = VarName::from_str(t.name());
            self.locals
                .insert(name.clone(), VarInfo::new(Type, muty, Private, Builtin));
            self.consts.insert(name.clone(), ValueObj::t(t.clone()));
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
            self.poly_classes.insert(name, (t, ctx));
        }
    }

    fn register_poly_trait(&mut self, t: Type, ctx: Self, muty: Mutability) {
        if self.poly_traits.contains_key(&t.name()) {
            panic!("{} has already been registered", t.name());
        } else {
            let mut tv_ctx = TyVarContext::new(self.level, ctx.type_params_bounds(), self);
            let t = Self::instantiate_t(t, &mut tv_ctx);
            let name = VarName::from_str(t.name());
            self.locals
                .insert(name.clone(), VarInfo::new(Type, muty, Private, Builtin));
            self.consts.insert(name.clone(), ValueObj::t(t.clone()));
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
            self.poly_traits.insert(name, (t, ctx));
        }
    }

    fn register_patch(&mut self, name: &'static str, ctx: Self, muty: Mutability) {
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
        let named = Self::mono_trait("Named", vec![], Self::TOP_LEVEL);
        let mut mutable = Self::mono_trait("Mutable", vec![], Self::TOP_LEVEL);
        let proj = mono_proj(mono_q("Self"), "ImmutType");
        let f_t = func(vec![param_t("old", proj.clone())], vec![], proj);
        let t = pr1_met(mono_q("Self"), None, f_t, NoneType);
        let t = quant(t, set! { subtypeof(mono_q("Self"), trait_("Immutizable")) });
        mutable.register_decl("update!", t, Public);
        let mut immutizable =
            Self::mono_trait("Immutizable", vec![trait_("Mutable")], Self::TOP_LEVEL);
        immutizable.register_decl("ImmutType", Type, Public);
        let mut mutizable = Self::mono_trait("Mutizable", vec![], Self::TOP_LEVEL);
        mutizable.register_decl("MutType!", Type, Public);
        let mut in_ = Self::poly_trait(
            "In",
            vec![PS::t("T", NonDefault)],
            vec![poly_trait("Input", vec![ty_tp(mono_q("T"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("T"), mono_q("I"), Bool);
        let op_t = quant(
            op_t,
            set! { static_instance("T", Type), subtypeof(mono_q("I"), poly_trait("In", vec![ty_tp(mono_q("T"))])) },
        );
        in_.register_decl("__in__", op_t, Public);
        // Erg does not have a trait equivalent to `PartialEq` in Rust
        // This means, Erg's `Float` cannot be compared with other `Float`
        // use `l - r < EPSILON` to check if two floats are almost equal
        let mut eq = Self::poly_trait(
            "Eq",
            vec![PS::t("R", WithDefault)],
            vec![poly_trait("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        // __eq__: |Self <: Eq()| Self.(Self) -> Bool
        let op_t = fn1_met(mono_q("Self"), mono_q("R"), Bool);
        let op_t = quant(
            op_t,
            set! {
                subtypeof(mono_q("Self"), poly_trait("Eq", vec![ty_tp(mono_q("R"))])),
                static_instance("R", Type)
            },
        );
        eq.register_decl("__eq__", op_t.clone(), Public);
        let mut partial_ord = Self::poly_trait(
            "PartialOrd",
            vec![PS::t("R", WithDefault)],
            vec![poly_trait("PartialEq", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("Self"), mono_q("R"), Bool);
        let op_t = quant(
            op_t,
            set! {
                subtypeof(mono_q("Self"), poly_trait("PartialOrd", vec![ty_tp(mono_q("R"))])),
                static_instance("R", Type)
            },
        );
        partial_ord.register_decl("__lt__", op_t.clone(), Public);
        let ord = Self::mono_trait(
            "Ord",
            vec![poly_trait("Eq", vec![]), poly_trait("PartialOrd", vec![])],
            Self::TOP_LEVEL,
        );
        let num = Self::mono_trait(
            "Num",
            vec![
                poly_trait("Add", vec![]),
                poly_trait("Sub", vec![]),
                poly_trait("Mul", vec![]),
            ],
            Self::TOP_LEVEL,
        );
        let mut seq = Self::poly_trait(
            "Seq",
            vec![PS::t("T", NonDefault)],
            vec![poly_trait("Output", vec![ty_tp(mono_q("T"))])],
            Self::TOP_LEVEL,
        );
        let self_t = mono_q("Self");
        let t = fn0_met(self_t.clone(), Nat);
        let t = quant(
            t,
            set! {subtypeof(self_t.clone(), poly_trait("Seq", vec![TyParam::erased(Type)]))},
        );
        seq.register_decl("__len__", t, Public);
        let t = fn1_met(self_t.clone(), Nat, mono_q("T"));
        let t = quant(
            t,
            set! {subtypeof(self_t, poly_trait("Seq", vec![ty_tp(mono_q("T"))])), static_instance("T", Type)},
        );
        // Seq.get: |Self <: Seq(T)| Self.(Nat) -> T
        seq.register_decl("get", t, Public);
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
            vec![poly_trait("Output", vec![ty_tp(mono_q("R"))])], // Rについて共変(__add__の型とは関係ない)
            Self::TOP_LEVEL,
        );
        let self_bound = subtypeof(mono_q("Self"), poly_trait("Add", ty_params.clone()));
        let op_t = fn1_met(mono_q("Self"), r.clone(), mono_proj(mono_q("Self"), "AddO"));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        add.register_decl("__add__", op_t, Public);
        add.register_decl("AddO", Type, Public);
        let mut sub = Self::poly_trait(
            "Sub",
            params.clone(),
            vec![poly_trait("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("Self"), r.clone(), mono_proj(mono_q("Self"), "SubO"));
        let self_bound = subtypeof(mono_q("Self"), poly_trait("Sub", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        sub.register_decl("__sub__", op_t, Public);
        sub.register_decl("SubO", Type, Public);
        let mut mul = Self::poly_trait(
            "Mul",
            params.clone(),
            vec![poly_trait("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("Self"), r.clone(), mono_proj(mono_q("Self"), "MulO"));
        let self_bound = subtypeof(mono_q("Self"), poly_trait("Mul", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        mul.register_decl("__mul__", op_t, Public);
        mul.register_decl("MulO", Type, Public);
        let mut div = Self::poly_trait(
            "Div",
            params.clone(),
            vec![poly_trait("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(mono_q("Self"), r, mono_proj(mono_q("Self"), "DivO"));
        let self_bound = subtypeof(mono_q("Self"), poly_trait("Div", ty_params.clone()));
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        div.register_decl("__div__", op_t, Public);
        div.register_decl("DivO", Type, Public);
        self.register_type(trait_("Named"), named, Const);
        self.register_type(trait_("Mutable"), mutable, Const);
        self.register_type(trait_("Immutizable"), immutizable, Const);
        self.register_type(trait_("Mutizable"), mutizable, Const);
        self.register_type(poly_trait("In", vec![ty_tp(mono_q("T"))]), in_, Const);
        self.register_type(poly_trait("Eq", vec![ty_tp(mono_q("R"))]), eq, Const);
        self.register_type(
            poly_trait("PartialOrd", vec![ty_tp(mono_q("R"))]),
            partial_ord,
            Const,
        );
        self.register_type(trait_("Ord"), ord, Const);
        self.register_type(trait_("Num"), num, Const);
        self.register_type(poly_trait("Seq", vec![ty_tp(mono_q("T"))]), seq, Const);
        self.register_type(poly_trait("Input", vec![ty_tp(mono_q("T"))]), input, Const);
        self.register_type(
            poly_trait("Output", vec![ty_tp(mono_q("T"))]),
            output,
            Const,
        );
        self.register_type(poly_trait("Add", ty_params.clone()), add, Const);
        self.register_type(poly_trait("Sub", ty_params.clone()), sub, Const);
        self.register_type(poly_trait("Mul", ty_params.clone()), mul, Const);
        self.register_type(poly_trait("Div", ty_params), div, Const);
        self.register_const_param_defaults(
            "Eq",
            vec![ConstTemplate::Obj(ValueObj::t(mono_q("Self")))],
        );
        self.register_const_param_defaults(
            "PartialOrd",
            vec![ConstTemplate::app("Self", vec![], vec![])],
        );
        self.register_const_param_defaults(
            "Add",
            vec![ConstTemplate::Obj(ValueObj::t(mono_q("Self")))],
        );
        self.register_const_param_defaults(
            "Sub",
            vec![ConstTemplate::Obj(ValueObj::t(mono_q("Self")))],
        );
        self.register_const_param_defaults(
            "Mul",
            vec![ConstTemplate::Obj(ValueObj::t(mono_q("Self")))],
        );
        self.register_const_param_defaults(
            "Div",
            vec![ConstTemplate::Obj(ValueObj::t(mono_q("Self")))],
        );
    }

    fn init_builtin_classes(&mut self) {
        let mut obj = Self::mono_class("Obj", vec![], vec![], Self::TOP_LEVEL);
        let t = fn0_met(mono_q("Self"), mono_q("Self"));
        let t = quant(t, set! {subtypeof(mono_q("Self"), class("Obj"))});
        obj.register_impl("clone", t, Const, Public);
        obj.register_impl("__module__", Str, Const, Public);
        obj.register_impl("__sizeof__", fn0_met(Obj, Nat), Const, Public);
        obj.register_impl("__repr__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_impl("__str__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_impl("__dict__", fn0_met(Obj, dict(Str, Obj)), Immutable, Public);
        obj.register_impl("__bytes__", fn0_met(Obj, class("Bytes")), Immutable, Public);
        obj.register_const("MutType!", ValueObj::t(class("Obj!")));
        // let mut record = Self::mono_trait("Record", vec![Obj], Self::TOP_LEVEL);
        // let mut class = Self::mono_class("Class", vec![Type, Obj], Self::TOP_LEVEL);
        let mut int = Self::mono_class(
            "Int",
            vec![Obj],
            vec![
                trait_("Ord"),
                poly_trait("Eq", vec![ty_tp(Int)]),
                poly_trait("Add", vec![ty_tp(Int)]),
                poly_trait("Sub", vec![ty_tp(Int)]),
                poly_trait("Mul", vec![ty_tp(Int)]),
                poly_trait("Div", vec![ty_tp(Int)]),
                trait_("Num"),
                // trait_("Rational"),
                // trait_("Integral"),
                trait_("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        int.register_impl("abs", fn0_met(Int, Nat), Immutable, Public);
        // __div__ is not included in Int (cast to Float)
        let op_t = fn1_met(Int, Int, Int);
        int.register_impl("__add__", op_t.clone(), Const, Public);
        int.register_impl("__sub__", op_t.clone(), Const, Public);
        int.register_impl("__mul__", op_t, Const, Public);
        int.register_const("AddO", ValueObj::t(Int));
        int.register_const("SubO", ValueObj::t(Int));
        int.register_const("MulO", ValueObj::t(Int));
        int.register_const("DivO", ValueObj::t(Ratio));
        int.register_const("MutType!", ValueObj::t(class("Int!")));
        int.register_impl("Real", Int, Const, Public);
        int.register_impl("Imag", Int, Const, Public);
        let mut nat = Self::mono_class(
            "Nat",
            vec![Int, Obj],
            vec![
                trait_("Ord"),
                poly_trait("Eq", vec![ty_tp(Nat)]),
                poly_trait("Add", vec![ty_tp(Nat)]),
                poly_trait("Sub", vec![ty_tp(Nat)]),
                poly_trait("Mul", vec![ty_tp(Nat)]),
                poly_trait("Div", vec![ty_tp(Nat)]),
                trait_("Num"),
                // trait_("Rational"),
                // trait_("Integral"),
                trait_("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        // __sub__, __div__ is not included in Nat (cast to Int)
        let op_t = fn1_met(Nat, Nat, Nat);
        nat.register_impl("__add__", op_t.clone(), Const, Public);
        nat.register_impl("__mul__", op_t, Const, Public);
        nat.register_impl(
            "times!",
            pr_met(
                Nat,
                None,
                vec![param_t("p", nd_proc(vec![], NoneType))],
                vec![],
                NoneType,
            ),
            Immutable,
            Public,
        );
        nat.register_const("AddO", ValueObj::t(Nat));
        nat.register_const("SubO", ValueObj::t(Int));
        nat.register_const("MulO", ValueObj::t(Nat));
        nat.register_const("DivO", ValueObj::t(Ratio));
        nat.register_const("MutType!", ValueObj::t(class("Nat!")));
        nat.register_impl("Real", Nat, Const, Public);
        nat.register_impl("Imag", Nat, Const, Public);
        let mut float = Self::mono_class(
            "Float",
            vec![Obj],
            vec![
                trait_("Num"),
                // trait_("Eq"), // Float doesn't have an Eq implementation
                trait_("Ord"),
                poly_trait("Add", vec![ty_tp(Float)]),
                poly_trait("Sub", vec![ty_tp(Float)]),
                poly_trait("Mul", vec![ty_tp(Float)]),
                poly_trait("Div", vec![ty_tp(Float)]),
                trait_("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(Float, Float, Float);
        float.register_impl("__add__", op_t.clone(), Const, Public);
        float.register_impl("__sub__", op_t.clone(), Const, Public);
        float.register_impl("__mul__", op_t.clone(), Const, Public);
        float.register_impl("__div__", op_t, Const, Public);
        float.register_const("AddO", ValueObj::t(Float));
        float.register_const("SubO", ValueObj::t(Float));
        float.register_const("MulO", ValueObj::t(Float));
        float.register_const("DivO", ValueObj::t(Float));
        float.register_const("MutType!", ValueObj::t(class("Float!")));
        float.register_impl("Real", Float, Const, Public);
        float.register_impl("Imag", Float, Const, Public);
        let mut ratio = Self::mono_class(
            "Ratio",
            vec![Obj],
            vec![
                trait_("Num"),
                poly_trait("Eq", vec![ty_tp(Ratio)]),
                trait_("Ord"),
                poly_trait("Add", vec![ty_tp(Ratio)]),
                poly_trait("Sub", vec![ty_tp(Ratio)]),
                poly_trait("Mul", vec![ty_tp(Ratio)]),
                poly_trait("Div", vec![ty_tp(Ratio)]),
                trait_("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(Ratio, Ratio, Ratio);
        ratio.register_impl("__add__", op_t.clone(), Const, Public);
        ratio.register_impl("__sub__", op_t.clone(), Const, Public);
        ratio.register_impl("__mul__", op_t.clone(), Const, Public);
        ratio.register_impl("__div__", op_t, Const, Public);
        ratio.register_const("AddO", ValueObj::t(Ratio));
        ratio.register_const("SubO", ValueObj::t(Ratio));
        ratio.register_const("MulO", ValueObj::t(Ratio));
        ratio.register_const("DivO", ValueObj::t(Ratio));
        ratio.register_const("MutType!", ValueObj::t(class("Ratio!")));
        ratio.register_impl("Real", Ratio, Const, Public);
        ratio.register_impl("Imag", Ratio, Const, Public);
        let mut bool_ = Self::mono_class(
            "Bool",
            vec![Nat, Int, Obj],
            vec![
                trait_("Num"),
                // trait_("Rational"),
                // trait_("Integral"),
                poly_trait("Eq", vec![ty_tp(Bool)]),
                trait_("Ord"),
                // mono("SelfAdd"),
                // mono("SelfSub"),
                // mono("SelfMul"),
                // mono("SelfDiv"),
                trait_("Mutizable"),
            ],
            Self::TOP_LEVEL,
        );
        bool_.register_impl("__and__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_impl("__or__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_const("MutType!", ValueObj::t(class("Bool!")));
        let mut str_ = Self::mono_class(
            "Str",
            vec![Obj],
            vec![
                poly_trait("Eq", vec![ty_tp(Str)]),
                trait_("Ord"),
                trait_("Mutizable"),
                poly_trait("Seq", vec![ty_tp(Str)]),
                poly_trait("Add", vec![ty_tp(Str)]),
                poly_trait("Mul", vec![ty_tp(Nat)]),
            ],
            Self::TOP_LEVEL,
        );
        str_.register_impl("__add__", fn1_met(Str, Str, Str), Const, Public);
        str_.register_impl(
            "replace",
            fn_met(
                Str,
                vec![param_t("pat", Str), param_t("into", Str)],
                vec![],
                Str,
            ),
            Immutable,
            Public,
        );
        str_.register_const("AddO", ValueObj::t(Str));
        str_.register_const("MulO", ValueObj::t(Str));
        str_.register_const("MutType!", ValueObj::t(class("Str!")));
        let mut type_ = Self::mono_class(
            "Type",
            vec![Obj],
            vec![
                poly_trait("Eq", vec![ty_tp(Type)]),
                poly_trait("In", vec![ty_tp(Obj)]), // x in Type
                trait_("Named"),
            ],
            Self::TOP_LEVEL,
        );
        type_.register_impl("mro", array(Type, TyParam::erased(Nat)), Immutable, Public);
        let module = Self::mono_class(
            "Module",
            vec![Obj],
            vec![poly_trait("Eq", vec![ty_tp(Module)]), trait_("Named")],
            Self::TOP_LEVEL,
        );
        let mut array_ = Self::poly_class(
            "Array",
            vec![PS::t_nd("T"), PS::named_nd("N", Nat)],
            vec![Obj],
            vec![
                poly_trait(
                    "Eq",
                    vec![ty_tp(poly_class(
                        "Array",
                        vec![ty_tp(mono_q("T")), mono_q_tp("N")],
                    ))],
                ),
                trait_("Mutizable"),
                poly_trait("Seq", vec![ty_tp(mono_q("T"))]),
                poly_trait("Output", vec![ty_tp(mono_q("T"))]),
            ],
            Self::TOP_LEVEL,
        );
        let n = mono_q_tp("N");
        let m = mono_q_tp("M");
        let array_t = array(mono_q("T"), n.clone());
        let t = fn_met(
            array_t.clone(),
            vec![param_t("rhs", array(mono_q("T"), m.clone()))],
            vec![],
            array(mono_q("T"), n + m),
        );
        let t = quant(
            t,
            set! {static_instance("N", Nat), static_instance("M", Nat)},
        );
        array_.register_impl("concat", t, Immutable, Public);
        let n = mono_q_tp("N");
        let array_inner = mono_q("T");
        let array_t = array(array_inner.clone(), n.clone());
        let proj_t = mono_proj(array_inner.clone(), "ImmutType");
        let t = fn_met(
            array_t.clone(),
            vec![param_t("f", nd_func(vec![anon(proj_t.clone())], proj_t))],
            vec![],
            NoneType,
        );
        let t = quant(
            t,
            set! {static_instance("N", Nat), static_instance("T", trait_("Mutable"))},
        );
        array_.register_impl("map!", t, Immutable, Public);
        let mut_type = ValueObj::t(poly_class(
            "Array!",
            vec![TyParam::t(mono_q("T")), TyParam::mono_q("N").mutate()],
        ));
        // [T; N].MutType! = [T; !N] (neither [T!; N] nor [T; N]!)
        array_.register_const("MutType!", mut_type);
        let mut int_mut = Self::mono_class(
            "Int!",
            vec![Int, Obj],
            vec![trait_("Mutable")],
            Self::TOP_LEVEL,
        );
        // TODO: make Tuple6, Tuple7, ... etc.
        let tuple_ = Self::mono_class(
            "Tuple",
            vec![Obj],
            vec![poly_trait("Eq", vec![ty_tp(class("Tuple"))])],
            Self::TOP_LEVEL,
        );
        let tuple1 = Self::poly_class(
            "Tuple1",
            vec![PS::t_nd("A")],
            vec![class("Tuple"), Obj],
            vec![poly_trait(
                "Eq",
                vec![ty_tp(poly_class("Tuple1", vec![ty_tp(mono_q("A"))]))],
            )],
            Self::TOP_LEVEL,
        );
        let tuple2 = Self::poly_class(
            "Tuple2",
            vec![PS::t_nd("A"), PS::t_nd("B")],
            vec![class("Tuple"), Obj],
            vec![poly_trait(
                "Eq",
                vec![ty_tp(poly_class(
                    "Tuple2",
                    vec![ty_tp(mono_q("A")), ty_tp(mono_q("B"))],
                ))],
            )],
            Self::TOP_LEVEL,
        );
        let tuple3 = Self::poly_class(
            "Tuple3",
            vec![PS::t_nd("A"), PS::t_nd("B"), PS::t_nd("C")],
            vec![class("Tuple"), Obj],
            vec![poly_trait(
                "Eq",
                vec![ty_tp(poly_class(
                    "Tuple3",
                    vec![ty_tp(mono_q("A")), ty_tp(mono_q("B")), ty_tp(mono_q("C"))],
                ))],
            )],
            Self::TOP_LEVEL,
        );
        let tuple4 = Self::poly_class(
            "Tuple4",
            vec![PS::t_nd("A"), PS::t_nd("B"), PS::t_nd("C"), PS::t_nd("D")],
            vec![class("Tuple"), Obj],
            vec![poly_trait(
                "Eq",
                vec![ty_tp(poly_class(
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
            vec![class("Tuple"), Obj],
            vec![poly_trait(
                "Eq",
                vec![ty_tp(poly_class(
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
            vec![class("Tuple"), Obj],
            vec![poly_trait(
                "Eq",
                vec![ty_tp(poly_class(
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
            vec![class("Tuple"), Obj],
            vec![poly_trait(
                "Eq",
                vec![ty_tp(poly_class(
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
            vec![class("Tuple"), Obj],
            vec![poly_trait(
                "Eq",
                vec![ty_tp(poly_class(
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
        int_mut.register_const("ImmutType", ValueObj::t(Int));
        let f_t = param_t("f", func(vec![param_t("old", Int)], vec![], Int));
        let t = pr_met(class("Int!"), None, vec![f_t], vec![], NoneType);
        int_mut.register_impl("update!", t, Immutable, Public);
        let mut nat_mut = Self::mono_class(
            "Nat!",
            vec![Nat, Obj],
            vec![trait_("Mutable")],
            Self::TOP_LEVEL,
        );
        nat_mut.register_const("ImmutType", ValueObj::t(Nat));
        let f_t = param_t("f", func(vec![param_t("old", Nat)], vec![], Nat));
        let t = pr_met(class("Nat!"), None, vec![f_t], vec![], NoneType);
        nat_mut.register_impl("update!", t, Immutable, Public);
        let mut float_mut = Self::mono_class(
            "Float!",
            vec![Float, Obj],
            vec![trait_("Mutable")],
            Self::TOP_LEVEL,
        );
        float_mut.register_const("ImmutType", ValueObj::t(Float));
        let f_t = param_t("f", func(vec![param_t("old", Float)], vec![], Float));
        let t = pr_met(class("Float!"), None, vec![f_t], vec![], NoneType);
        float_mut.register_impl("update!", t, Immutable, Public);
        let mut ratio_mut = Self::mono_class(
            "Ratio!",
            vec![Ratio, Obj],
            vec![trait_("Mutable")],
            Self::TOP_LEVEL,
        );
        ratio_mut.register_const("ImmutType", ValueObj::t(Ratio));
        let f_t = param_t(
            "f",
            func(vec![param_t("old", class("Ratio"))], vec![], class("Ratio")),
        );
        let t = pr_met(class("Ratio!"), None, vec![f_t], vec![], NoneType);
        ratio_mut.register_impl("update!", t, Immutable, Public);
        let mut bool_mut = Self::mono_class(
            "Bool!",
            vec![Bool, Obj],
            vec![trait_("Mutable")],
            Self::TOP_LEVEL,
        );
        bool_mut.register_const("ImmutType", ValueObj::t(Bool));
        let f_t = param_t("f", func(vec![param_t("old", Bool)], vec![], Bool));
        let t = pr_met(class("Bool!"), None, vec![f_t], vec![], NoneType);
        bool_mut.register_impl("update!", t, Immutable, Public);
        let mut str_mut = Self::mono_class(
            "Str!",
            vec![Str, Obj],
            vec![trait_("Mutable")],
            Self::TOP_LEVEL,
        );
        str_mut.register_const("ImmutType", ValueObj::t(Str));
        let f_t = param_t("f", func(vec![param_t("old", Str)], vec![], Str));
        let t = pr_met(class("Str!"), None, vec![f_t], vec![], NoneType);
        str_mut.register_impl("update!", t, Immutable, Public);
        let array_mut_t = poly_class("Array!", vec![ty_tp(mono_q("T")), mono_q_tp("N")]);
        let mut array_mut = Self::poly_class(
            "Array!",
            vec![PS::t_nd("T"), PS::named_nd("N", class("Nat!"))],
            vec![
                poly_class("Range", vec![ty_tp(mono_q("T")), mono_q_tp("N")]),
                Obj,
            ],
            vec![
                trait_("Mutizable"),
                poly_trait("Seq", vec![ty_tp(mono_q("T"))]),
            ],
            Self::TOP_LEVEL,
        );
        let t = pr_met(
            ref_mut(array_mut_t.clone()),
            Some(ref_mut(poly_class(
                "Array!",
                vec![ty_tp(mono_q("T")), mono_q_tp("N") + value(1)],
            ))),
            vec![param_t("elem", mono_q("T"))],
            vec![],
            NoneType,
        );
        let t = quant(
            t,
            set! {static_instance("T", Type), static_instance("N", class("Nat!"))},
        );
        array_mut.register_impl("push!", t, Immutable, Public);
        let f_t = param_t(
            "f",
            func(
                vec![param_t("old", array_t.clone())],
                vec![],
                array_t.clone(),
            ),
        );
        let t = pr_met(array_mut_t.clone(), None, vec![f_t], vec![], NoneType);
        array_mut.register_impl("update!", t, Immutable, Public);
        let range_t = poly_class("Range", vec![TyParam::t(mono_q("T"))]);
        let range = Self::poly_class(
            "Range",
            vec![PS::t_nd("T")],
            vec![Obj],
            vec![
                poly_trait(
                    "Eq",
                    vec![ty_tp(poly_class("Range", vec![ty_tp(mono_q("T"))]))],
                ),
                trait_("Mutizable"),
                poly_trait("Seq", vec![ty_tp(mono_q("T"))]),
                poly_trait("Output", vec![ty_tp(mono_q("T"))]),
            ],
            Self::TOP_LEVEL,
        );
        let func = Self::mono_class(
            "Function",
            vec![Obj],
            vec![trait_("Named")],
            Self::TOP_LEVEL,
        );
        let qfunc = Self::mono_class(
            "QuantifiedFunction",
            vec![class("Function"), Obj],
            vec![],
            Self::TOP_LEVEL,
        );
        self.register_type(Obj, obj, Const);
        // self.register_type(mono("Record"), vec![], record, Const);
        // self.register_type(mono("Class"), vec![], class, Const);
        self.register_type(Int, int, Const);
        self.register_type(Nat, nat, Const);
        self.register_type(Float, float, Const);
        self.register_type(Ratio, ratio, Const);
        self.register_type(Bool, bool_, Const);
        self.register_type(Str, str_, Const);
        self.register_type(Type, type_, Const);
        self.register_type(Module, module, Const);
        self.register_type(array_t, array_, Const);
        self.register_type(tuple(vec![mono_q("A")]), tuple1, Const);
        self.register_type(tuple(vec![mono_q("A"), mono_q("B")]), tuple2, Const);
        self.register_type(
            tuple(vec![mono_q("A"), mono_q("B"), mono_q("C")]),
            tuple3,
            Const,
        );
        self.register_type(
            tuple(vec![mono_q("A"), mono_q("B"), mono_q("C"), mono_q("D")]),
            tuple4,
            Const,
        );
        self.register_type(
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
        self.register_type(
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
        self.register_type(
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
        self.register_type(
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
        self.register_type(class("Int!"), int_mut, Const);
        self.register_type(class("Nat!"), nat_mut, Const);
        self.register_type(class("Float!"), float_mut, Const);
        self.register_type(class("Ratio!"), ratio_mut, Const);
        self.register_type(class("Bool!"), bool_mut, Const);
        self.register_type(class("Str!"), str_mut, Const);
        self.register_type(array_mut_t, array_mut, Const);
        self.register_type(range_t, range, Const);
        self.register_type(class("Tuple"), tuple_, Const);
        self.register_type(class("Function"), func, Const);
        self.register_type(class("QuantifiedFunction"), qfunc, Const);
    }

    fn init_builtin_funcs(&mut self) {
        let t_abs = nd_func(vec![param_t("n", trait_("Num"))], Nat);
        let t_assert = func(
            vec![param_t("condition", Bool)],
            vec![param_t("err_message", Str)],
            NoneType,
        );
        let t_classof = nd_func(vec![param_t("old", Obj)], option(Class));
        let t_compile = nd_func(vec![param_t("src", Str)], Code);
        let t_cond = nd_func(
            vec![
                param_t("condition", Bool),
                param_t("then", mono_q("T")),
                param_t("else", mono_q("T")),
            ],
            mono_q("T"),
        );
        let t_cond = quant(t_cond, set! {static_instance("T", Type)});
        let t_discard = nd_func(vec![param_t("old", Obj)], NoneType);
        let t_id = nd_func(vec![param_t("old", Obj)], Nat);
        // FIXME: quantify
        let t_if = func(
            vec![
                param_t("cond", Bool),
                param_t("then", nd_func(vec![], mono_q("T"))),
            ],
            vec![param_t("else", nd_func(vec![], mono_q("T")))],
            option(mono_q("T")),
        );
        let t_if = quant(t_if, set! {static_instance("T", Type)});
        let t_import = nd_func(vec![param_t("path", Str)], Module);
        let t_log = nd_func(vec![param_t("objs", var_args(Obj))], NoneType);
        let t_pyimport = nd_func(vec![param_t("path", Str)], Module);
        let t_quit = func(vec![], vec![param_t("code", Int)], NoneType);
        self.register_impl("abs", t_abs, Const, Private);
        self.register_impl("assert", t_assert, Const, Private);
        self.register_impl("classof", t_classof, Const, Private);
        self.register_impl("compile", t_compile, Const, Private);
        self.register_impl("cond", t_cond, Const, Private);
        self.register_impl("discard", t_discard, Const, Private);
        self.register_impl("id", t_id, Const, Private);
        self.register_impl("if", t_if, Const, Private);
        self.register_impl("log", t_log, Const, Private);
        self.register_impl("import", t_import, Const, Private);
        if cfg!(feature = "debug") {
            self.register_impl("py", t_pyimport.clone(), Const, Private);
        }
        self.register_impl("pyimport", t_pyimport, Const, Private);
        self.register_impl("quit", t_quit, Const, Private);
    }

    fn init_builtin_procs(&mut self) {
        let t_print = proc(
            vec![param_t("objects", var_args(ref_(Obj)))],
            vec![
                param_t("sep", Str),
                param_t("end", Str),
                param_t("file", class("Write")),
                param_t("flush", Bool),
            ],
            NoneType,
        );
        let t_input = nd_proc(vec![param_t("msg", Str)], Str);
        let t_if = proc(
            vec![
                param_t("cond", Bool),
                param_t("then", nd_proc(vec![], mono_q("T"))),
            ],
            vec![param_t("else", nd_proc(vec![], mono_q("T")))],
            option(mono_q("T")),
        );
        let t_if = quant(t_if, set! {static_instance("T", Type)});
        let t_for = nd_proc(
            vec![
                param_t("iter", iter(mono_q("T"))),
                param_t("p", nd_proc(vec![anon(mono_q("T"))], NoneType)),
            ],
            NoneType,
        );
        let t_for = quant(t_for, set! {static_instance("T", Type)});
        let t_while = nd_proc(
            vec![
                param_t("cond", class("Bool!")),
                param_t("p", nd_proc(vec![], NoneType)),
            ],
            NoneType,
        );
        self.register_impl("print!", t_print, Const, Private);
        self.register_impl("input!", t_input, Const, Private);
        self.register_impl("if!", t_if, Const, Private);
        self.register_impl("for!", t_for, Const, Private);
        self.register_impl("while!", t_while, Const, Private);
    }

    fn init_builtin_operators(&mut self) {
        /* binary */
        let l = mono_q("L");
        let r = mono_q("R");
        let params = vec![ty_tp(mono_q("R"))];
        let op_t = nd_func(
            vec![param_t("lhs", l.clone()), param_t("rhs", r.clone())],
            mono_proj(mono_q("L"), "AddO"),
        );
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly_trait("Add", params.clone()))
            },
        );
        self.register_impl("__add__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "SubO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly_trait("Sub", params.clone()))
            },
        );
        self.register_impl("__sub__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "MulO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly_trait("Mul", params.clone()))
            },
        );
        self.register_impl("__mul__", op_t, Const, Private);
        let op_t = bin_op(l.clone(), r.clone(), mono_proj(mono_q("L"), "DivO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l, poly_trait("Mul", params.clone()))
            },
        );
        self.register_impl("__div__", op_t, Const, Private);
        let m = mono_q("M");
        let op_t = bin_op(m.clone(), m.clone(), m.clone());
        let op_t = quant(op_t, set! {subtypeof(m, poly_trait("Mul", vec![]))});
        // TODO: add bound: M == MulO
        self.register_impl("__pow__", op_t, Const, Private);
        let d = mono_q("D");
        let op_t = bin_op(d.clone(), d.clone(), d.clone());
        let op_t = quant(op_t, set! {subtypeof(d, poly_trait("Div", vec![]))});
        self.register_impl("__mod__", op_t, Const, Private);
        let e = mono_q("E");
        let op_t = bin_op(e.clone(), e.clone(), Bool);
        let op_t = quant(op_t, set! {subtypeof(e, poly_trait("Eq", vec![]))});
        self.register_impl("__eq__", op_t.clone(), Const, Private);
        self.register_impl("__ne__", op_t, Const, Private);
        let o = mono_q("O");
        let op_t = bin_op(o.clone(), o.clone(), Bool);
        let op_t = quant(op_t, set! {subtypeof(o, trait_("Ord"))});
        self.register_impl("__lt__", op_t.clone(), Const, Private);
        self.register_impl("__le__", op_t.clone(), Const, Private);
        self.register_impl("__gt__", op_t.clone(), Const, Private);
        self.register_impl("__ge__", op_t, Const, Private);
        self.register_impl("__and__", bin_op(Bool, Bool, Bool), Const, Private);
        self.register_impl("__or__", bin_op(Bool, Bool, Bool), Const, Private);
        let t = mono_q("T");
        let op_t = bin_op(t.clone(), t.clone(), range(t.clone()));
        let op_t = quant(op_t, set! {subtypeof(t.clone(), trait_("Ord"))});
        self.register_decl("__rng__", op_t.clone(), Private);
        self.register_decl("__lorng__", op_t.clone(), Private);
        self.register_decl("__rorng__", op_t.clone(), Private);
        self.register_decl("__orng__", op_t, Private);
        // TODO: use existential type: |T: Type| (T, In(T)) -> Bool
        let op_t = bin_op(mono_q("T"), mono_q("I"), Bool);
        let op_t = quant(
            op_t,
            set! { static_instance("T", Type), subtypeof(mono_q("I"), poly_trait("In", vec![ty_tp(mono_q("T"))])) },
        );
        self.register_impl("__in__", op_t, Const, Private);
        /* unary */
        // TODO: Boolの+/-は警告を出したい
        let op_t = func1(mono_q("T"), mono_proj(mono_q("T"), "MutType!"));
        let op_t = quant(op_t, set! {subtypeof(mono_q("T"), trait_("Mutizable"))});
        self.register_impl("__mutate__", op_t, Const, Private);
        let n = mono_q("N");
        let op_t = func1(n.clone(), n.clone());
        let op_t = quant(op_t, set! {subtypeof(n, trait_("Num"))});
        self.register_decl("__pos__", op_t.clone(), Private);
        self.register_decl("__neg__", op_t, Private);
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
                poly_trait("Add", vec![TyParam::from(&o..=&p)]),
                poly_trait("Sub", vec![TyParam::from(&o..=&p)]),
            ],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(
            Type::from(&m..=&n),
            Type::from(&o..=&p),
            Type::from(m.clone() + o.clone()..=n.clone() + p.clone()),
        );
        interval.register_impl("__add__", op_t, Const, Public);
        let op_t = fn1_met(
            Type::from(&m..=&n),
            Type::from(&o..=&p),
            Type::from(m.clone() - p.clone()..=n.clone() - o.clone()),
        );
        interval.register_impl("__sub__", op_t, Const, Public);
        interval.register_const(
            "AddO",
            ValueObj::t(Type::from(m.clone() + o.clone()..=n.clone() + p.clone())),
        );
        interval.register_const("SubO", ValueObj::t(Type::from(m - p..=n - o)));
        self.register_patch("Interval", interval, Const);
        // eq.register_impl("__ne__", op_t,         Const, Public);
        // ord.register_impl("__le__", op_t.clone(), Const, Public);
        // ord.register_impl("__gt__", op_t.clone(), Const, Public);
        // ord.register_impl("__ge__", op_t,         Const, Public);
    }

    pub(crate) fn init_py_math_mod() -> Self {
        let mut math = Context::module("math".into(), 10);
        math.register_impl("pi", Float, Immutable, Public);
        math.register_impl("tau", Float, Immutable, Public);
        math.register_impl("e", Float, Immutable, Public);
        math.register_impl("sin", func1(Float, Float), Immutable, Public);
        math.register_impl("cos", func1(Float, Float), Immutable, Public);
        math.register_impl("tan", func1(Float, Float), Immutable, Public);
        math
    }

    pub(crate) fn init_py_random_mod() -> Self {
        let mut random = Context::module("random".into(), 10);
        random.register_impl(
            "seed!",
            proc(
                vec![],
                vec![
                    param_t("a", trait_("Num")), // TODO: NoneType, int, float, str, bytes, bytearray
                    param_t("version", Int),
                ],
                NoneType,
            ),
            Immutable,
            Public,
        );
        random.register_impl(
            "randint!",
            nd_proc(vec![param_t("a", Int), param_t("b", Int)], Int),
            Immutable,
            Public,
        );
        let t = nd_proc(
            vec![param_t("seq", poly_trait("Seq", vec![ty_tp(mono_q("T"))]))],
            mono_q("T"),
        );
        let t = quant(t, set! {static_instance("T", Type)});
        random.register_impl("choice!", t, Immutable, Public);
        random
    }

    pub(crate) fn init_builtins() -> Self {
        // TODO: capacityを正確に把握する
        let mut ctx = Context::module("<builtins>".into(), 40);
        ctx.init_builtin_funcs();
        ctx.init_builtin_procs();
        ctx.init_builtin_operators();
        ctx.init_builtin_traits();
        ctx.init_builtin_classes();
        ctx.init_builtin_patches();
        ctx
    }

    pub fn new_root_module() -> Self {
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
