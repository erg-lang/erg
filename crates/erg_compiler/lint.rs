//! Perform basic linting on source code.
//! What is implemented here affects subsequent optimizations,
//! and `erg_linter` does linting that does not affect optimizations.

#[allow(unused_imports)]
use erg_common::log;
use erg_common::traits::{Locational, Runnable, Stream};
use erg_common::Str;
use erg_parser::ast::AST;
use erg_parser::build_ast::ASTBuilder;

use crate::context::ContextKind;
use crate::link_ast::ASTLinker;
use crate::ty::{HasType, Type, ValueObj, VisibilityModifier};

use crate::error::{
    CompileErrors, LowerError, LowerResult, LowerWarning, LowerWarnings, SingleLowerResult,
};
use crate::hir::{self, Expr, Signature, HIR};
use crate::lower::ASTLowerer;
use crate::varinfo::VarInfo;

impl ASTLowerer {
    pub(crate) fn var_result_t_check(
        &self,
        loc: &impl Locational,
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
                    loc.loc(),
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

    pub(crate) fn warn_unused_expr(&mut self, module: &hir::Module, mode: &str) {
        if mode == "eval" {
            return;
        }
        for chunk in module.iter() {
            if let Err(warns) = self.expr_use_check(chunk) {
                self.warns.extend(warns);
            }
        }
    }

    /// OK: exec `i: Int`
    /// OK: exec `i: Int = 1`
    /// NG: exec `1 + 2`
    /// OK: exec `None`
    fn expr_use_check(&self, expr: &hir::Expr) -> LowerResult<()> {
        if !expr.ref_t().is_nonelike() && !expr.is_type_asc() && !expr.is_doc_comment() {
            if expr.ref_t().is_subr() {
                Err(LowerWarnings::from(
                    LowerWarning::unused_subroutine_warning(
                        self.cfg().input.clone(),
                        line!() as usize,
                        expr,
                        String::from(&self.module.context.name[..]),
                    ),
                ))
            } else {
                Err(LowerWarnings::from(LowerWarning::unused_expr_warning(
                    self.cfg().input.clone(),
                    line!() as usize,
                    expr,
                    String::from(&self.module.context.name[..]),
                )))
            }
        } else {
            self.block_use_check(expr)
        }
    }

    fn block_use_check(&self, expr: &hir::Expr) -> LowerResult<()> {
        let mut warns = LowerWarnings::empty();
        match expr {
            hir::Expr::Def(def) => {
                let last = def.body.block.len() - 1;
                for (i, chunk) in def.body.block.iter().enumerate() {
                    if i == last {
                        if let Err(ws) = self.block_use_check(chunk) {
                            warns.extend(ws);
                        }
                        break;
                    }
                    if let Err(ws) = self.expr_use_check(chunk) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::Lambda(lambda) => {
                let last = lambda.body.len() - 1;
                for (i, chunk) in lambda.body.iter().enumerate() {
                    if i == last {
                        if let Err(ws) = self.block_use_check(chunk) {
                            warns.extend(ws);
                        }
                        break;
                    }
                    if let Err(ws) = self.expr_use_check(chunk) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::ClassDef(class_def) => {
                for chunk in class_def.methods.iter() {
                    if let Err(ws) = self.expr_use_check(chunk) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::PatchDef(patch_def) => {
                for chunk in patch_def.methods.iter() {
                    if let Err(ws) = self.expr_use_check(chunk) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::Call(call) => {
                for arg in call.args.pos_args.iter() {
                    if let Err(ws) = self.block_use_check(&arg.expr) {
                        warns.extend(ws);
                    }
                }
                if let Some(var_args) = &call.args.var_args {
                    if let Err(ws) = self.block_use_check(&var_args.expr) {
                        warns.extend(ws);
                    }
                }
                for arg in call.args.kw_args.iter() {
                    if let Err(ws) = self.block_use_check(&arg.expr) {
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
        self.module.context.inc_ref(vi, name, &self.module.context);
    }

    pub(crate) fn warn_unused_vars(&mut self, mode: &str) {
        if mode == "eval" {
            return;
        }
        for (referee, value) in self.module.context.index().iter() {
            let code = referee.code();
            let name = code.as_ref().map(|s| &s[..]).unwrap_or("");
            let name_is_auto = name == "_"; // || name.starts_with(['%']);
            if value.referrers.is_empty() && value.vi.vis.is_private() && !name_is_auto {
                let input = referee
                    .module
                    .as_ref()
                    .map_or(self.input().clone(), |path| path.as_path().into());
                let warn = LowerWarning::unused_warning(
                    input,
                    line!() as usize,
                    referee.loc,
                    name,
                    self.module.context.caused_by(),
                );
                self.warns.push(warn);
            }
        }
    }

    pub(crate) fn check_doc_comments(&mut self, hir: &HIR) {
        for chunk in hir.module.iter() {
            self.check_doc_comment(chunk);
        }
    }

    fn check_doc_comment(&mut self, chunk: &Expr) {
        match chunk {
            Expr::Lit(lit) if lit.is_doc_comment() => {
                let first_line = lit.ln_begin().unwrap_or(1);
                let ValueObj::Str(content) = &lit.value else { return; };
                if content.starts_with("erg\n") {
                    let code = content.trim_start_matches("erg\n");
                    let indent = code.chars().take_while(|c| c.is_whitespace()).count();
                    let code = if indent > 0 {
                        format!(
                            "{}_ =\n{code}\n{}None",
                            "\n".repeat(first_line as usize - 1),
                            " ".repeat(indent)
                        )
                    } else {
                        format!("{}{code}", "\n".repeat(first_line as usize))
                    };
                    match ASTBuilder::new(self.cfg().clone()).build(code) {
                        Ok(ast) => {
                            self.check_doc_ast(ast);
                        }
                        Err(errs) => {
                            let errs = CompileErrors::from(errs);
                            self.errs.extend(errs);
                        }
                    }
                }
            }
            Expr::ClassDef(class_def) => {
                for chunk in class_def.methods.iter() {
                    self.check_doc_comment(chunk);
                }
            }
            Expr::PatchDef(patch_def) => {
                for chunk in patch_def.methods.iter() {
                    self.check_doc_comment(chunk);
                }
            }
            _ => {}
        }
    }

    fn check_doc_ast(&mut self, ast: AST) {
        let Ok(ast) = ASTLinker::new(self.cfg().clone()).link(ast, "exec") else {
            return;
        };
        self.module.context.grow(
            "<doc>",
            ContextKind::Instant,
            VisibilityModifier::Private,
            None,
        );
        let mut module = hir::Module::with_capacity(ast.module.len());
        if let Err(errs) = self.module.context.preregister(ast.module.block()) {
            self.errs.extend(errs);
        }
        for chunk in ast.module.into_iter() {
            match self.lower_chunk(chunk) {
                Ok(chunk) => {
                    module.push(chunk);
                }
                Err(errs) => {
                    self.errs.extend(errs);
                }
            }
        }
        self.module.context.clear_invalid_vars();
        self.module.context.check_decls().unwrap_or_else(|errs| {
            self.errs.extend(errs);
        });
        let hir = HIR::new(ast.name, module);
        let hir = match self.module.context.resolve(hir) {
            Ok(hir) => hir,
            Err((_, errs)) => {
                self.errs.extend(errs);
                self.module.context.pop();
                return;
            }
        };
        self.warn_unused_expr(&hir.module, "exec");
        // self.warn_unused_vars("exec");
        self.check_doc_comments(&hir);
        self.module.context.pop();
    }

    pub(crate) fn warn_implicit_union(&mut self, hir: &HIR) {
        for chunk in hir.module.iter() {
            self.warn_implicit_union_chunk(chunk);
        }
    }

    fn warn_implicit_union_chunk(&mut self, chunk: &Expr) {
        match chunk {
            Expr::ClassDef(class_def) => {
                for chunk in class_def.methods.iter() {
                    self.warn_implicit_union_chunk(chunk);
                }
            }
            Expr::PatchDef(patch_def) => {
                for chunk in patch_def.methods.iter() {
                    self.warn_implicit_union_chunk(chunk);
                }
            }
            Expr::Def(def) => {
                if let Signature::Subr(subr) = &def.sig {
                    let return_t = subr.ref_t().return_t().unwrap();
                    if return_t.union_pair().is_some() && subr.return_t_spec.is_none() {
                        let typ = if cfg!(feature = "debug") {
                            return_t.clone()
                        } else {
                            self.module.context.readable_type(return_t.clone())
                        };
                        let warn = LowerWarning::union_return_type_warning(
                            self.input().clone(),
                            line!() as usize,
                            subr.loc(),
                            self.module.context.caused_by(),
                            subr.ident.inspect(),
                            &typ,
                        );
                        self.warns.push(warn);
                    }
                }
            }
            _ => {}
        }
    }
}
