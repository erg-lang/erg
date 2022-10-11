use erg_common::vis::Visibility;

use crate::ty::constructors::{array, array_mut, func0, func1, mono, proc1};
use crate::ty::typaram::TyParam;
use crate::ty::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_sys_mod() -> Self {
        let mut sys = Context::builtin_module("sys", 15);
        sys.register_builtin_impl("argv", array(Str, TyParam::erased(Nat)), Immutable, Public);
        sys.register_builtin_impl("byteorder", Str, Immutable, Public);
        sys.register_builtin_impl(
            "builtin_module_names",
            array(Str, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        sys.register_builtin_impl("copyright", Str, Immutable, Public);
        sys.register_builtin_impl("executable", Str, Immutable, Public);
        sys.register_builtin_impl("exit", func1(Int, Never), Immutable, Public);
        sys.register_builtin_impl("getdefaultencoding", func0(Str), Immutable, Public);
        sys.register_builtin_impl(
            "path",
            array_mut(Str, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        sys.register_builtin_impl("platform", Str, Immutable, Public);
        sys.register_builtin_impl("prefix", Str, Immutable, Public);
        sys.register_builtin_impl("ps1", mono("Str!"), Immutable, Public);
        sys.register_builtin_impl("ps2", mono("Str!"), Immutable, Public);
        sys.register_builtin_impl(
            "setrecursionlimit!",
            proc1(Int, NoneType),
            Immutable,
            Public,
        );
        sys.register_builtin_impl("stderr", mono("TextIOWrapper!"), Immutable, Public);
        sys.register_builtin_impl("stdin", mono("TextIOWrapper!"), Immutable, Public);
        sys.register_builtin_impl("stdout", mono("TextIOWrapper!"), Immutable, Public);
        sys.register_builtin_impl("version", Str, Immutable, Public);
        sys
    }
}
