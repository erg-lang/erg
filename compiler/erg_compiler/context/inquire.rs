// (type) getters & validators
use std::cmp::Ordering;
use std::option::Option; // conflicting to Type::Option

use erg_common::color::{GREEN, RED};
use erg_common::dict::Dict;
use erg_common::error::ErrorCore;
use erg_common::levenshtein::levenshtein;
use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::{Field, Visibility};
use erg_common::Str;
use erg_common::{enum_unwrap, fmt_option, fmt_slice, log, set};
use Type::*;

use ast::VarName;
use erg_parser::ast;
use erg_parser::token::Token;

use erg_type::free::Constraint;
use erg_type::typaram::TyParam;
use erg_type::value::ValueObj;
use erg_type::{HasType, ParamTy, SubrKind, SubrType, TyBound, Type};

use crate::context::instantiate::{ConstTemplate, TyVarContext};
use crate::context::{Context, ContextKind, RegistrationMode, TraitInstancePair, Variance};
use crate::error::readable_name;
use crate::error::{binop_to_dname, unaryop_to_dname, TyCheckError, TyCheckResult};
use crate::hir;
use crate::varinfo::VarInfo;
use RegistrationMode::*;
use Visibility::*;

impl Context {
    pub(crate) fn validate_var_sig_t(
        &self,
        sig: &ast::VarSignature,
        body_t: &Type,
        mode: RegistrationMode,
    ) -> TyCheckResult<()> {
        let spec_t = self.instantiate_var_sig_t(sig, None, mode)?;
        match &sig.pat {
            ast::VarPattern::Discard(token) => {
                if self
                    .sub_unify(body_t, &spec_t, None, Some(sig.loc()))
                    .is_err()
                {
                    return Err(TyCheckError::type_mismatch_error(
                        line!() as usize,
                        token.loc(),
                        self.caused_by(),
                        "_",
                        &spec_t,
                        body_t,
                    ));
                }
            }
            ast::VarPattern::Ident(ident) => {
                if self
                    .sub_unify(body_t, &spec_t, None, Some(sig.loc()))
                    .is_err()
                {
                    return Err(TyCheckError::type_mismatch_error(
                        line!() as usize,
                        ident.loc(),
                        self.caused_by(),
                        ident.inspect(),
                        &spec_t,
                        body_t,
                    ));
                }
            }
            ast::VarPattern::Array(a) => {
                for (elem, inf_elem_t) in a.iter().zip(body_t.inner_ts().iter()) {
                    self.validate_var_sig_t(elem, inf_elem_t, mode)?;
                }
            }
            _ => todo!(),
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
                    &Type::mono("LambdaFunc"),
                    t,
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
                    pos_args[i + 1].expr.signature_t().unwrap().typarams_len(),
                ));
            }
            let rhs = self.instantiate_param_sig_t(&lambda.params.non_defaults[0], None, Normal)?;
            union_pat_t = self.rec_union(&union_pat_t, &rhs);
        }
        // NG: expr_t: Nat, union_pat_t: {1, 2}
        // OK: expr_t: Int, union_pat_t: {1} or 'T
        if self
            .sub_unify(match_target_expr_t, &union_pat_t, None, None)
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
        let mut return_t = branch_ts[0].ty.return_t().unwrap().clone();
        for arg_t in branch_ts.iter().skip(1) {
            return_t = self.rec_union(&return_t, arg_t.ty.return_t().unwrap());
        }
        let param_ty = ParamTy::anonymous(match_target_expr_t.clone());
        let param_ts = [vec![param_ty], branch_ts.to_vec()].concat();
        let t = Type::func(param_ts, vec![], return_t);
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
        for ctx in self.rec_sorted_sup_type_ctxs(&self_t) {
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
            for ctx in self.rec_sorted_sup_type_ctxs(obj.ref_t()) {
                if let Some(vi) = ctx.locals.get(method_name.inspect()) {
                    return Ok(vi.t());
                } else if let Some(vi) = ctx.decls.get(method_name.inspect()) {
                    return Ok(vi.t());
                }
            }
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
            let receiver_t = callee.receiver_t().unwrap();
            self.reunify(receiver_t, after, Some(callee.loc()), None)?;
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
                let (sub, sup) = enum_unwrap!(
                    fv.crack_constraint().clone(),
                    Constraint::Sandwiched { sub, sup }
                );
                let (new_sub, new_sup) = (self.resolve_trait(sub)?, self.resolve_trait(sup)?);
                let new_constraint = Constraint::sandwiched(new_sub, new_sup);
                fv.update_constraint(new_constraint);
                Ok(Type::FreeVar(fv))
            }
            Type::Poly { name, params } if params.iter().all(|tp| tp.has_no_unbound_var()) => {
                let t_name = name.clone();
                let t_params = params.clone();
                let maybe_trait = Type::Poly { name, params };
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
                    Ok(Type::poly(t_name, new_params))
                } else {
                    Ok(min)
                }
            }
            Type::Subr(subr) => {
                let mut new_non_default_params = Vec::with_capacity(subr.non_default_params.len());
                for param in subr.non_default_params.into_iter() {
                    let t = self.resolve_trait(param.ty)?;
                    new_non_default_params.push(ParamTy::new(param.name, t));
                }
                let mut new_default_params = Vec::with_capacity(subr.default_params.len());
                for param in subr.default_params.into_iter() {
                    let t = self.resolve_trait(param.ty)?;
                    new_default_params.push(ParamTy::new(param.name, t));
                }
                let new_return_t = self.resolve_trait(*subr.return_t)?;
                let t = Type::subr(
                    subr.kind, // TODO: resolve self
                    new_non_default_params,
                    new_default_params,
                    new_return_t,
                );
                Ok(t)
            }
            Type::MonoProj { lhs, rhs } => {
                let new_lhs = self.resolve_trait(*lhs)?;
                Ok(Type::mono_proj(new_lhs, rhs))
            }
            Type::Refinement(refine) => {
                let new_t = self.resolve_trait(*refine.t)?;
                Ok(Type::refinement(refine.var, new_t, refine.preds))
            }
            Type::Ref(t) => {
                let new_t = self.resolve_trait(*t)?;
                Ok(Type::ref_(new_t))
            }
            Type::RefMut(t) => {
                let new_t = self.resolve_trait(*t)?;
                Ok(Type::ref_mut(new_t))
            }
            Type::VarArgs(t) => {
                let new_t = self.resolve_trait(*t)?;
                Ok(Type::var_args(new_t))
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
                if params_len < pos_args.len() + kw_args.len() {
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
                let params = subr
                    .non_default_params
                    .iter()
                    .chain(subr.default_params.iter());
                for (param_ty, pos_arg) in params.clone().zip(pos_args) {
                    let arg_t = pos_arg.expr.ref_t();
                    let param_t = &param_ty.ty;
                    self.sub_unify(arg_t, param_t, Some(pos_arg.loc()), None)
                        .map_err(|e| {
                            log!("{RED}semi-unification failed with {callee} ({arg_t} <:? {param_t})");
                            log!("errno: {}{GREEN}", e.core.errno);
                            // REVIEW:
                            let name = callee.var_full_name().unwrap_or_else(|| "".to_string());
                            let name = name
                                + "::"
                                + param_ty
                                    .name
                                    .as_ref()
                                    .map(|s| readable_name(&s[..]))
                                    .unwrap_or("");
                            TyCheckError::type_mismatch_error(
                                line!() as usize,
                                e.core.loc,
                                e.caused_by,
                                &name[..],
                                param_t,
                                arg_t,
                            )
                        })?;
                    if let Some(name) = &param_ty.name {
                        if passed_params.contains(name) {
                            return Err(TyCheckError::multiple_args_error(
                                line!() as usize,
                                callee.loc(),
                                &callee.to_string(),
                                self.caused_by(),
                                name,
                            ));
                        } else {
                            passed_params.insert(name);
                        }
                    }
                }
                let param_ts = {
                    let mut param_ts = Dict::new();
                    for param_ty in params {
                        if let Some(name) = &param_ty.name {
                            param_ts.insert(name, &param_ty.ty);
                        }
                    }
                    param_ts
                };
                for kw_arg in kw_args.iter() {
                    if let Some(param_ty) = param_ts.get(kw_arg.keyword.inspect()) {
                        self.sub_unify(kw_arg.expr.ref_t(), param_ty, Some(kw_arg.loc()), None)?;
                    } else {
                        return Err(TyCheckError::unexpected_kw_arg_error(
                            line!() as usize,
                            kw_arg.keyword.loc(),
                            &callee.to_string(),
                            self.caused_by(),
                            kw_arg.keyword.inspect(),
                        ));
                    }
                }
                Ok(())
            }
            other => todo!("{other}"),
        }
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
        log!("Substituted:\ninstance: {instance}");
        let res = self.eval.eval_t_params(instance, &self, self.level)?;
        log!("Params evaluated:\nres: {res}\n");
        self.propagate(&res, obj)?;
        log!("Propagated:\nres: {res}\n");
        let res = self.resolve_trait(res)?;
        log!("Trait resolved:\nres: {res}\n");
        Ok(res)
    }

    pub(crate) fn get_local(&self, name: &Token, namespace: &Str) -> TyCheckResult<ValueObj> {
        if let Some(obj) = self.consts.get(name.inspect()) {
            Ok(obj.clone())
        } else {
            if let Some(parent) = self.outer.as_ref() {
                return parent.get_local(name, namespace);
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

    pub(crate) fn _get_attr(
        &self,
        obj: &hir::Expr,
        name: &Token,
        namespace: &Str,
    ) -> TyCheckResult<ValueObj> {
        let self_t = obj.t();
        for ctx in self.sorted_sup_type_ctxs(&self_t) {
            if let Ok(t) = ctx.get_local(name, namespace) {
                return Ok(t);
            }
        }
        // TODO: dependent type widening
        if let Some(parent) = self.outer.as_ref() {
            parent._get_attr(obj, name, namespace)
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

    pub(crate) fn get_similar_attr<'a>(&'a self, self_t: &'a Type, name: &str) -> Option<&'a Str> {
        for ctx in self.rec_sorted_sup_type_ctxs(self_t) {
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

    // TODO: unify with type_sort
    fn sort_type_ctxs<'a>(
        &self,
        type_and_ctxs: impl Iterator<Item = (&'a Type, &'a Context)>,
    ) -> Vec<(&'a Type, &'a Context)> {
        let mut buffers: Vec<Vec<(&Type, &Context)>> = vec![];
        for t_ctx in type_and_ctxs {
            let mut found = false;
            for buf in buffers.iter_mut() {
                if buf
                    .iter()
                    .all(|(buf_inner, _)| self.related(buf_inner, t_ctx.0))
                {
                    found = true;
                    buf.push(t_ctx);
                    break;
                }
            }
            if !found {
                buffers.push(vec![t_ctx]);
            }
        }
        for buf in buffers.iter_mut() {
            // this unwrap should be safe
            buf.sort_by(|(lhs, _), (rhs, _)| self.cmp_t(lhs, rhs).try_into().unwrap());
        }
        let mut concatenated = buffers.into_iter().flatten().collect::<Vec<_>>();
        let mut idx = 0;
        let len = concatenated.len();
        while let Some((maybe_sup, _)) = concatenated.get(idx) {
            if let Some(pos) = concatenated
                .iter()
                .take(len - idx - 1)
                .rposition(|(t, _)| self.supertype_of(maybe_sup, t))
            {
                let sup = concatenated.remove(idx);
                concatenated.insert(pos, sup); // not `pos + 1` because the element was removed at idx
            }
            idx += 1;
        }
        concatenated
    }

    pub(crate) fn sort_type_pairs(
        &self,
        type_and_traits: impl Iterator<Item = TraitInstancePair>,
    ) -> Vec<TraitInstancePair> {
        let mut buffers: Vec<Vec<TraitInstancePair>> = vec![];
        for t_trait in type_and_traits {
            let mut found = false;
            for buf in buffers.iter_mut() {
                if buf
                    .iter()
                    .all(|pair| self.related(&pair.sup_trait, &t_trait.sub_type))
                {
                    found = true;
                    buf.push(t_trait.clone());
                    break;
                }
            }
            if !found {
                buffers.push(vec![t_trait]);
            }
        }
        for buf in buffers.iter_mut() {
            // this unwrap should be safe
            buf.sort_by(|lhs, rhs| {
                self.cmp_t(&lhs.sup_trait, &rhs.sup_trait)
                    .try_into()
                    .unwrap()
            });
        }
        let mut concatenated = buffers.into_iter().flatten().collect::<Vec<_>>();
        let mut idx = 0;
        let len = concatenated.len();
        while let Some(pair) = concatenated.get(idx) {
            if let Some(pos) = concatenated
                .iter()
                .take(len - idx - 1)
                .rposition(|p| self.supertype_of(&pair.sup_trait, &p.sup_trait))
            {
                let sup = concatenated.remove(idx);
                concatenated.insert(pos, sup); // not `pos + 1` because the element was removed at idx
            }
            idx += 1;
        }
        concatenated
    }

    pub(crate) fn rec_sorted_sup_type_ctxs<'a>(
        &'a self,
        t: &'a Type,
    ) -> impl Iterator<Item = &'a Context> {
        let i = self.sorted_sup_type_ctxs(t);
        if i.size_hint().1 == Some(0) {
            if let Some(outer) = &self.outer {
                return outer.sorted_sup_type_ctxs(t);
            }
        }
        i
    }

    /// Return `Context`s equal to or greater than `t`
    /// tと一致ないしそれよりも大きい型のContextを返す
    pub(crate) fn sorted_sup_type_ctxs<'a>(
        &'a self,
        t: &'a Type,
    ) -> impl Iterator<Item = &'a Context> {
        log!("{t}");
        let mut ctxs = self._sup_type_ctxs(t).collect::<Vec<_>>();
        log!("{t}");
        // Avoid heavy sorting as much as possible for efficiency
        let mut cheap_sort_succeed = true;
        ctxs.sort_by(|(lhs, _), (rhs, _)| match self.cmp_t(lhs, rhs).try_into() {
            Ok(ord) => ord,
            Err(_) => {
                cheap_sort_succeed = false;
                Ordering::Equal
            }
        });
        let sorted = if cheap_sort_succeed {
            ctxs
        } else {
            self.sort_type_ctxs(ctxs.into_iter())
        };
        sorted.into_iter().map(|(_, ctx)| ctx)
    }

    pub(crate) fn _just_type_ctxs<'a>(&'a self, t: &'a Type) -> Option<(&'a Type, &'a Context)> {
        self.types.iter().find(move |(maybe_sup, ctx)| {
            let maybe_sup_inst = if maybe_sup.has_qvar() {
                let bounds = ctx.type_params_bounds();
                let mut tv_ctx = TyVarContext::new(self.level, bounds, self);
                Self::instantiate_t((*maybe_sup).clone(), &mut tv_ctx)
            } else {
                (*maybe_sup).clone()
            };
            self.same_type_of(&maybe_sup_inst, t)
        })
    }

    /// this method is for `sorted_type_ctxs` only
    fn _sup_type_ctxs<'a>(&'a self, t: &'a Type) -> impl Iterator<Item = (&'a Type, &'a Context)> {
        log!("{t}");
        self.types.iter().filter_map(move |(maybe_sup, ctx)| {
            let maybe_sup_inst = if maybe_sup.has_qvar() {
                let bounds = ctx.type_params_bounds();
                let mut tv_ctx = TyVarContext::new(self.level, bounds, self);
                Self::instantiate_t(maybe_sup.clone(), &mut tv_ctx)
            } else {
                maybe_sup.clone()
            };
            log!("{maybe_sup}, {t}");
            if self.supertype_of(&maybe_sup_inst, t) {
                Some((maybe_sup, ctx))
            } else {
                None
            }
        })
    }

    pub(crate) fn rec_get_trait_impls(&self, name: &Str) -> Vec<TraitInstancePair> {
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

    pub(crate) fn rec_get_glue_patch_and_types(&self) -> Vec<(VarName, TraitInstancePair)> {
        if let Some(outer) = &self.outer {
            [
                &self.glue_patch_and_types[..],
                &outer.rec_get_glue_patch_and_types(),
            ]
            .concat()
        } else {
            self.glue_patch_and_types.clone()
        }
    }

    pub(crate) fn rec_get_patch(&self, name: &VarName) -> Option<&Context> {
        if let Some(patch) = self.patches.get(name) {
            Some(patch)
        } else if let Some(outer) = &self.outer {
            outer.rec_get_patch(name)
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

    pub(crate) fn rec_get_const_obj(&self, name: &str) -> Option<&ValueObj> {
        if let Some(val) = self.consts.get(name) {
            Some(val)
        } else if let Some(outer) = &self.outer {
            outer.rec_get_const_obj(name)
        } else {
            None
        }
    }

    pub(crate) fn rec_type_ctx_by_name<'a>(&'a self, t_name: &'a str) -> Option<&'a Context> {
        if let Some((_, ctx)) = self.types.iter().find(|(t, _ctx)| &t.name()[..] == t_name) {
            return Some(ctx);
        }
        if let Some(outer) = &self.outer {
            outer.rec_type_ctx_by_name(t_name)
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
