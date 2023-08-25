use std::cmp::Ordering::*;
use std::fmt;
use std::ops::{Deref, DerefMut};

use erg_common::traits::Stream;
use erg_compiler::erg_parser::ast;
use erg_compiler::erg_parser::ast::Module;
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

impl fmt::Display for ASTDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deletion(idx) => write!(f, "Deletion({idx})"),
            Self::Addition(idx, expr) => write!(f, "Addition({idx}, {expr})"),
            Self::Modification(idx, expr) => write!(f, "Modification({idx}, {expr})"),
            Self::Nop => write!(f, "Nop"),
        }
    }
}

/// diff(old: {x, y, z}, new: {x, a, y, z}) => ASTDiff::Addition(1)
/// diff(old: {x, y}, new: {x, y, a}) => ASTDiff::Addition(2)
/// diff(old: {x, y}, new: {x}) => ASTDiff::Deletion(1)
/// diff(old: {x, y, z}, new: {x, z}) => ASTDiff::Deletion(1)
/// diff(old: {x, y, z}, new: {x, ya, z}) => ASTDiff::Modification(1)
/// diff(old: {x, y, z}, new: {x, a, z}) => ASTDiff::Modification(1)
/// diff(old: {x, y, z}, new: {x, y, z}) => ASTDiff::Nop
impl ASTDiff {
    pub fn diff<M1: Deref<Target = Module>, M2: Deref<Target = Module>>(
        old: M1,
        new: M2,
    ) -> ASTDiff {
        match old.len().cmp(&new.len()) {
            Less => {
                let idx = new
                    .iter()
                    .zip(old.iter())
                    .position(|(new, old)| new != old)
                    .unwrap_or(new.len() - 1);
                Self::Addition(idx, new.get(idx).unwrap().clone())
            }
            Greater => Self::Deletion(
                old.iter()
                    .zip(new.iter())
                    .position(|(old, new)| old != new)
                    .unwrap_or(old.len() - 1),
            ),
            Equal => old
                .iter()
                .zip(new.iter())
                .position(|(old, new)| old != new)
                .map(|idx| Self::Modification(idx, new.get(idx).unwrap().clone()))
                .unwrap_or(Self::Nop),
        }
    }

    pub const fn is_nop(&self) -> bool {
        matches!(self, Self::Nop)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HIRDiff {
    Deletion(usize),
    Addition(usize, hir::Expr),
    Modification(usize, hir::Expr),
    Nop,
}

impl fmt::Display for HIRDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deletion(idx) => write!(f, "Deletion({idx})"),
            Self::Addition(idx, expr) => write!(f, "Addition({idx}, {expr})"),
            Self::Modification(idx, expr) => write!(f, "Modification({idx}, {expr})"),
            Self::Nop => write!(f, "Nop"),
        }
    }
}

impl HIRDiff {
    pub fn new(diff: ASTDiff, lowerer: &mut ASTLowerer) -> Option<Self> {
        match diff {
            ASTDiff::Deletion(idx) => Some(Self::Deletion(idx)),
            ASTDiff::Addition(idx, expr) => {
                let expr = lowerer
                    .lower_chunk(expr)
                    .map_err(|err| {
                        crate::_log!("err: {err}");
                    })
                    .ok()?;
                Some(Self::Addition(idx, expr))
            }
            ASTDiff::Modification(idx, expr) => {
                if let ast::Expr::Def(def)
                | ast::Expr::ClassDef(ast::ClassDef { def, .. })
                | ast::Expr::PatchDef(ast::PatchDef { def, .. }) = &expr
                {
                    if let Some(name) = def.sig.name_as_str() {
                        lowerer.unregister(name);
                    }
                }
                let expr = lowerer
                    .lower_chunk(expr)
                    .map_err(|err| {
                        crate::_log!("err: {err}");
                    })
                    .ok()?;
                Some(Self::Modification(idx, expr))
            }
            ASTDiff::Nop => Some(Self::Nop),
        }
    }

    pub fn update<H: DerefMut<Target = HIR>>(self, mut old: H) {
        match self {
            Self::Addition(idx, expr) => {
                if idx > old.module.len() {
                    old.module.push(expr);
                } else {
                    old.module.insert(idx, expr);
                }
            }
            Self::Deletion(usize) => {
                if old.module.get(usize).is_some() {
                    old.module.remove(usize);
                }
            }
            Self::Modification(idx, expr) => {
                if let Some(old_expr) = old.module.get_mut(idx) {
                    *old_expr = expr;
                }
            }
            Self::Nop => {}
        }
    }
}
