use std::ffi::OsStr;
use std::fmt;
use std::fs::{metadata, remove_file, File};
use std::io::{stdout, BufRead, BufReader, Write};
use std::marker::PhantomData;
use std::option::Option;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use erg_common::config::ErgMode;

use erg_common::config::ErgConfig;
use erg_common::consts::{ELS, ERG_MODE, PARALLEL};
use erg_common::debug_power_assert;
use erg_common::dict::Dict;
use erg_common::env::is_std_decl_path;
use erg_common::error::MultiErrorDisplay;
use erg_common::io::Input;
#[allow(unused)]
use erg_common::log;
use erg_common::pathutil::{mod_name, project_entry_dir_of, NormalizedPathBuf};
use erg_common::set::Set;
use erg_common::spawn::spawn_new_thread;
use erg_common::str::Str;
use erg_common::traits::{ExitStatus, New, Runnable, Stream};

use erg_common::vfs::VFS;
use erg_parser::ast::{
    ClassAttr, Expr, InlineModule, Module, Record, RecordAttrOrIdent, VarName, AST,
};
use erg_parser::build_ast::{ASTBuildable, ASTBuilder as DefaultASTBuilder};
use erg_parser::parse::SimpleParser;

use crate::artifact::{
    BuildRunnable, Buildable, CompleteArtifact, ErrorArtifact, IncompleteArtifact,
};
use crate::context::{Context, ContextProvider, ModuleContext};
use crate::error::{CompileError, CompileErrors};
use crate::lower::GenericASTLowerer;
use crate::module::{ModuleGraph, SharedCompilerResource};
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

impl CheckStatus {
    pub const fn is_succeed(&self) -> bool {
        matches!(self, CheckStatus::Succeed)
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
pub struct ResolveError {
    path: NormalizedPathBuf,
    _submod_input: Input,
}

pub type ResolveResult<T> = Result<T, Vec<ResolveError>>;

#[derive(Debug, Clone)]
pub struct ASTEntry {
    name: Str,
    ast: AST,
}

impl ASTEntry {
    pub const fn new(name: Str, ast: AST) -> Self {
        Self { name, ast }
    }
}

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
    // key: inlined module, value: inliner module (child)
    inlines: Dict<NormalizedPathBuf, NormalizedPathBuf>,
    asts: Dict<NormalizedPathBuf, ASTEntry>,
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
        self.finalize();
        self.main_builder.clear();
        // don't initialize the ownership checker
    }

