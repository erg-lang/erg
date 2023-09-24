use erg_common::{enum_unwrap, traits::Stream};

use erg_parser::ast::*;

#[derive(Debug)]
pub struct ASTTransformer {
    ast: Module,
}

impl ASTTransformer {
    pub fn new(ast: Module) -> Self {
        Self { ast }
    }

    pub fn transform(mut self) -> Module {
        self.reorder_record_fields();
        self.ast
    }

    fn transform_expr(mut desugar: impl FnMut(Expr) -> Expr, expr: Expr) -> Expr {
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
                Set::Normal(set) => {
                    let (elems, ..) = set.elems.deconstruct();
                    let elems = elems
                        .into_iter()
                        .map(|elem| PosArg::new(desugar(elem.expr)))
                        .collect();
                    let elems = Args::pos_only(elems, None);
                    let set = NormalSet::new(set.l_brace, set.r_brace, elems);
                    Expr::Set(Set::Normal(set))
                }
                Set::WithLength(set) => {
                    let elem = PosArg::new(desugar(set.elem.expr));
                    let len = desugar(*set.len);
                    let set = SetWithLength::new(set.l_brace, set.r_brace, elem, len);
                    Expr::Set(Set::WithLength(set))
                }
                Set::Comprehension(set) => {
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
                    Expr::Set(Set::Comprehension(set))
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
                let args = Self::transform_args(desugar, call.args);
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
                    subr.params = Self::transform_params(desugar, subr.params);
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
                let attr = Self::transform_acc(desugar, redef.attr);
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
                lambda.sig.params = Self::transform_params(desugar, lambda.sig.params);
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
            Expr::Accessor(acc) => Expr::Accessor(Self::transform_acc(desugar, acc)),
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

    fn transform_args(mut desugar: impl FnMut(Expr) -> Expr, args: Args) -> Args {
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

    fn transform_acc(mut desugar: impl FnMut(Expr) -> Expr, acc: Accessor) -> Accessor {
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
                        TypeAppArgsKind::Args(Self::transform_args(desugar, args))
                    }
                    other => other,
                };
                let type_args =
                    TypeAppArgs::new(tapp.type_args.l_vbar, args, tapp.type_args.r_vbar);
                obj.type_app(type_args)
            }
        }
    }

    fn transform_params(mut desugar: impl FnMut(Expr) -> Expr, mut params: Params) -> Params {
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

    fn reorder_record_fields_(expr: Expr) -> Expr {
        expr
    }

    fn reorder_record_fields(&mut self) {
        let mut new = Vec::with_capacity(self.ast.len());
        for chunk in self.ast.take_all() {
            new.push(Self::transform_expr(Self::reorder_record_fields_, chunk));
        }
        self.ast = Module::new(new);
    }
}
