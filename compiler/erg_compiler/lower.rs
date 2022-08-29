//! implements `ASTLowerer`.
//!
//! ASTLowerer(ASTからHIRへの変換器)を実装
use erg_common::color::{GREEN, RED, RESET};
use erg_common::error::Location;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::{enum_unwrap, get_hash};
use erg_common::{fn_name, log, switch_lang};

use erg_parser::ast;
use erg_parser::ast::AST;

use erg_type::constructors::{array, array_mut, class, free_var, func, poly_class, proc, quant};
use erg_type::free::Constraint;
use erg_type::typaram::TyParam;
use erg_type::value::ValueObj;
use erg_type::{HasType, ParamTy, Type};

use crate::context::{Context, ContextKind, RegistrationMode};
use crate::error::{LowerError, LowerErrors, LowerResult, LowerWarnings};
use crate::hir;
use crate::hir::HIR;
use Visibility::*;

/// Singleton that checks types of an AST, and convert (lower) it into a HIR
#[derive(Debug)]
pub struct ASTLowerer {
    pub(crate) ctx: Context,
    errs: LowerErrors,
    warns: LowerWarnings,
}

impl ASTLowerer {
    pub fn new() -> Self {
        Self {
            ctx: Context::new_root_module(),
            errs: LowerErrors::empty(),
            warns: LowerWarnings::empty(),
        }
    }

    fn return_t_check(
        &self,
        loc: Location,
        name: &str,
        expect: &Type,
        found: &Type,
    ) -> LowerResult<()> {
        self.ctx
            .sub_unify(found, expect, None, Some(loc))
            .map_err(|_| {
                LowerError::type_mismatch_error(
                    line!() as usize,
                    loc,
                    self.ctx.caused_by(),
                    name,
                    expect,
                    found,
                )
            })
    }

    fn use_check(&self, expr: hir::Expr, mode: &str) -> LowerResult<hir::Expr> {
        if mode != "eval" && !expr.ref_t().is_nonelike() {
            Err(LowerError::syntax_error(
                0,
                expr.loc(),
                self.ctx.name.clone(),
                switch_lang!(
                    "japanese" => "式の評価結果が使われていません",
                    "simplified_chinese" => "表达式评估结果未使用",
                    "traditional_chinese" => "表達式評估結果未使用",
                    "english" => "the evaluation result of the expression is not used",
                ),
                Some(
                    switch_lang!(
                        "japanese" => "値を使わない場合は、discard関数を使用してください",
                        "simplified_chinese" => "如果您不想使用该值，请使用discard函数",
                        "traditional_chinese" => "如果您不想使用該值，請使用discard函數",
                        "english" => "if you don't use the value, use discard function",
                    )
                    .into(),
                ),
            ))
        } else {
            Ok(expr)
        }
    }

    fn pop_append_errs(&mut self) {
        if let Err(mut errs) = self.ctx.pop() {
            self.errs.append(&mut errs);
        }
    }

    fn lower_array(&mut self, array: ast::Array) -> LowerResult<hir::Array> {
        log!("[DEBUG] entered {}({array})", fn_name!());
        match array {
            ast::Array::Normal(arr) => Ok(hir::Array::Normal(self.lower_normal_array(arr)?)),
            ast::Array::WithLength(arr) => {
                Ok(hir::Array::WithLength(self.lower_array_with_length(arr)?))
            }
            other => todo!("{other}"),
        }
    }

    fn lower_normal_array(&mut self, array: ast::NormalArray) -> LowerResult<hir::NormalArray> {
        log!("[DEBUG] entered {}({array})", fn_name!());
        let mut new_array = vec![];
        let (elems, _) = array.elems.into_iters();
        let mut union = Type::Never;
        for elem in elems {
            let elem = self.lower_expr(elem.expr)?;
            union = self.ctx.rec_union(&union, elem.ref_t());
            if matches!(union, Type::Or(_, _)) {
                return Err(LowerError::syntax_error(
                    line!() as usize,
                    elem.loc(),
                    self.ctx.name.clone(),
                    switch_lang!(
                        "japanese" => "配列の要素は全て同じ型である必要があります",
                        "simplified_chinese" => "数组元素必须全部是相同类型",
                        "traditional_chinese" => "數組元素必須全部是相同類型",
                        "english" => "all elements of an array must be of the same type",
                    ),
                    Some(
                        switch_lang!(
                            "japanese" => "Int or Strなど明示的に型を指定してください",
                            "simplified_chinese" => "明确指定类型，例如：Int or Str",
                            "traditional_chinese" => "明確指定類型，例如：Int or Str",
                            "english" => "please specify the type explicitly, e.g. Int or Str",
                        )
                        .into(),
                    ),
                ));
            }
            new_array.push(elem);
        }
        let elem_t = if union == Type::Never {
            free_var(self.ctx.level, Constraint::type_of(Type::Type))
        } else {
            union
        };
        Ok(hir::NormalArray::new(
            array.l_sqbr,
            array.r_sqbr,
            elem_t,
            hir::Args::from(new_array),
        ))
    }

