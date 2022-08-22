//! Desugaring syntax sugars.
//!
//! Syntax sugarをdesugarする
//! e.g. Literal parameters, Multi assignment
//! 型チェックなどによる検証は行わない
#![allow(dead_code)]

use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{enum_unwrap, set};

use crate::ast::{
    Accessor, Args, Block, Call, Def, DefBody, Expr, Lambda, LambdaSignature, Module, ParamPattern,
    ParamSignature, Params, PosArg, Signature, SubrSignature, TypeBoundSpecs, VarName, VarPattern,
};
use crate::token::{Token, TokenKind};

#[derive(Debug)]
pub struct Desugarer {
    desugared: Set<Str>,
}

impl Desugarer {
    pub fn new() -> Desugarer {
        Self {
            desugared: Set::default(),
        }
    }

    pub fn desugar(&mut self, module: Module) -> Module {
        self.desugar_multiple_pattern_def(module)
    }

    fn desugar_ubar_lambda(&self, _module: Module) -> Module {
        todo!()
    }

    /// `fib 0 = 0; fib 1 = 1; fib n = fib(n-1) + fib(n-2)`
    /// -> `fib n = match n, (0 -> 0), (1 -> 1), n -> fib(n-1) + fib(n-2)`
    fn desugar_multiple_pattern_def(&self, mut module: Module) -> Module {
        let mut new = Module::with_capacity(module.len());
        while let Some(chunk) = module.lpop() {
            match chunk {
                Expr::Def(def) if def.is_subr() => {
                    if let Some(Expr::Def(previous)) = new.last() {
                        if previous.is_subr() && previous.sig.name_as_str() == def.sig.name_as_str()
                        {
                            let mut previous = enum_unwrap!(new.pop().unwrap(), Expr::Def);
                            let name = def.sig.ident().unwrap().clone();
                            let op = Token::from_str(TokenKind::FuncArrow, "->");
                            let (call, return_t_spec) = if previous.body.block.len() == 1
                                && previous.body.block.first().unwrap().is_match_call()
                            {
                                let mut call =
                                    enum_unwrap!(previous.body.block.remove(0), Expr::Call);
                                let sig = enum_unwrap!(def.sig, Signature::Subr);
                                let return_t_spec = sig.return_t_spec;
                                let first_arg = sig.params.non_defaults.first().unwrap();
                                // 最後の定義の引数名を関数全体の引数名にする
                                if let Some(name) = first_arg.inspect() {
                                    call.args.remove_pos(0);
                                    let arg = PosArg::new(Expr::local(
                                        name,
                                        first_arg.ln_begin().unwrap(),
                                        first_arg.col_begin().unwrap(),
                                    ));
                                    call.args.insert_pos(0, arg);
                                }
                                let sig = LambdaSignature::new(
                                    sig.params,
                                    return_t_spec.clone(),
                                    sig.bounds,
                                );
                                let new_branch = Lambda::new(sig, op, def.body.block, def.body.id);
                                call.args.push_pos(PosArg::new(Expr::Lambda(new_branch)));
                                (call, return_t_spec)
                            } else {
                                let sig = enum_unwrap!(previous.sig, Signature::Subr);
                                let match_symbol = Expr::static_local("match");
                                let sig =
                                    LambdaSignature::new(sig.params, sig.return_t_spec, sig.bounds);
                                let first_branch = Lambda::new(
                                    sig,
                                    op.clone(),
                                    previous.body.block,
                                    previous.body.id,
                                );
                                let sig = enum_unwrap!(def.sig, Signature::Subr);
                                let return_t_spec = sig.return_t_spec;
                                let sig = LambdaSignature::new(
                                    sig.params,
                                    return_t_spec.clone(),
                                    sig.bounds,
                                );
                                let second_branch =
                                    Lambda::new(sig, op, def.body.block, def.body.id);
                                let args = Args::new(
                                    vec![
                                        PosArg::new(Expr::dummy_local("_")), // dummy argument, will be removed in line 56
                                        PosArg::new(Expr::Lambda(first_branch)),
                                        PosArg::new(Expr::Lambda(second_branch)),
                                    ],
                                    vec![],
                                    None,
                                );
                                let call = Call::new(match_symbol, None, args);
                                (call, return_t_spec)
                            };
                            let param_name = enum_unwrap!(&call.args.pos_args().iter().next().unwrap().expr, Expr::Accessor:(Accessor::Local:(_))).inspect();
                            // FIXME: multiple params
                            let param = VarName::new(Token::new(
                                TokenKind::Symbol,
                                param_name,
                                name.ln_begin().unwrap(),
                                name.col_end().unwrap() + 1, // HACK: `(name) %x = ...`という形を想定
                            ));
                            let param =
                                ParamSignature::new(ParamPattern::VarName(param), None, None);
                            let params = Params::new(vec![param], vec![], None);
                            let sig = Signature::Subr(SubrSignature::new(
                                set! {},
                                name,
                                params,
                                return_t_spec,
                                TypeBoundSpecs::empty(),
                            ));
                            let body = DefBody::new(
                                def.body.op,
                                Block::new(vec![Expr::Call(call)]),
                                def.body.id,
                            );
                            let def = Def::new(sig, body);
                            new.push(Expr::Def(def));
                        } else {
                            new.push(Expr::Def(def));
                        }
                    } else {
                        new.push(Expr::Def(def));
                    }
                }
                other => {
                    new.push(other);
                }
            }
        }
        new
    }

    /// `f 0 = 1` -> `f _: {0} = 1`
    fn desugar_literal_pattern(&self, _mod: Module) -> Module {
        todo!()
    }

    /// `[i, j] = [1, 2]` -> `i = 1; j = 2`
    /// `[i, j] = l` -> `i = l[0]; j = l[1]`
    /// `[i, [j, k]] = l` -> `i = l[0]; j = l[1][0]; k = l[1][1]`
    /// `(i, j) = t` -> `i = t.0; j = t.1`
    fn desugar_nest_vars_pattern(&self, mut module: Module) -> Module {
        let mut new = Module::with_capacity(module.len());
        while let Some(chunk) = module.lpop() {
            match chunk {
                Expr::Def(def) => {
                    if let Signature::Var(v) = &def.sig {
                        match &v.pat {
                            VarPattern::Array(_a) => {}
                            VarPattern::Record(_r) => {}
                            _ => {}
                        }
                    }
                    new.push(Expr::Def(def));
                }
                other => {
                    new.push(other);
                }
            }
        }
        new
    }

    /// `{i; j} = s` -> `i = s.i; j = s.j`
    fn desugar_record_pattern(&self, _mod: Module) -> Module {
        todo!()
    }

    /// `F(I | I > 0)` -> `F(I: {I: Int | I > 0})`
    fn desugar_refinement_pattern(&self, _mod: Module) -> Module {
        todo!()
    }

    /// `show! x: Show := print! x` -> `show! x: '1 | '1 <: Show := print! x`
    fn desugar_trait_parameter(&self, _mod: Module) -> Module {
        todo!()
    }
}

impl Default for Desugarer {
    fn default() -> Self {
        Self::new()
    }
}
