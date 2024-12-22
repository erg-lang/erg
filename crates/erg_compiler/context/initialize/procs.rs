#[allow(unused_imports)]
use erg_common::log;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::{Type, Visibility};
use Type::*;

use crate::context::initialize::*;
use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;

impl Context {
    pub(super) fn init_builtin_procs(&mut self) {
        let vis = if PYTHON_MODE {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::BUILTIN_PRIVATE
        };
        let t_breakpoint = proc(
            vec![],
            Some(kw(KW_ARGS, Obj)),
            vec![],
            Some(kw(KW_KWARGS, Obj)),
            NoneType,
        );
        let T = mono_q("T", instanceof(Type));
        let U = mono_q("U", instanceof(Type));
        let t_dir = no_var_proc(
            vec![],
            vec![kw(KW_OBJECT, ref_(Obj))],
            out_list_t(Str, TyParam::erased(Nat)),
        );
        let t_print = proc(
            vec![],
            Some(kw("objects", ref_(Obj))),
            vec![
                kw(KW_SEP, Str),
                kw(KW_END, Str),
                kw(KW_FILE, mono("Writable!")),
                kw(KW_FLUSH, Bool),
            ],
            None,
            NoneType,
        );
        let t_id = nd_func(vec![kw(KW_OLD, Obj)], None, Nat);
        let t_input = no_var_proc(vec![], vec![kw(KW_MSG, Str)], Str);
        let t_if = no_var_proc(
            vec![
                kw(KW_COND, Bool),
                kw(KW_THEN, nd_proc(vec![], None, T.clone())),
            ],
            vec![kw_default(
                KW_ELSE,
                nd_proc(vec![], None, U.clone()),
                nd_proc(vec![], None, NoneType),
            )],
            or(T.clone(), U.clone()),
        )
        .quantify();
        let t_proc_ret = if PYTHON_MODE { Obj } else { NoneType };
        let t_for = proc(
            vec![
                kw(KW_ITERABLE, poly("Iterable", vec![ty_tp(T.clone())])),
                kw(
                    KW_PROC,
                    nd_proc(vec![anon(T.clone())], None, t_proc_ret.clone()),
                ),
            ],
            None,
            vec![kw("else!", nd_proc(vec![], None, t_proc_ret.clone()))],
            None,
            NoneType,
        )
        .quantify();
        let t_globals = no_var_proc(vec![], vec![], out_dict_t(dict! { Str => Obj }.into()));
        let t_help = nd_proc(vec![kw(KW_OBJECT, ref_(Obj))], None, NoneType);
        let t_locals = no_var_proc(vec![], vec![], out_dict_t(dict! { Str => Obj }.into()));
        let t_next = proc(
            vec![kw(
                KW_ITERABLE,
                ref_mut(poly("Iterable", vec![ty_tp(T.clone())]), None),
            )],
            None,
            vec![kw(KW_DEFAULT, T.clone())],
            None,
            T.clone(),
        )
        .quantify();
        let t_cond = if PYTHON_MODE {
            Bool
        } else {
            // not Bool! type because `cond` may be the result of evaluation of a mutable object's method returns Bool.
            nd_proc(vec![], None, Bool)
        };
        let t_while = proc(
            vec![
                kw("cond!", t_cond),
                kw(KW_PROC, nd_proc(vec![], None, t_proc_ret.clone())),
            ],
            None,
            vec![kw("else!", nd_proc(vec![], None, t_proc_ret.clone()))],
            None,
            NoneType,
        );
        let P = mono_q("P", subtypeof(mono("PathLike")));
        let t_open = no_var_proc(
            vec![kw(KW_FILE, P)],
            vec![
                kw(KW_MODE, Str),
                kw("buffering", Int),
                kw(KW_ENCODING, or(Str, NoneType)),
                kw(KW_ERRORS, or(Str, NoneType)),
                kw("newline", or(Str, NoneType)),
                kw("closefd", Bool),
                // param_t("opener", option),
            ],
            mono("File!"),
        )
        .quantify();
        let C = if PYTHON_MODE {
            mono("ContextManager").structuralize()
        } else {
            mono("ContextManager")
        };
        let t_with = if PYTHON_MODE {
            nd_proc(
                vec![
                    kw(KW_OBJ, C),
                    kw(KW_PROC, nd_proc(vec![anon(T)], None, Obj.clone())),
                ],
                None,
                NoneType,
            )
            .quantify()
        } else {
            nd_proc(
                vec![
                    kw(KW_OBJ, C),
                    kw(KW_PROC, nd_proc(vec![anon(T)], None, U.clone())),
                ],
                None,
                U,
            )
            .quantify()
        };
        self.register_builtin_py_impl(
            "breakpoint!",
            t_breakpoint,
            Immutable,
            vis.clone(),
            Some("breakpoint"),
        );
        self.register_builtin_py_impl("dir!", t_dir, Immutable, vis.clone(), Some("dir"));
        self.register_py_builtin("print!", t_print, Some("print"), 81);
        self.register_builtin_py_impl("id!", t_id, Immutable, vis.clone(), Some("id"));
        self.register_builtin_py_impl("input!", t_input, Immutable, vis.clone(), Some("input"));
        self.register_builtin_py_impl(
            "globals!",
            t_globals,
            Immutable,
            vis.clone(),
            Some("globals"),
        );
        self.register_builtin_py_impl("help!", t_help, Immutable, vis.clone(), Some("help"));
        self.register_builtin_py_impl("locals!", t_locals, Immutable, vis.clone(), Some("locals"));
        self.register_builtin_py_impl("next!", t_next, Immutable, vis.clone(), Some("next"));
        self.register_py_builtin("open!", t_open, Some("open"), 198);
        let name = if PYTHON_MODE { "if" } else { "if__" };
        self.register_builtin_py_impl("if!", t_if, Immutable, vis.clone(), Some(name));
        let name = if PYTHON_MODE { "for" } else { "for__" };
        self.register_builtin_py_impl("for!", t_for, Immutable, vis.clone(), Some(name));
        let name = if PYTHON_MODE { "while" } else { "while__" };
        self.register_builtin_py_impl("while!", t_while, Immutable, vis.clone(), Some(name));
        let name = if PYTHON_MODE { "with" } else { "with__" };
        self.register_builtin_py_impl("with!", t_with, Immutable, vis, Some(name));
    }
}
