use erg_common::config::ErgConfig;
use erg_common::log;
use erg_common::traits::Stream;

use erg_parser::token::Token;

use erg_type::value::TypeKind;
use erg_type::Type;

use crate::hir::{Block, ClassDef, Expr, Record, RecordAttrs, HIR};
use crate::mod_cache::SharedModuleCache;

pub struct Linker {}

impl Linker {
    pub fn link(cfg: ErgConfig, mut main: HIR, mod_cache: SharedModuleCache) -> HIR {
        log!(info "the linking process has started.");
        for chunk in main.module.iter_mut() {
            match chunk {
                // x = import "mod"
                // ↓
                // class x:
                //     ...
                Expr::Def(ref def) if def.def_kind().is_module() => {
                    // In the case of REPL, entries cannot be used up
                    let hir = if cfg.input.is_repl() {
                        mod_cache
                            .get(&def.sig.ident().inspect()[..])
                            .and_then(|entry| entry.hir.clone())
                    } else {
                        mod_cache
                            .remove(&def.sig.ident().inspect()[..])
                            .and_then(|entry| entry.hir)
                    };
                    // let sig = option_enum_unwrap!(&def.sig, Signature::Var)
                    //    .unwrap_or_else(|| todo!("module subroutines are not allowed"));
                    if let Some(hir) = hir {
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
        log!(info "linked: {main}");
        main
    }
}