    fn set_input(&mut self, input: Input) {
        self.cfg.input = input;
        self.main_builder.set_input(self.cfg.input.clone());
    }

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let src = self.cfg_mut().input.read();
        let artifact = self
            .build(src, self.cfg.input.mode())
            .map_err(|arti| arti.errors)?;
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
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
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
            inlines: Dict::new(),
            asts: Dict::new(),
            parse_errors: ErrorArtifact::new(CompileErrors::empty(), CompileErrors::empty()),
            _parser: PhantomData,
        }
    }

    pub fn shared(&self) -> &SharedCompilerResource {
        &self.shared
    }

    pub fn finalize(&mut self) {
        self.cyclic.clear();
        self.inlines.clear();
        self.asts.clear();
        self.parse_errors.clear();
    }

    pub fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let ast = match ast_builder.build_ast(src) {
            Ok(art) => art.ast,
            // continue analysis if ELS mode
            Err(iart) if self.cfg.mode == ErgMode::LanguageServer => {
                if let Some(ast) = iart.ast {
                    self.shared.warns.extend(iart.warns.into());
                    self.shared.errors.extend(iart.errors.into());
                    ast
                } else {
                    self.finalize();
                    return Err(IncompleteArtifact::new(
                        None,
                        iart.errors.into(),
                        iart.warns.into(),
                    ));
                }
            }
            Err(iart) => {
                self.finalize();
                return Err(IncompleteArtifact::new(
                    None,
                    iart.errors.into(),
                    iart.warns.into(),
                ));
            }
        };
        self.build_root(ast, mode)
    }

    pub fn build_module(&mut self) -> Result<CompleteArtifact, IncompleteArtifact> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let ast = match ast_builder.build_ast(self.cfg.input.read()) {
            Ok(art) => art.ast,
            Err(iart) if self.cfg.mode == ErgMode::LanguageServer => {
                if let Some(ast) = iart.ast {
                    self.shared.warns.extend(iart.warns.into());
                    self.shared.errors.extend(iart.errors.into());
                    ast
                } else {
                    self.finalize();
                    return Err(IncompleteArtifact::new(
                        None,
                        iart.errors.into(),
                        iart.warns.into(),
                    ));
                }
            }
            Err(iart) => {
                self.finalize();
                return Err(IncompleteArtifact::new(
                    None,
                    iart.errors.into(),
                    iart.warns.into(),
                ));
            }
        };
        self.build_root(ast, self.cfg.input.mode())
    }

    pub fn build_root(
        &mut self,
        mut ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let cfg = self.cfg.copy();
        log!(info "Start dependency resolution process");
        let res = self.resolve(&mut ast, &cfg);
        debug_assert!(res.is_ok(), "{:?}", res.unwrap_err());
        log!(info "Dependency resolution process completed");
        log!("graph:\n{}", self.shared.graph.display());
        self.shared.errors.extend(self.parse_errors.errors.flush());
        self.shared.warns.extend(self.parse_errors.warns.flush());
        self.execute(ast, mode)
    }

    /// Analyze ASTs and make the dependencies graph.
    /// If circular dependencies are found, inline submodules to eliminate the circularity.
    fn resolve(&mut self, ast: &mut AST, cfg: &ErgConfig) -> ResolveResult<()> {
        let mut errs = vec![];
        for chunk in ast.module.iter_mut() {
            if let Err(err) = self.check_import(chunk, cfg) {
                errs.extend(err);
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }

    fn check_import(&mut self, expr: &mut Expr, cfg: &ErgConfig) -> ResolveResult<()> {
        let mut errs = vec![];
        match expr {
            Expr::Call(call) => {
                for pos in call.args.pos_args.iter_mut() {
                    if let Err(err) = self.check_import(&mut pos.expr, cfg) {
                        errs.extend(err);
                    }
                }
                if let Some(var) = call.args.var_args.as_mut() {
                    if let Err(err) = self.check_import(&mut var.expr, cfg) {
                        errs.extend(err);
                    }
                }
                for kw in call.args.kw_args.iter_mut() {
                    if let Err(err) = self.check_import(&mut kw.expr, cfg) {
                        errs.extend(err);
                    }
                }
                if let Some(kw_var) = call.args.kw_var_args.as_mut() {
                    if let Err(err) = self.check_import(&mut kw_var.expr, cfg) {
                        errs.extend(err);
                    }
                }
                if call.additional_operation().is_some_and(|op| op.is_import()) {
                    if let Err(err) = self.register(expr, cfg) {
                        errs.extend(err);
                    }
                }
            }
            Expr::Def(def) => {
                for expr in def.body.block.iter_mut() {
                    if let Err(err) = self.check_import(expr, cfg) {
                        errs.extend(err);
                    }
                }
            }
            Expr::ClassDef(class_def) => {
                for expr in class_def.def.body.block.iter_mut() {
                    if let Err(err) = self.check_import(expr, cfg) {
                        errs.extend(err);
                    }
                }
                for methods in class_def.methods_list.iter_mut() {
                    for attr in methods.attrs.iter_mut() {
                        if let ClassAttr::Def(def) = attr {
                            for chunk in def.body.block.iter_mut() {
                                if let Err(err) = self.check_import(chunk, cfg) {
                                    errs.extend(err);
                                }
                            }
                        }
                    }
                }
            }
            Expr::Methods(methods) => {
                for attr in methods.attrs.iter_mut() {
                    if let ClassAttr::Def(def) = attr {
                        for chunk in def.body.block.iter_mut() {
                            if let Err(err) = self.check_import(chunk, cfg) {
                                errs.extend(err);
                            }
                        }
                    }
                }
            }
            Expr::PatchDef(patch_def) => {
                for expr in patch_def.def.body.block.iter_mut() {
                    if let Err(err) = self.check_import(expr, cfg) {
                        errs.extend(err);
                    }
                }
                for methods in patch_def.methods_list.iter_mut() {
                    for attr in methods.attrs.iter_mut() {
                        if let ClassAttr::Def(def) = attr {
                            for chunk in def.body.block.iter_mut() {
                                if let Err(err) = self.check_import(chunk, cfg) {
                                    errs.extend(err);
                                }
                            }
                        }
                    }
                }
            }
            Expr::Record(Record::Normal(rec)) => {
                for attr in rec.attrs.iter_mut() {
                    for chunk in attr.body.block.iter_mut() {
                        if let Err(err) = self.check_import(chunk, cfg) {
                            errs.extend(err);
                        }
                    }
                }
            }
            Expr::Record(Record::Mixed(rec)) => {
                for attr in rec.attrs.iter_mut() {
                    if let RecordAttrOrIdent::Attr(def) = attr {
                        for chunk in def.body.block.iter_mut() {
                            if let Err(err) = self.check_import(chunk, cfg) {
                                errs.extend(err);
                            }
                        }
                    }
                }
            }
            Expr::Lambda(lambda) => {
                for chunk in lambda.body.iter_mut() {
                    if let Err(err) = self.check_import(chunk, cfg) {
                        errs.extend(err);
                    }
                }
            }
            Expr::Dummy(chunks) => {
                for chunk in chunks.iter_mut() {
                    if let Err(err) = self.check_import(chunk, cfg) {
                        errs.extend(err);
                    }
                }
            }
            Expr::Compound(chunks) => {
                for chunk in chunks.iter_mut() {
                    if let Err(err) = self.check_import(chunk, cfg) {
                        errs.extend(err);
                    }
                }
            }
            _ => {}
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
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
            if self.cfg.use_pylyzer {
                let (out, err) = if self.cfg.mode == ErgMode::LanguageServer || self.cfg.quiet_repl
                {
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
                    .arg(path.to_str().unwrap_or_default())
                    .stdout(out)
                    .stderr(err)
                    .spawn()
                    .and_then(|mut child| child.wait())
                {
                    if let Some(path) = self
                        .cfg
                        .input
                        .resolve_decl_path(Path::new(&__name__[..]), &self.cfg)
                    {
                        let size = metadata(&path).or(Err(()))?.len();
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
        let resolved = if call.additional_operation().unwrap().is_erg_import() {
            cfg.input
                .resolve_real_path(path, cfg)
                .or_else(|| cfg.input.resolve_decl_path(path, cfg))
        } else {
            cfg.input
                .resolve_decl_path(path, cfg)
                .or_else(|| cfg.input.resolve_real_path(path, cfg))
        };
        VFS.cache_path(cfg.input.clone(), path.to_path_buf(), resolved.clone());
        let import_path = match resolved {
            Some(path) => path,
            None if ERG_MODE => {
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
            None => return Ok(()),
        };
        let from_path = NormalizedPathBuf::from(cfg.input.path());
        let import_cfg = cfg.inherit(import_path.clone());
        let import_path = NormalizedPathBuf::from(import_path.clone());
        self.shared.graph.add_node_if_none(&import_path);
        // If we import `foo/bar`, we also need to import `foo`
        let first = __name__.split('/').next().unwrap_or_default();
        let root_path = if !first.is_empty() && first != "." && first != &__name__[..] {
            Some(Path::new(first))
        } else {
            None
        };
        let root_import_path = root_path.and_then(|path| cfg.input.resolve_path(path, cfg));
        if let Some(root_import_path) = root_import_path.map(NormalizedPathBuf::from) {
            if project_entry_dir_of(&root_import_path) != project_entry_dir_of(&from_path) {
                let root_import_cfg = cfg.inherit(root_import_path.to_path_buf());
                self.shared.graph.add_node_if_none(&root_import_path);
                let _ = self
                    .shared
                    .graph
                    .inc_ref(&from_path, root_import_path.clone());
                if root_import_path == from_path
                    || self.inlines.contains_key(&root_import_path)
                    || self.asts.contains_key(&root_import_path)
                {
                    // pass
                } else if let Some(mut ast) = self.parse(&root_import_path) {
                    let _ = self.resolve(&mut ast, &root_import_cfg);
                    let entry = ASTEntry::new(__name__.clone(), ast);
                    let prev = self.asts.insert(root_import_path, entry);
                    debug_assert!(prev.is_none());
                }
            }
        }
        // root -> a -> b -> a
        // b: submodule
        if let Err(_err) = self.shared.graph.inc_ref(&from_path, import_path.clone()) {
            return Err(vec![ResolveError {
                path: import_path,
                _submod_input: cfg.input.clone(),
            }]);
        }
        if import_path == from_path
            || self.inlines.contains_key(&import_path)
            || self.asts.contains_key(&import_path)
        {
            return Ok(());
        }
        let mut ast = self
            .parse(&import_path)
            .unwrap_or_else(|| AST::new(__name__.clone(), Module::new(vec![])));
        if let Err(mut errs) = self.resolve(&mut ast, &import_cfg) {
            self.inlines.insert(import_path.clone(), from_path.clone());
            *expr = Expr::InlineModule(InlineModule::new(
                Input::file(import_path.to_path_buf()),
                ast,
                call.clone(),
            ));
            errs.retain(|ResolveError { path, .. }| path != &from_path);
            if errs.is_empty() {
                self.cyclic.push(from_path);
                return Ok(());
            } else {
                return Err(errs);
            }
        }
        let entry = ASTEntry::new(__name__.clone(), ast);
        let prev = self.asts.insert(import_path, entry);
        debug_assert!(prev.is_none());
        Ok(())
    }

    /// Parse the file and build the AST. It may return `Some()` even if there are errors.
    fn parse(&mut self, import_path: &NormalizedPathBuf) -> Option<AST> {
        let Ok(src) = import_path.try_read() else {
            return None;
        };
        let cfg = self.cfg.inherit(import_path.to_path_buf());
        let result = if import_path.extension() == Some(OsStr::new("er")) {
            let mut ast_builder = DefaultASTBuilder::new(cfg.copy());
            ast_builder.build_ast(src)
        } else {
            let mut ast_builder = ASTBuilder::new(cfg.copy());
            ast_builder.build_ast(src)
        };
        match result {
            Ok(art) => {
                self.parse_errors
                    .warns
                    .extend(CompileErrors::from(art.warns));
                Some(art.ast)
            }
            Err(iart) => {
                self.parse_errors
                    .errors
                    .extend(CompileErrors::from(iart.errors));
                self.parse_errors
                    .warns
                    .extend(CompileErrors::from(iart.warns));
                iart.ast
            }
        }
    }

    /// Launch the analysis processes in order according to the dependency graph.
    fn execute(&mut self, ast: AST, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        log!(info "Start to spawn dependencies processes");
        let root = NormalizedPathBuf::from(self.cfg.input.path());
        let mut graph = self.shared.graph.clone_inner();
        let deps = self.build_deps_and_module(&root, &mut graph);
        log!(info "All dependencies have started to analyze");
        debug_power_assert!(self.asts.len(), ==, 0);
        if self.cfg.mode != ErgMode::LanguageServer {
            for path in self.shared.graph.ancestors(&root) {
                assert!(
                    self.shared.promises.is_registered(&path),
                    "{path} is not registered"
                );
            }
        }
        self.finalize();
        match self.main_builder.build_from_ast(ast, mode) {
            Ok(artifact) => Ok(CompleteArtifact::new(
                artifact.object.with_dependencies(deps),
                artifact.warns,
            )),
            Err(artifact) => Err(IncompleteArtifact::new(
                artifact.object.map(|hir| hir.with_dependencies(deps)),
                artifact.errors,
                artifact.warns,
            )),
        }
    }

    fn build_deps_and_module(
        &mut self,
        path: &NormalizedPathBuf,
        graph: &mut ModuleGraph,
    ) -> Set<NormalizedPathBuf> {
        let mut deps = Set::new();
        let mut ancestors = graph.ancestors(path).cloned().into_vec();
        let nmods = ancestors.len();
        let pad = nmods.to_string().len();
        let print_progress =
            nmods > 0 && !self.cfg.mode.is_language_server() && self.cfg.verbose >= 2;
        if print_progress && !self.inlines.contains_key(path) {
            let mut out = stdout().lock();
            write!(out, "Checking 0/{nmods}").unwrap();
            out.flush().unwrap();
        }
        while let Some(ancestor) = ancestors.pop() {
            if graph
                .parents(&ancestor)
                .is_none_or(|parents| parents.is_empty())
            {
                graph.remove(&ancestor);
                if let Some(entry) = self.asts.remove(&ancestor) {
                    deps.insert(ancestor.clone());
                    if print_progress {
                        let name = ancestor.file_name().unwrap_or_default().to_string_lossy();
                        let checked = nmods - ancestors.len();
                        let percentage = (checked as f64 / nmods as f64) * 100.0;
                        let spaces = " ".repeat(((100.0 - percentage) / 5.0) as usize);
                        let eqs = "=".repeat((percentage / 5.0) as usize);
                        let mut out = stdout().lock();
                        write!(
                            out,
                            "\rChecking [{eqs}{spaces}] {checked:>pad$}/{nmods}: {name:<30}"
                        )
                        .unwrap();
                        out.flush().unwrap();
                    }
                    self.start_analysis_process(entry.ast, entry.name, ancestor);
                } else {
                    self.build_inlined_module(&ancestor, graph);
                }
            } else {
                ancestors.insert(0, ancestor);
            }
        }
        if print_progress {
            println!();
        }
        deps
    }

    // REVIEW: should return dep files?
    fn build_inlined_module(&mut self, path: &NormalizedPathBuf, graph: &mut ModuleGraph) {
        if self.shared.get_module(path).is_some() {
            // do nothing
        } else if self.shared.promises.is_registered(path) {
            self.shared.promises.wait_until_finished(path);
        } else if let Some(inliner) = self.inlines.get(path).cloned() {
            self.build_deps_and_module(&inliner, graph);
            self.shared.promises.mark_as_joined(path.clone());
        } else {
            unreachable!("{path} is not found in self.inlines and self.asts");
        }
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
        let mode = if _path.to_string_lossy().ends_with(".d.er")
            || _path.to_string_lossy().ends_with(".pyi")
        {
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
                        CheckStatus::Succeed,
                    );
                    shared.warns.extend(artifact.warns);
                }
                Err(artifact) => {
                    cache.register(
                        _path.clone(),
                        raw_ast,
                        artifact.object,
                        builder.pop_context().unwrap(),
                        CheckStatus::Failed,
                    );
                    shared.warns.extend(artifact.warns);
                    shared.errors.extend(artifact.errors);
                }
            }
        };
        if PARALLEL {
            let handle = spawn_new_thread(run, &__name__);
            self.shared.promises.insert(path, handle);
        } else {
            run();
            self.shared.promises.mark_as_joined(path);
        }
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
        let mut builder = HIRBuilder::inherit_with_name(cfg, mod_name(&path), self.shared.clone());
        match builder.build_from_ast(ast, "declare") {
            Ok(artifact) => {
                let ctx = builder.pop_context().unwrap();
                py_mod_cache.register(
                    path.clone(),
                    raw_ast,
                    Some(artifact.object),
                    ctx,
                    CheckStatus::Succeed,
                );
                self.shared.warns.extend(artifact.warns);
            }
            Err(artifact) => {
                let ctx = builder.pop_context().unwrap();
                py_mod_cache.register(
                    path.clone(),
                    raw_ast,
                    artifact.object,
                    ctx,
                    CheckStatus::Failed,
                );
                self.shared.warns.extend(artifact.warns);
                self.shared.errors.extend(artifact.errors);
            }
        }
        self.shared.promises.mark_as_joined(path);
    }
}
