//! Desugaring syntax sugars.
//!
//! Syntax sugarをdesugarする
//! e.g. Literal parameters, Multi assignment
//! 型チェックなどによる検証は行わない
#![allow(dead_code)]

use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{enum_unwrap, get_hash, set};

use crate::ast::{
    Accessor, Args, Block, Call, Def, DefBody, DefId, Expr, Identifier, Lambda, LambdaSignature,
    Literal, Local, Module, ParamPattern, ParamSignature, Params, PosArg, Signature, SubrSignature,
    TypeBoundSpecs, TypeSpec, VarName, VarPattern, VarSignature,
};
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
enum BufIndex<'s> {
    Array(usize),
    Tuple(usize),
    Record(&'s str),
}

#[derive(Debug)]
pub struct Desugarer {
    desugared: Set<Str>,
    var_id: usize,
}

impl Desugarer {
    pub fn new() -> Desugarer {
        Self {
            desugared: Set::default(),
            var_id: 0,
        }
    }

    fn fresh_var_name(&mut self) -> String {
        let var_name = format!("%v{}", self.var_id);
        self.var_id += 1;
        var_name
    }

    pub fn desugar(&mut self, module: Module) -> Module {
        let module = self.desugar_multiple_pattern_def(module);
        self.desugar_pattern(module)
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
                            let previous = enum_unwrap!(new.pop().unwrap(), Expr::Def);
                            let name = def.sig.ident().unwrap().clone();
                            let id = def.body.id;
                            let op = def.body.op.clone();
                            let (call, return_t_spec) = if previous.body.block.len() == 1
                                && previous.body.block.first().unwrap().is_match_call()
                            {
                                self.add_arg_to_match_call(previous, def)
                            } else {
                                self.gen_match_call(previous, def)
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
                            let params = Params::new(vec![param], None, vec![], None);
                            let sig = Signature::Subr(SubrSignature::new(
                                set! {},
                                name,
                                params,
                                return_t_spec,
                                TypeBoundSpecs::empty(),
                            ));
                            let body = DefBody::new(op, Block::new(vec![Expr::Call(call)]), id);
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

    fn add_arg_to_match_call(&self, mut previous: Def, def: Def) -> (Call, Option<TypeSpec>) {
        let op = Token::from_str(TokenKind::FuncArrow, "->");
        let mut call = enum_unwrap!(previous.body.block.remove(0), Expr::Call);
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
        let sig = LambdaSignature::new(sig.params, return_t_spec.clone(), sig.bounds);
        let new_branch = Lambda::new(sig, op, def.body.block, def.body.id);
        call.args.push_pos(PosArg::new(Expr::Lambda(new_branch)));
        (call, return_t_spec)
    }

    fn gen_match_call(&self, previous: Def, def: Def) -> (Call, Option<TypeSpec>) {
        let op = Token::from_str(TokenKind::FuncArrow, "->");
        let sig = enum_unwrap!(previous.sig, Signature::Subr);
        let match_symbol = Expr::static_local("match");
        let sig = LambdaSignature::new(sig.params, sig.return_t_spec, sig.bounds);
        let first_branch = Lambda::new(sig, op.clone(), previous.body.block, previous.body.id);
        let sig = enum_unwrap!(def.sig, Signature::Subr);
        let return_t_spec = sig.return_t_spec;
        let sig = LambdaSignature::new(sig.params, return_t_spec.clone(), sig.bounds);
        let second_branch = Lambda::new(sig, op, def.body.block, def.body.id);
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
    }

    /// `f 0 = 1` -> `f _: {0} = 1`
    fn desugar_literal_pattern(&self, _mod: Module) -> Module {
        todo!()
    }

    /// `[i, j] = [1, 2]` -> `i = 1; j = 2`
    /// `[i, j] = l` -> `i = l[0]; j = l[1]`
    /// `[i, [j, k]] = l` -> `i = l[0]; j = l[1][0]; k = l[1][1]`
    /// `(i, j) = t` -> `i = t.0; j = t.1`
    /// `{i; j} = s` -> `i = s.i; j = s.j`
    fn desugar_pattern(&mut self, mut module: Module) -> Module {
        let mut new = Module::with_capacity(module.len());
        while let Some(chunk) = module.lpop() {
            match chunk {
                Expr::Def(Def {
                    sig: Signature::Var(v),
                    body,
                }) => match &v.pat {
                    VarPattern::Tuple(tup) => {
                        let buf_name = self.fresh_var_name();
                        let buf_sig = Signature::Var(VarSignature::new(
                            VarPattern::Ident(Identifier::private_with_line(
                                Str::rc(&buf_name),
                                v.ln_begin().unwrap(),
                            )),
                            v.t_spec,
                        ));
                        let buf_def = Def::new(buf_sig, body);
                        new.push(Expr::Def(buf_def));
                        for (n, elem) in tup.elems.iter().enumerate() {
                            self.desugar_nested_var_pattern(
                                &mut new,
                                elem,
                                &buf_name,
                                BufIndex::Tuple(n),
                            );
                        }
                    }
                    VarPattern::Array(arr) => {
                        let buf_name = self.fresh_var_name();
                        let buf_sig = Signature::Var(VarSignature::new(
                            VarPattern::Ident(Identifier::private_with_line(
                                Str::rc(&buf_name),
                                v.ln_begin().unwrap(),
                            )),
                            v.t_spec,
                        ));
                        let buf_def = Def::new(buf_sig, body);
                        new.push(Expr::Def(buf_def));
                        for (n, elem) in arr.elems.iter().enumerate() {
                            self.desugar_nested_var_pattern(
                                &mut new,
                                elem,
                                &buf_name,
                                BufIndex::Array(n),
                            );
                        }
                    }
                    VarPattern::Record(_rec) => todo!(),
                    VarPattern::Ident(_i) => {
                        let def = Def::new(Signature::Var(v), body);
                        new.push(Expr::Def(def));
                    }
                    _ => {}
                },
                other => {
                    new.push(other);
                }
            }
        }
        new
    }

    fn desugar_nested_var_pattern(
        &mut self,
        new_module: &mut Module,
        sig: &VarSignature,
        buf_name: &str,
        buf_index: BufIndex,
    ) {
        let obj = Expr::local(buf_name, sig.ln_begin().unwrap(), sig.col_begin().unwrap());
        let acc = match buf_index {
            BufIndex::Tuple(n) => {
                Accessor::tuple_attr(obj, Literal::nat(n, sig.ln_begin().unwrap()))
            }
            BufIndex::Array(n) => {
                Accessor::subscr(obj, Expr::Lit(Literal::nat(n, sig.ln_begin().unwrap())))
            }
            BufIndex::Record(attr) => {
                // TODO: visibility
                Accessor::attr(
                    obj,
                    Token::from_str(TokenKind::Dot, "."),
                    Local::dummy_with_line(attr, sig.ln_begin().unwrap()),
                )
            }
        };
        let id = DefId(get_hash(&(&acc, buf_name)));
        let block = Block::new(vec![Expr::Accessor(acc)]);
        let op = Token::from_str(TokenKind::Equal, "=");
        let body = DefBody::new(op, block, id);
        match &sig.pat {
            VarPattern::Tuple(tup) => {
                let buf_name = self.fresh_var_name();
                let buf_sig = Signature::Var(VarSignature::new(
                    VarPattern::Ident(Identifier::private_with_line(
                        Str::rc(&buf_name),
                        sig.ln_begin().unwrap(),
                    )),
                    None,
                ));
                let buf_def = Def::new(buf_sig, body);
                new_module.push(Expr::Def(buf_def));
                for (n, elem) in tup.elems.iter().enumerate() {
                    self.desugar_nested_var_pattern(
                        new_module,
                        elem,
                        &buf_name,
                        BufIndex::Tuple(n),
                    );
                }
            }
            VarPattern::Array(arr) => {
                let buf_name = self.fresh_var_name();
                let buf_sig = Signature::Var(VarSignature::new(
                    VarPattern::Ident(Identifier::private_with_line(
                        Str::rc(&buf_name),
                        sig.ln_begin().unwrap(),
                    )),
                    None,
                ));
                let buf_def = Def::new(buf_sig, body);
                new_module.push(Expr::Def(buf_def));
                for (n, elem) in arr.elems.iter().enumerate() {
                    self.desugar_nested_var_pattern(
                        new_module,
                        elem,
                        &buf_name,
                        BufIndex::Array(n),
                    );
                }
            }
            VarPattern::Record(_rec) => todo!(),
            VarPattern::Ident(_ident) => {
                let def = Def::new(Signature::Var(sig.clone()), body);
                new_module.push(Expr::Def(def));
            }
            _ => {}
        }
    }

    /// `F(I | I > 0)` -> `F(I: {I: Int | I > 0})`
    fn desugar_refinement_pattern(&self, _mod: Module) -> Module {
        todo!()
    }

    /// ```erg
    /// @deco
    /// f x = ...
    /// ```
    /// ↓
    /// ```erg
    /// _f x = ...
    /// f = deco _f
    /// ```
    fn desugar_decorators(&self, _mod: Module) -> Module {
        todo!()
    }
}

impl Default for Desugarer {
    fn default() -> Self {
        Self::new()
    }
}
