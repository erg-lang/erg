//! implements `ASTLowerer`.
//!
//! ASTLowerer(ASTからHIRへの変換器)を実装

use erg_common::config::ErgConfig;
use erg_common::dict;
use erg_common::error::{Location, MultiErrorDisplay};
use erg_common::set;
use erg_common::set::Set;
use erg_common::traits::{Locational, NoTypeDisplay, Runnable, Stream};
use erg_common::vis::Visibility;
use erg_common::{enum_unwrap, fmt_option, fn_name, get_hash, log, switch_lang, Str};

use erg_parser::ast;
use erg_parser::ast::{OperationKind, AST};
use erg_parser::build_ast::ASTBuilder;
use erg_parser::token::{Token, TokenKind};
use erg_parser::Parser;

use crate::artifact::{CompleteArtifact, IncompleteArtifact};
use crate::context::instantiate::TyVarCache;
use crate::ty::constructors::{
    array_mut, array_t, free_var, func, mono, poly, proc, set_mut, set_t, ty_tp,
};
use crate::ty::free::Constraint;
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{HasType, ParamTy, Type};

use crate::context::{
    ClassDefType, Context, ContextKind, ContextProvider, RegistrationMode, TypeRelationInstance,
};
use crate::error::{
    CompileError, CompileErrors, LowerError, LowerErrors, LowerResult, LowerWarning, LowerWarnings,
    SingleLowerResult,
};
use crate::hir;
use crate::hir::HIR;
use crate::mod_cache::SharedModuleCache;
use crate::reorder::Reorderer;
use crate::varinfo::{VarInfo, VarKind};
use crate::AccessKind;
use crate::{feature_error, unreachable_error};
use Visibility::*;

/// Checks & infers types of an AST, and convert (lower) it into a HIR
#[derive(Debug)]
pub struct ASTLowerer {
    cfg: ErgConfig,
    pub(crate) ctx: Context,
    pub(crate) errs: LowerErrors,
    pub(crate) warns: LowerWarnings,
}

impl Default for ASTLowerer {
    fn default() -> Self {
        Self::new_with_cache(
            ErgConfig::default(),
            Str::ever("<module>"),
            SharedModuleCache::new(ErgConfig::default()),
            SharedModuleCache::new(ErgConfig::default()),
        )
    }
}

impl Runnable for ASTLowerer {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg lowerer";

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        &mut self.cfg
    }

    fn new(cfg: ErgConfig) -> Self {
        Self::new_with_cache(
            cfg.copy(),
            Str::ever("<module>"),
            SharedModuleCache::new(cfg.copy()),
            SharedModuleCache::new(cfg),
        )
    }

    #[inline]
    fn finish(&mut self) {}

    fn initialize(&mut self) {
        self.ctx.initialize();
        self.errs.clear();
        self.warns.clear();
    }

    fn clear(&mut self) {
        self.errs.clear();
        self.warns.clear();
    }

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let ast = ast_builder.build(self.input().read())?;
        let artifact = self
            .lower(ast, "exec")
            .map_err(|artifact| artifact.errors)?;
        artifact.warns.fmt_all_stderr();
        println!("{}", artifact.object);
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let ast = ast_builder.build(src)?;
        let artifact = self
            .lower(ast, "eval")
            .map_err(|artifact| artifact.errors)?;
        artifact.warns.fmt_all_stderr();
        Ok(format!("{}", artifact.object))
    }
}

use erg_parser::ast::VarName;
impl ContextProvider for ASTLowerer {
    fn dir(&self) -> Vec<(&VarName, &VarInfo)> {
        self.ctx.dir()
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.ctx.get_receiver_ctx(receiver_name)
    }

    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.ctx.get_var_info(name)
    }
}

impl ASTLowerer {
    pub fn new_with_cache<S: Into<Str>>(
        cfg: ErgConfig,
        mod_name: S,
        mod_cache: SharedModuleCache,
        py_mod_cache: SharedModuleCache,
    ) -> Self {
        Self {
            ctx: Context::new_module(mod_name, cfg.clone(), mod_cache, py_mod_cache),
            cfg,
            errs: LowerErrors::empty(),
            warns: LowerWarnings::empty(),
        }
    }

