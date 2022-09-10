//! implements `ASTLowerer`.
//!
//! ASTLowerer(ASTからHIRへの変換器)を実装
use erg_common::error::Location;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Visibility;
use erg_common::{enum_unwrap, fmt_option, fn_name, get_hash, log, switch_lang, Str};

use erg_parser::ast;
use erg_parser::ast::AST;

use erg_parser::token::{Token, TokenKind};
use erg_type::constructors::{array, array_mut, free_var, func, mono, poly, proc, quant};
use erg_type::free::Constraint;
use erg_type::typaram::TyParam;
use erg_type::value::{TypeObj, ValueObj};
use erg_type::{HasType, ParamTy, Type};

use crate::context::{Context, ContextKind, RegistrationMode};
use crate::error::{LowerError, LowerErrors, LowerResult, LowerWarnings};
use crate::hir;
use crate::hir::HIR;
use crate::varinfo::VarKind;
use Visibility::*;

/// HACK: Cannot be methodized this because a reference has been taken immediately before.
macro_rules! check_inheritable {
    ($self: ident, $type_obj: expr, $sup_class: expr, $sub_sig: expr) => {
        match $type_obj.require_or_sup.as_ref() {
            TypeObj::Generated(gen) => {
                if let Some(impls) = gen.impls.as_ref() {
                    if !impls.contains_intersec(&mono("InheritableType")) {
                        $self.errs.push(LowerError::inheritance_error(
                            line!() as usize,
                            $sup_class.to_string(),
                            $sup_class.loc(),
                            $sub_sig.ident().inspect().clone(),
                        ));
                    }
                } else {
                    $self.errs.push(LowerError::inheritance_error(
                        line!() as usize,
                        $sup_class.to_string(),
                        $sup_class.loc(),
                        $sub_sig.ident().inspect().clone(),
                    ));
                }
            }
            _ => {}
        }
    };
}

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
            ctx: Context::new_main_module(),
            errs: LowerErrors::empty(),
            warns: LowerWarnings::empty(),
        }
    }

    fn return_t_check(
        &self,
        loc: Location,
        name: &Str,
        expect: &Type,
        found: &Type,
    ) -> LowerResult<()> {
        self.ctx
            .sub_unify(found, expect, None, Some(loc), Some(name))
            .map_err(|_| {
                LowerError::type_mismatch_error(
                    line!() as usize,
                    loc,
                    self.ctx.caused_by(),
                    name,
                    expect,
                    found,
                    self.ctx.get_type_mismatch_hint(expect, found),
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
        log!(info "entered {}({array})", fn_name!());
        match array {
            ast::Array::Normal(arr) => Ok(hir::Array::Normal(self.lower_normal_array(arr)?)),
            ast::Array::WithLength(arr) => {
                Ok(hir::Array::WithLength(self.lower_array_with_length(arr)?))
            }
            other => todo!("{other}"),
        }
    }

    fn lower_normal_array(&mut self, array: ast::NormalArray) -> LowerResult<hir::NormalArray> {
        log!(info "entered {}({array})", fn_name!());
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
            free_var(self.ctx.level, Constraint::new_type_of(Type::Type))
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
        log!(info "entered {}({array})", fn_name!());
        let elem = self.lower_expr(array.elem.expr)?;
        let array_t = self.gen_array_with_length_type(&elem, &array.len);
        let len = self.lower_expr(*array.len)?;
        let hir_array = hir::ArrayWithLength::new(array.l_sqbr, array.r_sqbr, array_t, elem, len);
        Ok(hir_array)
    }

    fn gen_array_with_length_type(&self, elem: &hir::Expr, len: &ast::Expr) -> Type {
        let maybe_len = self.ctx.eval_const_expr(len, None);
        match maybe_len {
            Ok(v @ ValueObj::Nat(_)) => {
                if elem.ref_t().is_mut() {
                    poly(
                        "ArrayWithMutType!",
                        vec![TyParam::t(elem.t()), TyParam::Value(v)],
                    )
                } else {
                    array(elem.t(), TyParam::Value(v))
                }
            }
            Ok(v @ ValueObj::Mut(_)) if v.class() == mono("Nat!") => {
                if elem.ref_t().is_mut() {
                    poly(
                        "ArrayWithMutTypeAndLength!",
                        vec![TyParam::t(elem.t()), TyParam::Value(v)],
                    )
                } else {
                    array_mut(elem.t(), TyParam::Value(v))
                }
            }
            Ok(other) => todo!("{other} is not a Nat object"),
            // REVIEW: is it ok to ignore the error?
            Err(_e) => {
                if elem.ref_t().is_mut() {
                    poly(
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
        log!(info "entered {}({tuple})", fn_name!());
        match tuple {
            ast::Tuple::Normal(tup) => Ok(hir::Tuple::Normal(self.lower_normal_tuple(tup)?)),
        }
    }

    fn lower_normal_tuple(&mut self, tuple: ast::NormalTuple) -> LowerResult<hir::NormalTuple> {
        log!(info "entered {}({tuple})", fn_name!());
        let mut new_tuple = vec![];
        let (elems, _) = tuple.elems.into_iters();
        for elem in elems {
            let elem = self.lower_expr(elem.expr)?;
            new_tuple.push(elem);
        }
        Ok(hir::NormalTuple::new(hir::Args::from(new_tuple)))
    }

    fn lower_record(&mut self, record: ast::Record) -> LowerResult<hir::Record> {
        log!(info "entered {}({record})", fn_name!());
        match record {
            ast::Record::Normal(rec) => self.lower_normal_record(rec),
            ast::Record::Shortened(_rec) => unreachable!(), // should be desugared
        }
    }

    fn lower_normal_record(&mut self, record: ast::NormalRecord) -> LowerResult<hir::Record> {
        log!(info "entered {}({record})", fn_name!());
        let mut hir_record =
            hir::Record::new(record.l_brace, record.r_brace, hir::RecordAttrs::empty());
        self.ctx.grow("<record>", ContextKind::Dummy, Private)?;
        for attr in record.attrs.into_iter() {
            let attr = self.lower_def(attr).map_err(|e| {
                self.pop_append_errs();
                e
            })?;
            hir_record.push(attr);
        }
        self.pop_append_errs();
        Ok(hir_record)
    }

    fn lower_acc(&mut self, acc: ast::Accessor) -> LowerResult<hir::Accessor> {
        log!(info "entered {}({acc})", fn_name!());
        match acc {
            ast::Accessor::Ident(ident) => {
                let ident = self.lower_ident(ident)?;
                let acc = hir::Accessor::Ident(ident);
                Ok(acc)
            }
            ast::Accessor::Attr(attr) => {
                let obj = self.lower_expr(*attr.obj)?;
                let t = self.ctx.rec_get_attr_t(&obj, &attr.ident, &self.ctx.name)?;
                let ident = hir::Identifier::bare(attr.ident.dot, attr.ident.name);
                let acc = hir::Accessor::Attr(hir::Attribute::new(obj, ident, t));
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

    fn lower_ident(&self, ident: ast::Identifier) -> LowerResult<hir::Identifier> {
        // `match` is an untypable special form
        // `match`は型付け不可能な特殊形式
        let (t, __name__) = if ident.vis().is_private() && &ident.inspect()[..] == "match" {
            (Type::Failure, None)
        } else {
            (
                self.ctx.rec_get_var_t(&ident, &self.ctx.name)?,
                self.ctx.get_local_uniq_obj_name(ident.name.token()),
            )
        };
        let ident = hir::Identifier::new(ident.dot, ident.name, __name__, t);
        Ok(ident)
    }

    fn lower_bin(&mut self, bin: ast::BinOp) -> LowerResult<hir::BinOp> {
        log!(info "entered {}({bin})", fn_name!());
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
        log!(info "entered {}({unary})", fn_name!());
        let mut args = unary.args.into_iter();
        let arg = hir::PosArg::new(self.lower_expr(*args.next().unwrap())?);
        let args = [arg];
        let t = self.ctx.get_unaryop_t(&unary.op, &args, &self.ctx.name)?;
        let mut args = args.into_iter();
        let expr = args.next().unwrap().expr;
        Ok(hir::UnaryOp::new(unary.op, expr, t))
    }

    fn lower_call(&mut self, call: ast::Call) -> LowerResult<hir::Call> {
        log!(info "entered {}({}{}(...))", fn_name!(), call.obj, fmt_option!(pre ".", call.method_name));
        let (pos_args, kw_args, paren) = call.args.deconstruct();
        let mut hir_args = hir::Args::new(
            Vec::with_capacity(pos_args.len()),
            None,
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
        let sig_t = self.ctx.get_call_t(
            &obj,
            &call.method_name,
            &hir_args.pos_args,
            &hir_args.kw_args,
            &self.ctx.name,
        )?;
        let method_name = if let Some(method_name) = call.method_name {
            Some(hir::Identifier::new(
                method_name.dot,
                method_name.name,
                None,
                Type::Uninited,
            ))
        } else {
            None
        };
        Ok(hir::Call::new(obj, method_name, hir_args, sig_t))
    }

    fn lower_pack(&mut self, pack: ast::DataPack) -> LowerResult<hir::Call> {
        log!(info "entered {}({pack})", fn_name!());
        let class = self.lower_expr(*pack.class)?;
        let args = self.lower_record(pack.args)?;
        let args = vec![hir::PosArg::new(hir::Expr::Record(args))];
        let method_name = ast::Identifier::new(
            Some(Token::new(
                TokenKind::Dot,
                Str::ever("."),
                pack.connector.lineno,
                pack.connector.col_begin,
            )),
            ast::VarName::new(Token::new(
                TokenKind::Symbol,
                Str::ever("new"),
                pack.connector.lineno,
                pack.connector.col_begin,
            )),
        );
        let sig_t = self.ctx.get_call_t(
            &class,
            &Some(method_name.clone()),
            &args,
            &[],
            &self.ctx.name,
        )?;
        let args = hir::Args::new(args, None, vec![], None);
        let method_name = hir::Identifier::bare(method_name.dot, method_name.name);
        Ok(hir::Call::new(class, Some(method_name), args, sig_t))
    }

    /// TODO: varargs
    fn lower_lambda(&mut self, lambda: ast::Lambda) -> LowerResult<hir::Lambda> {
        log!(info "entered {}({lambda})", fn_name!());
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
        self.ctx.preregister(&lambda.body).map_err(|e| {
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
                ParamTy::pos(name.as_ref().map(|n| n.inspect().clone()), vi.t.clone())
            })
            .collect();
        let default_params = default_params
            .into_iter()
            .map(|(name, vi)| ParamTy::kw(name.as_ref().unwrap().inspect().clone(), vi.t.clone()))
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
            proc(non_default_params, None, default_params, body.t())
        } else {
            func(non_default_params, None, default_params, body.t())
        };
        let t = if bounds.is_empty() {
            t
        } else {
            quant(t, bounds)
        };
        Ok(hir::Lambda::new(id, lambda.sig.params, lambda.op, body, t))
    }

    fn lower_def(&mut self, def: ast::Def) -> LowerResult<hir::Def> {
        log!(info "entered {}({})", fn_name!(), def.sig);
        if def.body.block.len() >= 1 {
            let name = if let Some(name) = def.sig.name_as_str() {
                name
            } else {
                "<lambda>"
            };
            self.ctx.grow(name, ContextKind::Instant, def.sig.vis())?;
            let res = match def.sig {
                ast::Signature::Subr(sig) => self.lower_subr_def(sig, def.body),
                ast::Signature::Var(sig) => self.lower_var_def(sig, def.body),
            };
            // TODO: Context上の関数に型境界情報を追加
            self.pop_append_errs();
            return res;
        }
        match def.sig {
            ast::Signature::Subr(sig) => self.lower_subr_def(sig, def.body),
            ast::Signature::Var(sig) => self.lower_var_def(sig, def.body),
        }
    }

    fn lower_var_def(
        &mut self,
        sig: ast::VarSignature,
        body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        log!(info "entered {}({sig})", fn_name!());
        self.ctx.preregister(&body.block)?;
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
            // TODO: expect_body_t is smaller for constants
            // TODO: 定数の場合、expect_body_tのほうが小さくなってしまう
            if !sig.is_const() {
                if let Err(e) =
                    self.return_t_check(sig.loc(), ident.inspect(), &expect_body_t, found_body_t)
                {
                    self.errs.push(e);
                }
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
        let ident = hir::Identifier::bare(ident.dot.clone(), ident.name.clone());
        let sig = hir::VarSignature::new(ident, found_body_t.clone());
        let body = hir::DefBody::new(body.op, block, body.id);
        Ok(hir::Def::new(hir::Signature::Var(sig), body))
    }

    // NOTE: 呼ばれている間はinner scopeなので注意
    fn lower_subr_def(
        &mut self,
        sig: ast::SubrSignature,
        body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        log!(info "entered {}({sig})", fn_name!());
        let t = self
            .ctx
            .outer
            .as_ref()
            .unwrap()
            .get_current_scope_var(sig.ident.inspect())
            .unwrap()
            .t
            .clone();
        self.ctx.assign_params(&sig.params, None)?;
        self.ctx.preregister(&body.block)?;
        let block = self.lower_block(body.block)?;
        let found_body_t = block.ref_t();
        let expect_body_t = t.return_t().unwrap();
        if !sig.is_const() {
            if let Err(e) =
                self.return_t_check(sig.loc(), sig.ident.inspect(), &expect_body_t, found_body_t)
            {
                self.errs.push(e);
            }
        }
        let id = body.id;
        self.ctx
            .outer
            .as_mut()
            .unwrap()
            .assign_subr(&sig, id, found_body_t)?;
        let ident = hir::Identifier::bare(sig.ident.dot, sig.ident.name);
        let sig = hir::SubrSignature::new(ident, sig.params, t);
        let body = hir::DefBody::new(body.op, block, body.id);
        Ok(hir::Def::new(hir::Signature::Subr(sig), body))
    }

    fn lower_class_def(&mut self, class_def: ast::ClassDef) -> LowerResult<hir::ClassDef> {
        log!(info "entered {}({class_def})", fn_name!());
        let mut hir_def = self.lower_def(class_def.def)?;
        let mut private_methods = hir::RecordAttrs::empty();
        let mut public_methods = hir::RecordAttrs::empty();
        for mut methods in class_def.methods_list.into_iter() {
            let class = self
                .ctx
                .instantiate_typespec(&methods.class, RegistrationMode::Normal)?;
            self.ctx
                .grow(&class.name(), ContextKind::MethodDefs, Private)?;
            for def in methods.defs.iter_mut() {
                if methods.vis.is(TokenKind::Dot) {
                    def.sig.ident_mut().unwrap().dot = Some(Token::new(
                        TokenKind::Dot,
                        ".",
                        def.sig.ln_begin().unwrap(),
                        def.sig.col_begin().unwrap(),
                    ));
                }
                self.ctx.preregister_def(def)?;
            }
            for def in methods.defs.into_iter() {
                if methods.vis.is(TokenKind::Dot) {
                    let def = self.lower_def(def).map_err(|e| {
                        self.pop_append_errs();
                        e
                    })?;
                    public_methods.push(def);
                } else {
                    let def = self.lower_def(def).map_err(|e| {
                        self.pop_append_errs();
                        e
                    })?;
                    private_methods.push(def);
                }
            }
            match self.ctx.pop() {
                Ok(methods) => {
                    self.check_override(&class, &methods);
                    if let Some((_, class_root)) = self.ctx.rec_get_mut_nominal_type_ctx(&class) {
                        for (newly_defined_name, _vi) in methods.locals.iter() {
                            for (_, already_defined_methods) in class_root.methods_list.iter_mut() {
                                // TODO: 特殊化なら同じ名前でもOK
                                // TODO: 定義のメソッドもエラー表示
                                if let Some((_already_defined_name, already_defined_vi)) =
                                    already_defined_methods
                                        .get_local_kv(&newly_defined_name.inspect())
                                {
                                    if already_defined_vi.kind != VarKind::Auto {
                                        self.errs.push(LowerError::duplicate_definition_error(
                                            line!() as usize,
                                            newly_defined_name.loc(),
                                            methods.name.clone(),
                                            newly_defined_name.inspect(),
                                        ));
                                    } else {
                                        already_defined_methods
                                            .locals
                                            .remove(&newly_defined_name.inspect()[..]);
                                    }
                                }
                            }
                        }
                        class_root.methods_list.push((class, methods));
                    } else {
                        todo!()
                    }
                }
                Err(mut errs) => {
                    self.errs.append(&mut errs);
                }
            }
        }
        let (_, ctx) = self
            .ctx
            .rec_get_nominal_type_ctx(&mono(hir_def.sig.ident().inspect()))
            .unwrap();
        let type_obj = enum_unwrap!(self.ctx.rec_get_const_obj(hir_def.sig.ident().inspect()).unwrap(), ValueObj::Type:(TypeObj::Generated:(_)));
        let sup_type = enum_unwrap!(&hir_def.body.block.first().unwrap(), hir::Expr::Call)
            .args
            .get_left_or_key("Super")
            .unwrap();
        check_inheritable!(self, type_obj, sup_type, &hir_def.sig);
        // vi.t.non_default_params().unwrap()[0].typ().clone()
        let (__new__, need_to_gen_new) = if let (Some(dunder_new_vi), Some(new_vi)) = (
            ctx.get_current_scope_var("__new__"),
            ctx.get_current_scope_var("new"),
        ) {
            (dunder_new_vi.t.clone(), new_vi.kind == VarKind::Auto)
        } else {
            todo!()
        };
        let require_or_sup = self.get_require_or_sup(hir_def.body.block.remove(0));
        Ok(hir::ClassDef::new(
            type_obj.kind,
            hir_def.sig,
            require_or_sup,
            need_to_gen_new,
            __new__,
            private_methods,
            public_methods,
        ))
    }

    fn check_override(&mut self, class: &Type, ctx: &Context) {
        if let Some(sups) = self.ctx.rec_get_nominal_super_type_ctxs(class) {
            for (sup_t, sup) in sups.skip(1) {
                for (method_name, vi) in ctx.locals.iter() {
                    if let Some(_sup_vi) = sup.get_current_scope_var(&method_name.inspect()) {
                        // must `@Override`
                        if let Some(decos) = &vi.comptime_decos {
                            if decos.contains("Override") {
                                continue;
                            }
                        }
                        self.errs.push(LowerError::override_error(
                            line!() as usize,
                            method_name.inspect(),
                            method_name.loc(),
                            sup_t,
                            ctx.caused_by(),
                        ));
                    }
                }
            }
        }
    }

    fn get_require_or_sup(&self, expr: hir::Expr) -> hir::Expr {
        match expr {
            acc @ hir::Expr::Accessor(_) => acc,
            hir::Expr::Call(mut call) => match call.obj.show_acc().as_ref().map(|s| &s[..]) {
                Some("Class") => call.args.remove_left_or_key("Requirement").unwrap(),
                Some("Inherit") => call.args.remove_left_or_key("Super").unwrap(),
                Some("Inheritable") => {
                    self.get_require_or_sup(call.args.remove_left_or_key("Class").unwrap())
                }
                _ => todo!(),
            },
            other => todo!("{other}"),
        }
    }

    // Call.obj == Accessor cannot be type inferred by itself (it can only be inferred with arguments)
    // so turn off type checking (check=false)
    fn lower_expr(&mut self, expr: ast::Expr) -> LowerResult<hir::Expr> {
        log!(info "entered {}", fn_name!());
        match expr {
            ast::Expr::Lit(lit) => Ok(hir::Expr::Lit(hir::Literal::from(lit.token))),
            ast::Expr::Array(arr) => Ok(hir::Expr::Array(self.lower_array(arr)?)),
            ast::Expr::Tuple(tup) => Ok(hir::Expr::Tuple(self.lower_tuple(tup)?)),
            ast::Expr::Record(rec) => Ok(hir::Expr::Record(self.lower_record(rec)?)),
            ast::Expr::Accessor(acc) => Ok(hir::Expr::Accessor(self.lower_acc(acc)?)),
            ast::Expr::BinOp(bin) => Ok(hir::Expr::BinOp(self.lower_bin(bin)?)),
            ast::Expr::UnaryOp(unary) => Ok(hir::Expr::UnaryOp(self.lower_unary(unary)?)),
            ast::Expr::Call(call) => Ok(hir::Expr::Call(self.lower_call(call)?)),
            ast::Expr::DataPack(pack) => Ok(hir::Expr::Call(self.lower_pack(pack)?)),
            ast::Expr::Lambda(lambda) => Ok(hir::Expr::Lambda(self.lower_lambda(lambda)?)),
            ast::Expr::Def(def) => Ok(hir::Expr::Def(self.lower_def(def)?)),
            ast::Expr::ClassDef(defs) => Ok(hir::Expr::ClassDef(self.lower_class_def(defs)?)),
            other => todo!("{other}"),
        }
    }

    fn lower_block(&mut self, ast_block: ast::Block) -> LowerResult<hir::Block> {
        log!(info "entered {}", fn_name!());
        let mut hir_block = Vec::with_capacity(ast_block.len());
        for expr in ast_block.into_iter() {
            let expr = self.lower_expr(expr)?;
            hir_block.push(expr);
        }
        Ok(hir::Block::new(hir_block))
    }

    pub fn lower(&mut self, ast: AST, mode: &str) -> Result<(HIR, LowerWarnings), LowerErrors> {
        log!(info "the AST lowering process has started.");
        log!(info "the type-checking process has started.");
        let mut module = hir::Module::with_capacity(ast.module.len());
        self.ctx.preregister(ast.module.block())?;
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
        log!(info "HIR (not derefed):\n{hir}");
        log!(
            c GREEN,
            "the type-checking process has completed, found errors: {}",
            self.errs.len()
        );
        let hir = self.ctx.deref_toplevel(hir)?;
        if self.errs.is_empty() {
            log!(info "HIR:\n{hir}");
            log!(info "the AST lowering process has completed.");
            Ok((hir, LowerWarnings::from(self.warns.take_all())))
        } else {
            log!(err "the AST lowering process has failed.");
            Err(LowerErrors::from(self.errs.take_all()))
        }
    }
}

impl Default for ASTLowerer {
    fn default() -> Self {
        Self::new()
    }
}
