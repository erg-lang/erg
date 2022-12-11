use erg_common::config::ErgConfig;
use erg_common::error::{ErrorDisplay, ErrorKind, MultiErrorDisplay};
use erg_common::traits::{BlockKind, Runnable, Stream};
use erg_common::Str;

use crate::ast::AST;
use crate::desugar::Desugarer;
use crate::error::{ParserRunnerError, ParserRunnerErrors};
use crate::parse::ParserRunner;

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
    fn finish(&mut self) {}

    #[inline]
    fn initialize(&mut self) {}

    #[inline]
    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let ast = self.build(self.input().read())?;
        println!("{ast}");
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, ParserRunnerErrors> {
        let ast = self.build(src)?;
        Ok(format!("{ast}"))
    }

    fn expect_block(&self, src: &str) -> BlockKind {
        let mut parser = ParserRunner::new(self.cfg().clone());
        match parser.eval(src.to_string()) {
            Err(errs) => {
                let (has_next_line, kind) = errs
                    .first()
                    .map(|e| {
                        let enl = e.core().kind == ErrorKind::ExpectNextLine;
                        let msg = if enl {
                            // if enl is true, sub_message and hint must exit
                            // if not, it is a bug
                            let msg = e.core().sub_messages.last().unwrap();
                            msg.get_hint().unwrap().to_string()
                        } else {
                            String::new()
                        };
                        (enl, msg)
                    })
                    .unwrap_or((false, String::new()));
                if has_next_line {
                    return match kind.as_str() {
                        "Lambda" => BlockKind::Lambda,
                        "Assignment" => BlockKind::Assignment,
                        "ColonCall" => BlockKind::ColonCall,
                        "ClassAttr" => BlockKind::ClassAttr,
                        "ClassAttrDecl" => BlockKind::ClassAttrDecl,
                        _ => BlockKind::Error,
                    };
                }
                if errs
                    .first()
                    .map(|e| {
                        e.core().kind == ErrorKind::SyntaxError
                            && e.core().main_message.contains("\"\"\"")
                    })
                    .unwrap_or(false)
                {
                    return BlockKind::MultiLineStr;
                }
                errs.fmt_all_stderr();
                BlockKind::Error
            }
            Ok(_) => BlockKind::None,
        }
    }
}

impl ASTBuilder {
    pub fn build(&mut self, src: String) -> Result<AST, ParserRunnerErrors> {
        let module = self.runner.parse(src)?;
        let mut desugarer = Desugarer::new();
        let module = desugarer.desugar(module);
        let ast = AST::new(Str::rc(self.runner.cfg().input.full_path()), module);
        Ok(ast)
    }
}
