use erg_common::traits::{Locational, Runnable, Stream};
use erg_common::{enum_unwrap, fn_name, log, Str};

use erg_parser::ast;
use erg_parser::ast::AST;

use crate::context::instantiate::TyVarCache;
use crate::lower::ASTLowerer;
use crate::ty::constructors::mono;
use crate::ty::free::HasLevel;
use crate::ty::value::{GenTypeObj, TypeObj};
use crate::ty::{HasType, Type};

use crate::context::RegistrationMode;
use crate::error::{LowerError, LowerErrors, LowerResult};
use crate::hir;
use crate::hir::HIR;
use crate::varinfo::{Mutability, VarInfo, VarKind};

impl ASTLowerer {
    fn declare_var(
        &mut self,
        sig: ast::VarSignature,
        mut body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        log!(info "entered {}({sig})", fn_name!());
        if body.block.len() > 1 {
            return Err(LowerErrors::from(LowerError::declare_error(
                self.cfg().input.clone(),
                line!() as usize,
                body.block.loc(),
                self.module.context.caused_by(),
            )));
        }
        let opt_spec_t = if let Some(t_spec) = &sig.t_spec {
            let mut dummy_tv_cache =
                TyVarCache::new(self.module.context.level, &self.module.context);
            let t = self.module.context.instantiate_typespec(
                t_spec,
                None,
                &mut dummy_tv_cache,
                RegistrationMode::Normal,
                false,
            )?;
            t.lift();
            Some(self.module.context.generalize_t(t))
        } else {
            None
        };
        let chunk = self.declare_chunk(body.block.remove(0))?;
        let py_name = if let hir::Expr::TypeAsc(tasc) = &chunk {
            enum_unwrap!(tasc.expr.as_ref(), hir::Expr::Accessor)
                .local_name()
                .map(Str::rc)
        } else {
            sig.inspect().cloned()
        };
        let block = hir::Block::new(vec![chunk]);
        let found_body_t = block.ref_t();
        let ast::VarPattern::Ident(ident) = &sig.pat else { unreachable!() };
        let id = body.id;
        if let Some(spec_t) = opt_spec_t {
            self.module
                .context
                .sub_unify(found_body_t, &spec_t, sig.loc(), None)?;
        }
        if let Some(py_name) = &py_name {
            self.declare_instance(ident, found_body_t, py_name.clone())?;
        } else {
            self.module
                .context
                .assign_var_sig(&sig, found_body_t, id, py_name.clone())?;
        }
        let mut ident = hir::Identifier::bare(ident.dot.clone(), ident.name.clone());
        ident.vi.t = found_body_t.clone();
        ident.vi.py_name = py_name;
        let sig = hir::VarSignature::new(ident);
        let body = hir::DefBody::new(body.op, block, body.id);
        Ok(hir::Def::new(hir::Signature::Var(sig), body))
    }

    /// allowed: alias, import, const functions (e.g. Class)
    fn declare_def(&mut self, def: ast::Def) -> LowerResult<hir::Def> {
        log!(info "entered {}({})", fn_name!(), def.sig);
        let name = if let Some(name) = def.sig.name_as_str() {
            name.clone()
        } else {
            Str::ever("<lambda>")
        };
        if self
            .module
            .context
            .registered_info(&name, def.sig.is_const())
            .is_some()
            && def.sig.vis().is_private()
        {
            return Err(LowerErrors::from(LowerError::reassign_error(
                self.cfg().input.clone(),
                line!() as usize,
                def.sig.loc(),
                self.module.context.caused_by(),
                &name,
            )));
        }
        #[allow(clippy::let_and_return)]
        let res = match def.sig {
            ast::Signature::Subr(sig) => {
                return Err(LowerErrors::from(LowerError::declare_error(
                    self.cfg().input.clone(),
                    line!() as usize,
                    sig.loc(),
                    self.module.context.caused_by(),
                )));
            }
            ast::Signature::Var(sig) => self.declare_var(sig, def.body),
        };
        // self.pop_append_errs();
        res
    }

    fn declare_class_def(&mut self, _class_def: ast::ClassDef) -> LowerResult<hir::ClassDef> {
        todo!()
    }

