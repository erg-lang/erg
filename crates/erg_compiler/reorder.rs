use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::log;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;

use erg_parser::ast::{
    Accessor, ClassAttr, ClassDef, Expr, Methods, Module, PatchDef, PreDeclTypeSpec,
    TypeAscription, TypeSpec, AST,
};

use crate::error::{TyCheckError, TyCheckErrors};

/// Combine method definitions across multiple modules, specialized class contexts, etc.
#[derive(Debug, Default)]
pub struct Reorderer {
    cfg: ErgConfig,
    // TODO: inner scope types
    pub def_root_pos_map: Dict<Str, usize>,
    pub deps: Dict<Str, Vec<Str>>,
    pub errs: TyCheckErrors,
}

impl Reorderer {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            def_root_pos_map: Dict::new(),
            deps: Dict::new(),
            errs: TyCheckErrors::empty(),
        }
    }

    pub fn reorder(mut self, ast: AST, mode: &str) -> Result<AST, TyCheckErrors> {
        log!(info "the reordering process has started.");
        let mut new = vec![];
        for chunk in ast.module.into_iter() {
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
                                Some("Patch") => {
                                    self.def_root_pos_map.insert(
                                        def.sig.ident().unwrap().inspect().clone(),
                                        new.len(),
                                    );
                                    let type_def = PatchDef::new(def, vec![]);
                                    new.push(Expr::PatchDef(type_def));
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
                        self.link_methods(simple.ident.inspect().clone(), &mut new, methods, mode)
                    }
                    TypeSpec::TypeApp { spec, .. } => {
                        if let TypeSpec::PreDeclTy(PreDeclTypeSpec::Simple(simple)) = spec.as_ref()
                        {
                            self.link_methods(
                                simple.ident.inspect().clone(),
                                &mut new,
                                methods,
                                mode,
                            )
                        } else {
                            let similar_name = self
                                .def_root_pos_map
                                .keys()
                                .fold("".to_string(), |acc, key| acc + &key[..] + ",");
                            self.errs.push(TyCheckError::no_var_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                methods.class.loc(),
                                "".into(),
                                &methods.class.to_string(),
                                Some(&Str::from(similar_name)),
                            ));
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
        log!(info "the reordering process has completed:\n{}", ast);
        if self.errs.is_empty() {
            Ok(ast)
        } else {
            Err(self.errs)
        }
    }

    /// ```erg
    /// C.
    ///     x: Int
    ///     f: (self: Self) -> Int
    /// ```
    /// ↓
    /// ```erg
    /// C.x: Int
    /// C.y: (self: C) -> Int
    /// ```
    fn flatten_method_decls(&mut self, new: &mut Vec<Expr>, methods: Methods) {
        let class = methods.class_as_expr.as_ref();
        for method in methods.attrs.into_iter() {
            match method {
                ClassAttr::Decl(decl) => {
                    let Expr::Accessor(Accessor::Ident(ident)) = *decl.expr else {
                        self.errs.push(TyCheckError::syntax_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            decl.expr.loc(),
                            "".into(),
                            "".into(),
                            None
                        ));
                        continue;
                    };
                    let expr = class.clone().attr_expr(ident);
                    let decl = TypeAscription::new(expr, decl.t_spec);
                    new.push(Expr::TypeAscription(decl));
                }
                ClassAttr::Doc(doc) => {
                    new.push(Expr::Literal(doc));
                }
                ClassAttr::Def(def) => {
                    self.errs.push(TyCheckError::syntax_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        def.loc(),
                        "".into(),
                        "".into(),
                        None,
                    ));
                }
            }
        }
    }

    fn link_methods(&mut self, name: Str, new: &mut Vec<Expr>, methods: Methods, mode: &str) {
        if let Some(pos) = self.def_root_pos_map.get(&name) {
            match new.remove(*pos) {
                Expr::ClassDef(mut class_def) => {
                    class_def.methods_list.push(methods);
                    new.insert(*pos, Expr::ClassDef(class_def));
                }
                Expr::PatchDef(mut patch_def) => {
                    patch_def.methods_list.push(methods);
                    new.insert(*pos, Expr::PatchDef(patch_def));
                }
                _ => unreachable!(),
            }
        } else if mode == "declare" {
            self.flatten_method_decls(new, methods);
        } else {
            let similar_name = self
                .def_root_pos_map
                .keys()
                .fold("".to_string(), |acc, key| acc + &key[..] + ",");
            self.errs.push(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                methods.class.loc(),
                "".into(),
                &name,
                Some(&Str::from(similar_name)),
            ));
        }
    }
}
