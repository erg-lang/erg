use erg_common::vis::Visibility;

use crate::ty::constructors::{array_t, kw, proc};
use crate::ty::typaram::TyParam;
use crate::ty::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_glob_mod() -> Self {
        let mut glob = Context::builtin_module("glob", 10);
        glob.register_builtin_py_impl(
            "glob!",
            proc(
                vec![kw("pathname", Str)],
                None,
                vec![kw("recursive", Bool)],
                array_t(Str, TyParam::erased(Nat)),
            ),
            Immutable,
            Public,
            Some("glob"),
        );
        glob
    }
}
