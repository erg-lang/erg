use std::fs::File;
use std::io::Write;

use erg_common::config::ErgConfig;
#[allow(unused)]
use erg_common::log;
use erg_common::traits::{ExitStatus, Runnable, Stream};
use erg_parser::ast::{
    Accessor, Args, Array, BinOp, Block, Call, Def, Expr, ExprKind, GetExprKind, Lambda, Literal,
    Module, NonDefaultParamSignature, NormalArray, ParamPattern, Params, Signature, SubrSignature,
    UnaryOp, VarPattern, VarSignature,
};
use erg_parser::error::{ParserRunnerError, ParserRunnerErrors};
use erg_parser::token::TokenKind;
use erg_parser::ParserRunner;

use crate::transform::ASTTransformer;

const INDENT: &str = "    ";
const MAX_LINE_LENGTH: usize = 80;

#[derive(Debug, Default, Clone)]
pub struct Formatter {
    cfg: ErgConfig,
    block_level: usize,
    expr_level: usize,
    column: usize,
    outer_precedence: usize,
    previous_expr: ExprKind,
    buffer: Vec<Vec<String>>,
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
            block_level: 0,
            expr_level: 0,
            column: 0,
            outer_precedence: 0,
            previous_expr: ExprKind::Expr,
            buffer: Vec::new(),
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

    fn new_buffer(&mut self) {
        self.buffer.push(vec!["".to_string()]);
    }

    fn current_buffer(&mut self) -> &mut Vec<String> {
        self.buffer.last_mut().unwrap()
    }

    fn pop_buffer(&mut self) -> Vec<String> {
        self.buffer.pop().unwrap()
    }

    #[inline]
    fn push(&mut self, s: &str) {
        if self.buffer.last().is_some() {
            self.current_buffer().last_mut().unwrap().push_str(s);
        } else {
            self.result.push_str(s);
        }
        self.column += s.chars().count();
    }

    fn split(&mut self) {
        self.current_buffer().push("".to_string());
    }

    /// last is lambda, no others
    /// ```erg
    /// for xs, x ->
    ///     ...
    /// ```
    fn for_style_fold(&mut self) {
        let l_enc = self.current_buffer().remove(0);
        match &l_enc[..] {
            "(" | " " => {
                self.result.push(' ');
            }
            _ => unreachable!(),
        }
        let last_lambda = self
            .current_buffer()
            .iter()
            .rfind(|&s| s != ", " && s != ")")
            .unwrap()
            .clone();
        for s in self.pop_buffer() {
            if s == last_lambda {
                let indented = s.replace("->", "->\n").replace("\n    ", "\n        ");
                self.result.push_str(&indented);
            } else if s == ")" {
                // self.result.push('\n');
                break;
            } else {
                self.result.push_str(&s);
            }
        }
    }

    /// last is lambda, others exist
    /// ```erg
    /// match x:
    ///     x -> ...
    ///     y -> ...
    /// ```
    fn match_style_fold(&mut self) {
        let l_enc = self.current_buffer().remove(0);
        match &l_enc[..] {
            "(" | " " => {
                self.result.push(' ');
            }
            _ => unreachable!(),
        }
        let fst = self.current_buffer().remove(0);
        self.result.push_str(&fst);
        self.result.push_str(":\n");
        for s in self.pop_buffer() {
            if s == ")" {
                self.result.push('\n');
                break;
            } else if s.trim().is_empty() || s == ", " {
                continue;
            } else {
                self.result.push_str(&INDENT.repeat(self.block_level + 1));
                let indented = s.replace("\n    ", "\n        ");
                self.result.push_str(&indented);
            }
            if !s.ends_with('\n') {
                self.result.push('\n');
            }
        }
    }

    fn normal_style_fold(&mut self) {
        let fst = self.current_buffer().remove(0);
        match &fst[..] {
            "(" | "[" | "{" => {
                self.result.push_str(&fst);
                self.result.push('\n');
            }
            " " => {
                self.result.push_str("(\n");
            }
            _ => unreachable!(),
        }
        for s in self.pop_buffer() {
            if s.trim().is_empty() {
                continue;
            } else if &s == ", " {
                self.result.push_str(&s);
                self.result.pop();
                self.result.push('\n');
            } else {
                self.result.push_str(&INDENT.repeat(self.block_level + 1));
                self.result.push_str(&s);
            }
        }
        self.column = 0;
        if &fst == " " {
            self.result.push('\n');
            self.push(")");
        }
    }

