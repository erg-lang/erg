use erg_common::consts::PYTHON_MODE;
use erg_common::traits::Stream;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::context::register::PylyzerStatus;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::erg_parser::token::{Token, TokenCategory};
use erg_compiler::hir::{Def, Expr};
use erg_compiler::ty::HasType;
use erg_compiler::varinfo::VarInfo;

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Position, Url};

use crate::server::{send_log, ELSResult, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
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

    pub(crate) fn get_definition_location(
        &self,
        uri: &NormalizedUrl,
        pos: Position,
    ) -> ELSResult<Option<Location>> {
        if let Some(token) = self.file_cache.get_symbol(uri, pos) {
            if let Some(vi) = self.get_definition(uri, &token)? {
                // If the target variable is an imported one, jump to the definition file.
                // Else if the target variable is an alias, jump to the definition of it.
                // `foo = import "foo"` => jump to `foo.er`
                // `{x;} = import "foo"` => jump to `x` of `foo.er`
                if vi.def_loc.module == Some(util::uri_to_path(uri))
                    && vi.def_loc.loc == token.loc()
                {
                    if let Some(def) = self.get_min::<Def>(uri, pos) {
                        if def.def_kind().is_import() {
                            if vi.t.is_module() {
                                if let Some(path) = self
                                    .get_local_ctx(uri, pos)
                                    .first()
                                    .and_then(|ctx| ctx.get_path_with_mod_t(&vi.t))
                                {
                                    let mod_uri = Url::from_file_path(path).unwrap();
                                    return Ok(Some(lsp_types::Location::new(
                                        mod_uri,
                                        lsp_types::Range::default(),
                                    )));
                                }
                            } else {
                                // line of module member definitions may no longer match after the desugaring process
                                let mod_t = def.body.ref_t();
                                if let Some((_, vi)) = self
                                    .get_local_ctx(uri, pos)
                                    .first()
                                    .and_then(|ctx| ctx.get_mod_with_t(mod_t))
                                    .and_then(|mod_ctx| mod_ctx.get_var_info(token.inspect()))
                                {
                                    let Some(path) = vi.def_loc.module.as_ref() else {
                                        return Ok(None);
                                    };
                                    let def_uri = Url::from_file_path(path).unwrap();
                                    return Ok(Some(lsp_types::Location::new(
                                        def_uri,
                                        util::loc_to_range(vi.def_loc.loc).unwrap(),
                                    )));
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
                                    return Ok(Some(lsp_types::Location::new(def_uri, range)));
                                }
                                _ => {
                                    send_log("not found (maybe builtin)")?;
                                    return Ok(None);
                                }
                            }
                        }
                    }
                }
                match (vi.def_loc.module, util::loc_to_range(vi.def_loc.loc)) {
                    (Some(path), Some(range)) => {
                        let def_uri = Url::from_file_path(path).unwrap();
                        Ok(Some(lsp_types::Location::new(def_uri, range)))
                    }
                    _ => {
                        send_log("not found (maybe builtin)")?;
                        Ok(None)
                    }
                }
            } else {
                Ok(None)
            }
        } else {
            send_log("lex error occurred")?;
            Ok(None)
        }
    }

    pub(crate) fn handle_goto_definition(
        &mut self,
        params: GotoDefinitionParams,
    ) -> ELSResult<Option<GotoDefinitionResponse>> {
        send_log(format!("definition requested: {params:?}"))?;
        let uri = NormalizedUrl::new(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        let result = self.get_definition_location(&uri, pos)?;
        Ok(result.map(GotoDefinitionResponse::Scalar))
    }
}
