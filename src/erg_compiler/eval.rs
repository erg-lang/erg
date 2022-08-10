use std::mem;

use erg_common::Str;
use erg_common::{fn_name, set};
use erg_common::value::ValueObj;
use erg_common::dict::Dict;
use erg_common::rccell::RcCell;
use erg_common::set::{Set};
use erg_common::traits::Stream;
use erg_common::ty::{OpKind, TyParam, Type, Predicate, TyBound, ConstObj, SubrKind};
use OpKind::*;

use erg_parser::ast::*;
use erg_parser::token::Token;

use crate::table::{SymbolTable, TyVarTable};
use crate::error::{EvalError, EvalResult, TyCheckResult};

/// SubstTable::new([?T; 0], SymbolTable(Array(T, N))) => SubstTable{ params: { T: ?T; N: 0 } }
/// SubstTable::substitute([T; !N], SymbolTable(Array(T, N))): [?T; !0]
#[derive(Debug)]
struct SubstTable {
    params: Dict<Str, TyParam>,
}

impl SubstTable {
    pub fn new(substituted: &Type, ty_table: &SymbolTable) -> Self {
        let param_names = ty_table.impls.iter()
            .filter(|(_, vi)| vi.kind.is_parameter())
            .map(|(name, _)| name.inspect().clone());
        let self_ = SubstTable{
            params: param_names.zip(substituted.typarams().into_iter()).collect(),
        };
        // REVIEW: 順番は保証されるか? 引数がunnamed_paramsに入る可能性は?
        self_
    }

    fn substitute(&self, quant_t: Type, ty_table: &SymbolTable, level: usize) -> TyCheckResult<Type> {
        let bounds = ty_table.bounds();
        let tvtab = TyVarTable::new(level, bounds);
        let (inst, _) = SymbolTable::instantiate_t(quant_t, tvtab);
        for param in inst.typarams() {
            self.substitute_tp(&param, ty_table)?;
        }
        Ok(inst)
    }

    fn substitute_tp(&self, param: &TyParam, ty_table: &SymbolTable) -> TyCheckResult<()> {
        match param {
            TyParam::FreeVar(fv) => {
                if let Some(name) = fv.unbound_name() {
                    if let Some(v) = self.params.get(&name) {
                        ty_table.unify_tp(param, v, None, false)?;
                    }
                } else {
                    if fv.is_unbound() { panic!() }
                }
            }
            TyParam::BinOp{ lhs, rhs, .. } => {
                self.substitute_tp(lhs, ty_table)?;
                self.substitute_tp(rhs, ty_table)?;
            }
            TyParam::UnaryOp{ val, .. } => {
                self.substitute_tp(val, ty_table)?;
            }
            TyParam::Array(args)
            | TyParam::Tuple(args)
            | TyParam::App{ args, .. }
            | TyParam::PolyQVar{ args, .. } => {
                for arg in args.iter() {
                    self.substitute_tp(arg, ty_table)?;
                }
            }
            TyParam::Type(t) => { self.substitute_t(t, ty_table)?; },
            TyParam::MonoProj{ obj, attr } => todo!("{obj}.{attr}"),
            _ => {}
        }
        Ok(())
    }

