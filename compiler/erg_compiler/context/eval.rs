use std::fmt;
use std::mem;

use erg_common::dict::Dict;
use erg_common::enum_unwrap;
use erg_common::error::Location;
#[allow(unused)]
use erg_common::log;
use erg_common::set::Set;
use erg_common::shared::Shared;
use erg_common::traits::{Locational, Stream};
use erg_common::vis::Field;
use erg_common::{dict, fn_name, option_enum_unwrap, set};
use erg_common::{RcArray, Str};
use OpKind::*;

use erg_parser::ast::*;
use erg_parser::token::{Token, TokenKind};

use crate::ty::constructors::dict_t;
use crate::ty::constructors::proj_call;
use crate::ty::constructors::{
    array_t, mono, not, poly, proj, ref_, ref_mut, refinement, subr_t, v_enum,
};
use crate::ty::typaram::{OpKind, TyParam};
use crate::ty::value::ValueObj;
use crate::ty::{ConstSubr, HasType, Predicate, SubrKind, TyBound, Type, UserConstSubr, ValueArgs};

use crate::context::instantiate::TyVarInstContext;
use crate::context::{ClassDefType, Context, ContextKind, RegistrationMode};
use crate::error::{EvalError, EvalErrors, EvalResult, SingleEvalResult, TyCheckResult};

use super::Variance;

#[inline]
pub fn type_from_token_kind(kind: TokenKind) -> Type {
    use TokenKind::*;

    match kind {
        NatLit => Type::Nat,
        IntLit => Type::Int,
        RatioLit => Type::Ratio,
        StrLit => Type::Str,
        BoolLit => Type::Bool,
        NoneLit => Type::NoneType,
        NoImplLit => Type::NotImplemented,
        EllipsisLit => Type::Ellipsis,
        InfLit => Type::Inf,
        other => panic!("this has not type: {other}"),
    }
}

fn try_get_op_kind_from_token(kind: TokenKind) -> EvalResult<OpKind> {
    match kind {
        TokenKind::Plus => Ok(OpKind::Add),
        TokenKind::Minus => Ok(OpKind::Sub),
        TokenKind::Star => Ok(OpKind::Mul),
        TokenKind::Slash => Ok(OpKind::Div),
        TokenKind::FloorDiv => Ok(OpKind::FloorDiv),
        TokenKind::Pow => Ok(OpKind::Pow),
        TokenKind::Mod => Ok(OpKind::Mod),
        TokenKind::DblEq => Ok(OpKind::Eq),
        TokenKind::NotEq => Ok(OpKind::Ne),
        TokenKind::Less => Ok(OpKind::Lt),
        TokenKind::Gre => Ok(OpKind::Gt),
        TokenKind::LessEq => Ok(OpKind::Le),
        TokenKind::GreEq => Ok(OpKind::Ge),
        TokenKind::AndOp => Ok(OpKind::And),
        TokenKind::OrOp => Ok(OpKind::Or),
        TokenKind::BitAnd => Ok(OpKind::BitAnd),
        TokenKind::BitXor => Ok(OpKind::BitXor),
        TokenKind::BitOr => Ok(OpKind::BitOr),
        TokenKind::Shl => Ok(OpKind::Shl),
        TokenKind::Shr => Ok(OpKind::Shr),
        TokenKind::Mutate => Ok(OpKind::Mutate),
        _other => todo!("{_other}"),
    }
}

fn op_to_name(op: OpKind) -> &'static str {
    match op {
        OpKind::Add => "__add__",
        OpKind::Sub => "__sub__",
        OpKind::Mul => "__mul__",
        OpKind::Div => "__div__",
        OpKind::FloorDiv => "__floordiv__",
        OpKind::Mod => "__mod__",
        OpKind::Pow => "__pow__",
        OpKind::Pos => "__pos__",
        OpKind::Neg => "__neg__",
        OpKind::Eq => "__eq__",
        OpKind::Ne => "__ne__",
        OpKind::Lt => "__lt__",
        OpKind::Le => "__le__",
        OpKind::Gt => "__gt__",
        OpKind::Ge => "__ge__",
        OpKind::And => "__and__",
        OpKind::Or => "__or__",
        OpKind::Invert => "__invert__",
        OpKind::BitAnd => "__bitand__",
        OpKind::BitOr => "__bitor__",
        OpKind::BitXor => "__bitxor__",
        OpKind::Shl => "__shl__",
        OpKind::Shr => "__shr__",
        OpKind::Mutate => "__mutate__",
    }
}

/// Instantiate the polymorphic type from the quantified state.
///
/// e.g.
/// ```
/// SubstContext::new(Array(?T, 0), ...) => SubstContext{ params: { 'T: ?T; 'N: 0 }, ... }
/// self.substitute(Array!('T; !'N)): Array(?T, !0)
/// ```
#[derive(Debug)]
pub struct SubstContext<'c> {
    ctx: &'c Context,
    bounds: Set<TyBound>,
    params: Dict<Str, TyParam>,
    loc: Location,
}

impl fmt::Display for SubstContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubstContext{{ bounds: {}, params: {} }}",
            self.bounds, self.params
        )
    }
}

