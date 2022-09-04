//! implements SideEffectChecker
//! SideEffectCheckerを実装
//! 関数や不変型に副作用がないかチェックする

use erg_common::log;
use erg_common::traits::Stream;
use erg_common::vis::Visibility;
use erg_common::Str;
use Visibility::*;

use crate::error::{EffectError, EffectErrors, EffectResult};
use crate::hir::{Accessor, Def, Expr, Signature, HIR};

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

#[derive(Debug)]
pub struct SideEffectChecker {
    path_stack: Vec<(Str, Visibility)>,
    block_stack: Vec<BlockKind>,
    errs: EffectErrors,
}

impl SideEffectChecker {
    pub fn new() -> Self {
        Self {
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
    fn in_context_se_allowed(&self) -> bool {
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

    pub fn check(mut self, hir: HIR) -> EffectResult<HIR> {
        self.path_stack.push((hir.name.clone(), Private));
        self.block_stack.push(Module);
        log!(info "the side-effects checking process has started.{RESET}");
        // トップレベルでは副作用があっても問題なく、純粋性違反がないかのみチェックする
        for expr in hir.module.iter() {
            match expr {
                Expr::Def(def) => {
                    self.check_def(def);
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
                other => todo!("{other}"),
            }
        }
        log!(info "the side-effects checking process has completed, found errors: {}{RESET}", self.errs.len());
        if self.errs.is_empty() {
            Ok(hir)
        } else {
            Err(self.errs)
        }
    }

    fn check_def(&mut self, def: &Def) {
        let name_and_vis = match &def.sig {
            Signature::Var(var) => (var.inspect().clone(), var.vis()),
            Signature::Subr(subr) => (subr.ident.inspect().clone(), subr.ident.vis()),
        };
        self.path_stack.push(name_and_vis);
        // TODO: support raw identifier (``)
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
        for chunk in def.body.block.iter() {
            self.check_expr(chunk);
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
            Expr::Def(def) => {
                self.check_def(def);
            }
            // 引数がproceduralでも関数呼び出しなら副作用なし
            Expr::Call(call) => {
                if (self.is_procedural(&call.obj)
                    || call
                        .method_name
                        .as_ref()
                        .map(|name| name.is_procedural())
                        .unwrap_or(false))
                    && !self.in_context_se_allowed()
                {
                    self.errs.push(EffectError::has_effect(
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

impl Default for SideEffectChecker {
    fn default() -> Self {
        Self::new()
    }
}
