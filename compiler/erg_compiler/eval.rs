use std::mem;

use erg_common::dict::Dict;
use erg_common::rccell::RcCell;
use erg_common::set::Set;
use erg_common::traits::Stream;
use erg_common::vis::Field;
use erg_common::{fn_name, set};
use erg_common::{RcArray, Str};
use OpKind::*;

use erg_parser::ast::*;
use erg_parser::token::{Token, TokenKind};

use erg_type::constructors::{
    enum_t, mono_proj, poly_class, poly_trait, ref_, ref_mut, refinement, subr_t, var_args,
};
use erg_type::typaram::{OpKind, TyParam};
use erg_type::value::ValueObj;
use erg_type::{Predicate, SubrKind, TyBound, Type};

use crate::context::instantiate::TyVarContext;
use crate::context::Context;
use crate::error::{EvalError, EvalResult, TyCheckResult};

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

fn try_get_op_kind_from_token(kind: TokenKind) -> Result<OpKind, ()> {
    match kind {
        TokenKind::Plus => Ok(OpKind::Add),
        TokenKind::Minus => Ok(OpKind::Sub),
        TokenKind::Star => Ok(OpKind::Mul),
        TokenKind::Slash => Ok(OpKind::Div),
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
        _other => Err(()),
    }
}

#[inline]
pub(crate) fn eval_lit(lit: &Literal) -> ValueObj {
    let t = type_from_token_kind(lit.token.kind);
    ValueObj::from_str(t, lit.token.content.clone())
}

/// SubstContext::new([?T; 0], Context(Array(T, N))) => SubstContext{ params: { T: ?T; N: 0 } }
/// SubstContext::substitute([T; !N], Context(Array(T, N))): [?T; !0]
#[derive(Debug)]
struct SubstContext {
    params: Dict<Str, TyParam>,
}

impl SubstContext {
    pub fn new(substituted: &Type, ty_ctx: &Context) -> Self {
        let param_names = ty_ctx.params.iter().map(|(opt_name, _)| {
            opt_name
                .as_ref()
                .map_or_else(|| Str::ever("_"), |n| n.inspect().clone())
        });
        // REVIEW: 順番は保証されるか? 引数がunnamed_paramsに入る可能性は?
        SubstContext {
            params: param_names
                .zip(substituted.typarams().into_iter())
                .collect(),
        }
    }

    fn substitute(
        &self,
        quant_t: Type,
        ty_ctx: &Context,
        level: usize,
        ctx: &Context,
    ) -> TyCheckResult<Type> {
        let bounds = ty_ctx.type_params_bounds();
        let mut tv_ctx = TyVarContext::new(level, bounds, ctx);
        let inst = Context::instantiate_t(quant_t, &mut tv_ctx);
        for param in inst.typarams() {
            self.substitute_tp(&param, ty_ctx)?;
        }
        Ok(inst)
    }

    fn substitute_tp(&self, param: &TyParam, ty_ctx: &Context) -> TyCheckResult<()> {
        match param {
            TyParam::FreeVar(fv) => {
                if let Some(name) = fv.unbound_name() {
                    if let Some(v) = self.params.get(&name) {
                        ty_ctx.unify_tp(param, v, None, false)?;
                    }
                } else if fv.is_unbound() {
                    panic!()
                }
            }
            TyParam::BinOp { lhs, rhs, .. } => {
                self.substitute_tp(lhs, ty_ctx)?;
                self.substitute_tp(rhs, ty_ctx)?;
            }
            TyParam::UnaryOp { val, .. } => {
                self.substitute_tp(val, ty_ctx)?;
            }
            TyParam::Array(args)
            | TyParam::Tuple(args)
            | TyParam::App { args, .. }
            | TyParam::PolyQVar { args, .. } => {
                for arg in args.iter() {
                    self.substitute_tp(arg, ty_ctx)?;
                }
            }
            TyParam::Type(t) => {
                self.substitute_t(t, ty_ctx)?;
            }
            TyParam::MonoProj { obj, attr } => todo!("{obj}.{attr}"),
            _ => {}
        }
        Ok(())
    }

