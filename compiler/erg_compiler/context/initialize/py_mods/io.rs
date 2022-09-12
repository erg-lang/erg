use erg_common::vis::Visibility;
use erg_common::Str;

use erg_type::constructors::{mono, pr0_met, ref_};
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_io_mod() -> Self {
        let mut io = Context::module("io".into(), 15);
        let mut string_io = Context::mono_class(Str::ever("StringIO!"), 0);
        // FIXME: include Obj (pass main_ctx as a param)
        // string_io.register_superclass(Obj, obj);
        string_io.register_builtin_impl(
            "getvalue!",
            pr0_met(ref_(mono("StringIO!")), Str),
            Immutable,
            Public,
        );
        io.register_builtin_type(mono("StringIO!"), string_io, Const);
        io
    }
}
