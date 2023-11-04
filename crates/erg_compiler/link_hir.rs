use std::cell::RefCell;
use std::mem::{replace, take};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict as Dic;
use erg_common::fresh::SharedFreshNameGenerator;
use erg_common::log;
use erg_common::pathutil::squash;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;

use erg_parser::ast::{DefId, OperationKind};
use erg_parser::token::{Token, TokenKind, DOT, EQUAL};

use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::HasType;

use crate::hir::*;
use crate::module::SharedModuleCache;

pub struct Mod {
    variable: Expr,
    definition: Expr,
}

impl Mod {
    const fn new(variable: Expr, definition: Expr) -> Self {
        Self {
            variable,
            definition,
        }
    }
}

/// Link code using the module cache.
/// Erg links all non-Python modules into a single pyc file.
pub struct HIRLinker<'a> {
    cfg: &'a ErgConfig,
    mod_cache: &'a SharedModuleCache,
    removed_mods: Rc<RefCell<Dic<PathBuf, Mod>>>,
    fresh_gen: SharedFreshNameGenerator,
}

impl<'a> HIRLinker<'a> {
    pub fn new(cfg: &'a ErgConfig, mod_cache: &'a SharedModuleCache) -> Self {
        Self {
            cfg,
            mod_cache,
            removed_mods: Rc::new(RefCell::new(Dic::new())),
            fresh_gen: SharedFreshNameGenerator::new("hir_linker"),
        }
    }

    fn inherit(&self, cfg: &'a ErgConfig) -> Self {
        Self {
            cfg,
            mod_cache: self.mod_cache,
            removed_mods: self.removed_mods.clone(),
            fresh_gen: self.fresh_gen.clone(),
        }
    }

    pub fn link(&self, mut main: HIR) -> HIR {
        log!(info "the linking process has started.");
        for chunk in main.module.iter_mut() {
            self.replace_import(chunk);
        }
        // declare all modules first (due to cyclic modules)
        for (i, module) in self.removed_mods.borrow_mut().values_mut().enumerate() {
            main.module.insert(i, take(&mut module.definition));
        }
        for chunk in main.module.iter_mut() {
            Self::resolve_pymod_path(chunk);
        }
        log!(info "linked:\n{main}");
        main
    }

    fn link_child(&self, mut hir: HIR) -> HIR {
        for chunk in hir.module.iter_mut() {
            self.replace_import(chunk);
        }
        for chunk in hir.module.iter_mut() {
            Self::resolve_pymod_path(chunk);
        }
        hir
    }

