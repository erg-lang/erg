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

#[cfg(feature = "pylib")]
use pyo3::prelude::*;

/// parse(code: str) -> erg_parser.Module
/// --
///
/// parse an Erg code as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "parse")]
fn _parse(code: String) -> Result<ast::Module, error::ParseErrors> {
    parse::SimpleParser::parse(code)
        .map(|art| art.ast)
        .map_err(|iart| iart.errors)
}

#[cfg(feature = "pylib_parser")]
#[pymodule]
fn erg_parser(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_parse, m)?)?;
    let expr = PyModule::new(py, "expr")?;
    expr.add_class::<ast::Literal>()?;
    expr.add_class::<ast::NormalArray>()?;
    expr.add_class::<ast::NormalTuple>()?;
    expr.add_class::<ast::NormalDict>()?;
    expr.add_class::<ast::NormalSet>()?;
    expr.add_class::<ast::NormalRecord>()?;
    expr.add_class::<ast::BinOp>()?;
    expr.add_class::<ast::UnaryOp>()?;
    expr.add_class::<ast::Call>()?;
    expr.add_class::<ast::DataPack>()?;
    expr.add_class::<ast::Lambda>()?;
    expr.add_class::<ast::TypeAscription>()?;
    expr.add_class::<ast::Def>()?;
    expr.add_class::<ast::Methods>()?;
    expr.add_class::<ast::ClassDef>()?;
    expr.add_class::<ast::PatchDef>()?;
    expr.add_class::<ast::ReDef>()?;
    expr.add_class::<ast::Compound>()?;
    expr.add_class::<ast::InlineModule>()?;
    expr.add_class::<ast::Dummy>()?;
    m.add_submodule(expr)?;

    let ast = PyModule::new(py, "ast")?;
    ast.add_class::<token::Token>()?;
    ast.add_class::<token::TokenKind>()?;
    ast.add_class::<ast::Literal>()?;
    ast.add_class::<ast::Identifier>()?;
    ast.add_class::<ast::Attribute>()?;
    ast.add_class::<ast::TupleAttribute>()?;
    ast.add_class::<ast::Subscript>()?;
    ast.add_class::<ast::TypeApp>()?;
    ast.add_class::<ast::NormalArray>()?;
    ast.add_class::<ast::NormalTuple>()?;
    ast.add_class::<ast::NormalDict>()?;
    ast.add_class::<ast::NormalSet>()?;
    ast.add_class::<ast::NormalRecord>()?;
    ast.add_class::<ast::BinOp>()?;
    ast.add_class::<ast::UnaryOp>()?;
    ast.add_class::<ast::Call>()?;
    ast.add_class::<ast::Args>()?;
    ast.add_class::<ast::Block>()?;
    ast.add_class::<ast::DataPack>()?;
    ast.add_class::<ast::Lambda>()?;
    ast.add_class::<ast::TypeAscription>()?;
    ast.add_class::<ast::Def>()?;
    ast.add_class::<ast::Methods>()?;
    ast.add_class::<ast::ClassDef>()?;
    ast.add_class::<ast::PatchDef>()?;
    ast.add_class::<ast::ReDef>()?;
    ast.add_class::<ast::Compound>()?;
    ast.add_class::<ast::InlineModule>()?;
    ast.add_class::<ast::Dummy>()?;
    ast.add_class::<ast::Module>()?;
    ast.add_class::<ast::AST>()?;
    m.add_submodule(ast)?;
    Ok(())
}