impl<'c> SubstContext<'c> {
    /// `substituted` is used to obtain real argument information. So it must be instantiated as `Array(?T, 0)` and so on.
    ///
    /// `ctx` is used to obtain information on the names and variance of the parameters.
    pub fn new(substituted: &Type, ctx: &'c Context, loc: Location) -> Self {
        let ty_ctx = ctx
            .get_nominal_type_ctx(substituted)
            .unwrap_or_else(|| todo!("{substituted} not found"));
        let bounds = ty_ctx.type_params_bounds();
        let param_names = ty_ctx.params.iter().map(|(opt_name, _)| {
            opt_name
                .as_ref()
                .map_or_else(|| Str::ever("_"), |n| n.inspect().clone())
        });
        if param_names.len() != substituted.typarams().len() {
            let param_names = param_names.collect::<Vec<_>>();
            panic!(
                "{} param_names: {param_names:?} != {} substituted_params: [{}]",
                ty_ctx.name,
                substituted.qual_name(),
                erg_common::fmt_vec(&substituted.typarams())
            );
        }
        let params = param_names
            .zip(substituted.typarams().into_iter())
            .collect::<Dict<_, _>>();
        if cfg!(feature = "debug") {
            for v in params.values() {
                if v.has_qvar() {
                    panic!("{} has qvar", v);
                }
            }
        }
        // REVIEW: 順番は保証されるか? 引数がunnamed_paramsに入る可能性は?
        SubstContext {
            ctx,
            bounds,
            params,
            loc,
        }
    }

    pub fn substitute(&self, quant_t: Type) -> TyCheckResult<Type> {
        let tv_ctx = TyVarInstContext::new(self.ctx.level, self.bounds.clone(), self.ctx);
        let inst = self.ctx.instantiate_t(quant_t, &tv_ctx, self.loc)?;
        for param in inst.typarams() {
            self.substitute_tp(&param)?;
        }
        Ok(inst)
    }

    fn substitute_tp(&self, param: &TyParam) -> TyCheckResult<()> {
        match param {
            TyParam::FreeVar(fv) => {
                if let Some(name) = fv.unbound_name() {
                    if let Some(tp) = self.params.get(&name) {
                        self.ctx.sub_unify_tp(param, tp, None, self.loc, false)?;
                    }
                } else if fv.is_unbound() {
                    panic!()
                }
            }
            TyParam::BinOp { lhs, rhs, .. } => {
                self.substitute_tp(lhs)?;
                self.substitute_tp(rhs)?;
            }
            TyParam::UnaryOp { val, .. } => {
                self.substitute_tp(val)?;
            }
            TyParam::Array(args)
            | TyParam::Tuple(args)
            | TyParam::App { args, .. }
            | TyParam::PolyQVar { args, .. } => {
                for arg in args.iter() {
                    self.substitute_tp(arg)?;
                }
            }
            TyParam::Type(t) => {
                self.substitute_t(t)?;
            }
            TyParam::Proj { obj, .. } => {
                self.substitute_tp(obj)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn substitute_t(&self, param_t: &Type) -> TyCheckResult<()> {
        match param_t {
            Type::FreeVar(fv) => {
                if let Some(name) = fv.unbound_name() {
                    if let Some(tp) = self.params.get(&name) {
                        if let TyParam::Type(t) = tp {
                            self.ctx.sub_unify(param_t, t, Location::Unknown, None)?;
                        } else {
                            panic!()
                        }
                    }
                }
            }
            Type::Subr(subr) => {
                for nd_param in subr.non_default_params.iter() {
                    self.substitute_t(nd_param.typ())?;
                }
                if let Some(var_params) = &subr.var_params {
                    self.substitute_t(var_params.typ())?;
                }
                for d_param in subr.default_params.iter() {
                    self.substitute_t(d_param.typ())?;
                }
                self.substitute_t(&subr.return_t)?;
            }
            Type::And(l, r) | Type::Or(l, r) | Type::Not(l, r) => {
                self.substitute_t(l)?;
                self.substitute_t(r)?;
            }
            Type::Proj { lhs, .. } => {
                self.substitute_t(lhs)?;
            }
            Type::Record(rec) => {
                for (_, t) in rec.iter() {
                    self.substitute_t(t)?;
                }
            }
            Type::Ref(t) => {
                self.substitute_t(t)?;
            }
            Type::RefMut { before, after } => {
                self.substitute_t(before)?;
                if let Some(aft) = after {
                    self.substitute_t(aft)?;
                }
            }
            Type::Refinement(refine) => {
                self.substitute_t(&refine.t)?;
            }
            Type::Poly { params, .. } | Type::PolyQVar { params, .. } => {
                for param in params.iter() {
                    self.substitute_tp(param)?;
                }
            }
            t => todo!("{t:?}"),
        }
        Ok(())
    }
}

impl Context {
    fn eval_const_acc(&self, acc: &Accessor) -> EvalResult<ValueObj> {
        match acc {
            Accessor::Ident(ident) => {
                if let Some(val) = self.rec_get_const_obj(ident.inspect()) {
                    Ok(val.clone())
                } else if ident.is_const() {
                    Err(EvalErrors::from(EvalError::no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        ident.loc(),
                        self.caused_by(),
                        ident.inspect(),
                        self.get_similar_name(ident.inspect()),
                    )))
                } else {
                    Err(EvalErrors::from(EvalError::not_const_expr(
                        self.cfg.input.clone(),
                        line!() as usize,
                        acc.loc(),
                        self.caused_by(),
                    )))
                }
            }
            Accessor::Attr(attr) => {
                let obj = self.eval_const_expr(&attr.obj)?;
                Ok(self.eval_attr(obj, &attr.ident)?)
            }
            _ => todo!(),
        }
    }

    fn eval_attr(&self, obj: ValueObj, ident: &Identifier) -> SingleEvalResult<ValueObj> {
        if let Some(val) = obj.try_get_attr(&Field::from(ident)) {
            return Ok(val);
        }
        if let ValueObj::Type(t) = &obj {
            if let Some(sups) = self.get_nominal_super_type_ctxs(t.typ()) {
                for ctx in sups {
                    if let Some(val) = ctx.consts.get(ident.inspect()) {
                        return Ok(val.clone());
                    }
                    for (_, methods) in ctx.methods_list.iter() {
                        if let Some(v) = methods.consts.get(ident.inspect()) {
                            return Ok(v.clone());
                        }
                    }
                }
            }
        }
        Err(EvalError::no_attr_error(
            self.cfg.input.clone(),
            line!() as usize,
            ident.loc(),
            self.caused_by(),
            &obj.t(),
            ident.inspect(),
            None,
        ))
    }

    fn eval_const_bin(&self, bin: &BinOp) -> EvalResult<ValueObj> {
        let lhs = self.eval_const_expr(&bin.args[0])?;
        let rhs = self.eval_const_expr(&bin.args[1])?;
        let op = try_get_op_kind_from_token(bin.op.kind)?;
        self.eval_bin(op, lhs, rhs)
    }

    fn eval_const_unary(&self, unary: &UnaryOp) -> EvalResult<ValueObj> {
        let val = self.eval_const_expr(&unary.args[0])?;
        let op = try_get_op_kind_from_token(unary.op.kind)?;
        self.eval_unary(op, val)
    }

    fn eval_args(&self, args: &Args) -> EvalResult<ValueArgs> {
        let mut evaluated_pos_args = vec![];
        for arg in args.pos_args().iter() {
            let val = self.eval_const_expr(&arg.expr)?;
            evaluated_pos_args.push(val);
        }
        let mut evaluated_kw_args = dict! {};
        for arg in args.kw_args().iter() {
            let val = self.eval_const_expr(&arg.expr)?;
            evaluated_kw_args.insert(arg.keyword.inspect().clone(), val);
        }
        Ok(ValueArgs::new(evaluated_pos_args, evaluated_kw_args))
    }

    fn eval_const_call(&self, call: &Call) -> EvalResult<ValueObj> {
        if let Expr::Accessor(acc) = call.obj.as_ref() {
            match acc {
                Accessor::Ident(ident) => {
                    let obj = self.rec_get_const_obj(ident.inspect()).ok_or_else(|| {
                        EvalError::no_var_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            ident.loc(),
                            self.caused_by(),
                            ident.inspect(),
                            self.get_similar_name(ident.inspect()),
                        )
                    })?;
                    let subr = option_enum_unwrap!(obj, ValueObj::Subr)
                        .ok_or_else(|| {
                            EvalError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                ident.loc(),
                                self.caused_by(),
                                ident.inspect(),
                                &mono("Subroutine"),
                                &obj.t(),
                                self.get_candidates(&obj.t()),
                                None,
                            )
                        })?
                        .clone();
                    let args = self.eval_args(&call.args)?;
                    self.call(subr, args, call.loc())
                }
                Accessor::Attr(_attr) => todo!(),
                Accessor::TupleAttr(_attr) => todo!(),
                Accessor::Subscr(_subscr) => todo!(),
                Accessor::TypeApp(_type_app) => todo!(),
            }
        } else {
            todo!()
        }
    }

