//! Desugaring syntax sugars.
//!
//! Syntax sugarをdesugarする
//! e.g. Literal parameters, Multi assignment
//! 型チェックなどによる検証は行わない
#![allow(dead_code)]

use erg_common::set::Set;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{enum_unwrap, get_hash, log, set};

use crate::ast::{
    Accessor, Args, Array, ArrayComprehension, ArrayWithLength, BinOp, Block, Call, DataPack, Def,
    DefBody, DefId, Expr, Identifier, KwArg, Lambda, LambdaSignature, Literal, Methods, Module,
    NormalArray, NormalRecord, NormalTuple, ParamPattern, ParamSignature, Params, PosArg, Record,
    RecordAttrs, ShortenedRecord, Signature, SubrSignature, Tuple, TypeAscription, TypeBoundSpecs,
    TypeSpec, UnaryOp, VarName, VarPattern, VarRecordAttr, VarSignature,
};
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
enum BufIndex<'i> {
    Array(usize),
    Tuple(usize),
    Record(&'i Identifier),
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

    #[allow(clippy::let_and_return)]
    pub fn desugar(&mut self, module: Module) -> Module {
        log!(info "the desugaring process has started.");
        let module = self.desugar_multiple_pattern_def(module);
        let module = self.desugar_pattern(module);
        let module = self.desugar_shortened_record(module);
        log!(info "AST (desugared):\n{module}");
        log!(info "the desugaring process has completed.");
        module
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
                            let param_name = enum_unwrap!(&call.args.pos_args().iter().next().unwrap().expr, Expr::Accessor:(Accessor::Ident:(_))).inspect();
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
                                TypeBoundSpecs::empty(),
                                params,
                                return_t_spec,
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

    fn gen_buf_name_and_sig(
        &mut self,
        line: usize,
        t_spec: Option<TypeSpec>,
    ) -> (String, Signature) {
        let buf_name = self.fresh_var_name();
        let buf_sig = Signature::Var(VarSignature::new(
            VarPattern::Ident(Identifier::private_with_line(Str::rc(&buf_name), line)),
            t_spec,
        ));
        (buf_name, buf_sig)
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
                        let (buf_name, buf_sig) =
                            self.gen_buf_name_and_sig(v.ln_begin().unwrap(), v.t_spec);
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
                        let (buf_name, buf_sig) =
                            self.gen_buf_name_and_sig(v.ln_begin().unwrap(), v.t_spec);
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
                    VarPattern::Record(rec) => {
                        let (buf_name, buf_sig) =
                            self.gen_buf_name_and_sig(v.ln_begin().unwrap(), v.t_spec);
                        let buf_def = Def::new(buf_sig, body);
                        new.push(Expr::Def(buf_def));
                        for VarRecordAttr { lhs, rhs } in rec.attrs.iter() {
                            self.desugar_nested_var_pattern(
                                &mut new,
                                rhs,
                                &buf_name,
                                BufIndex::Record(lhs),
                            );
                        }
                    }
                    VarPattern::DataPack(pack) => {
                        let (buf_name, buf_sig) = self.gen_buf_name_and_sig(
                            v.ln_begin().unwrap(),
                            Some(pack.class.clone()), // TODO: これだとvの型指定の意味がなくなる
                        );
                        let buf_def = Def::new(buf_sig, body);
                        new.push(Expr::Def(buf_def));
                        for VarRecordAttr { lhs, rhs } in pack.args.attrs.iter() {
                            self.desugar_nested_var_pattern(
                                &mut new,
                                rhs,
                                &buf_name,
                                BufIndex::Record(lhs),
                            );
                        }
                    }
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
            BufIndex::Record(attr) => Accessor::attr(obj, attr.clone()),
        };
        let id = DefId(get_hash(&(&acc, buf_name)));
        let block = Block::new(vec![Expr::Accessor(acc)]);
        let op = Token::from_str(TokenKind::Equal, "=");
        let body = DefBody::new(op, block, id);
        match &sig.pat {
            VarPattern::Tuple(tup) => {
                let (buf_name, buf_sig) = self.gen_buf_name_and_sig(sig.ln_begin().unwrap(), None);
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
                let (buf_name, buf_sig) = self.gen_buf_name_and_sig(sig.ln_begin().unwrap(), None);
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
            VarPattern::Record(rec) => {
                let (buf_name, buf_sig) = self.gen_buf_name_and_sig(sig.ln_begin().unwrap(), None);
                let buf_def = Def::new(buf_sig, body);
                new_module.push(Expr::Def(buf_def));
                for VarRecordAttr { lhs, rhs } in rec.attrs.iter() {
                    self.desugar_nested_var_pattern(
                        new_module,
                        rhs,
                        &buf_name,
                        BufIndex::Record(lhs),
                    );
                }
            }
            VarPattern::DataPack(pack) => {
                let (buf_name, buf_sig) =
                    self.gen_buf_name_and_sig(sig.ln_begin().unwrap(), Some(pack.class.clone()));
                let buf_def = Def::new(buf_sig, body);
                new_module.push(Expr::Def(buf_def));
                for VarRecordAttr { lhs, rhs } in pack.args.attrs.iter() {
                    self.desugar_nested_var_pattern(
                        new_module,
                        rhs,
                        &buf_name,
                        BufIndex::Record(lhs),
                    );
                }
            }
            VarPattern::Ident(_ident) => {
                let def = Def::new(Signature::Var(sig.clone()), body);
                new_module.push(Expr::Def(def));
            }
            _ => {}
        }
    }

    /// `{x; y}` -> `{x = x; y = y}`
    fn desugar_shortened_record(&self, mut module: Module) -> Module {
        let mut new = Module::with_capacity(module.len());
        while let Some(chunk) = module.lpop() {
            new.push(self.rec_desugar_shortened_record(chunk));
        }
        new
    }

    fn rec_desugar_shortened_record(&self, expr: Expr) -> Expr {
        match expr {
            Expr::Record(Record::Shortened(rec)) => {
                let rec = self.desugar_shortened_record_inner(rec);
                Expr::Record(Record::Normal(rec))
            }
            Expr::DataPack(pack) => {
                if let Record::Shortened(rec) = pack.args {
                    let class = self.rec_desugar_shortened_record(*pack.class);
                    let rec = self.desugar_shortened_record_inner(rec);
                    let args = Record::Normal(rec);
                    Expr::DataPack(DataPack::new(class, pack.connector, args))
                } else {
                    Expr::DataPack(pack)
                }
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    let (elems, _, _) = arr.elems.deconstruct();
                    let elems = elems
                        .into_iter()
                        .map(|elem| PosArg::new(self.rec_desugar_shortened_record(elem.expr)))
                        .collect();
                    let elems = Args::new(elems, vec![], None);
                    let arr = NormalArray::new(arr.l_sqbr, arr.r_sqbr, elems);
                    Expr::Array(Array::Normal(arr))
                }
                Array::WithLength(arr) => {
                    let elem = PosArg::new(self.rec_desugar_shortened_record(arr.elem.expr));
                    let len = self.rec_desugar_shortened_record(*arr.len);
                    let arr = ArrayWithLength::new(arr.l_sqbr, arr.r_sqbr, elem, len);
                    Expr::Array(Array::WithLength(arr))
                }
                Array::Comprehension(arr) => {
                    let elem = self.rec_desugar_shortened_record(*arr.elem);
                    let generators = arr
                        .generators
                        .into_iter()
                        .map(|(ident, gen)| (ident, self.rec_desugar_shortened_record(gen)))
                        .collect();
                    let guards = arr
                        .guards
                        .into_iter()
                        .map(|guard| self.rec_desugar_shortened_record(guard))
                        .collect();
                    let arr =
                        ArrayComprehension::new(arr.l_sqbr, arr.r_sqbr, elem, generators, guards);
                    Expr::Array(Array::Comprehension(arr))
                }
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    let (elems, _, paren) = tup.elems.deconstruct();
                    let elems = elems
                        .into_iter()
                        .map(|elem| PosArg::new(self.rec_desugar_shortened_record(elem.expr)))
                        .collect();
                    let new_tup = Args::new(elems, vec![], paren);
                    let tup = NormalTuple::new(new_tup);
                    Expr::Tuple(Tuple::Normal(tup))
                }
            },
            Expr::Set(set) => {
                todo!("{set}")
            }
            Expr::Dict(dict) => {
                todo!("{dict}")
            }
            Expr::BinOp(binop) => {
                let mut args = binop.args.into_iter();
                let lhs = self.rec_desugar_shortened_record(*args.next().unwrap());
                let rhs = self.rec_desugar_shortened_record(*args.next().unwrap());
                Expr::BinOp(BinOp::new(binop.op, lhs, rhs))
            }
            Expr::UnaryOp(unaryop) => {
                let mut args = unaryop.args.into_iter();
                let expr = self.rec_desugar_shortened_record(*args.next().unwrap());
                Expr::UnaryOp(UnaryOp::new(unaryop.op, expr))
            }
            Expr::Call(call) => {
                let obj = self.rec_desugar_shortened_record(*call.obj);
                let (pos_args, kw_args, paren) = call.args.deconstruct();
                let pos_args = pos_args
                    .into_iter()
                    .map(|arg| PosArg::new(self.rec_desugar_shortened_record(arg.expr)))
                    .collect();
                let kw_args = kw_args
                    .into_iter()
                    .map(|arg| {
                        let expr = self.rec_desugar_shortened_record(arg.expr);
                        KwArg::new(arg.keyword, arg.t_spec, expr) // TODO: t_spec
                    })
                    .collect();
                let args = Args::new(pos_args, kw_args, paren);
                Expr::Call(Call::new(obj, call.method_name, args))
            }
            Expr::Def(def) => {
                let mut chunks = vec![];
                for chunk in def.body.block.into_iter() {
                    chunks.push(self.rec_desugar_shortened_record(chunk));
                }
                let body = DefBody::new(def.body.op, Block::new(chunks), def.body.id);
                Expr::Def(Def::new(def.sig, body))
            }
            Expr::Lambda(lambda) => {
                let mut chunks = vec![];
                for chunk in lambda.body.into_iter() {
                    chunks.push(self.rec_desugar_shortened_record(chunk));
                }
                let body = Block::new(chunks);
                Expr::Lambda(Lambda::new(lambda.sig, lambda.op, body, lambda.id))
            }
            Expr::TypeAsc(tasc) => {
                let expr = self.rec_desugar_shortened_record(*tasc.expr);
                Expr::TypeAsc(TypeAscription::new(expr, tasc.op, tasc.t_spec))
            }
            Expr::Methods(method_defs) => {
                let mut new_defs = vec![];
                for def in method_defs.defs.into_iter() {
                    let mut chunks = vec![];
                    for chunk in def.body.block.into_iter() {
                        chunks.push(self.rec_desugar_shortened_record(chunk));
                    }
                    let body = DefBody::new(def.body.op, Block::new(chunks), def.body.id);
                    new_defs.push(Def::new(def.sig, body));
                }
                let new_defs = RecordAttrs::from(new_defs);
                Expr::Methods(Methods::new(method_defs.class, method_defs.vis, new_defs))
            }
            // TODO: Accessorにも一応レコードを入れられる
            other => other,
        }
    }

    fn desugar_shortened_record_inner(&self, rec: ShortenedRecord) -> NormalRecord {
        let mut attrs = vec![];
        for attr in rec.idents.into_iter() {
            let var = VarSignature::new(VarPattern::Ident(attr.clone()), None);
            let sig = Signature::Var(var);
            let body = DefBody::new(
                Token::from_str(TokenKind::Equal, "="),
                Block::new(vec![Expr::local(
                    attr.inspect(),
                    attr.ln_begin().unwrap(),
                    attr.col_begin().unwrap(),
                )]),
                DefId(get_hash(&(&sig, attr.inspect()))),
            );
            let def = Def::new(sig, body);
            attrs.push(def);
        }
        let attrs = RecordAttrs::new(attrs);
        NormalRecord::new(rec.l_brace, rec.r_brace, attrs)
    }

    fn desugar_self(&self, mut module: Module) -> Module {
        let mut new = Module::with_capacity(module.len());
        while let Some(chunk) = module.lpop() {
            new.push(self.rec_desugar_self(chunk));
        }
        new
    }

    fn rec_desugar_self(&self, _expr: Expr) -> Expr {
        todo!()
    }
    /// `F(I | I > 0)` -> `F(I: {I: Int | I > 0})`
    fn desugar_refinement_pattern(&self, _mod: Module) -> Module {
        todo!()
    }
}

impl Default for Desugarer {
    fn default() -> Self {
        Self::new()
    }
}