    fn lower_array_with_length(
        &mut self,
        array: ast::ArrayWithLength,
    ) -> LowerResult<hir::ArrayWithLength> {
        log!("[DEBUG] entered {}({array})", fn_name!());
        let elem = self.lower_expr(array.elem.expr)?;
        let array_t = self.gen_array_with_length_type(&elem, &array.len);
        let len = self.lower_expr(*array.len)?;
        let hir_array = hir::ArrayWithLength::new(array.l_sqbr, array.r_sqbr, array_t, elem, len);
        Ok(hir_array)
    }

    fn gen_array_with_length_type(&self, elem: &hir::Expr, len: &ast::Expr) -> Type {
        let maybe_len = self.ctx.eval.eval_const_expr(len, &self.ctx);
        match maybe_len {
            Some(v @ ValueObj::Nat(_)) => {
                if elem.ref_t().is_mut() {
                    poly_class(
                        "ArrayWithMutType!",
                        vec![TyParam::t(elem.t()), TyParam::Value(v)],
                    )
                } else {
                    array(elem.t(), TyParam::Value(v))
                }
            }
            Some(v @ ValueObj::Mut(_)) if v.class() == class("Nat!") => {
                if elem.ref_t().is_mut() {
                    poly_class(
                        "ArrayWithMutTypeAndLength!",
                        vec![TyParam::t(elem.t()), TyParam::Value(v)],
                    )
                } else {
                    array_mut(elem.t(), TyParam::Value(v))
                }
            }
            Some(other) => todo!("{other} is not a Nat object"),
            // TODO: [T; !_]
            None => {
                if elem.ref_t().is_mut() {
                    poly_class(
                        "ArrayWithMutType!",
                        vec![TyParam::t(elem.t()), TyParam::erased(Type::Nat)],
                    )
                } else {
                    array(elem.t(), TyParam::erased(Type::Nat))
                }
            }
        }
    }

    fn lower_tuple(&mut self, tuple: ast::Tuple) -> LowerResult<hir::Tuple> {
        log!("[DEBUG] entered {}({tuple})", fn_name!());
        match tuple {
            ast::Tuple::Normal(tup) => Ok(hir::Tuple::Normal(self.lower_normal_tuple(tup)?)),
        }
    }

    fn lower_normal_tuple(&mut self, tuple: ast::NormalTuple) -> LowerResult<hir::NormalTuple> {
        log!("[DEBUG] entered {}({tuple})", fn_name!());
        let mut new_tuple = vec![];
        let (elems, _) = tuple.elems.into_iters();
        for elem in elems {
            let elem = self.lower_expr(elem.expr)?;
            new_tuple.push(elem);
        }
        Ok(hir::NormalTuple::new(hir::Args::from(new_tuple)))
    }

    fn lower_record(&mut self, record: ast::Record) -> LowerResult<hir::Record> {
        log!("[DEBUG] entered {}({record})", fn_name!());
        let mut hir_record =
            hir::Record::new(record.l_brace, record.r_brace, hir::RecordAttrs::new());
        self.ctx.grow("<record>", ContextKind::Dummy, Private)?;
        for attr in record.attrs.into_iter() {
            let attr = self.lower_def(attr)?;
            hir_record.push(attr);
        }
        self.pop_append_errs();
        Ok(hir_record)
    }

