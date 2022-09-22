use erg_common::config::ErgConfig;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{enum_unwrap, log};

use erg_parser::ast::DefId;
use erg_parser::token::Token;

use erg_type::Type;

use crate::hir::{Accessor, Args, Block, Call, Def, DefBody, Expr, Identifier, PosArg, HIR};
use crate::mod_cache::SharedModuleCache;

pub struct Linker {}

impl Linker {
    pub fn link(cfg: ErgConfig, mut main: HIR, mod_cache: SharedModuleCache) -> HIR {
        log!(info "the linking process has started.");
        for chunk in main.module.iter_mut() {
            match chunk {
                // x = import "mod"
                // ↓
                // x = ModuleType("mod")
                // exec(code, x.__dict__) # `code` is the mod's content
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
                    let mod_name = enum_unwrap!(def.body.block.first().unwrap(), Expr::Call)
                        .args
                        .get_left_or_key("path")
                        .unwrap();
                    // let sig = option_enum_unwrap!(&def.sig, Signature::Var)
                    //    .unwrap_or_else(|| todo!("module subroutines are not allowed"));
                    if let Some(hir) = hir {
                        let code = Expr::Code(Block::new(Vec::from(hir.module)));
                        let module_type = Expr::Accessor(Accessor::private_with_line(
                            Str::ever("#ModuleType"),
                            def.ln_begin().unwrap(),
                        ));
                        let args =
                            Args::new(vec![PosArg::new(mod_name.clone())], None, vec![], None);
                        let block = Block::new(vec![Expr::Call(Call::new(
                            module_type,
                            None,
                            args,
                            Type::Uninited,
                        ))]);
                        let mod_def = Expr::Def(Def::new(
                            def.sig.clone(),
                            DefBody::new(Token::dummy(), block, DefId(0)),
                        ));
                        let exec = Expr::Accessor(Accessor::public_with_line(
                            Str::ever("exec"),
                            mod_def.ln_begin().unwrap(),
                        ));
                        let module = Expr::Accessor(Accessor::Ident(def.sig.ident().clone()));
                        let __dict__ = Identifier::public("__dict__");
                        let m_dict =
                            Expr::Accessor(Accessor::attr(module, __dict__, Type::Uninited));
                        let args = Args::new(
                            vec![PosArg::new(code), PosArg::new(m_dict)],
                            None,
                            vec![],
                            None,
                        );
                        let exec_code = Expr::Call(Call::new(exec, None, args, Type::Uninited));
                        let compound = Block::new(vec![mod_def, exec_code]);
                        *chunk = Expr::Compound(compound);
                    }
                }
                _ => {}
            }
        }
        log!(info "linked: {main}");
        main
    }
}
