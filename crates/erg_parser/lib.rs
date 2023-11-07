//! Implements `Parser` for Erg. `Parser` parses the source code to generate `AST`.
//! The generated `AST`s are guaranteed to be identical if the source code is identical.
//! However, identical `AST`s may be generated even if the source code is (a bit) different.
#![allow(clippy::large_enum_variant)]
extern crate erg_common;

pub mod ast;
pub mod build_ast;
pub mod convert;
pub mod desugar;
pub mod error;
pub mod lex;
pub mod parse;
pub mod token;
pub mod typespec;
pub mod visitor;

pub use parse::{Parser, ParserRunner};
pub use visitor::ASTVisitor;
