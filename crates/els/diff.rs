use std::cmp::Ordering::*;

use erg_common::traits::Stream;
use erg_compiler::erg_parser::ast;
use erg_compiler::erg_parser::ast::AST;
use erg_compiler::hir;
use erg_compiler::hir::HIR;
use erg_compiler::lower::ASTLowerer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ASTDiff {
    Deletion(usize),
    Addition(usize, ast::Expr),
    Modification(usize, ast::Expr),
    Nop,
}

/// diff(old: {x, y, z}, new: {x, a, y, z}) => ASTDiff::Addition(1)
/// diff(old: {x, y}, new: {x, y, a}) => ASTDiff::Addition(2)
/// diff(old: {x, y}, new: {x}) => ASTDiff::Deletion(1)
/// diff(old: {x, y, z}, new: {x, z}) => ASTDiff::Deletion(1)
/// diff(old: {x, y, z}, new: {x, ya, z}) => ASTDiff::Modification(1)
/// diff(old: {x, y, z}, new: {x, a, z}) => ASTDiff::Modification(1)
/// diff(old: {x, y, z}, new: {x, y, z}) => ASTDiff::Nop
impl ASTDiff {
    pub fn diff(old: AST, new: AST) -> ASTDiff {
        match old.module.len().cmp(&new.module.len()) {
            Less => {
                let idx = new
                    .module
                    .iter()
                    .zip(old.module.iter())
                    .position(|(new, old)| new != old)
                    .unwrap();
                Self::Addition(idx, new.module.get(idx).unwrap().clone())
            }
            Greater => Self::Deletion(
                old.module
                    .iter()
                    .zip(new.module.iter())
                    .position(|(old, new)| old != new)
                    .unwrap(),
            ),
            Equal => old
                .module
                .iter()
                .zip(new.module.iter())
                .position(|(old, new)| old != new)
                .map(|idx| Self::Modification(idx, new.module.get(idx).unwrap().clone()))
                .unwrap_or(Self::Nop),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HIRDiff {
    Deletion(usize),
    Addition(usize, hir::Expr),
    Modification(usize, hir::Expr),
    Nop,
}

impl HIRDiff {
    pub fn new(diff: ASTDiff, lowerer: &mut ASTLowerer) -> Option<Self> {
        match diff {
            ASTDiff::Deletion(idx) => Some(Self::Deletion(idx)),
            ASTDiff::Addition(idx, expr) => {
                let expr = lowerer.lower_expr(expr).ok()?;
                Some(Self::Addition(idx, expr))
            }
            ASTDiff::Modification(idx, expr) => {
                let expr = lowerer.lower_expr(expr).ok()?;
                Some(Self::Modification(idx, expr))
            }
            ASTDiff::Nop => Some(Self::Nop),
        }
    }

    pub fn update(self, old: &mut HIR) {
        match self {
            Self::Addition(idx, expr) => {
                old.module.insert(idx, expr);
            }
            Self::Deletion(usize) => {
                old.module.remove(usize);
            }
            Self::Modification(idx, expr) => {
                *old.module.get_mut(idx).unwrap() = expr;
            }
            Self::Nop => {}
        }
    }
}
