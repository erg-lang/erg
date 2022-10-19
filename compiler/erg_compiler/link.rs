use std::mem;
use std::path::{Path, PathBuf};

use erg_common::config::{ErgConfig, Input};
use erg_common::python_util::BUILTIN_PYTHON_MODS;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{enum_unwrap, log};

use erg_parser::ast::{DefId, OperationKind};
use erg_parser::token::{Token, TokenKind};

use crate::ty::free::fresh_varname;
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::HasType;

use crate::hir::*;
use crate::mod_cache::SharedModuleCache;

pub struct Linker<'a> {
    cfg: &'a ErgConfig,
    mod_cache: &'a SharedModuleCache,
}

impl<'a> Linker<'a> {
    pub fn new(cfg: &'a ErgConfig, mod_cache: &'a SharedModuleCache) -> Self {
        Self { cfg, mod_cache }
    }

    pub fn link(&self, mut main: HIR) -> HIR {
        log!(info "the linking process has started.");
        for chunk in main.module.iter_mut() {
            self.replace_import(chunk);
        }
        for chunk in main.module.iter_mut() {
            self.resolve_pymod_path(chunk);
        }
        log!(info "linked: {main}");
        main
    }

    /// ```erg
    /// urllib = pyimport "urllib"
    /// urllib.request.urlopen! "https://example.com"
    /// ```
    /// ↓
    /// ```python
    /// urllib = __import__("urllib.request")
    /// import urllib.request
    /// urllib.request.urlopen("https://example.com")
    /// ```
    fn resolve_pymod_path(&self, expr: &mut Expr) {
        match expr {
            Expr::Lit(_) => {}
            Expr::Accessor(acc) => {
                if matches!(acc, Accessor::Attr(_)) && acc.ref_t().is_py_module() {
                    let import = Expr::Import(acc.clone());
                    *expr = Expr::Compound(Block::new(vec![import, mem::take(expr)]));
                }
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    for elem in arr.elems.pos_args.iter_mut() {
                        self.resolve_pymod_path(&mut elem.expr);
                    }
                }
                Array::WithLength(arr) => {
                    self.resolve_pymod_path(&mut arr.elem);
                    self.resolve_pymod_path(&mut arr.len);
                }
                _ => todo!(),
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    for elem in tup.elems.pos_args.iter_mut() {
                        self.resolve_pymod_path(&mut elem.expr);
                    }
                }
            },
            Expr::Set(set) => match set {
                Set::Normal(st) => {
                    for elem in st.elems.pos_args.iter_mut() {
                        self.resolve_pymod_path(&mut elem.expr);
                    }
                }
                Set::WithLength(st) => {
                    self.resolve_pymod_path(&mut st.elem);
                    self.resolve_pymod_path(&mut st.len);
                }
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dic) => {
                    for elem in dic.kvs.iter_mut() {
                        self.resolve_pymod_path(&mut elem.key);
                        self.resolve_pymod_path(&mut elem.value);
                    }
                }
                other => todo!("{other}"),
            },
            Expr::Record(record) => {
                for attr in record.attrs.iter_mut() {
                    for chunk in attr.body.block.iter_mut() {
                        self.resolve_pymod_path(chunk);
                    }
                }
            }
            Expr::BinOp(binop) => {
                self.resolve_pymod_path(&mut binop.lhs);
                self.resolve_pymod_path(&mut binop.rhs);
            }
            Expr::UnaryOp(unaryop) => {
                self.resolve_pymod_path(&mut unaryop.expr);
            }
            Expr::Call(call) => {
                self.resolve_pymod_path(&mut call.obj);
                for arg in call.args.pos_args.iter_mut() {
                    self.resolve_pymod_path(&mut arg.expr);
                }
                for arg in call.args.kw_args.iter_mut() {
                    self.resolve_pymod_path(&mut arg.expr);
                }
            }
            Expr::Decl(_decl) => {}
            Expr::Def(def) => {
                for chunk in def.body.block.iter_mut() {
                    self.resolve_pymod_path(chunk);
                }
            }
            Expr::Lambda(lambda) => {
                for chunk in lambda.body.iter_mut() {
                    self.resolve_pymod_path(chunk);
                }
            }
            Expr::ClassDef(class_def) => {
                for def in class_def.methods.iter_mut() {
                    self.resolve_pymod_path(def);
                }
            }
            Expr::AttrDef(attr_def) => {
                // REVIEW:
                for chunk in attr_def.block.iter_mut() {
                    self.resolve_pymod_path(chunk);
                }
            }
            Expr::TypeAsc(tasc) => self.resolve_pymod_path(&mut tasc.expr),
            Expr::Code(chunks) | Expr::Compound(chunks) => {
                for chunk in chunks.iter_mut() {
                    self.resolve_pymod_path(chunk);
                }
            }
            Expr::Import(_) => unreachable!(),
        }
    }

    fn replace_import(&self, expr: &mut Expr) {
        match expr {
            Expr::Lit(_) => {}
            Expr::Accessor(acc) => {
                /*if acc.ref_t().is_py_module() {
                    let import = Expr::Import(acc.clone());
                    *expr = Expr::Compound(Block::new(vec![import, mem::take(expr)]));
                }*/
                match acc {
                    Accessor::Attr(attr) => {
                        self.replace_import(&mut attr.obj);
                    }
                    Accessor::Ident(_) => {}
                }
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    for elem in arr.elems.pos_args.iter_mut() {
                        self.replace_import(&mut elem.expr);
                    }
                }
                Array::WithLength(arr) => {
                    self.replace_import(&mut arr.elem);
                    self.replace_import(&mut arr.len);
                }
                _ => todo!(),
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    for elem in tup.elems.pos_args.iter_mut() {
                        self.replace_import(&mut elem.expr);
                    }
                }
            },
            Expr::Set(set) => match set {
                Set::Normal(st) => {
                    for elem in st.elems.pos_args.iter_mut() {
                        self.replace_import(&mut elem.expr);
                    }
                }
                Set::WithLength(st) => {
                    self.replace_import(&mut st.elem);
                    self.replace_import(&mut st.len);
                }
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dic) => {
                    for elem in dic.kvs.iter_mut() {
                        self.replace_import(&mut elem.key);
                        self.replace_import(&mut elem.value);
                    }
                }
                other => todo!("{other}"),
            },
            Expr::Record(record) => {
                for attr in record.attrs.iter_mut() {
                    for chunk in attr.body.block.iter_mut() {
                        self.replace_import(chunk);
                    }
                }
            }
            Expr::BinOp(binop) => {
                self.replace_import(&mut binop.lhs);
                self.replace_import(&mut binop.rhs);
            }
            Expr::UnaryOp(unaryop) => {
                self.replace_import(&mut unaryop.expr);
            }
            Expr::Call(call) => match call.additional_operation() {
                Some(OperationKind::Import) => {
                    self.replace_erg_import(expr);
                }
                Some(OperationKind::PyImport) => {
                    self.replace_py_import(expr);
                }
                _ => {
                    self.replace_import(&mut call.obj);
                    for arg in call.args.pos_args.iter_mut() {
                        self.replace_import(&mut arg.expr);
                    }
                    for arg in call.args.kw_args.iter_mut() {
                        self.replace_import(&mut arg.expr);
                    }
                }
            },
            Expr::Decl(_decl) => {}
            Expr::Def(def) => {
                for chunk in def.body.block.iter_mut() {
                    self.replace_import(chunk);
                }
            }
            Expr::Lambda(lambda) => {
                for chunk in lambda.body.iter_mut() {
                    self.replace_import(chunk);
                }
            }
            Expr::ClassDef(class_def) => {
                for def in class_def.methods.iter_mut() {
                    self.replace_import(def);
                }
            }
            Expr::AttrDef(attr_def) => {
                // REVIEW:
                for chunk in attr_def.block.iter_mut() {
                    self.replace_import(chunk);
                }
            }
            Expr::TypeAsc(tasc) => self.replace_import(&mut tasc.expr),
            Expr::Code(chunks) | Expr::Compound(chunks) => {
                for chunk in chunks.iter_mut() {
                    self.replace_import(chunk);
                }
            }
            Expr::Import(_) => unreachable!(),
        }
    }

    /// ```erg
    /// x = import "mod"
    /// ```
    /// ↓
    /// ```python
    /// x =
    ///     _x = ModuleType("mod")
    ///     _x.__dict__.update(locals()) # `Nat`, etc. are in locals but treated as globals, so they cannot be put in the third argument of exec.
    ///     exec(code, _x.__dict__)  # `code` is the mod's content
    ///     _x
    /// ```
    fn replace_erg_import(&self, expr: &mut Expr) {
        let line = expr.ln_begin().unwrap_or(0);
        let path =
            enum_unwrap!(expr.ref_t().typarams().remove(0), TyParam::Value:(ValueObj::Str:(_)));
        let path = Path::new(&path[..]);
        let path = self.cfg.input.local_resolve(path).unwrap();
        // In the case of REPL, entries cannot be used up
        let hir = if self.cfg.input.is_repl() {
            self.mod_cache
                .get(path.as_path())
                .and_then(|entry| entry.hir.clone())
        } else {
            self.mod_cache
                .remove(path.as_path())
                .and_then(|entry| entry.hir)
        };
        let mod_name = enum_unwrap!(expr, Expr::Call)
            .args
            .get_left_or_key("Path")
            .unwrap();
        // let sig = option_enum_unwrap!(&def.sig, Signature::Var)
        //    .unwrap_or_else(|| todo!("module subroutines are not allowed"));
        if let Some(hir) = hir {
            let code = Expr::Code(Block::new(Vec::from(hir.module)));
            let module_type =
                Expr::Accessor(Accessor::private_with_line(Str::ever("#ModuleType"), line));
            let args = Args::new(vec![PosArg::new(mod_name.clone())], None, vec![], None);
            let block = Block::new(vec![module_type.call_expr(args)]);
            let tmp =
                Identifier::private_with_line(Str::from(fresh_varname()), expr.ln_begin().unwrap());
            let mod_def = Expr::Def(Def::new(
                Signature::Var(VarSignature::new(tmp.clone())),
                DefBody::new(Token::dummy(), block, DefId(0)),
            ));
            let module = Expr::Accessor(Accessor::Ident(tmp));
            let __dict__ = Identifier::public("__dict__");
            let m_dict = module.clone().attr_expr(__dict__);
            let locals = Expr::Accessor(Accessor::public_with_line(Str::ever("locals"), line));
            let locals_call = locals.call_expr(Args::empty());
            let args = Args::new(vec![PosArg::new(locals_call)], None, vec![], None);
            let mod_update = Expr::Call(Call::new(
                m_dict.clone(),
                Some(Identifier::public("update")),
                args,
            ));
            let exec = Expr::Accessor(Accessor::public_with_line(Str::ever("exec"), line));
            let args = Args::new(
                vec![PosArg::new(code), PosArg::new(m_dict)],
                None,
                vec![],
                None,
            );
            let exec_code = exec.call_expr(args);
            let compound = Block::new(vec![mod_def, mod_update, exec_code, module]);
            *expr = Expr::Compound(compound);
        }
    }

    /// ```erg
    /// x = pyimport "x" # called from dir "a"
    /// ```
    /// ↓
    /// ```python
    /// x = __import__("a.x").x
    /// ```
    fn replace_py_import(&self, expr: &mut Expr) {
        let mut dir = if let Input::File(mut path) = self.cfg.input.clone() {
            path.pop();
            path
        } else {
            PathBuf::new()
        };
        let args = &mut enum_unwrap!(expr, Expr::Call).args;
        let mod_name_lit = enum_unwrap!(args.remove_left_or_key("Path").unwrap(), Expr::Lit);
        let mod_name_str = enum_unwrap!(mod_name_lit.value.clone(), ValueObj::Str);
        if BUILTIN_PYTHON_MODS.contains(&&mod_name_str[..]) {
            args.push_pos(PosArg::new(Expr::Lit(mod_name_lit)));
            return;
        }
        let mod_name_str = if let Some(stripped) = mod_name_str.strip_prefix("./") {
            stripped
        } else {
            &mod_name_str
        };
        dir.push(mod_name_str);
        let mut comps = dir.components();
        let _first = comps.next().unwrap();
        let path = dir.to_string_lossy().replace('/', ".").replace('\\', ".");
        let token = Token::new(
            TokenKind::StrLit,
            path,
            mod_name_lit.ln_begin().unwrap(),
            mod_name_lit.col_begin().unwrap(),
        );
        let mod_name = Expr::Lit(Literal::try_from(token).unwrap());
        args.insert_pos(0, PosArg::new(mod_name));
        let line = expr.ln_begin().unwrap_or(0);
        for attr in comps {
            *expr = mem::replace(expr, Expr::Code(Block::empty())).attr_expr(
                Identifier::public_with_line(
                    Token::dummy(),
                    Str::rc(attr.as_os_str().to_str().unwrap()),
                    line,
                ),
            );
        }
    }
}