    fn fold(&mut self, last_is_lambda: bool, has_lambdas: bool) {
        match (last_is_lambda, has_lambdas) {
            (true, true) => self.match_style_fold(),
            (true, false) => self.for_style_fold(),
            _ => self.normal_style_fold(),
        }
    }

    fn flush(&mut self, arg_kinds: Option<Vec<ExprKind>>) {
        let last_is_lambda = arg_kinds
            .as_ref()
            .is_some_and(|kinds| kinds.last().is_some_and(|last| last == &ExprKind::Lambda));
        let has_lambdas = arg_kinds.as_ref().is_some_and(|kinds| {
            kinds
                .iter()
                .filter(|&kind| kind == &ExprKind::Lambda)
                .count()
                >= 2
        });
        if self.expr_level == 0 && self.column >= MAX_LINE_LENGTH {
            self.fold(last_is_lambda, has_lambdas);
        } else if self.expr_level == 0 && has_lambdas {
            self.match_style_fold();
        } else {
            for s in self.pop_buffer() {
                self.push(&s);
            }
        }
    }

    fn emit_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Literal(lit) => self.emit_literal(lit),
            Expr::Accessor(acc) => self.emit_acc(acc),
            Expr::Call(call) => self.emit_call(call),
            Expr::UnaryOp(unary) => self.emit_unary(unary),
            Expr::BinOp(bin) => self.emit_binop(bin),
            Expr::Array(Array::Normal(array)) => self.emit_normal_array(array),
            Expr::Lambda(lambda) => self.emit_lambda(lambda),
            Expr::Def(def) => self.emit_def(def),
            other => todo!("{other}"),
        }
    }

    fn emit_literal(&mut self, lit: Literal) {
        let s = lit.to_string();
        self.column += s.chars().count();
        self.push(&s);
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
                let lev = self.expr_level;
                self.expr_level += 1;
                let outer_precedence = self.outer_precedence;
                self.outer_precedence = TokenKind::Dot.precedence().unwrap();
                self.emit_expr(*attr.obj);
                self.push(&attr.ident.to_string());
                self.outer_precedence = outer_precedence;
                self.expr_level = lev;
            }
            other => todo!("{other}"),
        }
    }

    fn emit_args(&mut self, args: Args) {
        let lev = self.expr_level;
        self.expr_level += 1;
        self.new_buffer();
        if args.paren.is_some() {
            self.push("(");
        } else {
            self.push(" ");
        }
        self.split();
        let (pos, var, kw, paren) = args.deconstruct();
        let has_pos = !pos.is_empty();
        let has_var = var.is_some();
        let mut kinds = vec![];
        for (i, pos) in pos.into_iter().enumerate() {
            kinds.push(pos.expr.detailed_expr_kind());
            if i > 0 {
                self.push(", ");
                self.split();
            }
            self.emit_expr(pos.expr);
            self.split();
        }
        if let Some(var) = var {
            kinds.push(var.expr.detailed_expr_kind());
            if has_pos {
                self.push(", ");
                self.split();
            }
            self.push("*");
            self.emit_expr(var.expr);
            self.split();
        }
        for (i, kw) in kw.into_iter().enumerate() {
            kinds.push(kw.expr.detailed_expr_kind());
            if i > 0 || has_pos || has_var {
                self.push(", ");
                self.split();
            }
            self.push(&kw.keyword.content);
            self.push(":=");
            self.emit_expr(kw.expr);
            self.split();
        }
        if paren.is_some() {
            self.push(")");
            self.split();
        }
        self.expr_level = lev;
        self.flush(Some(kinds));
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
        let lev = self.expr_level;
        self.expr_level += 1;
        let outer_precedence = self.outer_precedence;
        self.outer_precedence = unary.op.kind.precedence().unwrap();
        self.push(&unary.op.content);
        self.emit_expr(*unary.args.into_iter().next().unwrap());
        self.outer_precedence = outer_precedence;
        self.expr_level = lev;
    }

    fn emit_binop(&mut self, bin: BinOp) {
        let lev = self.expr_level;
        self.expr_level += 1;
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
        self.expr_level = lev;
    }

    fn emit_normal_array(&mut self, array: NormalArray) {
        let lev = self.expr_level;
        self.expr_level += 1;
        self.new_buffer();
        self.push("[");
        self.split();
        let (pos, ..) = array.elems.deconstruct();
        for (i, pos) in pos.into_iter().enumerate() {
            if i > 0 {
                self.push(", ");
                self.split();
            }
            self.emit_expr(pos.expr);
            self.split();
        }
        self.expr_level = lev;
        self.push("]");
        self.split();
        self.flush(None);
    }

    fn emit_nd_param(&mut self, param: NonDefaultParamSignature) {
        self.emit_param_pat(param.pat);
        if let Some(t_spec) = param.t_spec {
            self.push(": ");
            self.push(&t_spec.t_spec.to_string());
        }
    }

    fn emit_param_pat(&mut self, pat: ParamPattern) {
        match pat {
            ParamPattern::VarName(ident) => {
                self.push(ident.inspect());
            }
            ParamPattern::Discard(_) => self.push("_"),
            ParamPattern::Array(array) => {
                self.push("[");
                self.emit_params(array.elems);
                self.push("]");
            }
            ParamPattern::Tuple(tuple) => {
                self.emit_params(tuple.elems);
            }
            ParamPattern::Record(record) => {
                self.push("{");
                for (i, attr) in record.elems.into_iter().enumerate() {
                    if i > 0 {
                        self.push("; ");
                    }
                    if attr.lhs.vis.is_public() {
                        self.push(".");
                    }
                    self.push(attr.lhs.inspect());
                    self.push(" = ");
                    self.emit_nd_param(attr.rhs);
                }
                self.push("}");
            }
            other => todo!("{other}"),
        }
    }

    fn emit_params(&mut self, params: Params) {
        if params.parens.is_some() {
            self.push("(");
        } else {
            // self.push(" ");
        }
        let has_non_defaults = !params.non_defaults.is_empty();
        for (i, nd) in params.non_defaults.into_iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            self.emit_nd_param(nd);
        }
        let has_var_params = params.var_params.is_some();
        if let Some(rest) = params.var_params {
            if has_non_defaults {
                self.push(", ");
            }
            self.emit_nd_param(*rest);
        }
        for (i, default) in params.defaults.into_iter().enumerate() {
            if i > 0 || has_non_defaults || has_var_params {
                self.push(", ");
            }
            self.emit_nd_param(default.sig);
            self.push(" := ");
            self.emit_expr(default.default_val);
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
            self.push("@");
            let s = deco.0.to_string();
            self.push(&s);
            self.push("\n");
            self.column = 0;
        }
        if subr.ident.vis.is_public() {
            self.push(".");
        }
        self.push(subr.ident.inspect());
        if subr.params.parens.is_none() {
            self.push(" ");
        }
        self.emit_params(subr.params);
    }

    fn emit_signature(&mut self, sig: Signature) {
        match sig {
            Signature::Var(var) => self.emit_var_sig(var),
            Signature::Subr(subr) => self.emit_subr_sig(subr),
        }
    }

    fn emit_block(&mut self, block: Block) {
        let block_lev = self.block_level;
        let single_block = block.len() == 1;
        if single_block {
            self.push(" ");
        } else {
            self.push("\n");
            self.column = 0;
            self.block_level += 1;
        }
        let expr_lev = self.expr_level;
        self.expr_level = 0;
        for chunk in block.into_iter() {
            if !single_block {
                self.push(INDENT.repeat(self.block_level).as_str());
            }
            self.emit_expr(chunk);
            if !single_block {
                self.push("\n");
                self.column = 0;
            }
        }
        self.block_level = block_lev;
        self.expr_level = expr_lev;
    }

    fn emit_lambda(&mut self, mut lambda: Lambda) {
        let op_is_do = &lambda.op.content == "do" || &lambda.op.content == "do!";
        if op_is_do {
            lambda.sig.params.parens = None;
        }
        self.emit_params(lambda.sig.params);
        if !op_is_do {
            self.push(" ");
        }
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
