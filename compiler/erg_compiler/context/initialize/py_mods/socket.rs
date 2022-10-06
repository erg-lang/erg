use erg_common::vis::Visibility;

use erg_type::constructors::{builtin_mono, func, kw, or};
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_socket_mod() -> Self {
        let mut socket = Context::builtin_module("socket", 15);
        let mut sock = Context::builtin_mono_class("Socket!", 0);
        // FIXME: include Obj (pass main_ctx as a param)
        // sock.register_superclass(Obj, obj);
        sock.register_builtin_impl(
            "new",
            func(
                vec![],
                None,
                vec![
                    kw("family", Int),
                    kw("type", Int),
                    kw("proto", Int),
                    kw("fileno", or(Int, NoneType)),
                ],
                builtin_mono("Socket!"),
            ),
            Immutable,
            Public,
        );
        socket.register_builtin_type(builtin_mono("Socket!"), sock, Const);
        socket
    }
}
