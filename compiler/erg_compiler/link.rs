use erg_common::dict::Dict;
use erg_common::log;
use erg_common::traits::Stream;
use erg_common::Str;

use erg_parser::ast::{ClassDef, Expr, Module, PreDeclTypeSpec, TypeSpec, AST};

/// Combine method definitions across multiple modules, specialized class contexts, etc.
#[derive(Debug)]
pub struct Linker {
    // TODO: inner scope types
    pub def_root_pos_map: Dict<Str, usize>,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            def_root_pos_map: Dict::new(),
        }
    }

    pub fn link(&mut self, mut ast: AST) -> AST {
        log!(info "the linking process has started.");
        let mut new = vec![];
        while let Some(chunk) = ast.module.lpop() {
            match chunk {
                Expr::Def(def) => {
                    match def.body.block.first().unwrap() {
                        Expr::Call(call) => {
                            match call.obj.get_name().map(|s| &s[..]) {
                                // TODO: decorator
                                Some("Class" | "Inherit" | "Inheritable") => {
                                    self.def_root_pos_map.insert(
                                        def.sig.ident().unwrap().inspect().clone(),
                                        new.len(),
                                    );
                                    let type_def = ClassDef::new(def, vec![]);
                                    new.push(Expr::ClassDef(type_def));
                                }
                                _ => {
                                    new.push(Expr::Def(def));
                                }
                            }
                        }
                        _ => {
                            new.push(Expr::Def(def));
                        }
                    }
                }
                Expr::Methods(methods) => match &methods.class {
                    TypeSpec::PreDeclTy(PreDeclTypeSpec::Simple(simple)) => {
                        if let Some(pos) = self.def_root_pos_map.get(simple.name.inspect()) {
                            let mut type_def = match new.remove(*pos) {
                                Expr::ClassDef(type_def) => type_def,
                                _ => unreachable!(),
                            };
                            type_def.methods_list.push(methods);
                            new.insert(*pos, Expr::ClassDef(type_def));
                        } else {
                            log!("{}", simple.name.inspect());
                            log!("{}", self.def_root_pos_map);
                            todo!()
                        }
                    }
                    other => todo!("{other}"),
                },
                other => {
                    new.push(other);
                }
            }
        }
        let ast = AST::new(ast.name, Module::new(new));
        log!(info "the linking process has completed:\n{}", ast);
        ast
    }
}
