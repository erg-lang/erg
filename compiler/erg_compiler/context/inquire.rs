// (type) getters & validators
use std::option::Option; // conflicting to Type::Option
use std::path::{Path, PathBuf};

use erg_common::config::Input;
use erg_common::error::{ErrorCore, ErrorKind, Location};
use erg_common::levenshtein::get_similar_name;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::{Field, Visibility};
use erg_common::{enum_unwrap, fmt_option, fmt_slice, log, set};
use erg_common::{option_enum_unwrap, Str};
use Type::*;

use ast::VarName;
use erg_parser::ast::{self, Identifier};
use erg_parser::token::Token;

use erg_type::constructors::{builtin_mono, func, module, mono, mono_proj, v_enum};
use erg_type::typaram::TyParam;
use erg_type::value::{GenTypeObj, TypeObj, ValueObj};
use erg_type::{HasType, ParamTy, SubrKind, SubrType, TyBound, Type};

use crate::context::instantiate::ConstTemplate;
use crate::context::{Context, RegistrationMode, TraitInstance, Variance};
use crate::error::{
    binop_to_dname, readable_name, unaryop_to_dname, SingleTyCheckResult, TyCheckError,
    TyCheckErrors, TyCheckResult,
};
use crate::hir;
use crate::varinfo::VarInfo;
use RegistrationMode::*;
use Visibility::*;

