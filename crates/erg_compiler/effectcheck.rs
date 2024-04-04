//! implements SideEffectChecker
//! SideEffectCheckerを実装
//! 関数や不変型に副作用がないかチェックする

use erg_common::config::ErgConfig;
use erg_common::log;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_parser::token::TokenKind;

use crate::context::Context;
use crate::error::{EffectError, EffectErrors};
use crate::hir::{Call, Def, Dict, Expr, List, Params, Set, Signature, Tuple, HIR};
use crate::ty::{HasType, Visibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BlockKind {
    // forbid side effects
    Func,
    ConstFunc,
    ConstInstant, // e.g. Type definition
    // allow side effects
    Proc,
    Instant,
    Module,
}

use BlockKind::*;

/// Checks code for side effects.
/// For example:
/// * check if expressions with side effects are not used in functions
/// * check if methods that change internal state are not defined in immutable classes
#[derive(Debug)]
pub struct SideEffectChecker<'c> {
    cfg: ErgConfig,
    path_stack: Vec<Visibility>,
    block_stack: Vec<BlockKind>,
    errs: EffectErrors,
    ctx: &'c Context,
}

impl<'c> SideEffectChecker<'c> {
    pub fn new(cfg: ErgConfig, ctx: &'c Context) -> Self {
        Self {
            cfg,
            path_stack: vec![],
            block_stack: vec![],
            errs: EffectErrors::empty(),
            ctx,
        }
    }

    fn full_path(&self) -> String {
        self.path_stack
            .iter()
            .fold(String::new(), |acc, vis| {
                if vis.is_public() {
                    acc + "." + &vis.def_namespace[..]
                } else {
                    acc + "::" + &vis.def_namespace[..]
                }
            })
            .trim_start_matches(['.', ':'].as_ref())
            .to_string()
    }

    /// It is permitted to define a procedure in a function,
    /// and of course it is permitted to cause side effects in a procedure
    ///
    /// However, it is not permitted to cause side effects within an instant block in a function
    /// (side effects are allowed in instant blocks in procedures and modules)
    fn in_context_effects_allowed(&self) -> bool {
        // if toplevel
        if self.block_stack.len() == 1 {
            return true;
        }
        match (
            self.block_stack.get(self.block_stack.len() - 2).unwrap(),
            self.block_stack.last().unwrap(),
        ) {
            (_, Func | ConstInstant) => false,
            (_, Proc) => true,
            (Proc | Module | Instant, Instant) => true,
            _ => false,
        }
    }

