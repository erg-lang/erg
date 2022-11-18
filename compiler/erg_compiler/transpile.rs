use std::fs::File;
use std::io::Write;

use erg_common::config::ErgConfig;
use erg_common::log;
use erg_common::traits::{Runnable, Stream};
use erg_common::Str;

use erg_parser::ast::ParamPattern;

use crate::build_hir::HIRBuilder;
use crate::desugar_hir::HIRDesugarer;
use crate::error::{CompileError, CompileErrors};
use crate::hir::{Accessor, Block, Call, Expr, Identifier, Params, Signature, HIR};
use crate::link::Linker;
use crate::mod_cache::SharedModuleCache;

#[derive(Debug, Clone)]
pub struct PyScript {
    pub filename: Str,
    pub code: String,
}

/// Generates a `PyScript` from an String or other File inputs.
#[derive(Debug, Default)]
pub struct Transpiler {
    pub cfg: ErgConfig,
    builder: HIRBuilder,
    mod_cache: SharedModuleCache,
    script_generator: ScriptGenerator,
}

impl Runnable for Transpiler {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg transpiler";

    fn new(cfg: ErgConfig) -> Self {
        let mod_cache = SharedModuleCache::new();
        let py_mod_cache = SharedModuleCache::new();
        Self {
            builder: HIRBuilder::new_with_cache(
                cfg.copy(),
                "<module>",
                mod_cache.clone(),
                py_mod_cache,
            ),
            script_generator: ScriptGenerator::new(),
            mod_cache,
            cfg,
        }
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

    fn clear(&mut self) {
        // self.builder.clear();
    }

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let path = self.input().filename().replace(".er", ".py");
        let script = self.transpile(self.input().read(), "exec")?;
        let mut f = File::create(&path).unwrap();
        f.write_all(script.code.as_bytes()).unwrap();
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, CompileErrors> {
        let script = self.transpile(src, "eval")?;
        Ok(script.code)
    }
}

impl Transpiler {
    pub fn transpile(&mut self, src: String, mode: &str) -> Result<PyScript, CompileErrors> {
        log!(info "the transpiling process has started.");
        let hir = self.build_link_desugar(src, mode)?;
        let script = self.script_generator.transpile(hir);
        log!(info "code:\n{}", script.code);
        log!(info "the transpiling process has completed");
        Ok(script)
    }

    fn build_link_desugar(&mut self, src: String, mode: &str) -> Result<HIR, CompileErrors> {
        let artifact = self
            .builder
            .build(src, mode)
            .map_err(|artifact| artifact.errors)?;
        let linker = Linker::new(&self.cfg, &self.mod_cache);
        let hir = linker.link(artifact.hir);
        Ok(HIRDesugarer::desugar(hir))
    }
}

#[derive(Debug, Default)]
pub struct ScriptGenerator {
    level: usize,
}

impl ScriptGenerator {
    pub const fn new() -> Self {
        Self { level: 0 }
    }

    pub fn transpile(&mut self, hir: HIR) -> PyScript {
        let mut code = String::new();
        for chunk in hir.module.into_iter() {
            code += &self.transpile_expr(chunk);
            code.push('\n');
        }
        PyScript {
            filename: hir.name,
            code,
        }
    }

    pub fn transpile_expr(&mut self, expr: Expr) -> String {
        match expr {
            Expr::Lit(lit) => lit.token.content.to_string(),
            Expr::Call(call) => self.transpile_call(call),
            Expr::BinOp(bin) => {
                let mut code = self.transpile_expr(*bin.lhs);
                code += &bin.op.content;
                code += &self.transpile_expr(*bin.rhs);
                code
            }
            Expr::UnaryOp(unary) => {
                let mut code = unary.op.content.to_string();
                code += &self.transpile_expr(*unary.expr);
                code
            }
            Expr::Accessor(acc) => match acc {
                Accessor::Ident(ident) => Self::transpile_ident(ident),
                Accessor::Attr(attr) => {
                    format!(
                        "{}.{}",
                        self.transpile_expr(*attr.obj),
                        Self::transpile_ident(attr.ident)
                    )
                }
            },
            Expr::Def(mut def) => match def.sig {
                Signature::Var(var) => {
                    let mut code = format!("{} = ", Self::transpile_ident(var.ident));
                    if def.body.block.len() > 1 {
                        todo!("transpile instant blocks")
                    }
                    let expr = def.body.block.remove(0);
                    code += &self.transpile_expr(expr);
                    code
                }
                Signature::Subr(subr) => {
                    let mut code = format!(
                        "def {}({}):\n",
                        Self::transpile_ident(subr.ident),
                        self.transpile_params(subr.params)
                    );
                    code += &self.transpile_block(def.body.block);
                    code
                }
            },
            other => todo!("transpile {other}"),
        }
    }

    fn transpile_call(&mut self, mut call: Call) -> String {
        match call.obj.local_name() {
            Some("assert") => {
                let mut code = format!("assert {}", self.transpile_expr(call.args.remove(0)));
                if let Some(msg) = call.args.try_remove(0) {
                    code += &format!(", {}", self.transpile_expr(msg));
                }
                code
            }
            _ => self.transpile_simple_call(call),
        }
    }

    fn transpile_simple_call(&mut self, mut call: Call) -> String {
        let mut code = self.transpile_expr(*call.obj);
        code.push('(');
        while let Some(arg) = call.args.try_remove_pos(0) {
            code += &self.transpile_expr(arg.expr);
            code.push(',');
        }
        while let Some(arg) = call.args.try_remove_kw(0) {
            code += &format!("{}={},", arg.keyword, self.transpile_expr(arg.expr));
        }
        code.push(')');
        code
    }

    fn transpile_ident(ident: Identifier) -> String {
        if let Some(py_name) = ident.vi.py_name {
            py_name.to_string()
        } else if ident.dot.is_some() {
            ident.name.into_token().content.to_string()
        } else {
            let name = ident.name.into_token().content;
            let name = name.replace('!', "__erg_proc__");
            let name = name.replace('$', "__erg_shared__");
            format!("__{name}")
        }
    }

    fn transpile_params(&mut self, params: Params) -> String {
        let mut code = String::new();
        for non_default in params.non_defaults {
            let ParamPattern::VarName(param) = non_default.pat else { todo!() };
            code += &format!("{},", param.into_token().content);
        }
        for default in params.defaults {
            let ParamPattern::VarName(param) = default.sig.pat else { todo!() };
            code += &format!(
                "{}={},",
                param.into_token().content,
                self.transpile_expr(default.default_val)
            );
        }
        code
    }

    fn transpile_block(&mut self, block: Block) -> String {
        self.level += 1;
        let mut code = String::new();
        let last = block.len() - 1;
        for (i, chunk) in block.into_iter().enumerate() {
            code += &"    ".repeat(self.level);
            if i == last {
                code += "return ";
            }
            code += &self.transpile_expr(chunk);
            code.push('\n');
        }
        self.level -= 1;
        code
    }
}
