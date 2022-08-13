//! implements SideEffectChecker
//! SideEffectCheckerを実装
//! 関数や不変型に副作用がないかチェックする

use erg_common::color::{GREEN, RESET};
use erg_common::log;
use erg_common::traits::Stream;
use erg_common::Str;

use crate::error::{EffectError, EffectErrors, EffectResult};
use crate::hir::{Accessor, Def, Expr, Signature, HIR};
use crate::varinfo::Visibility;
use Visibility::*;

#[derive(Debug)]
pub struct SideEffectChecker {
    path_stack: Vec<(Str, Visibility)>,
    errs: EffectErrors,
}

impl SideEffectChecker {
    pub fn new() -> Self {
        Self {
            path_stack: vec![],
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

    pub fn check(mut self, hir: HIR) -> EffectResult<HIR> {
        self.path_stack.push((hir.name.clone(), Private));
        log!("{GREEN}[DEBUG] the side-effect checking process has started.{RESET}");
        // トップレベルでは副作用があっても問題なく、純粋性違反がないかのみチェックする
        for expr in hir.module.iter() {
            match expr {
                Expr::Def(def) => {
                    self.check_def(def, true);
                }
                Expr::Call(call) => {
                    for parg in call.args.pos_args().iter() {
                        self.check_expr(&parg.expr, true);
                    }
                    for kwarg in call.args.kw_args().iter() {
                        self.check_expr(&kwarg.expr, true);
                    }
                }
                Expr::BinOp(bin) => {
                    self.check_expr(&bin.lhs, true);
                    self.check_expr(&bin.rhs, true);
                }
                Expr::UnaryOp(unary) => {
                    self.check_expr(&unary.expr, true);
                }
                Expr::Accessor(_) | Expr::Lit(_) => {}
                other => todo!("{other}"),
            }
        }
        log!("{GREEN}[DEBUG] the side-effect checking process has completed, found errors: {}{RESET}", self.errs.len());
        if self.errs.is_empty() {
            Ok(hir)
        } else {
            Err(self.errs)
        }
    }

    fn check_def(&mut self, def: &Def, allow_inner_effect: bool) {
        let name_and_vis = match &def.sig {
            Signature::Var(var) =>
            // TODO: visibility
            {
                if let Some(name) = var.inspect() {
                    (name.clone(), Private)
                } else {
                    (Str::ever("::<instant>"), Private)
                }
            }
            Signature::Subr(subr) => (subr.name.inspect().clone(), Private),
        };
        self.path_stack.push(name_and_vis);
        // TODO: support raw identifier (``)
        let is_procedural = def.sig.is_procedural();
        let is_subr = def.sig.is_subr();
        let is_const = def.sig.is_const();
        let is_type = def.body.is_type();
        match (is_procedural, is_subr) {
            (true, _) => {
                if !allow_inner_effect {
                    let expr = Expr::Def(def.clone());
                    self.errs
                        .push(EffectError::has_effect(&expr, self.full_path()));
                }
                for chunk in def.body.block.iter() {
                    self.check_expr(chunk, allow_inner_effect);
                }
            }
            (false, false) => {
                for chunk in def.body.block.iter() {
                    self.check_expr(chunk, allow_inner_effect);
                }
            }
            (false, true) => {
                self.check_func(def);
            }
        }
        if is_const {
            self.check_const(def);
        }
        if !is_procedural && is_type {
            self.check_immut_type(def);
        }
        self.path_stack.pop();
    }

    fn check_func(&mut self, funcdef: &Def) {
        for chunk in funcdef.body.block.iter() {
            self.check_expr(chunk, false);
        }
    }

    fn check_immut_type(&mut self, _typedef: &Def) {
        todo!()
    }

    fn check_const(&mut self, _constdef: &Def) {
        todo!()
    }

    /// check if `expr` has side-effects / purity violations.
    ///
    /// returns effects, purity violations will be appended to `self.errs`.
    ///
    /// causes side-effects:
    /// ```
    /// p!() // 1 side-effect
    /// p!(q!()) // 2 side-effects
    /// x =
    ///     y = r!()
    ///     y + 1 // 1 side-effect
    /// ```
    /// causes no side-effects:
    /// ```
    /// q! = p!
    /// y = f(p!)
    /// ```
    /// purity violation:
    /// ```
    /// for iter, i -> print! i
    /// ```
    fn check_expr(&mut self, expr: &Expr, allow_self_effect: bool) {
        match expr {
            Expr::Def(def) => {
                self.check_def(def, allow_self_effect);
            }
            // 引数がproceduralでも関数呼び出しなら副作用なし
            Expr::Call(call) => {
                if self.is_procedural(&call.obj) && !allow_self_effect {
                    self.errs
                        .push(EffectError::has_effect(expr, self.full_path()));
                }
                call.args
                    .pos_args()
                    .iter()
                    .for_each(|parg| self.check_expr(&parg.expr, allow_self_effect));
                call.args
                    .kw_args()
                    .iter()
                    .for_each(|kwarg| self.check_expr(&kwarg.expr, allow_self_effect));
            }
            Expr::UnaryOp(unary) => {
                self.check_expr(&unary.expr, allow_self_effect);
            }
            Expr::BinOp(bin) => {
                self.check_expr(&bin.lhs, allow_self_effect);
                self.check_expr(&bin.rhs, allow_self_effect);
            }
            Expr::Lambda(lambda) => {
                let is_proc = lambda.is_procedural();
                if is_proc {
                    self.path_stack.push((Str::ever("<lambda!>"), Private));
                } else {
                    self.path_stack.push((Str::ever("<lambda>"), Private));
                }
                if !allow_self_effect && is_proc {
                    self.errs
                        .push(EffectError::has_effect(expr, self.full_path()));
                }
                lambda
                    .body
                    .iter()
                    .for_each(|chunk| self.check_expr(chunk, allow_self_effect && is_proc));
                self.path_stack.pop();
            }
            _ => {}
        }
    }

    fn is_procedural(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Lambda(lambda) => lambda.is_procedural(),
            // 引数がproceduralでも関数呼び出しなら副作用なし
            Expr::Call(call) => self.is_procedural(&call.obj),
            Expr::Accessor(Accessor::Local(local)) => local.name.is_procedural(),
            // procedural: x.y! (e.g. Array.sample!)
            // !procedural: !x.y
            Expr::Accessor(Accessor::Attr(attr)) => attr.name.is_procedural(),
            Expr::Accessor(_) => todo!(),
            _ => false,
        }
    }
}
