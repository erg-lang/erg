use erg_common::vis::Visibility;
use erg_common::Str;

use erg_type::constructors::{mono, pr0_met};
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_io_mod() -> Self {
        let mut io = Context::module("io".into(), 15);
        let mut string_io = Context::mono_class(Str::ever("StringIO!"), vec![Obj], vec![], 0);
        string_io.register_impl(
            "getvalue!",
            pr0_met(mono("StringIO!"), None, Str),
            Immutable,
            Public,
        );
        io.register_type(mono("StringIO!"), string_io, Const);
        io
    }
}
