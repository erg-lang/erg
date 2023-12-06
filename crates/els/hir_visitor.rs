use erg_common::consts::ERG_MODE;
use erg_common::shared::MappedRwLockReadGuard;
use erg_common::traits::Locational;
use erg_common::Str;
use erg_compiler::erg_parser::token::Token;
use erg_compiler::hir::*;
use erg_compiler::varinfo::VarInfo;
use lsp_types::Position;

use crate::file_cache::FileCache;
use crate::util::{self, pos_to_loc, NormalizedUrl};

trait PosLocational {
    fn ln_begin(&self) -> Option<u32>;
    fn ln_end(&self) -> Option<u32>;
    fn loc(&self) -> erg_common::error::Location;
}

impl PosLocational for Position {
    fn ln_begin(&self) -> Option<u32> {
        Some(self.line + 1)
    }
    fn ln_end(&self) -> Option<u32> {
        Some(self.line + 1)
    }
    fn loc(&self) -> erg_common::error::Location {
        pos_to_loc(*self)
    }
}

pub trait GetExprKind {
    const KIND: ExprKind;
}
impl GetExprKind for Expr {
    const KIND: ExprKind = ExprKind::Expr;
}
impl GetExprKind for Call {
    const KIND: ExprKind = ExprKind::Call;
}
impl GetExprKind for Def {
    const KIND: ExprKind = ExprKind::Def;
}

#[derive(Debug, Copy, Clone)]
pub enum ExprKind {
    Call,
    Def,
    Expr,
}

impl ExprKind {
    pub const fn matches(&self, expr: &Expr) -> bool {
        match (self, expr) {
            (ExprKind::Call, Expr::Call(_)) | (ExprKind::Expr, _) => true,
            (ExprKind::Def, Expr::Def(_)) => true,
            // (ExprKind::Def, Expr::Def(def)) => def.sig.is_subr(),
            _ => false,
        }
    }
}

/// This struct provides:
/// * namespace where the cursor is located (`get_namespace`)
/// * cursor(`Token`) -> `Expr` mapping (`get_min_expr`)
/// * cursor(`Token`) -> `VarInfo` mapping (`get_info`)
pub struct HIRVisitor<'a> {
    hir: MappedRwLockReadGuard<'a, HIR>,
    file_cache: &'a FileCache,
    uri: NormalizedUrl,
    strict_cmp: bool,
    search: ExprKind,
}

impl<'a> HIRVisitor<'a> {
    pub fn new(
        hir: MappedRwLockReadGuard<'a, HIR>,
        file_cache: &'a FileCache,
        uri: NormalizedUrl,
    ) -> Self {
        Self {
            hir,
            file_cache,
            uri,
            strict_cmp: ERG_MODE,
            search: ExprKind::Expr,
        }
    }

    pub fn new_searcher(
        hir: MappedRwLockReadGuard<'a, HIR>,
        file_cache: &'a FileCache,
        uri: NormalizedUrl,
        search: ExprKind,
    ) -> Self {
        Self {
            hir,
            file_cache,
            uri,
            strict_cmp: ERG_MODE,
            search,
        }
    }

    pub fn get_namespace(&self, pos: Position) -> Vec<Str> {
        let name = self
            .uri
            .path()
            .split('/')
            .last()
            .unwrap_or_default()
            .split('.')
            .next()
            .unwrap_or_default();
        let namespace = vec![Str::rc(name)];
        if let Some(ns) = self.get_exprs_ns(namespace.clone(), self.hir.module.iter(), pos) {
            ns
        } else {
            namespace
        }
    }

    fn is_new_final_line(&self, chunk: &Expr, pos: Position) -> bool {
        let ln_end = chunk.ln_end().unwrap_or(0);
        let line = self
            .file_cache
            .get_line(&self.uri, ln_end)
            .unwrap_or_default();
        let indent_len = line.len() - line.trim_start_matches(' ').len();
        let cond = ln_end == pos.line && pos.character as usize == indent_len + 1;
        cond && matches!(
            chunk,
            Expr::Call(_) | Expr::Lambda(_) | Expr::Def(_) | Expr::ClassDef(_)
        )
    }

