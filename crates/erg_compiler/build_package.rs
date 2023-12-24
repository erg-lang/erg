use std::ffi::OsStr;
use std::fmt;
use std::fs::{metadata, remove_file, File};
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::option::Option;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use erg_common::config::ErgMode;

use erg_common::config::ErgConfig;
use erg_common::consts::ELS;
use erg_common::debug_power_assert;
use erg_common::dict::Dict;
use erg_common::env::is_std_decl_path;
use erg_common::error::MultiErrorDisplay;
use erg_common::io::Input;
#[allow(unused)]
use erg_common::log;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::spawn::spawn_new_thread;
use erg_common::str::Str;
use erg_common::traits::{ExitStatus, New, Runnable, Stream};

use erg_parser::ast::{Expr, InlineModule, VarName, AST};
use erg_parser::build_ast::{ASTBuildable, ASTBuilder as DefaultASTBuilder};
use erg_parser::parse::SimpleParser;

use crate::artifact::{
    BuildRunnable, Buildable, CompleteArtifact, ErrorArtifact, IncompleteArtifact,
};
use crate::context::{Context, ContextProvider, ModuleContext};
use crate::error::{CompileError, CompileErrors};
use crate::lower::GenericASTLowerer;
use crate::module::SharedCompilerResource;
use crate::ty::ValueObj;
use crate::varinfo::VarInfo;
use crate::GenericHIRBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckStatus {
    Succeed,
    Failed,
    Ongoing,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckStatus::Succeed => write!(f, "succeed"),
            CheckStatus::Failed => write!(f, "failed"),
            CheckStatus::Ongoing => write!(f, "ongoing"),
        }
    }
}

impl std::str::FromStr for CheckStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "succeed" => Ok(CheckStatus::Succeed),
            "failed" => Ok(CheckStatus::Failed),
            "ongoing" => Ok(CheckStatus::Ongoing),
            _ => Err(format!("invalid status: {s}")),
        }
    }
}

/// format:
/// ```python
/// #[pylyzer] succeed foo.py 1234567890
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PylyzerStatus {
    pub status: CheckStatus,
    pub file: PathBuf,
    pub timestamp: SystemTime,
    pub hash: u64,
}

impl fmt::Display for PylyzerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "##[pylyzer] {} {} {} {}",
            self.status,
            self.file.display(),
            self.timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            self.hash,
        )
    }
}

impl std::str::FromStr for PylyzerStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split_whitespace();
        let pylyzer = iter.next().ok_or("no pylyzer")?;
        if pylyzer != "##[pylyzer]" {
            return Err("not pylyzer".to_string());
        }
        let status = iter.next().ok_or("no succeed")?;
        let status = status.parse()?;
        let file = iter.next().ok_or("no file")?;
        let file = PathBuf::from(file);
        let timestamp = iter.next().ok_or("no timestamp")?;
        let timestamp = SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(
                timestamp
                    .parse()
                    .map_err(|e| format!("timestamp parse error: {e}"))?,
            ))
            .ok_or("timestamp overflow")?;
        let hash = iter.next().ok_or("no hash")?;
        let hash = hash.parse().map_err(|e| format!("hash parse error: {e}"))?;
        Ok(PylyzerStatus {
            status,
            file,
            timestamp,
            hash,
        })
    }
}

enum Availability {
    Available,
    InProgress,
    NotFound,
    Unreadable,
    OutOfDate,
}

use Availability::*;

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
pub struct GenericPackageBuilder<
    ASTBuilder: ASTBuildable = DefaultASTBuilder,
    HIRBuilder: Buildable = GenericHIRBuilder,
> {
    cfg: ErgConfig,
    shared: SharedCompilerResource,
    pub(crate) main_builder: HIRBuilder,
    cyclic: Vec<NormalizedPathBuf>,
    submodules: Vec<NormalizedPathBuf>,
    asts: Dict<NormalizedPathBuf, (Str, AST)>,
    parse_errors: ErrorArtifact,
    _parser: PhantomData<fn() -> ASTBuilder>,
}

pub type PackageBuilder = GenericPackageBuilder<DefaultASTBuilder, GenericHIRBuilder>;
pub type PackageTypeChecker =
    GenericPackageBuilder<DefaultASTBuilder, GenericASTLowerer<DefaultASTBuilder>>;

