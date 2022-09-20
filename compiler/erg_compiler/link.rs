use erg_common::traits::Stream;

use crate::hir::{Expr, HIR};
use crate::mod_cache::SharedModuleCache;

pub struct Linker {}

impl Linker {
    pub fn link(mod_cache: SharedModuleCache) -> HIR {
        let mut main_mod_hir = mod_cache.remove("<module>").unwrap().hir.unwrap();
        for chunk in main_mod_hir.module.iter_mut() {
            match chunk {
                Expr::Def(def) if def.def_kind().is_module() => {}
                _ => {}
            }
        }
        main_mod_hir
    }
}
