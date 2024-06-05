use std::mem;
use std::ops::Drop;

use erg_common::consts::DEBUG_MODE;
use erg_common::dict::Dict;
use erg_common::error::Location;
#[allow(unused)]
use erg_common::log;
use erg_common::set::Set;
use erg_common::shared::Shared;
use erg_common::traits::{Locational, Stream};
use erg_common::{dict, fmt_vec, fn_name, option_enum_unwrap, set, Triple};
use erg_common::{ArcArray, Str};
use OpKind::*;

use erg_parser::ast::Dict as AstDict;
use erg_parser::ast::Set as AstSet;
use erg_parser::ast::*;
use erg_parser::desugar::Desugarer;
use erg_parser::token::{Token, TokenKind};

use crate::ty::constructors::{
    bounded, closed_range, dict_t, func, guard, list_t, mono, mono_q, named_free_var, poly, proj,
    proj_call, ref_, ref_mut, refinement, set_t, subr_t, subtypeof, tp_enum, try_v_enum, tuple_t,
    unknown_len_list_t, v_enum,
};
use crate::ty::free::HasLevel;
use crate::ty::typaram::{OpKind, TyParam};
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{
    ConstSubr, HasType, Predicate, SubrKind, Type, UserConstSubr, ValueArgs, Visibility,
};

use crate::context::instantiate_spec::ParamKind;
use crate::context::{ClassDefType, Context, ContextKind, RegistrationMode};
use crate::error::{EvalError, EvalErrors, EvalResult, Failable, SingleEvalResult};
use crate::varinfo::{AbsLocation, VarInfo};

use super::instantiate::TyVarCache;
use Type::{Failure, Never, Subr};

macro_rules! feature_error {
    ($ctx: expr, $loc: expr, $name: expr) => {
        $crate::feature_error!(EvalErrors, EvalError, $ctx, $loc, $name)
    };
}
macro_rules! unreachable_error {
    ($ctx: expr) => {
        $crate::unreachable_error!(EvalErrors, EvalError, $ctx)
    };
}

#[inline]
pub fn type_from_token_kind(kind: TokenKind) -> Type {
    use TokenKind::*;

    match kind {
        NatLit | BinLit | OctLit | HexLit => Type::Nat,
        IntLit => Type::Int,
        RatioLit => Type::Ratio,
        StrLit | DocComment => Type::Str,
        BoolLit => Type::Bool,
        NoneLit => Type::NoneType,
        EllipsisLit => Type::Ellipsis,
        InfLit => Type::Inf,
        other => panic!("this has no type: {other}"),
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
        OpKind::As => "__as__",
        OpKind::And => "__and__",
        OpKind::Or => "__or__",
        OpKind::Not => "__not__",
        OpKind::Invert => "__invert__",
        OpKind::BitAnd => "__bitand__",
        OpKind::BitOr => "__bitor__",
        OpKind::BitXor => "__bitxor__",
        OpKind::Shl => "__shl__",
        OpKind::Shr => "__shr__",
        OpKind::ClosedRange => "__rng__",
        OpKind::LeftOpenRange => "__lorng__",
        OpKind::RightOpenRange => "__rorng__",
        OpKind::OpenRange => "__orng__",
    }
}

#[derive(Debug, Default)]
pub struct UndoableLinkedList {
    tys: Shared<Vec<Type>>, // not Set
    tps: Shared<Vec<TyParam>>,
}

impl Drop for UndoableLinkedList {
    fn drop(&mut self) {
        for t in self.tys.borrow().iter() {
            t.undo();
        }
        for tp in self.tps.borrow().iter() {
            tp.undo();
        }
    }
}

impl UndoableLinkedList {
    pub fn new() -> Self {
        Self {
            tys: Shared::new(vec![]),
            tps: Shared::new(vec![]),
        }
    }

    pub fn push_t(&self, t: &Type) {
        self.tys.borrow_mut().push(t.clone());
    }

    pub fn push_tp(&self, tp: &TyParam) {
        self.tps.borrow_mut().push(tp.clone());
    }
}

/// Substitute concrete type/type parameters to the type containing type variables and hold until dropped.
#[derive(Debug)]
pub struct Substituter<'c> {
    ctx: &'c Context,
    undoable_linked: UndoableLinkedList,
    child: Option<Box<Substituter<'c>>>,
}

impl<'c> Substituter<'c> {
    fn new(ctx: &'c Context) -> Self {
        Self {
            ctx,
            undoable_linked: UndoableLinkedList::new(),
            child: None,
        }
    }

    /// e.g.
    /// ```erg
    /// qt: List(T, N), st: List(Int, 3)
    /// qt: T or NoneType, st: NoneType or Int (T == Int)
    /// ```
    /// invalid (no effect):
    /// ```erg
    /// qt: Iterable(T), st: List(Int, 3)
    /// qt: List(T, N), st: List!(Int, 3) # TODO
    /// ```
    pub(crate) fn substitute_typarams(
        ctx: &'c Context,
        qt: &Type,
        st: &Type,
    ) -> EvalResult<Option<Self>> {
        let qtps = qt.typarams();
        let mut stps = st.typarams();
        // Or, And are commutative, choose fitting order
        if qt.qual_name() == st.qual_name() && (st.qual_name() == "Or" || st.qual_name() == "And") {
            // REVIEW: correct condition?
            if ctx.covariant_supertype_of_tp(&qtps[0], &stps[1])
                && ctx.covariant_supertype_of_tp(&qtps[1], &stps[0])
                && qt != st
            {
                stps.swap(0, 1);
            }
        } else if qt.qual_name() != st.qual_name() || qtps.len() != stps.len() {
            // e.g. qt: Iterable(T), st: Vec(<: Iterable(Int))
            if let Some(st_sups) = ctx.get_super_types(st) {
                for sup in st_sups.skip(1) {
                    if sup.qual_name() == qt.qual_name() {
                        return Self::substitute_typarams(ctx, qt, &sup);
                    }
                }
            }
            if let Some(inner) = st.ref_inner().or_else(|| st.ref_mut_inner()) {
                return Self::substitute_typarams(ctx, qt, &inner);
            } else if let Some(sub) = st.get_sub() {
                return Self::substitute_typarams(ctx, qt, &sub);
            }
            log!(err "{qt} / {st}");
            log!(err "[{}] [{}]", erg_common::fmt_vec(&qtps), erg_common::fmt_vec(&stps));
            return Ok(None); // TODO: e.g. Sub(Int) / Eq and Sub(?T)
        }
        let mut self_ = Self::new(ctx);
        let mut errs = EvalErrors::empty();
        for (qtp, stp) in qtps.into_iter().zip(stps.into_iter()) {
            if let Err(err) = self_.substitute_typaram(qtp, stp) {
                errs.extend(err);
            }
        }
        if !errs.is_empty() {
            Err(errs)
        } else {
            Ok(Some(self_))
        }
    }

    pub(crate) fn overwrite_typarams(
        ctx: &'c Context,
        qt: &Type,
        st: &Type,
    ) -> EvalResult<Option<Self>> {
        let qtps = qt.typarams();
        let stps = st.typarams();
        if qt.qual_name() != st.qual_name() || qtps.len() != stps.len() {
            // e.g. qt: Iterable(T), st: Vec(<: Iterable(Int))
            if let Some(st_sups) = ctx.get_super_types(st) {
                for sup in st_sups.skip(1) {
                    if sup.qual_name() == qt.qual_name() {
                        return Self::overwrite_typarams(ctx, qt, &sup);
                    }
                }
            }
            if let Some(inner) = st.ref_inner().or_else(|| st.ref_mut_inner()) {
                return Self::overwrite_typarams(ctx, qt, &inner);
            } else if let Some(sub) = st.get_sub() {
                return Self::overwrite_typarams(ctx, qt, &sub);
            }
            log!(err "{qt} / {st}");
            log!(err "[{}] [{}]", erg_common::fmt_vec(&qtps), erg_common::fmt_vec(&stps));
            return Ok(None); // TODO: e.g. Sub(Int) / Eq and Sub(?T)
        }
        let mut self_ = Self::new(ctx);
        let mut errs = EvalErrors::empty();
        for (qtp, stp) in qtps.into_iter().zip(stps.into_iter()) {
            if let Err(err) = self_.overwrite_typaram(qtp, stp) {
                errs.extend(err);
            }
        }
        if !errs.is_empty() {
            Err(errs)
        } else {
            Ok(Some(self_))
        }
    }

