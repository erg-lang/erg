//! Implements `Parser` for Erg. `Parser` parses the source code to generate `AST`,
//! and performs type checking and other optimizations if necessary.
extern crate common;

pub mod desugar;
pub mod error;
pub mod ast;
pub mod lex;
pub mod parse;
pub mod token;

pub use parse::{Parser, ParserRunner};
