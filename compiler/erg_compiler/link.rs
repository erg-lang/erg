use erg_common::log;
use erg_common::traits::Stream;

use erg_parser::token::Token;

use erg_type::value::TypeKind;
use erg_type::Type;

use crate::hir::{Block, ClassDef, Expr, Record, RecordAttrs, HIR};
use crate::mod_cache::{ModuleEntry, SharedModuleCache};

pub struct Linker {}

impl Linker {
    pub fn link(mod_cache: SharedModuleCache) -> HIR {
        log!(info "the linking process has started.");
        let mut main_mod_hir = mod_cache.remove("<module>").unwrap().hir.unwrap();
        for chunk in main_mod_hir.module.iter_mut() {
            match chunk {
                // x = import "mod"
                // ↓
                // class x:
                //     ...
                Expr::Def(ref def) if def.def_kind().is_module() => {
                    // let sig = option_enum_unwrap!(&def.sig, Signature::Var)
                    //    .unwrap_or_else(|| todo!("module subroutines are not allowed"));
                    if let Some(ModuleEntry { hir: Some(hir), .. }) =
                        mod_cache.remove(&def.sig.ident().inspect()[..])
                    {
                        let block = Block::new(Vec::from(hir.module));
                        let def = ClassDef::new(
                            TypeKind::Class,
                            def.sig.clone(),
                            Expr::Record(Record::new(
                                Token::dummy(),
                                Token::dummy(),
                                RecordAttrs::empty(),
                            )),
                            false,
                            Type::Uninited,
                            block,
                        );
                        *chunk = Expr::ClassDef(def);
                    }
                }
                _ => {}
            }
        }
        log!(info "linked: {main_mod_hir}");
        main_mod_hir
    }
}
