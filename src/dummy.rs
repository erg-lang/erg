use common::config::{ErgConfig, Input, SEMVER, BUILD_INFO};
use common::python_util::eval_pyc;
use common::str::Str;
use common::traits::Runnable;

use compiler::Compiler;
use compiler::error::{CompileError, CompileErrors};

#[derive(Debug)]
pub struct DummyVM {
    cfg: ErgConfig,
    compiler: Compiler,
}

impl Runnable for DummyVM {
    type Err = CompileError;
    type Errs = CompileErrors;

    fn new(cfg: ErgConfig) -> Self {
        Self {
            compiler: Compiler::new(cfg.copy()),
            cfg,
        }
    }

    #[inline]
    fn input(&self) -> &Input { &self.cfg.input }

    #[inline]
    fn start_message(&self) -> String { format!("Erg interpreter {} {}\n", SEMVER, &*BUILD_INFO) }

    fn clear(&mut self) {
        self.compiler.clear();
    }

    fn eval(&mut self, src: Str) -> Result<String, CompileErrors> {
        self.compiler.compile_and_dump_as_pyc(src, "o.pyc")?;
        Ok(eval_pyc("o.pyc"))
    }
}
