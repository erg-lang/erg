use erg_common::consts::{ERG_MODE, PYTHON_MODE};
use erg_common::fresh::FRESH_GEN;
#[allow(unused_imports)]
use erg_common::log;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{IntervalOp, Type, Visibility};
use ParamSpec as PS;
use Type::*;

use crate::context::initialize::*;
use crate::context::{Context, ParamSpec};
use crate::varinfo::Mutability;
use Mutability::*;

impl Context {
    // NOTE: Registering traits that a class implements requires type checking,
    // which means that registering a class requires that the preceding types have already been registered,
    // so `register_builtin_type` should be called as early as possible.
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
        self.register_builtin_type(Never, never, vis.clone(), Const, Some(NEVER));
        /* Obj */
        let mut obj = Self::builtin_mono_class(OBJ, 2);
        obj.register_py_builtin(
            FUNDAMENTAL_DICT,
            dict! {Str => Obj}.into(),
            Some(FUNDAMENTAL_DICT),
            3,
        );
        obj.register_py_builtin(FUNDAMENTAL_MODULE, Str, Some(FUNDAMENTAL_MODULE), 4);
        obj.register_py_builtin(
            FUNDAMENTAL_HASH,
            fn0_met(Obj, Int),
            Some(FUNDAMENTAL_HASH),
            5,
        );
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
        obj.register_py_builtin(FUNDAMENTAL_CLASS, ClassType, Some(FUNDAMENTAL_CLASS), 10);
        obj.register_py_builtin(FUNDAMENTAL_DOC, ClassType, Some(FUNDAMENTAL_DOC), 11);
        obj.register_py_builtin(
            FUNDAMENTAL_DIR,
            fn0_met(Obj, unknown_len_list_t(Str)),
            Some(FUNDAMENTAL_DIR),
            12,
        );
        obj.register_py_builtin(
            FUNDAMENTAL_GETATTRIBUTE,
            fn1_met(Obj, Str, Obj),
            Some(FUNDAMENTAL_GETATTRIBUTE),
            13,
        );
        obj.register_py_builtin(
            FUNDAMENTAL_FORMAT,
            fn1_met(Obj, Str, Str),
            Some(FUNDAMENTAL_FORMAT),
            14,
        );
        // Obj does not implement Eq
        let mut complex = Self::builtin_mono_class(COMPLEX, 2);
        complex.register_superclass(Obj, &obj);
        // TODO: support multi platform
        complex.register_builtin_const(
            EPSILON,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::from(2.220446049250313e-16),
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
        let t = no_var_func(vec![], vec![kw(REAL, Float), kw(IMAG, Float)], Complex)
            & no_var_func(vec![kw(KW_OBJECT, Str | Float)], vec![], Complex);
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
        let mut float_to_int = Self::builtin_methods(Some(mono(TO_INT)), 1);
        float_to_int.register_builtin_erg_impl(
            FUNDAMENTAL_INT,
            fn0_met(Float, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait_methods(Float, float_to_int);
        let mut float_to_float = Self::builtin_methods(Some(mono(TO_FLOAT)), 1);
        float_to_float.register_builtin_erg_impl(
            FUNDAMENTAL_FLOAT,
            fn0_met(Float, Float),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait_methods(Float, float_to_float);
        let mut float_to_bool = Self::builtin_methods(Some(mono(TO_BOOL)), 1);
        float_to_bool.register_builtin_erg_impl(
            FUNDAMENTAL_BOOL,
            fn0_met(Float, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait_methods(Float, float_to_bool);
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
        let t_nearly_eq = fn_met(
            Float,
            vec![kw(KW_OTHER, Float)],
            None,
            vec![kw(KW_EPSILON, Float)],
            None,
            Bool,
        );
        float.register_builtin_erg_impl(
            FUNC_NEARLY_EQ,
            t_nearly_eq,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let t_call = func1(Obj, Float);
        float.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait(self, mono(NUM)).unwrap();
        let mut float_partial_ord = Self::builtin_methods(Some(mono(PARTIAL_ORD)), 2);
        float_partial_ord.register_builtin_erg_impl(
            OP_CMP,
            fn1_met(Float, Float, mono(ORDERING)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait_methods(Float, float_partial_ord);
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
            None,
            ValueObj::builtin_class(Float),
        );
        float.register_trait_methods(Float, float_add);
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
            None,
            ValueObj::builtin_class(Float),
        );
        float.register_trait_methods(Float, float_sub);
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
            None,
            ValueObj::builtin_class(Float),
        );
        float_mul.register_builtin_const(
            POW_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Float),
        );
        float.register_trait_methods(Float, float_mul);
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
            None,
            ValueObj::builtin_class(Float),
        );
        float_div.register_builtin_const(
            MOD_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Float),
        );
        float.register_trait_methods(Float, float_div);
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
            None,
            ValueObj::builtin_class(Float),
        );
        float.register_trait_methods(Float, float_floordiv);
        let mut float_pos = Self::builtin_methods(Some(mono(POS)), 2);
        float_pos.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Float),
        );
        float_pos.register_builtin_erg_impl(
            OP_POS,
            fn0_met(Float, Float),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait_methods(Float, float_pos);
        let mut float_neg = Self::builtin_methods(Some(mono(NEG)), 2);
        float_neg.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Float),
        );
        float_neg.register_builtin_erg_impl(
            OP_NEG,
            fn0_met(Float, Float),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        float.register_trait_methods(Float, float_neg);
        let mut float_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        float_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(mono(MUT_FLOAT)),
        );
        float.register_trait_methods(Float, float_mutizable);
        let mut float_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        let t = fn0_met(Float, Str);
        float_show.register_builtin_py_impl(
            FUNDAMENTAL_STR,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_STR),
        );
        float.register_trait_methods(Float, float_show);

        /* Ratio */
        // TODO: Int, Nat, Boolの継承元をRatioにする(今はFloat)
        let mut ratio = Self::builtin_mono_class(RATIO, 2);
        ratio.register_superclass(Obj, &obj);
        ratio.register_builtin_py_impl(REAL, Ratio, Const, Visibility::BUILTIN_PUBLIC, Some(REAL));
        ratio.register_builtin_py_impl(IMAG, Ratio, Const, Visibility::BUILTIN_PUBLIC, Some(IMAG));
        ratio.register_trait(self, mono(NUM)).unwrap();
        ratio.register_trait(self, mono(ORD)).unwrap();
        let mut ratio_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        ratio_ord.register_builtin_erg_impl(
            OP_CMP,
            fn1_met(Ratio, Ratio, mono(ORDERING)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio.register_trait_methods(Ratio, ratio_ord);
        let mut ratio_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        ratio_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Ratio, Ratio, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio.register_trait_methods(Ratio, ratio_eq);
        let mut ratio_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        ratio_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(Ratio, Nat),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio.register_trait_methods(Ratio, ratio_hash);
        ratio.register_trait(self, mono(EQ_HASH)).unwrap();
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
            None,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait_methods(Ratio, ratio_add);
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
            None,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait_methods(Ratio, ratio_sub);
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
            None,
            ValueObj::builtin_class(Ratio),
        );
        ratio_mul.register_builtin_const(
            POW_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait_methods(Ratio, ratio_mul);
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
            None,
            ValueObj::builtin_class(Ratio),
        );
        ratio_div.register_builtin_const(
            MOD_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait_methods(Ratio, ratio_div);
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
            None,
            ValueObj::builtin_class(Ratio),
        );
        ratio.register_trait_methods(Ratio, ratio_floordiv);
        let mut ratio_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        ratio_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(mono(MUT_RATIO)),
        );
        ratio.register_trait_methods(Ratio, ratio_mutizable);
        let mut ratio_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        let t = fn0_met(Ratio, Str);
        ratio_show.register_builtin_erg_impl(
            FUNDAMENTAL_STR,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio.register_trait_methods(Ratio, ratio_show);

        /* Int */
        let mut int = Self::builtin_mono_class(INT, 2);
        int.register_superclass(Float, &float); // TODO: Float -> Ratio
        int.register_trait(self, mono(NUM)).unwrap();
        // class("Rational"),
        // class("Integral"),
        let i_abs = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ABS,
            int_abs,
            fn0_met(Int, Nat),
            None,
        )));
        int.register_py_builtin_const(
            FUNC_ABS,
            Visibility::BUILTIN_PUBLIC,
            Some(fn0_met(Int, Nat)),
            i_abs,
            Some(OP_ABS),
            Some(11),
        );
        int.register_py_builtin(FUNC_SUCC, fn0_met(Int, Int), Some(FUNC_SUCC), 54);
        int.register_py_builtin(FUNC_PRED, fn0_met(Int, Int), Some(FUNC_PRED), 47);
        int.register_py_builtin(
            FUNC_BIT_LENGTH,
            fn0_met(Int, Nat),
            Some(FUNC_BIT_LENGTH),
            38,
        );
        int.register_py_builtin(FUNC_BIT_COUNT, fn0_met(Int, Nat), Some(FUNC_BIT_COUNT), 27);
        let t_from_bytes = no_var_func(
            vec![kw(
                BYTES,
                or(
                    mono(BYTES),
                    list_t(Type::from(value(0)..=value(255)), TyParam::erased(Nat)),
                ),
            )],
            vec![kw(
                FUNC_BYTEORDER,
                v_enum(
                    set! {ValueObj::Str(TOKEN_BIG_ENDIAN.into()), ValueObj::Str(TOKEN_LITTLE_ENDIAN.into())},
                ),
            )],
            Int,
        );
        int.register_py_builtin(FUNC_FROM_BYTES, t_from_bytes, Some(FUNC_FROM_BYTES), 40);
        let t_to_bytes = no_var_func(
            vec![kw(KW_SELF, Int)],
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
        let t_call = no_var_func(vec![pos(Obj)], vec![kw(KW_BASE, Nat)], Int);
        int.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let t_lshift = fn1_met(Int, Nat, Int);
        int.register_builtin_erg_impl(OP_LSHIFT, t_lshift, Const, Visibility::BUILTIN_PUBLIC);
        let t_rshift = fn1_met(Int, Nat, Int);
        int.register_builtin_erg_impl(OP_RSHIFT, t_rshift, Const, Visibility::BUILTIN_PUBLIC);
        let t_or = fn1_met(Int, Int, Int);
        int.register_builtin_erg_impl(OP_OR, t_or, Const, Visibility::BUILTIN_PUBLIC);
        let t_and = fn1_met(Int, Int, Int);
        int.register_builtin_erg_impl(OP_AND, t_and, Const, Visibility::BUILTIN_PUBLIC);
        let mut int_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        int_ord.register_builtin_erg_impl(
            OP_PARTIAL_CMP,
            fn1_met(Int, Int, or(mono(ORDERING), NoneType)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait_methods(Int, int_ord);
        let mut int_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        int_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Int, Int, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait_methods(Int, int_eq);
        let mut int_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        int_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(Int, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait_methods(Int, int_hash);
        int.register_trait(self, mono(EQ_HASH)).unwrap();
        // __div__ is not included in Int (cast to Ratio)
        let op_t = fn1_met(Int, Int, Int);
        let mut int_add = Self::builtin_methods(Some(poly(ADD, vec![ty_tp(Int)])), 2);
        int_add.register_builtin_erg_impl(OP_ADD, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        int_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Int),
        );
        int.register_trait_methods(Int, int_add);
        let mut int_sub = Self::builtin_methods(Some(poly(SUB, vec![ty_tp(Int)])), 2);
        int_sub.register_builtin_erg_impl(OP_SUB, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        int_sub.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Int),
        );
        int.register_trait_methods(Int, int_sub);
        let mut int_mul = Self::builtin_methods(Some(poly(MUL, vec![ty_tp(Int)])), 2);
        int_mul.register_builtin_erg_impl(OP_MUL, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        int_mul.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Int),
        );
        int_mul.register_builtin_const(
            POW_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Nat),
        );
        int.register_trait_methods(Int, int_mul);
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
            None,
            ValueObj::builtin_class(Int),
        );
        int.register_trait_methods(Int, int_floordiv);
        // needed for implementing `%` operator
        let mut int_div = Self::builtin_methods(Some(poly(DIV, vec![ty_tp(Int)])), 2);
        int_div.register_builtin_erg_impl(
            OP_DIV,
            fn1_met(Int, Int, Float),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int_div.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Float),
        );
        int_div.register_builtin_const(
            MOD_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Int),
        );
        int.register_trait_methods(Int, int_div);
        let mut int_pos = Self::builtin_methods(Some(mono(POS)), 2);
        int_pos.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Int),
        );
        int_pos.register_builtin_erg_impl(
            OP_POS,
            fn0_met(Int, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait_methods(Int, int_pos);
        let mut int_neg = Self::builtin_methods(Some(mono(NEG)), 2);
        int_neg.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Int),
        );
        int_neg.register_builtin_erg_impl(
            OP_NEG,
            fn0_met(Int, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        int.register_trait_methods(Int, int_neg);
        let mut int_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        int_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(mono(MUT_INT)),
        );
        int.register_trait_methods(Int, int_mutizable);
        let mut int_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        let t = fn0_met(Int, Str);
        int_show.register_builtin_py_impl(
            FUNDAMENTAL_STR,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_STR),
        );
        int.register_trait_methods(Int, int_show);
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
        let t_call = no_var_func(vec![pos(Obj)], vec![kw(KW_BASE, Nat)], Nat);
        nat.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        nat.register_trait(self, mono(NUM)).unwrap();
        let mut nat_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        nat_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Nat, Nat, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        nat.register_trait_methods(Nat, nat_eq);
        let mut nat_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        nat_ord.register_builtin_erg_impl(
            OP_CMP,
            fn1_met(Nat, Nat, mono(ORDERING)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        nat.register_trait_methods(Nat, nat_ord);
        // __sub__, __div__ is not included in Nat (cast to Int/ Ratio)
        let op_t = fn1_met(Nat, Nat, Nat);
        let mut nat_add = Self::builtin_methods(Some(poly(ADD, vec![ty_tp(Nat)])), 2);
        nat_add.register_builtin_erg_impl(OP_ADD, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        nat_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Nat),
        );
        nat.register_trait_methods(Nat, nat_add);
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
            None,
            ValueObj::builtin_class(Int),
        );
        nat.register_trait_methods(Nat, nat_sub);
        let mut nat_mul = Self::builtin_methods(Some(poly(MUL, vec![ty_tp(Nat)])), 2);
        nat_mul.register_builtin_erg_impl(OP_MUL, op_t.clone(), Const, Visibility::BUILTIN_PUBLIC);
        nat_mul.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Nat),
        );
        nat.register_trait_methods(Nat, nat_mul);
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
            None,
            ValueObj::builtin_class(Nat),
        );
        nat.register_trait_methods(Nat, nat_floordiv);
        // needed for implementing `%` operator
        let mut nat_div = Self::builtin_methods(Some(poly(DIV, vec![ty_tp(Nat)])), 2);
        nat_div.register_builtin_erg_impl(
            OP_DIV,
            fn1_met(Nat, Nat, Float),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        nat_div.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Float),
        );
        nat_div.register_builtin_const(
            MOD_OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Nat),
        );
        nat.register_trait_methods(Nat, nat_div);
        let mut nat_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        nat_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(mono(MUT_NAT)),
        );
        nat.register_trait_methods(Nat, nat_mutizable);
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
        bool_.register_trait(self, mono(NUM)).unwrap();
        let mut bool_ord = Self::builtin_methods(Some(mono(ORD)), 2);
        bool_ord.register_builtin_erg_impl(
            OP_CMP,
            fn1_met(Bool, Bool, mono(ORDERING)),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_.register_trait_methods(Bool, bool_ord);
        let mut bool_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        bool_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Bool, Bool, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_.register_trait_methods(Bool, bool_eq);
        let mut bool_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        bool_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(mono(MUT_BOOL)),
        );
        bool_.register_trait_methods(Bool, bool_mutizable);
        let mut bool_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        bool_show.register_builtin_erg_impl(
            FUNDAMENTAL_STR,
            fn0_met(Bool, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_.register_trait_methods(Bool, bool_show);
        let t = fn0_met(Bool, Bool);
        bool_.register_py_builtin(FUNC_INVERT, t, Some(FUNC_INVERT), 9);
        /* Str */
        let mut str_ = Self::builtin_mono_class(STR, 10);
        str_.register_superclass(Obj, &obj);
        str_.register_py_builtin(OP_GT, fn1_met(Str, Str, Bool), Some(OP_GT), 0);
        str_.register_py_builtin(OP_GE, fn1_met(Str, Str, Bool), Some(OP_GE), 0);
        str_.register_py_builtin(OP_LT, fn1_met(Str, Str, Bool), Some(OP_LT), 0);
        str_.register_py_builtin(OP_LE, fn1_met(Str, Str, Bool), Some(OP_LE), 0);
        if PYTHON_MODE {
            str_.register_py_builtin(OP_MOD, fn1_met(Str, Obj, Str), Some(OP_MOD), 0);
        }
        str_.register_trait(self, mono(ORD)).unwrap();
        str_.register_trait(self, mono(PATH_LIKE)).unwrap();
        let t_s_replace = fn_met(
            Str,
            vec![kw(KW_PAT, Str), kw(KW_INTO, Str)],
            None,
            vec![kw(KW_COUNT, Int)],
            None,
            Str,
        );
        let s_replace = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_REPLACE,
            str_replace,
            t_s_replace.clone(),
            None,
        )));
        str_.register_builtin_const(
            FUNC_REPLACE,
            Visibility::BUILTIN_PUBLIC,
            Some(t_s_replace),
            s_replace,
        );
        str_.register_py_builtin(
            FUNC_ENCODE,
            fn_met(
                Str,
                vec![],
                None,
                vec![kw(KW_ENCODING, Str), kw(KW_ERRORS, Str)],
                None,
                mono(BYTES),
            ),
            Some(FUNC_ENCODE),
            60,
        );
        str_.register_py_builtin(FUNC_CASEFOLD, fn0_met(Str, Str), Some(FUNC_CASEFOLD), 23);
        str_.register_py_builtin(
            FUNC_CENTER,
            fn_met(
                Str,
                vec![kw(KW_WIDTH, Nat)],
                None,
                vec![kw(KW_FILLCHAR, Str)],
                None,
                Str,
            ),
            Some(FUNC_CENTER),
            33,
        );
        str_.register_builtin_erg_impl(
            FUNC_FORMAT,
            fn_met(
                Str,
                vec![],
                Some(kw(KW_ARGS, Obj)),
                vec![],
                Some(kw(KW_KWARGS, Obj)),
                Str,
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_builtin_erg_impl(
            FUNC_LOWER,
            fn_met(Str, vec![], None, vec![], None, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_builtin_erg_impl(
            FUNC_UPPER,
            fn_met(Str, vec![], None, vec![], None, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_builtin_erg_impl(
            FUNC_TO_INT,
            fn_met(Str, vec![], None, vec![], None, or(Int, NoneType)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let t_s_startswith = fn1_met(Str, Str, Bool);
        let s_startswith = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_STARTSWITH,
            str_startswith,
            t_s_startswith.clone(),
            None,
        )));
        str_.register_py_builtin_const(
            FUNC_STARTSWITH,
            Visibility::BUILTIN_PUBLIC,
            Some(t_s_startswith),
            s_startswith,
            Some(FUNC_STARTSWITH),
            None,
        );
        let t_s_endswith = fn1_met(Str, Str, Bool);
        let s_endswith = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ENDSWITH,
            str_endswith,
            t_s_endswith.clone(),
            None,
        )));
        str_.register_py_builtin_const(
            FUNC_ENDSWITH,
            Visibility::BUILTIN_PUBLIC,
            Some(t_s_endswith),
            s_endswith,
            Some(FUNC_ENDSWITH),
            Some(69),
        );
        str_.register_builtin_py_impl(
            FUNC_SPLIT,
            fn_met(
                Str,
                vec![kw(KW_SEP, Str)],
                None,
                vec![kw(KW_MAXSPLIT, Nat)],
                None,
                unknown_len_list_t(Str),
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
                None,
                unknown_len_list_t(Str),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_SPLITLINES),
        );
        let t_s_join = fn1_met(Str, poly(ITERABLE, vec![ty_tp(Str)]), Str);
        let s_join = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_JOIN,
            str_join,
            t_s_join.clone(),
            None,
        )));
        str_.register_py_builtin_const(
            FUNC_JOIN,
            Visibility::BUILTIN_PUBLIC,
            Some(t_s_join),
            s_join,
            Some(FUNC_JOIN),
            None,
        );
        str_.register_py_builtin(
            FUNC_INDEX,
            fn_met(
                Str,
                vec![kw(KW_SUB, Str)],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                None,
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
                None,
                or(Nat, Never),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_RINDEX),
        );
        let t_s_find = fn_met(
            Str,
            vec![kw(KW_SUB, Str)],
            None,
            vec![kw(KW_START, Nat), kw(KW_END, Nat)],
            None,
            or(Nat, v_enum(set! {(-1).into()})),
        );
        let s_find = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_FIND,
            str_find,
            t_s_find.clone(),
            None,
        )));
        str_.register_py_builtin_const(
            FUNC_FIND,
            Visibility::BUILTIN_PUBLIC,
            Some(t_s_find),
            s_find,
            Some(FUNC_FIND),
            Some(93),
        );
        str_.register_builtin_py_impl(
            FUNC_RFIND,
            fn_met(
                Str,
                vec![kw(KW_SUB, Str)],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                None,
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
                None,
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
        str_.register_builtin_py_impl(
            FUNC_STRIP,
            fn_met(
                Str,
                vec![],
                None,
                vec![kw(KW_CHARS, Str | NoneType)],
                None,
                Str,
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_STRIP),
        );
        str_.register_builtin_py_impl(
            FUNC_REMOVEPREFIX,
            fn1_met(Str, Str, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_REMOVEPREFIX),
        );
        str_.register_builtin_py_impl(
            FUNC_REMOVESUFFIX,
            fn1_met(Str, Str, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_REMOVESUFFIX),
        );
        str_.register_builtin_py_impl(
            FUNC_ISALNUM,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISALNUM),
        );
        let t_s_isalpha = fn0_met(Str, Bool);
        let s_isalpha = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ISALPHA,
            str_isalpha,
            t_s_isalpha.clone(),
            None,
        )));
        str_.register_py_builtin_const(
            FUNC_ISALPHA,
            Visibility::BUILTIN_PUBLIC,
            Some(t_s_isalpha),
            s_isalpha,
            Some(FUNC_ISALPHA),
            None,
        );
        let t_s_isascii = fn0_met(Str, Bool);
        let s_isascii = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ISASCII,
            str_isascii,
            t_s_isascii.clone(),
            None,
        )));
        str_.register_py_builtin_const(
            FUNC_ISASCII,
            Visibility::BUILTIN_PUBLIC,
            Some(t_s_isascii),
            s_isascii,
            Some(FUNC_ISASCII),
            None,
        );
        let s_t_isdecimal = fn0_met(Str, Bool);
        let s_isdecimal = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ISDECIMAL,
            str_isdecimal,
            s_t_isdecimal.clone(),
            None,
        )));
        str_.register_py_builtin_const(
            FUNC_ISDECIMAL,
            Visibility::BUILTIN_PUBLIC,
            Some(s_t_isdecimal),
            s_isdecimal,
            Some(FUNC_ISDECIMAL),
            None,
        );
        str_.register_builtin_py_impl(
            FUNC_ISDIGIT,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISDIGIT),
        );
        str_.register_builtin_py_impl(
            FUNC_ISLOWER,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISLOWER),
        );
        str_.register_builtin_py_impl(
            FUNC_ISNUMERIC,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISNUMERIC),
        );
        str_.register_builtin_py_impl(
            FUNC_ISSPACE,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISSPACE),
        );
        str_.register_builtin_py_impl(
            FUNC_ISTITLE,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISTITLE),
        );
        str_.register_builtin_py_impl(
            FUNC_ISUPPER,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISUPPER),
        );
        str_.register_builtin_py_impl(
            FUNC_ISIDENTIFIER,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISIDENTIFIER),
        );
        str_.register_builtin_py_impl(
            FUNC_ISPRINTABLE,
            fn0_met(Str, Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISPRINTABLE),
        );
        str_.register_builtin_py_impl(
            FUNC_FROM,
            no_var_fn_met(Str, vec![kw(KW_NTH, Nat)], vec![], Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_FROM_),
        );
        let idx_t = if PYTHON_MODE {
            Int | poly(RANGE, vec![ty_tp(Int)]) | mono(SLICE)
        } else {
            Nat | poly(RANGE, vec![ty_tp(Int)]) | mono(SLICE)
        };
        let str_getitem_t = fn1_kw_met(Str, kw(KW_IDX, idx_t), Str);
        str_.register_builtin_erg_impl(
            FUNDAMENTAL_GETITEM,
            str_getitem_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_trait(self, poly(INDEXABLE, vec![ty_tp(Nat), ty_tp(Str)]))
            .unwrap();
        let t_call = func(vec![], None, vec![kw(KW_OBJECT, Obj)], None, Str)
            & no_var_func(
                vec![kw(KW_BYTES_OR_BUFFER, mono(BYTES)), kw(KW_ENCODING, Str)],
                vec![kw(KW_ERRORS, Str)],
                Str,
            );
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
        str_.register_trait_methods(Str, str_eq);
        let mut str_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        str_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(Str, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_trait_methods(Str, str_hash);
        str_.register_trait(self, mono(EQ_HASH)).unwrap();
        let mut str_seq = Self::builtin_methods(Some(poly(SEQUENCE, vec![ty_tp(Str)])), 2);
        str_seq.register_builtin_erg_impl(
            FUNDAMENTAL_LEN,
            fn0_met(Str, Nat),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_trait_methods(Str, str_seq);
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
            None,
            ValueObj::builtin_class(Str),
        );
        str_.register_trait_methods(Str, str_add);
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
            None,
            ValueObj::builtin_class(Str),
        );
        str_.register_trait_methods(Str, str_mul);
        let mut str_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        str_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(mono(MUT_STR)),
        );
        str_.register_trait_methods(Str, str_mutizable);
        let mut str_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        str_show.register_builtin_erg_impl(
            FUNDAMENTAL_STR,
            fn0_met(Str, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_trait_methods(Str, str_show);
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
            None,
            ValueObj::builtin_class(mono(STR_ITERATOR)),
        );
        str_.register_trait_methods(Str, str_iterable);
        let mut str_collection = Self::builtin_methods(Some(poly(COLLECTION, vec![ty_tp(Str)])), 4);
        str_collection.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            fn1_met(Str, Str, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        str_.register_trait_methods(Str, str_collection);
        str_.register_trait(self, poly(COLLECTION, vec![ty_tp(Str)]))
            .unwrap();
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
        nonetype.register_trait_methods(NoneType, nonetype_eq);
        let mut nonetype_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        nonetype_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(NoneType, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        nonetype.register_trait_methods(NoneType, nonetype_hash);
        nonetype.register_trait(self, mono(EQ_HASH)).unwrap();
        let mut nonetype_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        nonetype_show.register_builtin_erg_impl(
            FUNDAMENTAL_STR,
            fn0_met(NoneType, Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        nonetype.register_trait_methods(NoneType, nonetype_show);
        /* Type */
        let mut type_ = Self::builtin_mono_class(TYPE, 2);
        type_.register_superclass(Obj, &obj);
        type_.register_builtin_erg_impl(
            FUNC_MRO,
            fn0_met(Type, list_t(Type, TyParam::erased(Nat))),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        // TODO: PolyType
        type_.register_builtin_erg_impl(
            FUNDAMENTAL_ARGS,
            list_t(Type, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let t = if PYTHON_MODE { Type | NoneType } else { Type };
        type_.register_builtin_erg_impl(
            OP_OR,
            fn1_met(t.clone(), t.clone(), Type),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        type_.register_builtin_erg_impl(
            OP_AND,
            fn1_met(t.clone(), t.clone(), Type),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        type_.register_trait(self, mono(NAMED)).unwrap();
        let mut type_container = Self::builtin_methods(Some(poly(CONTAINER, vec![ty_tp(Obj)])), 2);
        type_container.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            fn1_met(Type, Obj, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        type_.register_trait_methods(Type, type_container);
        let mut type_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        type_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(Type, Type, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        type_.register_trait_methods(Type, type_eq);
        let mut type_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        type_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(Type, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        type_.register_trait_methods(Type, type_hash);
        let mut class_type = Self::builtin_mono_class(CLASS_TYPE, 2);
        class_type.register_superclass(Type, &type_);
        class_type.register_trait(self, mono(NAMED)).unwrap();
        let mut class_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        class_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(ClassType, ClassType, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        class_type.register_trait_methods(ClassType, class_eq);
        let mut trait_type = Self::builtin_mono_class(TRAIT_TYPE, 2);
        trait_type.register_superclass(Type, &type_);
        trait_type.register_trait(self, mono(NAMED)).unwrap();
        let mut trait_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        trait_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(TraitType, TraitType, Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        trait_type.register_trait_methods(TraitType, trait_eq);
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
            list_t(Str, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_CONSTS,
            list_t(Obj, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_NAMES,
            list_t(Str, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_FREEVARS,
            list_t(Str, TyParam::erased(Nat)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_builtin_erg_impl(
            FUNC_CO_CELLVARS,
            list_t(Str, TyParam::erased(Nat)),
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
        code.register_trait_methods(Code, code_eq);
        let mut code_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        code_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(Code, Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        code.register_trait_methods(Code, code_hash);
        code.register_trait(self, mono(EQ_HASH)).unwrap();
        let mut frame = Self::builtin_mono_class(FRAME, 10);
        frame.register_builtin_erg_impl(
            F_BUILTINS,
            mono(GENERIC_DICT),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        frame.register_builtin_erg_impl(F_CODE, Code, Immutable, Visibility::BUILTIN_PUBLIC);
        frame.register_builtin_erg_impl(
            F_GLOBALS,
            mono(GENERIC_DICT),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        frame.register_builtin_erg_impl(F_LASTI, Nat, Immutable, Visibility::BUILTIN_PUBLIC);
        frame.register_builtin_erg_impl(F_LINENO, Nat, Immutable, Visibility::BUILTIN_PUBLIC);
        frame.register_builtin_erg_impl(
            F_LOCALS,
            mono(GENERIC_DICT),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let g_module_t = mono(GENERIC_MODULE);
        let mut generic_module = Self::builtin_mono_class(GENERIC_MODULE, 2);
        generic_module.register_superclass(Obj, &obj);
        generic_module.register_trait(self, mono(NAMED)).unwrap();
        let mut generic_module_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        generic_module_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(g_module_t.clone(), g_module_t.clone(), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_module.register_trait_methods(g_module_t.clone(), generic_module_eq);
        let mut generic_module_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        generic_module_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(g_module_t.clone(), Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_module.register_trait_methods(g_module_t.clone(), generic_module_hash);
        generic_module.register_trait(self, mono(EQ_HASH)).unwrap();
        let Path = mono_q_tp(PATH, instanceof(Str));
        let module_t = module(Path.clone());
        let py_module_t = py_module(Path);
        let mut module = Self::builtin_poly_class(MODULE, vec![PS::named_nd(PATH, Str)], 2);
        module.register_superclass(g_module_t.clone(), &generic_module);
        let mut py_module = Self::builtin_poly_class(PY_MODULE, vec![PS::named_nd(PATH, Str)], 2);
        if ERG_MODE {
            py_module.register_superclass(g_module_t.clone(), &generic_module);
        }
        /* GenericList */
        let mut generic_list = Self::builtin_mono_class(GENERIC_LIST, 1);
        generic_list.register_superclass(Obj, &obj);
        let mut list_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        list_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(mono(GENERIC_LIST), mono(GENERIC_LIST), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_list.register_trait_methods(mono(GENERIC_LIST), list_eq);
        let t_call = func1(
            poly(ITERABLE, vec![ty_tp(T.clone())]),
            list_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        generic_list.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut list_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        list_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(mono(GENERIC_LIST), Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_list.register_trait_methods(mono(GENERIC_LIST), list_hash);
        let unsized_list_t = poly(UNSIZED_LIST, vec![ty_tp(T.clone())]);
        let mut unsized_list =
            Self::builtin_poly_class(UNSIZED_LIST, vec![ParamSpec::t_nd(TY_T)], 1);
        unsized_list.register_superclass(Obj, &obj);
        unsized_list.register_builtin_decl(KW_ELEM, T.clone(), vis.clone(), Some(KW_ELEM));
        /* List */
        let mut list_ =
            Self::builtin_poly_class(LIST, vec![PS::t_nd(TY_T), PS::default(TY_N, Nat)], 10);
        list_.register_superclass(mono(GENERIC_LIST), &generic_list);
        list_
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let lis_t = list_t(T.clone(), N.clone());
        let t = no_var_fn_met(
            lis_t.clone(),
            vec![kw(KW_RHS, list_t(T.clone(), M.clone()))],
            vec![],
            list_t(T.clone(), N.clone() + M.clone()),
        )
        .quantify();
        list_.register_py_builtin(FUNC_CONCAT, t.clone(), Some(OP_ADD), 9);
        let t_count =
            no_var_fn_met(lis_t.clone(), vec![kw(KW_X, T.clone())], vec![], Nat).quantify();
        list_.register_py_builtin(FUNC_COUNT, t_count, Some(FUNC_COUNT), 17);
        let t_get = no_var_fn_met(
            lis_t.clone(),
            vec![pos(Nat)],
            vec![ParamTy::kw_default(KW_DEFAULT.into(), U.clone(), NoneType)],
            or(T.clone(), U.clone()),
        )
        .quantify();
        list_.register_builtin_erg_impl(FUNC_GET, t_get, Immutable, Visibility::BUILTIN_PUBLIC);
        let t_index = fn1_met(lis_t.clone(), T.clone(), Nat).quantify();
        list_.register_builtin_erg_impl(FUNC_INDEX, t_index, Immutable, Visibility::BUILTIN_PUBLIC);
        // List(T, N)|<: Add(List(T, M))|.
        //     Output = List(T, N + M)
        //     __add__: (self: List(T, N), other: List(T, M)) -> List(T, N + M) = List.concat
        let mut list_add = Self::builtin_methods(
            Some(poly(ADD, vec![ty_tp(list_t(T.clone(), M.clone()))])),
            2,
        );
        list_add.register_builtin_erg_impl(OP_ADD, t, Immutable, Visibility::BUILTIN_PUBLIC);
        let out_t = list_t(T.clone(), N.clone() + M.clone());
        list_add.register_builtin_const(
            OUTPUT,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(out_t),
        );
        list_.register_trait_methods(lis_t.clone(), list_add);
        let t = no_var_fn_met(
            lis_t.clone(),
            vec![kw(KW_ELEM, T.clone())],
            vec![],
            list_t(T.clone(), N.clone() + value(1usize)),
        )
        .quantify();
        list_.register_builtin_erg_impl(FUNC_PUSH, t, Immutable, Visibility::BUILTIN_PUBLIC);
        let repeat_t = no_var_fn_met(
            lis_t.clone(),
            vec![pos(singleton(Nat, M.clone()))],
            vec![],
            list_t(T.clone(), N.clone() * M.clone()),
        )
        .quantify();
        list_.register_builtin_erg_impl(
            FUNC_REPEAT,
            repeat_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        // [T; N].MutType! = [T; !N] (neither [T!; N] nor [T; N]!)
        let mut_type =
            ValueObj::builtin_class(poly(MUT_LIST, vec![TyParam::t(T.clone()), N.clone()]));
        let mut list_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        list_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            mut_type,
        );
        list_.register_trait_methods(lis_t.clone(), list_mutizable);
        let var = FRESH_GEN.fresh_varname();
        let input = if PYTHON_MODE {
            Int
        } else {
            refinement(
                var.clone(),
                Nat,
                Predicate::le(var, N.clone() - value(1usize)),
            )
        };
        // __getitem__: |T, N|(self: [T; N], _: {I: Nat | I <= N}) -> T
        //              and (self: [T; N], _: Range(Int) | Slice) -> [T; _]
        let list_getitem_t =
            (fn1_kw_met(list_t(T.clone(), N.clone()), anon(input.clone()), T.clone())
                & fn1_kw_met(
                    list_t(T.clone(), N.clone()),
                    anon(poly(RANGE, vec![ty_tp(Int)]) | mono(SLICE)),
                    unknown_len_list_t(T.clone()),
                ))
            .quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNDAMENTAL_GETITEM,
            __list_getitem__,
            list_getitem_t,
            None,
        )));
        list_.register_builtin_const(
            FUNDAMENTAL_GETITEM,
            Visibility::BUILTIN_PUBLIC,
            None,
            get_item,
        );
        let list_insert_t = no_var_fn_met(
            list_t(T.clone(), N.clone()),
            vec![pos(Nat), kw(KW_ELEM, T.clone())],
            vec![],
            list_t(T.clone(), N.clone() + value(1usize)),
        )
        .quantify();
        let list_insert = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_INSERT,
            list_insert_at,
            list_insert_t.clone(),
            None,
        )));
        list_._register_builtin_const(
            FUNC_INSERT,
            Visibility::BUILTIN_PUBLIC,
            Some(list_insert_t),
            list_insert,
            Some(FUNC_INSERT_AT.into()),
        );
        let list_remove_at_t = no_var_fn_met(
            list_t(T.clone(), N.clone()),
            vec![pos(Nat)],
            vec![],
            list_t(T.clone(), N.clone() - value(1usize)),
        )
        .quantify();
        let list_remove_at = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_REMOVE_AT,
            list_remove_at,
            list_remove_at_t.clone(),
            None,
        )));
        list_.register_builtin_const(
            FUNC_REMOVE_AT,
            Visibility::BUILTIN_PUBLIC,
            Some(list_remove_at_t),
            list_remove_at,
        );
        let list_remove_all_t = no_var_fn_met(
            list_t(T.clone(), N.clone()),
            vec![kw(KW_ELEM, T.clone())],
            vec![],
            list_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let list_remove_all = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_REMOVE_ALL,
            list_remove_all,
            list_remove_all_t.clone(),
            None,
        )));
        list_.register_builtin_const(
            FUNC_REMOVE_ALL,
            Visibility::BUILTIN_PUBLIC,
            Some(list_remove_all_t),
            list_remove_all,
        );
        let list_from = no_var_fn_met(
            unknown_len_list_t(T.clone()),
            vec![kw(KW_NTH, Nat)],
            vec![],
            unknown_len_list_t(T.clone()),
        )
        .quantify();
        list_.register_builtin_py_impl(
            FUNC_FROM,
            list_from,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_FROM_),
        );
        list_
            .register_trait(self, poly(INDEXABLE, vec![ty_tp(input), ty_tp(T.clone())]))
            .unwrap();
        list_
            .register_trait(
                self,
                poly(
                    HAS_SHAPE,
                    vec![ty_tp(lis_t.clone()).proj_call(FUNC_SHAPE.into(), vec![])],
                ),
            )
            .unwrap();
        list_
            .register_trait(
                self,
                poly(
                    HAS_SCALAR_TYPE,
                    vec![ty_tp(lis_t.clone()).proj_call(FUNC_SCALAR_TYPE.into(), vec![])],
                ),
            )
            .unwrap();
        let mut list_sized = Self::builtin_methods(Some(mono(SIZED)), 2);
        list_sized.register_builtin_erg_impl(
            FUNDAMENTAL_LEN,
            fn0_met(lis_t.clone(), Nat).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        list_.register_trait_methods(lis_t.clone(), list_sized);
        // union: (self: [Type; _]) -> Type
        let list_union_t = fn0_met(list_t(Type, TyParam::erased(Nat)), Type).quantify();
        let union = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_UNION,
            list_union,
            list_union_t,
            None,
        )));
        list_.register_builtin_const(FUNC_UNION, Visibility::BUILTIN_PUBLIC, None, union);
        // shape: (self: [Type; _]) -> [Nat; _]
        let list_shape_t =
            fn0_met(list_t(Type, TyParam::erased(Nat)), unknown_len_list_t(Nat)).quantify();
        let shape = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_SHAPE,
            list_shape,
            list_shape_t,
            None,
        )));
        list_.register_builtin_const(FUNC_SHAPE, Visibility::BUILTIN_PUBLIC, None, shape);
        let list_scalar_type_t = fn0_met(Type, Type).quantify();
        let list_scalar_type = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_SCALAR_TYPE,
            list_scalar_type,
            list_scalar_type_t,
            None,
        )));
        list_.register_builtin_const(
            FUNC_SCALAR_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            list_scalar_type,
        );
        let mut list_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        list_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(lis_t.clone(), lis_t.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        list_.register_trait_methods(lis_t.clone(), list_eq);
        list_
            .register_trait(self, poly(SEQUENCE, vec![ty_tp(T.clone())]))
            .unwrap();
        list_.unregister_trait(&poly(INDEXABLE, vec![ty_tp(Nat), ty_tp(T.clone())]));
        let mut list_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        list_show.register_builtin_py_impl(
            FUNDAMENTAL_STR,
            fn0_met(lis_t.clone(), Str).quantify(),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_STR),
        );
        list_.register_trait_methods(lis_t.clone(), list_show);
        let mut list_iterable =
            Self::builtin_methods(Some(poly(ITERABLE, vec![ty_tp(T.clone())])), 2);
        let list_iter = poly(LIST_ITERATOR, vec![ty_tp(T.clone())]);
        let t = fn0_met(list_t(T.clone(), TyParam::erased(Nat)), list_iter.clone()).quantify();
        list_iterable.register_builtin_py_impl(
            FUNC_ITER,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        list_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            None,
            ValueObj::builtin_class(list_iter),
        );
        list_.register_trait_methods(lis_t.clone(), list_iterable);
        let mut list_collection =
            Self::builtin_methods(Some(poly(COLLECTION, vec![ty_tp(T.clone())])), 4);
        list_collection.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            fn1_met(lis_t.clone(), T.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        list_.register_trait_methods(lis_t.clone(), list_collection);
        list_
            .register_trait(self, poly(COLLECTION, vec![ty_tp(T.clone())]))
            .unwrap();
        let t = fn1_met(
            list_t(T.clone(), TyParam::erased(Nat)),
            func1(T.clone(), Bool),
            tuple_t(vec![
                list_t(T.clone(), TyParam::erased(Nat)),
                list_t(T.clone(), TyParam::erased(Nat)),
            ]),
        );
        list_.register_py_builtin(FUNC_PARTITION, t.quantify(), Some(FUNC_PARTITION), 37);
        let t = no_var_fn_met(
            list_t(T.clone(), TyParam::erased(Nat)),
            vec![],
            vec![kw(
                KW_SAME_BUCKET,
                or(func2(T.clone(), T.clone(), Bool), NoneType),
            )],
            list_t(T.clone(), TyParam::erased(Nat)),
        );
        list_.register_py_builtin(FUNC_DEDUP, t.quantify(), Some(FUNC_DEDUP), 28);
        let sum_t = no_var_fn_met(
            list_t(T.clone(), TyParam::erased(Nat)),
            vec![],
            vec![kw(KW_START, T.clone())],
            T.clone(),
        );
        let sum = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_SUM,
            list_sum,
            sum_t.quantify(),
            None,
        )));
        list_.register_builtin_const(FUNC_SUM, Visibility::BUILTIN_PUBLIC, None, sum);
        let prod_t = no_var_fn_met(
            list_t(T.clone(), TyParam::erased(Nat)),
            vec![],
            vec![kw(KW_START, T.clone())],
            T.clone(),
        );
        let prod = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_PROD,
            list_prod,
            prod_t.quantify(),
            None,
        )));
        list_.register_builtin_const(FUNC_PROD, Visibility::BUILTIN_PUBLIC, None, prod);
        let reversed_t = no_var_fn_met(
            list_t(T.clone(), TyParam::erased(Nat)),
            vec![],
            vec![],
            list_t(T.clone(), TyParam::erased(Nat)),
        );
        let reversed = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_REVERSED,
            list_reversed,
            reversed_t.quantify(),
            None,
        )));
        list_.register_builtin_const(FUNC_REVERSED, Visibility::BUILTIN_PUBLIC, None, reversed);
        /* Slice */
        let mut slice = Self::builtin_mono_class(SLICE, 3);
        slice.register_superclass(Obj, &obj);
        slice.register_builtin_erg_impl(KW_START, Int, Immutable, Visibility::BUILTIN_PUBLIC);
        slice.register_builtin_erg_impl(KW_STOP, Int, Immutable, Visibility::BUILTIN_PUBLIC);
        slice.register_builtin_erg_impl(KW_STEP, Int, Immutable, Visibility::BUILTIN_PUBLIC);
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
        generic_set.register_trait_methods(mono(GENERIC_SET), set_eq);
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
        let mut generic_set_sized = Self::builtin_methods(Some(mono(SIZED)), 2);
        generic_set_sized.register_builtin_erg_impl(
            FUNDAMENTAL_LEN,
            fn0_met(mono(GENERIC_SET), Nat),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_set.register_trait_methods(mono(GENERIC_SET), generic_set_sized);
        /* Set */
        let mut set_ =
            Self::builtin_poly_class(SET, vec![PS::t_nd(TY_T), PS::named_nd(TY_N, Nat)], 10);
        let set_t = set_t(T.clone(), TyParam::erased(Nat));
        set_.register_superclass(mono(GENERIC_SET), &generic_set);
        set_.register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let t = no_var_fn_met(
            set_t.clone(),
            vec![kw(KW_RHS, set_t.clone())],
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
            None,
            mut_type,
        );
        set_.register_trait_methods(set_t.clone(), set_mutizable);
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
            None,
            ValueObj::builtin_class(set_iter),
        );
        set_.register_trait_methods(set_t.clone(), set_iterable);
        let mut set_collection =
            Self::builtin_methods(Some(poly(COLLECTION, vec![ty_tp(T.clone())])), 4);
        set_collection.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            fn1_met(set_t.clone(), T.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        set_.register_trait_methods(set_t.clone(), set_collection);
        set_.register_trait(self, poly(COLLECTION, vec![ty_tp(T.clone())]))
            .unwrap();
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
        set_.register_trait_methods(set_t.clone(), set_eq);
        set_.register_trait(self, mono(MUTIZABLE)).unwrap();
        set_.register_trait(self, poly(SEQUENCE, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut set_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        set_show.register_builtin_erg_impl(
            FUNDAMENTAL_STR,
            fn0_met(set_t.clone(), Str).quantify(),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        set_.register_trait_methods(set_t.clone(), set_show);
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
        generic_dict.register_trait_methods(g_dict_t.clone(), generic_dict_eq);
        let mut generic_dict_sized = Self::builtin_methods(Some(mono(SIZED)), 2);
        generic_dict_sized.register_builtin_erg_impl(
            FUNDAMENTAL_LEN,
            fn0_met(g_dict_t.clone(), Nat).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_dict.register_trait_methods(g_dict_t.clone(), generic_dict_sized);
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
        .quantify()
            & func(
                vec![],
                None,
                vec![],
                Some(ParamTy::Pos(Obj)),
                mono(GENERIC_DICT),
            );
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
        dict_
            .register_trait(self, poly(OUTPUT, vec![D.clone()]))
            .unwrap();
        let mut dict_mutizable = Self::builtin_methods(Some(mono(MUTIZABLE)), 2);
        dict_mutizable.register_builtin_const(
            MUTABLE_MUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(poly(MUT_DICT, vec![D.clone()])),
        );
        dict_.register_trait_methods(dict_t.clone(), dict_mutizable);
        // __getitem__: _: T -> D[T]
        let dict_getitem_out = proj_call(D.clone(), FUNDAMENTAL_GETITEM, vec![ty_tp(T.clone())]);
        let dict_getitem_t =
            fn1_met(dict_t.clone(), T.clone(), dict_getitem_out.clone()).quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNDAMENTAL_GETITEM,
            __dict_getitem__,
            dict_getitem_t,
            None,
        )));
        dict_.register_builtin_const(
            FUNDAMENTAL_GETITEM,
            Visibility::BUILTIN_PUBLIC,
            None,
            get_item,
        );
        dict_
            .register_trait(
                self,
                poly(MAPPING, vec![ty_tp(T.clone()), ty_tp(dict_getitem_out)]),
            )
            .unwrap();
        let dict_keys_iterator = poly(DICT_KEYS, vec![ty_tp(proj_call(D.clone(), KEYS, vec![]))]);
        let dict_keys_t = fn0_met(dict_t.clone(), dict_keys_iterator.clone()).quantify();
        let keys = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            KEYS,
            dict_keys,
            dict_keys_t,
            None,
        )));
        dict_.register_builtin_const(KEYS, Visibility::BUILTIN_PUBLIC, None, keys);
        let mut dict_iterable = Self::builtin_methods(
            Some(poly(
                ITERABLE,
                vec![ty_tp(proj_call(D.clone(), KEYS, vec![]))],
            )),
            2,
        );
        // Dict(D) -> DictKeys(D.keys())
        let t = fn0_met(dict_t.clone(), dict_keys_iterator.clone()).quantify();
        dict_iterable.register_builtin_py_impl(
            FUNC_ITER,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        dict_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            None,
            ValueObj::builtin_class(dict_keys_iterator),
        );
        dict_.register_trait_methods(dict_t.clone(), dict_iterable);
        let mut dict_collection = Self::builtin_methods(
            Some(poly(
                CONTAINER,
                vec![ty_tp(proj_call(D.clone(), KEYS, vec![]))],
            )),
            4,
        );
        // TODO: Obj => D.keys() (Structural { .__contains__ = ... })
        dict_collection.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            fn1_met(dict_t.clone(), Obj, Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        dict_.register_trait_methods(dict_t.clone(), dict_collection);
        let dict_values_t = fn0_met(
            dict_t.clone(),
            poly(
                DICT_VALUES,
                vec![ty_tp(proj_call(D.clone(), VALUES, vec![]))],
            ),
        )
        .quantify();
        let values = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            VALUES,
            dict_values,
            dict_values_t,
            None,
        )));
        dict_.register_builtin_const(VALUES, Visibility::BUILTIN_PUBLIC, None, values);
        let dict_items_t = fn0_met(
            dict_t.clone(),
            poly(DICT_ITEMS, vec![ty_tp(proj_call(D.clone(), ITEMS, vec![]))]),
        )
        .quantify();
        let items = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            ITEMS,
            dict_items,
            dict_items_t,
            None,
        )));
        dict_.register_builtin_const(ITEMS, Visibility::BUILTIN_PUBLIC, None, items);
        let as_record_t =
            fn0_met(dict_t.clone(), proj_call(D.clone(), FUNC_AS_RECORD, vec![])).quantify();
        let as_record = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_AS_RECORD,
            as_record,
            as_record_t,
            None,
        )));
        dict_.register_builtin_const(FUNC_AS_RECORD, Visibility::BUILTIN_PUBLIC, None, as_record);
        let Def = type_q(TY_DEFAULT);
        let get_t = no_var_fn_met(
            dict_t.clone(),
            vec![kw(KW_KEY, T.clone())],
            vec![kw_default(KW_DEFAULT, Def.clone(), NoneType)],
            or(
                proj_call(D.clone(), FUNDAMENTAL_GETITEM, vec![ty_tp(T.clone())]),
                Def,
            ),
        )
        .quantify();
        dict_.register_py_builtin(FUNC_GET, get_t, Some(FUNC_GET), 9);
        let copy_t = fn0_met(ref_(dict_t.clone()), dict_t.clone()).quantify();
        let mut dict_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        dict_copy.register_py_builtin(FUNC_COPY, copy_t, Some(FUNC_COPY), 7);
        dict_.register_trait_methods(dict_t.clone(), dict_copy);
        let D2 = mono_q_tp(TY_D2, instanceof(mono(GENERIC_DICT)));
        let other_dict_t = poly(DICT, vec![D2.clone()]);
        let dict_concat_t = fn1_met(
            dict_t.clone(),
            other_dict_t.clone(),
            poly(
                DICT,
                vec![D.clone().proj_call(FUNC_CONCAT.into(), vec![D2.clone()])],
            ),
        )
        .quantify();
        let concat = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_CONCAT,
            dict_concat,
            dict_concat_t,
            None,
        )));
        dict_.register_builtin_const(FUNC_CONCAT, Visibility::BUILTIN_PUBLIC, None, concat);
        let dict_diff_t = fn1_met(
            dict_t.clone(),
            other_dict_t.clone(),
            poly(
                DICT,
                vec![D.clone().proj_call(FUNC_DIFF.into(), vec![D2.clone()])],
            ),
        )
        .quantify();
        let diff = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_DIFF,
            dict_diff,
            dict_diff_t,
            None,
        )));
        dict_.register_builtin_const(FUNC_DIFF, Visibility::BUILTIN_PUBLIC, None, diff);
        /* Bytes */
        let mut bytes = Self::builtin_mono_class(BYTES, 2);
        bytes.register_superclass(Obj, &obj);
        bytes.register_py_builtin(
            OP_GT,
            fn1_met(mono(BYTES), mono(BYTES), Bool),
            Some(OP_GT),
            0,
        );
        bytes.register_py_builtin(
            OP_GE,
            fn1_met(mono(BYTES), mono(BYTES), Bool),
            Some(OP_GE),
            0,
        );
        bytes.register_py_builtin(
            OP_LT,
            fn1_met(mono(BYTES), mono(BYTES), Bool),
            Some(OP_LT),
            0,
        );
        bytes.register_py_builtin(
            OP_LE,
            fn1_met(mono(BYTES), mono(BYTES), Bool),
            Some(OP_LE),
            0,
        );
        bytes.register_trait(self, mono(ORD)).unwrap();
        let decode_t = pr_met(
            mono(BYTES),
            vec![],
            None,
            vec![kw(KW_ENCODING, Str), kw(KW_ERRORS, Str)],
            Str,
        );
        bytes.register_py_builtin(FUNC_DECODE, decode_t, Some(FUNC_DECODE), 6);
        let idx_t = if PYTHON_MODE { Int } else { Nat };
        let bytes_getitem_t = fn1_kw_met(mono(BYTES), kw(KW_IDX, idx_t), Int)
            & fn1_kw_met(
                mono(BYTES),
                kw(KW_IDX, poly(RANGE, vec![ty_tp(Int)]) | mono(SLICE)),
                mono(BYTES),
            );
        bytes.register_builtin_erg_impl(
            FUNDAMENTAL_GETITEM,
            bytes_getitem_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        bytes.register_builtin_py_impl(
            FUNC_CENTER,
            fn_met(
                mono(BYTES),
                vec![kw(KW_WIDTH, Nat)],
                None,
                vec![kw(KW_FILLCHAR, mono(BYTES))],
                None,
                mono(BYTES),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_CENTER),
        );
        bytes.register_builtin_erg_impl(
            FUNC_LOWER,
            fn_met(mono(BYTES), vec![], None, vec![], None, mono(BYTES)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        bytes.register_builtin_erg_impl(
            FUNC_UPPER,
            fn_met(mono(BYTES), vec![], None, vec![], None, mono(BYTES)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let t_startswith = fn1_met(mono(BYTES), mono(BYTES), Bool);
        bytes.register_builtin_py_impl(
            FUNC_STARTSWITH,
            t_startswith,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_STARTSWITH),
        );
        let t_endswith = fn1_met(mono(BYTES), mono(BYTES), Bool);
        bytes.register_builtin_py_impl(
            FUNC_ENDSWITH,
            t_endswith,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ENDSWITH),
        );
        // TODO: resolve type inference conflict with `Str.split`
        /*bytes.register_builtin_py_impl(
            FUNC_SPLIT,
            fn_met(
                mono(BYTES),
                vec![kw(KW_SEP, mono(BYTES))],
                None,
                vec![kw(KW_MAXSPLIT, Nat)],
                None,
                unknown_len_list_t(mono(BYTES)),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_SPLIT),
        );
        let t_replace = fn_met(
            mono(BYTES),
            vec![kw(KW_PAT, mono(BYTES)), kw(KW_INTO, mono(BYTES))],
            None,
            vec![],
            None,
            mono(BYTES),
        );
        bytes.register_builtin_py_impl(
            FUNC_REPLACE,
            t_replace,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_REPLACE),
        );*/
        bytes.register_builtin_py_impl(
            FUNC_SPLITLINES,
            fn_met(
                mono(BYTES),
                vec![],
                None,
                vec![kw(KW_KEEPENDS, Bool)],
                None,
                unknown_len_list_t(mono(BYTES)),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_SPLITLINES),
        );
        let t_join = fn1_met(
            mono(BYTES),
            poly(ITERABLE, vec![ty_tp(mono(BYTES))]),
            mono(BYTES),
        );
        bytes.register_builtin_py_impl(
            FUNC_JOIN,
            t_join,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_JOIN),
        );
        bytes.register_builtin_py_impl(
            FUNC_INDEX,
            fn_met(
                mono(BYTES),
                vec![kw(KW_SUB, mono(BYTES))],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                None,
                Nat,
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_INDEX),
        );
        bytes.register_builtin_py_impl(
            FUNC_RINDEX,
            fn_met(
                mono(BYTES),
                vec![kw(KW_SUB, mono(BYTES))],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                None,
                Nat,
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_RINDEX),
        );
        let t_find = fn_met(
            mono(BYTES),
            vec![kw(KW_SUB, mono(BYTES))],
            None,
            vec![kw(KW_START, Nat), kw(KW_END, Nat)],
            None,
            or(Nat, v_enum(set! {(-1).into()})),
        );
        bytes.register_builtin_py_impl(
            FUNC_FIND,
            t_find,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_FIND),
        );
        bytes.register_builtin_py_impl(
            FUNC_RFIND,
            fn_met(
                mono(BYTES),
                vec![kw(KW_SUB, mono(BYTES))],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                None,
                or(Nat, v_enum(set! {(-1).into()})),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_RFIND),
        );
        bytes.register_builtin_py_impl(
            FUNC_COUNT,
            fn_met(
                mono(BYTES),
                vec![kw(KW_SUB, mono(BYTES))],
                None,
                vec![kw(KW_START, Nat), kw(KW_END, Nat)],
                None,
                Nat,
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_COUNT),
        );
        bytes.register_builtin_py_impl(
            FUNC_CAPITALIZE,
            fn0_met(mono(BYTES), mono(BYTES)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_CAPITALIZE),
        );
        bytes.register_builtin_py_impl(
            FUNC_STRIP,
            fn_met(
                mono(BYTES),
                vec![],
                None,
                vec![kw(KW_CHARS, mono(BYTES) | NoneType)],
                None,
                mono(BYTES),
            ),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_STRIP),
        );
        bytes.register_builtin_py_impl(
            FUNC_REMOVEPREFIX,
            fn1_met(mono(BYTES), mono(BYTES), mono(BYTES)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_REMOVEPREFIX),
        );
        bytes.register_builtin_py_impl(
            FUNC_REMOVESUFFIX,
            fn1_met(mono(BYTES), mono(BYTES), mono(BYTES)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_REMOVESUFFIX),
        );
        bytes.register_builtin_py_impl(
            FUNC_ISALNUM,
            fn0_met(mono(BYTES), Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISALNUM),
        );
        let t_isalpha = fn0_met(mono(BYTES), Bool);
        bytes.register_builtin_py_impl(
            FUNC_ISALPHA,
            t_isalpha,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISALPHA),
        );
        let t_isascii = fn0_met(mono(BYTES), Bool);
        bytes.register_builtin_py_impl(
            FUNC_ISASCII,
            t_isascii,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISASCII),
        );
        bytes.register_builtin_py_impl(
            FUNC_ISDIGIT,
            fn0_met(mono(BYTES), Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISDIGIT),
        );
        bytes.register_builtin_py_impl(
            FUNC_ISLOWER,
            fn0_met(mono(BYTES), Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISLOWER),
        );
        bytes.register_builtin_py_impl(
            FUNC_ISSPACE,
            fn0_met(mono(BYTES), Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISSPACE),
        );
        bytes.register_builtin_py_impl(
            FUNC_ISTITLE,
            fn0_met(mono(BYTES), Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISTITLE),
        );
        bytes.register_builtin_py_impl(
            FUNC_ISUPPER,
            fn0_met(mono(BYTES), Bool),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISUPPER),
        );
        let mut bytes_seq = Self::builtin_methods(Some(poly(SEQUENCE, vec![ty_tp(Int)])), 2);
        bytes_seq.register_builtin_erg_impl(
            FUNDAMENTAL_LEN,
            fn0_met(mono(BYTES), Nat),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bytes.register_trait_methods(mono(BYTES), bytes_seq);
        bytes
            .register_trait(self, poly(SEQUENCE, vec![ty_tp(Int)]))
            .unwrap();
        let mut bytes_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        bytes_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(mono(BYTES), mono(BYTES), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bytes.register_trait_methods(mono(BYTES), bytes_eq);
        let mut bytes_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        bytes_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(mono(BYTES), Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bytes.register_trait_methods(mono(BYTES), bytes_hash);
        bytes.register_trait(self, mono(EQ_HASH)).unwrap();
        let t_call = func0(mono(BYTES))
            & no_var_func(
                vec![kw(KW_STR, Str), kw(KW_ENCODING, Str)],
                vec![kw(KW_ERRORS, Str)],
                mono(BYTES),
            )
            // (iterable_of_ints) -> bytes | (bytes_or_buffer) -> bytes & nat -> bytes
            & nd_func(
                // TODO: Bytes-like
                vec![pos(poly(ITERABLE, vec![ty_tp(Nat)]) | Nat | mono(BYTES) | mono(MUT_BYTEARRAY))],
                None,
                mono(BYTES),
            );
        bytes.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        /* GenericTuple */
        let mut generic_tuple = Self::builtin_mono_class(GENERIC_TUPLE, 1);
        generic_tuple.register_superclass(Obj, &obj);
        // tuple doesn't have a constructor, use `List` instead
        let mut tuple_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        tuple_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(mono(GENERIC_TUPLE), mono(GENERIC_TUPLE), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_tuple.register_trait_methods(mono(GENERIC_TUPLE), tuple_eq);
        let mut tuple_hash = Self::builtin_methods(Some(mono(HASH)), 1);
        tuple_hash.register_builtin_erg_impl(
            OP_HASH,
            fn0_met(mono(GENERIC_TUPLE), Int),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_tuple.register_trait_methods(mono(GENERIC_TUPLE), tuple_hash);
        generic_tuple.register_trait(self, mono(EQ_HASH)).unwrap();
        let mut generic_tuple_sized = Self::builtin_methods(Some(mono(SIZED)), 2);
        generic_tuple_sized.register_builtin_erg_impl(
            FUNDAMENTAL_LEN,
            fn0_met(mono(GENERIC_TUPLE), Nat),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        generic_tuple.register_trait_methods(mono(GENERIC_TUPLE), generic_tuple_sized);
        let t_call = func1(
            poly(ITERABLE, vec![ty_tp(T.clone())]),
            tuple_t(vec![T.clone()]),
        )
        .quantify();
        generic_tuple.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        /* HomogenousTuple */
        let mut homo_tuple = Self::builtin_poly_class(HOMOGENOUS_TUPLE, vec![PS::t_nd(TY_T)], 1);
        homo_tuple.register_superclass(mono(GENERIC_TUPLE), &generic_tuple);
        homo_tuple
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let homo_tuple_t = poly(HOMOGENOUS_TUPLE, vec![ty_tp(T.clone())]);
        // __getitem__: (self: HomogenousTuple(T), Nat) -> T
        let idx_t = if PYTHON_MODE { Int } else { Nat };
        let tuple_getitem_t = fn1_met(homo_tuple_t.clone(), idx_t, T.clone()).quantify();
        homo_tuple.register_builtin_py_impl(
            FUNDAMENTAL_GETITEM,
            tuple_getitem_t,
            Const,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_GETITEM),
        );
        let mut homo_tuple_seq =
            Self::builtin_methods(Some(poly(SEQUENCE, vec![ty_tp(T.clone())])), 4);
        homo_tuple_seq.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            fn1_met(homo_tuple_t.clone(), T.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        let tuple_iter = poly(TUPLE_ITERATOR, vec![ty_tp(T.clone())]);
        let t = fn0_met(list_t(T.clone(), TyParam::erased(Nat)), tuple_iter.clone()).quantify();
        homo_tuple_seq.register_builtin_py_impl(
            FUNC_ITER,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        homo_tuple_seq.register_builtin_const(
            ITERATOR,
            vis.clone(),
            None,
            ValueObj::builtin_class(tuple_iter),
        );
        homo_tuple.register_trait_methods(homo_tuple_t.clone(), homo_tuple_seq);
        homo_tuple
            .register_trait(self, poly(SEQUENCE, vec![ty_tp(T.clone())]))
            .unwrap();
        let Ts = mono_q_tp(TY_TS, instanceof(list_t(Type, N.clone())));
        // Ts <: GenericList
        let _tuple_t = poly(TUPLE, vec![Ts.clone()]);
        let mut tuple_ =
            Self::builtin_poly_class(TUPLE, vec![PS::named_nd(TY_TS, list_t(Type, N.clone()))], 2);
        tuple_.register_superclass(
            poly(
                HOMOGENOUS_TUPLE,
                vec![Ts.clone().proj_call(FUNC_UNION.into(), vec![])],
            ),
            &homo_tuple,
        );
        tuple_
            .register_trait(self, poly(OUTPUT, vec![Ts.clone()]))
            .unwrap();
        // __Tuple_getitem__: (self: Tuple(Ts), _: {N}) -> Ts[N]
        let input_t = tp_enum(Nat, set! {N.clone()});
        let slice_t = poly(RANGE, vec![ty_tp(Int)]) | mono(SLICE);
        let return_t = proj_call(Ts.clone(), FUNDAMENTAL_GETITEM, vec![N.clone()]);
        let tuple_getitem_t = (fn1_met(_tuple_t.clone(), input_t.clone(), return_t.clone())
            & fn1_met(_tuple_t.clone(), slice_t.clone(), _tuple_t.clone()))
        .quantify();
        tuple_.register_builtin_py_impl(
            FUNDAMENTAL_TUPLE_GETITEM,
            tuple_getitem_t.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_GETITEM),
        );
        tuple_
            .register_trait(self, poly(INDEXABLE, vec![ty_tp(input_t), ty_tp(return_t)]))
            .unwrap();
        // `__Tuple_getitem__` and `__getitem__` are the same thing
        // but `x.0` => `x__Tuple_getitem__(0)` determines that `x` is a tuple, which is better for type inference.
        tuple_.register_builtin_py_impl(
            FUNDAMENTAL_GETITEM,
            tuple_getitem_t,
            Const,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_GETITEM),
        );
        /* record */
        let mut record = Self::builtin_mono_class(RECORD, 2);
        record.register_superclass(Obj, &obj);
        let mut record_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        record_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(mono(RECORD), mono(RECORD), Bool),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        record.register_trait_methods(mono(RECORD), record_eq);
        let Slf = mono_q(SELF, subtypeof(mono(RECORD)));
        let as_dict_t =
            fn0_met(Slf.clone(), proj_call(ty_tp(Slf), FUNC_AS_DICT, vec![])).quantify();
        let as_dict = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_AS_DICT,
            as_dict,
            as_dict_t,
            None,
        )));
        record.register_py_builtin_const(
            FUNC_AS_DICT,
            Visibility::BUILTIN_PUBLIC,
            None,
            as_dict,
            Some(FUNC_ASDICT),
            None,
        );
        let mut record_meta_type = Self::builtin_mono_class(RECORD_META_TYPE, 2);
        record_meta_type.register_superclass(mono(RECORD), &record);
        record_meta_type.register_superclass(Type, &type_);
        /* GenericNamedTuple */
        let mut generic_named_tuple = Self::builtin_mono_class(GENERIC_NAMED_TUPLE, 2);
        generic_named_tuple.register_superclass(mono(GENERIC_TUPLE), &generic_tuple);
        let Slf = mono_q(SELF, subtypeof(mono(GENERIC_NAMED_TUPLE)));
        let input_t = tp_enum(Nat, set! {N.clone()});
        let return_t = proj_call(ty_tp(Slf.clone()), FUNDAMENTAL_GETITEM, vec![N.clone()]);
        let named_tuple_getitem =
            fn1_met(Slf.clone(), input_t.clone(), return_t.clone()).quantify();
        let mut named_tuple_indexable = Self::builtin_methods(
            Some(poly(INDEXABLE, vec![ty_tp(input_t), ty_tp(return_t)])),
            2,
        );
        named_tuple_indexable.register_builtin_py_impl(
            FUNDAMENTAL_TUPLE_GETITEM,
            named_tuple_getitem.clone(),
            Const,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_GETITEM),
        );
        generic_named_tuple
            .register_trait_methods(mono(GENERIC_NAMED_TUPLE), named_tuple_indexable);
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNDAMENTAL_GETITEM,
            __named_tuple_getitem__,
            named_tuple_getitem,
            None,
        )));
        generic_named_tuple.register_builtin_const(
            FUNDAMENTAL_GETITEM,
            Visibility::BUILTIN_PUBLIC,
            None,
            get_item,
        );
        let mut named_tuple_iterable = Self::builtin_methods(
            Some(poly(
                ITERABLE,
                vec![ty_tp(proj_call(ty_tp(Slf.clone()), FUNC_UNION, vec![]))],
            )),
            2,
        );
        let named_tuple_iterator = poly(
            TUPLE_ITERATOR,
            vec![ty_tp(proj_call(ty_tp(Slf.clone()), FUNC_UNION, vec![]))],
        );
        let t = fn0_met(Slf.clone(), named_tuple_iterator.clone()).quantify();
        named_tuple_iterable.register_builtin_py_impl(
            FUNC_ITER,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        named_tuple_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            None,
            ValueObj::builtin_class(named_tuple_iterator),
        );
        generic_named_tuple.register_trait_methods(mono(GENERIC_NAMED_TUPLE), named_tuple_iterable);
        // union: (self: NamedTuple({...})) -> Type
        let named_tuple_union_t = fn0_met(Slf, Type).quantify();
        let union = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_UNION,
            named_tuple_union,
            named_tuple_union_t,
            None,
        )));
        generic_named_tuple.register_builtin_const(
            FUNC_UNION,
            Visibility::BUILTIN_PUBLIC,
            None,
            union,
        );
        self.register_builtin_type(
            mono(GENERIC_TUPLE),
            generic_tuple,
            vis.clone(),
            Const,
            Some(FUNC_TUPLE),
        );
        self.register_builtin_type(
            homo_tuple_t,
            homo_tuple,
            vis.clone(),
            Const,
            Some(FUNC_TUPLE),
        );
        self.register_builtin_type(_tuple_t, tuple_, vis.clone(), Const, Some(FUNC_TUPLE));
        /* Or (true or type) */
        let or_t = poly(OR, vec![ty_tp(L), ty_tp(R)]);
        let mut or = Self::builtin_poly_class(OR, vec![PS::t_nd(TY_L), PS::t_nd(TY_R)], 2);
        or.register_superclass(Obj, &obj);
        /* Iterators */
        let mut str_iterator = Self::builtin_mono_class(STR_ITERATOR, 1);
        str_iterator.register_superclass(Obj, &obj);
        str_iterator
            .register_trait(self, poly(ITERATOR, vec![ty_tp(Str)]))
            .unwrap();
        str_iterator
            .register_trait(self, poly(OUTPUT, vec![ty_tp(Str)]))
            .unwrap();
        let mut list_iterator = Self::builtin_poly_class(LIST_ITERATOR, vec![PS::t_nd(TY_T)], 1);
        list_iterator.register_superclass(Obj, &obj);
        list_iterator
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        list_iterator
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut set_iterator = Self::builtin_poly_class(SET_ITERATOR, vec![PS::t_nd(TY_T)], 1);
        set_iterator.register_superclass(Obj, &obj);
        set_iterator
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        set_iterator
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut tuple_iterator = Self::builtin_poly_class(TUPLE_ITERATOR, vec![PS::t_nd(TY_T)], 1);
        tuple_iterator.register_superclass(Obj, &obj);
        tuple_iterator
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        tuple_iterator
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut range_iterator = Self::builtin_poly_class(RANGE_ITERATOR, vec![PS::t_nd(TY_T)], 1);
        range_iterator.register_superclass(Obj, &obj);
        range_iterator
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        range_iterator
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut dict_keys = Self::builtin_poly_class(DICT_KEYS, vec![PS::t_nd(TY_T)], 1);
        dict_keys.register_superclass(Obj, &obj);
        dict_keys
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        dict_keys
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut dict_values = Self::builtin_poly_class(DICT_VALUES, vec![PS::t_nd(TY_T)], 1);
        dict_values.register_superclass(Obj, &obj);
        dict_values
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        dict_values
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut dict_items = Self::builtin_poly_class(DICT_ITEMS, vec![PS::t_nd(TY_T)], 1);
        dict_items.register_superclass(Obj, &obj);
        dict_items
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        dict_items
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        /* Enumerate */
        let mut enumerate = Self::builtin_poly_class(ENUMERATE, vec![PS::t_nd(TY_T)], 2);
        enumerate.register_superclass(Obj, &obj);
        enumerate
            .register_trait(
                self,
                poly(ITERATOR, vec![ty_tp(tuple_t(vec![Nat, T.clone()]))]),
            )
            .unwrap();
        enumerate
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        /* Filter */
        let mut filter = Self::builtin_poly_class(FILTER, vec![PS::t_nd(TY_T)], 2);
        filter.register_superclass(Obj, &obj);
        filter
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        filter
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        /* Map */
        let mut map = Self::builtin_poly_class(MAP, vec![PS::t_nd(TY_T)], 2);
        map.register_superclass(Obj, &obj);
        map.register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        map.register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        /* Reversed */
        let mut reversed = Self::builtin_poly_class(REVERSED, vec![PS::t_nd(TY_T)], 2);
        reversed.register_superclass(Obj, &obj);
        reversed
            .register_trait(self, poly(ITERATOR, vec![ty_tp(T.clone())]))
            .unwrap();
        reversed
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        /* Zip */
        let mut zip = Self::builtin_poly_class(ZIP, vec![PS::t_nd(TY_T), PS::t_nd(TY_U)], 2);
        zip.register_superclass(Obj, &obj);
        zip.register_trait(
            self,
            poly(ITERATOR, vec![ty_tp(tuple_t(vec![T.clone(), U.clone()]))]),
        )
        .unwrap();
        zip.register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        zip.register_trait(self, poly(OUTPUT, vec![ty_tp(U.clone())]))
            .unwrap();
        let fset_t = poly(FROZENSET, vec![ty_tp(T.clone())]);
        let mut frozenset = Self::builtin_poly_class(FROZENSET, vec![PS::t_nd(TY_T)], 2);
        frozenset.register_superclass(Obj, &obj);
        frozenset
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut fset_iterable =
            Self::builtin_methods(Some(poly(ITERABLE, vec![ty_tp(T.clone())])), 2);
        let set_iter = poly(SET_ITERATOR, vec![ty_tp(T.clone())]);
        let t = fn0_met(fset_t.clone(), set_iter.clone()).quantify();
        fset_iterable.register_builtin_py_impl(
            FUNC_ITER,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        fset_iterable.register_builtin_const(
            ITERATOR,
            vis.clone(),
            None,
            ValueObj::builtin_class(set_iter),
        );
        frozenset.register_trait_methods(fset_t.clone(), fset_iterable);
        let mut fset_collection =
            Self::builtin_methods(Some(poly(COLLECTION, vec![ty_tp(T.clone())])), 4);
        fset_collection.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            fn1_met(fset_t.clone(), T.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        frozenset.register_trait_methods(fset_t.clone(), fset_collection);
        frozenset
            .register_trait(self, poly(COLLECTION, vec![ty_tp(T.clone())]))
            .unwrap();
        frozenset
            .register_trait(self, poly(SEQUENCE, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut fset_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        fset_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(fset_t.clone(), fset_t.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        frozenset.register_trait_methods(fset_t.clone(), fset_eq);
        let mut fset_show = Self::builtin_methods(Some(mono(SHOW)), 1);
        fset_show.register_builtin_erg_impl(
            FUNDAMENTAL_STR,
            fn0_met(fset_t.clone(), Str).quantify(),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        frozenset.register_trait_methods(fset_t.clone(), fset_show);
        let t = fn0_met(fset_t.clone(), fset_t.clone()).quantify();
        let mut frozenset_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        frozenset_copy.register_py_builtin(FUNC_COPY, t, Some(FUNC_COPY), 3);
        frozenset.register_trait_methods(fset_t.clone(), frozenset_copy);
        let bin_t = fn1_met(ref_(fset_t.clone()), fset_t.clone(), fset_t.clone()).quantify();
        frozenset.register_py_builtin(FUNC_DIFFERENCE, bin_t.clone(), Some(FUNC_DIFFERENCE), 3);
        frozenset.register_py_builtin(FUNC_INTERSECTION, bin_t.clone(), Some(FUNC_INTERSECTION), 3);
        let bool_t = fn1_met(fset_t.clone(), fset_t.clone(), Bool).quantify();
        frozenset.register_py_builtin(FUNC_ISDISJOINT, bool_t.clone(), Some(FUNC_ISDISJOINT), 3);
        frozenset.register_py_builtin(FUNC_ISSUBSET, bool_t.clone(), Some(FUNC_ISSUBSET), 3);
        frozenset.register_py_builtin(FUNC_ISSUPERSET, bool_t, Some(FUNC_ISSUPERSET), 3);
        frozenset.register_py_builtin(
            FUNC_SYMMETRIC_DIFFERENCE,
            bin_t.clone(),
            Some(FUNC_SYMMETRIC_DIFFERENCE),
            3,
        );
        frozenset.register_py_builtin(FUNC_UNION, bin_t, Some(FUNC_UNION), 3);
        let memview_t = mono(MEMORYVIEW);
        let mut memoryview = Self::builtin_mono_class(MEMORYVIEW, 2);
        memoryview.register_superclass(Obj, &obj);
        let mut obj_mut = Self::builtin_mono_class(MUTABLE_OBJ, 2);
        obj_mut.register_superclass(Obj, &obj);
        let mut obj_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        obj_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Obj),
        );
        let f_t = kw(KW_FUNC, no_var_func(vec![kw(KW_OLD, Int)], vec![], Int));
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
        obj_mut.register_trait_methods(mono(MUTABLE_OBJ), obj_mut_mutable);
        /* Float! */
        let mut float_mut = Self::builtin_mono_class(MUT_FLOAT, 2);
        float_mut.register_superclass(Float, &float);
        let mut float_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        float_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Float),
        );
        let f_t = kw(KW_FUNC, no_var_func(vec![kw(KW_OLD, Float)], vec![], Float));
        let t = pr_met(
            ref_mut(mono(MUT_FLOAT), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        float_mut_mutable.register_builtin_py_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_UPDATE),
        );
        let t = pr_met(
            ref_mut(mono(MUT_FLOAT), None),
            vec![kw(KW_VALUE, Float)],
            None,
            vec![],
            NoneType,
        );
        float_mut_mutable.register_builtin_py_impl(
            PROC_INC,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_INC),
        );
        let t = pr_met(
            ref_mut(mono(MUT_FLOAT), None),
            vec![kw(KW_VALUE, Float)],
            None,
            vec![],
            NoneType,
        );
        float_mut_mutable.register_builtin_py_impl(
            PROC_DEC,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_DEC),
        );
        float_mut.register_trait_methods(mono(MUT_FLOAT), float_mut_mutable);
        let mut float_mut_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        float_mut_copy.register_builtin_erg_impl(
            FUNC_COPY,
            fn0_met(ref_(mono(MUT_FLOAT)), mono(MUT_FLOAT)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        float_mut.register_trait_methods(mono(MUT_FLOAT), float_mut_copy);
        /* Ratio! */
        let mut ratio_mut = Self::builtin_mono_class(MUT_RATIO, 2);
        ratio_mut.register_superclass(Ratio, &ratio);
        let mut ratio_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        ratio_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Ratio),
        );
        let f_t = kw(KW_FUNC, no_var_func(vec![kw(KW_OLD, Ratio)], vec![], Ratio));
        let t = pr_met(
            ref_mut(mono(MUT_RATIO), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        ratio_mut_mutable.register_builtin_py_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_UPDATE),
        );
        ratio_mut.register_trait_methods(mono(MUT_RATIO), ratio_mut_mutable);
        let mut ratio_mut_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        ratio_mut_copy.register_builtin_erg_impl(
            FUNC_COPY,
            fn0_met(ref_(mono(MUT_RATIO)), mono(MUT_RATIO)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        ratio_mut.register_trait_methods(mono(MUT_RATIO), ratio_mut_copy);
        /* Int! */
        let mut int_mut = Self::builtin_mono_class(MUT_INT, 2);
        int_mut.register_superclass(Int, &int);
        int_mut.register_superclass(mono(MUT_FLOAT), &float_mut);
        let t = pr_met(mono(MUT_INT), vec![], None, vec![kw(KW_I, Int)], NoneType);
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
            None,
            ValueObj::builtin_class(Int),
        );
        let f_t = kw(KW_FUNC, no_var_func(vec![kw(KW_OLD, Int)], vec![], Int));
        let t = pr_met(
            ref_mut(mono(MUT_INT), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        int_mut_mutable.register_builtin_py_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_UPDATE),
        );
        int_mut.register_trait_methods(mono(MUT_INT), int_mut_mutable);
        let mut int_mut_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        int_mut_copy.register_builtin_erg_impl(
            FUNC_COPY,
            fn0_met(ref_(mono(MUT_INT)), mono(MUT_INT)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        int_mut.register_trait_methods(mono(MUT_INT), int_mut_copy);
        let mut nat_mut = Self::builtin_mono_class(MUT_NAT, 2);
        nat_mut.register_superclass(Nat, &nat);
        nat_mut.register_superclass(mono(MUT_INT), &int_mut);
        /* Nat! */
        let mut nat_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        nat_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Nat),
        );
        let f_t = kw(KW_FUNC, no_var_func(vec![kw(KW_OLD, Nat)], vec![], Nat));
        let t = pr_met(
            ref_mut(mono(MUT_NAT), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        nat_mut_mutable.register_builtin_py_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_UPDATE),
        );
        nat_mut.register_trait_methods(mono(MUT_NAT), nat_mut_mutable);
        let mut nat_mut_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        nat_mut_copy.register_builtin_erg_impl(
            FUNC_COPY,
            fn0_met(ref_(mono(MUT_NAT)), mono(MUT_NAT)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        nat_mut.register_trait_methods(mono(MUT_NAT), nat_mut_copy);
        /* Bool! */
        let mut bool_mut = Self::builtin_mono_class(MUT_BOOL, 2);
        bool_mut.register_superclass(Bool, &bool_);
        bool_mut.register_superclass(mono(MUT_NAT), &nat_mut);
        let mut bool_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        bool_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Bool),
        );
        let f_t = kw(KW_FUNC, no_var_func(vec![kw(KW_OLD, Bool)], vec![], Bool));
        let t = pr_met(
            ref_mut(mono(MUT_BOOL), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        bool_mut_mutable.register_builtin_py_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_UPDATE),
        );
        bool_mut.register_trait_methods(mono(MUT_BOOL), bool_mut_mutable);
        let mut bool_mut_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        bool_mut_copy.register_builtin_erg_impl(
            FUNC_COPY,
            fn0_met(ref_(mono(MUT_BOOL)), mono(MUT_BOOL)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        bool_mut.register_trait_methods(mono(MUT_BOOL), bool_mut_copy);
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
        str_mut.register_superclass(Str, &str_);
        let mut str_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        str_mut_mutable.register_builtin_const(
            IMMUT_TYPE,
            Visibility::BUILTIN_PUBLIC,
            None,
            ValueObj::builtin_class(Str),
        );
        let f_t = kw(KW_FUNC, no_var_func(vec![kw(KW_OLD, Str)], vec![], Str));
        let t = pr_met(
            ref_mut(mono(MUT_STR), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        );
        str_mut_mutable.register_builtin_py_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_UPDATE),
        );
        str_mut.register_trait_methods(mono(MUT_STR), str_mut_mutable);
        let mut str_mut_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        str_mut_copy.register_builtin_erg_impl(
            FUNC_COPY,
            fn0_met(ref_(mono(MUT_STR)), mono(MUT_STR)),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        str_mut.register_trait_methods(mono(MUT_STR), str_mut_copy);
        let t = pr_met(
            ref_mut(mono(MUT_STR), None),
            vec![kw(KW_S, Str)],
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
            vec![kw(KW_IDX, Nat), kw(KW_S, Str)],
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
            vec![kw(KW_IDX, Nat)],
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
        file_mut.register_trait_methods(mono(MUT_FILE), file_mut_readable);
        let mut file_mut_writable = Self::builtin_methods(Some(mono(MUT_WRITABLE)), 1);
        file_mut_writable.register_builtin_py_impl(
            PROC_WRITE,
            pr1_kw_met(ref_mut(mono(MUT_FILE), None), kw(KW_S, Str), Nat),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_WRITE),
        );
        file_mut.register_trait_methods(mono(MUT_FILE), file_mut_writable);
        file_mut.register_trait(self, mono(FILE_LIKE)).unwrap();
        file_mut.register_trait(self, mono(MUT_FILE_LIKE)).unwrap();
        file_mut
            .register_trait(self, mono(CONTEXT_MANAGER))
            .unwrap();
        /* List! */
        let list_mut_t = poly(MUT_LIST, vec![ty_tp(T.clone()), N.clone()]);
        let mut list_mut_ =
            Self::builtin_poly_class(MUT_LIST, vec![PS::t_nd(TY_T), PS::default(TY_N, Nat)], 2);
        list_mut_.register_superclass(lis_t.clone(), &list_);
        let t = pr_met(
            ref_mut(
                list_mut_t.clone(),
                Some(poly(
                    MUT_LIST,
                    vec![ty_tp(T.clone()), N.clone() + value(1usize)],
                )),
            ),
            vec![kw(KW_ELEM, T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_PUSH, t, Some(FUNC_APPEND), 15);
        let t_copy = fn0_met(ref_(list_mut_t.clone()), list_mut_t.clone()).quantify();
        let mut list_mut_copy = Self::builtin_methods(Some(mono(COPY)), 2);
        list_mut_copy.register_py_builtin(FUNC_COPY, t_copy, Some(FUNC_COPY), 116);
        list_mut_.register_trait_methods(list_mut_t.clone(), list_mut_copy);
        let t_extend = pr_met(
            ref_mut(
                list_mut_t.clone(),
                Some(poly(MUT_LIST, vec![ty_tp(T.clone()), TyParam::erased(Nat)])),
            ),
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_EXTEND, t_extend, Some(FUNC_EXTEND), 24);
        let t_insert = pr_met(
            ref_mut(
                list_mut_t.clone(),
                Some(poly(
                    MUT_LIST,
                    vec![ty_tp(T.clone()), N.clone() + value(1usize)],
                )),
            ),
            vec![kw(KW_INDEX, Nat), kw(KW_ELEM, T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_INSERT, t_insert, Some(FUNC_INSERT), 33);
        let t_remove = pr_met(
            ref_mut(
                list_mut_t.clone(),
                Some(poly(
                    MUT_LIST,
                    vec![ty_tp(T.clone()), N.clone() - value(1usize)],
                )),
            ),
            vec![kw(KW_X, T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_REMOVE, t_remove, Some(FUNC_REMOVE), 42);
        let t_pop = pr_met(
            ref_mut(
                list_mut_t.clone(),
                Some(poly(
                    MUT_LIST,
                    vec![ty_tp(T.clone()), N.clone() - value(1usize)],
                )),
            ),
            vec![],
            None,
            vec![kw(KW_INDEX, Nat)],
            T.clone(),
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_POP, t_pop, Some(FUNC_POP), 52);
        let t_clear = pr0_met(
            ref_mut(
                list_mut_t.clone(),
                Some(poly(MUT_LIST, vec![ty_tp(T.clone()), value(0usize)])),
            ),
            NoneType,
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_CLEAR, t_clear, Some(FUNC_CLEAR), 61);
        let t_sort = pr_met(
            ref_mut(list_mut_t.clone(), None),
            vec![],
            None,
            vec![kw(
                KW_KEY,
                no_var_func(vec![kw(KW_X, T.clone())], vec![], mono(ORD)),
            )],
            NoneType,
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_SORT, t_sort, Some(FUNC_SORT), 78);
        let t_reverse = pr0_met(ref_mut(list_mut_t.clone(), None), NoneType).quantify();
        list_mut_.register_py_builtin(PROC_REVERSE, t_reverse, Some(FUNC_REVERSE), 87);
        let t = pr_met(
            list_mut_t.clone(),
            vec![kw(KW_FUNC, nd_func(vec![anon(T.clone())], None, T.clone()))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_STRICT_MAP, t, None, 96);
        let t_update_nth = pr_met(
            ref_mut(list_mut_t.clone(), None),
            vec![kw(KW_IDX, Nat), kw(KW_FUNC, func1(T.clone(), T.clone()))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        list_mut_.register_py_builtin(PROC_UPDATE_NTH, t_update_nth, Some(FUNC_UPDATE_NTH), 105);
        let f_t = kw(
            KW_FUNC,
            no_var_func(vec![kw(KW_OLD, lis_t.clone())], vec![], lis_t.clone()),
        );
        let t = pr_met(
            ref_mut(list_mut_t.clone(), None),
            vec![f_t],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        let mut list_mut_mutable = Self::builtin_methods(Some(mono(MUTABLE)), 2);
        list_mut_mutable.register_builtin_py_impl(
            PROC_UPDATE,
            t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_UPDATE),
        );
        list_mut_.register_trait_methods(list_mut_t.clone(), list_mut_mutable);
        self.register_builtin_type(lis_t, list_, vis.clone(), Const, Some(LIST));
        self.register_builtin_type(list_mut_t, list_mut_, vis.clone(), Const, Some(LIST));
        /* ByteArray! */
        let bytearray_mut_t = mono(MUT_BYTEARRAY);
        let mut bytearray_mut = Self::builtin_mono_class(MUT_BYTEARRAY, 2);
        let mut bytearray_seq = Self::builtin_methods(Some(poly(SEQUENCE, vec![ty_tp(Int)])), 2);
        bytearray_seq.register_builtin_erg_impl(
            FUNDAMENTAL_LEN,
            fn0_met(mono(MUT_BYTEARRAY), Nat),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        bytearray_mut.register_trait_methods(mono(MUT_BYTEARRAY), bytearray_seq);
        bytearray_mut
            .register_trait(self, poly(SEQUENCE, vec![ty_tp(Int)]))
            .unwrap();
        let t_append = pr_met(
            ref_mut(bytearray_mut_t.clone(), None),
            vec![kw(KW_ELEM, int_interval(IntervalOp::Closed, 0, 255))],
            None,
            vec![],
            NoneType,
        );
        bytearray_mut.register_builtin_py_impl(
            PROC_PUSH,
            t_append,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_APPEND),
        );
        let t_copy = fn0_met(ref_(bytearray_mut_t.clone()), bytearray_mut_t.clone());
        let mut bytearray_mut_copy = Self::builtin_methods(Some(mono(COPY)), 2);
        bytearray_mut_copy.register_builtin_py_impl(
            FUNC_COPY,
            t_copy,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_COPY),
        );
        bytearray_mut.register_trait_methods(bytearray_mut_t.clone(), bytearray_mut_copy);
        let t_extend = pr_met(
            ref_mut(bytearray_mut_t.clone(), None),
            vec![kw(
                KW_ITERABLE,
                poly(
                    ITERABLE,
                    vec![ty_tp(int_interval(IntervalOp::Closed, 0, 255))],
                ),
            )],
            None,
            vec![],
            NoneType,
        );
        bytearray_mut.register_builtin_py_impl(
            PROC_EXTEND,
            t_extend,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_EXTEND),
        );
        let t_insert = pr_met(
            ref_mut(bytearray_mut_t.clone(), None),
            vec![
                kw(KW_INDEX, Nat),
                kw(KW_ELEM, int_interval(IntervalOp::Closed, 0, 255)),
            ],
            None,
            vec![],
            NoneType,
        );
        bytearray_mut.register_builtin_py_impl(
            PROC_INSERT,
            t_insert,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_INSERT),
        );
        let t_pop = pr0_met(
            ref_mut(bytearray_mut_t.clone(), None),
            int_interval(IntervalOp::Closed, 0, 255),
        );
        bytearray_mut.register_builtin_py_impl(
            PROC_POP,
            t_pop,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_POP),
        );
        let t_reverse = pr0_met(ref_mut(bytearray_mut_t.clone(), None), NoneType);
        bytearray_mut.register_builtin_py_impl(
            PROC_REVERSE,
            t_reverse,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_REVERSE),
        );
        let t_call = func0(bytearray_mut_t.clone())
            & no_var_func(
                vec![kw(KW_STR, Str), kw(KW_ENCODING, Str)],
                vec![kw(KW_ERRORS, Str)],
                bytearray_mut_t.clone(),
            )
            // (iterable_of_ints) -> bytes | (bytes_or_buffer) -> bytes & nat -> bytes
            & nd_func(
                // TODO: Bytes-like
                vec![pos(poly(ITERABLE, vec![ty_tp(Nat)]) | Nat | mono(BYTES) | bytearray_mut_t.clone())],
                None,
                bytearray_mut_t.clone(),
            );
        bytearray_mut.register_builtin_erg_impl(
            FUNDAMENTAL_CALL,
            t_call,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        /* Dict! */
        let dict_mut_t = poly(MUT_DICT, vec![D.clone()]);
        let mut dict_mut =
            Self::builtin_poly_class(MUT_DICT, vec![PS::named_nd(TY_D, mono(GENERIC_DICT))], 3);
        dict_mut.register_superclass(dict_t.clone(), &dict_);
        let K = type_q(TY_K);
        let V = type_q(TY_V);
        let insert_t = pr_met(
            ref_mut(
                dict_mut_t.clone(),
                Some(poly(
                    MUT_DICT,
                    vec![D.clone() + dict! { K.clone() => V.clone() }.into()],
                )),
            ),
            vec![kw(KW_KEY, K.clone()), kw(KW_VALUE, V.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        dict_mut.register_py_builtin(PROC_INSERT, insert_t, Some(FUNDAMENTAL_SETITEM), 12);
        let remove_t = pr_met(
            ref_mut(
                dict_mut_t.clone(),
                Some(poly(
                    MUT_DICT,
                    vec![D
                        .clone()
                        .proj_call(FUNC_DIFF.into(), vec![dict! { K.clone() => Never }.into()])],
                )),
            ),
            vec![kw(KW_KEY, K.clone())],
            None,
            vec![],
            proj_call(D.clone(), FUNDAMENTAL_GETITEM, vec![ty_tp(K.clone())]) | NoneType,
        )
        .quantify();
        dict_mut.register_py_builtin(PROC_REMOVE, remove_t, Some(FUNC_REMOVE), 19);
        let update_t = pr_met(
            ref_mut(
                dict_mut_t.clone(),
                Some(poly(
                    MUT_DICT,
                    vec![D.clone() + dict! { K.clone() => V.clone() }.into()],
                )),
            ),
            vec![kw(
                KW_ITERABLE,
                poly(ITERABLE, vec![ty_tp(tuple_t(vec![K.clone(), V.clone()]))]),
            )],
            None,
            vec![kw(
                KW_CONFLICT_RESOLVER,
                func2(V.clone(), V.clone(), V.clone()),
            )],
            NoneType,
        )
        .quantify();
        dict_mut.register_py_builtin(PROC_UPDATE, update_t, Some(FUNC_UPDATE), 26);
        let merge_t = pr_met(
            ref_mut(
                dict_mut_t.clone(),
                Some(poly(
                    MUT_DICT,
                    vec![D.proj_call(FUNC_CONCAT.into(), vec![D2.clone()])],
                )),
            ),
            vec![kw(KW_OTHER, poly(DICT, vec![D2.clone()]))],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        dict_mut.register_py_builtin(PROC_MERGE, merge_t, Some(FUNC_MERGE), 32);
        /* Set! */
        let set_mut_t = poly(MUT_SET, vec![ty_tp(T.clone()), N]);
        let mut set_mut_ =
            Self::builtin_poly_class(MUT_SET, vec![PS::t_nd(TY_T), PS::named_nd(TY_N, Nat)], 2);
        set_mut_.register_superclass(set_t.clone(), &set_);
        let mut set_mut_copy = Self::builtin_methods(Some(mono(COPY)), 1);
        set_mut_copy.register_py_builtin(
            FUNC_COPY,
            fn0_met(ref_(set_mut_t.clone()), set_mut_t.clone()).quantify(),
            Some(FUNC_COPY),
            9,
        );
        set_mut_.register_trait_methods(set_mut_t.clone(), set_mut_copy);
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
            no_var_func(vec![kw(KW_OLD, set_t.clone())], vec![], set_t.clone()),
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
        set_mut_.register_trait_methods(set_mut_t.clone(), set_mut_mutable);
        /* Range */
        let range_t = poly(RANGE, vec![TyParam::t(T.clone())]);
        let mut range = Self::builtin_poly_class(RANGE, vec![PS::t_nd(TY_T)], 2);
        // range.register_superclass(Obj, &obj);
        range.register_superclass(Type, &type_);
        range
            .register_trait(self, poly(OUTPUT, vec![ty_tp(T.clone())]))
            .unwrap();
        range
            .register_trait(self, poly(SEQUENCE, vec![ty_tp(T.clone())]))
            .unwrap();
        let mut range_eq = Self::builtin_methods(Some(mono(EQ)), 2);
        range_eq.register_builtin_erg_impl(
            OP_EQ,
            fn1_met(range_t.clone(), range_t.clone(), Bool).quantify(),
            Const,
            Visibility::BUILTIN_PUBLIC,
        );
        range.register_trait_methods(range_t.clone(), range_eq);
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
            None,
            ValueObj::builtin_class(range_iter),
        );
        range.register_trait_methods(range_t.clone(), range_iterable);
        let range_getitem_t = fn1_kw_met(range_t.clone(), anon(T.clone()), T.clone()).quantify();
        let get_item = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNDAMENTAL_GETITEM,
            __range_getitem__,
            range_getitem_t,
            None,
        )));
        range.register_builtin_const(
            FUNDAMENTAL_GETITEM,
            Visibility::BUILTIN_PUBLIC,
            None,
            get_item,
        );
        let mut subr = Self::builtin_mono_class(SUBROUTINE, 2);
        subr.register_superclass(Obj, &obj);
        let t_return = fn1_met(mono(SUBROUTINE), Obj, Never).quantify();
        subr.register_builtin_erg_impl(
            FUNC_RETURN,
            t_return,
            Immutable,
            Visibility::BUILTIN_PRIVATE,
        );
        let mut g_generator = Self::builtin_mono_class(GENERIC_GENERATOR, 2);
        g_generator.register_superclass(mono(SUBROUTINE), &subr);
        let t_yield = fn1_met(mono(GENERIC_GENERATOR), Obj, Never).quantify();
        g_generator.register_builtin_erg_impl(
            FUNC_YIELD,
            t_yield,
            Immutable,
            Visibility::BUILTIN_PRIVATE,
        );
        let mut base_exception = Self::builtin_mono_class(BASE_EXCEPTION, 2);
        base_exception.register_superclass(Obj, &obj);
        base_exception.register_builtin_erg_impl(
            ATTR_ARGS,
            unknown_len_list_t(Str),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let EXC = type_q(TY_E);
        base_exception.register_builtin_erg_impl(
            FUNC_WITH_TRACEBACK,
            fn1_met(EXC.clone(), mono(TRACEBACK), EXC).quantify(),
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        base_exception.register_builtin_erg_impl(
            FUNDAMENTAL_TRACEBACK,
            mono(TRACEBACK) | NoneType,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        base_exception.register_builtin_erg_impl(
            FUNDAMENTAL_SUPPRESS_CONTEXT,
            Bool,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        base_exception.register_builtin_erg_impl(
            FUNDAMENTAL_CAUSE,
            mono(BASE_EXCEPTION) | NoneType,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        base_exception.register_builtin_erg_impl(
            FUNDAMENTAL_CONTEXT,
            mono(BASE_EXCEPTION) | NoneType,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut traceback = Self::builtin_mono_class(TRACEBACK, 2);
        traceback.register_superclass(Obj, &obj);
        traceback.register_builtin_erg_impl(ATTR_TB_FRAME, Frame, Immutable, vis.clone());
        traceback.register_builtin_erg_impl(ATTR_TB_LASTI, Nat, Immutable, vis.clone());
        traceback.register_builtin_erg_impl(ATTR_TB_LINENO, Nat, Immutable, vis.clone());
        traceback.register_builtin_erg_impl(
            ATTR_TB_NEXT,
            mono(TRACEBACK) | NoneType,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut exception = Self::builtin_mono_class(EXCEPTION, 2);
        exception.register_superclass(mono(BASE_EXCEPTION), &base_exception);
        let mut system_exit = Self::builtin_mono_class(SYSTEM_EXIT, 2);
        system_exit.register_superclass(mono(BASE_EXCEPTION), &base_exception);
        system_exit.register_builtin_erg_impl(ATTR_CODE, Obj, Immutable, vis.clone());
        let mut keyboard_interrupt = Self::builtin_mono_class(KEYBOARD_INTERRUPT, 1);
        keyboard_interrupt.register_superclass(mono(BASE_EXCEPTION), &base_exception);
        let mut generator_exit = Self::builtin_mono_class(GENERATOR_EXIT, 2);
        generator_exit.register_superclass(mono(BASE_EXCEPTION), &base_exception);
        let mut stop_iteration = Self::builtin_mono_class(STOP_ITERATION, 2);
        stop_iteration.register_superclass(mono(EXCEPTION), &exception);
        stop_iteration.register_builtin_erg_impl(ATTR_VALUE, Obj, Immutable, vis.clone());
        let mut stop_async_iteration = Self::builtin_mono_class(STOP_ASYNC_ITERATION, 2);
        stop_async_iteration.register_superclass(mono(EXCEPTION), &exception);
        stop_async_iteration.register_builtin_erg_impl(ATTR_VALUE, Obj, Immutable, vis.clone());
        let mut arithmetic_error = Self::builtin_mono_class(ARITHMETIC_ERROR, 2);
        arithmetic_error.register_superclass(mono(EXCEPTION), &exception);
        let mut floating_point_error = Self::builtin_mono_class(FLOATING_POINT_ERROR, 2);
        floating_point_error.register_superclass(mono(ARITHMETIC_ERROR), &arithmetic_error);
        let mut overflow_error = Self::builtin_mono_class(OVERFLOW_ERROR, 2);
        overflow_error.register_superclass(mono(ARITHMETIC_ERROR), &arithmetic_error);
        let mut zero_division_error = Self::builtin_mono_class(ZERO_DIVISION_ERROR, 2);
        zero_division_error.register_superclass(mono(ARITHMETIC_ERROR), &arithmetic_error);
        let mut assertion_error = Self::builtin_mono_class(ASSERTION_ERROR, 2);
        assertion_error.register_superclass(mono(EXCEPTION), &exception);
        let mut attribute_error = Self::builtin_mono_class(ATTRIBUTE_ERROR, 2);
        attribute_error.register_superclass(mono(EXCEPTION), &exception);
        let mut buffer_error = Self::builtin_mono_class(BUFFER_ERROR, 2);
        buffer_error.register_superclass(mono(EXCEPTION), &exception);
        let mut eof_error = Self::builtin_mono_class(EOF_ERROR, 2);
        eof_error.register_superclass(mono(EXCEPTION), &exception);
        let mut import_error = Self::builtin_mono_class(IMPORT_ERROR, 2);
        import_error.register_superclass(mono(EXCEPTION), &exception);
        import_error.register_builtin_erg_impl(ATTR_MSG, Str, Immutable, vis.clone());
        import_error.register_builtin_erg_impl(ATTR_NAME, Str, Immutable, vis.clone());
        import_error.register_builtin_erg_impl(ATTR_PATH, Str, Immutable, vis.clone());
        let mut module_not_found_error = Self::builtin_mono_class(MODULE_NOT_FOUND_ERROR, 2);
        module_not_found_error.register_superclass(mono(IMPORT_ERROR), &import_error);
        let mut lookup_error = Self::builtin_mono_class(LOOKUP_ERROR, 1);
        lookup_error.register_superclass(mono(EXCEPTION), &exception);
        let mut index_error = Self::builtin_mono_class(INDEX_ERROR, 1);
        index_error.register_superclass(mono(LOOKUP_ERROR), &lookup_error);
        let mut key_error = Self::builtin_mono_class(KEY_ERROR, 1);
        key_error.register_superclass(mono(LOOKUP_ERROR), &lookup_error);
        let mut memory_error = Self::builtin_mono_class(MEMORY_ERROR, 1);
        memory_error.register_superclass(mono(EXCEPTION), &exception);
        let mut name_error = Self::builtin_mono_class(NAME_ERROR, 1);
        name_error.register_superclass(mono(EXCEPTION), &exception);
        let mut unbound_local_error = Self::builtin_mono_class(UNBOUND_LOCAL_ERROR, 2);
        unbound_local_error.register_superclass(mono(NAME_ERROR), &name_error);
        let mut os_error = Self::builtin_mono_class(OS_ERROR, 2);
        os_error.register_superclass(mono(EXCEPTION), &exception);
        os_error.register_builtin_erg_impl(ATTR_ERRNO, Int, Immutable, vis.clone());
        os_error.register_builtin_erg_impl(ATTR_FILENAME, Str, Immutable, vis.clone());
        os_error.register_builtin_erg_impl(ATTR_FILENAME2, Str, Immutable, vis.clone());
        os_error.register_builtin_erg_impl(ATTR_STRERROR, Str, Immutable, vis.clone());
        let mut blocking_io_error = Self::builtin_mono_class(BLOCKING_IO_ERROR, 1);
        blocking_io_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut child_process_error = Self::builtin_mono_class(CHILD_PROCESS_ERROR, 1);
        child_process_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut connection_error = Self::builtin_mono_class(CONNECTION_ERROR, 1);
        connection_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut broken_pipe_error = Self::builtin_mono_class(BROKEN_PIPE_ERROR, 1);
        broken_pipe_error.register_superclass(mono(CONNECTION_ERROR), &connection_error);
        let mut connection_aborted_error = Self::builtin_mono_class(CONNECTION_ABORTED_ERROR, 1);
        connection_aborted_error.register_superclass(mono(CONNECTION_ERROR), &connection_error);
        let mut connection_refused_error = Self::builtin_mono_class(CONNECTION_REFUSED_ERROR, 1);
        connection_refused_error.register_superclass(mono(CONNECTION_ERROR), &connection_error);
        let mut connection_reset_error = Self::builtin_mono_class(CONNECTION_RESET_ERROR, 1);
        connection_reset_error.register_superclass(mono(CONNECTION_ERROR), &connection_error);
        let mut file_exists_error = Self::builtin_mono_class(FILE_EXISTS_ERROR, 1);
        file_exists_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut file_not_found_error = Self::builtin_mono_class(FILE_NOT_FOUND_ERROR, 1);
        file_not_found_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut interrupted_error = Self::builtin_mono_class(INTERRUPTED_ERROR, 1);
        interrupted_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut is_a_directory_error = Self::builtin_mono_class(IS_A_DIRECTORY_ERROR, 1);
        is_a_directory_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut not_a_directory_error = Self::builtin_mono_class(NOT_A_DIRECTORY_ERROR, 1);
        not_a_directory_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut permission_error = Self::builtin_mono_class(PERMISSION_ERROR, 1);
        permission_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut process_lookup_error = Self::builtin_mono_class(PROCESS_LOOKUP_ERROR, 1);
        process_lookup_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut timeout_error = Self::builtin_mono_class(TIMEOUT_ERROR, 1);
        timeout_error.register_superclass(mono(OS_ERROR), &os_error);
        let mut reference_error = Self::builtin_mono_class(REFERENCE_ERROR, 1);
        reference_error.register_superclass(mono(EXCEPTION), &exception);
        let mut runtime_error = Self::builtin_mono_class(RUNTIME_ERROR, 2);
        runtime_error.register_superclass(mono(EXCEPTION), &exception);
        let mut not_implemented_error = Self::builtin_mono_class(NOT_IMPLEMENTED_ERROR, 1);
        not_implemented_error.register_superclass(mono(RUNTIME_ERROR), &runtime_error);
        let mut recursion_error = Self::builtin_mono_class(RECURSION_ERROR, 1);
        recursion_error.register_superclass(mono(RUNTIME_ERROR), &runtime_error);
        let mut syntax_error = Self::builtin_mono_class(SYNTAX_ERROR, 2);
        syntax_error.register_superclass(mono(EXCEPTION), &exception);
        let mut indentation_error = Self::builtin_mono_class(INDENTATION_ERROR, 2);
        indentation_error.register_superclass(mono(SYNTAX_ERROR), &syntax_error);
        let mut tab_error = Self::builtin_mono_class(TAB_ERROR, 2);
        tab_error.register_superclass(mono(INDENTATION_ERROR), &indentation_error);
        let mut system_error = Self::builtin_mono_class(SYSTEM_ERROR, 2);
        system_error.register_superclass(mono(EXCEPTION), &exception);
        let mut type_error = Self::builtin_mono_class(TYPE_ERROR, 2);
        type_error.register_superclass(mono(EXCEPTION), &exception);
        let mut value_error = Self::builtin_mono_class(VALUE_ERROR, 2);
        value_error.register_superclass(mono(EXCEPTION), &exception);
        let mut unicode_error = Self::builtin_mono_class(UNICODE_ERROR, 2);
        unicode_error.register_superclass(mono(VALUE_ERROR), &value_error);
        let mut unicode_encode_error = Self::builtin_mono_class(UNICODE_ENCODE_ERROR, 2);
        unicode_encode_error.register_superclass(mono(UNICODE_ERROR), &unicode_error);
        let mut unicode_decode_error = Self::builtin_mono_class(UNICODE_DECODE_ERROR, 2);
        unicode_decode_error.register_superclass(mono(UNICODE_ERROR), &unicode_error);
        let mut unicode_translate_error = Self::builtin_mono_class(UNICODE_TRANSLATE_ERROR, 2);
        unicode_translate_error.register_superclass(mono(UNICODE_ERROR), &unicode_error);
        let mut warning = Self::builtin_mono_class(WARNING, 2);
        warning.register_superclass(mono(EXCEPTION), &exception);
        let mut deprecation_warning = Self::builtin_mono_class(DEPRECATION_WARNING, 2);
        deprecation_warning.register_superclass(mono(WARNING), &warning);
        let mut pending_deprecation_warning =
            Self::builtin_mono_class(PENDING_DEPRECATION_WARNING, 2);
        pending_deprecation_warning
            .register_superclass(mono(DEPRECATION_WARNING), &deprecation_warning);
        let mut runtime_warning = Self::builtin_mono_class(RUNTIME_WARNING, 2);
        runtime_warning.register_superclass(mono(WARNING), &warning);
        let mut syntax_warning = Self::builtin_mono_class(SYNTAX_WARNING, 2);
        syntax_warning.register_superclass(mono(WARNING), &warning);
        let mut user_warning = Self::builtin_mono_class(USER_WARNING, 2);
        user_warning.register_superclass(mono(WARNING), &warning);
        let mut future_warning = Self::builtin_mono_class(FUTURE_WARNING, 2);
        future_warning.register_superclass(mono(WARNING), &warning);
        let mut import_warning = Self::builtin_mono_class(IMPORT_WARNING, 2);
        import_warning.register_superclass(mono(WARNING), &warning);
        let mut unicode_warning = Self::builtin_mono_class(UNICODE_WARNING, 2);
        unicode_warning.register_superclass(mono(WARNING), &warning);
        let mut bytes_warning = Self::builtin_mono_class(BYTES_WARNING, 2);
        bytes_warning.register_superclass(mono(WARNING), &warning);
        let mut resource_warning = Self::builtin_mono_class(RESOURCE_WARNING, 2);
        resource_warning.register_superclass(mono(WARNING), &warning);
        /* Proc */
        let mut proc = Self::builtin_mono_class(PROC, 2);
        proc.register_superclass(mono(SUBROUTINE), &subr);
        let mut named_proc = Self::builtin_mono_class(NAMED_PROC, 2);
        named_proc.register_superclass(mono(PROC), &proc);
        named_proc.register_trait(self, mono(NAMED)).unwrap();
        /* Func */
        let mut func = Self::builtin_mono_class(FUNC, 2);
        func.register_superclass(mono(PROC), &proc);
        let mut named_func = Self::builtin_mono_class(NAMED_FUNC, 2);
        named_func.register_superclass(mono(FUNC), &func);
        named_func.register_trait(self, mono(NAMED)).unwrap();
        let mut quant = Self::builtin_mono_class(QUANTIFIED, 2);
        quant.register_superclass(mono(PROC), &proc);
        let mut qproc = Self::builtin_mono_class(QUANTIFIED_PROC, 2);
        qproc.register_superclass(mono(PROC), &proc);
        let mut qfunc = Self::builtin_mono_class(QUANTIFIED_FUNC, 2);
        qfunc.register_superclass(mono(QUANTIFIED_PROC), &qproc);
        qfunc.register_superclass(mono(FUNC), &func);
        let mut proc_meta_type = Self::builtin_mono_class(PROC_META_TYPE, 2);
        proc_meta_type.register_superclass(mono(PROC), &proc);
        proc_meta_type.register_superclass(Type, &type_);
        let mut func_meta_type = Self::builtin_mono_class(FUNC_META_TYPE, 2);
        func_meta_type.register_superclass(mono(FUNC), &func);
        func_meta_type.register_superclass(mono(PROC_META_TYPE), &proc_meta_type);
        let mut qproc_meta_type = Self::builtin_mono_class(QUANTIFIED_PROC_META_TYPE, 2);
        qproc_meta_type.register_superclass(mono(PROC_META_TYPE), &proc);
        qproc_meta_type.register_superclass(mono(QUANTIFIED_PROC), &qproc);
        let mut qfunc_meta_type = Self::builtin_mono_class(QUANTIFIED_FUNC_META_TYPE, 2);
        qfunc_meta_type.register_superclass(mono(QUANTIFIED_PROC_META_TYPE), &qproc_meta_type);
        qfunc_meta_type.register_superclass(mono(QUANTIFIED_FUNC), &qfunc);
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
        self.register_builtin_type(Frame, frame, vis.clone(), Const, Some(FRAME_TYPE));
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
            mono(GENERIC_LIST),
            generic_list,
            vis.clone(),
            Const,
            Some(LIST),
        );
        self.register_builtin_type(
            unsized_list_t,
            unsized_list,
            vis.clone(),
            Const,
            Some(UNSIZED_LIST),
        );
        self.register_builtin_type(mono(SLICE), slice, vis.clone(), Const, Some(FUNC_SLICE));
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
        self.register_builtin_type(mono(RECORD), record, vis.clone(), Const, Some(RECORD));
        self.register_builtin_type(
            mono(RECORD_META_TYPE),
            record_meta_type,
            vis.clone(),
            Const,
            Some(FUNC_META_TYPE),
        );
        self.register_builtin_type(
            mono(GENERIC_NAMED_TUPLE),
            generic_named_tuple,
            vis.clone(),
            Const,
            Some(GENERIC_NAMED_TUPLE),
        );
        self.register_builtin_type(or_t, or, vis.clone(), Const, Some(UNION));
        self.register_builtin_type(
            mono(STR_ITERATOR),
            str_iterator,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_STR_ITERATOR),
        );
        self.register_builtin_type(
            poly(LIST_ITERATOR, vec![ty_tp(T.clone())]),
            list_iterator,
            Visibility::BUILTIN_PRIVATE,
            Const,
            Some(FUNC_LIST_ITERATOR),
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
        self.register_builtin_type(
            bytearray_mut_t,
            bytearray_mut,
            vis.clone(),
            Const,
            Some(FUNC_BYTEARRAY),
        );
        self.register_builtin_type(dict_mut_t, dict_mut, vis.clone(), Const, Some(DICT));
        self.register_builtin_type(set_mut_t, set_mut_, vis.clone(), Const, Some(SET));
        self.register_builtin_type(mono(SUBROUTINE), subr, vis.clone(), Const, Some(SUBROUTINE));
        self.register_builtin_type(
            mono(GENERIC_GENERATOR),
            g_generator,
            vis.clone(),
            Const,
            Some(GENERATOR),
        );
        self.register_builtin_type(
            mono(BASE_EXCEPTION),
            base_exception,
            vis.clone(),
            Const,
            Some(BASE_EXCEPTION),
        );
        self.register_builtin_type(
            mono(TRACEBACK),
            traceback,
            vis.clone(),
            Const,
            Some(TRACEBACK),
        );
        self.register_builtin_type(
            mono(EXCEPTION),
            exception,
            vis.clone(),
            Const,
            Some(EXCEPTION),
        );
        self.register_builtin_type(
            mono(SYSTEM_EXIT),
            system_exit,
            vis.clone(),
            Const,
            Some(SYSTEM_EXIT),
        );
        self.register_builtin_type(
            mono(KEYBOARD_INTERRUPT),
            keyboard_interrupt,
            vis.clone(),
            Const,
            Some(KEYBOARD_INTERRUPT),
        );
        self.register_builtin_type(
            mono(GENERATOR_EXIT),
            generator_exit,
            vis.clone(),
            Const,
            Some(GENERATOR_EXIT),
        );
        self.register_builtin_type(
            mono(STOP_ITERATION),
            stop_iteration,
            vis.clone(),
            Const,
            Some(STOP_ITERATION),
        );
        self.register_builtin_type(
            mono(STOP_ASYNC_ITERATION),
            stop_async_iteration,
            vis.clone(),
            Const,
            Some(STOP_ASYNC_ITERATION),
        );
        self.register_builtin_type(
            mono(ARITHMETIC_ERROR),
            arithmetic_error,
            vis.clone(),
            Const,
            Some(ARITHMETIC_ERROR),
        );
        self.register_builtin_type(
            mono(FLOATING_POINT_ERROR),
            floating_point_error,
            vis.clone(),
            Const,
            Some(FLOATING_POINT_ERROR),
        );
        self.register_builtin_type(
            mono(OVERFLOW_ERROR),
            overflow_error,
            vis.clone(),
            Const,
            Some(OVERFLOW_ERROR),
        );
        self.register_builtin_type(
            mono(ZERO_DIVISION_ERROR),
            zero_division_error,
            vis.clone(),
            Const,
            Some(ZERO_DIVISION_ERROR),
        );
        self.register_builtin_type(
            mono(ASSERTION_ERROR),
            assertion_error,
            vis.clone(),
            Const,
            Some(ASSERTION_ERROR),
        );
        self.register_builtin_type(
            mono(ATTRIBUTE_ERROR),
            attribute_error,
            vis.clone(),
            Const,
            Some(ATTRIBUTE_ERROR),
        );
        self.register_builtin_type(
            mono(BUFFER_ERROR),
            buffer_error,
            vis.clone(),
            Const,
            Some(BUFFER_ERROR),
        );
        self.register_builtin_type(
            mono(EOF_ERROR),
            eof_error,
            vis.clone(),
            Const,
            Some(EOF_ERROR),
        );
        self.register_builtin_type(
            mono(IMPORT_ERROR),
            import_error,
            vis.clone(),
            Const,
            Some(IMPORT_ERROR),
        );
        self.register_builtin_type(
            mono(MODULE_NOT_FOUND_ERROR),
            module_not_found_error,
            vis.clone(),
            Const,
            Some(MODULE_NOT_FOUND_ERROR),
        );
        self.register_builtin_type(
            mono(LOOKUP_ERROR),
            lookup_error,
            vis.clone(),
            Const,
            Some(LOOKUP_ERROR),
        );
        self.register_builtin_type(
            mono(INDEX_ERROR),
            index_error,
            vis.clone(),
            Const,
            Some(INDEX_ERROR),
        );
        self.register_builtin_type(
            mono(KEY_ERROR),
            key_error,
            vis.clone(),
            Const,
            Some(KEY_ERROR),
        );
        self.register_builtin_type(
            mono(MEMORY_ERROR),
            memory_error,
            vis.clone(),
            Const,
            Some(MEMORY_ERROR),
        );
        self.register_builtin_type(
            mono(NAME_ERROR),
            name_error,
            vis.clone(),
            Const,
            Some(NAME_ERROR),
        );
        self.register_builtin_type(
            mono(UNBOUND_LOCAL_ERROR),
            unbound_local_error,
            vis.clone(),
            Const,
            Some(UNBOUND_LOCAL_ERROR),
        );
        self.register_builtin_type(mono(OS_ERROR), os_error, vis.clone(), Const, Some(OS_ERROR));
        self.register_builtin_type(
            mono(BLOCKING_IO_ERROR),
            blocking_io_error,
            vis.clone(),
            Const,
            Some(BLOCKING_IO_ERROR),
        );
        self.register_builtin_type(
            mono(CHILD_PROCESS_ERROR),
            child_process_error,
            vis.clone(),
            Const,
            Some(CHILD_PROCESS_ERROR),
        );
        self.register_builtin_type(
            mono(CONNECTION_ERROR),
            connection_error,
            vis.clone(),
            Const,
            Some(CONNECTION_ERROR),
        );
        self.register_builtin_type(
            mono(BROKEN_PIPE_ERROR),
            broken_pipe_error,
            vis.clone(),
            Const,
            Some(BROKEN_PIPE_ERROR),
        );
        self.register_builtin_type(
            mono(CONNECTION_ABORTED_ERROR),
            connection_aborted_error,
            vis.clone(),
            Const,
            Some(CONNECTION_ABORTED_ERROR),
        );
        self.register_builtin_type(
            mono(CONNECTION_REFUSED_ERROR),
            connection_refused_error,
            vis.clone(),
            Const,
            Some(CONNECTION_REFUSED_ERROR),
        );
        self.register_builtin_type(
            mono(CONNECTION_RESET_ERROR),
            connection_reset_error,
            vis.clone(),
            Const,
            Some(CONNECTION_RESET_ERROR),
        );
        self.register_builtin_type(
            mono(FILE_EXISTS_ERROR),
            file_exists_error,
            vis.clone(),
            Const,
            Some(FILE_EXISTS_ERROR),
        );
        self.register_builtin_type(
            mono(FILE_NOT_FOUND_ERROR),
            file_not_found_error,
            vis.clone(),
            Const,
            Some(FILE_NOT_FOUND_ERROR),
        );
        self.register_builtin_type(
            mono(INTERRUPTED_ERROR),
            interrupted_error,
            vis.clone(),
            Const,
            Some(INTERRUPTED_ERROR),
        );
        self.register_builtin_type(
            mono(IS_A_DIRECTORY_ERROR),
            is_a_directory_error,
            vis.clone(),
            Const,
            Some(IS_A_DIRECTORY_ERROR),
        );
        self.register_builtin_type(
            mono(NOT_A_DIRECTORY_ERROR),
            not_a_directory_error,
            vis.clone(),
            Const,
            Some(NOT_A_DIRECTORY_ERROR),
        );
        self.register_builtin_type(
            mono(PERMISSION_ERROR),
            permission_error,
            vis.clone(),
            Const,
            Some(PERMISSION_ERROR),
        );
        self.register_builtin_type(
            mono(PROCESS_LOOKUP_ERROR),
            process_lookup_error,
            vis.clone(),
            Const,
            Some(PROCESS_LOOKUP_ERROR),
        );
        self.register_builtin_type(
            mono(TIMEOUT_ERROR),
            timeout_error,
            vis.clone(),
            Const,
            Some(TIMEOUT_ERROR),
        );
        self.register_builtin_type(
            mono(REFERENCE_ERROR),
            reference_error,
            vis.clone(),
            Const,
            Some(REFERENCE_ERROR),
        );
        self.register_builtin_type(
            mono(RUNTIME_ERROR),
            runtime_error,
            vis.clone(),
            Const,
            Some(RUNTIME_ERROR),
        );
        self.register_builtin_type(
            mono(NOT_IMPLEMENTED_ERROR),
            not_implemented_error,
            vis.clone(),
            Const,
            Some(NOT_IMPLEMENTED_ERROR),
        );
        self.register_builtin_type(
            mono(RECURSION_ERROR),
            recursion_error,
            vis.clone(),
            Const,
            Some(RECURSION_ERROR),
        );
        self.register_builtin_type(
            mono(SYNTAX_ERROR),
            syntax_error,
            vis.clone(),
            Const,
            Some(SYNTAX_ERROR),
        );
        self.register_builtin_type(
            mono(INDENTATION_ERROR),
            indentation_error,
            vis.clone(),
            Const,
            Some(INDENTATION_ERROR),
        );
        self.register_builtin_type(
            mono(TAB_ERROR),
            tab_error,
            vis.clone(),
            Const,
            Some(TAB_ERROR),
        );
        self.register_builtin_type(
            mono(SYSTEM_ERROR),
            system_error,
            vis.clone(),
            Const,
            Some(SYSTEM_ERROR),
        );
        self.register_builtin_type(
            mono(TYPE_ERROR),
            type_error,
            vis.clone(),
            Const,
            Some(TYPE_ERROR),
        );
        self.register_builtin_type(
            mono(VALUE_ERROR),
            value_error,
            vis.clone(),
            Const,
            Some(VALUE_ERROR),
        );
        self.register_builtin_type(
            mono(UNICODE_ERROR),
            unicode_error,
            vis.clone(),
            Const,
            Some(UNICODE_ERROR),
        );
        self.register_builtin_type(
            mono(UNICODE_ENCODE_ERROR),
            unicode_encode_error,
            vis.clone(),
            Const,
            Some(UNICODE_ENCODE_ERROR),
        );
        self.register_builtin_type(
            mono(UNICODE_DECODE_ERROR),
            unicode_decode_error,
            vis.clone(),
            Const,
            Some(UNICODE_DECODE_ERROR),
        );
        self.register_builtin_type(
            mono(UNICODE_TRANSLATE_ERROR),
            unicode_translate_error,
            vis.clone(),
            Const,
            Some(UNICODE_TRANSLATE_ERROR),
        );
        self.register_builtin_type(mono(WARNING), warning, vis.clone(), Const, Some(WARNING));
        self.register_builtin_type(
            mono(DEPRECATION_WARNING),
            deprecation_warning,
            vis.clone(),
            Const,
            Some(DEPRECATION_WARNING),
        );
        self.register_builtin_type(
            mono(PENDING_DEPRECATION_WARNING),
            pending_deprecation_warning,
            vis.clone(),
            Const,
            Some(PENDING_DEPRECATION_WARNING),
        );
        self.register_builtin_type(
            mono(RUNTIME_WARNING),
            runtime_warning,
            vis.clone(),
            Const,
            Some(RUNTIME_WARNING),
        );
        self.register_builtin_type(
            mono(SYNTAX_WARNING),
            syntax_warning,
            vis.clone(),
            Const,
            Some(SYNTAX_WARNING),
        );
        self.register_builtin_type(
            mono(USER_WARNING),
            user_warning,
            vis.clone(),
            Const,
            Some(USER_WARNING),
        );
        self.register_builtin_type(
            mono(FUTURE_WARNING),
            future_warning,
            vis.clone(),
            Const,
            Some(FUTURE_WARNING),
        );
        self.register_builtin_type(
            mono(IMPORT_WARNING),
            import_warning,
            vis.clone(),
            Const,
            Some(IMPORT_WARNING),
        );
        self.register_builtin_type(
            mono(UNICODE_WARNING),
            unicode_warning,
            vis.clone(),
            Const,
            Some(UNICODE_WARNING),
        );
        self.register_builtin_type(
            mono(BYTES_WARNING),
            bytes_warning,
            vis.clone(),
            Const,
            Some(BYTES_WARNING),
        );
        self.register_builtin_type(
            mono(RESOURCE_WARNING),
            resource_warning,
            vis.clone(),
            Const,
            Some(RESOURCE_WARNING),
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
                mono(QUANTIFIED_PROC),
                qproc,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(QUANTIFIED_PROC),
            );
            self.register_builtin_type(
                mono(QUANTIFIED_FUNC),
                qfunc,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(QUANTIFIED_FUNC),
            );
            self.register_builtin_type(
                mono(PROC_META_TYPE),
                proc_meta_type,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(PROC_META_TYPE),
            );
            self.register_builtin_type(
                mono(FUNC_META_TYPE),
                func_meta_type,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(FUNC_META_TYPE),
            );
            self.register_builtin_type(
                mono(QUANTIFIED_PROC_META_TYPE),
                qproc_meta_type,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(QUANTIFIED_PROC_META_TYPE),
            );
            self.register_builtin_type(
                mono(QUANTIFIED_FUNC_META_TYPE),
                qfunc_meta_type,
                Visibility::BUILTIN_PRIVATE,
                Const,
                Some(QUANTIFIED_FUNC_META_TYPE),
            );
        } else {
            self.register_builtin_const(MUT_INT, vis.clone(), None, ValueObj::builtin_class(Int));
            self.register_builtin_const(MUT_STR, vis, None, ValueObj::builtin_class(Str));
        }
    }
}
