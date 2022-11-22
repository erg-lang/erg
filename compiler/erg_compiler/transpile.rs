use std::fs::File;
use std::io::Write;

use erg_common::config::ErgConfig;
use erg_common::log;
use erg_common::traits::{Runnable, Stream};
use erg_common::Str;

use erg_parser::ast::{ParamPattern, VarName};

use crate::build_hir::HIRBuilder;
use crate::context::{Context, ContextProvider};
use crate::desugar_hir::HIRDesugarer;
use crate::error::{CompileError, CompileErrors};
use crate::hir::{
    Accessor, Array, Block, Call, ClassDef, Def, Dict, Expr, Identifier, Lambda, Params, Set,
    Signature, Tuple, HIR,
};
use crate::link::Linker;
use crate::mod_cache::SharedModuleCache;
use crate::ty::Type;
use crate::varinfo::VarInfo;

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

impl ContextProvider for Transpiler {
    fn dir(&self) -> Vec<(&VarName, &VarInfo)> {
        self.builder.dir()
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.builder.get_receiver_ctx(receiver_name)
    }

    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.builder.get_var_info(name)
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

    pub fn pop_mod_ctx(&mut self) -> Context {
        self.builder.pop_mod_ctx()
    }

    pub fn dir(&mut self) -> Vec<(&VarName, &VarInfo)> {
        ContextProvider::dir(self)
    }

