use std::fs::File;
use std::io::Write;

use erg_common::config::ErgConfig;
use erg_common::traits::{ExitStatus, Runnable, Stream};
use erg_parser::ast::{
    Accessor, Args, BinOp, Block, Call, Def, Expr, ExprKind, GetExprKind, Lambda, Literal, Module,
    Params, Signature, SubrSignature, UnaryOp, VarPattern, VarSignature,
};
use erg_parser::error::{ParserRunnerError, ParserRunnerErrors};
use erg_parser::token::TokenKind;
use erg_parser::ParserRunner;

use crate::transform::ASTTransformer;

#[derive(Debug, Default, Clone)]
pub struct Formatter {
    cfg: ErgConfig,
    level: usize,
    column: usize,
    outer_precedence: usize,
    previous_expr: ExprKind,
    result: String,
}

impl Runnable for Formatter {
    type Err = ParserRunnerError;
    type Errs = ParserRunnerErrors;
    const NAME: &'static str = "Erg formatter";

    #[inline]
    fn new(cfg: ErgConfig) -> Self {
        Self::new(cfg)
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        &mut self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

    #[inline]
    fn initialize(&mut self) {}

    #[inline]
    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let code = self.cfg.input.read();
        let mut parser = ParserRunner::new(self.cfg.copy());
        let art = parser.parse(code).map_err(ParserRunnerErrors::from)?;
        let code = self.format(art.ast);
        let mut f = File::options()
            .write(true)
            .create(true)
            .open(self.cfg.dump_path())
            .unwrap();
        f.set_len(0).unwrap();
        f.write_all(code.as_bytes()).unwrap();
        Ok(ExitStatus::OK)
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let mut parser = ParserRunner::new(self.cfg.copy());
        let art = parser.parse(src).map_err(ParserRunnerErrors::from)?;
        let artifact = self.format(art.ast);
        Ok(artifact)
    }
}

