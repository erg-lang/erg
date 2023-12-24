use std::mem;

use erg_common::consts::PYTHON_MODE;
use erg_common::traits::{Locational, Runnable, Stream};
use erg_common::{enum_unwrap, fn_name, log, set, Str, Triple};

use erg_parser::ast::{self, AscriptionKind, DefId, Identifier, TypeAppArgsKind, VarName, AST};
use erg_parser::build_ast::ASTBuildable;
use erg_parser::desugar::Desugarer;

use crate::context::instantiate::TyVarCache;
use crate::context::{ClassDefType, Context, MethodContext, MethodPair, TraitImpl};
use crate::lower::GenericASTLowerer;
use crate::ty::constructors::{array_t, mono, mono_q_tp, poly, v_enum};
use crate::ty::free::{Constraint, HasLevel};
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{HasType, TyParam, Type, Visibility};

use crate::compile::AccessKind;
use crate::error::{LowerError, LowerErrors, LowerResult};
use crate::hir::HIR;
use crate::varinfo::{Mutability, VarInfo, VarKind};
use crate::{feature_error, hir};

impl<A: ASTBuildable> GenericASTLowerer<A> {
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
        let chunk = self.declare_chunk(body.block.remove(0), true)?;
        let py_name = match &chunk {
            hir::Expr::TypeAsc(tasc) => enum_unwrap!(tasc.expr.as_ref(), hir::Expr::Accessor)
                .local_name()
                .map(Str::rc),
            hir::Expr::Accessor(hir::Accessor::Ident(ident)) => ident.vi.py_name.clone(),
            _ => sig.escaped(),
        };
        let found_body_t = chunk.ref_t();
        let ident = match &sig.pat {
            ast::VarPattern::Ident(ident) => ident,
            ast::VarPattern::Discard(token) => {
                return Err(LowerErrors::from(LowerError::declare_error(
                    self.cfg().input.clone(),
                    line!() as usize,
                    token.loc(),
                    self.module.context.caused_by(),
                )));
            }
            _ => unreachable!(),
        };
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
                .assign_var_sig(&sig, found_body_t, id, Some(&chunk), None)?;
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

    fn fake_lower_literal(&self, lit: ast::Literal) -> LowerResult<hir::Literal> {
        let loc = lit.loc();
        let lit = hir::Literal::try_from(lit.token).map_err(|_| {
            LowerError::invalid_literal(
                self.cfg.input.clone(),
                line!() as usize,
                loc,
                self.module.context.caused_by(),
            )
        })?;
        Ok(lit)
    }

