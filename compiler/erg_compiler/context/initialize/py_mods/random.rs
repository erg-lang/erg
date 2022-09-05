use erg_common::set;
use erg_common::vis::Visibility;

use erg_type::constructors::{
    mono, mono_q, nd_proc, param_t, poly, proc, quant, static_instance, ty_tp,
};
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_random_mod() -> Self {
        let mut random = Context::module("random".into(), 10);
        random.register_builtin_impl(
            "seed!",
            proc(
                vec![],
                None,
                vec![
                    param_t("a", mono("Num")), // TODO: NoneType, int, float, str, bytes, bytearray
                    param_t("version", Int),
                ],
                NoneType,
            ),
            Immutable,
            Public,
        );
        random.register_builtin_impl(
            "randint!",
            nd_proc(vec![param_t("a", Int), param_t("b", Int)], None, Int),
            Immutable,
            Public,
        );
        let t = nd_proc(
            vec![param_t("seq", poly("Seq", vec![ty_tp(mono_q("T"))]))],
            None,
            mono_q("T"),
        );
        let t = quant(t, set! {static_instance("T", Type)});
        random.register_builtin_impl("choice!", t, Immutable, Public);
        random
    }
}
