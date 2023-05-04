use erg_common::consts::{ERG_MODE, PYTHON_MODE};
#[allow(unused_imports)]
use erg_common::log;
use erg_common::Str as StrStruct;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{Type, Visibility};
use ParamSpec as PS;
use Type::*;

use crate::context::initialize::*;
use crate::context::{Context, ParamSpec};
use crate::varinfo::Mutability;
use Mutability::*;

impl Context {
    pub(super) fn init_builtin_classes(&mut self) {
        let vis = if PYTHON_MODE {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::BUILTIN_PRIVATE
        };
        let T = mono_q(TY_T, instanceof(Type));
        let U = mono_q(TY_U, instanceof(Type));
        let L = mono_q(TY_L, instanceof(Type));
        let R = mono_q(TY_R, instanceof(Type));
        let N = mono_q_tp(TY_N, instanceof(Nat));
        let M = mono_q_tp(TY_M, instanceof(Nat));
        let never = Self::builtin_mono_class(NEVER, 1);
        /* Obj */
        let mut obj = Self::builtin_mono_class(OBJ, 2);
        let Slf = mono_q(SELF, subtypeof(Obj));
        let t = fn0_met(Slf.clone(), Slf).quantify();
        obj.register_py_builtin(
            FUNDAMENTAL_DICT,
            dict! {Str => Obj}.into(),
            Some(FUNDAMENTAL_DICT),
            3,
        );
        obj.register_py_builtin(FUNDAMENTAL_MODULE, Str, Some(FUNDAMENTAL_MODULE), 4);
        obj.register_py_builtin(FUNC_CLONE, t, Some(FUNC_CLONE), 5);
        obj.register_py_builtin(
            FUNDAMENTAL_BYTES,
            fn0_met(Obj, mono(BYTES)),
            Some(FUNDAMENTAL_BYTES),
            6,
        );
        obj.register_py_builtin(
            FUNDAMENTAL_REPR,
            fn0_met(Obj, Str),
            Some(FUNDAMENTAL_REPR),
            7,
        );
        obj.register_py_builtin(
            FUNDAMENTAL_SIZEOF,
            fn0_met(Obj, Nat),
            Some(FUNDAMENTAL_SIZEOF),
            8,
        );
        obj.register_py_builtin(FUNDAMENTAL_STR, fn0_met(Obj, Str), Some(FUNDAMENTAL_STR), 9);
        let mut obj_in = Self::builtin_methods(Some(poly(IN, vec![ty_tp(Type)])), 2);
        obj_in.register_builtin_erg_impl(
            OP_IN,
            fn1_met(Obj, Type, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        obj.register_trait(Obj, obj_in);
        // Obj does not implement Eq
        let mut complex = Self::builtin_mono_class(COMPLEX, 2);
        complex.register_superclass(Obj, &obj);
        // TODO: support multi platform
        complex.register_builtin_const(
            EPSILON,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::Float(2.220446049250313e-16),
        );
        complex.register_builtin_py_impl(
            REAL,
            Float,
            Const,
            Visibility::BUILTIN_PUBLIC,
            Some(REAL),
        );
        complex.register_builtin_py_impl(
            IMAG,
            Float,
            Const,
            Visibility::BUILTIN_PUBLIC,
            Some(IMAG),
        );
        complex.register_builtin_py_impl(
            FUNC_CONJUGATE,
            fn0_met(Complex, Complex),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_CONJUGATE),
        );
        let t = func(
            vec![],
            None,
            vec![kw(REAL, Float), kw(IMAG, Float)],
            Complex,
        );
        complex.register_builtin_py_impl(
            FUNDAMENTAL_CALL,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_CALL),
        );
        complex.register_builtin_py_impl(
            FUNDAMENTAL_HASH,
            fn0_met(Float, Nat),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_HASH),
        );
        /* Float */
        let mut float = Self::builtin_mono_class(FLOAT, 2);
        float.register_superclass(Complex, &complex);
        float.register_py_builtin(
            FUNC_AS_INTEGER_RATIO,
            fn0_met(Float, tuple_t(vec![Int, Int])),
            Some(FUNC_AS_INTEGER_RATIO),
            38,
        );
        float.register_py_builtin(
            FUNC_CONJUGATE,
            fn0_met(Float, Float),
            Some(FUNC_CONJUGATE),
            45,
        );
        float.register_py_builtin(FUNC_HEX, fn0_met(Float, Str), Some(FUNC_HEX), 24);
        float.register_py_builtin(
            FUNC_IS_INTEGER,
            fn0_met(Float, Bool),
            Some(FUNC_IS_INTEGER),
            69,
        );
        float.register_py_builtin(
            FUNC_FROMHEX,
            nd_func(vec![kw(KW_S, Str)], None, Float),
            Some(FUNC_FROMHEX),
            53,
        );
        float.register_py_builtin(
            FUNDAMENTAL_INT,
            fn0_met(Float, Int),
            Some(FUNDAMENTAL_INT),
            0,
        );
        float.register_py_builtin(OP_GT, fn1_met(Float, Float, Bool), Some(OP_GT), 0);
        float.register_py_builtin(OP_GE, fn1_met(Float, Float, Bool), Some(OP_GE), 0);
        float.register_py_builtin(OP_LT, fn1_met(Float, Float, Bool), Some(OP_LT), 0);
        float.register_py_builtin(OP_LE, fn1_met(Float, Float, Bool), Some(OP_LE), 0);
        let t_call = func1(Obj, Float);
        float.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_marker_trait(self, mono(NUM));
        float.register_marker_trait(self, mono(ORD));
        let mut float_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        float_ord.register_builtin_erg_impl(
            OP_CMP,
            fn1_met(Float, Float, mono(ORDERING)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait(Float, float_ord);
        // Float doesn't have an `Eq` implementation
        let op_t = fn1_met(Float, Float, Float);
        let mut float_add = Self::builtin_methods(Some(poly(ADD, vec![ty_tp(Float)])), 2);
        float_add.register_builtin_erg_impl(
            OP_ADD,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float.register_trait(Float, float_add);
        let mut float_sub = Self::builtin_methods(Some(poly(SUB, vec![ty_tp(Float)])), 2);
        float_sub.register_builtin_erg_impl(
            OP_SUB,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float_sub.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float.register_trait(Float, float_sub);
        let mut float_mul = Self::builtin_methods(Some(poly(MUL, vec![ty_tp(Float)])), 2);
        float_mul.register_builtin_erg_impl(
            OP_MUL,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float_mul.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float_mul.register_builtin_const(
            POW_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float.register_trait(Float, float_mul);
        let mut float_div = Self::builtin_methods(Some(poly(DIV, vec![ty_tp(Float)])), 2);
        float_div.register_builtin_erg_impl(
            OP_DIV,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float_div.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float_div.register_builtin_const(
            MOD_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float.register_trait(Float, float_div);
        let mut float_floordiv =
            Self::builtin_methods(Some(poly(FLOOR_DIV, vec![ty_tp(Float)])), 2);
        float_floordiv.register_builtin_erg_impl(
            OP_FLOOR_DIV,
            op_t,
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float_floordiv.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float.register_trait(Float, float_floordiv);
        let mut float_pos = Self::builtin_methods(Some(mono(POS)), 2);
        float_pos.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float_pos.register_builtin_erg_impl(
            OP_POS,
            fn0_met(Float, Float),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait(Float, float_pos);
        let mut float_neg = Self::builtin_methods(Some(mono(NEG)), 2);
        float_neg.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        float_neg.register_builtin_erg_impl(
            OP_NEG,
            fn0_met(Float, Float),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait(Float, float_neg);
        let mut float_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        float_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(mono(MUT_FLOAT)),
        );
        float.register_trait(Float, float_mutizable);
        let mut float_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        let t = fn0_met(Float, Str);
        float_show.register_builtin_py_impl(
            TO_STR,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_STR),
        );
        float.register_trait(Float, float_show);

        /* Ratio */
        // TODO: Int, Nat, Boolの継承元をRatioにする(今はFloat)
        let mut ratio = Self::builtin_mono_class(RATIO, 2);
        ratio.register_superclass(Obj, &obj);
        ratio.register_builtin_py_impl(REAL, Ratio, Const, Visibility::BUILTIN_PUBLIC, Some(REAL));
        ratio.register_builtin_py_impl(IMAG, Ratio, Const, Visibility::BUILTIN_PUBLIC, Some(IMAG));
        ratio.register_marker_trait(self, mono(NUM));
        ratio.register_marker_trait(self, mono(ORD));
        let mut ratio_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        ratio_ord.register_builtin_erg_impl(
            OP_CMP,
            fn1_met(Ratio, Ratio, mono(ORDERING)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio.register_trait(Ratio, ratio_ord);
        let mut ratio_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        ratio_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Ratio, Ratio, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio.register_trait(Ratio, ratio_eq);
        let op_t = fn1_met(Ratio, Ratio, Ratio);
        let mut ratio_add = Self::builtin_methods(Some(poly(ADD, vec![ty_tp(Ratio)])), 2);
        ratio_add.register_builtin_erg_impl(
            OP_ADD,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait(Ratio, ratio_add);
        let mut ratio_sub = Self::builtin_methods(Some(poly(SUB, vec![ty_tp(Ratio)])), 2);
        ratio_sub.register_builtin_erg_impl(
            OP_SUB,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio_sub.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait(Ratio, ratio_sub);
        let mut ratio_mul = Self::builtin_methods(Some(poly(MUL, vec![ty_tp(Ratio)])), 2);
        ratio_mul.register_builtin_erg_impl(
            OP_MUL,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio_mul.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Ratio),
        );
        ratio_mul.register_builtin_const(
            POW_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait(Ratio, ratio_mul);
        let mut ratio_div = Self::builtin_methods(Some(poly(DIV, vec![ty_tp(Ratio)])), 2);
        ratio_div.register_builtin_erg_impl(
            OP_DIV,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio_div.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Ratio),
        );
        ratio_div.register_builtin_const(
            MOD_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait(Ratio, ratio_div);
        let mut ratio_floordiv =
            Self::builtin_methods(Some(poly(FLOOR_DIV, vec![ty_tp(Ratio)])), 2);
        ratio_floordiv.register_builtin_erg_impl(
            OP_FLOOR_DIV,
            op_t,
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio_floordiv.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait(Ratio, ratio_floordiv);
        let mut ratio_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        ratio_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(mono(MUT_RATIO)),
        );
        ratio.register_trait(Ratio, ratio_mutizable);
        let mut ratio_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        let t = fn0_met(Ratio, Str);
        ratio_show.register_builtin_erg_impl(TO_STR, t, Immutable, Visibility::BUILTIN_PUBLIC);
        ratio.register_trait(Ratio, ratio_show);

        /* Int */
        let mut int = Self::builtin_mono_class(INT, 2);
        int.register_superclass(Float, &float); // TODO: Float -> Ratio
        int.register_marker_trait(self, mono(NUM));
        // class("Rational"),
        // class("Integral"),
        int.register_py_builtin(FUNC_ABS, fn0_met(Int, Nat), Some(OP_ABS), 11);
        int.register_py_builtin(FUNC_SUCC, fn0_met(Int, Int), Some(FUNC_SUCC), 54);
        int.register_py_builtin(FUNC_PRED, fn0_met(Int, Int), Some(FUNC_PRED), 47);
        int.register_py_builtin(
            FUNC_BIT_LENGTH,
            fn0_met(Int, Nat),
            Some(FUNC_BIT_LENGTH),
            38,
        );
        int.register_py_builtin(FUNC_BIT_COUNT, fn0_met(Int, Nat), Some(FUNC_BIT_COUNT), 27);
        let t_from_bytes = func(
            vec![kw(
                BYTES,
                or(
                    mono(BYTES),
                    array_t(Type::from(value(0)..=value(255)), TyParam::erased(Nat)),
                ),
            )],
            None,
            vec![kw(
                FUNC_BYTEORDER,
                v_enum(
                    set! {ValueObj::Str(TOKEN_BIG_ENDIAN.into()), ValueObj::Str(TOKEN_LITTLE_ENDIAN.into())},
                ),
            )],
            Int,
        );
        int.register_py_builtin(FUNC_FROM_BYTES, t_from_bytes, Some(FUNC_FROM_BYTES), 40);
        let t_to_bytes = func(
            vec![kw(KW_SELF, Int)],
            None,
            vec![
                kw(KW_LENGTH, Nat),
                kw(
                    FUNC_BYTEORDER,
                    v_enum(
                        set! {ValueObj::Str(TOKEN_BIG_ENDIAN.into()), ValueObj::Str(TOKEN_LITTLE_ENDIAN.into())},
                    ),
                ),
            ],
            mono(BYTES),
        );
        int.register_py_builtin(FUNC_TO_BYTES, t_to_bytes, Some(FUNC_TO_BYTES), 55);
        let t_call = func(vec![pos(Obj)], None, vec![kw("base", Nat)], Int);
        int.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut int_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        int_ord.register_builtin_erg_impl(
            OP_PARTIAL_CMP,
            fn1_met(Int, Int, or(mono(ORDERING), NoneType)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait(Int, int_ord);
        let mut int_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        int_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Int, Int, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait(Int, int_eq);
        // __div__ is not included in Int (cast to Ratio)
        let op_t = fn1_met(Int, Int, Int);
        let mut int_add = Self::builtin_methods(Some(poly(ADD, vec![ty_tp(Int)])), 2);
        int_add.register_builtin_erg_impl(OP_ADD, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        int_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Int),
        );
        int.register_trait(Int, int_add);
        let mut int_sub = Self::builtin_methods(Some(poly(SUB, vec![ty_tp(Int)])), 2);
        int_sub.register_builtin_erg_impl(OP_SUB, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        int_sub.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Int),
        );
        int.register_trait(Int, int_sub);
        let mut int_mul = Self::builtin_methods(Some(poly(MUL, vec![ty_tp(Int)])), 2);
        int_mul.register_builtin_erg_impl(OP_MUL, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        int_mul.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Int),
        );
        int_mul.register_builtin_const(
            POW_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Nat),
        );
        int.register_trait(Int, int_mul);
        let mut int_floordiv = Self::builtin_methods(Some(poly(FLOOR_DIV, vec![ty_tp(Int)])), 2);
        int_floordiv.register_builtin_erg_impl(
            OP_FLOOR_DIV,
            op_t,
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int_floordiv.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Int),
        );
        int.register_trait(Int, int_floordiv);
        let mut int_pos = Self::builtin_methods(Some(mono(POS)), 2);
        int_pos.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Int),
        );
        int_pos.register_builtin_erg_impl(
            OP_POS,
            fn0_met(Int, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait(Int, int_pos);
        let mut int_neg = Self::builtin_methods(Some(mono(NEG)), 2);
        int_neg.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Int),
        );
        int_neg.register_builtin_erg_impl(
            OP_NEG,
            fn0_met(Int, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait(Int, int_neg);
        let mut int_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        int_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(mono(MUT_INT)),
        );
        int.register_trait(Int, int_mutizable);
        let mut int_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        let t = fn0_met(Int, Str);
        int_show.register_builtin_py_impl(
            TO_STR,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_STR),
        );
        int.register_trait(Int, int_show);
        int.register_builtin_py_impl(REAL, Int, Const, Visibility::BUILTIN_PUBLIC, Some(REAL));
        int.register_builtin_py_impl(IMAG, Int, Const, Visibility::BUILTIN_PUBLIC, Some(IMAG));

        /* Nat */
        let mut nat = Self::builtin_mono_class(NAT, 10);
        nat.register_superclass(Int, &int);
        // class("Rational"),
        // class("Integral"),
        nat.register_py_builtin(
            PROC_TIMES,
            pr_met(
                Nat,
                vec![kw(KW_PROC, nd_proc(vec![], None, NoneType))],
                None,
                vec![],
                NoneType,
            ),
            Some(FUNC_TIMES),
            13,
        );
        let t_call = func(vec![pos(Obj)], None, vec![kw("base", Nat)], Nat);
        nat.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        nat.register_marker_trait(self, mono(NUM));
        let mut nat_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        nat_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Nat, Nat, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        nat.register_trait(Nat, nat_eq);
        let mut nat_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        nat_ord.register_builtin_erg_impl(
            OP_CMP,
            fn1_met(Nat, Nat, mono(ORDERING)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        nat.register_trait(Nat, nat_ord);
        // __sub__, __div__ is not included in Nat (cast to Int/ Ratio)
        let op_t = fn1_met(Nat, Nat, Nat);
        let mut nat_add = Self::builtin_methods(Some(poly(ADD, vec![ty_tp(Nat)])), 2);
        nat_add.register_builtin_erg_impl(OP_ADD, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        nat_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Nat),
        );
        nat.register_trait(Nat, nat_add);
        /*
        Since `Int <: Sub(Int)`, the fact that `Nat <: Sub(Nat)` is not inherently necessary.
        However, if there were a constraint `T <: Sub(T)`, we would not be able to let `T = Nat`, so we temporarily insert this trait implementation.
        In the future, it will be implemented automatically by glue patch.
        */
        let op_t_ = fn1_met(Nat, Nat, Int);
        let mut nat_sub = Self::builtin_methods(Some(poly(SUB, vec![ty_tp(Nat)])), 2);
        nat_sub.register_builtin_erg_impl(OP_SUB, op_t_, Const, Visibility::BUILTIN_PUBLIC);
        nat_sub.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Int),
        );
        nat.register_trait(Nat, nat_sub);
        let mut nat_mul = Self::builtin_methods(Some(poly(MUL, vec![ty_tp(Nat)])), 2);
        nat_mul.register_builtin_erg_impl(OP_MUL, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        nat_mul.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Nat),
        );
        nat.register_trait(Nat, nat_mul);
        let mut nat_floordiv = Self::builtin_methods(Some(poly(FLOOR_DIV, vec![ty_tp(Nat)])), 2);
        nat_floordiv.register_builtin_erg_impl(
            OP_FLOOR_DIV,
            op_t,
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        nat_floordiv.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Nat),
        );
        nat.register_trait(Nat, nat_floordiv);
        let mut nat_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        nat_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(mono(MUT_NAT)),
        );
        nat.register_trait(Nat, nat_mutizable);
        nat.register_builtin_erg_impl(REAL, Nat, Const, Visibility::BUILTIN_PUBLIC);
        nat.register_builtin_erg_impl(IMAG, Nat, Const, Visibility::BUILTIN_PUBLIC);

        /* Bool */
        let mut bool_ = Self::builtin_mono_class(BOOL, 10);
        bool_.register_superclass(Nat, &nat);
        // class("Rational"),
        // class("Integral"),
        bool_.register_builtin_erg_impl(
            OP_AND,
            fn1_met(Bool, Bool, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_.register_builtin_erg_impl(
            OP_OR,
            fn1_met(Bool, Bool, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        let t_call = func1(Obj, Bool);
        bool_.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_.register_marker_trait(self, mono(NUM));
        let mut bool_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        bool_ord.register_builtin_erg_impl(
            OP_CMP,
            fn1_met(Bool, Bool, mono(ORDERING)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_.register_trait(Bool, bool_ord);
        let mut bool_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        bool_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Bool, Bool, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_.register_trait(Bool, bool_eq);
        let mut bool_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        bool_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(mono(MUT_BOOL)),
        );
        bool_.register_trait(Bool, bool_mutizable);
        let mut bool_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        bool_show.register_builtin_erg_impl(
            TO_STR,
            fn0_met(Bool, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_.register_trait(Bool, bool_show);
        let t = fn0_met(Bool, Bool);
        bool_.register_py_builtin(FUNC_INVERT, t, Some(FUNC_INVERT), 9);
        /* Str */
        let mut str_ = Self::builtin_mono_class(STR, 10);
        str_.register_superclass(Obj, &obj);
        str_.register_marker_trait(self, mono(ORD));
        str_.register_marker_trait(self, mono(PATH_LIKE));
        str_.register_builtin_erg_impl(
            FUNC_REPLACE,
            fn_met(
                Str,
                vec![kw(KW_PAT, Str), kw(KW_INTO, Str)],
                None,
                vec![],
                Str,
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_py_builtin(
            FUNC_ENCODE,
            fn_met(
                Str,
                vec![],
                None,
                vec![kw(KW_ENCODING, Str), kw(KW_ERRORS, Str)],
                mono(BYTES),
            ),
            Some(FUNC_ENCODE),
            60,
        );
        str_.register_builtin_erg_impl(
            FUNC_FORMAT,
            fn_met(Str, vec![], Some(kw(KW_ARGS, Obj)), vec![], Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_builtin_erg_impl(
            FUNC_LOWER,
            fn_met(Str, vec![], None, vec![], Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_builtin_erg_impl(
            FUNC_UPPER,
            fn_met(Str, vec![], None, vec![], Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_builtin_erg_impl(
            FUNC_TO_INT,
            fn_met(Str, vec![], None, vec![], or(Int, NoneType)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_builtin_py_impl(
            FUNC_STARTSWITH,
            fn1_met(Str, Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_STARTSWITH),
        );
        str_.register_py_builtin(
            FUNC_ENDSWITH,
            fn1_met(Str, Str, Bool),
            Some(FUNC_ENDSWITH),
            69,
        );
        str_.register_builtin_py_impl(
            FUNC_SPLIT,
            fn_met(
                Str,
                vec![kw(KW_SEP, Str)],
                None,
                vec![kw(KW_MAXSPLIT, Nat)],
                unknown_len_array_t(Str),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_SPLIT),
        );
        str_.register_builtin_py_impl(
            FUNC_SPLITLINES,
            fn_met(
                Str,
                vec![],
                None,
                vec![kw(KW_KEEPENDS, Bool)],
                unknown_len_array_t(Str),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_SPLITLINES),
        );
        str_.register_builtin_py_impl(
            FUNC_JOIN,
            fn1_met(Str, poly(ITERABLE, vec![ty_tp(Str)]), Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_JOIN),
        );
        str_.register_py_builtin(
            FUNC_INDEX,
            fn_met(
                Str,
                vec![kw(KW_SUB, Str)],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                or(Nat, Never),
            ),
            Some(FUNC_INDEX),
            126,
        );
        str_.register_builtin_py_impl(
            FUNC_RINDEX,
            fn_met(
                Str,
                vec![kw(KW_SUB, Str)],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                or(Nat, Never),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_RINDEX),
        );
        str_.register_py_builtin(
            FUNC_FIND,
            fn_met(
                Str,
                vec![kw(KW_SUB, Str)],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                or(Nat, v_enum(set! {(-1).into()})),
            ),
            Some(FUNC_FIND),
            93,
        );
        str_.register_builtin_py_impl(
            FUNC_RFIND,
            fn_met(
                Str,
                vec![kw(KW_SUB, Str)],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                or(Nat, v_enum(set! {(-1).into()})),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_RFIND),
        );
        str_.register_py_builtin(
            FUNC_COUNT,
            fn_met(
                Str,
                vec![kw(KW_SUB, Str)],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                Nat,
            ),
            Some(FUNC_COUNT),
            43,
        );
        str_.register_py_builtin(
            FUNC_CAPITALIZE,
            fn0_met(Str, Str),
            Some(FUNC_CAPITALIZE),
            13,
        );
        str_.register_builtin_erg_impl(
            FUNC_CONTAINS,
            fn1_met(Str, Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let str_getitem_t = fn1_kw_met(Str, kw(KW_IDX, Nat), Str);
        str_.register_builtin_erg_impl(
            FUNDAMENTAL_GETITEM,
            str_getitem_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let t_call = func(vec![], None, vec![kw("object", Obj)], Str);
        str_.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut str_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        str_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Str, Str, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_trait(Str, str_eq);
        let mut str_seq = Self::builtin_methods(Some(poly(SEQ, vec![ty_tp(Str)])), 2);
        str_seq.register_builtin_erg_impl(
            FUNC_LEN,
            fn0_met(Str, Nat),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        str_seq.register_builtin_erg_impl(
            FUNC_GET,
            fn1_met(Str, Nat, Str),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_trait(Str, str_seq);
        let mut str_add = Self::builtin_methods(Some(poly(ADD, vec![ty_tp(Str)])), 2);
        str_add.register_builtin_erg_impl(
            OP_ADD,
            fn1_met(Str, Str, Str),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        str_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Str),
        );
        str_.register_trait(Str, str_add);
        let mut str_mul = Self::builtin_methods(Some(poly(MUL, vec![ty_tp(Nat)])), 2);
        str_mul.register_builtin_erg_impl(
            OP_MUL,
            fn1_met(Str, Nat, Str),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        str_mul.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Str),
        );
        str_.register_trait(Str, str_mul);
        let mut str_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        str_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(mono(MUT_STR)),
        );
        str_.register_trait(Str, str_mutizable);
        let mut str_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        str_show.register_builtin_erg_impl(
            TO_STR,
            fn0_met(Str, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_trait(Str, str_show);
        let mut str_iterable = Self::builtin_methods(Some(poly(ITERABLE, vec![ty_tp(Str)])), 2);
        str_iterable.register_builtin_py_impl(
            FUNC_ITER,
            fn0_met(Str, mono(STR_ITERATOR)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        str_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            ValueObj::builtin_class(mono(STR_ITERATOR)),
        );
        str_.register_trait(Str, str_iterable);
        /* NoneType */
        let mut nonetype = Self::builtin_mono_class(NONE_TYPE, 10);
        nonetype.register_superclass(Obj, &obj);
        let mut nonetype_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        nonetype_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(NoneType, NoneType, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        nonetype.register_trait(NoneType, nonetype_eq);
        let mut nonetype_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        nonetype_show.register_builtin_erg_impl(
            TO_STR,
            fn0_met(NoneType, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        nonetype.register_trait(NoneType, nonetype_show);
        /* Type */
        let mut type_ = Self::builtin_mono_class(TYPE, 2);
        type_.register_superclass(Obj, &obj);
        type_.register_builtin_erg_impl(
            FUNC_MRO,
            array_t(Type, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        type_.register_marker_trait(self, mono(NAMED));
        let mut type_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        type_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Type, Type, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        type_.register_trait(Type, type_eq);
        let mut class_type = Self::builtin_mono_class(CLASS_TYPE, 2);
        class_type.register_superclass(Type, &type_);
        class_type.register_marker_trait(self, mono(NAMED));
        let mut class_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        class_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(ClassType, ClassType, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        class_type.register_trait(ClassType, class_eq);
        let mut trait_type = Self::builtin_mono_class(TRAIT_TYPE, 2);
        trait_type.register_superclass(Type, &type_);
        trait_type.register_marker_trait(self, mono(NAMED));
        let mut trait_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        trait_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(TraitType, TraitType, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        trait_type.register_trait(TraitType, trait_eq);
        let mut code = Self::builtin_mono_class(CODE, 10);
        code.register_superclass(Obj, &obj);
        code.register_builtin_erg_impl(
            FUNC_CO_ARGCOUNT,
            Nat,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_VARNAMES,
            array_t(Str, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_CONSTS,
            array_t(Obj, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_NAMES,
            array_t(Str, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_FREEVARS,
            array_t(Str, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_CELLVARS,
            array_t(Str, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_FILENAME,
            Str,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(FUNC_CO_NAME, Str, Immutable, Visibility::BUILTIN_PUBLIC);
        code.register_builtin_erg_impl(
            FUNC_CO_FIRSTLINENO,
            Nat,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_STACKSIZE,
            Nat,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(FUNC_CO_FLAGS, Nat, Immutable, Visibility::BUILTIN_PUBLIC);
        code.register_builtin_erg_impl(
            FUNC_CO_CODE,
            mono(BYTES),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_LNOTAB,
            mono(BYTES),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(FUNC_CO_NLOCALS, Nat, Immutable, Visibility::BUILTIN_PUBLIC);
        code.register_builtin_erg_impl(
            FUNC_CO_KWONLYARGCOUNT,
            Nat,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_POSONLYARGCOUNT,
            Nat,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut code_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        code_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Code, Code, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_trait(Code, code_eq);
        let g_module_t = mono(GENERIC_MODULE);
        let mut generic_module = Self::builtin_mono_class(GENERIC_MODULE, 2);
        generic_module.register_superclass(Obj, &obj);
        generic_module.register_marker_trait(self, mono(NAMED));
        let mut generic_module_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        generic_module_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(g_module_t.clone(), g_module_t.clone(), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_module.register_trait(g_module_t.clone(), generic_module_eq);
        let Path = mono_q_tp(PATH, instanceof(Str));
        let module_t = module(Path.clone());
        let py_module_t = py_module(Path);
        let mut module = Self::builtin_poly_class(MODULE, vec![PS::named_nd(PATH, Str)], 2);
        module.register_superclass(g_module_t.clone(), &generic_module);
        let mut py_module = Self::builtin_poly_class(PY_MODULE, vec![PS::named_nd(PATH, Str)], 2);
        if ERG_MODE {
            py_module.register_superclass(g_module_t.clone(), &generic_module);
        }
        /* GenericArray */
        let mut generic_array = Self::builtin_mono_class(GENERIC_ARRAY, 1);
        generic_array.register_superclass(Obj, &obj);
        let mut arr_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        arr_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(mono(GENERIC_ARRAY), mono(GENERIC_ARRAY), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_array.register_trait(mono(GENERIC_ARRAY), arr_eq);
        let t_call = func1(
            poly(ITERABLE, vec![ty_tp(T.clone())]),
            array_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        generic_array.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        /* Array */
        let mut array_ =
            Self::builtin_poly_class(ARRAY, vec![PS::t_nd(TY_T), PS::named_nd(TY_N, Nat)], 10);
        array_.register_superclass(mono(GENERIC_ARRAY), &generic_array);
        array_.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let arr_t = array_t(T.clone(), N.clone());
        let t = fn_met(
            arr_t.clone(),
            vec![kw(KW_RHS, array_t(T.clone(), M.clone()))],
            None,
            vec![],
            array_t(T.clone(), N.clone() + M.clone()),
        )
        .quantify();
        array_.register_py_builtin(FUNC_CONCAT, t.clone(), Some(OP_ADD), 9);
        let t_count =
            fn_met(arr_t.clone(), vec![kw(KW_X, T.clone())], None, vec![], Nat).quantify();
        array_.register_py_builtin(FUNC_COUNT, t_count, Some(FUNC_COUNT), 17);
        // Array(T, N)|<: Add(Array(T, M))|.
        //     Output = Array(T, N + M)
        //     __add__: (self: Array(T, N), other: Array(T, M)) -> Array(T, N + M) = Array.concat
        let mut array_add = Self::builtin_methods(
            Some(poly(ADD, vec![ty_tp(array_t(T.clone(), M.clone()))])),
            2,
        );
        array_add.register_builtin_erg_impl(OP_ADD, t, Immutable, Visibility::BUILTIN_PUBLIC);
        let out_t = array_t(T.clone(), N.clone() + M);
        array_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(out_t),
        );
        array_.register_trait(arr_t.clone(), array_add);
        let t = fn_met(
            arr_t.clone(),
            vec![kw(KW_ELEM, T.clone())],
            None,
            vec![],
            array_t(T.clone(), N.clone() + value(1usize)),
        )
        .quantify();
        array_.register_builtin_erg_impl(FUNC_PUSH, t, Immutable, Visibility::BUILTIN_PUBLIC);
        // [T; N].MutType! = [T; !N] (neither [T!; N] nor [T; N]!)
        let mut_type =
            ValueObj::builtin_class(poly(MUT_ARRAY, vec![TyParam::t(T.clone()), N.clone()]));
        let mut array_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        array_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            mut_type,
        );
        array_.register_trait(arr_t.clone(), array_mutizable);
        let var = StrStruct::from(fresh_varname());
        let input = refinement(
            var.clone(),
            Nat,
            Predicate::le(var, N.clone() - value(1usize)),
        );
        // __getitem__: |T, N|(self: [T; N], _: {I: Nat | I <= N}) -> T
        let array_getitem_t =
            fn1_kw_met(array_t(T.clone(), N.clone()), anon(input), T.clone()).quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNDAMENTAL_GETITEM,
            __array_getitem__,
            array_getitem_t,
            None,
        )));
        array_.register_builtin_const(FUNDAMENTAL_GETITEM, Visibility::BUILTIN_PUBLIC, get_item);
        // union: (self: [Type; _]) -> Type
        let array_union_t = fn0_met(array_t(Type, TyParam::erased(Nat)), Type).quantify();
        let union = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            UNION_FUNC,
            array_union,
            array_union_t,
            None,
        )));
        array_.register_builtin_const(UNION_FUNC, Visibility::BUILTIN_PUBLIC, union);
        let mut array_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        array_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(arr_t.clone(), arr_t.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        array_.register_trait(arr_t.clone(), array_eq);
        array_.register_marker_trait(self, poly(SEQ, vec![ty_tp(T.clone())]));
        let mut array_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        array_show.register_builtin_py_impl(
            TO_STR,
            fn0_met(arr_t.clone(), Str).quantify(),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_STR),
        );
        array_.register_trait(arr_t.clone(), array_show);
        let mut array_iterable =
            Self::builtin_methods(Some(poly(ITERABLE, vec![ty_tp(T.clone())])), 2);
        let array_iter = poly(ARRAY_ITERATOR, vec![ty_tp(T.clone())]);
        let t = fn0_met(array_t(T.clone(), TyParam::erased(Nat)), array_iter.clone()).quantify();
        array_iterable.register_builtin_py_impl(
            FUNC_ITER,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        array_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            ValueObj::builtin_class(array_iter),
        );
        array_.register_trait(arr_t.clone(), array_iterable);
        let t = fn1_met(
            array_t(T.clone(), TyParam::erased(Nat)),
            func1(T.clone(), Bool),
            tuple_t(vec![
                array_t(T.clone(), TyParam::erased(Nat)),
                array_t(T.clone(), TyParam::erased(Nat)),
            ]),
        );
        array_.register_py_builtin(FUNC_PARTITION, t.quantify(), Some(FUNC_PARTITION), 37);
        let t = fn_met(
            array_t(T.clone(), TyParam::erased(Nat)),
            vec![],
            None,
            vec![kw(
                "same_bucket",
                or(func2(T.clone(), T.clone(), Bool), NoneType),
            )],
            array_t(T.clone(), TyParam::erased(Nat)),
        );
        array_.register_py_builtin(FUNC_DEDUP, t.quantify(), Some(FUNC_DEDUP), 28);
        /* GenericSet */
        let mut generic_set = Self::builtin_mono_class(GENERIC_SET, 1);
        generic_set.register_superclass(Obj, &obj);
        let mut set_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        set_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(mono(GENERIC_SET), mono(GENERIC_SET), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_set.register_trait(mono(GENERIC_SET), set_eq);
        let t_call = func1(
            poly(ITERABLE, vec![ty_tp(T.clone())]),
            set_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        generic_set.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        /* Set */
        let mut set_ =
            Self::builtin_poly_class(SET, vec![PS::t_nd(TY_T), PS::named_nd(TY_N, Nat)], 10);
        let set_t = set_t(T.clone(), TyParam::erased(Nat));
        set_.register_superclass(mono(GENERIC_SET), &generic_set);
        set_.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let t = fn_met(
            set_t.clone(),
            vec![kw(KW_RHS, set_t.clone())],
            None,
            vec![],
            set_t.clone(),
        )
        .quantify();
        set_.register_builtin_erg_impl(FUNC_CONCAT, t, Immutable, Visibility::BUILTIN_PUBLIC);
        let mut_type = ValueObj::builtin_class(poly(MUT_SET, vec![TyParam::t(T.clone())]));
        let mut set_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        set_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            mut_type,
        );
        set_.register_trait(set_t.clone(), set_mutizable);
        let mut set_iterable =
            Self::builtin_methods(Some(poly(ITERABLE, vec![ty_tp(T.clone())])), 2);
        let set_iter = poly(SET_ITERATOR, vec![ty_tp(T.clone())]);
        let t = fn0_met(set_t.clone(), set_iter.clone()).quantify();
        set_iterable.register_builtin_py_impl(
            FUNC_ITER,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        set_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            ValueObj::builtin_class(set_iter),
        );
        set_.register_trait(set_t.clone(), set_iterable);
        let t_call = func1(poly(ITERABLE, vec![ty_tp(T.clone())]), set_t.clone()).quantify();
        set_.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut set_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        set_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(set_t.clone(), set_t.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        set_.register_trait(set_t.clone(), set_eq);
        set_.register_marker_trait(self, mono(MUTIZABLE));
        set_.register_marker_trait(self, poly(SEQ, vec![ty_tp(T.clone())]));
        let mut set_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        set_show.register_builtin_erg_impl(
            TO_STR,
            fn0_met(set_t.clone(), Str).quantify(),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        set_.register_trait(set_t.clone(), set_show);
        let g_dict_t = mono(GENERIC_DICT);
        let mut generic_dict = Self::builtin_mono_class(GENERIC_DICT, 2);
        generic_dict.register_superclass(Obj, &obj);
        let mut generic_dict_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        generic_dict_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(g_dict_t.clone(), g_dict_t.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_dict.register_trait(g_dict_t.clone(), generic_dict_eq);
        let D = mono_q_tp(TY_D, instanceof(mono(GENERIC_DICT)));
        // .get: _: T -> T or None
        let dict_get_t = fn1_met(g_dict_t.clone(), T.clone(), or(T.clone(), NoneType)).quantify();
        generic_dict.register_builtin_erg_impl(
            FUNC_GET,
            dict_get_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let inner = ty_tp(tuple_t(vec![T.clone(), U.clone()]));
        let t_call = func1(
            poly(ITERABLE, vec![inner]),
            dict! { T.clone() => U.clone() }.into(),
        )
        .quantify();
        generic_dict.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let dict_t = poly(DICT, vec![D.clone()]);
        let mut dict_ =
            // TODO: D <: GenericDict
            Self::builtin_poly_class(DICT, vec![PS::named_nd(TY_D, mono(GENERIC_DICT))], 10);
        dict_.register_superclass(g_dict_t.clone(), &generic_dict);
        dict_.register_marker_trait(self, poly(OUTPUT, vec![D.clone()]));
        let mut dict_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        dict_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(poly(MUT_DICT, vec![D.clone()])),
        );
        dict_.register_trait(dict_t.clone(), dict_mutizable);
        // __getitem__: _: T -> D[T]
        let dict_getitem_t = fn1_met(
            dict_t.clone(),
            T.clone(),
            proj_call(D.clone(), FUNDAMENTAL_GETITEM, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNDAMENTAL_GETITEM,
            __dict_getitem__,
            dict_getitem_t,
            None,
        )));
        dict_.register_builtin_const(FUNDAMENTAL_GETITEM, Visibility::BUILTIN_PUBLIC, get_item);
        let dict_keys_t = fn0_met(dict_t.clone(), proj_call(D.clone(), KEYS, vec![])).quantify();
        let keys = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            KEYS,
            dict_keys,
            dict_keys_t,
            None,
        )));
        dict_.register_builtin_const(KEYS, Visibility::BUILTIN_PUBLIC, keys);
        let dict_values_t =
            fn0_met(dict_t.clone(), proj_call(D.clone(), VALUES, vec![])).quantify();
        let values = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            VALUES,
            dict_values,
            dict_values_t,
            None,
        )));
        dict_.register_builtin_const(VALUES, Visibility::BUILTIN_PUBLIC, values);
        let dict_items_t = fn0_met(dict_t.clone(), proj_call(D.clone(), ITEMS, vec![])).quantify();
        let items = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            ITEMS,
            dict_items,
            dict_items_t,
            None,
        )));
        dict_.register_builtin_const(ITEMS, Visibility::BUILTIN_PUBLIC, items);
        let Def = type_q("Default");
        let get_t = fn_met(
            dict_t.clone(),
            vec![kw("key", T.clone())],
            None,
            vec![kw_default("default", Def.clone(), NoneType)],
            or(
                proj_call(D.clone(), FUNDAMENTAL_GETITEM, vec![ty_tp(T.clone())]),
                Def,
            ),
        )
        .quantify();
        dict_.register_py_builtin(FUNC_GET, get_t, Some(FUNC_GET), 9);
        let copy_t = fn0_met(dict_t.clone(), dict_t.clone()).quantify();
        dict_.register_py_builtin(COPY, copy_t, Some(COPY), 7);
        /* Bytes */
        let mut bytes = Self::builtin_mono_class(BYTES, 2);
        bytes.register_superclass(Obj, &obj);
        let decode_t = pr_met(
            mono(BYTES),
            vec![],
            None,
            vec![kw(KW_ENCODING, Str), kw(KW_ERRORS, Str)],
            Str,
        );
        bytes.register_py_builtin(FUNC_DECODE, decode_t, Some(FUNC_DECODE), 6);
        /* GenericTuple */
        let mut generic_tuple = Self::builtin_mono_class(GENERIC_TUPLE, 1);
        generic_tuple.register_superclass(Obj, &obj);
        // tuple doesn't have a constructor, use `Array` instead
        let mut tuple_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        tuple_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(mono(GENERIC_TUPLE), mono(GENERIC_TUPLE), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_tuple.register_trait(mono(GENERIC_TUPLE), tuple_eq);
        let Ts = mono_q_tp(TY_TS, instanceof(array_t(Type, N.clone())));
        // Ts <: GenericArray
        let _tuple_t = poly(TUPLE, vec![Ts.clone()]);
        let mut tuple_ = Self::builtin_poly_class(
            TUPLE,
            vec![PS::named_nd(TY_TS, array_t(Type, N.clone()))],
            2,
        );
        tuple_.register_superclass(mono(GENERIC_TUPLE), &generic_tuple);
        tuple_.register_marker_trait(self, poly(OUTPUT, vec![Ts.clone()]));
        // __Tuple_getitem__: (self: Tuple(Ts), _: {N}) -> Ts[N]
        let return_t = proj_call(Ts.clone(), FUNDAMENTAL_GETITEM, vec![N.clone()]);
        let tuple_getitem_t =
            fn1_met(_tuple_t.clone(), tp_enum(Nat, set! {N.clone()}), return_t).quantify();
        tuple_.register_builtin_py_impl(
            FUNDAMENTAL_TUPLE_GETITEM,
            tuple_getitem_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_GETITEM),
        );
        // `__Tuple_getitem__` and `__getitem__` are the same thing
        // but `x.0` => `x__Tuple_getitem__(0)` determines that `x` is a tuple, which is better for type inference.
        tuple_.register_builtin_py_impl(
            FUNDAMENTAL_GETITEM,
            tuple_getitem_t,
            Const,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_GETITEM),
        );
        let mut tuple_iterable = Self::builtin_methods(
            Some(poly(
                ITERABLE,
                vec![ty_tp(proj_call(Ts.clone(), UNION_FUNC, vec![]))],
            )),
            2,
        );
        let tuple_iterator = poly(
            TUPLE_ITERATOR,
            vec![ty_tp(proj_call(Ts, UNION_FUNC, vec![]))],
        );
        // Tuple(Ts) -> TupleIterator(Ts.union())
        let t = fn0_met(_tuple_t.clone(), tuple_iterator.clone()).quantify();
        tuple_iterable.register_builtin_py_impl(
            FUNC_ITER,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        tuple_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            ValueObj::builtin_class(tuple_iterator),
        );
        tuple_.register_trait(_tuple_t.clone(), tuple_iterable);
        /* record */
        let mut record = Self::builtin_mono_class(RECORD, 2);
        record.register_superclass(Obj, &obj);
        /* Or (true or type) */
        let or_t = poly(OR, vec![ty_tp(L), ty_tp(R)]);
        let mut or = Self::builtin_poly_class(OR, vec![PS::t_nd(TY_L), PS::t_nd(TY_R)], 2);
        or.register_superclass(Obj, &obj);
        /* Iterators */
        let mut str_iterator = Self::builtin_mono_class(STR_ITERATOR, 1);
        str_iterator.register_superclass(Obj, &obj);
        str_iterator.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(Str)]));
        str_iterator.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(Str)]));
        let mut array_iterator = Self::builtin_poly_class(ARRAY_ITERATOR, vec![PS::t_nd(TY_T)], 1);
        array_iterator.register_superclass(Obj, &obj);
        array_iterator.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        array_iterator.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let mut set_iterator = Self::builtin_poly_class(SET_ITERATOR, vec![PS::t_nd(TY_T)], 1);
        set_iterator.register_superclass(Obj, &obj);
        set_iterator.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        set_iterator.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let mut tuple_iterator = Self::builtin_poly_class(TUPLE_ITERATOR, vec![PS::t_nd(TY_T)], 1);
        tuple_iterator.register_superclass(Obj, &obj);
        tuple_iterator.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        tuple_iterator.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let mut range_iterator = Self::builtin_poly_class(RANGE_ITERATOR, vec![PS::t_nd(TY_T)], 1);
        range_iterator.register_superclass(Obj, &obj);
        range_iterator.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        range_iterator.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let mut dict_keys = Self::builtin_poly_class(DICT_KEYS, vec![PS::t_nd(TY_T)], 1);
        dict_keys.register_superclass(Obj, &obj);
        dict_keys.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        dict_keys.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let mut dict_values = Self::builtin_poly_class(DICT_VALUES, vec![PS::t_nd(TY_T)], 1);
        dict_values.register_superclass(Obj, &obj);
        dict_values.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        dict_values.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let mut dict_items = Self::builtin_poly_class(DICT_ITEMS, vec![PS::t_nd(TY_T)], 1);
        dict_items.register_superclass(Obj, &obj);
        dict_items.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        dict_items.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        /* Enumerate */
        let mut enumerate = Self::builtin_poly_class(ENUMERATE, vec![PS::t_nd(TY_T)], 2);
        enumerate.register_superclass(Obj, &obj);
        enumerate.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        enumerate.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        /* Filter */
        let mut filter = Self::builtin_poly_class(FILTER, vec![PS::t_nd(TY_T)], 2);
        filter.register_superclass(Obj, &obj);
        filter.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        filter.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        /* Map */
        let mut map = Self::builtin_poly_class(MAP, vec![PS::t_nd(TY_T)], 2);
        map.register_superclass(Obj, &obj);
        map.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        map.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        /* Reversed */
        let mut reversed = Self::builtin_poly_class(REVERSED, vec![PS::t_nd(TY_T)], 2);
        reversed.register_superclass(Obj, &obj);
        reversed.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        reversed.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        /* Zip */
        let mut zip = Self::builtin_poly_class(ZIP, vec![PS::t_nd(TY_T), PS::t_nd(TY_U)], 2);
        zip.register_superclass(Obj, &obj);
        zip.register_marker_trait(
            self,
            poly(ITERABLE, vec![ty_tp(tuple_t(vec![T.clone(), U.clone()]))]),
        );
        zip.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        zip.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(U.clone())]));
        let fset_t = poly(FROZENSET, vec![ty_tp(T.clone())]);
        let mut frozenset = Self::builtin_poly_class(FROZENSET, vec![PS::t_nd(TY_T)], 2);
        frozenset.register_superclass(Obj, &obj);
        frozenset.register_marker_trait(self, poly(ITERABLE, vec![ty_tp(T.clone())]));
        frozenset.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        let t = fn0_met(fset_t.clone(), fset_t.clone()).quantify();
        frozenset.register_py_builtin(COPY, t, Some(COPY), 3);
        let bin_t = fn1_met(fset_t.clone(), fset_t.clone(), fset_t.clone()).quantify();
        frozenset.register_py_builtin(DIFFERENCE, bin_t.clone(), Some(DIFFERENCE), 3);
        frozenset.register_py_builtin(INTERSECTION, bin_t.clone(), Some(INTERSECTION), 3);
        let bool_t = fn1_met(fset_t.clone(), fset_t.clone(), Bool).quantify();
        frozenset.register_py_builtin(ISDISJOINT, bool_t.clone(), Some(ISDISJOINT), 3);
        frozenset.register_py_builtin(ISSUBSET, bool_t.clone(), Some(ISSUBSET), 3);
        frozenset.register_py_builtin(ISSUPERSET, bool_t, Some(ISSUPERSET), 3);
        frozenset.register_py_builtin(
            SYMMETRIC_DIFFERENCE,
            bin_t.clone(),
            Some(SYMMETRIC_DIFFERENCE),
            3,
        );
        frozenset.register_py_builtin(UNION_FUNC, bin_t, Some(UNION_FUNC), 3);
        let memview_t = mono(MEMORYVIEW);
        let mut memoryview = Self::builtin_mono_class(MEMORYVIEW, 2);
        memoryview.register_superclass(Obj, &obj);
        let mut obj_mut = Self::builtin_mono_class(MUTABLE_OBJ, 2);
        obj_mut.register_superclass(Obj, &obj);
        let mut obj_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        obj_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Obj),
        );
        let f_t = kw(KW_FUNC, func(vec![kw(KW_OLD, Int)], None, vec![], Int));
        let t = pr_met(
            ref_mut(mono(MUTABLE_OBJ), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        obj_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        obj_mut.register_trait(mono(MUTABLE_OBJ), obj_mut_mutable);
        /* Float! */
        let mut float_mut = Self::builtin_mono_class(MUT_FLOAT, 2);
        float_mut.register_superclass(Float, &float);
        let mut float_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        float_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Float),
        );
        let f_t = kw(KW_FUNC, func(vec![kw(KW_OLD, Float)], None, vec![], Float));
        let t = pr_met(
            ref_mut(mono(MUT_FLOAT), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        float_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        float_mut.register_trait(mono(MUT_FLOAT), float_mut_mutable);
        /* Ratio! */
        let mut ratio_mut = Self::builtin_mono_class(MUT_RATIO, 2);
        ratio_mut.register_superclass(Ratio, &ratio);
        let mut ratio_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        ratio_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Ratio),
        );
        let f_t = kw(KW_FUNC, func(vec![kw(KW_OLD, Ratio)], None, vec![], Ratio));
        let t = pr_met(
            ref_mut(mono(MUT_RATIO), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        ratio_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio_mut.register_trait(mono(MUT_RATIO), ratio_mut_mutable);
        /* Int! */
        let mut int_mut = Self::builtin_mono_class(MUT_INT, 2);
        int_mut.register_superclass(Int, &int);
        int_mut.register_superclass(mono(MUT_FLOAT), &float_mut);
        let t = pr_met(mono(MUT_INT), vec![], None, vec![kw("i", Int)], NoneType);
        int_mut.register_builtin_py_impl(
            PROC_INC,
            t.clone(),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_INC),
        );
        int_mut.register_builtin_py_impl(
            PROC_DEC,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_DEC),
        );
        let mut int_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        int_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Int),
        );
        let f_t = kw(KW_FUNC, func(vec![kw(KW_OLD, Int)], None, vec![], Int));
        let t = pr_met(
            ref_mut(mono(MUT_INT), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        int_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        int_mut.register_trait(mono(MUT_INT), int_mut_mutable);
        let mut nat_mut = Self::builtin_mono_class(MUT_NAT, 2);
        nat_mut.register_superclass(Nat, &nat);
        nat_mut.register_superclass(mono(MUT_INT), &int_mut);
        /* Nat! */
        let mut nat_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        nat_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Nat),
        );
        let f_t = kw(KW_FUNC, func(vec![kw(KW_OLD, Nat)], None, vec![], Nat));
        let t = pr_met(
            ref_mut(mono(MUT_NAT), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        nat_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        nat_mut.register_trait(mono(MUT_NAT), nat_mut_mutable);
        /* Bool! */
        let mut bool_mut = Self::builtin_mono_class(MUT_BOOL, 2);
        bool_mut.register_superclass(Bool, &bool_);
        bool_mut.register_superclass(mono(MUT_NAT), &nat_mut);
        let mut bool_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        bool_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Bool),
        );
        let f_t = kw(KW_FUNC, func(vec![kw(KW_OLD, Bool)], None, vec![], Bool));
        let t = pr_met(
            ref_mut(mono(MUT_BOOL), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        bool_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_mut.register_trait(mono(MUT_BOOL), bool_mut_mutable);
        let t = pr0_met(mono(MUT_BOOL), NoneType);
        bool_mut.register_builtin_py_impl(
            PROC_INVERT,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_INVERT),
        );
        /* Str! */
        let mut str_mut = Self::builtin_mono_class(MUT_STR, 2);
        str_mut.register_superclass(Str, &nonetype);
        let mut str_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        str_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            ValueObj::builtin_class(Str),
        );
        let f_t = kw(KW_FUNC, func(vec![kw(KW_OLD, Str)], None, vec![], Str));
        let t = pr_met(
            ref_mut(mono(MUT_STR), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        str_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_mut.register_trait(mono(MUT_STR), str_mut_mutable);
        let t = pr_met(
            ref_mut(mono(MUT_STR), None),
            vec![kw("s", Str)],
            None,
            vec![],
            NoneType,
        );
        str_mut.register_builtin_py_impl(
            PROC_PUSH,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_PUSH),
        );
        let t = pr0_met(ref_mut(mono(MUT_STR), None), Str);
        str_mut.register_builtin_py_impl(
            PROC_POP,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_POP),
        );
        let t = pr0_met(ref_mut(mono(MUT_STR), None), NoneType);
        str_mut.register_builtin_py_impl(
            PROC_CLEAR,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_CLEAR),
        );
        let t = pr_met(
            ref_mut(mono(MUT_STR), None),
            vec![kw("idx", Nat), kw("s", Str)],
            None,
            vec![],
            NoneType,
        );
        str_mut.register_builtin_py_impl(
            PROC_INSERT,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_INSERT),
        );
        let t = pr_met(
            ref_mut(mono(MUT_STR), None),
            vec![kw("idx", Nat)],
            None,
            vec![],
            Str,
        );
        str_mut.register_builtin_py_impl(
            PROC_REMOVE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_REMOVE),
        );
        /* File! */
        let mut file_mut = Self::builtin_mono_class(MUT_FILE, 2);
        let mut file_mut_readable = Self::builtin_methods(Some(mono(MUT_READABLE)), 1);
        file_mut_readable.register_builtin_py_impl(
            PROC_READ,
            pr_met(
                ref_mut(mono(MUT_FILE), None),
                vec![],
                None,
                vec![kw(KW_N, Int)],
                Str,
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_READ),
        );
        file_mut.register_trait(mono(MUT_FILE), file_mut_readable);
        let mut file_mut_writable = Self::builtin_methods(Some(mono(MUT_WRITABLE)), 1);
        file_mut_writable.register_builtin_py_impl(
            PROC_WRITE,
            pr1_kw_met(ref_mut(mono(MUT_FILE), None), kw(KW_S, Str), Nat),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_WRITE),
        );
        file_mut.register_trait(mono(MUT_FILE), file_mut_writable);
        file_mut.register_marker_trait(self, mono(FILE_LIKE));
        file_mut.register_marker_trait(self, mono(MUT_FILE_LIKE));
        file_mut.register_marker_trait(self, mono(CONTEXT_MANAGER));
        /* Array! */
        let array_mut_t = poly(MUT_ARRAY, vec![ty_tp(T.clone()), N.clone()]);
        let mut array_mut_ =
            Self::builtin_poly_class(MUT_ARRAY, vec![PS::t_nd(TY_T), PS::named_nd(TY_N, Nat)], 2);
        array_mut_.register_superclass(arr_t.clone(), &array_);
        let t = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    MUT_ARRAY,
                    vec![ty_tp(T.clone()), N.clone() + value(1usize)],
                )),
            ),
            vec![kw(KW_ELEM, T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_py_builtin(PROC_PUSH, t, Some(FUNC_APPEND), 14);
        let t_extend = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    MUT_ARRAY,
                    vec![ty_tp(T.clone()), TyParam::erased(Nat)],
                )),
            ),
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_py_builtin(PROC_EXTEND, t_extend, Some(FUNC_EXTEND), 23);
        let t_insert = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    MUT_ARRAY,
                    vec![ty_tp(T.clone()), N.clone() + value(1usize)],
                )),
            ),
            vec![kw(KW_INDEX, Nat), kw(KW_ELEM, T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_py_builtin(PROC_INSERT, t_insert, Some(FUNC_INSERT), 32);
        let t_remove = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    MUT_ARRAY,
                    vec![ty_tp(T.clone()), N.clone() - value(1usize)],
                )),
            ),
            vec![kw(KW_X, T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_py_builtin(PROC_REMOVE, t_remove, Some(FUNC_REMOVE), 41);
        let t_pop = pr_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(
                    MUT_ARRAY,
                    vec![ty_tp(T.clone()), N.clone() - value(1usize)],
                )),
            ),
            vec![],
            None,
            vec![kw(KW_INDEX, Nat)],
            T.clone(),
        )
        .quantify();
        array_mut_.register_py_builtin(PROC_POP, t_pop, Some(FUNC_POP), 51);
        let t_clear = pr0_met(
            ref_mut(
                array_mut_t.clone(),
                Some(poly(MUT_ARRAY, vec![ty_tp(T.clone()), value(0usize)])),
            ),
            NoneType,
        )
        .quantify();
        array_mut_.register_py_builtin(PROC_CLEAR, t_clear, Some(FUNC_CLEAR), 60);
        let t_sort = pr_met(
            ref_mut(array_mut_t.clone(), None),
            vec![],
            None,
            vec![kw(
                KW_KEY,
                func(vec![kw(KW_X, T.clone())], None, vec![], mono(ORD)),
            )],
            NoneType,
        )
        .quantify();
        array_mut_.register_py_builtin(PROC_SORT, t_sort, Some(FUNC_SORT), 77);
        let t_reverse = pr0_met(ref_mut(array_mut_t.clone(), None), NoneType).quantify();
        array_mut_.register_py_builtin(PROC_REVERSE, t_reverse, Some(FUNC_REVERSE), 86);
        let t = pr_met(
            array_mut_t.clone(),
            vec![kw(KW_FUNC, nd_func(vec![anon(T.clone())], None, T.clone()))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        array_mut_.register_py_builtin(PROC_STRICT_MAP, t, None, 95);
        let f_t = kw(
            KW_FUNC,
            func(vec![kw(KW_OLD, arr_t.clone())], None, vec![], arr_t.clone()),
        );
        let t = pr_met(
            ref_mut(array_mut_t.clone(), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        let mut array_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        array_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        array_mut_.register_trait(array_mut_t.clone(), array_mut_mutable);
        /* Dict! */
        let dict_mut_t = poly(MUT_DICT, vec![D.clone()]);
        let mut dict_mut =
            Self::builtin_poly_class(MUT_DICT, vec![PS::named_nd(TY_D, mono(GENERIC_DICT))], 3);
        dict_mut.register_superclass(dict_t.clone(), &dict_);
        let K = type_q("K");
        let V = type_q("V");
        let insert_t = pr_met(
            ref_mut(
                dict_mut_t.clone(),
                Some(poly(
                    MUT_DICT,
                    vec![D + dict! { K.clone() => V.clone() }.into()],
                )),
            ),
            vec![kw(KW_KEY, K), kw(KW_VALUE, V)],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        dict_mut.register_py_builtin(PROC_INSERT, insert_t, Some(FUNDAMENTAL_SETITEM), 12);
        /* Set! */
        let set_mut_t = poly(MUT_SET, vec![ty_tp(T.clone()), N]);
        let mut set_mut_ =
            Self::builtin_poly_class(MUT_SET, vec![PS::t_nd(TY_T), PS::named_nd(TY_N, Nat)], 2);
        set_mut_.register_superclass(set_t.clone(), &set_);
        // `add!` will erase N
        let t = pr_met(
            ref_mut(
                set_mut_t.clone(),
                Some(poly(MUT_SET, vec![ty_tp(T.clone()), TyParam::erased(Nat)])),
            ),
            vec![kw(KW_ELEM, T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        set_mut_.register_builtin_py_impl(
            PROC_ADD,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ADD),
        );
        let t = pr_met(
            set_mut_t.clone(),
            vec![kw(KW_FUNC, nd_func(vec![anon(T.clone())], None, T.clone()))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        set_mut_.register_builtin_erg_impl(
            PROC_STRICT_MAP,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let f_t = kw(
            KW_FUNC,
            func(vec![kw(KW_OLD, set_t.clone())], None, vec![], set_t.clone()),
        );
        let t = pr_met(
            ref_mut(set_mut_t.clone(), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        let mut set_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        set_mut_mutable.register_builtin_erg_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        set_mut_.register_trait(set_mut_t.clone(), set_mut_mutable);
        /* Range */
        let range_t = poly(RANGE, vec![TyParam::t(T.clone())]);
        let mut range = Self::builtin_poly_class(RANGE, vec![PS::t_nd(TY_T)], 2);
        // range.register_superclass(Obj, &obj);
        range.register_superclass(Type, &type_);
        range.register_marker_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]));
        range.register_marker_trait(self, poly(SEQ, vec![ty_tp(T.clone())]));
        let mut range_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        range_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(range_t.clone(), range_t.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        range.register_trait(range_t.clone(), range_eq);
        let mut range_iterable =
            Self::builtin_methods(Some(poly(ITERABLE, vec![ty_tp(T.clone())])), 2);
        let range_iter = poly(RANGE_ITERATOR, vec![ty_tp(T.clone())]);
        range_iterable.register_builtin_py_impl(
            FUNC_ITER,
            fn0_met(range_t.clone(), range_iter.clone()).quantify(),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        range_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            ValueObj::builtin_class(range_iter),
        );
        range.register_trait(range_t.clone(), range_iterable);
        let range_getitem_t = fn1_kw_met(range_t.clone(), anon(T.clone()), T.clone()).quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNDAMENTAL_GETITEM,
            __range_getitem__,
            range_getitem_t,
            None,
        )));
        range.register_builtin_const(FUNDAMENTAL_GETITEM, Visibility::BUILTIN_PUBLIC, get_item);
        let mut g_callable = Self::builtin_mono_class(GENERIC_CALLABLE, 2);
        g_callable.register_superclass(Obj, &obj);
        let t_return = fn1_met(mono(GENERIC_CALLABLE), Obj, Never).quantify();
        g_callable.register_builtin_erg_impl(
            FUNC_RETURN,
            t_return,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut g_generator = Self::builtin_mono_class(GENERIC_GENERATOR, 2);
        g_generator.register_superclass(mono(GENERIC_CALLABLE), &g_callable);
        let t_yield = fn1_met(mono(GENERIC_GENERATOR), Obj, Never).quantify();
        g_generator.register_builtin_erg_impl(
            FUNC_YIELD,
            t_yield,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        /* Proc */
        let mut proc = Self::builtin_mono_class(PROC, 2);
        proc.register_superclass(mono(GENERIC_CALLABLE), &g_callable);
        let mut named_proc = Self::builtin_mono_class(NAMED_PROC, 2);
        named_proc.register_superclass(mono(PROC), &proc);
        named_proc.register_marker_trait(self, mono(NAMED));
        /* Func */
        let mut func = Self::builtin_mono_class(FUNC, 2);
        func.register_superclass(mono(PROC), &proc);
        let mut named_func = Self::builtin_mono_class(NAMED_FUNC, 2);
        named_func.register_superclass(mono(FUNC), &func);
        named_func.register_marker_trait(self, mono(NAMED));
        let mut quant = Self::builtin_mono_class(QUANTIFIED, 2);
        quant.register_superclass(mono(PROC), &proc);
        let mut qfunc = Self::builtin_mono_class(QUANTIFIED_FUNC, 2);
        qfunc.register_superclass(mono(FUNC), &func);
        self.register_builtin_type(Never, never, vis.clone(), Const, Some(NEVER));
        self.register_builtin_type(Obj, obj, vis.clone(), Const, Some(FUNC_OBJECT));
        // self.register_type(mono(RECORD), vec![], record, Visibility::BUILTIN_PRIVATE, Const);
        let name = if PYTHON_MODE { FUNC_INT } else { INT };
        self.register_builtin_type(Int, int, vis.clone(), Const, Some(name));
        self.register_builtin_type(Nat, nat, vis.clone(), Const, Some(NAT));
        let name = if PYTHON_MODE { FUNC_FLOAT } else { FLOAT };
        self.register_builtin_type(Complex, complex, vis.clone(), Const, Some(COMPLEX));
        self.register_builtin_type(Float, float, vis.clone(), Const, Some(name));
        self.register_builtin_type(Ratio, ratio, vis.clone(), Const, Some(RATIO));
        let name = if PYTHON_MODE { FUNC_BOOL } else { BOOL };
        self.register_builtin_type(Bool, bool_, vis.clone(), Const, Some(name));
        let name = if PYTHON_MODE { FUNC_STR } else { STR };
        self.register_builtin_type(Str, str_, vis.clone(), Const, Some(name));
        self.register_builtin_type(NoneType, nonetype, vis.clone(), Const, Some(NONE_TYPE));
        self.register_builtin_type(Type, type_, vis.clone(), Const, Some(FUNC_TYPE));
        self.register_builtin_type(ClassType, class_type, vis.clone(), Const, Some(CLASS_TYPE));
        self.register_builtin_type(TraitType, trait_type, vis.clone(), Const, Some(TRAIT_TYPE));
        self.register_builtin_type(Code, code, vis.clone(), Const, Some(CODE_TYPE));
        self.register_builtin_type(
            g_module_t,
            generic_module,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(MODULE_TYPE),
        );
        self.register_builtin_type(
            py_module_t,
            py_module,
            vis.clone(),
            Const,
            Some(MODULE_TYPE),
        );
        self.register_builtin_type(
            mono(GENERIC_ARRAY),
            generic_array,
            vis.clone(),
            Const,
            Some(ARRAY),
        );
        self.register_builtin_type(arr_t, array_, vis.clone(), Const, Some(ARRAY));
        self.register_builtin_type(
            mono(GENERIC_SET),
            generic_set,
            vis.clone(),
            Const,
            Some(SET),
        );
        self.register_builtin_type(set_t, set_, vis.clone(), Const, Some(SET));
        self.register_builtin_type(g_dict_t, generic_dict, vis.clone(), Const, Some(DICT));
        self.register_builtin_type(dict_t, dict_, vis.clone(), Const, Some(DICT));
        self.register_builtin_type(mono(BYTES), bytes, vis.clone(), Const, Some(BYTES));
        self.register_builtin_type(
            mono(GENERIC_TUPLE),
            generic_tuple,
            vis.clone(),
            Const,
            Some(FUNC_TUPLE),
        );
        self.register_builtin_type(_tuple_t, tuple_, vis.clone(), Const, Some(FUNC_TUPLE));
        self.register_builtin_type(mono(RECORD), record, vis.clone(), Const, Some(RECORD));
        self.register_builtin_type(or_t, or, vis.clone(), Const, Some(UNION));
        self.register_builtin_type(
            mono(STR_ITERATOR),
            str_iterator,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_STR_ITERATOR),
        );
        self.register_builtin_type(
            poly(ARRAY_ITERATOR, vec![ty_tp(T.clone())]),
            array_iterator,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_ARRAY_ITERATOR),
        );
        self.register_builtin_type(
            poly(SET_ITERATOR, vec![ty_tp(T.clone())]),
            set_iterator,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_SET_ITERATOR),
        );
        self.register_builtin_type(
            poly(TUPLE_ITERATOR, vec![ty_tp(T.clone())]),
            tuple_iterator,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_TUPLE_ITERATOR),
        );
        self.register_builtin_type(
            poly(RANGE_ITERATOR, vec![ty_tp(T.clone())]),
            range_iterator,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(RANGE_ITERATOR),
        );
        self.register_builtin_type(
            poly(DICT_KEYS, vec![ty_tp(T.clone())]),
            dict_keys,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_DICT_KEYS),
        );
        self.register_builtin_type(
            poly(DICT_VALUES, vec![ty_tp(T.clone())]),
            dict_values,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_DICT_VALUES),
        );
        self.register_builtin_type(
            poly(DICT_ITEMS, vec![ty_tp(T.clone())]),
            dict_items,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_DICT_ITEMS),
        );
        self.register_builtin_type(
            poly(ENUMERATE, vec![ty_tp(T.clone())]),
            enumerate,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_ENUMERATE),
        );
        self.register_builtin_type(
            poly(FILTER, vec![ty_tp(T.clone())]),
            filter,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_FILTER),
        );
        self.register_builtin_type(
            poly(MAP, vec![ty_tp(T.clone())]),
            map,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_MAP),
        );
        self.register_builtin_type(
            poly(REVERSED, vec![ty_tp(T.clone())]),
            reversed,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_REVERSED),
        );
        self.register_builtin_type(
            poly(ZIP, vec![ty_tp(T), ty_tp(U)]),
            zip,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_ZIP),
        );
        self.register_builtin_type(
            fset_t,
            frozenset,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_FROZENSET),
        );
        self.register_builtin_type(
            memview_t,
            memoryview,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(MEMORYVIEW),
        );
        self.register_builtin_type(mono(MUT_FILE), file_mut, vis.clone(), Const, Some(FILE));
        self.register_builtin_type(array_mut_t, array_mut_, vis.clone(), Const, Some(ARRAY));
        self.register_builtin_type(dict_mut_t, dict_mut, vis.clone(), Const, Some(DICT));
        self.register_builtin_type(set_mut_t, set_mut_, vis.clone(), Const, Some(SET));
        self.register_builtin_type(
            mono(GENERIC_CALLABLE),
            g_callable,
            vis.clone(),
            Const,
            Some(CALLABLE),
        );
        self.register_builtin_type(
            mono(GENERIC_GENERATOR),
            g_generator,
            vis.clone(),
            Const,
            Some(GENERATOR),
        );
        self.register_builtin_type(mono(PROC), proc, vis.clone(), Const, Some(PROC));
        self.register_builtin_type(mono(FUNC), func, vis.clone(), Const, Some(FUNC));
        self.register_builtin_type(range_t, range, vis.clone(), Const, Some(FUNC_RANGE));
        if ERG_MODE {
            self.register_builtin_type(module_t, module, vis.clone(), Const, Some(MODULE_TYPE));
            self.register_builtin_type(
                mono(MUTABLE_OBJ),
                obj_mut,
                vis.clone(),
                Const,
                Some(FUNC_OBJECT),
            );
            self.register_builtin_type(mono(MUT_INT), int_mut, vis.clone(), Const, Some(FUNC_INT));
            self.register_builtin_type(mono(MUT_NAT), nat_mut, vis.clone(), Const, Some(NAT));
            self.register_builtin_type(
                mono(MUT_FLOAT),
                float_mut,
                vis.clone(),
                Const,
                Some(FUNC_FLOAT),
            );
            self.register_builtin_type(mono(MUT_RATIO), ratio_mut, vis.clone(), Const, Some(RATIO));
            self.register_builtin_type(mono(MUT_BOOL), bool_mut, vis.clone(), Const, Some(BOOL));
            self.register_builtin_type(mono(MUT_STR), str_mut, vis, Const, Some(STR));
            self.register_builtin_type(
                mono(NAMED_PROC),
                named_proc,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(NAMED_PROC),
            );
            self.register_builtin_type(
                mono(NAMED_FUNC),
                named_func,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(NAMED_FUNC),
            );
            self.register_builtin_type(
                mono(QUANTIFIED),
                quant,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(QUANTIFIED),
            );
            self.register_builtin_type(
                mono(QUANTIFIED_FUNC),
                qfunc,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(QUANTIFIED_FUNC),
            );
        }
    }
}
