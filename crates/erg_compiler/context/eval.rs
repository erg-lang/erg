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
    array_t, bounded, dict_t, mono, mono_q, named_free_var, poly, proj, proj_call, ref_, ref_mut,
    refinement, set_t, subr_t, subtypeof, tp_enum, tuple_t, unknown_len_array_t, v_enum,
};
use crate::ty::free::{Constraint, HasLevel};
use crate::ty::typaram::{OpKind, TyParam};
use crate::ty::value::{GenTypeObj, TypeObj, ValueObj};
use crate::ty::{
    ConstSubr, HasType, Predicate, SubrKind, Type, UserConstSubr, ValueArgs, Visibility,
};

use crate::context::instantiate_spec::ParamKind;
use crate::context::{ClassDefType, Context, ContextKind, RegistrationMode};
use crate::error::{EvalError, EvalErrors, EvalResult, SingleEvalResult};
use crate::varinfo::VarInfo;

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
    /// qt: Array(T, N), st: Array(Int, 3)
    /// ```
    /// invalid (no effect):
    /// ```erg
    /// qt: Iterable(T), st: Array(Int, 3)
    /// qt: Array(T, N), st: Array!(Int, 3) # TODO
    /// ```
    pub(crate) fn substitute_typarams(
        ctx: &'c Context,
        qt: &Type,
        st: &Type,
    ) -> EvalResult<Option<Self>> {
        let qtps = qt.typarams();
        let stps = st.typarams();
        if qt.qual_name() != st.qual_name() || qtps.len() != stps.len() {
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
            if let Some(sub) = st.get_sub() {
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
            // e.g. Array(T, N) <: Add(Array(T, M))
            // Array((Int), (3)) <: Add(Array((Int), (4))): OK
            // Array((Int), (3)) <: Add(Array((Str), (4))): NG
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
            // e.g. `T` of Array(T, N) <: Add(T, M)
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

    fn eval_const_acc(&self, acc: &Accessor) -> EvalResult<ValueObj> {
        match acc {
            Accessor::Ident(ident) => self.eval_const_ident(ident),
            Accessor::Attr(attr) => match self.eval_const_expr(&attr.obj) {
                Ok(obj) => Ok(self.eval_attr(obj, &attr.ident)?),
                Err(err) => {
                    if let Expr::Accessor(acc) = attr.obj.as_ref() {
                        if let Some(mod_ctx) = self.get_mod_ctx_from_acc(acc) {
                            if let Ok(obj) = mod_ctx.eval_const_ident(&attr.ident) {
                                return Ok(obj);
                            }
                        }
                    }
                    Err(err)
                }
            },
            other => {
                feature_error!(self, other.loc(), &format!("eval {other}")).map_err(Into::into)
            }
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
        if let Some(val) = self.rec_get_const_obj(ident.inspect()) {
            Ok(val.clone())
        } else if let Some(val) = self.get_value_from_tv_cache(ident) {
            Ok(val)
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
            .map_err(|mut errs| errs.remove(0))?;
        if let Some(val) = obj.try_get_attr(&field) {
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
        let op = self.try_get_op_kind_from_token(&bin.op)?;
        self.eval_bin(op, lhs, rhs)
    }

    fn eval_const_unary(&self, unary: &UnaryOp) -> EvalResult<ValueObj> {
        let val = self.eval_const_expr(&unary.args[0])?;
        let op = self.try_get_op_kind_from_token(&unary.op)?;
        self.eval_unary_val(op, val)
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
        let tp = self.tp_eval_const_call(call)?;
        ValueObj::try_from(tp).map_err(|_| {
            EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                call.loc(),
                self.caused_by(),
            ))
        })
    }

    fn tp_eval_const_call(&self, call: &Call) -> EvalResult<TyParam> {
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
                                None,
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
                // TODO: eval attr
                Accessor::Attr(_attr) => Err(EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    call.loc(),
                    self.caused_by(),
                ))),
                // TODO: eval type app
                Accessor::TypeApp(_type_app) => Err(EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    call.loc(),
                    self.caused_by(),
                ))),
                other => Err(EvalErrors::from(EvalError::feature_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    other.loc(),
                    &format!("const call: {other}"),
                    self.caused_by(),
                ))),
            }
        } else {
            Err(EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                call.loc(),
                self.caused_by(),
            )))
        }
    }

    fn call(&self, subr: ConstSubr, args: ValueArgs, loc: Location) -> EvalResult<TyParam> {
        match subr {
            ConstSubr::User(user) => {
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
                    let name = VarName::from_str(sig.inspect().unwrap().clone());
                    subr_ctx.consts.insert(name, arg);
                }
                for (name, arg) in args.kw_args.into_iter() {
                    subr_ctx.consts.insert(VarName::from_str(name), arg);
                }
                subr_ctx.eval_const_block(&user.block()).map(TyParam::value)
            }
            ConstSubr::Builtin(builtin) => builtin.call(args, self).map_err(|mut e| {
                if e.0.loc.is_unknown() {
                    e.0.loc = loc;
                }
                EvalErrors::from(EvalError::new(
                    *e.0,
                    self.cfg.input.clone(),
                    self.caused_by(),
                ))
            }),
            ConstSubr::Gen(gen) => gen.call(args, self).map_err(|mut e| {
                if e.0.loc.is_unknown() {
                    e.0.loc = loc;
                }
                EvalErrors::from(EvalError::new(
                    *e.0,
                    self.cfg.input.clone(),
                    self.caused_by(),
                ))
            }),
        }
    }

    fn eval_const_def(&mut self, def: &Def) -> EvalResult<ValueObj> {
        if def.is_const() {
            let __name__ = def.sig.ident().unwrap().inspect();
            let vis = self.instantiate_vis_modifier(def.sig.vis())?;
            let tv_cache = match &def.sig {
                Signature::Subr(subr) => {
                    let ty_cache =
                        self.instantiate_ty_bounds(&subr.bounds, RegistrationMode::Normal)?;
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
            let (_ctx, errs) = self.check_decls_and_pop();
            self.register_gen_const(def.sig.ident().unwrap(), obj, def.def_kind().is_other())?;
            if errs.is_empty() {
                Ok(ValueObj::None)
            } else {
                Err(errs)
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

    pub(crate) fn eval_const_normal_array(&self, arr: &NormalArray) -> EvalResult<ValueObj> {
        let mut elems = vec![];
        for elem in arr.elems.pos_args().iter() {
            let elem = self.eval_const_expr(&elem.expr)?;
            elems.push(elem);
        }
        Ok(ValueObj::Array(ArcArray::from(elems)))
    }

    fn eval_const_array(&self, arr: &Array) -> EvalResult<ValueObj> {
        match arr {
            Array::Normal(arr) => self.eval_const_normal_array(arr),
            Array::WithLength(arr) => {
                let elem = self.eval_const_expr(&arr.elem.expr)?;
                match arr.len.as_ref() {
                    Expr::Accessor(Accessor::Ident(ident)) if ident.is_discarded() => {
                        Ok(ValueObj::UnsizedArray(Box::new(elem)))
                    }
                    other => {
                        let len = self.eval_const_expr(other)?;
                        let len = usize::try_from(&len).map_err(|_| {
                            EvalError::type_mismatch_error(
                                self.cfg.input.clone(),
                                line!() as usize,
                                other.loc(),
                                self.caused_by(),
                                "_",
                                None,
                                &Type::Nat,
                                &len.t(),
                                None,
                                None,
                            )
                        })?;
                        let arr = vec![elem; len];
                        Ok(ValueObj::Array(ArcArray::from(arr)))
                    }
                }
            }
            _ => Err(EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                arr.loc(),
                self.caused_by(),
            ))),
        }
    }

    fn eval_const_set(&self, set: &AstSet) -> EvalResult<ValueObj> {
        let mut elems = vec![];
        match set {
            AstSet::Normal(arr) => {
                for elem in arr.elems.pos_args().iter() {
                    let elem = self.eval_const_expr(&elem.expr)?;
                    elems.push(elem);
                }
                Ok(ValueObj::Set(Set::from(elems)))
            }
            _ => Err(EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                set.loc(),
                self.caused_by(),
            ))),
        }
    }

    fn eval_const_dict(&self, dict: &AstDict) -> EvalResult<ValueObj> {
        let mut elems = dict! {};
        match dict {
            AstDict::Normal(dic) => {
                for elem in dic.kvs.iter() {
                    let key = self.eval_const_expr(&elem.key)?;
                    let value = self.eval_const_expr(&elem.value)?;
                    elems.insert(key, value);
                }
                Ok(ValueObj::Dict(elems))
            }
            _ => Err(EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                dict.loc(),
                self.caused_by(),
            ))),
        }
    }

    fn eval_const_tuple(&self, tuple: &Tuple) -> EvalResult<ValueObj> {
        let mut elems = vec![];
        match tuple {
            Tuple::Normal(arr) => {
                for elem in arr.elems.pos_args().iter() {
                    let elem = self.eval_const_expr(&elem.expr)?;
                    elems.push(elem);
                }
            }
        }
        Ok(ValueObj::Tuple(ArcArray::from(elems)))
    }

    fn eval_const_record(&self, record: &Record) -> EvalResult<ValueObj> {
        match record {
            Record::Normal(rec) => self.eval_const_normal_record(rec),
            Record::Mixed(mixed) => self.eval_const_normal_record(
                &Desugarer::desugar_shortened_record_inner(mixed.clone()),
            ),
        }
    }

    fn eval_const_normal_record(&self, record: &NormalRecord) -> EvalResult<ValueObj> {
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
            let elem = record_ctx.eval_const_block(&attr.body.block)?;
            let ident = match &attr.sig {
                Signature::Var(var) => match &var.pat {
                    VarPattern::Ident(ident) => record_ctx.instantiate_field(ident)?,
                    other => {
                        return feature_error!(self, other.loc(), &format!("record field: {other}"))
                    }
                },
                other => {
                    return feature_error!(self, other.loc(), &format!("record field: {other}"))
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
            let vis = record_ctx.instantiate_vis_modifier(attr.sig.vis())?;
            let vis = Visibility::new(vis, record_ctx.name.clone());
            let vi = VarInfo::record_field(t, record_ctx.absolutize(attr.sig.loc()), vis);
            record_ctx.locals.insert(name, vi);
            attrs.push((ident, elem));
        }
        Ok(ValueObj::Record(attrs.into_iter().collect()))
    }

    /// FIXME: grow
    fn eval_const_lambda(&self, lambda: &Lambda) -> EvalResult<ValueObj> {
        let mut tmp_tv_cache =
            self.instantiate_ty_bounds(&lambda.sig.bounds, RegistrationMode::Normal)?;
        let mut non_default_params = Vec::with_capacity(lambda.sig.params.non_defaults.len());
        let mut errs = EvalErrors::empty();
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
        // HACK: should avoid cloning
        let mut lambda_ctx = Context::instant(
            Str::ever("<lambda>"),
            self.cfg.clone(),
            0,
            self.shared.clone(),
            self.clone(),
        );
        let return_t = v_enum(set! {lambda_ctx.eval_const_block(&lambda.body)?});
        let sig_t = subr_t(
            SubrKind::from(lambda.op.kind),
            non_default_params.clone(),
            var_params,
            default_params.clone(),
            return_t,
        );
        let block =
            erg_parser::Parser::validate_const_block(lambda.body.clone()).map_err(|_| {
                EvalErrors::from(EvalError::not_const_expr(
                    self.cfg.input.clone(),
                    line!() as usize,
                    lambda.loc(),
                    self.caused_by(),
                ))
            })?;
        let sig_t = self.generalize_t(sig_t);
        let subr = ConstSubr::User(UserConstSubr::new(
            Str::ever("<lambda>"),
            lambda.sig.params.clone(),
            block,
            sig_t,
        ));
        if errs.is_empty() {
            Ok(ValueObj::Subr(subr))
        } else {
            Err(errs)
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

    pub(crate) fn eval_const_expr(&self, expr: &Expr) -> EvalResult<ValueObj> {
        match expr {
            Expr::Literal(lit) => self.eval_lit(lit),
            Expr::Accessor(acc) => self.eval_const_acc(acc),
            Expr::BinOp(bin) => self.eval_const_bin(bin),
            Expr::UnaryOp(unary) => self.eval_const_unary(unary),
            Expr::Call(call) => self.eval_const_call(call),
            Expr::Array(arr) => self.eval_const_array(arr),
            Expr::Set(set) => self.eval_const_set(set),
            Expr::Dict(dict) => self.eval_const_dict(dict),
            Expr::Tuple(tuple) => self.eval_const_tuple(tuple),
            Expr::Record(rec) => self.eval_const_record(rec),
            Expr::Lambda(lambda) => self.eval_const_lambda(lambda),
            // FIXME: type check
            Expr::TypeAscription(tasc) => self.eval_const_expr(&tasc.expr),
            other => Err(EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                other.loc(),
                self.caused_by(),
            ))),
        }
    }

    // Evaluate compile-time expression (just Expr on AST) instead of evaluating ConstExpr
    // Return Err if it cannot be evaluated at compile time
    // ConstExprを評価するのではなく、コンパイル時関数の式(AST上ではただのExpr)を評価する
    // コンパイル時評価できないならNoneを返す
    pub(crate) fn eval_const_chunk(&mut self, expr: &Expr) -> EvalResult<ValueObj> {
        match expr {
            // TODO: ClassDef, PatchDef
            Expr::Def(def) => self.eval_const_def(def),
            Expr::Literal(lit) => self.eval_lit(lit),
            Expr::Accessor(acc) => self.eval_const_acc(acc),
            Expr::BinOp(bin) => self.eval_const_bin(bin),
            Expr::UnaryOp(unary) => self.eval_const_unary(unary),
            Expr::Call(call) => self.eval_const_call(call),
            Expr::Array(arr) => self.eval_const_array(arr),
            Expr::Set(set) => self.eval_const_set(set),
            Expr::Dict(dict) => self.eval_const_dict(dict),
            Expr::Tuple(tuple) => self.eval_const_tuple(tuple),
            Expr::Record(rec) => self.eval_const_record(rec),
            Expr::Lambda(lambda) => self.eval_const_lambda(lambda),
            Expr::TypeAscription(tasc) => self.eval_const_expr(&tasc.expr),
            other => Err(EvalErrors::from(EvalError::not_const_expr(
                self.cfg.input.clone(),
                line!() as usize,
                other.loc(),
                self.caused_by(),
            ))),
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
            _ => ValueObj::Illegal,
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
            (TyParam::Array(l), TyParam::Array(r)) if op == OpKind::Add => {
                Ok(TyParam::Array([l, r].concat()))
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

    fn eval_succ_func(&self, val: ValueObj) -> EvalResult<ValueObj> {
        match val {
            ValueObj::Bool(b) => Ok(ValueObj::Nat(b as u64 + 1)),
            ValueObj::Nat(n) => Ok(ValueObj::Nat(n + 1)),
            ValueObj::Int(n) => Ok(ValueObj::Int(n + 1)),
            // TODO:
            ValueObj::Float(n) => Ok(ValueObj::Float(n + f64::EPSILON)),
            ValueObj::Inf | ValueObj::NegInf => Ok(val),
            _ => Err(EvalErrors::from(EvalError::unreachable(
                self.cfg.input.clone(),
                fn_name!(),
                line!(),
            ))),
        }
    }

    fn eval_pred_func(&self, val: ValueObj) -> EvalResult<ValueObj> {
        match val {
            ValueObj::Bool(_) => Ok(ValueObj::Nat(0)),
            ValueObj::Nat(n) => Ok(ValueObj::Nat(n.saturating_sub(1))),
            ValueObj::Int(n) => Ok(ValueObj::Int(n - 1)),
            // TODO:
            ValueObj::Float(n) => Ok(ValueObj::Float(n - f64::EPSILON)),
            ValueObj::Inf | ValueObj::NegInf => Ok(val),
            _ => Err(EvalErrors::from(EvalError::unreachable(
                self.cfg.input.clone(),
                fn_name!(),
                line!(),
            ))),
        }
    }

    pub(crate) fn eval_app(&self, name: Str, args: Vec<TyParam>) -> EvalResult<TyParam> {
        if let Ok(mut value_args) = args
            .iter()
            .map(|tp| self.convert_tp_into_value(tp.clone()))
            .collect::<Result<Vec<_>, _>>()
        {
            match &name[..] {
                "succ" => self
                    .eval_succ_func(value_args.remove(0))
                    .map(TyParam::Value),
                "pred" => self
                    .eval_pred_func(value_args.remove(0))
                    .map(TyParam::Value),
                _ => {
                    log!(err "eval_app({name}({}))", fmt_vec(&args));
                    Ok(TyParam::app(name, args))
                }
            }
        } else {
            log!(err "eval_app({name}({}))", fmt_vec(&args));
            Ok(TyParam::app(name, args))
        }
    }

    /// Quantified variables, etc. are returned as is.
    /// 量化変数などはそのまま返す
    pub(crate) fn eval_tp(&self, p: TyParam) -> EvalResult<TyParam> {
        match p {
            TyParam::FreeVar(fv) if fv.is_linked() => {
                let tp = fv.crack().clone();
                self.eval_tp(tp)
            }
            TyParam::FreeVar(_) => Ok(p),
            TyParam::Mono(name) => self
                .rec_get_const_obj(&name)
                .map(|v| TyParam::value(v.clone()))
                .ok_or_else(|| {
                    EvalErrors::from(EvalError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    ))
                }),
            TyParam::App { name, args } => self.eval_app(name, args),
            TyParam::BinOp { op, lhs, rhs } => self.eval_bin_tp(op, *lhs, *rhs),
            TyParam::UnaryOp { op, val } => self.eval_unary_tp(op, *val),
            TyParam::Array(tps) => {
                let mut new_tps = Vec::with_capacity(tps.len());
                for tp in tps {
                    new_tps.push(self.eval_tp(tp)?);
                }
                Ok(TyParam::Array(new_tps))
            }
            TyParam::UnsizedArray(elem) => {
                let elem = self.eval_tp(*elem)?;
                Ok(TyParam::UnsizedArray(Box::new(elem)))
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
                for (k, v) in dic.into_iter() {
                    new_dic.insert(self.eval_tp(k)?, self.eval_tp(v)?);
                }
                Ok(TyParam::Dict(new_dic))
            }
            TyParam::Set(set) => {
                let mut new_set = set! {};
                for v in set.into_iter() {
                    new_set.insert(self.eval_tp(v)?);
                }
                Ok(TyParam::Set(new_set))
            }
            TyParam::Record(dict) => {
                let mut fields = dict! {};
                for (name, tp) in dict.into_iter() {
                    fields.insert(name, self.eval_tp(tp)?);
                }
                Ok(TyParam::Record(fields))
            }
            TyParam::Type(t) => self
                .eval_t_params(*t, self.level, &())
                .map(TyParam::t)
                .map_err(|(_, errs)| errs),
            TyParam::Erased(t) => self
                .eval_t_params(*t, self.level, &())
                .map(TyParam::erased)
                .map_err(|(_, errs)| errs),
            TyParam::Value(ValueObj::Type(mut t)) => {
                t.try_map_t(|t| {
                    self.eval_t_params(t, self.level, &())
                        .map_err(|(_, errs)| errs)
                })?;
                Ok(TyParam::Value(ValueObj::Type(t)))
            }
            TyParam::ProjCall { obj, attr, args } => self.eval_proj_call(*obj, attr, args, &()),
            TyParam::Value(_) => Ok(p.clone()),
            other => feature_error!(self, Location::Unknown, &format!("evaluating {other}")),
        }
    }

    /// Evaluate `substituted`.
    /// If the evaluation fails, return a harmless type (filled with `Failure`) and errors
    pub(crate) fn eval_t_params(
        &self,
        substituted: Type,
        level: usize,
        t_loc: &impl Locational,
    ) -> Result<Type, (Type, EvalErrors)> {
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
                let new_constraint = Constraint::new_sandwiched(sub, sup);
                fv.update_constraint(new_constraint, false);
                Ok(Type::FreeVar(fv))
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
                }
                match self.eval_t_params(*subr.return_t, level, t_loc) {
                    Ok(return_t) => Ok(subr_t(
                        subr.kind,
                        subr.non_default_params,
                        subr.var_params.map(|v| *v),
                        subr.default_params,
                        return_t,
                    )),
                    Err((_, errs)) => {
                        let subr = subr_t(
                            subr.kind,
                            subr.non_default_params,
                            subr.var_params.map(|v| *v),
                            subr.default_params,
                            Failure,
                        );
                        Err((subr, errs))
                    }
                }
            }
            Type::Refinement(refine) => {
                let pred = self
                    .eval_pred(*refine.pred)
                    .map_err(|errs| (Failure, errs))?;
                Ok(refinement(refine.var, *refine.t, pred))
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
                        Err(errs) => {
                            // TODO: detoxify `p`
                            return Err((poly(name, params), errs));
                        }
                    };
                }
                Ok(poly(name, params))
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
            other if other.is_monomorphic() => Ok(other),
            other => feature_error!(self, t_loc.loc(), "???").map_err(|errs| (other, errs)),
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
            Type::FreeVar(fv) if fv.is_unbound() => {
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
            match ctx.validate_and_project(&sub, opt_sup.as_ref(), &rhs, self, level, t_loc) {
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
            match self.validate_and_project(&sub, opt_sup.as_ref(), &rhs, ty_ctx, level, t_loc) {
                Triple::Ok(t) => return Ok(t),
                Triple::Err(err) => return Err(err),
                Triple::None => {}
            }
            for (class, methods) in ty_ctx.methods_list.iter() {
                match (&class, &opt_sup) {
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
                match self.validate_and_project(&sub, opt_sup.as_ref(), &rhs, methods, level, t_loc)
                {
                    Triple::Ok(t) => return Ok(t),
                    Triple::Err(err) => return Err(err),
                    Triple::None => {}
                }
            }
        }
        if let Some(fv) = lhs.as_free() {
            let (sub, sup) = fv.get_subsup().unwrap();
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
            TyParam::Array(tps) => {
                let mut union = Type::Never;
                let len = tps.len();
                for tp in tps {
                    union = self.union(&union, &self.convert_tp_into_type(tp)?);
                }
                Ok(array_t(union, TyParam::value(len)))
            }
            TyParam::UnsizedArray(elem) => {
                let elem = self.convert_tp_into_type(*elem)?;
                Ok(unknown_len_array_t(elem))
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
            // TyParam(Ts: Array(Type)) -> Type(Ts: Array(Type))
            TyParam::FreeVar(fv) if fv.get_type().is_some() => Ok(named_free_var(
                fv.unbound_name().unwrap(),
                fv.level().unwrap(),
                fv.constraint().unwrap(),
            )),
            TyParam::Type(t) => Ok(t.as_ref().clone()),
            TyParam::Mono(name) => Ok(Type::Mono(name)),
            TyParam::App { name, args } => Ok(Type::Poly { name, params: args }),
            TyParam::Proj { obj, attr } => {
                let lhs = self.convert_tp_into_type(*obj)?;
                Ok(lhs.proj(attr))
            }
            TyParam::ProjCall { obj, attr, args } => Ok(proj_call(*obj, attr, args)),
            // TyParam::Erased(_t) => Ok(Type::Obj),
            TyParam::Value(v) => self.convert_value_into_type(v).map_err(TyParam::Value),
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
            TyParam::Array(arr) => {
                let mut new = vec![];
                for elem in arr {
                    let elem = self.convert_tp_into_value(elem)?;
                    new.push(elem);
                }
                Ok(ValueObj::Array(new.into()))
            }
            TyParam::UnsizedArray(elem) => {
                let elem = self.convert_tp_into_value(*elem)?;
                Ok(ValueObj::UnsizedArray(Box::new(elem)))
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
            _ => Err(typ),
        }
    }

    pub(crate) fn convert_value_into_type(&self, val: ValueObj) -> Result<Type, ValueObj> {
        match val {
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
            ValueObj::Array(arr) => {
                let len = TyParam::value(arr.len());
                let mut union = Type::Never;
                for v in arr.iter().cloned() {
                    union = self.union(&union, &self.convert_value_into_type(v)?);
                }
                Ok(array_t(union, len))
            }
            ValueObj::UnsizedArray(elem) => {
                let elem = self.convert_value_into_type(*elem)?;
                Ok(unknown_len_array_t(elem))
            }
            ValueObj::Set(set) => Ok(v_enum(set)),
            ValueObj::Dict(dic) => {
                let dic = dic
                    .into_iter()
                    .map(|(k, v)| (TyParam::Value(k), TyParam::Value(v)))
                    .collect();
                Ok(dict_t(TyParam::Dict(dic)))
            }
            ValueObj::Subr(subr) => subr.as_type(self).ok_or(ValueObj::Subr(subr)),
            other => Err(other),
        }
    }

    /// * Ok if `value` can be upcast to `TyParam`
    /// * Err if it is simply converted to `TyParam::Value(value)`
    pub(crate) fn convert_value_into_tp(value: ValueObj) -> Result<TyParam, TyParam> {
        match value {
            ValueObj::Type(t) => Ok(TyParam::t(t.into_typ())),
            ValueObj::Array(arr) => {
                let mut new_arr = vec![];
                for v in arr.iter().cloned() {
                    let tp = match Self::convert_value_into_tp(v) {
                        Ok(tp) => tp,
                        Err(tp) => tp,
                    };
                    new_arr.push(tp);
                }
                Ok(TyParam::Array(new_arr))
            }
            ValueObj::UnsizedArray(elem) => {
                let tp = match Self::convert_value_into_tp(*elem) {
                    Ok(tp) => tp,
                    Err(tp) => tp,
                };
                Ok(TyParam::UnsizedArray(Box::new(tp)))
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

    pub(crate) fn convert_type_to_array(&self, ty: Type) -> Result<Vec<ValueObj>, Type> {
        match ty {
            Type::FreeVar(fv) if fv.is_linked() => {
                let t = fv.crack().clone();
                self.convert_type_to_array(t)
            }
            Type::Refinement(refine) => self.convert_type_to_array(*refine.t),
            Type::Poly { name, params } if &name[..] == "Array" || &name[..] == "Array!" => {
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

    pub(crate) fn convert_value_into_array(
        &self,
        val: ValueObj,
    ) -> Result<Vec<ValueObj>, ValueObj> {
        match val {
            ValueObj::Array(arr) => Ok(arr.to_vec()),
            ValueObj::Type(t) => self
                .convert_type_to_array(t.into_typ())
                .map_err(ValueObj::builtin_type),
            _ => Err(val),
        }
    }

    fn validate_and_project(
        &self,
        sub: &Type,
        opt_sup: Option<&Type>,
        rhs: &str,
        methods: &Context,
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
                let (quant_sub, _) = self.get_type_and_ctx(&sub.qual_name()).unwrap();
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
                let _sub_subs = match Substituter::substitute_typarams(self, quant_sub, sub) {
                    Ok(sub_subs) => sub_subs,
                    Err(errs) => {
                        return Triple::Err(errs);
                    }
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

    fn detach_tp(&self, tp: TyParam, tv_cache: &mut TyVarCache) -> TyParam {
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

    fn do_proj_call(
        &self,
        obj: ValueObj,
        lhs: TyParam,
        args: Vec<TyParam>,
        t_loc: &impl Locational,
    ) -> EvalResult<TyParam> {
        if let ValueObj::Subr(subr) = obj {
            let mut pos_args = vec![];
            if subr.sig_t().is_method() {
                match ValueObj::try_from(lhs) {
                    Ok(value) => {
                        pos_args.push(value);
                    }
                    Err(_) => {
                        return feature_error!(self, t_loc.loc(), "??");
                    }
                }
            }
            for pos_arg in args.into_iter() {
                match ValueObj::try_from(pos_arg) {
                    Ok(value) => {
                        pos_args.push(value);
                    }
                    Err(_) => {
                        return feature_error!(self, t_loc.loc(), "??");
                    }
                }
            }
            let args = ValueArgs::new(pos_args, dict! {});
            let tp = self.call(subr, args, t_loc.loc())?;
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
            for (_class, methods) in ty_ctx.methods_list.iter() {
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
            for (_class, methods) in ty_ctx.methods_list.iter() {
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

    pub(crate) fn eval_pred(&self, p: Predicate) -> EvalResult<Predicate> {
        match p {
            Predicate::Value(_) | Predicate::Const(_) => Ok(p),
            Predicate::Equal { lhs, rhs } => Ok(Predicate::eq(lhs, self.eval_tp(rhs)?)),
            Predicate::NotEqual { lhs, rhs } => Ok(Predicate::ne(lhs, self.eval_tp(rhs)?)),
            Predicate::LessEqual { lhs, rhs } => Ok(Predicate::le(lhs, self.eval_tp(rhs)?)),
            Predicate::GreaterEqual { lhs, rhs } => Ok(Predicate::ge(lhs, self.eval_tp(rhs)?)),
            Predicate::And(l, r) => Ok(self.eval_pred(*l)? & self.eval_pred(*r)?),
            Predicate::Or(l, r) => Ok(self.eval_pred(*l)? | self.eval_pred(*r)?),
            Predicate::Not(pred) => Ok(!self.eval_pred(*pred)?),
        }
    }

    pub(crate) fn get_tp_t(&self, p: &TyParam) -> EvalResult<Type> {
        let p = self
            .eval_tp(p.clone())
            .map_err(|errs| {
                log!(err "{errs}");
                errs
            })
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
            TyParam::App { name, args } => self
                .rec_get_const_obj(&name)
                .and_then(|v| {
                    let ty = self.convert_value_into_type(v.clone()).ok()?;
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
                .ok_or_else(|| {
                    EvalErrors::from(EvalError::unreachable(
                        self.cfg.input.clone(),
                        fn_name!(),
                        line!(),
                    ))
                }),
            TyParam::Array(tps) => {
                let tp_t = if let Some(fst) = tps.get(0) {
                    self.get_tp_t(fst)?
                } else {
                    Never
                };
                let t = array_t(tp_t, TyParam::value(tps.len()));
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
                let tp = self.eval_proj_call(*obj, attr, args, &())?;
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
        let p = self.eval_tp(p.clone())?;
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
                self.subtype_of(&self.get_tp_t(rhs).unwrap(), &Type::Type)
            }
            (_, TyParam::Type(r)) if r.is_unbound_var() => {
                let lhs = self.get_tp_t(lhs).unwrap();
                self.subtype_of(&lhs, &Type::Type)
            }
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
            (TyParam::Mono(m), TyParam::Value(l)) | (TyParam::Value(l), TyParam::Mono(m)) => {
                if let Some(o) = self.rec_get_const_obj(m) {
                    o == l
                } else {
                    true
                }
            }
            (TyParam::Erased(t), _) => t.as_ref() == &self.get_tp_t(rhs).unwrap(),
            (_, TyParam::Erased(t)) => t.as_ref() == &self.get_tp_t(lhs).unwrap(),
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
