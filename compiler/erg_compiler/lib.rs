//! defines the compiler for Erg (ergc).
#![allow(clippy::large_enum_variant)]
extern crate erg_common;
pub extern crate erg_parser;

pub mod artifact;
pub mod build_hir;
mod compile;
pub use compile::*;
mod codegen;
pub mod context;
pub mod declare;
pub mod desugar_hir;
pub mod effectcheck;
pub mod error;
pub mod hir;
pub mod link;
pub mod linter;
pub mod lower;
pub mod module;
pub mod optimize;
pub mod ownercheck;
pub mod reorder;
pub mod transpile;
pub mod ty;
pub mod varinfo;

pub use build_hir::HIRBuilder;
pub use erg_parser::build_ast::ASTBuilder;
pub use linter::Linter;
pub use transpile::Transpiler;
