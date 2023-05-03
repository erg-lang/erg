use erg_common::consts::PYTHON_MODE;
use erg_common::traits::{Locational, Stream};
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::context::register::PylyzerStatus;
use erg_compiler::erg_parser::token::{Token, TokenCategory};
use erg_compiler::hir::Expr;
use erg_compiler::ty::HasType;
use erg_compiler::varinfo::VarInfo;

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Url};

use crate::server::{send_log, ELSResult, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn get_definition(
        &self,
        uri: &NormalizedUrl,
        token: &Token,
    ) -> ELSResult<Option<VarInfo>> {
        if !token.category_is(TokenCategory::Symbol) {
            send_log(format!("not symbol: {token}"))?;
            Ok(None)
        } else if let Some(visitor) = self.get_visitor(uri) {
            Ok(visitor.get_info(token))
        } else {
            send_log("not found")?;
            Ok(None)
        }
    }

    fn get_definition_response(
        &self,
        params: GotoDefinitionParams,
    ) -> ELSResult<GotoDefinitionResponse> {
        let uri = NormalizedUrl::new(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        if let Some(token) = self.file_cache.get_token(&uri, pos) {
            if let Some(vi) = self.get_definition(&uri, &token)? {
                // If the target variable is an imported one, jump to the definition file.
                // Else if the target variable is an alias, jump to the definition of it.
                // `foo = import "foo"` => jump to `foo.er`
                // `{x;} = import "foo"` => jump to `x` of `foo.er`
                if vi.def_loc.module == Some(util::uri_to_path(&uri))
                    && vi.def_loc.loc == token.loc()
                {
                    if let Some((_, Expr::Def(def))) = self.get_min_expr(&uri, pos, 0) {
                        if def.def_kind().is_import() {
                            if vi.t.is_module() {
                                if let Some(path) = self
                                    .get_local_ctx(&uri, pos)
                                    .first()
                                    .and_then(|ctx| ctx.get_path_from_mod_t(&vi.t))
                                {
                                    let mod_uri = Url::from_file_path(path).unwrap();
                                    let resp = GotoDefinitionResponse::Array(vec![
                                        lsp_types::Location::new(
                                            mod_uri,
                                            lsp_types::Range::default(),
                                        ),
                                    ]);
                                    return Ok(resp);
                                }
                            } else {
                                // line of module member definitions may no longer match after the desugaring process
                                let mod_t = def.body.ref_t();
                                if let Some((_, vi)) = self
                                    .get_local_ctx(&uri, pos)
                                    .first()
                                    .and_then(|ctx| ctx.get_mod_from_t(mod_t))
                                    .and_then(|mod_ctx| mod_ctx.get_var_info(token.inspect()))
                                {
                                    let def_uri =
                                        Url::from_file_path(vi.def_loc.module.as_ref().unwrap())
                                            .unwrap();
                                    let resp = GotoDefinitionResponse::Array(vec![
                                        lsp_types::Location::new(
                                            def_uri,
                                            util::loc_to_range(vi.def_loc.loc).unwrap(),
                                        ),
                                    ]);
                                    return Ok(resp);
                                }
                            }
                        } else if let Expr::Accessor(acc) = def.body.block.last().unwrap() {
                            let vi = acc.var_info();
                            match (&vi.def_loc.module, util::loc_to_range(vi.def_loc.loc)) {
                                (Some(path), Some(range)) => {
                                    let def_uri = NormalizedUrl::try_from(path.as_path()).unwrap();
                                    let def_file = if PYTHON_MODE {
                                        let header = self
                                            .file_cache
                                            .get_line(&def_uri, 0)
                                            .unwrap_or_default();
                                        let py_file = header
                                            .parse::<PylyzerStatus>()
                                            .ok()
                                            .map(|stat| stat.file);
                                        py_file.unwrap_or(path.clone())
                                    } else {
                                        path.clone()
                                    };
                                    let def_uri = Url::from_file_path(def_file).unwrap();
                                    let resp = GotoDefinitionResponse::Array(vec![
                                        lsp_types::Location::new(def_uri, range),
                                    ]);
                                    return Ok(resp);
                                }
                                _ => {
                                    send_log("not found (maybe builtin)")?;
                                    return Ok(GotoDefinitionResponse::Array(vec![]));
                                }
                            }
                        }
                    }
                }
                match (vi.def_loc.module, util::loc_to_range(vi.def_loc.loc)) {
                    (Some(path), Some(range)) => {
                        let def_uri = Url::from_file_path(path).unwrap();
                        Ok(GotoDefinitionResponse::Array(vec![
                            lsp_types::Location::new(def_uri, range),
                        ]))
                    }
                    _ => {
                        send_log("not found (maybe builtin)")?;
                        Ok(GotoDefinitionResponse::Array(vec![]))
                    }
                }
            } else {
                Ok(GotoDefinitionResponse::Array(vec![]))
            }
        } else {
            send_log("lex error occurred")?;
            Ok(GotoDefinitionResponse::Array(vec![]))
        }
    }

    pub(crate) fn handle_goto_definition(
        &mut self,
        params: GotoDefinitionParams,
    ) -> ELSResult<Option<GotoDefinitionResponse>> {
        send_log(format!("definition requested: {params:?}"))?;
        let result = self.get_definition_response(params)?;
        Ok(Some(result))
    }
}
