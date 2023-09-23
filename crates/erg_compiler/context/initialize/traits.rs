#[allow(unused_imports)]
use erg_common::log;

use crate::ty::constructors::*;
use crate::ty::value::ValueObj;
use crate::ty::{Type, Visibility};
use ParamSpec as PS;
use Type::*;

use crate::context::initialize::*;
use crate::context::{ConstTemplate, Context, DefaultInfo, ParamSpec};
use crate::varinfo::Mutability;
use DefaultInfo::*;
use Mutability::*;

impl Context {
    /// see std/prelude.er
    /// All type boundaries are defined in each subroutine
    /// `push_subtype_bound`, etc. are used for type boundary determination in user-defined APIs
    // 型境界はすべて各サブルーチンで定義する
    // push_subtype_boundなどはユーザー定義APIの型境界決定のために使用する
    pub(super) fn init_builtin_traits(&mut self) {
        let vis = if PYTHON_MODE {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::BUILTIN_PRIVATE
        };
        let unpack = Self::builtin_mono_trait(UNPACK, 2);
        let inheritable_type = Self::builtin_mono_trait(INHERITABLE_TYPE, 2);
        let mut named = Self::builtin_mono_trait(NAMED, 2);
        named.register_builtin_erg_decl(FUNC_NAME, Str, Visibility::BUILTIN_PUBLIC);
        let mut sized = Self::builtin_mono_trait(SIZED, 2);
        let t = fn0_met(mono(SIZED), Nat).quantify();
        sized.register_builtin_erg_decl(FUNDAMENTAL_LEN, t, Visibility::BUILTIN_PUBLIC);
        let mut mutable = Self::builtin_mono_trait(MUTABLE, 2);
        let Slf = mono_q(SELF, subtypeof(mono(IMMUTIZABLE)));
        let immut_t = proj(Slf.clone(), IMMUT_TYPE);
        let f_t = func(vec![kw(KW_OLD, immut_t.clone())], None, vec![], immut_t);
        let t = pr1_met(ref_mut(Slf, None), f_t, NoneType).quantify();
        mutable.register_builtin_erg_decl(PROC_UPDATE, t, Visibility::BUILTIN_PUBLIC);
        // REVIEW: Immutatable?
        let mut immutizable = Self::builtin_mono_trait(IMMUTIZABLE, 2);
        immutizable.register_superclass(mono(MUTABLE), &mutable);
        immutizable.register_builtin_erg_decl(IMMUT_TYPE, Type, Visibility::BUILTIN_PUBLIC);
        // REVIEW: Mutatable?
        let mut mutizable = Self::builtin_mono_trait(MUTIZABLE, 2);
        mutizable.register_builtin_erg_decl(MUTABLE_MUT_TYPE, Type, Visibility::BUILTIN_PUBLIC);
        let pathlike = Self::builtin_mono_trait(PATH_LIKE, 2);
        /* Readable! */
        let mut readable = Self::builtin_mono_trait(MUTABLE_READABLE, 2);
        let Slf = mono(MUTABLE_READABLE);
        let t_read = pr_met(
            ref_mut(Slf.clone(), None),
            vec![],
            None,
            vec![kw(KW_N, Int)],
            Str,
        );
        readable.register_builtin_decl(
            PROC_READ,
            t_read,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_READ),
        );
        readable.register_builtin_decl(
            FUNC_READABLE,
            fn0_met(Slf.clone(), Bool),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_READABLE),
        );
        readable.register_builtin_decl(
            PROC_READLINE,
            pr0_met(ref_mut(Slf.clone(), None), Str),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_READLINE),
        );
        readable.register_builtin_decl(
            PROC_READLINES,
            pr0_met(ref_mut(Slf, None), unknown_len_array_t(Str)),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_READLINES),
        );
        /* IO! */
        let mut io = Self::builtin_mono_trait(MUTABLE_IO, 2);
        let Slf = mono(MUTABLE_IO);
        io.register_superclass(mono(MUTABLE_READABLE), &readable);
        io.register_builtin_decl(
            FUNC_MODE,
            fn0_met(Slf.clone(), Str),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_MODE),
        );
        io.register_builtin_decl(
            FUNC_NAME,
            fn0_met(Slf.clone(), Str),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_NAME),
        );
        io.register_builtin_decl(
            PROC_CLOSE,
            pr0_met(ref_mut(Slf.clone(), None), NoneType),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_CLOSE),
        );
        io.register_builtin_decl(
            FUNC_CLOSED,
            fn0_met(Slf.clone(), Bool),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_CLOSED),
        );
        io.register_builtin_decl(
            FUNC_FILENO,
            fn0_met(Slf.clone(), Nat),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_FILENO),
        );
        io.register_builtin_decl(
            PROC_FLUSH,
            pr0_met(ref_mut(Slf.clone(), None), NoneType),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_FLUSH),
        );
        io.register_builtin_decl(
            FUNC_ISATTY,
            fn0_met(Slf.clone(), Bool),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_ISATTY),
        );
        io.register_builtin_decl(
            PROC_SEEK,
            pr_met(
                ref_mut(Slf.clone(), None),
                vec![kw(KW_OFFSET, Nat)],
                None,
                vec![kw(KW_WHENCE, Nat)],
                Nat,
            ),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_SEEK),
        );
        io.register_builtin_decl(
            FUNC_SEEKABLE,
            pr0_met(ref_mut(Slf.clone(), None), Bool),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_SEEKABLE),
        );
        io.register_builtin_decl(
            FUNC_TELL,
            fn0_met(Slf, Nat),
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_TELL),
        );
        /* Writable! */
        let mut writable = Self::builtin_mono_trait(MUTABLE_WRITABLE, 2);
        let Slf = mono_q(SELF, subtypeof(mono(MUTABLE_WRITABLE)));
        let t_write = pr1_kw_met(ref_mut(Slf, None), kw("s", Str), Nat).quantify();
        writable.register_superclass(mono(MUTABLE_IO), &io);
        writable.register_builtin_decl(
            PROC_WRITE,
            t_write,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNC_WRITE),
        );
        // TODO: Add required methods
        let mut filelike = Self::builtin_mono_trait(FILE_LIKE, 2);
        filelike.register_superclass(mono(MUTABLE_READABLE), &readable);
        let mut filelike_mut = Self::builtin_mono_trait(MUTABLE_FILE_LIKE, 2);
        filelike_mut.register_superclass(mono(FILE_LIKE), &filelike);
        filelike_mut.register_superclass(mono(MUTABLE_WRITABLE), &writable);
        /* Show */
        let mut show = Self::builtin_mono_trait(SHOW, 2);
        let Slf = mono_q(SELF, subtypeof(mono(SHOW)));
        let t_str = fn0_met(ref_(Slf), Str).quantify();
        show.register_builtin_decl(
            FUNDAMENTAL_STR,
            t_str,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_STR),
        );
        /* Input/Output */
        let params = vec![PS::t_nd(TY_T)];
        let input = Self::builtin_poly_trait(INPUT, params.clone(), 2);
        let output = Self::builtin_poly_trait(OUTPUT, params, 2);
        let T = mono_q(TY_T, instanceof(Type));
        /* Eq */
        // Erg does not have a trait equivalent to `PartialEq` in Rust
        // This means, Erg's `Float` cannot be compared with other `Float`
        // use `l - r < EPSILON` to check if two floats are almost equal
        let mut eq = Self::builtin_mono_trait(EQ, 2);
        let Slf = mono_q(SELF, subtypeof(mono(EQ)));
        // __eq__: |Self <: Eq| (self: Self, other: Self) -> Bool
        let op_t = fn1_met(Slf.clone(), Slf, Bool).quantify();
        eq.register_builtin_erg_decl(OP_EQ, op_t, Visibility::BUILTIN_PUBLIC);
        /* Hash */
        let mut hash = Self::builtin_mono_trait(HASH, 2);
        let Slf = mono_q(SELF, subtypeof(mono(HASH)));
        let op_t = fn0_met(Slf.clone(), Int).quantify();
        hash.register_builtin_erg_decl(OP_HASH, op_t, Visibility::BUILTIN_PUBLIC);
        /* EqHash */
        let mut eq_hash = Self::builtin_mono_trait(EQ_HASH, 2);
        eq_hash.register_superclass(mono(HASH), &hash);
        eq_hash.register_superclass(mono(EQ), &eq);
        /* PartialOrd */
        let mut partial_ord = Self::builtin_mono_trait(PARTIAL_ORD, 2);
        let Slf = mono_q(SELF, subtypeof(mono(PARTIAL_ORD)));
        let op_t = fn1_met(Slf.clone(), Slf, or(mono(ORDERING), NoneType)).quantify();
        partial_ord.register_builtin_erg_decl(OP_CMP, op_t, Visibility::BUILTIN_PUBLIC);
        /* Ord */
        let mut ord = Self::builtin_mono_trait(ORD, 2);
        ord.register_superclass(mono(PARTIAL_ORD), &partial_ord);
        ord.register_superclass(mono(EQ), &eq);
        let Slf = mono_q(SELF, subtypeof(mono(ORD)));
        let op_t = fn1_met(Slf.clone(), Slf, or(mono(ORDERING), NoneType)).quantify();
        ord.register_builtin_erg_decl(OP_CMP, op_t, Visibility::BUILTIN_PUBLIC);
        /* Iterable */
        let mut iterable = Self::builtin_poly_trait(ITERABLE, vec![PS::t_nd(TY_T)], 2);
        iterable.register_superclass(poly(OUTPUT, vec![ty_tp(T.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(ITERABLE, vec![ty_tp(T.clone())])));
        let t = fn0_met(Slf.clone(), proj(Slf, ITER)).quantify();
        iterable.register_builtin_decl(
            FUNC_ITER,
            t,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_ITER),
        );
        iterable.register_builtin_erg_decl(ITER, Type, Visibility::BUILTIN_PUBLIC);
        /* Iterator */
        let mut iterator = Self::builtin_poly_trait(ITERATOR, vec![PS::t_nd(TY_T)], 2);
        iterator.register_superclass(poly(ITERABLE, vec![ty_tp(T.clone())]), &iterable);
        let Slf = mono_q(SELF, subtypeof(poly(ITERATOR, vec![ty_tp(T.clone())])));
        let t = fn0_met(Slf, or(T.clone(), NoneType)).quantify();
        iterator.register_builtin_erg_decl(FUNDAMENTAL_NEXT, t, Visibility::BUILTIN_PUBLIC);
        /* Container */
        let mut container = Self::builtin_poly_trait(CONTAINER, vec![PS::t_nd(TY_T)], 2);
        let op_t = fn1_met(mono(CONTAINER), T.clone(), Bool).quantify();
        container.register_superclass(poly(OUTPUT, vec![ty_tp(T.clone())]), &output);
        container.register_builtin_erg_decl(FUNDAMENTAL_CONTAINS, op_t, Visibility::BUILTIN_PUBLIC);
        /* Collection */
        let mut collection = Self::builtin_poly_trait(COLLECTION, vec![PS::t_nd(TY_T)], 2);
        collection.register_superclass(mono(SIZED), &sized);
        collection.register_superclass(poly(CONTAINER, vec![ty_tp(T.clone())]), &container);
        collection.register_superclass(poly(ITERABLE, vec![ty_tp(T.clone())]), &iterable);
        /* Indexable */
        let mut indexable =
            Self::builtin_poly_trait(INDEXABLE, vec![PS::t_nd(TY_K), PS::t_nd(TY_V)], 2);
        let K = type_q(TY_K);
        let V = type_q(TY_V);
        indexable.register_superclass(poly(INPUT, vec![ty_tp(K.clone())]), &input);
        indexable.register_superclass(poly(OUTPUT, vec![ty_tp(V.clone())]), &output);
        let Slf = mono_q(
            SELF,
            subtypeof(poly(INDEXABLE, vec![ty_tp(K.clone()), ty_tp(V.clone())])),
        );
        let t = fn1_met(Slf, K.clone(), V.clone()).quantify();
        indexable.register_builtin_erg_decl(FUNDAMENTAL_GETITEM, t, Visibility::BUILTIN_PUBLIC);
        /* Sequence */
        let mut sequence = Self::builtin_poly_trait(SEQUENCE, vec![PS::t_nd(TY_T)], 2);
        sequence.register_superclass(mono(SIZED), &sized);
        sequence.register_superclass(
            poly(INDEXABLE, vec![ty_tp(Nat), ty_tp(T.clone())]),
            &indexable,
        );
        sequence.register_superclass(poly(OUTPUT, vec![ty_tp(T.clone())]), &output);
        /* Sequence! */
        let mut mut_sequence = Self::builtin_poly_trait(MUTABLE_SEQUENCE, vec![PS::t_nd(TY_T)], 2);
        mut_sequence.register_superclass(poly(SEQUENCE, vec![ty_tp(T.clone())]), &sequence);
        let Slf = mono_q(
            SELF,
            subtypeof(poly(MUTABLE_SEQUENCE, vec![ty_tp(T.clone())])),
        );
        let t = pr_met(
            Slf,
            vec![kw("idx", Nat), kw("value", T.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        // .__setitem__!: |Self! <: Sequence!(T)| Self!.(idx: Nat, value: T) => NoneType
        mut_sequence.register_builtin_erg_decl(
            PROC_FUNDAMENTAL_SETITEM,
            t.clone(),
            Visibility::BUILTIN_PUBLIC,
        );
        mut_sequence.register_builtin_erg_decl(
            PROC_FUNDAMENTAL_DELITEM,
            t.clone(),
            Visibility::BUILTIN_PUBLIC,
        );
        mut_sequence.register_builtin_erg_decl(PROC_INSERT, t, Visibility::BUILTIN_PUBLIC);
        /* Mapping */
        let mut mapping =
            Self::builtin_poly_trait(MAPPING, vec![PS::t_nd(TY_K), PS::t_nd(TY_V)], 2);
        // mapping.register_superclass(poly(COLLECTION, vec![ty_tp(V.clone())]), &collection);
        mapping.register_superclass(
            poly(INDEXABLE, vec![ty_tp(K.clone()), ty_tp(V.clone())]),
            &indexable,
        );
        /* Mapping! */
        let mut mut_mapping =
            Self::builtin_poly_trait(MUTABLE_MAPPING, vec![PS::t_nd(TY_K), PS::t_nd(TY_V)], 2);
        mut_mapping.register_superclass(
            poly(MAPPING, vec![ty_tp(K.clone()), ty_tp(V.clone())]),
            &mapping,
        );
        let Slf = mono_q(
            SELF,
            subtypeof(poly(
                MUTABLE_SEQUENCE,
                vec![ty_tp(K.clone()), ty_tp(V.clone())],
            )),
        );
        let t = pr_met(
            Slf.clone(),
            vec![kw("key", K.clone()), kw("value", V.clone())],
            None,
            vec![],
            NoneType,
        )
        .quantify();
        // .__setitem__!: |Self! <: Mapping!(K, V)| Self!.(key: K, value: V) => NoneType
        mut_mapping.register_builtin_erg_decl(
            PROC_FUNDAMENTAL_SETITEM,
            t,
            Visibility::BUILTIN_PUBLIC,
        );
        let t = pr_met(Slf, vec![kw("key", K.clone())], None, vec![], NoneType).quantify();
        mut_mapping.register_builtin_erg_decl(
            PROC_FUNDAMENTAL_DELITEM,
            t,
            Visibility::BUILTIN_PUBLIC,
        );
        let mut context_manager = Self::builtin_mono_trait(CONTEXT_MANAGER, 2);
        let Slf = mono_q(SELF, subtypeof(mono(CONTEXT_MANAGER)));
        let t = fn0_met(Slf.clone(), NoneType).quantify();
        context_manager.register_builtin_decl(
            FUNDAMENTAL_ENTER,
            t,
            Visibility::BUILTIN_PUBLIC,
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
        context_manager.register_builtin_decl(
            FUNDAMENTAL_EXIT,
            t,
            Visibility::BUILTIN_PUBLIC,
            Some(FUNDAMENTAL_EXIT),
        );
        /* Shape */
        let S = mono_q_tp(TY_S, instanceof(unknown_len_array_t(Nat)));
        let params = vec![PS::named_nd("S", unknown_len_array_t(Nat))];
        let shape = Self::builtin_poly_trait(SHAPE, params.clone(), 2);
        /* Num */
        let R = mono_q(TY_R, instanceof(Type));
        let params = vec![PS::t(TY_R, false, WithDefault)];
        let ty_params = vec![ty_tp(R.clone())];
        /* Add */
        let mut add = Self::builtin_poly_trait(ADD, params.clone(), 2);
        // Covariant with `R` (independent of the type of __add__)
        add.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(ADD, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, OUTPUT)).quantify();
        add.register_builtin_erg_decl(OP_ADD, op_t, Visibility::BUILTIN_PUBLIC);
        add.register_builtin_erg_decl(OUTPUT, Type, Visibility::BUILTIN_PUBLIC);
        /* Sub */
        let mut sub = Self::builtin_poly_trait(SUB, params.clone(), 2);
        sub.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(SUB, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, OUTPUT)).quantify();
        sub.register_builtin_erg_decl(OP_SUB, op_t, Visibility::BUILTIN_PUBLIC);
        sub.register_builtin_erg_decl(OUTPUT, Type, Visibility::BUILTIN_PUBLIC);
        /* Mul */
        let mut mul = Self::builtin_poly_trait(MUL, params.clone(), 2);
        mul.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(MUL, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, OUTPUT)).quantify();
        mul.register_builtin_erg_decl(OP_MUL, op_t, Visibility::BUILTIN_PUBLIC);
        mul.register_builtin_erg_decl(OUTPUT, Type, Visibility::BUILTIN_PUBLIC);
        /* Div */
        let mut div = Self::builtin_poly_trait(DIV, params.clone(), 2);
        div.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(DIV, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R.clone(), proj(Slf, OUTPUT)).quantify();
        div.register_builtin_erg_decl(OP_DIV, op_t, Visibility::BUILTIN_PUBLIC);
        div.register_builtin_erg_decl(OUTPUT, Type, Visibility::BUILTIN_PUBLIC);
        /* FloorDiv */
        let mut floor_div = Self::builtin_poly_trait(FLOOR_DIV, params, 2);
        floor_div.register_superclass(poly(OUTPUT, vec![ty_tp(R.clone())]), &output);
        let Slf = mono_q(SELF, subtypeof(poly(FLOOR_DIV, ty_params.clone())));
        let op_t = fn1_met(Slf.clone(), R, proj(Slf.clone(), OUTPUT)).quantify();
        floor_div.register_builtin_erg_decl(OP_FLOOR_DIV, op_t, Visibility::BUILTIN_PUBLIC);
        floor_div.register_builtin_erg_decl(OUTPUT, Type, Visibility::BUILTIN_PUBLIC);
        /* Pos */
        let mut pos = Self::builtin_mono_trait(POS, 2);
        let _Slf = mono_q(SELF, subtypeof(mono(POS)));
        let op_t = fn0_met(_Slf.clone(), proj(_Slf, OUTPUT)).quantify();
        pos.register_builtin_erg_decl(OP_POS, op_t, Visibility::BUILTIN_PUBLIC);
        pos.register_builtin_erg_decl(OUTPUT, Type, Visibility::BUILTIN_PUBLIC);
        /* Neg */
        let mut neg = Self::builtin_mono_trait(NEG, 2);
        let _Slf = mono_q(SELF, subtypeof(mono(NEG)));
        let op_t = fn0_met(_Slf.clone(), proj(_Slf, OUTPUT)).quantify();
        neg.register_builtin_erg_decl(OP_NEG, op_t, Visibility::BUILTIN_PUBLIC);
        neg.register_builtin_erg_decl(OUTPUT, Type, Visibility::BUILTIN_PUBLIC);
        /* Num */
        let mut num = Self::builtin_mono_trait(NUM, 2);
        num.register_superclass(poly(ADD, vec![]), &add);
        num.register_superclass(poly(SUB, vec![]), &sub);
        num.register_superclass(poly(MUL, vec![]), &mul);
        self.register_builtin_type(mono(UNPACK), unpack, vis.clone(), Const, None);
        self.register_builtin_type(
            mono(INHERITABLE_TYPE),
            inheritable_type,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(mono(NAMED), named, vis.clone(), Const, None);
        self.register_builtin_type(mono(SIZED), sized, vis.clone(), Const, None);
        self.register_builtin_type(mono(MUTABLE), mutable, vis.clone(), Const, None);
        self.register_builtin_type(mono(IMMUTIZABLE), immutizable, vis.clone(), Const, None);
        self.register_builtin_type(mono(MUTIZABLE), mutizable, vis.clone(), Const, None);
        self.register_builtin_type(mono(PATH_LIKE), pathlike, vis.clone(), Const, None);
        self.register_builtin_type(
            mono(MUTABLE_READABLE),
            readable,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            mono(MUTABLE_IO),
            io,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            mono(MUTABLE_WRITABLE),
            writable,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(mono(FILE_LIKE), filelike, vis.clone(), Const, None);
        self.register_builtin_type(
            mono(MUTABLE_FILE_LIKE),
            filelike_mut,
            vis.clone(),
            Const,
            None,
        );
        self.register_builtin_type(mono(SHOW), show, vis.clone(), Const, None);
        self.register_builtin_type(
            poly(INPUT, vec![ty_tp(T.clone())]),
            input,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(OUTPUT, vec![ty_tp(T.clone())]),
            output,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(mono(EQ), eq, vis.clone(), Const, None);
        self.register_builtin_type(mono(HASH), hash, vis.clone(), Const, None);
        self.register_builtin_type(mono(EQ_HASH), eq_hash, vis.clone(), Const, None);
        self.register_builtin_type(mono(PARTIAL_ORD), partial_ord, vis.clone(), Const, None);
        self.register_builtin_type(mono(ORD), ord, vis.clone(), Const, None);
        self.register_builtin_type(mono(NUM), num, vis.clone(), Const, None);
        self.register_builtin_type(
            poly(SEQUENCE, vec![ty_tp(T.clone())]),
            sequence,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(MUTABLE_SEQUENCE, vec![ty_tp(T.clone())]),
            mut_sequence,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(ITERABLE, vec![ty_tp(T.clone())]),
            iterable,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(ITERATOR, vec![ty_tp(T.clone())]),
            iterator,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(CONTAINER, vec![ty_tp(T.clone())]),
            container,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(COLLECTION, vec![ty_tp(T)]),
            collection,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(INDEXABLE, vec![ty_tp(K.clone()), ty_tp(V.clone())]),
            indexable,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(MAPPING, vec![ty_tp(K.clone()), ty_tp(V.clone())]),
            mapping,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            poly(MUTABLE_MAPPING, vec![ty_tp(K), ty_tp(V)]),
            mut_mapping,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(
            mono(CONTEXT_MANAGER),
            context_manager,
            Visibility::BUILTIN_PRIVATE,
            Const,
            None,
        );
        self.register_builtin_type(poly(SHAPE, vec![S]), shape, vis.clone(), Const, None);
        self.register_builtin_type(poly(ADD, ty_params.clone()), add, vis.clone(), Const, None);
        self.register_builtin_type(poly(SUB, ty_params.clone()), sub, vis.clone(), Const, None);
        self.register_builtin_type(poly(MUL, ty_params.clone()), mul, vis.clone(), Const, None);
        self.register_builtin_type(poly(DIV, ty_params.clone()), div, vis.clone(), Const, None);
        self.register_builtin_type(
            poly(FLOOR_DIV, ty_params),
            floor_div,
            vis.clone(),
            Const,
            None,
        );
        self.register_builtin_type(mono(POS), pos, vis.clone(), Const, None);
        self.register_builtin_type(mono(NEG), neg, vis, Const, None);
        self.register_const_param_defaults(
            ADD,
            vec![ConstTemplate::Obj(ValueObj::builtin_type(Slf.clone()))],
        );
        self.register_const_param_defaults(
            SUB,
            vec![ConstTemplate::Obj(ValueObj::builtin_type(Slf.clone()))],
        );
        self.register_const_param_defaults(
            MUL,
            vec![ConstTemplate::Obj(ValueObj::builtin_type(Slf.clone()))],
        );
        self.register_const_param_defaults(
            DIV,
            vec![ConstTemplate::Obj(ValueObj::builtin_type(Slf.clone()))],
        );
        self.register_const_param_defaults(
            FLOOR_DIV,
            vec![ConstTemplate::Obj(ValueObj::builtin_type(Slf))],
        );
    }
}
