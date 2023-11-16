//! defines the compiler for Erg (ergc).
#![allow(clippy::large_enum_variant)]
#![allow(clippy::result_large_err)]
extern crate erg_common;
pub extern crate erg_parser;

pub mod artifact;
pub mod build_hir;
mod compile;
pub use compile::*;
pub mod build_package;
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

pub use build_hir::{GenericHIRBuilder, HIRBuilder};
pub use erg_parser::build_ast::ASTBuilder;
pub use transpile::Transpiler;

#[cfg(feature = "pylib")]
use pyo3::prelude::*;

/// compile(code: str) -> code
/// --
///
/// compile an Erg code as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile")]
fn _compile(py: Python<'_>, code: String) -> Result<PyObject, error::CompileErrors> {
    use erg_common::config::ErgConfig;
    use pyo3::types::{IntoPyDict, PyBytes};
    let cfg = ErgConfig::string(code);
    let mut compiler = Compiler::new(cfg);
    let code = compiler
        .compile_module()
        .map(|art| art.object)
        .map_err(|iart| iart.errors)?;
    let bytes = code.into_bytes(py.version().parse().unwrap());
    let dict = [("bytes", PyBytes::new(py, &bytes))].into_py_dict(py);
    py.run("import marshal", None, None).unwrap();
    Ok(py
        .eval("marshal.loads(bytes)", None, Some(dict))
        .unwrap()
        .into())
}

/// compile_ast(ast: erg_parser.AST) -> code
/// --
///
/// compile an Erg AST as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile_ast")]
fn _compile_ast(
    py: Python<'_>,
    ast: erg_parser::ast::AST,
) -> Result<PyObject, error::CompileErrors> {
    use erg_common::config::ErgConfig;
    use pyo3::types::{IntoPyDict, PyBytes};
    let cfg = ErgConfig::default();
    let mut compiler = Compiler::new(cfg);
    let code = compiler
        .compile_ast(ast, "exec")
        .map(|art| art.object)
        .map_err(|iart| iart.errors)?;
    let bytes = code.into_bytes(py.version().parse().unwrap());
    let dict = [("bytes", PyBytes::new(py, &bytes))].into_py_dict(py);
    py.run("import marshal", None, None).unwrap();
    Ok(py
        .eval("marshal.loads(bytes)", None, Some(dict))
        .unwrap()
        .into())
}

/// compile_file(path: str) -> code
/// --
///
/// compile an Erg file as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile_file")]
fn _compile_file(py: Python<'_>, path: String) -> Result<PyObject, error::CompileErrors> {
    let code = std::fs::read_to_string(path)?;
    _compile(py, code)
}

/// exec_module(code: str) -> module
/// --
///
/// compile and execute an Erg code as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "exec_module")]
fn _exec_module(py: Python<'_>, code: String) -> Result<PyObject, error::CompileErrors> {
    use pyo3::types::IntoPyDict;

    let code = _compile(py, code)?;
    let module = pyo3::types::PyModule::new(py, "<erg>").unwrap();
    let dic = [("code", code), ("dict", PyObject::from(module.dict()))].into_py_dict(py);
    py.run("exec(code, dict)", None, Some(dic)).unwrap();
    Ok(module.into())
}

/// __import__(name: str) -> module
/// --
///
/// import an Erg module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "__import__")]
fn _import(py: Python<'_>, name: String) -> Result<PyObject, error::CompileErrors> {
    let path = format!("{name}.er");
    let code = std::fs::read_to_string(path)?;
    _exec_module(py, code)
}

#[cfg(feature = "pylib")]
#[pymodule]
fn erg_compiler(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_compile, m)?)?;
    m.add_function(wrap_pyfunction!(_compile_file, m)?)?;
    m.add_function(wrap_pyfunction!(_exec_module, m)?)?;
    m.add_function(wrap_pyfunction!(_import, m)?)?;
    Ok(())
}
