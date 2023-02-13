#[allow(unused_imports)]
use erg_common::log;
use erg_common::vis::Visibility;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::Type;
use Type::*;

use crate::context::initialize::*;
use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(super) fn init_builtin_funcs(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let T = mono_q(TY_T, instanceof(Type));
        let U = mono_q(TY_U, instanceof(Type));
        let Path = mono_q_tp(PATH, instanceof(Str));
        let t_abs = nd_func(vec![kw(KW_N, mono(NUM))], None, Nat);
        let t_all = func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(Bool)]))],
            None,
            vec![],
            Bool,
        );
        let t_any = func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(Bool)]))],
            None,
            vec![],
            Bool,
        );
        let t_ascii = nd_func(vec![kw(KW_OBJECT, Obj)], None, Str);
        let t_assert = func(
            vec![kw(KW_TEST, Bool)],
            None,
            vec![kw(KW_MSG, Str)],
            NoneType,
        );
        let t_bin = nd_func(vec![kw(KW_N, Int)], None, Str);
        // TODO: overload: Iterable(Int) -> Bytes
        let t_bytes = nd_func(
            vec![kw(KW_STR, Str), kw(KW_ENCODING, Str)],
            None,
            mono(BYTES),
        );
        let t_chr = nd_func(
            vec![kw(KW_I, Type::from(value(0usize)..=value(1_114_111usize)))],
            None,
            Str,
        );
        let t_classof = nd_func(vec![kw(KW_OLD, Obj)], None, ClassType);
        let t_compile = nd_func(vec![kw(KW_SRC, Str)], None, Code);
        let t_cond = nd_func(
            vec![
                kw(KW_TEST, Bool),
                kw(KW_THEN, T.clone()),
                kw(KW_ELSE, T.clone()),
            ],
            None,
            T.clone(),
        )
        .quantify();
        let t_discard = nd_func(vec![kw(KW_OBJ, Obj)], None, NoneType);
        let t_enumerate = func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            None,
            vec![kw(KW_START, Int)],
            poly(ENUMERATE, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let t_if = func(
            vec![
                kw(KW_COND, Bool),
                kw(KW_THEN, nd_func(vec![], None, T.clone())),
            ],
            None,
            vec![kw_default(
                KW_ELSE,
                nd_func(vec![], None, U.clone()),
                nd_func(vec![], None, NoneType),
            )],
            or(T.clone(), U.clone()),
        )
        .quantify();
        let t_int = nd_func(vec![kw(KW_OBJ, Obj)], None, or(Int, NoneType));
        let t_import = nd_func(
            vec![anon(tp_enum(Str, set! {Path.clone()}))],
            None,
            module(Path.clone()),
        )
        .quantify();
        let t_isinstance = nd_func(
            vec![
                kw(KW_OBJECT, Obj),
                kw(KW_CLASSINFO, ClassType), // TODO: => ClassInfo
            ],
            None,
            Bool,
        );
        let t_issubclass = nd_func(
            vec![
                kw(KW_SUBCLASS, ClassType),
                kw(KW_CLASSINFO, ClassType), // TODO: => ClassInfo
            ],
            None,
            Bool,
        );
        let I = mono_q(TY_I, subtypeof(poly(ITERABLE, vec![ty_tp(T.clone())])));
        let t_iter = nd_func(vec![kw(KW_OBJECT, I.clone())], None, proj(I, ITERATOR)).quantify();
        let t_len = nd_func(
            vec![kw(KW_S, poly(SEQ, vec![TyParam::erased(Type)]))],
            None,
            Nat,
        );
        let t_log = func(
            vec![],
            Some(kw(KW_OBJECTS, ref_(Obj))),
            vec![
                kw(KW_SEP, Str),
                kw(KW_END, Str),
                kw(KW_FILE, mono(WRITE)),
                kw(KW_FLUSH, Bool),
            ],
            NoneType,
        );
        let t_map = nd_func(
            vec![
                kw(KW_PROC, nd_proc(vec![anon(T.clone())], None, T.clone())),
                kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())])),
            ],
            None,
            poly(MAP, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let O = mono_q(TY_O, subtypeof(mono(ORD)));
        // TODO: iterable should be non-empty
        let t_max = nd_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(O.clone())]))],
            None,
            O.clone(),
        )
        .quantify();
        let t_min = nd_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(O.clone())]))],
            None,
            O,
        )
        .quantify();
        let t_nat = nd_func(vec![kw(KW_OBJ, Obj)], None, or(Nat, NoneType));
        // e.g. not(b: Bool!): Bool!
        let B = mono_q(TY_B, subtypeof(Bool));
        let t_not = nd_func(vec![kw(KW_B, B.clone())], None, B).quantify();
        let t_oct = nd_func(vec![kw(KW_X, Int)], None, Str);
        let t_ord = nd_func(vec![kw(KW_C, Str)], None, Nat);
        let t_panic = nd_func(vec![kw(KW_MSG, Str)], None, Never);
        let M = mono_q(TY_M, Constraint::Uninited);
        let M = mono_q(TY_M, subtypeof(poly(MUL, vec![ty_tp(M)])));
        // TODO: mod
        let t_pow = nd_func(
            vec![kw(KW_BASE, M.clone()), kw(KW_EXP, M.clone())],
            None,
            proj(M, OUTPUT),
        )
        .quantify();
        let t_pyimport = nd_func(
            vec![anon(tp_enum(Str, set! {Path.clone()}))],
            None,
            py_module(Path),
        )
        .quantify();
        let t_pycompile = nd_func(
            vec![kw(KW_SRC, Str), kw(KW_FILENAME, Str), kw(KW_MODE, Str)],
            None,
            Code,
        );
        let t_quit = func(vec![], None, vec![kw(KW_CODE, Int)], Never);
        let t_exit = t_quit.clone();
        let t_repr = nd_func(vec![kw(KW_OBJECT, Obj)], None, Str);
        let t_reversed = nd_func(
            vec![kw(KW_SEQ, poly(SEQ, vec![ty_tp(T.clone())]))],
            None,
            poly(REVERSED, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let t_round = nd_func(vec![kw(KW_NUMBER, Float)], None, Int);
        let t_sorted = nd_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            None,
            array_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let t_str = nd_func(vec![kw(KW_OBJECT, Obj)], None, Str);
        let A = mono_q(TY_A, Constraint::Uninited);
        let A = mono_q(TY_A, subtypeof(poly(ADD, vec![ty_tp(A)])));
        let t_sum = func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(A.clone())]))],
            None,
            vec![kw_default(KW_START, or(A.clone(), Int), Int)],
            A,
        )
        .quantify();
        let t_unreachable = nd_func(vec![], None, Never);
        let t_zip = nd_func(
            vec![
                kw(KW_ITERABLE1, poly(ITERABLE, vec![ty_tp(T.clone())])),
                kw(KW_ITERABLE2, poly(ITERABLE, vec![ty_tp(U.clone())])),
            ],
            None,
            poly(ZIP, vec![ty_tp(T.clone()), ty_tp(U.clone())]),
        )
        .quantify();
        self.register_py_builtin(FUNC_ABS, t_abs, Some(FUNC_ABS), 17);
        self.register_py_builtin(FUNC_ALL, t_all, Some(FUNC_ALL), 29);
        self.register_py_builtin(FUNC_ANY, t_any, Some(FUNC_ANY), 41);
        self.register_py_builtin(FUNC_ASCII, t_ascii, Some(FUNC_ASCII), 67);
        // Leave as `Const`, as it may negatively affect assert casting.
        self.register_builtin_erg_impl(FUNC_ASSERT, t_assert, Const, vis);
        self.register_builtin_py_impl(FUNC_BIN, t_bin, Immutable, vis, Some(FUNC_BIN));
        self.register_builtin_py_impl(FUNC_BYTES, t_bytes, Immutable, vis, Some(FUNC_BYTES));
        self.register_builtin_py_impl(FUNC_CHR, t_chr, Immutable, vis, Some(FUNC_CHR));
        self.register_builtin_py_impl(FUNC_CLASSOF, t_classof, Immutable, vis, Some(FUNC_TYPE));
        self.register_builtin_py_impl(FUNC_COMPILE, t_compile, Immutable, vis, Some(FUNC_COMPILE));
        self.register_builtin_erg_impl(KW_COND, t_cond, Immutable, vis);
        self.register_builtin_py_impl(
            FUNC_ENUMERATE,
            t_enumerate,
            Immutable,
            vis,
            Some(FUNC_ENUMERATE),
        );
        self.register_builtin_py_impl(FUNC_EXIT, t_exit, Immutable, vis, Some(FUNC_EXIT));
        self.register_builtin_py_impl(
            FUNC_ISINSTANCE,
            t_isinstance,
            Immutable,
            vis,
            Some(FUNC_ISINSTANCE),
        );
        self.register_builtin_py_impl(
            FUNC_ISSUBCLASS,
            t_issubclass,
            Immutable,
            vis,
            Some(FUNC_ISSUBCLASS),
        );
        self.register_builtin_py_impl(FUNC_ITER, t_iter, Immutable, vis, Some(FUNC_ITER));
        self.register_builtin_py_impl(FUNC_LEN, t_len, Immutable, vis, Some(FUNC_LEN));
        self.register_builtin_py_impl(FUNC_MAP, t_map, Immutable, vis, Some(FUNC_MAP));
        self.register_builtin_py_impl(FUNC_MAX, t_max, Immutable, vis, Some(FUNC_MAX));
        self.register_builtin_py_impl(FUNC_MIN, t_min, Immutable, vis, Some(FUNC_MIN));
        self.register_builtin_py_impl(FUNC_NOT, t_not, Immutable, vis, None); // `not` is not a function in Python
        self.register_builtin_py_impl(FUNC_OCT, t_oct, Immutable, vis, Some(FUNC_OCT));
        self.register_builtin_py_impl(FUNC_ORD, t_ord, Immutable, vis, Some(FUNC_ORD));
        self.register_builtin_py_impl(FUNC_POW, t_pow, Immutable, vis, Some(FUNC_POW));
        self.register_builtin_py_impl(
            PYIMPORT,
            t_pyimport.clone(),
            Immutable,
            vis,
            Some(FUNDAMENTAL_IMPORT),
        );
        self.register_builtin_py_impl(FUNC_QUIT, t_quit, Immutable, vis, Some(FUNC_QUIT));
        self.register_builtin_py_impl(FUNC_REPR, t_repr, Immutable, vis, Some(FUNC_REPR));
        self.register_builtin_py_impl(
            FUNC_REVERSED,
            t_reversed,
            Immutable,
            vis,
            Some(FUNC_REVERSED),
        );
        self.register_builtin_py_impl(FUNC_ROUND, t_round, Immutable, vis, Some(FUNC_ROUND));
        self.register_builtin_py_impl(FUNC_SORTED, t_sorted, Immutable, vis, Some(FUNC_SORTED));
        self.register_builtin_py_impl(FUNC_STR, t_str, Immutable, vis, Some(FUNC_STR__));
        self.register_builtin_py_impl(FUNC_SUM, t_sum, Immutable, vis, Some(FUNC_SUM));
        self.register_builtin_py_impl(FUNC_ZIP, t_zip, Immutable, vis, Some(FUNC_ZIP));
        let name = if cfg!(feature = "py_compatible") {
            FUNC_INT
        } else {
            FUNC_INT__
        };
        self.register_builtin_py_impl(FUNC_INT, t_int, Immutable, vis, Some(name));
        if !cfg!(feature = "py_compatible") {
            self.register_builtin_py_impl(FUNC_IF, t_if, Immutable, vis, Some(FUNC_IF__));
            self.register_builtin_py_impl(
                FUNC_DISCARD,
                t_discard,
                Immutable,
                vis,
                Some(FUNC_DISCARD__),
            );
            self.register_builtin_py_impl(
                FUNC_IMPORT,
                t_import,
                Immutable,
                vis,
                Some(FUNDAMENTAL_IMPORT),
            );
            self.register_builtin_py_impl(FUNC_LOG, t_log, Immutable, vis, Some(FUNC_PRINT));
            self.register_builtin_py_impl(FUNC_NAT, t_nat, Immutable, vis, Some(FUNC_NAT__));
            self.register_builtin_py_impl(FUNC_PANIC, t_panic, Immutable, vis, Some(FUNC_QUIT));
            if cfg!(feature = "debug") {
                self.register_builtin_py_impl(
                    PY,
                    t_pyimport,
                    Immutable,
                    vis,
                    Some(FUNDAMENTAL_IMPORT),
                );
            }
            self.register_builtin_py_impl(
                PYCOMPILE,
                t_pycompile,
                Immutable,
                vis,
                Some(FUNC_COMPILE),
            );
            // TODO: original implementation
            self.register_builtin_py_impl(
                FUNC_UNREACHABLE,
                t_unreachable,
                Immutable,
                vis,
                Some(FUNC_EXIT),
            );
        } else {
            let t_range = func(
                vec![kw(KW_STOP, or(Int, NoneType))],
                None,
                vec![
                    kw(KW_START, or(Int, NoneType)),
                    kw(KW_STEP, or(Int, NoneType)),
                ],
                poly(RANGE, vec![ty_tp(Int)]),
            );
            self.register_builtin_py_impl(FUNC_RANGE, t_range, Immutable, vis, Some(FUNC_RANGE));
            let t_list = func(
                vec![],
                None,
                vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
                poly(ARRAY, vec![ty_tp(T.clone()), TyParam::erased(Nat)]),
            )
            .quantify();
            self.register_builtin_py_impl(FUNC_LIST, t_list, Immutable, vis, Some(FUNC_LIST));
            let t_dict = func(
                vec![],
                None,
                vec![kw(
                    KW_ITERABLE,
                    poly(ITERABLE, vec![ty_tp(tuple_t(vec![T.clone(), U.clone()]))]),
                )],
                dict! { T => U }.into(),
            )
            .quantify();
            self.register_builtin_py_impl(FUNC_DICT, t_dict, Immutable, vis, Some(FUNC_DICT));
        }
    }

    pub(super) fn init_builtin_const_funcs(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let class_t = func(
            vec![],
            None,
            vec![kw(KW_REQUIREMENT, or(Type, Ellipsis)), kw(KW_IMPL, Type)],
            ClassType,
        );
        let class = ConstSubr::Builtin(BuiltinConstSubr::new(CLASS, class_func, class_t, None));
        self.register_builtin_const(CLASS, vis, ValueObj::Subr(class));
        let inherit_t = func(
            vec![kw(KW_SUPER, ClassType)],
            None,
            vec![kw(KW_IMPL, Type), kw(KW_ADDITIONAL, Type)],
            ClassType,
        );
        let inherit = ConstSubr::Builtin(BuiltinConstSubr::new(
            INHERIT,
            inherit_func,
            inherit_t,
            None,
        ));
        self.register_builtin_const(INHERIT, vis, ValueObj::Subr(inherit));
        let trait_t = func(
            vec![kw(KW_REQUIREMENT, Type)],
            None,
            vec![kw(KW_IMPL, Type)],
            TraitType,
        );
        let trait_ = ConstSubr::Builtin(BuiltinConstSubr::new(TRAIT, trait_func, trait_t, None));
        self.register_builtin_const(TRAIT, vis, ValueObj::Subr(trait_));
        let subsume_t = func(
            vec![kw(KW_SUPER, TraitType)],
            None,
            vec![kw(KW_IMPL, Type), kw(KW_ADDITIONAL, Type)],
            TraitType,
        );
        let subsume = ConstSubr::Builtin(BuiltinConstSubr::new(
            SUBSUME,
            subsume_func,
            subsume_t,
            None,
        ));
        self.register_builtin_const(SUBSUME, vis, ValueObj::Subr(subsume));
        // decorators
        let inheritable_t = func1(ClassType, ClassType);
        let inheritable = ConstSubr::Builtin(BuiltinConstSubr::new(
            INHERITABLE,
            inheritable_func,
            inheritable_t,
            None,
        ));
        self.register_builtin_const(INHERITABLE, vis, ValueObj::Subr(inheritable));
        // TODO: register Del function object
        let t_del = nd_func(vec![kw(KW_OBJ, Obj)], None, NoneType);
        self.register_builtin_erg_impl(DEL, t_del, Immutable, vis);
        let patch_t = func(
            vec![kw(KW_REQUIREMENT, Type)],
            None,
            vec![kw(KW_IMPL, Type)],
            TraitType,
        );
        let patch = ConstSubr::Builtin(BuiltinConstSubr::new(PATCH, patch_func, patch_t, None));
        self.register_builtin_const(PATCH, vis, ValueObj::Subr(patch));
    }

    pub(super) fn init_builtin_operators(&mut self) {
        /* binary */
        let R = mono_q(TY_R, instanceof(Type));
        let params = vec![ty_tp(R.clone())];
        let L = mono_q(TY_L, subtypeof(poly(ADD, params.clone())));
        let op_t = nd_func(
            vec![kw(KW_LHS, L.clone()), kw(KW_RHS, R.clone())],
            None,
            proj(L, OUTPUT),
        )
        .quantify();
        self.register_builtin_erg_impl(OP_ADD, op_t, Const, Private);
        let L = mono_q(TY_L, subtypeof(poly(SUB, params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify();
        self.register_builtin_erg_impl(OP_SUB, op_t, Const, Private);
        let L = mono_q(TY_L, subtypeof(poly(MUL, params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify();
        self.register_builtin_erg_impl(OP_MUL, op_t, Const, Private);
        let L = mono_q(TY_L, subtypeof(poly(DIV, params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify();
        self.register_builtin_erg_impl(OP_DIV, op_t, Const, Private);
        let L = mono_q(TY_L, subtypeof(poly(FLOOR_DIV, params)));
        let op_t = bin_op(L.clone(), R, proj(L, OUTPUT)).quantify();
        self.register_builtin_erg_impl(OP_FLOOR_DIV, op_t, Const, Private);
        let P = mono_q(TY_P, Constraint::Uninited);
        let P = mono_q(TY_P, subtypeof(poly(MUL, vec![ty_tp(P)])));
        let op_t = bin_op(P.clone(), P.clone(), proj(P, POW_OUTPUT)).quantify();
        // TODO: add bound: M == M.Output
        self.register_builtin_erg_impl(OP_POW, op_t, Const, Private);
        let M = mono_q(TY_M, Constraint::Uninited);
        let M = mono_q(TY_M, subtypeof(poly(DIV, vec![ty_tp(M)])));
        let op_t = bin_op(M.clone(), M.clone(), proj(M, MOD_OUTPUT)).quantify();
        self.register_builtin_erg_impl(OP_MOD, op_t, Const, Private);
        let op_t = nd_proc(vec![kw(KW_LHS, Obj), kw(KW_RHS, Obj)], None, Bool);
        self.register_builtin_erg_impl(OP_IS, op_t.clone(), Const, Private);
        self.register_builtin_erg_impl(OP_IS_NOT, op_t, Const, Private);
        let E = mono_q(TY_E, subtypeof(mono(EQ)));
        let op_t = bin_op(E.clone(), E, Bool).quantify();
        self.register_builtin_erg_impl(OP_EQ, op_t.clone(), Const, Private);
        self.register_builtin_erg_impl(OP_NE, op_t, Const, Private);
        let O = mono_q(TY_O, subtypeof(mono(ORD)));
        let op_t = bin_op(O.clone(), O.clone(), Bool).quantify();
        self.register_builtin_erg_impl(OP_LT, op_t.clone(), Const, Private);
        self.register_builtin_erg_impl(OP_LE, op_t.clone(), Const, Private);
        self.register_builtin_erg_impl(OP_GT, op_t.clone(), Const, Private);
        self.register_builtin_erg_impl(OP_GE, op_t, Const, Private);
        let BT = mono_q(TY_BT, subtypeof(or(Bool, Type)));
        let op_t = bin_op(BT.clone(), BT.clone(), BT).quantify();
        self.register_builtin_erg_impl(OP_AND, op_t.clone(), Const, Private);
        self.register_builtin_erg_impl(OP_OR, op_t, Const, Private);
        let op_t = bin_op(O.clone(), O.clone(), range(O)).quantify();
        self.register_builtin_erg_decl(OP_RNG, op_t.clone(), Private);
        self.register_builtin_erg_decl(OP_LORNG, op_t.clone(), Private);
        self.register_builtin_erg_decl(OP_RORNG, op_t.clone(), Private);
        self.register_builtin_erg_decl(OP_ORNG, op_t, Private);
        // TODO: use existential type: |T: Type| (T, In(T)) -> Bool
        let T = mono_q(TY_T, instanceof(Type));
        let I = mono_q(KW_I, subtypeof(poly(IN, vec![ty_tp(T.clone())])));
        let op_t = bin_op(I, T, Bool).quantify();
        self.register_builtin_erg_impl(OP_IN, op_t.clone(), Const, Private);
        self.register_builtin_erg_impl(OP_NOT_IN, op_t, Const, Private);
        /* unary */
        // TODO: +/- Bool would like to be warned
        let M = mono_q(TY_M, subtypeof(mono(MUTIZABLE)));
        let op_t = func1(M.clone(), proj(M, MUTABLE_MUT_TYPE)).quantify();
        self.register_builtin_erg_impl(OP_MUTATE, op_t, Const, Private);
        let N = mono_q(TY_N, subtypeof(mono(NUM)));
        let op_t = func1(N.clone(), N).quantify();
        self.register_builtin_erg_decl(OP_POS, op_t.clone(), Private);
        self.register_builtin_erg_decl(OP_NEG, op_t, Private);
    }
}
