use erg_common::vis::Visibility;

use crate::ty::constructors::{func, kw};
use crate::ty::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_re_mod() -> Self {
        let mut re = Context::builtin_module("re", 10);
        re.register_builtin_impl(
            "sub",
            func(
                vec![kw("pattern", Str), kw("repl", Str), kw("string", Str)],
                None,
                vec![kw("count", Nat)],
                Str,
            ),
            Immutable,
            Public,
        );
        re
    }
}
