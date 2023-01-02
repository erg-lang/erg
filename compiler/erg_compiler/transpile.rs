use std::fs::File;
use std::io::Write;

use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::log;
use erg_common::traits::{Runnable, Stream};
use erg_common::Str;

use erg_parser::ast::{ParamPattern, TypeSpec, VarName};
use erg_parser::token::TokenKind;

use crate::artifact::{
    BuildRunnable, Buildable, CompleteArtifact, ErrorArtifact, IncompleteArtifact,
};
use crate::build_hir::HIRBuilder;
use crate::codegen::PyCodeGenerator;
use crate::context::{Context, ContextProvider, ModuleContext};
use crate::desugar_hir::HIRDesugarer;
use crate::error::{CompileError, CompileErrors};
use crate::hir::{
    Accessor, Args, Array, BinOp, Block, Call, ClassDef, Def, Dict, Expr, Identifier, Lambda,
    Literal, Params, PatchDef, ReDef, Record, Set, Signature, Tuple, UnaryOp, HIR,
};
use crate::link::Linker;
use crate::mod_cache::SharedModuleCache;
use crate::ty::value::ValueObj;
use crate::ty::Type;
use crate::varinfo::VarInfo;

/// patch method -> function
/// patch attr -> variable
fn debind(ident: &Identifier) -> Option<Str> {
    match ident.vi.py_name.as_ref().map(|s| &s[..]) {
        Some(name) if name.starts_with("Function::") => {
            Some(Str::from(name.replace("Function::", "")))
        }
        Some(patch_method) if patch_method.contains("::") || patch_method.contains('.') => {
            if ident.vis().is_private() {
                Some(Str::from(format!("{patch_method}__")))
            } else {
                Some(Str::rc(patch_method))
            }
        }
        _ => None,
    }
}

fn demangle(name: &str) -> String {
    name.trim_start_matches("::<module>")
        .replace("::", "__")
        .replace('.', "_")
}

// TODO:
fn replace_non_symbolic(name: String) -> String {
    name.replace('\'', "__single_quote__")
        .replace(' ', "__space__")
        .replace('+', "__plus__")
        .replace('-', "__minus__")
        .replace('*', "__star__")
        .replace('/', "__slash__")
        .replace('%', "__percent__")
        .replace('!', "__erg_proc__")
        .replace('$', "erg_shared__")
}

#[derive(Debug)]
pub enum LastLineOperation {
    Discard,
    Return,
    StoreTmp(Str),
}

use LastLineOperation::*;

impl LastLineOperation {
    pub const fn is_return(&self) -> bool {
        matches!(self, LastLineOperation::Return)
    }

