use erg_common::log;
use erg_common::traits::Stream;

use crate::hir::{Accessor, AttrDef, Block, Expr, HIR};

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
            let static_members = match chunk {
                Expr::ClassDef(class_def) => {
                    let class = Expr::Accessor(Accessor::Ident(class_def.sig.ident().clone()));
                    let methods = std::mem::take(class_def.methods.ref_mut_payload());
                    let (methods, static_members): (Vec<_>, Vec<_>) = methods
                        .into_iter()
                        .partition(|attr| matches!(attr, Expr::Def(def) if def.sig.is_subr()));
                    class_def.methods.extend(methods);
                    static_members
                        .into_iter()
                        .map(|expr| match expr {
                            Expr::Def(def) => {
                                let acc = class.clone().attr(def.sig.into_ident());
                                let attr_def = AttrDef::new(acc, def.body.block);
                                Expr::AttrDef(attr_def)
                            }
                            _ => expr,
                        })
                        .collect()
                }
                _ => vec![],
            };
            if !static_members.is_empty() {
                *chunk = Expr::Compound(Block::new(
                    [vec![std::mem::take(chunk)], static_members].concat(),
                ));
            }
        }
        hir
    }
}
