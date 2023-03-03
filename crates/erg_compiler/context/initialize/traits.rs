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
        let unpack = Self::builtin_mono_trait(UNPACK, 2);
        let inheritable_type = Self::builtin_mono_trait(INHERITABLE_TYPE, 2);
        let named = Self::builtin_mono_trait(NAMED, 2);
        let mut mutable = Self::builtin_mono_trait(MUTABLE, 2);
        let Slf = mono_q(SELF, subtypeof(mono(IMMUTIZABLE)));
        let immut_t = proj(Slf.clone(), IMMUT_TYPE);
        let f_t = func(vec![kw(KW_OLD, immut_t.clone())], None, vec![], immut_t);
        let t = pr1_met(ref_mut(Slf, None), f_t, NoneType).quantify();
        mutable.register_builtin_erg_decl(PROC_UPDATE, t, Public);
        // REVIEW: Immutatable?
        let mut immutizable = Self::builtin_mono_trait(IMMUTIZABLE, 2);
        immutizable.register_superclass(mono(MUTABLE), &mutable);
        immutizable.register_builtin_erg_decl(IMMUT_TYPE, Type, Public);
        // REVIEW: Mutatable?
        let mut mutizable = Self::builtin_mono_trait(MUTIZABLE, 2);
        mutizable.register_builtin_erg_decl(MUTABLE_MUT_TYPE, Type, Public);
        let pathlike = Self::builtin_mono_trait(PATH_LIKE, 2);
        /* Readable */
        let mut readable = Self::builtin_mono_trait(MUTABLE_READABLE, 2);
        let Slf = mono_q(SELF, subtypeof(mono(MUTABLE_READABLE)));
        let t_read = pr_met(ref_mut(Slf, None), vec![], None, vec![kw(KW_N, Int)], Str).quantify();
        readable.register_builtin_py_decl(PROC_READ, t_read, Public, Some(FUNC_READ));
        /* Writable */
        let mut writable = Self::builtin_mono_trait(MUTABLE_WRITABLE, 2);
        let Slf = mono_q(SELF, subtypeof(mono(MUTABLE_WRITABLE)));
        let t_write = pr1_kw_met(ref_mut(Slf, None), kw("s", Str), Nat).quantify();
        writable.register_builtin_py_decl(PROC_WRITE, t_write, Public, Some(FUNC_WRITE));
        // TODO: Add required methods
        let mut filelike = Self::builtin_mono_trait(FILE_LIKE, 2);
        filelike.register_superclass(mono(READABLE), &readable);
        let mut filelike_mut = Self::builtin_mono_trait(MUTABLE_FILE_LIKE, 2);
        filelike_mut.register_superclass(mono(FILE_LIKE), &filelike);
        filelike_mut.register_superclass(mono(MUTABLE_WRITABLE), &writable);
        /* Show */
        let mut show = Self::builtin_mono_trait(SHOW, 2);
        let Slf = mono_q(SELF, subtypeof(mono(SHOW)));
        let t_show = fn0_met(ref_(Slf), Str).quantify();
        show.register_builtin_py_decl(TO_STR, t_show, Public, Some(FUNDAMENTAL_STR));
        /* In */
        let mut in_ = Self::builtin_poly_trait(IN, vec![PS::t_nd(TY_T)], 2);
        let params = vec![PS::t_nd(TY_T)];
        let input = Self::builtin_poly_trait(INPUT, params.clone(), 2);
        let output = Self::builtin_poly_trait(OUTPUT, params, 2);
        let T = mono_q(TY_T, instanceof(Type));
        let I = mono_q(TY_I, subtypeof(poly(IN, vec![ty_tp(T.clone())])));
        in_.register_superclass(poly(INPUT, vec![ty_tp(T.clone())]), &input);
        let op_t = fn1_met(T.clone(), I, Bool).quantify();
        in_.register_builtin_erg_decl(OP_IN, op_t, Public);
        /* Eq */
        // Erg does not have a trait equivalent to `PartialEq` in Rust
        // This means, Erg's `Float` cannot be compared with other `Float`
        // use `l - r < EPSILON` to check if two floats are almost equal
        let mut eq = Self::builtin_mono_trait(EQ, 2);
        let Slf = mono_q(SELF, subtypeof(mono(EQ)));
        // __eq__: |Self <: Eq| (self: Self, other: Self) -> Bool
        let op_t = fn1_met(Slf.clone(), Slf, Bool).quantify();
        eq.register_builtin_erg_decl(OP_EQ, op_t, Public);
        /* Ord */
        let mut ord = Self::builtin_mono_trait(ORD, 2);
        ord.register_superclass(mono(EQ), &eq);
        let Slf = mono_q(SELF, subtypeof(mono(ORD)));
        let op_t = fn1_met(Slf.clone(), Slf, or(mono(ORDERING), NoneType)).quantify();
        ord.register_builtin_erg_decl(OP_CMP, op_t, Public);
        // FIXME: poly trait
        /* Num */
        let num = Self::builtin_mono_trait(NUM, 2);
        /* vec![
            poly(ADD, vec![]),
            poly(SUB, vec![]),
            poly(MUL, vec![]),
        ], */
        /* Seq */
        let mut seq = Self::builtin_poly_trait(SEQ, vec![PS::t_nd(TY_T)], 2);
        seq.register_superclass(poly(OUTPUT, vec![ty_tp(T.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(SEQ, vec![TyParam::erased(Type)])));
        let t = fn0_met(Slf.clone(), Nat).quantify();
        seq.register_builtin_erg_decl(FUNC_LEN, t, Public);
        let t = fn1_met(Slf, Nat, T.clone()).quantify();
        // Seq.get: |Self <: Seq(T)| Self.(Nat) -> T
        seq.register_builtin_erg_decl(FUNC_GET, t, Public);
        /* Iterable */
        let mut iterable = Self::builtin_poly_trait(ITERABLE, vec![PS::t_nd(TY_T)], 2);
        iterable.register_superclass(poly(OUTPUT, vec![ty_tp(T.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(ITERABLE, vec![ty_tp(T.clone())])));
        let t = fn0_met(Slf.clone(), proj(Slf, ITER)).quantify();
        iterable.register_builtin_py_decl(FUNC_ITER, t, Public, Some(FUNDAMENTAL_ITER));
        iterable.register_builtin_erg_decl(ITER, Type, Public);
        let mut context_manager = Self::builtin_mono_trait(CONTEXT_MANAGER, 2);
        let Slf = mono_q(SELF, subtypeof(mono(CONTEXT_MANAGER)));
        let t = fn0_met(Slf.clone(), NoneType).quantify();
        context_manager.register_builtin_py_decl(
            FUNDAMENTAL_ENTER,
            t,
            Public,
            Some(FUNDAMENTAL_ENTER),
        );
        let t = fn_met(
            Slf,
            vec![
                kw(EXC_TYPE, ClassType),
                kw(EXC_VALUE, Obj),
                kw(TRACEBACK, Obj), // TODO:
            ],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        context_manager.register_builtin_py_decl(
            FUNDAMENTAL_EXIT,
            t,
            Public,
            Some(FUNDAMENTAL_EXIT),
        );
        let R = mono_q(TY_R, instanceof(Type));
        let params = vec![PS::t(TY_R, false, WithDefault)];
        let ty_params = vec![ty_tp(R.clone())];
        /* Num */
        /* Add */
        let mut add = Self::builtin_poly_trait(ADD, params.clone(), 2);
        // Covariant with `R` (independent of the type of __add__)
        add.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(ADD, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, OUTPUT)).quantify();
        add.register_builtin_erg_decl(OP_ADD, op_t, Public);
        add.register_builtin_erg_decl(OUTPUT, Type, Public);
        /* Sub */
        let mut sub = Self::builtin_poly_trait(SUB, params.clone(), 2);
        sub.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(SUB, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, OUTPUT)).quantify();
        sub.register_builtin_erg_decl(OP_SUB, op_t, Public);
        sub.register_builtin_erg_decl(OUTPUT, Type, Public);
        /* Mul */
        let mut mul = Self::builtin_poly_trait(MUL, params.clone(), 2);
        mul.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(MUL, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, OUTPUT)).quantify();
        mul.register_builtin_erg_decl(OP_MUL, op_t, Public);
        mul.register_builtin_erg_decl(OUTPUT, Type, Public);
        /* Div */
        let mut div = Self::builtin_poly_trait(DIV, params.clone(), 2);
        div.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(DIV, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, OUTPUT)).quantify();
        div.register_builtin_erg_decl(OP_DIV, op_t, Public);
        div.register_builtin_erg_decl(OUTPUT, Type, Public);
        /* FloorDiv */
        let mut floor_div = Self::builtin_poly_trait(FLOOR_DIV, params, 2);
        floor_div.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(FLOOR_DIV, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R, proj(Slf.clone(), OUTPUT)).quantify();
        floor_div.register_builtin_erg_decl(OP_FLOOR_DIV, op_t, Public);
        floor_div.register_builtin_erg_decl(OUTPUT, Type, Public);
        /* Pos */
        let mut pos = Self::builtin_mono_trait(POS, 2);
        let _Slf = mono_q(SELF, subtypeof(mono(POS)));
        let op_t = fn0_met(_Slf.clone(), proj(_Slf, OUTPUT)).quantify();
        pos.register_builtin_erg_decl(OP_POS, op_t, Public);
        pos.register_builtin_erg_decl(OUTPUT, Type, Public);
        /* Neg */
        let mut neg = Self::builtin_mono_trait(NEG, 2);
        let _Slf = mono_q(SELF, subtypeof(mono(NEG)));
        let op_t = fn0_met(_Slf.clone(), proj(_Slf, OUTPUT)).quantify();
        neg.register_builtin_erg_decl(OP_NEG, op_t, Public);
        neg.register_builtin_erg_decl(OUTPUT, Type, Public);
        self.register_builtin_type(mono(UNPACK), unpack, vis, Const, None);
        self.register_builtin_type(
            mono(INHERITABLE_TYPE),
            inheritable_type,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(mono(NAMED), named, vis, Const, None);
        self.register_builtin_type(mono(MUTABLE), mutable, vis, Const, None);
        self.register_builtin_type(mono(IMMUTIZABLE), immutizable, vis, Const, None);
        self.register_builtin_type(mono(MUTIZABLE), mutizable, vis, Const, None);
        self.register_builtin_type(mono(PATH_LIKE), pathlike, vis, Const, None);
        self.register_builtin_type(
            mono(MUTABLE_READABLE),
            readable,
            Private,
            Const,
            Some(READABLE),
        );
        self.register_builtin_type(
            mono(MUTABLE_WRITABLE),
            writable,
            Private,
            Const,
            Some(WRITABLE),
        );
        self.register_builtin_type(mono(FILE_LIKE), filelike, vis, Const, None);
        self.register_builtin_type(mono(MUTABLE_FILE_LIKE), filelike_mut, vis, Const, None);
        self.register_builtin_type(mono(SHOW), show, vis, Const, None);
        self.register_builtin_type(
            poly(INPUT, vec![ty_tp(T.clone())]),
            input,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(OUTPUT, vec![ty_tp(T.clone())]),
            output,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(poly(IN, vec![ty_tp(T.clone())]), in_, Private, Const, None);
        self.register_builtin_type(mono(EQ), eq, vis, Const, None);
        self.register_builtin_type(mono(ORD), ord, vis, Const, None);
        self.register_builtin_type(mono(NUM), num, vis, Const, None);
        self.register_builtin_type(poly(SEQ, vec![ty_tp(T.clone())]), seq, Private, Const, None);
        self.register_builtin_type(
            poly(ITERABLE, vec![ty_tp(T)]),
            iterable,
            Private,
            Const,
            None,
        );
        self.register_builtin_type(mono(CONTEXT_MANAGER), context_manager, Private, Const, None);
        self.register_builtin_type(poly(ADD, ty_params.clone()), add, vis, Const, None);
        self.register_builtin_type(poly(SUB, ty_params.clone()), sub, vis, Const, None);
        self.register_builtin_type(poly(MUL, ty_params.clone()), mul, vis, Const, None);
        self.register_builtin_type(poly(DIV, ty_params.clone()), div, vis, Const, None);
        self.register_builtin_type(poly(FLOOR_DIV, ty_params), floor_div, vis, Const, None);
        self.register_builtin_type(mono(POS), pos, vis, Const, None);
        self.register_builtin_type(mono(NEG), neg, vis, Const, None);
        self.register_const_param_defaults(
            ADD,
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            SUB,
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            MUL,
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            DIV,
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf.clone()))],
        );
        self.register_const_param_defaults(
            FLOOR_DIV,
            vec![ConstTemplate::Obj(ValueObj::builtin_t(Slf))],
        );
    }
}
