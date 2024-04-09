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

#[allow(unused)]
use erg_common::config::{ErgConfig, Package};
#[allow(unused)]
use erg_common::io::Input;
#[allow(unused)]
use erg_common::traits::Runnable;

pub use build_hir::{GenericHIRBuilder, HIRBuilder};
pub use erg_parser::build_ast::ASTBuilder;
pub use transpile::Transpiler;

#[cfg(feature = "pylib")]
use pyo3::prelude::*;
#[cfg(feature = "pylib")]
use pyo3::types::{IntoPyDict, PyBytes};

#[cfg(feature = "pylib")]
#[pyclass(unsendable)]
#[pyo3(name = "Compiler")]
#[derive(Debug)]
struct _Compiler {
    compiler: compile::Compiler,
}

#[cfg(feature = "pylib")]
#[pymethods]
impl _Compiler {
    #[new]
    fn new(deps: Vec<Package>, path: Option<String>) -> Self {
        let input = path.map_or(Input::repl(), |path| Input::file(path.into()));
        let cfg = ErgConfig {
            packages: erg_common::ArcArray::from(deps),
            input,
            ..ErgConfig::default()
        };
        Self {
            compiler: compile::Compiler::new(cfg),
        }
    }

    #[pyo3(name = "clear")]
    fn _clear(&mut self) {
        self.compiler.clear();
    }

    /// compile(code: str, mode: str) -> code
    /// --
    ///
    /// compile an Erg code as a module at runtime
    #[pyo3(name = "compile")]
    fn _compile(
        &mut self,
        py: Python<'_>,
        code: String,
        mode: &str,
    ) -> Result<PyObject, error::CompileErrors> {
        self.compiler.set_input(Input::str(code));
        let src = self.compiler.cfg_mut().input.read();
        let code = self
            .compiler
            .compile(src, mode)
            .map(|art| art.object)
            .map_err(|iart| iart.errors)?;
        let bytes = code.into_bytes(py.version().parse().unwrap());
        let dict = [("bytes", PyBytes::new(py, &bytes))].into_py_dict(py);
        py.run("import marshal", None, None).unwrap();
        let code = py.eval("marshal.loads(bytes)", None, Some(dict)).unwrap();
        Ok(code.into())
    }

    /// compile_file(path: str) -> code
    /// --
    ///
    /// compile an Erg file as a module at runtime
    #[pyo3(name = "compile_file")]
    fn _compile_file(
        &mut self,
        py: Python<'_>,
        path: String,
    ) -> Result<PyObject, error::CompileErrors> {
        let code =
            std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{err}, path: {path}"));
        self._compile(py, code, "exec")
    }

    /// compile_ast(ast: erg_parser.AST, mode: str) -> code
    /// --
    ///
    /// compile an Erg AST as a module at runtime with dependencies
    #[pyo3(name = "compile_ast")]
    fn _compile_ast(
        &mut self,
        py: Python<'_>,
        ast: erg_parser::ast::AST,
        mode: &str,
    ) -> Result<PyObject, error::CompileErrors> {
        let code = self
            .compiler
            .compile_ast(ast, mode)
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
}

/// compile_with_dependencies(code: str, mode: str, pkgs: list[Package]) -> code
/// --
///
/// compile an Erg code as a module at runtime with dependencies
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile_with_dependencies")]
fn _compile_with_dependencies(
    py: Python<'_>,
    code: String,
    mode: &str,
    pkgs: Vec<Package>,
    path: Option<String>,
) -> Result<PyObject, error::CompileErrors> {
    let mut compiler = _Compiler::new(pkgs, path);
    compiler._compile(py, code, mode)
}

/// compile(code: str, mode: str) -> code
/// --
///
/// compile an Erg code as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile")]
fn _compile(
    py: Python<'_>,
    code: String,
    mode: &str,
    path: Option<String>,
) -> Result<PyObject, error::CompileErrors> {
    _compile_with_dependencies(py, code, mode, vec![], path)
}

/// compile_ast_with_dependencies(ast: erg_parser.AST, mode: str, pkgs: list[Package]) -> code
/// --
///
/// compile an Erg AST as a module at runtime with dependencies
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile_ast_with_dependencies")]
fn _compile_ast_with_dependencies(
    py: Python<'_>,
    ast: erg_parser::ast::AST,
    mode: &str,
    pkgs: Vec<Package>,
    path: Option<String>,
) -> Result<PyObject, error::CompileErrors> {
    let mut compiler = _Compiler::new(pkgs, path);
    compiler._compile_ast(py, ast, mode)
}

/// compile_ast(ast: erg_parser.AST, mode: str) -> code
/// --
///
/// compile an Erg AST as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile_ast")]
fn _compile_ast(
    py: Python<'_>,
    ast: erg_parser::ast::AST,
    mode: &str,
    path: Option<String>,
) -> Result<PyObject, error::CompileErrors> {
    _compile_ast_with_dependencies(py, ast, mode, vec![], path)
}

/// compile_file_with_dependencies(path: str, pkgs: list[Package]) -> code
/// --
///
/// compile an Erg file as a module at runtime with dependencies
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile_file_with_dependencies")]
fn _compile_file_with_dependencies(
    py: Python<'_>,
    path: String,
    pkgs: Vec<Package>,
) -> Result<PyObject, error::CompileErrors> {
    let code = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{err}, path: {path}"));
    _compile_with_dependencies(py, code, "exec", pkgs, Some(path))
}

/// compile_file(path: str) -> code
/// --
///
/// compile an Erg file as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "compile_file")]
fn _compile_file(py: Python<'_>, path: String) -> Result<PyObject, error::CompileErrors> {
    _compile_file_with_dependencies(py, path, vec![])
}

/// exec_with_dependencies(code: str, pkgs: list[Package]) -> module
/// --
///
/// compile and execute an Erg code as a module at runtime with dependencies
///
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "exec_with_dependencies")]
fn _exec_with_dependencies(
    py: Python<'_>,
    code: String,
    pkgs: Vec<Package>,
    path: Option<String>,
) -> Result<PyObject, error::CompileErrors> {
    let code = _compile_with_dependencies(py, code, "exec", pkgs, path)?;
    let module = pyo3::types::PyModule::new(py, "<erg>").unwrap();
    let dic = [("code", code), ("dict", PyObject::from(module.dict()))].into_py_dict(py);
    py.run("exec(code, dict)", None, Some(dic)).unwrap();
    Ok(module.into())
}

