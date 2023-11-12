use erg_common::switch_lang;
use erg_common::traits::{Locational, Stream};

use crate::ast::*;
use crate::desugar::Desugarer;
use crate::error::ParseError;
use crate::token::TokenKind;
use crate::Parser;

// The APIs defined below are also used by `ASTLowerer` to interpret expressions as types.
impl Parser {
    pub fn validate_const_expr(expr: Expr) -> Result<ConstExpr, ParseError> {
        match expr {
            Expr::Literal(l) => Ok(ConstExpr::Lit(l)),
            Expr::Accessor(acc) => match acc {
                Accessor::Ident(local) => Ok(ConstExpr::Accessor(ConstAccessor::Local(local))),
                Accessor::Attr(attr) => {
                    let expr = Self::validate_const_expr(*attr.obj)?;
                    Ok(ConstExpr::Accessor(ConstAccessor::Attr(
                        ConstAttribute::new(expr, attr.ident),
                    )))
                }
                other => Err(ParseError::feature_error(
                    line!() as usize,
                    other.loc(),
                    "complex const accessor",
                )),
            },
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    let (elems, ..) = arr.elems.deconstruct();
                    let mut const_elems = vec![];
                    for elem in elems.into_iter() {
                        let const_expr = Self::validate_const_expr(elem.expr)?;
                        const_elems.push(ConstPosArg::new(const_expr));
                    }
                    let elems = ConstArgs::pos_only(const_elems, None);
                    let const_arr = ConstNormalArray::new(arr.l_sqbr, arr.r_sqbr, elems, None);
                    Ok(ConstExpr::Array(ConstArray::Normal(const_arr)))
                }
                Array::WithLength(arr) => {
                    let elem = Self::validate_const_expr(arr.elem.expr)?;
                    let len = Self::validate_const_expr(*arr.len)?;
                    let const_arr = ConstArrayWithLength::new(arr.l_sqbr, arr.r_sqbr, elem, len);
                    Ok(ConstExpr::Array(ConstArray::WithLength(const_arr)))
                }
                other => Err(ParseError::feature_error(
                    line!() as usize,
                    other.loc(),
                    "const array comprehension",
                )),
            },
            Expr::Set(set) => match set {
                Set::Normal(set) => {
                    let (elems, ..) = set.elems.deconstruct();
                    let mut const_elems = vec![];
                    for elem in elems.into_iter() {
                        let const_expr = Self::validate_const_expr(elem.expr)?;
                        const_elems.push(ConstPosArg::new(const_expr));
                    }
                    let elems = ConstArgs::pos_only(const_elems, None);
                    let const_set = ConstNormalSet::new(set.l_brace, set.r_brace, elems);
                    Ok(ConstExpr::Set(ConstSet::Normal(const_set)))
                }
                Set::Comprehension(set) => {
                    let elem = set
                        .layout
                        .map(|ex| Self::validate_const_expr(*ex))
                        .transpose()?;
                    let mut generators = vec![];
                    for (name, gen) in set.generators.into_iter() {
                        let pred = Self::validate_const_expr(gen)?;
                        generators.push((name, pred));
                    }
                    let guard = set
                        .guard
                        .map(|ex| Self::validate_const_expr(*ex))
                        .transpose()?;
                    let const_set_comp = ConstSetComprehension::new(
                        set.l_brace,
                        set.r_brace,
                        elem,
                        generators,
                        guard,
                    );
                    Ok(ConstExpr::Set(ConstSet::Comprehension(const_set_comp)))
                }
                other => Err(ParseError::feature_error(
                    line!() as usize,
                    other.loc(),
                    "const set with length",
                )),
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dict) => {
                    let mut const_kvs = vec![];
                    for kv in dict.kvs.into_iter() {
                        let key = Self::validate_const_expr(kv.key)?;
                        let value = Self::validate_const_expr(kv.value)?;
                        const_kvs.push(ConstKeyValue::new(key, value));
                    }
                    let const_dict = ConstDict::new(dict.l_brace, dict.r_brace, const_kvs);
                    Ok(ConstExpr::Dict(const_dict))
                }
                other => Err(ParseError::feature_error(
                    line!() as usize,
                    other.loc(),
                    "const dict comprehension",
                )),
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    let (elems, _, _, _, paren) = tup.elems.deconstruct();
                    let mut const_elems = vec![];
                    for elem in elems.into_iter() {
                        let const_expr = Self::validate_const_expr(elem.expr)?;
                        const_elems.push(ConstPosArg::new(const_expr));
                    }
                    let elems = ConstArgs::pos_only(const_elems, paren);
                    let const_tup = ConstTuple::new(elems);
                    Ok(ConstExpr::Tuple(const_tup))
                }
            },
            Expr::BinOp(bin) => {
                let mut args = bin.args.into_iter();
                let lhs = Self::validate_const_expr(*args.next().unwrap())?;
                let rhs = Self::validate_const_expr(*args.next().unwrap())?;
                Ok(ConstExpr::BinOp(ConstBinOp::new(bin.op, lhs, rhs)))
            }
            Expr::UnaryOp(unary) => {
                let mut args = unary.args.into_iter();
                let arg = Self::validate_const_expr(*args.next().unwrap())?;
                Ok(ConstExpr::UnaryOp(ConstUnaryOp::new(unary.op, arg)))
            }
            Expr::Call(call) => {
                let obj = Self::validate_const_expr(*call.obj)?;
                /*let ConstExpr::Accessor(acc) = obj else {
                    return Err(ParseError::feature_error(
                        line!() as usize,
                        obj.loc(),
                        "complex const function call",
                    ));
                };*/
                let attr_name = call.attr_name;
                let (pos_args, _, _, _, paren) = call.args.deconstruct();
                let mut const_pos_args = vec![];
                for elem in pos_args.into_iter() {
                    let const_expr = Self::validate_const_expr(elem.expr)?;
                    const_pos_args.push(ConstPosArg::new(const_expr));
                }
                let args = ConstArgs::pos_only(const_pos_args, paren);
                Ok(ConstExpr::App(ConstApp::new(obj, attr_name, args)))
            }
            Expr::Def(def) => Self::validate_const_def(def).map(ConstExpr::Def),
            Expr::Lambda(lambda) => {
                let body = Self::validate_const_block(lambda.body)?;
                let lambda = ConstLambda::new(lambda.sig, lambda.op, body, lambda.id);
                Ok(ConstExpr::Lambda(lambda))
            }
            Expr::Record(rec) => {
                let rec = match rec {
                    Record::Normal(rec) => rec,
                    Record::Mixed(mixed) => Desugarer::desugar_shortened_record_inner(mixed),
                };
                let mut const_fields = vec![];
                for attr in rec.attrs.into_iter() {
                    const_fields.push(Self::validate_const_def(attr)?);
                }
                Ok(ConstExpr::Record(ConstRecord::new(
                    rec.l_brace,
                    rec.r_brace,
                    const_fields,
                )))
            }
            Expr::TypeAscription(tasc) => {
                let expr = Self::validate_const_expr(*tasc.expr)?;
                Ok(ConstExpr::TypeAsc(ConstTypeAsc::new(expr, tasc.t_spec)))
            }
            other => Err(ParseError::syntax_error(
                line!() as usize,
                other.loc(),
                switch_lang!(
                    "japanese" => "この式はコンパイル時計算できないため、型引数には使用できません",
                    "simplified_chinese" => "此表达式在编译时不可计算，因此不能用作类型参数",
                    "traditional_chinese" => "此表達式在編譯時不可計算，因此不能用作類型參數",
                    "english" => "this expression is not computable at the compile-time, so cannot used as a type-argument",
                ),
                None,
            )),
        }
    }

    pub fn validate_const_block(block: Block) -> Result<ConstBlock, ParseError> {
        let mut const_block = vec![];
        for expr in block.into_iter() {
            let const_expr = Self::validate_const_expr(expr)?;
            const_block.push(const_expr);
        }
        Ok(ConstBlock::new(const_block))
    }

    fn validate_const_def(def: Def) -> Result<ConstDef, ParseError> {
        let block = Self::validate_const_block(def.body.block)?;
        let body = ConstDefBody::new(def.body.op, block, def.body.id);
        Ok(ConstDef::new(def.sig.ident().unwrap().clone(), body))
    }

    fn accessor_to_type_spec(accessor: Accessor) -> Result<TypeSpec, ParseError> {
        let t_spec = match accessor {
            Accessor::Ident(ident) => TypeSpec::mono(ident),
            Accessor::TypeApp(tapp) => {
                let spec = Self::expr_to_type_spec(*tapp.obj)?;
                TypeSpec::type_app(spec, tapp.type_args)
            }
            Accessor::Attr(attr) => {
                let namespace = *attr.obj;
                let predecl = PreDeclTypeSpec::attr(namespace, attr.ident);
                TypeSpec::PreDeclTy(predecl)
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                return Err(err);
            }
        };
        Ok(t_spec)
    }

    pub(crate) fn call_to_predecl_type_spec(call: Call) -> Result<PreDeclTypeSpec, ParseError> {
        match *call.obj {
            Expr::Accessor(Accessor::Ident(ident)) => {
                let (_pos_args, _var_args, _kw_args, _kw_var, paren) = call.args.deconstruct();
                let mut pos_args = vec![];
                for arg in _pos_args.into_iter() {
                    let const_expr = Self::validate_const_expr(arg.expr)?;
                    pos_args.push(ConstPosArg::new(const_expr));
                }
                let var_args = if let Some(var_args) = _var_args {
                    let const_var_args = Self::validate_const_expr(var_args.expr)?;
                    Some(ConstPosArg::new(const_var_args))
                } else {
                    None
                };
                let mut kw_args = vec![];
                for arg in _kw_args.into_iter() {
                    let const_expr = Self::validate_const_expr(arg.expr)?;
                    kw_args.push(ConstKwArg::new(arg.keyword, const_expr));
                }
                let kw_var = if let Some(kw_var) = _kw_var {
                    let const_kw_var = Self::validate_const_expr(kw_var.expr)?;
                    Some(ConstPosArg::new(const_kw_var))
                } else {
                    None
                };
                let acc = if let Some(attr) = call.attr_name {
                    ConstAccessor::attr(ConstExpr::Accessor(ConstAccessor::Local(ident)), attr)
                } else {
                    ConstAccessor::Local(ident)
                };
                Ok(PreDeclTypeSpec::poly(
                    acc,
                    ConstArgs::new(pos_args, var_args, kw_args, kw_var, paren),
                ))
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                Err(err)
            }
        }
    }

    fn lambda_to_subr_type_spec(mut lambda: Lambda) -> Result<SubrTypeSpec, ParseError> {
        let bounds = lambda.sig.bounds;
        let lparen = lambda.sig.params.parens.map(|(l, _)| l);
        let mut non_defaults = vec![];
        for param in lambda.sig.params.non_defaults.into_iter() {
            let param = match (param.pat, param.t_spec) {
                (ParamPattern::VarName(name), Some(t_spec_with_op)) => {
                    ParamTySpec::new(Some(name.into_token()), t_spec_with_op.t_spec)
                }
                (ParamPattern::VarName(name), None) => ParamTySpec::anonymous(TypeSpec::mono(
                    Identifier::new(VisModifierSpec::Private, name),
                )),
                (ParamPattern::Discard(_), Some(t_spec_with_op)) => {
                    ParamTySpec::anonymous(t_spec_with_op.t_spec)
                }
                (param, _t_spec) => {
                    let err =
                        ParseError::feature_error(line!() as usize, param.loc(), "param pattern");
                    return Err(err);
                }
            };
            non_defaults.push(param);
        }
        let var_params =
            lambda
                .sig
                .params
                .var_params
                .map(|var_args| match (var_args.pat, var_args.t_spec) {
                    (ParamPattern::VarName(name), Some(t_spec_with_op)) => {
                        ParamTySpec::new(Some(name.into_token()), t_spec_with_op.t_spec)
                    }
                    (ParamPattern::VarName(name), None) => ParamTySpec::anonymous(TypeSpec::mono(
                        Identifier::new(VisModifierSpec::Private, name),
                    )),
                    (ParamPattern::Discard(_), Some(t_spec_with_op)) => {
                        ParamTySpec::anonymous(t_spec_with_op.t_spec)
                    }
                    (param, t_spec) => todo!("{param}: {t_spec:?}"),
                });
        let mut defaults = vec![];
        for param in lambda.sig.params.defaults.into_iter() {
            let param = match (param.sig.pat, param.sig.t_spec) {
                (ParamPattern::VarName(name), Some(t_spec_with_op)) => {
                    let param_spec =
                        ParamTySpec::new(Some(name.into_token()), t_spec_with_op.t_spec);
                    let default_spec = Self::expr_to_type_spec(param.default_val)?;
                    DefaultParamTySpec::new(param_spec, default_spec)
                }
                (ParamPattern::VarName(name), None) => {
                    let default_spec = Self::expr_to_type_spec(param.default_val)?;
                    let param_spec =
                        ParamTySpec::new(Some(name.into_token()), default_spec.clone());
                    DefaultParamTySpec::new(param_spec, default_spec)
                }
                (param, _t_spec) => {
                    let err =
                        ParseError::feature_error(line!() as usize, param.loc(), "param pattern");
                    return Err(err);
                }
            };
            defaults.push(param);
        }
        let return_t = Self::expr_to_type_spec(lambda.body.remove(0))?;
        Ok(SubrTypeSpec::new(
            bounds,
            lparen,
            non_defaults,
            var_params,
            defaults,
            lambda.op,
            return_t,
        ))
    }

    fn array_to_array_type_spec(array: Array) -> Result<ArrayTypeSpec, ParseError> {
        match array {
            Array::Normal(arr) => {
                // TODO: add hint
                let err = ParseError::simple_syntax_error(line!() as usize, arr.loc());
                Err(err)
            }
            Array::WithLength(arr) => {
                let t_spec = Self::expr_to_type_spec(arr.elem.expr)?;
                let len = Self::validate_const_expr(*arr.len)?;
                Ok(ArrayTypeSpec::new(
                    t_spec,
                    len,
                    Some((arr.l_sqbr, arr.r_sqbr)),
                ))
            }
            Array::Comprehension(arr) => {
                // TODO: add hint
                let err = ParseError::simple_syntax_error(line!() as usize, arr.loc());
                Err(err)
            }
        }
    }

    fn set_to_set_type_spec(set: Set) -> Result<TypeSpec, ParseError> {
        match set {
            Set::Normal(set) => {
                let mut elem_ts = vec![];
                let (elems, .., paren) = set.elems.deconstruct();
                for elem in elems.into_iter() {
                    let const_expr = Self::validate_const_expr(elem.expr)?;
                    elem_ts.push(ConstPosArg::new(const_expr));
                }
                Ok(TypeSpec::Enum(ConstArgs::pos_only(elem_ts, paren)))
            }
            Set::WithLength(set) => {
                let t_spec = Self::expr_to_type_spec(set.elem.expr)?;
                let len = Self::validate_const_expr(*set.len)?;
                Ok(TypeSpec::SetWithLen(SetWithLenTypeSpec::new(t_spec, len)))
            }
            Set::Comprehension(set) => {
                if set.layout.is_none() && set.generators.len() == 1 && set.guard.is_some() {
                    let (ident, expr) = set.generators.into_iter().next().unwrap();
                    let typ = Self::expr_to_type_spec(expr)?;
                    let pred = Self::validate_const_expr(*set.guard.unwrap())?;
                    let refine = RefinementTypeSpec::new(ident.name.into_token(), typ, pred);
                    Ok(TypeSpec::Refinement(refine))
                } else {
                    Err(ParseError::simple_syntax_error(line!() as usize, set.loc()))
                }
            }
        }
    }

    fn dict_to_dict_type_spec(dict: Dict) -> Result<DictTypeSpec, ParseError> {
        let (l, r) = dict.braces();
        let braces = (l.clone(), r.clone());
        let kvs = match dict {
            Dict::Normal(dic) => {
                let mut kvs = vec![];
                for kv in dic.kvs.into_iter() {
                    let key = Self::expr_to_type_spec(kv.key)?;
                    let value = Self::expr_to_type_spec(kv.value)?;
                    kvs.push((key, value));
                }
                Ok(kvs)
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                Err(err)
            }
        }?;
        Ok(DictTypeSpec::new(Some(braces), kvs))
    }

    fn record_to_record_type_spec(record: Record) -> Result<RecordTypeSpec, ParseError> {
        let (l, r) = record.braces();
        let braces = (l.clone(), r.clone());
        let attrs = match record {
            Record::Normal(rec) => rec
                .attrs
                .into_iter()
                .map(|mut def| {
                    let ident = def.sig.ident().unwrap().clone();
                    // TODO: check block.len() == 1
                    let value = Self::expr_to_type_spec(def.body.block.pop().unwrap())?;
                    Ok((ident, value))
                })
                .collect::<Result<Vec<_>, ParseError>>(),
            Record::Mixed(rec) => rec
                .attrs
                .into_iter()
                .map(|attr_or_ident| match attr_or_ident {
                    RecordAttrOrIdent::Attr(mut def) => {
                        let ident = def.sig.ident().unwrap().clone();
                        // TODO: check block.len() == 1
                        let value = Self::expr_to_type_spec(def.body.block.pop().unwrap())?;
                        Ok((ident, value))
                    }
                    RecordAttrOrIdent::Ident(_ident) => {
                        todo!("TypeSpec for shortened record is not implemented.")
                    }
                })
                .collect::<Result<Vec<_>, ParseError>>(),
        }?;
        Ok(RecordTypeSpec::new(Some(braces), attrs))
    }

    fn tuple_to_tuple_type_spec(tuple: Tuple) -> Result<TupleTypeSpec, ParseError> {
        match tuple {
            Tuple::Normal(tup) => {
                let mut tup_spec = vec![];
                let (elems, .., parens) = tup.elems.deconstruct();
                for elem in elems.into_iter() {
                    let value = Self::expr_to_type_spec(elem.expr)?;
                    tup_spec.push(value);
                }
                Ok(TupleTypeSpec::new(parens, tup_spec))
            }
        }
    }

    pub fn expr_to_type_spec(rhs: Expr) -> Result<TypeSpec, ParseError> {
        match rhs {
            Expr::Accessor(acc) => Self::accessor_to_type_spec(acc),
            Expr::Call(call) => {
                let predecl = Self::call_to_predecl_type_spec(call)?;
                Ok(TypeSpec::PreDeclTy(predecl))
            }
            Expr::Lambda(lambda) => {
                let lambda = Self::lambda_to_subr_type_spec(lambda)?;
                Ok(TypeSpec::Subr(lambda))
            }
            Expr::Array(array) => {
                let array = Self::array_to_array_type_spec(array)?;
                Ok(TypeSpec::Array(array))
            }
            Expr::Set(set) => {
                let set = Self::set_to_set_type_spec(set)?;
                Ok(set)
            }
            Expr::Dict(dict) => {
                let dict = Self::dict_to_dict_type_spec(dict)?;
                Ok(TypeSpec::Dict(dict))
            }
            Expr::Record(rec) => {
                let rec = Self::record_to_record_type_spec(rec)?;
                Ok(TypeSpec::Record(rec))
            }
            Expr::Tuple(tup) => {
                let tup = Self::tuple_to_tuple_type_spec(tup)?;
                Ok(TypeSpec::Tuple(tup))
            }
            Expr::BinOp(bin) => {
                if bin.op.kind.is_range_op() {
                    let op = bin.op;
                    let mut args = bin.args.into_iter();
                    let lhs = Self::validate_const_expr(*args.next().unwrap())?;
                    let rhs = Self::validate_const_expr(*args.next().unwrap())?;
                    Ok(TypeSpec::Interval { op, lhs, rhs })
                } else if bin.op.kind == TokenKind::AndOp {
                    let mut args = bin.args.into_iter();
                    let lhs = Self::expr_to_type_spec(*args.next().unwrap())?;
                    let rhs = Self::expr_to_type_spec(*args.next().unwrap())?;
                    Ok(TypeSpec::and(lhs, rhs))
                } else if bin.op.kind == TokenKind::OrOp {
                    let mut args = bin.args.into_iter();
                    let lhs = Self::expr_to_type_spec(*args.next().unwrap())?;
                    let rhs = Self::expr_to_type_spec(*args.next().unwrap())?;
                    Ok(TypeSpec::or(lhs, rhs))
                } else {
                    let err = ParseError::simple_syntax_error(line!() as usize, bin.loc());
                    Err(err)
                }
            }
            Expr::Literal(lit) => {
                let mut err = ParseError::simple_syntax_error(line!() as usize, lit.loc());
                if lit.is(TokenKind::NoneLit) {
                    err.set_hint("you mean: `NoneType`?");
                }
                Err(err)
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                Err(err)
            }
        }
    }
}