    pub const fn is_store_tmp(&self) -> bool {
        matches!(self, LastLineOperation::StoreTmp(_))
    }
}

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
        let mod_cache = SharedModuleCache::new(cfg.copy());
        let py_mod_cache = SharedModuleCache::new(cfg.copy());
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
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        &mut self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

    fn initialize(&mut self) {
        self.builder.initialize();
        // mod_cache will be cleared by the builder
        // self.mod_cache.initialize();
    }

    fn clear(&mut self) {
        self.builder.clear();
    }

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let path = self.cfg.dump_path().replace(".er", ".py");
        let artifact = self
            .transpile(self.input().read(), "exec")
            .map_err(|eart| {
                eart.warns.fmt_all_stderr();
                eart.errors
            })?;
        artifact.warns.fmt_all_stderr();
        let mut f = File::create(path).unwrap();
        f.write_all(artifact.object.code.as_bytes()).unwrap();
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, CompileErrors> {
        let artifact = self.transpile(src, "eval").map_err(|eart| {
            eart.warns.fmt_all_stderr();
            eart.errors
        })?;
        artifact.warns.fmt_all_stderr();
        Ok(artifact.object.code)
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

impl Buildable<PyScript> for Transpiler {
    fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<PyScript>, IncompleteArtifact<PyScript>> {
        self.transpile(src, mode)
            .map_err(|err| IncompleteArtifact::new(None, err.errors, err.warns))
    }
    fn pop_context(&mut self) -> Option<ModuleContext> {
        self.builder.pop_context()
    }
    fn get_context(&self) -> Option<&ModuleContext> {
        self.builder.get_context()
    }
}

impl BuildRunnable<PyScript> for Transpiler {}

impl Transpiler {
    pub fn transpile(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<PyScript>, ErrorArtifact> {
        log!(info "the transpiling process has started.");
        let artifact = self.build_link_desugar(src, mode)?;
        let script = self.script_generator.transpile(artifact.object);
        log!(info "code:\n{}", script.code);
        log!(info "the transpiling process has completed");
        Ok(CompleteArtifact::new(script, artifact.warns))
    }

    fn build_link_desugar(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, ErrorArtifact> {
        let artifact = self.builder.build(src, mode)?;
        let linker = Linker::new(&self.cfg, &self.mod_cache);
        let hir = linker.link(artifact.object);
        let desugared = HIRDesugarer::desugar(hir);
        Ok(CompleteArtifact::new(desugared, artifact.warns))
    }

    pub fn pop_mod_ctx(&mut self) -> Option<ModuleContext> {
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
    fresh_var_n: usize,
    namedtuple_loaded: bool,
    mutate_op_loaded: bool,
    in_op_loaded: bool,
    range_ops_loaded: bool,
    builtin_types_loaded: bool,
    builtin_control_loaded: bool,
    convertors_loaded: bool,
    prelude: String,
}

impl ScriptGenerator {
    pub const fn new() -> Self {
        Self {
            level: 0,
            fresh_var_n: 0,
            namedtuple_loaded: false,
            mutate_op_loaded: false,
            in_op_loaded: false,
            range_ops_loaded: false,
            builtin_types_loaded: false,
            builtin_control_loaded: false,
            convertors_loaded: false,
            prelude: String::new(),
        }
    }

    pub fn transpile(&mut self, hir: HIR) -> PyScript {
        let mut code = String::new();
        for chunk in hir.module.into_iter() {
            code += &self.transpile_expr(chunk);
            code.push('\n');
        }
        code = std::mem::take(&mut self.prelude) + &code;
        PyScript {
            filename: hir.name,
            code,
        }
    }

    // TODO: more smart way
    fn replace_import(src: &str) -> String {
        src.replace("from _erg_nat import Nat", "")
            .replace("from _erg_int import IntMut", "")
            .replace("from _erg_int import Int", "")
            .replace("from _erg_bool import Bool", "")
            .replace("from _erg_str import Str", "")
            .replace("from _erg_array import Array", "")
            .replace("from _erg_range import Range", "")
            .replace("from _erg_result import Error", "")
            .replace("from _erg_result import is_ok", "")
    }

    fn load_namedtuple(&mut self) {
        self.prelude += "from collections import namedtuple as NamedTuple__\n";
    }

    // TODO: name escaping
    fn load_range_ops(&mut self) {
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_result.py"));
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_int.py"));
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_nat.py"));
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_str.py"));
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_range.py"));
    }

    fn load_in_op(&mut self) {
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_result.py"));
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_range.py"));
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_in_operator.py"));
    }

    fn load_mutate_op(&mut self) {
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_mutate_operator.py"));
    }

    fn load_builtin_types(&mut self) {
        if self.range_ops_loaded {
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_array.py"));
        } else if self.in_op_loaded {
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_int.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_nat.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_bool.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_str.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_array.py"));
        } else {
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_result.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_int.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_nat.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_bool.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_str.py"));
            self.prelude += &Self::replace_import(include_str!("lib/std/_erg_array.py"));
        }
    }

    fn load_builtin_controls(&mut self) {
        self.prelude += include_str!("lib/std/_erg_control.py");
    }

    fn load_convertors(&mut self) {
        self.prelude += &Self::replace_import(include_str!("lib/std/_erg_convertors.py"));
    }

    fn escape_str(s: &str) -> String {
        s.replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
            .replace('\'', "\\'")
            .replace('\0', "\\0")
    }

    fn transpile_expr(&mut self, expr: Expr) -> String {
        match expr {
            Expr::Lit(lit) => self.transpile_lit(lit),
            Expr::Call(call) => self.transpile_call(call),
            Expr::BinOp(bin) => self.transpile_binop(bin),
            Expr::UnaryOp(unary) => self.transpile_unaryop(unary),
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
            Expr::Record(rec) => self.transpile_record(rec),
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
            Expr::Accessor(acc) => self.transpile_acc(acc),
            Expr::Def(def) => self.transpile_def(def),
            Expr::Lambda(lambda) => self.transpile_lambda(lambda),
            Expr::ClassDef(classdef) => self.transpile_classdef(classdef),
            Expr::PatchDef(patchdef) => self.transpile_patchdef(patchdef),
            Expr::ReDef(redef) => self.transpile_attrdef(redef),
            // TODO:
            Expr::Compound(comp) => {
                let mut code = "".to_string();
                for expr in comp.into_iter() {
                    code += &self.transpile_expr(expr);
                    code += &format!("\n{}", "    ".repeat(self.level));
                }
                code
            }
            Expr::Import(acc) => {
                let full_name = Str::from(acc.show());
                let root = PyCodeGenerator::get_root(&acc);
                self.prelude += &format!(
                    "{} = __import__(\"{full_name}\")\n",
                    Self::transpile_ident(root)
                );
                String::new()
            }
            Expr::TypeAsc(tasc) => self.transpile_expr(*tasc.expr),
            Expr::Code(_) => todo!("transpiling importing user-defined code"),
            Expr::Dummy(_) => "".to_string(),
        }
    }

    fn transpile_lit(&mut self, lit: Literal) -> String {
        let escaped = Self::escape_str(&lit.token.content);
        if matches!(
            &lit.value,
            ValueObj::Bool(_) | ValueObj::Int(_) | ValueObj::Nat(_) | ValueObj::Str(_)
        ) {
            if !self.builtin_types_loaded {
                self.load_builtin_types();
                self.builtin_types_loaded = true;
            }
            format!("{}({escaped})", lit.value.class())
        } else {
            escaped
        }
    }

    fn transpile_record(&mut self, rec: Record) -> String {
        if !self.namedtuple_loaded {
            self.load_namedtuple();
            self.namedtuple_loaded = true;
        }
        let mut attrs = "[".to_string();
        let mut values = "(".to_string();
        for mut attr in rec.attrs.into_iter() {
            attrs += &format!("'{}',", Self::transpile_ident(attr.sig.into_ident()));
            if attr.body.block.len() > 1 {
                let name = format!("instant_block_{}__", self.fresh_var_n);
                self.fresh_var_n += 1;
                let mut code = format!("def {name}():\n");
                code += &self.transpile_block(attr.body.block, Return);
                self.prelude += &code;
                values += &format!("{name}(),");
            } else {
                let expr = attr.body.block.remove(0);
                values += &format!("{},", self.transpile_expr(expr));
            }
        }
        attrs += "]";
        values += ")";
        format!("NamedTuple__('Record', {attrs}){values}")
    }

    fn transpile_binop(&mut self, bin: BinOp) -> String {
        match bin.op.kind {
            TokenKind::Closed | TokenKind::LeftOpen | TokenKind::RightOpen | TokenKind::Open => {
                if !self.range_ops_loaded {
                    self.load_range_ops();
                    self.range_ops_loaded = true;
                }
                let mut code = match bin.op.kind {
                    TokenKind::Closed => "ClosedRange(",
                    TokenKind::LeftOpen => "LeftOpenRange(",
                    TokenKind::RightOpen => "RightOpenRange(",
                    TokenKind::Open => "OpenRange(",
                    _ => unreachable!(),
                }
                .to_string();
                code += &self.transpile_expr(*bin.lhs);
                code.push(',');
                code += &self.transpile_expr(*bin.rhs);
                code.push(')');
                code
            }
            TokenKind::InOp => {
                if !self.in_op_loaded {
                    self.load_in_op();
                    self.in_op_loaded = true;
                }
                let mut code = "in_operator(".to_string();
                code += &self.transpile_expr(*bin.lhs);
                code.push(',');
                code += &self.transpile_expr(*bin.rhs);
                code.push(')');
                code
            }
            _ => {
                let mut code = "(".to_string();
                code += &self.transpile_expr(*bin.lhs);
                code.push(' ');
                code += &bin.op.content;
                code.push(' ');
                code += &self.transpile_expr(*bin.rhs);
                code += ")";
                code
            }
        }
    }

    fn transpile_unaryop(&mut self, unary: UnaryOp) -> String {
        let mut code = "".to_string();
        if unary.op.kind == TokenKind::Mutate {
            if !self.mutate_op_loaded {
                self.load_mutate_op();
                self.mutate_op_loaded = true;
            }
            code += "mutate_operator(";
        } else {
            code += "(";
            code += &unary.op.content;
        }
        code += &self.transpile_expr(*unary.expr);
        code += ")";
        code
    }

    fn transpile_acc(&mut self, acc: Accessor) -> String {
        match acc {
            Accessor::Ident(ident) => {
                match &ident.inspect()[..] {
                    "Str" | "Bool" | "Nat" | "Array" if !self.builtin_types_loaded => {
                        self.load_builtin_types();
                        self.builtin_types_loaded = true;
                    }
                    "if" | "if!" | "for!" | "while" | "discard" if !self.builtin_control_loaded => {
                        self.load_builtin_controls();
                        self.builtin_control_loaded = true;
                    }
                    "int" | "nat" if !self.convertors_loaded => {
                        self.load_convertors();
                        self.convertors_loaded = true;
                    }
                    _ => {}
                }
                Self::transpile_ident(ident)
            }
            Accessor::Attr(attr) => {
                if let Some(name) = debind(&attr.ident) {
                    demangle(&name)
                } else {
                    format!(
                        "({}).{}",
                        self.transpile_expr(*attr.obj),
                        Self::transpile_ident(attr.ident)
                    )
                }
            }
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
            Some("not") => format!("(not ({}))", self.transpile_expr(call.args.remove(0))),
            Some("if" | "if!") => self.transpile_if(call),
            Some("for" | "for!") => {
                let mut code = "for ".to_string();
                let iter = call.args.remove(0);
                let Expr::Lambda(block) = call.args.remove(0) else { todo!() };
                let sig = block.params.non_defaults.get(0).unwrap();
                let ParamPattern::VarName(param) = &sig.pat else { todo!() };
                code += &format!("{}__ ", &param.token().content);
                code += &format!("in {}:\n", self.transpile_expr(iter));
                code += &self.transpile_block(block.body, Discard);
                code
            }
            Some("while" | "while!") => {
                let mut code = "while ".to_string();
                let cond = call.args.remove(0);
                let Expr::Lambda(block) = call.args.remove(0) else { todo!() };
                code += &format!("{}:\n", self.transpile_expr(cond));
                code += &self.transpile_block(block.body, Discard);
                code
            }
            Some("match" | "match!") => self.transpile_match(call),
            _ => self.transpile_simple_call(call),
        }
    }

    fn transpile_if(&mut self, mut call: Call) -> String {
        let cond = self.transpile_expr(call.args.remove(0));
        let Expr::Lambda(mut then_block) = call.args.remove(0) else { todo!() };
        let else_block = call.args.try_remove(0).map(|ex| {
            if let Expr::Lambda(blk) = ex {
                blk
            } else {
                todo!()
            }
        });
        if then_block.body.len() == 1
            && else_block
                .as_ref()
                .map(|blk| blk.body.len() == 1)
                .unwrap_or(true)
        {
            let then = self.transpile_expr(then_block.body.remove(0));
            if let Some(mut else_block) = else_block {
                let els = self.transpile_expr(else_block.body.remove(0));
                return format!("{then} if {cond} else {els}");
            } else {
                return format!("{then} if {cond} else None");
            }
        }
        let tmp = Str::from(format!("if_tmp_{}__", self.fresh_var_n));
        self.fresh_var_n += 1;
        let tmp_func = Str::from(format!("if_tmp_func_{}__", self.fresh_var_n));
        self.fresh_var_n += 1;
        let mut code = format!("def {tmp_func}():\n");
        code += &format!("    if {cond}:\n");
        let level = self.level;
        self.level = 1;
        code += &self.transpile_block(then_block.body, StoreTmp(tmp.clone()));
        self.level = level;
        if let Some(else_block) = else_block {
            code += "    else:\n";
            let level = self.level;
            self.level = 1;
            code += &self.transpile_block(else_block.body, StoreTmp(tmp.clone()));
            self.level = level;
        } else {
            code += "    else:\n";
            code += &format!("        {tmp} = None\n");
        }
        code += &format!("    return {tmp}\n");
        self.prelude += &code;
        // ~~ NOTE: In Python, the variable environment of a function is determined at call time
        // This is a very bad design, but can be used for this code ~~
        // FIXME: this trick only works in the global namespace
        format!("{tmp_func}()")
    }

    fn transpile_match(&mut self, mut call: Call) -> String {
        let tmp = Str::from(format!("match_tmp_{}__", self.fresh_var_n));
        self.fresh_var_n += 1;
        let tmp_func = Str::from(format!("match_tmp_func_{}__", self.fresh_var_n));
        self.fresh_var_n += 1;
        let mut code = format!("def {tmp_func}():\n");
        self.level += 1;
        code += &"    ".repeat(self.level);
        code += "match ";
        let cond = call.args.remove(0);
        code += &format!("{}:\n", self.transpile_expr(cond));
        while let Some(Expr::Lambda(arm)) = call.args.try_remove(0) {
            self.level += 1;
            code += &"    ".repeat(self.level);
            let target = arm.params.non_defaults.get(0).unwrap();
            match &target.pat {
                ParamPattern::VarName(param) => {
                    code += &format!("case {}__:\n", &param.token().content);
                    code += &self.transpile_block(arm.body, StoreTmp(tmp.clone()));
                    self.level -= 1;
                }
                ParamPattern::Discard(_) => {
                    match target.t_spec.as_ref().map(|t| &t.t_spec) {
                        Some(TypeSpec::Enum(enum_t)) => {
                            let values = ValueObj::vec_from_const_args(enum_t.clone());
                            if values.len() == 1 {
                                code += &format!("case {}:\n", values[0]);
                            } else {
                                todo!()
                            }
                        }
                        Some(_) => todo!(),
                        None => {
                            code += "case _:\n";
                        }
                    }
                    code += &self.transpile_block(arm.body, StoreTmp(tmp.clone()));
                    self.level -= 1;
                }
                _ => todo!(),
            }
        }
        code += &"    ".repeat(self.level);
        code += &format!("return {tmp}\n");
        self.prelude += &code;
        format!("{tmp_func}()")
    }

    fn transpile_simple_call(&mut self, call: Call) -> String {
        let is_py_api = if let Some(attr) = &call.attr_name {
            let is_py_api = attr.is_py_api();
            if let Some(name) = debind(attr) {
                let name = demangle(&name);
                return format!(
                    "{name}({}, {})",
                    self.transpile_expr(*call.obj),
                    self.transpile_args(call.args, is_py_api, false)
                );
            }
            is_py_api
        } else {
            call.obj.is_py_api()
        };
        let mut code = format!("({})", self.transpile_expr(*call.obj));
        if let Some(attr) = call.attr_name {
            code += &format!(".{}", Self::transpile_ident(attr));
        }
        code += &self.transpile_args(call.args, is_py_api, true);
        code
    }

    fn transpile_args(&mut self, mut args: Args, is_py_api: bool, paren: bool) -> String {
        let mut code = String::new();
        if paren {
            code.push('(');
        }
        while let Some(arg) = args.try_remove_pos(0) {
            code += &self.transpile_expr(arg.expr);
            code.push(',');
        }
        while let Some(arg) = args.try_remove_kw(0) {
            let escape = if is_py_api { "" } else { "__" };
            code += &format!(
                "{}{escape}={},",
                arg.keyword.content,
                self.transpile_expr(arg.expr)
            );
        }
        if paren {
            code.push(')');
        }
        code
    }

    fn transpile_ident(ident: Identifier) -> String {
        if let Some(py_name) = ident.vi.py_name {
            return demangle(&py_name);
        }
        let name = ident.name.into_token().content.to_string();
        let name = replace_non_symbolic(name);
        if ident.dot.is_some() {
            name
        } else {
            format!("{name}__")
        }
    }

    fn transpile_params(&mut self, params: Params) -> String {
        let mut code = String::new();
        for non_default in params.non_defaults {
            match non_default.pat {
                ParamPattern::VarName(param) => {
                    code += &format!("{}__,", param.into_token().content);
                }
                ParamPattern::Discard(_) => {
                    code += &format!("_{},", self.fresh_var_n);
                    self.fresh_var_n += 1;
                }
                _ => unreachable!(),
            }
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

    fn transpile_block(&mut self, block: Block, last_op: LastLineOperation) -> String {
        self.level += 1;
        let mut code = String::new();
        let last = block.len().saturating_sub(1);
        for (i, chunk) in block.into_iter().enumerate() {
            code += &"    ".repeat(self.level);
            if i == last {
                match last_op {
                    Return => {
                        code += "return ";
                    }
                    Discard => {}
                    StoreTmp(ref tmp) => {
                        code += &format!("{tmp} = ");
                    }
                }
            }
            code += &self.transpile_expr(chunk);
            code.push('\n');
        }
        self.level -= 1;
        code
    }

    fn transpile_lambda(&mut self, lambda: Lambda) -> String {
        if lambda.body.len() > 1 {
            let name = format!("lambda_{}__", self.fresh_var_n);
            self.fresh_var_n += 1;
            let mut code = format!("def {name}({}):\n", self.transpile_params(lambda.params));
            code += &self.transpile_block(lambda.body, Return);
            self.prelude += &code;
            name
        } else {
            let mut code = format!("(lambda {}:", self.transpile_params(lambda.params));
            code += &self.transpile_block(lambda.body, Discard);
            code.pop(); // \n
            code.push(')');
            code
        }
    }

    // TODO: trait definition
    fn transpile_def(&mut self, mut def: Def) -> String {
        match def.sig {
            Signature::Var(var) => {
                let mut code = format!("{} = ", Self::transpile_ident(var.ident));
                if def.body.block.len() > 1 {
                    let name = format!("instant_block_{}__", self.fresh_var_n);
                    self.fresh_var_n += 1;
                    let mut code = format!("def {name}():\n");
                    code += &self.transpile_block(def.body.block, Return);
                    self.prelude += &code;
                    format!("{name}()")
                } else {
                    let expr = def.body.block.remove(0);
                    code += &self.transpile_expr(expr);
                    code
                }
            }
            Signature::Subr(subr) => {
                let mut code = format!(
                    "def {}({}):\n",
                    Self::transpile_ident(subr.ident),
                    self.transpile_params(subr.params)
                );
                code += &self.transpile_block(def.body.block, Return);
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
                    let vis = if field.vis.is_private() { "__" } else { "" };
                    init_method += &format!(
                        "{}self.{}{vis} = param__.{}{vis}\n",
                        "    ".repeat(self.level + 2),
                        field.symbol,
                        field.symbol,
                    );
                }
            }
            other => todo!("{other}"),
        }
        code += &init_method;
        if classdef.need_to_gen_new {
            code += &"    ".repeat(self.level + 1);
            code += &format!("def new(x): return {class_name}.__call__(x)\n");
        }
        code += &self.transpile_block(classdef.methods, Discard);
        code
    }

    fn transpile_patchdef(&mut self, patch_def: PatchDef) -> String {
        let mut code = String::new();
        for chunk in patch_def.methods.into_iter() {
            let Expr::Def(mut def) = chunk else { todo!() };
            let name = format!(
                "{}{}",
                demangle(&patch_def.sig.ident().to_string_without_type()),
                demangle(&def.sig.ident().to_string_without_type()),
            );
            def.sig.ident_mut().name = VarName::from_str(Str::from(name));
            code += &"    ".repeat(self.level);
            code += &self.transpile_def(def);
            code.push('\n');
        }
        code
    }

    fn transpile_attrdef(&mut self, mut redef: ReDef) -> String {
        let mut code = format!("{} = ", self.transpile_expr(Expr::Accessor(redef.attr)));
        if redef.block.len() > 1 {
            let name = format!("instant_block_{}__", self.fresh_var_n);
            self.fresh_var_n += 1;
            let mut code = format!("def {name}():\n");
            code += &self.transpile_block(redef.block, Return);
            self.prelude += &code;
            format!("{name}()")
        } else {
            let expr = redef.block.remove(0);
            code += &self.transpile_expr(expr);
            code
        }
    }
}
