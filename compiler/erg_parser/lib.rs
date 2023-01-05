//! Implements `Parser` for Erg. `Parser` parses the source code to generate `AST`,
//! and performs type checking and other optimizations if necessary.
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

pub use parse::{Parser, ParserRunner};
