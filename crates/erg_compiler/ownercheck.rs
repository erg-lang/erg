use std::mem;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::error::Location;
use erg_common::set::Set;
use erg_common::style::colors::DEBUG_MAIN;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{impl_display_from_debug, log};
use erg_parser::ast::{ParamPattern, VarName};

use crate::ty::{HasType, Ownership, Visibility};

use crate::error::{OwnershipError, OwnershipErrors};
use crate::hir::{self, Accessor, Array, Block, Def, Expr, Identifier, Signature, Tuple, HIR};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrapperKind {
    Ref,
    Rc,
    Box,
}

#[derive(Debug, Default)]
struct LocalVars {
    alive_vars: Set<Str>,
    dropped_vars: Dict<Str, Location>,
}

impl_display_from_debug!(LocalVars);

/// Check code ownership.
/// for example:
/// * Check if moved variables are not used again.
/// * Checks whether a mutable reference method is called in an immutable reference method.
#[derive(Debug)]
pub struct OwnershipChecker {
    cfg: ErgConfig,
    path_stack: Vec<Visibility>,
    dict: Dict<Str, LocalVars>,
    errs: OwnershipErrors,
}

impl OwnershipChecker {
    pub fn new(cfg: ErgConfig) -> Self {
        OwnershipChecker {
            cfg,
            path_stack: vec![],
            dict: Dict::new(),
            errs: OwnershipErrors::empty(),
        }
    }

    fn full_path(&self) -> String {
        self.path_stack.iter().fold(String::new(), |acc, vis| {
            if vis.is_public() {
                acc + "." + &vis.def_namespace[..]
            } else {
                acc + "::" + &vis.def_namespace[..]
            }
        })
    }

    // moveされた後の変数が使用されていないかチェックする
    // ProceduralでないメソッドでRefMutが使われているかはSideEffectCheckerでチェックする
    pub fn check(&mut self, hir: HIR) -> Result<HIR, (HIR, OwnershipErrors)> {
        log!(info "the ownership checking process has started.{RESET}");
        if self.full_path() != ("::".to_string() + &hir.name[..]) {
            self.path_stack.push(Visibility::private(hir.name.clone()));
            self.dict
                .insert(Str::from(self.full_path()), LocalVars::default());
        }
        for chunk in hir.module.iter() {
            self.check_expr(chunk, Ownership::Owned, true);
        }
        log!(
            "{DEBUG_MAIN}[DEBUG] the ownership checking process has completed, found errors: {}{RESET}",
            self.errs.len()
        );
        if self.errs.is_empty() {
            Ok(hir)
        } else {
            Err((hir, mem::take(&mut self.errs)))
        }
    }

    fn check_block(&mut self, block: &Block) {
        if block.len() == 1 {
            return self.check_expr(block.first().unwrap(), Ownership::Owned, false);
        }
        for chunk in block.iter() {
            self.check_expr(chunk, Ownership::Owned, true);
        }
    }

