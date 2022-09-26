use erg_common::vis::Visibility;
use erg_common::Str;

use erg_type::constructors::{builtin_mono, pr0_met, ref_};
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_io_mod() -> Self {
        let mut io = Context::module("io".into(), None, None, 15);
        let mut string_io = Context::mono_class(Str::ever("StringIO!"), None, None, 0);
        // FIXME: include Obj (pass main_ctx as a param)
        // string_io.register_superclass(Obj, obj);
        string_io.register_builtin_impl(
            "getvalue!",
            pr0_met(ref_(builtin_mono("StringIO!")), Str),
            Immutable,
            Public,
        );
        io.register_builtin_type(builtin_mono("StringIO!"), string_io, Const);
        io
    }
}
