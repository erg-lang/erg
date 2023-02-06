//! Perform basic linting on source code.
//! What is implemented here affects subsequent optimizations,
//! and `erg_linter` does linting that does not affect optimizations.

use erg_common::error::Location;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::traits::{Locational, Runnable, Stream};
use erg_common::Str;

use crate::ty::{HasType, Type};

use crate::error::{LowerError, LowerResult, LowerWarning, LowerWarnings, SingleLowerResult};
use crate::hir;
use crate::lower::ASTLowerer;
use crate::varinfo::VarInfo;

impl ASTLowerer {
    pub(crate) fn var_result_t_check(
        &self,
        loc: Location,
        name: &Str,
        expect: &Type,
        found: &Type,
    ) -> SingleLowerResult<()> {
        self.module
            .context
            .sub_unify(found, expect, loc, Some(name))
            .map_err(|_| {
                LowerError::type_mismatch_error(
                    self.cfg().input.clone(),
                    line!() as usize,
                    loc,
                    self.module.context.caused_by(),
                    name,
                    None,
                    expect,
                    found,
                    None, // self.ctx.get_candidates(found),
                    self.module
                        .context
                        .get_simple_type_mismatch_hint(expect, found),
                )
            })
    }

    /// OK: exec `i: Int`
    /// OK: exec `i: Int = 1`
    /// NG: exec `1 + 2`
    /// OK: exec `None`
    pub(crate) fn use_check(&self, expr: &hir::Expr, mode: &str) -> LowerResult<()> {
        if mode != "eval"
            && !expr.ref_t().is_nonelike()
            && !expr.is_type_asc()
            && !expr.is_doc_comment()
        {
            Err(LowerWarnings::from(LowerWarning::unused_expr_warning(
                self.cfg().input.clone(),
                line!() as usize,
                expr,
                String::from(&self.module.context.name[..]),
            )))
        } else {
            self.block_use_check(expr, mode)
        }
    }

    fn block_use_check(&self, expr: &hir::Expr, mode: &str) -> LowerResult<()> {
        let mut warns = LowerWarnings::empty();
        match expr {
            hir::Expr::Def(def) => {
                let last = def.body.block.len() - 1;
                for (i, chunk) in def.body.block.iter().enumerate() {
                    if i == last {
                        if let Err(ws) = self.block_use_check(chunk, mode) {
                            warns.extend(ws);
                        }
                        break;
                    }
                    if let Err(ws) = self.use_check(chunk, mode) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::Lambda(lambda) => {
                let last = lambda.body.len() - 1;
                for (i, chunk) in lambda.body.iter().enumerate() {
                    if i == last {
                        if let Err(ws) = self.block_use_check(chunk, mode) {
                            warns.extend(ws);
                        }
                        break;
                    }
                    if let Err(ws) = self.use_check(chunk, mode) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::ClassDef(class_def) => {
                for chunk in class_def.methods.iter() {
                    if let Err(ws) = self.use_check(chunk, mode) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::PatchDef(patch_def) => {
                for chunk in patch_def.methods.iter() {
                    if let Err(ws) = self.use_check(chunk, mode) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::Call(call) => {
                for arg in call.args.pos_args.iter() {
                    if let Err(ws) = self.block_use_check(&arg.expr, mode) {
                        warns.extend(ws);
                    }
                }
                if let Some(var_args) = &call.args.var_args {
                    if let Err(ws) = self.block_use_check(&var_args.expr, mode) {
                        warns.extend(ws);
                    }
                }
                for arg in call.args.kw_args.iter() {
                    if let Err(ws) = self.block_use_check(&arg.expr, mode) {
                        warns.extend(ws);
                    }
                }
            }
            // TODO: unary, binary, array, ...
            _ => {}
        }
        if warns.is_empty() {
            Ok(())
        } else {
            Err(warns)
        }
    }

    pub(crate) fn inc_ref<L: Locational>(&self, vi: &VarInfo, name: &L) {
        self.module.context.inc_ref(vi, name);
    }

    pub(crate) fn warn_unused_vars(&mut self) {
        if let Some(shared) = self.module.context.shared() {
            for (referee, value) in shared.index.iter() {
                if value.referrers.is_empty() && value.vi.vis.is_private() {
                    let warn = LowerWarning::unused_warning(
                        self.input().clone(),
                        line!() as usize,
                        referee.loc,
                        referee.code().as_ref().map(|s| &s[..]).unwrap_or(""),
                        self.module.context.caused_by(),
                    );
                    self.warns.push(warn);
                }
            }
        }
    }
}
