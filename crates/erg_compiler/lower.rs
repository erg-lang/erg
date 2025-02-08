//! implements `ASTLowerer`.
//!
//! ASTLowerer(ASTからHIRへの変換器)を実装
use std::marker::PhantomData;
use std::mem;

use erg_common::config::{ErgConfig, ErgMode};
use erg_common::consts::{ELS, ERG_MODE, PYTHON_MODE};
use erg_common::dict;
use erg_common::dict::Dict;
use erg_common::error::{ErrorDisplay, ErrorKind};
use erg_common::error::{Location, MultiErrorDisplay};
use erg_common::fresh::FreshNameGenerator;
use erg_common::pathutil::{mod_name, NormalizedPathBuf};
use erg_common::set;
use erg_common::set::Set;
use erg_common::traits::BlockKind;
use erg_common::traits::New;
use erg_common::traits::OptionalTranspose;
use erg_common::traits::{ExitStatus, Locational, NoTypeDisplay, Runnable, Stream};
use erg_common::triple::Triple;
use erg_common::{fmt_option, fn_name, log, switch_lang, Str};

use erg_parser::ast::{self, AscriptionKind, DefId, InlineModule, VisModifierSpec};
use erg_parser::ast::{OperationKind, TypeSpecWithOp, VarName, AST};
use erg_parser::build_ast::{ASTBuildable, ASTBuilder as DefaultASTBuilder};
use erg_parser::desugar::Desugarer;
use erg_parser::token::{Token, TokenKind};
use erg_parser::Parser;
use erg_parser::ParserRunner;

use crate::artifact::{BuildRunnable, Buildable, CompleteArtifact, IncompleteArtifact};
use crate::build_package::CheckStatus;
use crate::module::SharedCompilerResource;
use crate::ty::constructors::{
    free_var, from_str, func, guard, list_t, mono, poly, proc, refinement, set_t, singleton, ty_tp,
    unsized_list_t, v_enum,
};
use crate::ty::free::Constraint;
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{
    CastTarget, Field, GuardType, HasType, ParamTy, Predicate, SubrType, Type, VisibilityModifier,
};

use crate::context::{
    ClassDefType, Context, ContextKind, ContextProvider, ControlKind, MethodContext, ModuleContext,
    RegistrationMode, TypeContext,
};
use crate::error::{
    CompileError, CompileErrors, CompileWarning, Failable, FailableOption, LowerError, LowerErrors,
    LowerResult, LowerWarning, LowerWarnings, SingleLowerResult,
};
use crate::hir;
use crate::hir::HIR;
use crate::link_ast::ASTLinker;
use crate::varinfo::{VarInfo, VarKind};
use crate::{feature_error, unreachable_error};
use crate::{AccessKind, GenericHIRBuilder};