    fn lower_acc(&mut self, acc: ast::Accessor) -> LowerResult<hir::Accessor> {
        log!("[DEBUG] entered {}({acc})", fn_name!());
        match acc {
            ast::Accessor::Local(local) => {
                // `match` is an untypable special form
                // `match`は型付け不可能な特殊形式
                let (t, __name__) = if &local.inspect()[..] == "match" {
                    (Type::Failure, None)
                } else {
                    (
                        self.ctx
                            .rec_get_var_t(&local.symbol, Private, &self.ctx.name)?,
                        self.ctx.get_local_uniq_obj_name(&local.symbol),
                    )
                };
                let acc = hir::Accessor::Local(hir::Local::new(local.symbol, __name__, t));
                Ok(acc)
            }
            ast::Accessor::Public(public) => {
                let (t, __name__) = (
                    self.ctx
                        .rec_get_var_t(&public.symbol, Public, &self.ctx.name)?,
                    self.ctx.get_local_uniq_obj_name(&public.symbol),
                );
                let public = hir::Public::new(public.dot, public.symbol, __name__, t);
                let acc = hir::Accessor::Public(public);
                Ok(acc)
            }
            ast::Accessor::Attr(attr) => {
                let obj = self.lower_expr(*attr.obj)?;
                let t = self
                    .ctx
                    .rec_get_attr_t(&obj, &attr.name.symbol, &self.ctx.name)?;
                let acc = hir::Accessor::Attr(hir::Attribute::new(obj, attr.name.symbol, t));
                Ok(acc)
            }
            ast::Accessor::TupleAttr(t_attr) => {
                let obj = self.lower_expr(*t_attr.obj)?;
                let index = hir::Literal::from(t_attr.index.token);
                let n = enum_unwrap!(index.value, ValueObj::Nat);
                let t = enum_unwrap!(
                    obj.ref_t().typarams().get(n as usize).unwrap().clone(),
                    TyParam::Type
                );
                let acc = hir::Accessor::TupleAttr(hir::TupleAttribute::new(obj, index, *t));
                Ok(acc)
            }
            ast::Accessor::Subscr(subscr) => {
                let obj = self.lower_expr(*subscr.obj)?;
                let index = self.lower_expr(*subscr.index)?;
                // FIXME: 配列とは限らない！
                let t = enum_unwrap!(
                    obj.ref_t().typarams().get(0).unwrap().clone(),
                    TyParam::Type
                );
                let acc = hir::Accessor::Subscr(hir::Subscript::new(obj, index, *t));
                Ok(acc)
            }
        }
    }

    fn lower_bin(&mut self, bin: ast::BinOp) -> LowerResult<hir::BinOp> {
        log!("[DEBUG] entered {}({bin})", fn_name!());
        let mut args = bin.args.into_iter();
        let lhs = hir::PosArg::new(self.lower_expr(*args.next().unwrap())?);
        let rhs = hir::PosArg::new(self.lower_expr(*args.next().unwrap())?);
        let args = [lhs, rhs];
        let t = self.ctx.get_binop_t(&bin.op, &args, &self.ctx.name)?;
        let mut args = args.into_iter();
        let lhs = args.next().unwrap().expr;
        let rhs = args.next().unwrap().expr;
        Ok(hir::BinOp::new(bin.op, lhs, rhs, t))
    }

    fn lower_unary(&mut self, unary: ast::UnaryOp) -> LowerResult<hir::UnaryOp> {
        log!("[DEBUG] entered {}({unary})", fn_name!());
        let mut args = unary.args.into_iter();
        let arg = hir::PosArg::new(self.lower_expr(*args.next().unwrap())?);
        let args = [arg];
        let t = self.ctx.get_unaryop_t(&unary.op, &args, &self.ctx.name)?;
        let mut args = args.into_iter();
        let expr = args.next().unwrap().expr;
        Ok(hir::UnaryOp::new(unary.op, expr, t))
    }

    fn lower_call(&mut self, call: ast::Call) -> LowerResult<hir::Call> {
        log!("[DEBUG] entered {}({}(...))", fn_name!(), call.obj);
        let (pos_args, kw_args, paren) = call.args.deconstruct();
        let mut hir_args = hir::Args::new(
            Vec::with_capacity(pos_args.len()),
            Vec::with_capacity(kw_args.len()),
            paren,
        );
        for arg in pos_args.into_iter() {
            hir_args.push_pos(hir::PosArg::new(self.lower_expr(arg.expr)?));
        }
        for arg in kw_args.into_iter() {
            hir_args.push_kw(hir::KwArg::new(arg.keyword, self.lower_expr(arg.expr)?));
        }
        let obj = self.lower_expr(*call.obj)?;
        let t = self.ctx.get_call_t(
            &obj,
            &call.method_name,
            &hir_args.pos_args,
            &hir_args.kw_args,
            &self.ctx.name,
        )?;
        Ok(hir::Call::new(obj, call.method_name, hir_args, t))
    }