    fn call(&self, subr: ConstSubr, args: ValueArgs, loc: Location) -> EvalResult<ValueObj> {
        match subr {
            ConstSubr::User(_user) => todo!(),
            ConstSubr::Builtin(builtin) => builtin.call(args, self).map_err(|mut e| {
                e.loc = loc;
                EvalErrors::from(EvalError::new(e, self.cfg.input.clone(), self.caused_by()))
            }),
        }
    }

    fn eval_const_def(&mut self, def: &Def) -> EvalResult<ValueObj> {
        if def.is_const() {
            let __name__ = def.sig.ident().unwrap().inspect();
            let vis = def.sig.vis();
            let tv_ctx = match &def.sig {
                Signature::Subr(subr) => {
                    let bounds =
                        self.instantiate_ty_bounds(&subr.bounds, RegistrationMode::Normal)?;
                    Some(TyVarInstContext::new(self.level, bounds, self))
                }
                Signature::Var(_) => None,
            };
            // TODO: set params
            self.grow(__name__, ContextKind::Instant, vis, tv_ctx);
            let obj = self.eval_const_block(&def.body.block).map_err(|e| {
                self.pop();
                e
            })?;
            match self.check_decls_and_pop() {
                Ok(_) => {
                    self.register_gen_const(def.sig.ident().unwrap(), obj)?;
                    Ok(ValueObj::None)
                }
                Err(errs) => {
                    self.register_gen_const(def.sig.ident().unwrap(), obj)?;
                    Err(errs)
                }
            }
        } else {
            Err(EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                def.body.block.loc(),
                self.caused_by(),
            )))
        }
    }

    fn eval_const_array(&self, arr: &Array) -> EvalResult<ValueObj> {
        let mut elems = vec![];
        match arr {
            Array::Normal(arr) => {
                for elem in arr.elems.pos_args().iter() {
                    let elem = self.eval_const_expr(&elem.expr)?;
                    elems.push(elem);
                }
            }
            _ => {
                todo!()
            }
        }
        Ok(ValueObj::Array(RcArray::from(elems)))
    }

    fn eval_const_record(&self, record: &Record) -> EvalResult<ValueObj> {
        match record {
            Record::Normal(rec) => self.eval_const_normal_record(rec),
            Record::Shortened(_rec) => unreachable!(), // should be desugared
        }
    }

    fn eval_const_normal_record(&self, record: &NormalRecord) -> EvalResult<ValueObj> {
        let mut attrs = vec![];
        // HACK: should avoid cloning
        let mut record_ctx = Context::instant(
            Str::ever("<unnamed record>"),
            self.cfg.clone(),
            2,
            self.mod_cache.clone(),
            self.py_mod_cache.clone(),
            self.clone(),
        );
        for attr in record.attrs.iter() {
            // let name = attr.sig.ident().map(|i| i.inspect());
            let elem = record_ctx.eval_const_block(&attr.body.block)?;
            let ident = match &attr.sig {
                Signature::Var(var) => match &var.pat {
                    VarPattern::Ident(ident) => Field::new(ident.vis(), ident.inspect().clone()),
                    _ => todo!(),
                },
                _ => todo!(),
            };
            attrs.push((ident, elem));
        }
        Ok(ValueObj::Record(attrs.into_iter().collect()))
    }

    /// FIXME: grow
    fn eval_const_lambda(&self, lambda: &Lambda) -> EvalResult<ValueObj> {
        let bounds = self.instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal)?;
        let tv_ctx = TyVarInstContext::new(self.level, bounds, self);
        let mut non_default_params = Vec::with_capacity(lambda.sig.params.non_defaults.len());
        for sig in lambda.sig.params.non_defaults.iter() {
            let pt =
                self.instantiate_param_ty(sig, None, Some(&tv_ctx), RegistrationMode::Normal)?;
            non_default_params.push(pt);
        }
        let var_params = if let Some(p) = lambda.sig.params.var_args.as_ref() {
            let pt = self.instantiate_param_ty(p, None, Some(&tv_ctx), RegistrationMode::Normal)?;
            Some(pt)
        } else {
            None
        };
        let mut default_params = Vec::with_capacity(lambda.sig.params.defaults.len());
        for sig in lambda.sig.params.defaults.iter() {
            let pt =
                self.instantiate_param_ty(sig, None, Some(&tv_ctx), RegistrationMode::Normal)?;
            default_params.push(pt);
        }
        // HACK: should avoid cloning
        let mut lambda_ctx = Context::instant(
            Str::ever("<lambda>"),
            self.cfg.clone(),
            0,
            self.mod_cache.clone(),
            self.py_mod_cache.clone(),
            self.clone(),
        );
        let return_t = lambda_ctx.eval_const_block(&lambda.body)?;
        // FIXME: lambda: i: Int -> Int
        // => sig_t: (i: Type) -> Type
        // => as_type: (i: Int) -> Int
        let sig_t = subr_t(
            SubrKind::from(lambda.op.kind),
            non_default_params.clone(),
            var_params.clone(),
            default_params.clone(),
            v_enum(set![return_t.clone()]),
        );
        let sig_t = self.generalize_t(sig_t);
        let as_type = subr_t(
            SubrKind::from(lambda.op.kind),
            non_default_params,
            var_params,
            default_params,
            // TODO: unwrap
            return_t.as_type().unwrap().into_typ(),
        );
        let as_type = self.generalize_t(as_type);
        let subr = ConstSubr::User(UserConstSubr::new(
            Str::ever("<lambda>"),
            lambda.sig.params.clone(),
            lambda.body.clone(),
            sig_t,
            Some(as_type),
        ));
        Ok(ValueObj::Subr(subr))
    }

    pub(crate) fn eval_lit(&self, lit: &Literal) -> EvalResult<ValueObj> {
        let t = type_from_token_kind(lit.token.kind);
        ValueObj::from_str(t, lit.token.content.clone()).ok_or_else(|| {
            EvalError::invalid_literal(
                self.cfg.input.clone(),
                line!() as usize,
                lit.token.loc(),
                self.caused_by(),
            )
            .into()
        })
    }

    pub(crate) fn eval_const_expr(&self, expr: &Expr) -> EvalResult<ValueObj> {
        match expr {
            Expr::Lit(lit) => self.eval_lit(lit),
            Expr::Accessor(acc) => self.eval_const_acc(acc),
            Expr::BinOp(bin) => self.eval_const_bin(bin),
            Expr::UnaryOp(unary) => self.eval_const_unary(unary),
            Expr::Call(call) => self.eval_const_call(call),
            Expr::Array(arr) => self.eval_const_array(arr),
            Expr::Record(rec) => self.eval_const_record(rec),
            Expr::Lambda(lambda) => self.eval_const_lambda(lambda),
            other => todo!("{other}"),
        }
    }

    // ConstExprを評価するのではなく、コンパイル時関数の式(AST上ではただのExpr)を評価する
    // コンパイル時評価できないならNoneを返す
    pub(crate) fn eval_const_chunk(&mut self, expr: &Expr) -> EvalResult<ValueObj> {
        match expr {
            Expr::Lit(lit) => self.eval_lit(lit),
            Expr::Accessor(acc) => self.eval_const_acc(acc),
            Expr::BinOp(bin) => self.eval_const_bin(bin),
            Expr::UnaryOp(unary) => self.eval_const_unary(unary),
            Expr::Call(call) => self.eval_const_call(call),
            Expr::Def(def) => self.eval_const_def(def),
            Expr::Array(arr) => self.eval_const_array(arr),
            Expr::Record(rec) => self.eval_const_record(rec),
            Expr::Lambda(lambda) => self.eval_const_lambda(lambda),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn eval_const_block(&mut self, block: &Block) -> EvalResult<ValueObj> {
        for chunk in block.iter().rev().skip(1).rev() {
            self.eval_const_chunk(chunk)?;
        }
        self.eval_const_chunk(block.last().unwrap())
    }

    fn eval_bin(&self, op: OpKind, lhs: ValueObj, rhs: ValueObj) -> EvalResult<ValueObj> {
        match op {
            Add => lhs.try_add(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Sub => lhs.try_sub(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Mul => lhs.try_mul(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Div => lhs.try_div(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            FloorDiv => lhs.try_floordiv(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Gt => lhs.try_gt(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Ge => lhs.try_ge(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Lt => lhs.try_lt(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Le => lhs.try_le(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Eq => lhs.try_eq(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Ne => lhs.try_ne(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn eval_bin_tp(
        &self,
        op: OpKind,
        lhs: &TyParam,
        rhs: &TyParam,
    ) -> EvalResult<TyParam> {
        match (lhs, rhs) {
            (TyParam::Value(ValueObj::Mut(lhs)), TyParam::Value(rhs)) => self
                .eval_bin(op, lhs.borrow().clone(), rhs.clone())
                .map(|v| TyParam::Value(ValueObj::Mut(Shared::new(v)))),
            (TyParam::Value(lhs), TyParam::Value(rhs)) => self
                .eval_bin(op, lhs.clone(), rhs.clone())
                .map(TyParam::value),
            (TyParam::FreeVar(fv), r) if fv.is_linked() => self.eval_bin_tp(op, &*fv.crack(), r),
            (TyParam::FreeVar(_), _) if op.is_comparison() => Ok(TyParam::value(true)),
            // _: Nat <= 10 => true
            // TODO: maybe this is wrong, we should do the type-checking of `<=`
            (TyParam::Erased(t), _)
                if op.is_comparison() && self.supertype_of(t, &self.get_tp_t(rhs).unwrap()) =>
            {
                Ok(TyParam::value(true))
            }
            (TyParam::FreeVar(_), _) => Ok(TyParam::bin(op, lhs.clone(), rhs.clone())),
            (l, TyParam::FreeVar(fv)) if fv.is_linked() => self.eval_bin_tp(op, l, &*fv.crack()),
            (_, TyParam::FreeVar(_)) if op.is_comparison() => Ok(TyParam::value(true)),
            // 10 <= _: Nat => true
            (_, TyParam::Erased(t))
                if op.is_comparison() && self.supertype_of(&self.get_tp_t(lhs).unwrap(), t) =>
            {
                Ok(TyParam::value(true))
            }
            (_, TyParam::FreeVar(_)) => Ok(TyParam::bin(op, lhs.clone(), rhs.clone())),
            (e @ TyParam::Erased(_), _) | (_, e @ TyParam::Erased(_)) => Ok(e.clone()),
            (l, r) => todo!("{l:?} {op} {r:?}"),
        }
    }

    fn eval_unary(&self, op: OpKind, val: ValueObj) -> EvalResult<ValueObj> {
        match op {
            Pos => todo!(),
            Neg => todo!(),
            Invert => todo!(),
            Mutate => Ok(ValueObj::Mut(Shared::new(val))),
            other => todo!("{other}"),
        }
    }

    fn eval_unary_tp(&self, op: OpKind, val: &TyParam) -> EvalResult<TyParam> {
        match val {
            TyParam::Value(c) => self.eval_unary(op, c.clone()).map(TyParam::Value),
            TyParam::FreeVar(fv) if fv.is_linked() => self.eval_unary_tp(op, &*fv.crack()),
            e @ TyParam::Erased(_) => Ok(e.clone()),
            TyParam::MonoQVar(n) => todo!("not instantiated variable: {n}"),
            other => todo!("{op} {other}"),
        }
    }

    fn eval_app(&self, _name: &Str, _args: &[TyParam]) -> EvalResult<TyParam> {
        todo!()
    }

    /// 量化変数などはそのまま返す
    pub(crate) fn eval_tp(&self, p: &TyParam) -> EvalResult<TyParam> {
        match p {
            TyParam::FreeVar(fv) if fv.is_linked() => self.eval_tp(&fv.crack()),
            TyParam::Mono(name) => self
                .rec_get_const_obj(name)
                .map(|v| TyParam::value(v.clone()))
                .ok_or_else(|| {
                    EvalErrors::from(EvalError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    ))
                }),
            TyParam::BinOp { op, lhs, rhs } => self.eval_bin_tp(*op, lhs, rhs),
            TyParam::UnaryOp { op, val } => self.eval_unary_tp(*op, val),
            TyParam::App { name, args } => self.eval_app(name, args),
            TyParam::Array(tps) => {
                let mut new_tps = Vec::with_capacity(tps.len());
                for tp in tps {
                    new_tps.push(self.eval_tp(tp)?);
                }
                Ok(TyParam::Array(new_tps))
            }
            TyParam::Tuple(tps) => {
                let mut new_tps = Vec::with_capacity(tps.len());
                for tp in tps {
                    new_tps.push(self.eval_tp(tp)?);
                }
                Ok(TyParam::Tuple(new_tps))
            }
            TyParam::Dict(dic) => {
                let mut new_dic = dict! {};
                for (k, v) in dic.iter() {
                    new_dic.insert(self.eval_tp(k)?, self.eval_tp(v)?);
                }
                Ok(TyParam::Dict(new_dic))
            }
            p @ (TyParam::Type(_)
            | TyParam::Erased(_)
            | TyParam::Value(_)
            | TyParam::FreeVar(_)
            | TyParam::MonoQVar(_)) => Ok(p.clone()),
            _other => Err(EvalErrors::from(EvalError::feature_error(
                self.cfg.input.clone(),
                Location::Unknown,
                "???",
                self.caused_by(),
            ))),
        }
    }

    pub(crate) fn eval_t_params(
        &self,
        substituted: Type,
        level: usize,
        t_loc: Location,
    ) -> EvalResult<Type> {
        match substituted {
            Type::FreeVar(fv) if fv.is_linked() => {
                self.eval_t_params(fv.crack().clone(), level, t_loc)
            }
            Type::Subr(mut subr) => {
                for pt in subr.non_default_params.iter_mut() {
                    *pt.typ_mut() = self.eval_t_params(mem::take(pt.typ_mut()), level, t_loc)?;
                }
                if let Some(var_args) = subr.var_params.as_mut() {
                    *var_args.typ_mut() =
                        self.eval_t_params(mem::take(var_args.typ_mut()), level, t_loc)?;
                }
                for pt in subr.default_params.iter_mut() {
                    *pt.typ_mut() = self.eval_t_params(mem::take(pt.typ_mut()), level, t_loc)?;
                }
                let return_t = self.eval_t_params(*subr.return_t, level, t_loc)?;
                Ok(subr_t(
                    subr.kind,
                    subr.non_default_params,
                    subr.var_params.map(|v| *v),
                    subr.default_params,
                    return_t,
                ))
            }
            Type::Refinement(refine) => {
                let mut preds = Set::with_capacity(refine.preds.len());
                for pred in refine.preds.into_iter() {
                    preds.insert(self.eval_pred(pred)?);
                }
                Ok(refinement(refine.var, *refine.t, preds))
            }
            // [?T; 0].MutType! == [?T; !0]
            // ?T(<: Add(?R(:> Int))).Output == ?T(<: Add(?R)).Output
            // ?T(:> Int, <: Add(?R(:> Int))).Output == Int
            Type::Proj { lhs, rhs } => self.eval_proj(*lhs, rhs, level, t_loc),
            Type::ProjCall {
                lhs,
                attr_name,
                args,
            } => self.eval_proj_call(*lhs, attr_name, args, level, t_loc),
            Type::Ref(l) => Ok(ref_(self.eval_t_params(*l, level, t_loc)?)),
            Type::RefMut { before, after } => {
                let before = self.eval_t_params(*before, level, t_loc)?;
                let after = if let Some(after) = after {
                    Some(self.eval_t_params(*after, level, t_loc)?)
                } else {
                    None
                };
                Ok(ref_mut(before, after))
            }
            Type::Poly { name, mut params } => {
                for p in params.iter_mut() {
                    *p = self.eval_tp(&mem::take(p))?;
                }
                Ok(poly(name, params))
            }
            Type::And(l, r) => {
                let l = self.eval_t_params(*l, level, t_loc)?;
                let r = self.eval_t_params(*r, level, t_loc)?;
                Ok(self.intersection(&l, &r))
            }
            Type::Or(l, r) => {
                let l = self.eval_t_params(*l, level, t_loc)?;
                let r = self.eval_t_params(*r, level, t_loc)?;
                Ok(self.union(&l, &r))
            }
            Type::Not(l, r) => {
                let l = self.eval_t_params(*l, level, t_loc)?;
                let r = self.eval_t_params(*r, level, t_loc)?;
                Ok(not(l, r))
            }
            other if other.is_monomorphic() => Ok(other),
            _other => Err(EvalErrors::from(EvalError::feature_error(
                self.cfg.input.clone(),
                t_loc,
                "???",
                self.caused_by(),
            ))),
        }
    }

    fn eval_proj(&self, lhs: Type, rhs: Str, level: usize, t_loc: Location) -> EvalResult<Type> {
        // Currently Erg does not allow projection-types to be evaluated with type variables included.
        // All type variables will be dereferenced or fail.
        let (sub, opt_sup) = match lhs.clone() {
            Type::FreeVar(fv) if fv.is_linked() => {
                return self.eval_t_params(proj(fv.crack().clone(), rhs), level, t_loc)
            }
            Type::FreeVar(fv) if fv.is_unbound() => {
                let (sub, sup) = fv.get_bound_types().unwrap();
                (sub, Some(sup))
            }
            other => (other, None),
        };
        // cannot determine at this point
        if sub == Type::Never {
            return Ok(proj(lhs, rhs));
        }
        for ty_ctx in self.get_nominal_super_type_ctxs(&sub).ok_or_else(|| {
            EvalError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                t_loc,
                self.caused_by(),
                &rhs,
                None, // TODO:
            )
        })? {
            if let Ok(obj) = ty_ctx.get_const_local(&Token::symbol(&rhs), &self.name) {
                if let ValueObj::Type(quant_t) = obj {
                    let subst_ctx = SubstContext::new(&sub, self, t_loc);
                    let t = subst_ctx.substitute(quant_t.typ().clone())?;
                    let t = self.eval_t_params(t, level, t_loc)?;
                    return Ok(t);
                } else {
                    todo!()
                }
            }
            for (class, methods) in ty_ctx.methods_list.iter() {
                match (class, &opt_sup) {
                    (ClassDefType::ImplTrait { impl_trait, .. }, Some(sup)) => {
                        if !self.supertype_of(impl_trait, sup) {
                            continue;
                        }
                    }
                    (ClassDefType::ImplTrait { impl_trait, .. }, None) => {
                        if !self.supertype_of(impl_trait, &sub) {
                            continue;
                        }
                    }
                    _ => {}
                }
                if let Ok(obj) = methods.get_const_local(&Token::symbol(&rhs), &self.name) {
                    if let ValueObj::Type(quant_t) = obj {
                        let subst_ctx = SubstContext::new(&sub, self, t_loc);
                        let t = subst_ctx.substitute(quant_t.typ().clone())?;
                        let t = self.eval_t_params(t, level, t_loc)?;
                        return Ok(t);
                    } else {
                        todo!()
                    }
                }
            }
        }
        if lhs.is_unbound_var() {
            let (sub, sup) = enum_unwrap!(&lhs, Type::FreeVar).get_bound_types().unwrap();
            if self.is_trait(&sup) && !self.trait_impl_exists(&sub, &sup) {
                return Err(EvalErrors::from(EvalError::no_trait_impl_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    &sub,
                    &sup,
                    t_loc,
                    self.caused_by(),
                    None,
                )));
            }
        }
        // if the target can't be found in the supertype, the type will be dereferenced.
        // In many cases, it is still better to determine the type variable than if the target is not found.
        let coerced = self.deref_tyvar(lhs.clone(), Variance::Covariant, t_loc)?;
        if lhs != coerced {
            let proj = proj(coerced, rhs);
            self.eval_t_params(proj, level, t_loc).map(|t| {
                self.coerce(&lhs);
                t
            })
        } else {
            let proj = proj(lhs, rhs);
            Err(EvalErrors::from(EvalError::no_candidate_error(
                self.cfg.input.clone(),
                line!() as usize,
                &proj,
                t_loc,
                self.caused_by(),
                self.get_no_candidate_hint(&proj),
            )))
        }
    }

    fn eval_proj_call(
        &self,
        lhs: TyParam,
        attr_name: Str,
        args: Vec<TyParam>,
        level: usize,
        t_loc: Location,
    ) -> EvalResult<Type> {
        let t = self.get_tp_t(&lhs)?;
        for ty_ctx in self.get_nominal_super_type_ctxs(&t).ok_or_else(|| {
            EvalError::no_var_error(
                self.cfg.input.clone(),
                line!() as usize,
                t_loc,
                self.caused_by(),
                &attr_name,
                None, // TODO:
            )
        })? {
            if let Ok(obj) = ty_ctx.get_const_local(&Token::symbol(&attr_name), &self.name) {
                if let ValueObj::Subr(subr) = obj {
                    let is_method = subr.sig_t().self_t().is_some();
                    let mut pos_args = vec![];
                    if is_method {
                        pos_args.push(ValueObj::try_from(lhs).unwrap());
                    }
                    for pos_arg in args.into_iter() {
                        pos_args.push(ValueObj::try_from(pos_arg).unwrap());
                    }
                    let args = ValueArgs::new(pos_args, dict! {});
                    let t = self.call(subr, args, t_loc)?;
                    let t = enum_unwrap!(t, ValueObj::Type); // TODO: error handling
                    return Ok(t.into_typ());
                } else {
                    todo!()
                }
            }
            for (_class, methods) in ty_ctx.methods_list.iter() {
                if let Ok(obj) = methods.get_const_local(&Token::symbol(&attr_name), &self.name) {
                    if let ValueObj::Subr(subr) = obj {
                        let mut pos_args = vec![];
                        for pos_arg in args.into_iter() {
                            pos_args.push(ValueObj::try_from(pos_arg).unwrap());
                        }
                        let args = ValueArgs::new(pos_args, dict! {});
                        let t = self.call(subr, args, t_loc)?;
                        let t = enum_unwrap!(t, ValueObj::Type); // TODO: error handling
                        return Ok(t.into_typ());
                    } else {
                        todo!()
                    }
                }
            }
        }
        if lhs.is_unbound_var() {
            let (sub, sup) = enum_unwrap!(&lhs, TyParam::FreeVar)
                .get_bound_types()
                .unwrap();
            if self.is_trait(&sup) && !self.trait_impl_exists(&sub, &sup) {
                return Err(EvalErrors::from(EvalError::no_trait_impl_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    &sub,
                    &sup,
                    t_loc,
                    self.caused_by(),
                    None,
                )));
            }
        }
        // if the target can't be found in the supertype, the type will be dereferenced.
        // In many cases, it is still better to determine the type variable than if the target is not found.
        let coerced = self.deref_tp(lhs.clone(), Variance::Covariant, t_loc)?;
        if lhs != coerced {
            let proj = proj_call(coerced, attr_name, args);
            self.eval_t_params(proj, level, t_loc).map(|t| {
                self.coerce_tp(&lhs);
                t
            })
        } else {
            let proj = proj_call(lhs, attr_name, args);
            Err(EvalErrors::from(EvalError::no_candidate_error(
                self.cfg.input.clone(),
                line!() as usize,
                &proj,
                t_loc,
                self.caused_by(),
                self.get_no_candidate_hint(&proj),
            )))
        }
    }

    pub(crate) fn _eval_bound(
        &self,
        bound: TyBound,
        level: usize,
        t_loc: Location,
    ) -> EvalResult<TyBound> {
        match bound {
            TyBound::Sandwiched { sub, mid, sup } => {
                let sub = self.eval_t_params(sub, level, t_loc)?;
                let mid = self.eval_t_params(mid, level, t_loc)?;
                let sup = self.eval_t_params(sup, level, t_loc)?;
                Ok(TyBound::sandwiched(sub, mid, sup))
            }
            TyBound::Instance { name: inst, t } => Ok(TyBound::instance(
                inst,
                self.eval_t_params(t, level, t_loc)?,
            )),
        }
    }

    pub(crate) fn eval_pred(&self, p: Predicate) -> EvalResult<Predicate> {
        match p {
            Predicate::Value(_) | Predicate::Const(_) => Ok(p),
            Predicate::Equal { lhs, rhs } => Ok(Predicate::eq(lhs, self.eval_tp(&rhs)?)),
            Predicate::NotEqual { lhs, rhs } => Ok(Predicate::ne(lhs, self.eval_tp(&rhs)?)),
            Predicate::LessEqual { lhs, rhs } => Ok(Predicate::le(lhs, self.eval_tp(&rhs)?)),
            Predicate::GreaterEqual { lhs, rhs } => Ok(Predicate::ge(lhs, self.eval_tp(&rhs)?)),
            Predicate::And(l, r) => Ok(Predicate::and(self.eval_pred(*l)?, self.eval_pred(*r)?)),
            Predicate::Or(l, r) => Ok(Predicate::or(self.eval_pred(*l)?, self.eval_pred(*r)?)),
            Predicate::Not(l, r) => Ok(Predicate::not(self.eval_pred(*l)?, self.eval_pred(*r)?)),
        }
    }

    pub(crate) fn get_tp_t(&self, p: &TyParam) -> EvalResult<Type> {
        let p = self.eval_tp(p)?;
        match p {
            TyParam::Value(ValueObj::Mut(v)) => Ok(v.borrow().class().mutate()),
            TyParam::Value(v) => Ok(v_enum(set![v])),
            TyParam::Erased(t) => Ok((*t).clone()),
            TyParam::FreeVar(fv) => {
                if let Some(t) = fv.get_type() {
                    Ok(t)
                } else {
                    todo!()
                }
            }
            TyParam::Type(typ) => {
                if let Some(ctx) = self.get_nominal_type_ctx(&typ) {
                    let t = match ctx.kind {
                        ContextKind::Class => Type::ClassType,
                        ContextKind::Trait | ContextKind::StructuralTrait => Type::TraitType,
                        _ => unreachable!(),
                    };
                    Ok(t)
                } else {
                    Ok(Type::Type)
                }
            }
            TyParam::Mono(name) => self
                .rec_get_const_obj(&name)
                .map(|v| v_enum(set![v.clone()]))
                .ok_or_else(|| {
                    EvalErrors::from(EvalError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    ))
                }),
            TyParam::MonoQVar(name) => {
                panic!("Not instantiated type variable: {name}")
            }
            TyParam::Array(tps) => {
                let tp_t = self.get_tp_t(&tps[0])?;
                let t = array_t(tp_t, TyParam::value(tps.len()));
                Ok(t)
            }
            dict @ TyParam::Dict(_) => Ok(dict_t(dict)),
            TyParam::UnaryOp { op, val } => match op {
                OpKind::Mutate => Ok(self.get_tp_t(&val)?.mutate()),
                _ => todo!(),
            },
            TyParam::BinOp { op, lhs, rhs } => {
                let op_name = op_to_name(op);
                todo!("get type: {op_name}({lhs}, {rhs})")
            }
            other => todo!("{other}"),
        }
    }

    pub(crate) fn _get_tp_class(&self, p: &TyParam) -> EvalResult<Type> {
        let p = self.eval_tp(p)?;
        match p {
            TyParam::Value(v) => Ok(v.class()),
            TyParam::Erased(t) => Ok((*t).clone()),
            TyParam::FreeVar(fv) => {
                if let Some(t) = fv.get_type() {
                    Ok(t)
                } else {
                    todo!()
                }
            }
            TyParam::Type(_) => Ok(Type::Type),
            TyParam::Mono(name) => {
                self.rec_get_const_obj(&name)
                    .map(|v| v.class())
                    .ok_or_else(|| {
                        EvalErrors::from(EvalError::unreachable(
                            self.cfg.input.clone(),
                            fn_name!(),
                            line!(),
                        ))
                    })
            }
            other => todo!("{other}"),
        }
    }

    /// NOTE: lとrが型の場合はContextの方で判定する
    pub(crate) fn shallow_eq_tp(&self, lhs: &TyParam, rhs: &TyParam) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(l), TyParam::Type(r)) => l == r,
            (TyParam::Value(l), TyParam::Value(r)) => l == r,
            (TyParam::Erased(l), TyParam::Erased(r)) => l == r,
            (TyParam::Array(l), TyParam::Array(r)) => l == r,
            (TyParam::Tuple(l), TyParam::Tuple(r)) => l == r,
            (TyParam::Set(l), TyParam::Set(r)) => l == r, // FIXME:
            (TyParam::Dict(l), TyParam::Dict(r)) => l == r,
            (TyParam::FreeVar { .. }, TyParam::FreeVar { .. }) => true,
            (TyParam::Mono(l), TyParam::Mono(r)) => {
                if l == r {
                    true
                } else if let (Some(l), Some(r)) =
                    (self.rec_get_const_obj(l), self.rec_get_const_obj(r))
                {
                    l == r
                } else {
                    // lとrが型の場合は...
                    false
                }
            }
            (TyParam::BinOp { .. }, TyParam::BinOp { .. }) => todo!(),
            (TyParam::UnaryOp { .. }, TyParam::UnaryOp { .. }) => todo!(),
            (TyParam::App { .. }, TyParam::App { .. }) => todo!(),
            (TyParam::Mono(m), TyParam::Value(l)) | (TyParam::Value(l), TyParam::Mono(m)) => {
                if let Some(o) = self.rec_get_const_obj(m) {
                    o == l
                } else {
                    true
                }
            }
            (TyParam::Erased(t), _) => t.as_ref() == &self.get_tp_t(rhs).unwrap(),
            (_, TyParam::Erased(t)) => t.as_ref() == &self.get_tp_t(lhs).unwrap(),
            (TyParam::MonoQVar(_), _) | (_, TyParam::MonoQVar(_)) => false,
            (l, r) => todo!("l: {l}, r: {r}"),
        }
    }
}
