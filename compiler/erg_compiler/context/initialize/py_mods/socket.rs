use erg_common::vis::Visibility;
use erg_common::Str;

use erg_type::constructors::{class, func, option, param_t};
use erg_type::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_socket_mod() -> Self {
        let mut socket = Context::module("socket".into(), 15);
        let mut sock = Context::mono_class(Str::ever("Socket!"), vec![Obj], vec![], 0);
        sock.register_impl(
            "new",
            func(
                vec![],
                None,
                vec![
                    param_t("family", Int),
                    param_t("type", Int),
                    param_t("proto", Int),
                    param_t("fileno", option(Int)),
                ],
                class("Socket!"),
            ),
            Immutable,
            Public,
        );
        socket.register_type(class("Socket!"), sock, Const);
        socket
    }
}
