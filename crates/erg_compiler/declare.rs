use erg_common::consts::PYTHON_MODE;
use erg_common::traits::{Locational, Runnable, Stream};
use erg_common::{enum_unwrap, fn_name, log, set, Str};

use erg_parser::ast::{self, AscriptionKind, Identifier, VarName, AST};

use crate::lower::ASTLowerer;
use crate::ty::constructors::{mono, v_enum};
use crate::ty::free::HasLevel;
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{HasType, Type, Visibility};

use crate::compile::AccessKind;
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
            let t = self.module.context.instantiate_typespec(&t_spec.t_spec)?;
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
        let found_body_t = chunk.ref_t();
        let ast::VarPattern::Ident(ident) = &sig.pat else { unreachable!() };
        let id = body.id;
        if let Some(spec_t) = opt_spec_t {
            self.module
                .context
                .sub_unify(found_body_t, &spec_t, &sig, None)?;
        }
        if let Some(py_name) = &py_name {
            self.declare_instance(ident, found_body_t, py_name.clone())?;
        } else {
            self.module
                .context
                .assign_var_sig(&sig, found_body_t, id, None)?;
        }
        let mut ident = hir::Identifier::bare(ident.clone());
        let t = match found_body_t {
            Type::ClassType => {
                let t = mono(format!("{}{}", self.module.context.path(), ident.raw));
                v_enum(set! { ValueObj::builtin_class(t) })
            }
            Type::TraitType => {
                let t = mono(format!("{}{}", self.module.context.path(), ident.raw));
                v_enum(set! { ValueObj::builtin_trait(t) })
            }
            _ => found_body_t.clone(),
        };
        // Typ = 'typ': ClassType
        // => 'typ': {<type Typ>}
        if let hir::Expr::TypeAsc(hir::TypeAscription { expr, .. }) = &chunk {
            if let hir::Expr::Accessor(acc) = expr.as_ref() {
                if let Some(name) = acc.local_name() {
                    let name = VarName::from_str(Str::rc(name));
                    if let Some(vi) = self.module.context.get_mut_current_scope_var(&name) {
                        vi.t = t.clone();
                    }
                }
            }
        }
        ident.vi.t = t;
        ident.vi.py_name = py_name;
        ident.vi.def_loc = self.module.context.absolutize(ident.raw.name.loc());
        let t_spec = if let Some(ts) = sig.t_spec {
            let spec_t = self.module.context.instantiate_typespec(&ts.t_spec)?;
            let expr = self.fake_lower_expr(*ts.t_spec_as_expr.clone())?;
            Some(hir::TypeSpecWithOp::new(ts, expr, spec_t))
        } else {
            None
        };
        let sig = hir::VarSignature::new(ident, t_spec);
        let body = hir::DefBody::new(body.op, hir::Block::new(vec![chunk]), body.id);
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

    fn fake_lower_acc(&self, acc: ast::Accessor) -> LowerResult<hir::Accessor> {
        match acc {
            ast::Accessor::Ident(ident) => {
                // to resolve `py_name`
                let vi = self
                    .module
                    .context
                    .rec_get_var_info(&ident, AccessKind::Name, self.input(), &self.module.context)
                    .unwrap_or(VarInfo::default());
                let ident = hir::Identifier::new(ident, None, vi);
                let acc = hir::Accessor::Ident(ident);
                Ok(acc)
            }
            ast::Accessor::Attr(attr) => {
                let obj = self.fake_lower_expr(*attr.obj)?;
                let ident = hir::Identifier::bare(attr.ident);
                Ok(obj.attr(ident))
            }
            other => Err(LowerErrors::from(LowerError::declare_error(
                self.cfg().input.clone(),
                line!() as usize,
                other.loc(),
                self.module.context.caused_by(),
            ))),
        }
    }

    fn fake_lower_args(&self, args: ast::Args) -> LowerResult<hir::Args> {
        let (pos_args_, var_args_, kw_args_, paren) = args.deconstruct();
        let mut pos_args = vec![];
        for arg in pos_args_.into_iter() {
            let arg = self.fake_lower_expr(arg.expr)?;
            pos_args.push(hir::PosArg::new(arg));
        }
        let var_args = match var_args_ {
            Some(var_args) => {
                let var_args = self.fake_lower_expr(var_args.expr)?;
                Some(hir::PosArg::new(var_args))
            }
            None => None,
        };
        let mut kw_args = vec![];
        for kw_arg in kw_args_.into_iter() {
            let expr = self.fake_lower_expr(kw_arg.expr)?;
            kw_args.push(hir::KwArg::new(kw_arg.keyword, expr));
        }
        let args = hir::Args::new(pos_args, var_args, kw_args, paren);
        Ok(args)
    }

    fn fake_lower_call(&self, call: ast::Call) -> LowerResult<hir::Call> {
        let obj = self.fake_lower_expr(*call.obj)?;
        let attr_name = call.attr_name.map(hir::Identifier::bare);
        let args = self.fake_lower_args(call.args)?;
        Ok(hir::Call::new(obj, attr_name, args))
    }

    fn fake_lower_binop(&self, binop: ast::BinOp) -> LowerResult<hir::BinOp> {
        let mut args = binop.args.into_iter();
        let lhs = self.fake_lower_expr(*args.next().unwrap())?;
        let rhs = self.fake_lower_expr(*args.next().unwrap())?;
        Ok(hir::BinOp::new(binop.op, lhs, rhs, VarInfo::default()))
    }

    fn fake_lower_unaryop(&self, unaryop: ast::UnaryOp) -> LowerResult<hir::UnaryOp> {
        let mut args = unaryop.args.into_iter();
        let expr = self.fake_lower_expr(*args.next().unwrap())?;
        Ok(hir::UnaryOp::new(unaryop.op, expr, VarInfo::default()))
    }

    fn fake_lower_array(&self, arr: ast::Array) -> LowerResult<hir::Array> {
        match arr {
            ast::Array::WithLength(arr) => {
                let len = self.fake_lower_expr(*arr.len)?;
                let elem = self.fake_lower_expr(arr.elem.expr)?;
                Ok(hir::Array::WithLength(hir::ArrayWithLength::new(
                    arr.l_sqbr,
                    arr.r_sqbr,
                    Type::Failure,
                    len,
                    elem,
                )))
            }
            ast::Array::Normal(arr) => {
                let mut elems = Vec::new();
                let (elems_, ..) = arr.elems.deconstruct();
                for elem in elems_.into_iter() {
                    let elem = self.fake_lower_expr(elem.expr)?;
                    elems.push(hir::PosArg::new(elem));
                }
                let elems = hir::Args::new(elems, None, vec![], None);
                Ok(hir::Array::Normal(hir::NormalArray::new(
                    arr.l_sqbr,
                    arr.r_sqbr,
                    Type::Failure,
                    elems,
                )))
            }
            other => Err(LowerErrors::from(LowerError::declare_error(
                self.cfg().input.clone(),
                line!() as usize,
                other.loc(),
                self.module.context.caused_by(),
            ))),
        }
    }

    fn fake_lower_tuple(&self, tup: ast::Tuple) -> LowerResult<hir::Tuple> {
        match tup {
            ast::Tuple::Normal(tup) => {
                let mut elems = Vec::new();
                let (elems_, _, _, paren) = tup.elems.deconstruct();
                for elem in elems_.into_iter() {
                    let elem = self.fake_lower_expr(elem.expr)?;
                    elems.push(hir::PosArg::new(elem));
                }
                let elems = hir::Args::pos_only(elems, paren);
                Ok(hir::Tuple::Normal(hir::NormalTuple::new(elems)))
            }
        }
    }

    fn fake_lower_signature(&self, sig: ast::Signature) -> LowerResult<hir::Signature> {
        match sig {
            ast::Signature::Var(var) => {
                let ident = var.ident().unwrap().clone();
                let ident = hir::Identifier::bare(ident);
                let t_spec = if let Some(ts) = var.t_spec {
                    let expr = self.fake_lower_expr(*ts.t_spec_as_expr.clone())?;
                    Some(hir::TypeSpecWithOp::new(ts, expr, Type::Failure))
                } else {
                    None
                };
                let sig = hir::VarSignature::new(ident, t_spec);
                Ok(hir::Signature::Var(sig))
            }
            ast::Signature::Subr(subr) => {
                let ident = hir::Identifier::bare(subr.ident);
                let params = self.fake_lower_params(subr.params)?;
                let sig = hir::SubrSignature::new(ident, subr.bounds, params, subr.return_t_spec);
                Ok(hir::Signature::Subr(sig))
            }
        }
    }

    fn fake_lower_def(&self, def: ast::Def) -> LowerResult<hir::Def> {
        let sig = self.fake_lower_signature(def.sig)?;
        let block = self.fake_lower_block(def.body.block)?;
        let body = hir::DefBody::new(def.body.op, block, def.body.id);
        Ok(hir::Def::new(sig, body))
    }

    fn fake_lower_record(&self, rec: ast::Record) -> LowerResult<hir::Record> {
        let rec = match rec {
            ast::Record::Normal(rec) => rec,
            ast::Record::Mixed(_mixed) => unreachable!(),
        };
        let mut elems = Vec::new();
        for elem in rec.attrs.into_iter() {
            let elem = self.fake_lower_def(elem)?;
            elems.push(elem);
        }
        let attrs = hir::RecordAttrs::new(elems);
        Ok(hir::Record::new(rec.l_brace, rec.r_brace, attrs))
    }

    fn fake_lower_set(&self, set: ast::Set) -> LowerResult<hir::Set> {
        match set {
            ast::Set::Normal(set) => {
                let mut elems = Vec::new();
                let (elems_, ..) = set.elems.deconstruct();
                for elem in elems_.into_iter() {
                    let elem = self.fake_lower_expr(elem.expr)?;
                    elems.push(hir::PosArg::new(elem));
                }
                let elems = hir::Args::pos_only(elems, None);
                Ok(hir::Set::Normal(hir::NormalSet::new(
                    set.l_brace,
                    set.r_brace,
                    Type::Failure,
                    elems,
                )))
            }
            ast::Set::WithLength(set) => {
                let len = self.fake_lower_expr(*set.len)?;
                let elem = self.fake_lower_expr(set.elem.expr)?;
                Ok(hir::Set::WithLength(hir::SetWithLength::new(
                    set.l_brace,
                    set.r_brace,
                    Type::Failure,
                    len,
                    elem,
                )))
            }
        }
    }

    fn fake_lower_dict(&self, dict: ast::Dict) -> LowerResult<hir::Dict> {
        match dict {
            ast::Dict::Normal(dict) => {
                let mut kvs = Vec::new();
                for elem in dict.kvs.into_iter() {
                    let key = self.fake_lower_expr(elem.key)?;
                    let val = self.fake_lower_expr(elem.value)?;
                    kvs.push(hir::KeyValue::new(key, val));
                }
                let tys = erg_common::dict::Dict::new();
                Ok(hir::Dict::Normal(hir::NormalDict::new(
                    dict.l_brace,
                    dict.r_brace,
                    tys,
                    kvs,
                )))
            }
            other => Err(LowerErrors::from(LowerError::declare_error(
                self.cfg().input.clone(),
                line!() as usize,
                other.loc(),
                self.module.context.caused_by(),
            ))),
        }
    }

    fn fake_lower_params(&self, params: ast::Params) -> LowerResult<hir::Params> {
        let (non_defaults_, var_params_, defaults_, parens) = params.deconstruct();
        let mut non_defaults = vec![];
        for non_default_ in non_defaults_.into_iter() {
            let non_default =
                hir::NonDefaultParamSignature::new(non_default_, VarInfo::default(), None);
            non_defaults.push(non_default);
        }
        let var_args = var_params_.map(|var_args| {
            Box::new(hir::NonDefaultParamSignature::new(
                *var_args,
                VarInfo::default(),
                None,
            ))
        });
        let mut defaults = vec![];
        for default_ in defaults_.into_iter() {
            let default_val = self.fake_lower_expr(default_.default_val)?;
            let sig = hir::NonDefaultParamSignature::new(default_.sig, VarInfo::default(), None);
            let default = hir::DefaultParamSignature::new(sig, default_val);
            defaults.push(default);
        }
        Ok(hir::Params::new(non_defaults, var_args, defaults, parens))
    }

    fn fake_lower_block(&self, block: ast::Block) -> LowerResult<hir::Block> {
        let mut chunks = vec![];
        for chunk in block.into_iter() {
            let chunk = self.fake_lower_expr(chunk)?;
            chunks.push(chunk);
        }
        Ok(hir::Block::new(chunks))
    }

    fn fake_lower_lambda(&self, lambda: ast::Lambda) -> LowerResult<hir::Lambda> {
        let params = self.fake_lower_params(lambda.sig.params)?;
        let body = self.fake_lower_block(lambda.body)?;
        Ok(hir::Lambda::new(
            lambda.id.0,
            params,
            lambda.op,
            body,
            Type::Failure,
        ))
    }

    fn fake_lower_dummy(&self, dummy: ast::Dummy) -> LowerResult<hir::Dummy> {
        let mut dummy_ = vec![];
        for elem in dummy.into_iter() {
            let elem = self.fake_lower_expr(elem)?;
            dummy_.push(elem);
        }
        Ok(hir::Dummy::new(dummy_))
    }

    fn fake_lower_type_asc(&self, tasc: ast::TypeAscription) -> LowerResult<hir::TypeAscription> {
        let expr = self.fake_lower_expr(*tasc.expr)?;
        let t_spec_as_expr = self.fake_lower_expr(*tasc.t_spec.t_spec_as_expr.clone())?;
        let spec_t = self
            .module
            .context
            .instantiate_typespec(&tasc.t_spec.t_spec)?;
        let spec = hir::TypeSpecWithOp::new(tasc.t_spec, t_spec_as_expr, spec_t);
        Ok(hir::TypeAscription::new(expr, spec))
    }

    pub(crate) fn fake_lower_expr(&self, expr: ast::Expr) -> LowerResult<hir::Expr> {
        match expr {
            ast::Expr::Literal(lit) => Ok(hir::Expr::Lit(self.lower_literal(lit)?)),
            ast::Expr::BinOp(binop) => Ok(hir::Expr::BinOp(self.fake_lower_binop(binop)?)),
            ast::Expr::UnaryOp(unop) => Ok(hir::Expr::UnaryOp(self.fake_lower_unaryop(unop)?)),
            ast::Expr::Array(arr) => Ok(hir::Expr::Array(self.fake_lower_array(arr)?)),
            ast::Expr::Tuple(tup) => Ok(hir::Expr::Tuple(self.fake_lower_tuple(tup)?)),
            ast::Expr::Record(rec) => Ok(hir::Expr::Record(self.fake_lower_record(rec)?)),
            ast::Expr::Set(set) => Ok(hir::Expr::Set(self.fake_lower_set(set)?)),
            ast::Expr::Dict(dict) => Ok(hir::Expr::Dict(self.fake_lower_dict(dict)?)),
            ast::Expr::Accessor(acc) => Ok(hir::Expr::Accessor(self.fake_lower_acc(acc)?)),
            ast::Expr::Call(call) => Ok(hir::Expr::Call(self.fake_lower_call(call)?)),
            ast::Expr::Lambda(lambda) => Ok(hir::Expr::Lambda(self.fake_lower_lambda(lambda)?)),
            ast::Expr::Dummy(dummy) => Ok(hir::Expr::Dummy(self.fake_lower_dummy(dummy)?)),
            ast::Expr::TypeAscription(tasc) => {
                Ok(hir::Expr::TypeAsc(self.fake_lower_type_asc(tasc)?))
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
        let kind = tasc.kind();
        match *tasc.expr {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) => {
                let py_name = Str::rc(ident.inspect().trim_end_matches('!'));
                let t = self
                    .module
                    .context
                    .instantiate_typespec(&tasc.t_spec.t_spec)?;
                t.lift();
                let t = self.module.context.generalize_t(t);
                match kind {
                    AscriptionKind::TypeOf | AscriptionKind::AsCast => {
                        self.declare_instance(&ident, &t, py_name.clone())?;
                    }
                    AscriptionKind::SubtypeOf => {
                        self.declare_subtype(&ident, &t)?;
                    }
                    _ => {
                        log!(err "supertype ascription is not supported yet");
                    }
                }
                let muty = Mutability::from(&ident.inspect()[..]);
                let vis = self.module.context.instantiate_vis_modifier(&ident.vis)?;
                let vi = VarInfo::new(
                    t,
                    muty,
                    Visibility::new(vis, self.module.context.name.clone()),
                    VarKind::Declared,
                    None,
                    None,
                    Some(py_name),
                    self.module.context.absolutize(ident.name.loc()),
                );
                let ident = hir::Identifier::new(ident, None, vi);
                let t_spec_expr = self.fake_lower_expr(*tasc.t_spec.t_spec_as_expr.clone())?;
                let t_spec = hir::TypeSpecWithOp::new(tasc.t_spec, t_spec_expr, Type::Failure);
                Ok(hir::Expr::Accessor(hir::Accessor::Ident(ident)).type_asc(t_spec))
            }
            ast::Expr::Accessor(ast::Accessor::Attr(attr)) => {
                let py_name = Str::rc(attr.ident.inspect().trim_end_matches('!'));
                let t = self
                    .module
                    .context
                    .instantiate_typespec(&tasc.t_spec.t_spec)?;
                let ctx = self
                    .module
                    .context
                    .get_mut_singular_ctx(attr.obj.as_ref(), &self.module.context.name.clone())?;
                ctx.assign_var_sig(
                    &ast::VarSignature::new(ast::VarPattern::Ident(attr.ident.clone()), None),
                    &t,
                    ast::DefId(0),
                    Some(py_name.clone()),
                )?;
                let obj = self.fake_lower_expr(*attr.obj)?;
                let muty = Mutability::from(&attr.ident.inspect()[..]);
                let vis = self
                    .module
                    .context
                    .instantiate_vis_modifier(&attr.ident.vis)?;
                let vi = VarInfo::new(
                    t,
                    muty,
                    Visibility::new(vis, self.module.context.name.clone()),
                    VarKind::Declared,
                    None,
                    None,
                    Some(py_name),
                    self.module.context.absolutize(attr.ident.name.loc()),
                );
                let ident = hir::Identifier::new(attr.ident, None, vi);
                let attr = obj.attr_expr(ident);
                let t_spec_expr = self.fake_lower_expr(*tasc.t_spec.t_spec_as_expr.clone())?;
                let t_spec = hir::TypeSpecWithOp::new(tasc.t_spec, t_spec_expr, Type::Failure);
                Ok(attr.type_asc(t_spec))
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
            let vis = self.module.context.instantiate_vis_modifier(&ident.vis)?;
            let vi = VarInfo::new(
                t.clone(),
                Mutability::Const,
                Visibility::new(vis, self.module.context.name.clone()),
                VarKind::Declared,
                None,
                None,
                Some(py_name.clone()),
                self.module.context.absolutize(ident.name.loc()),
            );
            let name = if PYTHON_MODE {
                let mut symbol = ident.name.clone().into_token();
                symbol.content = py_name.clone();
                VarName::new(symbol)
            } else {
                ident.name.clone()
            };
            self.module.context.decls.insert(name, vi);
        }
        let new_ident = if PYTHON_MODE {
            let mut symbol = ident.name.clone().into_token();
            symbol.content = py_name.clone();
            Identifier::new(ident.vis.clone(), VarName::new(symbol))
        } else {
            ident.clone()
        };
        let (t, ty_obj) = match t {
            Type::ClassType => {
                let t = mono(format!("{}{ident}", self.module.context.path()));
                let ty_obj = GenTypeObj::class(t.clone(), None, None);
                let t = v_enum(set! { ValueObj::builtin_class(t) });
                (t, Some(ty_obj))
            }
            Type::TraitType => {
                let t = mono(format!("{}{ident}", self.module.context.path()));
                let ty_obj =
                    GenTypeObj::trait_(t.clone(), TypeObj::builtin_type(Type::Uninited), None);
                let t = v_enum(set! { ValueObj::builtin_trait(t) });
                (t, Some(ty_obj))
            }
            _ => (t.clone(), None),
        };
        self.module.context.assign_var_sig(
            &ast::VarSignature::new(ast::VarPattern::Ident(ident.clone()), None),
            &t,
            ast::DefId(0),
            Some(py_name),
        )?;
        if let Some(gen) = ty_obj {
            self.module.context.register_gen_type(&new_ident, gen)?;
        }
        Ok(())
    }

    fn declare_subtype(&mut self, ident: &ast::Identifier, trait_: &Type) -> LowerResult<()> {
        if ident.is_raw() {
            return Ok(());
        }
        let name = if PYTHON_MODE {
            self.module
                .context
                .erg_to_py_names
                .get(ident.inspect())
                .map_or(Str::ever("?"), |s| s.clone())
        } else {
            ident.inspect().clone()
        };
        if let Some((_, ctx)) = self.module.context.rec_get_mut_type(&name) {
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
            ast::Expr::Literal(lit) if lit.is_doc_comment() => {
                Ok(hir::Expr::Lit(self.lower_literal(lit)?))
            }
            ast::Expr::Def(def) => Ok(hir::Expr::Def(self.declare_def(def)?)),
            ast::Expr::TypeAscription(tasc) => Ok(hir::Expr::TypeAsc(self.declare_ident(tasc)?)),
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
        let _ = self.module.context.preregister(ast.module.block());
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
