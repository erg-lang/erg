use erg_common::config::ErgConfig;
use erg_common::traits::{ExitStatus, Runnable};
use erg_common::Str;

use crate::ast::AST;
use crate::desugar::Desugarer;
use crate::error::{CompleteArtifact, IncompleteArtifact, ParserRunnerError, ParserRunnerErrors};
use crate::parse::ParserRunner;

pub trait ASTBuildable {
    fn new(cfg: ErgConfig) -> Self;
    fn build_ast(
        &mut self,
        src: String,
    ) -> Result<
        CompleteArtifact<AST, ParserRunnerErrors>,
        IncompleteArtifact<AST, ParserRunnerErrors>,
    >;
}

/// Summarize parsing and desugaring
#[derive(Debug, Default)]
pub struct ASTBuilder {
    runner: ParserRunner,
}

impl Runnable for ASTBuilder {
    type Err = ParserRunnerError;
    type Errs = ParserRunnerErrors;
    const NAME: &'static str = "Erg AST builder";

    #[inline]
    fn new(cfg: ErgConfig) -> Self {
        Self {
            runner: ParserRunner::new(cfg),
        }
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        self.runner.cfg()
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        self.runner.cfg_mut()
    }

    #[inline]
    fn finish(&mut self) {}

    #[inline]
    fn initialize(&mut self) {}

    #[inline]
    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let src = self.cfg_mut().input.read();
        let artifact = self.build(src).map_err(|iart| iart.errors)?;
        println!("{}", artifact.ast);
        Ok(ExitStatus::OK)
    }

    fn eval(&mut self, src: String) -> Result<String, ParserRunnerErrors> {
        let artifact = self.build(src).map_err(|iart| iart.errors)?;
        Ok(format!("{}", artifact.ast))
    }
}

impl ASTBuildable for ASTBuilder {
    fn new(cfg: ErgConfig) -> Self {
        Self {
            runner: ParserRunner::new(cfg),
        }
    }

    fn build_ast(
        &mut self,
        src: String,
    ) -> Result<
        CompleteArtifact<AST, ParserRunnerErrors>,
        IncompleteArtifact<AST, ParserRunnerErrors>,
    > {
        self.build(src)
    }
}

impl ASTBuilder {
    pub fn build(
        &mut self,
        src: String,
    ) -> Result<
        CompleteArtifact<AST, ParserRunnerErrors>,
        IncompleteArtifact<AST, ParserRunnerErrors>,
    > {
        let name = Str::from(self.runner.cfg().input.filename());
        let mut desugarer = Desugarer::new();
        let artifact = self.runner.parse(src).map_err(|iart| {
            iart.map_mod(|module| {
                let module = desugarer.desugar(module);
                AST::new(name.clone(), module)
            })
        })?;
        let module = desugarer.desugar(artifact.ast);
        let ast = AST::new(name, module);
        Ok(CompleteArtifact::new(
            ast,
            ParserRunnerErrors::convert(self.input(), artifact.warns),
        ))
    }

    pub fn build_without_desugaring(
        &mut self,
        src: String,
    ) -> Result<
        CompleteArtifact<AST, ParserRunnerErrors>,
        IncompleteArtifact<AST, ParserRunnerErrors>,
    > {
        let name = Str::from(self.runner.cfg().input.filename());
        let artifact = self
            .runner
            .parse(src)
            .map_err(|iart| iart.map_mod(|module| AST::new(name.clone(), module)))?;
        let ast = AST::new(name, artifact.ast);
        Ok(CompleteArtifact::new(
            ast,
            ParserRunnerErrors::convert(self.input(), artifact.warns),
        ))
    }
}
