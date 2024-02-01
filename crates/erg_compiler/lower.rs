//! implements `ASTLowerer`.
//!
//! ASTLowerer(ASTからHIRへの変換器)を実装
use std::marker::PhantomData;
use std::mem;

use erg_common::config::{ErgConfig, ErgMode};
use erg_common::consts::{ELS, ERG_MODE, PYTHON_MODE};
use erg_common::dict;
use erg_common::dict::Dict;
use erg_common::error::{Location, MultiErrorDisplay};
use erg_common::fresh::FreshNameGenerator;
use erg_common::pathutil::{mod_name, NormalizedPathBuf};
use erg_common::set;
use erg_common::set::Set;
use erg_common::traits::{ExitStatus, Locational, NoTypeDisplay, Runnable, Stream};
use erg_common::traits::{New, OptionalTranspose};
use erg_common::triple::Triple;
use erg_common::{fmt_option, fn_name, log, switch_lang, Str};

use erg_parser::ast::{self, AscriptionKind, DefId, InlineModule, VisModifierSpec};
use erg_parser::ast::{OperationKind, TypeSpecWithOp, VarName, AST};
use erg_parser::build_ast::{ASTBuildable, ASTBuilder as DefaultASTBuilder};
use erg_parser::desugar::Desugarer;
use erg_parser::token::{Token, TokenKind};
use erg_parser::Parser;

use crate::artifact::{BuildRunnable, Buildable, CompleteArtifact, IncompleteArtifact};
use crate::build_package::CheckStatus;
use crate::module::SharedCompilerResource;
use crate::ty::constructors::{
    array_t, free_var, func, guard, mono, poly, proc, refinement, set_t, singleton, ty_tp,
    unsized_array_t, v_enum,
};
use crate::ty::free::Constraint;
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{
    CastTarget, GuardType, HasType, ParamTy, Predicate, SubrType, Type, VisibilityModifier,
};

use crate::context::{
    ClassDefType, Context, ContextKind, ContextProvider, ControlKind, MethodContext, ModuleContext,
    RegistrationMode, TypeContext,
};
use crate::error::{
    CompileError, CompileErrors, CompileWarning, LowerError, LowerErrors, LowerResult,
    LowerWarning, LowerWarnings, SingleLowerResult,
};
use crate::hir;
use crate::hir::HIR;
use crate::link_ast::ASTLinker;
use crate::varinfo::{VarInfo, VarKind};
use crate::{feature_error, unreachable_error};
use crate::{AccessKind, GenericHIRBuilder};

use VisibilityModifier::*;

pub fn expr_to_cast_target(expr: &ast::Expr) -> CastTarget {
    match expr {
        ast::Expr::Accessor(ast::Accessor::Ident(ident)) => CastTarget::Var {
            name: ident.inspect().clone(),
            loc: ident.loc(),
        },
        _ => CastTarget::expr(expr.clone()),
    }
}

/// Checks & infers types of an AST, and convert (lower) it into a HIR
#[derive(Debug)]
pub struct GenericASTLowerer<ASTBuilder: ASTBuildable = DefaultASTBuilder> {
    pub(crate) cfg: ErgConfig,
    pub(crate) module: ModuleContext,
    pub(crate) errs: LowerErrors,
    pub(crate) warns: LowerWarnings,
    fresh_gen: FreshNameGenerator,
    _parser: PhantomData<fn() -> ASTBuilder>,
}

pub type ASTLowerer = GenericASTLowerer<DefaultASTBuilder>;

impl<A: ASTBuildable> Default for GenericASTLowerer<A> {
    fn default() -> Self {
        Self::new_with_cache(
            ErgConfig::default(),
            Str::ever("<module>"),
            SharedCompilerResource::default(),
        )
    }
}

impl<A: ASTBuildable> New for GenericASTLowerer<A> {
    fn new(cfg: ErgConfig) -> Self {
        Self::new_with_cache(
            cfg.copy(),
            Str::ever("<module>"),
            SharedCompilerResource::new(cfg),
        )
    }
}

impl<ASTBuilder: ASTBuildable> Runnable for GenericASTLowerer<ASTBuilder> {
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

    #[inline]
    fn finish(&mut self) {}

    fn initialize(&mut self) {
        self.module.context.initialize();
        self.errs.clear();
        self.warns.clear();
    }

    fn clear(&mut self) {
        self.errs.clear();
        self.warns.clear();
    }

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let artifact = ast_builder
            .build_ast(self.cfg.input.read())
            .map_err(|artifact| artifact.errors)?;
        artifact.warns.write_all_to(&mut self.cfg.output);
        let artifact = self
            .lower(artifact.ast, "exec")
            .map_err(|artifact| artifact.errors)?;
        artifact.warns.write_all_to(&mut self.cfg.output);
        use std::io::Write;
        write!(self.cfg.output, "{}", artifact.object).unwrap();
        Ok(ExitStatus::compile_passed(artifact.warns.len()))
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let artifact = ast_builder
            .build_ast(src)
            .map_err(|artifact| artifact.errors)?;
        artifact.warns.write_all_stderr();
        let artifact = self
            .lower(artifact.ast, "eval")
            .map_err(|artifact| artifact.errors)?;
        artifact.warns.write_all_stderr();
        Ok(format!("{}", artifact.object))
    }
}

impl<A: ASTBuildable> ContextProvider for GenericASTLowerer<A> {
    fn dir(&self) -> Dict<&VarName, &VarInfo> {
        self.module.context.dir()
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.module.context.get_receiver_ctx(receiver_name)
    }

    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.module.context.get_var_info(name)
    }
}

impl<ASTBuilder: ASTBuildable> Buildable for GenericASTLowerer<ASTBuilder> {
    fn inherit(cfg: ErgConfig, shared: SharedCompilerResource) -> Self {
        let mod_name = Str::from(cfg.input.file_stem());
        Self::new_with_cache(cfg, mod_name, shared)
    }

    fn inherit_with_name(cfg: ErgConfig, mod_name: Str, shared: SharedCompilerResource) -> Self {
        Self::new_with_cache(cfg, mod_name, shared)
    }

    fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<HIR>, IncompleteArtifact<HIR>> {
        let mut ast_builder = ASTBuilder::new(self.cfg.copy());
        let artifact = ast_builder.build_ast(src).map_err(|artifact| {
            IncompleteArtifact::new(None, artifact.errors.into(), artifact.warns.into())
        })?;
        self.lower(artifact.ast, mode)
    }

    fn build_from_ast(
        &mut self,
        ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact<HIR>, IncompleteArtifact<HIR>> {
        self.lower(ast, mode)
    }

    fn pop_context(&mut self) -> Option<ModuleContext> {
        self.pop_mod_ctx()
    }
    fn get_context(&self) -> Option<&ModuleContext> {
        Some(self.get_mod_ctx())
    }
}

impl<A: ASTBuildable + 'static> BuildRunnable for GenericASTLowerer<A> {}

impl<ASTBuilder: ASTBuildable> GenericASTLowerer<ASTBuilder> {
    pub fn new(cfg: ErgConfig) -> Self {
        New::new(cfg)
    }

    pub fn new_with_cache<S: Into<Str>>(
        cfg: ErgConfig,
        mod_name: S,
        shared: SharedCompilerResource,
    ) -> Self {
        let toplevel = Context::new_module(mod_name, cfg.clone(), shared);
        let module = ModuleContext::new(toplevel, dict! {});
        Self {
            module,
            cfg,
            errs: LowerErrors::empty(),
            warns: LowerWarnings::empty(),
            fresh_gen: FreshNameGenerator::new("lower"),
            _parser: PhantomData,
        }
    }

    pub fn new_with_ctx(module: ModuleContext) -> Self {
        Self {
            cfg: module.get_top_cfg(),
            module,
            errs: LowerErrors::empty(),
            warns: LowerWarnings::empty(),
            fresh_gen: FreshNameGenerator::new("lower"),
            _parser: PhantomData,
        }
    }

    fn pop_append_errs(&mut self) {
        let (ctx, errs) = self.module.context.check_decls_and_pop();
        if self.cfg.mode == ErgMode::LanguageServer && !ctx.dir().is_empty() {
            self.module.scope.insert(ctx.name.clone(), ctx);
        }
        self.errs.extend(errs);
    }

    pub fn pop_mod_ctx(&mut self) -> Option<ModuleContext> {
        let opt_module = self.module.context.pop_mod();
        opt_module.map(|module| ModuleContext::new(module, mem::take(&mut self.module.scope)))
    }

    pub fn pop_mod_ctx_or_default(&mut self) -> ModuleContext {
        std::mem::take(&mut self.module)
    }

    pub fn get_mod_ctx(&self) -> &ModuleContext {
        &self.module
    }

    pub fn dir(&self) -> Dict<&VarName, &VarInfo> {
        ContextProvider::dir(self)
    }

    pub fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        ContextProvider::get_receiver_ctx(self, receiver_name)
    }

    pub fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        ContextProvider::get_var_info(self, name)
    }

    pub fn clear(&mut self) {
        Runnable::clear(self);
    }

    pub fn unregister(&mut self, name: &str) -> Option<VarInfo> {
        self.module.context.unregister(name)
    }
}

impl<A: ASTBuildable> GenericASTLowerer<A> {
    pub(crate) fn lower_literal(
        &mut self,
        lit: ast::Literal,
        expect: Option<&Type>,
    ) -> LowerResult<hir::Literal> {
        let loc = lit.loc();
        let lit = hir::Literal::try_from(lit.token).map_err(|_| {
            LowerError::invalid_literal(
                self.cfg.input.clone(),
                line!() as usize,
                loc,
                self.module.context.caused_by(),
            )
        })?;
        if let Some(expect) = expect {
            if let Err(_errs) = self.module.context.sub_unify(&lit.t(), expect, &loc, None) {
                // self.errs.extend(errs);
            }
        }
        Ok(lit)
    }

    fn lower_array(&mut self, array: ast::Array, expect: Option<&Type>) -> LowerResult<hir::Array> {
        log!(info "entered {}({array})", fn_name!());
        match array {
            ast::Array::Normal(arr) => {
                Ok(hir::Array::Normal(self.lower_normal_array(arr, expect)?))
            }
            ast::Array::WithLength(arr) => Ok(hir::Array::WithLength(
                self.lower_array_with_length(arr, expect)?,
            )),
            other => feature_error!(
                LowerErrors,
                LowerError,
                self.module.context,
                other.loc(),
                "array comprehension"
            ),
        }
    }

