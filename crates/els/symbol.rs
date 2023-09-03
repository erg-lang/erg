use erg_common::traits::Locational;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::ast::DefKind;
use erg_compiler::erg_parser::parse::Parsable;

use erg_compiler::hir::Expr;
use erg_compiler::ty::{HasType, Type};
use erg_compiler::varinfo::VarInfo;
use lsp_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, SymbolInformation, SymbolKind,
    WorkspaceSymbolParams,
};

use crate::_log;
use crate::server::{ELSResult, Server};
use crate::util::{abs_loc_to_lsp_loc, loc_to_range, NormalizedUrl};

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
                if name.inspect().starts_with(['%']) {
                    continue;
                }
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

    pub(crate) fn handle_document_symbol(
        &mut self,
        params: DocumentSymbolParams,
    ) -> ELSResult<Option<DocumentSymbolResponse>> {
        _log!("document symbol requested: {params:?}");
        let uri = NormalizedUrl::new(params.text_document.uri);
        if let Some(result) = self.analysis_result.get(&uri) {
            if let Some(hir) = &result.artifact.object {
                let mut res = vec![];
                for chunk in hir.module.iter() {
                    let symbol = self.symbol(chunk);
                    res.extend(symbol);
                }
                return Ok(Some(DocumentSymbolResponse::Nested(res)));
            }
        }
        Ok(None)
    }

    fn symbol(&self, chunk: &Expr) -> Option<DocumentSymbol> {
        match chunk {
            Expr::Def(def) => {
                if def.sig.inspect().starts_with(['%']) {
                    return None;
                }
                let range = loc_to_range(def.loc())?;
                let selection_range = loc_to_range(def.sig.loc())?;
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: def.sig.name().to_string(),
                    detail: Some(def.sig.ident().ref_t().to_string()),
                    kind: symbol_kind(&def.sig.ident().vi),
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range,
                    children: Some(self.child_symbols(chunk)),
                })
            }
            Expr::ClassDef(def) => {
                let range = loc_to_range(def.loc())?;
                let selection_range = loc_to_range(def.sig.loc())?;
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: def.sig.name().to_string(),
                    detail: Some(def.sig.ident().ref_t().to_string()),
                    kind: symbol_kind(&def.sig.ident().vi),
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range,
                    children: Some(self.child_symbols(chunk)),
                })
            }
            Expr::PatchDef(def) => {
                let range = loc_to_range(def.loc())?;
                let selection_range = loc_to_range(def.sig.loc())?;
                #[allow(deprecated)]
                Some(DocumentSymbol {
                    name: def.sig.name().to_string(),
                    detail: Some(def.sig.ident().ref_t().to_string()),
                    kind: symbol_kind(&def.sig.ident().vi),
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range,
                    children: Some(self.child_symbols(chunk)),
                })
            }
            _ => None,
        }
    }

    fn child_symbols(&self, chunk: &Expr) -> Vec<DocumentSymbol> {
        match chunk {
            Expr::Def(def) => match def.def_kind() {
                DefKind::Class | DefKind::Trait => {
                    if let Some(base) = def.get_base() {
                        let mut res = vec![];
                        for member in base.attrs.iter() {
                            let symbol = self.symbol(&Expr::Def(member.clone()));
                            res.extend(symbol);
                        }
                        res
                    } else {
                        vec![]
                    }
                }
                _ => vec![],
            },
            Expr::ClassDef(def) => {
                let mut res = vec![];
                if let Some(Expr::Record(rec)) = def.require_or_sup.as_deref() {
                    for member in rec.attrs.iter() {
                        let symbol = self.symbol(&Expr::Def(member.clone()));
                        res.extend(symbol);
                    }
                }
                for method in def.methods.iter() {
                    let symbol = self.symbol(method);
                    res.extend(symbol);
                }
                res
            }
            Expr::PatchDef(def) => {
                let mut res = vec![];
                for method in def.methods.iter() {
                    let symbol = self.symbol(method);
                    res.extend(symbol);
                }
                res
            }
            _ => vec![],
        }
    }
}
