use erg_common::vis::Visibility;

use erg_type::constructors::{array, array_mut, class, func0, func1, proc1};
use erg_type::typaram::TyParam;
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_sys_mod() -> Self {
        let mut sys = Context::module("sys".into(), 15);
        sys.register_impl("argv", array(Str, TyParam::erased(Nat)), Immutable, Public);
        sys.register_impl("byteorder", Str, Immutable, Public);
        sys.register_impl(
            "builtin_module_names",
            array(Str, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        sys.register_impl("copyright", Str, Immutable, Public);
        sys.register_impl("executable", Str, Immutable, Public);
        sys.register_impl("exit", func1(Int, Never), Immutable, Public);
        sys.register_impl("getdefaultencoding", func0(Str), Immutable, Public);
        sys.register_impl(
            "path",
            array_mut(Str, TyParam::erased(Nat)),
            Immutable,
            Public,
        );
        sys.register_impl("platform", Str, Immutable, Public);
        sys.register_impl("prefix", Str, Immutable, Public);
        sys.register_impl("ps1", class("Str!"), Immutable, Public);
        sys.register_impl("ps2", class("Str!"), Immutable, Public);
        sys.register_impl(
            "setrecursionlimit!",
            proc1(Int, NoneType),
            Immutable,
            Public,
        );
        sys.register_impl("stderr", class("TextIOWrapper!"), Immutable, Public);
        sys.register_impl("stdin", class("TextIOWrapper!"), Immutable, Public);
        sys.register_impl("stdout", class("TextIOWrapper!"), Immutable, Public);
        sys.register_impl("version", Str, Immutable, Public);
        sys
    }
}
