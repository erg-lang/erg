use erg_common::consts::{DEBUG_MODE, ERG_MODE, GAL, PYTHON_MODE};
#[allow(unused_imports)]
use erg_common::log;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{CastTarget, Field, Type, Visibility};
use Type::*;

use crate::context::initialize::*;
use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;

impl Context {
    pub(super) fn init_builtin_funcs(&mut self) {
        let vis = if PYTHON_MODE {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::BUILTIN_PRIVATE
        };
        let T = mono_q(TY_T, instanceof(Type));
        let U = mono_q(TY_U, instanceof(Type));
        let Path = mono_q_tp(PATH, instanceof(Str));
        let t_abs = nd_func(vec![kw(KW_N, mono(NUM))], None, Nat);
        let abs = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ABS,
            abs_func,
            t_abs.clone(),
            None,
        )));
        let t_all = no_var_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(Bool)]))],
            vec![],
            Bool,
        );
        let all = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ALL,
            all_func,
            t_all.clone(),
            None,
        )));
        let t_any = no_var_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(Bool)]))],
            vec![],
            Bool,
        );
        let any = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ANY,
            any_func,
            t_any.clone(),
            None,
        )));
        let t_ascii = nd_func(vec![kw(KW_OBJECT, Obj)], None, Str);
        let t_assert = no_var_func(vec![kw(KW_TEST, Bool)], vec![kw(KW_MSG, Str)], NoneType);
        let t_bin = nd_func(vec![kw(KW_N, Int)], None, Str);
        let t_bytes = func0(mono(BYTES))
            & no_var_func(
                vec![kw(KW_STR, Str), kw(KW_ENCODING, Str)],
                vec![kw(KW_ERRORS, Str)],
                mono(BYTES),
            )
            // (iterable_of_ints) -> bytes | (bytes_or_buffer) -> bytes & nat -> bytes
            & nd_func(
                // TODO: Bytes-like
                vec![pos(poly(ITERABLE, vec![ty_tp(Nat)]) | Nat | mono(BYTES))],
                None,
                mono(BYTES),
            );
        let t_bytes_array = no_var_func(
            vec![],
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(Int)]))],
            mono(MUT_BYTEARRAY),
        );
        let t_callable = func1(Obj, Bool);
        let t_chr_in = if PYTHON_MODE {
            Int
        } else {
            Type::from(value(0usize)..=value(1_114_111usize))
        };
        let t_chr = nd_func(vec![kw(KW_I, t_chr_in)], None, Str);
        let F = mono_q(TY_F, instanceof(mono(GENERIC_CALLABLE)));
        let t_classmethod = nd_func(vec![kw(KW_FUNC, F.clone())], None, F.clone()).quantify();
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
        let t_dict = no_var_func(
            vec![],
            vec![kw(
                KW_ITERABLE,
                poly(ITERABLE, vec![ty_tp(tuple_t(vec![T.clone(), U.clone()]))]),
            )],
            dict! { T.clone() => U.clone() }.into(),
        )
        .quantify();
        let t_discard = nd_func(vec![kw(KW_OBJ, Obj)], None, NoneType);
        let M = mono_q(TY_M, Constraint::Uninited);
        let M = mono_q(TY_M, subtypeof(poly(MUL, vec![ty_tp(M)])));
        let Out = M.clone().proj(MOD_OUTPUT);
        let t_divmod = nd_func(
            vec![kw(KW_A, M.clone()), kw(KW_B, M.clone())],
            None,
            tuple_t(vec![Out.clone(), Out]),
        )
        .quantify();
        let t_enumerate = no_var_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            vec![kw(KW_START, Int)],
            poly(ENUMERATE, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let grd = guard(
            "<builtins>".into(),
            CastTarget::arg(0, "x".into(), Location::Unknown),
            U.clone(),
        );
        let t_filter = nd_func(
            vec![
                kw(KW_FUNC, nd_func(vec![kw("x", T.clone())], None, grd)),
                kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())])),
            ],
            None,
            poly(FILTER, vec![ty_tp(T.clone() & U.clone())]),
        )
        .quantify()
            & nd_func(
                vec![
                    kw(KW_FUNC, nd_func(vec![anon(T.clone())], None, Bool)),
                    kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())])),
                ],
                None,
                poly(FILTER, vec![ty_tp(T.clone())]),
            )
            .quantify();
        let filter = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_FILTER,
            filter_func,
            t_filter.clone(),
            None,
        )));
        let t_float = default_func(vec![kw(KW_OBJ, Obj)], Float);
        let t_format = no_var_func(vec![kw(KW_VALUE, Obj)], vec![kw(KW_SPEC, Str)], Str);
        let t_frozenset = no_var_func(
            vec![],
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            poly(FROZENSET, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let getattr_t = no_var_func(
            vec![kw(KW_OBJ, Obj), kw(KW_NAME, Str)],
            vec![kw_default(KW_DEFAULT, T.clone(), Obj)],
            T.clone(),
        )
        .quantify();
        let hasattr_t = no_var_func(vec![kw(KW_OBJ, Obj), kw(KW_NAME, Str)], vec![], Bool);
        let t_hash = func1(mono(HASH), Int);
        let t_hex = nd_func(vec![kw(KW_N, Int)], None, Str);
        let t_if = no_var_func(
            vec![
                kw(KW_COND, Bool),
                kw(KW_THEN, nd_func(vec![], None, T.clone())),
            ],
            vec![kw_default(
                KW_ELSE,
                nd_func(vec![], None, U.clone()),
                nd_func(vec![], None, NoneType),
            )],
            or(T.clone(), U.clone()),
        )
        .quantify();
        let t_int = no_var_func(vec![kw(KW_OBJ, Obj)], vec![kw(KW_BASE, Nat)], Int);
        let t_import = nd_func(
            vec![anon(tp_enum(Str, set! {Path.clone()}))],
            None,
            module(TyParam::app(FUNC_RESOLVE_PATH.into(), vec![Path.clone()])),
        )
        .quantify();
        let t_isinstance = nd_func(
            vec![
                kw(KW_OBJECT, Obj),
                kw(
                    KW_CLASSINFO,
                    ClassType | type_poly(HOMOGENOUS_TUPLE, vec![ClassType]),
                ), // TODO: => ClassInfo
            ],
            None,
            Bool,
        );
        let t_issubclass = nd_func(
            vec![
                kw(KW_SUBCLASS, ClassType),
                kw(
                    KW_CLASSINFO,
                    ClassType | type_poly(HOMOGENOUS_TUPLE, vec![ClassType]),
                ), // TODO: => ClassInfo
            ],
            None,
            Bool,
        );
        let I = mono_q(TY_I, subtypeof(poly(ITERABLE, vec![ty_tp(T.clone())])));
        let t_iter = nd_func(vec![kw(KW_OBJECT, I.clone())], None, proj(I, ITERATOR)).quantify();
        // Python : |L|(seq: Structural({ .__len__ = (L) -> Nat })) -> Nat
        let t_len = if ERG_MODE {
            nd_func(
                vec![kw(KW_S, poly(SEQUENCE, vec![TyParam::erased(Type)]))],
                None,
                Nat,
            )
        } else {
            let S =
                Type::from(dict! { Field::public(FUNDAMENTAL_LEN.into()) => fn0_met(Never, Nat) })
                    .structuralize();
            func1(S, Nat)
        };
        let len = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_LEN,
            len_func,
            t_len.clone(),
            None,
        )));
        let t_list = no_var_func(
            vec![],
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            list_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let t_log = func(
            vec![],
            Some(kw(KW_OBJECTS, ref_(Obj))),
            vec![
                kw(KW_SEP, Str),
                kw(KW_END, Str),
                kw(KW_FILE, mono(WRITE)),
                kw(KW_FLUSH, Bool),
            ],
            None,
            NoneType,
        );
        let t_map = nd_func(
            vec![
                kw(KW_FUNC, nd_func(vec![anon(T.clone())], None, U.clone())),
                kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())])),
            ],
            None,
            poly(MAP, vec![ty_tp(U.clone())]),
        )
        .quantify();
        let map = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_MAP,
            map_func,
            t_map.clone(),
            None,
        )));
        let O = mono_q(TY_O, subtypeof(mono(ORD)));
        // TODO: iterable should be non-empty
        let t_max = nd_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(O.clone())]))],
            None,
            O.clone(),
        )
        .quantify();
        let max = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_MAX,
            max_func,
            t_max.clone(),
            None,
        )));
        let t_memoryview = nd_func(
            vec![kw(
                KW_OBJ,
                mono(BYTES) | mono(MUT_BYTEARRAY) | mono("array.Array!"),
            )],
            None,
            mono(MEMORYVIEW),
        );
        let t_min = nd_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(O.clone())]))],
            None,
            O,
        )
        .quantify();
        let min = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_MIN,
            min_func,
            t_min.clone(),
            None,
        )));
        let t_nat = nd_func(vec![kw(KW_OBJ, Obj)], None, Nat);
        // e.g. not(b: Bool!): Bool!
        let B = mono_q(TY_B, subtypeof(Bool));
        let t_not = nd_func(vec![kw(KW_B, B.clone())], None, B).quantify();
        let not = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_NOT,
            not_func,
            t_not.clone(),
            None,
        )));
        let t_object = nd_func(vec![], None, Obj);
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
            py_module(TyParam::app(FUNC_RESOLVE_DECL_PATH.into(), vec![Path])),
        )
        .quantify();
        let t_pycompile = nd_func(
            vec![kw(KW_SRC, Str), kw(KW_FILENAME, Str), kw(KW_MODE, Str)],
            None,
            Code,
        );
        let t_quit = no_var_func(vec![], vec![kw(KW_CODE, Int)], Never);
        let t_exit = t_quit.clone();
        let t_repr = nd_func(vec![kw(KW_OBJECT, Obj)], None, Str);
        let t_reversed = nd_func(
            vec![kw(KW_SEQ, poly(SEQUENCE, vec![ty_tp(T.clone())]))],
            None,
            poly(REVERSED, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let reversed = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_REVERSED,
            reversed_func,
            t_reversed.clone(),
            None,
        )));
        let t_round = nd_func(vec![kw(KW_NUMBER, Float)], None, Int);
        let t_set = no_var_func(
            vec![],
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            set_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let t_slice = no_var_func(
            vec![kw(KW_START, Int)],
            vec![kw(KW_STOP, Int), kw(KW_STEP, Int)],
            mono(SLICE),
        );
        let t_sorted = nd_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            None,
            list_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let t_staticmethod = nd_func(vec![kw(KW_FUNC, F.clone())], None, F.clone()).quantify();
        let t_str = nd_func(vec![kw(KW_OBJECT, Obj)], None, Str)
            & no_var_func(
                vec![kw(KW_BYTES_OR_BUFFER, mono(BYTES)), kw(KW_ENCODING, Str)],
                vec![kw(KW_ERRORS, Str)],
                Str,
            );
        let str_ = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_STR,
            str_func,
            t_str.clone(),
            None,
        )));
        let A = mono_q(TY_A, Constraint::Uninited);
        let A = mono_q(TY_A, subtypeof(poly(ADD, vec![ty_tp(A)])));
        let t_sum = no_var_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(A.clone())]))],
            vec![kw_default(KW_START, or(A.clone(), Int), Int)],
            A,
        )
        .quantify();
        let sum = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_SUM,
            sum_func,
            t_sum.clone(),
            None,
        )));
        let t_unreachable = d_func(vec![kw(KW_MSG, Obj)], Never);
        let t_vars = no_var_func(
            vec![],
            vec![kw_default(KW_OBJECT, Obj, Obj)],
            dict! { Str => Obj }.into(),
        );
        let t_zip = nd_func(
            vec![
                kw(KW_ITERABLE1, poly(ITERABLE, vec![ty_tp(T.clone())])),
                kw(KW_ITERABLE2, poly(ITERABLE, vec![ty_tp(U.clone())])),
            ],
            None,
            poly(ZIP, vec![ty_tp(T.clone()), ty_tp(U.clone())]),
        )
        .quantify();
        let zip = ValueObj::Subr(ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_ZIP,
            zip_func,
            t_zip.clone(),
            None,
        )));
        self.register_py_builtin_const(
            FUNC_ABS,
            vis.clone(),
            Some(t_abs),
            abs,
            Some(FUNC_ABS),
            Some(11),
        );
        self.register_py_builtin_const(
            FUNC_ALL,
            vis.clone(),
            Some(t_all),
            all,
            Some(FUNC_ALL),
            Some(22),
        );
        self.register_py_builtin_const(
            FUNC_ANY,
            vis.clone(),
            Some(t_any),
            any,
            Some(FUNC_ANY),
            Some(33),
        );
        self.register_py_builtin(FUNC_LIST, t_list, Some(FUNC_LIST), 215);
        self.register_py_builtin(FUNC_ASCII, t_ascii, Some(FUNC_ASCII), 53);
        // Leave as `Const`, as it may negatively affect assert casting.
        let name = if PYTHON_MODE { FUNC_ASSERT } else { "assert__" };
        self.register_builtin_py_impl(FUNC_ASSERT, t_assert, Const, vis.clone(), Some(name));
        self.register_builtin_py_impl(FUNC_BIN, t_bin, Immutable, vis.clone(), Some(FUNC_BIN));
        self.register_builtin_py_impl(
            FUNC_BYTES,
            t_bytes,
            Immutable,
            vis.clone(),
            Some(FUNC_BYTES),
        );
        self.register_builtin_py_impl(
            FUNC_BYTEARRAY,
            t_bytes_array,
            Immutable,
            vis.clone(),
            Some(FUNC_BYTEARRAY),
        );
        self.register_builtin_py_impl(
            FUNC_CALLABLE,
            t_callable,
            Immutable,
            vis.clone(),
            Some(FUNC_CALLABLE),
        );
        self.register_builtin_py_impl(FUNC_CHR, t_chr, Immutable, vis.clone(), Some(FUNC_CHR));
        self.register_builtin_py_impl(
            FUNC_CLASSMETHOD,
            t_classmethod,
            Immutable,
            vis.clone(),
            Some(FUNC_CLASSMETHOD),
        );
        self.register_builtin_py_impl(
            FUNC_COMPILE,
            t_compile,
            Immutable,
            vis.clone(),
            Some(FUNC_COMPILE),
        );
        self.register_builtin_erg_impl(KW_COND, t_cond, Immutable, vis.clone());
        self.register_py_builtin(FUNC_DICT, t_dict, Some(FUNC_DICT), 224);
        self.register_builtin_py_impl(
            FUNC_DIVMOD,
            t_divmod,
            Immutable,
            vis.clone(),
            Some(FUNC_DIVMOD),
        );
        self.register_builtin_py_impl(
            FUNC_ENUMERATE,
            t_enumerate,
            Immutable,
            vis.clone(),
            Some(FUNC_ENUMERATE),
        );
        self.register_builtin_py_impl(FUNC_EXIT, t_exit, Immutable, vis.clone(), Some(FUNC_EXIT));
        self.register_builtin_py_impl(
            FUNC_FLOAT,
            t_float,
            Immutable,
            vis.clone(),
            Some(FUNC_FLOAT),
        );
        self.register_builtin_py_impl(
            FUNC_FORMAT,
            t_format,
            Immutable,
            vis.clone(),
            Some(FUNC_FORMAT),
        );
        self.register_py_builtin_const(
            FUNC_FILTER,
            vis.clone(),
            Some(t_filter),
            filter,
            Some(FUNC_FILTER),
            None,
        );
        self.register_builtin_py_impl(
            FUNC_FROZENSET,
            t_frozenset,
            Immutable,
            vis.clone(),
            Some(FUNC_FROZENSET),
        );
        self.register_builtin_py_impl(
            FUNC_GETATTR,
            getattr_t,
            Immutable,
            vis.clone(),
            Some(FUNC_GETATTR),
        );
        self.register_builtin_py_impl(
            FUNC_HASATTR,
            hasattr_t,
            Immutable,
            vis.clone(),
            Some(FUNC_HASATTR),
        );
        self.register_builtin_py_impl(FUNC_HASH, t_hash, Immutable, vis.clone(), Some(FUNC_HASH));
        self.register_builtin_py_impl(FUNC_HEX, t_hex, Immutable, vis.clone(), Some(FUNC_HEX));
        self.register_builtin_py_impl(
            FUNC_ISINSTANCE,
            t_isinstance,
            Immutable,
            vis.clone(),
            Some(FUNC_ISINSTANCE),
        );
        self.register_builtin_py_impl(
            FUNC_ISSUBCLASS,
            t_issubclass,
            Immutable,
            vis.clone(),
            Some(FUNC_ISSUBCLASS),
        );
        self.register_builtin_py_impl(FUNC_ITER, t_iter, Immutable, vis.clone(), Some(FUNC_ITER));
        self.register_py_builtin_const(
            FUNC_LEN,
            vis.clone(),
            Some(t_len),
            len,
            Some(FUNC_LEN),
            None,
        );
        self.register_py_builtin_const(
            FUNC_MAP,
            vis.clone(),
            Some(t_map),
            map,
            Some(FUNC_MAP),
            None,
        );
        self.register_py_builtin_const(
            FUNC_MAX,
            vis.clone(),
            Some(t_max),
            max,
            Some(FUNC_MAX),
            None,
        );
        self.register_builtin_py_impl(
            FUNC_MEMORYVIEW,
            t_memoryview,
            Immutable,
            vis.clone(),
            Some(FUNC_MEMORYVIEW),
        );
        self.register_py_builtin_const(
            FUNC_MIN,
            vis.clone(),
            Some(t_min),
            min,
            Some(FUNC_MIN),
            None,
        );
        self.register_py_builtin_const(FUNC_NOT, vis.clone(), Some(t_not), not, None, None); // `not` is not a function in Python
        self.register_builtin_py_impl(
            FUNC_OBJECT,
            t_object,
            Immutable,
            vis.clone(),
            Some(FUNC_OBJECT),
        );
        self.register_builtin_py_impl(FUNC_OCT, t_oct, Immutable, vis.clone(), Some(FUNC_OCT));
        self.register_builtin_py_impl(FUNC_ORD, t_ord, Immutable, vis.clone(), Some(FUNC_ORD));
        self.register_builtin_py_impl(FUNC_POW, t_pow, Immutable, vis.clone(), Some(FUNC_POW));
        self.register_builtin_py_impl(
            PYIMPORT,
            t_pyimport.clone(),
            Immutable,
            vis.clone(),
            Some(FUNDAMENTAL_IMPORT),
        );
        self.register_builtin_py_impl(FUNC_QUIT, t_quit, Immutable, vis.clone(), Some(FUNC_QUIT));
        let MAX = mono_q_tp("MAX", instanceof(Int));
        let MIN = mono_q_tp("MIN", instanceof(Int));
        let t_range = nd_func(
            vec![kw(KW_START, singleton(Int, MAX.clone()))],
            None,
            poly(
                RANGE,
                vec![ty_tp((TyParam::value(0u64)..MAX.clone()).into())],
            ),
        )
        .quantify()
            & nd_func(
                vec![
                    kw(KW_START, singleton(Int, MIN.clone())),
                    kw(KW_STOP, singleton(Int, MAX.clone())),
                ],
                None,
                poly(RANGE, vec![ty_tp((MIN.clone()..MAX.clone()).into())]),
            )
            .quantify()
            & nd_func(
                vec![
                    kw(KW_START, singleton(Int, MIN.clone())),
                    kw(KW_STOP, singleton(Int, MAX.clone())),
                    kw(KW_STEP, Int),
                ],
                None,
                poly(RANGE, vec![ty_tp((MIN..MAX).into())]),
            )
            .quantify()
            & nd_func(vec![kw(KW_START, Int)], None, poly(RANGE, vec![ty_tp(Int)]))
            & nd_func(
                vec![kw(KW_START, Int), kw(KW_STOP, Int)],
                None,
                poly(RANGE, vec![ty_tp(Int)]),
            )
            & nd_func(
                vec![kw(KW_START, Int), kw(KW_STOP, Int), kw(KW_STEP, Int)],
                None,
                poly(RANGE, vec![ty_tp(Int)]),
            );
        self.register_builtin_py_impl(
            FUNC_RANGE,
            t_range,
            Immutable,
            vis.clone(),
            Some(FUNC_RANGE),
        );
        self.register_builtin_py_impl(FUNC_REPR, t_repr, Immutable, vis.clone(), Some(FUNC_REPR));
        self.register_py_builtin_const(
            FUNC_REVERSED,
            vis.clone(),
            Some(t_reversed),
            reversed,
            Some(FUNC_REVERSED),
            None,
        );
        self.register_builtin_py_impl(
            FUNC_ROUND,
            t_round,
            Immutable,
            vis.clone(),
            Some(FUNC_ROUND),
        );
        self.register_py_builtin(FUNC_SET, t_set, Some(FUNC_SET), 233);
        self.register_builtin_py_impl(
            FUNC_SLICE,
            t_slice,
            Immutable,
            vis.clone(),
            Some(FUNC_SLICE),
        );
        self.register_builtin_py_impl(
            FUNC_SORTED,
            t_sorted,
            Immutable,
            vis.clone(),
            Some(FUNC_SORTED),
        );
        self.register_builtin_py_impl(
            FUNC_STATICMETHOD,
            t_staticmethod,
            Immutable,
            vis.clone(),
            Some(FUNC_STATICMETHOD),
        );
        self.register_py_builtin_const(
            FUNC_STR,
            vis.clone(),
            Some(t_str),
            str_,
            Some(FUNC_STR__),
            None,
        );
        self.register_py_builtin_const(
            FUNC_SUM,
            vis.clone(),
            Some(t_sum),
            sum,
            Some(FUNC_SUM),
            None,
        );
        self.register_builtin_py_impl(FUNC_VARS, t_vars, Immutable, vis.clone(), Some(FUNC_VARS));
        self.register_py_builtin_const(
            FUNC_ZIP,
            vis.clone(),
            Some(t_zip),
            zip,
            Some(FUNC_ZIP),
            None,
        );
        let name = if PYTHON_MODE { FUNC_INT } else { FUNC_INT__ };
        self.register_builtin_py_impl(FUNC_INT, t_int, Immutable, vis.clone(), Some(name));
        if DEBUG_MODE {
            self.register_builtin_py_impl(
                PY,
                t_pyimport.clone(),
                Immutable,
                vis.clone(),
                Some(FUNDAMENTAL_IMPORT),
            );
        }
        if GAL {
            self.register_builtin_py_impl(
                RSIMPORT,
                t_pyimport.clone(),
                Immutable,
                vis.clone(),
                Some(FUNDAMENTAL_IMPORT),
            );
        }
        if ERG_MODE {
            self.register_builtin_py_impl(FUNC_IF, t_if, Immutable, vis.clone(), Some(FUNC_IF__));
            self.register_builtin_py_impl(
                FUNC_DISCARD,
                t_discard,
                Immutable,
                vis.clone(),
                Some(FUNC_DISCARD__),
            );
            self.register_builtin_py_impl(
                FUNC_IMPORT,
                t_import,
                Immutable,
                vis.clone(),
                Some(FUNDAMENTAL_IMPORT),
            );
            self.register_builtin_py_impl(
                FUNC_LOG,
                t_log,
                Immutable,
                vis.clone(),
                Some(FUNC_PRINT),
            );
            self.register_builtin_py_impl(
                FUNC_NAT,
                t_nat,
                Immutable,
                vis.clone(),
                Some(FUNC_NAT__),
            );
            self.register_builtin_py_impl(
                FUNC_PANIC,
                t_panic,
                Immutable,
                vis.clone(),
                Some(FUNC_QUIT),
            );
            self.register_builtin_py_impl(
                PYCOMPILE,
                t_pycompile,
                Immutable,
                vis.clone(),
                Some(FUNC_COMPILE),
            );
            // TODO: original implementation
            self.register_builtin_py_impl(
                FUNC_UNREACHABLE,
                t_unreachable.clone(),
                Immutable,
                vis.clone(),
                Some(FUNC_EXIT),
            );
            self.register_builtin_py_impl(
                FUNC_TODO,
                t_unreachable,
                Immutable,
                vis,
                Some(FUNC_EXIT),
            );
        } else {
            self.register_builtin_py_impl(
                PYIMPORT,
                t_pyimport,
                Immutable,
                Visibility::BUILTIN_PRIVATE,
                None,
            );
        }
    }

    pub(super) fn init_builtin_const_funcs(&mut self) {
        let vis = if PYTHON_MODE {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::BUILTIN_PRIVATE
        };
        let class_t = no_var_func(
            vec![],
            vec![kw(KW_REQUIREMENT, or(Type, Ellipsis)), kw(KW_IMPL, Type)],
            ClassType,
        );
        let class = ConstSubr::Builtin(BuiltinConstSubr::new(CLASS, class_func, class_t, None));
        self.register_builtin_const(CLASS, vis.clone(), None, ValueObj::Subr(class));
        let inherit_t = no_var_func(
            vec![kw(KW_SUPER, ClassType)],
            vec![kw(KW_IMPL, Type), kw(KW_ADDITIONAL, Type)],
            ClassType,
        );
        let inherit = ConstSubr::Builtin(BuiltinConstSubr::new(
            INHERIT,
            inherit_func,
            inherit_t,
            None,
        ));
        self.register_builtin_const(INHERIT, vis.clone(), None, ValueObj::Subr(inherit));
        let trait_t = no_var_func(
            vec![kw(KW_REQUIREMENT, Type)],
            vec![kw(KW_IMPL, Type)],
            TraitType,
        );
        let trait_ = ConstSubr::Builtin(BuiltinConstSubr::new(TRAIT, trait_func, trait_t, None));
        self.register_builtin_const(TRAIT, vis.clone(), None, ValueObj::Subr(trait_));
        let subsume_t = no_var_func(
            vec![kw(KW_SUPER, TraitType)],
            vec![kw(KW_IMPL, Type), kw(KW_ADDITIONAL, Type)],
            TraitType,
        );
        let subsume = ConstSubr::Builtin(BuiltinConstSubr::new(
            SUBSUME,
            subsume_func,
            subsume_t,
            None,
        ));
        self.register_builtin_const(SUBSUME, vis.clone(), None, ValueObj::Subr(subsume));
        let structural = ConstSubr::Builtin(BuiltinConstSubr::new(
            STRUCTURAL,
            structural_func,
            func1(Type, Type),
            None,
        ));
        self.register_builtin_const(STRUCTURAL, vis.clone(), None, ValueObj::Subr(structural));
        // decorators
        let inheritable_t = func1(ClassType, ClassType);
        let inheritable = ConstSubr::Builtin(BuiltinConstSubr::new(
            INHERITABLE,
            inheritable_func,
            inheritable_t,
            None,
        ));
        self.register_builtin_const(INHERITABLE, vis.clone(), None, ValueObj::Subr(inheritable));
        let F = mono_q(TY_F, instanceof(mono(GENERIC_CALLABLE)));
        let override_t = func1(F.clone(), F).quantify();
        let override_ = ConstSubr::Builtin(BuiltinConstSubr::new(
            OVERRIDE,
            override_func,
            override_t,
            None,
        ));
        self.register_builtin_const(OVERRIDE, vis.clone(), None, ValueObj::Subr(override_));
        // TODO: register Del function object
        let t_del = nd_func(vec![kw(KW_OBJ, Obj)], None, NoneType);
        self.register_builtin_erg_impl(DEL, t_del, Immutable, vis.clone());
        let patch_t = no_var_func(
            vec![kw(KW_REQUIREMENT, Type)],
            vec![kw(KW_IMPL, Type)],
            TraitType,
        );
        let patch = ConstSubr::Builtin(BuiltinConstSubr::new(PATCH, patch_func, patch_t, None));
        self.register_builtin_const(PATCH, vis.clone(), None, ValueObj::Subr(patch));
        let t_resolve_path = nd_func(vec![kw(KW_PATH, Str)], None, Str);
        let resolve_path = ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_RESOLVE_PATH,
            resolve_path_func,
            t_resolve_path,
            None,
        ));
        self.register_builtin_const(
            FUNC_RESOLVE_PATH,
            vis.clone(),
            None,
            ValueObj::Subr(resolve_path),
        );
        let t_resolve_decl_path = nd_func(vec![kw(KW_PATH, Str)], None, Str);
        let resolve_decl_path = ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_RESOLVE_DECL_PATH,
            resolve_decl_path_func,
            t_resolve_decl_path,
            None,
        ));
        self.register_builtin_const(
            FUNC_RESOLVE_DECL_PATH,
            vis.clone(),
            None,
            ValueObj::Subr(resolve_decl_path),
        );
        let t_succ = nd_func(vec![kw(KW_N, Nat)], None, Nat);
        let succ = ConstSubr::Builtin(BuiltinConstSubr::new(FUNC_SUCC, succ_func, t_succ, None));
        self.register_builtin_const(FUNC_SUCC, vis.clone(), None, ValueObj::Subr(succ));
        let t_pred = nd_func(vec![kw(KW_N, Nat)], None, Nat);
        let pred = ConstSubr::Builtin(BuiltinConstSubr::new(FUNC_PRED, pred_func, t_pred, None));
        self.register_builtin_const(FUNC_PRED, vis.clone(), None, ValueObj::Subr(pred));
        let t_derefine = nd_func(vec![kw(KW_T, Type)], None, Type);
        let derefine = ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_DEREFINE,
            derefine_func,
            t_derefine,
            None,
        ));
        self.register_builtin_const(FUNC_DEREFINE, vis.clone(), None, ValueObj::Subr(derefine));
        let t_classof = nd_func(vec![kw(KW_OBJ, Obj)], None, ClassType);
        let classof = ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_CLASSOF,
            classof_func,
            t_classof,
            None,
        ));
        self._register_builtin_const(
            FUNC_CLASSOF,
            vis.clone(),
            None,
            ValueObj::Subr(classof),
            Some(FUNC_TYPE.into()),
        );
        let t_fill_ord = nd_func(vec![kw(KW_T, Type)], None, Type);
        let fill_ord = ConstSubr::Builtin(BuiltinConstSubr::new(
            FUNC_FILL_ORD,
            fill_ord_func,
            t_fill_ord,
            None,
        ));
        self.register_builtin_const(FUNC_FILL_ORD, vis.clone(), None, ValueObj::Subr(fill_ord));
    }

    pub(super) fn init_builtin_py_specific_funcs(&mut self) {
        let setattr_t = no_var_func(
            vec![kw(KW_OBJ, Obj), kw(KW_NAME, Str), kw(KW_VALUE, Obj)],
            vec![],
            NoneType,
        );
        self.register_builtin_py_impl(
            FUNC_SETATTR,
            setattr_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            None,
        );
        let delattr_t = no_var_func(vec![kw(KW_OBJ, Obj), kw(KW_NAME, Str)], vec![], NoneType);
        self.register_builtin_py_impl(
            FUNC_DELATTR,
            delattr_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            None,
        );
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
        self.register_builtin_py_impl(
            OP_ADD,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_ADD),
        );
        let L = mono_q(TY_L, subtypeof(poly(SUB, params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_SUB,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_SUB),
        );
        let L = mono_q(TY_L, subtypeof(poly(MUL, params.clone())));
        let L2 = type_q(TY_L);
        let R2 = mono_q(TY_R, subtypeof(poly(RMUL, vec![ty_tp(L2.clone())])));
        let op_t = (bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify()
            & bin_op(L2.clone(), R2.clone(), proj(R2, OUTPUT)).quantify())
        .with_default_intersec_index(0);
        self.register_builtin_py_impl(
            OP_MUL,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_MUL),
        );
        let L = mono_q(TY_L, subtypeof(poly(DIV, params.clone())));
        let L2 = type_q(TY_L);
        let R2 = mono_q(TY_R, subtypeof(poly(RDIV, vec![ty_tp(L2.clone())])));
        let op_t = (bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify()
            & bin_op(L2.clone(), R2.clone(), proj(R2, OUTPUT)).quantify())
        .with_default_intersec_index(0);
        self.register_builtin_py_impl(
            OP_DIV,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_TRUEDIV),
        );
        let L = mono_q(TY_L, subtypeof(poly(FLOOR_DIV, params)));
        let op_t = bin_op(L.clone(), R, proj(L, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_FLOOR_DIV,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_FLOORDIV),
        );
        let P = mono_q(TY_P, Constraint::Uninited);
        let P = mono_q(TY_P, subtypeof(poly(MUL, vec![ty_tp(P)])));
        let op_t = bin_op(P.clone(), P.clone(), proj(P, POW_OUTPUT)).quantify();
        // TODO: add bound: M == M.Output
        self.register_builtin_py_impl(
            OP_POW,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_POW),
        );
        let M = mono_q(TY_M, Constraint::Uninited);
        let M = mono_q(TY_M, subtypeof(poly(DIV, vec![ty_tp(M)])));
        let op_t = bin_op(M.clone(), M.clone(), proj(M, MOD_OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_MOD,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_MOD),
        );
        let op_t = nd_proc(vec![kw(KW_LHS, Obj), kw(KW_RHS, Obj)], None, Bool);
        self.register_builtin_py_impl(
            OP_IS,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_IS),
        );
        self.register_builtin_py_impl(
            OP_IS_NOT,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_IS_NOT),
        );
        let E = mono_q(TY_E, subtypeof(mono(EQ)));
        let E2 = mono_q(TY_E, subtypeof(mono(IRREGULAR_EQ)));
        let op_t = (bin_op(E.clone(), E, Bool).quantify()
            & bin_op(E2.clone(), E2.clone(), E2.proj(OUTPUT)).quantify())
        .with_default_intersec_index(0);
        self.register_builtin_py_impl(
            OP_EQ,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_EQ),
        );
        self.register_builtin_py_impl(
            OP_NE,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_NE),
        );
        let PO = mono_q(TY_O, subtypeof(mono(PARTIAL_ORD)));
        let op_t = bin_op(PO.clone(), PO.clone(), Bool).quantify();
        self.register_builtin_py_impl(
            OP_LT,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_LT),
        );
        self.register_builtin_py_impl(
            OP_LE,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_LE),
        );
        self.register_builtin_py_impl(
            OP_GT,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_GT),
        );
        self.register_builtin_py_impl(
            OP_GE,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_GE),
        );
        let T = type_q(TY_T);
        let U = type_q(TY_U);
        let or_t = bin_op(Bool, Bool, Bool)
            & bin_op(
                tp_enum(Type, set! { ty_tp(T.clone()) }),
                tp_enum(Type, set! { ty_tp(U.clone()) }),
                tp_enum(Type, set! { ty_tp(T.clone() | U.clone()) }),
            )
            .quantify()
            & bin_op(Type, Type, Type);
        self.register_builtin_py_impl(
            OP_OR,
            or_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_OR),
        );
        let and_t = bin_op(Bool, Bool, Bool)
            & bin_op(
                tp_enum(Type, set! { ty_tp(T.clone()) }),
                tp_enum(Type, set! { ty_tp(U.clone()) }),
                tp_enum(Type, set! { ty_tp(T & U) }),
            )
            .quantify();
        self.register_builtin_py_impl(
            OP_AND,
            and_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_AND),
        );
        let xor_t = bin_op(Bool, Bool, Bool);
        self.register_builtin_py_impl(
            OP_XOR,
            xor_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_XOR),
        );
        let shift_t = bin_op(Int, Nat, Int);
        self.register_builtin_py_impl(
            OP_LSHIFT,
            shift_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_LSHIFT),
        );
        self.register_builtin_py_impl(
            OP_RSHIFT,
            shift_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(FUNC_RSHIFT),
        );
        let O = mono_q(TY_O, subtypeof(mono(ORD)));
        let op_t = bin_op(
            O.clone(),
            O.clone(),
            range(poly(FUNC_FILL_ORD, vec![ty_tp(O)])),
        )
        .quantify();
        self.register_builtin_erg_decl(OP_RNG, op_t.clone(), Visibility::BUILTIN_PRIVATE);
        self.register_builtin_erg_decl(OP_LORNG, op_t.clone(), Visibility::BUILTIN_PRIVATE);
        self.register_builtin_erg_decl(OP_RORNG, op_t.clone(), Visibility::BUILTIN_PRIVATE);
        self.register_builtin_erg_decl(OP_ORNG, op_t, Visibility::BUILTIN_PRIVATE);
        // TODO: use existential type: |T: Type| (Container(T), T) -> Bool
        // __contains__: |T, C <: Container(T)| (C, T) -> Bool
        let T = mono_q(TY_T, instanceof(Type));
        let C = mono_q(TY_C, subtypeof(poly(CONTAINER, vec![ty_tp(T.clone())])));
        let op_t = bin_op(C, T, Bool).quantify();
        self.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
        );
        /* unary */
        // TODO: +/- Bool would like to be warned
        let M = mono_q(TY_M, subtypeof(mono(MUTIZABLE)));
        let op_t = func1(M.clone(), proj(M, MUTABLE_MUT_TYPE)).quantify();
        self.register_builtin_erg_impl(OP_MUTATE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let P = mono_q(TY_N, subtypeof(mono(POS)));
        let op_t = func1(P.clone(), proj(P, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_POS,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(OP_POS),
        );
        let N = mono_q(TY_N, subtypeof(mono(NEG)));
        let op_t = func1(N.clone(), proj(N, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_NEG,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(OP_NEG),
        );
        let invert_t = func1(Int, Int);
        self.register_builtin_py_impl(
            OP_INVERT,
            invert_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(OP_INVERT),
        );
    }

    pub(super) fn init_py_compat_builtin_operators(&mut self) {
        /* binary */
        let R = type_q(TY_R);
        let O = type_q(TY_O);
        // Erg    : |L <: Add(R), R: Type|(lhs: L, rhs: R) -> L.Output
        // Python : |L, R, O: Type|(lhs: Structural({ .__add__ = (L, R) -> O }), rhs: R) -> O
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_ADD.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_ADD, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_SUB.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_SUB, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_MUL.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_MUL, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_DIV.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_DIV, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_FLOOR_DIV.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_FLOOR_DIV, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_POW.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_POW, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_MOD.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_MOD, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = nd_proc(vec![kw(KW_LHS, Obj), kw(KW_RHS, Obj)], None, Bool);
        self.register_builtin_erg_impl(OP_IS, op_t.clone(), Const, Visibility::BUILTIN_PRIVATE);
        self.register_builtin_erg_impl(OP_IS_NOT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let E = type_q(TY_E);
        let op_t = bin_op(E.clone(), E, Bool).quantify();
        self.register_builtin_erg_impl(OP_EQ, op_t.clone(), Const, Visibility::BUILTIN_PRIVATE);
        self.register_builtin_erg_impl(OP_NE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_LT.into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        self.register_builtin_erg_impl(OP_LT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_LE.into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        self.register_builtin_erg_impl(OP_LE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_GT.into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        self.register_builtin_erg_impl(OP_GT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_GE.into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        self.register_builtin_erg_impl(OP_GE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_AND.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_AND, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_OR.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_OR, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_XOR.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_XOR, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_LSHIFT.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_LSHIFT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(OP_RSHIFT.into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O).quantify()
        };
        self.register_builtin_erg_impl(OP_RSHIFT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public(FUNDAMENTAL_CONTAINS.into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        let op_t2 = {
            let S = Type::from(
                dict! { Field::public(FUNDAMENTAL_ITER.into()) => fn0_met(Never, poly(ITERATOR, vec![ty_tp(R.clone())])) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        let op_t = (op_t & op_t2).with_default_intersec_index(0);
        self.register_builtin_erg_impl(
            FUNDAMENTAL_CONTAINS,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
        );
        /* unary */
        let M = mono_q(TY_M, subtypeof(mono(MUTIZABLE)));
        let op_t = func1(M.clone(), proj(M, MUTABLE_MUT_TYPE)).quantify();
        self.register_builtin_erg_impl(OP_MUTATE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(dict! { Field::public(OP_POS.into()) => fn0_met(Never, R.clone()) })
                .structuralize();
            func1(S, R.clone()).quantify()
        };
        self.register_builtin_erg_decl(OP_POS, op_t, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(dict! { Field::public(OP_NEG.into()) => fn0_met(Never, R.clone()) })
                .structuralize();
            func1(S, R.clone()).quantify()
        };
        self.register_builtin_erg_decl(OP_NEG, op_t, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S =
                Type::from(dict! { Field::public(OP_INVERT.into()) => fn0_met(Never, R.clone()) })
                    .structuralize();
            func1(S, R).quantify()
        };
        self.register_builtin_erg_decl(OP_INVERT, op_t, Visibility::BUILTIN_PRIVATE);
    }
}