    pub fn check(mut self, hir: HIR, namespace: Str) -> Result<HIR, (HIR, EffectErrors)> {
        self.path_stack.push(Visibility::private(namespace));
        self.block_stack.push(Module);
        log!(info "the side-effects checking process has started.{RESET}");
        // At the top level, there is no problem with side effects, only check for purity violations.
        // トップレベルでは副作用があっても問題なく、純粋性違反がないかのみチェックする
        for expr in hir.module.iter() {
            match expr {
                Expr::Def(def) => {
                    self.check_def(def);
                }
                Expr::ClassDef(class_def) => {
                    if let Some(req_sup) = &class_def.require_or_sup {
                        self.check_expr(req_sup);
                    }
                    let name_and_vis = Visibility::new(
                        class_def.sig.vis().clone(),
                        class_def.sig.inspect().clone(),
                    );
                    self.path_stack.push(name_and_vis);
                    for def in class_def.all_methods() {
                        self.check_expr(def);
                    }
                    self.path_stack.pop();
                }
                Expr::PatchDef(patch_def) => {
                    self.check_expr(patch_def.base.as_ref());
                    // TODO: grow
                    for def in patch_def.methods.iter() {
                        self.check_expr(def);
                    }
                }
                Expr::Call(call) => {
                    for parg in call.args.pos_args.iter() {
                        self.check_expr(&parg.expr);
                    }
                    for kwarg in call.args.kw_args.iter() {
                        self.check_expr(&kwarg.expr);
                    }
                }
                Expr::BinOp(bin) => {
                    self.check_expr(&bin.lhs);
                    self.check_expr(&bin.rhs);
                }
                Expr::UnaryOp(unary) => {
                    self.check_expr(&unary.expr);
                }
                Expr::Accessor(_) | Expr::Literal(_) => {}
                Expr::List(list) => match list {
                    List::Normal(lis) => {
                        for elem in lis.elems.pos_args.iter() {
                            self.check_expr(&elem.expr);
                        }
                    }
                    List::WithLength(lis) => {
                        self.check_expr(&lis.elem);
                        if let Some(len) = &lis.len {
                            self.check_expr(len);
                        }
                    }
                    List::Comprehension(lis) => {
                        self.check_expr(&lis.elem);
                        self.check_expr(&lis.guard);
                    }
                },
                Expr::Tuple(tuple) => match tuple {
                    Tuple::Normal(tuple) => {
                        for elem in tuple.elems.pos_args.iter() {
                            self.check_expr(&elem.expr);
                        }
                    }
                },
                Expr::Record(rec) => {
                    self.path_stack
                        .push(Visibility::private(Str::ever("<record>")));
                    self.block_stack.push(Instant);
                    for attr in rec.attrs.iter() {
                        self.check_def(attr);
                    }
                    self.path_stack.pop();
                    self.block_stack.pop();
                }
                Expr::Set(set) => match set {
                    Set::Normal(set) => {
                        for elem in set.elems.pos_args.iter() {
                            self.check_expr(&elem.expr);
                        }
                    }
                    Set::WithLength(set) => {
                        self.check_expr(&set.elem);
                        self.check_expr(&set.len);
                    }
                },
                Expr::Dict(dict) => match dict {
                    Dict::Normal(dict) => {
                        for kv in dict.kvs.iter() {
                            self.check_expr(&kv.key);
                            self.check_expr(&kv.value);
                        }
                    }
                    other => todo!("{other}"),
                },
                Expr::TypeAsc(tasc) => {
                    self.check_expr(&tasc.expr);
                }
                Expr::Lambda(lambda) => {
                    let is_proc = lambda.is_procedural();
                    if is_proc {
                        self.path_stack
                            .push(Visibility::private(Str::ever("<lambda!>")));
                        self.block_stack.push(Proc);
                    } else {
                        self.path_stack
                            .push(Visibility::private(Str::ever("<lambda>")));
                        self.block_stack.push(Func);
                    }
                    lambda.body.iter().for_each(|chunk| self.check_expr(chunk));
                    self.path_stack.pop();
                    self.block_stack.pop();
                }
                Expr::ReDef(_)
                | Expr::Code(_)
                | Expr::Compound(_)
                | Expr::Import(_)
                | Expr::Dummy(_) => {}
            }
        }
        log!(info "the side-effects checking process has completed, found errors: {}{RESET}", self.errs.len());
        if self.errs.is_empty() {
            Ok(hir)
        } else {
            Err((hir, self.errs))
        }
    }

