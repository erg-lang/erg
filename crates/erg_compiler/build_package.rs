use std::path::Path;

use erg_common::config::ErgConfig;
use erg_common::consts::ELS;
use erg_common::debug_power_assert;
use erg_common::dict::Dict;
use erg_common::error::MultiErrorDisplay;
use erg_common::io::Input;
#[allow(unused)]
use erg_common::log;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::spawn::spawn_new_thread;
use erg_common::str::Str;
use erg_common::traits::{ExitStatus, Runnable, Stream};

use erg_parser::ast::{Expr, InlineModule, VarName, AST};
use erg_parser::build_ast::ASTBuilder;
use erg_parser::parse::SimpleParser;

use crate::artifact::{
    BuildRunnable, Buildable, CompleteArtifact, ErrorArtifact, IncompleteArtifact,
};
use crate::context::{Context, ContextProvider, ModuleContext};
use crate::error::{CompileError, CompileErrors};
use crate::module::SharedCompilerResource;
use crate::ty::ValueObj;
use crate::varinfo::VarInfo;
use crate::HIRBuilder;

#[derive(Debug)]
pub enum ResolveError {
    CycleDetected {
        path: NormalizedPathBuf,
        submod_input: Input,
    },
}

pub type ResolveResult<T> = Result<T, ResolveError>;

/// Resolve dependencies and build a package.
/// This object should be a singleton.
///
/// Invariant condition: `build_module` must be idempotent.
/// That is, the only thing that may differ as a result of analyzing the same package is the elapsed time.
#[derive(Debug)]
pub struct PackageBuilder<Builder: Buildable = HIRBuilder> {
    cfg: ErgConfig,
    shared: SharedCompilerResource,
    pub(crate) main_builder: Builder,
    cyclic: Vec<NormalizedPathBuf>,
    submodules: Vec<NormalizedPathBuf>,
    asts: Dict<NormalizedPathBuf, (Str, AST)>,
    parse_errors: ErrorArtifact,
}

impl<Builder: Buildable> Default for PackageBuilder<Builder> {
    fn default() -> Self {
        let cfg = ErgConfig::default();
        PackageBuilder::new(cfg.copy(), SharedCompilerResource::new(cfg))
    }
}

impl<Builder: BuildRunnable> Runnable for PackageBuilder<Builder> {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg package builder";

    fn new(cfg: ErgConfig) -> Self {
        PackageBuilder::new(cfg.copy(), SharedCompilerResource::new(cfg))
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        self.main_builder.cfg()
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        self.main_builder.cfg_mut()
    }

    #[inline]
    fn finish(&mut self) {
        self.main_builder.finish();
    }

    fn initialize(&mut self) {
        self.main_builder.initialize();
    }

    fn clear(&mut self) {
        self.main_builder.clear();
        // don't initialize the ownership checker
    }

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let src = self.cfg_mut().input.read();
        let artifact = self.build(src, "exec").map_err(|arti| arti.errors)?;
        artifact.warns.write_all_stderr();
        println!("{}", artifact.object);
        Ok(ExitStatus::compile_passed(artifact.warns.len()))
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let artifact = self.build(src, "eval").map_err(|arti| arti.errors)?;
        artifact.warns.write_all_stderr();
        Ok(artifact.object.to_string())
    }
}

impl<Builder: Buildable> Buildable for PackageBuilder<Builder> {
    fn inherit(cfg: ErgConfig, shared: SharedCompilerResource) -> Self {
        let mod_name = Str::from(cfg.input.file_stem());
        Self::new_with_cache(cfg, mod_name, shared)
    }
    fn inherit_with_name(cfg: ErgConfig, mod_name: Str, shared: SharedCompilerResource) -> Self {
        Self::new_with_cache(cfg, mod_name, shared)
    }
    fn build(&mut self, src: String, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        self.build(src, mode)
    }
    fn build_from_ast(
        &mut self,
        ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact<crate::hir::HIR>, IncompleteArtifact<crate::hir::HIR>> {
        self.build_module(ast, mode)
    }
    fn pop_context(&mut self) -> Option<ModuleContext> {
        self.main_builder.pop_context()
    }
    fn get_context(&self) -> Option<&ModuleContext> {
        self.main_builder.get_context()
    }
}

impl<Builder: BuildRunnable + 'static> BuildRunnable for PackageBuilder<Builder> {}

impl<Builder: Buildable + ContextProvider> ContextProvider for PackageBuilder<Builder> {
    fn dir(&self) -> Dict<&VarName, &VarInfo> {
        self.main_builder.dir()
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.main_builder.get_receiver_ctx(receiver_name)
    }

    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.main_builder.get_var_info(name)
    }
}

impl<Builder: Buildable> PackageBuilder<Builder> {
    pub fn new(cfg: ErgConfig, shared: SharedCompilerResource) -> Self {
        Self::new_with_cache(cfg, "<module>".into(), shared)
    }

