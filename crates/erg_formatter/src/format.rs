use std::fs::File;
use std::io::Write;

use erg_common::config::ErgConfig;
use erg_common::traits::{ExitStatus, Runnable};
use erg_parser::ast::Module;
use erg_parser::error::{ParserRunnerError, ParserRunnerErrors};
use erg_parser::ParserRunner;

use crate::transform::ASTTransformer;

#[derive(Debug, Default, Clone)]
pub struct Formatter {
    cfg: ErgConfig,
    _level: usize,
    result: String,
}

impl Runnable for Formatter {
    type Err = ParserRunnerError;
    type Errs = ParserRunnerErrors;
    const NAME: &'static str = "Erg formatter";

    #[inline]
    fn new(cfg: ErgConfig) -> Self {
        Self::new(cfg)
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

    #[inline]
    fn initialize(&mut self) {}

    #[inline]
    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let code = self.cfg.input.read();
        let mut parser = ParserRunner::new(self.cfg.copy());
        let art = parser.parse(code).map_err(ParserRunnerErrors::from)?;
        let code = self.format(art.ast);
        let mut f = File::options()
            .write(true)
            .open(self.cfg.dump_path())
            .unwrap();
        f.write_all(code.as_bytes()).unwrap();
        Ok(ExitStatus::OK)
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let mut parser = ParserRunner::new(self.cfg.copy());
        let art = parser.parse(src).map_err(ParserRunnerErrors::from)?;
        let artifact = self.format(art.ast);
        Ok(artifact)
    }
}

impl Formatter {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            _level: 0,
            result: String::new(),
        }
    }

    /// Transform and emit
    pub fn format(&mut self, ast: Module) -> String {
        let transformer = ASTTransformer::new(ast);
        let ast = transformer.transform();
        self.emit(ast)
    }

    fn emit(&mut self, _ast: Module) -> String {
        std::mem::take(&mut self.result)
    }
}
