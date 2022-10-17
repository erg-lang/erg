use erg_common::vis::Visibility;

use crate::ty::constructors::{proc0, proc1};
use crate::ty::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_time_mod() -> Self {
        let mut time = Context::builtin_module("time", 15);
        time.register_builtin_py_impl(
            "sleep!",
            proc1(Float, NoneType),
            Immutable,
            Public,
            Some("sleep"),
        );
        time.register_builtin_py_impl("time!", proc0(Float), Immutable, Public, Some("time"));
        time
    }
}
