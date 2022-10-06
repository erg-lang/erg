use erg_common::set;
use erg_common::vis::Visibility;

use erg_type::constructors::{
    builtin_mono, builtin_poly, kw, mono_q, nd_proc, proc, quant, static_instance, ty_tp,
};
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_random_mod() -> Self {
        let mut random = Context::builtin_module("random", 10);
        random.register_builtin_impl(
            "seed!",
            proc(
                vec![],
                None,
                vec![
                    kw("a", builtin_mono("Num")), // TODO: NoneType, int, float, str, bytes, bytearray
                    kw("version", Int),
                ],
                NoneType,
            ),
            Immutable,
            Public,
        );
        random.register_builtin_impl(
            "randint!",
            nd_proc(vec![kw("a", Int), kw("b", Int)], None, Int),
            Immutable,
            Public,
        );
        let t = nd_proc(
            vec![kw("seq", builtin_poly("Seq", vec![ty_tp(mono_q("T"))]))],
            None,
            mono_q("T"),
        );
        let t = quant(t, set! {static_instance("T", Type)});
        random.register_builtin_impl("choice!", t, Immutable, Public);
        random
    }
}
