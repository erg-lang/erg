//! Desugaring syntax sugars.
//!
//! Syntax sugarをdesugarする
//! e.g. Literal parameters, Multi assignment
//! 型チェックなどによる検証は行わない

use erg_common::consts::PYTHON_MODE;
use erg_common::error::Location;
use erg_common::fresh::FreshNameGenerator;
use erg_common::traits::{Locational, Stream};
use erg_common::{debug_power_assert, Str};
use erg_common::{enum_unwrap, get_hash, log, set};

use crate::ast::{
    Accessor, Args, Array, ArrayComprehension, ArrayTypeSpec, ArrayWithLength, BinOp, Block, Call,
    ClassAttr, ClassAttrs, ClassDef, ConstExpr, DataPack, Def, DefBody, DefId,
    DefaultParamSignature, Dict, Dummy, Expr, Identifier, KeyValue, KwArg, Lambda, LambdaSignature,
    Literal, Methods, MixedRecord, Module, NonDefaultParamSignature, NormalArray, NormalDict,
    NormalRecord, NormalSet, NormalTuple, ParamPattern, ParamRecordAttr, ParamTuplePattern, Params,
    PatchDef, PosArg, ReDef, Record, RecordAttrOrIdent, RecordAttrs, RecordTypeSpec, Set as astSet,
    SetComprehension, SetWithLength, Signature, SubrSignature, Tuple, TupleTypeSpec, TypeAppArgs,
    TypeAppArgsKind, TypeBoundSpecs, TypeSpec, TypeSpecWithOp, UnaryOp, VarName, VarPattern,
    VarRecordAttr, VarSignature, VisModifierSpec,
};
use crate::token::{Token, TokenKind, COLON, DOT};

