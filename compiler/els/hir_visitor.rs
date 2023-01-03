use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_compiler::erg_parser::token::Token;
use erg_compiler::hir::*;
use erg_compiler::ty::{HasType, Type};
use lsp_types::{Position, Url};

use crate::util;

pub struct HIRVisitor<'a> {
    hir: &'a HIR,
    uri: Url,
    strict_cmp: bool,
}

impl<'a> HIRVisitor<'a> {
    pub fn new(hir: &'a HIR, uri: Url, strict_cmp: bool) -> Self {
        Self {
            hir,
            uri,
            strict_cmp,
        }
    }

    pub(crate) fn get_namespace(&self, pos: Position) -> Vec<Str> {
        // TODO: other than <module>
        let namespace = vec![Str::ever("<module>")];
        if let Some(ns) = self.visit_exprs_ns(namespace.clone(), self.hir.module.iter(), pos) {
            ns
        } else {
            namespace
        }
    }

    fn is_new_final_line(&self, chunk: &Expr, pos: Position) -> bool {
        let ln_end = chunk.ln_end().unwrap_or(0);
        let line = util::get_line_from_uri(&self.uri, ln_end).unwrap();
        let indent_len = line.len() - line.trim_start_matches(' ').len();
        let cond = ln_end == pos.line as usize && pos.character as usize == indent_len + 1;
        matches!(chunk, Expr::Call(_) | Expr::Lambda(_) | Expr::Def(_) | Expr::ClassDef(_) if cond)
    }

    fn visit_expr_ns(&self, cur_ns: Vec<Str>, chunk: &Expr, pos: Position) -> Option<Vec<Str>> {
        if !(util::pos_in_loc(chunk, pos) || self.is_new_final_line(chunk, pos)) {
            return None;
        }
        match chunk {
            Expr::Lit(_)
            | Expr::Accessor(_)
            | Expr::BinOp(_)
            | Expr::UnaryOp(_)
            | Expr::Array(_)
            | Expr::Dict(_)
            | Expr::Set(_)
            | Expr::Tuple(_)
            | Expr::Import(_) => None,
            Expr::PatchDef(_) | Expr::Record(_) | Expr::ReDef(_) => None,
            Expr::Call(call) => self.visit_call_ns(cur_ns, call, pos),
            Expr::ClassDef(class_def) => self.visit_class_def_ns(cur_ns, class_def, pos),
            Expr::Def(def) => self.visit_def_ns(cur_ns, def, pos),
            // Expr::PatchDef(patch_def) => self.visit_patchdef_ns(cur_ns, patch_def, pos),
            Expr::Lambda(lambda) => self.visit_lambda_ns(cur_ns, lambda, pos),
            Expr::TypeAsc(type_asc) => self.visit_expr_ns(cur_ns, &type_asc.expr, pos),
            Expr::Dummy(dummy) => self.visit_dummy_ns(cur_ns, dummy, pos),
            Expr::Compound(block) | Expr::Code(block) => self.visit_block_ns(cur_ns, block, pos),
        }
    }

    fn visit_exprs_ns<'s, I: Iterator<Item = &'s Expr>>(
        &self,
        cur_ns: Vec<Str>,
        exprs: I,
        pos: Position,
    ) -> Option<Vec<Str>> {
        let mut namespaces = vec![];
        for expr in exprs {
            if let Some(ns) = self.visit_expr_ns(cur_ns.clone(), expr, pos) {
                namespaces.push(ns);
            }
        }
        namespaces.into_iter().max_by(|l, r| l.len().cmp(&r.len()))
    }

    fn visit_call_ns(&self, cur_ns: Vec<Str>, call: &Call, pos: Position) -> Option<Vec<Str>> {
        self.visit_exprs_ns(cur_ns, call.args.pos_args.iter().map(|arg| &arg.expr), pos)
    }

    fn visit_class_def_ns(
        &self,
        cur_ns: Vec<Str>,
        class_def: &ClassDef,
        pos: Position,
    ) -> Option<Vec<Str>> {
        self.visit_exprs_ns(cur_ns, class_def.methods.iter(), pos)
    }

