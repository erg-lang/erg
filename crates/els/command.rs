use erg_compiler::varinfo::AbsLocation;
use serde_json::Value;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::hir::Expr;

use lsp_types::{Command, ExecuteCommandParams, Location, Url};

use crate::_log;
use crate::server::{ELSResult, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn handle_execute_command(
        &mut self,
        params: ExecuteCommandParams,
    ) -> ELSResult<Option<Value>> {
        _log!("command requested: {}", params.command);
        #[allow(clippy::match_single_binding)]
        match &params.command[..] {
            other => {
                _log!("unknown command {other}: {params:?}");
                Ok(None)
            }
        }
    }

    pub(crate) fn gen_show_trait_impls_command(
        &self,
        trait_loc: AbsLocation,
    ) -> ELSResult<Option<Command>> {
        let refs = self.get_refs_from_abs_loc(&trait_loc);
        let filter = |loc: Location| {
            let uri = NormalizedUrl::new(loc.uri.clone());
            let token = self.file_cache.get_token(&uri, loc.range.start)?;
            let opt_visitor = self.get_visitor(&uri);
            let min_expr = opt_visitor
                .as_ref()
                .and_then(|visitor| visitor.get_min_expr(&token))?;
            matches!(min_expr, Expr::ClassDef(_)).then_some(loc)
        };
        let impls = refs.into_iter().filter_map(filter).collect::<Vec<_>>();
        let impl_len = impls.len();
        let locations = serde_json::to_value(impls)?;
        let Ok(uri) = trait_loc.module.ok_or(()).and_then(Url::from_file_path) else {
            return Ok(None);
        };
        let uri = serde_json::to_value(uri)?;
        let Some(position) = util::loc_to_pos(trait_loc.loc) else {
            return Ok(None);
        };
        let position = serde_json::to_value(position)?;
        Ok(Some(Command {
            title: format!("{impl_len} implementations"),
            // the command is defined in: https://github.com/erg-lang/vscode-erg/blob/20e6e2154b045ab56fedbc8769d03633acfd12e0/src/extension.ts#L92-L94
            command: "erg.showReferences".to_string(),
            arguments: Some(vec![uri, position, locations]),
        }))
    }
}