impl<ASTBuilder: ASTBuildable, HIRBuilder: Buildable> Default
    for GenericPackageBuilder<ASTBuilder, HIRBuilder>
{
    fn default() -> Self {
        let cfg = ErgConfig::default();
        GenericPackageBuilder::new(cfg.copy(), SharedCompilerResource::new(cfg))
    }
}

impl<A: ASTBuildable, H: BuildRunnable> New for GenericPackageBuilder<A, H> {
    fn new(cfg: ErgConfig) -> Self {
        GenericPackageBuilder::new(cfg.copy(), SharedCompilerResource::new(cfg))
    }
}

impl<ASTBuilder: ASTBuildable, HIRBuilder: BuildRunnable> Runnable
    for GenericPackageBuilder<ASTBuilder, HIRBuilder>
{
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg package builder";

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

impl<ASTBuilder: ASTBuildable, HIRBuilder: Buildable> Buildable
    for GenericPackageBuilder<ASTBuilder, HIRBuilder>
{
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
        self.build_root(ast, mode)
    }
    fn pop_context(&mut self) -> Option<ModuleContext> {
        self.main_builder.pop_context()
    }
    fn get_context(&self) -> Option<&ModuleContext> {
        self.main_builder.get_context()
    }
}

impl<ASTBuilder: ASTBuildable + 'static, HIRBuilder: BuildRunnable + 'static> BuildRunnable
    for GenericPackageBuilder<ASTBuilder, HIRBuilder>
{
}