    /// For batch compilation mode, `mod_name` of the entry point must be `<module>`.
    ///
    /// For ELS mode, `mod_name` must be the file name of the entry point.
    pub fn new_with_cache(cfg: ErgConfig, mod_name: Str, shared: SharedCompilerResource) -> Self {
        Self {
            cfg: cfg.copy(),
            shared: shared.clone(),
            main_builder: Builder::inherit_with_name(cfg, mod_name, shared),
            cyclic: vec![],
            submodules: vec![],
            asts: Dict::new(),
            parse_errors: ErrorArtifact::new(CompileErrors::empty(), CompileErrors::empty()),
        }
    }

    pub fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let artifact = ast_builder
            .build(src)
            .map_err(|err| IncompleteArtifact::new(None, err.errors.into(), err.warns.into()))?;
        self.build_module(artifact.ast, mode)
    }

    pub fn build_module(
        &mut self,
        mut ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let cfg = self.cfg.copy();
        log!(info "Start dependency resolution process");
        let _ = self.resolve(&mut ast, &cfg);
        log!(info "Dependency resolution process completed");
        if self.parse_errors.errors.is_empty() {
            self.shared.warns.extend(self.parse_errors.warns.flush());
        } else {
            return Err(IncompleteArtifact::new(
                None,
                self.parse_errors.errors.flush(),
                self.parse_errors.warns.flush(),
            ));
        }
        self.execute(ast, mode)
    }

    /// Analyze ASTs and make the dependencies graph.
    /// If circular dependencies are found, inline submodules to eliminate the circularity.
    fn resolve(&mut self, ast: &mut AST, cfg: &ErgConfig) -> ResolveResult<()> {
        let mut result = Ok(());
        for chunk in ast.module.iter_mut() {
            if let Err(err) = self.check_import(chunk, cfg) {
                result = Err(err);
            }
        }
        result
    }

    fn check_import(&mut self, expr: &mut Expr, cfg: &ErgConfig) -> ResolveResult<()> {
        let mut result = Ok(());
        match expr {
            Expr::Call(call) if call.additional_operation().is_some_and(|op| op.is_import()) => {
                if let Err(err) = self.register(expr, cfg) {
                    result = Err(err);
                }
            }
            Expr::Def(def) => {
                for expr in def.body.block.iter_mut() {
                    if let Err(err) = self.check_import(expr, cfg) {
                        result = Err(err);
                    }
                }
            }
            _ => {}
        }
        result
    }

    fn register(&mut self, expr: &mut Expr, cfg: &ErgConfig) -> ResolveResult<()> {
        let Expr::Call(call) = expr else {
            unreachable!()
        };
        let Some(Expr::Literal(mod_name)) = call.args.get_left_or_key("Path") else {
            return Ok(());
        };
        let Ok(mod_name) = crate::hir::Literal::try_from(mod_name.token.clone()) else {
            return Ok(());
        };
        let ValueObj::Str(__name__) = &mod_name.value else {
            return Ok(());
        };
        let import_path = match cfg.input.resolve_path(Path::new(&__name__[..])) {
            Some(path) => path,
            None => {
                // error will be reported in `Context::import_erg_mod`
                return Ok(());
            }
        };
        let from_path = NormalizedPathBuf::from(cfg.input.path());
        let mut import_cfg = cfg.inherit(import_path.clone());
        let Ok(src) = import_cfg.input.try_read() else {
            return Ok(());
        };
        let import_path = NormalizedPathBuf::from(import_path.clone());
        self.shared.graph.add_node_if_none(&import_path);
        let mut ast_builder = ASTBuilder::new(cfg.copy());
        let mut ast = match ast_builder.build(src) {
            Ok(art) => {
                self.parse_errors
                    .warns
                    .extend(CompileErrors::from(art.warns));
                art.ast
            }
            Err(iart) => {
                self.parse_errors
                    .errors
                    .extend(CompileErrors::from(iart.errors));
                self.parse_errors
                    .warns
                    .extend(CompileErrors::from(iart.warns));
                if let Some(ast) = iart.ast {
                    ast
                } else {
                    return Ok(());
                }
            }
        };
        // root -> a -> b -> a
        // b: submodule
        if self
            .shared
            .graph
            .inc_ref(&from_path, import_path.clone())
            .is_err()
        {
            self.submodules.push(from_path.clone());
            return Err(ResolveError::CycleDetected {
                path: import_path,
                submod_input: cfg.input.clone(),
            });
        }
        if import_path == from_path
            || self.submodules.contains(&import_path)
            || self.asts.contains_key(&import_path)
        {
            return Ok(());
        }
        if let Err(ResolveError::CycleDetected { path, submod_input }) =
            self.resolve(&mut ast, &import_cfg)
        {
            *expr = Expr::InlineModule(InlineModule::new(submod_input.clone(), ast, call.clone()));
            if path != from_path {
                return Err(ResolveError::CycleDetected { path, submod_input });
            } else {
                self.cyclic.push(path);
                return Ok(());
            }
        }
        let prev = self.asts.insert(import_path, (__name__.clone(), ast));
        debug_assert!(prev.is_none());
        Ok(())
    }

    /// Launch the analysis processes in order according to the dependency graph.
    fn execute(&mut self, ast: AST, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        log!(info "Start to spawn dependencies processes");
        let path = NormalizedPathBuf::from(self.cfg.input.path());
        let mut graph = self.shared.graph.clone_inner();
        let mut ancestors = graph.ancestors(&path).into_vec();
        while let Some(ancestor) = ancestors.pop() {
            if self.cyclic.contains(&ancestor) || graph.ancestors(&ancestor).is_empty() {
                graph.remove(&ancestor);
                if let Some((__name__, ancestor_ast)) = self.asts.remove(&ancestor) {
                    self.start_analysis_process(ancestor_ast, __name__, ancestor);
                } else if !self.submodules.contains(&ancestor) {
                    panic!("not found: {ancestor}");
                }
            } else {
                ancestors.insert(0, ancestor);
            }
        }
        log!(info "All dependencies have started to analyze");
        debug_power_assert!(self.asts.len(), ==, 0);
        self.cyclic.clear();
        self.submodules.clear();
        self.main_builder.build_from_ast(ast, mode)
    }

    fn start_analysis_process(&self, ast: AST, __name__: Str, path: NormalizedPathBuf) {
        if self
            .main_builder
            .get_context()
            .is_some_and(|ctx| ctx.context.mod_registered(&path))
        {
            return;
        }
        // for cache comparing
        let raw_ast = if ELS {
            let mut cfg = self.cfg.inherit(path.to_path_buf());
            let src = cfg.input.read();
            SimpleParser::parse(src.clone())
                .ok()
                .map(|artifact| artifact.ast)
        } else {
            None
        };
        let name = __name__.clone();
        let _path = path.to_path_buf();
        let cfg = self.cfg.inherit(path.to_path_buf());
        let shared = self.shared.inherit(path.clone());
        let mode = if _path.to_string_lossy().ends_with(".d.er") {
            "declare"
        } else {
            "exec"
        };
        if mode == "declare" {
            self.build_decl_mod(ast, path);
            return;
        }
        let run = move || {
            let mut builder = HIRBuilder::new_with_cache(cfg, name, shared.clone());
            let cache = if mode == "exec" {
                &shared.mod_cache
            } else {
                &shared.py_mod_cache
            };
            match builder.check(ast, mode) {
                Ok(artifact) => {
                    cache.register(
                        _path.clone(),
                        raw_ast,
                        Some(artifact.object),
                        builder.pop_mod_ctx().unwrap(),
                    );
                    shared.warns.extend(artifact.warns);
                }
                Err(artifact) => {
                    cache.register(
                        _path.clone(),
                        raw_ast,
                        artifact.object,
                        builder.pop_mod_ctx().unwrap(),
                    );
                    shared.warns.extend(artifact.warns);
                    shared.errors.extend(artifact.errors);
                }
            }
        };
        let handle = spawn_new_thread(run, &__name__);
        self.shared.promises.insert(path, handle);
    }

    /// e.g. http.d/client.d.er -> http.client
    /// math.d.er -> math
    fn mod_name(&self, path: &Path) -> Str {
        let mut name = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_end_matches(".d.er")
            .to_string();
        for parent in path.components().rev().skip(1) {
            let parent = parent.as_os_str().to_str().unwrap();
            if parent.ends_with(".d") {
                name = parent.trim_end_matches(".d").to_string() + "." + &name;
            } else {
                break;
            }
        }
        Str::from(name)
    }

    /// FIXME: bug with inter-process sharing of type variables (pyimport "math")
    fn build_decl_mod(&self, ast: AST, path: NormalizedPathBuf) {
        let py_mod_cache = &self.shared.py_mod_cache;
        let mut cfg = self.cfg.inherit(path.to_path_buf());
        let raw_ast = if ELS {
            let src = cfg.input.read();
            SimpleParser::parse(src.clone())
                .ok()
                .map(|artifact| artifact.ast)
        } else {
            None
        };
        let mut builder =
            HIRBuilder::new_with_cache(cfg, self.mod_name(&path), self.shared.clone());
        match builder.check(ast, "declare") {
            Ok(artifact) => {
                let ctx = builder.pop_mod_ctx().unwrap();
                py_mod_cache.register(path.clone(), raw_ast, Some(artifact.object), ctx);
                self.shared.warns.extend(artifact.warns);
            }
            Err(artifact) => {
                let ctx = builder.pop_mod_ctx().unwrap();
                py_mod_cache.register(path, raw_ast, artifact.object, ctx);
                self.shared.warns.extend(artifact.warns);
                self.shared.errors.extend(artifact.errors);
            }
        }
    }
}