    fn get_expr_ns(&self, cur_ns: Vec<Str>, chunk: &Expr, pos: Position) -> Option<Vec<Str>> {
        if !(util::pos_in_loc(chunk, pos) || self.is_new_final_line(chunk, pos)) {
            return None;
        }
        match chunk {
            Expr::Literal(_)
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
        self.get_exprs_ns(cur_ns, class_def.all_methods(), pos)
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
    pub fn get_min_expr(&self, pos: Position) -> Option<&Expr> {
        for chunk in self.hir.module.iter() {
            if let Some(expr) = self.get_expr(chunk, pos) {
                return Some(expr);
            }
        }
        None
    }

    fn return_expr_if_same<'e>(&'e self, expr: &'e Expr, l: &Token, r: Position) -> Option<&Expr> {
        if !self.search.matches(expr) {
            return None;
        }
        if l.loc().contains(r.loc()) {
            Some(expr)
        } else {
            None
        }
    }

    fn return_expr_if_contains<'e>(
        &'e self,
        expr: &'e Expr,
        pos: Position,
        loc: &impl Locational,
    ) -> Option<&Expr> {
        (loc.loc().contains(pos.loc()) && self.search.matches(expr)).then_some(expr)
    }

    fn get_expr<'e>(&'e self, expr: &'e Expr, pos: Position) -> Option<&'e Expr> {
        if expr.ln_end() < pos.ln_begin() || expr.ln_begin() > pos.ln_end() {
            return None;
        }
        match expr {
            Expr::Literal(lit) => self.return_expr_if_same(expr, &lit.token, pos),
            Expr::Accessor(acc) => self.get_expr_from_acc(expr, acc, pos),
            Expr::BinOp(bin) => self.get_expr_from_bin(expr, bin, pos),
            Expr::UnaryOp(unary) => self.get_expr(&unary.expr, pos),
            Expr::Call(call) => self.get_expr_from_call(expr, call, pos),
            Expr::ClassDef(class_def) => self.get_expr_from_class_def(expr, class_def, pos),
            Expr::Def(def) => self.get_expr_from_def(expr, def, pos),
            Expr::PatchDef(patch_def) => self.get_expr_from_patch_def(expr, patch_def, pos),
            Expr::Lambda(lambda) => self.get_expr_from_lambda(expr, lambda, pos),
            Expr::Array(arr) => self.get_expr_from_array(expr, arr, pos),
            Expr::Dict(dict) => self.get_expr_from_dict(expr, dict, pos),
            Expr::Record(record) => self.get_expr_from_record(expr, record, pos),
            Expr::Set(set) => self.get_expr_from_set(expr, set, pos),
            Expr::Tuple(tuple) => self.get_expr_from_tuple(expr, tuple, pos),
            Expr::TypeAsc(type_asc) => self.get_expr_from_type_asc(expr, type_asc, pos),
            Expr::Dummy(dummy) => self.get_expr_from_dummy(dummy, pos),
            Expr::Compound(block) | Expr::Code(block) => {
                self.get_expr_from_block(block.iter(), pos)
            }
            Expr::ReDef(redef) => self.get_expr_from_redef(expr, redef, pos),
            Expr::Import(_) => None,
        }
    }