pub fn symop_to_dname(op: &str) -> Option<&'static str> {
    match op {
        "`_+_`" => Some("__add__"),
        "`_-_`" => Some("__sub__"),
        "`*`" | "`cross`" => Some("__mul__"),
        "`/`" => Some("__div__"),
        "`//`" => Some("__floordiv__"),
        "`**`" => Some("__pow__"),
        "`%`" => Some("__mod__"),
        "`@`" | "`dot`" => Some("__matmul__"),
        "`&&`" => Some("__and__"),
        "`||`" => Some("__or__"),
        "`^`" => Some("__xor__"),
        "`==`" => Some("__eq__"),
        "`!=`" => Some("__ne__"),
        "`<`" => Some("__lt__"),
        "`<=`" => Some("__le__"),
        "`>`" => Some("__gt__"),
        "`>=`" => Some("__ge__"),
        "`<<`" => Some("__lshift__"),
        "`>>`" => Some("__rshift__"),
        "`+_`" => Some("__pos__"),
        "`-_`" => Some("__neg__"),
        "`~`" => Some("__invert__"),
        "`!`" => Some("__mutate__"),
        "`...`" => Some("__spread__"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BufIndex<'i> {
    Array(usize),
    Tuple(usize),
    Record(&'i Identifier),
}

#[derive(Debug)]
pub struct Desugarer {
    // _desugared: Set<Str>,
    var_gen: FreshNameGenerator,
}

impl Desugarer {
    pub fn new() -> Desugarer {
        Self {
            // _desugared: Set::default(),
            var_gen: FreshNameGenerator::new("desugar"),
        }
    }

    pub fn desugar(&mut self, module: Module) -> Module {
        log!(info "the desugaring process has started.");
        let module = self.desugar_multiple_pattern_def(module);
        let module = self.desugar_pattern_in_module(module);
        let module = Self::desugar_shortened_record(module);
        let module = Self::desugar_acc(module);
        let module = Self::desugar_operator(module);
        let module = Self::desugar_comprehension(module);
        log!(info "AST (desugared):\n{module}");
        log!(info "the desugaring process has completed.");
        module
    }

    pub fn desugar_simple_expr(expr: Expr) -> Expr {
        let expr = Self::rec_desugar_shortened_record(expr);
        let expr = Self::rec_desugar_lambda_pattern(&mut Desugarer::new(), expr);
        Self::rec_desugar_acc(expr)
    }

    fn desugar_all_chunks(module: Module, desugar: impl Fn(Expr) -> Expr) -> Module {
        module.into_iter().map(desugar).collect()
    }

    fn desugar_args(mut desugar: impl FnMut(Expr) -> Expr, args: Args) -> Args {
        let (pos_args, var_args, kw_args, paren) = args.deconstruct();
        let pos_args = pos_args
            .into_iter()
            .map(|arg| PosArg::new(desugar(arg.expr)))
            .collect();
        let var_args = var_args.map(|arg| PosArg::new(desugar(arg.expr)));
        let kw_args = kw_args
            .into_iter()
            .map(|arg| {
                KwArg::new(arg.keyword, arg.t_spec, desugar(arg.expr)) // TODO: t_spec
            })
            .collect();
        Args::new(pos_args, var_args, kw_args, paren)
    }

    fn perform_desugar_acc(mut desugar: impl FnMut(Expr) -> Expr, acc: Accessor) -> Accessor {
        match acc {
            Accessor::Ident(ident) => Accessor::Ident(ident),
            Accessor::Attr(attr) => desugar(*attr.obj).attr(attr.ident),
            Accessor::TupleAttr(tup) => {
                let obj = desugar(*tup.obj);
                obj.tuple_attr(tup.index)
            }
            Accessor::Subscr(sub) => {
                let obj = desugar(*sub.obj);
                let index = desugar(*sub.index);
                obj.subscr(index, sub.r_sqbr)
            }
            Accessor::TypeApp(tapp) => {
                let obj = desugar(*tapp.obj);
                let args = match tapp.type_args.args {
                    TypeAppArgsKind::Args(args) => {
                        TypeAppArgsKind::Args(Self::desugar_args(desugar, args))
                    }
                    other => other,
                };
                let type_args =
                    TypeAppArgs::new(tapp.type_args.l_vbar, args, tapp.type_args.r_vbar);
                obj.type_app(type_args)
            }
        }
    }

    fn perform_desugar(mut desugar: impl FnMut(Expr) -> Expr, expr: Expr) -> Expr {
        match expr {
            Expr::Literal(_) => expr,
            Expr::Record(record) => match record {
                Record::Normal(rec) => {
                    let mut new_attrs = vec![];
                    for attr in rec.attrs {
                        new_attrs.push(enum_unwrap!(desugar(Expr::Def(attr)), Expr::Def));
                    }
                    Expr::Record(Record::Normal(NormalRecord::new(
                        rec.l_brace,
                        rec.r_brace,
                        RecordAttrs::new(new_attrs),
                    )))
                }
                Record::Mixed(mixed) => {
                    let mut new_attrs = vec![];
                    for attr in mixed.attrs {
                        match attr {
                            RecordAttrOrIdent::Attr(attr) => {
                                let attr = RecordAttrOrIdent::Attr(enum_unwrap!(
                                    desugar(Expr::Def(attr)),
                                    Expr::Def
                                ));
                                new_attrs.push(attr);
                            }
                            RecordAttrOrIdent::Ident(ident) => {
                                new_attrs.push(RecordAttrOrIdent::Ident(ident));
                            }
                        }
                    }
                    Expr::Record(Record::Mixed(MixedRecord::new(
                        mixed.l_brace,
                        mixed.r_brace,
                        new_attrs,
                    )))
                }
            },
            Expr::DataPack(pack) => {
                let class = desugar(*pack.class);
                let Expr::Record(args) = desugar(Expr::Record(pack.args)) else {
                    unreachable!()
                };
                Expr::DataPack(DataPack::new(class, pack.connector, args))
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    let (elems, ..) = arr.elems.deconstruct();
                    let elems = elems
                        .into_iter()
                        .map(|elem| PosArg::new(desugar(elem.expr)))
                        .collect();
                    let elems = Args::pos_only(elems, None);
                    let arr = NormalArray::new(arr.l_sqbr, arr.r_sqbr, elems);
                    Expr::Array(Array::Normal(arr))
                }
                Array::WithLength(arr) => {
                    let elem = PosArg::new(desugar(arr.elem.expr));
                    let len = desugar(*arr.len);
                    let arr = ArrayWithLength::new(arr.l_sqbr, arr.r_sqbr, elem, len);
                    Expr::Array(Array::WithLength(arr))
                }
                Array::Comprehension(arr) => {
                    let layout = arr.layout.map(|ex| desugar(*ex));
                    let generators = arr
                        .generators
                        .into_iter()
                        .map(|(ident, gen)| (ident, desugar(gen)))
                        .collect();
                    let guard = arr.guard.map(|ex| desugar(*ex));
                    let arr =
                        ArrayComprehension::new(arr.l_sqbr, arr.r_sqbr, layout, generators, guard);
                    Expr::Array(Array::Comprehension(arr))
                }
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    let (elems, _, _, paren) = tup.elems.deconstruct();
                    let elems = elems
                        .into_iter()
                        .map(|elem| PosArg::new(desugar(elem.expr)))
                        .collect();
                    let new_tup = Args::pos_only(elems, paren);
                    let tup = NormalTuple::new(new_tup);
                    Expr::Tuple(Tuple::Normal(tup))
                }
            },
            Expr::Set(set) => match set {
                astSet::Normal(set) => {
                    let (elems, ..) = set.elems.deconstruct();
                    let elems = elems
                        .into_iter()
                        .map(|elem| PosArg::new(desugar(elem.expr)))
                        .collect();
                    let elems = Args::pos_only(elems, None);
                    let set = NormalSet::new(set.l_brace, set.r_brace, elems);
                    Expr::Set(astSet::Normal(set))
                }
                astSet::WithLength(set) => {
                    let elem = PosArg::new(desugar(set.elem.expr));
                    let len = desugar(*set.len);
                    let set = SetWithLength::new(set.l_brace, set.r_brace, elem, len);
                    Expr::Set(astSet::WithLength(set))
                }
                astSet::Comprehension(set) => {
                    let elem = set.layout.map(|ex| desugar(*ex));
                    let mut new_generators = vec![];
                    for (ident, gen) in set.generators.into_iter() {
                        new_generators.push((ident, desugar(gen)));
                    }
                    let new_guard = set.guard.map(|ex| desugar(*ex));
                    let set = SetComprehension::new(
                        set.l_brace,
                        set.r_brace,
                        elem,
                        new_generators,
                        new_guard,
                    );
                    Expr::Set(astSet::Comprehension(set))
                }
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dic) => {
                    let new_kvs = dic
                        .kvs
                        .into_iter()
                        .map(|elem| {
                            let key = desugar(elem.key);
                            let value = desugar(elem.value);
                            KeyValue::new(key, value)
                        })
                        .collect();
                    let tup = NormalDict::new(dic.l_brace, dic.r_brace, new_kvs);
                    Expr::Dict(Dict::Normal(tup))
                }
                _ => todo!("dict comprehension"),
            },
            Expr::BinOp(binop) => {
                let mut args = binop.args.into_iter();
                let lhs = desugar(*args.next().unwrap());
                let rhs = desugar(*args.next().unwrap());
                Expr::BinOp(BinOp::new(binop.op, lhs, rhs))
            }
            Expr::UnaryOp(unaryop) => {
                let mut args = unaryop.args.into_iter();
                let expr = desugar(*args.next().unwrap());
                Expr::UnaryOp(UnaryOp::new(unaryop.op, expr))
            }
            Expr::Call(call) => {
                let obj = desugar(*call.obj);
                let args = Self::desugar_args(desugar, call.args);
                Expr::Call(Call::new(obj, call.attr_name, args))
            }
            Expr::Def(mut def) => {
                let mut chunks = vec![];
                for chunk in def.body.block.into_iter() {
                    chunks.push(desugar(chunk));
                }
                if let Some(t_op) = def.sig.t_spec_op_mut() {
                    *t_op.t_spec_as_expr = desugar(*t_op.t_spec_as_expr.clone());
                }
                if let Signature::Subr(mut subr) = def.sig {
                    subr.params = Self::perform_desugar_params(desugar, subr.params);
                    def.sig = Signature::Subr(subr);
                }
                let body = DefBody::new(def.body.op, Block::new(chunks), def.body.id);
                Expr::Def(Def::new(def.sig, body))
            }
            Expr::ClassDef(class_def) => {
                let Expr::Def(def) = desugar(Expr::Def(class_def.def)) else {
                    unreachable!()
                };
                let methods = class_def
                    .methods_list
                    .into_iter()
                    .map(|method| enum_unwrap!(desugar(Expr::Methods(method)), Expr::Methods))
                    .collect();
                Expr::ClassDef(ClassDef::new(def, methods))
            }
            Expr::PatchDef(class_def) => {
                let Expr::Def(def) = desugar(Expr::Def(class_def.def)) else {
                    unreachable!()
                };
                let methods = class_def
                    .methods_list
                    .into_iter()
                    .map(|method| enum_unwrap!(desugar(Expr::Methods(method)), Expr::Methods))
                    .collect();
                Expr::PatchDef(PatchDef::new(def, methods))
            }
            Expr::ReDef(redef) => {
                let expr = desugar(*redef.expr);
                let attr = Self::perform_desugar_acc(desugar, redef.attr);
                Expr::ReDef(ReDef::new(attr, expr))
            }
            Expr::Lambda(mut lambda) => {
                let mut chunks = vec![];
                for chunk in lambda.body.into_iter() {
                    chunks.push(desugar(chunk));
                }
                if let Some(t_op) = &mut lambda.sig.return_t_spec {
                    *t_op.t_spec_as_expr = desugar(*t_op.t_spec_as_expr.clone());
                }
                lambda.sig.params = Self::perform_desugar_params(desugar, lambda.sig.params);
                let body = Block::new(chunks);
                Expr::Lambda(Lambda::new(lambda.sig, lambda.op, body, lambda.id))
            }
            Expr::TypeAscription(tasc) => {
                let expr = desugar(*tasc.expr);
                let t_spec_as_expr = desugar(*tasc.t_spec.t_spec_as_expr);
                let t_spec =
                    TypeSpecWithOp::new(tasc.t_spec.op, tasc.t_spec.t_spec, t_spec_as_expr);
                expr.type_asc_expr(t_spec)
            }
            Expr::Methods(method_defs) => {
                let mut new_attrs = vec![];
                for attr in method_defs.attrs.into_iter() {
                    let mut chunks = vec![];
                    match attr {
                        ClassAttr::Def(def) => {
                            for chunk in def.body.block.into_iter() {
                                chunks.push(desugar(chunk));
                            }
                            let body = DefBody::new(def.body.op, Block::new(chunks), def.body.id);
                            new_attrs.push(ClassAttr::Def(Def::new(def.sig, body)));
                        }
                        ClassAttr::Decl(decl) => {
                            let expr = desugar(*decl.expr);
                            let t_spec_as_expr = desugar(*decl.t_spec.t_spec_as_expr);
                            let t_spec = TypeSpecWithOp::new(
                                decl.t_spec.op,
                                decl.t_spec.t_spec,
                                t_spec_as_expr,
                            );
                            new_attrs.push(ClassAttr::Decl(expr.type_asc(t_spec)));
                        }
                        ClassAttr::Doc(doc) => {
                            new_attrs.push(ClassAttr::Doc(doc));
                        }
                    }
                }
                let new_attrs = ClassAttrs::from(new_attrs);
                Expr::Methods(Methods::new(
                    method_defs.class,
                    *method_defs.class_as_expr,
                    method_defs.vis,
                    new_attrs,
                ))
            }
            Expr::Accessor(acc) => Expr::Accessor(Self::perform_desugar_acc(desugar, acc)),
            Expr::Dummy(exprs) => {
                let loc = exprs.loc;
                let mut chunks = vec![];
                for chunk in exprs.into_iter() {
                    chunks.push(desugar(chunk));
                }
                Expr::Dummy(Dummy::new(loc, chunks))
            }
        }
    }

    fn perform_desugar_params(mut desugar: impl FnMut(Expr) -> Expr, mut params: Params) -> Params {
        let mut non_defaults = vec![];
        for mut non_default in params.non_defaults.into_iter() {
            non_default.t_spec = non_default.t_spec.map(|t_spec| {
                TypeSpecWithOp::new(t_spec.op, t_spec.t_spec, desugar(*t_spec.t_spec_as_expr))
            });
            non_defaults.push(non_default);
        }
        params.var_params = params.var_params.map(|mut var_params| {
            var_params.t_spec = var_params.t_spec.map(|t_spec| {
                TypeSpecWithOp::new(t_spec.op, t_spec.t_spec, desugar(*t_spec.t_spec_as_expr))
            });
            var_params
        });
        let mut defaults = vec![];
        for mut default in params.defaults.into_iter() {
            let default_val = desugar(default.default_val);
            default.sig.t_spec = default.sig.t_spec.map(|t_spec| {
                TypeSpecWithOp::new(t_spec.op, t_spec.t_spec, desugar(*t_spec.t_spec_as_expr))
            });
            defaults.push(DefaultParamSignature {
                default_val,
                ..default
            });
        }
        params.non_defaults = non_defaults;
        params.defaults = defaults;
        params
    }

    /// `fib 0 = 0; fib 1 = 1; fib n = fib(n-1) + fib(n-2)`
    /// -> `fib n = match n, (0 -> 0), (1 -> 1), n -> fib(n-1) + fib(n-2)`
    fn desugar_multiple_pattern_def(&self, module: Module) -> Module {
        let mut new = Module::with_capacity(module.len());
        for chunk in module.into_iter() {
            match chunk {
                Expr::Def(def) if def.is_subr() => {
                    if let Some(Expr::Def(previous)) = new.last() {
                        if previous.is_subr() && previous.sig.name_as_str() == def.sig.name_as_str()
                        {
                            let Some(Expr::Def(previous)) = new.pop() else {
                                unreachable!()
                            };
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
                            let params = match &call.args.pos_args().iter().next().unwrap().expr {
                                Expr::Tuple(Tuple::Normal(tup)) => {
                                    let mut params = vec![];
                                    for arg in tup.elems.pos_args().iter() {
                                        match &arg.expr {
                                            Expr::Accessor(Accessor::Ident(ident)) => {
                                                let param_name = ident.inspect();
                                                let col_begin = name.col_end().unwrap_or(0) + 1;
                                                let col_end =
                                                    col_begin + param_name.chars().count() as u32;
                                                let param = VarName::new(Token::new_fake(
                                                    TokenKind::Symbol,
                                                    param_name,
                                                    name.ln_begin().unwrap_or(1),
                                                    col_begin,
                                                    col_end,
                                                ));
                                                let param = NonDefaultParamSignature::new(
                                                    ParamPattern::VarName(param),
                                                    None,
                                                );
                                                params.push(param);
                                            }
                                            _ => unreachable!(),
                                        }
                                    }
                                    Params::new(params, None, vec![], None)
                                }
                                Expr::Accessor(Accessor::Ident(ident)) => {
                                    let param_name = ident.inspect();
                                    let col_begin = name.col_end().unwrap_or(0) + 1; // HACK: `(name) %x = ...`という形を想定
                                    let col_end = col_begin + param_name.chars().count() as u32;
                                    let param = VarName::new(Token::new_fake(
                                        TokenKind::Symbol,
                                        param_name,
                                        name.ln_begin().unwrap_or(1),
                                        col_begin,
                                        col_end,
                                    ));
                                    let param = NonDefaultParamSignature::new(
                                        ParamPattern::VarName(param),
                                        None,
                                    );
                                    Params::single(param)
                                }
                                _ => unreachable!(),
                            };
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

    fn add_arg_to_match_call(&self, mut previous: Def, def: Def) -> (Call, Option<TypeSpecWithOp>) {
        let op = Token::from_str(TokenKind::FuncArrow, "->");
        let Expr::Call(mut call) = previous.body.block.remove(0) else {
            unreachable!()
        };
        let Signature::Subr(sig) = def.sig else {
            unreachable!()
        };
        let return_t_spec = sig.return_t_spec;
        let first_arg = sig.params.non_defaults.first().unwrap();
        // 最後の定義の引数名を関数全体の引数名にする
        if let Some(name) = first_arg.inspect() {
            call.args.remove_pos(0);
            let arg = PosArg::new(Expr::local(
                name,
                first_arg.ln_begin().unwrap_or(1),
                first_arg.col_begin().unwrap_or(0),
                first_arg.col_end().unwrap_or(0),
            ));
            call.args.insert_pos(0, arg);
        }
        // f(x, y, z) = ... => match x, ((x, y, z),) -> ...
        let params = if sig.params.len() == 1 {
            sig.params
        } else {
            let pat = ParamPattern::Tuple(ParamTuplePattern::new(sig.params));
            Params::single(NonDefaultParamSignature::new(pat, None))
        };
        let sig = LambdaSignature::new(params, return_t_spec.clone(), sig.bounds);
        let new_branch = Lambda::new(sig, op, def.body.block, def.body.id);
        call.args.push_pos(PosArg::new(Expr::Lambda(new_branch)));
        (call, return_t_spec)
    }

    // TODO: procedural match
    fn gen_match_call(&self, previous: Def, def: Def) -> (Call, Option<TypeSpecWithOp>) {
        let op = Token::from_str(TokenKind::FuncArrow, "->");
        let Signature::Subr(prev_sig) = previous.sig else {
            unreachable!()
        };
        let params_len = prev_sig.params.len();
        let params = if params_len == 1 {
            prev_sig.params
        } else {
            let pat = ParamPattern::Tuple(ParamTuplePattern::new(prev_sig.params));
            Params::single(NonDefaultParamSignature::new(pat, None))
        };
        let match_symbol = Expr::static_local("match");
        let sig = LambdaSignature::new(params, prev_sig.return_t_spec, prev_sig.bounds);
        let first_branch = Lambda::new(sig, op.clone(), previous.body.block, previous.body.id);
        let Signature::Subr(sig) = def.sig else {
            unreachable!()
        };
        let params = if sig.params.len() == 1 {
            sig.params
        } else {
            let pat = ParamPattern::Tuple(ParamTuplePattern::new(sig.params));
            Params::single(NonDefaultParamSignature::new(pat, None))
        };
        let return_t_spec = sig.return_t_spec;
        let sig = LambdaSignature::new(params, return_t_spec.clone(), sig.bounds);
        let second_branch = Lambda::new(sig, op, def.body.block, def.body.id);
        let first_arg = if params_len == 1 {
            Expr::dummy_local(&self.var_gen.fresh_varname())
        } else {
            let args = (0..params_len)
                .map(|_| PosArg::new(Expr::dummy_local(&self.var_gen.fresh_varname())));
            Expr::Tuple(Tuple::Normal(NormalTuple::new(Args::pos_only(
                args.collect(),
                None,
            ))))
        };
        let args = Args::pos_only(
            vec![
                PosArg::new(first_arg), // dummy argument, will be removed in line 56
                PosArg::new(Expr::Lambda(first_branch)),
                PosArg::new(Expr::Lambda(second_branch)),
            ],
            None,
        );
        let call = match_symbol.call(args);
        (call, return_t_spec)
    }

    /// `f 0 = 1` -> `f _: {0} = 1`
    fn _desugar_literal_pattern(&self, _mod: Module) -> Module {
        todo!()
    }

    fn gen_buf_name_and_sig(
        &mut self,
        loc: Location,
        t_spec: Option<TypeSpecWithOp>,
    ) -> (Str, Signature) {
        let buf_name = self.var_gen.fresh_varname();
        let buf_sig = Signature::Var(VarSignature::new(
            VarPattern::Ident(Identifier::private_with_loc(Str::rc(&buf_name), loc)),
            t_spec,
        ));
        (buf_name, buf_sig)
    }

    fn gen_buf_nd_param(&mut self, loc: Location) -> (Str, ParamPattern) {
        let buf_name = self.var_gen.fresh_varname();
        let pat = ParamPattern::VarName(VarName::from_str_and_loc(buf_name.clone(), loc));
        (buf_name, pat)
    }

    fn rec_desugar_lambda_pattern(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Lambda(mut lambda) => {
                self.desugar_params_patterns(&mut lambda.sig.params, &mut lambda.body);
                lambda.body = self.desugar_pattern_in_block(lambda.body);
                Expr::Lambda(lambda)
            }
            expr => Self::perform_desugar(|ex| self.rec_desugar_lambda_pattern(ex), expr),
        }
    }

    fn desugar_pattern_in_module(&mut self, module: Module) -> Module {
        // https://github.com/rust-lang/rust-clippy/issues/11300
        #[allow(clippy::useless_conversion)]
        Module::new(self.desugar_pattern(module.into_iter()))
    }

    fn desugar_pattern_in_block(&mut self, block: Block) -> Block {
        #[allow(clippy::useless_conversion)]
        Block::new(self.desugar_pattern(block.into_iter()))
    }

    fn desugar_def_pattern(&mut self, def: Def, new: &mut Vec<Expr>) {
        match def {
            Def {
                sig: Signature::Var(mut v),
                body,
            } => match &v.pat {
                VarPattern::Tuple(tup) => {
                    let (buf_name, buf_sig) = self.gen_buf_name_and_sig(v.loc(), v.t_spec);
                    let block = body
                        .block
                        .into_iter()
                        .map(|ex| self.rec_desugar_lambda_pattern(ex))
                        .collect();
                    let block = self.desugar_pattern_in_block(block);
                    let buf_def = Def::new(buf_sig, DefBody::new(body.op, block, body.id));
                    new.push(Expr::Def(buf_def));
                    for (n, elem) in tup.elems.iter().enumerate() {
                        self.desugar_nested_var_pattern(new, elem, &buf_name, BufIndex::Tuple(n));
                    }
                }
                VarPattern::Array(arr) => {
                    let (buf_name, buf_sig) = self.gen_buf_name_and_sig(v.loc(), v.t_spec);
                    let block = body
                        .block
                        .into_iter()
                        .map(|ex| self.rec_desugar_lambda_pattern(ex))
                        .collect();
                    let block = self.desugar_pattern_in_block(block);
                    let buf_def = Def::new(buf_sig, DefBody::new(body.op, block, body.id));
                    new.push(Expr::Def(buf_def));
                    for (n, elem) in arr.elems.iter().enumerate() {
                        self.desugar_nested_var_pattern(new, elem, &buf_name, BufIndex::Array(n));
                    }
                }
                VarPattern::Record(rec) => {
                    let (buf_name, buf_sig) = self.gen_buf_name_and_sig(v.loc(), v.t_spec);
                    let block = body
                        .block
                        .into_iter()
                        .map(|ex| self.rec_desugar_lambda_pattern(ex))
                        .collect();
                    let block = self.desugar_pattern_in_block(block);
                    let buf_def = Def::new(buf_sig, DefBody::new(body.op, block, body.id));
                    new.push(Expr::Def(buf_def));
                    for VarRecordAttr { lhs, rhs } in rec.attrs.iter() {
                        self.desugar_nested_var_pattern(new, rhs, &buf_name, BufIndex::Record(lhs));
                    }
                }
                VarPattern::DataPack(pack) => {
                    let t_spec =
                        TypeSpecWithOp::new(COLON, pack.class.clone(), *pack.class_as_expr.clone());
                    let (buf_name, buf_sig) = self.gen_buf_name_and_sig(
                        v.loc(),
                        Some(t_spec), // TODO: これだとvの型指定の意味がなくなる
                    );
                    let block = body
                        .block
                        .into_iter()
                        .map(|ex| self.rec_desugar_lambda_pattern(ex))
                        .collect();
                    let block = self.desugar_pattern_in_block(block);
                    let buf_def = Def::new(buf_sig, DefBody::new(body.op, block, body.id));
                    new.push(Expr::Def(buf_def));
                    for VarRecordAttr { lhs, rhs } in pack.args.attrs.iter() {
                        self.desugar_nested_var_pattern(new, rhs, &buf_name, BufIndex::Record(lhs));
                    }
                }
                VarPattern::Ident(_) | VarPattern::Discard(_) => {
                    if let VarPattern::Ident(ident) = v.pat {
                        v.pat = VarPattern::Ident(Self::desugar_ident(ident));
                    }
                    let block = body
                        .block
                        .into_iter()
                        .map(|ex| self.rec_desugar_lambda_pattern(ex))
                        .collect();
                    let block = self.desugar_pattern_in_block(block);
                    let body = DefBody::new(body.op, block, body.id);
                    let def = Def::new(Signature::Var(v), body);
                    new.push(Expr::Def(def));
                }
            },
            Def {
                sig: Signature::Subr(mut subr),
                mut body,
            } => {
                subr.ident = Self::desugar_ident(subr.ident);
                self.desugar_params_patterns(&mut subr.params, &mut body.block);
                let block = body
                    .block
                    .into_iter()
                    .map(|ex| self.rec_desugar_lambda_pattern(ex))
                    .collect();
                let block = self.desugar_pattern_in_block(block);
                let body = DefBody::new(body.op, block, body.id);
                let def = Def::new(Signature::Subr(subr), body);
                new.push(Expr::Def(def));
            }
        }
    }

    // TODO: nested function pattern
    /// `[i, j] = [1, 2]` -> `i = 1; j = 2`
    /// `[i, j] = l` -> `i = l[0]; j = l[1]`
    /// `[i, [j, k]] = l` -> `i = l[0]; j = l[1][0]; k = l[1][1]`
    /// `(i, j) = t` -> `i = t.0; j = t.1`
    /// `{i; j} = s` -> `i = s.i; j = s.j`
    fn desugar_pattern<I>(&mut self, chunks: I) -> Vec<Expr>
    where
        I: IntoIterator<Item = Expr> + ExactSizeIterator,
    {
        let mut new = Vec::with_capacity(chunks.len());
        for chunk in chunks.into_iter() {
            match chunk {
                Expr::Def(def) => {
                    self.desugar_def_pattern(def, &mut new);
                }
                Expr::Methods(methods) => {
                    let mut new_attrs = Vec::with_capacity(methods.attrs.len());
                    for attr in methods.attrs.into_iter() {
                        match attr {
                            ClassAttr::Def(def) => {
                                let mut new = vec![];
                                self.desugar_def_pattern(def, &mut new);
                                let Expr::Def(def) = new.remove(0) else {
                                    todo!("{new:?}")
                                };
                                new_attrs.push(ClassAttr::Def(def));
                            }
                            _ => {
                                new_attrs.push(attr);
                            }
                        }
                    }
                    let methods = Methods::new(
                        methods.class,
                        *methods.class_as_expr,
                        methods.vis,
                        ClassAttrs::from(new_attrs),
                    );
                    new.push(Expr::Methods(methods));
                }
                Expr::Dummy(dummy) => {
                    let loc = dummy.loc;
                    let new_dummy = self.desugar_pattern(dummy.into_iter());
                    new.push(Expr::Dummy(Dummy::new(loc, new_dummy)));
                }
                other => {
                    new.push(self.rec_desugar_lambda_pattern(other));
                }
            }
        }
        new
    }

    fn desugar_params_patterns(&mut self, params: &mut Params, body: &mut Block) {
        for param in params.non_defaults.iter_mut() {
            self.desugar_nd_param(param, body);
        }
        if let Some(var_params) = params.var_params.as_mut() {
            self.desugar_nd_param(var_params, body);
        }
        for param in params.defaults.iter_mut() {
            self.desugar_nd_param(&mut param.sig, body);
        }
    }

    fn desugar_nested_var_pattern(
        &mut self,
        new_module: &mut Vec<Expr>,
        sig: &VarSignature,
        buf_name: &str,
        buf_index: BufIndex,
    ) {
        let obj = Expr::local(
            buf_name,
            sig.ln_begin().unwrap_or(1),
            sig.col_begin().unwrap_or(0),
            sig.col_end().unwrap_or(0),
        );
        let acc = match buf_index {
            BufIndex::Tuple(n) => obj.tuple_attr(Literal::nat(n, sig.ln_begin().unwrap_or(1))),
            BufIndex::Array(n) => {
                let r_brace = Token::new(
                    TokenKind::RBrace,
                    "]",
                    sig.ln_begin().unwrap_or(1),
                    sig.col_begin().unwrap_or(0),
                );
                obj.subscr(
                    Expr::Literal(Literal::nat(n, sig.ln_begin().unwrap_or(1))),
                    r_brace,
                )
            }
            BufIndex::Record(attr) => {
                let attr = Identifier::new(VisModifierSpec::Auto, attr.name.clone());
                obj.attr(attr)
            }
        };
        let id = DefId(get_hash(&(&acc, buf_name)));
        let block = Block::new(vec![Expr::Accessor(acc)]);
        let op = Token::from_str(TokenKind::Assign, "=");
        let body = DefBody::new(op, block, id);
        match &sig.pat {
            VarPattern::Tuple(tup) => {
                let (buf_name, buf_sig) = self.gen_buf_name_and_sig(sig.loc(), None);
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
                let (buf_name, buf_sig) = self.gen_buf_name_and_sig(sig.loc(), None);
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
                let (buf_name, buf_sig) = self.gen_buf_name_and_sig(sig.loc(), None);
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
                let t_spec =
                    TypeSpecWithOp::new(COLON, pack.class.clone(), *pack.class_as_expr.clone());
                let (buf_name, buf_sig) = self.gen_buf_name_and_sig(sig.loc(), Some(t_spec));
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
            VarPattern::Ident(_) | VarPattern::Discard(_) => {
                let def = Def::new(Signature::Var(sig.clone()), body);
                new_module.push(Expr::Def(def));
            }
        }
    }

    /// `{x; y}` -> `{x = x; y = y}`
    fn desugar_shortened_record(module: Module) -> Module {
        Self::desugar_all_chunks(module, Self::rec_desugar_shortened_record)
    }

    fn rec_desugar_shortened_record(expr: Expr) -> Expr {
        match expr {
            Expr::Record(Record::Mixed(record)) => {
                let rec = Self::desugar_shortened_record_inner(record);
                Expr::Record(Record::Normal(rec))
            }
            Expr::DataPack(pack) => {
                if let Record::Mixed(rec) = pack.args {
                    let class = Self::rec_desugar_shortened_record(*pack.class);
                    let rec = Self::desugar_shortened_record_inner(rec);
                    let args = Record::Normal(rec);
                    Expr::DataPack(DataPack::new(class, pack.connector, args))
                } else {
                    Expr::DataPack(pack)
                }
            }
            expr => Self::perform_desugar(Self::rec_desugar_shortened_record, expr),
        }
    }

    pub fn desugar_shortened_record_inner(record: MixedRecord) -> NormalRecord {
        let attrs = record
            .attrs
            .into_iter()
            .map(|attr_or_ident| match attr_or_ident {
                RecordAttrOrIdent::Attr(def) => def,
                RecordAttrOrIdent::Ident(ident) => {
                    let var = VarSignature::new(VarPattern::Ident(ident.clone()), None);
                    let sig = Signature::Var(var);
                    let body = DefBody::new(
                        Token::from_str(TokenKind::Assign, "="),
                        Block::new(vec![Expr::local(
                            ident.inspect(),
                            ident.ln_begin().unwrap_or(1),
                            ident.col_begin().unwrap_or(0),
                            ident.col_end().unwrap_or(0),
                        )]),
                        DefId(get_hash(&(&sig, ident.inspect()))),
                    );
                    Def::new(sig, body)
                }
            })
            .collect();
        let attrs = RecordAttrs::new(attrs);
        NormalRecord::new(record.l_brace, record.r_brace, attrs)
    }

    fn dummy_array_expr(len: Literal) -> Expr {
        let l_sqbr = Token {
            content: "[".into(),
            kind: TokenKind::LSqBr,
            ..len.token
        };
        let r_sqbr = Token {
            content: "]".into(),
            kind: TokenKind::RSqBr,
            ..len.token
        };
        let elem = Expr::local("Obj", l_sqbr.lineno, l_sqbr.col_begin, l_sqbr.col_end);
        let array = Array::WithLength(ArrayWithLength::new(
            l_sqbr,
            r_sqbr,
            PosArg::new(elem),
            Expr::Literal(len),
        ));
        Expr::Array(array)
    }

    fn dummy_set_expr(lit: Literal) -> Expr {
        let l_brace = Token {
            content: "{".into(),
            kind: TokenKind::LBrace,
            ..lit.token
        };
        let r_brace = Token {
            content: "}".into(),
            kind: TokenKind::RBrace,
            ..lit.token
        };
        let args = Args::single(PosArg::new(Expr::Literal(lit)));
        Expr::from(NormalSet::new(l_brace, r_brace, args))
    }

    /// ```erg
    /// f [x, y] =
    ///     ...
    /// ```
    /// ↓
    /// ```erg
    /// f %1 =
    ///    x = %1[0]
    ///    y = %1[1]
    ///    ...
    /// ```
    /// ```erg
    /// f [x, [y, z]] =
    ///     ...
    /// ```
    /// ↓
    /// ```erg
    /// f %1 =
    ///    x = %1[0]
    ///    %2 = %1[1]
    ///    y = %2[0]
    ///    z = %2[1]
    ///    ...
    /// ```
    /// ```erg
    /// f 1, 2 =
    ///     ...
    /// ```
    /// ↓
    /// ```erg
    /// f _: {1}, _: {2} = ...
    /// ```
    fn desugar_nd_param(&mut self, param: &mut NonDefaultParamSignature, body: &mut Block) {
        let mut insertion_idx = 0;
        let line = param.ln_begin().unwrap_or(1);
        match &mut param.pat {
            ParamPattern::VarName(_v) => {}
            ParamPattern::Lit(l) => {
                let lit = l.clone();
                param.pat = ParamPattern::Discard(Token::new_fake(
                    TokenKind::UBar,
                    "_",
                    l.ln_begin().unwrap_or(1),
                    l.col_begin().unwrap_or(0),
                    l.col_end().unwrap_or(0),
                ));
                let l_brace = Token {
                    content: "{".into(),
                    kind: TokenKind::LBrace,
                    ..lit.token
                };
                let r_brace = Token {
                    content: "}".into(),
                    kind: TokenKind::RBrace,
                    ..lit.token
                };
                let t_spec = TypeSpec::enum_t_spec(vec![lit.clone()]);
                let args = Args::single(PosArg::new(Expr::Literal(lit)));
                let t_spec_as_expr = Expr::from(NormalSet::new(l_brace, r_brace, args));
                let t_spec = TypeSpecWithOp::new(COLON, t_spec, t_spec_as_expr);
                param.t_spec = Some(t_spec);
            }
            ParamPattern::Tuple(tup) => {
                let (buf_name, buf_param) = self.gen_buf_nd_param(tup.loc());
                let mut ty_specs = vec![];
                let mut ty_exprs = vec![];
                for (n, elem) in tup.elems.non_defaults.iter_mut().enumerate() {
                    insertion_idx = self.desugar_nested_param_pattern(
                        body,
                        elem,
                        &buf_name,
                        BufIndex::Tuple(n),
                        insertion_idx,
                    );
                    let infer = Token::new_fake(TokenKind::Try, "?", line, 0, 0);
                    let ty_expr = elem
                        .t_spec
                        .as_ref()
                        .map(|ts| *ts.t_spec_as_expr.clone())
                        .unwrap_or(Expr::local(
                            "Obj",
                            infer.lineno,
                            infer.col_begin,
                            infer.col_end,
                        ));
                    ty_exprs.push(PosArg::new(ty_expr));
                    ty_specs.push(
                        elem.t_spec
                            .as_ref()
                            .map(|ts| ts.t_spec.clone())
                            .unwrap_or(TypeSpec::Infer(infer))
                            .clone(),
                    );
                }
                if param.t_spec.is_none() {
                    let t_spec =
                        TypeSpec::Tuple(TupleTypeSpec::new(tup.elems.parens.clone(), ty_specs));
                    let t_spec_as_expr = Expr::from(NormalTuple::new(Args::pos_only(
                        ty_exprs,
                        tup.elems.parens.clone(),
                    )));
                    param.t_spec = Some(TypeSpecWithOp::new(COLON, t_spec, t_spec_as_expr));
                }
                param.pat = buf_param;
            }
            ParamPattern::Array(arr) => {
                let expr = Expr::try_from(&*arr);
                let (buf_name, buf_param) = self.gen_buf_nd_param(arr.loc());
                for (n, elem) in arr.elems.non_defaults.iter_mut().enumerate() {
                    insertion_idx = self.desugar_nested_param_pattern(
                        body,
                        elem,
                        &buf_name,
                        BufIndex::Array(n),
                        insertion_idx,
                    );
                }
                if param.t_spec.is_none() {
                    let len = arr.elems.non_defaults.len();
                    let len = Literal::new(Token::new_fake(
                        TokenKind::NatLit,
                        len.to_string(),
                        line,
                        0,
                        0,
                    ));
                    let infer = Token::new_fake(TokenKind::Try, "?", line, 0, 0);
                    let t_spec = ArrayTypeSpec::new(
                        TypeSpec::Infer(infer),
                        ConstExpr::Lit(len.clone()),
                        Some((arr.l_sqbr.clone(), arr.r_sqbr.clone())),
                    );
                    // [1, 2] -> ...
                    // => _: {[1, 2]} -> ...
                    let t_spec_as_expr = if let Ok(expr) = expr {
                        Expr::Set(astSet::Normal(NormalSet::new(
                            Token::DUMMY,
                            Token::DUMMY,
                            Args::single(PosArg::new(expr)),
                        )))
                    } else {
                        Self::dummy_array_expr(len)
                    };
                    param.t_spec = Some(TypeSpecWithOp::new(
                        Token::dummy(TokenKind::Colon, ":"),
                        TypeSpec::Array(t_spec),
                        t_spec_as_expr,
                    ));
                }
                param.pat = buf_param;
            }
            ParamPattern::Record(rec) => {
                let (buf_name, buf_param) = self.gen_buf_nd_param(rec.loc());
                for ParamRecordAttr { lhs, rhs } in rec.elems.iter_mut() {
                    insertion_idx = self.desugar_nested_param_pattern(
                        body,
                        rhs,
                        &buf_name,
                        BufIndex::Record(lhs),
                        insertion_idx,
                    );
                }
                if param.t_spec.is_none() {
                    let mut attrs = RecordAttrs::new(vec![]);
                    let mut tys = vec![];
                    for ParamRecordAttr { lhs, rhs } in rec.elems.iter() {
                        let lhs = Identifier {
                            vis: VisModifierSpec::Public(Token::DUMMY),
                            ..lhs.clone()
                        };
                        let infer = Token::new_fake(TokenKind::Try, "?", line, 0, 0);
                        let expr = rhs
                            .t_spec
                            .as_ref()
                            .map(|ts| *ts.t_spec_as_expr.clone())
                            .unwrap_or(Expr::local(
                                "Obj",
                                infer.lineno,
                                infer.col_begin,
                                infer.col_end,
                            ));
                        let attr =
                            Def::new(Signature::new_var(lhs.clone()), DefBody::new_single(expr));
                        attrs.push(attr);
                        tys.push((
                            lhs.clone(),
                            rhs.t_spec
                                .as_ref()
                                .map(|ts| ts.t_spec.clone())
                                .unwrap_or(TypeSpec::Infer(infer))
                                .clone(),
                        ));
                    }
                    let t_spec = TypeSpec::Record(RecordTypeSpec::new(
                        Some((rec.l_brace.clone(), rec.r_brace.clone())),
                        tys,
                    ));
                    let t_spec_as_expr = Expr::from(NormalRecord::new(
                        rec.l_brace.clone(),
                        rec.r_brace.clone(),
                        attrs,
                    ));
                    param.t_spec = Some(TypeSpecWithOp::new(COLON, t_spec, t_spec_as_expr));
                }
                param.pat = buf_param;
            }
            /*ParamPattern::DataPack(pack) => {
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
            }*/
            _ => {}
        }
    }

    fn desugar_nested_param_pattern(
        &mut self,
        new_body: &mut Block,
        sig: &mut NonDefaultParamSignature,
        buf_name: &str,
        buf_index: BufIndex,
        mut insertion_idx: usize,
    ) -> usize {
        let obj = Expr::local(
            buf_name,
            sig.ln_begin().unwrap_or(1),
            sig.col_begin().unwrap_or(0),
            sig.col_end().unwrap_or(0),
        );
        let acc = match buf_index {
            BufIndex::Tuple(n) => obj.tuple_attr(Literal::nat(n, sig.ln_begin().unwrap_or(1))),
            BufIndex::Array(n) => {
                let r_brace = Token::new(
                    TokenKind::RBrace,
                    "]",
                    sig.ln_begin().unwrap_or(1),
                    sig.col_begin().unwrap_or(0),
                );
                obj.subscr(
                    Expr::Literal(Literal::nat(n, sig.ln_begin().unwrap_or(1))),
                    r_brace,
                )
            }
            BufIndex::Record(attr) => {
                let attr = Identifier::new(VisModifierSpec::Auto, attr.name.clone());
                obj.attr(attr)
            }
        };
        let id = DefId(get_hash(&(&acc, buf_name)));
        let block = Block::new(vec![Expr::Accessor(acc)]);
        let op = Token::from_str(TokenKind::Assign, "=");
        let body = DefBody::new(op, block, id);
        let line = sig.ln_begin().unwrap_or(1);
        match &mut sig.pat {
            ParamPattern::Tuple(tup) => {
                let (buf_name, buf_sig) = self.gen_buf_nd_param(tup.loc());
                let ident = Identifier::private(Str::from(&buf_name));
                new_body.insert(
                    insertion_idx,
                    Expr::Def(Def::new(
                        Signature::Var(VarSignature::new(
                            VarPattern::Ident(ident),
                            sig.t_spec.clone(),
                        )),
                        body,
                    )),
                );
                insertion_idx += 1;
                let mut ty_exprs = vec![];
                let mut tys = vec![];
                for (n, elem) in tup.elems.non_defaults.iter_mut().enumerate() {
                    insertion_idx = self.desugar_nested_param_pattern(
                        new_body,
                        elem,
                        &buf_name,
                        BufIndex::Tuple(n),
                        insertion_idx,
                    );
                    let infer = Token::new_fake(TokenKind::Try, "?", line, 0, 0);
                    let ty_expr = elem
                        .t_spec
                        .as_ref()
                        .map(|ts| *ts.t_spec_as_expr.clone())
                        .unwrap_or(Expr::local(
                            "Obj",
                            infer.lineno,
                            infer.col_begin,
                            infer.col_end,
                        ));
                    ty_exprs.push(PosArg::new(ty_expr));
                    tys.push(
                        elem.t_spec
                            .as_ref()
                            .map(|ts| ts.t_spec.clone())
                            .unwrap_or(TypeSpec::Infer(infer))
                            .clone(),
                    );
                }
                if sig.t_spec.is_none() {
                    let t_spec = TypeSpec::Tuple(TupleTypeSpec::new(tup.elems.parens.clone(), tys));
                    let t_spec_as_expr = Expr::from(NormalTuple::new(Args::pos_only(
                        ty_exprs,
                        tup.elems.parens.clone(),
                    )));
                    sig.t_spec = Some(TypeSpecWithOp::new(COLON, t_spec, t_spec_as_expr));
                }
                sig.pat = buf_sig;
                insertion_idx
            }
            ParamPattern::Array(arr) => {
                let (buf_name, buf_sig) = self.gen_buf_nd_param(arr.loc());
                new_body.insert(
                    insertion_idx,
                    Expr::Def(Def::new(
                        Signature::Var(VarSignature::new(
                            VarPattern::Ident(Identifier::private(Str::from(&buf_name))),
                            sig.t_spec.clone(),
                        )),
                        body,
                    )),
                );
                insertion_idx += 1;
                for (n, elem) in arr.elems.non_defaults.iter_mut().enumerate() {
                    insertion_idx = self.desugar_nested_param_pattern(
                        new_body,
                        elem,
                        &buf_name,
                        BufIndex::Array(n),
                        insertion_idx,
                    );
                }
                if sig.t_spec.is_none() {
                    let len = arr.elems.non_defaults.len();
                    let len = Literal::new(Token::new_fake(
                        TokenKind::NatLit,
                        len.to_string(),
                        line,
                        0,
                        0,
                    ));
                    let infer = Token::new_fake(TokenKind::Try, "?", line, 0, 0);
                    let t_spec = ArrayTypeSpec::new(
                        TypeSpec::Infer(infer),
                        ConstExpr::Lit(len.clone()),
                        Some((arr.l_sqbr.clone(), arr.r_sqbr.clone())),
                    );
                    let t_spec_as_expr = Self::dummy_array_expr(len);
                    sig.t_spec = Some(TypeSpecWithOp::new(
                        COLON,
                        TypeSpec::Array(t_spec),
                        t_spec_as_expr,
                    ));
                }
                sig.pat = buf_sig;
                insertion_idx
            }
            ParamPattern::Record(rec) => {
                let (buf_name, buf_sig) = self.gen_buf_nd_param(rec.loc());
                new_body.insert(
                    insertion_idx,
                    Expr::Def(Def::new(
                        Signature::Var(VarSignature::new(
                            VarPattern::Ident(Identifier::private(Str::from(&buf_name))),
                            sig.t_spec.clone(),
                        )),
                        body,
                    )),
                );
                insertion_idx += 1;
                let mut attrs = RecordAttrs::new(vec![]);
                let mut tys = vec![];
                for ParamRecordAttr { lhs, rhs } in rec.elems.iter_mut() {
                    let lhs = Identifier {
                        vis: VisModifierSpec::Public(Token::DUMMY),
                        ..lhs.clone()
                    };
                    insertion_idx = self.desugar_nested_param_pattern(
                        new_body,
                        rhs,
                        &buf_name,
                        BufIndex::Record(&lhs),
                        insertion_idx,
                    );
                    let infer = Token::new_fake(TokenKind::Try, "?", line, 0, 0);
                    let expr = rhs
                        .t_spec
                        .as_ref()
                        .map(|ts| *ts.t_spec_as_expr.clone())
                        .unwrap_or(Expr::local(
                            "Obj",
                            infer.lineno,
                            infer.col_begin,
                            infer.col_end,
                        ));
                    let attr = Def::new(Signature::new_var(lhs.clone()), DefBody::new_single(expr));
                    attrs.push(attr);
                    tys.push((
                        lhs.clone(),
                        rhs.t_spec
                            .as_ref()
                            .map(|ts| ts.t_spec.clone())
                            .unwrap_or(TypeSpec::Infer(infer))
                            .clone(),
                    ));
                }
                if sig.t_spec.is_none() {
                    let t_spec = TypeSpec::Record(RecordTypeSpec::new(
                        Some((rec.l_brace.clone(), rec.r_brace.clone())),
                        tys,
                    ));
                    let t_spec_as_expr = Expr::from(NormalRecord::new(
                        rec.l_brace.clone(),
                        rec.r_brace.clone(),
                        attrs,
                    ));
                    sig.t_spec = Some(TypeSpecWithOp::new(COLON, t_spec, t_spec_as_expr));
                }
                sig.pat = buf_sig;
                insertion_idx
            }
            /*
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
            */
            ParamPattern::VarName(name) => {
                let ident = Identifier::new(VisModifierSpec::Private, name.clone());
                let v = VarSignature::new(VarPattern::Ident(ident), sig.t_spec.clone());
                let def = Def::new(Signature::Var(v), body);
                new_body.insert(insertion_idx, Expr::Def(def));
                insertion_idx += 1;
                insertion_idx
            }
            ParamPattern::Lit(l) => {
                let lit = l.clone();
                sig.pat = ParamPattern::Discard(Token::new_fake(
                    TokenKind::UBar,
                    "_",
                    l.ln_begin().unwrap_or(1),
                    l.col_begin().unwrap_or(1),
                    l.col_end().unwrap_or(1),
                ));
                let t_spec = TypeSpec::enum_t_spec(vec![lit.clone()]);
                let t_spec_as_expr = Self::dummy_set_expr(lit);
                sig.t_spec = Some(TypeSpecWithOp::new(COLON, t_spec, t_spec_as_expr));
                insertion_idx
            }
            _ => insertion_idx,
        }
    }

    fn _desugar_self(module: Module) -> Module {
        Self::desugar_all_chunks(module, Self::_desugar_self_inner)
    }

    fn _desugar_self_inner(_expr: Expr) -> Expr {
        todo!()
    }

    /// `F(I | I > 0)` -> `F(I: {I: Int | I > 0})`
    fn _desugar_refinement_pattern(_mod: Module) -> Module {
        todo!()
    }

    /// x[y] => x.__getitem__(y)
    /// x.0 => x.__Tuple_getitem__(0)
    /// `==`(x, y) => __eq__(x, y)
    /// x.`==` y => x.__eq__ y
    fn desugar_acc(module: Module) -> Module {
        Self::desugar_all_chunks(module, Self::rec_desugar_acc)
    }

    fn rec_desugar_acc(expr: Expr) -> Expr {
        match expr {
            Expr::Accessor(acc) => Self::desugar_acc_inner(acc),
            expr => Self::perform_desugar(Self::rec_desugar_acc, expr),
        }
    }

    fn desugar_acc_inner(acc: Accessor) -> Expr {
        match acc {
            // x[y] => x.__getitem__(y)
            Accessor::Subscr(subscr) => {
                let loc = subscr.loc();
                let args = Args::single(PosArg::new(Self::rec_desugar_acc(*subscr.index)));
                let call = Call::new(
                    Self::rec_desugar_acc(*subscr.obj),
                    Some(Identifier::public_with_loc(
                        DOT,
                        Str::ever("__getitem__"),
                        loc,
                    )),
                    args,
                );
                Expr::Call(call)
            }
            // x.0 => x.__Tuple_getitem__(0)
            Accessor::TupleAttr(tattr) => {
                let loc = tattr.loc();
                let args = Args::single(PosArg::new(Expr::Literal(tattr.index)));
                let call = Call::new(
                    Self::rec_desugar_acc(*tattr.obj),
                    Some(Identifier::public_with_loc(
                        DOT,
                        Str::ever("__Tuple_getitem__"),
                        loc,
                    )),
                    args,
                );
                Expr::Call(call)
            }
            Accessor::TypeApp(mut tapp) => {
                tapp.obj = Box::new(Self::rec_desugar_acc(*tapp.obj));
                // REVIEW: tapp.type_args
                Expr::Accessor(Accessor::TypeApp(tapp))
            }
            Accessor::Attr(mut attr) => {
                attr.obj = Box::new(Self::rec_desugar_acc(*attr.obj));
                attr.ident = Self::desugar_ident(attr.ident);
                Expr::Accessor(Accessor::Attr(attr))
            }
            Accessor::Ident(ident) => Expr::Accessor(Accessor::Ident(Self::desugar_ident(ident))),
        }
    }

    fn desugar_ident(mut ident: Identifier) -> Identifier {
        if let Some(name) = symop_to_dname(ident.inspect()) {
            ident.name.rename(name.into());
        }
        ident
    }

    // TODO: pipeline desugaring (move from `Parser`)
    fn desugar_operator(module: Module) -> Module {
        Self::desugar_all_chunks(module, Self::rec_desugar_operator)
    }

    /// `l in r => r contains l`
    /// `l notin r => not r contains l`
    fn rec_desugar_operator(expr: Expr) -> Expr {
        match expr {
            Expr::BinOp(bin) if bin.op.is(TokenKind::InOp) => {
                let (mut op, lhs, rhs) = bin.deconstruct();
                op.content = Str::from("contains");
                op.kind = TokenKind::ContainsOp;
                Expr::BinOp(BinOp::new(op, rhs, lhs))
            }
            Expr::BinOp(bin) if bin.op.is(TokenKind::NotInOp) => {
                let (mut op, lhs, rhs) = bin.deconstruct();
                op.content = Str::from("contains");
                op.kind = TokenKind::ContainsOp;
                let not = Identifier::private("not".into());
                let bin = Expr::BinOp(BinOp::new(op, rhs, lhs));
                Expr::Accessor(Accessor::Ident(not)).call1(bin)
            }
            expr => Self::perform_desugar(Self::rec_desugar_operator, expr),
        }
    }

    fn desugar_comprehension(module: Module) -> Module {
        Self::desugar_all_chunks(module, Self::rec_desugar_comprehension)
    }

    /// ```erg
    /// [y | x <- xs] ==> array(map(x -> y, xs))
    /// [(a, b) | x <- xs; y <- ys] ==> array(map(((x, y),) -> (a, b), itertools.product(xs, ys)))
    /// {k: v | x <- xs} ==> dict(map(x -> (k, v), xs))
    /// {y | x <- xs} ==> set(map(x -> y, xs))
    /// {x <- xs | x <= 10} ==> set(filter(x -> x <= 10, xs))
    /// {x + 1 | x <- xs | x <= 10} ==> set(map(x -> x + 1, filter(x -> x <= 10, xs)))
    /// ```
    fn rec_desugar_comprehension(expr: Expr) -> Expr {
        match expr {
            Expr::Array(Array::Comprehension(mut comp)) => {
                debug_power_assert!(comp.generators.len(), >, 0);
                if comp.generators.len() != 1 {
                    return Expr::Array(Array::Comprehension(comp));
                }
                let (ident, iter) = comp.generators.remove(0);
                let iterator = Self::desugar_layout_and_guard(ident, iter, comp.layout, comp.guard);
                let array = if PYTHON_MODE { "list" } else { "array" };
                Identifier::auto(array.into()).call1(iterator.into()).into()
            }
            Expr::Dict(Dict::Comprehension(mut comp)) => {
                debug_power_assert!(comp.generators.len(), >, 0);
                if comp.generators.len() != 1 {
                    return Expr::Dict(Dict::Comprehension(comp));
                }
                let (ident, iter) = comp.generators.remove(0);
                let params = Params::single(NonDefaultParamSignature::new(
                    ParamPattern::VarName(ident.name),
                    None,
                ));
                let sig = LambdaSignature::new(params, None, TypeBoundSpecs::empty());
                let tuple = Tuple::Normal(NormalTuple::new(Args::pos_only(
                    vec![PosArg::new(comp.kv.key), PosArg::new(comp.kv.value)],
                    None,
                )));
                let body = Block::new(vec![tuple.into()]);
                let lambda = Lambda::new(sig, Token::DUMMY, body, DefId(0));
                let map = Identifier::private("map".into()).call2(lambda.into(), iter);
                Identifier::auto("dict".into()).call1(map.into()).into()
            }
            Expr::Set(astSet::Comprehension(mut comp)) => {
                debug_power_assert!(comp.generators.len(), >, 0);
                if comp.generators.len() != 1 {
                    return Expr::Set(astSet::Comprehension(comp));
                }
                let (ident, iter) = comp.generators.remove(0);
                let iterator = Self::desugar_layout_and_guard(ident, iter, comp.layout, comp.guard);
                Identifier::auto("set".into()).call1(iterator.into()).into()
            }
            expr => Self::perform_desugar(Self::rec_desugar_comprehension, expr),
        }
    }

    fn desugar_layout_and_guard(
        ident: Identifier,
        iter: Expr,
        layout: Option<Box<Expr>>,
        guard: Option<Box<Expr>>,
    ) -> Call {
        let params = Params::single(NonDefaultParamSignature::new(
            ParamPattern::VarName(ident.name),
            None,
        ));
        let sig = LambdaSignature::new(params, None, TypeBoundSpecs::empty());
        match (layout, guard) {
            (Some(elem), Some(guard)) => {
                let f_body = Block::new(vec![*guard]);
                let f_lambda = Lambda::new(sig.clone(), Token::DUMMY, f_body, DefId(0));
                let filter = Identifier::auto("filter".into()).call2(f_lambda.into(), iter);
                let m_body = Block::new(vec![*elem]);
                let m_lambda = Lambda::new(sig, Token::DUMMY, m_body, DefId(0));
                Identifier::auto("map".into()).call2(m_lambda.into(), filter.into())
            }
            (Some(elem), None) => {
                let body = Block::new(vec![*elem]);
                let lambda = Lambda::new(sig, Token::DUMMY, body, DefId(0));
                Identifier::auto("map".into()).call2(lambda.into(), iter)
            }
            (None, Some(guard)) => {
                let body = Block::new(vec![*guard]);
                let lambda = Lambda::new(sig, Token::DUMMY, body, DefId(0));
                Identifier::auto("filter".into()).call2(lambda.into(), iter)
            }
            (None, None) => todo!(),
        }
    }
}

impl Default for Desugarer {
    fn default() -> Self {
        Self::new()
    }
}
