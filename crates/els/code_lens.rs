use erg_compiler::artifact::BuildRunnable;
use erg_compiler::hir::Expr;

use lsp_types::{CodeLens, CodeLensParams};

use crate::server::{send_log, ELSResult, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn handle_code_lens(
        &mut self,
        params: CodeLensParams,
    ) -> ELSResult<Option<Vec<CodeLens>>> {
        send_log("code lens requested")?;
        let uri = NormalizedUrl::new(params.text_document.uri);
        // TODO: parallelize
        let result = [
            self.send_trait_impls_lens(&uri)?,
            self.send_class_inherits_lens(&uri)?,
        ]
        .concat();
        Ok(Some(result))
    }

    fn send_trait_impls_lens(&mut self, uri: &NormalizedUrl) -> ELSResult<Vec<CodeLens>> {
        let mut result = vec![];
        if let Some(analysis) = self.artifacts.get(uri) {
            if let Some(hir) = &analysis.artifact.object {
                for chunk in hir.module.iter() {
                    match chunk {
                        Expr::Def(def) if def.def_kind().is_trait() => {
                            let trait_loc = &def.sig.ident().vi.def_loc;
                            let Some(range) = util::loc_to_range(trait_loc.loc) else {
                                continue;
                            };
                            let command = self.gen_show_trait_impls_command(trait_loc.clone())?;
                            let lens = CodeLens {
                                range,
                                command,
                                data: None,
                            };
                            result.push(lens);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(result)
    }

    fn send_class_inherits_lens(&mut self, _uri: &NormalizedUrl) -> ELSResult<Vec<CodeLens>> {
        Ok(vec![])
    }
}
