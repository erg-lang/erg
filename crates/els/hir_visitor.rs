use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_compiler::erg_parser::token::Token;
use erg_compiler::hir::*;
use erg_compiler::ty::{HasType, Type};
use erg_compiler::varinfo::VarInfo;
use lsp_types::{Position, Range, Url};

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

    pub fn get_namespace(&self, pos: Position) -> Vec<Str> {
        // TODO: other than <module>
        let namespace = vec![Str::ever("<module>")];
        if let Some(ns) = self.get_exprs_ns(namespace.clone(), self.hir.module.iter(), pos) {
            ns
        } else {
            namespace
        }
    }

    fn is_new_final_line(&self, chunk: &Expr, pos: Position) -> bool {
        let ln_end = chunk.ln_end().unwrap_or(0);
        let line = util::get_line_from_uri(&self.uri, ln_end).unwrap();
        let indent_len = line.len() - line.trim_start_matches(' ').len();
        let cond = ln_end == pos.line && pos.character as usize == indent_len + 1;
        matches!(chunk, Expr::Call(_) | Expr::Lambda(_) | Expr::Def(_) | Expr::ClassDef(_) if cond)
    }

    pub fn get_min_expr(&self, range: Range) -> Option<&Expr> {
        self.get_min_expr_from_exprs(self.hir.module.iter(), range)
    }

    // TODO:
    fn get_min_expr_from_expr<'e>(&self, expr: &'e Expr, range: Range) -> Option<&'e Expr> {
        match expr {
            Expr::Def(def) => {
                if let Some(expr) = self.get_min_expr_from_exprs(def.body.block.iter(), range) {
                    Some(expr)
                } else if util::pos_in_loc(def, range.start) {
                    Some(expr)
                } else {
                    None
                }
            }
            Expr::Lambda(lambda) => {
                if let Some(expr) = self.get_min_expr_from_exprs(lambda.body.iter(), range) {
                    Some(expr)
                } else if util::pos_in_loc(lambda, range.start) {
                    Some(expr)
                } else {
                    None
                }
            }
            Expr::ClassDef(cdef) => {
                if let Some(expr) = self.get_min_expr_from_exprs(cdef.methods.iter(), range) {
                    Some(expr)
                } else if util::pos_in_loc(cdef, range.start) {
                    Some(expr)
                } else {
                    None
                }
            }
            Expr::PatchDef(pdef) => {
                if let Some(expr) = self.get_min_expr_from_exprs(pdef.methods.iter(), range) {
                    Some(expr)
                } else if util::pos_in_loc(pdef, range.start) {
                    Some(expr)
                } else {
                    None
                }
            }
            other => {
                if util::pos_in_loc(other, range.start) {
                    Some(expr)
                } else {
                    None
                }
            }
        }
    }

    fn get_min_expr_from_exprs<'e>(
        &self,
        iter: impl Iterator<Item = &'e Expr>,
        range: Range,
    ) -> Option<&'e Expr> {
        for chunk in iter {
            if let Some(expr) = self.get_min_expr_from_expr(chunk, range) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_ns(&self, cur_ns: Vec<Str>, chunk: &Expr, pos: Position) -> Option<Vec<Str>> {
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
            Expr::Call(call) => self.get_call_ns(cur_ns, call, pos),
            Expr::ClassDef(class_def) => self.get_class_def_ns(cur_ns, class_def, pos),
            Expr::Def(def) => self.get_def_ns(cur_ns, def, pos),
            // Expr::PatchDef(patch_def) => self.get_patchdef_ns(cur_ns, patch_def, pos),
            Expr::Lambda(lambda) => self.get_lambda_ns(cur_ns, lambda, pos),
            Expr::TypeAsc(type_asc) => self.get_expr_ns(cur_ns, &type_asc.expr, pos),
            Expr::Dummy(dummy) => self.get_dummy_ns(cur_ns, dummy, pos),
            Expr::Compound(block) | Expr::Code(block) => self.get_block_ns(cur_ns, block, pos),
        }
    }

    fn get_exprs_ns<'s, I: Iterator<Item = &'s Expr>>(
        &self,
        cur_ns: Vec<Str>,
        exprs: I,
        pos: Position,
    ) -> Option<Vec<Str>> {
        let mut namespaces = vec![];
        for expr in exprs {
            if let Some(ns) = self.get_expr_ns(cur_ns.clone(), expr, pos) {
                namespaces.push(ns);
            }
        }
        namespaces.into_iter().max_by(|l, r| l.len().cmp(&r.len()))
    }

    fn get_call_ns(&self, cur_ns: Vec<Str>, call: &Call, pos: Position) -> Option<Vec<Str>> {
        self.get_exprs_ns(cur_ns, call.args.pos_args.iter().map(|arg| &arg.expr), pos)
    }

    fn get_class_def_ns(
        &self,
        mut cur_ns: Vec<Str>,
        class_def: &ClassDef,
        pos: Position,
    ) -> Option<Vec<Str>> {
        let ns = class_def.sig.ident().to_string_notype();
        cur_ns.push(Str::from(ns));
        self.get_exprs_ns(cur_ns, class_def.methods.iter(), pos)
    }

    fn get_def_ns(&self, mut cur_ns: Vec<Str>, def: &Def, pos: Position) -> Option<Vec<Str>> {
        let ns = def.sig.ident().to_string_notype();
        cur_ns.push(Str::from(ns));
        match self.get_block_ns(cur_ns.clone(), &def.body.block, pos) {
            Some(ns) => Some(ns),
            None => Some(cur_ns),
        }
    }

    fn get_lambda_ns(
        &self,
        mut cur_ns: Vec<Str>,
        lambda: &Lambda,
        pos: Position,
    ) -> Option<Vec<Str>> {
        let ns = lambda.name_to_string();
        cur_ns.push(Str::from(ns));
        match self.get_block_ns(cur_ns.clone(), &lambda.body, pos) {
            Some(ns) => Some(ns),
            None => Some(cur_ns),
        }
    }

    fn get_block_ns(&self, cur_ns: Vec<Str>, block: &Block, pos: Position) -> Option<Vec<Str>> {
        self.get_exprs_ns(cur_ns, block.iter(), pos)
    }

    fn get_dummy_ns(&self, cur_ns: Vec<Str>, dummy: &Dummy, pos: Position) -> Option<Vec<Str>> {
        self.get_exprs_ns(cur_ns, dummy.iter(), pos)
    }

    /// Returns the smallest expression containing `token`. Literals, accessors, containers, etc. are returned.
    pub fn get_t(&self, token: &Token) -> Option<Type> {
        for chunk in self.hir.module.iter() {
            if let Some(expr) = self.get_expr_t(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn return_expr_t_if_same<T: HasType>(&self, expr: &T, l: &Token, r: &Token) -> Option<Type> {
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

    fn get_expr_t(&self, expr: &Expr, token: &Token) -> Option<Type> {
        if expr.ln_end() < token.ln_begin() || expr.ln_begin() > token.ln_end() {
            return None;
        }
        match expr {
            Expr::Lit(lit) => self.return_expr_t_if_same(expr, &lit.token, token),
            Expr::Accessor(acc) => self.get_acc_t(expr, acc, token),
            Expr::BinOp(bin) => self.get_bin_t(bin, token),
            Expr::UnaryOp(unary) => self.get_expr_t(&unary.expr, token),
            Expr::Call(call) => self.get_call_t(call, token),
            Expr::ClassDef(class_def) => self.get_class_def_t(class_def, token),
            Expr::Def(def) => self.get_def_t(def, token),
            Expr::PatchDef(patch_def) => self.get_patchdef_t(patch_def, token),
            Expr::Lambda(lambda) => self.get_block_t(&lambda.body, token),
            Expr::Array(arr) => self.get_array_t(arr, token),
            Expr::Dict(dict) => self.get_dict_t(dict, token),
            Expr::Record(record) => self.get_record_t(record, token),
            Expr::Set(set) => self.get_set_t(set, token),
            Expr::Tuple(tuple) => self.get_tuple_t(tuple, token),
            Expr::TypeAsc(type_asc) => self.get_expr_t(&type_asc.expr, token),
            Expr::Dummy(dummy) => self.get_dummy_t(dummy, token),
            Expr::Compound(block) | Expr::Code(block) => self.get_block_t(block, token),
            Expr::ReDef(redef) => self.get_redef_t(expr, redef, token),
            Expr::Import(_) => None,
        }
    }

    fn get_acc_t(&self, expr: &Expr, acc: &Accessor, token: &Token) -> Option<Type> {
        match acc {
            Accessor::Ident(ident) => self.return_expr_t_if_same(expr, ident.name.token(), token),
            Accessor::Attr(attr) => self
                .return_expr_t_if_same(expr, attr.ident.name.token(), token)
                .or_else(|| self.get_expr_t(&attr.obj, token)),
        }
    }

    fn get_bin_t(&self, bin: &BinOp, token: &Token) -> Option<Type> {
        self.get_expr_t(&bin.lhs, token)
            .or_else(|| self.get_expr_t(&bin.rhs, token))
    }

    fn get_call_t(&self, call: &Call, token: &Token) -> Option<Type> {
        if let Some(attr) = &call.attr_name {
            if let Some(t) = self.return_expr_t_if_same(attr, attr.name.token(), token) {
                return Some(t);
            }
        }
        self.get_expr_t(&call.obj, token)
            .or_else(|| self.get_args_t(&call.args, token))
    }

    fn get_args_t(&self, args: &Args, token: &Token) -> Option<Type> {
        for arg in args.pos_args.iter() {
            if let Some(expr) = self.get_expr_t(&arg.expr, token) {
                return Some(expr);
            }
        }
        if let Some(var) = &args.var_args {
            if let Some(expr) = self.get_expr_t(&var.expr, token) {
                return Some(expr);
            }
        }
        for arg in args.kw_args.iter() {
            if let Some(expr) = self.get_expr_t(&arg.expr, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_def_t(&self, def: &Def, token: &Token) -> Option<Type> {
        self.return_expr_t_if_same(def.sig.ident(), def.sig.ident().name.token(), token)
            .or_else(|| self.get_block_t(&def.body.block, token))
    }

    fn get_class_def_t(&self, class_def: &ClassDef, token: &Token) -> Option<Type> {
        class_def
            .require_or_sup
            .as_ref()
            .and_then(|req_sup| self.get_expr_t(req_sup, token))
            .or_else(|| {
                self.return_expr_t_if_same(
                    class_def.sig.ident(),
                    class_def.sig.ident().name.token(),
                    token,
                )
            })
            .or_else(|| self.get_block_t(&class_def.methods, token))
    }

    fn get_block_t(&self, block: &Block, token: &Token) -> Option<Type> {
        for chunk in block.iter() {
            if let Some(expr) = self.get_expr_t(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_redef_t(&self, expr: &Expr, redef: &ReDef, token: &Token) -> Option<Type> {
        self.get_acc_t(expr, &redef.attr, token)
            .or_else(|| self.get_block_t(&redef.block, token))
    }

    fn get_dummy_t(&self, dummy: &Dummy, token: &Token) -> Option<Type> {
        for chunk in dummy.iter() {
            if let Some(expr) = self.get_expr_t(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_patchdef_t(&self, patch_def: &PatchDef, token: &Token) -> Option<Type> {
        self.get_expr_t(&patch_def.base, token)
            .or_else(|| self.get_block_t(&patch_def.methods, token))
    }

    fn get_array_t(&self, arr: &Array, token: &Token) -> Option<Type> {
        match arr {
            Array::Normal(arr) => self.get_args_t(&arr.elems, token),
            _ => None, // todo!(),
        }
    }

    fn get_dict_t(&self, dict: &Dict, token: &Token) -> Option<Type> {
        match dict {
            Dict::Normal(dict) => {
                for kv in &dict.kvs {
                    if let Some(expr) = self.get_expr_t(&kv.key, token) {
                        return Some(expr);
                    } else if let Some(expr) = self.get_expr_t(&kv.value, token) {
                        return Some(expr);
                    }
                }
                None
            }
            _ => None, // todo!(),
        }
    }

    fn get_record_t(&self, record: &Record, token: &Token) -> Option<Type> {
        for field in record.attrs.iter() {
            if let Some(expr) = self.get_block_t(&field.body.block, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_set_t(&self, set: &Set, token: &Token) -> Option<Type> {
        match set {
            Set::Normal(set) => self.get_args_t(&set.elems, token),
            _ => None, // todo!(),
        }
    }

    fn get_tuple_t(&self, tuple: &Tuple, token: &Token) -> Option<Type> {
        match tuple {
            Tuple::Normal(tuple) => self.get_args_t(&tuple.elems, token),
            // _ => None, // todo!(),
        }
    }
}

impl<'a> HIRVisitor<'a> {
    /// Returns the smallest expression containing `token`. Literals, accessors, containers, etc. are returned.
    pub fn get_info(&self, token: &Token) -> Option<VarInfo> {
        for chunk in self.hir.module.iter() {
            if let Some(expr) = self.get_expr_info(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn return_var_info_if_same(&self, ident: &Identifier, l: &Token, r: &Token) -> Option<VarInfo> {
        if self.strict_cmp {
            if l.deep_eq(r) {
                Some(ident.vi.clone())
            } else {
                None
            }
        } else if l == r {
            Some(ident.vi.clone())
        } else {
            None
        }
    }

    fn get_expr_info(&self, expr: &Expr, token: &Token) -> Option<VarInfo> {
        if expr.ln_end() < token.ln_begin() || expr.ln_begin() > token.ln_end() {
            return None;
        }
        match expr {
            Expr::Lit(_lit) => None,
            Expr::Accessor(acc) => self.get_acc_info(acc, token),
            Expr::BinOp(bin) => self.get_bin_info(bin, token),
            Expr::UnaryOp(unary) => self.get_expr_info(&unary.expr, token),
            Expr::Call(call) => self.get_call_info(call, token),
            Expr::ClassDef(class_def) => self.get_class_def_info(class_def, token),
            Expr::Def(def) => self.get_def_info(def, token),
            Expr::PatchDef(patch_def) => self.get_patchdef_info(patch_def, token),
            Expr::Lambda(lambda) => self.get_lambda_info(lambda, token),
            Expr::Array(arr) => self.get_array_info(arr, token),
            Expr::Dict(dict) => self.get_dict_info(dict, token),
            Expr::Record(record) => self.get_record_info(record, token),
            Expr::Set(set) => self.get_set_info(set, token),
            Expr::Tuple(tuple) => self.get_tuple_info(tuple, token),
            Expr::TypeAsc(type_asc) => self.get_expr_info(&type_asc.expr, token),
            Expr::Dummy(dummy) => self.get_dummy_info(dummy, token),
            Expr::Compound(block) | Expr::Code(block) => self.get_block_info(block, token),
            Expr::ReDef(redef) => self.get_redef_info(redef, token),
            Expr::Import(_) => None,
        }
    }

    fn get_acc_info(&self, acc: &Accessor, token: &Token) -> Option<VarInfo> {
        match acc {
            Accessor::Ident(ident) => {
                self.return_var_info_if_same(ident, ident.name.token(), token)
            }
            Accessor::Attr(attr) => self
                .return_var_info_if_same(&attr.ident, attr.ident.name.token(), token)
                .or_else(|| self.get_expr_info(&attr.obj, token)),
        }
    }

    fn get_bin_info(&self, bin: &BinOp, token: &Token) -> Option<VarInfo> {
        self.get_expr_info(&bin.lhs, token)
            .or_else(|| self.get_expr_info(&bin.rhs, token))
    }

    fn get_call_info(&self, call: &Call, token: &Token) -> Option<VarInfo> {
        if let Some(attr) = &call.attr_name {
            if let Some(t) = self.return_var_info_if_same(attr, attr.name.token(), token) {
                return Some(t);
            }
        }
        self.get_expr_info(&call.obj, token)
            .or_else(|| self.get_args_info(&call.args, token))
    }

    fn get_args_info(&self, args: &Args, token: &Token) -> Option<VarInfo> {
        for arg in args.pos_args.iter() {
            if let Some(expr) = self.get_expr_info(&arg.expr, token) {
                return Some(expr);
            }
        }
        if let Some(var) = &args.var_args {
            if let Some(expr) = self.get_expr_info(&var.expr, token) {
                return Some(expr);
            }
        }
        for arg in args.kw_args.iter() {
            if let Some(expr) = self.get_expr_info(&arg.expr, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_sig_info(&self, sig: &Signature, token: &Token) -> Option<VarInfo> {
        match sig {
            Signature::Var(var) => {
                self.return_var_info_if_same(&var.ident, var.ident.name.token(), token)
            }
            Signature::Subr(subr) => self
                .return_var_info_if_same(&subr.ident, subr.ident.name.token(), token)
                .or_else(|| self.get_params_info(&subr.params, token)),
        }
    }

    fn get_params_info(&self, params: &Params, token: &Token) -> Option<VarInfo> {
        for param in params.non_defaults.iter() {
            if param.raw.pat.loc() == token.loc() {
                return Some(param.vi.clone());
            }
        }
        if let Some(var) = &params.var_params {
            if var.raw.pat.loc() == token.loc() {
                return Some(var.vi.clone());
            }
        }
        for param in params.defaults.iter() {
            if param.sig.raw.pat.loc() == token.loc() {
                return Some(param.sig.vi.clone());
            } else if let Some(vi) = self.get_expr_info(&param.default_val, token) {
                return Some(vi);
            }
        }
        None
    }

    fn get_def_info(&self, def: &Def, token: &Token) -> Option<VarInfo> {
        self.get_sig_info(&def.sig, token)
            .or_else(|| self.get_block_info(&def.body.block, token))
    }

    fn get_class_def_info(&self, class_def: &ClassDef, token: &Token) -> Option<VarInfo> {
        class_def
            .require_or_sup
            .as_ref()
            .and_then(|req_sup| self.get_expr_info(req_sup, token))
            .or_else(|| self.get_sig_info(&class_def.sig, token))
            .or_else(|| self.get_block_info(&class_def.methods, token))
    }

    fn get_block_info(&self, block: &Block, token: &Token) -> Option<VarInfo> {
        for chunk in block.iter() {
            if let Some(expr) = self.get_expr_info(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_redef_info(&self, redef: &ReDef, token: &Token) -> Option<VarInfo> {
        self.get_acc_info(&redef.attr, token)
            .or_else(|| self.get_block_info(&redef.block, token))
    }

    fn get_dummy_info(&self, dummy: &Dummy, token: &Token) -> Option<VarInfo> {
        for chunk in dummy.iter() {
            if let Some(expr) = self.get_expr_info(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_patchdef_info(&self, patch_def: &PatchDef, token: &Token) -> Option<VarInfo> {
        self.get_expr_info(&patch_def.base, token)
            .or_else(|| self.get_block_info(&patch_def.methods, token))
    }

    fn get_lambda_info(&self, lambda: &Lambda, token: &Token) -> Option<VarInfo> {
        self.get_params_info(&lambda.params, token)
            .or_else(|| self.get_block_info(&lambda.body, token))
    }

    fn get_array_info(&self, arr: &Array, token: &Token) -> Option<VarInfo> {
        match arr {
            Array::Normal(arr) => self.get_args_info(&arr.elems, token),
            _ => None, // todo!(),
        }
    }

    fn get_dict_info(&self, dict: &Dict, token: &Token) -> Option<VarInfo> {
        match dict {
            Dict::Normal(dict) => {
                for kv in &dict.kvs {
                    if let Some(expr) = self.get_expr_info(&kv.key, token) {
                        return Some(expr);
                    } else if let Some(expr) = self.get_expr_info(&kv.value, token) {
                        return Some(expr);
                    }
                }
                None
            }
            _ => None, // todo!(),
        }
    }

    fn get_record_info(&self, record: &Record, token: &Token) -> Option<VarInfo> {
        for field in record.attrs.iter() {
            if let Some(expr) = self.get_def_info(field, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_set_info(&self, set: &Set, token: &Token) -> Option<VarInfo> {
        match set {
            Set::Normal(set) => self.get_args_info(&set.elems, token),
            _ => None, // todo!(),
        }
    }

    fn get_tuple_info(&self, tuple: &Tuple, token: &Token) -> Option<VarInfo> {
        match tuple {
            Tuple::Normal(tuple) => self.get_args_info(&tuple.elems, token),
            // _ => None, // todo!(),
        }
    }
}
