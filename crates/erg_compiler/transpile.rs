use std::fs::File;
use std::io::Write;

use erg_common::error::{ErrorDisplay, ErrorKind, MultiErrorDisplay};
use erg_common::log;
use erg_common::set::Set as HashSet;
use erg_common::traits::BlockKind;
use erg_common::traits::{ExitStatus, Locational, New, Runnable, Stream};
use erg_common::Str;
use erg_common::{config::ErgConfig, dict};
use erg_common::{config::TranspileTarget, dict::Dict as HashMap};

use erg_parser::ast::{ParamPattern, TypeSpec, VarName, AST};
use erg_parser::token::TokenKind;
use erg_parser::ParserRunner;

use crate::artifact::{
    BuildRunnable, Buildable, CompleteArtifact, ErrorArtifact, IncompleteArtifact,
};
use crate::build_package::PackageBuilder;
use crate::codegen::PyCodeGenerator;
use crate::context::{Context, ContextProvider, ModuleContext};
use crate::desugar_hir::HIRDesugarer;
use crate::error::{CompileError, CompileErrors, CompileResult};
use crate::hir::{
    Accessor, Args, BinOp, Block, Call, ClassDef, Def, Dict, Expr, Identifier, Lambda, List,
    Literal, Params, PatchDef, ReDef, Record, Set, Signature, Tuple, UnaryOp, HIR,
};
use crate::link_hir::HIRLinker;
use crate::module::SharedCompilerResource;
use crate::ty::typaram::OpKind;
use crate::ty::value::ValueObj;
use crate::ty::{Field, HasType, Type, VisibilityModifier};
use crate::varinfo::{AbsLocation, VarInfo};

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
fn replace_non_symbolic(name: &str) -> String {
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

pub enum Enclosure {
    /// ()
    Paren,
    /// []
    Bracket,
    /// {}
    Brace,
    None,
}

impl Enclosure {
    pub const fn open(&self) -> char {
        match self {
            Enclosure::Paren => '(',
            Enclosure::Bracket => '[',
            Enclosure::Brace => '{',
            Enclosure::None => ' ',
        }
    }

    pub const fn close(&self) -> char {
        match self {
            Enclosure::Paren => ')',
            Enclosure::Bracket => ']',
            Enclosure::Brace => '}',
            Enclosure::None => ' ',
        }
    }
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

#[derive(Debug)]
pub enum TranspiledFile {
    PyScript(PyScript),
    Json(Json),
}

impl TranspiledFile {
    pub fn code(&self) -> &str {
        match self {
            Self::PyScript(script) => &script.code,
            Self::Json(json) => &json.code,
        }
    }

    pub fn into_code(self) -> String {
        match self {
            Self::PyScript(script) => script.code,
            Self::Json(json) => json.code,
        }
    }

    pub fn filename(&self) -> &str {
        match self {
            Self::PyScript(script) => &script.filename,
            Self::Json(json) => &json.filename,
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            Self::PyScript(_) => "py",
            Self::Json(_) => "json",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PyScript {
    pub filename: Str,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct Json {
    pub filename: Str,
    pub code: String,
}

/// Generates a `PyScript` from an String or other File inputs.
#[derive(Debug)]
pub struct Transpiler {
    pub cfg: ErgConfig,
    builder: PackageBuilder,
    shared: SharedCompilerResource,
    script_generator: PyScriptGenerator,
}

impl Default for Transpiler {
    fn default() -> Self {
        Self::new(ErgConfig::default())
    }
}

impl New for Transpiler {
    fn new(cfg: ErgConfig) -> Self {
        let shared = SharedCompilerResource::new(cfg.copy());
        Self {
            shared: shared.clone(),
            builder: PackageBuilder::new_with_cache(cfg.copy(), "<module>".into(), shared),
            script_generator: PyScriptGenerator::new(),
            cfg,
        }
    }
}

impl Runnable for Transpiler {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg transpiler";

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

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let mut path = self.cfg.dump_path();
        let src = self.cfg.input.read();
        let artifact = self.transpile(src, "exec").map_err(|eart| {
            eart.warns.write_all_stderr();
            eart.errors
        })?;
        artifact.warns.write_all_stderr();
        path.set_extension(artifact.object.extension());
        let mut f = File::create(path).unwrap();
        f.write_all(artifact.object.code().as_bytes()).unwrap();
        Ok(ExitStatus::compile_passed(artifact.warns.len()))
    }

    fn eval(&mut self, src: String) -> Result<String, CompileErrors> {
        let artifact = self.transpile(src, "eval").map_err(|eart| {
            eart.warns.write_all_stderr();
            eart.errors
        })?;
        artifact.warns.write_all_stderr();
        Ok(artifact.object.into_code())
    }

    fn expect_block(&self, src: &str) -> BlockKind {
        let mut parser = ParserRunner::new(self.cfg().clone());
        match parser.eval(src.to_string()) {
            Err(errs) => {
                let kind = errs
                    .iter()
                    .filter(|e| e.core().kind == ErrorKind::ExpectNextLine)
                    .map(|e| {
                        let msg = e.core().sub_messages.last().unwrap();
                        // ExpectNextLine error must have msg otherwise it's a bug
                        msg.get_msg().first().unwrap().to_owned()
                    })
                    .next();
                if let Some(kind) = kind {
                    return BlockKind::from(kind.as_str());
                }
                if errs
                    .iter()
                    .any(|err| err.core.main_message.contains("\"\"\""))
                {
                    return BlockKind::MultiLineStr;
                }
                BlockKind::Error
            }
            Ok(_) => {
                if src.contains("Class") {
                    return BlockKind::ClassDef;
                }
                BlockKind::None
            }
        }
    }
}

impl ContextProvider for Transpiler {
    fn dir(&self) -> HashMap<&VarName, &VarInfo> {
        self.builder.dir()
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.builder.get_receiver_ctx(receiver_name)
    }

    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.builder.get_var_info(name)
    }
}

impl Buildable<TranspiledFile> for Transpiler {
    fn inherit(cfg: ErgConfig, shared: SharedCompilerResource) -> Self {
        let mod_name = Str::from(cfg.input.file_stem());
        Self::new_with_cache(cfg, mod_name, shared)
    }
    fn inherit_with_name(cfg: ErgConfig, mod_name: Str, shared: SharedCompilerResource) -> Self {
        Self::new_with_cache(cfg, mod_name, shared)
    }
    fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<TranspiledFile>, IncompleteArtifact<TranspiledFile>> {
        self.transpile(src, mode)
            .map_err(|err| IncompleteArtifact::new(None, err.errors, err.warns))
    }
    fn build_from_ast(
        &mut self,
        ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact<TranspiledFile>, IncompleteArtifact<TranspiledFile>> {
        self.transpile_from_ast(ast, mode)
            .map_err(|err| IncompleteArtifact::new(None, err.errors, err.warns))
    }
    fn pop_context(&mut self) -> Option<ModuleContext> {
        self.builder.pop_context()
    }
    fn get_context(&self) -> Option<&ModuleContext> {
        self.builder.get_context()
    }
}

impl BuildRunnable<TranspiledFile> for Transpiler {}

impl Transpiler {
    pub fn new(cfg: ErgConfig) -> Self {
        New::new(cfg)
    }

    pub fn new_with_cache(cfg: ErgConfig, mod_name: Str, shared: SharedCompilerResource) -> Self {
        Self {
            shared: shared.clone(),
            builder: PackageBuilder::new_with_cache(cfg.copy(), mod_name, shared),
            script_generator: PyScriptGenerator::new(),
            cfg,
        }
    }

    pub fn transpile(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<TranspiledFile>, ErrorArtifact> {
        log!(info "the transpiling process has started.");
        let artifact = self.build_link_desugar(src, mode)?;
        let file = self.lower(artifact.object)?;
        log!(info "code:\n{}", file.code());
        log!(info "the transpiling process has completed");
        Ok(CompleteArtifact::new(file, artifact.warns))
    }

    pub fn transpile_from_ast(
        &mut self,
        ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact<TranspiledFile>, ErrorArtifact> {
        log!(info "the transpiling process has started.");
        let artifact = self.builder.build_from_ast(ast, mode)?;
        let file = self.lower(artifact.object)?;
        log!(info "code:\n{}", file.code());
        log!(info "the transpiling process has completed");
        Ok(CompleteArtifact::new(file, artifact.warns))
    }

    fn lower(&mut self, hir: HIR) -> CompileResult<TranspiledFile> {
        match self.cfg.transpile_target {
            Some(TranspileTarget::Json) => {
                let mut gen = JsonGenerator::new(self.cfg.copy());
                Ok(TranspiledFile::Json(gen.transpile(hir)?))
            }
            _ => Ok(TranspiledFile::PyScript(
                self.script_generator.transpile(hir),
            )),
        }
    }

    pub fn transpile_module(&mut self) -> Result<CompleteArtifact<TranspiledFile>, ErrorArtifact> {
        let src = self.cfg.input.read();
        self.transpile(src, "exec")
    }

    fn build_link_desugar(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, ErrorArtifact> {
        let artifact = self.builder.build(src, mode)?;
        self.link_desugar(artifact)
    }

    fn link_desugar(
        &mut self,
        artifact: CompleteArtifact,
    ) -> Result<CompleteArtifact, ErrorArtifact> {
        let linker = HIRLinker::new(&self.cfg, &self.shared.mod_cache);
        let hir = linker.link(artifact.object);
        let desugared = HIRDesugarer::desugar(hir);
        Ok(CompleteArtifact::new(desugared, artifact.warns))
    }

    pub fn pop_mod_ctx(&mut self) -> Option<ModuleContext> {
        self.builder.pop_context()
    }

    pub fn dir(&mut self) -> HashMap<&VarName, &VarInfo> {
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
pub struct PyScriptGenerator {
    globals: HashSet<String>,
    level: usize,
    fresh_var_n: usize,
    namedtuple_loaded: bool,
    mutate_op_loaded: bool,
    contains_op_loaded: bool,
    range_ops_loaded: bool,
    builtin_types_loaded: bool,
    builtin_control_loaded: bool,
    convertors_loaded: bool,
    prelude: String,
}

impl PyScriptGenerator {
    pub fn new() -> Self {
        Self {
            globals: HashSet::new(),
            level: 0,
            fresh_var_n: 0,
            namedtuple_loaded: false,
            mutate_op_loaded: false,
            contains_op_loaded: false,
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
        src.replace("from _erg_nat import NatMut", "")
            .replace("from _erg_nat import Nat", "")
            .replace("from _erg_int import IntMut", "")
            .replace("from _erg_int import Int", "")
            .replace("from _erg_bool import BoolMut", "")
            .replace("from _erg_bool import Bool", "")
            .replace("from _erg_str import StrMut", "")
            .replace("from _erg_str import Str", "")
            .replace("from _erg_float import FloatMut", "")
            .replace("from _erg_float import Float", "")
            .replace("from _erg_list import List", "")
            .replace("from _erg_range import Range", "")
            .replace("from _erg_result import Error", "")
            .replace("from _erg_result import is_ok", "")
            .replace("from _erg_control import then__", "")
            .replace("from _erg_contains_operator import contains_operator", "")
            .replace("from _erg_type import is_type", "")
            .replace("from _erg_type import _isinstance", "")
            .replace("from _erg_type import UnionType", "")
            .replace("from _erg_type import MutType", "")
    }

    fn load_namedtuple_if_not(&mut self) {
        if !self.namedtuple_loaded {
            self.prelude += "from collections import namedtuple as NamedTuple__\n";
            self.namedtuple_loaded = true;
        }
    }

    // TODO: name escaping
    fn load_range_ops_if_not(&mut self) {
        if !self.range_ops_loaded {
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_result.py"));
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_int.py"));
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_nat.py"));
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_str.py"));
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_range.py"));
            self.range_ops_loaded = true;
        }
    }

    fn load_contains_op_if_not(&mut self) {
        if !self.contains_op_loaded {
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_result.py"));
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_range.py"));
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_type.py"));
            self.prelude +=
                &Self::replace_import(include_str!("lib/core/_erg_contains_operator.py"));
            self.contains_op_loaded = true;
        }
    }

    fn load_mutate_op_if_not(&mut self) {
        if !self.mutate_op_loaded {
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_mutate_operator.py"));
            self.mutate_op_loaded = true;
        }
    }

    fn load_builtin_types_if_not(&mut self) {
        if !self.builtin_types_loaded {
            self.load_builtin_controls_if_not();
            self.load_contains_op_if_not();
            if self.range_ops_loaded {
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_float.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_list.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_dict.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_set.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_bytes.py"));
            } else {
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_int.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_nat.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_bool.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_str.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_float.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_list.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_dict.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_set.py"));
                self.prelude += &Self::replace_import(include_str!("lib/core/_erg_bytes.py"));
            }
            self.builtin_types_loaded = true;
        }
    }

    fn load_builtin_controls_if_not(&mut self) {
        if !self.builtin_control_loaded {
            self.prelude += include_str!("lib/core/_erg_control.py");
            self.builtin_control_loaded = true;
        }
    }

    fn load_convertors_if_not(&mut self) {
        if !self.convertors_loaded {
            self.prelude += &Self::replace_import(include_str!("lib/core/_erg_convertors.py"));
            self.convertors_loaded = true;
        }
    }

    fn escape_str(s: &str) -> String {
        s.replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
            // .replace('\'', "\\'")
            .replace('\0', "\\0")
    }

    fn transpile_expr(&mut self, expr: Expr) -> String {
        match expr {
            Expr::Literal(lit) => self.transpile_lit(lit),
            Expr::Call(call) => self.transpile_call(call),
            Expr::BinOp(bin) => self.transpile_binop(bin),
            Expr::UnaryOp(unary) => self.transpile_unaryop(unary),
            Expr::List(list) => match list {
                List::Normal(lis) => {
                    self.load_builtin_types_if_not();
                    let mut code = "List([".to_string();
                    for elem in lis.elems.pos_args {
                        code += &format!("{},", self.transpile_expr(elem.expr));
                    }
                    code += "])";
                    code
                }
                other => todo!("transpiling {other}"),
            },
            Expr::Set(set) => match set {
                Set::Normal(st) => {
                    self.load_builtin_types_if_not();
                    let mut code = "Set({".to_string();
                    for elem in st.elems.pos_args {
                        code += &format!("{},", self.transpile_expr(elem.expr));
                    }
                    code += "})";
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
                    self.load_builtin_types_if_not();
                    let mut code = "Dict({".to_string();
                    for kv in dic.kvs {
                        code += &format!(
                            "({}): ({}),",
                            self.transpile_expr(kv.key),
                            self.transpile_expr(kv.value)
                        );
                    }
                    code += "})";
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
            ValueObj::Bool(_)
                | ValueObj::Int(_)
                | ValueObj::Nat(_)
                | ValueObj::Str(_)
                | ValueObj::Float(_)
        ) {
            self.load_builtin_types_if_not();
            format!("{}({escaped})", lit.value.class())
        } else {
            escaped
        }
    }

    fn transpile_record(&mut self, rec: Record) -> String {
        self.load_namedtuple_if_not();
        let mut attrs = "[".to_string();
        let mut values = "(".to_string();
        for mut attr in rec.attrs.into_iter() {
            attrs += &format!("'{}',", Self::transpile_ident(attr.sig.into_ident()));
            if attr.body.block.len() > 1 {
                let name = format!("instant_block_{}__", self.fresh_var_n);
                self.fresh_var_n += 1;
                let mut instant = format!("def {name}():\n");
                instant += &self.transpile_block(attr.body.block, Return);
                self.prelude += &instant;
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
                self.load_range_ops_if_not();
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
            TokenKind::ContainsOp => {
                self.load_contains_op_if_not();
                let mut code = "contains_operator(".to_string();
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
            self.load_mutate_op_if_not();
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
        let mut prefix = "".to_string();
        match acc.ref_t().derefine() {
            v @ (Type::Bool | Type::Nat | Type::Int | Type::Float | Type::Str) => {
                self.load_builtin_types_if_not();
                prefix.push_str(&v.qual_name());
                prefix.push('(');
            }
            other => {
                if let t @ ("Bytes" | "List" | "Dict" | "Set") = &other.qual_name()[..] {
                    self.load_builtin_types_if_not();
                    prefix.push_str(t);
                    prefix.push('(');
                }
            }
        }
        let postfix = if prefix.is_empty() { "" } else { ")" };
        match acc {
            Accessor::Ident(ident) => {
                match &ident.inspect()[..] {
                    "Str" | "Bytes" | "Bool" | "Nat" | "Int" | "Float" | "List" | "Dict"
                    | "Set" | "Str!" | "Bytes!" | "Bool!" | "Nat!" | "Int!" | "Float!"
                    | "List!" => {
                        self.load_builtin_types_if_not();
                    }
                    "if" | "if!" | "for!" | "while" | "discard" => {
                        self.load_builtin_controls_if_not();
                    }
                    "int" | "nat" | "float" | "str" => {
                        self.load_convertors_if_not();
                    }
                    _ => {}
                }
                prefix + &Self::transpile_ident(ident) + postfix
            }
            Accessor::Attr(attr) => {
                if let Some(name) = debind(&attr.ident) {
                    demangle(&name)
                } else {
                    format!(
                        "{prefix}({}).{}{postfix}",
                        self.transpile_expr(*attr.obj),
                        Self::transpile_ident(attr.ident),
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
                let Expr::Lambda(block) = call.args.remove(0) else {
                    todo!()
                };
                let non_default = block.params.non_defaults.first().unwrap();
                let param_token = match &non_default.raw.pat {
                    ParamPattern::VarName(name) => name.token(),
                    ParamPattern::Discard(token) => token,
                    _ => unreachable!(),
                };
                code += &Self::transpile_name(
                    &VisibilityModifier::Private,
                    param_token.inspect(),
                    &non_default.vi,
                );
                code += &format!(" in {}:\n", self.transpile_expr(iter));
                code += &self.transpile_block(block.body, Discard);
                code
            }
            Some("while" | "while!") => {
                let mut code = "while ".to_string();
                let Expr::Lambda(mut cond) = call.args.remove(0) else {
                    todo!()
                };
                let Expr::Lambda(block) = call.args.remove(0) else {
                    todo!()
                };
                code += &format!("{}:\n", self.transpile_expr(cond.body.remove(0)));
                code += &self.transpile_block(block.body, Discard);
                code
            }
            Some("match" | "match!") => self.transpile_match(call),
            _ => self.transpile_simple_call(call),
        }
    }

    fn transpile_if(&mut self, mut call: Call) -> String {
        let cond = self.transpile_expr(call.args.remove(0));
        let Expr::Lambda(mut then_block) = call.args.remove(0) else {
            todo!()
        };
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
            let target = arm.params.non_defaults.first().unwrap();
            match &target.raw.pat {
                ParamPattern::VarName(param) => {
                    let param = Self::transpile_name(
                        &VisibilityModifier::Private,
                        param.inspect(),
                        &target.vi,
                    );
                    match target.raw.t_spec.as_ref().map(|t| &t.t_spec) {
                        Some(TypeSpec::Enum(enum_t)) => {
                            let values = ValueObj::vec_from_const_args(enum_t.clone());
                            let patterns = values
                                .iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(" | ");
                            code += &format!("case ({patterns}) as {param}:\n");
                        }
                        Some(other) => {
                            if let Some(Expr::Set(Set::Normal(set))) = &target.t_spec_as_expr {
                                let patterns = set
                                    .elems
                                    .pos_args
                                    .iter()
                                    .map(|elem| self.transpile_expr(elem.expr.clone()))
                                    .collect::<Vec<_>>()
                                    .join(" | ");
                                code += &format!("case ({patterns}) as {param}:\n");
                            } else {
                                todo!("{other}")
                            }
                        }
                        None => {
                            code += &format!("case {param}:\n");
                        }
                    }
                    code += &self.transpile_block(arm.body, StoreTmp(tmp.clone()));
                    self.level -= 1;
                }
                ParamPattern::Discard(_) => {
                    match target.raw.t_spec.as_ref().map(|t| &t.t_spec) {
                        Some(TypeSpec::Enum(enum_t)) => {
                            let values = ValueObj::vec_from_const_args(enum_t.clone());
                            let patterns = values
                                .iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(" | ");
                            code += &format!("case {patterns}:\n");
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
        self.level -= 1;
        format!("{tmp_func}()")
    }

    fn transpile_simple_call(&mut self, call: Call) -> String {
        let enc = if call.obj.ref_t().is_poly_type_meta() {
            Enclosure::Bracket
        } else {
            Enclosure::Paren
        };
        let is_py_api = if let Some(attr) = &call.attr_name {
            let is_py_api = attr.is_py_api();
            if let Some(name) = debind(attr) {
                let name = demangle(&name);
                return format!(
                    "{name}({}, {})",
                    self.transpile_expr(*call.obj),
                    self.transpile_args(call.args, is_py_api, enc)
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
        code += &self.transpile_args(call.args, is_py_api, enc);
        code
    }

    fn transpile_args(&mut self, mut args: Args, is_py_api: bool, enc: Enclosure) -> String {
        let mut code = String::new();
        code.push(enc.open());
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
        code.push(enc.close());
        code
    }

    fn transpile_ident(ident: Identifier) -> String {
        Self::transpile_name(ident.vis(), ident.inspect(), &ident.vi)
    }

    fn transpile_name(vis: &VisibilityModifier, name: &Str, vi: &VarInfo) -> String {
        if let Some(py_name) = &vi.py_name {
            return demangle(py_name);
        }
        let name = replace_non_symbolic(name);
        if vis.is_public() || &name == "_" {
            name.to_string()
        } else {
            let def_line = vi.def_loc.loc.ln_begin().unwrap_or(0);
            let def_col = vi.def_loc.loc.col_begin().unwrap_or(0);
            let line_mangling = match (def_line, def_col) {
                (0, 0) => "".to_string(),
                (0, _) => format!("_C{def_col}"),
                (_, 0) => format!("_L{def_line}"),
                (_, _) => format!("_L{def_line}_C{def_col}"),
            };
            format!("{name}{line_mangling}")
        }
    }

    fn transpile_params(&mut self, params: Params) -> String {
        let mut code = String::new();
        for non_default in params.non_defaults {
            match non_default.raw.pat {
                ParamPattern::VarName(param) => {
                    code += &Self::transpile_name(
                        &VisibilityModifier::Private,
                        param.inspect(),
                        &non_default.vi,
                    );
                    code += ",";
                }
                ParamPattern::Discard(_) => {
                    code += &format!("_{},", self.fresh_var_n);
                    self.fresh_var_n += 1;
                }
                _ => unreachable!(),
            }
        }
        for default in params.defaults {
            match default.sig.raw.pat {
                ParamPattern::VarName(param) => {
                    code += &format!(
                        "{} = {},",
                        Self::transpile_name(
                            &VisibilityModifier::Private,
                            param.inspect(),
                            &default.sig.vi
                        ),
                        self.transpile_expr(default.default_val),
                    );
                }
                ParamPattern::Discard(_) => {
                    let n = self.fresh_var_n;
                    code += &format!("_{n} = {},", self.transpile_expr(default.default_val),);
                    self.fresh_var_n += 1;
                }
                _ => unreachable!(),
            }
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
        // HACK: allow reference to local variables in tmp functions
        let mut code = if self.level == 0 {
            "".to_string()
        } else {
            let name = Self::transpile_ident(def.sig.ident().clone());
            if self.globals.contains(&name) {
                "".to_string()
            } else {
                self.globals.insert(name.clone());
                format!("global {name}\n{}", "    ".repeat(self.level))
            }
        };
        match def.sig {
            Signature::Var(var) => {
                code += &format!("{} = ", Self::transpile_ident(var.ident));
                if def.body.block.len() > 1 {
                    let name = format!("instant_block_{}__", self.fresh_var_n);
                    self.fresh_var_n += 1;
                    let mut instant = format!("def {name}():\n");
                    instant += &self.transpile_block(def.body.block, Return);
                    self.prelude += &instant;
                    code + &format!("{name}()")
                } else {
                    let expr = def.body.block.remove(0);
                    code += &self.transpile_expr(expr);
                    code
                }
            }
            Signature::Subr(subr) => {
                code += &format!(
                    "def {}({}):\n",
                    Self::transpile_ident(subr.ident),
                    self.transpile_params(subr.params)
                );
                code += &self.transpile_block(def.body.block, Return);
                code
            }
            Signature::Glob(_) => todo!(),
        }
    }

    fn transpile_classdef(&mut self, classdef: ClassDef) -> String {
        let class_name = Self::transpile_ident(classdef.sig.into_ident());
        let mut code = format!("class {class_name}():\n");
        let mut init_method = format!(
            "{}def __init__(self, param__):\n",
            "    ".repeat(self.level + 1)
        );
        match classdef.constructor.non_default_params().unwrap()[0].typ() {
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
        let methods = ClassDef::take_all_methods(classdef.methods_list);
        code += &self.transpile_block(methods, Discard);
        code
    }

    fn transpile_patchdef(&mut self, patch_def: PatchDef) -> String {
        let mut code = String::new();
        for chunk in patch_def.methods.into_iter() {
            let Expr::Def(mut def) = chunk else { todo!() };
            let name = format!(
                "{}{}",
                demangle(&patch_def.sig.ident().to_string_notype()),
                demangle(&def.sig.ident().to_string_notype()),
            );
            def.sig.ident_mut().raw.name = VarName::from_str(Str::from(name));
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
            let mut instant = format!("def {name}():\n");
            instant += &self.transpile_block(redef.block, Return);
            self.prelude += &instant;
            code + &format!("{name}()")
        } else {
            let expr = redef.block.remove(0);
            code += &self.transpile_expr(expr);
            code
        }
    }
}

#[derive(Debug, Default)]
pub struct JsonGenerator {
    cfg: ErgConfig,
    binds: HashMap<AbsLocation, ValueObj>,
    errors: CompileErrors,
}

impl JsonGenerator {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            binds: HashMap::new(),
            errors: CompileErrors::empty(),
        }
    }

    pub fn transpile(&mut self, hir: HIR) -> CompileResult<Json> {
        let mut code = "".to_string();
        let mut len = 0;
        for (i, chunk) in hir.module.into_iter().enumerate() {
            if i > 0 && len > 0 {
                code += ",\n";
            }
            let expr = self.transpile_expr(chunk);
            len = expr.len();
            code += &expr;
        }
        if self.errors.is_empty() {
            Ok(Json {
                filename: hir.name,
                code: format!("{{\n{code}\n}}"),
            })
        } else {
            Err(self.errors.take_all().into())
        }
    }

    fn expr_into_value(&self, expr: Expr) -> Option<ValueObj> {
        match expr {
            Expr::List(List::Normal(lis)) => {
                let mut vals = vec![];
                for elem in lis.elems.pos_args {
                    if let Some(val) = self.expr_into_value(elem.expr) {
                        vals.push(val);
                    } else {
                        return None;
                    }
                }
                Some(ValueObj::List(vals.into()))
            }
            Expr::List(List::WithLength(lis)) => {
                let len = lis
                    .len
                    .and_then(|len| self.expr_into_value(*len))
                    .and_then(|v| usize::try_from(&v).ok())?;
                let vals = vec![self.expr_into_value(*lis.elem)?; len];
                Some(ValueObj::List(vals.into()))
            }
            Expr::Tuple(Tuple::Normal(tup)) => {
                let mut vals = vec![];
                for elem in tup.elems.pos_args {
                    if let Some(val) = self.expr_into_value(elem.expr) {
                        vals.push(val);
                    } else {
                        return None;
                    }
                }
                Some(ValueObj::Tuple(vals.into()))
            }
            Expr::Dict(Dict::Normal(dic)) => {
                let mut kvs = dict! {};
                for kv in dic.kvs {
                    let key = self.expr_into_value(kv.key)?;
                    let val = self.expr_into_value(kv.value)?;
                    kvs.insert(key, val);
                }
                Some(ValueObj::Dict(kvs))
            }
            Expr::Record(rec) => {
                let mut attrs = dict! {};
                for mut attr in rec.attrs {
                    let field = Field::from(attr.sig.ident());
                    let val = self.expr_into_value(attr.body.block.remove(0))?;
                    attrs.insert(field, val);
                }
                Some(ValueObj::Record(attrs))
            }
            Expr::Literal(lit) => Some(lit.value),
            Expr::Accessor(acc) => self.binds.get(&acc.var_info().def_loc).cloned(),
            Expr::BinOp(bin) => {
                let lhs = self.expr_into_value(*bin.lhs)?;
                let rhs = self.expr_into_value(*bin.rhs)?;
                lhs.try_binary(rhs, OpKind::try_from(bin.op.kind).ok()?)
            }
            _ => None,
        }
    }

    fn transpile_def(&mut self, mut def: Def) -> String {
        self.register_def(&def);
        if def.sig.vis().is_public() {
            let expr = self.transpile_expr(def.body.block.remove(0));
            format!("\"{}\": {expr}", def.sig.inspect())
        } else {
            "".to_string()
        }
    }

    fn register_def(&mut self, def: &Def) {
        if let Some(val) = def
            .body
            .block
            .first()
            .cloned()
            .and_then(|expr| self.expr_into_value(expr))
        {
            self.binds.insert(def.sig.ident().vi.def_loc.clone(), val);
        }
    }

    fn transpile_expr(&mut self, expr: Expr) -> String {
        match expr {
            Expr::Literal(lit) => lit.token.content.to_string(),
            Expr::Accessor(acc) => {
                if let Some(val) = self.binds.get(&acc.var_info().def_loc) {
                    val.to_string()
                } else {
                    replace_non_symbolic(&acc.to_string())
                }
            }
            Expr::List(list) => match list {
                List::Normal(lis) => {
                    let mut code = "[".to_string();
                    for (i, elem) in lis.elems.pos_args.into_iter().enumerate() {
                        if i > 0 {
                            code += ", ";
                        }
                        code += &self.transpile_expr(elem.expr);
                    }
                    code += "]";
                    code
                }
                other => todo!("{other}"),
            },
            Expr::Tuple(tup) => match tup {
                Tuple::Normal(tup) => {
                    let mut code = "[".to_string();
                    for (i, elem) in tup.elems.pos_args.into_iter().enumerate() {
                        if i > 0 {
                            code += ", ";
                        }
                        code += &self.transpile_expr(elem.expr);
                    }
                    code += "]";
                    code
                }
            },
            Expr::Record(rec) => {
                let mut code = "".to_string();
                for (i, mut attr) in rec.attrs.into_iter().enumerate() {
                    if i > 0 {
                        code += ", ";
                    }
                    let expr = self.transpile_expr(attr.body.block.remove(0));
                    code += &format!("\"{}\": {expr}", attr.sig.inspect());
                }
                format!("{{{code}}}")
            }
            Expr::Dict(dic) => match dic {
                Dict::Normal(dic) => {
                    let mut code = "".to_string();
                    for (i, kv) in dic.kvs.into_iter().enumerate() {
                        if i > 0 {
                            code += ", ";
                        }
                        code += &format!(
                            "{}: {}",
                            self.transpile_expr(kv.key),
                            self.transpile_expr(kv.value)
                        );
                    }
                    format!("{{{code}}}")
                }
                Dict::Comprehension(other) => todo!("{other}"),
            },
            Expr::Def(def) => self.transpile_def(def),
            other => {
                let loc = other.loc();
                if let Some(val) = self.expr_into_value(other) {
                    val.to_string()
                } else {
                    self.errors.push(CompileError::not_const_expr(
                        self.cfg.input.clone(),
                        line!() as usize,
                        loc,
                        "".into(),
                    ));
                    "".to_string()
                }
            }
        }
    }
}