    pub fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        ContextProvider::get_receiver_ctx(self, receiver_name)
    }

    pub fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        ContextProvider::get_var_info(self, name)
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
        let mut code = self.load_prelude();
        for chunk in hir.module.into_iter() {
            code += &self.transpile_expr(chunk);
            code.push('\n');
        }
        PyScript {
            filename: hir.name,
            code,
        }
    }

    fn load_prelude(&mut self) -> String {
        "from collections import namedtuple as NamedTuple__\n".to_string()
    }

    fn transpile_expr(&mut self, expr: Expr) -> String {
        match expr {
            Expr::Lit(lit) => lit.token.content.to_string(),
            Expr::Call(call) => self.transpile_call(call),
            Expr::BinOp(bin) => {
                let mut code = "(".to_string();
                code += &self.transpile_expr(*bin.lhs);
                code += &bin.op.content;
                code += &self.transpile_expr(*bin.rhs);
                code += ")";
                code
            }
            Expr::UnaryOp(unary) => {
                let mut code = "(".to_string();
                code += &unary.op.content;
                code += &self.transpile_expr(*unary.expr);
                code += ")";
                code
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    let mut code = "[".to_string();
                    for elem in arr.elems.pos_args {
                        code += &format!("{},", self.transpile_expr(elem.expr));
                    }
                    code += "]";
                    code
                }
                other => todo!("transpiling {other}"),
            },
            Expr::Set(set) => match set {
                Set::Normal(st) => {
                    let mut code = "{".to_string();
                    for elem in st.elems.pos_args {
                        code += &format!("{},", self.transpile_expr(elem.expr));
                    }
                    code += "}";
                    code
                }
                other => todo!("transpiling {other}"),
            },
            Expr::Record(rec) => {
                let mut attrs = "[".to_string();
                let mut values = "(".to_string();
                for mut attr in rec.attrs.into_iter() {
                    attrs += &format!("'{}',", Self::transpile_ident(attr.sig.into_ident()));
                    if attr.body.block.len() > 1 {
                        todo!("transpile instant blocks")
                    }
                    values += &format!("{},", self.transpile_expr(attr.body.block.remove(0)));
                }
                attrs += "]";
                values += ")";
                format!("NamedTuple__('Record', {attrs}){values}")
            }
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    let mut code = "(".to_string();
                    for elem in tup.elems.pos_args {
                        code += &format!("{},", self.transpile_expr(elem.expr));
                    }
                    code += ")";
                    code
                }
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dic) => {
                    let mut code = "{".to_string();
                    for kv in dic.kvs {
                        code += &format!(
                            "({}): ({}),",
                            self.transpile_expr(kv.key),
                            self.transpile_expr(kv.value)
                        );
                    }
                    code += "}";
                    code
                }
                other => todo!("transpiling {other}"),
            },
            Expr::Accessor(acc) => match acc {
                Accessor::Ident(ident) => Self::transpile_ident(ident),
                Accessor::Attr(attr) => {
                    format!(
                        "({}).{}",
                        self.transpile_expr(*attr.obj),
                        Self::transpile_ident(attr.ident)
                    )
                }
            },
            Expr::Def(def) => self.transpile_def(def),
            Expr::Lambda(lambda) => self.transpile_lambda(lambda),
            Expr::ClassDef(classdef) => self.transpile_classdef(classdef),
            Expr::AttrDef(mut adef) => {
                let mut code = format!("{} = ", self.transpile_expr(Expr::Accessor(adef.attr)));
                if adef.block.len() > 1 {
                    todo!("transpile instant blocks")
                }
                let expr = adef.block.remove(0);
                code += &self.transpile_expr(expr);
                code
            }
            // TODO:
            Expr::Compound(comp) => {
                let mut code = "".to_string();
                for expr in comp.into_iter() {
                    code += &self.transpile_expr(expr);
                    code += &format!("\n{}", "    ".repeat(self.level));
                }
                code
            }
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
            Some("if" | "if!") => {
                let cond = self.transpile_expr(call.args.remove(0));
                let Expr::Lambda(mut block) = call.args.remove(0) else { todo!() };
                let then = self.transpile_expr(block.body.remove(0));
                if let Some(Expr::Lambda(mut block)) = call.args.try_remove(0) {
                    let els = self.transpile_expr(block.body.remove(0));
                    format!("{then} if {cond} else {els}")
                } else {
                    format!("{then} if {cond} else None")
                }
            }
            Some("for" | "for!") => {
                let mut code = "for ".to_string();
                let iter = call.args.remove(0);
                let Expr::Lambda(block) = call.args.remove(0) else { todo!() };
                let sig = block.params.non_defaults.get(0).unwrap();
                let ParamPattern::VarName(param) = &sig.pat else { todo!() };
                code += &format!("{}__ ", &param.token().content);
                code += &format!("in {}:\n", self.transpile_expr(iter));
                code += &self.transpile_block(block.body, false);
                code
            }
            Some("while" | "while!") => {
                let mut code = "while ".to_string();
                let cond = call.args.remove(0);
                let Expr::Lambda(block) = call.args.remove(0) else { todo!() };
                code += &format!("{}:\n", self.transpile_expr(cond));
                code += &self.transpile_block(block.body, false);
                code
            }
            // TODO:
            Some("match" | "match!") => {
                let mut code = "match ".to_string();
                let cond = call.args.remove(0);
                code += &format!("{}:\n", self.transpile_expr(cond));
                while let Some(Expr::Lambda(arm)) = call.args.try_remove(0) {
                    self.level += 1;
                    code += &"    ".repeat(self.level);
                    let target = arm.params.non_defaults.get(0).unwrap();
                    let ParamPattern::VarName(param) = &target.pat else { todo!() };
                    code += &format!("case {}__:\n", &param.token().content);
                    code += &self.transpile_block(arm.body, false);
                    self.level -= 1;
                }
                code
            }
            _ => self.transpile_simple_call(call),
        }
    }

    fn transpile_simple_call(&mut self, mut call: Call) -> String {
        let mut code = format!("({})", self.transpile_expr(*call.obj));
        if let Some(attr) = call.attr_name {
            code += &format!(".{}", Self::transpile_ident(attr));
        }
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
            let name = name.replace('$', "erg_shared__");
            format!("{name}__")
        }
    }

    fn transpile_params(&mut self, params: Params) -> String {
        let mut code = String::new();
        for non_default in params.non_defaults {
            let ParamPattern::VarName(param) = non_default.pat else { todo!() };
            code += &format!("{}__,", param.into_token().content);
        }
        for default in params.defaults {
            let ParamPattern::VarName(param) = default.sig.pat else { todo!() };
            code += &format!(
                "{}__ = {},",
                param.into_token().content,
                self.transpile_expr(default.default_val)
            );
        }
        code
    }

    fn transpile_block(&mut self, block: Block, return_last: bool) -> String {
        self.level += 1;
        let mut code = String::new();
        let last = block.len().saturating_sub(1);
        for (i, chunk) in block.into_iter().enumerate() {
            code += &"    ".repeat(self.level);
            if i == last && return_last {
                code += "return ";
            }
            code += &self.transpile_expr(chunk);
            code.push('\n');
        }
        self.level -= 1;
        code
    }

    fn transpile_lambda(&mut self, lambda: Lambda) -> String {
        let mut code = format!("(lambda {}:", self.transpile_params(lambda.params));
        if lambda.body.len() > 1 {
            todo!("multi line lambda");
        }
        code += &self.transpile_block(lambda.body, false);
        code.pop(); // \n
        code.push(')');
        code
    }

    fn transpile_def(&mut self, mut def: Def) -> String {
        match def.sig {
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
                code += &self.transpile_block(def.body.block, true);
                code
            }
        }
    }

    fn transpile_classdef(&mut self, classdef: ClassDef) -> String {
        let class_name = Self::transpile_ident(classdef.sig.into_ident());
        let mut code = format!("class {class_name}():\n");
        let mut init_method = format!(
            "{}def __init__(self, param__):\n",
            "    ".repeat(self.level + 1)
        );
        match classdef.__new__.non_default_params().unwrap()[0].typ() {
            Type::Record(rec) => {
                for field in rec.keys() {
                    init_method += &format!(
                        "{}self.{} = param__.{}\n",
                        "    ".repeat(self.level + 2),
                        field.symbol,
                        field.symbol
                    );
                }
            }
            other => todo!("{other}"),
        }
        code += &init_method;
        if classdef.need_to_gen_new {
            code += &format!("def new(x): return {class_name}.__call__(x)\n");
        }
        code += &self.transpile_block(classdef.methods, false);
        code
    }
}
