use crate::erg_common::traits::Stream;

use crate::ast::{ClassDef, Expr, AST};

pub struct ASTVisitor<'a> {
    pub ast: &'a AST,
}

impl<'a> ASTVisitor<'a> {
    pub fn new(ast: &'a AST) -> Self {
        Self { ast }
    }

    pub fn get_var_value(&self, var_name: &str) -> Option<&'a Expr> {
        for chunk in self.ast.module.iter() {
            if let Some(expr) = Self::get_var_value_of_expr(chunk, var_name) {
                return Some(expr);
            }
        }
        None
    }

    fn get_var_value_of_expr(chunk: &'a Expr, var_name: &str) -> Option<&'a Expr> {
        match chunk {
            Expr::Def(def) if def.is_var() => {
                if def
                    .sig
                    .ident()
                    .is_some_and(|ident| ident.inspect() == var_name)
                {
                    return def.body.block.last();
                }
            }
            Expr::Dummy(chunks) => {
                for chunk in chunks.iter() {
                    if let Some(expr) = Self::get_var_value_of_expr(chunk, var_name) {
                        return Some(expr);
                    }
                }
            }
            Expr::Compound(chunks) => {
                for chunk in chunks.iter() {
                    if let Some(expr) = Self::get_var_value_of_expr(chunk, var_name) {
                        return Some(expr);
                    }
                }
            }
            _ => {}
        }
        None
    }

    pub fn get_classes(&self) -> impl Iterator<Item = &'a ClassDef> {
        self.ast.module.iter().flat_map(Self::get_classes_of_expr)
    }

    fn get_classes_of_expr(chunk: &'a Expr) -> Vec<&'a ClassDef> {
        match chunk {
            Expr::ClassDef(class) => vec![class],
            Expr::Dummy(chunks) => chunks.iter().flat_map(Self::get_classes_of_expr).collect(),
            Expr::Compound(chunks) => chunks.iter().flat_map(Self::get_classes_of_expr).collect(),
            _ => vec![],
        }
    }
}
