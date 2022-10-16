use erg_common::vis::Visibility;

use crate::ty::constructors::{array_t, mono, proc, proc0, nd_proc1, kw};
use crate::ty::typaram::TyParam;
use crate::ty::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_os_mod() -> Self {
        let mut os = Context::builtin_module("os", 15);
        os.register_builtin_impl("chdir!", nd_proc1(kw("path", mono("PathLike")), NoneType), Immutable, Public);
        os.register_builtin_impl("getcwd!", proc0(Str), Immutable, Public);
        os.register_builtin_impl("getenv!", nd_proc1(kw("key", Str), Str), Immutable, Public);
        os.register_builtin_impl("listdir!", proc(vec![], None, vec![kw("path", Str)], array_t(Str, TyParam::erased(Nat))), Immutable, Public);
        os.register_builtin_impl("mkdir!", nd_proc1(kw("path", mono("PathLike")), NoneType), Immutable, Public);
        os.register_builtin_impl("name", Str, Immutable, Public);
        os.register_builtin_impl("putenv!", proc(vec![kw("key", Str), kw("value", Str)], None, vec![], NoneType), Immutable, Public);
        os.register_builtin_impl("remove!", nd_proc1(kw("path", mono("PathLike")), NoneType), Immutable, Public);
        os.register_builtin_impl("removedirs!", nd_proc1(kw("name", mono("PathLike")), NoneType), Immutable, Public);
        os.register_builtin_impl("rename!", proc(vec![kw("src", mono("PathLike")), kw("dst", mono("PathLike"))], None, vec![], NoneType), Immutable, Public);
        os.register_builtin_impl("rmdir!", nd_proc1(kw("path", mono("PathLike")), NoneType), Immutable, Public);
        if cfg!(unix) {
            os.register_builtin_impl("uname!", proc0(mono("posix.UnameResult")), Immutable, Public);
        }
        // TODO
        os.register_builtin_impl("path", mono("GenericModule"), Immutable, Public);
        os
    }
}
