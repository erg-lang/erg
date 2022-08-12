//! Implements `Parser` for Erg. `Parser` parses the source code to generate `AST`,
//! and performs type checking and other optimizations if necessary.
extern crate erg_common;

pub mod ast;
pub mod desugar;
pub mod error;
pub mod lex;
pub mod parse;
pub mod token;

pub use parse::{Parser, ParserRunner};