    fn fake_lower_acc(&self, acc: ast::Accessor) -> LowerResult<hir::Accessor> {
        // TypeApp is lowered in `fake_lower_expr`
        match acc {
            ast::Accessor::Ident(ident) => {
                // to resolve `py_name`
                let vi = self
                    .module
                    .context
                    .rec_get_var_info(&ident, AccessKind::Name, self.input(), &self.module.context)
                    .unwrap_or_default();
                let ident = hir::Identifier::new(ident, None, vi);
                let acc = hir::Accessor::Ident(ident);
                Ok(acc)
            }
            ast::Accessor::Attr(attr) => {
                let obj = self.fake_lower_expr(*attr.obj)?;
                let mut ident = hir::Identifier::bare(attr.ident);
                if let Ok(ctxs) = self
                    .module
                    .context
                    .get_singular_ctxs_by_hir_expr(&obj, &self.module.context)
                {
                    for ctx in ctxs {
                        if let Triple::Ok(vi) = ctx.rec_get_var_info(
                            &ident.raw,
                            AccessKind::UnboundAttr,
                            self.input(),
                            &self.module.context,
                        ) {
                            ident.vi = vi;
                            break;
                        }
                    }
                }
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
        let (pos_args_, var_args_, kw_args_, kw_var_, paren) = args.deconstruct();
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
        let kw_var = match kw_var_ {
            Some(kw_var) => {
                let kw_var = self.fake_lower_expr(kw_var.expr)?;
                Some(hir::PosArg::new(kw_var))
            }
            None => None,
        };
        let args = hir::Args::new(pos_args, var_args, kw_args, kw_var, paren);
        Ok(args)
    }

    fn fake_lower_call(&self, mut call: ast::Call) -> LowerResult<hir::Call> {
        let obj = self.fake_lower_expr(*call.obj)?;
        if call
            .attr_name
            .as_ref()
            .is_some_and(|attr| attr.inspect() == "__Tuple_getitem__")
        {
            call.attr_name
                .as_mut()
                .unwrap()
                .name
                .rename("__getitem__".into());
        }
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
                    elem,
                    Some(len),
                )))
            }
            ast::Array::Normal(arr) => {
                let mut elems = Vec::new();
                let (elems_, ..) = arr.elems.deconstruct();
                for elem in elems_.into_iter() {
                    let elem = self.fake_lower_expr(elem.expr)?;
                    elems.push(hir::PosArg::new(elem));
                }
                let elems = hir::Args::new(elems, None, vec![], None, None);
                let t = array_t(Type::Failure, TyParam::value(elems.len()));
                Ok(hir::Array::Normal(hir::NormalArray::new(
                    arr.l_sqbr, arr.r_sqbr, t, elems,
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
                let (elems_, _, _, _, paren) = tup.elems.deconstruct();
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
                let mut decorators = set! {};
                for decorator in subr.decorators.into_iter() {
                    let decorator = self.fake_lower_expr(decorator.0)?;
                    decorators.insert(decorator);
                }
                let ident = hir::Identifier::bare(subr.ident);
                let params = self.fake_lower_params(subr.params)?;
                let ret_t_spec = if let Some(ts) = subr.return_t_spec {
                    let spec_t = self.module.context.instantiate_typespec(&ts.t_spec)?;
                    let expr = self.fake_lower_expr(*ts.t_spec_as_expr.clone())?;
                    Some(hir::TypeSpecWithOp::new(ts, expr, spec_t))
                } else {
                    None
                };
                let sig = hir::SubrSignature::new(
                    decorators,
                    ident,
                    subr.bounds,
                    params,
                    ret_t_spec,
                    vec![],
                );
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
            ast::Record::Mixed(mixed) => Desugarer::desugar_shortened_record_inner(mixed),
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
            // TODO:
            ast::Set::Comprehension(set) => Ok(hir::Set::Normal(hir::NormalSet::new(
                set.l_brace,
                set.r_brace,
                Type::Failure,
                hir::Args::empty(),
            ))),
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
        let (non_defaults_, var_params_, defaults_, kw_var_, guards_, parens) =
            params.deconstruct();
        let mut non_defaults = vec![];
        for non_default_ in non_defaults_.into_iter() {
            let t_spec_as_expr = non_default_
                .t_spec
                .as_ref()
                .map(|t_spec| self.fake_lower_expr(*t_spec.t_spec_as_expr.clone()))
                .transpose()?;
            let non_default = hir::NonDefaultParamSignature::new(
                non_default_,
                VarInfo::default(),
                t_spec_as_expr,
            );
            non_defaults.push(non_default);
        }
        let var_params = if let Some(var_params) = var_params_ {
            let t_spec_as_expr = var_params
                .t_spec
                .as_ref()
                .map(|t_spec| self.fake_lower_expr(*t_spec.t_spec_as_expr.clone()))
                .transpose()?;
            Some(Box::new(hir::NonDefaultParamSignature::new(
                *var_params,
                VarInfo::default(),
                t_spec_as_expr,
            )))
        } else {
            None
        };
        let mut defaults = vec![];
        for default_ in defaults_.into_iter() {
            let t_spec_as_expr = default_
                .sig
                .t_spec
                .as_ref()
                .map(|t_spec| self.fake_lower_expr(*t_spec.t_spec_as_expr.clone()))
                .transpose()?;
            let default_val = self.fake_lower_expr(default_.default_val)?;
            let sig = hir::NonDefaultParamSignature::new(
                default_.sig,
                VarInfo::default(),
                t_spec_as_expr,
            );
            let default = hir::DefaultParamSignature::new(sig, default_val);
            defaults.push(default);
        }
        let kw_var = if let Some(kw_var) = kw_var_ {
            let t_spec_as_expr = kw_var
                .t_spec
                .as_ref()
                .map(|t_spec| self.fake_lower_expr(*t_spec.t_spec_as_expr.clone()))
                .transpose()?;
            Some(Box::new(hir::NonDefaultParamSignature::new(
                *kw_var,
                VarInfo::default(),
                t_spec_as_expr,
            )))
        } else {
            None
        };
        let mut guards = vec![];
        for guard in guards_.into_iter() {
            let guard = match guard {
                ast::GuardClause::Condition(cond) => {
                    hir::GuardClause::Condition(self.fake_lower_expr(cond)?)
                }
                ast::GuardClause::Bind(bind) => hir::GuardClause::Bind(self.fake_lower_def(bind)?),
            };
            guards.push(guard);
        }
        Ok(hir::Params::new(
            non_defaults,
            var_params,
            defaults,
            kw_var,
            guards,
            parens,
        ))
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
        let return_t_spec = lambda.sig.return_t_spec.map(|t_spec| t_spec.t_spec);
        let body = self.fake_lower_block(lambda.body)?;
        Ok(hir::Lambda::new(
            lambda.id.0,
            params,
            lambda.op,
            return_t_spec,
            vec![],
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

    fn fake_lower_compound(&self, compound: ast::Compound) -> LowerResult<hir::Block> {
        let mut chunks = vec![];
        for chunk in compound.into_iter() {
            let chunk = self.fake_lower_expr(chunk)?;
            chunks.push(chunk);
        }
        Ok(hir::Block::new(chunks))
    }

    pub(crate) fn fake_lower_expr(&self, expr: ast::Expr) -> LowerResult<hir::Expr> {
        match expr {
            ast::Expr::Literal(lit) => Ok(hir::Expr::Literal(self.fake_lower_literal(lit)?)),
            ast::Expr::BinOp(binop) => Ok(hir::Expr::BinOp(self.fake_lower_binop(binop)?)),
            ast::Expr::UnaryOp(unop) => Ok(hir::Expr::UnaryOp(self.fake_lower_unaryop(unop)?)),
            ast::Expr::Array(arr) => Ok(hir::Expr::Array(self.fake_lower_array(arr)?)),
            ast::Expr::Tuple(tup) => Ok(hir::Expr::Tuple(self.fake_lower_tuple(tup)?)),
            ast::Expr::Record(rec) => Ok(hir::Expr::Record(self.fake_lower_record(rec)?)),
            ast::Expr::Set(set) => Ok(hir::Expr::Set(self.fake_lower_set(set)?)),
            ast::Expr::Dict(dict) => Ok(hir::Expr::Dict(self.fake_lower_dict(dict)?)),
            ast::Expr::Accessor(ast::Accessor::TypeApp(tapp)) => self.fake_lower_expr(*tapp.obj),
            ast::Expr::Accessor(acc) => Ok(hir::Expr::Accessor(self.fake_lower_acc(acc)?)),
            ast::Expr::Call(call) => Ok(hir::Expr::Call(self.fake_lower_call(call)?)),
            ast::Expr::Lambda(lambda) => Ok(hir::Expr::Lambda(self.fake_lower_lambda(lambda)?)),
            ast::Expr::Compound(compound) => {
                Ok(hir::Expr::Compound(self.fake_lower_compound(compound)?))
            }
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

    fn get_tv_ctx(&self, ident: &ast::Identifier, args: &ast::Args) -> TyVarCache {
        let mut tv_ctx = TyVarCache::new(self.module.context.level, &self.module.context);
        if let Some(ctx) = self.module.context.get_type_ctx(ident.inspect()) {
            let arg_ts = ctx.params.iter().map(|(_, vi)| &vi.t);
            for ((tp, arg), arg_t) in ctx.typ.typarams().iter().zip(args.pos_args()).zip(arg_ts) {
                if let ast::Expr::Accessor(ast::Accessor::Ident(ident)) = &arg.expr {
                    if arg_t.is_type() {
                        if let Ok(tv) = self.module.context.convert_tp_into_type(tp.clone()) {
                            tv_ctx.push_or_init_tyvar(&ident.name, &tv, &self.module.context);
                            continue;
                        }
                    }
                    tv_ctx.push_or_init_typaram(&ident.name, tp, &self.module.context);
                }
            }
        }
        tv_ctx
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
                    self.module.context.kind.clone(),
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
                let mut tv_cache = if let Ok(call) = ast::Call::try_from(*attr.obj.clone()) {
                    let ast::Expr::Accessor(ast::Accessor::Ident(ident)) = *call.obj else {
                        return feature_error!(
                            LowerErrors,
                            LowerError,
                            &self.module.context,
                            call.obj.loc(),
                            "complex polymorphic type declaration"
                        );
                    };
                    self.get_tv_ctx(&ident, &call.args)
                } else {
                    TyVarCache::new(self.module.context.level, &self.module.context)
                };
                let t = self
                    .module
                    .context
                    .instantiate_typespec_with_tv_cache(&tasc.t_spec.t_spec, &mut tv_cache)?;
                let impl_trait = if let ast::Expr::Accessor(ast::Accessor::TypeApp(tapp)) =
                    attr.obj.as_ref()
                {
                    match &tapp.type_args.args {
                        TypeAppArgsKind::SubtypeOf(typ) => {
                            let trait_ = self
                                .module
                                .context
                                .instantiate_typespec_with_tv_cache(&typ.t_spec, &mut tv_cache)?;
                            Some(trait_)
                        }
                        TypeAppArgsKind::Args(args) => {
                            log!(err "{args}");
                            None
                        }
                    }
                } else {
                    None
                };
                let ctx = self.module.context.get_mut_singular_ctx_and_t(
                    attr.obj.as_ref(),
                    &self.module.context.name.clone(),
                )?;
                let class = ctx.typ.clone();
                let ctx = if let Some(impl_trait) = impl_trait {
                    match ctx
                        .methods_list
                        .iter_mut()
                        .find(|ctx| ctx.typ.is_impl_of(&impl_trait))
                    {
                        Some(impl_ctx) => impl_ctx,
                        None => {
                            let impl_ctx = Context::methods(
                                Some(impl_trait.clone()),
                                self.cfg.copy(),
                                ctx.shared.clone(),
                                0,
                                ctx.level,
                            );
                            ctx.super_traits.push(impl_trait.clone());
                            if let Some(mut impls) =
                                ctx.trait_impls().get_mut(&impl_trait.qual_name())
                            {
                                impls.insert(TraitImpl::new(class.clone(), impl_trait.clone()));
                            }
                            ctx.methods_list.push(MethodContext::new(
                                DefId(0),
                                ClassDefType::impl_trait(class.clone(), impl_trait),
                                impl_ctx,
                            ));
                            &mut ctx.methods_list.iter_mut().last().unwrap().ctx
                        }
                    }
                } else {
                    ctx
                };
                let vi = ctx.assign_var_sig(
                    &ast::VarSignature::new(ast::VarPattern::Ident(attr.ident.clone()), None),
                    &t,
                    ast::DefId(0),
                    None,
                    Some(py_name.clone()),
                )?;
                if let Some(types) = self
                    .module
                    .context
                    .method_to_classes
                    .get_mut(attr.ident.inspect())
                {
                    types.push(MethodPair::new(class, vi.clone()));
                } else {
                    self.module.context.method_to_classes.insert(
                        attr.ident.inspect().clone(),
                        vec![MethodPair::new(class, vi.clone())],
                    );
                }
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
                    self.module.context.kind.clone(),
                    Some(py_name),
                    self.module.context.absolutize(attr.ident.name.loc()),
                );
                let ident = hir::Identifier::new(attr.ident, None, vi);
                let attr = obj.attr_expr(ident);
                let t_spec_expr = self.fake_lower_expr(*tasc.t_spec.t_spec_as_expr.clone())?;
                let t_spec = hir::TypeSpecWithOp::new(tasc.t_spec, t_spec_expr, Type::Failure);
                Ok(attr.type_asc(t_spec))
            }
            ast::Expr::Call(call) => {
                let ast::Expr::Accessor(ast::Accessor::Ident(ident)) = *call.obj else {
                    return feature_error!(
                        LowerErrors,
                        LowerError,
                        &self.module.context,
                        call.obj.loc(),
                        "complex polymorphic type declaration"
                    );
                };
                let py_name = Str::rc(ident.inspect().trim_end_matches('!'));
                let mut tv_cache = self.get_tv_ctx(&ident, &call.args);
                let t = self
                    .module
                    .context
                    .instantiate_typespec_with_tv_cache(&tasc.t_spec.t_spec, &mut tv_cache)?;
                t.lift();
                let t = self.module.context.generalize_t(t);
                match kind {
                    AscriptionKind::TypeOf | AscriptionKind::AsCast => {
                        self.declare_instance(&ident, &t, py_name)?;
                    }
                    AscriptionKind::SubtypeOf => {
                        self.declare_subtype(&ident, &t)?;
                    }
                    _ => {
                        log!(err "supertype ascription is not supported yet");
                    }
                }
                let acc = self.fake_lower_acc(ast::Accessor::Ident(ident))?;
                let args = self.fake_lower_args(call.args)?;
                let t_spec_expr = self.fake_lower_expr(*tasc.t_spec.t_spec_as_expr.clone())?;
                let t_spec = hir::TypeSpecWithOp::new(tasc.t_spec, t_spec_expr, Type::Failure);
                Ok(hir::Expr::Accessor(acc).call_expr(args).type_asc(t_spec))
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
        // in case of:
        // ```
        // .Foo = 'foo': ClassType
        // .Foo.
        //     __call__: (T, U) -> .Foo
        // .foo: (T, U) -> .Foo # ignore this definition
        // ```
        let mut type_as_function = false;
        if PYTHON_MODE
            && self
                .module
                .context
                .registered_info(ident.inspect(), ident.is_const())
                .is_some_and(|(_, vi)| t.is_type() || vi.t.is_type())
        {
            let typ = self.module.context.get_type_ctx(ident.inspect());
            if typ.is_some_and(|ctx| ctx.has("__call__")) {
                type_as_function = true;
            }
        }
        if !type_as_function
            && self
                .module
                .context
                .registered_info(ident.inspect(), ident.is_const())
                .is_some_and(|(_, vi)| !vi.kind.is_builtin())
        {
            return Err(LowerErrors::from(LowerError::reassign_error(
                self.cfg().input.clone(),
                line!() as usize,
                ident.loc(),
                self.module.context.caused_by(),
                ident.inspect(),
            )));
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
                let ty_obj = GenTypeObj::class(t.clone(), None, None, true);
                let t = v_enum(set! { ValueObj::builtin_class(t) });
                (t, Some(ty_obj))
            }
            Type::TraitType => {
                let t = mono(format!("{}{ident}", self.module.context.path()));
                let ty_obj = GenTypeObj::trait_(
                    t.clone(),
                    TypeObj::builtin_type(Type::Uninited),
                    None,
                    true,
                );
                let t = v_enum(set! { ValueObj::builtin_trait(t) });
                (t, Some(ty_obj))
            }
            Type::Subr(subr) if subr.return_t.is_class_type() => {
                let params = subr
                    .non_default_params
                    .iter()
                    .map(|p| {
                        let c = Constraint::new_type_of(p.typ().clone());
                        mono_q_tp(p.name().unwrap_or(&Str::ever("_")), c)
                    })
                    .collect();
                let t = poly(format!("{}{ident}", self.module.context.path()), params);
                let ty_obj = GenTypeObj::class(t.clone(), None, None, true);
                let t = v_enum(set! { ValueObj::builtin_class(t) });
                (t, Some(ty_obj))
            }
            _ => (t.clone(), None),
        };
        self.module.context.assign_var_sig(
            &ast::VarSignature::new(ast::VarPattern::Ident(ident.clone()), None),
            &t,
            ast::DefId(0),
            None,
            Some(py_name),
        )?;
        if let Some(gen) = ty_obj {
            self.module.context.register_gen_type(&new_ident, gen)?;
        }
        Ok(())
    }

    fn declare_subtype(&mut self, ident: &ast::Identifier, sup: &Type) -> LowerResult<()> {
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
        if let Some(ctx) = self.module.context.rec_get_mut_type(&name) {
            let mut tmp = mem::take(ctx);
            let res = if self.module.context.is_class(sup) {
                tmp.register_base_class(&self.module.context, sup.clone())
            } else {
                tmp.register_marker_trait(&self.module.context, sup.clone())
            };
            res.map_err(|err| {
                let ctx = self.module.context.rec_get_mut_type(&name).unwrap();
                mem::swap(ctx, &mut tmp);
                err
            })?;
            let ctx = self.module.context.rec_get_mut_type(&name).unwrap();
            mem::swap(ctx, &mut tmp);
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

    fn declare_chunk(&mut self, expr: ast::Expr, allow_acc: bool) -> LowerResult<hir::Expr> {
        log!(info "entered {}", fn_name!());
        match expr {
            ast::Expr::Literal(lit) if lit.is_doc_comment() => {
                Ok(hir::Expr::Literal(self.lower_literal(lit, None)?))
            }
            ast::Expr::Accessor(acc) if allow_acc => {
                Ok(hir::Expr::Accessor(self.lower_acc(acc, None)?))
            }
            ast::Expr::Def(def) => Ok(hir::Expr::Def(self.declare_def(def)?)),
            ast::Expr::TypeAscription(tasc) => Ok(hir::Expr::TypeAsc(self.declare_ident(tasc)?)),
            ast::Expr::Call(call)
                if call
                    .additional_operation()
                    .map(|op| op.is_import())
                    .unwrap_or(false) =>
            {
                Ok(hir::Expr::Call(self.lower_call(call, None)))
            }
            ast::Expr::Compound(compound) => {
                let mut chunks = vec![];
                for chunk in compound.into_iter() {
                    let chunk = self.declare_chunk(chunk, true)?;
                    chunks.push(chunk);
                }
                Ok(hir::Expr::Compound(hir::Block::new(chunks)))
            }
            ast::Expr::Dummy(dummy) => {
                let mut dummy_ = vec![];
                for elem in dummy.into_iter() {
                    let elem = self.declare_chunk(elem, true)?;
                    dummy_.push(elem);
                }
                Ok(hir::Expr::Dummy(hir::Dummy::new(dummy_)))
            }
            ast::Expr::InlineModule(inline) => {
                let import = self.lower_inline_module(inline, None);
                Ok(hir::Expr::Call(import))
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
        let _ = self.module.context.register_const(ast.module.block());
        for chunk in ast.module.into_iter() {
            match self.declare_chunk(chunk, false) {
                Ok(chunk) => {
                    module.push(chunk);
                }
                Err(errs) => {
                    self.errs.extend(errs);
                }
            }
        }
        let hir = HIR::new(ast.name, module);
        self.lint(&hir, "declare");
        hir
    }
}
