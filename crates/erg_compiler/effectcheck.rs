//! implements SideEffectChecker
//! SideEffectCheckerを実装
//! 関数や不変型に副作用がないかチェックする

use erg_common::config::ErgConfig;
use erg_common::log;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::Str;
use erg_parser::token::TokenKind;
use Visibility::*;

use crate::ty::HasType;

use crate::error::{EffectError, EffectErrors};
use crate::hir::{Array, Def, Dict, Expr, Set, Signature, Tuple, HIR};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BlockKind {
    // forbid side effects
    Func,
    ConstFunc,
    ConstInstant, // e.g. Type definition
    // allow side effects
    Proc,
    Instant,
    Module,
}

use BlockKind::*;

/// Checks code for side effects.
/// For example:
/// * check if expressions with side effects are not used in functions
/// * check if methods that change internal state are not defined in immutable classes
#[derive(Debug)]
pub struct SideEffectChecker {
    cfg: ErgConfig,
    path_stack: Vec<(Str, Visibility)>,
    block_stack: Vec<BlockKind>,
    errs: EffectErrors,
}

impl SideEffectChecker {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            path_stack: vec![],
            block_stack: vec![],
            errs: EffectErrors::empty(),
        }
    }

    fn full_path(&self) -> String {
        self.path_stack
            .iter()
            .fold(String::new(), |acc, (path, vis)| {
                if vis.is_public() {
                    acc + "." + &path[..]
                } else {
                    acc + "::" + &path[..]
                }
            })
    }

    /// It is permitted to define a procedure in a function,
    /// and of course it is permitted to cause side effects in a procedure
    ///
    /// However, it is not permitted to cause side effects within an instant block in a function
    /// (side effects are allowed in instant blocks in procedures and modules)
    fn in_context_effects_allowed(&self) -> bool {
        // if toplevel
        if self.block_stack.len() == 1 {
            return true;
        }
        match (
            self.block_stack.get(self.block_stack.len() - 2).unwrap(),
            self.block_stack.last().unwrap(),
        ) {
            (_, Func | ConstInstant) => false,
            (_, Proc) => true,
            (Proc | Module | Instant, Instant) => true,
            _ => false,
        }
    }

    pub fn check(mut self, hir: HIR) -> Result<HIR, (HIR, EffectErrors)> {
        self.path_stack.push((hir.name.clone(), Private));
        self.block_stack.push(Module);
        log!(info "the side-effects checking process has started.{RESET}");
        // At the top level, there is no problem with side effects, only check for purity violations.
        // トップレベルでは副作用があっても問題なく、純粋性違反がないかのみチェックする
        for expr in hir.module.iter() {
            match expr {
                Expr::Def(def) => {
                    self.check_def(def);
                }
                Expr::ClassDef(class_def) => {
                    if let Some(req_sup) = &class_def.require_or_sup {
                        self.check_expr(req_sup);
                    }
                    // TODO: grow
                    for def in class_def.methods.iter() {
                        self.check_expr(def);
                    }
                }
                Expr::PatchDef(patch_def) => {
                    self.check_expr(patch_def.base.as_ref());
                    // TODO: grow
                    for def in patch_def.methods.iter() {
                        self.check_expr(def);
                    }
                }
                Expr::Call(call) => {
                    for parg in call.args.pos_args.iter() {
                        self.check_expr(&parg.expr);
                    }
                    for kwarg in call.args.kw_args.iter() {
                        self.check_expr(&kwarg.expr);
                    }
                }
                Expr::BinOp(bin) => {
                    self.check_expr(&bin.lhs);
                    self.check_expr(&bin.rhs);
                }
                Expr::UnaryOp(unary) => {
                    self.check_expr(&unary.expr);
                }
                Expr::Accessor(_) | Expr::Lit(_) => {}
                Expr::Array(array) => match array {
                    Array::Normal(arr) => {
                        for elem in arr.elems.pos_args.iter() {
                            self.check_expr(&elem.expr);
                        }
                    }
                    Array::WithLength(arr) => {
                        self.check_expr(&arr.elem);
                        self.check_expr(&arr.len);
                    }
                    Array::Comprehension(arr) => {
                        self.check_expr(&arr.elem);
                        self.check_expr(&arr.guard);
                    }
                },
                Expr::Tuple(tuple) => match tuple {
                    Tuple::Normal(tuple) => {
                        for elem in tuple.elems.pos_args.iter() {
                            self.check_expr(&elem.expr);
                        }
                    }
                },
                Expr::Record(rec) => {
                    self.path_stack.push((Str::ever("<record>"), Private));
                    self.block_stack.push(Instant);
                    for attr in rec.attrs.iter() {
                        self.check_def(attr);
                    }
                    self.path_stack.pop();
                    self.block_stack.pop();
                }
                Expr::Set(set) => match set {
                    Set::Normal(set) => {
                        for elem in set.elems.pos_args.iter() {
                            self.check_expr(&elem.expr);
                        }
                    }
                    Set::WithLength(set) => {
                        self.check_expr(&set.elem);
                        self.check_expr(&set.len);
                    }
                },
                Expr::Dict(dict) => match dict {
                    Dict::Normal(dict) => {
                        for kv in dict.kvs.iter() {
                            self.check_expr(&kv.key);
                            self.check_expr(&kv.value);
                        }
                    }
                    other => todo!("{other}"),
                },
                Expr::TypeAsc(tasc) => {
                    self.check_expr(&tasc.expr);
                }
                Expr::Lambda(lambda) => {
                    let is_proc = lambda.is_procedural();
                    if is_proc {
                        self.path_stack.push((Str::ever("<lambda!>"), Private));
                        self.block_stack.push(Proc);
                    } else {
                        self.path_stack.push((Str::ever("<lambda>"), Private));
                        self.block_stack.push(Func);
                    }
                    lambda.body.iter().for_each(|chunk| self.check_expr(chunk));
                    self.path_stack.pop();
                    self.block_stack.pop();
                }
                Expr::ReDef(_)
                | Expr::Code(_)
                | Expr::Compound(_)
                | Expr::Import(_)
                | Expr::Dummy(_) => {}
            }
        }
        log!(info "the side-effects checking process has completed, found errors: {}{RESET}", self.errs.len());
        if self.errs.is_empty() {
            Ok(hir)
        } else {
            Err((hir, self.errs))
        }
    }

    fn check_def(&mut self, def: &Def) {
        let name_and_vis = match &def.sig {
            Signature::Var(var) => (var.inspect().clone(), var.vis()),
            Signature::Subr(subr) => (subr.ident.inspect().clone(), subr.ident.vis()),
        };
        self.path_stack.push(name_and_vis);
        let is_procedural = def.sig.is_procedural();
        let is_subr = def.sig.is_subr();
        let is_const = def.sig.is_const();
        match (is_procedural, is_subr, is_const) {
            (true, true, true) => {
                panic!("user-defined constant procedures are not allowed");
            }
            (true, true, false) => {
                self.block_stack.push(Proc);
            }
            (_, false, false) => {
                self.block_stack.push(Instant);
            }
            (false, true, true) => {
                self.block_stack.push(ConstFunc);
            }
            (false, true, false) => {
                self.block_stack.push(Func);
            }
            (_, false, true) => {
                self.block_stack.push(ConstInstant);
            }
        }
        if let Signature::Subr(sig) = &def.sig {
            let t = sig.ident.ref_t();
            for (nd_param, nd_type) in sig
                .params
                .non_defaults
                .iter()
                .zip(t.non_default_params().unwrap())
            {
                if nd_type.typ().is_procedure() && !nd_param.inspect().unwrap().ends_with('!') {
                    self.errs.push(EffectError::proc_assign_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        nd_param.pat.loc(),
                        self.full_path(),
                    ));
                }
            }
            if let Some((var_arg, va_type)) = sig.params.var_args.as_ref().zip(t.var_params()) {
                if va_type.typ().is_procedure() && !var_arg.inspect().unwrap().ends_with('!') {
                    self.errs.push(EffectError::proc_assign_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        var_arg.pat.loc(),
                        self.full_path(),
                    ));
                }
            }
            for (d_param, d_type) in sig.params.defaults.iter().zip(t.default_params().unwrap()) {
                if d_type.typ().is_procedure() && !d_param.inspect().unwrap().ends_with('!') {
                    self.errs.push(EffectError::proc_assign_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        d_param.sig.pat.loc(),
                        self.full_path(),
                    ));
                }
            }
        }
        let last_idx = def.body.block.len() - 1;
        for (i, chunk) in def.body.block.iter().enumerate() {
            self.check_expr(chunk);
            // e.g. `echo = print!`
            if i == last_idx
                && self.block_stack.last().unwrap() == &Instant
                && !def.sig.is_procedural()
                && chunk.t().is_procedure()
            {
                self.errs.push(EffectError::proc_assign_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    def.sig.loc(),
                    self.full_path(),
                ));
            }
        }
        self.path_stack.pop();
        self.block_stack.pop();
    }

    /// check if `expr` has side-effects / purity violations.
    ///
    /// returns effects, purity violations will be appended to `self.errs`.
    ///
    /// causes side-effects:
    /// ```python
    /// p!() // 1 side-effect
    /// p!(q!()) // 2 side-effects
    /// x =
    ///     y = r!()
    ///     y + 1 // 1 side-effect
    /// ```
    /// causes no side-effects:
    /// ```python
    /// q! = p!
    /// y = f(p!)
    /// ```
    /// purity violation:
    /// ```python
    /// for iter, i -> print! i
    /// ```
    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Lit(_) => {}
            Expr::Def(def) => {
                self.check_def(def);
            }
            Expr::ClassDef(class_def) => {
                if let Some(req_sup) = &class_def.require_or_sup {
                    self.check_expr(req_sup);
                }
                for def in class_def.methods.iter() {
                    self.check_expr(def);
                }
            }
            Expr::PatchDef(patch_def) => {
                self.check_expr(patch_def.base.as_ref());
                for def in patch_def.methods.iter() {
                    self.check_expr(def);
                }
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    for elem in arr.elems.pos_args.iter() {
                        self.check_expr(&elem.expr);
                    }
                }
                Array::WithLength(arr) => {
                    self.check_expr(&arr.elem);
                    self.check_expr(&arr.len);
                }
                Array::Comprehension(arr) => {
                    self.check_expr(&arr.elem);
                    self.check_expr(&arr.guard);
                }
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    for arg in tup.elems.pos_args.iter() {
                        self.check_expr(&arg.expr);
                    }
                }
            },
            Expr::Record(record) => {
                self.path_stack.push((Str::ever("<record>"), Private));
                self.block_stack.push(Instant);
                for attr in record.attrs.iter() {
                    self.check_def(attr);
                }
                self.path_stack.pop();
                self.block_stack.pop();
            }
            Expr::Set(set) => match set {
                Set::Normal(set) => {
                    for elem in set.elems.pos_args.iter() {
                        self.check_expr(&elem.expr);
                    }
                }
                Set::WithLength(set) => {
                    self.check_expr(&set.elem);
                    self.check_expr(&set.len);
                }
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dict) => {
                    for kv in dict.kvs.iter() {
                        self.check_expr(&kv.key);
                        self.check_expr(&kv.value);
                    }
                }
                other => todo!("{other}"),
            },
            Expr::Call(call) => {
                if (call.obj.t().is_procedure()
                    || call
                        .attr_name
                        .as_ref()
                        .map(|name| name.is_procedural())
                        .unwrap_or(false))
                    && !self.in_context_effects_allowed()
                {
                    self.errs.push(EffectError::has_effect(
                        self.cfg.input.clone(),
                        line!() as usize,
                        expr,
                        self.full_path(),
                    ));
                }
                call.args
                    .pos_args
                    .iter()
                    .for_each(|parg| self.check_expr(&parg.expr));
                call.args
                    .kw_args
                    .iter()
                    .for_each(|kwarg| self.check_expr(&kwarg.expr));
            }
            Expr::UnaryOp(unary) => {
                self.check_expr(&unary.expr);
            }
            Expr::BinOp(bin) => {
                self.check_expr(&bin.lhs);
                self.check_expr(&bin.rhs);
                if (bin.op.kind == TokenKind::IsOp || bin.op.kind == TokenKind::IsNotOp)
                    && !self.in_context_effects_allowed()
                {
                    self.errs.push(EffectError::has_effect(
                        self.cfg.input.clone(),
                        line!() as usize,
                        expr,
                        self.full_path(),
                    ));
                }
            }
            Expr::Lambda(lambda) => {
                let is_proc = lambda.is_procedural();
                if is_proc {
                    self.path_stack.push((Str::ever("<lambda!>"), Private));
                    self.block_stack.push(Proc);
                } else {
                    self.path_stack.push((Str::ever("<lambda>"), Private));
                    self.block_stack.push(Func);
                }
                lambda.body.iter().for_each(|chunk| self.check_expr(chunk));
                self.path_stack.pop();
                self.block_stack.pop();
            }
            Expr::TypeAsc(type_asc) => {
                self.check_expr(&type_asc.expr);
            }
            Expr::Accessor(acc) => {
                if !self.in_context_effects_allowed() && acc.ref_t().is_mut_type() {
                    self.errs.push(EffectError::touch_mut_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        expr,
                        self.full_path(),
                    ));
                }
            }
            Expr::ReDef(_)
            | Expr::Code(_)
            | Expr::Compound(_)
            | Expr::Import(_)
            | Expr::Dummy(_) => {}
        }
    }
}
