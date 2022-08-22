//! defines type information for builtin objects (in `Context`)
//!
//! 組み込みオブジェクトの型情報を(Contextに)定義
use erg_common::ty::type_constrs::*;
use erg_common::ty::{TyParam, Type};
use erg_common::value::{ValueObj, Visibility};
use erg_common::Str;
use erg_common::{debug_power_assert, set};
use ParamSpec as PS;
use Type::*;

use erg_parser::ast::VarName;

use crate::context::{ConstTemplate, Context, ContextKind, DefaultInfo, ParamSpec};
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
            self.consts.insert(Str::ever(name), obj);
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
        if self.types.contains_key(&t) {
            panic!("{} has already been registered", t.name());
        } else {
            let name = VarName::from_str(Str::rc(t.name()));
            self.locals
                .insert(name, VarInfo::new(Type, muty, Private, Builtin));
            for impl_trait in ctx.super_traits.iter() {
                if !impl_trait.is_monomorphic() {
                    if let Some(impls) = self.poly_trait_impls.get_mut(impl_trait.name()) {
                        impls.push((t.clone(), impl_trait.clone()));
                    } else {
                        self.poly_trait_impls.insert(
                            Str::rc(impl_trait.name()),
                            vec![(t.clone(), impl_trait.clone())],
                        );
                    }
                }
            }
            self.types.insert(t, ctx);
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
            debug_power_assert!(ctx.super_classes.len(), ==, 1);
            if let Some(target_type) = ctx.super_classes.first() {
                for impl_trait in ctx.super_traits.iter() {
                    self.glue_patch_and_types.push((
                        VarName::from_str(ctx.name.clone()),
                        target_type.clone(),
                        impl_trait.clone(),
                    ));
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
        // Erg does not have a trait equivalent to `PartialEq` in Rust
        // This means, Erg's `Float` cannot be compared with other `Float`
        // use `l - r < EPSILON` to check if two floats are almost equal
        let mut eq = Self::poly_trait("Eq", vec![PS::t("R", WithDefault)], vec![], Self::TOP_LEVEL);
        // __eq__: |Self <: Eq()| Self.(Self) -> Bool
        let op_t = fn1_met(mono_q("Self"), mono_q("R"), Bool);
        let op_t = quant(
            op_t,
            set! {
                subtypeof(mono_q("Self"), poly("Eq", vec![ty_tp(mono_q("R"))])),
                static_instance("R", Type)
            },
        );
        eq.register_decl("__eq__", op_t.clone(), Public);
        let mut partial_ord = Self::poly_trait(
            "PartialOrd",
            vec![PS::t("R", WithDefault)],
            vec![poly("PartialEq", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(poly("Self", vec![ty_tp(mono_q("R"))]), mono_q("R"), Bool);
        let op_t = quant(
            op_t,
            set! {
                subtypeof(mono_q("Self"), poly("PartialOrd", vec![ty_tp(mono_q("R"))])),
                static_instance("R", Type)
            },
        );
        partial_ord.register_decl("__lt__", op_t.clone(), Public);
        let ord = Self::mono_trait(
            "Ord",
            vec![poly("Eq", vec![]), poly("PartialOrd", vec![])],
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
        seq.register_decl("__len__", t, Public);
        let t = Type::fn1_met(self_t.clone(), Nat, mono_q("T"));
        let t = quant(
            t,
            set! {subtypeof(self_t, poly("Seq", vec![ty_tp(mono_q("T"))])), static_instance("T", Type)},
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
            vec![poly("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let self_bound = subtypeof(mono_q("Self"), poly("Add", ty_params.clone()));
        let op_t = fn1_met(
            poly_q("Self", ty_params.clone()),
            r.clone(),
            mono_proj(mono_q("Self"), "AddO"),
        );
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        add.register_decl("__add__", op_t, Public);
        add.register_decl("AddO", Type, Public);
        let mut sub = Self::poly_trait(
            "Sub",
            params.clone(),
            vec![poly("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let self_bound = subtypeof(mono_q("Self"), poly("Sub", ty_params.clone()));
        let op_t = fn1_met(
            poly_q("Self", ty_params.clone()),
            r.clone(),
            mono_proj(mono_q("Self"), "SubO"),
        );
        let op_t = quant(op_t, set! {r_bound.clone(), self_bound});
        sub.register_decl("__sub__", op_t, Public);
        sub.register_decl("SubO", Type, Public);
        let mut mul = Self::poly_trait(
            "Mul",
            params.clone(),
            vec![poly("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(
            poly("Mul", ty_params.clone()),
            r.clone(),
            mono_proj(mono_q("Self"), "MulO"),
        );
        mul.register_decl("__mul__", op_t, Public);
        mul.register_decl("MulO", Type, Public);
        let mut div = Self::poly_trait(
            "Div",
            params.clone(),
            vec![poly("Output", vec![ty_tp(mono_q("R"))])],
            Self::TOP_LEVEL,
        );
        let op_t = fn1_met(
            poly("Div", ty_params.clone()),
            r,
            mono_proj(mono_q("Self"), "DivO"),
        );
        div.register_decl("__div__", op_t, Public);
        div.register_decl("DivO", Type, Public);
        self.register_type(mono("Named"), named, Const);
        self.register_type(poly("Eq", vec![ty_tp(mono_q("R"))]), eq, Const);
        self.register_type(
            poly("PartialOrd", vec![ty_tp(mono_q("R"))]),
            partial_ord,
            Const,
        );
        self.register_type(mono("Ord"), ord, Const);
        self.register_type(poly("Seq", vec![ty_tp(mono_q("T"))]), seq, Const);
        self.register_type(poly("Input", vec![ty_tp(mono_q("T"))]), input, Const);
        self.register_type(poly("Output", vec![ty_tp(mono_q("T"))]), output, Const);
        self.register_type(poly("Add", ty_params.clone()), add, Const);
        self.register_type(poly("Sub", ty_params.clone()), sub, Const);
        self.register_type(poly("Mul", ty_params.clone()), mul, Const);
        self.register_type(poly("Div", ty_params), div, Const);
        // self.register_type(mono("Num"), num, Const);
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
        let t = quant(t, set! {subtypeof(mono_q("Self"), mono("Obj"))});
        obj.register_impl("clone", t, Const, Public);
        obj.register_impl("__module__", Str, Const, Public);
        obj.register_impl("__sizeof__", fn0_met(Obj, Nat), Const, Public);
        obj.register_impl("__repr__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_impl("__str__", fn0_met(Obj, Str), Immutable, Public);
        obj.register_impl(
            "__dict__",
            fn0_met(Obj, Type::dict(Str, Obj)),
            Immutable,
            Public,
        );
        obj.register_impl(
            "__bytes__",
            fn0_met(Obj, Type::mono("Bytes")),
            Immutable,
            Public,
        );
        obj.register_const("MutType!", ValueObj::t(mono("Obj!")));
        // let mut record = Self::mono_trait("Record", vec![Obj], Self::TOP_LEVEL);
        // let mut class = Self::mono_class("Class", vec![Type, Obj], Self::TOP_LEVEL);
        let mut int = Self::mono_class(
            "Int",
            vec![Obj],
            vec![
                mono("Ord"),
                poly("Eq", vec![ty_tp(Int)]),
                poly("Add", vec![ty_tp(Int)]),
                poly("Sub", vec![ty_tp(Int)]),
                poly("Mul", vec![ty_tp(Int)]),
                poly("Div", vec![ty_tp(Int)]),
                mono("Num"),
                mono("Rational"),
                mono("Integral"),
                mono("Mutate"),
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
        int.register_const("MutType!", ValueObj::t(mono("Int!")));
        int.register_impl("Real", Int, Const, Public);
        int.register_impl("Imag", Int, Const, Public);
        let mut nat = Self::mono_class(
            "Nat",
            vec![Int, Obj],
            vec![
                mono("Ord"),
                poly("Eq", vec![ty_tp(Nat)]),
                poly("Add", vec![ty_tp(Nat)]),
                poly("Sub", vec![ty_tp(Nat)]),
                poly("Mul", vec![ty_tp(Nat)]),
                poly("Div", vec![ty_tp(Nat)]),
                mono("Num"),
                mono("Rational"),
                mono("Integral"),
                mono("Mutate"),
            ],
            Self::TOP_LEVEL,
        );
        // __sub__, __div__ is not included in Nat (cast to Int)
        let op_t = fn1_met(Nat, Nat, Nat);
        nat.register_impl("__add__", op_t.clone(), Const, Public);
        nat.register_impl("__mul__", op_t, Const, Public);
        nat.register_impl(
            "times!",
            Type::pr_met(
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
        nat.register_const("MutType!", ValueObj::t(mono("Nat!")));
        nat.register_impl("Real", Nat, Const, Public);
        nat.register_impl("Imag", Nat, Const, Public);
        let mut float = Self::mono_class(
            "Float",
            vec![Obj],
            vec![
                mono("Num"),
                // mono("Eq"), // Float doesn't have an Eq implementation
                mono("Ord"),
                poly("Add", vec![ty_tp(Float)]),
                poly("Sub", vec![ty_tp(Float)]),
                poly("Mul", vec![ty_tp(Float)]),
                poly("Div", vec![ty_tp(Float)]),
                mono("Mutate"),
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
        float.register_const("MutType!", ValueObj::t(mono("Float!")));
        float.register_impl("Real", Float, Const, Public);
        float.register_impl("Imag", Float, Const, Public);
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
                mono("Mutate"),
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
        ratio.register_const("MutType!", ValueObj::t(mono("Ratio!")));
        ratio.register_impl("Real", Ratio, Const, Public);
        ratio.register_impl("Imag", Ratio, Const, Public);
        let mut bool_ = Self::mono_class(
            "Bool",
            vec![Nat, Int, Obj],
            vec![
                mono("Num"),
                mono("Rational"),
                mono("Integral"),
                poly("Eq", vec![ty_tp(Bool)]),
                mono("Ord"),
                // mono("SelfAdd"),
                // mono("SelfSub"),
                // mono("SelfMul"),
                // mono("SelfDiv"),
                mono("Mutate"),
            ],
            Self::TOP_LEVEL,
        );
        bool_.register_impl("__and__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_impl("__or__", fn1_met(Bool, Bool, Bool), Const, Public);
        bool_.register_const("MutType!", ValueObj::t(mono("Bool!")));
        let mut str_ = Self::mono_class(
            "Str",
            vec![Obj],
            vec![
                poly("Eq", vec![ty_tp(Str)]),
                mono("Ord"),
                mono("Mutate"),
                poly("Seq", vec![ty_tp(Str)]),
                poly("Add", vec![ty_tp(Str)]),
                poly("Mul", vec![ty_tp(Nat)]),
            ],
            Self::TOP_LEVEL,
        );
        str_.register_impl("__add__", fn1_met(Str, Str, Str), Const, Public);
        str_.register_impl(
            "replace",
            Type::fn_met(
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
        str_.register_const("MutType!", ValueObj::t(mono("Str!")));
        let mut type_ = Self::mono_class(
            "Type",
            vec![Obj],
            vec![poly("Eq", vec![ty_tp(Type)]), mono("Named")],
            Self::TOP_LEVEL,
        );
        type_.register_impl(
            "mro",
            Type::array(Type, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        let module = Self::mono_class(
            "Module",
            vec![Obj],
            vec![poly("Eq", vec![ty_tp(Module)]), mono("Named")],
            Self::TOP_LEVEL,
        );
        let mut array = Self::poly_class(
            "Array",
            vec![PS::t_nd("T"), PS::named_nd("N", Nat)],
            vec![Obj],
            vec![
                poly(
                    "Eq",
                    vec![ty_tp(Type::poly(
                        "Array",
                        vec![ty_tp(mono_q("T")), mono_q_tp("N")],
                    ))],
                ),
                mono("Mutate"),
                poly("Seq", vec![ty_tp(mono_q("T"))]),
                poly("Output", vec![ty_tp(mono_q("T"))]),
            ],
            Self::TOP_LEVEL,
        );
        let n = mono_q_tp("N");
        let m = mono_q_tp("M");
        let array_t = Type::array(mono_q("T"), n.clone());
        let t = Type::fn_met(
            array_t.clone(),
            vec![param_t("rhs", Type::array(mono_q("T"), m.clone()))],
            vec![],
            Type::array(mono_q("T"), n + m),
        );
        let t = quant(
            t,
            set! {static_instance("N", Nat), static_instance("M", Nat)},
        );
        array.register_impl("concat", t, Immutable, Public);
        let n = mono_q_tp("N");
        let array_inner = mono_q("T");
        let array_t = Type::array(array_inner.clone(), n.clone());
        let proj_t = mono_proj(array_inner.clone(), "ImmutType");
        let t = Type::fn_met(
            array_t.clone(),
            vec![param_t(
                "f",
                Type::nd_func(vec![anon(proj_t.clone())], proj_t),
            )],
            vec![],
            Type::NoneType,
        );
        let t = quant(
            t,
            set! {static_instance("N", Nat), static_instance("T", Type::mono("Mutable"))},
        );
        array.register_impl("map!", t, Immutable, Public);
        let mut_type = ValueObj::t(Type::poly(
            "Array!",
            vec![TyParam::t(mono_q("T")), TyParam::mono_q("N").mutate()],
        ));
        let mut int_mut = Self::mono_class(
            "Int!",
            vec![Int, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        int_mut.register_const("ImmutType", ValueObj::t(Int));
        let mut nat_mut = Self::mono_class(
            "Int!",
            vec![Nat, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        nat_mut.register_const("ImmutType", ValueObj::t(Nat));
        let mut float_mut = Self::mono_class(
            "Float!",
            vec![Float, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        float_mut.register_const("ImmutType", ValueObj::t(Float));
        let mut ratio_mut = Self::mono_class(
            "Ratio!",
            vec![Ratio, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        ratio_mut.register_const("ImmutType", ValueObj::t(Ratio));
        let mut bool_mut = Self::mono_class(
            "Bool!",
            vec![Bool, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        bool_mut.register_const("ImmutType", ValueObj::t(Bool));
        let mut str_mut = Self::mono_class(
            "Str!",
            vec![Str, Obj],
            vec![mono("Mutable")],
            Self::TOP_LEVEL,
        );
        str_mut.register_const("ImmutType", ValueObj::t(Str));
        // [T; N].MutType! = [T; !N] (neither [T!; N] nor [T; N]!)
        array.register_const("MutType!", mut_type);
        let array_mut_t = Type::poly("Array!", vec![ty_tp(mono_q("T")), mono_q_tp("N")]);
        let mut array_mut = Self::poly_class(
            "Array!",
            vec![PS::t_nd("T"), PS::named_nd("N", mono("Nat!"))],
            vec![poly("Range", vec![ty_tp(mono_q("T")), mono_q_tp("N")]), Obj],
            vec![mono("Mutate"), poly("Seq", vec![ty_tp(mono_q("T"))])],
            Self::TOP_LEVEL,
        );
        let t = Type::pr_met(
            Type::ref_mut(array_mut_t.clone()),
            Some(Type::ref_mut(poly(
                "Array!",
                vec![ty_tp(mono_q("T")), mono_q_tp("N") + value(1)],
            ))),
            vec![param_t("elem", mono_q("T"))],
            vec![],
            Type::NoneType,
        );
        let t = quant(
            t,
            set! {static_instance("T", Type), static_instance("N", mono("Nat!"))},
        );
        array_mut.register_impl("push!", t, Immutable, Public);
        let range_t = Type::poly("Range", vec![TyParam::t(mono_q("T"))]);
        let range = Self::poly_class(
            "Range",
            vec![PS::t_nd("T")],
            vec![Obj],
            vec![
                poly(
                    "Eq",
                    vec![ty_tp(Type::poly("Range", vec![ty_tp(mono_q("T"))]))],
                ),
                mono("Mutate"),
                poly("Seq", vec![ty_tp(mono_q("T"))]),
                poly("Output", vec![ty_tp(mono_q("T"))]),
            ],
            Self::TOP_LEVEL,
        );
        self.register_type(Obj, obj, Const);
        // self.register_type(Type::mono("Record"), vec![], record, Const);
        // self.register_type(Type::mono("Class"), vec![], class, Const);
        self.register_type(Int, int, Const);
        self.register_type(Nat, nat, Const);
        self.register_type(Float, float, Const);
        self.register_type(Ratio, ratio, Const);
        self.register_type(Bool, bool_, Const);
        self.register_type(Str, str_, Const);
        self.register_type(Type, type_, Const);
        self.register_type(Module, module, Const);
        self.register_type(array_t, array, Const);
        self.register_type(mono("Int!"), int_mut, Const);
        self.register_type(mono("Nat!"), nat_mut, Const);
        self.register_type(mono("Float!"), float_mut, Const);
        self.register_type(mono("Ratio!"), ratio_mut, Const);
        self.register_type(mono("Bool!"), bool_mut, Const);
        self.register_type(mono("Str!"), str_mut, Const);
        self.register_type(array_mut_t, array_mut, Const);
        self.register_type(range_t, range, Const);
    }

    fn init_builtin_funcs(&mut self) {
        let t_abs = nd_func(vec![param_t("n", mono("Num"))], Nat);
        let t_assert = func(
            vec![param_t("condition", Bool)],
            vec![param_t("err_message", Str)],
            NoneType,
        );
        let t_classof = nd_func(vec![param_t("o", Obj)], Type::option(Class));
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
        let t_discard = nd_func(vec![param_t("o", Obj)], NoneType);
        let t_id = nd_func(vec![param_t("o", Obj)], Nat);
        // FIXME: quantify
        let t_if = func(
            vec![
                param_t("cond", Bool),
                param_t("then", nd_func(vec![], mono_q("T"))),
            ],
            vec![param_t("else", nd_func(vec![], mono_q("T")))],
            Type::option(mono_q("T")),
        );
        let t_if = quant(t_if, set! {static_instance("T", Type)});
        let t_import = nd_func(vec![param_t("path", Str)], Module);
        let t_log = nd_func(vec![param_t("objs", Type::var_args(Obj))], NoneType);
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
            vec![param_t("objects", Type::var_args(Type::ref_(Obj)))],
            vec![
                param_t("sep", Str),
                param_t("end", Str),
                param_t("file", mono("Write")),
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
            Type::option(mono_q("T")),
        );
        let t_if = quant(t_if, set! {static_instance("T", Type)});
        let t_for = nd_proc(
            vec![
                param_t("iter", Type::iter(mono_q("T"))),
                param_t("p", nd_proc(vec![anon(mono_q("T"))], NoneType)),
            ],
            NoneType,
        );
        let t_for = quant(t_for, set! {static_instance("T", Type)});
        let t_while = nd_proc(
            vec![
                param_t("cond", mono("Bool!")),
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
        let op_t = Type::func2(l.clone(), r.clone(), mono_proj(mono_q("L"), "AddO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly("Add", params.clone()))
            },
        );
        self.register_impl("__add__", op_t, Const, Private);
        let op_t = Type::func2(l.clone(), r.clone(), mono_proj(mono_q("L"), "SubO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly("Sub", params.clone()))
            },
        );
        self.register_impl("__sub__", op_t, Const, Private);
        let op_t = Type::func2(l.clone(), r.clone(), mono_proj(mono_q("L"), "MulO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l.clone(), poly("Mul", params.clone()))
            },
        );
        self.register_impl("__mul__", op_t, Const, Private);
        let op_t = Type::func2(l.clone(), r.clone(), mono_proj(mono_q("L"), "DivO"));
        let op_t = quant(
            op_t,
            set! {
                static_instance("R", Type),
                subtypeof(l, poly("Mul", params.clone()))
            },
        );
        self.register_impl("__div__", op_t, Const, Private);
        let m = mono_q("M");
        let op_t = Type::func2(m.clone(), m.clone(), m.clone());
        let op_t = quant(op_t, set! {subtypeof(m, poly("Mul", vec![]))});
        // TODO: add bound: M == MulO
        self.register_impl("__pow__", op_t, Const, Private);
        let d = mono_q("D");
        let op_t = Type::func2(d.clone(), d.clone(), d.clone());
        let op_t = quant(op_t, set! {subtypeof(d, poly("Div", vec![]))});
        self.register_impl("__mod__", op_t, Const, Private);
        let e = mono_q("E");
        let op_t = Type::func2(e.clone(), e.clone(), Bool);
        let op_t = quant(op_t, set! {subtypeof(e, poly("Eq", vec![]))});
        self.register_impl("__eq__", op_t.clone(), Const, Private);
        self.register_impl("__ne__", op_t, Const, Private);
        let o = mono_q("O");
        let op_t = Type::func2(o.clone(), o.clone(), Bool);
        let op_t = quant(op_t, set! {subtypeof(o, mono("Ord"))});
        self.register_impl("__lt__", op_t.clone(), Const, Private);
        self.register_impl("__le__", op_t.clone(), Const, Private);
        self.register_impl("__gt__", op_t.clone(), Const, Private);
        self.register_impl("__ge__", op_t, Const, Private);
        self.register_impl("__and__", Type::func2(Bool, Bool, Bool), Const, Private);
        self.register_impl("__or__", Type::func2(Bool, Bool, Bool), Const, Private);
        /* unary */
        // TODO: Boolの+/-は警告を出したい
        let n = mono_q("N");
        let op_t = fn0_met(n.clone(), n.clone());
        let op_t = quant(op_t, set! {subtypeof(n, mono("Num"))});
        self.register_decl("__pos__", op_t.clone(), Private);
        self.register_decl("__neg__", op_t, Private);
        let t = mono_q("T");
        let op_t = Type::func2(t.clone(), t.clone(), Type::range(t.clone()));
        let op_t = quant(op_t, set! {subtypeof(t.clone(), mono("Ord"))});
        self.register_decl("__rng__", op_t.clone(), Private);
        self.register_decl("__lorng__", op_t.clone(), Private);
        self.register_decl("__rorng__", op_t.clone(), Private);
        self.register_decl("__orng__", op_t, Private);
        let op_t = Type::func1(mono_q("T"), Type::mono_proj(mono_q("T"), "MutType!"));
        let op_t = quant(op_t, set! {subtypeof(mono_q("T"), mono("Mutate"))});
        self.register_impl("__mutate__", op_t, Const, Private);
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
        math.register_impl("pi", Type::Float, Immutable, Public);
        math.register_impl("tau", Type::Float, Immutable, Public);
        math.register_impl("e", Type::Float, Immutable, Public);
        math.register_impl("sin", Type::func1(Float, Float), Immutable, Public);
        math.register_impl("cos", Type::func1(Float, Float), Immutable, Public);
        math.register_impl("tan", Type::func1(Float, Float), Immutable, Public);
        math
    }

    pub(crate) fn init_py_random_mod() -> Self {
        let mut random = Context::module("random".into(), 10);
        random.register_impl(
            "seed!",
            Type::proc(
                vec![],
                vec![
                    param_t("a", Type::mono("Num")), // TODO: NoneType, int, float, str, bytes, bytearray
                    param_t("version", Type::Int),
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
            vec![param_t("seq", Type::poly("Seq", vec![ty_tp(mono_q("T"))]))],
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
