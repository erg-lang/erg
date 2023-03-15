//! defines the compiler for Erg (ergc).
#![allow(clippy::large_enum_variant)]
#![allow(clippy::result_large_err)]
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
pub mod link_ast;
pub mod link_hir;
pub mod lint;
pub mod lower;
pub mod module;
pub mod optimize;
pub mod ownercheck;
pub mod transpile;
pub mod ty;
pub mod varinfo;

pub use build_hir::HIRBuilder;
pub use erg_parser::build_ast::ASTBuilder;
pub use transpile::Transpiler;
