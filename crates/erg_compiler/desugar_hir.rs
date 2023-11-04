use erg_common::log;
use erg_common::traits::Stream;

use crate::hir::{Accessor, Block, Expr, ReDef, HIR};

/// Desugares HIR to make it more like Python semantics.
pub struct HIRDesugarer {}

impl HIRDesugarer {
    pub fn desugar(hir: HIR) -> HIR {
        log!(info "HIR desugaring process has started.");
        let hir = Self::desugar_class_member(hir);
        log!(info "HIR desugaring process has completed.");
        hir
    }

    /// ```erg
    /// C = Class ...
    /// C.
    ///     _Self = C
    ///     a = C.x
    ///     x = 1
    ///     m(self) = ...
    /// ```
    /// ↓
    /// ```python
    /// class C:
    ///     def m(self): ...
    /// C._Self = C
    /// C.a = C.x
    /// C.x = 1
    /// ```
    fn desugar_class_member(mut hir: HIR) -> HIR {
        for chunk in hir.module.iter_mut() {
            Self::desugar_class_member_expr(chunk);
        }
        hir
    }

    fn desugar_class_member_expr(chunk: &mut Expr) {
        match chunk {
            Expr::ClassDef(class_def) => {
                let class = Expr::Accessor(Accessor::Ident(class_def.sig.ident().clone()));
                let mut static_members = vec![];
                for methods_ in class_def.methods_list.iter_mut() {
                    let block = std::mem::take(&mut methods_.defs);
                    let (methods, statics): (Vec<_>, Vec<_>) = block
                        .into_iter()
                        .partition(|attr| matches!(attr, Expr::Def(def) if def.sig.is_subr()));
                    methods_.defs.extend(methods);
                    static_members.extend(statics.into_iter().map(|expr| match expr {
                        Expr::Def(def) => {
                            let acc = class.clone().attr(def.sig.into_ident());
                            let redef = ReDef::new(acc, def.body.block);
                            Expr::ReDef(redef)
                        }
                        _ => expr,
                    }));
                }
                if !static_members.is_empty() {
                    *chunk = Expr::Compound(Block::new(
                        [vec![std::mem::take(chunk)], static_members].concat(),
                    ));
                }
            }
            Expr::Code(block) | Expr::Compound(block) => {
                for expr in block.iter_mut() {
                    Self::desugar_class_member_expr(expr);
                }
            }
            Expr::Def(def) => {
                for chunk in def.body.block.iter_mut() {
                    Self::desugar_class_member_expr(chunk);
                }
            }
            // `HIRLinker` binds the modules and embed as the argument for the `exec` function call
            Expr::Call(call) => {
                call.args.pos_args.iter_mut().for_each(|arg| {
                    Self::desugar_class_member_expr(&mut arg.expr);
                });
            }
            _ => {}
        };
    }
}
