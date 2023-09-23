use erg_common::consts::{DEBUG_MODE, ERG_MODE, PYTHON_MODE};
#[allow(unused_imports)]
use erg_common::log;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{Field, Type, Visibility};
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
        let t_array = func(
            vec![],
            None,
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            array_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let t_assert = func(
            vec![kw(KW_TEST, Bool)],
            None,
            vec![kw(KW_MSG, Str)],
            NoneType,
        );
        let t_bin = nd_func(vec![kw(KW_N, Int)], None, Str);
        // TODO: overload: Iterable(Int) -> Bytes
        let t_bytes = func(
            vec![],
            None,
            vec![kw(KW_STR, Str), kw(KW_ENCODING, Str)],
            mono(BYTES),
        );
        let t_bytes_array = func(
            vec![],
            None,
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(Int)]))],
            mono(BYTEARRAY),
        );
        let t_chr = nd_func(
            vec![kw(KW_I, Type::from(value(0usize)..=value(1_114_111usize)))],
            None,
            Str,
        );
        let F = mono_q(TY_F, instanceof(mono(GENERIC_CALLABLE)));
        let t_classmethod = nd_func(vec![kw(KW_FUNC, F.clone())], None, F.clone()).quantify();
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
        let t_dict = func(
            vec![],
            None,
            vec![kw(
                KW_ITERABLE,
                poly(ITERABLE, vec![ty_tp(tuple_t(vec![T.clone(), U.clone()]))]),
            )],
            dict! { T.clone() => U.clone() }.into(),
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
        let t_filter = nd_func(
            vec![
                kw(KW_FUNC, nd_func(vec![anon(T.clone())], None, Bool)),
                kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())])),
            ],
            None,
            poly(FILTER, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let t_frozenset = nd_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            None,
            poly(FROZENSET, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let t_hash = func1(mono(HASH), Int);
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
        let t_int = func(vec![kw(KW_OBJ, Obj)], None, vec![kw(KW_BASE, Nat)], Int);
        let t_import = nd_func(
            vec![anon(tp_enum(Str, set! {Path.clone()}))],
            None,
            module(Path.clone()),
        )
        .quantify();
        let t_isinstance = nd_func(
            vec![
                kw(KW_OBJECT, Obj),
                kw(KW_CLASSINFO, ClassType | unknown_len_array_t(ClassType)), // TODO: => ClassInfo
            ],
            None,
            Bool,
        );
        let t_issubclass = nd_func(
            vec![
                kw(KW_SUBCLASS, ClassType),
                kw(KW_CLASSINFO, ClassType | unknown_len_array_t(ClassType)), // TODO: => ClassInfo
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
            let S = Type::from(dict! { Field::public("__len__".into()) => fn0_met(Never, Nat) })
                .structuralize();
            func1(S, Nat)
        };
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
                kw(KW_FUNC, nd_func(vec![anon(T.clone())], None, U.clone())),
                kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())])),
            ],
            None,
            poly(MAP, vec![ty_tp(U.clone())]),
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
        let t_memoryview = nd_func(
            vec![kw(
                KW_OBJ,
                mono(BYTES) | mono(BYTEARRAY) | mono("array.Array!"),
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
        let t_nat = nd_func(vec![kw(KW_OBJ, Obj)], None, Nat);
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
            vec![kw(KW_SEQ, poly(SEQUENCE, vec![ty_tp(T.clone())]))],
            None,
            poly(REVERSED, vec![ty_tp(T.clone())]),
        )
        .quantify();
        let t_round = nd_func(vec![kw(KW_NUMBER, Float)], None, Int);
        let t_set = func(
            vec![],
            None,
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            set_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let t_slice = func(
            vec![kw(KW_START, Int)],
            None,
            vec![kw(KW_STOP, Int), kw(KW_STEP, Int)],
            mono(SLICE),
        );
        let t_sorted = nd_func(
            vec![kw(KW_ITERABLE, poly(ITERABLE, vec![ty_tp(T.clone())]))],
            None,
            array_t(T.clone(), TyParam::erased(Nat)),
        )
        .quantify();
        let t_staticmethod = nd_func(vec![kw(KW_FUNC, F.clone())], None, F.clone()).quantify();
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
        let t_unreachable = d_func(vec![kw(KW_MSG, Obj)], Never);
        let t_zip = nd_func(
            vec![
                kw(KW_ITERABLE1, poly(ITERABLE, vec![ty_tp(T.clone())])),
                kw(KW_ITERABLE2, poly(ITERABLE, vec![ty_tp(U.clone())])),
            ],
            None,
            poly(ZIP, vec![ty_tp(T.clone()), ty_tp(U.clone())]),
        )
        .quantify();
        self.register_py_builtin(FUNC_ABS, t_abs, Some(FUNC_ABS), 11);
        self.register_py_builtin(FUNC_ALL, t_all, Some(FUNC_ALL), 22);
        self.register_py_builtin(FUNC_ANY, t_any, Some(FUNC_ANY), 33);
        self.register_py_builtin(FUNC_ARRAY, t_array, Some(FUNC_LIST), 215);
        self.register_py_builtin(FUNC_ASCII, t_ascii, Some(FUNC_ASCII), 53);
        // Leave as `Const`, as it may negatively affect assert casting.
        self.register_builtin_erg_impl(FUNC_ASSERT, t_assert, Const, vis.clone());
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
        self.register_builtin_py_impl(FUNC_CHR, t_chr, Immutable, vis.clone(), Some(FUNC_CHR));
        self.register_builtin_py_impl(
            FUNC_CLASSMETHOD,
            t_classmethod,
            Immutable,
            vis.clone(),
            Some(FUNC_CLASSMETHOD),
        );
        self.register_builtin_py_impl(
            FUNC_CLASSOF,
            t_classof,
            Immutable,
            vis.clone(),
            Some(FUNC_TYPE),
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
            FUNC_ENUMERATE,
            t_enumerate,
            Immutable,
            vis.clone(),
            Some(FUNC_ENUMERATE),
        );
        self.register_builtin_py_impl(FUNC_EXIT, t_exit, Immutable, vis.clone(), Some(FUNC_EXIT));
        self.register_builtin_py_impl(
            FUNC_FILTER,
            t_filter,
            Immutable,
            vis.clone(),
            Some(FUNC_FILTER),
        );
        self.register_builtin_py_impl(FUNC_FROZENSET, t_frozenset, Immutable, vis.clone(), None);
        self.register_builtin_py_impl(FUNC_HASH, t_hash, Immutable, vis.clone(), Some(FUNC_HASH));
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
        self.register_builtin_py_impl(FUNC_LEN, t_len, Immutable, vis.clone(), Some(FUNC_LEN));
        self.register_builtin_py_impl(FUNC_MAP, t_map, Immutable, vis.clone(), Some(FUNC_MAP));
        self.register_builtin_py_impl(FUNC_MAX, t_max, Immutable, vis.clone(), Some(FUNC_MAX));
        self.register_builtin_py_impl(
            FUNC_MEMORYVIEW,
            t_memoryview,
            Immutable,
            vis.clone(),
            Some(FUNC_MEMORYVIEW),
        );
        self.register_builtin_py_impl(FUNC_MIN, t_min, Immutable, vis.clone(), Some(FUNC_MIN));
        self.register_builtin_py_impl(FUNC_NOT, t_not, Immutable, vis.clone(), None); // `not` is not a function in Python
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
        self.register_builtin_py_impl(
            FUNC_REVERSED,
            t_reversed,
            Immutable,
            vis.clone(),
            Some(FUNC_REVERSED),
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
        self.register_builtin_py_impl(FUNC_STR, t_str, Immutable, vis.clone(), Some(FUNC_STR__));
        self.register_builtin_py_impl(FUNC_SUM, t_sum, Immutable, vis.clone(), Some(FUNC_SUM));
        self.register_builtin_py_impl(FUNC_ZIP, t_zip, Immutable, vis.clone(), Some(FUNC_ZIP));
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
        let class_t = func(
            vec![],
            None,
            vec![kw(KW_REQUIREMENT, or(Type, Ellipsis)), kw(KW_IMPL, Type)],
            ClassType,
        );
        let class = ConstSubr::Builtin(BuiltinConstSubr::new(CLASS, class_func, class_t, None));
        self.register_builtin_const(CLASS, vis.clone(), ValueObj::Subr(class));
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
        self.register_builtin_const(INHERIT, vis.clone(), ValueObj::Subr(inherit));
        let trait_t = func(
            vec![kw(KW_REQUIREMENT, Type)],
            None,
            vec![kw(KW_IMPL, Type)],
            TraitType,
        );
        let trait_ = ConstSubr::Builtin(BuiltinConstSubr::new(TRAIT, trait_func, trait_t, None));
        self.register_builtin_const(TRAIT, vis.clone(), ValueObj::Subr(trait_));
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
        self.register_builtin_const(SUBSUME, vis.clone(), ValueObj::Subr(subsume));
        let structural = ConstSubr::Builtin(BuiltinConstSubr::new(
            STRUCTURAL,
            structural_func,
            func1(Type, Type),
            None,
        ));
        self.register_builtin_const(STRUCTURAL, vis.clone(), ValueObj::Subr(structural));
        // decorators
        let inheritable_t = func1(ClassType, ClassType);
        let inheritable = ConstSubr::Builtin(BuiltinConstSubr::new(
            INHERITABLE,
            inheritable_func,
            inheritable_t,
            None,
        ));
        self.register_builtin_const(INHERITABLE, vis.clone(), ValueObj::Subr(inheritable));
        let F = mono_q(TY_F, instanceof(mono(GENERIC_CALLABLE)));
        let override_t = func1(F.clone(), F).quantify();
        let override_ = ConstSubr::Builtin(BuiltinConstSubr::new(
            OVERRIDE,
            override_func,
            override_t,
            None,
        ));
        self.register_builtin_const(OVERRIDE, vis.clone(), ValueObj::Subr(override_));
        // TODO: register Del function object
        let t_del = nd_func(vec![kw(KW_OBJ, Obj)], None, NoneType);
        self.register_builtin_erg_impl(DEL, t_del, Immutable, vis.clone());
        let patch_t = func(
            vec![kw(KW_REQUIREMENT, Type)],
            None,
            vec![kw(KW_IMPL, Type)],
            TraitType,
        );
        let patch = ConstSubr::Builtin(BuiltinConstSubr::new(PATCH, patch_func, patch_t, None));
        self.register_builtin_const(PATCH, vis, ValueObj::Subr(patch));
    }

    pub(super) fn init_builtin_py_specific_funcs(&mut self) {
        let hasattr_t = func(vec![kw(KW_OBJ, Obj), kw(KW_NAME, Str)], None, vec![], Bool);
        self.register_builtin_py_impl(
            FUNC_HASATTR,
            hasattr_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            None,
        );
        let T = type_q("T");
        let getattr_t = func(
            vec![kw(KW_OBJ, Obj), kw(KW_NAME, Str)],
            None,
            vec![kw_default(KW_DEFAULT, T.clone(), Obj)],
            T,
        )
        .quantify();
        self.register_builtin_py_impl(
            FUNC_GETATTR,
            getattr_t,
            Immutable,
            Visibility::BUILTIN_PUBLIC,
            None,
        );
        let setattr_t = func(
            vec![kw(KW_OBJ, Obj), kw(KW_NAME, Str), kw(KW_VALUE, Obj)],
            None,
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
        let delattr_t = func(
            vec![kw(KW_OBJ, Obj), kw(KW_NAME, Str)],
            None,
            vec![],
            NoneType,
        );
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
            Some("add"),
        );
        let L = mono_q(TY_L, subtypeof(poly(SUB, params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_SUB,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("sub"),
        );
        let L = mono_q(TY_L, subtypeof(poly(MUL, params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_MUL,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("mul"),
        );
        let L = mono_q(TY_L, subtypeof(poly(DIV, params.clone())));
        let op_t = bin_op(L.clone(), R.clone(), proj(L, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_DIV,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("truediv"),
        );
        let L = mono_q(TY_L, subtypeof(poly(FLOOR_DIV, params)));
        let op_t = bin_op(L.clone(), R, proj(L, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_FLOOR_DIV,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("floordiv"),
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
            Some("pow"),
        );
        let M = mono_q(TY_M, Constraint::Uninited);
        let M = mono_q(TY_M, subtypeof(poly(DIV, vec![ty_tp(M)])));
        let op_t = bin_op(M.clone(), M.clone(), proj(M, MOD_OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_MOD,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("mod"),
        );
        let op_t = nd_proc(vec![kw(KW_LHS, Obj), kw(KW_RHS, Obj)], None, Bool);
        self.register_builtin_py_impl(
            OP_IS,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("is_"),
        );
        self.register_builtin_py_impl(
            OP_IS_NOT,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("is_not"),
        );
        let E = mono_q(TY_E, subtypeof(mono(EQ)));
        let op_t = bin_op(E.clone(), E, Bool).quantify();
        self.register_builtin_py_impl(
            OP_EQ,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("eq"),
        );
        self.register_builtin_py_impl(OP_NE, op_t, Const, Visibility::BUILTIN_PRIVATE, Some("ne"));
        let O = mono_q(TY_O, subtypeof(mono(PARTIAL_ORD)));
        let op_t = bin_op(O.clone(), O.clone(), Bool).quantify();
        self.register_builtin_py_impl(
            OP_LT,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("lt"),
        );
        self.register_builtin_py_impl(
            OP_LE,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("le"),
        );
        self.register_builtin_py_impl(
            OP_GT,
            op_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("gt"),
        );
        self.register_builtin_py_impl(OP_GE, op_t, Const, Visibility::BUILTIN_PRIVATE, Some("ge"));
        let T = type_q(TY_T);
        let U = type_q(TY_U);
        let or_t = bin_op(Bool, Bool, Bool)
            & bin_op(
                tp_enum(Type, set! { ty_tp(T.clone()) }),
                tp_enum(Type, set! { ty_tp(U.clone()) }),
                tp_enum(Type, set! { ty_tp(T.clone() | U.clone()) }),
            )
            .quantify();
        self.register_builtin_py_impl(OP_OR, or_t, Const, Visibility::BUILTIN_PRIVATE, Some("or_"));
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
            Some("and_"),
        );
        let xor_t = bin_op(Bool, Bool, Bool);
        self.register_builtin_py_impl(
            OP_XOR,
            xor_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("xor"),
        );
        let shift_t = bin_op(Int, Nat, Int);
        self.register_builtin_py_impl(
            OP_LSHIFT,
            shift_t.clone(),
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("lshift"),
        );
        self.register_builtin_py_impl(
            OP_RSHIFT,
            shift_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("rshift"),
        );
        let op_t = bin_op(O.clone(), O.clone(), range(O)).quantify();
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
            Some("__pos__"),
        );
        let N = mono_q(TY_N, subtypeof(mono(NEG)));
        let op_t = func1(N.clone(), proj(N, OUTPUT)).quantify();
        self.register_builtin_py_impl(
            OP_NEG,
            op_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("__neg__"),
        );
        let invert_t = func1(Int, Int);
        self.register_builtin_py_impl(
            OP_INVERT,
            invert_t,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some("__invert__"),
        );
    }

    pub(super) fn init_py_compat_builtin_operators(&mut self) {
        /* binary */
        let R = type_q("R");
        let O = type_q("O");
        // Erg    : |L <: Add(R), R: Type|(lhs: L, rhs: R) -> L.Output
        // Python : |L, R, O: Type|(lhs: Structural({ .__add__ = (L, R) -> O }), rhs: R) -> O
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__add__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_ADD, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__sub__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_SUB, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__mul__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_MUL, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__div__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_DIV, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__floordiv__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_FLOOR_DIV, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__pow__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_POW, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__mod__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_MOD, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = nd_proc(vec![kw(KW_LHS, Obj), kw(KW_RHS, Obj)], None, Bool);
        self.register_builtin_erg_impl(OP_IS, op_t.clone(), Const, Visibility::BUILTIN_PRIVATE);
        self.register_builtin_erg_impl(OP_IS_NOT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let E = type_q("E");
        let op_t = bin_op(E.clone(), E, Bool).quantify();
        self.register_builtin_erg_impl(OP_EQ, op_t.clone(), Const, Visibility::BUILTIN_PRIVATE);
        self.register_builtin_erg_impl(OP_NE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__lt__".into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        self.register_builtin_erg_impl(OP_LT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__le__".into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        self.register_builtin_erg_impl(OP_LE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__gt__".into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        self.register_builtin_erg_impl(OP_GT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__ge__".into()) => fn1_met(Never, R.clone(), Bool) },
            )
            .structuralize();
            bin_op(S, R.clone(), Bool).quantify()
        };
        self.register_builtin_erg_impl(OP_GE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__and__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_AND, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__or__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_OR, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__xor__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_XOR, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__lshift__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O.clone()).quantify()
        };
        self.register_builtin_erg_impl(OP_LSHIFT, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__rshift__".into()) => fn1_met(Never, R.clone(), O.clone()) },
            )
            .structuralize();
            bin_op(S, R.clone(), O).quantify()
        };
        self.register_builtin_erg_impl(OP_RSHIFT, op_t, Const, Visibility::BUILTIN_PRIVATE);
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
        let M = mono_q(TY_M, subtypeof(mono(MUTIZABLE)));
        let op_t = func1(M.clone(), proj(M, MUTABLE_MUT_TYPE)).quantify();
        self.register_builtin_erg_impl(OP_MUTATE, op_t, Const, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S =
                Type::from(dict! { Field::public("__pos__".into()) => fn0_met(Never, R.clone()) })
                    .structuralize();
            func1(S, R.clone()).quantify()
        };
        self.register_builtin_erg_decl(OP_POS, op_t, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S =
                Type::from(dict! { Field::public("__neg__".into()) => fn0_met(Never, R.clone()) })
                    .structuralize();
            func1(S, R.clone()).quantify()
        };
        self.register_builtin_erg_decl(OP_NEG, op_t, Visibility::BUILTIN_PRIVATE);
        let op_t = {
            let S = Type::from(
                dict! { Field::public("__invert__".into()) => fn0_met(Never, R.clone()) },
            )
            .structuralize();
            func1(S, R).quantify()
        };
        self.register_builtin_erg_decl(OP_INVERT, op_t, Visibility::BUILTIN_PRIVATE);
    }
}