    fn elem_err(&self, l: &Type, r: &Type, elem: &hir::Expr) -> LowerErrors {
        let elem_disp_notype = elem.to_string_notype();
        let l = self.module.context.readable_type(l.clone());
        let r = self.module.context.readable_type(r.clone());
        LowerErrors::from(LowerError::syntax_error(
            self.cfg.input.clone(),
            line!() as usize,
            elem.loc(),
            String::from(&self.module.context.name[..]),
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

    fn lower_normal_array(
        &mut self,
        array: ast::NormalArray,
        expect: Option<&Type>,
    ) -> LowerResult<hir::NormalArray> {
        log!(info "entered {}({array})", fn_name!());
        let mut new_array = vec![];
        let eval_result = self.module.context.eval_const_normal_array(&array);
        let (elems, ..) = array.elems.deconstruct();
        let expect_elem = expect.and_then(|t| {
            // REVIEW: are these all?
            if !(t.is_array() || t.is_array_mut() || t.is_iterable()) {
                return None;
            }
            self.module
                .context
                .convert_tp_into_type(t.typarams().first()?.clone())
                .ok()
        });
        let mut union = Type::Never;
        for elem in elems.into_iter() {
            let elem = self.lower_expr(elem.expr, expect_elem.as_ref())?;
            let union_ = self.module.context.union(&union, elem.ref_t());
            self.homogeneity_check(expect_elem.as_ref(), &union_, &union, &elem)?;
            union = union_;
            new_array.push(elem);
        }
        let elem_t = if union == Type::Never {
            free_var(
                self.module.context.level,
                Constraint::new_type_of(Type::Type),
            )
        } else {
            union
        };
        let elems = hir::Args::values(new_array, None);
        let t = array_t(elem_t, TyParam::value(elems.len()));
        let t = if let Ok(value) = eval_result {
            singleton(t, TyParam::Value(value))
        } else {
            t
        };
        Ok(hir::NormalArray::new(array.l_sqbr, array.r_sqbr, t, elems))
    }

    fn homogeneity_check(
        &self,
        expect_elem: Option<&Type>,
        union_: &Type,
        union: &Type,
        elem: &hir::Expr,
    ) -> LowerResult<()> {
        if ERG_MODE && expect_elem.is_none() {
            if let Some((l, r)) = union_.union_pair() {
                match (l.is_unbound_var(), r.is_unbound_var()) {
                    // e.g. [1, "a"]
                    (false, false) => {
                        if let hir::Expr::TypeAsc(type_asc) = elem {
                            // e.g. [1, "a": Str or NoneType]
                            if !self
                                .module
                                .context
                                .supertype_of(&type_asc.spec.spec_t, union)
                            {
                                return Err(self.elem_err(&l, &r, elem));
                            } // else(OK): e.g. [1, "a": Str or Int]
                        }
                        // OK: ?T(:> {"a"}) or ?U(:> {"b"}) or {"c", "d"} => {"a", "b", "c", "d"} <: Str
                        else if self
                            .module
                            .context
                            .coerce(union_.clone(), &())
                            .map_or(true, |coerced| coerced.union_pair().is_some())
                        {
                            return Err(self.elem_err(&l, &r, elem));
                        }
                    }
                    // TODO: check if the type is compatible with the other type
                    (true, false) => {}
                    (false, true) => {}
                    (true, true) => {}
                }
            }
        }
        Ok(())
    }

    fn lower_array_with_length(
        &mut self,
        array: ast::ArrayWithLength,
        expect: Option<&Type>,
    ) -> LowerResult<hir::ArrayWithLength> {
        log!(info "entered {}({array})", fn_name!());
        let expect_elem = expect.and_then(|t| {
            if !(t.is_array() || t.is_array_mut() || t.is_iterable()) {
                return None;
            }
            self.module
                .context
                .convert_tp_into_type(t.typarams().first()?.clone())
                .ok()
        });
        let elem = self.lower_expr(array.elem.expr, expect_elem.as_ref())?;
        let array_t = self.gen_array_with_length_type(&elem, &array.len);
        let len = match *array.len {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) if ident.is_discarded() => None,
            len => Some(self.lower_expr(len, Some(&Type::Nat))?),
        };
        let hir_array = hir::ArrayWithLength::new(array.l_sqbr, array.r_sqbr, array_t, elem, len);
        Ok(hir_array)
    }

    fn gen_array_with_length_type(&self, elem: &hir::Expr, len: &ast::Expr) -> Type {
        match len {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) if ident.is_discarded() => {
                return unsized_array_t(elem.t());
            }
            _ => {}
        }
        let maybe_len = self.module.context.eval_const_expr(len);
        match maybe_len {
            Ok(v @ ValueObj::Nat(_)) => array_t(elem.t(), TyParam::Value(v)),
            Ok(other) => todo!("{other} is not a Nat object"),
            Err(err) => todo!("{err}"),
        }
    }

    fn lower_tuple(&mut self, tuple: ast::Tuple, expect: Option<&Type>) -> LowerResult<hir::Tuple> {
        log!(info "entered {}({tuple})", fn_name!());
        match tuple {
            ast::Tuple::Normal(tup) => {
                Ok(hir::Tuple::Normal(self.lower_normal_tuple(tup, expect)?))
            }
        }
    }

    fn lower_normal_tuple(
        &mut self,
        tuple: ast::NormalTuple,
        expect: Option<&Type>,
    ) -> LowerResult<hir::NormalTuple> {
        log!(info "entered {}({tuple})", fn_name!());
        let expect = expect.and_then(|exp| {
            if !exp.is_tuple() {
                return None;
            }
            self.module
                .context
                .convert_type_to_tuple_type(exp.clone())
                .ok()
        });
        let mut new_tuple = vec![];
        let (elems, .., paren) = tuple.elems.deconstruct();
        let expect_ts = expect.transpose(elems.len());
        for (elem, expect) in elems.into_iter().zip(expect_ts) {
            let elem = self.lower_expr(elem.expr, expect.as_ref())?;
            new_tuple.push(elem);
        }
        Ok(hir::NormalTuple::new(hir::Args::values(new_tuple, paren)))
    }

    fn lower_record(
        &mut self,
        record: ast::Record,
        expect: Option<&Type>,
    ) -> LowerResult<hir::Record> {
        log!(info "entered {}({record})", fn_name!());
        match record {
            ast::Record::Normal(rec) => self.lower_normal_record(rec, expect),
            ast::Record::Mixed(mixed) => {
                self.lower_normal_record(Desugarer::desugar_shortened_record_inner(mixed), None)
            }
        }
    }

    fn lower_normal_record(
        &mut self,
        record: ast::NormalRecord,
        _expect: Option<&Type>,
    ) -> LowerResult<hir::Record> {
        log!(info "entered {}({record})", fn_name!());
        let mut hir_record =
            hir::Record::new(record.l_brace, record.r_brace, hir::RecordAttrs::empty());
        self.module
            .context
            .grow("<record>", ContextKind::Dummy, Private, None);
        for attr in record.attrs.iter() {
            if attr.sig.is_const() {
                self.module
                    .context
                    .register_const_def(attr)
                    .map_err(|errs| {
                        self.pop_append_errs();
                        errs
                    })?;
            }
        }
        for attr in record.attrs.into_iter() {
            let attr = self.lower_def(attr).map_err(|errs| {
                self.pop_append_errs();
                errs
            })?;
            hir_record.push(attr);
        }
        self.pop_append_errs();
        Ok(hir_record)
    }

    fn lower_set(&mut self, set: ast::Set, expect: Option<&Type>) -> LowerResult<hir::Set> {
        log!(info "enter {}({set})", fn_name!());
        match set {
            ast::Set::Normal(set) => Ok(hir::Set::Normal(self.lower_normal_set(set, expect)?)),
            ast::Set::WithLength(set) => Ok(hir::Set::WithLength(
                self.lower_set_with_length(set, expect)?,
            )),
            ast::Set::Comprehension(set) => feature_error!(
                LowerErrors,
                LowerError,
                self.module.context,
                set.loc(),
                "set comprehension"
            ),
        }
    }

    fn lower_normal_set(
        &mut self,
        set: ast::NormalSet,
        expect: Option<&Type>,
    ) -> LowerResult<hir::NormalSet> {
        log!(info "entered {}({set})", fn_name!());
        let (elems, ..) = set.elems.deconstruct();
        let expect_elem = expect.and_then(|t| {
            if !t.is_set() {
                return None;
            }
            self.module
                .context
                .convert_tp_into_type(t.typarams().first()?.clone())
                .ok()
        });
        let mut union = Type::Never;
        let mut new_set = vec![];
        for elem in elems {
            let elem = self.lower_expr(elem.expr, expect_elem.as_ref())?;
            union = self.module.context.union(&union, elem.ref_t());
            if ERG_MODE && union.is_union_type() {
                return Err(LowerErrors::from(LowerError::syntax_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    elem.loc(),
                    String::from(&self.module.context.name[..]),
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
            free_var(
                self.module.context.level,
                Constraint::new_type_of(Type::Type),
            )
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
        // check if elem_t is Eq and Hash
        let eq_hash = mono("Eq") & mono("Hash");
        if let Err(errs) = self
            .module
            .context
            .sub_unify(&elem_t, &eq_hash, &elems, None)
        {
            self.errs.extend(errs);
        }
        Ok(hir::NormalSet::new(set.l_brace, set.r_brace, elem_t, elems))
    }

    /// This (e.g. {"a"; 3}) is meaningless as an object, but makes sense as a type (e.g. {Int; 3}).
    fn lower_set_with_length(
        &mut self,
        set: ast::SetWithLength,
        expect: Option<&Type>,
    ) -> LowerResult<hir::SetWithLength> {
        log!("entered {}({set})", fn_name!());
        let expect_elem = expect.and_then(|t| {
            if !t.is_set() {
                return None;
            }
            self.module
                .context
                .convert_tp_into_type(t.typarams().first()?.clone())
                .ok()
        });
        let elem = self.lower_expr(set.elem.expr, expect_elem.as_ref())?;
        let set_t = self.gen_set_with_length_type(&elem, &set.len);
        let len = self.lower_expr(*set.len, Some(&Type::Nat))?;
        let hir_set = hir::SetWithLength::new(set.l_brace, set.r_brace, set_t, elem, len);
        Ok(hir_set)
    }

    fn gen_set_with_length_type(&mut self, elem: &hir::Expr, len: &ast::Expr) -> Type {
        let maybe_len = self.module.context.eval_const_expr(len);
        match maybe_len {
            Ok(v @ ValueObj::Nat(_)) => {
                if self.module.context.subtype_of(&elem.t(), &Type::Type) {
                    poly("SetType", vec![TyParam::t(elem.t()), TyParam::Value(v)])
                } else {
                    set_t(elem.t(), TyParam::Value(v))
                }
            }
            Ok(other) => todo!("{other} is not a Nat object"),
            Err(_e) => set_t(elem.t(), TyParam::erased(Type::Nat)),
        }
    }

    fn lower_dict(&mut self, dict: ast::Dict, expect: Option<&Type>) -> LowerResult<hir::Dict> {
        log!(info "enter {}({dict})", fn_name!());
        match dict {
            ast::Dict::Normal(set) => Ok(hir::Dict::Normal(self.lower_normal_dict(set, expect)?)),
            other => feature_error!(
                LowerErrors,
                LowerError,
                self.module.context,
                other.loc(),
                "dict comprehension"
            ),
            // ast::Dict::WithLength(set) => Ok(hir::Dict::WithLength(self.lower_dict_with_length(set, expect)?)),
        }
    }

    fn lower_normal_dict(
        &mut self,
        dict: ast::NormalDict,
        expect: Option<&Type>,
    ) -> LowerResult<hir::NormalDict> {
        log!(info "enter {}({dict})", fn_name!());
        let mut union = dict! {};
        let mut new_kvs = vec![];
        let expect = expect
            .and_then(|exp| {
                if !(exp.is_dict() || exp.is_dict_mut()) {
                    return None;
                }
                self.module
                    .context
                    .convert_type_to_dict_type(exp.clone())
                    .ok()
            })
            .and_then(|dict| (dict.len() == 1).then_some(dict));
        for kv in dict.kvs.into_iter() {
            let expect_key = expect.as_ref().map(|dict| dict.keys().next().unwrap());
            let expect_value = expect.as_ref().map(|dict| dict.values().next().unwrap());
            let key = self.lower_expr(kv.key, expect_key)?;
            let value = self.lower_expr(kv.value, expect_value)?;
            if let Some(popped_val_t) = union.insert(key.t(), value.t()) {
                if PYTHON_MODE {
                    let val_t = union.get_mut(key.ref_t()).unwrap();
                    *val_t = self.module.context.union(&mem::take(val_t), &popped_val_t);
                } else {
                    return Err(LowerErrors::from(LowerError::syntax_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        Location::concat(&key, &value),
                        String::from(&self.module.context.name[..]),
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
            }
            new_kvs.push(hir::KeyValue::new(key, value));
        }
        for key_t in union.keys() {
            let loc = &(&dict.l_brace, &dict.r_brace);
            // check if key_t is Eq and Hash
            let eq_hash = mono("Eq") & mono("Hash");
            if let Err(errs) = self.module.context.sub_unify(key_t, &eq_hash, loc, None) {
                self.errs.extend(errs);
            }
        }
        let kv_ts = if union.is_empty() {
            dict! {
                ty_tp(free_var(self.module.context.level, Constraint::new_type_of(Type::Type))) =>
                    ty_tp(free_var(self.module.context.level, Constraint::new_type_of(Type::Type)))
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

    pub(crate) fn lower_acc(
        &mut self,
        acc: ast::Accessor,
        expect: Option<&Type>,
    ) -> LowerResult<hir::Accessor> {
        log!(info "entered {}({acc})", fn_name!());
        match acc {
            ast::Accessor::Ident(ident) => {
                let ident = self.lower_ident(ident, expect);
                let acc = hir::Accessor::Ident(ident);
                Ok(acc)
            }
            ast::Accessor::Attr(attr) => {
                let obj = self.lower_expr(*attr.obj, None)?;
                let vi = match self.module.context.get_attr_info(
                    &obj,
                    &attr.ident,
                    &self.cfg.input,
                    &self.module.context,
                    expect,
                ) {
                    Triple::Ok(vi) => {
                        self.inc_ref(attr.ident.inspect(), &vi, &attr.ident.name);
                        vi
                    }
                    Triple::Err(errs) => {
                        self.errs.push(errs);
                        VarInfo::ILLEGAL
                    }
                    Triple::None => {
                        let self_t = obj.t();
                        let (similar_info, similar_name) = self
                            .module
                            .context
                            .get_similar_attr_and_info(&self_t, attr.ident.inspect())
                            .unzip();
                        let err = LowerError::detailed_no_attr_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            attr.ident.loc(),
                            self.module.context.caused_by(),
                            &self_t,
                            attr.ident.inspect(),
                            similar_name,
                            similar_info,
                        );
                        self.errs.push(err);
                        VarInfo::ILLEGAL
                    }
                };
                if let Some(expect) = expect {
                    if let Err(_errs) =
                        self.module
                            .context
                            .sub_unify(&vi.t, expect, &attr.ident.loc(), None)
                    {
                        // self.errs.extend(errs);
                    }
                }
                let ident = hir::Identifier::new(attr.ident, None, vi);
                let acc = hir::Accessor::Attr(hir::Attribute::new(obj, ident));
                Ok(acc)
            }
            ast::Accessor::TypeApp(t_app) => feature_error!(
                LowerErrors,
                LowerError,
                self.module.context,
                t_app.loc(),
                "type application"
            ),
            // TupleAttr, Subscr are desugared
            _ => unreachable_error!(LowerErrors, LowerError, self.module.context),
        }
    }

    fn lower_ident(&mut self, ident: ast::Identifier, expect: Option<&Type>) -> hir::Identifier {
        // `match` is a special form, typing is magic
        let (vi, __name__) = if ident.vis.is_private()
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
            let res = match self.module.context.rec_get_var_info(
                &ident,
                AccessKind::Name,
                &self.cfg.input,
                &self.module.context,
            ) {
                Triple::Ok(vi) => vi,
                Triple::Err(err) => {
                    self.errs.push(err);
                    VarInfo::ILLEGAL
                }
                Triple::None => {
                    let (similar_info, similar_name) = self
                        .module
                        .context
                        .get_similar_name_and_info(ident.inspect())
                        .unzip();
                    let err = LowerError::detailed_no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        ident.loc(),
                        self.module.context.caused_by(),
                        ident.inspect(),
                        similar_name,
                        similar_info,
                    );
                    self.errs.push(err);
                    VarInfo::ILLEGAL
                }
            };
            (
                res,
                self.module
                    .context
                    .get_singular_ctxs_by_ident(&ident, &self.module.context)
                    .ok()
                    .map(|ctx| ctx.first().unwrap().name.clone()),
            )
        };
        self.inc_ref(ident.inspect(), &vi, &ident.name);
        if let Some(expect) = expect {
            if let Err(_errs) = self
                .module
                .context
                .sub_unify(&vi.t, expect, &ident.loc(), None)
            {
                // self.errs.extend(errs);
            }
        }
        let ident = hir::Identifier::new(ident, __name__, vi);
        if !ident.vi.is_toplevel()
            && ident.vi.def_namespace() != &self.module.context.name
            && ident.vi.kind.can_capture()
            && self.module.context.current_true_function_ctx().is_some()
        {
            self.module.context.captured_names.push(ident.clone());
        }
        ident
    }

    fn get_guard_type(&self, expr: &ast::Expr) -> Option<Type> {
        match expr {
            ast::Expr::BinOp(bin) => {
                let lhs = &bin.args[0];
                let rhs = &bin.args[1];
                self.get_bin_guard_type(&bin.op, lhs, rhs)
            }
            ast::Expr::Call(call) => self.get_call_guard_type(call),
            _ => None,
        }
    }

    fn get_call_guard_type(&self, call: &ast::Call) -> Option<Type> {
        if let (ast::Expr::Accessor(ast::Accessor::Ident(ident)), None, Some(lhs), Some(rhs)) = (
            call.obj.as_ref(),
            &call.attr_name,
            &call.args.nth_or_key(0, "object"),
            &call.args.nth_or_key(1, "classinfo"),
        ) {
            self.get_bin_guard_type(ident.name.token(), lhs, rhs)
        } else {
            None
        }
    }

    fn get_bin_guard_type(&self, op: &Token, lhs: &ast::Expr, rhs: &ast::Expr) -> Option<Type> {
        match op.kind {
            TokenKind::AndOp => {
                let lhs = self.get_guard_type(lhs)?;
                let rhs = self.get_guard_type(rhs)?;
                return Some(self.module.context.intersection(&lhs, &rhs));
            }
            TokenKind::OrOp => {
                let lhs = self.get_guard_type(lhs)?;
                let rhs = self.get_guard_type(rhs)?;
                return Some(self.module.context.union(&lhs, &rhs));
            }
            _ => {}
        }
        let target = if op.kind == TokenKind::ContainsOp {
            expr_to_cast_target(rhs)
        } else {
            expr_to_cast_target(lhs)
        };
        let namespace = self.module.context.name.clone();
        match op.kind {
            // l in T -> T contains l
            TokenKind::ContainsOp => {
                let to = self.module.context.expr_to_type(lhs.clone())?;
                Some(guard(namespace, target, to))
            }
            TokenKind::Symbol if &op.content[..] == "isinstance" => {
                let to = self.module.context.expr_to_type(rhs.clone())?;
                Some(guard(namespace, target, to))
            }
            TokenKind::IsOp | TokenKind::DblEq => {
                let rhs = self.module.context.expr_to_value(rhs.clone())?;
                Some(guard(namespace, target, v_enum(set! { rhs })))
            }
            TokenKind::IsNotOp | TokenKind::NotEq => {
                let rhs = self.module.context.expr_to_value(rhs.clone())?;
                let ty = guard(namespace, target, v_enum(set! { rhs }));
                Some(self.module.context.complement(&ty))
            }
            TokenKind::Gre => {
                let rhs = self.module.context.expr_to_tp(rhs.clone())?;
                let t = self.module.context.get_tp_t(&rhs).unwrap_or(Type::Obj);
                let varname = self.fresh_gen.fresh_varname();
                let pred = Predicate::gt(varname.clone(), rhs);
                let refine = refinement(varname, t, pred);
                Some(guard(namespace, target, refine))
            }
            TokenKind::GreEq => {
                let rhs = self.module.context.expr_to_tp(rhs.clone())?;
                let t = self.module.context.get_tp_t(&rhs).unwrap_or(Type::Obj);
                let varname = self.fresh_gen.fresh_varname();
                let pred = Predicate::ge(varname.clone(), rhs);
                let refine = refinement(varname, t, pred);
                Some(guard(namespace, target, refine))
            }
            TokenKind::Less => {
                let rhs = self.module.context.expr_to_tp(rhs.clone())?;
                let t = self.module.context.get_tp_t(&rhs).unwrap_or(Type::Obj);
                let varname = self.fresh_gen.fresh_varname();
                let pred = Predicate::lt(varname.clone(), rhs);
                let refine = refinement(varname, t, pred);
                Some(guard(namespace, target, refine))
            }
            TokenKind::LessEq => {
                let rhs = self.module.context.expr_to_tp(rhs.clone())?;
                let t = self.module.context.get_tp_t(&rhs).unwrap_or(Type::Obj);
                let varname = self.fresh_gen.fresh_varname();
                let pred = Predicate::le(varname.clone(), rhs);
                let refine = refinement(varname, t, pred);
                Some(guard(namespace, target, refine))
            }
            _ => None,
        }
    }

    fn lower_bin(&mut self, bin: ast::BinOp, expect: Option<&Type>) -> hir::BinOp {
        log!(info "entered {}({bin})", fn_name!());
        let mut args = bin.args.into_iter();
        let lhs = *args.next().unwrap();
        let rhs = *args.next().unwrap();
        let guard = self.get_bin_guard_type(&bin.op, &lhs, &rhs);
        let lhs = self.lower_expr(lhs, None).unwrap_or_else(|errs| {
            self.errs.extend(errs);
            hir::Expr::Dummy(hir::Dummy::new(vec![]))
        });
        let lhs = hir::PosArg::new(lhs);
        let rhs = self.lower_expr(rhs, None).unwrap_or_else(|errs| {
            self.errs.extend(errs);
            hir::Expr::Dummy(hir::Dummy::new(vec![]))
        });
        let rhs = hir::PosArg::new(rhs);
        let args = [lhs, rhs];
        let mut vi = self
            .module
            .context
            .get_binop_t(&bin.op, &args, &self.cfg.input, &self.module.context)
            .unwrap_or_else(|errs| {
                self.errs.extend(errs);
                VarInfo::ILLEGAL
            });
        if let Some(guard) = guard {
            if let Some(return_t) = vi.t.mut_return_t() {
                debug_assert!(
                    self.module.context.subtype_of(return_t, &Type::Bool),
                    "{return_t} is not a subtype of Bool"
                );
                *return_t = guard;
            } else if let Some(mut return_t) = vi.t.tyvar_mut_return_t() {
                debug_assert!(
                    self.module.context.subtype_of(&return_t, &Type::Bool),
                    "{return_t} is not a subtype of Bool"
                );
                *return_t = guard;
            }
        }
        if let Some(expect) = expect {
            if let Err(_errs) =
                self.module
                    .context
                    .sub_unify(vi.t.return_t().unwrap(), expect, &args, None)
            {
                // self.errs.extend(errs);
            }
        }
        let mut args = args.into_iter();
        let lhs = args.next().unwrap().expr;
        let rhs = args.next().unwrap().expr;
        hir::BinOp::new(bin.op, lhs, rhs, vi)
    }

    fn lower_unary(&mut self, unary: ast::UnaryOp, expect: Option<&Type>) -> hir::UnaryOp {
        log!(info "entered {}({unary})", fn_name!());
        let mut args = unary.args.into_iter();
        let arg = self
            .lower_expr(*args.next().unwrap(), None)
            .unwrap_or_else(|errs| {
                self.errs.extend(errs);
                hir::Expr::Dummy(hir::Dummy::new(vec![]))
            });
        let args = [hir::PosArg::new(arg)];
        let vi = self
            .module
            .context
            .get_unaryop_t(&unary.op, &args, &self.cfg.input, &self.module.context)
            .unwrap_or_else(|errs| {
                self.errs.extend(errs);
                VarInfo::ILLEGAL
            });
        if let Some(expect) = expect {
            if let Err(_errs) =
                self.module
                    .context
                    .sub_unify(vi.t.return_t().unwrap(), expect, &args, None)
            {
                // self.errs.extend(errs);
            }
        }
        let mut args = args.into_iter();
        let expr = args.next().unwrap().expr;
        hir::UnaryOp::new(unary.op, expr, vi)
    }

    fn lower_args(
        &mut self,
        args: ast::Args,
        expect: Option<&SubrType>,
        errs: &mut LowerErrors,
    ) -> hir::Args {
        let (pos_args, var_args, kw_args, kw_var, paren) = args.deconstruct();
        let mut hir_args = hir::Args::new(
            Vec::with_capacity(pos_args.len()),
            None,
            Vec::with_capacity(kw_args.len()),
            None,
            paren,
        );
        let pos_params = expect
            .as_ref()
            .map(|subr| subr.pos_params().map(|p| Some(p.typ())))
            .map_or(vec![None; pos_args.len()], |params| {
                params.take(pos_args.len()).collect()
            });
        let mut pos_args = pos_args
            .into_iter()
            .zip(pos_params)
            .enumerate()
            .collect::<Vec<_>>();
        // `if` may perform assert casting, so don't sort args
        if self
            .module
            .context
            .current_control_flow()
            .map_or(true, |kind| !kind.is_if())
            && expect.is_some_and(|subr| !subr.essential_qnames().is_empty())
        {
            pos_args
                .sort_by(|(_, (l, _)), (_, (r, _))| l.expr.complexity().cmp(&r.expr.complexity()));
        }
        let mut hir_pos_args =
            vec![hir::PosArg::new(hir::Expr::Dummy(hir::Dummy::empty())); pos_args.len()];
        for (nth, (arg, param)) in pos_args {
            match self.lower_expr(arg.expr, param) {
                Ok(expr) => {
                    if let Some(kind) = self.module.context.control_kind() {
                        self.push_guard(nth, kind, expr.ref_t());
                    }
                    hir_pos_args[nth] = hir::PosArg::new(expr);
                }
                Err(es) => {
                    errs.extend(es);
                }
            }
        }
        hir_args.pos_args = hir_pos_args;
        // TODO: expect var_args
        if let Some(var_args) = var_args {
            match self.lower_expr(var_args.expr, None) {
                Ok(expr) => hir_args.var_args = Some(Box::new(hir::PosArg::new(expr))),
                Err(es) => {
                    errs.extend(es);
                    let dummy = hir::Expr::Dummy(hir::Dummy::empty());
                    hir_args.var_args = Some(Box::new(hir::PosArg::new(dummy)));
                }
            }
        }
        for arg in kw_args.into_iter() {
            let kw_param = expect.as_ref().and_then(|subr| {
                subr.non_var_params()
                    .find(|pt| pt.name().is_some_and(|n| n == &arg.keyword.content))
                    .map(|pt| pt.typ())
            });
            match self.lower_expr(arg.expr, kw_param) {
                Ok(expr) => hir_args.push_kw(hir::KwArg::new(arg.keyword, expr)),
                Err(es) => {
                    errs.extend(es);
                    hir_args.push_kw(hir::KwArg::new(
                        arg.keyword,
                        hir::Expr::Dummy(hir::Dummy::empty()),
                    ));
                }
            }
        }
        if let Some(kw_var) = kw_var {
            match self.lower_expr(kw_var.expr, None) {
                Ok(expr) => hir_args.kw_var = Some(Box::new(hir::PosArg::new(expr))),
                Err(es) => {
                    errs.extend(es);
                    let dummy = hir::Expr::Dummy(hir::Dummy::empty());
                    hir_args.kw_var = Some(Box::new(hir::PosArg::new(dummy)));
                }
            }
        }
        hir_args
    }

    /// ```erg
    /// x: Int or NoneType
    /// if x != None:
    ///     do: ... # x: Int (x != None)
    ///     do: ... # x: NoneType (complement(x != None))
    /// ```
    fn push_guard(&mut self, nth: usize, kind: ControlKind, t: &Type) {
        match t {
            Type::Guard(guard) => match nth {
                0 if kind.is_conditional() => {
                    self.replace_or_push_guard(guard.clone());
                }
                1 if kind.is_if() => {
                    let guard = GuardType::new(
                        guard.namespace.clone(),
                        guard.target.clone(),
                        self.module.context.complement(&guard.to),
                    );
                    self.replace_or_push_guard(guard);
                }
                _ => {}
            },
            Type::And(lhs, rhs) => {
                self.push_guard(nth, kind, lhs);
                self.push_guard(nth, kind, rhs);
            }
            _ => {}
        }
    }

    fn replace_or_push_guard(&mut self, guard: GuardType) {
        if let Some(idx) = self.module.context.guards.iter().position(|existing| {
            existing.namespace == guard.namespace
                && existing.target == guard.target
                && self.module.context.supertype_of(&existing.to, &guard.to)
        }) {
            self.module.context.guards[idx] = guard;
        } else {
            self.module.context.guards.push(guard);
        }
    }

    /// returning `Ok(call)` does not mean the call is valid, just means it is syntactically valid
    /// `ASTLowerer` is designed to cause as little information loss in HIR as possible
    pub(crate) fn lower_call(&mut self, call: ast::Call, expect: Option<&Type>) -> hir::Call {
        log!(info "entered {}({}{}(...))", fn_name!(), call.obj, fmt_option!(call.attr_name));
        let pushed = if let (Some(name), None) = (call.obj.get_name(), &call.attr_name) {
            self.module.context.higher_order_caller.push(name.clone());
            true
        } else {
            false
        };
        let mut errs = LowerErrors::empty();
        let guard = self.get_call_guard_type(&call);
        let mut obj = match self.lower_expr(*call.obj, None) {
            Ok(obj) => obj,
            Err(es) => {
                if pushed {
                    self.module.context.higher_order_caller.pop();
                }
                errs.extend(es);
                hir::Expr::Dummy(hir::Dummy::empty())
            }
        };
        let opt_vi = self.module.context.get_call_t_without_args(
            &obj,
            &call.attr_name,
            &self.cfg.input,
            expect,
            &self.module.context,
        );
        let expect_subr = opt_vi
            .as_ref()
            .ok()
            .and_then(|vi| <&SubrType>::try_from(&vi.t).ok());
        if let Some((subr_return_t, expect)) =
            expect_subr.map(|subr| subr.return_t.as_ref()).zip(expect)
        {
            if let Err(_errs) = self
                .module
                .context
                .sub_unify(subr_return_t, expect, &(), None)
            {
                // self.errs.extend(errs);
            }
        }
        let hir_args = self.lower_args(call.args, expect_subr, &mut errs);
        let mut vi = match self.module.context.get_call_t(
            &obj,
            &call.attr_name,
            &hir_args.pos_args,
            &hir_args.kw_args,
            &self.cfg.input,
            &self.module.context,
        ) {
            Ok(vi) => vi,
            Err((vi, es)) => {
                if pushed {
                    self.module.context.higher_order_caller.pop();
                }
                errs.extend(es);
                vi.unwrap_or(VarInfo::ILLEGAL)
            }
        };
        if let Err(es) = self.module.context.propagate(&mut vi.t, &obj) {
            errs.extend(es);
        }
        if let Some(guard) = guard {
            debug_assert!(
                self.module
                    .context
                    .subtype_of(vi.t.return_t().unwrap(), &Type::Bool),
                "{} is not a subtype of Bool",
                vi.t.return_t().unwrap()
            );
            if let Some(ret_t) = vi.t.mut_return_t() {
                *ret_t = guard;
            } else if let Some(mut ref_t) = vi.t.tyvar_mut_return_t() {
                *ref_t = guard;
            }
        }
        let attr_name = if let Some(attr_name) = call.attr_name {
            self.inc_ref(attr_name.inspect(), &vi, &attr_name.name);
            Some(hir::Identifier::new(attr_name, None, vi))
        } else {
            if let hir::Expr::Call(call) = &obj {
                if call.return_t().is_some() {
                    *obj.ref_mut_t().unwrap() = vi.t;
                }
            } else {
                *obj.ref_mut_t().unwrap() = vi.t;
            }
            None
        };
        let mut call = hir::Call::new(obj, attr_name, hir_args);
        if let Some((found, expect)) = call
            .signature_t()
            .and_then(|sig| sig.return_t())
            .zip(expect)
        {
            if let Err(_errs) = self.module.context.sub_unify(found, expect, &call, None) {
                // self.errs.extend(errs);
            }
        }
        if pushed {
            self.module.context.higher_order_caller.pop();
        }
        if errs.is_empty() {
            if let Err(es) = self.exec_additional_op(&mut call) {
                errs.extend(es);
            }
        }
        self.errs.extend(errs);
        call
    }

    /// importing is done in [preregister](https://github.com/erg-lang/erg/blob/ffd33015d540ff5a0b853b28c01370e46e0fcc52/crates/erg_compiler/context/register.rs#L819)
    fn exec_additional_op(&mut self, call: &mut hir::Call) -> LowerResult<()> {
        match call.additional_operation() {
            Some(OperationKind::Del) => match call.args.get_left_or_key("obj").unwrap() {
                hir::Expr::Accessor(hir::Accessor::Ident(ident)) => {
                    self.module.context.del(ident)?;
                    Ok(())
                }
                other => {
                    return Err(LowerErrors::from(LowerError::syntax_error(
                        self.input().clone(),
                        line!() as usize,
                        other.loc(),
                        self.module.context.caused_by(),
                        format!("expected identifier, but found {}", other.name()),
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
                    other => {
                        log!(err "todo: {other}");
                        return unreachable_error!(LowerErrors, LowerError, self.module.context);
                    }
                };
                let arg_t = call.args.get(0).unwrap().ref_t();
                self.module.context.sub_unify(arg_t, &ret_t, call, None)?;
                Ok(())
            }
            Some(OperationKind::Assert) => {
                if let Some(Type::Guard(guard)) =
                    call.args.get_left_or_key("test").map(|exp| exp.ref_t())
                {
                    self.module.context.cast(guard.clone(), &mut vec![])?;
                }
                Ok(())
            }
            Some(OperationKind::Cast) => {
                self.warns.push(LowerWarning::use_cast_warning(
                    self.input().clone(),
                    line!() as usize,
                    call.loc(),
                    self.module.context.caused_by(),
                ));
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn lower_pack(
        &mut self,
        pack: ast::DataPack,
        _expect: Option<&Type>,
    ) -> LowerResult<hir::Call> {
        log!(info "entered {}({pack})", fn_name!());
        let class = self.lower_expr(*pack.class, None)?;
        let args = self.lower_record(pack.args, None)?;
        let args = vec![hir::PosArg::new(hir::Expr::Record(args))];
        let attr_name = ast::Identifier::new(
            VisModifierSpec::Public(Token::new(
                TokenKind::Dot,
                Str::ever("."),
                pack.connector.ln_begin().unwrap(),
                pack.connector.col_begin().unwrap(),
            )),
            ast::VarName::new(Token::new(
                TokenKind::Symbol,
                Str::ever("new"),
                pack.connector.ln_begin().unwrap(),
                pack.connector.col_begin().unwrap(),
            )),
        );
        let vi = match self.module.context.get_call_t(
            &class,
            &Some(attr_name.clone()),
            &args,
            &[],
            &self.cfg.input,
            &self.module.context,
        ) {
            Ok(vi) => vi,
            Err((vi, errs)) => {
                self.errs.extend(errs);
                vi.unwrap_or(VarInfo::ILLEGAL)
            }
        };
        let args = hir::Args::pos_only(args, None);
        let attr_name = hir::Identifier::new(attr_name, None, vi);
        Ok(hir::Call::new(class, Some(attr_name), args))
    }

    fn lower_non_default_param(
        &mut self,
        non_default: ast::NonDefaultParamSignature,
    ) -> LowerResult<hir::NonDefaultParamSignature> {
        let t_spec_as_expr = non_default
            .t_spec
            .as_ref()
            .map(|t_spec_op| self.fake_lower_expr(*t_spec_op.t_spec_as_expr.clone()))
            .transpose()?;
        // TODO: define here (not assign_params)
        let vi = VarInfo::default();
        let sig = hir::NonDefaultParamSignature::new(non_default, vi, t_spec_as_expr);
        Ok(sig)
    }

    fn lower_type_spec_with_op(
        &mut self,
        type_spec_with_op: ast::TypeSpecWithOp,
        spec_t: Type,
    ) -> LowerResult<hir::TypeSpecWithOp> {
        let expr = self.fake_lower_expr(*type_spec_with_op.t_spec_as_expr.clone())?;
        Ok(hir::TypeSpecWithOp::new(type_spec_with_op, expr, spec_t))
    }

    /// lower and assign params
    fn lower_params(
        &mut self,
        params: ast::Params,
        bounds: ast::TypeBoundSpecs,
        expect: Option<&SubrType>,
    ) -> LowerResult<hir::Params> {
        log!(info "entered {}({})", fn_name!(), params);
        let mut tmp_tv_ctx = self
            .module
            .context
            .instantiate_ty_bounds(&bounds, RegistrationMode::Normal)?;
        let mut errs = LowerErrors::empty();
        let mut hir_non_defaults = vec![];
        for non_default in params.non_defaults.into_iter() {
            match self.lower_non_default_param(non_default) {
                Ok(sig) => hir_non_defaults.push(sig),
                Err(es) => errs.extend(es),
            }
        }
        let hir_var_params = match params.var_params {
            Some(var_params) => match self.lower_non_default_param(*var_params) {
                Ok(sig) => Some(Box::new(sig)),
                Err(es) => {
                    errs.extend(es);
                    None
                }
            },
            None => None,
        };
        let mut hir_defaults = vec![];
        for default in params.defaults.into_iter() {
            match self.lower_expr(default.default_val, None) {
                Ok(default_val) => {
                    let sig = self.lower_non_default_param(default.sig)?;
                    hir_defaults.push(hir::DefaultParamSignature::new(sig, default_val));
                }
                Err(es) => errs.extend(es),
            }
        }
        let hir_kw_var_params = match params.kw_var_params {
            Some(kw_var_params) => match self.lower_non_default_param(*kw_var_params) {
                Ok(sig) => Some(Box::new(sig)),
                Err(es) => {
                    errs.extend(es);
                    None
                }
            },
            None => None,
        };
        let mut hir_params = hir::Params::new(
            hir_non_defaults,
            hir_var_params,
            hir_defaults,
            hir_kw_var_params,
            vec![],
            params.parens,
        );
        if let Err(errs) =
            self.module
                .context
                .assign_params(&mut hir_params, &mut tmp_tv_ctx, expect.cloned())
        {
            self.errs.extend(errs);
        }
        for guard in params.guards.into_iter() {
            let lowered = match guard {
                ast::GuardClause::Bind(bind) => self.lower_def(bind).map(hir::GuardClause::Bind),
                // TODO: no fake
                ast::GuardClause::Condition(cond) => {
                    self.fake_lower_expr(cond).map(hir::GuardClause::Condition)
                }
            };
            match lowered {
                Ok(guard) => {
                    if hir_params.guards.contains(&guard) {
                        log!(err "duplicate guard: {guard}");
                        continue;
                    }
                    hir_params.guards.push(guard)
                }
                Err(es) => self.errs.extend(es),
            }
        }
        if !errs.is_empty() {
            Err(errs)
        } else {
            Ok(hir_params)
        }
    }

    fn lower_lambda(
        &mut self,
        lambda: ast::Lambda,
        expect: Option<&Type>,
    ) -> LowerResult<hir::Lambda> {
        let expect = expect.and_then(|t| <&SubrType>::try_from(t).ok());
        let return_t = expect.map(|subr| subr.return_t.as_ref());
        log!(info "entered {}({lambda})", fn_name!());
        let in_statement = PYTHON_MODE
            && self
                .module
                .context
                .control_kind()
                .map_or(false, |k| k.makes_scope());
        let is_procedural = lambda.is_procedural();
        let id = lambda.id.0;
        let name = format!("<lambda_{id}>");
        let kind = if is_procedural {
            ContextKind::LambdaProc(self.module.context.control_kind())
        } else {
            ContextKind::LambdaFunc(self.module.context.control_kind())
        };
        let tv_cache = self
            .module
            .context
            .instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal)?;
        if !in_statement {
            self.module
                .context
                .grow(&name, kind, Private, Some(tv_cache));
        }
        let params = self
            .lower_params(lambda.sig.params, lambda.sig.bounds, expect)
            .map_err(|errs| {
                if !in_statement {
                    self.pop_append_errs();
                }
                errs
            })?;
        let overwritten = {
            let mut overwritten = vec![];
            let guards = if in_statement {
                mem::take(&mut self.module.context.guards)
            } else {
                mem::take(&mut self.module.context.get_mut_outer().unwrap().guards)
            };
            for guard in guards.into_iter() {
                if let Err(errs) = self.module.context.cast(guard, &mut overwritten) {
                    self.errs.extend(errs);
                }
            }
            overwritten
        };
        if let Err(errs) = self.module.context.register_const(&lambda.body) {
            self.errs.extend(errs);
        }
        let body = self.lower_block(lambda.body, return_t).map_err(|errs| {
            if !in_statement {
                self.pop_append_errs();
            }
            errs
        })?;
        if in_statement {
            for (var, vi) in overwritten.into_iter() {
                if vi.kind.is_parameter() {
                    // removed from `locals` and remains in `params`
                    self.module.context.locals.remove(&var);
                } else {
                    self.module.context.locals.insert(var, vi);
                }
            }
        }
        // suppress warns of lambda types, e.g. `(x: Int, y: Int) -> Int`
        if self.module.context.subtype_of(body.ref_t(), &Type::Type) {
            for param in params.non_defaults.iter() {
                self.inc_ref(param.inspect().unwrap_or(&"_".into()), &param.vi, param);
            }
            if let Some(var_param) = params.var_params.as_deref() {
                self.inc_ref(
                    var_param.inspect().unwrap_or(&"_".into()),
                    &var_param.vi,
                    var_param,
                );
            }
            for default in params.defaults.iter() {
                self.inc_ref(
                    default.sig.inspect().unwrap_or(&"_".into()),
                    &default.sig.vi,
                    &default.sig,
                );
            }
            if let Some(kw_var_param) = params.kw_var_params.as_deref() {
                self.inc_ref(
                    kw_var_param.inspect().unwrap_or(&"_".into()),
                    &kw_var_param.vi,
                    kw_var_param,
                );
            }
        }
        let (non_default_params, default_params): (Vec<_>, Vec<_>) = self
            .module
            .context
            .params
            .iter()
            .partition(|(_, vi)| !vi.kind.has_default());
        #[cfg(not(feature = "py_compat"))]
        let (var_params, non_default_params) = {
            let (var_params, non_default_params): (Vec<_>, Vec<_>) = non_default_params
                .into_iter()
                .partition(|(_, vi)| vi.kind.is_var_params());
            // vi.t: `[T; _]`
            // pt: `name: T`
            let var_params = var_params.first().map(|(name, vi)| {
                ParamTy::pos_or_kw(
                    name.as_ref().map(|n| n.inspect().clone()),
                    vi.t.inner_ts().remove(0),
                )
            });
            (var_params, non_default_params.into_iter())
        };
        #[cfg(feature = "py_compat")]
        let (var_params, non_default_params) = {
            let (var_params, non_default_params): (Vec<_>, Vec<_>) = non_default_params
                .into_iter()
                .partition(|(_, vi)| vi.kind.is_var_params());
            let var_params = var_params.get(0).map(|(name, vi)| {
                ParamTy::pos_or_kw(
                    name.as_ref().map(|n| n.inspect().clone()),
                    vi.t.inner_ts().remove(0),
                )
            });
            let non_default_params = non_default_params.into_iter().filter(|(name, _)| {
                params
                    .non_defaults
                    .iter()
                    .any(|nd| nd.name() == name.as_ref())
            });
            (var_params, non_default_params)
        };
        let non_default_param_tys = non_default_params
            .map(|(name, vi)| {
                ParamTy::pos_or_kw(name.as_ref().map(|n| n.inspect().clone()), vi.t.clone())
            })
            .collect();
        #[cfg(not(feature = "py_compat"))]
        let default_params = default_params.into_iter();
        #[cfg(feature = "py_compat")]
        let default_params = default_params
            .into_iter()
            .filter(|(name, _)| params.defaults.iter().any(|d| d.name() == name.as_ref()));
        let default_param_tys = default_params
            .map(|(name, vi)| ParamTy::kw(name.as_ref().unwrap().inspect().clone(), vi.t.clone()))
            .collect();
        let captured_names = mem::take(&mut self.module.context.captured_names);
        if in_statement {
            // For example, `i` in `for i in ...` is a parameter,
            // but should be treated as a local variable in the later analysis, so move it to locals
            for nd_param in params.non_defaults.iter() {
                if let Some(idx) = self
                    .module
                    .context
                    .params
                    .iter()
                    .position(|(name, _)| name.as_ref() == nd_param.name())
                {
                    let (name, vi) = self.module.context.params.remove(idx);
                    if let Some(name) = name {
                        self.module.context.locals.insert(name, vi);
                    }
                }
            }
        } else {
            self.pop_append_errs();
        }
        let ty = if is_procedural {
            proc(
                non_default_param_tys,
                var_params,
                default_param_tys,
                None,
                body.t(),
            )
        } else {
            func(
                non_default_param_tys,
                var_params,
                default_param_tys,
                None,
                body.t(),
            )
        };
        let t = if ty.has_unbound_var() {
            // TODO:
            // use crate::ty::free::HasLevel;
            // ty.lift();
            // self.module.context.generalize_t(ty)
            ty.quantify()
        } else {
            ty
        };
        let return_t_spec = lambda.sig.return_t_spec.map(|t_spec| t_spec.t_spec);
        Ok(hir::Lambda::new(
            id,
            params,
            lambda.op,
            return_t_spec,
            captured_names,
            body,
            t,
        ))
    }

    fn lower_def(&mut self, def: ast::Def) -> LowerResult<hir::Def> {
        log!(info "entered {}({})", fn_name!(), def.sig);
        if def.def_kind().is_class_or_trait() && self.module.context.kind != ContextKind::Module {
            self.module
                .context
                .decls
                .remove(def.sig.ident().unwrap().inspect());
            return Err(LowerErrors::from(LowerError::inner_typedef_error(
                self.cfg.input.clone(),
                line!() as usize,
                def.loc(),
                self.module.context.caused_by(),
            )));
        }
        let name = if let Some(name) = def.sig.name_as_str() {
            name.clone()
        } else {
            Str::ever("<lambda>")
        };
        if ERG_MODE && (&name[..] == "module" || &name[..] == "global") {
            return Err(LowerErrors::from(
                LowerError::shadow_special_namespace_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    def.sig.loc(),
                    self.module.context.caused_by(),
                    &name,
                ),
            ));
        } else if self
            .module
            .context
            .registered_info(&name, def.sig.is_const())
            .is_some()
            && def.sig.vis().is_private()
        {
            return Err(LowerErrors::from(LowerError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                def.sig.loc(),
                self.module.context.caused_by(),
                &name,
            )));
        } else if self
            .module
            .context
            .get_builtins()
            .and_then(|ctx| ctx.get_var_info(&name))
            .is_some()
            && def.sig.vis().is_private()
        {
            self.warns.push(LowerWarning::builtin_exists_warning(
                self.cfg.input.clone(),
                line!() as usize,
                def.sig.loc(),
                self.module.context.caused_by(),
                &name,
            ));
        }
        let kind = ContextKind::from(&def);
        let vis = self
            .module
            .context
            .instantiate_vis_modifier(def.sig.vis())?;
        let res = match def.sig {
            ast::Signature::Subr(sig) => {
                let tv_cache = self
                    .module
                    .context
                    .instantiate_ty_bounds(&sig.bounds, RegistrationMode::Normal)?;
                self.module.context.grow(&name, kind, vis, Some(tv_cache));
                self.lower_subr_def(sig, def.body)
            }
            ast::Signature::Var(sig) => {
                self.module.context.grow(&name, kind, vis, None);
                self.lower_var_def(sig, def.body)
            }
        };
        // TODO: Context上の関数に型境界情報を追加
        self.pop_append_errs();
        // remove from decls regardless of success or failure to lower
        self.module.context.decls.remove(&name);
        res
    }

    fn lower_var_def(
        &mut self,
        sig: ast::VarSignature,
        body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        log!(info "entered {}({sig})", fn_name!());
        if let Err(errs) = self.module.context.register_const(&body.block) {
            self.errs.extend(errs);
        }
        let outer = self.module.context.outer.as_ref().unwrap();
        let expect_body_t = sig
            .t_spec
            .as_ref()
            .and_then(|t_spec| {
                self.module
                    .context
                    .instantiate_var_sig_t(Some(&t_spec.t_spec), RegistrationMode::PreRegister)
                    .ok()
            })
            .or_else(|| {
                sig.ident()
                    .and_then(|ident| outer.get_current_scope_var(&ident.name))
                    .map(|vi| vi.t.clone())
            });
        match self.lower_block(body.block, expect_body_t.as_ref()) {
            Ok(block) => {
                let found_body_t = block.ref_t();
                let ident = match &sig.pat {
                    ast::VarPattern::Ident(ident) => ident.clone(),
                    ast::VarPattern::Discard(token) => {
                        ast::Identifier::private_from_token(token.clone())
                    }
                    _ => unreachable!(),
                };
                if let Some(expect_body_t) = expect_body_t {
                    // TODO: expect_body_t is smaller for constants
                    // TODO: 定数の場合、expect_body_tのほうが小さくなってしまう
                    if !sig.is_const() {
                        if let Err(e) = self.var_result_t_check(
                            &sig,
                            ident.inspect(),
                            &expect_body_t,
                            found_body_t,
                        ) {
                            self.errs.push(e);
                        }
                    }
                }
                let vi = self.module.context.outer.as_mut().unwrap().assign_var_sig(
                    &sig,
                    found_body_t,
                    body.id,
                    block.last(),
                    None,
                )?;
                let ident = hir::Identifier::new(ident, None, vi);
                let t_spec = if let Some(ts) = sig.t_spec {
                    let spec_t = self.module.context.instantiate_typespec(&ts.t_spec)?;
                    let expr = self.fake_lower_expr(*ts.t_spec_as_expr.clone())?;
                    Some(hir::TypeSpecWithOp::new(ts, expr, spec_t))
                } else {
                    None
                };
                let sig = hir::VarSignature::new(ident, t_spec);
                let body = hir::DefBody::new(body.op, block, body.id);
                Ok(hir::Def::new(hir::Signature::Var(sig), body))
            }
            Err(errs) => {
                self.module.context.outer.as_mut().unwrap().assign_var_sig(
                    &sig,
                    &Type::Failure,
                    ast::DefId(0),
                    None,
                    None,
                )?;
                Err(errs)
            }
        }
    }

    // NOTE: Note that this is in the inner scope while being called.
    fn lower_subr_def(
        &mut self,
        sig: ast::SubrSignature,
        body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        log!(info "entered {}({sig})", fn_name!());
        let registered_t = self
            .module
            .context
            .outer
            .as_ref()
            .unwrap()
            .get_current_scope_var(&sig.ident.name)
            .map(|vi| vi.t.clone())
            .unwrap_or(Type::Failure);
        let mut decorators = set! {};
        for deco in sig.decorators.iter() {
            // exclude comptime decorators
            if self.module.context.eval_const_expr(&deco.0).is_ok() {
                continue;
            }
            let deco = match self.lower_expr(deco.0.clone(), Some(&mono("GenericCallable"))) {
                Ok(deco) => deco,
                Err(es) => {
                    self.errs.extend(es);
                    continue;
                }
            };
            decorators.insert(deco);
        }
        match registered_t {
            Type::Subr(subr_t) => self.lower_subr_block(subr_t, sig, decorators, body),
            quant @ Type::Quantified(_) => {
                let instance = self.module.context.instantiate_dummy(quant)?;
                let subr_t = SubrType::try_from(instance).unwrap();
                self.lower_subr_block(subr_t, sig, decorators, body)
            }
            Type::Failure => {
                let params = self.lower_params(sig.params, sig.bounds.clone(), None)?;
                if let Err(errs) = self.module.context.register_const(&body.block) {
                    self.errs.extend(errs);
                }
                self.module
                    .context
                    .outer
                    .as_mut()
                    .unwrap()
                    .fake_subr_assign(&sig.ident, &sig.decorators, Type::Failure)?;
                let block = self.lower_block(body.block, None)?;
                let ident = hir::Identifier::bare(sig.ident);
                let ret_t_spec = if let Some(ts) = sig.return_t_spec {
                    let spec_t = self.module.context.instantiate_typespec(&ts.t_spec)?;
                    let expr = self.fake_lower_expr(*ts.t_spec_as_expr.clone())?;
                    Some(hir::TypeSpecWithOp::new(ts, expr, spec_t))
                } else {
                    None
                };
                let captured_names = mem::take(&mut self.module.context.captured_names);
                let sig = hir::SubrSignature::new(
                    decorators,
                    ident,
                    sig.bounds,
                    params,
                    ret_t_spec,
                    captured_names,
                );
                let body = hir::DefBody::new(body.op, block, body.id);
                Ok(hir::Def::new(hir::Signature::Subr(sig), body))
            }
            _ => unreachable_error!(LowerErrors, LowerError, self),
        }
    }

    fn lower_subr_block(
        &mut self,
        registered_subr_t: SubrType,
        sig: ast::SubrSignature,
        decorators: Set<hir::Expr>,
        body: ast::DefBody,
    ) -> LowerResult<hir::Def> {
        let params = self.lower_params(
            sig.params.clone(),
            sig.bounds.clone(),
            Some(&registered_subr_t),
        )?;
        if let Err(errs) = self.module.context.register_const(&body.block) {
            self.errs.extend(errs);
        }
        let return_t = registered_subr_t
            .return_t
            .has_no_unbound_var()
            .then_some(registered_subr_t.return_t.as_ref());
        match self.lower_block(body.block, return_t) {
            Ok(block) => {
                let found_body_t = self.module.context.squash_tyvar(block.t());
                let vi = match self.module.context.outer.as_mut().unwrap().assign_subr(
                    &sig,
                    body.id,
                    &params,
                    &found_body_t,
                    block.last().unwrap(),
                ) {
                    Ok(vi) => vi,
                    Err((errs, vi)) => {
                        self.errs.extend(errs);
                        vi
                    }
                };
                let ident = hir::Identifier::new(sig.ident, None, vi);
                let ret_t_spec = if let Some(ts) = sig.return_t_spec {
                    let spec_t = self.module.context.instantiate_typespec(&ts.t_spec)?;
                    let expr = self.fake_lower_expr(*ts.t_spec_as_expr.clone())?;
                    Some(hir::TypeSpecWithOp::new(ts, expr, spec_t))
                } else {
                    None
                };
                let captured_names = mem::take(&mut self.module.context.captured_names);
                let sig = hir::SubrSignature::new(
                    decorators,
                    ident,
                    sig.bounds,
                    params,
                    ret_t_spec,
                    captured_names,
                );
                let body = hir::DefBody::new(body.op, block, body.id);
                Ok(hir::Def::new(hir::Signature::Subr(sig), body))
            }
            Err(errs) => {
                self.errs.extend(errs);
                let vi = match self.module.context.outer.as_mut().unwrap().assign_subr(
                    &sig,
                    ast::DefId(0),
                    &params,
                    &Type::Failure,
                    &sig,
                ) {
                    Ok(vi) => vi,
                    Err((errs, vi)) => {
                        self.errs.extend(errs);
                        vi
                    }
                };
                let ident = hir::Identifier::new(sig.ident, None, vi);
                let ret_t_spec = if let Some(ts) = sig.return_t_spec {
                    let spec_t = self.module.context.instantiate_typespec(&ts.t_spec)?;
                    let expr = self.fake_lower_expr(*ts.t_spec_as_expr.clone())?;
                    Some(hir::TypeSpecWithOp::new(ts, expr, spec_t))
                } else {
                    None
                };
                let captured_names = mem::take(&mut self.module.context.captured_names);
                let sig = hir::SubrSignature::new(
                    decorators,
                    ident,
                    sig.bounds,
                    params,
                    ret_t_spec,
                    captured_names,
                );
                let block = hir::Block::new(vec![hir::Expr::Dummy(hir::Dummy::new(vec![]))]);
                let body = hir::DefBody::new(body.op, block, body.id);
                Ok(hir::Def::new(hir::Signature::Subr(sig), body))
            }
        }
    }

    fn lower_class_def(&mut self, class_def: ast::ClassDef) -> LowerResult<hir::ClassDef> {
        log!(info "entered {}({class_def})", fn_name!());
        let mut hir_def = self.lower_def(class_def.def)?;
        let mut hir_methods_list = vec![];
        for methods in class_def.methods_list.into_iter() {
            let mut hir_methods = hir::Block::empty();
            let (class, impl_trait) = self
                .module
                .context
                .get_class_and_impl_trait(&methods.class)?;
            if let Some(class_root) = self.module.context.get_nominal_type_ctx(&class) {
                if !class_root.kind.is_class() {
                    return Err(LowerErrors::from(LowerError::method_definition_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        methods.loc(),
                        self.module.context.caused_by(),
                        &class.qual_name(),
                        None,
                    )));
                }
            } else {
                return Err(LowerErrors::from(LowerError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    methods.class.loc(),
                    self.module.context.caused_by(),
                    &class.qual_name(),
                    self.module.context.get_similar_name(&class.local_name()),
                )));
            }
            let methods_list = &mut self
                .module
                .context
                .get_mut_nominal_type_ctx(&class)
                .unwrap()
                .methods_list;
            let methods_idx = methods_list.iter().position(|m| m.id == methods.id);
            let methods_ctx = methods_idx
                .map(|idx| methods_list.remove(idx))
                .unwrap_or_else(|| todo!());
            self.module.context.replace(methods_ctx.ctx);
            for attr in methods.attrs.iter() {
                match attr {
                    ast::ClassAttr::Def(def) => {
                        if let Some(ident) = def.sig.ident() {
                            if self
                                .module
                                .context
                                .get_instance_attr(ident.inspect())
                                .is_some()
                            {
                                self.warns
                                    .push(CompileWarning::same_name_instance_attr_warning(
                                        self.cfg.input.clone(),
                                        line!() as usize,
                                        ident.loc(),
                                        self.module.context.caused_by(),
                                        ident.inspect(),
                                    ));
                            }
                        }
                    }
                    ast::ClassAttr::Decl(_) | ast::ClassAttr::Doc(_) => {}
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
                    ast::ClassAttr::Decl(decl) => match self.lower_type_asc(decl, None) {
                        Ok(decl) => {
                            hir_methods.push(hir::Expr::TypeAsc(decl));
                        }
                        Err(errs) => {
                            self.errs.extend(errs);
                        }
                    },
                    ast::ClassAttr::Doc(doc) => match self.lower_literal(doc, None) {
                        Ok(doc) => {
                            hir_methods.push(hir::Expr::Literal(doc));
                        }
                        Err(errs) => {
                            self.errs.extend(errs);
                        }
                    },
                }
            }
            if let Err(errs) = self.module.context.check_decls() {
                self.errs.extend(errs);
            }
            if let Some((trait_, _)) = &impl_trait {
                self.check_override(&class, Some(trait_));
            } else {
                self.check_override(&class, None);
            }
            if let Err(err) = self.check_trait_impl(impl_trait.clone(), &class) {
                self.errs.push(err);
            }
            let impl_trait = impl_trait.map(|(t, _)| t);
            self.check_collision_and_push(methods.id, class.clone(), impl_trait.clone());
            hir_methods_list.push(hir::Methods::new(class, impl_trait, hir_methods));
        }
        let class = self.module.context.gen_type(&hir_def.sig.ident().raw);
        let Some(class_ctx) = self.module.context.get_nominal_type_ctx(&class) else {
            return Err(LowerErrors::from(LowerError::type_not_found(
                self.cfg.input.clone(),
                line!() as usize,
                hir_def.sig.loc(),
                self.module.context.caused_by(),
                &class,
            )));
        };
        let Some(class_type) = self
            .module
            .context
            .rec_get_const_obj(hir_def.sig.ident().inspect())
        else {
            return unreachable_error!(LowerErrors, LowerError, self);
        };
        let ValueObj::Type(TypeObj::Generated(type_obj)) = class_type else {
            return unreachable_error!(LowerErrors, LowerError, self);
        };
        let Some(hir::Expr::Call(call)) = hir_def.body.block.first() else {
            return unreachable_error!(LowerErrors, LowerError, self);
        };
        if let Some(sup_type) = call.args.get_left_or_key("Super") {
            if let Err(err) = self.check_inheritable(type_obj, sup_type, &hir_def.sig) {
                self.errs.extend(err);
            }
        }
        let Some(__new__) = class_ctx
            .get_current_scope_var(&VarName::from_static("__new__"))
            .or(class_ctx.get_current_scope_var(&VarName::from_static("__call__")))
        else {
            return unreachable_error!(LowerErrors, LowerError, self);
        };
        let need_to_gen_new = class_ctx
            .get_current_scope_var(&VarName::from_static("new"))
            .map_or(false, |vi| vi.kind == VarKind::Auto);
        let require_or_sup = Self::get_require_or_sup_or_base(hir_def.body.block.remove(0));
        Ok(hir::ClassDef::new(
            type_obj.clone(),
            hir_def.sig,
            require_or_sup,
            need_to_gen_new,
            __new__.t.clone(),
            hir_methods_list,
        ))
    }

    fn lower_patch_def(&mut self, class_def: ast::PatchDef) -> LowerResult<hir::PatchDef> {
        log!(info "entered {}({class_def})", fn_name!());
        let base_t = {
            let Some(ast::Expr::Call(call)) = class_def.def.body.block.get(0) else {
                return unreachable_error!(LowerErrors, LowerError, self);
            };
            let base_t_expr = call.args.get_left_or_key("Base").unwrap();
            let spec = Parser::expr_to_type_spec(base_t_expr.clone()).unwrap();
            self.module.context.instantiate_typespec(&spec)?
        };
        let mut hir_def = self.lower_def(class_def.def)?;
        let base = Self::get_require_or_sup_or_base(hir_def.body.block.remove(0)).unwrap();
        let mut hir_methods = hir::Block::empty();
        for mut methods in class_def.methods_list.into_iter() {
            let kind = ContextKind::PatchMethodDefs(base_t.clone());
            self.module.context.grow(
                hir_def.sig.ident().inspect(),
                kind,
                hir_def.sig.vis().clone(),
                None,
            );
            for attr in methods.attrs.iter_mut() {
                match attr {
                    ast::ClassAttr::Def(def) => {
                        if methods.vis.is_public() {
                            def.sig.ident_mut().unwrap().vis = VisModifierSpec::Public(Token::new(
                                TokenKind::Dot,
                                ".",
                                def.sig.ln_begin().unwrap(),
                                def.sig.col_begin().unwrap(),
                            ));
                        }
                        self.module
                            .context
                            .register_const_def(def)
                            .map_err(|errs| {
                                self.pop_append_errs();
                                errs
                            })?;
                    }
                    ast::ClassAttr::Decl(_) | ast::ClassAttr::Doc(_) => {}
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
                    ast::ClassAttr::Decl(decl) => match self.lower_type_asc(decl, None) {
                        Ok(decl) => {
                            hir_methods.push(hir::Expr::TypeAsc(decl));
                        }
                        Err(errs) => {
                            self.errs.extend(errs);
                        }
                    },
                    ast::ClassAttr::Doc(doc) => match self.lower_literal(doc, None) {
                        Ok(doc) => {
                            hir_methods.push(hir::Expr::Literal(doc));
                        }
                        Err(errs) => {
                            self.errs.extend(errs);
                        }
                    },
                }
            }
            if let Err(errs) = self.module.context.check_decls() {
                self.errs.extend(errs);
            }
            self.push_patch(methods.id);
        }
        Ok(hir::PatchDef::new(hir_def.sig, base, hir_methods))
    }

    fn lower_redef(&mut self, redef: ast::ReDef) -> LowerResult<hir::ReDef> {
        log!(info "entered {}({redef})", fn_name!());
        let mut attr = self.lower_acc(redef.attr, None)?;
        let expr = self.lower_expr(*redef.expr, None)?;
        if let Err(err) =
            self.var_result_t_check(&attr, &Str::from(attr.show()), attr.ref_t(), expr.ref_t())
        {
            if PYTHON_MODE {
                let derefined = attr.ref_t().derefine();
                match self.var_result_t_check(
                    &attr,
                    &Str::from(attr.show()),
                    &derefined,
                    expr.ref_t(),
                ) {
                    Err(err) => self.errs.push(err),
                    Ok(_) => {
                        *attr.ref_mut_t().unwrap() = derefined.clone();
                        if let hir::Accessor::Ident(ident) = &attr {
                            if let Some(vi) = self
                                .module
                                .context
                                .rec_get_mut_var_info(&ident.raw, AccessKind::Name)
                            {
                                vi.t = derefined;
                            }
                        }
                    }
                }
            } else {
                self.errs.push(err);
            }
        }
        Ok(hir::ReDef::new(attr, hir::Block::new(vec![expr])))
    }

    fn check_inheritable(
        &self,
        type_obj: &GenTypeObj,
        sup_class: &hir::Expr,
        sub_sig: &hir::Signature,
    ) -> LowerResult<()> {
        if let Some(TypeObj::Generated(gen)) = type_obj.base_or_sup() {
            if let Some(ctx) = self.module.context.get_nominal_type_ctx(gen.typ()) {
                for super_trait in ctx.super_traits.iter() {
                    if self
                        .module
                        .context
                        .subtype_of(super_trait, &mono("InheritableType"))
                    {
                        return Ok(());
                    }
                }
            }
            if let Some(impls) = gen.impls() {
                if impls.contains_intersec(&mono("InheritableType")) {
                    return Ok(());
                }
            }
            return Err(LowerError::inheritance_error(
                self.cfg.input.clone(),
                line!() as usize,
                sup_class.to_string(),
                sup_class.loc(),
                sub_sig.ident().inspect().into(),
            )
            .into());
        }
        Ok(())
    }

    fn check_override(&mut self, class: &Type, impl_trait: Option<&Type>) {
        if let Some(sups) = self.module.context.get_nominal_super_type_ctxs(class) {
            // exclude the first one because it is the class itself
            for sup in sups.into_iter().skip(1) {
                for (method_name, vi) in self.module.context.locals.iter().chain(
                    self.module
                        .context
                        .methods_list
                        .iter()
                        .flat_map(|c| c.locals.iter()),
                ) {
                    if let Some(sup_vi) = sup.get_current_scope_var(method_name) {
                        // must `@Override`
                        if let Some(decos) = &vi.comptime_decos {
                            if decos.contains("Override") {
                                continue;
                            }
                        }
                        if sup_vi.impl_of() != impl_trait {
                            continue;
                        }
                        self.errs.push(LowerError::override_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            method_name.inspect(),
                            method_name.loc(),
                            &mono(&sup.name), // TODO: get super type
                            self.module.context.caused_by(),
                        ));
                    }
                }
            }
        }
    }

    /// Inspect the Trait implementation for correctness,
    /// i.e., check that all required attributes are defined and that no extra attributes are defined
    fn check_trait_impl(
        &mut self, //: methods context
        impl_trait: Option<(Type, &TypeSpecWithOp)>,
        class: &Type,
    ) -> SingleLowerResult<()> {
        if let Some((impl_trait, t_spec)) = impl_trait {
            let impl_trait = impl_trait.normalize();
            let (unverified_names, mut errors) = if let Some(typ_ctx) = self
                .module
                .context
                .get_outer_scope()
                .unwrap()
                .get_nominal_type_ctx(&impl_trait)
            {
                self.check_methods_compatibility(&impl_trait, class, typ_ctx, t_spec)
            } else {
                return Err(LowerError::no_type_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    t_spec.loc(),
                    self.module.context.caused_by(),
                    &impl_trait.qual_name(),
                    self.module
                        .context
                        .get_similar_name(&impl_trait.local_name()),
                ));
            };
            for unverified in unverified_names {
                errors.push(LowerError::not_in_trait_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    self.module.context.caused_by(),
                    unverified.inspect(),
                    &impl_trait,
                    class,
                    None,
                    unverified.loc(),
                ));
            }
            self.errs.extend(errors);
        }
        Ok(())
    }

    fn check_methods_compatibility(
        &self,
        impl_trait: &Type,
        class: &Type,
        TypeContext {
            typ: trait_type,
            ctx: trait_ctx,
        }: &TypeContext,
        t_spec: &TypeSpecWithOp,
    ) -> (Set<&VarName>, CompileErrors) {
        let mut errors = CompileErrors::empty();
        let mut unverified_names = self.module.context.locals.keys().collect::<Set<_>>();
        for (decl_name, decl_vi) in trait_ctx.decls.iter() {
            if let Some((name, vi)) = self.module.context.get_var_kv(decl_name.inspect()) {
                let def_t = &vi.t;
                let replaced_decl_t = decl_vi
                    .t
                    .clone()
                    .replace(trait_type, impl_trait)
                    .replace(impl_trait, class);
                unverified_names.remove(name);
                if !self.module.context.supertype_of(&replaced_decl_t, def_t) {
                    let hint = self
                        .module
                        .context
                        .get_simple_type_mismatch_hint(&replaced_decl_t, def_t);
                    errors.push(LowerError::trait_member_type_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        name.loc(),
                        self.module.context.caused_by(),
                        name.inspect(),
                        impl_trait,
                        &decl_vi.t,
                        &vi.t,
                        hint,
                    ));
                }
            } else {
                errors.push(LowerError::trait_member_not_defined_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    self.module.context.caused_by(),
                    decl_name.inspect(),
                    impl_trait,
                    class,
                    None,
                    t_spec.loc(),
                ));
            }
        }
        (unverified_names, errors)
    }

    fn check_collision_and_push(&mut self, id: DefId, class: Type, impl_trait: Option<Type>) {
        let methods = self.module.context.pop();
        self.module.context.register_methods(&class, &methods);
        let Some(class_root) = self.module.context.get_mut_nominal_type_ctx(&class) else {
            log!(err "{class} not found");
            return;
        };
        for (newly_defined_name, vi) in methods.locals.clone().into_iter() {
            for already_defined_methods in class_root.methods_list.iter_mut() {
                // TODO: 特殊化なら同じ名前でもOK
                // TODO: 定義のメソッドもエラー表示
                if let Some((_already_defined_name, already_defined_vi)) =
                    already_defined_methods.get_var_kv(newly_defined_name.inspect())
                {
                    if already_defined_vi.kind != VarKind::Auto
                        && already_defined_vi.impl_of() == vi.impl_of()
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
        let typ = if let Some(impl_trait) = impl_trait {
            ClassDefType::impl_trait(class, impl_trait)
        } else {
            ClassDefType::Simple(class)
        };
        class_root
            .methods_list
            .push(MethodContext::new(id, typ, methods));
    }

    fn push_patch(&mut self, id: DefId) {
        let methods = self.module.context.pop();
        let ContextKind::PatchMethodDefs(base) = &methods.kind else {
            unreachable!()
        };
        let patch_name = *methods.name.split_with(&["::", "."]).last().unwrap();
        let patch_root = self
            .module
            .context
            .patches
            .get_mut(patch_name)
            .unwrap_or_else(|| todo!("{} not found", methods.name));
        for (newly_defined_name, vi) in methods.locals.clone().into_iter() {
            for already_defined_methods in patch_root.methods_list.iter_mut() {
                // TODO: 特殊化なら同じ名前でもOK
                // TODO: 定義のメソッドもエラー表示
                if let Some((_already_defined_name, already_defined_vi)) =
                    already_defined_methods.get_var_kv(newly_defined_name.inspect())
                {
                    if already_defined_vi.kind != VarKind::Auto
                        && already_defined_vi.impl_of() == vi.impl_of()
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
        patch_root.methods_list.push(MethodContext::new(
            id,
            ClassDefType::Simple(base.clone()),
            methods,
        ));
    }

    fn get_require_or_sup_or_base(expr: hir::Expr) -> Option<hir::Expr> {
        match expr {
            acc @ hir::Expr::Accessor(_) => Some(acc),
            hir::Expr::Call(mut call) => match call.obj.show_acc().as_ref().map(|s| &s[..]) {
                Some("Class" | "Trait") => call.args.remove_left_or_key("Requirement"),
                Some("Inherit") => call.args.remove_left_or_key("Super"),
                Some("Inheritable") => {
                    Self::get_require_or_sup_or_base(call.args.remove_left_or_key("Class").unwrap())
                }
                Some("Structural") => call.args.remove_left_or_key("Type"),
                Some("Patch") => call.args.remove_left_or_key("Base"),
                _ => todo!(),
            },
            other => todo!("{other}"),
        }
    }

    fn lower_type_asc(
        &mut self,
        tasc: ast::TypeAscription,
        expect: Option<&Type>,
    ) -> LowerResult<hir::TypeAscription> {
        log!(info "entered {}({tasc})", fn_name!());
        let kind = tasc.kind();
        let spec_t = match self
            .module
            .context
            .instantiate_typespec(&tasc.t_spec.t_spec)
        {
            Ok(spec_t) => spec_t,
            Err(errs) => {
                self.errs.extend(errs);
                Type::Failure
            }
        };
        let expect = expect.map_or(Some(&spec_t), |exp| {
            self.module.context.min(exp, &spec_t).ok().or(Some(&spec_t))
        });
        let expr = match self.lower_expr(*tasc.expr, expect) {
            Ok(expr) => expr,
            Err(errs) => {
                self.errs.extend(errs);
                hir::Expr::Dummy(hir::Dummy::new(vec![]))
            }
        };
        match kind {
            AscriptionKind::TypeOf | AscriptionKind::AsCast => {
                self.module.context.sub_unify(
                    expr.ref_t(),
                    &spec_t,
                    &expr,
                    Some(&Str::from(expr.to_string())),
                )?;
            }
            AscriptionKind::SubtypeOf => {
                let &ctx = self
                    .module
                    .context
                    .get_singular_ctxs_by_hir_expr(&expr, &self.module.context)?
                    .first()
                    .unwrap();
                // REVIEW: need to use subtype_of?
                if ctx.super_traits.iter().all(|trait_| trait_ != &spec_t)
                    && ctx.super_classes.iter().all(|class| class != &spec_t)
                {
                    return Err(LowerErrors::from(LowerError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        expr.ref_t(), // FIXME:
                        &spec_t,
                        Location::concat(&expr, &tasc.t_spec),
                        self.module.context.caused_by(),
                    )));
                }
            }
            _ => {}
        }
        let t_spec = self.lower_type_spec_with_op(tasc.t_spec, spec_t)?;
        Ok(expr.type_asc(t_spec))
    }

    fn lower_decl(&mut self, tasc: ast::TypeAscription) -> LowerResult<hir::TypeAscription> {
        log!(info "entered {}({tasc})", fn_name!());
        let kind = tasc.kind();
        let spec_t = self
            .module
            .context
            .instantiate_typespec(&tasc.t_spec.t_spec)?;
        let ast::Expr::Accessor(ast::Accessor::Ident(ident)) = *tasc.expr else {
            return Err(LowerErrors::from(LowerError::syntax_error(
                self.cfg.input.clone(),
                line!() as usize,
                tasc.expr.loc(),
                self.module.context.caused_by(),
                switch_lang!(
                    "japanese" => "無効な型宣言です(左辺には記名型のみ使用出来ます)".to_string(),
                    "simplified_chinese" => "无效的类型声明".to_string(),
                    "traditional_chinese" => "無效的型宣告".to_string(),
                    "english" => "Invalid type declaration (currently only nominal types are allowed at LHS)".to_string(),
                ),
                None,
            )));
        };
        let ident_vi = self
            .module
            .context
            .rec_get_decl_info(
                &ident,
                AccessKind::Name,
                &self.cfg.input,
                &self.module.context,
            )
            .none_or_else(|| {
                self.module.context.rec_get_var_info(
                    &ident,
                    AccessKind::Name,
                    &self.cfg.input,
                    &self.module.context,
                )
            })
            .none_or_result(|| {
                let (similar_info, similar_name) = self
                    .module
                    .context
                    .get_similar_name_and_info(ident.inspect())
                    .unzip();
                LowerError::detailed_no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    ident.loc(),
                    self.module.context.caused_by(),
                    ident.inspect(),
                    similar_name,
                    similar_info,
                )
            })?;
        match kind {
            AscriptionKind::TypeOf | AscriptionKind::AsCast => {
                self.module.context.sub_unify(
                    &ident_vi.t,
                    &spec_t,
                    &ident,
                    Some(ident.inspect()),
                )?;
            }
            AscriptionKind::SubtypeOf => {
                if self.module.context.subtype_of(&ident_vi.t, &spec_t) {
                    return Err(LowerErrors::from(LowerError::subtyping_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &ident_vi.t,
                        &spec_t,
                        ident.loc(),
                        self.module.context.caused_by(),
                    )));
                }
            }
            _ => {}
        }
        let qual_name = self
            .module
            .context
            .get_singular_ctxs_by_ident(&ident, &self.module.context)
            .ok()
            .map(|ctx| ctx.first().unwrap().name.clone());
        let ident = hir::Identifier::new(ident, qual_name, ident_vi);
        let expr = hir::Expr::Accessor(hir::Accessor::Ident(ident));
        let t_spec = self.lower_type_spec_with_op(tasc.t_spec, spec_t)?;
        Ok(expr.type_asc(t_spec))
    }

    // Call.obj == Accessor cannot be type inferred by itself (it can only be inferred with arguments)
    // so turn off type checking (check=false)
    fn lower_expr(&mut self, expr: ast::Expr, expect: Option<&Type>) -> LowerResult<hir::Expr> {
        log!(info "entered {}", fn_name!());
        let casted = self.module.context.get_casted_type(&expr);
        let mut expr = match expr {
            ast::Expr::Literal(lit) => hir::Expr::Literal(self.lower_literal(lit, expect)?),
            ast::Expr::Array(arr) => hir::Expr::Array(self.lower_array(arr, expect)?),
            ast::Expr::Tuple(tup) => hir::Expr::Tuple(self.lower_tuple(tup, expect)?),
            ast::Expr::Record(rec) => hir::Expr::Record(self.lower_record(rec, expect)?),
            ast::Expr::Set(set) => hir::Expr::Set(self.lower_set(set, expect)?),
            ast::Expr::Dict(dict) => hir::Expr::Dict(self.lower_dict(dict, expect)?),
            ast::Expr::Accessor(acc) => hir::Expr::Accessor(self.lower_acc(acc, expect)?),
            ast::Expr::BinOp(bin) => hir::Expr::BinOp(self.lower_bin(bin, expect)),
            ast::Expr::UnaryOp(unary) => hir::Expr::UnaryOp(self.lower_unary(unary, expect)),
            ast::Expr::Call(call) => hir::Expr::Call(self.lower_call(call, expect)),
            ast::Expr::DataPack(pack) => hir::Expr::Call(self.lower_pack(pack, expect)?),
            ast::Expr::Lambda(lambda) => hir::Expr::Lambda(self.lower_lambda(lambda, expect)?),
            ast::Expr::TypeAscription(tasc) => {
                hir::Expr::TypeAsc(self.lower_type_asc(tasc, expect)?)
            }
            ast::Expr::Compound(comp) => hir::Expr::Compound(self.lower_compound(comp, expect)?),
            // Checking is also performed for expressions in Dummy. However, it has no meaning in code generation
            ast::Expr::Dummy(dummy) => hir::Expr::Dummy(self.lower_dummy(dummy, expect)?),
            ast::Expr::InlineModule(inline) => {
                hir::Expr::Call(self.lower_inline_module(inline, expect))
            }
            other => {
                log!(err "unreachable: {other}");
                return unreachable_error!(LowerErrors, LowerError, self.module.context);
            }
        };
        if let Some(casted) = casted {
            if expr.ref_t().is_projection() || self.module.context.subtype_of(&casted, expr.ref_t())
            {
                if let Some(ref_mut_t) = expr.ref_mut_t() {
                    *ref_mut_t = casted;
                }
            }
        }
        Ok(expr)
    }

    /// The meaning of TypeAscription changes between chunk and expr.
    /// For example, `x: Int`, as expr, is `x` itself,
    /// but as chunk, it declares that `x` is of type `Int`, and is valid even before `x` is defined.
    pub fn lower_chunk(
        &mut self,
        chunk: ast::Expr,
        expect: Option<&Type>,
    ) -> LowerResult<hir::Expr> {
        log!(info "entered {}", fn_name!());
        match chunk {
            ast::Expr::Def(def) => Ok(hir::Expr::Def(self.lower_def(def)?)),
            ast::Expr::ClassDef(defs) => Ok(hir::Expr::ClassDef(self.lower_class_def(defs)?)),
            ast::Expr::PatchDef(defs) => Ok(hir::Expr::PatchDef(self.lower_patch_def(defs)?)),
            ast::Expr::ReDef(redef) => Ok(hir::Expr::ReDef(self.lower_redef(redef)?)),
            ast::Expr::TypeAscription(tasc) => Ok(hir::Expr::TypeAsc(self.lower_decl(tasc)?)),
            other => self.lower_expr(other, expect),
        }
    }

    fn lower_block(
        &mut self,
        ast_block: ast::Block,
        expect: Option<&Type>,
    ) -> LowerResult<hir::Block> {
        log!(info "entered {}", fn_name!());
        let mut hir_block = Vec::with_capacity(ast_block.len());
        let last = ast_block.len().saturating_sub(1);
        for (i, chunk) in ast_block.into_iter().enumerate() {
            let expect = if i == last { expect } else { None };
            let chunk = match self.lower_chunk(chunk, expect) {
                Ok(chunk) => chunk,
                Err(errs) => {
                    self.errs.extend(errs);
                    hir::Expr::Dummy(hir::Dummy::new(vec![]))
                }
            };
            hir_block.push(chunk);
        }
        Ok(hir::Block::new(hir_block))
    }

    fn lower_compound(
        &mut self,
        compound: ast::Compound,
        expect: Option<&Type>,
    ) -> LowerResult<hir::Block> {
        log!(info "entered {}", fn_name!());
        let mut hir_block = Vec::with_capacity(compound.len());
        let last = compound.len().saturating_sub(1);
        for (i, chunk) in compound.into_iter().enumerate() {
            let expect = if i == last { expect } else { None };
            let chunk = self.lower_chunk(chunk, expect)?;
            hir_block.push(chunk);
        }
        Ok(hir::Block::new(hir_block))
    }

    fn lower_dummy(
        &mut self,
        ast_dummy: ast::Dummy,
        expect: Option<&Type>,
    ) -> LowerResult<hir::Dummy> {
        log!(info "entered {}", fn_name!());
        let mut hir_dummy = Vec::with_capacity(ast_dummy.len());
        let last = ast_dummy.len().saturating_sub(1);
        for (i, chunk) in ast_dummy.into_iter().enumerate() {
            let expect = if i == last { expect } else { None };
            let chunk = self.lower_chunk(chunk, expect)?;
            hir_dummy.push(chunk);
        }
        Ok(hir::Dummy::new(hir_dummy))
    }

    pub(crate) fn lower_inline_module(
        &mut self,
        inline: InlineModule,
        expect: Option<&Type>,
    ) -> hir::Call {
        log!(info "entered {}", fn_name!());
        let path = NormalizedPathBuf::from(inline.input.path().to_path_buf());
        if self
            .module
            .context
            .shared()
            .get_module(&path)
            .is_some_and(|ent| ent.is_complete())
        {
            return self.lower_call(inline.import, expect);
        }
        let parent = self.get_mod_ctx().context.get_module().unwrap().clone();
        let mod_ctx = ModuleContext::new(parent, dict! {});
        let mod_name = mod_name(&path);
        let mut builder = GenericHIRBuilder::<A>::new_submodule(mod_ctx, &mod_name);
        builder.lowerer.module.context.cfg.input = inline.input.clone();
        builder.cfg_mut().input = inline.input.clone();
        let mode = if path.to_string_lossy().ends_with(".d.er") {
            "declare"
        } else {
            "exec"
        };
        let (hir, status) = match builder.check(inline.ast, mode) {
            Ok(art) => {
                self.warns.extend(art.warns);
                (Some(art.object), CheckStatus::Succeed)
            }
            Err(art) => {
                self.warns.extend(art.warns);
                self.errs.extend(art.errors);
                (art.object, CheckStatus::Failed)
            }
        };
        let ctx = builder.pop_mod_ctx().unwrap();
        let cache = if path.to_string_lossy().ends_with(".d.er") {
            &self.module.context.shared().py_mod_cache
        } else {
            &self.module.context.shared().mod_cache
        };
        cache.register(path.to_path_buf(), None, hir, ctx, status);
        self.lower_call(inline.import, expect)
    }

    fn return_incomplete_artifact(&mut self, hir: HIR) -> IncompleteArtifact {
        self.module.context.clear_invalid_vars();
        IncompleteArtifact::new(
            Some(hir),
            LowerErrors::from(self.errs.take_all()),
            LowerWarnings::from(self.warns.take_all()),
        )
    }

    pub(crate) fn lint(&mut self, hir: &HIR, mode: &str) {
        self.warn_implicit_union(hir);
        self.warn_unused_expr(&hir.module, mode);
        self.check_doc_comments(hir);
        self.warn_unused_local_vars(mode);
    }

    pub fn lower(&mut self, ast: AST, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        log!(info "the AST lowering process has started.");
        log!(info "the type-checking process has started.");
        let ast = ASTLinker::new(self.cfg.clone())
            .link(ast, mode)
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
        if let Err(errs) = self.module.context.preregister_const(ast.module.block()) {
            self.errs.extend(errs);
        }
        if let Err(errs) = self.module.context.register_const(ast.module.block()) {
            self.errs.extend(errs);
        }
        for chunk in ast.module.into_iter() {
            match self.lower_chunk(chunk, None) {
                Ok(chunk) => {
                    module.push(chunk);
                }
                Err(errs) => {
                    self.errs.extend(errs);
                }
            }
        }
        self.module.context.clear_invalid_vars();
        self.module.context.check_decls().unwrap_or_else(|errs| {
            self.errs.extend(errs);
        });
        let hir = HIR::new(ast.name, module);
        log!(info "HIR (not resolved, current errs: {}):\n{hir}", self.errs.len());
        let hir = match self.module.context.resolve(hir) {
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
        self.lint(&hir, mode);
        if &self.module.context.name[..] == "<module>" || ELS {
            if ELS {
                self.module.context.shared().promises.join_children();
            } else {
                self.module.context.shared().promises.join_all();
            }
            let errs = self.module.context.shared().errors.take();
            let warns = self.module.context.shared().warns.take();
            self.errs.extend(errs);
            self.warns.extend(warns);
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