impl Context {
    pub(crate) fn validate_var_sig_t(
        &self,
        ident: &ast::Identifier,
        t_spec: Option<&ast::TypeSpec>,
        body_t: &Type,
        mode: RegistrationMode,
    ) -> TyCheckResult<()> {
        let spec_t = self.instantiate_var_sig_t(t_spec, None, mode)?;
        if self
            .sub_unify(body_t, &spec_t, None, Some(ident.loc()), None)
            .is_err()
        {
            return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                self.cfg.input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                &spec_t,
                body_t,
                self.get_candidates(body_t),
                self.get_type_mismatch_hint(&spec_t, body_t),
            )));
        }
        Ok(())
    }

    pub(crate) fn get_current_scope_var(&self, name: &str) -> Option<&VarInfo> {
        self.locals
            .get(name)
            .or_else(|| self.decls.get(name))
            .or_else(|| {
                self.params
                    .iter()
                    .find(|(opt_name, _)| {
                        opt_name
                            .as_ref()
                            .map(|n| &n.inspect()[..] == name)
                            .unwrap_or(false)
                    })
                    .map(|(_, vi)| vi)
            })
            .or_else(|| {
                for (_, methods) in self.methods_list.iter() {
                    if let Some(vi) = methods.get_current_scope_var(name) {
                        return Some(vi);
                    }
                }
                None
            })
    }

    pub(crate) fn get_local_kv(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.locals.get_key_value(name)
    }

    pub fn get_singular_ctx(
        &self,
        obj: &hir::Expr,
        namespace: &Str,
    ) -> SingleTyCheckResult<&Context> {
        match obj {
            hir::Expr::Accessor(hir::Accessor::Ident(ident)) => {
                self.get_singular_ctx_from_ident(&ident.clone().downcast(), namespace)
            }
            hir::Expr::Accessor(hir::Accessor::Attr(attr)) => {
                // REVIEW: 両方singularとは限らない?
                let ctx = self.get_singular_ctx(&attr.obj, namespace)?;
                let attr = hir::Expr::Accessor(hir::Accessor::Ident(attr.ident.clone()));
                ctx.get_singular_ctx(&attr, namespace)
            }
            _ => todo!(),
        }
    }

    pub fn get_singular_ctx_from_ident(
        &self,
        ident: &ast::Identifier,
        namespace: &Str,
    ) -> SingleTyCheckResult<&Context> {
        self.get_mod(ident)
            .or_else(|| self.rec_get_type(ident.inspect()).map(|(_, ctx)| ctx))
            .ok_or_else(|| {
                TyCheckError::no_var_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    ident.loc(),
                    namespace.into(),
                    ident.inspect(),
                    self.get_similar_name(ident.inspect()),
                )
            })
    }

    pub fn get_mut_singular_ctx_from_ident(
        &mut self,
        ident: &ast::Identifier,
        namespace: &Str,
    ) -> SingleTyCheckResult<&mut Context> {
        let err = TyCheckError::no_var_error(
            self.cfg.input.clone(),
            line!() as usize,
            ident.loc(),
            namespace.into(),
            ident.inspect(),
            self.get_similar_name(ident.inspect()),
        );
        self.get_mut_type(ident.inspect())
            .map(|(_, ctx)| ctx)
            .ok_or(err)
    }

    pub fn get_mut_singular_ctx(
        &mut self,
        obj: &ast::Expr,
        namespace: &Str,
    ) -> SingleTyCheckResult<&mut Context> {
        match obj {
            ast::Expr::Accessor(ast::Accessor::Ident(ident)) => {
                self.get_mut_singular_ctx_from_ident(ident, namespace)
            }
            ast::Expr::Accessor(ast::Accessor::Attr(attr)) => {
                // REVIEW: 両方singularとは限らない?
                let ctx = self.get_mut_singular_ctx(&attr.obj, namespace)?;
                let attr = ast::Expr::Accessor(ast::Accessor::Ident(attr.ident.clone()));
                ctx.get_mut_singular_ctx(&attr, namespace)
            }
            _ => todo!(),
        }
    }

    fn get_match_call_t(
        &self,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<Type> {
        if !kw_args.is_empty() {
            todo!()
        }
        for pos_arg in pos_args.iter().skip(1) {
            let t = pos_arg.expr.ref_t();
            // Allow only anonymous functions to be passed as match arguments (for aesthetic reasons)
            if !matches!(&pos_arg.expr, hir::Expr::Lambda(_)) {
                return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    pos_arg.loc(),
                    self.caused_by(),
                    "match",
                    &builtin_mono("LambdaFunc"),
                    t,
                    self.get_candidates(t),
                    self.get_type_mismatch_hint(&builtin_mono("LambdaFunc"), t),
                )));
            }
        }
        let match_target_expr_t = pos_args[0].expr.ref_t();
        // Never or T => T
        let mut union_pat_t = Type::Never;
        for (i, pos_arg) in pos_args.iter().skip(1).enumerate() {
            let lambda = erg_common::enum_unwrap!(&pos_arg.expr, hir::Expr::Lambda);
            if !lambda.params.defaults.is_empty() {
                todo!()
            }
            // TODO: If the first argument of the match is a tuple?
            if lambda.params.len() != 1 {
                return Err(TyCheckErrors::from(TyCheckError::argument_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    pos_args[i + 1].loc(),
                    self.caused_by(),
                    1,
                    pos_args[i + 1]
                        .expr
                        .signature_t()
                        .unwrap()
                        .typarams_len()
                        .unwrap_or(0),
                )));
            }
            let rhs = self.instantiate_param_sig_t(
                &lambda.params.non_defaults[0],
                None,
                &mut None,
                Normal,
            )?;
            union_pat_t = self.union(&union_pat_t, &rhs);
        }
        // NG: expr_t: Nat, union_pat_t: {1, 2}
        // OK: expr_t: Int, union_pat_t: {1} or 'T
        if self
            .sub_unify(match_target_expr_t, &union_pat_t, None, None, None)
            .is_err()
        {
            return Err(TyCheckErrors::from(TyCheckError::match_error(
                self.cfg.input.clone(),
                line!() as usize,
                pos_args[0].loc(),
                self.caused_by(),
                match_target_expr_t,
            )));
        }
        let branch_ts = pos_args
            .iter()
            .skip(1)
            .map(|a| ParamTy::anonymous(a.expr.ref_t().clone()))
            .collect::<Vec<_>>();
        let mut return_t = branch_ts[0].typ().return_t().unwrap().clone();
        for arg_t in branch_ts.iter().skip(1) {
            return_t = self.union(&return_t, arg_t.typ().return_t().unwrap());
        }
        let param_ty = ParamTy::anonymous(match_target_expr_t.clone());
        let param_ts = [vec![param_ty], branch_ts.to_vec()].concat();
        let t = func(param_ts, None, vec![], return_t);
        Ok(t)
    }

    fn get_import_call_t(
        &self,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<Type> {
        let mod_name = pos_args
            .get(0)
            .map(|a| &a.expr)
            .or_else(|| {
                kw_args
                    .iter()
                    .find(|k| &k.keyword.inspect()[..] == "Path")
                    .map(|a| &a.expr)
            })
            .unwrap();
        let path = match mod_name {
            hir::Expr::Lit(lit) => {
                if self.subtype_of(&lit.value.class(), &Str) {
                    enum_unwrap!(&lit.value, ValueObj::Str)
                } else {
                    return Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        mod_name.loc(),
                        self.caused_by(),
                        "import::name",
                        &Str,
                        mod_name.ref_t(),
                        self.get_candidates(mod_name.ref_t()),
                        self.get_type_mismatch_hint(&Str, mod_name.ref_t()),
                    )));
                }
            }
            _other => {
                return Err(TyCheckErrors::from(TyCheckError::feature_error(
                    self.cfg.input.clone(),
                    mod_name.loc(),
                    "non-literal importing",
                    self.caused_by(),
                )))
            }
        };
        let path = PathBuf::from(&path[..]);
        let s = ValueObj::Str(Str::rc(path.to_str().unwrap()));
        let import_t = func(
            vec![ParamTy::anonymous(v_enum(set! {s.clone()}))],
            None,
            vec![],
            module(TyParam::Value(s)),
        );
        Ok(import_t)
    }

    pub(crate) fn rec_get_var_t(
        &self,
        ident: &Identifier,
        input: &Input,
        namespace: &Str,
    ) -> SingleTyCheckResult<Type> {
        if let Some(vi) = self.get_current_scope_var(&ident.inspect()[..]) {
            self.validate_visibility(ident, vi, input, namespace)?;
            Ok(vi.t())
        } else {
            if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
                return parent.rec_get_var_t(ident, input, namespace);
            }
            Err(TyCheckError::no_var_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                namespace.into(),
                ident.inspect(),
                self.get_similar_name(ident.inspect()),
            ))
        }
    }

    pub(crate) fn rec_get_attr_t(
        &self,
        obj: &hir::Expr,
        ident: &Identifier,
        input: &Input,
        namespace: &Str,
    ) -> SingleTyCheckResult<Type> {
        let self_t = obj.t();
        let name = ident.name.token();
        match self.get_attr_t_from_attributive(obj, &self_t, ident, namespace) {
            Ok(t) => {
                return Ok(t);
            }
            Err(e) if e.core.kind == ErrorKind::DummyError => {}
            Err(e) => {
                return Err(e);
            }
        }
        if let Ok(singular_ctx) = self.get_singular_ctx(obj, namespace) {
            match singular_ctx.rec_get_var_t(ident, input, namespace) {
                Ok(t) => {
                    return Ok(t);
                }
                Err(e) if e.core.kind == ErrorKind::NameError => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }
        for (_, ctx) in self.get_nominal_super_type_ctxs(&self_t).ok_or_else(|| {
            TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                obj.loc(),
                self.caused_by(),
                &self_t.to_string(),
                None, // TODO:
            )
        })? {
            match ctx.rec_get_var_t(ident, input, namespace) {
                Ok(t) => {
                    return Ok(t);
                }
                Err(e) if e.core.kind == ErrorKind::NameError => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
            parent.rec_get_attr_t(obj, ident, input, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                input.clone(),
                line!() as usize,
                name.loc(),
                namespace.into(),
                &self_t,
                name.inspect(),
                self.get_similar_attr(&self_t, name.inspect()),
            ))
        }
    }

    /// get type from given attributive type (Record).
    /// not ModuleType or ClassType etc.
    fn get_attr_t_from_attributive(
        &self,
        obj: &hir::Expr,
        t: &Type,
        ident: &Identifier,
        namespace: &Str,
    ) -> SingleTyCheckResult<Type> {
        match t {
            Type::FreeVar(fv) if fv.is_linked() => {
                self.get_attr_t_from_attributive(obj, &fv.crack(), ident, namespace)
            }
            Type::FreeVar(fv) => {
                let sup = fv.get_sup().unwrap();
                self.get_attr_t_from_attributive(obj, &sup, ident, namespace)
            }
            Type::Ref(t) => self.get_attr_t_from_attributive(obj, t, ident, namespace),
            Type::RefMut { before, .. } => {
                self.get_attr_t_from_attributive(obj, before, ident, namespace)
            }
            Type::Refinement(refine) => {
                self.get_attr_t_from_attributive(obj, &refine.t, ident, namespace)
            }
            Type::Record(record) => {
                // REVIEW: `rec.get(name.inspect())` returns None (Borrow<Str> is implemented for Field). Why?
                if let Some(attr) = record.get(&Field::new(Public, ident.inspect().clone())) {
                    Ok(attr.clone())
                } else {
                    let t = Type::Record(record.clone());
                    Err(TyCheckError::no_attr_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        ident.loc(),
                        namespace.into(),
                        &t,
                        ident.inspect(),
                        self.get_similar_attr(&t, ident.inspect()),
                    ))
                }
            }
            other => {
                if let Some(v) = self.rec_get_const_obj(&other.name()) {
                    match v {
                        ValueObj::Type(TypeObj::Generated(gen)) => self
                            .get_gen_t_require_attr_t(gen, &ident.inspect()[..])
                            .cloned()
                            .ok_or_else(|| {
                                TyCheckError::dummy(self.cfg.input.clone(), line!() as usize)
                            }),
                        ValueObj::Type(TypeObj::Builtin(_t)) => {
                            // FIXME:
                            Err(TyCheckError::dummy(
                                self.cfg.input.clone(),
                                line!() as usize,
                            ))
                        }
                        other => todo!("{other}"),
                    }
                } else {
                    Err(TyCheckError::dummy(
                        self.cfg.input.clone(),
                        line!() as usize,
                    ))
                }
            }
        }
    }

    /// 戻り値ではなく、call全体の型を返す
    fn search_callee_t(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Identifier>,
        input: &Input,
        namespace: &Str,
    ) -> SingleTyCheckResult<Type> {
        if let Some(method_name) = method_name.as_ref() {
            for (_, ctx) in self
                .get_nominal_super_type_ctxs(obj.ref_t())
                .ok_or_else(|| {
                    TyCheckError::no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        obj.loc(),
                        self.caused_by(),
                        &obj.to_string(),
                        None, // TODO:
                    )
                })?
            {
                if let Some(vi) = ctx
                    .locals
                    .get(method_name.inspect())
                    .or_else(|| ctx.decls.get(method_name.inspect()))
                {
                    self.validate_visibility(method_name, vi, input, namespace)?;
                    return Ok(vi.t());
                }
                for (_, methods_ctx) in ctx.methods_list.iter() {
                    if let Some(vi) = methods_ctx
                        .locals
                        .get(method_name.inspect())
                        .or_else(|| methods_ctx.decls.get(method_name.inspect()))
                    {
                        self.validate_visibility(method_name, vi, input, namespace)?;
                        return Ok(vi.t());
                    }
                }
            }
            if let Ok(singular_ctx) = self.get_singular_ctx(obj, namespace) {
                if let Some(vi) = singular_ctx
                    .locals
                    .get(method_name.inspect())
                    .or_else(|| singular_ctx.decls.get(method_name.inspect()))
                {
                    self.validate_visibility(method_name, vi, input, namespace)?;
                    return Ok(vi.t());
                }
                for (_, method_ctx) in singular_ctx.methods_list.iter() {
                    if let Some(vi) = method_ctx
                        .locals
                        .get(method_name.inspect())
                        .or_else(|| method_ctx.decls.get(method_name.inspect()))
                    {
                        self.validate_visibility(method_name, vi, input, namespace)?;
                        return Ok(vi.t());
                    }
                }
                return Err(TyCheckError::singular_no_attr_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    method_name.loc(),
                    namespace.into(),
                    obj.__name__().unwrap_or("?"),
                    obj.ref_t(),
                    method_name.inspect(),
                    self.get_similar_attr_from_singular(obj, method_name.inspect()),
                ));
            }
            match self.rec_get_method_traits(method_name) {
                Ok(trait_) => {
                    let (_, ctx) = self.get_nominal_type_ctx(trait_).unwrap();
                    return ctx.rec_get_var_t(method_name, input, namespace);
                }
                Err(err) if err.core.kind == ErrorKind::TypeError => {
                    return Err(err);
                }
                _ => {}
            }
            // TODO: patch
            Err(TyCheckError::no_attr_error(
                self.cfg.input.clone(),
                line!() as usize,
                method_name.loc(),
                namespace.into(),
                obj.ref_t(),
                method_name.inspect(),
                self.get_similar_attr(obj.ref_t(), method_name.inspect()),
            ))
        } else {
            Ok(obj.t())
        }
    }

    fn validate_visibility(
        &self,
        ident: &Identifier,
        vi: &VarInfo,
        input: &Input,
        namespace: &str,
    ) -> SingleTyCheckResult<()> {
        if ident.vis() != vi.vis {
            Err(TyCheckError::visibility_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                vi.vis,
            ))
        // check if the private variable is loaded from the other scope
        } else if vi.vis.is_private()
            && &self.name[..] != "<builtins>"
            && &self.name[..] != namespace
            && !namespace.contains(&self.name[..])
        {
            Err(TyCheckError::visibility_error(
                input.clone(),
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                Private,
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn get_binop_t(
        &self,
        op: &Token,
        args: &[hir::PosArg],
        input: &Input,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        erg_common::debug_power_assert!(args.len() == 2);
        let cont = binop_to_dname(op.inspect());
        let symbol = Token::new(op.kind, Str::rc(cont), op.lineno, op.col_begin);
        let t = self.rec_get_var_t(
            &Identifier::new(None, VarName::new(symbol.clone())),
            input,
            namespace,
        )?;
        let op = hir::Expr::Accessor(hir::Accessor::private(symbol, t));
        self.get_call_t(&op, &None, args, &[], input, namespace)
            .map_err(|errs| {
                let op = enum_unwrap!(op, hir::Expr::Accessor:(hir::Accessor::Ident:(_)));
                let lhs = args[0].expr.clone();
                let rhs = args[1].expr.clone();
                let bin = hir::BinOp::new(op.name.into_token(), lhs, rhs, op.t);
                TyCheckErrors::new(
                    errs.into_iter()
                        .map(|e| {
                            // HACK: dname.loc()はダミーLocationしか返さないので、エラーならop.loc()で上書きする
                            let core = ErrorCore::new(
                                e.core.errno,
                                e.core.kind,
                                bin.loc(),
                                e.core.desc,
                                e.core.hint,
                            );
                            TyCheckError::new(core, self.cfg.input.clone(), e.caused_by)
                        })
                        .collect(),
                )
            })
    }

    pub(crate) fn get_unaryop_t(
        &self,
        op: &Token,
        args: &[hir::PosArg],
        input: &Input,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        erg_common::debug_power_assert!(args.len() == 1);
        let cont = unaryop_to_dname(op.inspect());
        let symbol = Token::new(op.kind, Str::rc(cont), op.lineno, op.col_begin);
        let t = self.rec_get_var_t(
            &Identifier::new(None, VarName::new(symbol.clone())),
            input,
            namespace,
        )?;
        let op = hir::Expr::Accessor(hir::Accessor::private(symbol, t));
        self.get_call_t(&op, &None, args, &[], input, namespace)
            .map_err(|errs| {
                let op = enum_unwrap!(op, hir::Expr::Accessor:(hir::Accessor::Ident:(_)));
                let expr = args[0].expr.clone();
                let unary = hir::UnaryOp::new(op.name.into_token(), expr, op.t);
                TyCheckErrors::new(
                    errs.into_iter()
                        .map(|e| {
                            let core = ErrorCore::new(
                                e.core.errno,
                                e.core.kind,
                                unary.loc(),
                                e.core.desc,
                                e.core.hint,
                            );
                            TyCheckError::new(core, self.cfg.input.clone(), e.caused_by)
                        })
                        .collect(),
                )
            })
    }

    /// 可変依存型の変更を伝搬させる
    fn propagate(&self, t: &Type, callee: &hir::Expr) -> TyCheckResult<()> {
        if let Type::Subr(subr) = t {
            if let Some(after) = subr.self_t().and_then(|self_t| {
                if let RefMut { after, .. } = self_t {
                    after.as_ref()
                } else {
                    None
                }
            }) {
                self.reunify(callee.ref_t(), after, Some(callee.loc()), None)?;
            }
        }
        Ok(())
    }

    /// e.g.
    /// ```python
    /// substitute_call(instance: ((?T, ?U) -> ?T), [Int, Str], []) => instance: (Int, Str) -> Int
    /// substitute_call(instance: ((?T, Int) -> ?T), [Int, Nat], []) => instance: (Int, Int) -> Str
    /// substitute_call(instance: ((?M(: Nat)..?N(: Nat)) -> ?M+?N), [1..2], []) => instance: (1..2) -> {3}
    /// substitute_call(instance: ((?L(: Add(?R, ?O)), ?R) -> ?O), [1, 2], []) => instance: (Nat, Nat) -> Nat
    /// ```
    fn substitute_call(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Identifier>,
        instance: &Type,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<()> {
        match instance {
            Type::FreeVar(fv) if fv.is_linked() => {
                self.substitute_call(obj, method_name, &fv.crack(), pos_args, kw_args)
            }
            Type::Refinement(refine) => {
                self.substitute_call(obj, method_name, &refine.t, pos_args, kw_args)
            }
            Type::Subr(subr) => {
                let callee = if let Some(ident) = method_name {
                    let attr = hir::Attribute::new(
                        obj.clone(),
                        hir::Identifier::bare(ident.dot.clone(), ident.name.clone()),
                        Type::Uninited,
                    );
                    hir::Expr::Accessor(hir::Accessor::Attr(attr))
                } else {
                    obj.clone()
                };
                let params_len = subr.non_default_params.len() + subr.default_params.len();
                if (params_len < pos_args.len() || params_len < pos_args.len() + kw_args.len())
                    && subr.var_params.is_none()
                {
                    return Err(TyCheckErrors::from(TyCheckError::too_many_args_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        callee.loc(),
                        &callee.to_string(),
                        self.caused_by(),
                        params_len,
                        pos_args.len(),
                        kw_args.len(),
                    )));
                }
                let mut passed_params = set! {};
                let non_default_params_len = if method_name.is_some() {
                    subr.non_default_params.len() - 1
                } else {
                    subr.non_default_params.len()
                };
                if pos_args.len() >= non_default_params_len {
                    let (non_default_args, var_args) = pos_args.split_at(non_default_params_len);
                    let non_default_params = if subr
                        .non_default_params
                        .first()
                        .map(|p| p.name().map(|s| &s[..]) == Some("self"))
                        .unwrap_or(false)
                    {
                        let mut non_default_params = subr.non_default_params.iter();
                        let self_pt = non_default_params.next().unwrap();
                        self.sub_unify(
                            obj.ref_t(),
                            self_pt.typ(),
                            Some(obj.loc()),
                            None,
                            self_pt.name(),
                        )?;
                        non_default_params
                    } else {
                        subr.non_default_params.iter()
                    };
                    for (nd_arg, nd_param) in non_default_args.iter().zip(non_default_params) {
                        self.substitute_pos_arg(
                            &callee,
                            &nd_arg.expr,
                            nd_param,
                            &mut passed_params,
                        )?;
                    }
                    if let Some(var_param) = subr.var_params.as_ref() {
                        for var_arg in var_args.iter() {
                            self.substitute_var_arg(&callee, &var_arg.expr, var_param)?;
                        }
                    } else {
                        for (arg, pt) in var_args.iter().zip(subr.default_params.iter()) {
                            self.substitute_pos_arg(&callee, &arg.expr, pt, &mut passed_params)?;
                        }
                    }
                    for kw_arg in kw_args.iter() {
                        self.substitute_kw_arg(
                            &callee,
                            kw_arg,
                            &subr.default_params,
                            &mut passed_params,
                        )?;
                    }
                } else {
                    let missing_len = subr.non_default_params.len() - pos_args.len();
                    let missing_params = subr
                        .non_default_params
                        .iter()
                        .rev()
                        .take(missing_len)
                        .rev()
                        .map(|pt| pt.name().cloned().unwrap_or(Str::ever("")))
                        .collect();
                    return Err(TyCheckErrors::from(TyCheckError::args_missing_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        callee.loc(),
                        &callee.to_string(),
                        self.caused_by(),
                        missing_len,
                        missing_params,
                    )));
                }
                Ok(())
            }
            other => {
                if let Some(method_name) = method_name {
                    Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        Location::concat(obj, method_name),
                        self.caused_by(),
                        &(obj.to_string() + &method_name.to_string()),
                        &builtin_mono("Callable"),
                        other,
                        self.get_candidates(other),
                        None,
                    )))
                } else {
                    Err(TyCheckErrors::from(TyCheckError::type_mismatch_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        obj.loc(),
                        self.caused_by(),
                        &obj.to_string(),
                        &builtin_mono("Callable"),
                        other,
                        self.get_candidates(other),
                        None,
                    )))
                }
            }
        }
    }

    fn substitute_pos_arg(
        &self,
        callee: &hir::Expr,
        arg: &hir::Expr,
        param: &ParamTy,
        passed_params: &mut Set<Str>,
    ) -> TyCheckResult<()> {
        let arg_t = arg.ref_t();
        let param_t = &param.typ();
        self.sub_unify(arg_t, param_t, Some(arg.loc()), None, param.name())
            .map_err(|errs| {
                log!(err "semi-unification failed with {callee}\n{arg_t} !<: {param_t}");
                // REVIEW:
                let name = callee.show_acc().unwrap_or_else(|| "".to_string());
                let name = name + "::" + param.name().map(|s| readable_name(&s[..])).unwrap_or("");
                TyCheckErrors::new(
                    errs.into_iter()
                        .map(|e| {
                            TyCheckError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                e.core.loc,
                                e.caused_by,
                                &name[..],
                                param_t,
                                arg_t,
                                self.get_candidates(arg_t),
                                self.get_type_mismatch_hint(param_t, arg_t),
                            )
                        })
                        .collect(),
                )
            })?;
        if let Some(name) = param.name() {
            if passed_params.contains(name) {
                return Err(TyCheckErrors::from(TyCheckError::multiple_args_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    callee.loc(),
                    &callee.to_string(),
                    self.caused_by(),
                    name,
                )));
            } else {
                passed_params.insert(name.clone());
            }
        }
        Ok(())
    }

    fn substitute_var_arg(
        &self,
        callee: &hir::Expr,
        arg: &hir::Expr,
        param: &ParamTy,
    ) -> TyCheckResult<()> {
        let arg_t = arg.ref_t();
        let param_t = &param.typ();
        self.sub_unify(arg_t, param_t, Some(arg.loc()), None, param.name())
            .map_err(|errs| {
                log!(err "semi-unification failed with {callee}\n{arg_t} !<: {param_t}");
                // REVIEW:
                let name = callee.show_acc().unwrap_or_else(|| "".to_string());
                let name = name + "::" + param.name().map(|s| readable_name(&s[..])).unwrap_or("");
                TyCheckErrors::new(
                    errs.into_iter()
                        .map(|e| {
                            TyCheckError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                e.core.loc,
                                e.caused_by,
                                &name[..],
                                param_t,
                                arg_t,
                                self.get_candidates(arg_t),
                                self.get_type_mismatch_hint(param_t, arg_t),
                            )
                        })
                        .collect(),
                )
            })
    }

    fn substitute_kw_arg(
        &self,
        callee: &hir::Expr,
        arg: &hir::KwArg,
        default_params: &[ParamTy],
        passed_params: &mut Set<Str>,
    ) -> TyCheckResult<()> {
        let arg_t = arg.expr.ref_t();
        let kw_name = arg.keyword.inspect();
        if passed_params.contains(&kw_name[..]) {
            return Err(TyCheckErrors::from(TyCheckError::multiple_args_error(
                self.cfg.input.clone(),
                line!() as usize,
                callee.loc(),
                &callee.to_string(),
                self.caused_by(),
                arg.keyword.inspect(),
            )));
        } else {
            passed_params.insert(kw_name.clone());
        }
        if let Some(pt) = default_params
            .iter()
            .find(|pt| pt.name().unwrap() == kw_name)
        {
            self.sub_unify(arg_t, pt.typ(), Some(arg.loc()), None, Some(kw_name))
                .map_err(|errs| {
                    log!(err "semi-unification failed with {callee}\n{arg_t} !<: {}", pt.typ());
                    // REVIEW:
                    let name = callee.show_acc().unwrap_or_else(|| "".to_string());
                    let name = name + "::" + readable_name(kw_name);
                    TyCheckErrors::new(
                        errs.into_iter()
                            .map(|e| {
                                TyCheckError::type_mismatch_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    e.core.loc,
                                    e.caused_by,
                                    &name[..],
                                    pt.typ(),
                                    arg_t,
                                    self.get_candidates(arg_t),
                                    self.get_type_mismatch_hint(pt.typ(), arg_t),
                                )
                            })
                            .collect(),
                    )
                })?;
        } else {
            return Err(TyCheckErrors::from(TyCheckError::unexpected_kw_arg_error(
                self.cfg.input.clone(),
                line!() as usize,
                arg.keyword.loc(),
                &callee.to_string(),
                self.caused_by(),
                kw_name,
            )));
        }
        Ok(())
    }

    pub(crate) fn get_call_t(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Identifier>,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        input: &Input,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        if let hir::Expr::Accessor(hir::Accessor::Ident(local)) = obj {
            if local.vis().is_private() {
                match &local.inspect()[..] {
                    "match" => {
                        return self.get_match_call_t(pos_args, kw_args);
                    }
                    "import" | "pyimport" | "py" => {
                        return self.get_import_call_t(pos_args, kw_args);
                    }
                    // handle assert casting
                    /*"assert" => {
                        if let Some(arg) = pos_args.first() {
                            match &arg.expr {
                                hir::Expr::BinOp(bin) if bin.op.is(TokenKind::InOp) && bin.rhs.ref_t() == &Type => {
                                    let t = self.eval_const_expr(bin.lhs.as_ref(), None)?.as_type().unwrap();
                                }
                                _ => {}
                            }
                        }
                    },*/
                    _ => {}
                }
            }
        }
        let found = self.search_callee_t(obj, method_name, input, namespace)?;
        log!(
            "Found:\ncallee: {obj}{}\nfound: {found}",
            fmt_option!(pre ".", method_name.as_ref().map(|ident| &ident.name))
        );
        let instance = self.instantiate(found, obj)?;
        log!(
            "Instantiated:\ninstance: {instance}\npos_args: ({})\nkw_args: ({})",
            fmt_slice(pos_args),
            fmt_slice(kw_args)
        );
        self.substitute_call(obj, method_name, &instance, pos_args, kw_args)?;
        log!(info "Substituted:\ninstance: {instance}");
        let res = self.eval_t_params(instance, self.level, obj.loc())?;
        log!(info "Params evaluated:\nres: {res}\n");
        self.propagate(&res, obj)?;
        log!(info "Propagated:\nres: {res}\n");
        Ok(res)
    }

    pub(crate) fn get_const_local(
        &self,
        name: &Token,
        namespace: &Str,
    ) -> SingleTyCheckResult<ValueObj> {
        if let Some(obj) = self.consts.get(name.inspect()) {
            Ok(obj.clone())
        } else {
            if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
                return parent.get_const_local(name, namespace);
            }
            Err(TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                namespace.into(),
                name.inspect(),
                self.get_similar_name(name.inspect()),
            ))
        }
    }

    pub(crate) fn _get_const_attr(
        &self,
        obj: &hir::Expr,
        name: &Token,
        namespace: &Str,
    ) -> SingleTyCheckResult<ValueObj> {
        let self_t = obj.ref_t();
        for (_, ctx) in self.get_nominal_super_type_ctxs(self_t).ok_or_else(|| {
            TyCheckError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                obj.loc(),
                self.caused_by(),
                &self_t.to_string(),
                None, // TODO:
            )
        })? {
            if let Ok(t) = ctx.get_const_local(name, namespace) {
                return Ok(t);
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.get_outer().or_else(|| self.get_builtins()) {
            parent._get_const_attr(obj, name, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                namespace.into(),
                self_t,
                name.inspect(),
                self.get_similar_attr(self_t, name.inspect()),
            ))
        }
    }

    pub(crate) fn get_similar_name(&self, name: &str) -> Option<&str> {
        let name = readable_name(name);
        // TODO: add decls
        get_similar_name(
            self.params
                .iter()
                .filter_map(|(opt_name, _)| opt_name.as_ref().map(|n| &n.inspect()[..]))
                .chain(self.locals.keys().map(|name| &name.inspect()[..])),
            name,
        )
    }

    pub(crate) fn get_similar_attr_from_singular<'a>(
        &'a self,
        obj: &hir::Expr,
        name: &str,
    ) -> Option<&'a str> {
        if let Ok(ctx) = self.get_singular_ctx(obj, &self.name) {
            if let Some(name) = ctx.get_similar_name(name) {
                return Some(name);
            }
        }
        None
    }

    pub(crate) fn get_similar_attr<'a>(&'a self, self_t: &'a Type, name: &str) -> Option<&'a str> {
        for (_, ctx) in self.get_nominal_super_type_ctxs(self_t)? {
            if let Some(name) = ctx.get_similar_name(name) {
                return Some(name);
            }
        }
        None
    }

    pub(crate) fn type_params_bounds(&self) -> Set<TyBound> {
        self.params
            .iter()
            .filter(|(opt_name, vi)| vi.kind.is_parameter() && opt_name.is_some())
            .map(|(name, vi)| {
                TyBound::instance(name.as_ref().unwrap().inspect().clone(), vi.t.clone())
            })
            .collect()
    }

    // selfが示す型が、各パラメータTypeに対してどのような変性Varianceを持つかを返す
    // 特に指定されない型に対してはInvariant
    // e.g. K(T, U) = Class(..., Impl: F(T) and Output(U) and Input(T))
    // -> K.variance() == vec![Contravariant, Covariant]
    // TODO: support keyword arguments
    pub(crate) fn type_params_variance(&self) -> Vec<Variance> {
        self.params
            .iter()
            .map(|(opt_name, _)| {
                if let Some(name) = opt_name {
                    if let Some(t) = self.super_traits.iter().find(|t| {
                        (&t.name()[..] == "Input" || &t.name()[..] == "Output")
                            && t.inner_ts()
                                .first()
                                .map(|t| &t.name() == name.inspect())
                                .unwrap_or(false)
                    }) {
                        match &t.name()[..] {
                            "Output" => Variance::Covariant,
                            "Input" => Variance::Contravariant,
                            _ => unreachable!(),
                        }
                    } else {
                        Variance::Invariant
                    }
                } else {
                    Variance::Invariant
                }
            })
            .collect()
    }

    /// Perform types linearization.
    /// TODO: Current implementation may be very inefficient.
    ///
    /// C3 linearization requires prior knowledge of inter-type dependencies, and cannot be used for Erg structural subtype linearization
    ///
    /// Algorithm:
    /// ```python
    /// [Int, Str, Nat, Never, Obj, Str!, Module]
    /// => [], [Int, Str, Nat, Never, Obj, Str!, Module]
    /// => [[Int]], [Str, Nat, Never, Obj, Str!, Module]
    /// # 1. If related, put them in the same array; if not, put them in different arrays.
    /// => [[Int], [Str]], [Nat, Never, Obj, Str!, Module]
    /// => ...
    /// => [[Int, Nat, Never, Obj]], [Str, Str!], [Module]]
    /// # 2. Then, perform sorting on the arrays
    /// => [[Never, Nat, Int, Obj], [Str!, Str], [Module]]
    /// # 3. Concatenate the arrays
    /// => [Never, Nat, Int, Obj, Str!, Str, Module]
    /// # 4. From the left, "slide" types as far as it can.
    /// => [Never, Nat, Int, Str!, Str, Module, Obj]
    /// ```
    pub fn sort_types<'a>(&self, types: impl Iterator<Item = &'a Type>) -> Vec<&'a Type> {
        let mut buffers: Vec<Vec<&Type>> = vec![];
        for t in types {
            let mut found = false;
            for buf in buffers.iter_mut() {
                if buf.iter().all(|buf_inner| self.related(buf_inner, t)) {
                    found = true;
                    buf.push(t);
                    break;
                }
            }
            if !found {
                buffers.push(vec![t]);
            }
        }
        for buf in buffers.iter_mut() {
            // this unwrap should be safe
            buf.sort_by(|lhs, rhs| self.cmp_t(lhs, rhs).try_into().unwrap());
        }
        let mut concatenated = buffers.into_iter().flatten().collect::<Vec<_>>();
        let mut idx = 0;
        let len = concatenated.len();
        while let Some(maybe_sup) = concatenated.get(idx) {
            if let Some(pos) = concatenated
                .iter()
                .take(len - idx - 1)
                .rposition(|t| self.supertype_of(maybe_sup, t))
            {
                let sup = concatenated.remove(idx);
                concatenated.insert(pos, sup); // not `pos + 1` because the element was removed at idx
            }
            idx += 1;
        }
        concatenated
    }

    pub(crate) fn get_nominal_super_trait_ctxs<'a>(
        &'a self,
        t: &Type,
    ) -> Option<impl Iterator<Item = (&'a Type, &'a Context)>> {
        let (_ctx_t, ctx) = self.get_nominal_type_ctx(t)?;
        Some(ctx.super_traits.iter().map(|sup| {
            let (_t, sup_ctx) = self.get_nominal_type_ctx(sup).unwrap();
            (sup, sup_ctx)
        }))
    }

    pub(crate) fn get_nominal_super_class_ctxs<'a>(
        &'a self,
        t: &Type,
    ) -> Option<impl Iterator<Item = (&'a Type, &'a Context)>> {
        // if `t` is {S: Str | ...}, `ctx_t` will be Str
        // else if `t` is Array(Int, 10), `ctx_t` will be Array(T, N) (if Array(Int, 10) is not specialized)
        let (_ctx_t, ctx) = self.get_nominal_type_ctx(t)?;
        // t: {S: Str | ...} => ctx.super_traits: [Eq(Str), Mul(Nat), ...]
        // => return: [(Str, Eq(Str)), (Str, Mul(Nat)), ...] (the content of &'a Type isn't {S: Str | ...})
        Some(ctx.super_classes.iter().map(|sup| {
            let (_t, sup_ctx) = self.get_nominal_type_ctx(sup).unwrap();
            (sup, sup_ctx)
        }))
    }

    pub(crate) fn get_nominal_super_type_ctxs<'a>(
        &'a self,
        t: &Type,
    ) -> Option<impl Iterator<Item = (&'a Type, &'a Context)>> {
        let (t, ctx) = self.get_nominal_type_ctx(t)?;
        let sups = ctx
            .super_classes
            .iter()
            .chain(ctx.super_traits.iter())
            .map(|sup| self.get_nominal_type_ctx(sup).unwrap());
        Some(vec![(t, ctx)].into_iter().chain(sups))
    }

    // TODO: Never
    pub(crate) fn get_nominal_type_ctx<'a>(
        &'a self,
        typ: &Type,
    ) -> Option<(&'a Type, &'a Context)> {
        match typ {
            Type::FreeVar(fv) if fv.is_linked() => {
                if let Some(res) = self.get_nominal_type_ctx(&fv.crack()) {
                    return Some(res);
                }
            }
            Type::FreeVar(fv) => {
                let sup = fv.get_sup().unwrap();
                if let Some(res) = self.get_nominal_type_ctx(&sup) {
                    return Some(res);
                }
            }
            Type::Refinement(refine) => {
                if let Some(res) = self.get_nominal_type_ctx(&refine.t) {
                    return Some(res);
                }
            }
            Type::Quantified(_) => {
                if let Some(res) = self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_get_mono_type("QuantifiedFunc")
                {
                    return Some(res);
                }
            }
            Type::Subr(_subr) => match _subr.kind {
                SubrKind::Func => {
                    if let Some(res) = self
                        .get_builtins()
                        .unwrap_or(self)
                        .rec_get_mono_type("Func")
                    {
                        return Some(res);
                    }
                }
                SubrKind::Proc => {
                    if let Some(res) = self
                        .get_builtins()
                        .unwrap_or(self)
                        .rec_get_mono_type("Proc")
                    {
                        return Some(res);
                    }
                }
            },
            Type::BuiltinPoly { name, .. } => {
                if let Some(res) = self.get_builtins().unwrap_or(self).rec_get_poly_type(name) {
                    return Some(res);
                }
            }
            Type::Poly { path, name, .. } => {
                if self.path() == path {
                    if let Some((t, ctx)) = self.rec_get_mono_type(name) {
                        return Some((t, ctx));
                    }
                }
                let path = self.cfg.input.resolve(path.as_path()).ok()?;
                if let Some(ctx) = self
                    .mod_cache
                    .as_ref()
                    .and_then(|cache| cache.ref_ctx(path.as_path()))
                    .or_else(|| {
                        self.py_mod_cache
                            .as_ref()
                            .and_then(|cache| cache.ref_ctx(path.as_path()))
                    })
                {
                    if let Some((t, ctx)) = ctx.rec_get_mono_type(name) {
                        return Some((t, ctx));
                    }
                }
            }
            Type::Record(rec) if rec.values().all(|attr| self.supertype_of(&Type, attr)) => {
                return self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_get_mono_type("RecordType");
            }
            Type::Record(_) => {
                return self
                    .get_builtins()
                    .unwrap_or(self)
                    .rec_get_mono_type("Record");
            }
            Type::Mono { path, name } => {
                if self.path() == path {
                    if let Some((t, ctx)) = self.rec_get_mono_type(name) {
                        return Some((t, ctx));
                    }
                }
                let path = self.cfg.input.resolve(path.as_path()).ok()?;
                if let Some(ctx) = self
                    .mod_cache
                    .as_ref()
                    .and_then(|cache| cache.ref_ctx(path.as_path()))
                    .or_else(|| {
                        self.py_mod_cache
                            .as_ref()
                            .and_then(|cache| cache.ref_ctx(path.as_path()))
                    })
                {
                    if let Some((t, ctx)) = ctx.rec_get_mono_type(name) {
                        return Some((t, ctx));
                    }
                }
            }
            Type::BuiltinMono(name) => {
                if let Some(res) = self.get_builtins().unwrap_or(self).rec_get_mono_type(name) {
                    return Some(res);
                }
            }
            // FIXME: `F()`などの場合、実際は引数が省略されていてもmonomorphicになる
            other if other.is_monomorphic() => {
                if let Some((t, ctx)) = self.rec_get_mono_type(&other.name()) {
                    return Some((t, ctx));
                }
            }
            Type::Ref(t) | Type::RefMut { before: t, .. } => {
                if let Some(res) = self.get_nominal_type_ctx(t) {
                    return Some(res);
                }
            }
            other => {
                log!("{other} has no nominal definition");
            }
        }
        None
    }

    // TODO: Never
    pub(crate) fn get_mut_nominal_type_ctx<'a>(
        &'a mut self,
        typ: &Type,
    ) -> Option<(&'a Type, &'a mut Context)> {
        match typ {
            Type::FreeVar(fv) if fv.is_linked() => {
                if let Some(res) = self.get_mut_nominal_type_ctx(&fv.crack()) {
                    return Some(res);
                }
            }
            Type::FreeVar(fv) => {
                let sup = fv.get_sup().unwrap();
                if let Some(res) = self.get_mut_nominal_type_ctx(&sup) {
                    return Some(res);
                }
            }
            Type::Refinement(refine) => {
                if let Some(res) = self.get_mut_nominal_type_ctx(&refine.t) {
                    return Some(res);
                }
            }
            Type::Quantified(_) => {
                if let Some(res) = self.get_mut_nominal_type_ctx(&builtin_mono("QuantifiedFunc")) {
                    return Some(res);
                }
            }
            Type::BuiltinPoly { name, params: _ } => {
                if let Some((t, ctx)) = self.rec_get_mut_poly_type(name) {
                    return Some((t, ctx));
                }
            }
            /*Type::Record(rec) if rec.values().all(|attr| self.supertype_of(&Type, attr)) => {
                // TODO: reference RecordType (inherits Type)
                if let Some(res) = self.rec_get_nominal_type_ctx(&Type) {
                    return Some(res);
                }
            }*/
            // FIXME: `F()`などの場合、実際は引数が省略されていてもmonomorphicになる
            other if other.is_monomorphic() => {
                if let Some((t, ctx)) = self.rec_get_mut_mono_type(&other.name()) {
                    return Some((t, ctx));
                }
            }
            Type::Ref(t) | Type::RefMut { before: t, .. } => {
                if let Some(res) = self.get_mut_nominal_type_ctx(t) {
                    return Some(res);
                }
            }
            other => {
                log!("{other} has no nominal definition");
            }
        }
        None
    }

    pub(crate) fn rec_get_trait_impls(&self, name: &Str) -> Vec<TraitInstance> {
        let current = if let Some(impls) = self.trait_impls.get(name) {
            impls.clone()
        } else {
            vec![]
        };
        if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            [current, outer.rec_get_trait_impls(name)].concat()
        } else {
            current
        }
    }

    pub(crate) fn _rec_get_patch(&self, name: &VarName) -> Option<&Context> {
        if let Some(patch) = self.patches.get(name) {
            Some(patch)
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer._rec_get_patch(name)
        } else {
            None
        }
    }

    // FIXME: 現在の実装だとimportしたモジュールはどこからでも見れる
    fn get_mod(&self, ident: &ast::Identifier) -> Option<&Context> {
        let t = self
            .rec_get_var_t(ident, &self.cfg.input, &self.name)
            .ok()?;
        match t {
            Type::BuiltinPoly { name, mut params } if &name[..] == "Module" => {
                let path =
                    option_enum_unwrap!(params.remove(0), TyParam::Value:(ValueObj::Str:(_)))?;
                let path = Path::new(&path[..]);
                let path = self.cfg.input.resolve(path).ok()?;
                self.mod_cache
                    .as_ref()
                    .and_then(|cache| cache.ref_ctx(&path))
                    .or_else(|| {
                        self.py_mod_cache
                            .as_ref()
                            .and_then(|cache| cache.ref_ctx(&path))
                    })
            }
            _ => None,
        }
    }

    // rec_get_const_localとは違い、位置情報を持たないしエラーとならない
    pub(crate) fn rec_get_const_obj(&self, name: &str) -> Option<&ValueObj> {
        if let Some(val) = self.consts.get(name) {
            return Some(val);
        }
        for (_, ctx) in self.methods_list.iter() {
            if let Some(val) = ctx.consts.get(name) {
                return Some(val);
            }
        }
        if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_const_obj(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_get_const_param_defaults(&self, name: &str) -> Option<&Vec<ConstTemplate>> {
        if let Some(impls) = self.const_param_defaults.get(name) {
            Some(impls)
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_const_param_defaults(name)
        } else {
            None
        }
    }

    /// FIXME: if trait, returns a freevar
    pub(crate) fn rec_get_self_t(&self) -> Option<Type> {
        if self.kind.is_method_def() || self.kind.is_type() {
            // TODO: poly type
            let name = self.name.split(&[':', '.']).last().unwrap();
            let mono_t = mono(self.path(), Str::rc(name));
            if let Some((t, _)) = self.get_nominal_type_ctx(&mono_t) {
                Some(t.clone())
            } else {
                None
            }
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_self_t()
        } else {
            None
        }
    }

    pub(crate) fn rec_get_mono_type(&self, name: &str) -> Option<(&Type, &Context)> {
        if let Some((t, ctx)) = self.mono_types.get(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_mono_type(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_get_poly_type(&self, name: &str) -> Option<(&Type, &Context)> {
        if let Some((t, ctx)) = self.poly_types.get(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_poly_type(name)
        } else {
            None
        }
    }

    fn rec_get_mut_mono_type(&mut self, name: &str) -> Option<(&mut Type, &mut Context)> {
        if let Some((t, ctx)) = self.mono_types.get_mut(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.outer.as_mut() {
            // builtins cannot be got as mutable
            outer.rec_get_mut_mono_type(name)
        } else {
            None
        }
    }

    fn rec_get_mut_poly_type(&mut self, name: &str) -> Option<(&mut Type, &mut Context)> {
        if let Some((t, ctx)) = self.poly_types.get_mut(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.outer.as_mut() {
            outer.rec_get_mut_poly_type(name)
        } else {
            None
        }
    }

    fn rec_get_type(&self, name: &str) -> Option<(&Type, &Context)> {
        if let Some((t, ctx)) = self.mono_types.get(name) {
            Some((t, ctx))
        } else if let Some((t, ctx)) = self.poly_types.get(name) {
            Some((t, ctx))
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_type(name)
        } else {
            None
        }
    }

    fn get_mut_type(&mut self, name: &str) -> Option<(&Type, &mut Context)> {
        if let Some((t, ctx)) = self.mono_types.get_mut(name) {
            Some((t, ctx))
        } else if let Some((t, ctx)) = self.poly_types.get_mut(name) {
            Some((t, ctx))
        } else {
            None
        }
    }

    fn rec_get_method_traits(&self, name: &Identifier) -> SingleTyCheckResult<&Type> {
        if let Some(candidates) = self.method_traits.get(name.inspect()) {
            let first_t = candidates.first().unwrap();
            if candidates.iter().skip(1).all(|t| t == first_t) {
                Ok(&candidates[0])
            } else {
                Err(TyCheckError::ambiguous_type_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    name,
                    candidates,
                    self.caused_by(),
                ))
            }
        } else if let Some(outer) = self.get_outer().or_else(|| self.get_builtins()) {
            outer.rec_get_method_traits(name)
        } else {
            Err(TyCheckError::no_attr_error(
                self.cfg.input.clone(),
                line!() as usize,
                name.loc(),
                self.caused_by(),
                &Type::Failure,
                name.inspect(),
                None,
            ))
        }
    }

    fn get_gen_t_require_attr_t<'a>(&'a self, gen: &'a GenTypeObj, attr: &str) -> Option<&'a Type> {
        match gen.require_or_sup.typ() {
            Type::Record(rec) => {
                if let Some(t) = rec.get(attr) {
                    return Some(t);
                }
            }
            other => {
                let obj = self.rec_get_const_obj(&other.name());
                let obj = enum_unwrap!(obj, Some:(ValueObj::Type:(TypeObj::Generated:(_))));
                if let Some(t) = self.get_gen_t_require_attr_t(obj, attr) {
                    return Some(t);
                }
            }
        }
        if let Some(additional) = &gen.additional {
            if let Type::Record(gen) = additional.typ() {
                if let Some(t) = gen.get(attr) {
                    return Some(t);
                }
            }
        }
        None
    }

    // TODO: params, polymorphic types
    pub(crate) fn get_candidates(&self, t: &Type) -> Option<Set<Type>> {
        match t {
            Type::MonoProj { lhs, rhs } => Some(self.get_proj_candidates(lhs, rhs)),
            Type::Subr(subr) => {
                let candidates = self.get_candidates(&subr.return_t)?;
                Some(
                    candidates
                        .into_iter()
                        .map(|ret_t| {
                            let subr = SubrType::new(
                                subr.kind,
                                subr.non_default_params.clone(),
                                subr.var_params.as_ref().map(|p| *p.clone()),
                                subr.default_params.clone(),
                                ret_t,
                            );
                            Type::Subr(subr)
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }

    fn get_proj_candidates(&self, lhs: &Type, rhs: &Str) -> Set<Type> {
        match lhs {
            Type::FreeVar(fv) => {
                if let Some(sup) = fv.get_sup() {
                    let insts = self.rec_get_trait_impls(&sup.name());
                    let candidates = insts.into_iter().filter_map(move |inst| {
                        if self.supertype_of(&inst.sup_trait, &sup) {
                            self.eval_t_params(
                                mono_proj(inst.sub_type, rhs),
                                self.level,
                                Location::Unknown,
                            )
                            .ok()
                        } else {
                            None
                        }
                    });
                    return candidates.collect();
                }
            }
            _ => todo!(),
        }
        todo!("{lhs}.{rhs}")
    }

    pub(crate) fn is_class(&self, typ: &Type) -> bool {
        match typ {
            Type::And(_l, _r) => false,
            Type::Never => true,
            Type::FreeVar(fv) if fv.is_linked() => self.is_class(&fv.crack()),
            Type::FreeVar(_) => false,
            Type::Or(l, r) => self.is_class(l) && self.is_class(r),
            Type::MonoProj { lhs, rhs } => self
                .get_proj_candidates(lhs, rhs)
                .iter()
                .all(|t| self.is_class(t)),
            Type::Refinement(refine) => self.is_class(&refine.t),
            Type::Ref(t) | Type::RefMut { before: t, .. } => self.is_class(t),
            _ => {
                if let Some((_, ctx)) = self.get_nominal_type_ctx(typ) {
                    ctx.kind.is_class()
                } else {
                    // TODO: unknown types
                    false
                }
            }
        }
    }

    pub(crate) fn is_trait(&self, typ: &Type) -> bool {
        match typ {
            Type::Never => false,
            Type::FreeVar(fv) if fv.is_linked() => self.is_class(&fv.crack()),
            Type::FreeVar(_) => false,
            Type::And(l, r) | Type::Or(l, r) => self.is_trait(l) && self.is_trait(r),
            Type::MonoProj { lhs, rhs } => self
                .get_proj_candidates(lhs, rhs)
                .iter()
                .all(|t| self.is_trait(t)),
            Type::Refinement(refine) => self.is_trait(&refine.t),
            Type::Ref(t) | Type::RefMut { before: t, .. } => self.is_trait(t),
            _ => {
                if let Some((_, ctx)) = self.get_nominal_type_ctx(typ) {
                    ctx.kind.is_trait()
                } else {
                    false
                }
            }
        }
    }
}