    fn visit_def_ns(&self, mut cur_ns: Vec<Str>, def: &Def, pos: Position) -> Option<Vec<Str>> {
        let ns = def.sig.ident().to_string_notype();
        cur_ns.push(Str::from(ns));
        match self.visit_block_ns(cur_ns.clone(), &def.body.block, pos) {
            Some(ns) => Some(ns),
            None => Some(cur_ns),
        }
    }

    fn visit_lambda_ns(
        &self,
        mut cur_ns: Vec<Str>,
        lambda: &Lambda,
        pos: Position,
    ) -> Option<Vec<Str>> {
        let ns = lambda.name_to_string();
        cur_ns.push(Str::from(ns));
        match self.visit_block_ns(cur_ns.clone(), &lambda.body, pos) {
            Some(ns) => Some(ns),
            None => Some(cur_ns),
        }
    }

    fn visit_block_ns(&self, cur_ns: Vec<Str>, block: &Block, pos: Position) -> Option<Vec<Str>> {
        self.visit_exprs_ns(cur_ns, block.iter(), pos)
    }

    fn visit_dummy_ns(&self, cur_ns: Vec<Str>, dummy: &Dummy, pos: Position) -> Option<Vec<Str>> {
        self.visit_exprs_ns(cur_ns, dummy.iter(), pos)
    }

