use erg_common::vis::Visibility;

use crate::ty::constructors::func1;
use crate::ty::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_math_mod() -> Self {
        let mut math = Context::builtin_module("math", 10);
        math.register_builtin_impl("pi", Float, Immutable, Public);
        math.register_builtin_impl("tau", Float, Immutable, Public);
        math.register_builtin_impl("e", Float, Immutable, Public);
        math.register_builtin_impl("sin", func1(Float, Float), Immutable, Public);
        math.register_builtin_impl("cos", func1(Float, Float), Immutable, Public);
        math.register_builtin_impl("tan", func1(Float, Float), Immutable, Public);
        math
    }
}