    /// ```erg
    /// urllib = pyimport "urllib"
    /// urllib.request.urlopen! "https://example.com"
    /// ```
    /// ↓
    /// ```python
    /// urllib = __import__("urllib")
    /// import urllib.request
    /// urllib.request.urlopen("https://example.com")
    /// ```
    /// other example:
    /// ```erg
    /// mpl = pyimport "matplotlib"
    /// mpl.pyplot.plot! [1, 2, 3]
    /// ```
    /// ↓
    /// ```python
    /// mpl = __import__("matplotlib")
    /// import matplotlib.pyplot # mpl.pyplot.foo is now allowed
    /// mpl.pyplot.plot([1, 2, 3])
    /// ```
    fn resolve_pymod_path(expr: &mut Expr) {
        match expr {
            Expr::Literal(_) => {}
            Expr::Accessor(acc) => {
                if let Accessor::Attr(attr) = acc {
                    Self::resolve_pymod_path(&mut attr.obj);
                    if acc.ref_t().is_py_module() {
                        let import = Expr::Import(acc.clone());
                        *expr = Expr::Compound(Block::new(vec![import, take(expr)]));
                    }
                }
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    for elem in arr.elems.pos_args.iter_mut() {
                        Self::resolve_pymod_path(&mut elem.expr);
                    }
                }
                Array::WithLength(arr) => {
                    Self::resolve_pymod_path(&mut arr.elem);
                    if let Some(len) = arr.len.as_deref_mut() {
                        Self::resolve_pymod_path(len);
                    }
                }
                _ => todo!(),
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    for elem in tup.elems.pos_args.iter_mut() {
                        Self::resolve_pymod_path(&mut elem.expr);
                    }
                }
            },
            Expr::Set(set) => match set {
                Set::Normal(st) => {
                    for elem in st.elems.pos_args.iter_mut() {
                        Self::resolve_pymod_path(&mut elem.expr);
                    }
                }
                Set::WithLength(st) => {
                    Self::resolve_pymod_path(&mut st.elem);
                    Self::resolve_pymod_path(&mut st.len);
                }
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dic) => {
                    for elem in dic.kvs.iter_mut() {
                        Self::resolve_pymod_path(&mut elem.key);
                        Self::resolve_pymod_path(&mut elem.value);
                    }
                }
                other => todo!("{other}"),
            },
            Expr::Record(record) => {
                for attr in record.attrs.iter_mut() {
                    for chunk in attr.body.block.iter_mut() {
                        Self::resolve_pymod_path(chunk);
                    }
                }
            }
            Expr::BinOp(binop) => {
                Self::resolve_pymod_path(&mut binop.lhs);
                Self::resolve_pymod_path(&mut binop.rhs);
            }
            Expr::UnaryOp(unaryop) => {
                Self::resolve_pymod_path(&mut unaryop.expr);
            }
            Expr::Call(call) => {
                Self::resolve_pymod_path(&mut call.obj);
                for arg in call.args.pos_args.iter_mut() {
                    Self::resolve_pymod_path(&mut arg.expr);
                }
                for arg in call.args.kw_args.iter_mut() {
                    Self::resolve_pymod_path(&mut arg.expr);
                }
            }
            Expr::Def(def) => {
                for chunk in def.body.block.iter_mut() {
                    Self::resolve_pymod_path(chunk);
                }
            }
            Expr::Lambda(lambda) => {
                for chunk in lambda.body.iter_mut() {
                    Self::resolve_pymod_path(chunk);
                }
            }
            Expr::ClassDef(class_def) => {
                for def in class_def.all_methods_mut() {
                    Self::resolve_pymod_path(def);
                }
            }
            Expr::PatchDef(patch_def) => {
                for def in patch_def.methods.iter_mut() {
                    Self::resolve_pymod_path(def);
                }
            }
            Expr::ReDef(redef) => {
                // REVIEW:
                for chunk in redef.block.iter_mut() {
                    Self::resolve_pymod_path(chunk);
                }
            }
            Expr::TypeAsc(tasc) => Self::resolve_pymod_path(&mut tasc.expr),
            Expr::Code(chunks) | Expr::Compound(chunks) => {
                for chunk in chunks.iter_mut() {
                    Self::resolve_pymod_path(chunk);
                }
            }
            Expr::Import(_) => {}
            Expr::Dummy(_) => {}
        }
    }

    fn replace_import(&self, expr: &mut Expr) {
        match expr {
            Expr::Literal(_) => {}
            Expr::Accessor(acc) => {
                /*if acc.ref_t().is_py_module() {
                    let import = Expr::Import(acc.clone());
                    *expr = Expr::Compound(Block::new(vec![import, mem::take(expr)]));
                }*/
                match acc {
                    Accessor::Attr(attr) => {
                        self.replace_import(&mut attr.obj);
                    }
                    Accessor::Ident(ident) => match &ident.inspect()[..] {
                        "module" => {
                            *expr = Self::self_module();
                        }
                        "global" => {
                            *expr = Expr::from(Identifier::public("__builtins__"));
                        }
                        _ => {}
                    },
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
                    if let Some(len) = arr.len.as_deref_mut() {
                        self.replace_import(len);
                    }
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
                    if let Some(arg) = call.args.var_args.as_deref_mut() {
                        self.replace_py_import(&mut arg.expr);
                    }
                    for arg in call.args.kw_args.iter_mut() {
                        self.replace_import(&mut arg.expr);
                    }
                }
            },
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
                for def in class_def.all_methods_mut() {
                    self.replace_import(def);
                }
            }
            Expr::PatchDef(patch_def) => {
                for def in patch_def.methods.iter_mut() {
                    self.replace_import(def);
                }
            }
            Expr::ReDef(redef) => {
                // REVIEW:
                for chunk in redef.block.iter_mut() {
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
            Expr::Dummy(_) => {}
        }
    }

    fn self_module() -> Expr {
        let __import__ = Identifier::public("__import__");
        let __name__ = Identifier::public("__name__");
        Expr::from(__import__).call1(Expr::from(__name__))
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
        let TyParam::Value(ValueObj::Str(path)) = expr.ref_t().typarams().remove(0) else {
            unreachable!()
        };
        let path = Path::new(&path[..]);
        let path = self.cfg.input.resolve_real_path(path).unwrap();
        // # module.er
        // self = import "module"
        // ↓
        // # module.er
        // self = __import__(__name__)
        if matches!((path.canonicalize(), self.cfg.input.path().canonicalize()), (Ok(l), Ok(r)) if l == r)
        {
            *expr = Self::self_module();
            return;
        }
        // In the case of REPL, entries cannot be used up
        let hir_cfg = if self.cfg.input.is_repl() {
            self.mod_cache
                .get(path.as_path())
                .and_then(|entry| entry.hir.clone().map(|hir| (hir, entry.cfg().clone())))
        } else {
            self.mod_cache
                .remove(path.as_path())
                .and_then(|entry| entry.hir.map(|hir| (hir, entry.module.context.cfg.clone())))
        };
        let Expr::Call(call) = expr else {
            log!(err "{expr}");
            return;
        };
        let Some(mod_name) = call.args.get_left_or_key("Path") else {
            log!(err "{call}");
            return;
        };
        // let sig = option_enum_unwrap!(&def.sig, Signature::Var)
        //    .unwrap_or_else(|| todo!("module subroutines are not allowed"));
        if let Some((hir, cfg)) = hir_cfg {
            *expr = self.modularize(mod_name.clone(), hir, cfg, line, path);
        } else if let Some(module) = self.removed_mods.borrow().get(&path) {
            *expr = module.variable.clone();
        }
    }

    fn modularize(
        &self,
        mod_name: Expr,
        hir: HIR,
        cfg: ErgConfig,
        line: u32,
        path: PathBuf,
    ) -> Expr {
        let tmp = Identifier::private_with_line(self.fresh_gen.fresh_varname(), line);
        let mod_var = Expr::Accessor(Accessor::Ident(tmp.clone()));
        let module_type =
            Expr::Accessor(Accessor::private_with_line(Str::ever("#ModuleType"), line));
        let args = Args::single(PosArg::new(mod_name));
        let block = Block::new(vec![module_type.call_expr(args)]);
        let mod_def = Expr::Def(Def::new(
            Signature::Var(VarSignature::global(tmp, None)),
            DefBody::new(EQUAL, block, DefId(0)),
        ));
        self.removed_mods
            .borrow_mut()
            .insert(path, Mod::new(mod_var.clone(), mod_def));
        let linker = self.inherit(&cfg);
        let hir = linker.link_child(hir);
        let code = Expr::Code(Block::new(Vec::from(hir.module)));
        let __dict__ = Identifier::public("__dict__");
        let m_dict = mod_var.clone().attr_expr(__dict__);
        let locals = Expr::Accessor(Accessor::public_with_line(Str::ever("locals"), line));
        let locals_call = locals.call_expr(Args::empty());
        let args = Args::single(PosArg::new(locals_call));
        let mod_update = Expr::Call(Call::new(
            m_dict.clone(),
            Some(Identifier::public("update")),
            args,
        ));
        let exec = Expr::Accessor(Accessor::public_with_line(Str::ever("exec"), line));
        let args = Args::pos_only(vec![PosArg::new(code), PosArg::new(m_dict)], None);
        let exec_code = exec.call_expr(args);
        let compound = Block::new(vec![mod_update, exec_code, mod_var]);
        Expr::Compound(compound)
    }

    /// ```erg
    /// x = pyimport "x" # called from dir "a"
    /// ```
    /// ↓
    /// ```python
    /// x = __import__("a.x").x
    /// ```
    fn replace_py_import(&self, expr: &mut Expr) {
        let args = if let Expr::Call(call) = expr {
            &mut call.args
        } else {
            log!(err "{expr}");
            return;
        };
        let Some(Expr::Literal(mod_name_lit)) = args.remove_left_or_key("Path") else {
            log!(err "{args}");
            return;
        };
        let ValueObj::Str(mod_name_str) = mod_name_lit.value.clone() else {
            log!(err "{mod_name_lit}");
            return;
        };
        let mut dir = self.cfg.input.dir();
        let mod_path = self
            .cfg
            .input
            .resolve_decl_path(Path::new(&mod_name_str[..]))
            .unwrap();
        if !mod_path
            .canonicalize()
            .unwrap()
            .starts_with(&dir.canonicalize().unwrap())
        {
            dir = PathBuf::new();
        }
        let mod_name_str = if let Some(stripped) = mod_name_str.strip_prefix("./") {
            stripped
        } else {
            &mod_name_str
        };
        dir.push(mod_name_str);
        let dir = squash(dir);
        let mut comps = dir.components();
        let _first = comps.next().unwrap();
        let path = dir.to_string_lossy().replace(['/', '\\'], ".");
        let token = Token::new_fake(
            TokenKind::StrLit,
            format!("\"{path}\""),
            mod_name_lit.ln_begin().unwrap(),
            mod_name_lit.col_begin().unwrap(),
            mod_name_lit.col_end().unwrap(),
        );
        let mod_name = Expr::Literal(Literal::try_from(token).unwrap());
        args.insert_pos(0, PosArg::new(mod_name));
        let line = expr.ln_begin().unwrap_or(0);
        for attr in comps {
            *expr =
                replace(expr, Expr::Code(Block::empty())).attr_expr(Identifier::public_with_line(
                    DOT,
                    Str::rc(attr.as_os_str().to_str().unwrap()),
                    line,
                ));
        }
    }
}