    fn check_expr(&mut self, expr: &Expr, ownership: Ownership, chunk: bool) {
        match expr {
            Expr::Def(def) => {
                self.define(def);
                let name = match &def.sig {
                    Signature::Var(var) => var.inspect().clone(),
                    Signature::Subr(subr) => subr.ident.inspect().clone(),
                };
                self.path_stack
                    .push(Visibility::new(def.sig.vis().clone(), name));
                self.dict
                    .insert(Str::from(self.full_path()), LocalVars::default());
                if let Signature::Subr(subr) = &def.sig {
                    let (nd_params, var_params, d_params, kw_var, _) =
                        subr.params.ref_deconstruct();
                    for param in nd_params {
                        if let ParamPattern::VarName(name) = &param.raw.pat {
                            self.define_param(name);
                        }
                    }
                    if let Some(var) = var_params {
                        if let ParamPattern::VarName(name) = &var.raw.pat {
                            self.define_param(name);
                        }
                    }
                    for param in d_params {
                        if let ParamPattern::VarName(name) = &param.sig.raw.pat {
                            self.define_param(name);
                        }
                    }
                    if let Some(kw_var) = kw_var {
                        if let ParamPattern::VarName(name) = &kw_var.raw.pat {
                            self.define_param(name);
                        }
                    }
                }
                self.check_block(&def.body.block);
                self.path_stack.pop();
            }
            Expr::ClassDef(class_def) => {
                if let Some(req_sup) = &class_def.require_or_sup {
                    self.check_expr(req_sup, Ownership::Owned, false);
                }
                for def in class_def.all_methods() {
                    self.check_expr(def, Ownership::Owned, true);
                }
            }
            Expr::PatchDef(patch_def) => {
                self.check_expr(&patch_def.base, Ownership::Owned, false);
                for def in patch_def.methods.iter() {
                    self.check_expr(def, Ownership::Owned, true);
                }
            }
            // Access in chunks does not drop variables (e.g., access in REPL)
            Expr::Accessor(acc) => self.check_acc(acc, ownership, chunk),
            // TODO: referenced
            Expr::Call(call) => {
                let sig_t = call.signature_t().unwrap();
                if !sig_t.is_subr() {
                    return;
                }
                let args_owns = sig_t.args_ownership();
                let non_defaults_len = if call.is_method_call() {
                    args_owns.non_defaults.len() - 1
                } else {
                    args_owns.non_defaults.len()
                };
                let defaults_len = args_owns.defaults.len();
                if call.args.pos_args.len() > non_defaults_len {
                    let (non_default_args, var_args) =
                        call.args.pos_args.split_at(non_defaults_len);
                    for (nd_arg, (_, ownership)) in
                        non_default_args.iter().zip(args_owns.non_defaults.iter())
                    {
                        self.check_expr(&nd_arg.expr, *ownership, false);
                    }
                    if let Some((_, ownership)) = args_owns.var_params.as_ref() {
                        for var_arg in var_args.iter() {
                            self.check_expr(&var_arg.expr, *ownership, false);
                        }
                    } else {
                        let kw_args = var_args;
                        for (arg, (_, ownership)) in kw_args.iter().zip(args_owns.defaults.iter()) {
                            self.check_expr(&arg.expr, *ownership, false);
                        }
                    }
                }
                if call.args.kw_args.len() > defaults_len {
                    let (default_args, kw_var_args) = call.args.kw_args.split_at(defaults_len);
                    for kw_arg in default_args.iter() {
                        if let Some((_, ownership)) = args_owns
                            .defaults
                            .iter()
                            .find(|(k, _)| k == kw_arg.keyword.inspect())
                        {
                            self.check_expr(&kw_arg.expr, *ownership, false);
                        } else if let Some((_, ownership)) = args_owns
                            .non_defaults
                            .iter()
                            .find(|(k, _)| k.as_ref() == Some(kw_arg.keyword.inspect()))
                        {
                            self.check_expr(&kw_arg.expr, *ownership, false);
                        } else {
                            todo!()
                        }
                    }
                    if let Some((_, ownership)) = args_owns.kw_var_params.as_ref() {
                        for var_arg in kw_var_args.iter() {
                            self.check_expr(&var_arg.expr, *ownership, false);
                        }
                    } else {
                        let kw_args = kw_var_args;
                        for (arg, (_, ownership)) in kw_args.iter().zip(args_owns.defaults.iter()) {
                            self.check_expr(&arg.expr, *ownership, false);
                        }
                    }
                }
            }
            // TODO: referenced
            Expr::BinOp(binop) => {
                self.check_expr(&binop.lhs, ownership, false);
                self.check_expr(&binop.rhs, ownership, false);
            }
            Expr::UnaryOp(unary) => {
                self.check_expr(&unary.expr, ownership, false);
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    for a in arr.elems.pos_args.iter() {
                        self.check_expr(&a.expr, ownership, false);
                    }
                }
                Array::WithLength(arr) => {
                    self.check_expr(&arr.elem, ownership, false);
                    if let Some(len) = &arr.len {
                        self.check_expr(len, ownership, false);
                    }
                }
                _ => todo!(),
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(arr) => {
                    for a in arr.elems.pos_args.iter() {
                        self.check_expr(&a.expr, ownership, false);
                    }
                }
            },
            Expr::Dict(dict) => match dict {
                hir::Dict::Normal(dic) => {
                    for kv in dic.kvs.iter() {
                        self.check_expr(&kv.key, ownership, false);
                        self.check_expr(&kv.value, ownership, false);
                    }
                }
                other => todo!("{other}"),
            },
            Expr::Record(rec) => {
                for def in rec.attrs.iter() {
                    for chunk in def.body.block.iter() {
                        self.check_expr(chunk, ownership, false);
                    }
                }
            }
            Expr::Set(set) => match set {
                hir::Set::Normal(st) => {
                    for a in st.elems.pos_args.iter() {
                        self.check_expr(&a.expr, ownership, false);
                    }
                }
                hir::Set::WithLength(st) => {
                    self.check_expr(&st.elem, ownership, false);
                    self.check_expr(&st.len, ownership, false);
                }
            },
            // TODO: capturing
            Expr::Lambda(lambda) => {
                let name_and_vis =
                    Visibility::private(Str::from(format!("<lambda_{}>", lambda.id)));
                self.path_stack.push(name_and_vis);
                self.dict
                    .insert(Str::from(self.full_path()), LocalVars::default());
                self.check_block(&lambda.body);
                self.path_stack.pop();
            }
            Expr::TypeAsc(asc) => {
                self.check_expr(&asc.expr, ownership, chunk);
            }
            _ => {}
        }
    }

    fn check_acc(&mut self, acc: &Accessor, ownership: Ownership, chunk: bool) {
        match acc {
            Accessor::Ident(ident) => {
                if let Err(e) = self.check_if_dropped(ident.inspect(), ident) {
                    self.errs.push(e);
                    return;
                }
                if acc.ref_t().is_mut_type() && ownership.is_owned() && !chunk {
                    self.drop(ident);
                }
            }
            Accessor::Attr(attr) => {
                // REVIEW: is ownership the same?
                self.check_expr(&attr.obj, ownership, false)
            }
        }
    }

    /// TODO: このメソッドを呼ぶとき、スコープを再帰的に検索する
    #[inline]
    fn current_scope(&mut self) -> &mut LocalVars {
        self.dict.get_mut(&self.full_path()[..]).unwrap()
    }

    #[inline]
    fn nth_outer_scope(&mut self, n: usize) -> &mut LocalVars {
        let path = self.path_stack.iter().take(self.path_stack.len() - n).fold(
            String::new(),
            |acc, vis| {
                if vis.is_public() {
                    acc + "." + &vis.def_namespace[..]
                } else {
                    acc + "::" + &vis.def_namespace[..]
                }
            },
        );
        self.dict.get_mut(&path[..]).unwrap()
    }

    fn define(&mut self, def: &Def) {
        log!(info "define: {}", def.sig);
        match &def.sig {
            Signature::Var(sig) => {
                self.current_scope()
                    .alive_vars
                    .insert(sig.inspect().clone());
            }
            Signature::Subr(sig) => {
                self.current_scope()
                    .alive_vars
                    .insert(sig.ident.inspect().clone());
            }
        }
    }

    fn define_param(&mut self, name: &VarName) {
        log!(info "define: {}", name);
        self.current_scope()
            .alive_vars
            .insert(name.inspect().clone());
    }

    fn drop(&mut self, ident: &Identifier) {
        log!("drop: {ident} (in {})", ident.ln_begin().unwrap_or(0));
        for n in 0..self.path_stack.len() {
            if self.nth_outer_scope(n).alive_vars.remove(ident.inspect()) {
                self.nth_outer_scope(n)
                    .dropped_vars
                    .insert(ident.inspect().clone(), ident.loc());
                return;
            }
        }
        panic!("variable not found: {ident}");
    }

    fn check_if_dropped(
        &mut self,
        name: &Str,
        loc: &impl Locational,
    ) -> Result<(), OwnershipError> {
        for n in 0..self.path_stack.len() {
            if let Some(moved_loc) = self.nth_outer_scope(n).dropped_vars.get(name) {
                let moved_loc = *moved_loc;
                return Err(OwnershipError::move_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    name,
                    loc.loc(),
                    moved_loc,
                    self.full_path(),
                ));
            }
        }
        Ok(())
    }
}

impl Default for OwnershipChecker {
    fn default() -> Self {
        Self::new(ErgConfig::default())
    }
}