    fn substitute_t(&self, t: &Type, ty_ctx: &Context) -> TyCheckResult<()> {
        match t {
            Type::FreeVar(fv) => {
                if let Some(name) = fv.unbound_name() {
                    if let Some(v) = self.params.get(&name) {
                        if let TyParam::Type(v) = v {
                            ty_ctx.unify(t, v, None, None)?;
                        } else {
                            panic!()
                        }
                    }
                }
            }
            t => todo!("{t}"),
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Evaluator {}

impl Evaluator {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    fn eval_const_acc(&self, _acc: &Accessor, ctx: &Context) -> Option<ValueObj> {
        match _acc {
            Accessor::Local(local) => {
                if let Some(val) = ctx.rec_get_const_obj(local.inspect()) {
                    Some(val.clone())
                } else {
                    None
                }
            }
            Accessor::Attr(attr) => {
                let _obj = self.eval_const_expr(&attr.obj, ctx)?;
                todo!()
            }
            _ => todo!(),
        }
    }

    fn eval_const_bin(&self, bin: &BinOp) -> Option<ValueObj> {
        match (bin.args[0].as_ref(), bin.args[1].as_ref()) {
            (Expr::Lit(l), Expr::Lit(r)) => {
                let op = try_get_op_kind_from_token(bin.op.kind).ok()?;
                self.eval_bin_lit(op, eval_lit(l), eval_lit(r)).ok()
            }
            _ => None,
        }
    }

    fn eval_const_unary(&self, unary: &UnaryOp) -> Option<ValueObj> {
        match unary.args[0].as_ref() {
            Expr::Lit(lit) => {
                let op = try_get_op_kind_from_token(unary.op.kind).ok()?;
                self.eval_unary_lit(op, eval_lit(lit)).ok()
            }
            _ => None,
        }
    }

    // TODO: kw args
    fn eval_args(&self, _args: &Args) -> Option<Vec<ValueObj>> {
        todo!()
    }

    fn eval_const_call(&self, call: &Call, ctx: &Context) -> Option<ValueObj> {
        if let Expr::Accessor(acc) = call.obj.as_ref() {
            match acc {
                Accessor::Local(name) if name.is_const() => {
                    if let Some(ValueObj::Subr(subr)) = ctx.rec_get_const_obj(&name.inspect()) {
                        let args = self.eval_args(&call.args)?;
                        Some(subr.call(args))
                    } else {
                        None
                    }
                }
                Accessor::Local(_) => None,
                Accessor::Attr(_attr) => todo!(),
                Accessor::TupleAttr(_attr) => todo!(),
                Accessor::Public(_name) => todo!(),
                Accessor::Subscr(_subscr) => todo!(),
            }
        } else {
            None
        }
    }

    fn eval_const_def(&self, def: &Def) -> Option<ValueObj> {
        if def.is_const() {
            todo!()
        }
        None
    }

    fn eval_const_array(&self, arr: &Array, ctx: &Context) -> Option<ValueObj> {
        let mut elems = vec![];
        match arr {
            Array::Normal(arr) => {
                for elem in arr.elems.pos_args().iter() {
                    if let Some(elem) = self.eval_const_expr(&elem.expr, ctx) {
                        elems.push(elem);
                    } else {
                        return None;
                    }
                }
            }
            _ => {
                return None;
            }
        }
        Some(ValueObj::Array(RcArray::from(elems)))
    }

    fn eval_const_record(&self, record: &Record, ctx: &Context) -> Option<ValueObj> {
        let mut attrs = vec![];
        for attr in record.attrs.iter() {
            if let Some(elem) = self.eval_const_block(&attr.body.block, ctx) {
                let ident = match &attr.sig {
                    Signature::Var(var) => match &var.pat {
                        VarPattern::Ident(ident) => {
                            Field::new(ident.vis(), ident.inspect().clone())
                        }
                        _ => todo!(),
                    },
                    _ => todo!(),
                };
                attrs.push((ident, elem));
            } else {
                return None;
            }
        }
        Some(ValueObj::Record(attrs.into_iter().collect()))
    }

    // ConstExprを評価するのではなく、コンパイル時関数の式(AST上ではただのExpr)を評価する
    // コンパイル時評価できないならNoneを返す
    pub(crate) fn eval_const_expr(&self, expr: &Expr, ctx: &Context) -> Option<ValueObj> {
        match expr {
            Expr::Lit(lit) => Some(eval_lit(lit)),
            Expr::Accessor(acc) => self.eval_const_acc(acc, ctx),
            Expr::BinOp(bin) => self.eval_const_bin(bin),
            Expr::UnaryOp(unary) => self.eval_const_unary(unary),
            Expr::Call(call) => self.eval_const_call(call, ctx),
            Expr::Def(def) => self.eval_const_def(def),
            Expr::Array(arr) => self.eval_const_array(arr, ctx),
            Expr::Record(rec) => self.eval_const_record(rec, ctx),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn eval_const_block(&self, block: &Block, ctx: &Context) -> Option<ValueObj> {
        for chunk in block.iter().rev().skip(1).rev() {
            self.eval_const_expr(chunk, ctx)?;
        }
        self.eval_const_expr(block.last().unwrap(), ctx)
    }

    fn eval_bin_lit(&self, op: OpKind, lhs: ValueObj, rhs: ValueObj) -> EvalResult<ValueObj> {
        match op {
            Add => lhs
                .try_add(rhs)
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            Sub => lhs
                .try_sub(rhs)
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            Mul => lhs
                .try_mul(rhs)
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            Div => lhs
                .try_div(rhs)
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            Gt => lhs
                .try_gt(rhs)
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            Ge => lhs
                .try_ge(rhs)
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            Eq => lhs
                .try_eq(rhs)
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            Ne => lhs
                .try_ne(rhs)
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
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
                .eval_bin_lit(op, lhs.borrow().clone(), rhs.clone())
                .map(|v| TyParam::Value(ValueObj::Mut(RcCell::new(v)))),
            (TyParam::Value(lhs), TyParam::Value(rhs)) => self
                .eval_bin_lit(op, lhs.clone(), rhs.clone())
                .map(TyParam::value),
            (TyParam::FreeVar(fv), r) => {
                if fv.is_linked() {
                    self.eval_bin_tp(op, &*fv.crack(), r)
                } else {
                    Err(EvalError::unreachable(fn_name!(), line!()))
                }
            }
            (l, TyParam::FreeVar(fv)) => {
                if fv.is_linked() {
                    self.eval_bin_tp(op, l, &*fv.crack())
                } else {
                    Err(EvalError::unreachable(fn_name!(), line!()))
                }
            }
            (e @ TyParam::Erased(_), _) | (_, e @ TyParam::Erased(_)) => Ok(e.clone()),
            (l, r) => todo!("{l} {op} {r}"),
        }
    }

    fn eval_unary_lit(&self, op: OpKind, val: ValueObj) -> EvalResult<ValueObj> {
        match op {
            Pos => todo!(),
            Neg => todo!(),
            Invert => todo!(),
            Mutate => Ok(ValueObj::Mut(RcCell::new(val))),
            other => todo!("{other}"),
        }
    }

    fn eval_unary_tp(&self, op: OpKind, val: &TyParam) -> EvalResult<TyParam> {
        match val {
            TyParam::Value(c) => self
                .eval_unary_lit(op, c.clone())
                .map(|v| TyParam::Value(v)),
            TyParam::FreeVar(fv) if fv.is_linked() => self.eval_unary_tp(op, &*fv.crack()),
            e @ TyParam::Erased(_) => Ok(e.clone()),
            other => todo!("{op} {other}"),
        }
    }

    fn eval_app(&self, _name: &Str, _args: &[TyParam]) -> EvalResult<TyParam> {
        todo!()
    }

    /// 量化変数などはそのまま返す
    pub(crate) fn eval_tp(&self, p: &TyParam, ctx: &Context) -> EvalResult<TyParam> {
        match p {
            TyParam::FreeVar(fv) if fv.is_linked() => self.eval_tp(&fv.crack(), ctx),
            TyParam::Mono(name) => ctx
                .rec_get_const_obj(name)
                .map(|v| TyParam::value(v.clone()))
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            TyParam::BinOp { op, lhs, rhs } => self.eval_bin_tp(*op, lhs, rhs),
            TyParam::UnaryOp { op, val } => self.eval_unary_tp(*op, val),
            TyParam::App { name, args } => self.eval_app(name, args),
            p @ (TyParam::Type(_)
            | TyParam::Erased(_)
            | TyParam::Value(_)
            | TyParam::FreeVar(_)
            | TyParam::MonoQVar(_)) => Ok(p.clone()),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn eval_t_params(
        &self,
        substituted: Type,
        ctx: &Context,
        level: usize,
    ) -> EvalResult<Type> {
        match substituted {
            Type::FreeVar(fv) if fv.is_linked() => {
                self.eval_t_params(fv.crack().clone(), ctx, level)
            }
            Type::Subr(mut subr) => {
                let kind = match subr.kind {
                    SubrKind::FuncMethod(self_t) => {
                        SubrKind::fn_met(self.eval_t_params(*self_t, ctx, level)?)
                    }
                    SubrKind::ProcMethod { before, after } => {
                        let before = self.eval_t_params(*before, ctx, level)?;
                        if let Some(after) = after {
                            let after = self.eval_t_params(*after, ctx, level)?;
                            SubrKind::pr_met(before, Some(after))
                        } else {
                            SubrKind::pr_met(before, None)
                        }
                    }
                    other => other,
                };
                for p in subr.non_default_params.iter_mut() {
                    p.ty = self.eval_t_params(mem::take(&mut p.ty), ctx, level)?;
                }
                for p in subr.default_params.iter_mut() {
                    p.ty = self.eval_t_params(mem::take(&mut p.ty), ctx, level)?;
                }
                let return_t = self.eval_t_params(*subr.return_t, ctx, level)?;
                Ok(subr_t(
                    kind,
                    subr.non_default_params,
                    subr.default_params,
                    return_t,
                ))
            }
            Type::Refinement(refine) => {
                let mut preds = Set::with_capacity(refine.preds.len());
                for pred in refine.preds.into_iter() {
                    preds.insert(self.eval_pred(pred, ctx)?);
                }
                Ok(refinement(refine.var, *refine.t, preds))
            }
            // [?T; 0].MutType! == [?T; !0]
            Type::MonoProj { lhs, rhs } => {
                for (_ty, ty_ctx) in ctx.rec_get_nominal_super_type_ctxs(&lhs) {
                    if let Ok(obj) = ty_ctx.get_const_local(&Token::symbol(&rhs), &ctx.name) {
                        if let ValueObj::Type(quant_t) = obj {
                            let subst_ctx = SubstContext::new(&lhs, ty_ctx);
                            let t = subst_ctx.substitute(*quant_t, ty_ctx, level, ctx)?;
                            let t = self.eval_t_params(t, ctx, level)?;
                            return Ok(t);
                        } else {
                            todo!()
                        }
                    }
                }
                if let Some(outer) = &ctx.outer {
                    self.eval_t_params(mono_proj(*lhs, rhs), outer, level)
                } else {
                    todo!(
                        "{lhs}.{rhs} not found in [{}]",
                        erg_common::fmt_iter(
                            ctx.rec_get_nominal_super_type_ctxs(&lhs)
                                .map(|(_, ctx)| &ctx.name)
                        )
                    )
                }
            }
            Type::Ref(l) => Ok(ref_(self.eval_t_params(*l, ctx, level)?)),
            Type::RefMut(l) => Ok(ref_mut(self.eval_t_params(*l, ctx, level)?)),
            Type::VarArgs(l) => Ok(var_args(self.eval_t_params(*l, ctx, level)?)),
            Type::PolyClass { name, mut params } => {
                for p in params.iter_mut() {
                    *p = self.eval_tp(&mem::take(p), ctx)?;
                }
                Ok(poly_class(name, params))
            }
            Type::PolyTrait { name, mut params } => {
                for p in params.iter_mut() {
                    *p = self.eval_tp(&mem::take(p), ctx)?;
                }
                Ok(poly_trait(name, params))
            }
            other if other.is_monomorphic() => Ok(other),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn _eval_bound(
        &self,
        bound: TyBound,
        ctx: &Context,
        level: usize,
    ) -> EvalResult<TyBound> {
        match bound {
            TyBound::Sandwiched { sub, mid, sup } => {
                let sub = self.eval_t_params(sub, ctx, level)?;
                let mid = self.eval_t_params(mid, ctx, level)?;
                let sup = self.eval_t_params(sup, ctx, level)?;
                Ok(TyBound::sandwiched(sub, mid, sup))
            }
            TyBound::Instance { name: inst, t } => {
                Ok(TyBound::instance(inst, self.eval_t_params(t, ctx, level)?))
            }
        }
    }

    pub(crate) fn eval_pred(&self, p: Predicate, ctx: &Context) -> EvalResult<Predicate> {
        match p {
            Predicate::Value(_) | Predicate::Const(_) => Ok(p),
            Predicate::Equal { lhs, rhs } => Ok(Predicate::eq(lhs, self.eval_tp(&rhs, ctx)?)),
            Predicate::NotEqual { lhs, rhs } => Ok(Predicate::ne(lhs, self.eval_tp(&rhs, ctx)?)),
            Predicate::LessEqual { lhs, rhs } => Ok(Predicate::le(lhs, self.eval_tp(&rhs, ctx)?)),
            Predicate::GreaterEqual { lhs, rhs } => {
                Ok(Predicate::ge(lhs, self.eval_tp(&rhs, ctx)?))
            }
            Predicate::And(l, r) => Ok(Predicate::and(
                self.eval_pred(*l, ctx)?,
                self.eval_pred(*r, ctx)?,
            )),
            Predicate::Or(l, r) => Ok(Predicate::or(
                self.eval_pred(*l, ctx)?,
                self.eval_pred(*r, ctx)?,
            )),
            Predicate::Not(l, r) => Ok(Predicate::not(
                self.eval_pred(*l, ctx)?,
                self.eval_pred(*r, ctx)?,
            )),
        }
    }

    pub(crate) fn get_tp_t(&self, p: &TyParam, ctx: &Context) -> EvalResult<Type> {
        let p = self.eval_tp(p, ctx)?;
        match p {
            TyParam::Value(ValueObj::Mut(v)) => Ok(v.borrow().class().mutate()),
            TyParam::Value(v) => Ok(enum_t(set![v])),
            TyParam::Erased(t) => Ok((*t).clone()),
            TyParam::FreeVar(fv) => {
                if let Some(t) = fv.type_of() {
                    Ok(t)
                } else {
                    todo!()
                }
            }
            TyParam::Type(_) => Ok(Type::Type),
            TyParam::Mono(name) => ctx
                .consts
                .get(&name)
                .map(|v| enum_t(set![v.clone()]))
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            TyParam::MonoQVar(name) => {
                panic!("Not instantiated type variable: {name}")
            }
            TyParam::UnaryOp { op, val } => match op {
                OpKind::Mutate => Ok(self.get_tp_t(&val, ctx)?.mutate()),
                _ => todo!(),
            },
            other => todo!("{other}"),
        }
    }

    pub(crate) fn _get_tp_class(&self, p: &TyParam, ctx: &Context) -> EvalResult<Type> {
        let p = self.eval_tp(p, ctx)?;
        match p {
            TyParam::Value(v) => Ok(v.class()),
            TyParam::Erased(t) => Ok((*t).clone()),
            TyParam::FreeVar(fv) => {
                if let Some(t) = fv.type_of() {
                    Ok(t)
                } else {
                    todo!()
                }
            }
            TyParam::Type(_) => Ok(Type::Type),
            TyParam::Mono(name) => ctx
                .consts
                .get(&name)
                .map(|v| v.class())
                .ok_or_else(|| EvalError::unreachable(fn_name!(), line!())),
            other => todo!("{other}"),
        }
    }

    /// NOTE: lとrが型の場合はContextの方で判定する
    pub(crate) fn shallow_eq_tp(&self, lhs: &TyParam, rhs: &TyParam, ctx: &Context) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(l), TyParam::Type(r)) => l == r,
            (TyParam::Value(l), TyParam::Value(r)) => l == r,
            (TyParam::Erased(l), TyParam::Erased(r)) => l == r,
            (TyParam::FreeVar { .. }, TyParam::FreeVar { .. }) => true,
            (TyParam::Mono(l), TyParam::Mono(r)) => {
                if l == r {
                    true
                } else if let (Some(l), Some(r)) = (ctx.consts.get(l), ctx.consts.get(r)) {
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
                if let Some(o) = ctx.consts.get(m) {
                    o == l
                } else {
                    true
                }
            }
            (TyParam::MonoQVar(_), _) | (_, TyParam::MonoQVar(_)) => false,
            (l, r) => todo!("l: {l}, r: {r}"),
        }
    }
}