    fn check_params(&mut self, params: &Params) {
        for nd_param in params.non_defaults.iter() {
            if nd_param.vi.t.is_procedure() && !nd_param.inspect().unwrap().ends_with('!') {
                self.errs.push(EffectError::proc_assign_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    nd_param.raw.pat.loc(),
                    self.full_path(),
                ));
            }
        }
        if let Some(var_arg) = params.var_params.as_deref() {
            if var_arg.vi.t.is_procedure() && !var_arg.inspect().unwrap().ends_with('!') {
                self.errs.push(EffectError::proc_assign_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    var_arg.raw.pat.loc(),
                    self.full_path(),
                ));
            }
        }
        for d_param in params.defaults.iter() {
            if d_param.sig.vi.t.is_procedure() && !d_param.inspect().unwrap().ends_with('!') {
                self.errs.push(EffectError::proc_assign_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    d_param.sig.raw.pat.loc(),
                    self.full_path(),
                ));
            }
            self.check_expr(&d_param.default_val);
        }
    }

    fn check_def(&mut self, def: &Def) {
        let is_procedural = def.sig.is_procedural();
        let is_subr = def.sig.is_subr();
        let is_const = def.sig.is_const();
        if is_subr {
            let name_and_vis = Visibility::new(def.sig.vis().clone(), def.sig.inspect().clone());
            self.path_stack.push(name_and_vis);
        }
        match (is_procedural, is_subr, is_const) {
            (true, true, true) => {
                panic!("user-defined constant procedures are not allowed");
            }
            (true, true, false) => {
                self.block_stack.push(Proc);
            }
            (_, false, false) => {
                self.block_stack.push(Instant);
            }
            (false, true, true) => {
                self.block_stack.push(ConstFunc);
            }
            (false, true, false) => {
                self.block_stack.push(Func);
            }
            (_, false, true) => {
                self.block_stack.push(ConstInstant);
            }
        }
        if let Signature::Subr(sig) = &def.sig {
            self.check_params(&sig.params);
        }
        let last_idx = def.body.block.len().saturating_sub(1);
        for (i, chunk) in def.body.block.iter().enumerate() {
            self.check_expr(chunk);
            // e.g. `echo = print!`
            if i == last_idx
                && self.block_stack.last().unwrap() == &Instant
                && !def.sig.is_procedural()
                && chunk.t().is_procedure()
            {
                self.errs.push(EffectError::proc_assign_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    def.sig.loc(),
                    self.full_path(),
                ));
            }
        }
        if is_subr {
            self.path_stack.pop();
        }
        self.block_stack.pop();
    }

    /// check if `expr` has side-effects / purity violations.
    ///
    /// returns effects, purity violations will be appended to `self.errs`.
    ///
    /// causes side-effects:
    /// ```python
    /// p!() // 1 side-effect
    /// p!(q!()) // 2 side-effects
    /// x =
    ///     y = r!()
    ///     y + 1 // 1 side-effect
    /// ```
    /// causes no side-effects:
    /// ```python
    /// q! = p!
    /// y = f(p!)
    /// ```
    /// purity violation:
    /// ```python
    /// for iter, i -> print! i
    /// ```
    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(_) => {}
            Expr::Def(def) => {
                self.check_def(def);
            }
            Expr::ClassDef(class_def) => {
                if let Some(req_sup) = &class_def.require_or_sup {
                    self.check_expr(req_sup);
                }
                for def in class_def.all_methods() {
                    self.check_expr(def);
                }
            }
            Expr::PatchDef(patch_def) => {
                self.check_expr(patch_def.base.as_ref());
                for def in patch_def.methods.iter() {
                    self.check_expr(def);
                }
            }
            Expr::List(list) => match list {
                List::Normal(lis) => {
                    for elem in lis.elems.pos_args.iter() {
                        self.check_expr(&elem.expr);
                    }
                }
                List::WithLength(lis) => {
                    self.check_expr(&lis.elem);
                    if let Some(len) = &lis.len {
                        self.check_expr(len);
                    }
                }
                List::Comprehension(lis) => {
                    self.check_expr(&lis.elem);
                    self.check_expr(&lis.guard);
                }
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    for arg in tup.elems.pos_args.iter() {
                        self.check_expr(&arg.expr);
                    }
                }
            },
            Expr::Record(record) => {
                self.path_stack
                    .push(Visibility::private(Str::ever("<record>")));
                self.block_stack.push(Instant);
                for attr in record.attrs.iter() {
                    self.check_def(attr);
                }
                self.path_stack.pop();
                self.block_stack.pop();
            }
            Expr::Set(set) => match set {
                Set::Normal(set) => {
                    for elem in set.elems.pos_args.iter() {
                        self.check_expr(&elem.expr);
                    }
                }
                Set::WithLength(set) => {
                    self.check_expr(&set.elem);
                    self.check_expr(&set.len);
                }
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dict) => {
                    for kv in dict.kvs.iter() {
                        self.check_expr(&kv.key);
                        self.check_expr(&kv.value);
                    }
                }
                other => todo!("{other}"),
            },
            Expr::Call(call) => {
                self.constructor_destructor_check(call);
                self.check_expr(&call.obj);
                if (call.obj.t().is_procedure()
                    || call
                        .attr_name
                        .as_ref()
                        .map(|name| name.is_procedural())
                        .unwrap_or(false))
                    && !self.in_context_effects_allowed()
                {
                    self.errs.push(EffectError::has_effect(
                        self.cfg.input.clone(),
                        line!() as usize,
                        expr,
                        self.full_path(),
                    ));
                }
                call.args
                    .pos_args
                    .iter()
                    .for_each(|parg| self.check_expr(&parg.expr));
                call.args
                    .kw_args
                    .iter()
                    .for_each(|kwarg| self.check_expr(&kwarg.expr));
            }
            Expr::UnaryOp(unary) => {
                self.check_expr(&unary.expr);
            }
            Expr::BinOp(bin) => {
                self.check_expr(&bin.lhs);
                self.check_expr(&bin.rhs);
                if (bin.op.kind == TokenKind::IsOp || bin.op.kind == TokenKind::IsNotOp)
                    && !self.in_context_effects_allowed()
                {
                    self.errs.push(EffectError::has_effect(
                        self.cfg.input.clone(),
                        line!() as usize,
                        expr,
                        self.full_path(),
                    ));
                }
            }
            Expr::Lambda(lambda) => {
                let is_proc = lambda.is_procedural();
                if is_proc {
                    self.path_stack
                        .push(Visibility::private(Str::ever("<lambda!>")));
                    self.block_stack.push(Proc);
                } else {
                    self.path_stack
                        .push(Visibility::private(Str::ever("<lambda>")));
                    self.block_stack.push(Func);
                }
                self.check_params(&lambda.params);
                lambda.body.iter().for_each(|chunk| self.check_expr(chunk));
                self.path_stack.pop();
                self.block_stack.pop();
            }
            Expr::TypeAsc(type_asc) => {
                self.check_expr(&type_asc.expr);
            }
            Expr::Accessor(acc) => {
                if !self.in_context_effects_allowed()
                    && !acc.var_info().is_parameter()
                    && acc.ref_t().is_mut_type()
                    && acc.root_obj().map_or(true, |obj| !obj.ref_t().is_ref())
                    && acc.var_info().def_namespace() != &self.full_path()
                {
                    self.errs.push(EffectError::touch_mut_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        expr,
                        self.full_path(),
                    ));
                }
            }
            Expr::ReDef(_)
            | Expr::Code(_)
            | Expr::Compound(_)
            | Expr::Import(_)
            | Expr::Dummy(_) => {}
        }
    }

    fn constructor_destructor_check(&mut self, call: &Call) {
        let Some(gen_t) = call.signature_t().and_then(|sig| sig.return_t()) else {
            return;
        };
        // the call generates a new instance
        // REVIEW: is this correct?
        if !self.in_context_effects_allowed()
            && call
                .signature_t()
                .is_some_and(|sig| sig.param_ts().iter().all(|p| !p.contains_type(gen_t)))
        {
            if let Some(typ_ctx) = self.ctx.get_nominal_type_ctx(gen_t) {
                if typ_ctx.get_method_kv("__init__!").is_some()
                    || typ_ctx.get_method_kv("__del__!").is_some()
                {
                    self.errs.push(EffectError::constructor_destructor_error(
                        self.cfg.input.clone(),
                        line!() as usize,
                        call.loc(),
                        self.full_path(),
                    ));
                }
            }
        }
    }

    pub(crate) fn is_impure(expr: &Expr) -> bool {
        match expr {
            Expr::Call(call) => {
                call.ref_t().is_procedure()
                    || call
                        .args
                        .pos_args
                        .iter()
                        .any(|parg| Self::is_impure(&parg.expr))
                    || call
                        .args
                        .var_args
                        .iter()
                        .any(|varg| Self::is_impure(&varg.expr))
                    || call
                        .args
                        .kw_args
                        .iter()
                        .any(|kwarg| Self::is_impure(&kwarg.expr))
            }
            Expr::BinOp(bin) => Self::is_impure(&bin.lhs) || Self::is_impure(&bin.rhs),
            Expr::UnaryOp(unary) => Self::is_impure(&unary.expr),
            Expr::List(lis) => match lis {
                List::Normal(lis) => lis
                    .elems
                    .pos_args
                    .iter()
                    .any(|elem| Self::is_impure(&elem.expr)),
                List::WithLength(lis) => {
                    Self::is_impure(&lis.elem)
                        || lis.len.as_ref().map_or(false, |len| Self::is_impure(len))
                }
                _ => todo!(),
            },
            Expr::Tuple(tup) => match tup {
                Tuple::Normal(tup) => tup
                    .elems
                    .pos_args
                    .iter()
                    .any(|elem| Self::is_impure(&elem.expr)),
            },
            Expr::Set(set) => match set {
                Set::Normal(set) => set
                    .elems
                    .pos_args
                    .iter()
                    .any(|elem| Self::is_impure(&elem.expr)),
                Set::WithLength(set) => Self::is_impure(&set.elem) || Self::is_impure(&set.len),
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dict) => dict
                    .kvs
                    .iter()
                    .any(|kv| Self::is_impure(&kv.key) || Self::is_impure(&kv.value)),
                _ => todo!(),
            },
            Expr::Lambda(lambda) => {
                lambda.op.is_procedural() || lambda.body.iter().any(Self::is_impure)
            }
            Expr::Def(def) => def.sig.is_procedural() || def.body.block.iter().any(Self::is_impure),
            /*
            Expr::ClassDef(class_def) => {
                class_def.methods.iter().any(|def| Self::is_impure(def))
            }
            Expr::PatchDef(patch_def) => {
                patch_def.methods.iter().any(|def| Self::is_impure(def))
            }*/
            Expr::Code(block) | Expr::Compound(block) => block.iter().any(Self::is_impure),
            _ => false,
        }
    }

    pub(crate) fn is_pure(expr: &Expr) -> bool {
        !Self::is_impure(expr)
    }
}