/// exec(code: str) -> module
/// --
///
/// compile and execute an Erg code as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "exec")]
fn _exec(
    py: Python<'_>,
    code: String,
    path: Option<String>,
) -> Result<PyObject, error::CompileErrors> {
    _exec_with_dependencies(py, code, vec![], path)
}

/// exec_with_dependencies(code: str, pkgs: list[Package]) -> module
/// --
///
/// compile and execute an Erg code as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "exec_file_with_dependencies")]
fn _exec_file_with_dependencies(
    py: Python<'_>,
    path: String,
    pkgs: Vec<Package>,
) -> Result<PyObject, error::CompileErrors> {
    let code = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{err}, path: {path}"));
    _exec_with_dependencies(py, code, pkgs, Some(path))
}

/// exec_file(path: str) -> module
/// --
///
/// compile and execute an Erg file as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "exec_file")]
fn _exec_file(py: Python<'_>, path: String) -> Result<PyObject, error::CompileErrors> {
    _exec_file_with_dependencies(py, path, vec![])
}

/// exec_ast_with_dependencies(ast: erg_parser.AST, pkgs: list[Package]) -> module
/// --
///
/// compile and execute an Erg AST as a module at runtime with dependencies
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "exec_ast_with_dependencies")]
fn _exec_ast_with_dependencies(
    py: Python<'_>,
    ast: erg_parser::ast::AST,
    pkgs: Vec<Package>,
    path: Option<String>,
) -> Result<PyObject, error::CompileErrors> {
    let code = _compile_ast_with_dependencies(py, ast, "exec", pkgs, path)?;
    let module = pyo3::types::PyModule::new(py, "<erg>").unwrap();
    let dic = [("code", code), ("dict", PyObject::from(module.dict()))].into_py_dict(py);
    py.run("exec(code, dict)", None, Some(dic)).unwrap();
    Ok(module.into())
}

/// exec_ast(ast: erg_parser.AST) -> module
/// --
///
/// compile and execute an Erg AST as a module at runtime
#[cfg(feature = "pylib")]
#[pyfunction]
#[pyo3(name = "exec_ast")]
fn _exec_ast(
    py: Python<'_>,
    ast: erg_parser::ast::AST,
    path: Option<String>,
) -> Result<PyObject, error::CompileErrors> {
    _exec_ast_with_dependencies(py, ast, vec![], path)
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
    let code = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{err}, path: {path}"));
    _exec(py, code, Some(path))
}

#[cfg(feature = "pylib")]
#[pymodule]
fn erg_compiler(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Package>()?;
    m.add_class::<_Compiler>()?;
    m.add_function(wrap_pyfunction!(_compile, m)?)?;
    m.add_function(wrap_pyfunction!(_compile_with_dependencies, m)?)?;
    m.add_function(wrap_pyfunction!(_compile_ast, m)?)?;
    m.add_function(wrap_pyfunction!(_compile_ast_with_dependencies, m)?)?;
    m.add_function(wrap_pyfunction!(_compile_file, m)?)?;
    m.add_function(wrap_pyfunction!(_compile_file_with_dependencies, m)?)?;
    m.add_function(wrap_pyfunction!(_exec, m)?)?;
    m.add_function(wrap_pyfunction!(_exec_with_dependencies, m)?)?;
    m.add_function(wrap_pyfunction!(_exec_ast, m)?)?;
    m.add_function(wrap_pyfunction!(_exec_ast_with_dependencies, m)?)?;
    m.add_function(wrap_pyfunction!(_exec_file, m)?)?;
    m.add_function(wrap_pyfunction!(_exec_file_with_dependencies, m)?)?;
    m.add_function(wrap_pyfunction!(_import, m)?)?;

    use crate::erg_parser::erg_parser;
    let parser = PyModule::new(py, "erg_parser")?;
    erg_parser(py, parser)?;
    m.add_submodule(parser)?;

    py.run(
        "\
import sys
sys.modules['erg_compiler.erg_parser'] = erg_parser
sys.modules['erg_compiler.erg_parser.ast'] = erg_parser.ast
sys.modules['erg_compiler.erg_parser.expr'] = erg_parser.expr
",
        None,
        Some(m.dict()),
    )?;

    Ok(())
}