    fn substitute_typaram(&mut self, qtp: TyParam, stp: TyParam) -> EvalResult<()> {
        match qtp {
            TyParam::FreeVar(ref fv) if fv.is_generalized() => {
                qtp.undoable_link(&stp, &self.undoable_linked);
                /*if let Err(errs) = self.sub_unify_tp(&stp, &qtp, None, &(), false) {
                    log!(err "{errs}");
                }*/
                Ok(())
            }
            TyParam::Type(qt) => self.substitute_type(*qt, stp),
            TyParam::Value(ValueObj::Type(qt)) => self.substitute_type(qt.into_typ(), stp),
            TyParam::App { name: _, args } => {
                let tps = stp.typarams();
                debug_assert_eq!(args.len(), tps.len());
                let mut errs = EvalErrors::empty();
                for (qtp, stp) in args.iter().zip(tps.into_iter()) {
                    if let Err(err) = self.substitute_typaram(qtp.clone(), stp) {
                        errs.extend(err);
                    }
                }
                if !errs.is_empty() {
                    Err(errs)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    fn substitute_type(&mut self, qt: Type, stp: TyParam) -> EvalResult<()> {
        let st = self.ctx.convert_tp_into_type(stp).map_err(|tp| {
            EvalError::not_a_type_error(
                self.ctx.cfg.input.clone(),
                line!() as usize,
                ().loc(),
                self.ctx.caused_by(),
                &tp.to_string(),
            )
        })?;
        if !qt.is_undoable_linked_var() && qt.is_generalized() && qt.is_free_var() {
            qt.undoable_link(&st, &self.undoable_linked);
        } else if qt.is_undoable_linked_var() && qt != st {
            // e.g. List(T, N) <: Add(List(T, M))
            // List((Int), (3)) <: Add(List((Int), (4))): OK
            // List((Int), (3)) <: Add(List((Str), (4))): NG
            if let Some(union) = self.ctx.unify(&qt, &st) {
                qt.undoable_link(&union, &self.undoable_linked);
            } else {
                return Err(EvalError::unification_error(
                    self.ctx.cfg.input.clone(),
                    line!() as usize,
                    &qt,
                    &st,
                    ().loc(),
                    self.ctx.caused_by(),
                )
                .into());
            }
        }
        if !st.is_unbound_var() || !st.is_generalized() {
            self.child = Self::substitute_typarams(self.ctx, &qt, &st)?.map(Box::new);
        }
        if st.has_no_unbound_var() && qt.has_no_unbound_var() {
            return Ok(());
        }
        let qt = if qt.has_undoable_linked_var() {
            let mut tv_cache = TyVarCache::new(self.ctx.level, self.ctx);
            self.ctx.detach(qt, &mut tv_cache)
        } else {
            qt
        };
        if let Err(errs) = self
            .ctx
            .undoable_sub_unify(&st, &qt, &(), &self.undoable_linked, None)
        {
            log!(err "{errs}");
        }
        Ok(())
    }

    fn overwrite_typaram(&mut self, qtp: TyParam, stp: TyParam) -> EvalResult<()> {
        match qtp {
            TyParam::FreeVar(ref fv) if fv.is_undoable_linked() => {
                qtp.undoable_link(&stp, &self.undoable_linked);
                /*if let Err(errs) = self.sub_unify_tp(&stp, &qtp, None, &(), false) {
                    log!(err "{errs}");
                }*/
                Ok(())
            }
            // NOTE: Rarely, double overwriting occurs.
            // Whether this could be a problem is under consideration.
            // e.g. `T` of List(T, N) <: Add(T, M)
            TyParam::FreeVar(ref fv) if fv.is_generalized() => {
                qtp.undoable_link(&stp, &self.undoable_linked);
                /*if let Err(errs) = self.sub_unify_tp(&stp, &qtp, None, &(), false) {
                    log!(err "{errs}");
                }*/
                Ok(())
            }
            TyParam::Type(qt) => self.overwrite_type(*qt, stp),
            TyParam::Value(ValueObj::Type(qt)) => self.overwrite_type(qt.into_typ(), stp),
            TyParam::App { name: _, args } => {
                let tps = stp.typarams();
                debug_assert_eq!(args.len(), tps.len());
                let mut errs = EvalErrors::empty();
                for (qtp, stp) in args.iter().zip(tps.into_iter()) {
                    if let Err(err) = self.overwrite_typaram(qtp.clone(), stp) {
                        errs.extend(err);
                    }
                }
                if !errs.is_empty() {
                    Err(errs)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    fn overwrite_type(&mut self, qt: Type, stp: TyParam) -> EvalResult<()> {
        let st = self.ctx.convert_tp_into_type(stp).map_err(|tp| {
            EvalError::not_a_type_error(
                self.ctx.cfg.input.clone(),
                line!() as usize,
                ().loc(),
                self.ctx.caused_by(),
                &tp.to_string(),
            )
        })?;
        if qt.is_undoable_linked_var() {
            qt.undoable_link(&st, &self.undoable_linked);
        }
        if !st.is_unbound_var() || !st.is_generalized() {
            self.child = Self::overwrite_typarams(self.ctx, &qt, &st)?.map(Box::new);
        }
        let qt = if qt.has_undoable_linked_var() {
            let mut tv_cache = TyVarCache::new(self.ctx.level, self.ctx);
            self.ctx.detach(qt, &mut tv_cache)
        } else {
            qt
        };
        if let Err(errs) = self
            .ctx
            .undoable_sub_unify(&st, &qt, &(), &self.undoable_linked, None)
        {
            log!(err "{errs}");
        }
        Ok(())
    }

    /// ```erg
    /// substitute_self(Iterable('Self), Int)
    /// -> Iterable(Int)
    /// ```
    pub(crate) fn substitute_self(qt: &Type, subtype: &Type, ctx: &'c Context) -> Option<Self> {
        for t in qt.contained_ts() {
            if t.is_qvar()
                && &t.qual_name()[..] == "Self"
                && t.get_super()
                    .is_some_and(|sup| ctx.supertype_of(&sup, subtype))
            {
                let mut _self = Self::new(ctx);
                t.undoable_link(subtype, &_self.undoable_linked);
                return Some(_self);
            }
        }
        None
    }
}

impl Context {
    fn try_get_op_kind_from_token(&self, token: &Token) -> EvalResult<OpKind> {
        match token.kind {
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
            TokenKind::PrePlus => Ok(OpKind::Pos),
            TokenKind::PreMinus => Ok(OpKind::Neg),
            TokenKind::PreBitNot => Ok(OpKind::Invert),
            TokenKind::Closed => Ok(OpKind::ClosedRange),
            TokenKind::LeftOpen => Ok(OpKind::LeftOpenRange),
            TokenKind::RightOpen => Ok(OpKind::RightOpenRange),
            TokenKind::Open => Ok(OpKind::OpenRange),
            _other => Err(EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                token.loc(),
                self.caused_by(),
            ))),
        }
    }

    fn get_mod_ctx_from_acc(&self, acc: &Accessor) -> Option<&Context> {
        match acc {
            Accessor::Ident(ident) => self.get_mod(ident.inspect()),
            Accessor::Attr(attr) => {
                let Expr::Accessor(acc) = attr.obj.as_ref() else {
                    return None;
                };
                self.get_mod_ctx_from_acc(acc)
                    .and_then(|ctx| ctx.get_mod(attr.ident.inspect()))
            }
            _ => None,
        }
    }

    fn eval_const_acc(&self, acc: &Accessor) -> Failable<ValueObj> {
        match acc {
            Accessor::Ident(ident) => self
                .eval_const_ident(ident)
                .map_err(|err| (ValueObj::Failure, err)),
            Accessor::Attr(attr) => match self.eval_const_expr(&attr.obj) {
                Ok(obj) => Ok(self
                    .eval_attr(obj, &attr.ident)
                    .map_err(|e| (ValueObj::Failure, e.into()))?),
                Err((obj, err)) => {
                    if let Ok(attr) = self.eval_attr(obj.clone(), &attr.ident) {
                        return Err((attr, err));
                    }
                    if let Expr::Accessor(acc) = attr.obj.as_ref() {
                        if let Some(mod_ctx) = self.get_mod_ctx_from_acc(acc) {
                            if let Ok(obj) = mod_ctx.eval_const_ident(&attr.ident) {
                                return Ok(obj);
                            }
                        }
                    }
                    Err((obj, err))
                }
            },
            other => feature_error!(self, other.loc(), &format!("eval {other}"))
                .map_err(|err| (ValueObj::Failure, err)),
        }
    }

    fn get_value_from_tv_cache(&self, ident: &Identifier) -> Option<ValueObj> {
        if let Some(val) = self.tv_cache.as_ref().and_then(|tv| {
            tv.get_tyvar(ident.inspect())
                .map(|t| ValueObj::builtin_type(t.clone()))
        }) {
            Some(val)
        } else if let Some(TyParam::Value(val)) = self
            .tv_cache
            .as_ref()
            .and_then(|tv| tv.get_typaram(ident.inspect()))
        {
            Some(val.clone())
        } else {
            None
        }
    }

    fn eval_const_ident(&self, ident: &Identifier) -> EvalResult<ValueObj> {
        if let Some(val) = self.get_value_from_tv_cache(ident) {
            Ok(val)
        } else if let Some(val) = self.rec_get_const_obj(ident.inspect()) {
            Ok(val.clone())
        } else if self.kind.is_subr() {
            feature_error!(self, ident.loc(), "const parameters")
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
                ident.loc(),
                self.caused_by(),
            )))
        }
    }

    fn eval_attr(&self, obj: ValueObj, ident: &Identifier) -> SingleEvalResult<ValueObj> {
        let field = self
            .instantiate_field(ident)
            .map_err(|(_, mut errs)| errs.remove(0))?;
        if let Some(val) = obj.try_get_attr(&field) {
            return Ok(val);
        }
        if let ValueObj::Type(t) = &obj {
            if let Some(sups) = self.get_nominal_super_type_ctxs(t.typ()) {
                for ctx in sups {
                    if let Some(val) = ctx.consts.get(ident.inspect()) {
                        return Ok(val.clone());
                    }
                    for methods in ctx.methods_list.iter() {
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

    fn eval_const_bin(&self, bin: &BinOp) -> Failable<ValueObj> {
        let lhs = self.eval_const_expr(&bin.args[0])?;
        let rhs = self.eval_const_expr(&bin.args[1])?;
        let op = self
            .try_get_op_kind_from_token(&bin.op)
            .map_err(|e| (ValueObj::Failure, e))?;
        self.eval_bin(op, lhs, rhs)
            .map_err(|e| (ValueObj::Failure, e))
    }

    fn eval_const_unary(&self, unary: &UnaryOp) -> Failable<ValueObj> {
        let val = self.eval_const_expr(&unary.args[0])?;
        let op = self
            .try_get_op_kind_from_token(&unary.op)
            .map_err(|e| (ValueObj::Failure, e))?;
        self.eval_unary_val(op, val)
            .map_err(|e| (ValueObj::Failure, e))
    }

    fn eval_args(&self, args: &Args) -> Failable<ValueArgs> {
        let mut errs = EvalErrors::empty();
        let mut evaluated_pos_args = vec![];
        for arg in args.pos_args().iter() {
            match self.eval_const_expr(&arg.expr) {
                Ok(val) => evaluated_pos_args.push(val),
                Err((val, es)) => {
                    evaluated_pos_args.push(val);
                    errs.extend(es);
                }
            }
        }
        let mut evaluated_kw_args = dict! {};
        for arg in args.kw_args().iter() {
            match self.eval_const_expr(&arg.expr) {
                Ok(val) => {
                    evaluated_kw_args.insert(arg.keyword.inspect().clone(), val);
                }
                Err((val, es)) => {
                    evaluated_kw_args.insert(arg.keyword.inspect().clone(), val);
                    errs.extend(es);
                }
            }
        }
        let args = ValueArgs::new(evaluated_pos_args, evaluated_kw_args);
        if errs.is_empty() {
            Ok(args)
        } else {
            Err((args, errs))
        }
    }

    fn eval_const_call(&self, call: &Call) -> Failable<ValueObj> {
        let (tp, errs) = match self.tp_eval_const_call(call) {
            Ok(tp) => (tp, EvalErrors::empty()),
            Err((tp, errs)) => (tp, errs),
        };
        match ValueObj::try_from(tp) {
            Ok(val) => {
                if errs.is_empty() {
                    Ok(val)
                } else {
                    Err((val, errs))
                }
            }
            Err(()) => {
                if errs.is_empty() {
                    Err((
                        ValueObj::Failure,
                        EvalErrors::from(EvalError::not_const_expr(
                            self.cfg.input.clone(),
                            line!() as usize,
                            call.loc(),
                            self.caused_by(),
                        )),
                    ))
                } else {
                    Err((ValueObj::Failure, errs))
                }
            }
        }
    }

    fn tp_eval_const_call(&self, call: &Call) -> Failable<TyParam> {
        if let Expr::Accessor(acc) = call.obj.as_ref() {
            match acc {
                Accessor::Ident(ident) => {
                    let obj = self.rec_get_const_obj(ident.inspect()).ok_or_else(|| {
                        (
                            TyParam::Failure,
                            EvalError::not_comptime_fn_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                ident.loc(),
                                self.caused_by(),
                                ident.inspect(),
                                self.get_similar_name(ident.inspect()),
                            )
                            .into(),
                        )
                    })?;
                    let subr = option_enum_unwrap!(obj, ValueObj::Subr)
                        .ok_or_else(|| {
                            (
                                TyParam::Failure,
                                EvalError::type_mismatch_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    ident.loc(),
                                    self.caused_by(),
                                    ident.inspect(),
                                    None,
                                    &mono("Subroutine"),
                                    &obj.t(),
                                    self.get_candidates(&obj.t()),
                                    None,
                                )
                                .into(),
                            )
                        })?
                        .clone();
                    let (args, mut errs) = match self.eval_args(&call.args) {
                        Ok(args) => (args, EvalErrors::empty()),
                        Err((args, es)) => (args, es),
                    };
                    let tp = match self.call(subr, args, call.loc()) {
                        Ok(tp) => tp,
                        Err((tp, es)) => {
                            errs.extend(es);
                            tp
                        }
                    };
                    if errs.is_empty() {
                        Ok(tp)
                    } else {
                        Err((tp, errs))
                    }
                }
                // TODO: eval attr
                Accessor::Attr(_attr) => Err((
                    TyParam::Failure,
                    EvalErrors::from(EvalError::not_const_expr(
                        self.cfg.input.clone(),
                        line!() as usize,
                        call.loc(),
                        self.caused_by(),
                    )),
                )),
                // TODO: eval type app
                Accessor::TypeApp(_type_app) => Err((
                    TyParam::Failure,
                    EvalErrors::from(EvalError::not_const_expr(
                        self.cfg.input.clone(),
                        line!() as usize,
                        call.loc(),
                        self.caused_by(),
                    )),
                )),
                other => Err((
                    TyParam::Failure,
                    EvalErrors::from(EvalError::feature_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        other.loc(),
                        &format!("const call: {other}"),
                        self.caused_by(),
                    )),
                )),
            }
        } else {
            Err((
                TyParam::Failure,
                EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    call.loc(),
                    self.caused_by(),
                )),
            ))
        }
    }

    pub(crate) fn call(
        &self,
        subr: ConstSubr,
        args: ValueArgs,
        loc: impl Locational,
    ) -> Failable<TyParam> {
        match subr {
            ConstSubr::User(user) => {
                let mut errs = EvalErrors::empty();
                // HACK: should avoid cloning
                let mut subr_ctx = Context::instant(
                    user.name.clone(),
                    self.cfg.clone(),
                    2,
                    self.shared.clone(),
                    self.clone(),
                );
                // TODO: var_args
                for (arg, sig) in args
                    .pos_args
                    .into_iter()
                    .zip(user.params.non_defaults.iter())
                {
                    let Some(symbol) = sig.inspect() else {
                        errs.push(EvalError::feature_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            loc.loc(),
                            "_",
                            self.caused_by(),
                        ));
                        continue;
                    };
                    let name = VarName::from_str(symbol.clone());
                    subr_ctx.consts.insert(name, arg);
                }
                for (name, arg) in args.kw_args.into_iter() {
                    subr_ctx.consts.insert(VarName::from_str(name), arg);
                }
                let tp = match subr_ctx.eval_const_block(&user.block()) {
                    Ok(val) => TyParam::Value(val),
                    Err((val, es)) => {
                        errs.extend(es);
                        TyParam::value(val)
                    }
                };
                if errs.is_empty() {
                    Ok(tp)
                } else {
                    Err((tp, errs))
                }
            }
            ConstSubr::Builtin(builtin) => builtin.call(args, self).map_err(|mut e| {
                if e.core.loc.is_unknown() {
                    e.core.loc = loc.loc();
                }
                (
                    TyParam::Failure,
                    EvalErrors::from(EvalError::new(
                        *e.core,
                        self.cfg.input.clone(),
                        self.caused_by(),
                    )),
                )
            }),
            ConstSubr::Gen(gen) => gen.call(args, self).map_err(|mut e| {
                if e.core.loc.is_unknown() {
                    e.core.loc = loc.loc();
                }
                (
                    TyParam::Failure,
                    EvalErrors::from(EvalError::new(
                        *e.core,
                        self.cfg.input.clone(),
                        self.caused_by(),
                    )),
                )
            }),
        }
    }

    fn eval_const_def(&mut self, def: &Def) -> Failable<ValueObj> {
        if def.is_const() {
            let mut errs = EvalErrors::empty();
            let Some(ident) = def.sig.ident() else {
                return Err((
                    ValueObj::None,
                    EvalErrors::from(EvalError::feature_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        def.sig.loc(),
                        "complex pattern const-def",
                        self.caused_by(),
                    )),
                ));
            };
            let __name__ = ident.inspect();
            let vis = self
                .instantiate_vis_modifier(def.sig.vis())
                .map_err(|es| (ValueObj::None, es))?;
            let tv_cache = match &def.sig {
                Signature::Subr(subr) => {
                    let ty_cache =
                        match self.instantiate_ty_bounds(&subr.bounds, RegistrationMode::Normal) {
                            Ok(ty_cache) => ty_cache,
                            Err((ty_cache, es)) => {
                                errs.extend(es);
                                ty_cache
                            }
                        };
                    Some(ty_cache)
                }
                Signature::Var(_) => None,
            };
            // TODO: set params
            let kind = ContextKind::from(def);
            self.grow(__name__, kind, vis, tv_cache);
            let obj = self.eval_const_block(&def.body.block).map_err(|errs| {
                self.pop();
                errs
            })?;
            let call = if let Some(Expr::Call(call)) = &def.body.block.first() {
                Some(call)
            } else {
                None
            };
            let (_ctx, es) = self.check_decls_and_pop();
            errs.extend(es);
            if let Err(es) = self.register_gen_const(
                ident,
                def.sig.params(),
                obj,
                call,
                def.def_kind().is_other(),
            ) {
                errs.extend(es);
            }
            if errs.is_empty() {
                Ok(ValueObj::None)
            } else {
                Err((ValueObj::None, errs))
            }
        } else {
            Err((
                ValueObj::None,
                EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    def.body.block.loc(),
                    self.caused_by(),
                )),
            ))
        }
    }

    pub(crate) fn eval_const_normal_list(&self, lis: &NormalList) -> Failable<ValueObj> {
        let mut errs = EvalErrors::empty();
        let mut elems = vec![];
        for elem in lis.elems.pos_args().iter() {
            match self.eval_const_expr(&elem.expr) {
                Ok(val) => elems.push(val),
                Err((val, es)) => {
                    elems.push(val);
                    errs.extend(es);
                }
            }
        }
        let list = ValueObj::List(ArcArray::from(elems));
        if errs.is_empty() {
            Ok(list)
        } else {
            Err((list, errs))
        }
    }

    fn eval_const_list(&self, lis: &List) -> Failable<ValueObj> {
        match lis {
            List::Normal(lis) => self.eval_const_normal_list(lis),
            List::WithLength(lis) => {
                let mut errs = EvalErrors::empty();
                let elem = match self.eval_const_expr(&lis.elem.expr) {
                    Ok(val) => val,
                    Err((val, es)) => {
                        errs.extend(es);
                        val
                    }
                };
                let list = match lis.len.as_ref() {
                    Expr::Accessor(Accessor::Ident(ident)) if ident.is_discarded() => {
                        ValueObj::UnsizedList(Box::new(elem))
                    }
                    other => {
                        let len = match self.eval_const_expr(other) {
                            Ok(val) => val,
                            Err((val, es)) => {
                                errs.extend(es);
                                val
                            }
                        };
                        let len = match usize::try_from(&len) {
                            Ok(len) => len,
                            Err(_) => {
                                errs.push(EvalError::type_mismatch_error(
                                    self.cfg.input.clone(),
                                    line!() as usize,
                                    other.loc(),
                                    self.caused_by(),
                                    "_",
                                    None,
                                    &Type::Nat,
                                    &len.t(),
                                    self.get_candidates(&len.t()),
                                    None,
                                ));
                                0
                            }
                        };
                        let arr = vec![elem; len];
                        ValueObj::List(ArcArray::from(arr))
                    }
                };
                if errs.is_empty() {
                    Ok(list)
                } else {
                    Err((list, errs))
                }
            }
            _ => Err((
                ValueObj::Failure,
                EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    lis.loc(),
                    self.caused_by(),
                )),
            )),
        }
    }

    fn eval_const_set(&self, set: &AstSet) -> Failable<ValueObj> {
        let mut errs = EvalErrors::empty();
        let mut elems = vec![];
        match set {
            AstSet::Normal(lis) => {
                for elem in lis.elems.pos_args().iter() {
                    match self.eval_const_expr(&elem.expr) {
                        Ok(val) => elems.push(val),
                        Err((val, es)) => {
                            elems.push(val);
                            errs.extend(es);
                        }
                    }
                }
                let set = ValueObj::Set(Set::from(elems));
                if errs.is_empty() {
                    Ok(set)
                } else {
                    Err((set, errs))
                }
            }
            _ => Err((
                ValueObj::Failure,
                EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    set.loc(),
                    self.caused_by(),
                )),
            )),
        }
    }

    fn eval_const_dict(&self, dict: &AstDict) -> Failable<ValueObj> {
        let mut errs = EvalErrors::empty();
        let mut elems = dict! {};
        match dict {
            AstDict::Normal(dic) => {
                for elem in dic.kvs.iter() {
                    match (
                        self.eval_const_expr(&elem.key),
                        self.eval_const_expr(&elem.value),
                    ) {
                        (Ok(key), Ok(value)) => {
                            elems.insert(key, value);
                        }
                        (Ok(key), Err((value, es))) => {
                            elems.insert(key, value);
                            errs.extend(es);
                        }
                        (Err((key, es)), Ok(value)) => {
                            elems.insert(key, value);
                            errs.extend(es);
                        }
                        (Err((key, es1)), Err((value, es2))) => {
                            elems.insert(key, value);
                            errs.extend(es1);
                            errs.extend(es2);
                        }
                    }
                }
                let dict = ValueObj::Dict(elems);
                if errs.is_empty() {
                    Ok(dict)
                } else {
                    Err((dict, errs))
                }
            }
            _ => Err((
                ValueObj::Failure,
                EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    dict.loc(),
                    self.caused_by(),
                )),
            )),
        }
    }

    fn eval_const_tuple(&self, tuple: &Tuple) -> Failable<ValueObj> {
        let mut errs = EvalErrors::empty();
        let mut elems = vec![];
        match tuple {
            Tuple::Normal(lis) => {
                for elem in lis.elems.pos_args().iter() {
                    let elem = match self.eval_const_expr(&elem.expr) {
                        Ok(val) => val,
                        Err((val, es)) => {
                            errs.extend(es);
                            val
                        }
                    };
                    elems.push(elem);
                }
            }
        }
        let tuple = ValueObj::Tuple(ArcArray::from(elems));
        if errs.is_empty() {
            Ok(tuple)
        } else {
            Err((tuple, errs))
        }
    }

    fn eval_const_record(&self, record: &Record) -> Failable<ValueObj> {
        match record {
            Record::Normal(rec) => self.eval_const_normal_record(rec),
            Record::Mixed(mixed) => self.eval_const_normal_record(
                &Desugarer::desugar_shortened_record_inner(mixed.clone()),
            ),
        }
    }

    fn eval_const_normal_record(&self, record: &NormalRecord) -> Failable<ValueObj> {
        let mut errs = EvalErrors::empty();
        let mut attrs = vec![];
        // HACK: should avoid cloning
        let mut record_ctx = Context::instant(
            Str::ever("<unnamed record>"),
            self.cfg.clone(),
            2,
            self.shared.clone(),
            self.clone(),
        );
        for attr in record.attrs.iter() {
            // let name = attr.sig.ident().map(|i| i.inspect());
            let elem = match record_ctx.eval_const_block(&attr.body.block) {
                Ok(val) => val,
                Err((val, es)) => {
                    errs.extend(es);
                    val
                }
            };
            let ident = match &attr.sig {
                Signature::Var(var) => match &var.pat {
                    VarPattern::Ident(ident) => match record_ctx.instantiate_field(ident) {
                        Ok(field) => field,
                        Err((field, es)) => {
                            errs.extend(es);
                            field
                        }
                    },
                    other => {
                        let err = EvalError::feature_error(
                            self.cfg.input.clone(),
                            line!() as usize,
                            other.loc(),
                            &format!("record field: {other}"),
                            self.caused_by(),
                        );
                        errs.push(err);
                        continue;
                    }
                },
                other => {
                    let err = EvalError::feature_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        other.loc(),
                        &format!("record field: {other}"),
                        self.caused_by(),
                    );
                    errs.push(err);
                    continue;
                }
            };
            let name = VarName::from_str(ident.symbol.clone());
            // T = Trait { .Output = Type; ... }
            // -> .Output = Self(<: T).Output
            if self.kind.is_trait() && self.convert_value_into_type(elem.clone()).is_ok() {
                let slf = mono_q("Self", subtypeof(mono(self.name.clone())));
                let t = ValueObj::builtin_type(slf.proj(ident.symbol.clone()));
                record_ctx.consts.insert(name.clone(), t);
            } else {
                record_ctx.consts.insert(name.clone(), elem.clone());
            }
            let t = v_enum(set! { elem.clone() });
            let vis = match record_ctx.instantiate_vis_modifier(attr.sig.vis()) {
                Ok(vis) => vis,
                Err(es) => {
                    errs.extend(es);
                    continue;
                }
            };
            let vis = Visibility::new(vis, record_ctx.name.clone());
            let vi = VarInfo::record_field(t, record_ctx.absolutize(attr.sig.loc()), vis);
            record_ctx.locals.insert(name, vi);
            attrs.push((ident, elem));
        }
        let rec = ValueObj::Record(attrs.into_iter().collect());
        if errs.is_empty() {
            Ok(rec)
        } else {
            Err((rec, errs))
        }
    }

    /// FIXME: grow
    fn eval_const_lambda(&self, lambda: &Lambda) -> Failable<ValueObj> {
        let mut errs = EvalErrors::empty();
        let mut tmp_tv_cache =
            match self.instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal) {
                Ok(ty_cache) => ty_cache,
                Err((ty_cache, es)) => {
                    errs.extend(es);
                    ty_cache
                }
            };
        let mut non_default_params = Vec::with_capacity(lambda.sig.params.non_defaults.len());
        for sig in lambda.sig.params.non_defaults.iter() {
            match self.instantiate_param_ty(
                sig,
                None,
                &mut tmp_tv_cache,
                RegistrationMode::Normal,
                ParamKind::NonDefault,
                false,
            ) {
                Ok(pt) => non_default_params.push(pt),
                Err((pt, err)) => {
                    non_default_params.push(pt);
                    errs.extend(err)
                }
            }
        }
        let var_params = if let Some(p) = lambda.sig.params.var_params.as_ref() {
            match self.instantiate_param_ty(
                p,
                None,
                &mut tmp_tv_cache,
                RegistrationMode::Normal,
                ParamKind::VarParams,
                false,
            ) {
                Ok(pt) => Some(pt),
                Err((pt, err)) => {
                    errs.extend(err);
                    Some(pt)
                }
            }
        } else {
            None
        };
        let mut default_params = Vec::with_capacity(lambda.sig.params.defaults.len());
        for sig in lambda.sig.params.defaults.iter() {
            let expr = self.eval_const_expr(&sig.default_val)?;
            match self.instantiate_param_ty(
                &sig.sig,
                None,
                &mut tmp_tv_cache,
                RegistrationMode::Normal,
                ParamKind::Default(expr.t()),
                false,
            ) {
                Ok(pt) => default_params.push(pt),
                Err((pt, err)) => {
                    errs.extend(err);
                    default_params.push(pt)
                }
            }
        }
        let kw_var_params = if let Some(p) = lambda.sig.params.kw_var_params.as_ref() {
            match self.instantiate_param_ty(
                p,
                None,
                &mut tmp_tv_cache,
                RegistrationMode::Normal,
                ParamKind::KwVarParams,
                false,
            ) {
                Ok(pt) => Some(pt),
                Err((pt, err)) => {
                    errs.extend(err);
                    Some(pt)
                }
            }
        } else {
            None
        };
        // HACK: should avoid cloning
        let mut lambda_ctx = Context::instant(
            Str::ever("<lambda>"),
            self.cfg.clone(),
            0,
            self.shared.clone(),
            self.clone(),
        );
        for non_default in non_default_params.iter() {
            let name = non_default
                .name()
                .map(|name| VarName::from_str(name.clone()));
            let vi = VarInfo::nd_parameter(
                non_default.typ().clone(),
                AbsLocation::unknown(),
                lambda_ctx.name.clone(),
            );
            lambda_ctx.params.push((name, vi));
        }
        if let Some(var_param) = var_params.as_ref() {
            let name = var_param.name().map(|name| VarName::from_str(name.clone()));
            let vi = VarInfo::nd_parameter(
                var_param.typ().clone(),
                AbsLocation::unknown(),
                lambda_ctx.name.clone(),
            );
            lambda_ctx.params.push((name, vi));
        }
        for default in default_params.iter() {
            let name = default.name().map(|name| VarName::from_str(name.clone()));
            let vi = VarInfo::d_parameter(
                default.typ().clone(),
                AbsLocation::unknown(),
                lambda_ctx.name.clone(),
            );
            lambda_ctx.params.push((name, vi));
        }
        let return_t = v_enum(set! {lambda_ctx.eval_const_block(&lambda.body)?});
        let sig_t = subr_t(
            SubrKind::from(lambda.op.kind),
            non_default_params.clone(),
            var_params,
            default_params.clone(),
            kw_var_params,
            return_t,
        );
        let block = match erg_parser::Parser::validate_const_block(lambda.body.clone()) {
            Ok(block) => block,
            Err(_) => {
                return Err((
                    ValueObj::Failure,
                    EvalErrors::from(EvalError::not_const_expr(
                        self.cfg.input.clone(),
                        line!() as usize,
                        lambda.loc(),
                        self.caused_by(),
                    )),
                ));
            }
        };
        let sig_t = self.generalize_t(sig_t);
        let subr = ConstSubr::User(UserConstSubr::new(
            Str::ever("<lambda>"),
            lambda.sig.params.clone(),
            block,
            sig_t,
        ));
        let subr = ValueObj::Subr(subr);
        if errs.is_empty() {
            Ok(subr)
        } else {
            Err((subr, errs))
        }
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

    fn tp_eval_const_expr(&self, expr: &Expr) -> Failable<TyParam> {
        let tp = self.expr_to_tp(expr.clone())?;
        self.eval_tp(tp)
    }

    pub(crate) fn eval_const_expr(&self, expr: &Expr) -> Failable<ValueObj> {
        match expr {
            Expr::Literal(lit) => self.eval_lit(lit).map_err(|e| (ValueObj::Failure, e)),
            Expr::Accessor(acc) => self.eval_const_acc(acc),
            Expr::BinOp(bin) => self.eval_const_bin(bin),
            Expr::UnaryOp(unary) => self.eval_const_unary(unary),
            Expr::Call(call) => self.eval_const_call(call),
            Expr::List(lis) => self.eval_const_list(lis),
            Expr::Set(set) => self.eval_const_set(set),
            Expr::Dict(dict) => self.eval_const_dict(dict),
            Expr::Tuple(tuple) => self.eval_const_tuple(tuple),
            Expr::Record(rec) => self.eval_const_record(rec),
            Expr::Lambda(lambda) => self.eval_const_lambda(lambda),
            // FIXME: type check
            Expr::TypeAscription(tasc) => self.eval_const_expr(&tasc.expr),
            other => Err((
                ValueObj::Failure,
                EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    other.loc(),
                    self.caused_by(),
                )),
            )),
        }
    }

    // Evaluate compile-time expression (just Expr on AST) instead of evaluating ConstExpr
    // Return Err if it cannot be evaluated at compile time
    // ConstExprを評価するのではなく、コンパイル時関数の式(AST上ではただのExpr)を評価する
    // コンパイル時評価できないならNoneを返す
    pub(crate) fn eval_const_chunk(&mut self, expr: &Expr) -> Failable<ValueObj> {
        match expr {
            // TODO: ClassDef, PatchDef
            Expr::Def(def) => self.eval_const_def(def),
            Expr::Literal(lit) => self.eval_lit(lit).map_err(|e| (ValueObj::Failure, e)),
            Expr::Accessor(acc) => self.eval_const_acc(acc),
            Expr::BinOp(bin) => self.eval_const_bin(bin),
            Expr::UnaryOp(unary) => self.eval_const_unary(unary),
            Expr::Call(call) => self.eval_const_call(call),
            Expr::List(lis) => self.eval_const_list(lis),
            Expr::Set(set) => self.eval_const_set(set),
            Expr::Dict(dict) => self.eval_const_dict(dict),
            Expr::Tuple(tuple) => self.eval_const_tuple(tuple),
            Expr::Record(rec) => self.eval_const_record(rec),
            Expr::Lambda(lambda) => self.eval_const_lambda(lambda),
            Expr::TypeAscription(tasc) => self.eval_const_expr(&tasc.expr),
            other => Err((
                ValueObj::Failure,
                EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    other.loc(),
                    self.caused_by(),
                )),
            )),
        }
    }

    fn tp_eval_const_chunk(&mut self, expr: &Expr) -> Failable<TyParam> {
        match expr {
            Expr::Def(def) => self
                .eval_const_def(def)
                .map(TyParam::Value)
                .map_err(|(_, e)| (TyParam::Failure, e)),
            other => self.tp_eval_const_expr(other),
        }
    }

    pub(crate) fn eval_const_block(&mut self, block: &Block) -> Failable<ValueObj> {
        for chunk in block.iter().rev().skip(1).rev() {
            self.eval_const_chunk(chunk)?;
        }
        self.eval_const_chunk(block.last().unwrap())
    }

    pub(crate) fn tp_eval_const_block(&mut self, block: &Block) -> Failable<TyParam> {
        for chunk in block.iter().rev().skip(1).rev() {
            self.tp_eval_const_chunk(chunk)?;
        }
        self.tp_eval_const_chunk(block.last().unwrap())
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
            Pow => lhs.try_pow(rhs).ok_or_else(|| {
                EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))
            }),
            Mod => lhs.try_mod(rhs).ok_or_else(|| {
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
            Or | BitOr => self.eval_or(lhs, rhs),
            And | BitAnd => self.eval_and(lhs, rhs),
            BitXor => match (lhs, rhs) {
                (ValueObj::Bool(l), ValueObj::Bool(r)) => Ok(ValueObj::Bool(l ^ r)),
                (ValueObj::Int(l), ValueObj::Int(r)) => Ok(ValueObj::Int(l ^ r)),
                _ => Err(EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))),
            },
            ClosedRange => Ok(ValueObj::range(lhs, rhs)),
            _other => Err(EvalErrors::from(EvalError::unreachable(
                self.cfg.input.clone(),
                fn_name!(),
                line!(),
            ))),
        }
    }

    fn eval_or(&self, lhs: ValueObj, rhs: ValueObj) -> EvalResult<ValueObj> {
        match (lhs, rhs) {
            (ValueObj::Bool(l), ValueObj::Bool(r)) => Ok(ValueObj::Bool(l || r)),
            (ValueObj::Int(l), ValueObj::Int(r)) => Ok(ValueObj::Int(l | r)),
            (ValueObj::Type(lhs), ValueObj::Type(rhs)) => Ok(self.eval_or_type(lhs, rhs)),
            (lhs, rhs) => {
                let lhs = self.convert_value_into_type(lhs).ok();
                let rhs = self.convert_value_into_type(rhs).ok();
                if let Some((l, r)) = lhs.zip(rhs) {
                    self.eval_or(ValueObj::builtin_type(l), ValueObj::builtin_type(r))
                } else {
                    Err(EvalErrors::from(EvalError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    )))
                }
            }
        }
    }

    fn eval_or_type(&self, lhs: TypeObj, rhs: TypeObj) -> ValueObj {
        match (lhs, rhs) {
            (
                TypeObj::Builtin {
                    t: l,
                    meta_t: Type::ClassType,
                },
                TypeObj::Builtin {
                    t: r,
                    meta_t: Type::ClassType,
                },
            ) => ValueObj::builtin_class(self.union(&l, &r)),
            (
                TypeObj::Builtin {
                    t: l,
                    meta_t: Type::TraitType,
                },
                TypeObj::Builtin {
                    t: r,
                    meta_t: Type::TraitType,
                },
            ) => ValueObj::builtin_trait(self.union(&l, &r)),
            (TypeObj::Builtin { t: l, meta_t: _ }, TypeObj::Builtin { t: r, meta_t: _ }) => {
                ValueObj::builtin_type(self.union(&l, &r))
            }
            (lhs, rhs) => ValueObj::gen_t(GenTypeObj::union(
                self.union(lhs.typ(), rhs.typ()),
                lhs,
                rhs,
            )),
        }
    }

    fn eval_and(&self, lhs: ValueObj, rhs: ValueObj) -> EvalResult<ValueObj> {
        match (lhs, rhs) {
            (ValueObj::Bool(l), ValueObj::Bool(r)) => Ok(ValueObj::Bool(l && r)),
            (ValueObj::Int(l), ValueObj::Int(r)) => Ok(ValueObj::Int(l & r)),
            (ValueObj::Type(lhs), ValueObj::Type(rhs)) => Ok(self.eval_and_type(lhs, rhs)),
            (lhs, rhs) => {
                let lhs = self.convert_value_into_type(lhs).ok();
                let rhs = self.convert_value_into_type(rhs).ok();
                if let Some((l, r)) = lhs.zip(rhs) {
                    self.eval_and(ValueObj::builtin_type(l), ValueObj::builtin_type(r))
                } else {
                    Err(EvalErrors::from(EvalError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    )))
                }
            }
        }
    }

    fn eval_and_type(&self, lhs: TypeObj, rhs: TypeObj) -> ValueObj {
        match (lhs, rhs) {
            (
                TypeObj::Builtin {
                    t: l,
                    meta_t: Type::ClassType,
                },
                TypeObj::Builtin {
                    t: r,
                    meta_t: Type::ClassType,
                },
            ) => ValueObj::builtin_class(self.intersection(&l, &r)),
            (
                TypeObj::Builtin {
                    t: l,
                    meta_t: Type::TraitType,
                },
                TypeObj::Builtin {
                    t: r,
                    meta_t: Type::TraitType,
                },
            ) => ValueObj::builtin_trait(self.intersection(&l, &r)),
            (TypeObj::Builtin { t: l, meta_t: _ }, TypeObj::Builtin { t: r, meta_t: _ }) => {
                ValueObj::builtin_type(self.intersection(&l, &r))
            }
            (lhs, rhs) => ValueObj::gen_t(GenTypeObj::intersection(
                self.intersection(lhs.typ(), rhs.typ()),
                lhs,
                rhs,
            )),
        }
    }

    fn eval_not_type(&self, ty: TypeObj) -> ValueObj {
        match ty {
            TypeObj::Builtin {
                t,
                meta_t: Type::ClassType,
            } => ValueObj::builtin_class(self.complement(&t)),
            TypeObj::Builtin {
                t,
                meta_t: Type::TraitType,
            } => ValueObj::builtin_trait(self.complement(&t)),
            TypeObj::Builtin { t, meta_t: _ } => ValueObj::builtin_type(self.complement(&t)),
            // FIXME:
            _ => ValueObj::Failure,
        }
    }

    pub(crate) fn eval_bin_tp(
        &self,
        op: OpKind,
        lhs: TyParam,
        rhs: TyParam,
    ) -> EvalResult<TyParam> {
        match (lhs, rhs) {
            (TyParam::Value(lhs), TyParam::Value(rhs)) => {
                self.eval_bin(op, lhs, rhs).map(TyParam::value)
            }
            (TyParam::Dict(l), TyParam::Dict(r)) if op == OpKind::Add => {
                Ok(TyParam::Dict(l.concat(r)))
            }
            (TyParam::List(l), TyParam::List(r)) if op == OpKind::Add => {
                Ok(TyParam::List([l, r].concat()))
            }
            (TyParam::FreeVar(fv), r) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.eval_bin_tp(op, t, r)
            }
            (TyParam::FreeVar(_), _) if op.is_comparison() => Ok(TyParam::value(true)),
            // _: Nat <= 10 => true
            // TODO: maybe this is wrong, we should do the type-checking of `<=`
            (TyParam::Erased(t), rhs)
                if op.is_comparison()
                    && self.supertype_of(&t, &self.get_tp_t(&rhs).unwrap_or(Type::Obj)) =>
            {
                Ok(TyParam::value(true))
            }
            (l, TyParam::FreeVar(fv)) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.eval_bin_tp(op, l, t)
            }
            (_, TyParam::FreeVar(_)) if op.is_comparison() => Ok(TyParam::value(true)),
            // 10 <= _: Nat => true
            (lhs, TyParam::Erased(t))
                if op.is_comparison()
                    && self.supertype_of(&self.get_tp_t(&lhs).unwrap_or(Type::Obj), &t) =>
            {
                Ok(TyParam::value(true))
            }
            (e @ TyParam::Erased(_), _) | (_, e @ TyParam::Erased(_)) => Ok(e),
            (lhs @ TyParam::FreeVar(_), rhs) => Ok(TyParam::bin(op, lhs, rhs)),
            (lhs, rhs @ TyParam::FreeVar(_)) => Ok(TyParam::bin(op, lhs, rhs)),
            (TyParam::Value(lhs), rhs) => {
                let lhs = match Self::convert_value_into_tp(lhs) {
                    Ok(tp) => tp,
                    Err(lhs) => {
                        return feature_error!(
                            self,
                            Location::Unknown,
                            &format!("{lhs} {op} {rhs}")
                        );
                    }
                };
                self.eval_bin_tp(op, lhs, rhs)
            }
            (lhs, TyParam::Value(rhs)) => {
                let rhs = match Self::convert_value_into_tp(rhs) {
                    Ok(tp) => tp,
                    Err(rhs) => {
                        return feature_error!(
                            self,
                            Location::Unknown,
                            &format!("{lhs} {op} {rhs}")
                        );
                    }
                };
                self.eval_bin_tp(op, lhs, rhs)
            }
            (lhs @ TyParam::Mono(_), rhs @ TyParam::Mono(_)) => Ok(TyParam::bin(op, lhs, rhs)),
            (l, r) => feature_error!(self, Location::Unknown, &format!("{l} {op} {r}"))
                .map_err(Into::into),
        }
    }

    fn eval_unary_val(&self, op: OpKind, val: ValueObj) -> EvalResult<ValueObj> {
        match op {
            Pos => Err(EvalErrors::from(EvalError::unreachable(
                self.cfg.input.clone(),
                fn_name!(),
                line!(),
            ))),
            Neg => Err(EvalErrors::from(EvalError::unreachable(
                self.cfg.input.clone(),
                fn_name!(),
                line!(),
            ))),
            Invert => Err(EvalErrors::from(EvalError::unreachable(
                self.cfg.input.clone(),
                fn_name!(),
                line!(),
            ))),
            Not => match val {
                ValueObj::Bool(b) => Ok(ValueObj::Bool(!b)),
                ValueObj::Type(lhs) => Ok(self.eval_not_type(lhs)),
                _ => Err(EvalErrors::from(EvalError::unreachable(
                    self.cfg.input.clone(),
                    fn_name!(),
                    line!(),
                ))),
            },
            _other => unreachable_error!(self),
        }
    }

    pub(crate) fn eval_unary_tp(&self, op: OpKind, val: TyParam) -> EvalResult<TyParam> {
        match val {
            TyParam::Value(c) => self.eval_unary_val(op, c).map(TyParam::Value),
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.eval_unary_tp(op, t)
            }
            e @ TyParam::Erased(_) => Ok(e),
            TyParam::FreeVar(fv) if fv.is_unbound() => {
                feature_error!(self, Location::Unknown, &format!("{op} {fv}"))
            }
            other => feature_error!(self, Location::Unknown, &format!("{op} {other}")),
        }
    }

    pub(crate) fn eval_app(&self, name: Str, args: Vec<TyParam>) -> Failable<TyParam> {
        match args
            .iter()
            .map(|tp| self.convert_tp_into_value(tp.clone()))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(value_args) => {
                if let Some(ValueObj::Subr(subr)) = self.rec_get_const_obj(&name) {
                    let args = ValueArgs::pos_only(value_args);
                    self.call(subr.clone(), args, ().loc())
                } else {
                    log!(err "eval_app({name}({}))", fmt_vec(&args));
                    let err = EvalError::no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        Location::Unknown,
                        self.caused_by(),
                        &name,
                        None,
                    );
                    Err((TyParam::app(name, args), err.into()))
                }
            }
            Err(err) => {
                log!(err "failed: eval_app({name}({}))", fmt_vec(&args));
                feature_error!(self, Location::Unknown, &format!("{err}"))
                    .map_err(|err| (TyParam::app(name, args), err))
            }
        }
    }

    /// Quantified variables, etc. are returned as is.
    /// 量化変数などはそのまま返す
    pub(crate) fn eval_tp(&self, p: TyParam) -> Failable<TyParam> {
        let mut errs = EvalErrors::empty();
        let tp = match p {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let tp = fv.crack().clone();
                match self.eval_tp(tp) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                }
            }
            TyParam::FreeVar(_) => p,
            TyParam::Mono(name) => match self
                .rec_get_const_obj(&name)
                .map(|v| TyParam::value(v.clone()))
            {
                Some(tp) => tp,
                None => {
                    errs.push(EvalError::no_var_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        Location::Unknown,
                        self.caused_by(),
                        &name,
                        None,
                    ));
                    TyParam::mono(name)
                }
            },
            TyParam::App { name, args } => match self.eval_app(name, args) {
                Ok(tp) => tp,
                Err((tp, es)) => {
                    errs.extend(es);
                    tp
                }
            },
            TyParam::BinOp { op, lhs, rhs } => match self.eval_bin_tp(op, *lhs, *rhs) {
                Ok(tp) => tp,
                Err(es) => {
                    errs.extend(es);
                    return Err((TyParam::Failure, errs));
                }
            },
            TyParam::UnaryOp { op, val } => match self.eval_unary_tp(op, *val) {
                Ok(tp) => tp,
                Err(es) => {
                    errs.extend(es);
                    return Err((TyParam::Failure, errs));
                }
            },
            TyParam::List(tps) => {
                let mut new_tps = Vec::with_capacity(tps.len());
                for tp in tps {
                    match self.eval_tp(tp) {
                        Ok(tp) => new_tps.push(tp),
                        Err((tp, es)) => {
                            new_tps.push(tp);
                            errs.extend(es);
                        }
                    }
                }
                TyParam::List(new_tps)
            }
            TyParam::UnsizedList(elem) => {
                let elem = match self.eval_tp(*elem) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                TyParam::UnsizedList(Box::new(elem))
            }
            TyParam::Tuple(tps) => {
                let mut new_tps = Vec::with_capacity(tps.len());
                for tp in tps {
                    match self.eval_tp(tp) {
                        Ok(tp) => new_tps.push(tp),
                        Err((tp, es)) => {
                            new_tps.push(tp);
                            errs.extend(es);
                        }
                    }
                }
                TyParam::Tuple(new_tps)
            }
            TyParam::Dict(dic) => {
                let mut new_dic = dict! {};
                for (k, v) in dic.into_iter() {
                    match (self.eval_tp(k), self.eval_tp(v)) {
                        (Ok(k), Ok(v)) => {
                            new_dic.insert(k, v);
                        }
                        (Ok(k), Err((v, es))) => {
                            new_dic.insert(k, v);
                            errs.extend(es);
                        }
                        (Err((k, es)), Ok(v)) => {
                            new_dic.insert(k, v);
                            errs.extend(es);
                        }
                        (Err((k, es1)), Err((v, es2))) => {
                            new_dic.insert(k, v);
                            errs.extend(es1);
                            errs.extend(es2);
                        }
                    }
                }
                TyParam::Dict(new_dic)
            }
            TyParam::Set(set) => {
                let mut new_set = set! {};
                for v in set.into_iter() {
                    match self.eval_tp(v) {
                        Ok(v) => {
                            new_set.insert(v);
                        }
                        Err((v, es)) => {
                            new_set.insert(v);
                            errs.extend(es);
                        }
                    }
                }
                TyParam::Set(new_set)
            }
            TyParam::Record(dict) => {
                let mut fields = dict! {};
                for (name, tp) in dict.into_iter() {
                    match self.eval_tp(tp) {
                        Ok(tp) => {
                            fields.insert(name, tp);
                        }
                        Err((tp, es)) => {
                            fields.insert(name, tp);
                            errs.extend(es);
                        }
                    }
                }
                TyParam::Record(fields)
            }
            TyParam::Type(t) => match self.eval_t_params(*t, self.level, &()) {
                Ok(t) => TyParam::t(t),
                Err((t, es)) => {
                    errs.extend(es);
                    TyParam::t(t)
                }
            },
            TyParam::Erased(t) => match self.eval_t_params(*t, self.level, &()) {
                Ok(t) => TyParam::erased(t),
                Err((t, es)) => {
                    errs.extend(es);
                    TyParam::erased(t)
                }
            },
            TyParam::Value(ValueObj::Type(mut t)) => {
                match t.try_map_t(|t| self.eval_t_params(t, self.level, &())) {
                    Ok(_) => {}
                    Err((_t, es)) => {
                        errs.extend(es);
                        *t.typ_mut() = _t;
                    }
                }
                TyParam::Value(ValueObj::Type(t))
            }
            TyParam::ProjCall { obj, attr, args } => {
                match self.eval_proj_call(*obj, attr, args, &()) {
                    Ok(tp) => tp,
                    Err(es) => {
                        errs.extend(es);
                        return Err((TyParam::Failure, errs));
                    }
                }
            }
            TyParam::Proj { obj, attr } => match self.eval_tp_proj(*obj, attr, &()) {
                Ok(tp) => tp,
                Err(es) => {
                    errs.extend(es);
                    return Err((TyParam::Failure, errs));
                }
            },
            TyParam::Value(_) => p.clone(),
            other => {
                errs.push(EvalError::feature_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    Location::Unknown,
                    &format!("evaluating {other}"),
                    self.caused_by(),
                ));
                other
            }
        };
        if errs.is_empty() {
            Ok(tp)
        } else {
            Err((tp, errs))
        }
    }

    fn eval_tp_into_value(&self, tp: TyParam) -> Failable<ValueObj> {
        let (tp, mut errs) = match self.eval_tp(tp) {
            Ok(tp) => (tp, EvalErrors::empty()),
            Err((tp, errs)) => (tp, errs),
        };
        let val = match self.convert_tp_into_value(tp) {
            Ok(val) => val,
            Err(err) => {
                errs.push(EvalError::feature_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    Location::Unknown,
                    &format!("convert {err} into a value"),
                    self.caused_by(),
                ));
                return Err((ValueObj::Failure, errs));
            }
        };
        if errs.is_empty() {
            Ok(val)
        } else {
            Err((val, errs))
        }
    }

    /// Evaluate `substituted`.
    /// If the evaluation fails, return a harmless type (filled with `Failure`) and errors
    pub(crate) fn eval_t_params(
        &self,
        substituted: Type,
        level: usize,
        t_loc: &impl Locational,
    ) -> Failable<Type> {
        let mut errs = EvalErrors::empty();
        match substituted {
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.eval_t_params(t, level, t_loc)
            }
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = fv.get_subsup().unwrap();
                let sub = if sub.is_recursive() {
                    sub
                } else {
                    self.eval_t_params(sub, level, t_loc)?
                };
                let sup = if sup.is_recursive() {
                    sup
                } else {
                    self.eval_t_params(sup, level, t_loc)?
                };
                let fv = Type::FreeVar(fv);
                fv.update_tyvar(sub, sup, None, false);
                Ok(fv)
            }
            Type::Subr(mut subr) => {
                for pt in subr.non_default_params.iter_mut() {
                    *pt.typ_mut() = match self.eval_t_params(mem::take(pt.typ_mut()), level, t_loc)
                    {
                        Ok(t) => t,
                        Err((_, errs)) => {
                            // `mem::take` replaces the type with `Type::Failure`, so it can return as is
                            return Err((Subr(subr), errs));
                        }
                    };
                }
                if let Some(var_args) = subr.var_params.as_mut() {
                    *var_args.typ_mut() =
                        match self.eval_t_params(mem::take(var_args.typ_mut()), level, t_loc) {
                            Ok(t) => t,
                            Err((_, errs)) => return Err((Subr(subr), errs)),
                        };
                }
                for pt in subr.default_params.iter_mut() {
                    *pt.typ_mut() = match self.eval_t_params(mem::take(pt.typ_mut()), level, t_loc)
                    {
                        Ok(t) => t,
                        Err((_, errs)) => return Err((Subr(subr), errs)),
                    };
                    if let Some(default) = pt.default_typ_mut() {
                        *default = match self.eval_t_params(mem::take(default), level, t_loc) {
                            Ok(t) => t,
                            Err((_, errs)) => return Err((Subr(subr), errs)),
                        };
                    }
                }
                if let Some(kw_var_args) = subr.kw_var_params.as_mut() {
                    *kw_var_args.typ_mut() =
                        match self.eval_t_params(mem::take(kw_var_args.typ_mut()), level, t_loc) {
                            Ok(t) => t,
                            Err((_, errs)) => return Err((Subr(subr), errs)),
                        };
                }
                match self.eval_t_params(*subr.return_t, level, t_loc) {
                    Ok(return_t) => Ok(subr_t(
                        subr.kind,
                        subr.non_default_params,
                        subr.var_params.map(|v| *v),
                        subr.default_params,
                        subr.kw_var_params.map(|v| *v),
                        return_t,
                    )),
                    Err((_, errs)) => {
                        let subr = subr_t(
                            subr.kind,
                            subr.non_default_params,
                            subr.var_params.map(|v| *v),
                            subr.default_params,
                            subr.kw_var_params.map(|v| *v),
                            Failure,
                        );
                        Err((subr, errs))
                    }
                }
            }
            Type::Refinement(refine) => {
                if refine.pred.variables().is_empty() {
                    let pred = match self.eval_pred(*refine.pred) {
                        Ok(pred) => pred,
                        Err((pred, es)) => {
                            errs.extend(es);
                            pred
                        }
                    };
                    Ok(refinement(refine.var, *refine.t, pred))
                } else {
                    Ok(Type::Refinement(refine))
                }
            }
            // [?T; 0].MutType! == [?T; !0]
            // ?T(<: Add(?R(:> Int))).Output == ?T(<: Add(?R)).Output
            // ?T(:> Int, <: Add(?R(:> Int))).Output == Int
            Type::Proj { lhs, rhs } => self
                .eval_proj(*lhs, rhs, level, t_loc)
                .map_err(|errs| (Failure, errs)),
            Type::ProjCall {
                lhs,
                attr_name,
                args,
            } => self
                .eval_proj_call_t(*lhs, attr_name, args, level, t_loc)
                .map_err(|errs| (Failure, errs)),
            Type::Ref(l) => match self.eval_t_params(*l, level, t_loc) {
                Ok(t) => Ok(ref_(t)),
                Err((_, errs)) => Err((ref_(Failure), errs)),
            },
            Type::RefMut { before, after } => {
                let before = match self.eval_t_params(*before, level, t_loc) {
                    Ok(before) => before,
                    Err((_, errs)) => {
                        return Err((ref_mut(Failure, after.map(|x| *x)), errs));
                    }
                };
                let after = if let Some(after) = after {
                    let aft = match self.eval_t_params(*after, level, t_loc) {
                        Ok(aft) => aft,
                        Err((_, errs)) => {
                            return Err((ref_mut(before, Some(Failure)), errs));
                        }
                    };
                    Some(aft)
                } else {
                    None
                };
                Ok(ref_mut(before, after))
            }
            Type::Poly { name, mut params } => {
                for p in params.iter_mut() {
                    *p = match self.eval_tp(mem::take(p)) {
                        Ok(p) => p,
                        Err((p, es)) => {
                            errs.extend(es);
                            p
                        }
                    };
                }
                if let Some(ValueObj::Subr(subr)) = self.rec_get_const_obj(&name) {
                    if let Ok(args) = self.convert_args(None, subr, params.clone(), t_loc) {
                        let ret = self.call(subr.clone(), args, t_loc);
                        if let Some(t) = ret.ok().and_then(|tp| self.convert_tp_into_type(tp).ok())
                        {
                            return Ok(t);
                        }
                    }
                }
                let t = poly(name, params);
                if errs.is_empty() {
                    Ok(t)
                } else {
                    Err((t, errs))
                }
            }
            Type::And(l, r) => {
                let l = match self.eval_t_params(*l, level, t_loc) {
                    Ok(l) => l,
                    Err((_, errs)) => {
                        return Err((Failure, errs));
                    }
                };
                let r = match self.eval_t_params(*r, level, t_loc) {
                    Ok(r) => r,
                    Err((_, errs)) => {
                        // L and Never == Never
                        return Err((Failure, errs));
                    }
                };
                Ok(self.intersection(&l, &r))
            }
            Type::Or(l, r) => {
                let l = match self.eval_t_params(*l, level, t_loc) {
                    Ok(l) => l,
                    Err((_, errs)) => {
                        return Err((Failure, errs));
                    }
                };
                let r = match self.eval_t_params(*r, level, t_loc) {
                    Ok(r) => r,
                    Err((_, errs)) => {
                        // L or Never == L
                        return Err((l, errs));
                    }
                };
                Ok(self.union(&l, &r))
            }
            Type::Not(ty) => match self.eval_t_params(*ty, level, t_loc) {
                Ok(ty) => Ok(self.complement(&ty)),
                Err((_, errs)) => Err((Failure, errs)),
            },
            Type::Structural(typ) => {
                let typ = self.eval_t_params(*typ, level, t_loc)?;
                Ok(typ.structuralize())
            }
            Type::Record(rec) => {
                let mut fields = dict! {};
                for (name, tp) in rec.into_iter() {
                    fields.insert(name, self.eval_t_params(tp, level, t_loc)?);
                }
                Ok(Type::Record(fields))
            }
            Type::NamedTuple(tuple) => {
                let mut new_tuple = vec![];
                for (name, tp) in tuple.into_iter() {
                    new_tuple.push((name, self.eval_t_params(tp, level, t_loc)?));
                }
                Ok(Type::NamedTuple(new_tuple))
            }
            Type::Bounded { sub, sup } => {
                let sub = match self.eval_t_params(*sub, level, t_loc) {
                    Ok(sub) => sub,
                    Err((_, errs)) => {
                        return Err((Failure, errs));
                    }
                };
                let sup = match self.eval_t_params(*sup, level, t_loc) {
                    Ok(sup) => sup,
                    Err((_, errs)) => {
                        return Err((Failure, errs));
                    }
                };
                Ok(bounded(sub, sup))
            }
            Type::Guard(grd) => {
                let to = self.eval_t_params(*grd.to, level, t_loc)?;
                Ok(guard(grd.namespace, grd.target, to))
            }
            other if other.is_monomorphic() => Ok(other),
            other => feature_error!(self, t_loc.loc(), &format!("eval {other}"))
                .map_err(|errs| (other, errs)),
        }
    }

    /// lhs: mainly class
    pub(crate) fn eval_proj(
        &self,
        lhs: Type,
        rhs: Str,
        level: usize,
        t_loc: &impl Locational,
    ) -> EvalResult<Type> {
        if let Never | Failure = lhs {
            return Ok(lhs);
        }
        // Currently Erg does not allow projection-types to be evaluated with type variables included.
        // All type variables will be dereferenced or fail.
        let (sub, opt_sup) = match lhs.clone() {
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                return self
                    .eval_t_params(proj(t, rhs), level, t_loc)
                    .map_err(|(_, errs)| errs);
            }
            Type::FreeVar(fv) if fv.get_subsup().is_some() => {
                let (sub, sup) = fv.get_subsup().unwrap();
                (sub, Some(sup))
            }
            other => (other, None),
        };
        // cannot determine at this point
        if sub == Type::Never {
            return Ok(proj(lhs, rhs));
        }
        // in Methods
        if let Some(ctx) = self.get_same_name_context(&sub.qual_name()) {
            match ctx.validate_and_project(&sub, opt_sup.as_ref(), &rhs, self, None, level, t_loc) {
                Triple::Ok(t) => return Ok(t),
                Triple::Err(err) => return Err(err),
                Triple::None => {}
            }
        }
        let ty_ctxs = match self.get_nominal_super_type_ctxs(&sub) {
            Some(ty_ctxs) => ty_ctxs,
            None => {
                let errs = EvalErrors::from(EvalError::type_not_found(
                    self.cfg.input.clone(),
                    line!() as usize,
                    t_loc.loc(),
                    self.caused_by(),
                    &sub,
                ));
                return Err(errs);
            }
        };
        for ty_ctx in ty_ctxs {
            match self.validate_and_project(
                &sub,
                opt_sup.as_ref(),
                &rhs,
                ty_ctx,
                Some(&ty_ctx.typ),
                level,
                t_loc,
            ) {
                Triple::Ok(t) => return Ok(t),
                Triple::Err(err) => return Err(err),
                Triple::None => {}
            }
            for methods in ty_ctx.methods_list.iter() {
                match (&methods.typ, &opt_sup) {
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
                match self.validate_and_project(
                    &sub,
                    opt_sup.as_ref(),
                    &rhs,
                    methods,
                    Some(&ty_ctx.typ),
                    level,
                    t_loc,
                ) {
                    Triple::Ok(t) => return Ok(t),
                    Triple::Err(err) => return Err(err),
                    Triple::None => {}
                }
            }
        }
        if let Some((sub, sup)) = lhs.as_free().and_then(|fv| fv.get_subsup()) {
            if self.is_trait(&sup) && !self.trait_impl_exists(&sub, &sup) {
                // link to `Never..Obj` to prevent double errors from being reported
                lhs.destructive_link(&bounded(Never, Type::Obj));
                let sub = if cfg!(feature = "debug") {
                    sub
                } else {
                    self.coerce(sub, t_loc)?
                };
                let sup = if cfg!(feature = "debug") {
                    sup
                } else {
                    self.coerce(sup, t_loc)?
                };
                return Err(EvalErrors::from(EvalError::no_trait_impl_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    &sub,
                    &sup,
                    t_loc.loc(),
                    self.caused_by(),
                    self.get_simple_type_mismatch_hint(&sup, &sub),
                )));
            }
        }
        // if the target can't be found in the supertype, the type will be dereferenced.
        // In many cases, it is still better to determine the type variable than if the target is not found.
        let coerced = self.coerce(lhs.clone(), t_loc)?;
        if lhs != coerced {
            let proj = proj(coerced, rhs);
            self.eval_t_params(proj, level, t_loc)
                .map_err(|(_, errs)| errs)
        } else {
            let proj = proj(lhs, rhs);
            let errs = EvalErrors::from(EvalError::no_candidate_error(
                self.cfg.input.clone(),
                line!() as usize,
                &proj,
                t_loc.loc(),
                self.caused_by(),
                self.get_no_candidate_hint(&proj),
            ));
            Err(errs)
        }
    }

    pub(crate) fn eval_tp_proj(
        &self,
        lhs: TyParam,
        rhs: Str,
        t_loc: &impl Locational,
    ) -> EvalResult<TyParam> {
        // in Methods
        if let Some(ctx) = lhs
            .qual_name()
            .and_then(|name| self.get_same_name_context(&name))
        {
            if let Some(value) = ctx.rec_get_const_obj(&rhs) {
                return Ok(TyParam::value(value.clone()));
            }
        }
        let ty_ctxs = match self
            .get_tp_t(&lhs)
            .ok()
            .and_then(|t| self.get_nominal_super_type_ctxs(&t))
        {
            Some(ty_ctxs) => ty_ctxs,
            None => {
                let errs = EvalErrors::from(EvalError::type_not_found(
                    self.cfg.input.clone(),
                    line!() as usize,
                    t_loc.loc(),
                    self.caused_by(),
                    &Type::Obj,
                ));
                return Err(errs);
            }
        };
        for ty_ctx in ty_ctxs {
            if let Some(value) = ty_ctx.rec_get_const_obj(&rhs) {
                return Ok(TyParam::value(value.clone()));
            }
            for methods in ty_ctx.methods_list.iter() {
                if let Some(value) = methods.rec_get_const_obj(&rhs) {
                    return Ok(TyParam::value(value.clone()));
                }
            }
        }
        Ok(lhs.proj(rhs))
    }

    /// ```erg
    /// TyParam::Type(Int) => Int
    /// [{1}, {2}, {3}] => [{1, 2, 3}; 3]
    /// (Int, Str) => Tuple([Int, Str])
    /// {x = Int; y = Int} => Type::Record({x = Int, y = Int})
    /// {Str: Int} => Dict({Str: Int})
    /// {1, 2} => {I: Int | I == 1 or I == 2 } (== {1, 2})
    /// ```
    pub(crate) fn convert_tp_into_type(&self, tp: TyParam) -> Result<Type, TyParam> {
        match tp {
            TyParam::Tuple(tps) => {
                let mut ts = vec![];
                for elem_tp in tps {
                    ts.push(self.convert_tp_into_type(elem_tp)?);
                }
                Ok(tuple_t(ts))
            }
            TyParam::List(tps) => {
                let mut union = Type::Never;
                let len = tps.len();
                for tp in tps {
                    union = self.union(&union, &self.convert_tp_into_type(tp)?);
                }
                Ok(list_t(union, TyParam::value(len)))
            }
            TyParam::UnsizedList(elem) => {
                let elem = self.convert_tp_into_type(*elem)?;
                Ok(unknown_len_list_t(elem))
            }
            TyParam::Set(tps) => {
                let mut union = Type::Never;
                for tp in tps.iter() {
                    union = self.union(&union, &self.get_tp_t(tp).unwrap_or(Type::Obj));
                }
                Ok(tp_enum(union, tps))
            }
            TyParam::Record(rec) => {
                let mut fields = dict! {};
                for (name, tp) in rec {
                    fields.insert(name, self.convert_tp_into_type(tp)?);
                }
                Ok(Type::Record(fields))
            }
            TyParam::Dict(dict) => {
                let mut kvs = dict! {};
                for (key, val) in dict {
                    kvs.insert(
                        self.convert_tp_into_type(key)?,
                        self.convert_tp_into_type(val)?,
                    );
                }
                Ok(Type::from(kvs))
            }
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.convert_tp_into_type(t)
            }
            // TyParam(Ts: List(Type)) -> Type(Ts: List(Type))
            // TyParam(?S(: Str)) -> Err(...),
            // TyParam(?D(: GenericDict)) -> Ok(?D(: GenericDict)),
            // FIXME: GenericDict
            TyParam::FreeVar(fv)
                if fv.get_type().is_some_and(|t| {
                    self.subtype_of(&t, &Type::Type) || &t.qual_name() == "GenericDict"
                }) =>
            {
                // FIXME: This procedure is clearly erroneous because it breaks the type variable linkage.
                Ok(named_free_var(
                    fv.unbound_name().unwrap(),
                    fv.level().unwrap(),
                    fv.constraint().unwrap(),
                ))
            }
            TyParam::Type(t) => Ok(t.as_ref().clone()),
            TyParam::Mono(name) => Ok(Type::Mono(name)),
            // REVIEW: should be checked?
            TyParam::App { name, args } => Ok(Type::Poly { name, params: args }),
            TyParam::Proj { obj, attr } => {
                let lhs = self.convert_tp_into_type(*obj)?;
                Ok(lhs.proj(attr))
            }
            TyParam::ProjCall { obj, attr, args } => Ok(proj_call(*obj, attr, args)),
            // TyParam::Erased(_t) => Ok(Type::Obj),
            TyParam::Value(v) => self.convert_value_into_type(v).map_err(TyParam::Value),
            TyParam::Erased(t) if t.is_type() => Ok(Type::Obj),
            // TODO: Dict, Set
            other => Err(other),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn convert_tp_into_value(&self, tp: TyParam) -> Result<ValueObj, TyParam> {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let tp = fv.crack().clone();
                self.convert_tp_into_value(tp)
            }
            TyParam::Value(v) => Ok(v),
            TyParam::List(lis) => {
                let mut new = vec![];
                for elem in lis {
                    let elem = self.convert_tp_into_value(elem)?;
                    new.push(elem);
                }
                Ok(ValueObj::List(new.into()))
            }
            TyParam::UnsizedList(elem) => {
                let elem = self.convert_tp_into_value(*elem)?;
                Ok(ValueObj::UnsizedList(Box::new(elem)))
            }
            TyParam::Tuple(tys) => {
                let mut new = vec![];
                for elem in tys {
                    let elem = self.convert_tp_into_value(elem)?;
                    new.push(elem);
                }
                Ok(ValueObj::Tuple(new.into()))
            }
            TyParam::Record(rec) => {
                let mut new = dict! {};
                for (name, elem) in rec {
                    let elem = self.convert_tp_into_value(elem)?;
                    new.insert(name, elem);
                }
                Ok(ValueObj::Record(new))
            }
            TyParam::Set(set) => {
                let mut new = set! {};
                for elem in set {
                    let elem = self.convert_tp_into_value(elem)?;
                    new.insert(elem);
                }
                Ok(ValueObj::Set(new))
            }
            TyParam::App { name, args } => {
                let mut new = vec![];
                for elem in args.iter() {
                    let elem = self.convert_tp_into_value(elem.clone())?;
                    new.push(elem);
                }
                let Some(ValueObj::Subr(subr)) = self.rec_get_const_obj(&name) else {
                    return Err(TyParam::App { name, args });
                };
                let new = ValueArgs::pos_only(new);
                match self.call(subr.clone(), new, Location::Unknown) {
                    Ok(TyParam::Value(val)) => Ok(val),
                    _ => Err(TyParam::App { name, args }),
                }
            }
            TyParam::Lambda(lambda) => {
                let name = Str::from("<lambda>");
                let params = lambda.const_.sig.params;
                let block = lambda.const_.body;
                let sig_t = func(
                    lambda.nd_params,
                    lambda.var_params,
                    lambda.d_params,
                    lambda.kw_var_params,
                    // TODO:
                    Type::Obj,
                );
                Ok(ValueObj::Subr(ConstSubr::User(UserConstSubr::new(
                    name, params, block, sig_t,
                ))))
            }
            other => self.convert_tp_into_type(other).map(ValueObj::builtin_type),
        }
    }

    pub(crate) fn convert_singular_type_into_value(&self, typ: Type) -> Result<ValueObj, Type> {
        match typ {
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.convert_singular_type_into_value(t)
            }
            Type::Refinement(ref refine) => {
                if let Predicate::Equal { rhs, .. } = refine.pred.as_ref() {
                    self.convert_tp_into_value(rhs.clone()).map_err(|_| typ)
                } else {
                    Err(typ)
                }
            }
            Type::Quantified(quant) => self.convert_singular_type_into_value(*quant),
            Type::Subr(subr) => self.convert_singular_type_into_value(*subr.return_t),
            _ => Err(typ),
        }
    }

    pub(crate) fn convert_value_into_type(&self, val: ValueObj) -> Result<Type, ValueObj> {
        match val {
            ValueObj::Failure => Ok(Type::Failure),
            ValueObj::Ellipsis => Ok(Type::Ellipsis),
            ValueObj::Type(t) => Ok(t.into_typ()),
            ValueObj::Record(rec) => {
                let mut fields = dict! {};
                for (name, val) in rec.into_iter() {
                    fields.insert(name, self.convert_value_into_type(val)?);
                }
                Ok(Type::Record(fields))
            }
            ValueObj::Tuple(ts) => {
                let mut new_ts = vec![];
                for v in ts.iter() {
                    new_ts.push(self.convert_value_into_type(v.clone())?);
                }
                Ok(tuple_t(new_ts))
            }
            ValueObj::List(lis) => {
                let len = TyParam::value(lis.len());
                let mut union = Type::Never;
                for v in lis.iter().cloned() {
                    union = self.union(&union, &self.convert_value_into_type(v)?);
                }
                Ok(list_t(union, len))
            }
            ValueObj::UnsizedList(elem) => {
                let elem = self.convert_value_into_type(*elem)?;
                Ok(unknown_len_list_t(elem))
            }
            ValueObj::Set(set) => try_v_enum(set).map_err(ValueObj::Set),
            ValueObj::Dict(dic) => {
                let dic = dic
                    .into_iter()
                    .map(|(k, v)| (TyParam::Value(k), TyParam::Value(v)))
                    .collect();
                Ok(dict_t(TyParam::Dict(dic)))
            }
            ValueObj::Subr(subr) => subr.as_type(self).ok_or(ValueObj::Subr(subr)),
            ValueObj::DataClass { name, fields } if &name == "Range" => {
                let start = fields["start"].clone();
                let end = fields["end"].clone();
                Ok(closed_range(start.class(), start, end))
            }
            other => Err(other),
        }
    }

    /// * Ok if `value` can be upcast to `TyParam`
    /// * Err if it is simply converted to `TyParam::Value(value)`
    pub(crate) fn convert_value_into_tp(value: ValueObj) -> Result<TyParam, TyParam> {
        match value {
            ValueObj::Type(t) => Ok(TyParam::t(t.into_typ())),
            ValueObj::List(lis) => {
                let mut new_lis = vec![];
                for v in lis.iter().cloned() {
                    let tp = match Self::convert_value_into_tp(v) {
                        Ok(tp) => tp,
                        Err(tp) => tp,
                    };
                    new_lis.push(tp);
                }
                Ok(TyParam::List(new_lis))
            }
            ValueObj::UnsizedList(elem) => {
                let tp = match Self::convert_value_into_tp(*elem) {
                    Ok(tp) => tp,
                    Err(tp) => tp,
                };
                Ok(TyParam::UnsizedList(Box::new(tp)))
            }
            ValueObj::Tuple(vs) => {
                let mut new_ts = vec![];
                for v in vs.iter().cloned() {
                    let tp = match Self::convert_value_into_tp(v) {
                        Ok(tp) => tp,
                        Err(tp) => tp,
                    };
                    new_ts.push(tp);
                }
                Ok(TyParam::Tuple(new_ts))
            }
            ValueObj::Dict(dict) => {
                let mut new_dict = dict! {};
                for (k, v) in dict.into_iter() {
                    let k = match Self::convert_value_into_tp(k) {
                        Ok(tp) => tp,
                        Err(tp) => tp,
                    };
                    let v = match Self::convert_value_into_tp(v) {
                        Ok(tp) => tp,
                        Err(tp) => tp,
                    };
                    new_dict.insert(k, v);
                }
                Ok(TyParam::Dict(new_dict))
            }
            ValueObj::Set(set) => {
                let mut new_set = set! {};
                for v in set.into_iter() {
                    let tp = match Self::convert_value_into_tp(v) {
                        Ok(tp) => tp,
                        Err(tp) => tp,
                    };
                    new_set.insert(tp);
                }
                Ok(TyParam::Set(new_set))
            }
            ValueObj::Record(rec) => {
                let mut new_rec = dict! {};
                for (k, v) in rec.into_iter() {
                    let v = match Self::convert_value_into_tp(v) {
                        Ok(tp) => tp,
                        Err(tp) => tp,
                    };
                    new_rec.insert(k, v);
                }
                Ok(TyParam::Record(new_rec))
            }
            ValueObj::DataClass { name, fields } => {
                let mut new_fields = dict! {};
                for (k, v) in fields.into_iter() {
                    let v = match Self::convert_value_into_tp(v) {
                        Ok(tp) => tp,
                        Err(tp) => tp,
                    };
                    new_fields.insert(k, v);
                }
                Ok(TyParam::DataClass {
                    name,
                    fields: new_fields,
                })
            }
            _ => Err(TyParam::Value(value)),
        }
    }

    pub(crate) fn convert_type_to_dict_type(&self, ty: Type) -> Result<Dict<Type, Type>, ()> {
        match ty {
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.convert_type_to_dict_type(t)
            }
            Type::Refinement(refine) => self.convert_type_to_dict_type(*refine.t),
            Type::Poly { name, params } if &name[..] == "Dict" => {
                let dict = Dict::try_from(params[0].clone())?;
                let mut new_dict = dict! {};
                for (k, v) in dict.into_iter() {
                    let k = self.convert_tp_into_type(k).map_err(|_| ())?;
                    let v = self.convert_tp_into_type(v).map_err(|_| ())?;
                    new_dict.insert(k, v);
                }
                Ok(new_dict)
            }
            _ => Err(()),
        }
    }

    pub(crate) fn convert_type_to_tuple_type(&self, ty: Type) -> Result<Vec<Type>, ()> {
        match ty {
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.convert_type_to_tuple_type(t)
            }
            Type::Refinement(refine) => self.convert_type_to_tuple_type(*refine.t),
            Type::Poly { name, params } if &name[..] == "Tuple" => {
                let tps = Vec::try_from(params[0].clone())?;
                let mut tys = vec![];
                for elem in tps.into_iter() {
                    let elem = self.convert_tp_into_type(elem).map_err(|_| ())?;
                    tys.push(elem);
                }
                Ok(tys)
            }
            _ => Err(()),
        }
    }

    pub(crate) fn convert_type_to_list(&self, ty: Type) -> Result<Vec<ValueObj>, Type> {
        match ty {
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.convert_type_to_list(t)
            }
            Type::Refinement(refine) => self.convert_type_to_list(*refine.t),
            Type::Poly { name, params } if &name[..] == "List" || &name[..] == "List!" => {
                let Ok(t) = self.convert_tp_into_type(params[0].clone()) else {
                    log!(err "cannot convert to type: {}", params[0]);
                    return Err(poly(name, params));
                };
                let Ok(len) = usize::try_from(&params[1]) else {
                    log!(err "cannot convert to usize: {}", params[1]);
                    if DEBUG_MODE {
                        panic!("cannot convert to usize: {}", params[1]);
                    }
                    return Err(poly(name, params));
                };
                Ok(vec![ValueObj::builtin_type(t); len])
            }
            _ => Err(ty),
        }
    }

    pub(crate) fn convert_value_into_list(&self, val: ValueObj) -> Result<Vec<ValueObj>, ValueObj> {
        match val {
            ValueObj::List(lis) => Ok(lis.to_vec()),
            ValueObj::Tuple(ts) => Ok(ts.to_vec()),
            ValueObj::Type(t) => self
                .convert_type_to_list(t.into_typ())
                .map_err(ValueObj::builtin_type),
            _ => Err(val),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_and_project(
        &self,
        sub: &Type,
        opt_sup: Option<&Type>,
        rhs: &str,
        methods: &Context,
        methods_type: Option<&Type>,
        level: usize,
        t_loc: &impl Locational,
    ) -> Triple<Type, EvalErrors> {
        // e.g. sub: Int, opt_sup: Add(?T), rhs: Output, methods: Int.methods
        //      sub: [Int; 4], opt_sup: Add([Int; 2]), rhs: Output, methods: [T; N].methods
        if let Ok(obj) = methods.get_const_local(&Token::symbol(rhs), &self.name) {
            #[allow(clippy::single_match)]
            // opt_sup: Add(?T), methods.impl_of(): Add(Int)
            // opt_sup: Add([Int; 2]), methods.impl_of(): Add([T; M])
            match (&opt_sup, methods.impl_of()) {
                (Some(sup), Some(trait_)) => {
                    if !self.supertype_of(&trait_, sup) {
                        return Triple::None;
                    }
                }
                _ => {}
            }
            // obj: Int|<: Add(Int)|.Output == ValueObj::Type(<type Int>)
            // obj: [T; N]|<: Add([T; M])|.Output == ValueObj::Type(<type [T; M+N]>)
            if let ValueObj::Type(quant_projected_t) = obj {
                let projected_t = quant_projected_t.into_typ();
                let quant_sub = self.get_type_ctx(&sub.qual_name()).map(|ctx| &ctx.typ);
                let _sup_subs = if let Some((sup, quant_sup)) = opt_sup.zip(methods.impl_of()) {
                    // T -> Int, M -> 2
                    match Substituter::substitute_typarams(self, &quant_sup, sup) {
                        Ok(sub_subs) => sub_subs,
                        Err(errs) => {
                            return Triple::Err(errs);
                        }
                    }
                } else {
                    None
                };
                // T -> Int, N -> 4
                /*let _sub_subs = if quant_sub.has_undoable_linked_var() {
                    Substituter::overwrite_typarams(self, quant_sub, sub).ok()?
                } else {
                    Substituter::substitute_typarams(self, quant_sub, sub).ok()?
                };*/
                let _sub_subs =
                    match quant_sub.map(|qsub| Substituter::substitute_typarams(self, qsub, sub)) {
                        Some(Ok(sub_subs)) => sub_subs,
                        Some(Err(errs)) => {
                            return Triple::Err(errs);
                        }
                        None => None,
                    };
                let _met_t_subs = match methods_type
                    .map(|met_t| Substituter::substitute_typarams(self, met_t, sub))
                {
                    Some(Ok(subs)) => subs,
                    Some(Err(errs)) => {
                        return Triple::Err(errs);
                    }
                    None => None,
                };
                // [T; M+N] -> [Int; 4+2] -> [Int; 6]
                let res = self.eval_t_params(projected_t, level, t_loc).ok();
                if let Some(t) = res {
                    let mut tv_cache = TyVarCache::new(self.level, self);
                    let t = self.detach(t, &mut tv_cache);
                    // Int -> T, 2 -> M, 4 -> N
                    return Triple::Ok(t);
                }
            } else {
                log!(err "{obj}");
                if DEBUG_MODE {
                    todo!()
                }
            }
        }
        Triple::None
    }

    /// e.g.
    /// F((Int), 3) => F(Int, 3)
    /// F(?T, ?T) => F(?1, ?1)
    pub(crate) fn detach(&self, ty: Type, tv_cache: &mut TyVarCache) -> Type {
        match ty {
            Type::FreeVar(fv) if fv.is_linked() => fv.crack().clone(), // self.detach(fv.crack().clone(), tv_cache),
            Type::FreeVar(fv) => {
                let new_fv = fv.detach();
                let name = new_fv.unbound_name().unwrap();
                if let Some(t) = tv_cache.get_tyvar(&name) {
                    t.clone()
                } else {
                    let tv = Type::FreeVar(new_fv);
                    let varname = VarName::from_str(name.clone());
                    tv_cache.dummy_push_or_init_tyvar(&varname, &tv, self);
                    tv
                }
            }
            Type::Poly { name, params } => {
                let mut new_params = vec![];
                for param in params {
                    new_params.push(self.detach_tp(param, tv_cache));
                }
                poly(name, new_params)
            }
            _ => ty,
        }
    }

    pub(crate) fn detach_tp(&self, tp: TyParam, tv_cache: &mut TyVarCache) -> TyParam {
        match tp {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let tp = fv.crack().clone();
                self.detach_tp(tp, tv_cache)
            }
            TyParam::FreeVar(fv) => {
                let new_fv = fv.detach();
                let name = new_fv.unbound_name().unwrap();
                if let Some(tp) = tv_cache.get_typaram(&name) {
                    tp.clone()
                } else {
                    let tp = TyParam::FreeVar(new_fv);
                    let varname = VarName::from_str(name.clone());
                    tv_cache.dummy_push_or_init_typaram(&varname, &tp, self);
                    tp
                }
            }
            TyParam::Type(t) => TyParam::t(self.detach(*t, tv_cache)),
            _ => tp,
        }
    }

    fn convert_args(
        &self,
        lhs: Option<TyParam>,
        subr: &ConstSubr,
        args: Vec<TyParam>,
        t_loc: &impl Locational,
    ) -> EvalResult<ValueArgs> {
        let mut pos_args = vec![];
        if subr.sig_t().is_method() {
            let Some(lhs) = lhs else {
                return feature_error!(self, t_loc.loc(), "??");
            };
            if let Ok(value) = ValueObj::try_from(lhs.clone()) {
                pos_args.push(value);
            } else if let Ok(value) = self.eval_tp_into_value(lhs.clone()) {
                pos_args.push(value);
            } else {
                return feature_error!(self, t_loc.loc(), &format!("convert {lhs} to value"));
            }
        }
        for pos_arg in args.into_iter() {
            if let Ok(value) = ValueObj::try_from(pos_arg.clone()) {
                pos_args.push(value);
            } else if let Ok(value) = self.eval_tp_into_value(pos_arg.clone()) {
                pos_args.push(value);
            } else {
                return feature_error!(self, t_loc.loc(), &format!("convert {pos_arg} to value"));
            }
        }
        Ok(ValueArgs::new(pos_args, dict! {}))
    }

    fn do_proj_call(
        &self,
        obj: ValueObj,
        lhs: TyParam,
        args: Vec<TyParam>,
        t_loc: &impl Locational,
    ) -> EvalResult<TyParam> {
        if let ValueObj::Subr(subr) = obj {
            let args = self.convert_args(Some(lhs), &subr, args, t_loc)?;
            let tp = self.call(subr, args, t_loc.loc()).map_err(|(_, e)| e)?;
            Ok(tp)
        } else {
            feature_error!(self, t_loc.loc(), "do_proj_call: ??")
        }
    }

    fn do_proj_call_t(
        &self,
        obj: ValueObj,
        lhs: TyParam,
        args: Vec<TyParam>,
        t_loc: &impl Locational,
    ) -> EvalResult<Type> {
        let tp = self.do_proj_call(obj, lhs, args, t_loc)?;
        self.convert_tp_into_type(tp).map_err(|e| {
            EvalError::feature_error(
                self.cfg.input.clone(),
                line!() as usize,
                t_loc.loc(),
                &format!("converting {e} to a type"),
                self.caused_by(),
            )
            .into()
        })
    }

    pub(crate) fn eval_proj_call_t(
        &self,
        lhs: TyParam,
        attr_name: Str,
        args: Vec<TyParam>,
        level: usize,
        t_loc: &impl Locational,
    ) -> EvalResult<Type> {
        let t = self.get_tp_t(&lhs)?;
        for ty_ctx in self.get_nominal_super_type_ctxs(&t).ok_or_else(|| {
            EvalError::type_not_found(
                self.cfg.input.clone(),
                line!() as usize,
                t_loc.loc(),
                self.caused_by(),
                &t,
            )
        })? {
            if let Ok(obj) = ty_ctx.get_const_local(&Token::symbol(&attr_name), &self.name) {
                return self.do_proj_call_t(obj, lhs, args, t_loc);
            }
            for methods in ty_ctx.methods_list.iter() {
                if let Ok(obj) = methods.get_const_local(&Token::symbol(&attr_name), &self.name) {
                    return self.do_proj_call_t(obj, lhs, args, t_loc);
                }
            }
        }
        if let TyParam::FreeVar(fv) = &lhs {
            if let Some((sub, sup)) = fv.get_subsup() {
                if self.is_trait(&sup) && !self.trait_impl_exists(&sub, &sup) {
                    // to prevent double error reporting
                    lhs.destructive_link(&TyParam::t(Never));
                    let sub = if cfg!(feature = "debug") {
                        sub
                    } else {
                        self.readable_type(sub)
                    };
                    let sup = if cfg!(feature = "debug") {
                        sup
                    } else {
                        self.readable_type(sup)
                    };
                    return Err(EvalErrors::from(EvalError::no_trait_impl_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &sub,
                        &sup,
                        t_loc.loc(),
                        self.caused_by(),
                        self.get_simple_type_mismatch_hint(&sup, &sub),
                    )));
                }
            }
        }
        // if the target can't be found in the supertype, the type will be dereferenced.
        // In many cases, it is still better to determine the type variable than if the target is not found.
        let coerced = self.coerce_tp(lhs.clone(), t_loc)?;
        if lhs != coerced {
            let proj = proj_call(coerced, attr_name, args);
            self.eval_t_params(proj, level, t_loc)
                .map(|t| {
                    lhs.destructive_coerce();
                    t
                })
                .map_err(|(_, errs)| errs)
        } else {
            let proj = proj_call(lhs, attr_name, args);
            Err(EvalErrors::from(EvalError::no_candidate_error(
                self.cfg.input.clone(),
                line!() as usize,
                &proj,
                t_loc.loc(),
                self.caused_by(),
                self.get_no_candidate_hint(&proj),
            )))
        }
    }

    pub(crate) fn eval_proj_call(
        &self,
        lhs: TyParam,
        attr_name: Str,
        args: Vec<TyParam>,
        t_loc: &impl Locational,
    ) -> EvalResult<TyParam> {
        let t = self.get_tp_t(&lhs)?;
        for ty_ctx in self.get_nominal_super_type_ctxs(&t).ok_or_else(|| {
            EvalError::type_not_found(
                self.cfg.input.clone(),
                line!() as usize,
                t_loc.loc(),
                self.caused_by(),
                &t,
            )
        })? {
            if let Ok(obj) = ty_ctx.get_const_local(&Token::symbol(&attr_name), &self.name) {
                return self.do_proj_call(obj, lhs, args, t_loc);
            }
            for methods in ty_ctx.methods_list.iter() {
                if let Ok(obj) = methods.get_const_local(&Token::symbol(&attr_name), &self.name) {
                    return self.do_proj_call(obj, lhs, args, t_loc);
                }
            }
        }
        if let TyParam::FreeVar(fv) = &lhs {
            if let Some((sub, sup)) = fv.get_subsup() {
                if self.is_trait(&sup) && !self.trait_impl_exists(&sub, &sup) {
                    // to prevent double error reporting
                    lhs.destructive_link(&TyParam::t(Never));
                    let sub = if cfg!(feature = "debug") {
                        sub
                    } else {
                        self.readable_type(sub)
                    };
                    let sup = if cfg!(feature = "debug") {
                        sup
                    } else {
                        self.readable_type(sup)
                    };
                    return Err(EvalErrors::from(EvalError::no_trait_impl_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        &sub,
                        &sup,
                        t_loc.loc(),
                        self.caused_by(),
                        self.get_simple_type_mismatch_hint(&sup, &sub),
                    )));
                }
            }
        }
        // if the target can't be found in the supertype, the type will be dereferenced.
        // In many cases, it is still better to determine the type variable than if the target is not found.
        let coerced = self.coerce_tp(lhs.clone(), t_loc)?;
        if lhs != coerced {
            self.eval_proj_call(coerced, attr_name, args, t_loc)
        } else {
            let proj = proj_call(lhs, attr_name, args);
            Err(EvalErrors::from(EvalError::no_candidate_error(
                self.cfg.input.clone(),
                line!() as usize,
                &proj,
                t_loc.loc(),
                self.caused_by(),
                self.get_no_candidate_hint(&proj),
            )))
        }
    }

    pub(crate) fn eval_call(
        &self,
        lhs: TyParam,
        args: Vec<TyParam>,
        t_loc: &impl Locational,
    ) -> EvalResult<TyParam> {
        match lhs {
            /*TyParam::Lambda(lambda) => {
                todo!("{lambda}")
            }*/
            TyParam::Value(ValueObj::Subr(subr)) => {
                let args = self.convert_args(None, &subr, args, t_loc)?;
                self.call(subr, args, t_loc.loc()).map_err(|(_, e)| e)
            }
            TyParam::Mono(name) => {
                let obj = self.rec_get_const_obj(&name).ok_or_else(|| {
                    EvalError::type_not_found(
                        self.cfg.input.clone(),
                        line!() as usize,
                        t_loc.loc(),
                        self.caused_by(),
                        &Type::Mono(name),
                    )
                })?;
                self.eval_call(TyParam::Value(obj.clone()), args, t_loc)
            }
            other => Err(EvalErrors::from(EvalError::type_mismatch_error(
                self.cfg.input.clone(),
                line!() as usize,
                t_loc.loc(),
                self.caused_by(),
                &other.qual_name().unwrap_or(Str::from("_")),
                None,
                &mono("Callable"),
                &self.get_tp_t(&other).ok().unwrap_or(Type::Obj),
                None,
                None,
            ))),
        }
    }

    pub(crate) fn bool_eval_pred(&self, p: Predicate) -> Failable<bool> {
        match self.eval_pred(p) {
            Ok(evaled) => Ok(matches!(evaled, Predicate::Value(ValueObj::Bool(true)))),
            Err((evaled, errs)) => Err((
                matches!(evaled, Predicate::Value(ValueObj::Bool(true))),
                errs,
            )),
        }
    }

    pub(crate) fn eval_pred(&self, p: Predicate) -> Failable<Predicate> {
        let mut errs = EvalErrors::empty();
        let pred = match p {
            Predicate::Value(_) | Predicate::Const(_) | Predicate::Failure => p,
            Predicate::Call {
                receiver,
                name,
                args,
            } => {
                let receiver = match self.eval_tp(receiver) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                let mut new_args = vec![];
                for arg in args {
                    match self.eval_tp(arg) {
                        Ok(tp) => new_args.push(tp),
                        Err((tp, es)) => {
                            errs.extend(es);
                            new_args.push(tp);
                        }
                    }
                }
                let res = if let Some(name) = name {
                    self.eval_proj_call(receiver, name, new_args, &())
                } else {
                    self.eval_call(receiver, new_args, &())
                };
                let tp = match res {
                    Ok(tp) => tp,
                    Err(es) => {
                        errs.extend(es);
                        return Err((Predicate::Failure, errs));
                    }
                };
                if let Ok(v) = self.convert_tp_into_value(tp) {
                    Predicate::Value(v)
                } else {
                    errs.push(EvalError::feature_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        Location::Unknown,
                        "eval_pred: Predicate::Call",
                        self.caused_by(),
                    ));
                    Predicate::Failure
                }
            }
            Predicate::Attr { receiver, name } => {
                let receiver = match self.eval_tp(receiver) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                Predicate::attr(receiver, name)
            }
            Predicate::GeneralEqual { lhs, rhs } => {
                match (self.eval_pred(*lhs)?, self.eval_pred(*rhs)?) {
                    (Predicate::Value(lhs), Predicate::Value(rhs)) => {
                        Predicate::Value(ValueObj::Bool(lhs == rhs))
                    }
                    (lhs, rhs) => Predicate::general_eq(lhs, rhs),
                }
            }
            Predicate::GeneralNotEqual { lhs, rhs } => {
                match (self.eval_pred(*lhs)?, self.eval_pred(*rhs)?) {
                    (Predicate::Value(lhs), Predicate::Value(rhs)) => {
                        Predicate::Value(ValueObj::Bool(lhs != rhs))
                    }
                    (lhs, rhs) => Predicate::general_ne(lhs, rhs),
                }
            }
            Predicate::GeneralGreaterEqual { lhs, rhs } => {
                match (self.eval_pred(*lhs)?, self.eval_pred(*rhs)?) {
                    (Predicate::Value(lhs), Predicate::Value(rhs)) => {
                        let Some(ValueObj::Bool(res)) = lhs.try_ge(rhs) else {
                            errs.push(EvalError::feature_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                Location::Unknown,
                                "evaluating >=",
                                self.caused_by(),
                            ));
                            return Err((Predicate::Failure, errs));
                        };
                        Predicate::Value(ValueObj::Bool(res))
                    }
                    (lhs, rhs) => Predicate::general_ge(lhs, rhs),
                }
            }
            Predicate::GeneralLessEqual { lhs, rhs } => {
                match (self.eval_pred(*lhs)?, self.eval_pred(*rhs)?) {
                    (Predicate::Value(lhs), Predicate::Value(rhs)) => {
                        let Some(ValueObj::Bool(res)) = lhs.try_le(rhs) else {
                            errs.push(EvalError::feature_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                Location::Unknown,
                                "evaluating <=",
                                self.caused_by(),
                            ));
                            return Err((Predicate::Failure, errs));
                        };
                        Predicate::Value(ValueObj::Bool(res))
                    }
                    (lhs, rhs) => Predicate::general_le(lhs, rhs),
                }
            }
            Predicate::Equal { lhs, rhs } => {
                let rhs = match self.eval_tp(rhs) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                Predicate::eq(lhs, rhs)
            }
            Predicate::NotEqual { lhs, rhs } => {
                let rhs = match self.eval_tp(rhs) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                Predicate::ne(lhs, rhs)
            }
            Predicate::LessEqual { lhs, rhs } => {
                let rhs = match self.eval_tp(rhs) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                Predicate::le(lhs, rhs)
            }
            Predicate::GreaterEqual { lhs, rhs } => {
                let rhs = match self.eval_tp(rhs) {
                    Ok(tp) => tp,
                    Err((tp, es)) => {
                        errs.extend(es);
                        tp
                    }
                };
                Predicate::ge(lhs, rhs)
            }
            Predicate::And(l, r) => {
                let lhs = match self.eval_pred(*l) {
                    Ok(pred) => pred,
                    Err((pred, es)) => {
                        errs.extend(es);
                        pred
                    }
                };
                let rhs = match self.eval_pred(*r) {
                    Ok(pred) => pred,
                    Err((pred, es)) => {
                        errs.extend(es);
                        pred
                    }
                };
                lhs & rhs
            }
            Predicate::Or(l, r) => {
                let lhs = match self.eval_pred(*l) {
                    Ok(pred) => pred,
                    Err((pred, es)) => {
                        errs.extend(es);
                        pred
                    }
                };
                let rhs = match self.eval_pred(*r) {
                    Ok(pred) => pred,
                    Err((pred, es)) => {
                        errs.extend(es);
                        pred
                    }
                };
                lhs | rhs
            }
            Predicate::Not(pred) => {
                let pred = match self.eval_pred(*pred) {
                    Ok(pred) => pred,
                    Err((pred, es)) => {
                        errs.extend(es);
                        pred
                    }
                };
                !pred
            }
        };
        if errs.is_empty() {
            Ok(pred)
        } else {
            Err((pred, errs))
        }
    }

    pub(crate) fn get_tp_t(&self, p: &TyParam) -> EvalResult<Type> {
        let p = self
            .eval_tp(p.clone())
            .inspect_err(|(tp, errs)| log!(err "{tp} / {errs}"))
            .unwrap_or(p.clone());
        match p {
            TyParam::Value(v) => Ok(v_enum(set![v])),
            TyParam::Erased(t) => Ok((*t).clone()),
            TyParam::FreeVar(fv) if fv.is_linked() => self.get_tp_t(&fv.crack()),
            TyParam::FreeVar(fv) => {
                if let Some(t) = fv.get_type() {
                    Ok(t)
                } else {
                    feature_error!(self, Location::Unknown, "??")
                }
            }
            TyParam::Type(typ) => Ok(self.meta_type(&typ)),
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
            TyParam::App { ref name, ref args } => self
                .rec_get_const_obj(name)
                .and_then(|v| {
                    let ty = match self.convert_value_into_type(v.clone()) {
                        Ok(ty) => ty,
                        Err(ValueObj::Subr(subr)) => {
                            // REVIEW: evaluation of polymorphic types
                            return subr.sig_t().return_t().cloned();
                        }
                        Err(_) => {
                            return None;
                        }
                    };
                    let instance = self
                        .instantiate_def_type(&ty)
                        .map_err(|err| {
                            log!(err "{err}");
                            err
                        })
                        .ok()?;
                    for (param, arg) in instance.typarams().iter().zip(args.iter()) {
                        self.sub_unify_tp(arg, param, None, &(), false).ok()?;
                    }
                    let ty_obj = if self.is_class(&instance) {
                        ValueObj::builtin_class(instance)
                    } else if self.is_trait(&instance) {
                        ValueObj::builtin_trait(instance)
                    } else {
                        ValueObj::builtin_type(instance)
                    };
                    Some(v_enum(set![ty_obj]))
                })
                .or_else(|| {
                    let namespace = p.namespace();
                    if let Some(namespace) = self.get_namespace(&namespace) {
                        if namespace.name != self.name {
                            if let Some(typ) = p.local_name().and_then(|name| {
                                namespace.get_tp_t(&TyParam::app(name, args.clone())).ok()
                            }) {
                                return Some(typ);
                            }
                        }
                    }
                    None
                })
                .ok_or_else(|| {
                    EvalErrors::from(EvalError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    ))
                }),
            TyParam::List(tps) => {
                let tp_t = if let Some(fst) = tps.first() {
                    self.get_tp_t(fst)?
                } else {
                    Never
                };
                let t = list_t(tp_t, TyParam::value(tps.len()));
                Ok(t)
            }
            TyParam::Tuple(tps) => {
                let mut tps_t = vec![];
                for tp in tps {
                    tps_t.push(self.get_tp_t(&tp)?);
                }
                Ok(tuple_t(tps_t))
            }
            TyParam::Set(tps) => {
                let len = TyParam::value(tps.len());
                let mut union = Type::Never;
                for tp in tps {
                    let tp_t = self.get_tp_t(&tp)?;
                    union = self.union(&union, &tp_t);
                }
                Ok(set_t(union, len))
            }
            TyParam::Record(dict) => {
                let mut fields = dict! {};
                for (name, tp) in dict.into_iter() {
                    let tp_t = self.get_tp_t(&tp)?;
                    fields.insert(name, tp_t);
                }
                Ok(Type::Record(fields))
            }
            dict @ TyParam::Dict(_) => Ok(dict_t(dict)),
            TyParam::BinOp { op, lhs, rhs } => match op {
                OpKind::Or | OpKind::And => {
                    let lhs = self.get_tp_t(&lhs)?;
                    let rhs = self.get_tp_t(&rhs)?;
                    if self.subtype_of(&lhs, &Type::Bool) && self.subtype_of(&rhs, &Type::Bool) {
                        Ok(Type::Bool)
                    } else if self.subtype_of(&lhs, &Type::Type)
                        && self.subtype_of(&rhs, &Type::Type)
                    {
                        Ok(Type::Type)
                    } else {
                        let op_name = op_to_name(op);
                        feature_error!(
                            self,
                            Location::Unknown,
                            &format!("get type: {op_name}({lhs}, {rhs})")
                        )
                    }
                }
                _ => {
                    let op_name = op_to_name(op);
                    feature_error!(
                        self,
                        Location::Unknown,
                        &format!("get type: {op_name}({lhs}, {rhs})")
                    )
                }
            },
            TyParam::ProjCall { obj, attr, args } => {
                let Ok(tp) = self.eval_proj_call(*obj.clone(), attr.clone(), args, &()) else {
                    let Some(obj_ctx) = self.get_nominal_type_ctx(&self.get_tp_t(&obj)?) else {
                        return Ok(Type::Obj);
                    };
                    let value = obj_ctx.get_const_local(&Token::symbol(&attr), &self.name)?;
                    match value {
                        ValueObj::Subr(subr) => {
                            if let Some(ret_t) = subr.sig_t().return_t() {
                                return Ok(ret_t.clone());
                            } else {
                                return Err(EvalErrors::from(EvalError::unreachable(
                                    self.cfg.input.clone(),
                                    fn_name!(),
                                    line!(),
                                )));
                            }
                        }
                        _ => {
                            return Ok(Type::Obj);
                        }
                    }
                };
                let ty = self.get_tp_t(&tp).unwrap_or(Type::Obj).derefine();
                Ok(tp_enum(ty, set![tp]))
            }
            other => feature_error!(
                self,
                Location::Unknown,
                &format!("getting the type of {other}")
            ),
        }
    }

    pub(crate) fn _get_tp_class(&self, p: &TyParam) -> EvalResult<Type> {
        let p = match self.eval_tp(p.clone()) {
            Ok(tp) => tp,
            // TODO: handle errs
            Err((tp, errs)) => {
                log!(err "{tp} / {errs}");
                tp
            }
        };
        match p {
            TyParam::Value(v) => Ok(v.class()),
            TyParam::Erased(t) => Ok((*t).clone()),
            TyParam::FreeVar(fv) => {
                if let Some(t) = fv.get_type() {
                    Ok(t)
                } else {
                    feature_error!(self, Location::Unknown, "??")
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
            other => feature_error!(
                self,
                Location::Unknown,
                &format!("getting the class of {other}")
            ),
        }
    }

    /// NOTE: If l and r are types, the Context is used to determine the type.
    /// NOTE: lとrが型の場合はContextの方で判定する
    pub(crate) fn shallow_eq_tp(&self, lhs: &TyParam, rhs: &TyParam) -> bool {
        match (lhs, rhs) {
            (TyParam::Type(l), _) if l.is_unbound_var() => {
                let Ok(rhs) = self.get_tp_t(rhs) else {
                    log!(err "rhs: {rhs}");
                    return false;
                };
                self.subtype_of(&rhs, &Type::Type)
            }
            (_, TyParam::Type(r)) if r.is_unbound_var() => {
                let Ok(lhs) = self.get_tp_t(lhs) else {
                    log!(err "lhs: {lhs}");
                    return false;
                };
                self.subtype_of(&lhs, &Type::Type)
            }
            (TyParam::Type(l), TyParam::Type(r)) => l == r,
            (TyParam::Value(l), TyParam::Value(r)) => l == r,
            (TyParam::Erased(l), TyParam::Erased(r)) => l == r,
            (TyParam::List(l), TyParam::List(r)) => l == r,
            (TyParam::Tuple(l), TyParam::Tuple(r)) => l == r,
            (TyParam::Set(l), TyParam::Set(r)) => l == r, // FIXME:
            (TyParam::Dict(l), TyParam::Dict(r)) => l == r,
            (TyParam::Lambda(l), TyParam::Lambda(r)) => l == r,
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
            (TyParam::Mono(m), TyParam::Value(l)) | (TyParam::Value(l), TyParam::Mono(m)) => {
                if let Some(o) = self.rec_get_const_obj(m) {
                    o == l
                } else {
                    true
                }
            }
            (TyParam::Erased(t), _) => Some(t.as_ref()) == self.get_tp_t(rhs).ok().as_ref(),
            (_, TyParam::Erased(t)) => Some(t.as_ref()) == self.get_tp_t(lhs).ok().as_ref(),
            (TyParam::Value(v), _) => {
                if let Ok(tp) = Self::convert_value_into_tp(v.clone()) {
                    self.shallow_eq_tp(&tp, rhs)
                } else {
                    false
                }
            }
            (_, TyParam::Value(v)) => {
                if let Ok(tp) = Self::convert_value_into_tp(v.clone()) {
                    self.shallow_eq_tp(lhs, &tp)
                } else {
                    false
                }
            }
            (l, r) => {
                log!(err "l: {l}, r: {r}");
                l == r
            }
        }
    }
}