    fn var_result_t_check(
        &self,
        loc: Location,
        name: &Str,
        expect: &Type,
        found: &Type,
    ) -> SingleLowerResult<()> {
        self.ctx
            .sub_unify(found, expect, loc, Some(name))
            .map_err(|_| {
                LowerError::type_mismatch_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    loc,
                    self.ctx.caused_by(),
                    name,
                    None,
                    expect,
                    found,
                    self.ctx.get_candidates(found),
                    Context::get_simple_type_mismatch_hint(expect, found),
                )
            })
    }

    /// OK: exec `i: Int`
    /// OK: exec `i: Int = 1`
    /// NG: exec `1 + 2`
    /// OK: exec `None`
    fn use_check(&self, expr: &hir::Expr, mode: &str) -> LowerResult<()> {
        if mode != "eval" && !expr.ref_t().is_nonelike() && !expr.is_type_asc() {
            Err(LowerWarnings::from(LowerWarning::unused_expr_warning(
                self.cfg.input.clone(),
                line!() as usize,
                expr,
                String::from(&self.ctx.name[..]),
            )))
        } else {
            self.block_use_check(expr, mode)
        }
    }

    fn block_use_check(&self, expr: &hir::Expr, mode: &str) -> LowerResult<()> {
        let mut warns = LowerWarnings::empty();
        match expr {
            hir::Expr::Def(def) => {
                let last = def.body.block.len() - 1;
                for (i, chunk) in def.body.block.iter().enumerate() {
                    if i == last {
                        if let Err(ws) = self.block_use_check(chunk, mode) {
                            warns.extend(ws);
                        }
                        break;
                    }
                    if let Err(ws) = self.use_check(chunk, mode) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::Lambda(lambda) => {
                let last = lambda.body.len() - 1;
                for (i, chunk) in lambda.body.iter().enumerate() {
                    if i == last {
                        if let Err(ws) = self.block_use_check(chunk, mode) {
                            warns.extend(ws);
                        }
                        break;
                    }
                    if let Err(ws) = self.use_check(chunk, mode) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::ClassDef(class_def) => {
                for chunk in class_def.methods.iter() {
                    if let Err(ws) = self.use_check(chunk, mode) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::PatchDef(patch_def) => {
                for chunk in patch_def.methods.iter() {
                    if let Err(ws) = self.use_check(chunk, mode) {
                        warns.extend(ws);
                    }
                }
            }
            hir::Expr::Call(call) => {
                for arg in call.args.pos_args.iter() {
                    if let Err(ws) = self.block_use_check(&arg.expr, mode) {
                        warns.extend(ws);
                    }
                }
                if let Some(var_args) = &call.args.var_args {
                    if let Err(ws) = self.block_use_check(&var_args.expr, mode) {
                        warns.extend(ws);
                    }
                }
                for arg in call.args.kw_args.iter() {
                    if let Err(ws) = self.block_use_check(&arg.expr, mode) {
                        warns.extend(ws);
                    }
                }
            }
            // TODO: unary, binary, array, ...
            _ => {}
        }
        if warns.is_empty() {
            Ok(())
        } else {
            Err(warns)
        }
    }

    fn pop_append_errs(&mut self) {
        if let Err(mut errs) = self.ctx.check_decls_and_pop() {
            self.errs.append(&mut errs);
        }
    }

    pub fn pop_mod_ctx(&mut self) -> Option<Context> {
        self.ctx.pop_mod()
    }

    pub fn pop_mod_ctx_or_default(&mut self) -> Context {
        std::mem::take(&mut self.ctx)
    }

    pub fn get_mod_ctx(&self) -> &Context {
        &self.ctx
    }

    pub fn dir(&self) -> Vec<(&VarName, &VarInfo)> {
        ContextProvider::dir(self)
    }

    pub fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        ContextProvider::get_receiver_ctx(self, receiver_name)
    }

    pub fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        ContextProvider::get_var_info(self, name)
    }
}

impl ASTLowerer {
    fn lower_literal(&self, lit: ast::Literal) -> LowerResult<hir::Literal> {
        let loc = lit.loc();
        let lit = hir::Literal::try_from(lit.token).map_err(|_| {
            LowerError::invalid_literal(
                self.cfg.input.clone(),
                line!() as usize,
                loc,
                self.ctx.caused_by(),
            )
        })?;
        Ok(lit)
    }

    fn lower_array(&mut self, array: ast::Array) -> LowerResult<hir::Array> {
        log!(info "entered {}({array})", fn_name!());
        match array {
            ast::Array::Normal(arr) => Ok(hir::Array::Normal(self.lower_normal_array(arr)?)),
            ast::Array::WithLength(arr) => {
                Ok(hir::Array::WithLength(self.lower_array_with_length(arr)?))
            }
            other => feature_error!(
                LowerErrors,
                LowerError,
                self.ctx,
                other.loc(),
                "array comprehension"
            ),
        }
    }

    fn elem_err(&self, l: &Type, r: &Type, elem: &hir::Expr) -> LowerErrors {
        let elem_disp_notype = elem.to_string_notype();
        let l = Context::readable_type(l);
        let r = Context::readable_type(r);
        LowerErrors::from(LowerError::syntax_error(
            self.cfg.input.clone(),
            line!() as usize,
            elem.loc(),
            String::from(&self.ctx.name[..]),
            switch_lang!(
                "japanese" => "配列の要素は全て同じ型である必要があります",
                "simplified_chinese" => "数组元素必须全部是相同类型",
                "traditional_chinese" => "數組元素必須全部是相同類型",
                "english" => "all elements of an array must be of the same type",
            )
            .to_owned(),
            Some(switch_lang!(
                "japanese" => format!("[..., {elem_disp_notype}: {l} or {r}]など明示的に型を指定してください"),
                "simplified_chinese" => format!("请明确指定类型，例如: [..., {elem_disp_notype}: {l} or {r}]"),
                "traditional_chinese" => format!("請明確指定類型，例如: [..., {elem_disp_notype}: {l} or {r}]"),
                "english" => format!("please specify the type explicitly, e.g. [..., {elem_disp_notype}: {l} or {r}]"),
            )),
        ))
    }

    fn lower_normal_array(&mut self, array: ast::NormalArray) -> LowerResult<hir::NormalArray> {
        log!(info "entered {}({array})", fn_name!());
        let mut new_array = vec![];
        let (elems, _) = array.elems.into_iters();
        let mut union = Type::Never;
        for elem in elems {
            let elem = self.lower_expr(elem.expr)?;
            union = self.ctx.union(&union, elem.ref_t());
            if let Some((l, r)) = union.union_types() {
                match (l.is_unbound_var(), r.is_unbound_var()) {
                    (false, false) => {
                        return Err(self.elem_err(&l, &r, &elem));
                    }
                    // TODO: check if the type is compatible with the other type
                    (true, false) => {}
                    (false, true) => {}
                    (true, true) => {}
                }
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
            hir::Args::values(new_array, None),
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
        let maybe_len = self.ctx.eval_const_expr(len);
        match maybe_len {
            Ok(v @ ValueObj::Nat(_)) => {
                if elem.ref_t().is_mut_type() {
                    poly(
                        "ArrayWithMutType!",
                        vec![TyParam::t(elem.t()), TyParam::Value(v)],
                    )
                } else {
                    array_t(elem.t(), TyParam::Value(v))
                }
            }
            Ok(v @ ValueObj::Mut(_)) if v.class() == mono("Nat!") => {
                if elem.ref_t().is_mut_type() {
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
                if elem.ref_t().is_mut_type() {
                    poly(
                        "ArrayWithMutType!",
                        vec![TyParam::t(elem.t()), TyParam::erased(Type::Nat)],
                    )
                } else {
                    array_t(elem.t(), TyParam::erased(Type::Nat))
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
        let (elems, .., paren) = tuple.elems.deconstruct();
        for elem in elems {
            let elem = self.lower_expr(elem.expr)?;
            new_tuple.push(elem);
        }
        Ok(hir::NormalTuple::new(hir::Args::values(new_tuple, paren)))
    }

    fn lower_record(&mut self, record: ast::Record) -> LowerResult<hir::Record> {
        log!(info "entered {}({record})", fn_name!());
        match record {
            ast::Record::Normal(rec) => self.lower_normal_record(rec),
            ast::Record::Mixed(_rec) => unreachable!(), // should be desugared
        }
    }

    fn lower_normal_record(&mut self, record: ast::NormalRecord) -> LowerResult<hir::Record> {
        log!(info "entered {}({record})", fn_name!());
        let mut hir_record =
            hir::Record::new(record.l_brace, record.r_brace, hir::RecordAttrs::empty());
        self.ctx.grow("<record>", ContextKind::Dummy, Private, None);
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

    fn lower_set(&mut self, set: ast::Set) -> LowerResult<hir::Set> {
        log!(info "enter {}({set})", fn_name!());
        match set {
            ast::Set::Normal(set) => Ok(hir::Set::Normal(self.lower_normal_set(set)?)),
            ast::Set::WithLength(set) => Ok(hir::Set::WithLength(self.lower_set_with_length(set)?)),
        }
    }

    fn lower_normal_set(&mut self, set: ast::NormalSet) -> LowerResult<hir::NormalSet> {
        log!(info "entered {}({set})", fn_name!());
        let (elems, _) = set.elems.into_iters();
        let mut union = Type::Never;
        let mut new_set = vec![];
        for elem in elems {
            let elem = self.lower_expr(elem.expr)?;
            union = self.ctx.union(&union, elem.ref_t());
            if union.is_intersection_type() {
                return Err(LowerErrors::from(LowerError::syntax_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    elem.loc(),
                    String::from(&self.ctx.name[..]),
                    switch_lang!(
                        "japanese" => "集合の要素は全て同じ型である必要があります",
                        "simplified_chinese" => "集合元素必须全部是相同类型",
                        "traditional_chinese" => "集合元素必須全部是相同類型",
                        "english" => "all elements of a set must be of the same type",
                    )
                    .to_owned(),
                    Some(
                        switch_lang!(
                            "japanese" => "Int or Strなど明示的に型を指定してください",
                            "simplified_chinese" => "明确指定类型，例如: Int or Str",
                            "traditional_chinese" => "明確指定類型，例如: Int or Str",
                            "english" => "please specify the type explicitly, e.g. Int or Str",
                        )
                        .to_owned(),
                    ),
                )));
            }
            new_set.push(elem);
        }
        let elem_t = if union == Type::Never {
            free_var(self.ctx.level, Constraint::new_type_of(Type::Type))
        } else {
            union
        };
        // TODO: lint
        /*
        if is_duplicated {
            self.warns.push(LowerWarning::syntax_error(
                self.cfg.input.clone(),
                line!() as usize,
                normal_set.loc(),
                String::arc(&self.ctx.name[..]),
                switch_lang!(
                    "japanese" => "要素が重複しています",
                    "simplified_chinese" => "元素重复",
                    "traditional_chinese" => "元素重複",
                    "english" => "Elements are duplicated",
                ),
                None,
            ));
        }
        Ok(normal_set)
        */
        let elems = hir::Args::values(new_set, None);
        // check if elem_t is Eq
        if let Err(errs) = self.ctx.sub_unify(&elem_t, &mono("Eq"), elems.loc(), None) {
            self.errs.extend(errs);
        }
        Ok(hir::NormalSet::new(set.l_brace, set.r_brace, elem_t, elems))
    }

    /// This (e.g. {"a"; 3}) is meaningless as an object, but makes sense as a type (e.g. {Int; 3}).
    fn lower_set_with_length(
        &mut self,
        set: ast::SetWithLength,
    ) -> LowerResult<hir::SetWithLength> {
        log!("entered {}({set})", fn_name!());
        let elem = self.lower_expr(set.elem.expr)?;
        let set_t = self.gen_set_with_length_type(&elem, &set.len);
        let len = self.lower_expr(*set.len)?;
        let hir_set = hir::SetWithLength::new(set.l_brace, set.r_brace, set_t, elem, len);
        Ok(hir_set)
    }

    fn gen_set_with_length_type(&mut self, elem: &hir::Expr, len: &ast::Expr) -> Type {
        let maybe_len = self.ctx.eval_const_expr(len);
        match maybe_len {
            Ok(v @ ValueObj::Nat(_)) => {
                if elem.ref_t().is_mut_type() {
                    poly(
                        "SetWithMutType!",
                        vec![TyParam::t(elem.t()), TyParam::Value(v)],
                    )
                } else if self.ctx.subtype_of(&elem.t(), &Type::Type) {
                    poly("SetType", vec![TyParam::t(elem.t()), TyParam::Value(v)])
                } else {
                    set_t(elem.t(), TyParam::Value(v))
                }
            }
            Ok(v @ ValueObj::Mut(_)) if v.class() == mono("Nat!") => {
                if elem.ref_t().is_mut_type() {
                    poly(
                        "SetWithMutTypeAndLength!",
                        vec![TyParam::t(elem.t()), TyParam::Value(v)],
                    )
                } else {
                    set_mut(elem.t(), TyParam::Value(v))
                }
            }
            Ok(other) => todo!("{other} is not a Nat object"),
            Err(_e) => {
                if elem.ref_t().is_mut_type() {
                    poly(
                        "SetWithMutType!",
                        vec![TyParam::t(elem.t()), TyParam::erased(Type::Nat)],
                    )
                } else {
                    set_t(elem.t(), TyParam::erased(Type::Nat))
                }
            }
        }
    }

    fn lower_dict(&mut self, dict: ast::Dict) -> LowerResult<hir::Dict> {
        log!(info "enter {}({dict})", fn_name!());
        match dict {
            ast::Dict::Normal(set) => Ok(hir::Dict::Normal(self.lower_normal_dict(set)?)),
            other => feature_error!(
                LowerErrors,
                LowerError,
                self.ctx,
                other.loc(),
                "dict comprehension"
            ),
            // ast::Dict::WithLength(set) => Ok(hir::Dict::WithLength(self.lower_dict_with_length(set)?)),
        }
    }

    fn lower_normal_dict(&mut self, dict: ast::NormalDict) -> LowerResult<hir::NormalDict> {
        log!(info "enter {}({dict})", fn_name!());
        let mut union = dict! {};
        let mut new_kvs = vec![];
        for kv in dict.kvs {
            let loc = kv.loc();
            let key = self.lower_expr(kv.key)?;
            let value = self.lower_expr(kv.value)?;
            if union.insert(key.t(), value.t()).is_some() {
                return Err(LowerErrors::from(LowerError::syntax_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    loc,
                    String::from(&self.ctx.name[..]),
                    switch_lang!(
                        "japanese" => "Dictの値は全て同じ型である必要があります",
                        "simplified_chinese" => "Dict的值必须是同一类型",
                        "traditional_chinese" => "Dict的值必須是同一類型",
                        "english" => "Values of Dict must be the same type",
                    )
                    .to_owned(),
                    Some(
                        switch_lang!(
                            "japanese" => "Int or Strなど明示的に型を指定してください",
                            "simplified_chinese" => "明确指定类型，例如: Int or Str",
                            "traditional_chinese" => "明確指定類型，例如: Int or Str",
                            "english" => "please specify the type explicitly, e.g. Int or Str",
                        )
                        .to_owned(),
                    ),
                )));
            }
            new_kvs.push(hir::KeyValue::new(key, value));
        }
        for key_t in union.keys() {
            let loc = Location::concat(&dict.l_brace, &dict.r_brace);
            // check if key_t is Eq
            if let Err(errs) = self.ctx.sub_unify(key_t, &mono("Eq"), loc, None) {
                self.errs.extend(errs);
            }
        }
        let kv_ts = if union.is_empty() {
            dict! {
                ty_tp(free_var(self.ctx.level, Constraint::new_type_of(Type::Type))) =>
                    ty_tp(free_var(self.ctx.level, Constraint::new_type_of(Type::Type)))
            }
        } else {
            union
                .into_iter()
                .map(|(k, v)| (TyParam::t(k), TyParam::t(v)))
                .collect()
        };
        // TODO: lint
        /*
        if is_duplicated {
            self.warns.push(LowerWarning::syntax_error(
                self.cfg.input.clone(),
                line!() as usize,
                normal_set.loc(),
                String::arc(&self.ctx.name[..]),
                switch_lang!(
                    "japanese" => "要素が重複しています",
                    "simplified_chinese" => "元素重复",
                    "traditional_chinese" => "元素重複",
                    "english" => "Elements are duplicated",
                ),
                None,
            ));
        }
        Ok(normal_set)
        */
        Ok(hir::NormalDict::new(
            dict.l_brace,
            dict.r_brace,
            kv_ts,
            new_kvs,
        ))
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
                let vi = self.ctx.rec_get_attr_info(
                    &obj,
                    &attr.ident,
                    &self.cfg.input,
                    &self.ctx.name,
                )?;
                let ident = hir::Identifier::new(attr.ident.dot, attr.ident.name, None, vi);
                let acc = hir::Accessor::Attr(hir::Attribute::new(obj, ident));
                Ok(acc)
            }
            ast::Accessor::TypeApp(t_app) => feature_error!(
                LowerErrors,
                LowerError,
                self.ctx,
                t_app.loc(),
                "type application"
            ),
            // TupleAttr, Subscr are desugared
            _ => unreachable_error!(LowerErrors, LowerError, self.ctx),
        }
    }

    fn lower_ident(&self, ident: ast::Identifier) -> LowerResult<hir::Identifier> {
        // `match` is a special form, typing is magic
        let (vi, __name__) = if ident.vis().is_private()
            && (&ident.inspect()[..] == "match" || &ident.inspect()[..] == "match!")
        {
            (
                VarInfo {
                    t: mono("GenericCallable"),
                    ..VarInfo::default()
                },
                None,
            )
        } else {
            (
                self.ctx.rec_get_var_info(
                    &ident,
                    AccessKind::Name,
                    &self.cfg.input,
                    &self.ctx.name,
                )?,
                self.ctx
                    .get_singular_ctx_by_ident(&ident, &self.ctx.name)
                    .ok()
                    .map(|ctx| ctx.name.clone()),
            )
        };
        let ident = hir::Identifier::new(ident.dot, ident.name, __name__, vi);
        Ok(ident)
    }

    fn lower_bin(&mut self, bin: ast::BinOp) -> LowerResult<hir::BinOp> {
        log!(info "entered {}({bin})", fn_name!());
        let mut args = bin.args.into_iter();
        let lhs = hir::PosArg::new(self.lower_expr(*args.next().unwrap())?);
        let rhs = hir::PosArg::new(self.lower_expr(*args.next().unwrap())?);
        let args = [lhs, rhs];
        let t = self
            .ctx
            .get_binop_t(&bin.op, &args, &self.cfg.input, &self.ctx.name)?;
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
        let t = self
            .ctx
            .get_unaryop_t(&unary.op, &args, &self.cfg.input, &self.ctx.name)?;
        let mut args = args.into_iter();
        let expr = args.next().unwrap().expr;
        Ok(hir::UnaryOp::new(unary.op, expr, t))
    }

    pub(crate) fn lower_call(&mut self, call: ast::Call) -> LowerResult<hir::Call> {
        log!(info "entered {}({}{}(...))", fn_name!(), call.obj, fmt_option!(call.attr_name));
        let mut errs = LowerErrors::empty();
        let opt_cast_to = if call.is_assert_cast() {
            if let Some(typ) = call.assert_cast_target_type() {
                Some(Parser::expr_to_type_spec(typ.clone()).map_err(|e| {
                    let e = LowerError::new(e.into(), self.input().clone(), self.ctx.caused_by());
                    LowerErrors::from(e)
                })?)
            } else {
                return Err(LowerErrors::from(LowerError::syntax_error(
                    self.input().clone(),
                    line!() as usize,
                    call.args.loc(),
                    self.ctx.caused_by(),
                    "invalid assert casting type".to_owned(),
                    None,
                )));
            }
        } else {
            None
        };
        let (pos_args, kw_args, paren) = call.args.deconstruct();
        let mut hir_args = hir::Args::new(
            Vec::with_capacity(pos_args.len()),
            None,
            Vec::with_capacity(kw_args.len()),
            paren,
        );
        for arg in pos_args.into_iter() {
            match self.lower_expr(arg.expr) {
                Ok(expr) => hir_args.pos_args.push(hir::PosArg::new(expr)),
                Err(es) => errs.extend(es),
            }
        }
        for arg in kw_args.into_iter() {
            match self.lower_expr(arg.expr) {
                Ok(expr) => hir_args.push_kw(hir::KwArg::new(arg.keyword, expr)),
                Err(es) => errs.extend(es),
            }
        }
        let mut obj = match self.lower_expr(*call.obj) {
            Ok(obj) => obj,
            Err(es) => {
                errs.extend(es);
                return Err(errs);
            }
        };
        // Calling get_call_t with incorrect arguments does not provide meaningful information
        if !errs.is_empty() {
            return Err(errs);
        }
        let vi = match self.ctx.get_call_t(
            &obj,
            &call.attr_name,
            &hir_args.pos_args,
            &hir_args.kw_args,
            &self.cfg.input,
            &self.ctx.name,
        ) {
            Ok(vi) => vi,
            Err(es) => {
                errs.extend(es);
                return Err(errs);
            }
        };
        let attr_name = if let Some(attr_name) = call.attr_name {
            Some(hir::Identifier::new(
                attr_name.dot,
                attr_name.name,
                None,
                vi,
            ))
        } else {
            *obj.ref_mut_t() = vi.t;
            None
        };
        let mut call = hir::Call::new(obj, attr_name, hir_args);
        match call.additional_operation() {
            Some(kind @ (OperationKind::Import | OperationKind::PyImport)) => {
                let mod_name =
                    enum_unwrap!(call.args.get_left_or_key("Path").unwrap(), hir::Expr::Lit);
                if let Err(errs) = self.ctx.import_mod(kind, mod_name) {
                    self.errs.extend(errs);
                };
            }
            Some(OperationKind::Del) => match call.args.get_left_or_key("obj").unwrap() {
                hir::Expr::Accessor(hir::Accessor::Ident(ident)) => {
                    self.ctx.del(ident)?;
                }
                other => {
                    return Err(LowerErrors::from(LowerError::syntax_error(
                        self.input().clone(),
                        line!() as usize,
                        other.loc(),
                        self.ctx.caused_by(),
                        "".to_owned(),
                        None,
                    )))
                }
            },
            Some(OperationKind::Return | OperationKind::Yield) => {
                // (f: ?T -> ?U).return: (self: GenericCallable, arg: Obj) -> Never
                let callable_t = call.obj.ref_t();
                let ret_t = match callable_t {
                    Type::Subr(subr) => *subr.return_t.clone(),
                    Type::FreeVar(fv) if fv.is_unbound() => {
                        fv.get_sub().unwrap().return_t().unwrap().clone()
                    }
                    other => todo!("{other:?}"),
                };
                let arg_t = call.args.get(0).unwrap().ref_t();
                self.ctx.sub_unify(arg_t, &ret_t, call.loc(), None)?;
            }
            _ => {
                if let Some(type_spec) = opt_cast_to {
                    self.ctx.cast(type_spec, &mut call)?;
                }
            }
        }
        if errs.is_empty() {
            Ok(call)
        } else {
            Err(errs)
        }
    }

    fn lower_pack(&mut self, pack: ast::DataPack) -> LowerResult<hir::Call> {
        log!(info "entered {}({pack})", fn_name!());
        let class = self.lower_expr(*pack.class)?;
        let args = self.lower_record(pack.args)?;
        let args = vec![hir::PosArg::new(hir::Expr::Record(args))];
        let attr_name = ast::Identifier::new(
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
        let vi = self.ctx.get_call_t(
            &class,
            &Some(attr_name.clone()),
            &args,
            &[],
            &self.cfg.input,
            &self.ctx.name,
        )?;
        let args = hir::Args::new(args, None, vec![], None);
        let attr_name = hir::Identifier::new(attr_name.dot, attr_name.name, None, vi);
        Ok(hir::Call::new(class, Some(attr_name), args))
    }

    fn lower_params(&mut self, params: ast::Params) -> LowerResult<hir::Params> {
        log!(info "entered {}({})", fn_name!(), params);
        let mut errs = LowerErrors::empty();
        let mut hir_defaults = vec![];
        for default in params.defaults.into_iter() {
            match self.lower_expr(default.default_val) {
                Ok(default_val) => {
                    hir_defaults.push(hir::DefaultParamSignature::new(default.sig, default_val));
                }
                Err(es) => errs.extend(es),
            }
        }
        if !errs.is_empty() {
            Err(errs)
        } else {
            let hir_params = hir::Params::new(
                params.non_defaults,
                params.var_args,
                hir_defaults,
                params.parens,
            );
            Ok(hir_params)
        }
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
        let tv_cache = self
            .ctx
            .instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal)?;
        self.ctx.grow(&name, kind, Private, Some(tv_cache));
        let params = self.lower_params(lambda.sig.params).map_err(|errs| {
            self.pop_append_errs();
            errs
        })?;
        if let Err(errs) = self.ctx.assign_params(&params, None) {
            self.errs.extend(errs);
        }
        if let Err(errs) = self.ctx.preregister(&lambda.body) {
            self.errs.extend(errs);
        }
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
        self.pop_append_errs();
        let t = if is_procedural {
            proc(non_default_params, None, default_params, body.t())
        } else {
            func(non_default_params, None, default_params, body.t())
        };
        let t = if t.has_qvar() { t.quantify() } else { t };
        Ok(hir::Lambda::new(id, params, lambda.op, body, t))
    }

    fn lower_def(&mut self, def: ast::Def) -> LowerResult<hir::Def> {
        log!(info "entered {}({})", fn_name!(), def.sig);
        if def.def_kind().is_class_or_trait() && self.ctx.kind != ContextKind::Module {
            self.ctx.decls.remove(def.sig.ident().unwrap().inspect());
            return Err(LowerErrors::from(LowerError::inner_typedef_error(
                self.cfg.input.clone(),
                line!() as usize,
                def.loc(),
                self.ctx.caused_by(),
            )));
        }
        let name = if let Some(name) = def.sig.name_as_str() {
            name.clone()
        } else {
            Str::ever("<lambda>")
        };
        if self
            .ctx
            .registered_info(&name, def.sig.is_const())
            .is_some()
            && def.sig.vis().is_private()
        {
            return Err(LowerErrors::from(LowerError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                def.sig.loc(),
                self.ctx.caused_by(),
                &name,
            )));
        }
        let kind = ContextKind::from(def.def_kind());
        let vis = def.sig.vis();
        let res = match def.sig {
            ast::Signature::Subr(sig) => {
                let tv_cache = self
                    .ctx
                    .instantiate_ty_bounds(&sig.bounds, RegistrationMode::Normal)?;
                self.ctx.grow(&name, kind, vis, Some(tv_cache));
                self.lower_subr_def(sig, def.body)
            }
            ast::Signature::Var(sig) => {
                self.ctx.grow(&name, kind, vis, None);
                self.lower_var_def(sig, def.body)
            }
        };
        // TODO: Context上の関数に型境界情報を追加
        self.pop_append_errs();
        // remove from decls regardless of success or failure to lower
        self.ctx.decls.remove(&name);
        res
    }

    fn lower_var_def(
        &mut self,
        sig: ast::VarSignature,
        body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        log!(info "entered {}({sig})", fn_name!());
        if let Err(errs) = self.ctx.preregister(&body.block) {
            self.errs.extend(errs);
        }
        match self.lower_block(body.block) {
            Ok(block) => {
                let found_body_t = block.ref_t();
                let outer = self.ctx.outer.as_ref().unwrap();
                let opt_expect_body_t = sig
                    .inspect()
                    .and_then(|name| outer.get_current_scope_var(name).map(|vi| vi.t.clone()));
                let ident = match &sig.pat {
                    ast::VarPattern::Ident(ident) => ident,
                    ast::VarPattern::Discard(_) => ast::Identifier::UBAR,
                    _ => unreachable!(),
                };
                if let Some(expect_body_t) = opt_expect_body_t {
                    // TODO: expect_body_t is smaller for constants
                    // TODO: 定数の場合、expect_body_tのほうが小さくなってしまう
                    if !sig.is_const() {
                        if let Err(e) = self.var_result_t_check(
                            sig.loc(),
                            ident.inspect(),
                            &expect_body_t,
                            found_body_t,
                        ) {
                            self.errs.push(e);
                        }
                    }
                }
                self.ctx.outer.as_mut().unwrap().assign_var_sig(
                    &sig,
                    found_body_t,
                    body.id,
                    None,
                )?;
                let mut ident = hir::Identifier::bare(ident.dot.clone(), ident.name.clone());
                ident.vi.t = found_body_t.clone();
                let sig = hir::VarSignature::new(ident);
                let body = hir::DefBody::new(body.op, block, body.id);
                Ok(hir::Def::new(hir::Signature::Var(sig), body))
            }
            Err(errs) => {
                self.ctx.outer.as_mut().unwrap().assign_var_sig(
                    &sig,
                    &Type::Failure,
                    ast::DefId(0),
                    None,
                )?;
                Err(errs)
            }
        }
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
            .map(|vi| vi.t.clone())
            .unwrap_or(Type::Failure);
        match t {
            Type::Subr(subr_t) => {
                let params = self.lower_params(sig.params.clone())?;
                if let Err(errs) = self.ctx.assign_params(&params, Some(subr_t)) {
                    self.errs.extend(errs);
                }
                if let Err(errs) = self.ctx.preregister(&body.block) {
                    self.errs.extend(errs);
                }
                match self.lower_block(body.block) {
                    Ok(block) => {
                        let found_body_t = block.ref_t();
                        let t = self.ctx.outer.as_mut().unwrap().assign_subr(
                            &sig,
                            body.id,
                            found_body_t,
                        )?;
                        let return_t = t.return_t().unwrap();
                        if return_t.union_types().is_some() && sig.return_t_spec.is_none() {
                            let warn = LowerWarning::union_return_type_warning(
                                self.input().clone(),
                                line!() as usize,
                                sig.loc(),
                                self.ctx.caused_by(),
                                sig.ident.inspect(),
                                &Context::readable_type(return_t),
                            );
                            self.warns.push(warn);
                        }
                        let mut ident = hir::Identifier::bare(sig.ident.dot, sig.ident.name);
                        ident.vi.t = t;
                        let sig = hir::SubrSignature::new(ident, params);
                        let body = hir::DefBody::new(body.op, block, body.id);
                        Ok(hir::Def::new(hir::Signature::Subr(sig), body))
                    }
                    Err(errs) => {
                        self.ctx.outer.as_mut().unwrap().assign_subr(
                            &sig,
                            ast::DefId(0),
                            &Type::Failure,
                        )?;
                        Err(errs)
                    }
                }
            }
            Type::Failure => {
                let params = self.lower_params(sig.params)?;
                if let Err(errs) = self.ctx.assign_params(&params, None) {
                    self.errs.extend(errs);
                }
                if let Err(errs) = self.ctx.preregister(&body.block) {
                    self.errs.extend(errs);
                }
                self.ctx.outer.as_mut().unwrap().fake_subr_assign(
                    &sig.ident,
                    &sig.decorators,
                    Type::Failure,
                )?;
                let block = self.lower_block(body.block)?;
                let ident = hir::Identifier::bare(sig.ident.dot, sig.ident.name);
                let sig = hir::SubrSignature::new(ident, params);
                let body = hir::DefBody::new(body.op, block, body.id);
                Ok(hir::Def::new(hir::Signature::Subr(sig), body))
            }
            _ => unreachable_error!(LowerErrors, LowerError, self),
        }
    }

    fn lower_class_def(&mut self, class_def: ast::ClassDef) -> LowerResult<hir::ClassDef> {
        log!(info "entered {}({class_def})", fn_name!());
        let mut hir_def = self.lower_def(class_def.def)?;
        let mut hir_methods = hir::Block::empty();
        let mut dummy_tv_cache = TyVarCache::new(self.ctx.level, &self.ctx);
        for mut methods in class_def.methods_list.into_iter() {
            let (class, impl_trait) = match &methods.class {
                ast::TypeSpec::TypeApp { spec, args } => {
                    let (impl_trait, loc) = match &args.args.pos_args().first().unwrap().expr {
                        // TODO: check `tasc.op`
                        ast::Expr::TypeAsc(tasc) => (
                            self.ctx.instantiate_typespec(
                                &tasc.t_spec,
                                None,
                                &mut dummy_tv_cache,
                                RegistrationMode::Normal,
                                false,
                            )?,
                            tasc.t_spec.loc(),
                        ),
                        _ => return unreachable_error!(LowerErrors, LowerError, self),
                    };
                    (
                        self.ctx.instantiate_typespec(
                            spec,
                            None,
                            &mut dummy_tv_cache,
                            RegistrationMode::Normal,
                            false,
                        )?,
                        Some((impl_trait, loc)),
                    )
                }
                other => (
                    self.ctx.instantiate_typespec(
                        other,
                        None,
                        &mut dummy_tv_cache,
                        RegistrationMode::Normal,
                        false,
                    )?,
                    None,
                ),
            };
            // assume the class has implemented the trait, regardless of whether the implementation is correct
            if let Some((trait_, trait_loc)) = &impl_trait {
                self.register_trait_impl(&class, trait_, *trait_loc)?;
            }
            if let Some((_, class_root)) = self.ctx.get_nominal_type_ctx(&class) {
                if !class_root.kind.is_class() {
                    return Err(LowerErrors::from(LowerError::method_definition_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        methods.loc(),
                        self.ctx.caused_by(),
                        &class.qual_name(),
                        None,
                    )));
                }
            } else {
                return Err(LowerErrors::from(LowerError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    methods.class.loc(),
                    self.ctx.caused_by(),
                    &class.qual_name(),
                    self.ctx.get_similar_name(&class.local_name()),
                )));
            }
            let kind = ContextKind::MethodDefs(impl_trait.as_ref().map(|(t, _)| t.clone()));
            let vis = if self.cfg.python_compatible_mode {
                Public
            } else {
                Private
            };
            self.ctx.grow(&class.local_name(), kind, vis, None);
            for attr in methods.attrs.iter_mut() {
                match attr {
                    ast::ClassAttr::Def(def) => {
                        if methods.vis.is(TokenKind::Dot) {
                            def.sig.ident_mut().unwrap().dot = Some(Token::new(
                                TokenKind::Dot,
                                ".",
                                def.sig.ln_begin().unwrap_or(0),
                                def.sig.col_begin().unwrap_or(0),
                            ));
                        }
                        self.ctx.preregister_def(def).map_err(|errs| {
                            self.pop_append_errs();
                            errs
                        })?;
                    }
                    ast::ClassAttr::Decl(_decl) => {}
                }
            }
            for attr in methods.attrs.into_iter() {
                match attr {
                    ast::ClassAttr::Def(def) => match self.lower_def(def) {
                        Ok(def) => {
                            hir_methods.push(hir::Expr::Def(def));
                        }
                        Err(errs) => {
                            self.errs.extend(errs);
                        }
                    },
                    ast::ClassAttr::Decl(decl) => {
                        let decl = self.lower_type_asc(decl)?;
                        hir_methods.push(hir::Expr::TypeAsc(decl));
                    }
                }
            }
            if let Err(mut errs) = self.ctx.check_decls() {
                self.errs.append(&mut errs);
            }
            if let Some((trait_, _)) = &impl_trait {
                self.check_override(&class, Some(trait_));
            } else {
                self.check_override(&class, None);
            }
            if let Err(err) = self.check_trait_impl(impl_trait, &class) {
                self.errs.push(err);
            }
            self.check_collision_and_push(class);
        }
        let class = mono(hir_def.sig.ident().inspect());
        let Some((_, class_ctx)) = self.ctx.get_nominal_type_ctx(&class) else {
            return Err(LowerErrors::from(LowerError::type_not_found(
                self.cfg.input.clone(),
                line!() as usize,
                hir_def.sig.loc(),
                self.ctx.caused_by(),
                &class,
            )));
        };
        let type_obj = enum_unwrap!(self.ctx.rec_get_const_obj(hir_def.sig.ident().inspect()).unwrap(), ValueObj::Type:(TypeObj::Generated:(_)));
        let sup_type = enum_unwrap!(&hir_def.body.block.first().unwrap(), hir::Expr::Call)
            .args
            .get_left_or_key("Super")
            .unwrap();
        Self::check_inheritable(&self.cfg, &mut self.errs, type_obj, sup_type, &hir_def.sig);
        // vi.t.non_default_params().unwrap()[0].typ().clone()
        let (__new__, need_to_gen_new) = if let (Some(dunder_new_vi), Some(new_vi)) = (
            class_ctx.get_current_scope_var("__new__"),
            class_ctx.get_current_scope_var("new"),
        ) {
            (dunder_new_vi.t.clone(), new_vi.kind == VarKind::Auto)
        } else {
            return unreachable_error!(LowerErrors, LowerError, self);
        };
        let require_or_sup = self.get_require_or_sup_or_base(hir_def.body.block.remove(0));
        Ok(hir::ClassDef::new(
            type_obj.clone(),
            hir_def.sig,
            require_or_sup,
            need_to_gen_new,
            __new__,
            hir_methods,
        ))
    }

    fn lower_patch_def(&mut self, class_def: ast::PatchDef) -> LowerResult<hir::PatchDef> {
        log!(info "entered {}({class_def})", fn_name!());
        let base_t = {
            let base_t_expr =
                enum_unwrap!(class_def.def.body.block.get(0).unwrap(), ast::Expr::Call)
                    .args
                    .get_left_or_key("Base")
                    .unwrap();
            let spec = Parser::expr_to_type_spec(base_t_expr.clone()).unwrap();
            let mut dummy_tv_cache = TyVarCache::new(self.ctx.level, &self.ctx);
            self.ctx.instantiate_typespec(
                &spec,
                None,
                &mut dummy_tv_cache,
                RegistrationMode::Normal,
                false,
            )?
        };
        let mut hir_def = self.lower_def(class_def.def)?;
        let base = self.get_require_or_sup_or_base(hir_def.body.block.remove(0));
        let mut hir_methods = hir::Block::empty();
        for mut methods in class_def.methods_list.into_iter() {
            let kind = ContextKind::PatchMethodDefs(base_t.clone());
            self.ctx
                .grow(hir_def.sig.ident().inspect(), kind, hir_def.sig.vis(), None);
            for attr in methods.attrs.iter_mut() {
                match attr {
                    ast::ClassAttr::Def(def) => {
                        if methods.vis.is(TokenKind::Dot) {
                            def.sig.ident_mut().unwrap().dot = Some(Token::new(
                                TokenKind::Dot,
                                ".",
                                def.sig.ln_begin().unwrap(),
                                def.sig.col_begin().unwrap(),
                            ));
                        }
                        self.ctx.preregister_def(def).map_err(|errs| {
                            self.pop_append_errs();
                            errs
                        })?;
                    }
                    ast::ClassAttr::Decl(_decl) => {}
                }
            }
            for attr in methods.attrs.into_iter() {
                match attr {
                    ast::ClassAttr::Def(def) => match self.lower_def(def) {
                        Ok(def) => {
                            hir_methods.push(hir::Expr::Def(def));
                        }
                        Err(errs) => {
                            self.errs.extend(errs);
                        }
                    },
                    ast::ClassAttr::Decl(decl) => {
                        let decl = self.lower_type_asc(decl)?;
                        hir_methods.push(hir::Expr::TypeAsc(decl));
                    }
                }
            }
            if let Err(mut errs) = self.ctx.check_decls() {
                self.errs.append(&mut errs);
            }
            self.push_patch();
        }
        Ok(hir::PatchDef::new(hir_def.sig, base, hir_methods))
    }

    fn lower_attr_def(&mut self, attr_def: ast::AttrDef) -> LowerResult<hir::AttrDef> {
        log!(info "entered {}({attr_def})", fn_name!());
        let attr = self.lower_acc(attr_def.attr)?;
        let expr = self.lower_expr(*attr_def.expr)?;
        if let Err(err) = self.var_result_t_check(
            attr.loc(),
            &Str::from(attr.show()),
            attr.ref_t(),
            expr.ref_t(),
        ) {
            self.errs.push(err);
        }
        Ok(hir::AttrDef::new(attr, hir::Block::new(vec![expr])))
    }

    fn register_trait_impl(
        &mut self,
        class: &Type,
        trait_: &Type,
        trait_loc: Location,
    ) -> LowerResult<()> {
        // TODO: polymorphic trait
        if let Some(impls) = self.ctx.trait_impls.get_mut(&trait_.qual_name()) {
            impls.insert(TypeRelationInstance::new(class.clone(), trait_.clone()));
        } else {
            self.ctx.trait_impls.insert(
                trait_.qual_name(),
                set! {TypeRelationInstance::new(class.clone(), trait_.clone())},
            );
        }
        let trait_ctx = if let Some((_, trait_ctx)) = self.ctx.get_nominal_type_ctx(trait_) {
            trait_ctx.clone()
        } else {
            // TODO: maybe parameters are wrong
            return Err(LowerErrors::from(LowerError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                trait_loc,
                self.ctx.caused_by(),
                &trait_.local_name(),
                None,
            )));
        };
        let Some((_, class_ctx)) = self.ctx.get_mut_nominal_type_ctx(class) else {
            return Err(LowerErrors::from(LowerError::type_not_found(
                self.cfg.input.clone(),
                line!() as usize,
                trait_loc,
                self.ctx.caused_by(),
                class,
            )));
        };
        class_ctx.register_supertrait(trait_.clone(), &trait_ctx);
        Ok(())
    }

    /// HACK: Cannot be methodized this because `&self` has been taken immediately before.
    fn check_inheritable(
        cfg: &ErgConfig,
        errs: &mut LowerErrors,
        type_obj: &GenTypeObj,
        sup_class: &hir::Expr,
        sub_sig: &hir::Signature,
    ) {
        if let TypeObj::Generated(gen) = type_obj.require_or_sup().unwrap() {
            if let Some(impls) = gen.impls() {
                if !impls.contains_intersec(&mono("InheritableType")) {
                    errs.push(LowerError::inheritance_error(
                        cfg.input.clone(),
                        line!() as usize,
                        sup_class.to_string(),
                        sup_class.loc(),
                        sub_sig.ident().inspect().into(),
                    ));
                }
            } else {
                errs.push(LowerError::inheritance_error(
                    cfg.input.clone(),
                    line!() as usize,
                    sup_class.to_string(),
                    sup_class.loc(),
                    sub_sig.ident().inspect().into(),
                ));
            }
        }
    }

    fn check_override(&mut self, class: &Type, impl_trait: Option<&Type>) {
        if let Some(sups) = self.ctx.get_nominal_super_type_ctxs(class) {
            // exclude the first one because it is the class itself
            for sup in sups.into_iter().skip(1) {
                for (method_name, vi) in self.ctx.locals.iter().chain(
                    self.ctx
                        .methods_list
                        .iter()
                        .flat_map(|(_, c)| c.locals.iter()),
                ) {
                    if let Some(sup_vi) = sup.get_current_scope_var(method_name.inspect()) {
                        // must `@Override`
                        if let Some(decos) = &vi.comptime_decos {
                            if decos.contains("Override") {
                                continue;
                            }
                        }
                        if sup_vi.impl_of.as_ref() != impl_trait {
                            continue;
                        }
                        self.errs.push(LowerError::override_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            method_name.inspect(),
                            method_name.loc(),
                            &mono(&sup.name), // TODO: get super type
                            self.ctx.caused_by(),
                        ));
                    }
                }
            }
        }
    }

    /// Inspect the Trait implementation for correctness,
    /// i.e., check that all required attributes are defined and that no extra attributes are defined
    fn check_trait_impl(
        &mut self,
        impl_trait: Option<(Type, Location)>,
        class: &Type,
    ) -> SingleLowerResult<()> {
        if let Some((impl_trait, loc)) = impl_trait {
            let mut unverified_names = self.ctx.locals.keys().collect::<Set<_>>();
            if let Some(trait_obj) = self.ctx.rec_get_const_obj(&impl_trait.local_name()) {
                if let ValueObj::Type(typ) = trait_obj {
                    match typ {
                        TypeObj::Generated(gen) => match gen.require_or_sup().unwrap().typ() {
                            Type::Record(attrs) => {
                                for (field, decl_t) in attrs.iter() {
                                    if let Some((name, vi)) = self.ctx.get_local_kv(&field.symbol) {
                                        let def_t = &vi.t;
                                        //    A(<: Add(R)), R -> A.Output
                                        // => A(<: Int), R -> A.Output
                                        let replaced_decl_t =
                                            decl_t.clone().replace(&impl_trait, class);
                                        unverified_names.remove(name);
                                        // def_t must be subtype of decl_t
                                        if !self.ctx.supertype_of(&replaced_decl_t, def_t) {
                                            self.errs.push(LowerError::trait_member_type_error(
                                                self.cfg.input.clone(),
                                                line!() as usize,
                                                name.loc(),
                                                self.ctx.caused_by(),
                                                name.inspect(),
                                                &impl_trait,
                                                decl_t,
                                                &vi.t,
                                                None,
                                            ));
                                        }
                                    } else {
                                        self.errs.push(LowerError::trait_member_not_defined_error(
                                            self.cfg.input.clone(),
                                            line!() as usize,
                                            self.ctx.caused_by(),
                                            &field.symbol,
                                            &impl_trait,
                                            class,
                                            None,
                                        ));
                                    }
                                }
                            }
                            other => {
                                return feature_error!(
                                    LowerError,
                                    self.ctx,
                                    Location::Unknown,
                                    &format!("Impl {other}")
                                );
                            }
                        },
                        TypeObj::Builtin(_typ) => {
                            let (_, ctx) = self.ctx.get_nominal_type_ctx(_typ).unwrap();
                            for (decl_name, decl_vi) in ctx.decls.iter() {
                                if let Some((name, vi)) = self.ctx.get_local_kv(decl_name.inspect())
                                {
                                    let def_t = &vi.t;
                                    let replaced_decl_t =
                                        decl_vi.t.clone().replace(&impl_trait, class);
                                    unverified_names.remove(name);
                                    if !self.ctx.supertype_of(&replaced_decl_t, def_t) {
                                        self.errs.push(LowerError::trait_member_type_error(
                                            self.cfg.input.clone(),
                                            line!() as usize,
                                            name.loc(),
                                            self.ctx.caused_by(),
                                            name.inspect(),
                                            &impl_trait,
                                            &decl_vi.t,
                                            &vi.t,
                                            None,
                                        ));
                                    }
                                } else {
                                    self.errs.push(LowerError::trait_member_not_defined_error(
                                        self.cfg.input.clone(),
                                        line!() as usize,
                                        self.ctx.caused_by(),
                                        decl_name.inspect(),
                                        &impl_trait,
                                        class,
                                        None,
                                    ));
                                }
                            }
                        }
                    }
                } else {
                    return Err(LowerError::type_mismatch_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        loc,
                        self.ctx.caused_by(),
                        &impl_trait.qual_name(),
                        None,
                        &Type::TraitType,
                        &trait_obj.t(),
                        None,
                        None,
                    ));
                }
            } else {
                return Err(LowerError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    loc,
                    self.ctx.caused_by(),
                    &impl_trait.qual_name(),
                    self.ctx.get_similar_name(&impl_trait.local_name()),
                ));
            }
            for unverified in unverified_names {
                self.errs.push(LowerError::trait_member_not_defined_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    self.ctx.caused_by(),
                    unverified.inspect(),
                    &impl_trait,
                    class,
                    None,
                ));
            }
        }
        Ok(())
    }

    fn check_collision_and_push(&mut self, class: Type) {
        let methods = self.ctx.pop();
        let (_, class_root) = self
            .ctx
            .get_mut_nominal_type_ctx(&class)
            .unwrap_or_else(|| todo!("{class} not found"));
        for (newly_defined_name, vi) in methods.locals.clone().into_iter() {
            for (_, already_defined_methods) in class_root.methods_list.iter_mut() {
                // TODO: 特殊化なら同じ名前でもOK
                // TODO: 定義のメソッドもエラー表示
                if let Some((_already_defined_name, already_defined_vi)) =
                    already_defined_methods.get_local_kv(newly_defined_name.inspect())
                {
                    if already_defined_vi.kind != VarKind::Auto
                        && already_defined_vi.impl_of == vi.impl_of
                    {
                        self.errs.push(LowerError::duplicate_definition_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            newly_defined_name.loc(),
                            methods.caused_by(),
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
        class_root
            .methods_list
            .push((ClassDefType::Simple(class), methods));
    }

    fn push_patch(&mut self) {
        let methods = self.ctx.pop();
        let ContextKind::PatchMethodDefs(base) = &methods.kind else { unreachable!() };
        let patch_name = *methods.name.split_with(&["::", "."]).last().unwrap();
        let patch_root = self
            .ctx
            .patches
            .get_mut(patch_name)
            .unwrap_or_else(|| todo!("{} not found", methods.name));
        for (newly_defined_name, vi) in methods.locals.clone().into_iter() {
            for (_, already_defined_methods) in patch_root.methods_list.iter_mut() {
                // TODO: 特殊化なら同じ名前でもOK
                // TODO: 定義のメソッドもエラー表示
                if let Some((_already_defined_name, already_defined_vi)) =
                    already_defined_methods.get_local_kv(newly_defined_name.inspect())
                {
                    if already_defined_vi.kind != VarKind::Auto
                        && already_defined_vi.impl_of == vi.impl_of
                    {
                        self.errs.push(LowerError::duplicate_definition_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            newly_defined_name.loc(),
                            methods.caused_by(),
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
        patch_root
            .methods_list
            .push((ClassDefType::Simple(base.clone()), methods));
    }

    #[allow(clippy::only_used_in_recursion)]
    fn get_require_or_sup_or_base(&self, expr: hir::Expr) -> hir::Expr {
        match expr {
            acc @ hir::Expr::Accessor(_) => acc,
            hir::Expr::Call(mut call) => match call.obj.show_acc().as_ref().map(|s| &s[..]) {
                Some("Class") => call.args.remove_left_or_key("Requirement").unwrap(),
                Some("Inherit") => call.args.remove_left_or_key("Super").unwrap(),
                Some("Inheritable") => {
                    self.get_require_or_sup_or_base(call.args.remove_left_or_key("Class").unwrap())
                }
                Some("Patch") => call.args.remove_left_or_key("Base").unwrap(),
                _ => todo!(),
            },
            other => todo!("{other}"),
        }
    }

    fn lower_type_asc(&mut self, tasc: ast::TypeAscription) -> LowerResult<hir::TypeAscription> {
        log!(info "entered {}({tasc})", fn_name!());
        let is_instance_ascription = tasc.is_instance_ascription();
        let mut dummy_tv_cache = TyVarCache::new(self.ctx.level, &self.ctx);
        let t = self.ctx.instantiate_typespec(
            &tasc.t_spec,
            None,
            &mut dummy_tv_cache,
            RegistrationMode::Normal,
            false,
        )?;
        let loc = tasc.loc();
        let expr = self.lower_expr(*tasc.expr)?;
        if is_instance_ascription {
            self.ctx.sub_unify(
                expr.ref_t(),
                &t,
                expr.loc(),
                Some(&Str::from(expr.to_string())),
            )?;
        } else {
            let ctx = self
                .ctx
                .get_singular_ctx_by_hir_expr(&expr, &self.ctx.name)?;
            // REVIEW: need to use subtype_of?
            if ctx.super_traits.iter().all(|trait_| trait_ != &t)
                && ctx.super_classes.iter().all(|class| class != &t)
            {
                return Err(LowerErrors::from(LowerError::subtyping_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    expr.ref_t(), // FIXME:
                    &t,
                    loc,
                    self.ctx.caused_by(),
                )));
            }
        }
        Ok(expr.type_asc(tasc.t_spec))
    }

    // Call.obj == Accessor cannot be type inferred by itself (it can only be inferred with arguments)
    // so turn off type checking (check=false)
    fn lower_expr(&mut self, expr: ast::Expr) -> LowerResult<hir::Expr> {
        log!(info "entered {}", fn_name!());
        match expr {
            ast::Expr::Lit(lit) => Ok(hir::Expr::Lit(self.lower_literal(lit)?)),
            ast::Expr::Array(arr) => Ok(hir::Expr::Array(self.lower_array(arr)?)),
            ast::Expr::Tuple(tup) => Ok(hir::Expr::Tuple(self.lower_tuple(tup)?)),
            ast::Expr::Record(rec) => Ok(hir::Expr::Record(self.lower_record(rec)?)),
            ast::Expr::Set(set) => Ok(hir::Expr::Set(self.lower_set(set)?)),
            ast::Expr::Dict(dict) => Ok(hir::Expr::Dict(self.lower_dict(dict)?)),
            ast::Expr::Accessor(acc) => Ok(hir::Expr::Accessor(self.lower_acc(acc)?)),
            ast::Expr::BinOp(bin) => Ok(hir::Expr::BinOp(self.lower_bin(bin)?)),
            ast::Expr::UnaryOp(unary) => Ok(hir::Expr::UnaryOp(self.lower_unary(unary)?)),
            ast::Expr::Call(call) => Ok(hir::Expr::Call(self.lower_call(call)?)),
            ast::Expr::DataPack(pack) => Ok(hir::Expr::Call(self.lower_pack(pack)?)),
            ast::Expr::Lambda(lambda) => Ok(hir::Expr::Lambda(self.lower_lambda(lambda)?)),
            ast::Expr::Def(def) => Ok(hir::Expr::Def(self.lower_def(def)?)),
            ast::Expr::ClassDef(defs) => Ok(hir::Expr::ClassDef(self.lower_class_def(defs)?)),
            ast::Expr::PatchDef(defs) => Ok(hir::Expr::PatchDef(self.lower_patch_def(defs)?)),
            ast::Expr::AttrDef(adef) => Ok(hir::Expr::AttrDef(self.lower_attr_def(adef)?)),
            ast::Expr::TypeAsc(tasc) => Ok(hir::Expr::TypeAsc(self.lower_type_asc(tasc)?)),
            // Checking is also performed for expressions in Dummy. However, it has no meaning in code generation
            ast::Expr::Dummy(dummy) => Ok(hir::Expr::Dummy(self.lower_dummy(dummy)?)),
            other => todo!("{other}"),
        }
    }

    fn lower_block(&mut self, ast_block: ast::Block) -> LowerResult<hir::Block> {
        log!(info "entered {}", fn_name!());
        let mut hir_block = Vec::with_capacity(ast_block.len());
        for chunk in ast_block.into_iter() {
            let chunk = self.lower_expr(chunk)?;
            hir_block.push(chunk);
        }
        Ok(hir::Block::new(hir_block))
    }

    fn lower_dummy(&mut self, ast_dummy: ast::Dummy) -> LowerResult<hir::Dummy> {
        log!(info "entered {}", fn_name!());
        let mut hir_dummy = Vec::with_capacity(ast_dummy.len());
        for chunk in ast_dummy.into_iter() {
            let chunk = self.lower_expr(chunk)?;
            hir_dummy.push(chunk);
        }
        Ok(hir::Dummy::new(hir_dummy))
    }

    fn return_incomplete_artifact(&mut self, hir: HIR) -> IncompleteArtifact {
        self.ctx.clear_invalid_vars();
        IncompleteArtifact::new(
            Some(hir),
            LowerErrors::from(self.errs.take_all()),
            LowerWarnings::from(self.warns.take_all()),
        )
    }

    pub fn lower(&mut self, ast: AST, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        log!(info "the AST lowering process has started.");
        log!(info "the type-checking process has started.");
        let ast = Reorderer::new(self.cfg.clone())
            .reorder(ast)
            .map_err(|errs| {
                IncompleteArtifact::new(None, errs, LowerWarnings::from(self.warns.take_all()))
            })?;
        if mode == "declare" {
            let hir = self.declare_module(ast);
            if self.errs.is_empty() {
                log!(info "HIR:\n{hir}");
                log!(info "the declaring process has completed.");
                return Ok(CompleteArtifact::new(
                    hir,
                    LowerWarnings::from(self.warns.take_all()),
                ));
            } else {
                log!(err "the declaring process has failed.");
                return Err(self.return_incomplete_artifact(hir));
            }
        }
        let mut module = hir::Module::with_capacity(ast.module.len());
        if let Err(errs) = self.ctx.preregister(ast.module.block()) {
            self.errs.extend(errs);
        }
        for chunk in ast.module.into_iter() {
            match self.lower_expr(chunk) {
                Ok(chunk) => {
                    module.push(chunk);
                }
                Err(errs) => {
                    self.errs.extend(errs);
                }
            }
        }
        self.ctx.clear_invalid_vars();
        self.ctx.check_decls().unwrap_or_else(|mut errs| {
            self.errs.append(&mut errs);
        });
        let hir = HIR::new(ast.name, module);
        log!(info "HIR (not resolved, current errs: {}):\n{hir}", self.errs.len());
        let hir = match self.ctx.resolve(hir) {
            Ok(hir) => {
                log!(info "HIR (resolved):\n{hir}");
                hir
            }
            Err((hir, errs)) => {
                self.errs.extend(errs);
                log!(err "the resolving process has failed. errs:  {}", self.errs.len());
                return Err(self.return_incomplete_artifact(hir));
            }
        };
        // TODO: recursive check
        for chunk in hir.module.iter() {
            if let Err(warns) = self.use_check(chunk, mode) {
                self.warns.extend(warns);
            }
        }
        if self.errs.is_empty() {
            log!(info "the AST lowering process has completed.");
            Ok(CompleteArtifact::new(
                hir,
                LowerWarnings::from(self.warns.take_all()),
            ))
        } else {
            log!(err "the AST lowering process has failed. errs: {}", self.errs.len());
            Err(self.return_incomplete_artifact(hir))
        }
    }
}