    fn lower_lambda(&mut self, lambda: ast::Lambda) -> LowerResult<hir::Lambda> {
        log!("[DEBUG] entered {}({lambda})", fn_name!());
        let is_procedural = lambda.is_procedural();
        let id = get_hash(&lambda.sig);
        let name = format!("<lambda_{id}>");
        let kind = if is_procedural {
            ContextKind::Proc
        } else {
            ContextKind::Func
        };
        self.ctx.grow(&name, kind, Private)?;
        self.ctx
            .assign_params(&lambda.sig.params, None)
            .map_err(|e| {
                self.pop_append_errs();
                e
            })?;
        self.ctx
            .preregister(lambda.body.ref_payload())
            .map_err(|e| {
                self.pop_append_errs();
                e
            })?;
        let body = self.lower_block(lambda.body).map_err(|e| {
            self.pop_append_errs();
            e
        })?;
        let (non_default_params, default_params): (Vec<_>, Vec<_>) = self
            .ctx
            .params
            .iter()
            .partition(|(_, v)| !v.kind.has_default());
        let non_default_params = non_default_params
            .into_iter()
            .map(|(name, vi)| {
                ParamTy::new(name.as_ref().map(|n| n.inspect().clone()), vi.t.clone())
            })
            .collect();
        let default_params = default_params
            .into_iter()
            .map(|(name, vi)| {
                ParamTy::new(name.as_ref().map(|n| n.inspect().clone()), vi.t.clone())
            })
            .collect();
        let bounds = self
            .ctx
            .instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal)
            .map_err(|e| {
                self.pop_append_errs();
                e
            })?;
        self.pop_append_errs();
        let t = if is_procedural {
            proc(non_default_params, default_params, body.t())
        } else {
            func(non_default_params, default_params, body.t())
        };
        let t = if bounds.is_empty() {
            t
        } else {
            quant(t, bounds)
        };
        Ok(hir::Lambda::new(id, lambda.sig.params, lambda.op, body, t))
    }

    fn lower_def(&mut self, def: ast::Def) -> LowerResult<hir::Def> {
        log!("[DEBUG] entered {}({})", fn_name!(), def.sig);
        if let Some(name) = def.sig.name_as_str() {
            self.ctx.grow(name, ContextKind::Instant, Private)?;
            let res = match def.sig {
                ast::Signature::Subr(sig) => self.lower_subr_def(sig, def.body),
                ast::Signature::Var(sig) => self.lower_var_def(sig, def.body),
            };
            // TODO: Context上の関数に型境界情報を追加
            self.pop_append_errs();
            res
        } else {
            match def.sig {
                ast::Signature::Subr(sig) => self.lower_subr_def(sig, def.body),
                ast::Signature::Var(sig) => self.lower_var_def(sig, def.body),
            }
        }
    }

    fn lower_var_def(
        &mut self,
        sig: ast::VarSignature,
        body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        log!("[DEBUG] entered {}({sig})", fn_name!());
        self.ctx.preregister(body.block.ref_payload())?;
        let block = self.lower_block(body.block)?;
        let found_body_t = block.ref_t();
        let opt_expect_body_t = self
            .ctx
            .outer
            .as_ref()
            .unwrap()
            .get_current_scope_var(sig.inspect().unwrap())
            .map(|vi| vi.t.clone());
        let ident = match &sig.pat {
            ast::VarPattern::Ident(ident) => ident,
            _ => unreachable!(),
        };
        if let Some(expect_body_t) = opt_expect_body_t {
            if let Err(e) =
                self.return_t_check(sig.loc(), ident.inspect(), &expect_body_t, found_body_t)
            {
                self.errs.push(e);
            }
        }
        let id = body.id;
        // TODO: cover all VarPatterns
        self.ctx
            .outer
            .as_mut()
            .unwrap()
            .assign_var_sig(&sig, found_body_t, id)?;
        match block.first().unwrap() {
            hir::Expr::Call(call) => {
                if call.is_import_call() {
                    self.ctx
                        .outer
                        .as_mut()
                        .unwrap()
                        .import_mod(&ident.name, &call.args.pos_args.first().unwrap().expr)?;
                }
            }
            _other => {}
        }
        let sig = hir::VarSignature::new(ident.clone(), found_body_t.clone());
        let body = hir::DefBody::new(body.op, block, body.id);
        Ok(hir::Def::new(hir::Signature::Var(sig), body))
    }

    // NOTE: 呼ばれている間はinner scopeなので注意
    fn lower_subr_def(
        &mut self,
        sig: ast::SubrSignature,
        body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        log!("[DEBUG] entered {}({sig})", fn_name!());
        let t = self
            .ctx
            .outer
            .as_ref()
            .unwrap()
            .get_current_scope_var(sig.ident.inspect())
            .unwrap_or_else(|| {
                log!("{}\n", sig.ident.inspect());
                log!("{}\n", self.ctx.outer.as_ref().unwrap());
                panic!()
            }) // FIXME: or instantiate
            .t
            .clone();
        self.ctx.assign_params(&sig.params, None)?;
        self.ctx.preregister(body.block.ref_payload())?;
        let block = self.lower_block(body.block)?;
        let found_body_t = block.ref_t();
        let expect_body_t = t.return_t().unwrap();
        if let Err(e) =
            self.return_t_check(sig.loc(), sig.ident.inspect(), &expect_body_t, found_body_t)
        {
            self.errs.push(e);
        }
        let id = body.id;
        self.ctx
            .outer
            .as_mut()
            .unwrap()
            .assign_subr(&sig, id, found_body_t)?;
        let sig = hir::SubrSignature::new(sig.ident, sig.params, t);
        let body = hir::DefBody::new(body.op, block, body.id);
        Ok(hir::Def::new(hir::Signature::Subr(sig), body))
    }

    // Call.obj == Accessor cannot be type inferred by itself (it can only be inferred with arguments)
    // so turn off type checking (check=false)
    fn lower_expr(&mut self, expr: ast::Expr) -> LowerResult<hir::Expr> {
        log!("[DEBUG] entered {}", fn_name!());
        match expr {
            ast::Expr::Lit(lit) => Ok(hir::Expr::Lit(hir::Literal::from(lit.token))),
            ast::Expr::Array(arr) => Ok(hir::Expr::Array(self.lower_array(arr)?)),
            ast::Expr::Tuple(tup) => Ok(hir::Expr::Tuple(self.lower_tuple(tup)?)),
            ast::Expr::Record(rec) => Ok(hir::Expr::Record(self.lower_record(rec)?)),
            ast::Expr::Accessor(acc) => Ok(hir::Expr::Accessor(self.lower_acc(acc)?)),
            ast::Expr::BinOp(bin) => Ok(hir::Expr::BinOp(self.lower_bin(bin)?)),
            ast::Expr::UnaryOp(unary) => Ok(hir::Expr::UnaryOp(self.lower_unary(unary)?)),
            ast::Expr::Call(call) => Ok(hir::Expr::Call(self.lower_call(call)?)),
            ast::Expr::Lambda(lambda) => Ok(hir::Expr::Lambda(self.lower_lambda(lambda)?)),
            ast::Expr::Def(def) => Ok(hir::Expr::Def(self.lower_def(def)?)),
            other => todo!("{other}"),
        }
    }

    fn lower_block(&mut self, ast_block: ast::Block) -> LowerResult<hir::Block> {
        log!("[DEBUG] entered {}", fn_name!());
        let mut hir_block = Vec::with_capacity(ast_block.len());
        for expr in ast_block.into_iter() {
            let expr = self.lower_expr(expr)?;
            hir_block.push(expr);
        }
        Ok(hir::Block::new(hir_block))
    }

    pub fn lower(&mut self, ast: AST, mode: &str) -> Result<(HIR, LowerWarnings), LowerErrors> {
        log!("{GREEN}[DEBUG] the AST lowering process has started.");
        log!("{GREEN}[DEBUG] the type-checking process has started.");
        let mut module = hir::Module::with_capacity(ast.module.len());
        self.ctx.preregister(ast.module.ref_payload())?;
        for expr in ast.module.into_iter() {
            match self.lower_expr(expr).and_then(|e| self.use_check(e, mode)) {
                Ok(expr) => {
                    module.push(expr);
                }
                Err(e) => {
                    self.errs.push(e);
                }
            }
        }
        let hir = HIR::new(ast.name, module);
        log!("HIR (not derefed):\n{hir}");
        log!(
            "[DEBUG] the type-checking process has completed, found errors: {}",
            self.errs.len()
        );
        let hir = self.ctx.deref_toplevel(hir)?;
        if self.errs.is_empty() {
            log!("HIR:\n{hir}");
            log!("[DEBUG] the AST lowering process has completed.{RESET}");
            Ok((hir, LowerWarnings::from(self.warns.take_all())))
        } else {
            log!("{RED}[DEBUG] the AST lowering process has failed.{RESET}");
            Err(LowerErrors::from(self.errs.take_all()))
        }
    }
}

impl Default for ASTLowerer {
    fn default() -> Self {
        Self::new()
    }
}
