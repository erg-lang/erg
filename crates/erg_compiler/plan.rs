use std::path::Path;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
#[allow(unused)]
use erg_common::log;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::spawn::spawn_new_thread;
use erg_common::traits::Stream;

use erg_parser::ast::{Expr, AST};

use erg_common::traits::{Locational, Runnable};
use erg_parser::build_ast::ASTBuilder;

use crate::artifact::{CompleteArtifact, ErrorArtifact, IncompleteArtifact};
use crate::error::{CompileError, CompileErrors};
use crate::module::SharedCompilerResource;
use crate::ty::ValueObj;
use crate::ty::{HasType, Type};
use crate::HIRBuilder;

#[derive(Debug)]
pub struct Submodules(Vec<AST>);

#[derive(Debug)]
pub struct Planner {
    cfg: ErgConfig,
    shared: SharedCompilerResource,
    pub(crate) builder: HIRBuilder,
    asts: Dict<NormalizedPathBuf, AST>,
    parse_errors: ErrorArtifact,
    sub_mods: Dict<NormalizedPathBuf, Submodules>,
}

impl Planner {
    pub fn new(cfg: ErgConfig, shared: SharedCompilerResource) -> Self {
        Self {
            cfg: cfg.copy(),
            shared: shared.clone(),
            builder: HIRBuilder::new_with_cache(cfg, "<module>", shared),
            asts: Dict::new(),
            parse_errors: ErrorArtifact::new(CompileErrors::empty(), CompileErrors::empty()),
            sub_mods: Dict::new(),
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
        ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let from_path = self.cfg.input.path().to_path_buf();
        self.plan(&ast, &from_path);
        if !self.parse_errors.errors.is_empty() {
            return Err(IncompleteArtifact::new(
                None,
                self.parse_errors.errors.flush(),
                self.parse_errors.warns.flush(),
            ));
        }
        self.execute(ast, mode)
    }

    fn plan(&mut self, ast: &AST, from_path: &Path) {
        for chunk in ast.module.iter() {
            self.check_import(chunk, from_path);
        }
    }

    fn check_import(&mut self, expr: &Expr, from_path: &Path) {
        match expr {
            Expr::Call(call) if call.additional_operation().is_some_and(|op| op.is_import()) => {
                let op = call.additional_operation().unwrap();
                let Some(Expr::Literal(mod_name)) = call.args.get_left_or_key("Path") else {
                    return;
                };
                let Ok(mod_name) = crate::hir::Literal::try_from(mod_name.token.clone()) else {
                    return;
                };
                let ValueObj::Str(__name__) = &mod_name.value else {
                    let name = if op.is_erg_import() {
                        "import"
                    } else {
                        "pyimport"
                    };
                    let err = CompileError::type_mismatch_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        mod_name.loc(),
                        "?".into(),
                        name,
                        Some(1),
                        &Type::Str,
                        &mod_name.t(),
                        None,
                        None,
                    );
                    self.shared.errors.push(err);
                    return;
                };
                let import_path = match self.cfg.input.resolve_path(Path::new(&__name__[..])) {
                    Some(path) => path,
                    None => {
                        return; //Err(self.import_err(line!(), __name__, loc));
                    }
                };
                let mut cfg = self.cfg.inherit(import_path.clone());
                let src = cfg.input.try_read().unwrap(); // .map_err(|_| self.import_err(line!(), __name__, loc))?;
                let mut ast_builder = ASTBuilder::new(self.cfg.copy());
                let artifact = match ast_builder.build(src) {
                    Ok(art) => art,
                    Err(iart) => {
                        self.parse_errors
                            .errors
                            .extend(CompileErrors::from(iart.errors));
                        self.parse_errors
                            .warns
                            .extend(CompileErrors::from(iart.warns));
                        return;
                    }
                };
                if self
                    .shared
                    .graph
                    .inc_ref(from_path, import_path.clone())
                    .is_err()
                {
                    if let Some(subs) = self.sub_mods.get_mut(&import_path) {
                        subs.0.push(artifact.ast);
                    } else {
                        let path = NormalizedPathBuf::from(import_path);
                        self.sub_mods.insert(path, Submodules(vec![artifact.ast]));
                    }
                    return;
                }
                let path = NormalizedPathBuf::from(import_path);
                self.plan(&artifact.ast, &path);
                self.asts.insert(path, artifact.ast);
            }
            Expr::Def(def) => {
                def.body
                    .block
                    .iter()
                    .for_each(|expr| self.check_import(expr, from_path));
            }
            _ => {}
        }
    }

    fn execute(&mut self, ast: AST, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        let path = self.cfg.input.path().to_path_buf();
        let mut graph = self.shared.graph.clone_inner();
        let mut ancestors = graph.ancestors(&path).into_vec();
        while let Some(ancestor) = ancestors.pop() {
            let ancs = graph.ancestors(&ancestor);
            if ancs.is_empty() {
                graph.remove(&ancestor);
                let ancestor_ast = self.asts.remove(&ancestor).unwrap();
                if ancestor_ast.name == ast.name {
                    continue;
                }
                let name = ancestor_ast.name.clone();
                let _name = name.clone();
                let _path = path.clone();
                let cfg = self.cfg.inherit(ancestor.to_path_buf());
                let shared = self.shared.inherit(ancestor);
                let run = move || {
                    let mut builder = HIRBuilder::new_with_cache(cfg, _name, shared.clone());
                    let mode = if _path.to_string_lossy().ends_with(".d.er") {
                        "declare"
                    } else {
                        "exec"
                    };
                    let cache = if mode == "exec" {
                        &shared.mod_cache
                    } else {
                        &shared.py_mod_cache
                    };
                    match builder.check(ancestor_ast, mode) {
                        Ok(artifact) => {
                            cache.register(
                                _path.clone(),
                                None, // TODO:
                                Some(artifact.object),
                                builder.pop_mod_ctx().unwrap(),
                            );
                            shared.warns.extend(artifact.warns);
                        }
                        Err(artifact) => {
                            shared.warns.extend(artifact.warns);
                            shared.errors.extend(artifact.errors);
                        }
                    }
                };
                let handle = spawn_new_thread(run, &name);
                self.shared.promises.insert(path.clone(), handle);
            } else {
                ancestors.insert(0, ancestor);
            }
        }
        let mod_name = "<module>";
        let mut builder =
            HIRBuilder::new_with_cache(self.cfg.clone(), mod_name, self.shared.clone());
        builder.check(ast, mode)
    }
}