impl<ASTBuilder: ASTBuildable, HIRBuilder: Buildable + ContextProvider> ContextProvider
    for GenericPackageBuilder<ASTBuilder, HIRBuilder>
{
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

impl<ASTBuilder: ASTBuildable, HIRBuilder: Buildable>
    GenericPackageBuilder<ASTBuilder, HIRBuilder>
{
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
            main_builder: HIRBuilder::inherit_with_name(cfg, mod_name, shared),
            cyclic: vec![],
            submodules: vec![],
            asts: Dict::new(),
            parse_errors: ErrorArtifact::new(CompileErrors::empty(), CompileErrors::empty()),
            _parser: PhantomData,
        }
    }

    pub fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let artifact = ast_builder
            .build_ast(src)
            .map_err(|err| IncompleteArtifact::new(None, err.errors.into(), err.warns.into()))?;
        self.build_root(artifact.ast, mode)
    }

    pub fn build_module(&mut self) -> Result<CompleteArtifact, IncompleteArtifact> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let artifact = ast_builder
            .build_ast(self.cfg.input.read())
            .map_err(|err| IncompleteArtifact::new(None, err.errors.into(), err.warns.into()))?;
        self.build_root(artifact.ast, "exec")
    }

    pub fn build_root(
        &mut self,
        mut ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let cfg = self.cfg.copy();
        log!(info "Start dependency resolution process");
        let _ = self.resolve(&mut ast, &cfg);
        log!(info "Dependency resolution process completed");
        log!("graph:\n{}", self.shared.graph.display());
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
            Expr::Dummy(chunks) => {
                for chunk in chunks.iter_mut() {
                    if let Err(err) = self.check_import(chunk, cfg) {
                        result = Err(err);
                    }
                }
            }
            Expr::Compound(chunks) => {
                for chunk in chunks.iter_mut() {
                    if let Err(err) = self.check_import(chunk, cfg) {
                        result = Err(err);
                    }
                }
            }
            _ => {}
        }
        result
    }

    fn analysis_in_progress(path: &Path) -> bool {
        let Ok(meta) = metadata(path) else {
            return false;
        };
        !is_std_decl_path(path) && meta.len() == 0
    }

    fn availability(path: &Path) -> Availability {
        let Ok(file) = File::open(path) else {
            return Availability::NotFound;
        };
        if is_std_decl_path(path) {
            return Availability::Available;
        }
        let mut line = "".to_string();
        let Ok(_) = BufReader::new(file).read_line(&mut line) else {
            return Availability::Unreadable;
        };
        if line.is_empty() {
            return Availability::InProgress;
        }
        let Ok(status) = line.parse::<PylyzerStatus>() else {
            return Availability::Available;
        };
        let Some(meta) = metadata(&status.file).ok() else {
            return Availability::NotFound;
        };
        let dummy_hash = meta.len();
        if status.hash != dummy_hash {
            Availability::OutOfDate
        } else {
            Availability::Available
        }
    }

    fn try_gen_py_decl_file(&self, __name__: &Str) -> Result<PathBuf, ()> {
        if let Ok(path) = self.cfg.input.resolve_py(Path::new(&__name__[..])) {
            if self.cfg.input.path() == path.as_path() {
                return Ok(path);
            }
            let (out, err) = if self.cfg.mode == ErgMode::LanguageServer || self.cfg.quiet_repl {
                (Stdio::null(), Stdio::null())
            } else {
                (Stdio::inherit(), Stdio::inherit())
            };
            // pylyzer is a static analysis tool for Python (https://github.com/mtshiba/pylyzer).
            // It can convert a Python script to an Erg AST for code analysis.
            // There is also an option to output the analysis result as `d.er`. Use this if the system have pylyzer installed.
            // A type definition file may be generated even if not all type checks succeed.
            if let Ok(status) = Command::new("pylyzer")
                .arg("--dump-decl")
                .arg(path.to_str().unwrap())
                .stdout(out)
                .stderr(err)
                .spawn()
                .and_then(|mut child| child.wait())
            {
                if let Some(path) = self.cfg.input.resolve_decl_path(Path::new(&__name__[..])) {
                    let size = metadata(&path).unwrap().len();
                    // if pylyzer crashed
                    if !status.success() && size == 0 {
                        // The presence of the decl file indicates that the analysis is in progress or completed,
                        // so if pylyzer crashes in the middle of the analysis, delete the file.
                        remove_file(&path).unwrap();
                    } else {
                        return Ok(path);
                    }
                }
            }
        }
        Err(())
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
        if __name__ == "unsound"
            && call
                .additional_operation()
                .is_some_and(|op| op.is_erg_import())
        {
            if let Some(mod_ctx) = self.get_context() {
                mod_ctx.context.build_module_unsound();
            }
            return Ok(());
        }
        let path = Path::new(&__name__[..]);
        let import_path = match cfg.input.resolve_path(path) {
            Some(path) => path,
            None => {
                for _ in 0..600 {
                    if !Self::analysis_in_progress(path) {
                        break;
                    }
                    sleep(Duration::from_millis(100));
                }
                if matches!(Self::availability(path), OutOfDate | NotFound | Unreadable) {
                    if let Ok(path) = self.try_gen_py_decl_file(__name__) {
                        path
                    } else {
                        return Ok(());
                    }
                } else {
                    // error will be reported in `Context::import_erg_mod`
                    return Ok(());
                }
            }
        };
        let from_path = NormalizedPathBuf::from(cfg.input.path());
        let mut import_cfg = cfg.inherit(import_path.clone());
        let Ok(src) = import_cfg.input.try_read() else {
            return Ok(());
        };
        let import_path = NormalizedPathBuf::from(import_path.clone());
        self.shared.graph.add_node_if_none(&import_path);
        let result = if import_path.extension() == Some(OsStr::new("er")) {
            let mut ast_builder = DefaultASTBuilder::new(cfg.copy());
            ast_builder.build_ast(src)
        } else {
            let mut ast_builder = ASTBuilder::new(cfg.copy());
            ast_builder.build_ast(src)
        };
        let mut ast = match result {
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
            *expr = Expr::InlineModule(InlineModule::new(
                submod_input.clone(),
                ast,
                call.clone(),
                import_path,
            ));
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
            let mut builder = HIRBuilder::inherit_with_name(cfg, name, shared.clone());
            let cache = if mode == "exec" {
                &shared.mod_cache
            } else {
                &shared.py_mod_cache
            };
            match builder.build_from_ast(ast, mode) {
                Ok(artifact) => {
                    cache.register(
                        _path.clone(),
                        raw_ast,
                        Some(artifact.object),
                        builder.pop_context().unwrap(),
                    );
                    shared.warns.extend(artifact.warns);
                }
                Err(artifact) => {
                    cache.register(
                        _path.clone(),
                        raw_ast,
                        artifact.object,
                        builder.pop_context().unwrap(),
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
            HIRBuilder::inherit_with_name(cfg, self.mod_name(&path), self.shared.clone());
        match builder.build_from_ast(ast, "declare") {
            Ok(artifact) => {
                let ctx = builder.pop_context().unwrap();
                py_mod_cache.register(path.clone(), raw_ast, Some(artifact.object), ctx);
                self.shared.warns.extend(artifact.warns);
            }
            Err(artifact) => {
                let ctx = builder.pop_context().unwrap();
                py_mod_cache.register(path, raw_ast, artifact.object, ctx);
                self.shared.warns.extend(artifact.warns);
                self.shared.errors.extend(artifact.errors);
            }
        }
    }
}