    fn fake_lower_obj(&self, obj: ast::Expr) -> LowerResult<hir::Expr> {
        match obj {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) => {
                let acc = hir::Accessor::Ident(hir::Identifier::bare(ident.dot, ident.name));
                Ok(hir::Expr::Accessor(acc))
            }
            ast::Expr::Accessor(ast::Accessor::Attr(attr)) => {
                let obj = self.fake_lower_obj(*attr.obj)?;
                let ident = hir::Identifier::bare(attr.ident.dot, attr.ident.name);
                Ok(obj.attr_expr(ident))
            }
            other => Err(LowerErrors::from(LowerError::declare_error(
                self.cfg().input.clone(),
                line!() as usize,
                other.loc(),
                self.module.context.caused_by(),
            ))),
        }
    }

    fn declare_ident(&mut self, tasc: ast::TypeAscription) -> LowerResult<hir::TypeAscription> {
        log!(info "entered {}({})", fn_name!(), tasc);
        let is_instance_ascription = tasc.is_instance_ascription();
        let mut dummy_tv_cache = TyVarCache::new(self.module.context.level, &self.module.context);
        match *tasc.expr {
            ast::Expr::Accessor(ast::Accessor::Ident(mut ident)) => {
                if cfg!(feature = "py_compatible") {
                    ident.trim_end_proc_mark();
                }
                let py_name = Str::rc(ident.inspect().trim_end_matches('!'));
                let t = self.module.context.instantiate_typespec(
                    &tasc.t_spec,
                    None,
                    &mut dummy_tv_cache,
                    RegistrationMode::Normal,
                    false,
                )?;
                t.lift();
                let t = self.module.context.generalize_t(t);
                if is_instance_ascription {
                    self.declare_instance(&ident, &t, py_name)?;
                } else {
                    self.declare_subtype(&ident, &t)?;
                }
                let muty = Mutability::from(&ident.inspect()[..]);
                let vis = ident.vis();
                let py_name = Str::rc(ident.inspect().trim_end_matches('!'));
                let vi = VarInfo::new(t, muty, vis, VarKind::Declared, None, None, Some(py_name));
                let ident = hir::Identifier::new(ident.dot, ident.name, None, vi);
                Ok(hir::Expr::Accessor(hir::Accessor::Ident(ident)).type_asc(tasc.t_spec))
            }
            ast::Expr::Accessor(ast::Accessor::Attr(mut attr)) => {
                if cfg!(feature = "py_compatible") {
                    attr.ident.trim_end_proc_mark();
                }
                let py_name = Str::rc(attr.ident.inspect().trim_end_matches('!'));
                let t = self.module.context.instantiate_typespec(
                    &tasc.t_spec,
                    None,
                    &mut dummy_tv_cache,
                    RegistrationMode::Normal,
                    false,
                )?;
                let namespace = self.module.context.name.clone();
                let ctx = self
                    .module
                    .context
                    .get_mut_singular_ctx(attr.obj.as_ref(), &namespace)?;
                ctx.assign_var_sig(
                    &ast::VarSignature::new(ast::VarPattern::Ident(attr.ident.clone()), None),
                    &t,
                    ast::DefId(0),
                    Some(py_name),
                )?;
                let obj = self.fake_lower_obj(*attr.obj)?;
                let muty = Mutability::from(&attr.ident.inspect()[..]);
                let vis = attr.ident.vis();
                let py_name = Str::rc(attr.ident.inspect().trim_end_matches('!'));
                let vi = VarInfo::new(t, muty, vis, VarKind::Declared, None, None, Some(py_name));
                let ident = hir::Identifier::new(attr.ident.dot, attr.ident.name, None, vi);
                let attr = obj.attr_expr(ident);
                Ok(attr.type_asc(tasc.t_spec))
            }
            other => Err(LowerErrors::from(LowerError::declare_error(
                self.cfg().input.clone(),
                line!() as usize,
                other.loc(),
                self.module.context.caused_by(),
            ))),
        }
    }

    fn declare_instance(
        &mut self,
        ident: &ast::Identifier,
        t: &Type,
        py_name: Str,
    ) -> LowerResult<()> {
        // .X = 'x': Type
        if ident.is_raw() {
            return Ok(());
        }
        if ident.is_const() {
            let vi = VarInfo::new(
                t.clone(),
                Mutability::Const,
                ident.vis(),
                VarKind::Declared,
                None,
                None,
                Some(py_name.clone()),
            );
            self.module.context.decls.insert(ident.name.clone(), vi);
        }
        self.module.context.assign_var_sig(
            &ast::VarSignature::new(ast::VarPattern::Ident(ident.clone()), None),
            t,
            ast::DefId(0),
            Some(py_name),
        )?;
        match t {
            Type::ClassType => {
                let ty_obj = GenTypeObj::class(
                    mono(format!("{}{ident}", self.module.context.path())),
                    Some(TypeObj::Builtin(Type::Uninited)),
                    None,
                );
                self.module.context.register_gen_type(ident, ty_obj);
            }
            Type::TraitType => {
                let ty_obj = GenTypeObj::trait_(
                    mono(format!("{}{ident}", self.module.context.path())),
                    TypeObj::Builtin(Type::Uninited),
                    None,
                );
                self.module.context.register_gen_type(ident, ty_obj);
            }
            _ => {}
        }
        Ok(())
    }

    fn declare_subtype(&mut self, ident: &ast::Identifier, trait_: &Type) -> LowerResult<()> {
        if ident.is_raw() {
            return Ok(());
        }
        if let Some((_, ctx)) = self.module.context.get_mut_type(ident.inspect()) {
            ctx.register_marker_trait(trait_.clone());
            Ok(())
        } else {
            Err(LowerErrors::from(LowerError::no_var_error(
                self.cfg().input.clone(),
                line!() as usize,
                ident.loc(),
                self.module.context.caused_by(),
                ident.inspect(),
                self.module.context.get_similar_name(ident.inspect()),
            )))
        }
    }

    fn declare_chunk(&mut self, expr: ast::Expr) -> LowerResult<hir::Expr> {
        log!(info "entered {}", fn_name!());
        match expr {
            ast::Expr::Def(def) => Ok(hir::Expr::Def(self.declare_def(def)?)),
            ast::Expr::ClassDef(class_def) => {
                Ok(hir::Expr::ClassDef(self.declare_class_def(class_def)?))
            }
            ast::Expr::TypeAsc(tasc) => Ok(hir::Expr::TypeAsc(self.declare_ident(tasc)?)),
            ast::Expr::Call(call)
                if call
                    .additional_operation()
                    .map(|op| op.is_import())
                    .unwrap_or(false) =>
            {
                Ok(hir::Expr::Call(self.lower_call(call)?))
            }
            other => Err(LowerErrors::from(LowerError::declare_error(
                self.cfg().input.clone(),
                line!() as usize,
                other.loc(),
                self.module.context.caused_by(),
            ))),
        }
    }

    pub(crate) fn declare_module(&mut self, ast: AST) -> HIR {
        let mut module = hir::Module::with_capacity(ast.module.len());
        for chunk in ast.module.into_iter() {
            match self.declare_chunk(chunk) {
                Ok(chunk) => {
                    module.push(chunk);
                }
                Err(errs) => {
                    self.errs.extend(errs);
                }
            }
        }
        HIR::new(ast.name, module)
    }
}
