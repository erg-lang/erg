use erg_common::vis::Visibility;

use erg_type::constructors::proc1;
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_importlib_mod() -> Self {
        let mut importlib = Context::module("importlib".into(), 15);
        importlib.register_builtin_impl("reload!", proc1(Module, NoneType), Immutable, Public);
        importlib
    }
}
