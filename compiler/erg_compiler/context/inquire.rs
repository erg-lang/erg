// (type) getters & validators
use std::option::Option; // conflicting to Type::Option

use erg_common::error::ErrorCore;
use erg_common::levenshtein::levenshtein;
use erg_common::set::Set;
use erg_common::traits::Locational;
use erg_common::vis::{Field, Visibility};
use erg_common::Str;
use erg_common::{enum_unwrap, fmt_option, fmt_slice, log, set};
use Type::*;

use ast::VarName;
use erg_parser::ast;
use erg_parser::token::Token;

use erg_type::constructors::{
    class, func, mono_proj, poly_class, ref_, ref_mut, refinement, subr_t,
};
use erg_type::free::Constraint;
use erg_type::typaram::TyParam;
use erg_type::value::ValueObj;
use erg_type::{HasType, ParamTy, SubrKind, SubrType, TyBound, Type};

use crate::context::instantiate::ConstTemplate;
use crate::context::{Context, ContextKind, RegistrationMode, TraitInstance, Variance};
use crate::error::readable_name;
use crate::error::{binop_to_dname, unaryop_to_dname, TyCheckError, TyCheckResult};
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
            return Err(TyCheckError::type_mismatch_error(
                line!() as usize,
                ident.loc(),
                self.caused_by(),
                ident.inspect(),
                &spec_t,
                body_t,
                self.get_type_mismatch_hint(&spec_t, body_t),
            ));
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
    }

    fn get_context(
        &self,
        obj: &hir::Expr,
        kind: Option<ContextKind>,
        namespace: &Str,
    ) -> TyCheckResult<&Context> {
        match obj {
            hir::Expr::Accessor(hir::Accessor::Local(name)) => {
                if kind == Some(ContextKind::Module) {
                    if let Some(ctx) = self.rec_get_mod(name.inspect()) {
                        Ok(ctx)
                    } else {
                        Err(TyCheckError::no_var_error(
                            line!() as usize,
                            obj.loc(),
                            namespace.clone(),
                            name.inspect(),
                            self.get_similar_name(name.inspect()),
                        ))
                    }
                } else {
                    todo!()
                }
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
                return Err(TyCheckError::type_mismatch_error(
                    line!() as usize,
                    pos_arg.loc(),
                    self.caused_by(),
                    "match",
                    &class("LambdaFunc"),
                    t,
                    self.get_type_mismatch_hint(&class("LambdaFunc"), t),
                ));
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
                return Err(TyCheckError::argument_error(
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
                ));
            }
            let rhs = self.instantiate_param_sig_t(&lambda.params.non_defaults[0], None, Normal)?;
            union_pat_t = self.rec_union(&union_pat_t, &rhs);
        }
        // NG: expr_t: Nat, union_pat_t: {1, 2}
        // OK: expr_t: Int, union_pat_t: {1} or 'T
        if self
            .sub_unify(match_target_expr_t, &union_pat_t, None, None, None)
            .is_err()
        {
            return Err(TyCheckError::match_error(
                line!() as usize,
                pos_args[0].loc(),
                self.caused_by(),
                match_target_expr_t,
            ));
        }
        let branch_ts = pos_args
            .iter()
            .skip(1)
            .map(|a| ParamTy::anonymous(a.expr.ref_t().clone()))
            .collect::<Vec<_>>();
        let mut return_t = branch_ts[0].typ().return_t().unwrap().clone();
        for arg_t in branch_ts.iter().skip(1) {
            return_t = self.rec_union(&return_t, &arg_t.typ().return_t().unwrap());
        }
        let param_ty = ParamTy::anonymous(match_target_expr_t.clone());
        let param_ts = [vec![param_ty], branch_ts.to_vec()].concat();
        let t = func(param_ts, None, vec![], return_t);
        Ok(t)
    }

    pub(crate) fn get_local_uniq_obj_name(&self, name: &Token) -> Option<Str> {
        // TODO: types, functions, patches
        if let Some(ctx) = self.rec_get_mod(name.inspect()) {
            return Some(ctx.name.clone());
        }
        None
    }

    pub(crate) fn rec_get_var_t(
        &self,
        name: &Token,
        vis: Visibility,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        if let Some(vi) = self.get_current_scope_var(&name.inspect()[..]) {
            if vi.vis == vis {
                Ok(vi.t())
            } else {
                Err(TyCheckError::visibility_error(
                    line!() as usize,
                    name.loc(),
                    namespace.clone(),
                    name.inspect(),
                    vi.vis,
                ))
            }
        } else {
            if let Some(parent) = self.outer.as_ref() {
                return parent.rec_get_var_t(name, vis, namespace);
            }
            Err(TyCheckError::no_var_error(
                line!() as usize,
                name.loc(),
                namespace.clone(),
                name.inspect(),
                self.get_similar_name(name.inspect()),
            ))
        }
    }

    pub(crate) fn rec_get_attr_t(
        &self,
        obj: &hir::Expr,
        name: &Token,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        let self_t = obj.t();
        match self_t {
            Type => todo!(),
            Type::Record(rec) => {
                // REVIEW: `rec.get(name.inspect())` returns None (Borrow<Str> is implemented for Field). Why?
                if let Some(attr) = rec.get(&Field::new(Public, name.inspect().clone())) {
                    return Ok(attr.clone());
                } else {
                    let t = Type::Record(rec);
                    return Err(TyCheckError::no_attr_error(
                        line!() as usize,
                        name.loc(),
                        namespace.clone(),
                        &t,
                        name.inspect(),
                        self.get_similar_attr(&t, name.inspect()),
                    ));
                }
            }
            Module => {
                let mod_ctx = self.get_context(obj, Some(ContextKind::Module), namespace)?;
                let t = mod_ctx.rec_get_var_t(name, Public, namespace)?;
                return Ok(t);
            }
            _ => {}
        }
        for (_, ctx) in self.rec_get_nominal_super_type_ctxs(&self_t) {
            if let Ok(t) = ctx.rec_get_var_t(name, Public, namespace) {
                return Ok(t);
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.outer.as_ref() {
            parent.rec_get_attr_t(obj, name, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                line!() as usize,
                name.loc(),
                namespace.clone(),
                &self_t,
                name.inspect(),
                self.get_similar_attr(&self_t, name.inspect()),
            ))
        }
    }

    /// 戻り値ではなく、call全体の型を返す
    fn search_callee_t(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Token>,
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        if let Some(method_name) = method_name.as_ref() {
            for (_, ctx) in self.rec_get_nominal_super_type_ctxs(obj.ref_t()) {
                if let Some(vi) = ctx.locals.get(method_name.inspect()) {
                    return Ok(vi.t());
                } else if let Some(vi) = ctx.decls.get(method_name.inspect()) {
                    return Ok(vi.t());
                }
            }
            if let Some(ctx) = self.rec_get_singular_ctx(obj) {
                if let Some(vi) = ctx.locals.get(method_name.inspect()) {
                    return Ok(vi.t());
                } else if let Some(vi) = ctx.decls.get(method_name.inspect()) {
                    return Ok(vi.t());
                }
                return Err(TyCheckError::singular_no_attr_error(
                    line!() as usize,
                    method_name.loc(),
                    namespace.clone(),
                    obj.__name__().unwrap_or("?"),
                    obj.ref_t(),
                    method_name.inspect(),
                    self.get_similar_attr_from_singular(obj, method_name.inspect()),
                ));
            }
            // TODO: patch
            Err(TyCheckError::no_attr_error(
                line!() as usize,
                method_name.loc(),
                namespace.clone(),
                obj.ref_t(),
                method_name.inspect(),
                self.get_similar_attr(obj.ref_t(), method_name.inspect()),
            ))
        } else {
            Ok(obj.t())
        }
    }

    pub(crate) fn get_binop_t(
        &self,
        op: &Token,
        args: &[hir::PosArg],
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        erg_common::debug_power_assert!(args.len() == 2);
        let cont = binop_to_dname(op.inspect());
        let symbol = Token::new(op.kind, Str::rc(cont), op.lineno, op.col_begin);
        let t = self.rec_get_var_t(&symbol, Private, namespace)?;
        let op = hir::Expr::Accessor(hir::Accessor::local(symbol, t));
        self.get_call_t(&op, &None, args, &[], namespace)
            .map_err(|e| {
                let op = enum_unwrap!(op, hir::Expr::Accessor:(hir::Accessor::Local:(_)));
                let lhs = args[0].expr.clone();
                let rhs = args[1].expr.clone();
                let bin = hir::BinOp::new(op.name, lhs, rhs, op.t);
                // HACK: dname.loc()はダミーLocationしか返さないので、エラーならop.loc()で上書きする
                let core = ErrorCore::new(
                    e.core.errno,
                    e.core.kind,
                    bin.loc(),
                    e.core.desc,
                    e.core.hint,
                );
                TyCheckError::new(core, e.caused_by)
            })
    }

    pub(crate) fn get_unaryop_t(
        &self,
        op: &Token,
        args: &[hir::PosArg],
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        erg_common::debug_power_assert!(args.len() == 1);
        let cont = unaryop_to_dname(op.inspect());
        let symbol = Token::new(op.kind, Str::rc(cont), op.lineno, op.col_begin);
        let t = self.rec_get_var_t(&symbol, Private, namespace)?;
        let op = hir::Expr::Accessor(hir::Accessor::local(symbol, t));
        self.get_call_t(&op, &None, args, &[], namespace)
            .map_err(|e| {
                let op = enum_unwrap!(op, hir::Expr::Accessor:(hir::Accessor::Local:(_)));
                let expr = args[0].expr.clone();
                let unary = hir::UnaryOp::new(op.name, expr, op.t);
                let core = ErrorCore::new(
                    e.core.errno,
                    e.core.kind,
                    unary.loc(),
                    e.core.desc,
                    e.core.hint,
                );
                TyCheckError::new(core, e.caused_by)
            })
    }

    /// 可変依存型の変更を伝搬させる
    fn propagate(&self, t: &Type, callee: &hir::Expr) -> TyCheckResult<()> {
        if let Type::Subr(SubrType {
            kind: SubrKind::ProcMethod {
                after: Some(after), ..
            },
            ..
        }) = t
        {
            log!(info "{}, {}", callee.ref_t(), after);
            self.reunify(callee.ref_t(), after, Some(callee.loc()), None)?;
        }
        Ok(())
    }

    /// Replace monomorphised trait with concrete type
    /// Just return input if the type is already concrete (or there is still a type variable that cannot be resolved)
    /// 単相化されたトレイトを具体的な型に置換する
    /// 既に具体的な型である(か、まだ型変数があり解決できない)場合はそのまま返す
    /// ```erg
    /// instantiate_trait(Add(Int)) => Ok(Int)
    /// instantiate_trait(Array(Add(Int), 2)) => Ok(Array(Int, 2))
    /// instantiate_trait(Array(Int, 2)) => Ok(Array(Int, 2))
    /// instantiate_trait(Int) => Ok(Int)
    /// ```
    pub(crate) fn resolve_trait(&self, maybe_trait: Type) -> TyCheckResult<Type> {
        match maybe_trait {
            Type::FreeVar(fv) if fv.is_linked() => {
                let inner = fv.crack().clone();
                let t = self.resolve_trait(inner)?;
                fv.link(&t);
                Ok(Type::FreeVar(fv))
            }
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup, cyclic) = enum_unwrap!(
                    fv.crack_constraint().clone(),
                    Constraint::Sandwiched {
                        sub,
                        sup,
                        cyclicity
                    }
                );
                let (new_sub, new_sup) = (self.resolve_trait(sub)?, self.resolve_trait(sup)?);
                let new_constraint = Constraint::sandwiched(new_sub, new_sup, cyclic);
                fv.update_constraint(new_constraint);
                Ok(Type::FreeVar(fv))
            }
            Type::PolyTrait { name, params } if params.iter().all(|tp| tp.has_no_unbound_var()) => {
                let t_name = name.clone();
                let t_params = params.clone();
                let maybe_trait = Type::PolyTrait { name, params };
                let mut min = Type::Obj;
                for pair in self.rec_get_trait_impls(&t_name) {
                    if self.rec_supertype_of(&pair.sup_trait, &maybe_trait) {
                        let new_min = self.rec_min(&min, &pair.sub_type).unwrap_or(&min).clone();
                        min = new_min;
                    }
                }
                if min == Type::Obj {
                    // may be `Array(Add(Int), 2)`, etc.
                    let mut new_params = Vec::with_capacity(t_params.len());
                    for param in t_params.into_iter() {
                        match param {
                            TyParam::Type(t) => {
                                let new_t = self.resolve_trait(*t)?;
                                new_params.push(TyParam::t(new_t));
                            }
                            other => {
                                new_params.push(other);
                            }
                        }
                    }
                    Ok(poly_class(t_name, new_params))
                } else {
                    Ok(min)
                }
            }
            Type::Subr(subr) => {
                let mut new_non_default_params = Vec::with_capacity(subr.non_default_params.len());
                for param in subr.non_default_params.into_iter() {
                    let (name, ty) = param.deconstruct();
                    let ty = self.resolve_trait(ty)?;
                    new_non_default_params.push(ParamTy::pos(name, ty));
                }
                let var_args = if let Some(va) = subr.var_params {
                    let (name, ty) = va.deconstruct();
                    let ty = self.resolve_trait(ty)?;
                    Some(ParamTy::pos(name, ty))
                } else {
                    None
                };
                let mut new_default_params = Vec::with_capacity(subr.default_params.len());
                for param in subr.default_params.into_iter() {
                    let (name, ty) = param.deconstruct();
                    let ty = self.resolve_trait(ty)?;
                    new_default_params.push(ParamTy::kw(name.unwrap(), ty));
                }
                let new_return_t = self.resolve_trait(*subr.return_t)?;
                let t = subr_t(
                    subr.kind, // TODO: resolve self
                    new_non_default_params,
                    var_args,
                    new_default_params,
                    new_return_t,
                );
                Ok(t)
            }
            Type::MonoProj { lhs, rhs } => {
                let new_lhs = self.resolve_trait(*lhs)?;
                Ok(mono_proj(new_lhs, rhs))
            }
            Type::Refinement(refine) => {
                let new_t = self.resolve_trait(*refine.t)?;
                Ok(refinement(refine.var, new_t, refine.preds))
            }
            Type::Ref(t) => {
                let new_t = self.resolve_trait(*t)?;
                Ok(ref_(new_t))
            }
            Type::RefMut(t) => {
                let new_t = self.resolve_trait(*t)?;
                Ok(ref_mut(new_t))
            }
            Type::Callable { .. } => todo!(),
            Type::And(_, _) | Type::Or(_, _) | Type::Not(_, _) => todo!(),
            other => Ok(other),
        }
    }

    /// e.g.
    /// ```erg
    /// substitute_call(instance: ((?T, ?U) -> ?T), [Int, Str], []) => instance: (Int, Str) -> Int
    /// substitute_call(instance: ((?T, Int) -> ?T), [Int, Nat], []) => instance: (Int, Int) -> Str
    /// substitute_call(instance: ((?M(: Nat)..?N(: Nat)) -> ?M+?N), [1..2], []) => instance: (1..2) -> {3}
    /// substitute_call(instance: ((?L(: Add(?R, ?O)), ?R) -> ?O), [1, 2], []) => instance: (Nat, Nat) -> Nat
    /// ```
    fn substitute_call(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Token>,
        instance: &Type,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
    ) -> TyCheckResult<()> {
        match instance {
            Type::Subr(subr) => {
                let callee = if let Some(name) = method_name {
                    let attr = hir::Attribute::new(obj.clone(), name.clone(), Type::Ellipsis);
                    let acc = hir::Expr::Accessor(hir::Accessor::Attr(attr));
                    acc
                } else {
                    obj.clone()
                };
                let params_len = subr.non_default_params.len() + subr.default_params.len();
                if (params_len < pos_args.len() || params_len < pos_args.len() + kw_args.len())
                    && subr.var_params.is_none()
                {
                    return Err(TyCheckError::too_many_args_error(
                        line!() as usize,
                        callee.loc(),
                        &callee.to_string(),
                        self.caused_by(),
                        params_len,
                        pos_args.len(),
                        kw_args.len(),
                    ));
                }
                let mut passed_params = set! {};
                if pos_args.len() >= subr.non_default_params.len() {
                    let (non_default_args, var_args) =
                        pos_args.split_at(subr.non_default_params.len());
                    for (nd_arg, nd_param) in
                        non_default_args.iter().zip(subr.non_default_params.iter())
                    {
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
                    return Err(TyCheckError::args_missing_error(
                        line!() as usize,
                        callee.loc(),
                        &callee.to_string(),
                        self.caused_by(),
                        missing_len,
                        missing_params,
                    ));
                }
                Ok(())
            }
            other => todo!("{other}"),
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
            .map_err(|e| {
                log!(err "semi-unification failed with {callee}\n{arg_t} !<: {param_t}");
                log!(err "errno: {}", e.core.errno);
                // REVIEW:
                let name = callee.var_full_name().unwrap_or_else(|| "".to_string());
                let name = name + "::" + param.name().map(|s| readable_name(&s[..])).unwrap_or("");
                TyCheckError::type_mismatch_error(
                    line!() as usize,
                    e.core.loc,
                    e.caused_by,
                    &name[..],
                    param_t,
                    arg_t,
                    self.get_type_mismatch_hint(param_t, arg_t),
                )
            })?;
        if let Some(name) = param.name() {
            if passed_params.contains(name) {
                return Err(TyCheckError::multiple_args_error(
                    line!() as usize,
                    callee.loc(),
                    &callee.to_string(),
                    self.caused_by(),
                    name,
                ));
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
            .map_err(|e| {
                log!(err "semi-unification failed with {callee}\n{arg_t} !<: {param_t}");
                log!(err "errno: {}", e.core.errno);
                // REVIEW:
                let name = callee.var_full_name().unwrap_or_else(|| "".to_string());
                let name = name + "::" + param.name().map(|s| readable_name(&s[..])).unwrap_or("");
                TyCheckError::type_mismatch_error(
                    line!() as usize,
                    e.core.loc,
                    e.caused_by,
                    &name[..],
                    param_t,
                    arg_t,
                    self.get_type_mismatch_hint(param_t, arg_t),
                )
            })
    }

    fn substitute_kw_arg(
        &self,
        callee: &hir::Expr,
        arg: &hir::KwArg,
        default_params: &Vec<ParamTy>,
        passed_params: &mut Set<Str>,
    ) -> TyCheckResult<()> {
        let arg_t = arg.expr.ref_t();
        let kw_name = arg.keyword.inspect();
        if passed_params.contains(&kw_name[..]) {
            return Err(TyCheckError::multiple_args_error(
                line!() as usize,
                callee.loc(),
                &callee.to_string(),
                self.caused_by(),
                arg.keyword.inspect(),
            ));
        } else {
            passed_params.insert(kw_name.clone());
        }
        if let Some(pt) = default_params
            .iter()
            .find(|pt| pt.name().unwrap() == kw_name)
        {
            self.sub_unify(arg_t, pt.typ(), Some(arg.loc()), None, Some(kw_name))
                .map_err(|e| {
                    log!(err "semi-unification failed with {callee}\n{arg_t} !<: {}", pt.typ());
                    log!(err "errno: {}", e.core.errno);
                    // REVIEW:
                    let name = callee.var_full_name().unwrap_or_else(|| "".to_string());
                    let name = name + "::" + readable_name(kw_name);
                    TyCheckError::type_mismatch_error(
                        line!() as usize,
                        e.core.loc,
                        e.caused_by,
                        &name[..],
                        pt.typ(),
                        arg_t,
                        self.get_type_mismatch_hint(pt.typ(), arg_t),
                    )
                })?;
        } else {
            return Err(TyCheckError::unexpected_kw_arg_error(
                line!() as usize,
                arg.keyword.loc(),
                &callee.to_string(),
                self.caused_by(),
                kw_name,
            ));
        }
        Ok(())
    }

    pub(crate) fn get_call_t(
        &self,
        obj: &hir::Expr,
        method_name: &Option<Token>,
        pos_args: &[hir::PosArg],
        kw_args: &[hir::KwArg],
        namespace: &Str,
    ) -> TyCheckResult<Type> {
        match obj {
            hir::Expr::Accessor(hir::Accessor::Local(local)) if &local.inspect()[..] == "match" => {
                return self.get_match_call_t(pos_args, kw_args)
            }
            _ => {}
        }
        let found = self.search_callee_t(obj, method_name, namespace)?;
        log!(
            "Found:\ncallee: {obj}{}\nfound: {found}",
            fmt_option!(pre ".", method_name.as_ref().map(|t| &t.content))
        );
        let instance = self.instantiate(found, obj)?;
        log!(
            "Instantiated:\ninstance: {instance}\npos_args: ({})\nkw_args: ({})",
            fmt_slice(pos_args),
            fmt_slice(kw_args)
        );
        self.substitute_call(obj, method_name, &instance, pos_args, kw_args)?;
        log!(info "Substituted:\ninstance: {instance}");
        let level = self.level;
        let res = self.eval_t_params(instance, level)?;
        log!(info "Params evaluated:\nres: {res}\n");
        self.propagate(&res, obj)?;
        log!(info "Propagated:\nres: {res}\n");
        let res = self.resolve_trait(res)?;
        log!(info "Trait resolved:\nres: {res}\n");
        Ok(res)
    }

    pub(crate) fn get_const_local(&self, name: &Token, namespace: &Str) -> TyCheckResult<ValueObj> {
        if let Some(obj) = self.consts.get(name.inspect()) {
            Ok(obj.clone())
        } else {
            if let Some(parent) = self.outer.as_ref() {
                return parent.get_const_local(name, namespace);
            }
            Err(TyCheckError::no_var_error(
                line!() as usize,
                name.loc(),
                namespace.clone(),
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
    ) -> TyCheckResult<ValueObj> {
        let self_t = obj.ref_t();
        for (_, ctx) in self.rec_get_nominal_super_type_ctxs(self_t) {
            if let Ok(t) = ctx.get_const_local(name, namespace) {
                return Ok(t);
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.outer.as_ref() {
            parent._get_const_attr(obj, name, namespace)
        } else {
            Err(TyCheckError::no_attr_error(
                line!() as usize,
                name.loc(),
                namespace.clone(),
                self_t,
                name.inspect(),
                self.get_similar_attr(self_t, name.inspect()),
            ))
        }
    }

    pub(crate) fn get_similar_name(&self, name: &str) -> Option<&Str> {
        let name = readable_name(name);
        if name.len() <= 1 {
            return None;
        }
        // TODO: add `.decls`
        let most_similar_name = self
            .params
            .iter()
            .filter_map(|(opt_name, _)| opt_name.as_ref())
            .chain(self.locals.keys())
            .min_by_key(|v| levenshtein(readable_name(v.inspect()), name))?
            .inspect();
        let len = most_similar_name.len();
        if levenshtein(most_similar_name, name) >= len / 2 {
            let outer = self.outer.as_ref()?;
            outer.get_similar_name(name)
        } else {
            Some(most_similar_name)
        }
    }

    pub(crate) fn get_similar_attr_from_singular<'a>(
        &'a self,
        obj: &hir::Expr,
        name: &str,
    ) -> Option<&'a Str> {
        if let Some(ctx) = self.rec_get_singular_ctx(obj) {
            if let Some(name) = ctx.get_similar_name(name) {
                return Some(name);
            }
        }
        None
    }

    pub(crate) fn get_similar_attr<'a>(&'a self, self_t: &'a Type, name: &str) -> Option<&'a Str> {
        for (_, ctx) in self.rec_get_nominal_super_type_ctxs(self_t) {
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
    /// ```erg
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

    pub(crate) fn rec_get_nominal_super_trait_ctxs<'a>(
        &'a self,
        t: &Type,
    ) -> impl Iterator<Item = (&'a Type, &'a Context)> {
        if let Some((_ctx_t, ctx)) = self.rec_get_nominal_type_ctx(t) {
            ctx.super_traits.iter().map(|sup| {
                let (_t, sup_ctx) = self.rec_get_nominal_type_ctx(sup).unwrap();
                (sup, sup_ctx)
            })
        } else {
            todo!("{t} has no trait, or not a nominal type")
        }
    }

    pub(crate) fn rec_get_nominal_super_class_ctxs<'a>(
        &'a self,
        t: &Type,
    ) -> impl Iterator<Item = (&'a Type, &'a Context)> {
        // if `t` is {S: Str | ...}, `ctx_t` will be Str
        // else if `t` is Array(Int, 10), `ctx_t` will be Array(T, N) (if Array(Int, 10) is not specialized)
        if let Some((_ctx_t, ctx)) = self.rec_get_nominal_type_ctx(t) {
            // t: {S: Str | ...} => ctx.super_traits: [Eq(Str), Mul(Nat), ...]
            // => return: [(Str, Eq(Str)), (Str, Mul(Nat)), ...] (the content of &'a Type isn't {S: Str | ...})
            ctx.super_classes.iter().map(|sup| {
                let (_t, sup_ctx) = self.rec_get_nominal_type_ctx(sup).unwrap();
                (sup, sup_ctx)
            })
        } else {
            todo!("{t} has no class, or not a nominal type")
        }
    }

    pub(crate) fn rec_get_nominal_super_type_ctxs<'a>(
        &'a self,
        t: &Type,
    ) -> impl Iterator<Item = (&'a Type, &'a Context)> {
        if let Some((t, ctx)) = self.rec_get_nominal_type_ctx(t) {
            vec![(t, ctx)].into_iter().chain(
                ctx.super_classes
                    .iter()
                    .chain(ctx.super_traits.iter())
                    .map(|sup| self.rec_get_nominal_type_ctx(&sup).unwrap()),
            )
        } else {
            todo!("{t} not found")
        }
    }

    pub(crate) fn rec_get_nominal_type_ctx<'a>(
        &'a self,
        typ: &Type,
    ) -> Option<(&'a Type, &'a Context)> {
        match typ {
            Type::Refinement(refine) => {
                return self.rec_get_nominal_type_ctx(&refine.t);
            }
            Type::Quantified(_) => {
                return self.rec_get_nominal_type_ctx(&class("QuantifiedFunction"));
            }
            Type::PolyClass { name, params: _ } => {
                if let Some((t, ctx)) = self.poly_classes.get(name) {
                    return Some((t, ctx));
                }
            }
            Type::PolyTrait { name, params: _ } => {
                if let Some((t, ctx)) = self.poly_traits.get(name) {
                    return Some((t, ctx));
                }
            }
            Type::Record(rec) if rec.values().all(|attr| self.supertype_of(&Type, attr)) => {
                return self.rec_get_nominal_type_ctx(&Type)
            }
            // FIXME: `F()`などの場合、実際は引数が省略されていてもmonomorphicになる
            other if other.is_monomorphic() => {
                if let Some((t, ctx)) = self.mono_types.get(&typ.name()) {
                    return Some((t, ctx));
                }
            }
            other => todo!("{other}"),
        }
        if let Some(outer) = &self.outer {
            outer.rec_get_nominal_type_ctx(typ)
        } else {
            None
        }
    }

    fn rec_get_singular_ctx(&self, obj: &hir::Expr) -> Option<&Context> {
        match obj.ref_t() {
            // TODO: attr
            Type::Module => self.rec_get_mod(&obj.var_full_name()?),
            Type::Class => todo!(),
            Type::Trait => todo!(),
            _ => None,
        }
    }

    pub(crate) fn rec_get_trait_impls(&self, name: &Str) -> Vec<TraitInstance> {
        let current = if let Some(impls) = self.trait_impls.get(name) {
            impls.clone()
        } else {
            vec![]
        };
        if let Some(outer) = &self.outer {
            [current, outer.rec_get_trait_impls(name)].concat()
        } else {
            current
        }
    }

    pub(crate) fn _rec_get_patch(&self, name: &VarName) -> Option<&Context> {
        if let Some(patch) = self.patches.get(name) {
            Some(patch)
        } else if let Some(outer) = &self.outer {
            outer._rec_get_patch(name)
        } else {
            None
        }
    }

    fn rec_get_mod(&self, name: &str) -> Option<&Context> {
        if let Some(mod_) = self.mods.get(name) {
            Some(mod_)
        } else if let Some(outer) = &self.outer {
            outer.rec_get_mod(name)
        } else {
            None
        }
    }

    // rec_get_const_localとは違い、位置情報を持たないしエラーとならない
    pub(crate) fn rec_get_const_obj(&self, name: &str) -> Option<&ValueObj> {
        if let Some(val) = self.consts.get(name) {
            Some(val)
        } else if let Some(outer) = &self.outer {
            outer.rec_get_const_obj(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_get_const_param_defaults(&self, name: &str) -> Option<&Vec<ConstTemplate>> {
        if let Some(impls) = self.const_param_defaults.get(name) {
            return Some(impls);
        }
        if let Some(outer) = &self.outer {
            outer.rec_get_const_param_defaults(name)
        } else {
            None
        }
    }
}
