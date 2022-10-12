use erg_common::vis::Visibility;

use crate::ty::constructors::{mono, proc1};
use crate::ty::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_importlib_mod() -> Self {
        let mut importlib = Context::builtin_module("importlib", 15);
        importlib.register_builtin_impl(
            "reload!",
            proc1(mono("GenericModule"), NoneType),
            Immutable,
            Public,
        );
        importlib
    }
}
