use erg_common::traits::Locational;
use erg_common::Str;
use erg_compiler::erg_parser::token::Token;
use erg_compiler::hir::*;
use erg_compiler::varinfo::VarInfo;
use lsp_types::{Position, Url};

use crate::util;

/// This struct provides:
/// * namespace where the cursor is located (`get_namespace`)
/// * cursor(`Token`) -> `Expr` mapping (`get_min_expr`)
/// * cursor(`Token`) -> `VarInfo` mapping (`get_info`)
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
            Expr::Record(_) | Expr::ReDef(_) => None,
            Expr::Call(call) => self.get_call_ns(cur_ns, call, pos),
            Expr::ClassDef(class_def) => self.get_class_def_ns(cur_ns, class_def, pos),
            Expr::PatchDef(patch_def) => self.get_patch_def_ns(cur_ns, patch_def, pos),
            Expr::Def(def) => self.get_def_ns(cur_ns, def, pos),
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

    fn get_patch_def_ns(
        &self,
        mut cur_ns: Vec<Str>,
        patch_def: &PatchDef,
        pos: Position,
    ) -> Option<Vec<Str>> {
        let ns = patch_def.sig.ident().to_string_notype();
        cur_ns.push(Str::from(ns));
        self.get_exprs_ns(cur_ns, patch_def.methods.iter(), pos)
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
    pub fn get_min_expr(&self, token: &Token) -> Option<&Expr> {
        for chunk in self.hir.module.iter() {
            if let Some(expr) = self.get_expr(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn return_expr_if_same<'e>(&'e self, expr: &'e Expr, l: &Token, r: &Token) -> Option<&Expr> {
        if self.strict_cmp {
            if l.deep_eq(r) {
                Some(expr)
            } else {
                None
            }
        } else if l == r {
            Some(expr)
        } else {
            None
        }
    }

    fn get_expr<'e>(&'e self, expr: &'e Expr, token: &Token) -> Option<&'e Expr> {
        if expr.ln_end() < token.ln_begin() || expr.ln_begin() > token.ln_end() {
            return None;
        }
        match expr {
            Expr::Lit(lit) => self.return_expr_if_same(expr, &lit.token, token),
            Expr::Accessor(acc) => self.get_expr_from_acc(expr, acc, token),
            Expr::BinOp(bin) => self.get_expr_from_bin(expr, bin, token),
            Expr::UnaryOp(unary) => self.get_expr(&unary.expr, token),
            Expr::Call(call) => self.get_expr_from_call(expr, call, token),
            Expr::ClassDef(class_def) => self.get_expr_from_class_def(expr, class_def, token),
            Expr::Def(def) => self.get_expr_from_def(expr, def, token),
            Expr::PatchDef(patch_def) => self.get_expr_from_patch_def(expr, patch_def, token),
            Expr::Lambda(lambda) => self.get_expr_from_lambda(expr, lambda, token),
            Expr::Array(arr) => self.get_expr_from_array(expr, arr, token),
            Expr::Dict(dict) => self.get_expr_from_dict(expr, dict, token),
            Expr::Record(record) => self.get_expr_from_record(expr, record, token),
            Expr::Set(set) => self.get_expr_from_set(expr, set, token),
            Expr::Tuple(tuple) => self.get_expr_from_tuple(expr, tuple, token),
            Expr::TypeAsc(type_asc) => self.get_expr(&type_asc.expr, token),
            Expr::Dummy(dummy) => self.get_expr_from_dummy(dummy, token),
            Expr::Compound(block) | Expr::Code(block) => self.get_expr_from_block(block, token),
            Expr::ReDef(redef) => self.get_expr_from_redef(expr, redef, token),
            Expr::Import(_) => None,
        }
    }

    fn get_expr_from_acc<'e>(
        &'e self,
        expr: &'e Expr,
        acc: &'e Accessor,
        token: &Token,
    ) -> Option<&Expr> {
        match acc {
            Accessor::Ident(ident) => self.return_expr_if_same(expr, ident.name.token(), token),
            Accessor::Attr(attr) => self
                .return_expr_if_same(expr, attr.ident.name.token(), token)
                .or_else(|| self.get_expr(&attr.obj, token)),
        }
    }

    fn get_expr_from_bin<'e>(
        &'e self,
        expr: &'e Expr,
        bin: &'e BinOp,
        token: &Token,
    ) -> Option<&Expr> {
        if &bin.op == token {
            return Some(expr);
        }
        self.get_expr(&bin.lhs, token)
            .or_else(|| self.get_expr(&bin.rhs, token))
    }

    fn get_expr_from_call<'e>(
        &'e self,
        expr: &'e Expr,
        call: &'e Call,
        token: &Token,
    ) -> Option<&Expr> {
        // args: `(1, 2)`, token: `)`
        call.args
            .paren
            .as_ref()
            .and_then(|(_, end)| self.return_expr_if_same(expr, end, token))
            .or_else(|| {
                call.attr_name
                    .as_ref()
                    .and_then(|attr| self.return_expr_if_same(expr, attr.name.token(), token))
            })
            .or_else(|| self.get_expr(&call.obj, token))
            .or_else(|| self.get_expr_from_args(&call.args, token))
            .or_else(|| call.loc().contains(token.loc()).then_some(expr))
    }

    fn get_expr_from_args<'e>(&'e self, args: &'e Args, token: &Token) -> Option<&Expr> {
        for arg in args.pos_args.iter() {
            if let Some(expr) = self.get_expr(&arg.expr, token) {
                return Some(expr);
            }
        }
        if let Some(var) = &args.var_args {
            if let Some(expr) = self.get_expr(&var.expr, token) {
                return Some(expr);
            }
        }
        for arg in args.kw_args.iter() {
            if let Some(expr) = self.get_expr(&arg.expr, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_from_def<'e>(
        &'e self,
        expr: &'e Expr,
        def: &'e Def,
        token: &Token,
    ) -> Option<&Expr> {
        self.return_expr_if_same(expr, def.sig.ident().name.token(), token)
            .or_else(|| self.get_expr_from_block(&def.body.block, token))
            .or_else(|| def.loc().contains(token.loc()).then_some(expr))
    }

    fn get_expr_from_class_def<'e>(
        &'e self,
        expr: &'e Expr,
        class_def: &'e ClassDef,
        token: &Token,
    ) -> Option<&Expr> {
        class_def
            .require_or_sup
            .as_ref()
            .and_then(|req_sup| self.get_expr(req_sup, token))
            .or_else(|| self.get_expr_from_block(&class_def.methods, token))
            .or_else(|| class_def.loc().contains(token.loc()).then_some(expr))
    }

    fn get_expr_from_block<'e>(&'e self, block: &'e Block, token: &Token) -> Option<&Expr> {
        for chunk in block.iter() {
            if let Some(expr) = self.get_expr(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_from_redef<'e>(
        &'e self,
        expr: &'e Expr,
        redef: &'e ReDef,
        token: &Token,
    ) -> Option<&Expr> {
        self.get_expr_from_acc(expr, &redef.attr, token)
            .or_else(|| self.get_expr_from_block(&redef.block, token))
    }

    fn get_expr_from_dummy<'e>(&'e self, dummy: &'e Dummy, token: &Token) -> Option<&Expr> {
        for chunk in dummy.iter() {
            if let Some(expr) = self.get_expr(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_from_patch_def<'e>(
        &'e self,
        expr: &'e Expr,
        patch_def: &'e PatchDef,
        token: &Token,
    ) -> Option<&Expr> {
        self.return_expr_if_same(expr, patch_def.sig.ident().name.token(), token)
            .or_else(|| self.get_expr(&patch_def.base, token))
            .or_else(|| self.get_expr_from_block(&patch_def.methods, token))
            .or_else(|| patch_def.loc().contains(token.loc()).then_some(expr))
    }

    fn get_expr_from_lambda<'e>(
        &'e self,
        expr: &'e Expr,
        lambda: &'e Lambda,
        token: &Token,
    ) -> Option<&Expr> {
        if util::pos_in_loc(&lambda.params, util::loc_to_pos(token.loc())?) {
            return Some(expr);
        }
        self.get_expr_from_block(&lambda.body, token)
    }

    fn get_expr_from_array<'e>(
        &'e self,
        expr: &'e Expr,
        arr: &'e Array,
        token: &Token,
    ) -> Option<&Expr> {
        if arr.ln_end() == token.ln_end() {
            // arr: `[1, 2]`, token: `]`
            return Some(expr);
        }
        match arr {
            Array::Normal(arr) => self.get_expr_from_args(&arr.elems, token),
            _ => None, // todo!(),
        }
    }

    fn get_expr_from_dict<'e>(
        &'e self,
        expr: &'e Expr,
        dict: &'e Dict,
        token: &Token,
    ) -> Option<&Expr> {
        if dict.ln_end() == token.ln_end() {
            // arr: `{...}`, token: `}`
            return Some(expr);
        }
        match dict {
            Dict::Normal(dict) => {
                for kv in &dict.kvs {
                    if let Some(expr) = self
                        .get_expr(&kv.key, token)
                        .or_else(|| self.get_expr(&kv.value, token))
                    {
                        return Some(expr);
                    }
                }
                None
            }
            _ => None, // todo!(),
        }
    }

    fn get_expr_from_record<'e>(
        &'e self,
        expr: &'e Expr,
        record: &'e Record,
        token: &Token,
    ) -> Option<&Expr> {
        if record.ln_end() == token.ln_end() {
            // arr: `{...}`, token: `}`
            return Some(expr);
        }
        for field in record.attrs.iter() {
            if let Some(expr) = self.get_expr_from_block(&field.body.block, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_from_set<'e>(
        &'e self,
        expr: &'e Expr,
        set: &'e Set,
        token: &Token,
    ) -> Option<&Expr> {
        if set.ln_end() == token.ln_end() {
            // arr: `{...}`, token: `}`
            return Some(expr);
        }
        match set {
            Set::Normal(set) => self.get_expr_from_args(&set.elems, token),
            _ => None, // todo!(),
        }
    }

    fn get_expr_from_tuple<'e>(
        &'e self,
        expr: &'e Expr,
        tuple: &'e Tuple,
        token: &Token,
    ) -> Option<&Expr> {
        match tuple {
            Tuple::Normal(tuple) => {
                // arr: `(1, 2)`, token: `)`
                tuple
                    .elems
                    .paren
                    .as_ref()
                    .and_then(|(_, end)| self.return_expr_if_same(expr, end, token))
                    .or_else(|| self.get_expr_from_args(&tuple.elems, token))
            } // _ => None, // todo!(),
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
        let loc = expr.loc();
        if loc.ln_end() < token.ln_begin() || loc.ln_begin() > token.ln_end() {
            return None;
        }
        match expr {
            Expr::Lit(_lit) => None,
            Expr::Accessor(acc) => self.get_acc_info(acc, token),
            Expr::BinOp(bin) => self.get_bin_info(bin, token),
            Expr::UnaryOp(unary) => self.get_expr_info(&unary.expr, token),
            Expr::Call(call) => self.get_call_info(call, token),
            Expr::ClassDef(class_def) => self.get_class_def_info(class_def, token),
            Expr::PatchDef(patch_def) => self.get_patch_def_info(patch_def, token),
            Expr::Def(def) => self.get_def_info(def, token),
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

    fn get_patch_def_info(&self, patch_def: &PatchDef, token: &Token) -> Option<VarInfo> {
        self.get_expr_info(&patch_def.base, token)
            .or_else(|| self.get_sig_info(&patch_def.sig, token))
            .or_else(|| self.get_block_info(&patch_def.methods, token))
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
