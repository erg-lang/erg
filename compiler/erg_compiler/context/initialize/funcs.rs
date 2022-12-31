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

    pub(super) fn init_builtin_const_funcs(&mut self) {
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

    pub(super) fn init_builtin_operators(&mut self) {
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
}
