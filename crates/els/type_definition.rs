use std::path::Path;

use erg_common::shared::MappedRwLockReadGuard;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;

use lsp_types::request::{GotoTypeDefinitionParams, GotoTypeDefinitionResponse};
use lsp_types::{GotoDefinitionResponse, Position, Url};

use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::util::{self, NormalizedUrl};

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_goto_type_definition(
        &mut self,
        params: GotoTypeDefinitionParams,
    ) -> ELSResult<Option<GotoTypeDefinitionResponse>> {
        self.send_log(format!("type definition requested: {params:?}"))?;
        let uri = NormalizedUrl::new(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        Ok(self.get_type_def(&uri, pos))
    }

    fn get_type_def(&self, uri: &NormalizedUrl, pos: Position) -> Option<GotoDefinitionResponse> {
        let visitor = self.get_visitor(uri)?;
        let tok = self.file_cache.get_symbol(uri, pos)?;
        let typ = &visitor.get_info(&tok)?.t;
        let path = typ.namespace();
        let module = self
            .shared
            .get_module(Path::new(&path[..]))
            .map(|ent| MappedRwLockReadGuard::map(ent, |ent| &ent.module))
            .or_else(|| self.get_mod_ctx(uri))?;
        let (_, typ_info) = module.context.get_type_info(typ)?;
        let path = typ_info.def_loc.module.as_ref()?;
        let def_uri = Url::from_file_path(path).ok()?;
        let loc = lsp_types::Location::new(def_uri, util::loc_to_range(typ_info.def_loc.loc)?);
        Some(GotoDefinitionResponse::Scalar(loc))
    }
}
