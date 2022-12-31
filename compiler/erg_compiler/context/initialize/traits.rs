#[allow(unused_imports)]
use erg_common::log;
use erg_common::vis::Visibility;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::Type;
use ParamSpec as PS;
use Type::*;

use crate::context::initialize::*;
use crate::context::{ConstTemplate, Context, DefaultInfo, ParamSpec};
use crate::varinfo::Mutability;
use DefaultInfo::*;
use Mutability::*;
use Visibility::*;

impl Context {
    /// see std/prelude.er
    /// All type boundaries are defined in each subroutine
    /// `push_subtype_bound`, etc. are used for type boundary determination in user-defined APIs
    // 型境界はすべて各サブルーチンで定義する
    // push_subtype_boundなどはユーザー定義APIの型境界決定のために使用する
    pub(super) fn init_builtin_traits(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let unpack = Self::builtin_mono_trait("Unpack", 2);
        let inheritable_type = Self::builtin_mono_trait("InheritableType", 2);
        let named = Self::builtin_mono_trait("Named", 2);
        let mut mutable = Self::builtin_mono_trait("Mutable", 2);
        let Slf = mono_q("Self", subtypeof(mono("Immutizable")));
        let immut_t = proj(Slf.clone(), "ImmutType");
        let f_t = func(vec![kw("old", immut_t.clone())], None, vec![], immut_t);
        let t = pr1_met(ref_mut(Slf, None), f_t, NoneType).quantify();
        mutable.register_builtin_decl("update!", t, Public);
        // REVIEW: Immutatable?
        let mut immutizable = Self::builtin_mono_trait("Immutizable", 2);
        immutizable.register_superclass(mono("Mutable"), &mutable);
        immutizable.register_builtin_decl("ImmutType", Type, Public);
        // REVIEW: Mutatable?
        let mut mutizable = Self::builtin_mono_trait("Mutizable", 2);
        mutizable.register_builtin_decl("MutType!", Type, Public);
        let pathlike = Self::builtin_mono_trait("PathLike", 2);
        /* Readable */
        let mut readable = Self::builtin_mono_trait("Readable!", 2);
        let Slf = mono_q("Self", subtypeof(mono("Readable!")));
        let t_read = pr_met(ref_mut(Slf, None), vec![], None, vec![kw("n", Int)], Str).quantify();
        readable.register_builtin_py_decl("read!", t_read, Public, Some("read"));
        /* Writable */
        let mut writable = Self::builtin_mono_trait("Writable!", 2);
        let Slf = mono_q("Self", subtypeof(mono("Writable!")));
        let t_write = pr1_kw_met(ref_mut(Slf, None), kw("s", Str), Nat).quantify();
        writable.register_builtin_py_decl("write!", t_write, Public, Some("write"));
        // TODO: Add required methods
        let mut filelike = Self::builtin_mono_trait("FileLike", 2);
        filelike.register_superclass(mono("Readable"), &readable);
        let mut filelike_mut = Self::builtin_mono_trait("FileLike!", 2);
        filelike_mut.register_superclass(mono("FileLike"), &filelike);
        filelike_mut.register_superclass(mono("Writable!"), &writable);
        /* Show */
        let mut show = Self::builtin_mono_trait("Show", 2);
        let Slf = mono_q("Self", subtypeof(mono("Show")));
        let t_show = fn0_met(ref_(Slf), Str).quantify();
        show.register_builtin_py_decl("to_str", t_show, Public, Some("__str__"));
        /* In */
        let mut in_ = Self::builtin_poly_trait("In", vec![PS::t("T", NonDefault)], 2);
        let params = vec![PS::t("T", NonDefault)];
        let input = Self::builtin_poly_trait("Input", params.clone(), 2);
        let output = Self::builtin_poly_trait("Output", params, 2);
        let T = mono_q("T", instanceof(Type));
        let I = mono_q("I", subtypeof(poly("In", vec![ty_tp(T.clone())])));
        in_.register_superclass(poly("Input", vec![ty_tp(T.clone())]), &input);
        let op_t = fn1_met(T.clone(), I, Bool).quantify();
        in_.register_builtin_decl("__in__", op_t, Public);
        /* Eq */
        // Erg does not have a trait equivalent to `PartialEq` in Rust
        // This means, Erg's `Float` cannot be compared with other `Float`
        // use `l - r < EPSILON` to check if two floats are almost equal
        let mut eq = Self::builtin_mono_trait("Eq", 2);
        let Slf = mono_q("Self", subtypeof(mono("Eq")));
        // __eq__: |Self <: Eq| (self: Self, other: Self) -> Bool
        let op_t = fn1_met(Slf.clone(), Slf, Bool).quantify();
        eq.register_builtin_decl("__eq__", op_t, Public);
        /* Ord */
        let mut ord = Self::builtin_mono_trait("Ord", 2);
        ord.register_superclass(mono("Eq"), &eq);
        let Slf = mono_q("Self", subtypeof(mono("Ord")));
        let op_t = fn1_met(Slf.clone(), Slf, or(mono("Ordering"), NoneType)).quantify();
        ord.register_builtin_decl("__cmp__", op_t, Public);
        // FIXME: poly trait
        /* Num */
        let num = Self::builtin_mono_trait(NUM, 2);
        /* vec![
            poly("Add", vec![]),
            poly("Sub", vec![]),
            poly("Mul", vec![]),
        ], */
        /* Seq */
        let mut seq = Self::builtin_poly_trait("Seq", vec![PS::t("T", NonDefault)], 2);
        seq.register_superclass(poly("Output", vec![ty_tp(T.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Seq", vec![TyParam::erased(Type)])));
        let t = fn0_met(Slf.clone(), Nat).quantify();
        seq.register_builtin_decl("len", t, Public);
        let t = fn1_met(Slf, Nat, T.clone()).quantify();
        // Seq.get: |Self <: Seq(T)| Self.(Nat) -> T
        seq.register_builtin_decl("get", t, Public);
        /* Iterable */
        let mut iterable = Self::builtin_poly_trait("Iterable", vec![PS::t("T", NonDefault)], 2);
        iterable.register_superclass(poly("Output", vec![ty_tp(T.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Iterable", vec![ty_tp(T.clone())])));
        let t = fn0_met(Slf.clone(), proj(Slf, "Iter")).quantify();
        iterable.register_builtin_py_decl("iter", t, Public, Some("__iter__"));
        iterable.register_builtin_decl("Iter", Type, Public);
        let R = mono_q("R", instanceof(Type));
        let params = vec![PS::t("R", WithDefault)];
        let ty_params = vec![ty_tp(R.clone())];
        /* Num */
        let mut add = Self::builtin_poly_trait("Add", params.clone(), 2);
        // Covariant with `R` (independent of the type of __add__)
        add.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Add", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, "Output")).quantify();
        add.register_builtin_decl("__add__", op_t, Public);
        add.register_builtin_decl("Output", Type, Public);
        /* Sub */
        let mut sub = Self::builtin_poly_trait("Sub", params.clone(), 2);
        sub.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Sub", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, "Output")).quantify();
        sub.register_builtin_decl("__sub__", op_t, Public);
        sub.register_builtin_decl("Output", Type, Public);
        /* Mul */
        let mut mul = Self::builtin_poly_trait("Mul", params.clone(), 2);
        mul.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Mul", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, "Output")).quantify();
        mul.register_builtin_decl("__mul__", op_t, Public);
        mul.register_builtin_decl("Output", Type, Public);
        /* Div */
        let mut div = Self::builtin_poly_trait("Div", params.clone(), 2);
        div.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("Div", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, "Output")).quantify();
        div.register_builtin_decl("__div__", op_t, Public);
        div.register_builtin_decl("Output", Type, Public);
        /* FloorDiv */
        let mut floor_div = Self::builtin_poly_trait("FloorDiv", params, 2);
        floor_div.register_superclass(poly("Output", vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q("Self", subtypeof(poly("FloorDiv", ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R, proj(Slf.clone(), "Output")).quantify();
        floor_div.register_builtin_decl("__floordiv__", op_t, Public);
        floor_div.register_builtin_decl("Output", Type, Public);
        self.register_builtin_type(mono("Unpack"), unpack, vis, Const, None);
        self.register_builtin_type(
            mono("InheritableType"),
            inheritable_type,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(mono("Named"), named, vis, Const, None);
        self.register_builtin_type(mono("Mutable"), mutable, vis, Const, None);
        self.register_builtin_type(mono("Immutizable"), immutizable, vis, Const, None);
        self.register_builtin_type(mono("Mutizable"), mutizable, vis, Const, None);
        self.register_builtin_type(mono("PathLike"), pathlike, vis, Const, None);
        self.register_builtin_type(
            mono("Readable!"),
            readable,
            Private,
            Const,
            Some("Readable"),
        );
        self.register_builtin_type(
            mono("Writable!"),
            writable,
            Private,
            Const,
            Some("Writable"),
        );
        self.register_builtin_type(mono("FileLike"), filelike, vis, Const, None);
        self.register_builtin_type(mono("FileLike!"), filelike_mut, vis, Const, None);
        self.register_builtin_type(mono("Show"), show, vis, Const, None);
        self.register_builtin_type(
            poly("Input", vec![ty_tp(T.clone())]),
            input,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(
            poly("Output", vec![ty_tp(T.clone())]),
            output,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(
            poly("In", vec![ty_tp(T.clone())]),
            in_,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(mono("Eq"), eq, vis, Const, None);
        self.register_builtin_type(mono("Ord"), ord, vis, Const, None);
        self.register_builtin_type(mono(NUM), num, vis, Const, None);
        self.register_builtin_type(
            poly("Seq", vec![ty_tp(T.clone())]),
            seq,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(
            poly("Iterable", vec![ty_tp(T)]),
            iterable,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(poly("Add", ty_params.clone()), add, vis, Const, None);
        self.register_builtin_type(poly("Sub", ty_params.clone()), sub, vis, Const, None);
        self.register_builtin_type(poly("Mul", ty_params.clone()), mul, vis, Const, None);
        self.register_builtin_type(poly("Div", ty_params.clone()), div, vis, Const, None);
        self.register_builtin_type(poly("FloorDiv", ty_params), floor_div, vis, Const, None);
        self.register_const_param_defaults(
            "Add",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            "Sub",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            "Mul",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            "Div",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            "FloorDiv",
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf))],
        );
    }
}
