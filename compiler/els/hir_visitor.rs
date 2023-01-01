use erg_common::traits::{Locational, Stream};
use erg_compiler::erg_parser::token::Token;
use erg_compiler::hir::*;
use erg_compiler::ty::{HasType, Type};

pub struct HIRVisitor<'a> {
    hir: &'a HIR,
    strict_cmp: bool,
}

impl<'a> HIRVisitor<'a> {
    pub fn new(hir: &'a HIR, strict_cmp: bool) -> Self {
        Self { hir, strict_cmp }
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

#[allow(unused)]
/// Returns the smallest expression containing `token`. Literals, accessors, containers, etc. are returned.
pub(crate) fn visit_hir<'h>(hir: &'h HIR, token: &Token) -> Option<&'h Expr> {
    for chunk in hir.module.iter() {
        if let Some(expr) = visit_expr(chunk, token) {
            return Some(expr);
        }
    }
    None
}

fn visit_expr<'e>(expr: &'e Expr, token: &Token) -> Option<&'e Expr> {
    if expr.col_end() < token.col_begin() {
        return None;
    }
    match expr {
        Expr::Lit(lit) => {
            if lit.token.deep_eq(token) {
                Some(expr)
            } else {
                None
            }
        }
        Expr::Accessor(acc) => visit_acc(expr, acc, token),
        Expr::BinOp(bin) => visit_bin(bin, token),
        Expr::UnaryOp(unary) => visit_expr(&unary.expr, token),
        Expr::Call(call) => visit_call(call, token),
        Expr::ClassDef(class_def) => visit_class_def(class_def, token),
        Expr::Def(def) => visit_block(&def.body.block, token),
        Expr::PatchDef(patch_def) => visit_patchdef(patch_def, token),
        Expr::Lambda(lambda) => visit_block(&lambda.body, token),
        Expr::Array(arr) => visit_array(arr, token),
        Expr::Dict(dict) => visit_dict(dict, token),
        Expr::Record(record) => visit_record(record, token),
        Expr::Set(set) => visit_set(set, token),
        Expr::Tuple(tuple) => visit_tuple(tuple, token),
        Expr::TypeAsc(type_asc) => visit_expr(&type_asc.expr, token),
        Expr::Dummy(dummy) => visit_dummy(dummy, token),
        Expr::Compound(block) | Expr::Code(block) => visit_block(block, token),
        Expr::ReDef(redef) => visit_redef(expr, redef, token),
        Expr::Import(_) => None,
    }
}

fn visit_acc<'a>(expr: &'a Expr, acc: &'a Accessor, token: &Token) -> Option<&'a Expr> {
    match acc {
        Accessor::Ident(ident) => {
            if ident.name.token().deep_eq(token) {
                Some(expr)
            } else {
                None
            }
        }
        Accessor::Attr(attr) => {
            if attr.ident.name.token().deep_eq(token) {
                Some(expr)
            } else {
                visit_expr(&attr.obj, token)
            }
        }
    }
}

fn visit_bin<'b>(bin: &'b BinOp, token: &Token) -> Option<&'b Expr> {
    visit_expr(&bin.lhs, token).or_else(|| visit_expr(&bin.rhs, token))
}

fn visit_call<'c>(call: &'c Call, token: &Token) -> Option<&'c Expr> {
    visit_expr(&call.obj, token).or_else(|| visit_args(&call.args, token))
}

fn visit_args<'a>(args: &'a Args, token: &Token) -> Option<&'a Expr> {
    for arg in args.pos_args.iter() {
        if let Some(expr) = visit_expr(&arg.expr, token) {
            return Some(expr);
        }
    }
    if let Some(var) = &args.var_args {
        if let Some(expr) = visit_expr(&var.expr, token) {
            return Some(expr);
        }
    }
    for arg in args.kw_args.iter() {
        if let Some(expr) = visit_expr(&arg.expr, token) {
            return Some(expr);
        }
    }
    None
}

fn visit_class_def<'c>(class_def: &'c ClassDef, token: &Token) -> Option<&'c Expr> {
    class_def
        .require_or_sup
        .as_ref()
        .and_then(|req_sup| visit_expr(req_sup, token))
        .or_else(|| visit_block(&class_def.methods, token))
}

fn visit_block<'b>(block: &'b Block, token: &Token) -> Option<&'b Expr> {
    for chunk in block.iter() {
        if let Some(expr) = visit_expr(chunk, token) {
            return Some(expr);
        }
    }
    None
}

fn visit_redef<'r>(expr: &'r Expr, redef: &'r ReDef, token: &Token) -> Option<&'r Expr> {
    visit_acc(expr, &redef.attr, token).or_else(|| visit_block(&redef.block, token))
}

fn visit_dummy<'d>(dummy: &'d Dummy, token: &Token) -> Option<&'d Expr> {
    for chunk in dummy.iter() {
        if let Some(expr) = visit_expr(chunk, token) {
            return Some(expr);
        }
    }
    None
}

fn visit_patchdef<'p>(patch_def: &'p PatchDef, token: &Token) -> Option<&'p Expr> {
    visit_expr(&patch_def.base, token).or_else(|| visit_block(&patch_def.methods, token))
}

fn visit_array<'a>(arr: &'a Array, token: &Token) -> Option<&'a Expr> {
    match arr {
        Array::Normal(arr) => visit_args(&arr.elems, token),
        _ => None, // todo!(),
    }
}

fn visit_dict<'d>(dict: &'d Dict, token: &Token) -> Option<&'d Expr> {
    match dict {
        Dict::Normal(dict) => {
            for kv in &dict.kvs {
                if let Some(expr) = visit_expr(&kv.key, token) {
                    return Some(expr);
                } else if let Some(expr) = visit_expr(&kv.value, token) {
                    return Some(expr);
                }
            }
            None
        }
        _ => None, // todo!(),
    }
}

fn visit_record<'r>(record: &'r Record, token: &Token) -> Option<&'r Expr> {
    for field in record.attrs.iter() {
        if let Some(expr) = visit_block(&field.body.block, token) {
            return Some(expr);
        }
    }
    None
}

fn visit_set<'s>(set: &'s Set, token: &Token) -> Option<&'s Expr> {
    match set {
        Set::Normal(set) => visit_args(&set.elems, token),
        _ => None, // todo!(),
    }
}

fn visit_tuple<'t>(tuple: &'t Tuple, token: &Token) -> Option<&'t Expr> {
    match tuple {
        Tuple::Normal(tuple) => visit_args(&tuple.elems, token),
        // _ => None, // todo!(),
    }
}
