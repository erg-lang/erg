use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;

use erg_compiler::ty::Type;
use erg_compiler::varinfo::VarInfo;
use lsp_types::{SymbolInformation, SymbolKind, WorkspaceSymbolParams};

use crate::_log;
use crate::server::{ELSResult, Server};
use crate::util::abs_loc_to_lsp_loc;

pub(crate) fn symbol_kind(vi: &VarInfo) -> SymbolKind {
    match &vi.t {
        Type::Subr(subr) if subr.self_t().is_some() => SymbolKind::METHOD,
        Type::Quantified(quant) if quant.self_t().is_some() => SymbolKind::METHOD,
        Type::Subr(_) | Type::Quantified(_) => SymbolKind::FUNCTION,
        Type::ClassType => SymbolKind::CLASS,
        Type::TraitType => SymbolKind::INTERFACE,
        t if matches!(&t.qual_name()[..], "Module" | "PyModule" | "GenericModule") => {
            SymbolKind::MODULE
        }
        _ if vi.muty.is_const() => SymbolKind::CONSTANT,
        _ => SymbolKind::VARIABLE,
    }
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_workspace_symbol(
        &mut self,
        params: WorkspaceSymbolParams,
    ) -> ELSResult<Option<Vec<SymbolInformation>>> {
        _log!("workspace symbol requested: {params:?}");
        let mut res = vec![];
        for module in self.modules.values() {
            for (name, vi) in module.context.local_dir() {
                if !params.query.is_empty() && !name.inspect().contains(&params.query) {
                    continue;
                }
                let Some(location) = abs_loc_to_lsp_loc(&vi.def_loc) else {
                    continue;
                };
                #[allow(deprecated)]
                let info = SymbolInformation {
                    name: name.to_string(),
                    location,
                    kind: symbol_kind(vi),
                    container_name: None,
                    tags: None,
                    deprecated: None,
                };
                res.push(info);
            }
        }
        Ok(Some(res))
    }
}