    fn substitute_t(&self, t: &Type, ty_table: &SymbolTable) -> TyCheckResult<()> {
        match t {
            Type::FreeVar(fv) => {
                if let Some(name) = fv.unbound_name() {
                    if let Some(v) = self.params.get(&name) {
                        if let TyParam::Type(v) = v {
                            ty_table.unify(t, v, None, None)?;
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
pub struct Evaluator {
}

impl Evaluator {
    #[inline]
    pub fn new() -> Self { Self::default() }

    #[inline]
    pub(crate) fn eval_const_lit(&self, lit: &Literal) -> ValueObj { ValueObj::from(lit) }

    fn eval_const_acc(&self, _acc: &Accessor) -> Option<ValueObj> {
        todo!()
    }

    fn eval_const_bin(&self, _bin: &BinOp) -> Option<ValueObj> {
        todo!()
    }

    fn eval_const_unary(&self, _unary: &UnaryOp) -> Option<ValueObj> {
        todo!()
    }

    // TODO: kw args
    fn eval_args(&self, _args: &Args) -> Option<Vec<ValueObj>> {
        todo!()
    }

    fn eval_const_call(&self, call: &Call, table: &SymbolTable) -> Option<ValueObj> {
        if let Expr::Accessor(acc) = call.obj.as_ref() {
            match acc {
                Accessor::Local(name) if name.is_const() => {
                    if let Some(ConstObj::Subr(subr)) = table.consts.get(name.inspect()) {
                        let args = self.eval_args(&call.args)?;
                        Some(subr.call(args))
                    } else {
                        None
                    }
                }
                Accessor::Local(_) => None,
                Accessor::Attr(_attr) => todo!(),
                Accessor::TupleAttr(_attr) => todo!(),
                Accessor::SelfDot(_name) => todo!(),
                Accessor::Subscr(_subscr) => todo!(),
            }
        } else { None }
    }

    fn eval_const_def(&self, def: &Def) -> Option<ValueObj> {
        if def.is_const() {
            todo!()
        }
        None
    }

    // ConstExprを評価するのではなく、コンパイル時関数の式(AST上ではただのExpr)を評価する
    // コンパイル時評価できないならNoneを返す
    pub(crate) fn eval_const_expr(&self, expr: &Expr, table: &SymbolTable) -> Option<ValueObj> {
        match expr {
            Expr::Lit(lit) => Some(self.eval_const_lit(lit)),
            Expr::Accessor(acc) => self.eval_const_acc(acc),
            Expr::BinOp(bin) => self.eval_const_bin(bin),
            Expr::UnaryOp(unary) => self.eval_const_unary(unary),
            Expr::Call(call) => self.eval_const_call(call, table),
            Expr::Def(def) => self.eval_const_def(def),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn eval_const_block(&self, block: &Block, table: &SymbolTable) -> Option<ValueObj> {
        for chunk in block.iter().rev().skip(1).rev() {
            self.eval_const_expr(chunk, table)?;
        }
        self.eval_const_expr(block.last().unwrap(), table)
    }

    fn eval_bin_lit(&self, op: OpKind, lhs: ValueObj, rhs: ValueObj) -> EvalResult<ValueObj> {
        match op {
            Add => lhs.try_add(rhs).ok_or(EvalError::unreachable(fn_name!(), line!())),
            Sub => lhs.try_sub(rhs).ok_or(EvalError::unreachable(fn_name!(), line!())),
            Mul => lhs.try_mul(rhs).ok_or(EvalError::unreachable(fn_name!(), line!())),
            Div => lhs.try_div(rhs).ok_or(EvalError::unreachable(fn_name!(), line!())),
            Gt => lhs.try_gt(rhs).ok_or(EvalError::unreachable(fn_name!(), line!())),
            Ge => lhs.try_ge(rhs).ok_or(EvalError::unreachable(fn_name!(), line!())),
            Eq => lhs.try_eq(rhs).ok_or(EvalError::unreachable(fn_name!(), line!())),
            Ne => lhs.try_ne(rhs).ok_or(EvalError::unreachable(fn_name!(), line!())),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn eval_bin_tp(&self, op: OpKind, lhs: &TyParam, rhs: &TyParam) ->  EvalResult<TyParam> {
        match (lhs, rhs) {
            (TyParam::ConstObj(ConstObj::Value(lhs)), TyParam::ConstObj(ConstObj::Value(rhs))) =>
                self.eval_bin_lit(op, lhs.clone(), rhs.clone())
                    .map(|v| TyParam::value(v)),
            (TyParam::ConstObj(ConstObj::MutValue(lhs)), TyParam::ConstObj(ConstObj::Value(rhs))) =>
                self.eval_bin_lit(op, lhs.borrow().clone(), rhs.clone())
                    .map(|v| TyParam::ConstObj(ConstObj::MutValue(RcCell::new(v)))),
            (TyParam::FreeVar(fv), r) => {
                if fv.is_linked() {
                    self.eval_bin_tp(op, &*fv.crack(), r)
                } else {
                    Err(EvalError::unreachable(fn_name!(), line!()))
                }
            },
            (l, TyParam::FreeVar(fv)) => {
                if fv.is_linked() {
                    self.eval_bin_tp(op, l, &*fv.crack())
                } else {
                    Err(EvalError::unreachable(fn_name!(), line!()))
                }
            },
            (e @ TyParam::Erased(_), _)
            | (_, e @ TyParam::Erased(_)) => Ok(e.clone()),
            (l, r) => todo!("{l} {op} {r}"),
        }
    }

    fn eval_unary_lit(&self, op: OpKind, val: ConstObj) -> EvalResult<ConstObj> {
        match op {
            Pos => todo!(),
            Neg => todo!(),
            Invert => todo!(),
            Mutate => if let ConstObj::Value(v) = val {
                Ok(ConstObj::MutValue(RcCell::new(v)))
            } else { todo!() },
            other => todo!("{other}"),
        }
    }

    fn eval_unary_tp(&self, op: OpKind, val: &TyParam) ->  EvalResult<TyParam> {
        match val {
            TyParam::ConstObj(c) =>
                self.eval_unary_lit(op, c.clone()).map(|c| TyParam::cons(c)),
            TyParam::FreeVar(fv) if fv.is_linked() => {
                self.eval_unary_tp(op, &*fv.crack())
            },
            e @ TyParam::Erased(_) => Ok(e.clone()),
            other => todo!("{op} {other}"),
        }
    }

    fn eval_app(&self, _name: &Str, _args: &Vec<TyParam>) -> EvalResult<TyParam> {
        todo!()
    }

    /// 量化変数などはそのまま返す
    pub(crate) fn eval_tp(&self, p: &TyParam, table: &SymbolTable) -> EvalResult<TyParam> {
        match p {
            TyParam::FreeVar(fv) if fv.is_linked() =>
                self.eval_tp(&fv.crack(), table),
            TyParam::Mono(name) =>
                table.consts.get(name)
                    .and_then(|c| match c {
                        ConstObj::Value(v) => Some(TyParam::value(v.clone())),
                        _ => None,
                    }).ok_or(EvalError::unreachable(fn_name!(), line!())),
            TyParam::BinOp{ op, lhs, rhs } =>
                self.eval_bin_tp(*op, lhs, rhs),
            TyParam::UnaryOp{ op, val } =>
                self.eval_unary_tp(*op, val),
            TyParam::App{ name, args } =>
                self.eval_app(name, args),
            p @ (
                TyParam::Type(_) | TyParam::Erased(_) | TyParam::ConstObj(_) | TyParam::FreeVar(_) | TyParam::MonoQVar(_)
            ) => Ok(p.clone()),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn eval_t(&self, substituted: Type, table: &SymbolTable, level: usize) -> EvalResult<Type> {
        match substituted {
            Type::FreeVar(fv) if fv.is_linked() =>
                self.eval_t(fv.crack().clone(), table, level),
            Type::Subr(mut subr) => {
                let kind = match subr.kind {
                    SubrKind::FuncMethod(self_t) => {
                        SubrKind::fn_met(self.eval_t(*self_t, table, level)?)
                    }
                    SubrKind::ProcMethod{ before, after } => {
                        let before = self.eval_t(*before, table, level)?;
                        if let Some(after) = after {
                            let after = self.eval_t(*after, table, level)?;
                            SubrKind::pr_met(before, Some(after))
                        } else {
                            SubrKind::pr_met(before, None)
                        }
                    }
                    other => other,
                };
                for p in subr.non_default_params.iter_mut() {
                    p.ty = self.eval_t(mem::take(&mut p.ty), table, level)?;
                }
                for p in subr.default_params.iter_mut() {
                    p.ty = self.eval_t(mem::take(&mut p.ty), table, level)?;
                }
                let return_t = self.eval_t(*subr.return_t, table, level)?;
                Ok(Type::subr(kind, subr.non_default_params, subr.default_params, return_t))
            },
            Type::Array{ t, len } => {
                let t = self.eval_t(*t, table, level)?;
                let len = self.eval_tp(&len, table)?;
                Ok(Type::array(t, len))
            },
            Type::Refinement(refine) => {
                let mut preds = Set::with_capacity(refine.preds.len());
                for pred in refine.preds.into_iter() {
                    preds.insert(self.eval_pred(pred, table)?);
                }
                Ok(Type::refinement(refine.var, *refine.t, preds))
            },
            // [?T; 0].MutType! == [?T; !0]
            Type::MonoProj{ lhs, rhs } => {
                for ty_table in table.get_sorted_supertype_tables(&lhs) {
                    if let Ok(obj) = ty_table.get_local(&Token::symbol(&rhs), &table.name) {
                        if let ConstObj::Type(quant_t) = obj {
                            let subst_tab = SubstTable::new(&lhs, ty_table);
                            let t = subst_tab.substitute(*quant_t, ty_table, level)?;
                            let t = self.eval_t(t, &table, level)?;
                            return Ok(t)
                        } else { todo!() }
                    }
                }
                todo!()
            },
            Type::Range(l) => Ok(Type::range(self.eval_t(*l, table, level)?)),
            Type::Iter(l) => Ok(Type::iter(self.eval_t(*l, table, level)?)),
            Type::Ref(l) => Ok(Type::refer(self.eval_t(*l, table, level)?)),
            Type::RefMut(l) => Ok(Type::ref_mut(self.eval_t(*l, table, level)?)),
            Type::Option(l) => Ok(Type::option_mut(self.eval_t(*l, table, level)?)),
            Type::OptionMut(l) => Ok(Type::option_mut(self.eval_t(*l, table, level)?)),
            Type::VarArgs(l) => Ok(Type::var_args(self.eval_t(*l, table, level)?)),
            Type::Poly{ name, mut params } => {
                for p in params.iter_mut() {
                    *p = self.eval_tp(&mem::take(p), table)?;
                }
                Ok(Type::poly(name, params))
            },
            other if other.is_monomorphic() => Ok(other),
            other => todo!("{other}"),
        }
    }

    pub(crate) fn _eval_bound(&self, bound: TyBound, table: &SymbolTable, level: usize) -> EvalResult<TyBound> {
        match bound {
            TyBound::Subtype{ sub, sup } =>
                Ok(TyBound::subtype(
                    self.eval_t(sub, table, level)?,
                    self.eval_t(sup, table, level)?
                )),
            TyBound::Instance{ name: inst, t } =>
                Ok(TyBound::instance(inst, self.eval_t(t, table, level)?)),
        }
    }

    pub(crate) fn eval_pred(&self, p: Predicate, table: &SymbolTable) -> EvalResult<Predicate> {
        match p {
            Predicate::Value(_) | Predicate::Const(_) => Ok(p),
            Predicate::Equal{ lhs, rhs } =>
                Ok(Predicate::eq(lhs, self.eval_tp(&rhs, table)?)),
            Predicate::NotEqual{ lhs, rhs } =>
                Ok(Predicate::ne(lhs, self.eval_tp(&rhs, table)?)),
            Predicate::LessEqual{ lhs, rhs } =>
                Ok(Predicate::le(lhs, self.eval_tp(&rhs, table)?)),
            Predicate::GreaterEqual{ lhs, rhs } =>
                Ok(Predicate::ge(lhs, self.eval_tp(&rhs, table)?)),
            Predicate::And(l, r) =>
                Ok(Predicate::and(self.eval_pred(*l, table)?, self.eval_pred(*r, table)?)),
            Predicate::Or(l, r) =>
                Ok(Predicate::or(self.eval_pred(*l, table)?, self.eval_pred(*r, table)?)),
            Predicate::Not(l, r) =>
                Ok(Predicate::not(self.eval_pred(*l, table)?, self.eval_pred(*r, table)?)),
        }
    }

    pub(crate) fn get_tp_t(&self, p: &TyParam, bounds: Option<&Set<TyBound>>, table: &SymbolTable) -> EvalResult<Type> {
        let p = self.eval_tp(p, table)?;
        match p {
            TyParam::ConstObj(ConstObj::Value(v)) => Ok(Type::enum_t(set![v])),
            TyParam::ConstObj(ConstObj::MutValue(v)) => Ok(v.borrow().class().mutate()),
            TyParam::Erased(t) => Ok((&*t).clone()),
            TyParam::FreeVar(fv) =>
                if let Some(t) = fv.type_of() { Ok(t) } else { todo!() },
            TyParam::Type(_) => Ok(Type::Type),
            TyParam::Mono(name) =>
                table.consts.get(&name)
                    .and_then(|c| match c {
                        ConstObj::Value(v) => Some(Type::enum_t(set![v.clone()])),
                        _ => None,
                    }).ok_or(EvalError::unreachable(fn_name!(), line!())),
            TyParam::MonoQVar(name) => {
                if let Some(bs) = bounds {
                    if let Some(bound) = bs.iter().find(|b| b.mentions_as_instance(&name)) {
                        Ok(bound.t().clone())
                    } else { todo!() }
                } else { todo!() }
            },
            TyParam::UnaryOp{ op, val } => {
                match op {
                    OpKind::Mutate => Ok(self.get_tp_t(&val, bounds, table)?.mutate()),
                    _ => todo!(),
                }
            },
            other => todo!("{other}"),
        }
    }

    pub(crate) fn _get_tp_class(&self, p: &TyParam, table: &SymbolTable) -> EvalResult<Type> {
        let p = self.eval_tp(p, table)?;
        match p {
            TyParam::ConstObj(ConstObj::Value(v)) => Ok(v.class()),
            TyParam::Erased(t) => Ok((&*t).clone()),
            | TyParam::FreeVar(fv) =>
                if let Some(t) = fv.type_of() { Ok(t) } else { todo!() },
            TyParam::Type(_) => Ok(Type::Type),
            TyParam::Mono(name) =>
                table.consts.get(&name)
                    .and_then(|c| match c {
                        ConstObj::Value(v) => Some(v.class()),
                        _ => None,
                    }).ok_or(EvalError::unreachable(fn_name!(), line!())),
            other => todo!("{other}"),
        }
    }

    /// NOTE: lとrが型の場合はSymbolTableの方で判定する
    pub(crate) fn shallow_eq_tp(&self, lhs: &TyParam, rhs: &TyParam, table: &SymbolTable) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(l), TyParam::Type(r)) => l == r,
            (TyParam::ConstObj(l), TyParam::ConstObj(r)) => l == r,
            (TyParam::Erased(l), TyParam::Erased(r)) => l == r,
            (TyParam::FreeVar{ .. }, TyParam::FreeVar{ .. }) => true,
            (TyParam::Mono(l), TyParam::Mono(r)) => {
                if l == r { true }
                else if let (Some(l), Some(r)) = (table.consts.get(l), table.consts.get(r)) { l == r }
                else {
                    // lとrが型の場合は...
                    false
                }
            },
            (TyParam::BinOp{ .. }, TyParam::BinOp{ .. }) => todo!(),
            (TyParam::UnaryOp{ .. }, TyParam::UnaryOp{ .. }) => todo!(),
            (TyParam::App{ .. }, TyParam::App{ .. }) => todo!(),
            (TyParam::Mono(m), TyParam::ConstObj(l))
            | (TyParam::ConstObj(l), TyParam::Mono(m)) =>
                if let Some(o) = table.consts.get(m) { o == l } else { true },
            (TyParam::MonoQVar(_), _) | (_, TyParam::MonoQVar(_)) => false,
            (l, r) => todo!("l: {l}, r: {r}"),
        }
    }
}