use VisibilityModifier::*;

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

    fn set_input(&mut self, input: erg_common::io::Input) {
        self.module.context.cfg.input = input.clone();
        self.cfg.input = input;
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

    fn expect_block(&self, src: &str) -> BlockKind {
        let mut parser = ParserRunner::new(self.cfg().clone());
        match parser.eval(src.to_string()) {
            Err(errs) => {
                let kind = errs
                    .iter()
                    .filter(|e| e.core().kind == ErrorKind::ExpectNextLine)
                    .map(|e| {
                        let msg = e.core().sub_messages.last().unwrap();
                        // ExpectNextLine error must have msg otherwise it's a bug
                        msg.get_msg().first().unwrap().to_owned()
                    })
                    .next();
                if let Some(kind) = kind {
                    return BlockKind::from(kind.as_str());
                }
                if errs
                    .iter()
                    .any(|err| err.core.main_message.contains("\"\"\""))
                {
                    return BlockKind::MultiLineStr;
                }
                BlockKind::Error
            }
            Ok(_) => {
                if src.contains("Class") {
                    return BlockKind::ClassDef;
                }
                BlockKind::None
            }
        }
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

    fn lower_list(&mut self, list: ast::List, expect: Option<&Type>) -> FailableOption<hir::List> {
        log!(info "entered {}({list})", fn_name!());
        match list {
            ast::List::Normal(lis) => Ok(hir::List::Normal(
                self.lower_normal_list(lis, expect)
                    .map_err(|(lis, es)| (Some(hir::List::Normal(lis)), es))?,
            )),
            ast::List::WithLength(lis) => Ok(hir::List::WithLength(
                self.lower_list_with_length(lis, expect)
                    .map_err(|(lis, es)| (Some(hir::List::WithLength(lis)), es))?,
            )),
            other => feature_error!(
                LowerErrors,
                LowerError,
                self.module.context,
                other.loc(),
                "list comprehension"
            )
            .map_err(|es| (None, es)),
        }
    }

    fn elem_err(&self, union: Type, elem: &hir::Expr) -> LowerErrors {
        let elem_disp_notype = elem.to_string_notype();
        let union = self.module.context.readable_type(union);
        LowerErrors::from(LowerError::syntax_error(
            self.cfg.input.clone(),
            line!() as usize,
            elem.loc(),
            String::from(&self.module.context.name[..]),
            switch_lang!(
                "japanese" => "リストの要素は全て同じ型である必要があります",
                "simplified_chinese" => "数组元素必须全部是相同类型",
                "traditional_chinese" => "數組元素必須全部是相同類型",
                "english" => "all elements of a list must be of the same type",
            )
            .to_owned(),
            Some(switch_lang!(
                "japanese" => format!("[..., {elem_disp_notype}: {union}]など明示的に型を指定してください"),
                "simplified_chinese" => format!("请明确指定类型，例如: [..., {elem_disp_notype}: {union}]"),
                "traditional_chinese" => format!("請明確指定類型，例如: [..., {elem_disp_notype}: {union}]"),
                "english" => format!("please specify the type explicitly, e.g. [..., {elem_disp_notype}: {union}]"),
            )),
        ))
    }

    fn lower_normal_list(
        &mut self,
        list: ast::NormalList,
        expect: Option<&Type>,
    ) -> Failable<hir::NormalList> {
        log!(info "entered {}({list})", fn_name!());
        let mut errors = LowerErrors::empty();
        let mut new_list = vec![];
        let eval_result = self.module.context.eval_const_normal_list(&list);
        let (elems, ..) = list.elems.deconstruct();
        let expect_elem = expect.and_then(|t| {
            // REVIEW: are these all?
            if !(t.is_list() || t.is_list_mut() || t.is_iterable()) {
                return None;
            }
            self.module
                .context
                .convert_tp_into_type(t.typarams().first()?.clone())
                .ok()
        });
        let mut union = Type::Never;
        for elem in elems.into_iter() {
            let elem = match self.lower_expr(elem.expr, expect_elem.as_ref()) {
                Ok(elem) => elem,
                Err((expr, errs)) => {
                    errors.extend(errs);
                    expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
                }
            };
            let union_ = self.module.context.union(&union, elem.ref_t());
            if let Err(es) = self.homogeneity_check(expect_elem.as_ref(), &union_, &union, &elem) {
                errors.extend(es);
            }
            union = union_;
            new_list.push(elem);
        }
        let elem_t = if union == Type::Never {
            free_var(
                self.module.context.level,
                Constraint::new_type_of(Type::Type),
            )
        } else {
            union
        };
        let elems = hir::Args::values(new_list, None);
        let t = list_t(elem_t, TyParam::value(elems.len()));
        let t = if let Ok(value) = eval_result {
            singleton(t, TyParam::Value(value))
        } else {
            t
        };
        let list = hir::NormalList::new(list.l_sqbr, list.r_sqbr, t, elems);
        if errors.is_empty() {
            Ok(list)
        } else {
            Err((list, errors))
        }
    }

    fn homogeneity_check(
        &self,
        expect_elem: Option<&Type>,
        union_: &Type,
        union: &Type,
        elem: &hir::Expr,
    ) -> LowerResult<()> {
        if ERG_MODE && expect_elem.is_none() && union_.union_size() > 1 {
            if let hir::Expr::TypeAsc(type_asc) = elem {
                // e.g. [1, "a": Str or NoneType]
                if !self
                    .module
                    .context
                    .supertype_of(&type_asc.spec.spec_t, union)
                {
                    return Err(self.elem_err(union_.clone(), elem));
                } // else(OK): e.g. [1, "a": Str or Int]
            }
            // OK: ?T(:> {"a"}) or ?U(:> {"b"}) or {"c", "d"} => {"a", "b", "c", "d"} <: Str
            else if self
                .module
                .context
                .coerce(union_.derefine(), &())
                .map_or(true, |coerced| coerced.union_pair().is_some())
            {
                return Err(self.elem_err(union_.clone(), elem));
            }
        }
        Ok(())
    }

    fn lower_list_with_length(
        &mut self,
        list: ast::ListWithLength,
        expect: Option<&Type>,
    ) -> Failable<hir::ListWithLength> {
        log!(info "entered {}({list})", fn_name!());
        let mut errors = LowerErrors::empty();
        let expect_elem = expect.and_then(|t| {
            if !(t.is_list() || t.is_list_mut() || t.is_iterable()) {
                return None;
            }
            self.module
                .context
                .convert_tp_into_type(t.typarams().first()?.clone())
                .ok()
        });
        let elem = match self.lower_expr(list.elem.expr, expect_elem.as_ref()) {
            Ok(elem) => elem,
            Err((expr, errs)) => {
                errors.extend(errs);
                expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
            }
        };
        let list_t = self.gen_list_with_length_type(&elem, &list.len);
        let len = match *list.len {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) if ident.is_discarded() => None,
            len => match self.lower_expr(len, Some(&Type::Nat)) {
                Ok(len) => Some(len),
                Err((expr, errs)) => {
                    errors.extend(errs);
                    expr
                }
            },
        };
        let hir_list = hir::ListWithLength::new(list.l_sqbr, list.r_sqbr, list_t, elem, len);
        if errors.is_empty() {
            Ok(hir_list)
        } else {
            Err((hir_list, errors))
        }
    }

    fn gen_list_with_length_type(&self, elem: &hir::Expr, len: &ast::Expr) -> Type {
        match len {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) if ident.is_discarded() => {
                return unsized_list_t(elem.t());
            }
            _ => {}
        }
        let maybe_len = self.module.context.eval_const_expr(len);
        match maybe_len {
            Ok(v @ ValueObj::Nat(_)) => list_t(elem.t(), TyParam::Value(v)),
            Ok(other) => todo!("{other} is not a Nat object"),
            Err((_, err)) => todo!("{err}"),
        }
    }

    fn lower_tuple(&mut self, tuple: ast::Tuple, expect: Option<&Type>) -> Failable<hir::Tuple> {
        log!(info "entered {}({tuple})", fn_name!());
        match tuple {
            ast::Tuple::Normal(tup) => Ok(hir::Tuple::Normal(
                self.lower_normal_tuple(tup, expect)
                    .map_err(|(tup, errs)| (hir::Tuple::Normal(tup), errs))?,
            )),
        }
    }

    fn lower_normal_tuple(
        &mut self,
        tuple: ast::NormalTuple,
        expect: Option<&Type>,
    ) -> Failable<hir::NormalTuple> {
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
        let mut errors = LowerErrors::empty();
        let mut new_tuple = vec![];
        let (elems, .., paren) = tuple.elems.deconstruct();
        let expect_ts = expect.transpose(elems.len());
        for (elem, expect) in elems
            .into_iter()
            .zip(expect_ts.into_iter().chain(std::iter::repeat(None)))
        {
            match self.lower_expr(elem.expr, expect.as_ref()) {
                Ok(elem) => new_tuple.push(elem),
                Err((expr, errs)) => {
                    errors.extend(errs);
                    new_tuple.push(expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty())));
                }
            }
        }
        let tuple = hir::NormalTuple::new(hir::Args::values(new_tuple, paren));
        if errors.is_empty() {
            Ok(tuple)
        } else {
            Err((tuple, errors))
        }
    }

    fn lower_record(
        &mut self,
        record: ast::Record,
        expect: Option<&Type>,
    ) -> Failable<hir::Record> {
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
        expect: Option<&Type>,
    ) -> Failable<hir::Record> {
        log!(info "entered {}({record})", fn_name!());
        let expect_record = expect.and_then(|t| {
            if let Type::Record(rec) = t {
                Some(rec)
            } else {
                None
            }
        });
        let mut hir_record =
            hir::Record::new(record.l_brace, record.r_brace, hir::RecordAttrs::empty());
        let mut errs = LowerErrors::empty();
        self.module
            .context
            .grow("<record>", ContextKind::Dummy, Private, None);
        for attr in record.attrs.iter() {
            if attr.sig.is_const() {
                if let Err(es) = self.module.context.register_def(attr) {
                    errs.extend(es);
                }
            }
        }
        for attr in record.attrs.into_iter() {
            let expect =
                expect_record.and_then(|rec| attr.sig.ident().and_then(|id| rec.get(id.inspect())));
            let attr = match self.lower_def(attr, expect) {
                Ok(def) => def,
                Err((Some(def), es)) => {
                    errs.extend(es);
                    def
                }
                Err((None, es)) => {
                    errs.extend(es);
                    continue;
                }
            };
            hir_record.push(attr);
        }
        self.pop_append_errs();
        if errs.is_empty() {
            Ok(hir_record)
        } else {
            Err((hir_record, errs))
        }
    }

    fn lower_set(&mut self, set: ast::Set, expect: Option<&Type>) -> FailableOption<hir::Set> {
        log!(info "enter {}({set})", fn_name!());
        match set {
            ast::Set::Normal(set) => Ok(hir::Set::Normal(
                self.lower_normal_set(set, expect)
                    .map_err(|(set, es)| (Some(hir::Set::Normal(set)), es))?,
            )),
            ast::Set::WithLength(set) => Ok(hir::Set::WithLength(
                self.lower_set_with_length(set, expect)
                    .map_err(|(set, es)| (Some(hir::Set::WithLength(set)), es))?,
            )),
            ast::Set::Comprehension(set) => feature_error!(
                LowerErrors,
                LowerError,
                self.module.context,
                set.loc(),
                "set comprehension"
            )
            .map_err(|es| (None, es)),
        }
    }

    fn lower_normal_set(
        &mut self,
        set: ast::NormalSet,
        expect: Option<&Type>,
    ) -> Failable<hir::NormalSet> {
        log!(info "entered {}({set})", fn_name!());
        let mut errors = LowerErrors::empty();
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
            let elem = match self.lower_expr(elem.expr, expect_elem.as_ref()) {
                Ok(elem) => elem,
                Err((expr, errs)) => {
                    errors.extend(errs);
                    expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
                }
            };
            union = self.module.context.union(&union, elem.ref_t());
            if ERG_MODE && union.is_union_type() {
                let err = LowerError::set_homogeneity_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    elem.loc(),
                    self.module.context.caused_by(),
                );
                errors.push(err);
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
            errors.extend(errs);
        }
        let set = hir::NormalSet::new(set.l_brace, set.r_brace, elem_t, elems);
        if errors.is_empty() {
            Ok(set)
        } else {
            Err((set, errors))
        }
    }

    /// This (e.g. {"a"; 3}) is meaningless as an object, but makes sense as a type (e.g. {Int; 3}).
    fn lower_set_with_length(
        &mut self,
        set: ast::SetWithLength,
        expect: Option<&Type>,
    ) -> Failable<hir::SetWithLength> {
        log!("entered {}({set})", fn_name!());
        let mut errors = LowerErrors::empty();
        let expect_elem = expect.and_then(|t| {
            if !t.is_set() {
                return None;
            }
            self.module
                .context
                .convert_tp_into_type(t.typarams().first()?.clone())
                .ok()
        });
        let elem = match self.lower_expr(set.elem.expr, expect_elem.as_ref()) {
            Ok(elem) => elem,
            Err((expr, errs)) => {
                errors.extend(errs);
                expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
            }
        };
        let set_t = self.gen_set_with_length_type(&elem, &set.len);
        let len = match self.lower_expr(*set.len, Some(&Type::Nat)) {
            Ok(len) => len,
            Err((expr, errs)) => {
                errors.extend(errs);
                expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
            }
        };
        let hir_set = hir::SetWithLength::new(set.l_brace, set.r_brace, set_t, elem, len);
        if errors.is_empty() {
            Ok(hir_set)
        } else {
            Err((hir_set, errors))
        }
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

    fn lower_dict(&mut self, dict: ast::Dict, expect: Option<&Type>) -> FailableOption<hir::Dict> {
        log!(info "enter {}({dict})", fn_name!());
        match dict {
            ast::Dict::Normal(set) => Ok(hir::Dict::Normal(
                self.lower_normal_dict(set, expect)
                    .map_err(|(set, es)| (Some(hir::Dict::Normal(set)), es))?,
            )),
            other => feature_error!(
                LowerErrors,
                LowerError,
                self.module.context,
                other.loc(),
                "dict comprehension"
            )
            .map_err(|es| (None, es)),
            // ast::Dict::WithLength(set) => Ok(hir::Dict::WithLength(self.lower_dict_with_length(set, expect)?)),
        }
    }

    fn lower_normal_dict(
        &mut self,
        dict: ast::NormalDict,
        expect: Option<&Type>,
    ) -> Failable<hir::NormalDict> {
        log!(info "enter {}({dict})", fn_name!());
        let mut errors = LowerErrors::empty();
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
            let expect_key = expect.as_ref().and_then(|dict| dict.keys().next());
            let expect_value = expect.as_ref().and_then(|dict| dict.values().next());
            let key = match self.lower_expr(kv.key, expect_key) {
                Ok(key) => key,
                Err((expr, errs)) => {
                    errors.extend(errs);
                    expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
                }
            };
            let value = match self.lower_expr(kv.value, expect_value) {
                Ok(value) => value,
                Err((expr, errs)) => {
                    errors.extend(errs);
                    expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
                }
            };
            if let Some(popped_val_t) = union.insert(key.t(), value.t()) {
                if PYTHON_MODE {
                    if let Some(val_t) = union.linear_get_mut(key.ref_t()) {
                        *val_t = self.module.context.union(&mem::take(val_t), &popped_val_t);
                    }
                } else {
                    let err = LowerError::syntax_error(
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
                    );
                    errors.push(err);
                }
            }
            new_kvs.push(hir::KeyValue::new(key, value));
        }
        for key_t in union.keys() {
            let loc = &(&dict.l_brace, &dict.r_brace);
            // check if key_t is Eq and Hash
            let eq_hash = mono("Eq") & mono("Hash");
            if let Err(errs) = self.module.context.sub_unify(key_t, &eq_hash, loc, None) {
                errors.extend(errs);
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
        let dict = hir::NormalDict::new(dict.l_brace, dict.r_brace, kv_ts, new_kvs);
        if errors.is_empty() {
            Ok(dict)
        } else {
            Err((dict, errors))
        }
    }

    pub(crate) fn lower_acc(
        &mut self,
        acc: ast::Accessor,
        expect: Option<&Type>,
    ) -> Failable<hir::Accessor> {
        log!(info "entered {}({acc})", fn_name!());
        let mut errors = LowerErrors::empty();
        match acc {
            ast::Accessor::Ident(ident) => {
                let ident = match self.lower_ident(ident, expect) {
                    Ok(ident) => ident,
                    Err((ident, errs)) => {
                        errors.extend(errs);
                        ident
                    }
                };
                let acc = hir::Accessor::Ident(ident);
                // debug_assert!(acc.ref_t().has_no_qvar(), "{acc} has qvar");
                if errors.is_empty() {
                    Ok(acc)
                } else {
                    Err((acc, errors))
                }
            }
            ast::Accessor::Attr(attr) => {
                let obj = match self.lower_expr(*attr.obj, None) {
                    Ok(obj) => obj,
                    Err((expr, errs)) => {
                        errors.extend(errs);
                        expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
                    }
                };
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
                        errors.push(errs);
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
                        errors.push(err);
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
                // debug_assert!(acc.ref_t().has_no_qvar(), "{acc} has qvar");
                if errors.is_empty() {
                    Ok(acc)
                } else {
                    Err((acc, errors))
                }
            }
            ast::Accessor::TypeApp(t_app) => feature_error!(
                LowerErrors,
                LowerError,
                self.module.context,
                t_app.loc(),
                "type application"
            )
            .map_err(|errs| (hir::Accessor::public_with_line("<todo>".into(), 0), errs)),
            // TupleAttr, Subscr are desugared
            _ => unreachable_error!(LowerErrors, LowerError, self.module.context).map_err(|errs| {
                (
                    hir::Accessor::public_with_line("<unreachable>".into(), 0),
                    errs,
                )
            }),
        }
    }

    fn lower_ident(
        &mut self,
        ident: ast::Identifier,
        expect: Option<&Type>,
    ) -> Failable<hir::Identifier> {
        let mut errors = LowerErrors::empty();
        // `match` is a special form, typing is magic
        let (vi, __name__) = if ident.vis.is_private()
            && (&ident.inspect()[..] == "match" || &ident.inspect()[..] == "match!")
        {
            (
                VarInfo {
                    t: mono("Subroutine"),
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
                    errors.push(err);
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
                    errors.push(err);
                    VarInfo::ILLEGAL
                }
            };
            (
                res,
                self.module
                    .context
                    .get_singular_ctxs_by_ident(&ident, &self.module.context)
                    .ok()
                    .and_then(|ctxs| ctxs.first().map(|ctx| ctx.name.clone())),
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
        if errors.is_empty() {
            Ok(ident)
        } else {
            Err((ident, errors))
        }
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
        match (
            call.obj.as_ref(),
            &call.attr_name,
            &call.args.nth_or_key(0, "object"),
            &call.args.nth_or_key(1, "classinfo"),
        ) {
            (ast::Expr::Accessor(ast::Accessor::Ident(ident)), None, Some(lhs), Some(rhs)) => {
                match &ident.inspect()[..] {
                    "isinstance" | "issubclass" => {
                        self.get_bin_guard_type(ident.name.token(), lhs, rhs)
                    }
                    "hasattr" => {
                        let ast::Expr::Literal(lit) = rhs else {
                            return None;
                        };
                        if !lit.is(TokenKind::StrLit) {
                            return None;
                        }
                        let name = lit.token.content.trim_matches('\"');
                        let target = self.expr_to_cast_target(lhs);
                        let rec = dict! {
                            Field::new(VisibilityModifier::Public, Str::rc(name)) => Type::Obj,
                        };
                        let to = Type::Record(rec).structuralize();
                        Some(guard(self.module.context.name.clone(), target, to))
                    }
                    _ => None,
                }
            }
            (ast::Expr::Accessor(ast::Accessor::Ident(ident)), None, Some(arg), None) => {
                match &ident.inspect()[..] {
                    "not" => {
                        let arg = self.get_guard_type(arg)?;
                        Some(self.module.context.complement(&arg))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn expr_to_cast_target(&self, expr: &ast::Expr) -> CastTarget {
        match expr {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) => {
                if let Some(nth) = self
                    .module
                    .context
                    .params
                    .iter()
                    .position(|(name, _)| name.as_ref() == Some(&ident.name))
                {
                    CastTarget::Arg {
                        nth,
                        name: ident.inspect().clone(),
                        loc: ident.loc(),
                    }
                } else {
                    CastTarget::Var {
                        name: ident.inspect().clone(),
                        loc: ident.loc(),
                    }
                }
            }
            _ => CastTarget::expr(expr.clone()),
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
            self.expr_to_cast_target(rhs)
        } else {
            self.expr_to_cast_target(lhs)
        };
        let namespace = self.module.context.name.clone();
        match op.kind {
            // l in T -> T contains l
            TokenKind::ContainsOp => {
                let to = self.module.context.expr_to_type(lhs.clone()).ok()?;
                Some(guard(namespace, target, to))
            }
            TokenKind::Symbol if &op.content[..] == "isinstance" => {
                // isinstance(x, (T, U)) => x: T or U
                let to = if let ast::Expr::Tuple(ast::Tuple::Normal(tys)) = rhs {
                    tys.elems.pos_args.iter().fold(Type::Never, |acc, ex| {
                        let Ok(ty) = self.module.context.expr_to_type(ex.expr.clone()) else {
                            return acc;
                        };
                        self.module.context.union(&acc, &ty)
                    })
                } else {
                    self.module.context.expr_to_type(rhs.clone()).ok()?
                };
                Some(guard(namespace, target, to))
            }
            TokenKind::IsOp | TokenKind::DblEq => {
                let rhs = self.module.context.expr_to_value(rhs.clone()).ok()?;
                Some(guard(namespace, target, v_enum(set! { rhs })))
            }
            TokenKind::IsNotOp | TokenKind::NotEq => {
                let rhs = self.module.context.expr_to_value(rhs.clone()).ok()?;
                let ty = guard(namespace, target, v_enum(set! { rhs }));
                Some(self.module.context.complement(&ty))
            }
            TokenKind::Gre => {
                let rhs = self.module.context.expr_to_tp(rhs.clone()).ok()?;
                let t = self.module.context.get_tp_t(&rhs).unwrap_or(Type::Obj);
                let varname = self.fresh_gen.fresh_varname();
                let pred = Predicate::gt(varname.clone(), rhs);
                let refine = refinement(varname, t, pred);
                Some(guard(namespace, target, refine))
            }
            TokenKind::GreEq => {
                let rhs = self.module.context.expr_to_tp(rhs.clone()).ok()?;
                let t = self.module.context.get_tp_t(&rhs).unwrap_or(Type::Obj);
                let varname = self.fresh_gen.fresh_varname();
                let pred = Predicate::ge(varname.clone(), rhs);
                let refine = refinement(varname, t, pred);
                Some(guard(namespace, target, refine))
            }
            TokenKind::Less => {
                let rhs = self.module.context.expr_to_tp(rhs.clone()).ok()?;
                let t = self.module.context.get_tp_t(&rhs).unwrap_or(Type::Obj);
                let varname = self.fresh_gen.fresh_varname();
                let pred = Predicate::lt(varname.clone(), rhs);
                let refine = refinement(varname, t, pred);
                Some(guard(namespace, target, refine))
            }
            TokenKind::LessEq => {
                let rhs = self.module.context.expr_to_tp(rhs.clone()).ok()?;
                let t = self.module.context.get_tp_t(&rhs).unwrap_or(Type::Obj);
                let varname = self.fresh_gen.fresh_varname();
                let pred = Predicate::le(varname.clone(), rhs);
                let refine = refinement(varname, t, pred);
                Some(guard(namespace, target, refine))
            }
            _ => None,
        }
    }

    fn lower_bin(&mut self, bin: ast::BinOp, expect: Option<&Type>) -> Failable<hir::BinOp> {
        log!(info "entered {}({bin})", fn_name!());
        let mut errors = LowerErrors::empty();
        let mut args = bin.args.into_iter();
        let lhs = *args.next().unwrap();
        let rhs = *args.next().unwrap();
        let guard = self.get_bin_guard_type(&bin.op, &lhs, &rhs);
        let lhs = self.lower_expr(lhs, None).unwrap_or_else(|(expr, errs)| {
            errors.extend(errs);
            expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::new(vec![])))
        });
        let lhs = hir::PosArg::new(lhs);
        let rhs = self.lower_expr(rhs, None).unwrap_or_else(|(expr, errs)| {
            errors.extend(errs);
            expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::new(vec![])))
        });
        let rhs = hir::PosArg::new(rhs);
        let mut args = [lhs, rhs];
        let mut vi = self
            .module
            .context
            .get_binop_t(&bin.op, &mut args, &self.cfg.input, &self.module.context)
            .unwrap_or_else(|errs| {
                errors.extend(errs);
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
        let bin = hir::BinOp::new(bin.op, lhs, rhs, vi);
        if errors.is_empty() {
            Ok(bin)
        } else {
            Err((bin, errors))
        }
    }

    fn lower_unary(
        &mut self,
        unary: ast::UnaryOp,
        expect: Option<&Type>,
    ) -> Failable<hir::UnaryOp> {
        log!(info "entered {}({unary})", fn_name!());
        let mut errors = LowerErrors::empty();
        let mut args = unary.args.into_iter();
        let arg = self
            .lower_expr(*args.next().unwrap(), None)
            .unwrap_or_else(|(expr, errs)| {
                errors.extend(errs);
                expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::new(vec![])))
            });
        let mut args = [hir::PosArg::new(arg)];
        let vi = self
            .module
            .context
            .get_unaryop_t(&unary.op, &mut args, &self.cfg.input, &self.module.context)
            .unwrap_or_else(|errs| {
                errors.extend(errs);
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
        let unary = hir::UnaryOp::new(unary.op, expr, vi);
        if errors.is_empty() {
            Ok(unary)
        } else {
            Err((unary, errors))
        }
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
                    // e.g. `if not isinstance(x, Int)`
                    // push {x in not Int}, not {x in Int}
                    if let Some(kind) = self.module.context.current_control_flow() {
                        self.push_guard(nth, kind, expr.ref_t());
                    }
                    hir_pos_args[nth] = hir::PosArg::new(expr);
                }
                Err((expr, es)) => {
                    if let Some(expr) = expr {
                        hir_pos_args[nth] = hir::PosArg::new(expr);
                    }
                    errs.extend(es);
                }
            }
        }
        hir_args.pos_args = hir_pos_args;
        // TODO: expect var_args
        if let Some(var_args) = var_args {
            match self.lower_expr(var_args.expr, None) {
                Ok(expr) => hir_args.var_args = Some(Box::new(hir::PosArg::new(expr))),
                Err((expr, es)) => {
                    errs.extend(es);
                    let dummy = expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()));
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
                Err((expr, es)) => {
                    errs.extend(es);
                    hir_args.push_kw(hir::KwArg::new(
                        arg.keyword,
                        expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty())),
                    ));
                }
            }
        }
        if let Some(kw_var) = kw_var {
            match self.lower_expr(kw_var.expr, None) {
                Ok(expr) => hir_args.kw_var = Some(Box::new(hir::PosArg::new(expr))),
                Err((expr, es)) => {
                    errs.extend(es);
                    let dummy = expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()));
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
                        *guard.target.clone(),
                        self.module.context.complement(&guard.to),
                    );
                    self.replace_or_push_guard(guard);
                }
                _ => {}
            },
            Type::And(tys, _) => {
                for ty in tys {
                    self.push_guard(nth, kind, ty);
                }
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

    pub(crate) fn lower_call(
        &mut self,
        call: ast::Call,
        expect: Option<&Type>,
    ) -> Failable<hir::Call> {
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
            Err((obj, es)) => {
                if pushed {
                    self.module.context.higher_order_caller.pop();
                }
                errs.extend(es);
                obj.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
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
        let mut hir_args = self.lower_args(call.args, expect_subr, &mut errs);
        let mut vi = match self.module.context.get_call_t(
            &obj,
            &call.attr_name,
            &mut hir_args.pos_args,
            &hir_args.kw_args,
            (hir_args.var_args.as_deref(), hir_args.kw_var.as_deref()),
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
        if let Some((guard, ret)) = guard.zip(vi.t.return_t()) {
            debug_assert!(
                self.module.context.subtype_of(ret, &Type::Bool),
                "{ret} is not a subtype of Bool",
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
                    if let Some(ref_mut_t) = obj.ref_mut_t() {
                        *ref_mut_t = vi.t;
                    }
                }
            } else if let Some(ref_mut_t) = obj.ref_mut_t() {
                *ref_mut_t = vi.t;
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
        if errs.is_empty() {
            Ok(call)
        } else {
            Err((call, errs))
        }
    }

    /// importing is done in [preregister](https://github.com/erg-lang/erg/blob/ffd33015d540ff5a0b853b28c01370e46e0fcc52/crates/erg_compiler/context/register.rs#L819)
    fn exec_additional_op(&mut self, call: &mut hir::Call) -> LowerResult<()> {
        match call.additional_operation() {
            Some(OperationKind::Del) => match call.args.get_left_or_key("obj") {
                Some(hir::Expr::Accessor(hir::Accessor::Ident(ident))) => {
                    self.module.context.del(ident)?;
                    Ok(())
                }
                other => {
                    let expr = other.map_or("nothing", |expr| expr.name());
                    Err(LowerErrors::from(LowerError::syntax_error(
                        self.input().clone(),
                        line!() as usize,
                        other.loc(),
                        self.module.context.caused_by(),
                        format!("expected identifier, but found {expr}"),
                        None,
                    )))
                }
            },
            Some(OperationKind::Return | OperationKind::Yield) => {
                // (f: ?T -> ?U).return: (self: Subroutine, arg: Obj) -> Never
                let callable_t = call.obj.ref_t();
                let ret_t = match callable_t {
                    Type::Subr(subr) => *subr.return_t.clone(),
                    Type::FreeVar(fv) if fv.get_sub().is_some() => {
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
                    let test = call.args.get_left_or_key("test");
                    let test_args = if let Some(hir::Expr::Call(call)) = test {
                        Some(&call.args)
                    } else {
                        None
                    };
                    self.module
                        .context
                        .cast(guard.clone(), test_args, &mut vec![])?;
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

    fn lower_pack(&mut self, pack: ast::DataPack, _expect: Option<&Type>) -> Failable<hir::Call> {
        log!(info "entered {}({pack})", fn_name!());
        let mut errors = LowerErrors::empty();
        let class = match self.lower_expr(*pack.class, None) {
            Ok(class) => class,
            Err((class, es)) => {
                errors.extend(es);
                class.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
            }
        };
        let args = match self.lower_record(pack.args, None) {
            Ok(args) => args,
            Err((args, es)) => {
                errors.extend(es);
                args
            }
        };
        let mut args = vec![hir::PosArg::new(hir::Expr::Record(args))];
        let attr_name = ast::Identifier::new(
            VisModifierSpec::Public(
                Token::new(
                    TokenKind::Dot,
                    Str::ever("."),
                    pack.connector.ln_begin().unwrap_or(0),
                    pack.connector.col_begin().unwrap_or(0),
                )
                .loc(),
            ),
            ast::VarName::new(Token::new(
                TokenKind::Symbol,
                Str::ever("new"),
                pack.connector.ln_begin().unwrap_or(0),
                pack.connector.col_begin().unwrap_or(0),
            )),
        );
        let vi = match self.module.context.get_call_t(
            &class,
            &Some(attr_name.clone()),
            &mut args,
            &[],
            (None, None),
            &self.cfg.input,
            &self.module.context,
        ) {
            Ok(vi) => vi,
            Err((vi, errs)) => {
                errors.extend(errs);
                vi.unwrap_or(VarInfo::ILLEGAL)
            }
        };
        let args = hir::Args::pos_only(args, None);
        let attr_name = hir::Identifier::new(attr_name, None, vi);
        let call = hir::Call::new(class, Some(attr_name), args);
        if errors.is_empty() {
            Ok(call)
        } else {
            Err((call, errors))
        }
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
    ) -> Failable<hir::TypeSpecWithOp> {
        match self.fake_lower_expr(*type_spec_with_op.t_spec_as_expr.clone()) {
            Ok(expr) => Ok(hir::TypeSpecWithOp::new(type_spec_with_op, expr, spec_t)),
            Err(es) => {
                let expr = hir::Expr::Dummy(hir::Dummy::empty());
                let t_spec = hir::TypeSpecWithOp::new(type_spec_with_op, expr, spec_t);
                Err((t_spec, es))
            }
        }
    }

    /// lower and assign params
    fn lower_params(
        &mut self,
        params: ast::Params,
        bounds: ast::TypeBoundSpecs,
        expect: Option<&SubrType>,
    ) -> Failable<hir::Params> {
        log!(info "entered {}({})", fn_name!(), params);
        let mut errs = LowerErrors::empty();
        let mut tmp_tv_ctx = match self
            .module
            .context
            .instantiate_ty_bounds(&bounds, RegistrationMode::Normal)
        {
            Ok(tv_ctx) => tv_ctx,
            Err((tv_ctx, es)) => {
                errs.extend(es);
                tv_ctx
            }
        };
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
        for (n, default) in params.defaults.into_iter().enumerate() {
            let default_t =
                expect.and_then(|subr| subr.default_params.get(n).and_then(|pt| pt.default_typ()));
            match self.lower_expr(default.default_val, default_t) {
                Ok(default_val) => {
                    let sig = match self.lower_non_default_param(default.sig) {
                        Ok(sig) => sig,
                        Err(es) => {
                            errs.extend(es);
                            continue;
                        }
                    };
                    if let Some(expect) = expect.and_then(|subr| subr.default_params.get(n)) {
                        if !self
                            .module
                            .context
                            .subtype_of(default_val.ref_t(), expect.typ())
                        {
                            let err = LowerError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                default_val.loc(),
                                self.module.context.caused_by(),
                                sig.inspect().unwrap_or(&"_".into()),
                                None,
                                expect.typ(),
                                default_val.ref_t(),
                                None,
                                None,
                            );
                            errs.push(err);
                        }
                    }
                    hir_defaults.push(hir::DefaultParamSignature::new(sig, default_val));
                }
                Err((_default, es)) => errs.extend(es),
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
        if let Err(es) =
            self.module
                .context
                .assign_params(&mut hir_params, &mut tmp_tv_ctx, expect.cloned())
        {
            errs.extend(es);
        }
        for guard in params.guards.into_iter() {
            let lowered = match guard {
                ast::GuardClause::Bind(bind) => self
                    .lower_def(bind, None)
                    .map(hir::GuardClause::Bind)
                    .map_err(|(def, es)| (def.map(hir::GuardClause::Bind), es)),
                // TODO: no fake
                ast::GuardClause::Condition(cond) => self
                    .fake_lower_expr(cond)
                    .map(hir::GuardClause::Condition)
                    .map_err(|es| (None, es)),
            };
            match lowered {
                Ok(guard) => {
                    if hir_params.guards.contains(&guard) {
                        log!(err "duplicate guard: {guard}");
                        continue;
                    }
                    hir_params.guards.push(guard)
                }
                Err((guard, es)) => {
                    if let Some(guard) = guard {
                        hir_params.guards.push(guard);
                    }
                    errs.extend(es)
                }
            }
        }
        if !errs.is_empty() {
            Err((hir_params, errs))
        } else {
            Ok(hir_params)
        }
    }

    fn lower_lambda(
        &mut self,
        lambda: ast::Lambda,
        expect: Option<&Type>,
    ) -> Failable<hir::Lambda> {
        let mut errors = LowerErrors::empty();
        let expect = expect.and_then(|t| <&SubrType>::try_from(t).ok());
        let return_t = expect.map(|subr| subr.return_t.as_ref());
        log!(info "entered {}({lambda})", fn_name!());
        let in_statement = PYTHON_MODE
            && self
                .module
                .context
                .control_kind()
                .is_some_and(|k| k.makes_scope());
        let is_procedural = lambda.is_procedural();
        let id = lambda.id.0;
        let name = format!("<lambda_{id}>");
        let kind = if is_procedural {
            ContextKind::LambdaProc(self.module.context.control_kind())
        } else {
            ContextKind::LambdaFunc(self.module.context.control_kind())
        };
        let tv_cache = match self
            .module
            .context
            .instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal)
        {
            Ok(tv_cache) => tv_cache,
            Err((tv_cache, es)) => {
                errors.extend(es);
                tv_cache
            }
        };
        if !in_statement {
            self.module
                .context
                .grow(&name, kind, Private, Some(tv_cache));
        }
        let params = match self.lower_params(lambda.sig.params, lambda.sig.bounds, expect) {
            Ok(params) => params,
            Err((params, es)) => {
                errors.extend(es);
                params
            }
        };
        let overwritten = {
            let mut overwritten = vec![];
            let guards = if in_statement {
                mem::take(&mut self.module.context.guards)
            } else {
                mem::take(&mut self.module.context.get_mut_outer().unwrap().guards)
            };
            for guard in guards.into_iter() {
                if let Err(errs) = self.module.context.cast(guard, None, &mut overwritten) {
                    errors.extend(errs);
                }
            }
            overwritten
        };
        if let Err(errs) = self.module.context.register_defs(&lambda.body) {
            errors.extend(errs);
        }
        let body = match self.lower_block(lambda.body, return_t) {
            Ok(body) => body,
            Err((body, es)) => {
                errors.extend(es);
                body
            }
        };
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
        let non_default_params = params
            .non_defaults
            .iter()
            .map(|param| {
                ParamTy::pos_or_kw(
                    param.name().map(|n| n.inspect().clone()),
                    param.vi.t.clone(),
                )
            })
            .collect::<Vec<_>>();
        let default_params = params
            .defaults
            .iter()
            .map(|param| {
                ParamTy::kw(
                    param.name().unwrap().inspect().clone(),
                    param.sig.vi.t.clone(),
                )
            })
            .collect::<Vec<_>>();
        let var_params = params.var_params.as_ref().map(|param| {
            ParamTy::pos_or_kw(
                param.name().map(|n| n.inspect().clone()),
                param
                    .vi
                    .t
                    .inner_ts()
                    .first()
                    .map_or(Type::Obj, |t| t.clone()),
            )
        });
        let kw_var_params = params
            .kw_var_params
            .as_ref()
            .map(|param| ParamTy::kw(param.name().unwrap().inspect().clone(), param.vi.t.clone()));
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
                non_default_params,
                var_params,
                default_params,
                kw_var_params,
                body.t(),
            )
        } else {
            func(
                non_default_params,
                var_params,
                default_params,
                kw_var_params,
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
        let lambda = hir::Lambda::new(
            id,
            params,
            lambda.op,
            return_t_spec,
            captured_names,
            body,
            t,
        );
        if !errors.is_empty() {
            Err((lambda, errors))
        } else {
            Ok(lambda)
        }
    }

    fn lower_def(&mut self, def: ast::Def, expect_body: Option<&Type>) -> FailableOption<hir::Def> {
        log!(info "entered {}({})", fn_name!(), def.sig);
        let mut errors = LowerErrors::empty();
        if def.def_kind().is_class_or_trait() && self.module.context.kind != ContextKind::Module {
            self.module
                .context
                .decls
                .remove(def.sig.ident().unwrap().inspect());
            let err = LowerError::inner_typedef_error(
                self.cfg.input.clone(),
                line!() as usize,
                def.loc(),
                self.module.context.caused_by(),
            );
            errors.push(err);
        }
        let name = if let Some(name) = def.sig.name_as_str() {
            name.clone()
        } else {
            Str::ever("<lambda>")
        };
        if ERG_MODE && (&name[..] == "module" || &name[..] == "global") {
            let err = LowerError::shadow_special_namespace_error(
                self.cfg.input.clone(),
                line!() as usize,
                def.sig.loc(),
                self.module.context.caused_by(),
                &name,
            );
            errors.push(err);
        } else if self
            .module
            .context
            .registered_info(&name, def.sig.is_const())
            .is_some()
            && def.sig.vis().is_private()
        {
            let err = LowerError::reassign_error(
                self.cfg.input.clone(),
                line!() as usize,
                def.sig.loc(),
                self.module.context.caused_by(),
                &name,
            );
            errors.push(err);
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
        let vis = match self.module.context.instantiate_vis_modifier(def.sig.vis()) {
            Ok(vis) => vis,
            Err(errs) => {
                errors.extend(errs);
                VisibilityModifier::Public
            }
        };
        let res = match def.sig {
            ast::Signature::Subr(sig) => {
                let tv_cache = match self
                    .module
                    .context
                    .instantiate_ty_bounds(&sig.bounds, RegistrationMode::Normal)
                {
                    Ok(tv_cache) => tv_cache,
                    Err((tv_cache, errs)) => {
                        errors.extend(errs);
                        tv_cache
                    }
                };
                self.module.context.grow(&name, kind, vis, Some(tv_cache));
                self.lower_subr_def(sig, def.body)
                    .map_err(|(def, es)| (Some(def), errors.concat(es)))
            }
            ast::Signature::Var(sig) => {
                self.module.context.grow(&name, kind, vis, None);
                self.lower_var_def(sig, def.body, expect_body)
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
        expect_body: Option<&Type>,
    ) -> FailableOption<hir::Def> {
        log!(info "entered {}({sig})", fn_name!());
        let mut errors = LowerErrors::empty();
        if let Err(errs) = self.module.context.register_defs(&body.block) {
            errors.extend(errs);
        }
        let outer = self.module.context.outer.as_ref().unwrap();
        let existing_vi = sig
            .ident()
            .and_then(|ident| outer.get_current_scope_var(&ident.name))
            .cloned();
        let existing_t = existing_vi.as_ref().map(|vi| vi.t.clone());
        let expect_body_t = sig
            .t_spec
            .as_ref()
            .map(|t_spec| {
                match self
                    .module
                    .context
                    .instantiate_var_sig_t(Some(&t_spec.t_spec), RegistrationMode::PreRegister)
                {
                    Ok(t) => t,
                    // REVIEW:
                    Err((t, _errs)) => t,
                }
            })
            .or_else(|| {
                sig.ident()
                    .and_then(|ident| outer.get_current_param_or_decl(ident.inspect()))
                    .map(|vi| vi.t.clone())
            });
        match self.lower_block(body.block, expect_body.or(expect_body_t.as_ref())) {
            Ok(block) => {
                let found_body_t = block.ref_t();
                let ident = match &sig.pat {
                    ast::VarPattern::Ident(ident) | ast::VarPattern::Phi(ident) => ident.clone(),
                    ast::VarPattern::Discard(token) => {
                        ast::Identifier::private_from_token(token.clone())
                    }
                    ast::VarPattern::Glob(token) => {
                        return self
                            .lower_glob(token.clone(), hir::DefBody::new(body.op, block, body.id))
                            .map_err(|errs| (None, errors.concat(errs)));
                    }
                    other => {
                        log!(err "unexpected pattern: {other}");
                        return unreachable_error!(LowerErrors, LowerError, self.module.context)
                            .map_err(|errs| (None, errors.concat(errs)));
                    }
                };
                let mut no_reassign = false;
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
                            errors.push(e);
                            no_reassign = true;
                        }
                    }
                }
                let found_body_t = if sig.is_phi() {
                    self.module
                        .context
                        .union(existing_t.as_ref().unwrap_or(&Type::Never), found_body_t)
                } else {
                    found_body_t.clone()
                };
                let vi = if no_reassign {
                    VarInfo {
                        t: found_body_t,
                        ..existing_vi.unwrap_or_default()
                    }
                } else {
                    match self.module.context.outer.as_mut().unwrap().assign_var_sig(
                        &sig,
                        &found_body_t,
                        body.id,
                        block.last(),
                        None,
                    ) {
                        Ok(vi) => vi,
                        Err(errs) => {
                            errors.extend(errs);
                            VarInfo::ILLEGAL
                        }
                    }
                };
                let ident = hir::Identifier::new(ident, None, vi);
                let t_spec = if let Some(ts) = sig.t_spec {
                    let spec_t = match self.module.context.instantiate_typespec(&ts.t_spec) {
                        Ok(spec_t) => spec_t,
                        Err((spec_t, errs)) => {
                            errors.extend(errs);
                            spec_t
                        }
                    };
                    let expr = match self.fake_lower_expr(*ts.t_spec_as_expr.clone()) {
                        Ok(expr) => expr,
                        Err(errs) => {
                            errors.extend(errs);
                            hir::Expr::Dummy(hir::Dummy::empty())
                        }
                    };
                    Some(hir::TypeSpecWithOp::new(*ts, expr, spec_t))
                } else {
                    None
                };
                let sig = hir::VarSignature::new(ident, t_spec);
                let body = hir::DefBody::new(body.op, block, body.id);
                let def = hir::Def::new(hir::Signature::Var(sig), body);
                if errors.is_empty() {
                    Ok(def)
                } else {
                    Err((Some(def), errors))
                }
            }
            Err((block, errs)) => {
                errors.extend(errs);
                let found_body_t = block.ref_t();
                let ident = match &sig.pat {
                    ast::VarPattern::Ident(ident) | ast::VarPattern::Phi(ident) => ident.clone(),
                    ast::VarPattern::Discard(token) => {
                        ast::Identifier::private_from_token(token.clone())
                    }
                    ast::VarPattern::Glob(token) => {
                        return self
                            .lower_glob(token.clone(), hir::DefBody::new(body.op, block, body.id))
                            .map_err(|errs| (None, errors.concat(errs)));
                    }
                    other => {
                        log!(err "unexpected pattern: {other}");
                        return unreachable_error!(LowerErrors, LowerError, self.module.context)
                            .map_err(|errs| (None, errors.concat(errs)));
                    }
                };
                let found_body_t = if sig.is_phi() {
                    self.module
                        .context
                        .union(existing_t.as_ref().unwrap_or(&Type::Never), found_body_t)
                } else {
                    found_body_t.clone()
                };
                if let Err(errs) = self.module.context.outer.as_mut().unwrap().assign_var_sig(
                    &sig,
                    &found_body_t,
                    ast::DefId(0),
                    None,
                    None,
                ) {
                    errors.extend(errs);
                }
                let ident = hir::Identifier::new(ident, None, VarInfo::ILLEGAL);
                let sig = hir::VarSignature::new(ident, None);
                let body = hir::DefBody::new(body.op, block, body.id);
                let def = hir::Def::new(hir::Signature::Var(sig), body);
                Err((Some(def), errors))
            }
        }
    }

    fn lower_glob(&mut self, token: Token, body: hir::DefBody) -> LowerResult<hir::Def> {
        let names = vec![];
        if let Some(path) = body.ref_t().module_path() {
            if let Some(module) = self.module.context.get_mod_with_path(&path) {
                for (name, vi) in module.local_dir().cloned() {
                    self.module
                        .context
                        .get_mut_outer()
                        .unwrap()
                        .locals
                        .insert(name, vi);
                }
            }
        }
        let vis = VisibilityModifier::Public;
        let sig = hir::Signature::Glob(hir::GlobSignature::new(token, vis, names, body.t()));
        Ok(hir::Def::new(sig, body))
    }

    // NOTE: Note that this is in the inner scope while being called.
    fn lower_subr_def(
        &mut self,
        sig: ast::SubrSignature,
        body: ast::DefBody,
    ) -> Failable<hir::Def> {
        log!(info "entered {}({sig})", fn_name!());
        let mut errors = LowerErrors::empty();
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
            let deco = match self.lower_expr(deco.0.clone(), Some(&mono("Subroutine"))) {
                Ok(deco) => deco,
                Err((Some(deco), es)) => {
                    errors.extend(es);
                    deco
                }
                Err((None, es)) => {
                    errors.extend(es);
                    continue;
                }
            };
            decorators.insert(deco);
        }
        match registered_t {
            Type::Subr(subr_t) => self.lower_subr_block(subr_t, sig, decorators, body),
            quant @ Type::Quantified(_) => {
                let instance = match self.module.context.instantiate_dummy(quant) {
                    Ok(instance) => instance,
                    Err(errs) => {
                        errors.extend(errs);
                        Type::Failure
                    }
                };
                let subr_t = SubrType::try_from(instance).unwrap_or(SubrType::failed());
                self.lower_subr_block(subr_t, sig, decorators, body)
            }
            _ => {
                let params = match self.lower_params(sig.params, sig.bounds.clone(), None) {
                    Ok(params) => params,
                    Err((params, errs)) => {
                        errors.extend(errs);
                        params
                    }
                };
                if let Err(errs) = self.module.context.register_defs(&body.block) {
                    errors.extend(errs);
                }
                if let Err(es) = self
                    .module
                    .context
                    .outer
                    .as_mut()
                    .unwrap()
                    .fake_subr_assign(&sig.ident, &sig.decorators, Type::Failure)
                {
                    errors.extend(es);
                }
                let block = match self.lower_block(body.block, None) {
                    Ok(block) => block,
                    Err((block, errs)) => {
                        errors.extend(errs);
                        block
                    }
                };
                let ident = hir::Identifier::bare(sig.ident);
                let ret_t_spec = if let Some(ts) = sig.return_t_spec {
                    let spec_t = match self.module.context.instantiate_typespec(&ts.t_spec) {
                        Ok(spec_t) => spec_t,
                        Err((spec_t, errs)) => {
                            errors.extend(errs);
                            spec_t
                        }
                    };
                    let expr = match self.fake_lower_expr(*ts.t_spec_as_expr.clone()) {
                        Ok(expr) => expr,
                        Err(errs) => {
                            errors.extend(errs);
                            hir::Expr::Dummy(hir::Dummy::empty())
                        }
                    };
                    Some(hir::TypeSpecWithOp::new(*ts, expr, spec_t))
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
                let def = hir::Def::new(hir::Signature::Subr(sig), body);
                if errors.is_empty() {
                    Ok(def)
                } else {
                    Err((def, errors))
                }
            }
        }
    }

    fn lower_subr_block(
        &mut self,
        registered_subr_t: SubrType,
        sig: ast::SubrSignature,
        decorators: Set<hir::Expr>,
        body: ast::DefBody,
    ) -> Failable<hir::Def> {
        let mut errors = LowerErrors::empty();
        let params = match self.lower_params(
            sig.params.clone(),
            sig.bounds.clone(),
            Some(&registered_subr_t),
        ) {
            Ok(params) => params,
            Err((params, errs)) => {
                errors.extend(errs);
                params
            }
        };
        if let Err(errs) = self.module.context.register_defs(&body.block) {
            errors.extend(errs);
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
                        errors.extend(errs);
                        vi
                    }
                };
                let ident = hir::Identifier::new(sig.ident, None, vi);
                let ret_t_spec = if let Some(ts) = sig.return_t_spec {
                    let spec_t = match self.module.context.instantiate_typespec(&ts.t_spec) {
                        Ok(spec_t) => spec_t,
                        Err((spec_t, errs)) => {
                            errors.extend(errs);
                            spec_t
                        }
                    };
                    let expr = match self.fake_lower_expr(*ts.t_spec_as_expr.clone()) {
                        Ok(expr) => expr,
                        Err(errs) => {
                            errors.extend(errs);
                            hir::Expr::Dummy(hir::Dummy::empty())
                        }
                    };
                    Some(hir::TypeSpecWithOp::new(*ts, expr, spec_t))
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
                let def = hir::Def::new(hir::Signature::Subr(sig), body);
                if errors.is_empty() {
                    Ok(def)
                } else {
                    Err((def, errors))
                }
            }
            Err((block, errs)) => {
                errors.extend(errs);
                let found_body_t = self.module.context.squash_tyvar(block.t());
                let vi = match self.module.context.outer.as_mut().unwrap().assign_subr(
                    &sig,
                    ast::DefId(0),
                    &params,
                    &found_body_t,
                    &sig,
                ) {
                    Ok(vi) => vi,
                    Err((errs, vi)) => {
                        errors.extend(errs);
                        vi
                    }
                };
                let ident = hir::Identifier::new(sig.ident, None, vi);
                let ret_t_spec = if let Some(ts) = sig.return_t_spec {
                    let spec_t = match self.module.context.instantiate_typespec(&ts.t_spec) {
                        Ok(spec_t) => spec_t,
                        Err((spec_t, errs)) => {
                            errors.extend(errs);
                            spec_t
                        }
                    };
                    let expr = match self.fake_lower_expr(*ts.t_spec_as_expr.clone()) {
                        Ok(expr) => expr,
                        Err(errs) => {
                            errors.extend(errs);
                            hir::Expr::Dummy(hir::Dummy::empty())
                        }
                    };
                    Some(hir::TypeSpecWithOp::new(*ts, expr, spec_t))
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
                let def = hir::Def::new(hir::Signature::Subr(sig), body);
                if errors.is_empty() {
                    Ok(def)
                } else {
                    Err((def, errors))
                }
            }
        }
    }

    fn lower_class_def(&mut self, class_def: ast::ClassDef) -> Failable<hir::ClassDef> {
        log!(info "entered {}({class_def})", fn_name!());
        let mut errors = LowerErrors::empty();
        let (is_inherit, sig, mut block) = match self.lower_def(class_def.def, None) {
            Ok(def) => (def.def_kind().is_inherit(), def.sig, def.body.block),
            Err((Some(def), errs)) => {
                errors.extend(errs);
                (false, def.sig, def.body.block)
            }
            Err((None, errs)) => {
                errors.extend(errs);
                let var = hir::VarSignature::new(hir::Identifier::public("<error>"), None);
                (false, hir::Signature::Var(var), hir::Block::empty())
            }
        };
        let mut hir_methods_list = vec![];
        let mut implemented = set! {};
        for methods in class_def.methods_list.into_iter() {
            let mut hir_methods = hir::Block::empty();
            let (class, impl_trait) =
                match self.module.context.get_class_and_impl_trait(&methods.class) {
                    Ok(x) => x,
                    Err((class, trait_, errs)) => {
                        errors.extend(errs);
                        (class.unwrap_or(Type::Obj), trait_)
                    }
                };
            if let Some(class_root) = self.module.context.get_nominal_type_ctx(&class) {
                if !class_root.kind.is_class() {
                    let err = LowerError::method_definition_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        methods.loc(),
                        self.module.context.caused_by(),
                        &class.qual_name(),
                        None,
                    );
                    errors.push(err);
                }
            } else {
                let err = LowerError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    methods.class.loc(),
                    self.module.context.caused_by(),
                    &class.qual_name(),
                    self.module.context.get_similar_name(&class.local_name()),
                );
                errors.push(err);
            }
            let Some(methods_list) = self
                .module
                .context
                .get_mut_nominal_type_ctx(&class)
                .map(|ctx| &mut ctx.methods_list)
            else {
                let err = LowerError::unreachable(
                    self.cfg.input.clone(),
                    erg_common::fn_name!(),
                    line!(),
                );
                errors.push(err);
                continue;
            };
            let methods_idx = methods_list.iter().position(|m| m.id == methods.id);
            // We can't remove the method ctx here because methods' information must be hold
            let Some(methods_ctx) = methods_idx.map(|idx| methods_list[idx].clone()) else {
                let err = LowerError::unreachable(
                    self.cfg.input.clone(),
                    erg_common::fn_name!(),
                    line!(),
                );
                errors.push(err);
                continue;
            };
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
                    ast::ClassAttr::Def(def) => match self.lower_def(def, None) {
                        Ok(def) => {
                            hir_methods.push(hir::Expr::Def(def));
                        }
                        Err((def, errs)) => {
                            if let Some(def) = def {
                                hir_methods.push(hir::Expr::Def(def));
                            }
                            errors.extend(errs);
                        }
                    },
                    ast::ClassAttr::Decl(decl) => match self.lower_type_asc(decl, None) {
                        Ok(decl) => {
                            hir_methods.push(hir::Expr::TypeAsc(decl));
                        }
                        Err((tasc, errs)) => {
                            hir_methods.push(hir::Expr::TypeAsc(tasc));
                            errors.extend(errs);
                        }
                    },
                    ast::ClassAttr::Doc(doc) => match self.lower_literal(doc, None) {
                        Ok(doc) => {
                            hir_methods.push(hir::Expr::Literal(doc));
                        }
                        Err(errs) => {
                            errors.extend(errs);
                        }
                    },
                }
            }
            if let Some(methods_list) = self
                .module
                .context
                .get_mut_nominal_type_ctx(&class)
                .map(|ctx| &mut ctx.methods_list)
            {
                if let Some(idx) = methods_idx {
                    methods_list.remove(idx);
                }
            }
            if let Err(errs) = self.module.context.check_decls() {
                errors.extend(errs);
            }
            if let Some((trait_, _)) = &impl_trait {
                self.check_override(&class, Some(trait_));
            } else {
                self.check_override(&class, None);
            }
            if let Err(err) = self.check_trait_impl(impl_trait.clone(), &class, &mut implemented) {
                errors.push(err);
            }
            let impl_trait = impl_trait.map(|(t, _)| t);
            self.check_collision_and_push(methods.id, class.clone(), impl_trait.clone());
            hir_methods_list.push(hir::Methods::new(class, impl_trait, hir_methods));
        }
        let class = self.module.context.gen_type(&sig.ident().raw);
        let class_ctx = if let Some(ctx) = self.module.context.get_nominal_type_ctx(&class) {
            Some(ctx)
        } else {
            let err = LowerError::type_not_found(
                self.cfg.input.clone(),
                line!() as usize,
                sig.loc(),
                self.module.context.caused_by(),
                &class,
            );
            errors.push(err);
            None
        };
        let type_obj = match self
            .module
            .context
            .rec_get_const_obj(sig.ident().inspect())
            .cloned()
        {
            Some(ValueObj::Type(TypeObj::Generated(type_obj))) => type_obj,
            _ => GenTypeObj::class(Type::Failure, None, None, false),
        };
        if is_inherit {
            let call = match block.first() {
                Some(hir::Expr::Call(call)) => Some(call),
                _ => None,
            };
            if let Some(sup_type) = call.and_then(|call| call.args.get_left_or_key("Super")) {
                if let Err(err) = self.check_inheritable(&type_obj, sup_type, &sig) {
                    errors.extend(err);
                }
            }
        }
        let constructor = class_ctx
            .and_then(|ctx| {
                ctx.get_class_member(&VarName::from_static("__call__"), &self.module.context)
            })
            .map_or(Type::FAILURE, |vi| &vi.t);
        let need_to_gen_new = class_ctx
            .and_then(|ctx| ctx.get_current_scope_var(&VarName::from_static("new")))
            .is_some_and(|vi| vi.kind == VarKind::Auto);
        let require_or_sup = Self::get_require_or_sup_or_base(block.remove(0));
        let class_def = hir::ClassDef::new(
            type_obj,
            sig,
            require_or_sup,
            need_to_gen_new,
            constructor.clone(),
            hir_methods_list,
        );
        if errors.is_empty() {
            Ok(class_def)
        } else {
            Err((class_def, errors))
        }
    }

    fn lower_patch_def(&mut self, class_def: ast::PatchDef) -> FailableOption<hir::PatchDef> {
        log!(info "entered {}({class_def})", fn_name!());
        let mut errors = LowerErrors::empty();
        let base_t = {
            let Some(ast::Expr::Call(call)) = class_def.def.body.block.get(0) else {
                return unreachable_error!(LowerErrors, LowerError, self)
                    .map_err(|errs| (None, errors.concat(errs)));
            };
            let base_t_expr = call.args.get_left_or_key("Base").unwrap();
            let spec = Parser::expr_to_type_spec(base_t_expr.clone()).unwrap();
            match self.module.context.instantiate_typespec(&spec) {
                Ok(t) => t,
                Err((t, errs)) => {
                    errors.extend(errs);
                    t
                }
            }
        };
        let mut hir_def = match self.lower_def(class_def.def, None) {
            Ok(def) => def,
            Err((_def, errs)) => {
                errors.extend(errs);
                return Err((None, errors));
            }
        };
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
                            def.sig.ident_mut().unwrap().vis = VisModifierSpec::Public(
                                Token::new(
                                    TokenKind::Dot,
                                    ".",
                                    def.sig.ln_begin().unwrap_or(0),
                                    def.sig.col_begin().unwrap_or(0),
                                )
                                .loc(),
                            );
                        }
                        if let Err(es) = self.module.context.register_def(def) {
                            errors.extend(es);
                        }
                    }
                    ast::ClassAttr::Decl(_) | ast::ClassAttr::Doc(_) => {}
                }
            }
            for attr in methods.attrs.into_iter() {
                match attr {
                    ast::ClassAttr::Def(def) => match self.lower_def(def, None) {
                        Ok(def) => {
                            hir_methods.push(hir::Expr::Def(def));
                        }
                        Err((def, errs)) => {
                            if let Some(def) = def {
                                hir_methods.push(hir::Expr::Def(def));
                            }
                            errors.extend(errs);
                        }
                    },
                    ast::ClassAttr::Decl(decl) => match self.lower_type_asc(decl, None) {
                        Ok(decl) => {
                            hir_methods.push(hir::Expr::TypeAsc(decl));
                        }
                        Err((tasc, errs)) => {
                            hir_methods.push(hir::Expr::TypeAsc(tasc));
                            errors.extend(errs);
                        }
                    },
                    ast::ClassAttr::Doc(doc) => match self.lower_literal(doc, None) {
                        Ok(doc) => {
                            hir_methods.push(hir::Expr::Literal(doc));
                        }
                        Err(errs) => {
                            errors.extend(errs);
                        }
                    },
                }
            }
            if let Err(errs) = self.module.context.check_decls() {
                errors.extend(errs);
            }
            self.push_patch(methods.id);
        }
        let patch = hir::PatchDef::new(hir_def.sig, base, hir_methods);
        if errors.is_empty() {
            Ok(patch)
        } else {
            Err((Some(patch), errors))
        }
    }

    fn lower_redef(&mut self, redef: ast::ReDef) -> FailableOption<hir::Expr> {
        log!(info "entered {}({redef})", fn_name!());
        let mut errors = LowerErrors::empty();
        let mut attr = match self.lower_acc(redef.attr, None) {
            Ok(attr) => attr,
            Err((hir::Accessor::Ident(ident), _errs)) => {
                let pat = ast::VarPattern::Ident(ident.raw);
                let sig = ast::Signature::Var(ast::VarSignature::new(pat, None));
                let body = ast::DefBody::new_single(*redef.expr);
                let def = ast::Def::new(sig, body);
                return self
                    .lower_def(def, None)
                    .map(hir::Expr::Def)
                    .map_err(|(def, errs)| (def.map(hir::Expr::Def), errors.concat(errs)));
            }
            Err((acc, errs)) => {
                errors.extend(errs);
                acc
            }
        };
        let expr = match self.lower_expr(*redef.expr, None) {
            Ok(expr) => expr,
            Err((expr, errs)) => {
                errors.extend(errs);
                expr.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
            }
        };
        if let Err(err) =
            self.var_result_t_check(&attr, &Str::from(attr.show()), attr.ref_t(), expr.ref_t())
        {
            if PYTHON_MODE {
                if !self.widen_type(&mut attr, &expr) {
                    errors.push(err);
                }
            } else {
                errors.push(err);
            }
        }
        let redef = hir::ReDef::new(attr, hir::Block::new(vec![expr]));
        if errors.is_empty() {
            Ok(hir::Expr::ReDef(redef))
        } else {
            Err((Some(hir::Expr::ReDef(redef)), errors))
        }
    }

    fn widen_type(&mut self, attr: &mut hir::Accessor, expr: &hir::Expr) -> bool {
        for sup in self
            .module
            .context
            .get_super_classes_or_self(attr.ref_t())
            .skip(1)
        {
            if sup == Type::Obj {
                break;
            }
            if self
                .var_result_t_check(attr, &Str::from(attr.show()), &sup, expr.ref_t())
                .is_ok()
            {
                if let Some(attr_t) = attr.ref_mut_t() {
                    *attr_t = sup.clone();
                }
                if let hir::Accessor::Ident(ident) = &attr {
                    if let Some(vi) = self
                        .module
                        .context
                        .rec_get_mut_var_info(&ident.raw, AccessKind::Name)
                    {
                        vi.t = sup;
                    }
                }
                return true;
            }
        }
        false
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
                            &from_str(&sup.name), // TODO: get super type
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
        implemented: &mut Set<Type>,
    ) -> SingleLowerResult<()> {
        if let Some((impl_trait, t_spec)) = impl_trait {
            if let Some(mut sups) = self.module.context.get_super_traits(&impl_trait) {
                let outer = self.module.context.get_outer_scope().unwrap();
                let trait_ctx = outer.get_nominal_type_ctx(&impl_trait);
                let external_trait =
                    trait_ctx.map_or(true, |tr| !tr.name.starts_with(&outer.name[..]));
                if sups.any(|t| t == mono("Sealed")) && external_trait {
                    return Err(LowerError::sealed_trait_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        t_spec.loc(),
                        self.module.context.caused_by(),
                        &impl_trait.qual_name(),
                    ));
                }
            }
            let impl_trait = impl_trait.normalize();
            let (unverified_names, mut errors) = if let Some(typ_ctx) = self
                .module
                .context
                .get_outer_scope()
                .unwrap()
                .get_nominal_type_ctx(&impl_trait)
            {
                self.check_methods_compatibility(&impl_trait, class, typ_ctx, t_spec, implemented)
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
            if !PYTHON_MODE {
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
        implemented: &mut Set<Type>,
    ) -> (Set<&VarName>, CompileErrors) {
        let mut errors = CompileErrors::empty();
        let mut unverified_names = self.module.context.locals.keys().collect::<Set<_>>();
        let mut super_impls = set! {};
        let retained_decls = |ctx: &Context, super_impls: &Set<&VarName>| {
            ctx.decls.clone().retained(|k, _| {
                let implemented_in_super = super_impls.contains(k);
                let class_decl = ctx.kind.is_class();
                !implemented_in_super && !class_decl
            })
        };
        let tys_decls = if self.module.context.is_class(trait_type) {
            vec![(impl_trait.clone(), retained_decls(trait_ctx, &super_impls))]
        } else if let Some(sups) = self.module.context.get_super_types(trait_type) {
            sups.map(|sup| {
                if implemented.linear_contains(&sup) {
                    return (sup, Dict::new());
                }
                let decls =
                    self.module
                        .context
                        .get_nominal_type_ctx(&sup)
                        .map_or(Dict::new(), |ctx| {
                            super_impls.extend(ctx.locals.keys());
                            for methods in &ctx.methods_list {
                                super_impls.extend(methods.locals.keys());
                            }
                            retained_decls(ctx, &super_impls)
                        });
                (sup, decls)
            })
            .collect::<Vec<_>>()
        } else {
            vec![(impl_trait.clone(), retained_decls(trait_ctx, &super_impls))]
        };
        for (impl_trait, decls) in tys_decls {
            for (decl_name, decl_vi) in decls {
                if let Some((name, vi)) = self.module.context.get_var_kv(decl_name.inspect()) {
                    let def_t = &vi.t;
                    let replaced_decl_t = decl_vi
                        .t
                        .clone()
                        .replace(trait_type, &impl_trait)
                        .replace(&impl_trait, class);
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
                            &impl_trait,
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
                        &impl_trait,
                        class,
                        None,
                        t_spec.loc(),
                    ));
                }
            }
        }
        implemented.insert(trait_type.clone());
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
                Some("Inherit" | "Subsume") => call.args.remove_left_or_key("Super"),
                Some("Inheritable") => {
                    Self::get_require_or_sup_or_base(call.args.remove_left_or_key("Class")?)
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
    ) -> Failable<hir::TypeAscription> {
        log!(info "entered {}({tasc})", fn_name!());
        let mut errors = LowerErrors::empty();
        let kind = tasc.kind();
        let spec_t = match self
            .module
            .context
            .instantiate_typespec(&tasc.t_spec.t_spec)
        {
            Ok(spec_t) => spec_t,
            Err((t, errs)) => {
                errors.extend(errs);
                t
            }
        };
        let expect = expect.map_or(Some(&spec_t), |exp| {
            self.module.context.min(exp, &spec_t).ok().or(Some(&spec_t))
        });
        let expr = match self.lower_expr(*tasc.expr, expect) {
            Ok(expr) => expr,
            Err((exp, errs)) => {
                errors.extend(errs);
                exp.unwrap_or(hir::Expr::Dummy(hir::Dummy::empty()))
            }
        };
        match kind {
            AscriptionKind::TypeOf | AscriptionKind::AsCast => {
                if let Err(es) = self.module.context.sub_unify(
                    expr.ref_t(),
                    &spec_t,
                    &expr,
                    Some(&Str::from(expr.to_string())),
                ) {
                    errors.extend(es);
                }
            }
            AscriptionKind::SubtypeOf => {
                if let Ok(ctxs) = self
                    .module
                    .context
                    .get_singular_ctxs_by_hir_expr(&expr, &self.module.context)
                {
                    let ctx = ctxs.first().unwrap();
                    // REVIEW: need to use subtype_of?
                    if ctx.super_traits.iter().all(|trait_| trait_ != &spec_t)
                        && ctx.super_classes.iter().all(|class| class != &spec_t)
                    {
                        let errs = LowerError::subtyping_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            expr.ref_t(), // FIXME:
                            &spec_t,
                            Location::concat(&expr, &tasc.t_spec),
                            self.module.context.caused_by(),
                        );
                        errors.push(errs);
                    }
                } else {
                    errors.push(LowerError::no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        expr.loc(),
                        self.module.context.caused_by(),
                        &expr.to_string(),
                        None,
                    ));
                };
            }
            _ => {}
        }
        let t_spec = match self.lower_type_spec_with_op(tasc.t_spec, spec_t) {
            Ok(t_spec) => t_spec,
            Err((t_spec, errs)) => {
                errors.extend(errs);
                t_spec
            }
        };
        let tasc = expr.type_asc(t_spec);
        if errors.is_empty() {
            Ok(tasc)
        } else {
            Err((tasc, errors))
        }
    }

    fn lower_decl(&mut self, tasc: ast::TypeAscription) -> LowerResult<hir::TypeAscription> {
        log!(info "entered {}({tasc})", fn_name!());
        let mut errors = LowerErrors::empty();
        let kind = tasc.kind();
        let spec_t = match self
            .module
            .context
            .instantiate_typespec(&tasc.t_spec.t_spec)
        {
            Ok(spec_t) => spec_t,
            Err((t, errs)) => {
                errors.extend(errs);
                t
            }
        };
        let ast::Expr::Accessor(ast::Accessor::Ident(ident)) = *tasc.expr else {
            return Err(LowerErrors::from(LowerError::syntax_error(
                self.cfg.input.clone(),
                line!() as usize,
                tasc.expr.loc(),
                self.module.context.caused_by(),
                switch_lang!(
                    "japanese" => "無効な型宣言です(左辺には変数のみ使用出来ます)".to_string(),
                    "simplified_chinese" => "无效的类型声明".to_string(),
                    "traditional_chinese" => "無效的型宣告".to_string(),
                    "english" => "Invalid type declaration (currently only variables are allowed at LHS)".to_string(),
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
            .and_then(|ctxs| ctxs.first().map(|ctx| ctx.name.clone()));
        let ident = hir::Identifier::new(ident, qual_name, ident_vi);
        let expr = hir::Expr::Accessor(hir::Accessor::Ident(ident));
        let t_spec = match self.lower_type_spec_with_op(tasc.t_spec, spec_t) {
            Ok(t_spec) => t_spec,
            Err((t_spec, errs)) => {
                errors.extend(errs);
                t_spec
            }
        };
        let tasc = expr.type_asc(t_spec);
        if errors.is_empty() {
            Ok(tasc)
        } else {
            Err(errors)
        }
    }

    // Call.obj == Accessor cannot be type inferred by itself (it can only be inferred with arguments)
    // so turn off type checking (check=false)
    fn lower_expr(&mut self, expr: ast::Expr, expect: Option<&Type>) -> FailableOption<hir::Expr> {
        log!(info "entered {}", fn_name!());
        let casted = self.module.context.get_casted_type(&expr);
        let mut expr = match expr {
            ast::Expr::Literal(lit) => {
                hir::Expr::Literal(self.lower_literal(lit, expect).map_err(|es| (None, es))?)
            }
            ast::Expr::List(lis) => hir::Expr::List(
                self.lower_list(lis, expect)
                    .map_err(|(list, es)| (list.map(hir::Expr::List), es))?,
            ),
            ast::Expr::Tuple(tup) => hir::Expr::Tuple(
                self.lower_tuple(tup, expect)
                    .map_err(|(tuple, es)| (Some(hir::Expr::Tuple(tuple)), es))?,
            ),
            ast::Expr::Record(rec) => hir::Expr::Record(
                self.lower_record(rec, expect)
                    .map_err(|(rec, es)| (Some(hir::Expr::Record(rec)), es))?,
            ),
            ast::Expr::Set(set) => hir::Expr::Set(
                self.lower_set(set, expect)
                    .map_err(|(set, es)| (set.map(hir::Expr::Set), es))?,
            ),
            ast::Expr::Dict(dict) => hir::Expr::Dict(
                self.lower_dict(dict, expect)
                    .map_err(|(dict, es)| (dict.map(hir::Expr::Dict), es))?,
            ),
            ast::Expr::Accessor(acc) => hir::Expr::Accessor(
                self.lower_acc(acc, expect)
                    .map_err(|(acc, errs)| (Some(hir::Expr::Accessor(acc)), errs))?,
            ),
            ast::Expr::BinOp(bin) => hir::Expr::BinOp(
                self.lower_bin(bin, expect)
                    .map_err(|(bin, es)| (Some(hir::Expr::BinOp(bin)), es))?,
            ),
            ast::Expr::UnaryOp(unary) => hir::Expr::UnaryOp(
                self.lower_unary(unary, expect)
                    .map_err(|(un, es)| (Some(hir::Expr::UnaryOp(un)), es))?,
            ),
            ast::Expr::Call(call) => hir::Expr::Call(
                self.lower_call(call, expect)
                    .map_err(|(call, es)| (Some(hir::Expr::Call(call)), es))?,
            ),
            ast::Expr::DataPack(pack) => hir::Expr::Call(
                self.lower_pack(pack, expect)
                    .map_err(|(pack, es)| (Some(hir::Expr::Call(pack)), es))?,
            ),
            ast::Expr::Lambda(lambda) => hir::Expr::Lambda(
                self.lower_lambda(lambda, expect)
                    .map_err(|(lambda, es)| (Some(hir::Expr::Lambda(lambda)), es))?,
            ),
            ast::Expr::TypeAscription(tasc) => hir::Expr::TypeAsc(
                self.lower_type_asc(tasc, expect)
                    .map_err(|(tasc, es)| (Some(hir::Expr::TypeAsc(tasc)), es))?,
            ),
            ast::Expr::Compound(comp) => hir::Expr::Compound(
                self.lower_compound(comp, expect)
                    .map_err(|(cmp, es)| (Some(hir::Expr::Compound(cmp)), es))?,
            ),
            // Checking is also performed for expressions in Dummy. However, it has no meaning in code generation
            ast::Expr::Dummy(dummy) => hir::Expr::Dummy(
                self.lower_dummy(dummy, expect)
                    .map_err(|(dummy, es)| (Some(hir::Expr::Dummy(dummy)), es))?,
            ),
            ast::Expr::InlineModule(inline) => hir::Expr::Call(
                self.lower_inline_module(inline, expect)
                    .map_err(|(call, es)| (Some(hir::Expr::Call(call)), es))?,
            ),
            other => {
                log!(err "unreachable: {other}");
                return unreachable_error!(LowerErrors, LowerError, self.module.context)
                    .map_err(|errs| (None, errs));
            }
        };
        if let Some(casted) = casted {
            // e.g. casted == {x: Obj | x != None}, expr: Int or NoneType => intersec == Int
            let intersec = self.module.context.intersection(expr.ref_t(), &casted);
            // bad narrowing: C and Structural { foo = Foo }
            if expr.ref_t().is_proj()
                || (intersec != Type::Never && intersec.ands().iter().all(|t| !t.is_structural()))
            {
                if let Some(ref_mut_t) = expr.ref_mut_t() {
                    *ref_mut_t = intersec;
                }
            }
        }
        // debug_assert!(expr.ref_t().has_no_qvar(), "{expr} has qvar");
        Ok(expr)
    }

    /// The meaning of TypeAscription changes between chunk and expr.
    /// For example, `x: Int`, as expr, is `x` itself,
    /// but as chunk, it declares that `x` is of type `Int`, and is valid even before `x` is defined.
    pub(crate) fn lower_chunk(
        &mut self,
        chunk: ast::Expr,
        expect: Option<&Type>,
    ) -> FailableOption<hir::Expr> {
        log!(info "entered {}", fn_name!());
        match chunk {
            ast::Expr::Def(def) => Ok(hir::Expr::Def(
                self.lower_def(def, None)
                    .map_err(|(def, es)| (def.map(hir::Expr::Def), es))?,
            )),
            ast::Expr::ClassDef(defs) => Ok(hir::Expr::ClassDef(
                self.lower_class_def(defs)
                    .map_err(|(def, es)| (Some(hir::Expr::ClassDef(def)), es))?,
            )),
            ast::Expr::PatchDef(defs) => Ok(hir::Expr::PatchDef(
                self.lower_patch_def(defs)
                    .map_err(|(def, es)| (def.map(hir::Expr::PatchDef), es))?,
            )),
            ast::Expr::ReDef(redef) => Ok(self.lower_redef(redef)?),
            ast::Expr::TypeAscription(tasc) => Ok(hir::Expr::TypeAsc(
                self.lower_decl(tasc).map_err(|es| (None, es))?,
            )),
            other => self.lower_expr(other, expect),
        }
    }

    pub fn lower_and_resolve_chunk(
        &mut self,
        chunk: ast::Expr,
        expect: Option<&Type>,
    ) -> FailableOption<hir::Expr> {
        match self.lower_chunk(chunk, expect) {
            Ok(mut chunk) => {
                let _ = self.module.context.resolve_expr_t(&mut chunk, &set! {});
                Ok(chunk)
            }
            Err((Some(mut chunk), errs)) => {
                let _ = self.module.context.resolve_expr_t(&mut chunk, &set! {});
                Err((Some(chunk), errs))
            }
            Err((None, errs)) => Err((None, errs)),
        }
    }

    fn lower_block(
        &mut self,
        ast_block: ast::Block,
        expect: Option<&Type>,
    ) -> Failable<hir::Block> {
        log!(info "entered {}", fn_name!());
        let mut errors = LowerErrors::empty();
        let mut hir_block = Vec::with_capacity(ast_block.len());
        let last = ast_block.len().saturating_sub(1);
        for (i, chunk) in ast_block.into_iter().enumerate() {
            let expect = if i == last { expect } else { None };
            match self.lower_chunk(chunk, expect) {
                Ok(chunk) => hir_block.push(chunk),
                Err((chunk, errs)) => {
                    if let Some(chunk) = chunk {
                        hir_block.push(chunk);
                    }
                    errors.extend(errs);
                }
            };
        }
        let block = hir::Block::new(hir_block);
        if errors.is_empty() {
            Ok(block)
        } else {
            Err((block, errors))
        }
    }

    fn lower_compound(
        &mut self,
        compound: ast::Compound,
        expect: Option<&Type>,
    ) -> Failable<hir::Block> {
        log!(info "entered {}", fn_name!());
        let mut hir_block = Vec::with_capacity(compound.len());
        let mut errors = LowerErrors::empty();
        let last = compound.len().saturating_sub(1);
        for (i, chunk) in compound.into_iter().enumerate() {
            let expect = if i == last { expect } else { None };
            match self.lower_chunk(chunk, expect) {
                Ok(chunk) => hir_block.push(chunk),
                Err((Some(chunk), errs)) => {
                    hir_block.push(chunk);
                    errors.extend(errs);
                }
                Err((None, errs)) => errors.extend(errs),
            }
        }
        if errors.is_empty() {
            Ok(hir::Block::new(hir_block))
        } else {
            Err((hir::Block::new(hir_block), errors))
        }
    }

    fn lower_dummy(
        &mut self,
        ast_dummy: ast::Dummy,
        expect: Option<&Type>,
    ) -> Failable<hir::Dummy> {
        log!(info "entered {}", fn_name!());
        let mut hir_dummy = Vec::with_capacity(ast_dummy.len());
        let mut errors = LowerErrors::empty();
        let last = ast_dummy.len().saturating_sub(1);
        for (i, chunk) in ast_dummy.into_iter().enumerate() {
            let expect = if i == last { expect } else { None };
            match self.lower_chunk(chunk, expect) {
                Ok(chunk) => hir_dummy.push(chunk),
                Err((Some(chunk), errs)) => {
                    hir_dummy.push(chunk);
                    errors.extend(errs);
                }
                Err((None, errs)) => errors.extend(errs),
            }
        }
        if errors.is_empty() {
            Ok(hir::Dummy::new(hir_dummy))
        } else {
            Err((hir::Dummy::new(hir_dummy), errors))
        }
    }

    pub(crate) fn lower_inline_module(
        &mut self,
        inline: InlineModule,
        expect: Option<&Type>,
    ) -> Failable<hir::Call> {
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
        let ast = match ASTLinker::new(self.cfg.clone()).link(ast, mode) {
            Ok(ast) => ast,
            Err((ast, errs)) => {
                self.errs.extend(errs);
                ast
            }
        };
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
        if let Err(errs) = self.module.context.preregister_consts(ast.module.block()) {
            self.errs.extend(errs);
        }
        if let Err(errs) = self.module.context.register_defs(ast.module.block()) {
            self.errs.extend(errs);
        }
        for chunk in ast.module.into_iter() {
            match self.lower_chunk(chunk, None) {
                Ok(chunk) => {
                    module.push(chunk);
                }
                Err((chunk, errs)) => {
                    if let Some(chunk) = chunk {
                        module.push(chunk);
                    }
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
                self.module
                    .context
                    .shared()
                    .promises
                    .join_children(&self.cfg);
            } else {
                self.module.context.shared().promises.join_all(&self.cfg);
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