    /// Returns the smallest expression containing `token`. Literals, accessors, containers, etc. are returned.
    pub(crate) fn visit_hir_t(&self, token: &Token) -> Option<Type> {
        for chunk in self.hir.module.iter() {
            if let Some(expr) = self.visit_expr_t(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn cmp_return_expr_t<T: HasType>(&self, expr: &T, l: &Token, r: &Token) -> Option<Type> {
        if self.strict_cmp {
            if l.deep_eq(r) {
                Some(expr.t())
            } else {
                None
            }
        } else if l == r {
            Some(expr.t())
        } else {
            None
        }
    }

    fn visit_expr_t(&self, expr: &Expr, token: &Token) -> Option<Type> {
        if expr.ln_end() < token.ln_begin() || expr.ln_begin() > token.ln_end() {
            return None;
        }
        match expr {
            Expr::Lit(lit) => self.cmp_return_expr_t(expr, &lit.token, token),
            Expr::Accessor(acc) => self.visit_acc_t(expr, acc, token),
            Expr::BinOp(bin) => self.visit_bin_t(bin, token),
            Expr::UnaryOp(unary) => self.visit_expr_t(&unary.expr, token),
            Expr::Call(call) => self.visit_call_t(call, token),
            Expr::ClassDef(class_def) => self.visit_class_def_t(class_def, token),
            Expr::Def(def) => self.visit_block_t(&def.body.block, token),
            Expr::PatchDef(patch_def) => self.visit_patchdef_t(patch_def, token),
            Expr::Lambda(lambda) => self.visit_block_t(&lambda.body, token),
            Expr::Array(arr) => self.visit_array_t(arr, token),
            Expr::Dict(dict) => self.visit_dict_t(dict, token),
            Expr::Record(record) => self.visit_record_t(record, token),
            Expr::Set(set) => self.visit_set_t(set, token),
            Expr::Tuple(tuple) => self.visit_tuple_t(tuple, token),
            Expr::TypeAsc(type_asc) => self.visit_expr_t(&type_asc.expr, token),
            Expr::Dummy(dummy) => self.visit_dummy_t(dummy, token),
            Expr::Compound(block) | Expr::Code(block) => self.visit_block_t(block, token),
            Expr::ReDef(redef) => self.visit_redef_t(expr, redef, token),
            Expr::Import(_) => None,
        }
    }

    fn visit_acc_t(&self, expr: &Expr, acc: &Accessor, token: &Token) -> Option<Type> {
        match acc {
            Accessor::Ident(ident) => self.cmp_return_expr_t(expr, ident.name.token(), token),
            Accessor::Attr(attr) => self
                .cmp_return_expr_t(expr, attr.ident.name.token(), token)
                .or_else(|| self.visit_expr_t(&attr.obj, token)),
        }
    }

    fn visit_bin_t(&self, bin: &BinOp, token: &Token) -> Option<Type> {
        self.visit_expr_t(&bin.lhs, token)
            .or_else(|| self.visit_expr_t(&bin.rhs, token))
    }

    fn visit_call_t(&self, call: &Call, token: &Token) -> Option<Type> {
        if let Some(attr) = &call.attr_name {
            if let Some(t) = self.cmp_return_expr_t(attr, attr.name.token(), token) {
                return Some(t);
            }
        }
        self.visit_expr_t(&call.obj, token)
            .or_else(|| self.visit_args_t(&call.args, token))
    }

    fn visit_args_t(&self, args: &Args, token: &Token) -> Option<Type> {
        for arg in args.pos_args.iter() {
            if let Some(expr) = self.visit_expr_t(&arg.expr, token) {
                return Some(expr);
            }
        }
        if let Some(var) = &args.var_args {
            if let Some(expr) = self.visit_expr_t(&var.expr, token) {
                return Some(expr);
            }
        }
        for arg in args.kw_args.iter() {
            if let Some(expr) = self.visit_expr_t(&arg.expr, token) {
                return Some(expr);
            }
        }
        None
    }

    fn visit_class_def_t(&self, class_def: &ClassDef, token: &Token) -> Option<Type> {
        class_def
            .require_or_sup
            .as_ref()
            .and_then(|req_sup| self.visit_expr_t(req_sup, token))
            .or_else(|| self.visit_block_t(&class_def.methods, token))
    }

    fn visit_block_t(&self, block: &Block, token: &Token) -> Option<Type> {
        for chunk in block.iter() {
            if let Some(expr) = self.visit_expr_t(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn visit_redef_t(&self, expr: &Expr, redef: &ReDef, token: &Token) -> Option<Type> {
        self.visit_acc_t(expr, &redef.attr, token)
            .or_else(|| self.visit_block_t(&redef.block, token))
    }

    fn visit_dummy_t(&self, dummy: &Dummy, token: &Token) -> Option<Type> {
        for chunk in dummy.iter() {
            if let Some(expr) = self.visit_expr_t(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn visit_patchdef_t(&self, patch_def: &PatchDef, token: &Token) -> Option<Type> {
        self.visit_expr_t(&patch_def.base, token)
            .or_else(|| self.visit_block_t(&patch_def.methods, token))
    }

    fn visit_array_t(&self, arr: &Array, token: &Token) -> Option<Type> {
        match arr {
            Array::Normal(arr) => self.visit_args_t(&arr.elems, token),
            _ => None, // todo!(),
        }
    }

    fn visit_dict_t(&self, dict: &Dict, token: &Token) -> Option<Type> {
        match dict {
            Dict::Normal(dict) => {
                for kv in &dict.kvs {
                    if let Some(expr) = self.visit_expr_t(&kv.key, token) {
                        return Some(expr);
                    } else if let Some(expr) = self.visit_expr_t(&kv.value, token) {
                        return Some(expr);
                    }
                }
                None
            }
            _ => None, // todo!(),
        }
    }

    fn visit_record_t(&self, record: &Record, token: &Token) -> Option<Type> {
        for field in record.attrs.iter() {
            if let Some(expr) = self.visit_block_t(&field.body.block, token) {
                return Some(expr);
            }
        }
        None
    }

    fn visit_set_t(&self, set: &Set, token: &Token) -> Option<Type> {
        match set {
            Set::Normal(set) => self.visit_args_t(&set.elems, token),
            _ => None, // todo!(),
        }
    }

    fn visit_tuple_t(&self, tuple: &Tuple, token: &Token) -> Option<Type> {
        match tuple {
            Tuple::Normal(tuple) => self.visit_args_t(&tuple.elems, token),
            // _ => None, // todo!(),
        }
    }
}
