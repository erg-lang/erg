use common::Str;
use common::{debug_power_assert, log};
use common::color::{GREEN, RESET};
use common::dict::Dict;
use common::error::Location;
use common::set::Set;
use common::traits::{Stream, Locational, HasType};
use common::ty::{Type, ArgsOwnership, Ownership};

use crate::error::{OwnershipError, OwnershipErrors, OwnershipResult};
use crate::hir::{HIR, Def, Signature, Accessor, Block, Expr};

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
    current_scope_name: Str,
    dict: Dict<Str, LocalVars>,
    errs: OwnershipErrors,
}

impl OwnershipChecker {
    pub fn new() -> Self {
        OwnershipChecker {
            current_scope_name: Str::ever("<dummy>"),
            dict: Dict::new(),
            errs: OwnershipErrors::empty(),
        }
    }

    // moveされた後の変数が使用されていないかチェックする
    // ProceduralでないメソッドでRefMutが使われているかはSideEffectCheckerでチェックする
    pub fn check(mut self, hir: HIR) -> OwnershipResult<HIR> {
        log!("{GREEN}[DEBUG] the ownership checking process has started.{RESET}");
        self.current_scope_name = hir.name.clone();
        self.dict.insert(hir.name.clone(), LocalVars::default());
        for chunk in hir.module.iter() {
            self.check_expr(chunk, Ownership::Owned);
        }
        log!("{GREEN}[DEBUG] the ownership checking process has completed, found errors: {}{RESET}", self.errs.len());
        if self.errs.is_empty() {
            Ok(hir)
        } else {
            Err(self.errs)
        }
    }

    fn check_block(&mut self, block: &Block) {
        for chunk in block.iter() {
            self.check_expr(chunk, Ownership::Owned);
        }
    }

    // 参照を取るMethod Call中ならばreferenced: trueとなり、所有権は移動しない
    fn check_expr(&mut self, expr: &Expr, ownership: Ownership) {
        match expr {
            Expr::Def(def) => {
                self.define(&def);
                self.check_block(&def.body.block);
            },
            Expr::Accessor(Accessor::Local(local)) => {
                // TODO: スコープを再帰的にチェックする
                if let Some(moved_loc) = self.current_scope().dropped_vars.get(local.inspect()) {
                    let moved_loc = *moved_loc;
                    self.errs.push(OwnershipError::move_error(
                        local.inspect(),
                        local.loc(),
                        moved_loc,
                        Str::from("<dummy>"),
                    ));
                } else {
                    if expr.ref_t().is_mut() && ownership.is_owned() {
                        log!("dropped: {}", local.inspect());
                        self.drop(local.inspect(), expr.loc());
                    }
                }
            },
            Expr::Accessor(Accessor::Attr(a)) => {
                if a.ref_t() != &Type::ASTOmitted { todo!("{a}: {}", a.ref_t()) }
            },
            Expr::Accessor(_a) => todo!(),
            // TODO: referenced
            Expr::Call(call) => {
                self.check_expr(&call.obj, ownership);
                let args_ownership = call.signature_t().unwrap().args_ownership();
                match args_ownership {
                    ArgsOwnership::Args{ self_, non_defaults, defaults } => {
                        if let Some(ownership) = self_ {
                            self.check_expr(&call.obj, ownership);
                        }
                        let (nd_ownerships, d_ownerships): (Vec<_>, Vec<_>) = non_defaults.iter()
                            .enumerate()
                            .partition(|(i, _)| *i == call.args.pos_args().len());
                        for (parg, (_, ownership)) in call.args.pos_args()
                            .iter()
                            .zip(nd_ownerships.into_iter()) {
                                self.check_expr(&parg.expr, *ownership);
                        }
                        for (kwarg, (_, ownership)) in call.args.kw_args()
                            .iter()
                            .zip(d_ownerships.into_iter().chain(defaults.iter().enumerate())) {
                                self.check_expr(&kwarg.expr, *ownership);
                        }
                    },
                    ArgsOwnership::VarArgs(ownership) => {
                        for parg in call.args.pos_args().iter() {
                            self.check_expr(&parg.expr, ownership);
                        }
                        for kwarg in call.args.kw_args().iter() {
                            self.check_expr(&kwarg.expr, ownership);
                        }
                    },
                    other => todo!("{other:?}"),
                }
            },
            // TODO: referenced
            Expr::BinOp(binop) => {
                self.check_expr(&binop.lhs, ownership);
                self.check_expr(&binop.rhs, ownership);
            },
            Expr::UnaryOp(unary) => {
                self.check_expr(&unary.expr, ownership);
            },
            Expr::Array(arr) => {
                for a in arr.elems.pos_args().iter() {
                    self.check_expr(&a.expr, ownership);
                }
            },
            Expr::Dict(dict) => {
                for a in dict.attrs.kw_args().iter() {
                    // self.check_expr(&a.key);
                    self.check_expr(&a.expr, ownership);
                }
            },
            // TODO: capturing
            Expr::Lambda(lambda) => {
                self.check_block(&lambda.body);
            },
            _ => {},
        }
    }

    /// TODO: このメソッドを呼ぶとき、スコープを再帰的に検索する
    #[inline]
    fn current_scope(&mut self) -> &mut LocalVars {
        self.dict.get_mut(&self.current_scope_name).unwrap()
    }

    fn define(&mut self, def: &Def) {
        match &def.sig {
            Signature::Var(sig) => {
                for name in sig.pat.inspects() {
                    self.current_scope().alive_vars.insert(name.clone());
                }
            },
            Signature::Subr(sig) => {
                self.current_scope().alive_vars.insert(sig.name.inspect().clone());
            },
        }
    }

    fn drop(&mut self, name: &Str, moved_loc: Location) {
        debug_power_assert!(self.current_scope().alive_vars.remove(name), ==, true);
        self.current_scope().dropped_vars.insert(name.clone(), moved_loc);
    }
}
