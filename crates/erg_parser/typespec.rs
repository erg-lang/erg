use erg_common::switch_lang;
use erg_common::traits::{Locational, Stream};

use crate::ast::*;
use crate::error::ParseError;
use crate::token::{Token, TokenKind};
use crate::Parser;

// The APIs defined below are also used by `ASTLowerer` to interpret expressions as types.
impl Parser {
    pub fn validate_const_expr(expr: Expr) -> Result<ConstExpr, ParseError> {
        match expr {
            Expr::Literal(l) => Ok(ConstExpr::Lit(l)),
            Expr::Accessor(Accessor::Ident(local)) => {
                Ok(ConstExpr::Accessor(ConstAccessor::Local(local)))
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    let (elems, ..) = arr.elems.deconstruct();
                    let mut const_elems = vec![];
                    for elem in elems.into_iter() {
                        let const_expr = Self::validate_const_expr(elem.expr)?;
                        const_elems.push(ConstPosArg::new(const_expr));
                    }
                    let elems = ConstArgs::new(const_elems, None, vec![], None);
                    let const_arr = ConstArray::new(arr.l_sqbr, arr.r_sqbr, elems, None);
                    Ok(ConstExpr::Array(const_arr))
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
                    let elems = ConstArgs::new(const_elems, None, vec![], None);
                    let const_set = ConstSet::new(set.l_brace, set.r_brace, elems);
                    Ok(ConstExpr::Set(const_set))
                }
                other => Err(ParseError::feature_error(
                    line!() as usize,
                    other.loc(),
                    "const set comprehension",
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
                    let (elems, _, _, paren) = tup.elems.deconstruct();
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
            Expr::PrefixOp(unary) => {
                let mut args = unary.args.into_iter();
                let arg = Self::validate_const_expr(*args.next().unwrap())?;
                Ok(ConstExpr::UnaryOp(ConstUnaryOp::new(unary.op, arg)))
            }
            Expr::Call(call) => {
                let obj = Self::validate_const_expr(*call.obj)?;
                let ConstExpr::Accessor(acc) = obj else {
                    return Err(ParseError::feature_error(
                        line!() as usize,
                        obj.loc(),
                        "complex const function call",
                    ));
                };
                let (pos_args, _, _, paren) = call.args.deconstruct();
                let mut const_pos_args = vec![];
                for elem in pos_args.into_iter() {
                    let const_expr = Self::validate_const_expr(elem.expr)?;
                    const_pos_args.push(ConstPosArg::new(const_expr));
                }
                let args = ConstArgs::pos_only(const_pos_args, paren);
                Ok(ConstExpr::App(ConstApp::new(acc, args)))
            }
            // TODO: App, Record,
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

    fn ident_to_type_spec(ident: Identifier) -> SimpleTypeSpec {
        SimpleTypeSpec::new(ident, ConstArgs::empty())
    }

    fn accessor_to_type_spec(accessor: Accessor) -> Result<TypeSpec, ParseError> {
        let t_spec = match accessor {
            Accessor::Ident(ident) => {
                let predecl = PreDeclTypeSpec::Simple(Self::ident_to_type_spec(ident));
                TypeSpec::PreDeclTy(predecl)
            }
            Accessor::TypeApp(tapp) => {
                let spec = Self::expr_to_type_spec(*tapp.obj)?;
                TypeSpec::type_app(spec, tapp.type_args)
            }
            Accessor::Attr(attr) => {
                let namespace = attr.obj;
                let t = Self::ident_to_type_spec(attr.ident);
                let predecl = PreDeclTypeSpec::Attr { namespace, t };
                TypeSpec::PreDeclTy(predecl)
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                return Err(err);
            }
        };
        Ok(t_spec)
    }

    fn call_to_predecl_type_spec(call: Call) -> Result<PreDeclTypeSpec, ParseError> {
        match *call.obj {
            Expr::Accessor(Accessor::Ident(ident)) => {
                let (_pos_args, _var_args, _kw_args, paren) = call.args.deconstruct();
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
                Ok(PreDeclTypeSpec::Simple(SimpleTypeSpec::new(
                    ident,
                    ConstArgs::new(pos_args, var_args, kw_args, paren),
                )))
            }
            _ => todo!(),
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
                (ParamPattern::VarName(name), None) => {
                    ParamTySpec::anonymous(TypeSpec::PreDeclTy(PreDeclTypeSpec::Simple(
                        SimpleTypeSpec::new(Identifier::new(None, name), ConstArgs::empty()),
                    )))
                }
                _ => todo!(),
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
                    (ParamPattern::VarName(name), None) => {
                        ParamTySpec::anonymous(TypeSpec::PreDeclTy(PreDeclTypeSpec::Simple(
                            SimpleTypeSpec::new(Identifier::new(None, name), ConstArgs::empty()),
                        )))
                    }
                    _ => todo!(),
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
                (l, r) => todo!("{:?} {:?}", l, r),
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
                Ok(ArrayTypeSpec::new(t_spec, len))
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
        }
    }

    fn dict_to_dict_type_spec(dict: Dict) -> Result<Vec<(TypeSpec, TypeSpec)>, ParseError> {
        match dict {
            Dict::Normal(dic) => {
                let mut kvs = vec![];
                for kv in dic.kvs.into_iter() {
                    let key = Self::expr_to_type_spec(kv.key)?;
                    let value = Self::expr_to_type_spec(kv.value)?;
                    kvs.push((key, value));
                }
                Ok(kvs)
            }
            _ => todo!(),
        }
    }

    fn record_to_record_type_spec(
        record: Record,
    ) -> Result<Vec<(Identifier, TypeSpec)>, ParseError> {
        match record {
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
        }
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

    fn simple_type_spec_to_ident(simple: SimpleTypeSpec) -> Result<Identifier, ParseError> {
        Ok(simple.ident)
    }

    fn simple_type_spec_to_call(simple: SimpleTypeSpec) -> Result<Call, ParseError> {
        let (pos_args_, var_args_, kw_args_, paren) = simple.args.deconstruct();
        let pos_args = pos_args_
            .into_iter()
            .map(|arg| PosArg::new(arg.expr.downcast()))
            .collect::<Vec<_>>();
        let var_args = var_args_.map(|arg| PosArg::new(arg.expr.downcast()));
        let kw_args = kw_args_
            .into_iter()
            .map(|arg| KwArg::new(arg.keyword, None, arg.expr.downcast()))
            .collect::<Vec<_>>();
        let args = Args::new(pos_args, var_args, kw_args, paren);
        let call = Call::new(simple.ident.into(), None, args);
        Ok(call)
    }

    fn predecl_type_spec_to_expr(predecl: PreDeclTypeSpec) -> Result<Expr, ParseError> {
        match predecl {
            PreDeclTypeSpec::Simple(simple) if simple.args.is_empty() => {
                Ok(Self::simple_type_spec_to_ident(simple)?.into())
            }
            PreDeclTypeSpec::Simple(simple) => Ok(Self::simple_type_spec_to_call(simple)?.into()),
            PreDeclTypeSpec::Attr { namespace, t } => {
                let ident = Self::simple_type_spec_to_ident(t)?;
                Ok(namespace.attr_expr(ident))
            }
            other => Err(ParseError::feature_error(
                line!() as usize,
                other.loc(),
                "compound predecl type spec to call conversion",
            )),
        }
    }

    pub fn type_spec_to_expr(t_spec: TypeSpec) -> Result<Expr, ParseError> {
        match t_spec {
            TypeSpec::PreDeclTy(predecl) => Self::predecl_type_spec_to_expr(predecl),
            TypeSpec::Or(lhs, rhs) => {
                let lhs = Self::type_spec_to_expr(*lhs)?;
                let rhs = Self::type_spec_to_expr(*rhs)?;
                let op = Token::new(
                    TokenKind::OrOp,
                    "or",
                    lhs.ln_begin().unwrap(),
                    lhs.col_end().unwrap(),
                );
                let bin = BinOp::new(op, lhs, rhs);
                Ok(Expr::BinOp(bin))
            }
            TypeSpec::And(lhs, rhs) => {
                let lhs = Self::type_spec_to_expr(*lhs)?;
                let rhs = Self::type_spec_to_expr(*rhs)?;
                let op = Token::new(
                    TokenKind::AndOp,
                    "and",
                    lhs.ln_begin().unwrap(),
                    lhs.col_end().unwrap(),
                );
                let bin = BinOp::new(op, lhs, rhs);
                Ok(Expr::BinOp(bin))
            }
            other => Err(ParseError::feature_error(
                line!() as usize,
                other.loc(),
                "compound type spec to expr conversion",
            )),
        }
    }
}
