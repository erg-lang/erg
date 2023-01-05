#[allow(unused_imports)]
use erg_common::log;
use erg_common::vis::Visibility;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::Type;
use Type::*;

use crate::context::initialize::*;
use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(super) fn init_builtin_procs(&mut self) {
        let vis = if cfg!(feature = "py_compatible") {
            Public
        } else {
            Private
        };
        let T = mono_q("T", instanceof(Type));
        let U = mono_q("U", instanceof(Type));
        let t_dir = proc(
            vec![kw("obj", ref_(Obj))],
            None,
            vec![],
            array_t(Str, TyParam::erased(Nat)),
        );
        let t_print = proc(
            vec![],
            Some(kw("objects", ref_(Obj))),
            vec![
                kw("sep", Str),
                kw("end", Str),
                kw("file", mono("Write")),
                kw("flush", Bool),
            ],
            NoneType,
        );
        let t_id = nd_func(vec![kw("old", Obj)], None, Nat);
        let t_input = proc(vec![], None, vec![kw("msg", Str)], Str);
        let t_if = proc(
            vec![
                kw("cond", Bool),
                kw("then", nd_proc(vec![], None, T.clone())),
            ],
            None,
            vec![kw_default(
                "else",
                nd_proc(vec![], None, U.clone()),
                nd_proc(vec![], None, NoneType),
            )],
            or(T.clone(), U.clone()),
        )
        .quantify();
        let t_for = nd_proc(
            vec![
                kw("iterable", poly("Iterable", vec![ty_tp(T.clone())])),
                kw("proc!", nd_proc(vec![anon(T.clone())], None, NoneType)),
            ],
            None,
            NoneType,
        )
        .quantify();
        let t_globals = proc(vec![], None, vec![], dict! { Str => Obj }.into());
        let t_locals = proc(vec![], None, vec![], dict! { Str => Obj }.into());
        let t_next = nd_proc(
            vec![kw(
                "iterable",
                ref_mut(poly("Iterable", vec![ty_tp(T.clone())]), None),
            )],
            None,
            T.clone(),
        )
        .quantify();
        let t_cond = if cfg!(feature = "py_compatible") {
            Bool
        } else {
            // not Bool! type because `cond` may be the result of evaluation of a mutable object's method returns Bool.
            nd_proc(vec![], None, Bool)
        };
        let t_while = nd_proc(
            vec![
                kw("cond!", t_cond),
                kw("proc!", nd_proc(vec![], None, NoneType)),
            ],
            None,
            NoneType,
        );
        let P = mono_q("P", subtypeof(mono("PathLike")));
        let t_open = proc(
            vec![kw("file", P)],
            None,
            vec![
                kw("mode", Str),
                kw("buffering", Int),
                kw("encoding", or(Str, NoneType)),
                kw("errors", or(Str, NoneType)),
                kw("newline", or(Str, NoneType)),
                kw("closefd", Bool),
                // param_t("opener", option),
            ],
            mono("File!"),
        )
        .quantify();
        // TODO: T <: With
        let t_with = nd_proc(
            vec![
                kw("obj", T.clone()),
                kw("proc!", nd_proc(vec![anon(T)], None, U.clone())),
            ],
            None,
            U,
        )
        .quantify();
        self.register_builtin_py_impl("dir!", t_dir, Immutable, vis, Some("dir"));
        self.register_builtin_py_impl("print!", t_print, Immutable, vis, Some("print"));
        self.register_builtin_py_impl("id!", t_id, Immutable, vis, Some("id"));
        self.register_builtin_py_impl("input!", t_input, Immutable, vis, Some("input"));
        self.register_builtin_py_impl("globals!", t_globals, Immutable, vis, Some("globals"));
        self.register_builtin_py_impl("locals!", t_locals, Immutable, vis, Some("locals"));
        self.register_builtin_py_impl("next!", t_next, Immutable, vis, Some("next"));
        self.register_builtin_py_impl("open!", t_open, Immutable, vis, Some("open"));
        let name = if cfg!(feature = "py_compatible") {
            "if"
        } else {
            "if__"
        };
        self.register_builtin_py_impl("if!", t_if, Immutable, vis, Some(name));
        let name = if cfg!(feature = "py_compatible") {
            "for"
        } else {
            "for__"
        };
        self.register_builtin_py_impl("for!", t_for, Immutable, vis, Some(name));
        let name = if cfg!(feature = "py_compatible") {
            "while"
        } else {
            "while__"
        };
        self.register_builtin_py_impl("while!", t_while, Immutable, vis, Some(name));
        let name = if cfg!(feature = "py_compatible") {
            "with"
        } else {
            "with__"
        };
        self.register_builtin_py_impl("with!", t_with, Immutable, vis, Some(name));
    }
}