    fn get_expr_from_acc<'e>(
        &'e self,
        expr: &'e Expr,
        acc: &'e Accessor,
        pos: Position,
    ) -> Option<&Expr> {
        match acc {
            Accessor::Ident(ident) => self.return_expr_if_same(expr, ident.raw.name.token(), pos),
            Accessor::Attr(attr) => self
                .return_expr_if_same(expr, attr.ident.raw.name.token(), pos)
                .or_else(|| self.get_expr(&attr.obj, pos)),
        }
    }

    fn get_expr_from_bin<'e>(
        &'e self,
        expr: &'e Expr,
        bin: &'e BinOp,
        pos: Position,
    ) -> Option<&Expr> {
        if bin.op.loc().contains(pos.loc()) && self.search.matches(expr) {
            return Some(expr);
        }
        self.get_expr(&bin.lhs, pos)
            .or_else(|| self.get_expr(&bin.rhs, pos))
    }

    fn get_expr_from_call<'e>(
        &'e self,
        expr: &'e Expr,
        call: &'e Call,
        pos: Position,
    ) -> Option<&Expr> {
        // args: `(1, 2)`, pos: `)`
        call.args
            .paren
            .as_ref()
            .and_then(|(_, end)| self.return_expr_if_same(expr, end, pos))
            .or_else(|| {
                call.attr_name
                    .as_ref()
                    .and_then(|attr| self.return_expr_if_same(expr, attr.raw.name.token(), pos))
            })
            .or_else(|| self.get_expr(&call.obj, pos))
            .or_else(|| self.get_expr_from_args(&call.args, pos))
            .or_else(|| self.return_expr_if_contains(expr, pos, call))
    }

    fn get_expr_from_args<'e>(&'e self, args: &'e Args, pos: Position) -> Option<&Expr> {
        for arg in args.pos_args.iter() {
            if let Some(expr) = self.get_expr(&arg.expr, pos) {
                return Some(expr);
            }
        }
        if let Some(var) = &args.var_args {
            if let Some(expr) = self.get_expr(&var.expr, pos) {
                return Some(expr);
            }
        }
        for arg in args.kw_args.iter() {
            if let Some(expr) = self.get_expr(&arg.expr, pos) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_from_def<'e>(
        &'e self,
        expr: &'e Expr,
        def: &'e Def,
        pos: Position,
    ) -> Option<&Expr> {
        self.return_expr_if_same(expr, def.sig.ident().raw.name.token(), pos)
            .or_else(|| self.get_expr_from_block(def.body.block.iter(), pos))
            .or_else(|| self.return_expr_if_contains(expr, pos, def))
    }

    fn get_expr_from_class_def<'e>(
        &'e self,
        expr: &'e Expr,
        class_def: &'e ClassDef,
        pos: Position,
    ) -> Option<&Expr> {
        class_def
            .require_or_sup
            .as_ref()
            .and_then(|req_sup| self.get_expr(req_sup, pos))
            .or_else(|| self.get_expr_from_block(class_def.all_methods(), pos))
            .or_else(|| self.return_expr_if_contains(expr, pos, class_def))
    }

    fn get_expr_from_block<'e>(
        &'e self,
        block: impl Iterator<Item = &'e Expr>,
        pos: Position,
    ) -> Option<&Expr> {
        for chunk in block {
            if let Some(expr) = self.get_expr(chunk, pos) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_from_redef<'e>(
        &'e self,
        expr: &'e Expr,
        redef: &'e ReDef,
        pos: Position,
    ) -> Option<&Expr> {
        self.get_expr_from_acc(expr, &redef.attr, pos)
            .or_else(|| self.get_expr_from_block(redef.block.iter(), pos))
    }

    fn get_expr_from_dummy<'e>(&'e self, dummy: &'e Dummy, pos: Position) -> Option<&Expr> {
        for chunk in dummy.iter() {
            if let Some(expr) = self.get_expr(chunk, pos) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_from_patch_def<'e>(
        &'e self,
        expr: &'e Expr,
        patch_def: &'e PatchDef,
        pos: Position,
    ) -> Option<&Expr> {
        self.return_expr_if_same(expr, patch_def.sig.name().token(), pos)
            .or_else(|| self.get_expr(&patch_def.base, pos))
            .or_else(|| self.get_expr_from_block(patch_def.methods.iter(), pos))
            .or_else(|| self.return_expr_if_contains(expr, pos, patch_def))
    }

    fn get_expr_from_lambda<'e>(
        &'e self,
        expr: &'e Expr,
        lambda: &'e Lambda,
        pos: Position,
    ) -> Option<&Expr> {
        if util::pos_in_loc(&lambda.params, util::loc_to_pos(pos.loc())?)
            && self.search.matches(expr)
        {
            return Some(expr);
        }
        self.get_expr_from_block(lambda.body.iter(), pos)
    }

    fn get_expr_from_array<'e>(
        &'e self,
        expr: &'e Expr,
        arr: &'e Array,
        pos: Position,
    ) -> Option<&Expr> {
        if arr.ln_end() == pos.ln_end() && self.search.matches(expr) {
            // arr: `[1, 2]`, pos: `]`
            return Some(expr);
        }
        match arr {
            Array::Normal(arr) => self.get_expr_from_args(&arr.elems, pos),
            _ => None, // todo!(),
        }
    }

    fn get_expr_from_dict<'e>(
        &'e self,
        expr: &'e Expr,
        dict: &'e Dict,
        pos: Position,
    ) -> Option<&Expr> {
        if dict.ln_end() == pos.ln_end() && self.search.matches(expr) {
            // arr: `{...}`, pos: `}`
            return Some(expr);
        }
        match dict {
            Dict::Normal(dict) => {
                for kv in &dict.kvs {
                    if let Some(expr) = self
                        .get_expr(&kv.key, pos)
                        .or_else(|| self.get_expr(&kv.value, pos))
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
        pos: Position,
    ) -> Option<&Expr> {
        if record.ln_end() == pos.ln_end() && self.search.matches(expr) {
            // arr: `{...}`, pos: `}`
            return Some(expr);
        }
        for field in record.attrs.iter() {
            if let Some(expr) = self.get_expr_from_block(field.body.block.iter(), pos) {
                return Some(expr);
            }
        }
        None
    }

    fn get_expr_from_set<'e>(
        &'e self,
        expr: &'e Expr,
        set: &'e Set,
        pos: Position,
    ) -> Option<&Expr> {
        if set.ln_end() == pos.ln_end() && self.search.matches(expr) {
            // arr: `{...}`, pos: `}`
            return Some(expr);
        }
        match set {
            Set::Normal(set) => self.get_expr_from_args(&set.elems, pos),
            _ => None, // todo!(),
        }
    }

    fn get_expr_from_tuple<'e>(
        &'e self,
        expr: &'e Expr,
        tuple: &'e Tuple,
        pos: Position,
    ) -> Option<&Expr> {
        match tuple {
            Tuple::Normal(tuple) => {
                // arr: `(1, 2)`, pos: `)`
                tuple
                    .elems
                    .paren
                    .as_ref()
                    .and_then(|(_, end)| self.return_expr_if_same(expr, end, pos))
                    .or_else(|| self.get_expr_from_args(&tuple.elems, pos))
            } // _ => None, // todo!(),
        }
    }

    fn get_expr_from_type_asc<'e>(
        &'e self,
        expr: &'e Expr,
        type_asc: &'e TypeAscription,
        pos: Position,
    ) -> Option<&Expr> {
        self.get_expr(&type_asc.expr, pos)
            .or_else(|| self.get_expr(&type_asc.spec.expr, pos))
            .or_else(|| self.return_expr_if_contains(expr, pos, type_asc))
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
            Expr::Literal(_lit) => None,
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
            Expr::TypeAsc(type_asc) => self.get_tasc_info(type_asc, token),
            Expr::Dummy(dummy) => self.get_dummy_info(dummy, token),
            Expr::Compound(block) | Expr::Code(block) => self.get_block_info(block.iter(), token),
            Expr::ReDef(redef) => self.get_redef_info(redef, token),
            Expr::Import(_) => None,
        }
    }

    fn get_acc_info(&self, acc: &Accessor, token: &Token) -> Option<VarInfo> {
        match acc {
            Accessor::Ident(ident) => {
                self.return_var_info_if_same(ident, ident.raw.name.token(), token)
            }
            Accessor::Attr(attr) => self
                .return_var_info_if_same(&attr.ident, attr.ident.raw.name.token(), token)
                .or_else(|| self.get_expr_info(&attr.obj, token)),
        }
    }

    fn get_bin_info(&self, bin: &BinOp, token: &Token) -> Option<VarInfo> {
        self.get_expr_info(&bin.lhs, token)
            .or_else(|| self.get_expr_info(&bin.rhs, token))
    }

    fn get_call_info(&self, call: &Call, token: &Token) -> Option<VarInfo> {
        if let Some(attr) = &call.attr_name {
            if let Some(t) = self.return_var_info_if_same(attr, attr.raw.name.token(), token) {
                return Some(t);
            }
        }
        self.get_expr_info(&call.obj, token)
            .or_else(|| self.get_args_info(&call.args, token))
    }

    fn get_args_info(&self, args: &Args, token: &Token) -> Option<VarInfo> {
        for arg in args.pos_args.iter() {
            if let Some(vi) = self.get_expr_info(&arg.expr, token) {
                return Some(vi);
            }
        }
        if let Some(var) = &args.var_args {
            if let Some(vi) = self.get_expr_info(&var.expr, token) {
                return Some(vi);
            }
        }
        for arg in args.kw_args.iter() {
            if let Some(vi) = self.get_expr_info(&arg.expr, token) {
                return Some(vi);
            }
        }
        None
    }

    fn get_sig_info(&self, sig: &Signature, token: &Token) -> Option<VarInfo> {
        match sig {
            Signature::Var(var) => self
                .return_var_info_if_same(&var.ident, var.name().token(), token)
                .or_else(|| {
                    var.t_spec
                        .as_ref()
                        .and_then(|t_spec| self.get_expr_info(&t_spec.expr, token))
                }),
            Signature::Subr(subr) => self
                .return_var_info_if_same(&subr.ident, subr.name().token(), token)
                .or_else(|| self.get_params_info(&subr.params, token))
                .or_else(|| {
                    subr.return_t_spec
                        .as_ref()
                        .and_then(|t_spec| self.get_expr_info(&t_spec.expr, token))
                }),
        }
    }

    fn get_params_info(&self, params: &Params, token: &Token) -> Option<VarInfo> {
        for param in params.non_defaults.iter() {
            if let Some(vi) = param
                .t_spec_as_expr
                .as_ref()
                .and_then(|t_spec| self.get_expr_info(t_spec, token))
            {
                return Some(vi);
            } else if param.raw.pat.loc() == token.loc() {
                return Some(param.vi.clone());
            }
        }
        if let Some(var) = &params.var_params {
            if let Some(vi) = var
                .t_spec_as_expr
                .as_ref()
                .and_then(|t_spec| self.get_expr_info(t_spec, token))
            {
                return Some(vi);
            } else if var.raw.pat.loc() == token.loc() {
                return Some(var.vi.clone());
            }
        }
        for param in params.defaults.iter() {
            if let Some(vi) = self.get_expr_info(&param.default_val, token) {
                return Some(vi);
            } else if let Some(vi) = param
                .sig
                .t_spec_as_expr
                .as_ref()
                .and_then(|t_spec| self.get_expr_info(t_spec, token))
            {
                return Some(vi);
            } else if param.sig.raw.pat.loc() == token.loc() {
                return Some(param.sig.vi.clone());
            }
        }
        None
    }

    fn get_def_info(&self, def: &Def, token: &Token) -> Option<VarInfo> {
        self.get_sig_info(&def.sig, token)
            .or_else(|| self.get_block_info(def.body.block.iter(), token))
    }

    fn get_class_def_info(&self, class_def: &ClassDef, token: &Token) -> Option<VarInfo> {
        class_def
            .require_or_sup
            .as_ref()
            .and_then(|req_sup| self.get_expr_info(req_sup, token))
            .or_else(|| self.get_sig_info(&class_def.sig, token))
            .or_else(|| self.get_block_info(class_def.all_methods(), token))
    }

    fn get_patch_def_info(&self, patch_def: &PatchDef, token: &Token) -> Option<VarInfo> {
        self.get_expr_info(&patch_def.base, token)
            .or_else(|| self.get_sig_info(&patch_def.sig, token))
            .or_else(|| self.get_block_info(patch_def.methods.iter(), token))
    }

    fn get_block_info<'e>(
        &self,
        block: impl Iterator<Item = &'e Expr>,
        token: &Token,
    ) -> Option<VarInfo> {
        for chunk in block {
            if let Some(expr) = self.get_expr_info(chunk, token) {
                return Some(expr);
            }
        }
        None
    }

    fn get_redef_info(&self, redef: &ReDef, token: &Token) -> Option<VarInfo> {
        self.get_acc_info(&redef.attr, token)
            .or_else(|| self.get_block_info(redef.block.iter(), token))
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
            .or_else(|| self.get_block_info(lambda.body.iter(), token))
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

    fn get_tasc_info(&self, tasc: &TypeAscription, token: &Token) -> Option<VarInfo> {
        self.get_expr_info(&tasc.expr, token)
            .or_else(|| self.get_expr_info(&tasc.spec.expr, token))
    }
}
