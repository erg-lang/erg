use std::mem;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::error::Location;
use erg_common::log;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::Str;
use Visibility::*;

use erg_type::{HasType, Ownership};

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

#[derive(Debug)]
pub struct OwnershipChecker {
    cfg: ErgConfig,
    path_stack: Vec<(Str, Visibility)>,
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
        self.path_stack
            .iter()
            .fold(String::new(), |acc, (path, vis)| {
                if vis.is_public() {
                    acc + "." + &path[..]
                } else {
                    acc + "::" + &path[..]
                }
            })
    }

    // moveされた後の変数が使用されていないかチェックする
    // ProceduralでないメソッドでRefMutが使われているかはSideEffectCheckerでチェックする
    pub fn check(&mut self, hir: HIR) -> Result<HIR, (HIR, OwnershipErrors)> {
        log!(info "the ownership checking process has started.{RESET}");
        if self.full_path() != ("::".to_string() + &hir.name[..]) {
            self.path_stack.push((hir.name.clone(), Private));
            self.dict
                .insert(Str::from(self.full_path()), LocalVars::default());
        }
        for chunk in hir.module.iter() {
            self.check_expr(chunk, Ownership::Owned, true);
        }
        log!(
            "{GREEN}[DEBUG] the ownership checking process has completed, found errors: {}{RESET}",
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
                self.path_stack.push((name, def.sig.vis()));
                self.dict
                    .insert(Str::from(self.full_path()), LocalVars::default());
                self.check_block(&def.body.block);
                self.path_stack.pop();
            }
            // Access in chunks does not drop variables (e.g., access in REPL)
            Expr::Accessor(acc) => self.check_acc(acc, ownership, chunk),
            // TODO: referenced
            Expr::Call(call) => {
                let args_owns = call.signature_t().unwrap().args_ownership();
                let non_defaults_len = if call.method_name.is_some() {
                    args_owns.non_defaults.len() - 1
                } else {
                    args_owns.non_defaults.len()
                };
                let (non_default_args, var_args) = call.args.pos_args.split_at(non_defaults_len);
                for (nd_arg, ownership) in
                    non_default_args.iter().zip(args_owns.non_defaults.iter())
                {
                    self.check_expr(&nd_arg.expr, *ownership, false);
                }
                if let Some(ownership) = args_owns.var_params.as_ref() {
                    for var_arg in var_args.iter() {
                        self.check_expr(&var_arg.expr, *ownership, false);
                    }
                } else {
                    let kw_args = var_args;
                    for (arg, (_, ownership)) in kw_args.iter().zip(args_owns.defaults.iter()) {
                        self.check_expr(&arg.expr, *ownership, false);
                    }
                }
                for kw_arg in call.args.kw_args.iter() {
                    if let Some((_, ownership)) = args_owns
                        .defaults
                        .iter()
                        .find(|(k, _)| k == kw_arg.keyword.inspect())
                    {
                        self.check_expr(&kw_arg.expr, *ownership, false);
                    } else {
                        todo!()
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
                    for a in dic.attrs.kw_args.iter() {
                        // self.check_expr(&a.key);
                        self.check_expr(&a.expr, ownership, false);
                    }
                }
                _ => todo!(),
            },
            Expr::Record(rec) => {
                for def in rec.attrs.iter() {
                    for chunk in def.body.block.iter() {
                        self.check_expr(chunk, ownership, false);
                    }
                }
            }
            // TODO: capturing
            Expr::Lambda(lambda) => {
                let name_and_vis = (Str::from(format!("<lambda_{}>", lambda.id)), Private);
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
                if let Err(e) = self.check_if_dropped(ident.inspect(), ident.loc()) {
                    self.errs.push(e);
                    return;
                }
                if acc.ref_t().is_mut() && ownership.is_owned() && !chunk {
                    self.drop(ident);
                }
            }
            Accessor::Attr(attr) => {
                // REVIEW: is ownership the same?
                self.check_expr(&attr.obj, ownership, false)
            }
            Accessor::TupleAttr(t_attr) => self.check_expr(&t_attr.obj, ownership, false),
            Accessor::Subscr(subscr) => {
                self.check_expr(&subscr.obj, ownership, false);
                self.check_expr(&subscr.index, ownership, false);
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
            |acc, (path, vis)| {
                if vis.is_public() {
                    acc + "." + &path[..]
                } else {
                    acc + "::" + &path[..]
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

    fn check_if_dropped(&mut self, name: &Str, loc: Location) -> Result<(), OwnershipError> {
        for n in 0..self.path_stack.len() {
            if let Some(moved_loc) = self.nth_outer_scope(n).dropped_vars.get(name) {
                let moved_loc = *moved_loc;
                return Err(OwnershipError::move_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    name,
                    loc,
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