impl Formatter {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            level: 0,
            column: 0,
            outer_precedence: 0,
            previous_expr: ExprKind::Expr,
            result: String::new(),
        }
    }

    /// Transform and emit
    pub fn format(&mut self, ast: Module) -> String {
        let transformer = ASTTransformer::new(ast);
        let ast = transformer.transform();
        self.emit(ast)
    }

    fn emit(&mut self, ast: Module) -> String {
        for chunk in ast.into_iter() {
            let kind = chunk.detailed_expr_kind();
            self.emit_expr(chunk);
            self.push("\n");
            self.column = 0;
            self.previous_expr = kind;
        }
        std::mem::take(&mut self.result)
    }

    #[inline]
    fn push(&mut self, s: &str) {
        self.result.push_str(s);
        self.column += s.chars().count();
    }

    fn emit_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Literal(lit) => self.emit_literal(lit),
            Expr::Accessor(acc) => self.emit_acc(acc),
            Expr::Call(call) => self.emit_call(call),
            Expr::UnaryOp(unary) => self.emit_unary(unary),
            Expr::BinOp(bin) => self.emit_binop(bin),
            Expr::Def(def) => self.emit_def(def),
            Expr::Lambda(lambda) => self.emit_lambda(lambda),
            other => todo!("{other}"),
        }
    }

    fn emit_literal(&mut self, lit: Literal) {
        let s = lit.to_string();
        self.column += s.chars().count();
        self.result.push_str(&s);
    }

    fn emit_acc(&mut self, acc: Accessor) {
        match acc {
            Accessor::Ident(ident) => {
                if ident.vis.is_public() {
                    self.push(".");
                }
                self.push(ident.inspect());
            }
            Accessor::Attr(attr) => {
                let outer_precedence = self.outer_precedence;
                self.outer_precedence = TokenKind::Dot.precedence().unwrap();
                self.emit_expr(*attr.obj);
                self.push(&attr.ident.to_string());
                self.outer_precedence = outer_precedence;
            }
            other => todo!("{other}"),
        }
    }

    fn emit_args(&mut self, args: Args) {
        if args.paren.is_some() {
            self.push("(");
        } else {
            self.push(" ");
        }
        let (pos, var, kw, paren) = args.deconstruct();
        let has_pos = !pos.is_empty();
        let has_var = var.is_some();
        for (i, pos) in pos.into_iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            self.emit_expr(pos.expr);
        }
        if let Some(var) = var {
            if has_pos {
                self.push(", ");
            }
            self.push("*");
            self.emit_expr(var.expr);
        }
        for (i, kw) in kw.into_iter().enumerate() {
            if i > 0 || has_pos || has_var {
                self.push(", ");
            }
            self.push(&kw.keyword.content);
            self.push(":=");
            self.emit_expr(kw.expr);
        }
        if paren.is_some() {
            self.push(")");
        }
    }

    fn emit_call(&mut self, call: Call) {
        if let Some(attr) = call.attr_name {
            self.outer_precedence = TokenKind::Dot.precedence().unwrap();
            self.emit_expr(*call.obj);
            self.push(&attr.to_string());
        } else {
            self.emit_expr(*call.obj);
        }
        self.emit_args(call.args);
    }

    fn emit_unary(&mut self, unary: UnaryOp) {
        let outer_precedence = self.outer_precedence;
        self.outer_precedence = unary.op.kind.precedence().unwrap();
        self.push(&unary.op.content);
        self.emit_expr(*unary.args.into_iter().next().unwrap());
        self.outer_precedence = outer_precedence;
    }

    fn emit_binop(&mut self, bin: BinOp) {
        if self.outer_precedence > bin.op.kind.precedence().unwrap() {
            self.push("(");
        }
        let outer_precedence = self.outer_precedence;
        self.outer_precedence = bin.op.kind.precedence().unwrap();
        let mut args = bin.args.into_iter();
        self.emit_expr(*args.next().unwrap());
        self.push(" ");
        self.push(&bin.op.content);
        self.push(" ");
        self.emit_expr(*args.next().unwrap());
        self.outer_precedence = outer_precedence;
        if self.outer_precedence > bin.op.kind.precedence().unwrap() {
            self.push(")");
        }
    }

    fn emit_params(&mut self, params: Params) {
        if params.parens.is_some() {
            self.push("(");
        } else {
            self.push(" ");
        }
        let has_non_defaults = !params.non_defaults.is_empty();
        for (i, nd) in params.non_defaults.into_iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            self.push(&nd.to_string());
        }
        let has_var_params = params.var_params.is_some();
        if let Some(rest) = params.var_params {
            if has_non_defaults {
                self.push(", ");
            }
            self.push(&rest.to_string());
        }
        for (i, default) in params.defaults.into_iter().enumerate() {
            if i > 0 || has_non_defaults || has_var_params {
                self.push(", ");
            }
            self.push(&default.to_string());
        }
        if params.parens.is_some() {
            self.push(")");
        }
    }

    fn emit_var_pat(&mut self, pat: VarPattern) {
        match pat {
            VarPattern::Ident(ident) => {
                if ident.vis.is_public() {
                    self.push(".");
                }
                self.push(ident.inspect());
            }
            VarPattern::Discard(_) => self.push("_"),
            VarPattern::Array(array) => {
                self.push("[");
                for (i, sig) in array.elems.into_iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_var_sig(sig);
                }
                self.push("]");
            }
            VarPattern::Tuple(tuple) => {
                if tuple.paren.is_some() {
                    self.push("(");
                } else {
                    self.push(" ");
                }
                for (i, sig) in tuple.elems.into_iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_var_sig(sig);
                }
                if tuple.paren.is_some() {
                    self.push(")");
                }
            }
            VarPattern::Record(record) => {
                self.push("{");
                for (i, attr) in record.attrs.into_iter().enumerate() {
                    if i > 0 {
                        self.push("; ");
                    }
                    if attr.lhs.vis.is_public() {
                        self.push(".");
                    }
                    self.push(attr.lhs.inspect());
                    self.push(" = ");
                    self.emit_var_sig(attr.rhs);
                }
                self.push("}");
            }
            other => todo!("{other}"),
        }
    }

    fn emit_var_sig(&mut self, var: VarSignature) {
        self.emit_var_pat(var.pat);
        if let Some(t_spec) = var.t_spec {
            self.push(": ");
            self.push(&t_spec.t_spec.to_string());
        }
    }

    fn emit_subr_sig(&mut self, subr: SubrSignature) {
        for deco in subr.decorators {
            self.result.push('@');
            let s = deco.0.to_string();
            self.push(&s);
            self.push("\n");
            self.column = 0;
        }
        if subr.ident.vis.is_public() {
            self.push(".");
        }
        self.push(subr.ident.inspect());
        self.emit_params(subr.params);
    }

    fn emit_signature(&mut self, sig: Signature) {
        match sig {
            Signature::Var(var) => self.emit_var_sig(var),
            Signature::Subr(subr) => self.emit_subr_sig(subr),
        }
    }

    fn emit_block(&mut self, block: Block) {
        let lev = self.level;
        if block.len() == 1 {
            self.push(" ");
        } else {
            self.push("\n");
            self.column = 0;
            self.level += 1;
        }
        for chunk in block.into_iter() {
            self.push("    ".repeat(self.level).as_str());
            self.emit_expr(chunk);
            self.push("\n");
            self.column = 0;
        }
        self.level = lev;
    }

    fn emit_lambda(&mut self, lambda: Lambda) {
        self.emit_params(lambda.sig.params);
        self.push(" ");
        self.push(lambda.op.inspect());
        self.emit_block(lambda.body);
    }

    fn emit_def(&mut self, def: Def) {
        if def.sig.is_subr() && self.previous_expr != ExprKind::SubrDef {
            self.push("\n");
            self.column = 0;
        }
        self.emit_signature(def.sig);
        self.push(" =");
        self.emit_block(def.body.block);
    }
}
