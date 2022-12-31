use erg_common::traits::{Locational, Stream};
use erg_common::{fn_name, log, option_enum_unwrap, set};

use crate::ast::*;
use crate::debug_call_info;
use crate::error::{ParseError, ParseResult};
use crate::token::TokenKind;
use crate::Parser;

impl Parser {
    /// Call: F(x) -> SubrSignature: F(x)
    pub(crate) fn convert_rhs_to_sig(&mut self, rhs: Expr) -> ParseResult<Signature> {
        debug_call_info!(self);
        match rhs {
            Expr::Accessor(accessor) => {
                let var = self
                    .convert_accessor_to_var_sig(accessor)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(Signature::Var(var))
            }
            Expr::Call(call) => {
                let subr = self
                    .convert_call_to_subr_sig(call)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(Signature::Subr(subr))
            }
            Expr::Array(array) => {
                let array_pat = self
                    .convert_array_to_array_pat(array)
                    .map_err(|_| self.stack_dec())?;
                let var = VarSignature::new(VarPattern::Array(array_pat), None);
                self.level -= 1;
                Ok(Signature::Var(var))
            }
            Expr::Record(record) => {
                let record_pat = self
                    .convert_record_to_record_pat(record)
                    .map_err(|_| self.stack_dec())?;
                let var = VarSignature::new(VarPattern::Record(record_pat), None);
                self.level -= 1;
                Ok(Signature::Var(var))
            }
            Expr::DataPack(pack) => {
                let data_pack = self
                    .convert_data_pack_to_data_pack_pat(pack)
                    .map_err(|_| self.stack_dec())?;
                let var = VarSignature::new(VarPattern::DataPack(data_pack), None);
                self.level -= 1;
                Ok(Signature::Var(var))
            }
            Expr::Tuple(tuple) => {
                let tuple_pat = self
                    .convert_tuple_to_tuple_pat(tuple)
                    .map_err(|_| self.stack_dec())?;
                let var = VarSignature::new(VarPattern::Tuple(tuple_pat), None);
                self.level -= 1;
                Ok(Signature::Var(var))
            }
            Expr::TypeAsc(tasc) => {
                let sig = self
                    .convert_type_asc_to_sig(tasc)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(sig)
            }
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_accessor_to_var_sig(&mut self, accessor: Accessor) -> ParseResult<VarSignature> {
        debug_call_info!(self);
        match accessor {
            Accessor::Ident(ident) => {
                let pat = if &ident.inspect()[..] == "_" {
                    VarPattern::Discard(ident.name.into_token())
                } else {
                    VarPattern::Ident(ident)
                };
                self.level -= 1;
                Ok(VarSignature::new(pat, None))
            }
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_array_to_array_pat(&mut self, array: Array) -> ParseResult<VarArrayPattern> {
        debug_call_info!(self);
        match array {
            Array::Normal(arr) => {
                let mut vars = Vars::empty();
                for elem in arr.elems.into_iters().0 {
                    let pat = self
                        .convert_rhs_to_sig(elem.expr)
                        .map_err(|_| self.stack_dec())?;
                    match pat {
                        Signature::Var(v) => {
                            vars.push(v);
                        }
                        Signature::Subr(subr) => {
                            self.level -= 1;
                            let err = ParseError::simple_syntax_error(line!() as usize, subr.loc());
                            self.errs.push(err);
                            return Err(());
                        }
                    }
                }
                let pat = VarArrayPattern::new(arr.l_sqbr, vars, arr.r_sqbr);
                self.level -= 1;
                Ok(pat)
            }
            Array::Comprehension(arr) => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, arr.loc());
                self.errs.push(err);
                Err(())
            }
            Array::WithLength(arr) => {
                self.level -= 1;
                let err = ParseError::feature_error(
                    line!() as usize,
                    arr.loc(),
                    "array-with-length pattern",
                );
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_def_to_var_record_attr(&mut self, mut attr: Def) -> ParseResult<VarRecordAttr> {
        let lhs = option_enum_unwrap!(attr.sig, Signature::Var).unwrap_or_else(|| todo!());
        let lhs = option_enum_unwrap!(lhs.pat, VarPattern::Ident).unwrap_or_else(|| todo!());
        assert_eq!(attr.body.block.len(), 1);
        let rhs = option_enum_unwrap!(attr.body.block.remove(0), Expr::Accessor)
            .unwrap_or_else(|| todo!());
        let rhs = self.convert_accessor_to_var_sig(rhs)?;
        Ok(VarRecordAttr::new(lhs, rhs))
    }

    fn convert_record_to_record_pat(&mut self, record: Record) -> ParseResult<VarRecordPattern> {
        debug_call_info!(self);
        match record {
            Record::Normal(rec) => {
                let pats = rec
                    .attrs
                    .into_iter()
                    .map(|attr| self.convert_def_to_var_record_attr(attr))
                    .collect::<ParseResult<Vec<_>>>()
                    .map_err(|_| self.stack_dec())?;
                let attrs = VarRecordAttrs::new(pats);
                self.stack_dec();
                Ok(VarRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
            Record::Mixed(rec) => {
                let pats = rec
                    .attrs
                    .into_iter()
                    .map(|attr_or_ident| match attr_or_ident {
                        RecordAttrOrIdent::Attr(attr) => self.convert_def_to_var_record_attr(attr),
                        RecordAttrOrIdent::Ident(ident) => {
                            let rhs = VarSignature::new(VarPattern::Ident(ident.clone()), None);
                            Ok(VarRecordAttr::new(ident, rhs))
                        }
                    })
                    .collect::<ParseResult<Vec<_>>>()
                    .map_err(|_| self.stack_dec())?;
                let attrs = VarRecordAttrs::new(pats);
                self.stack_dec();
                Ok(VarRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
        }
    }

    fn convert_data_pack_to_data_pack_pat(
        &mut self,
        pack: DataPack,
    ) -> ParseResult<VarDataPackPattern> {
        debug_call_info!(self);
        let class = Self::expr_to_type_spec(*pack.class).map_err(|e| self.errs.push(e))?;
        let args = self
            .convert_record_to_record_pat(pack.args)
            .map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(VarDataPackPattern::new(class, args))
    }

    fn convert_tuple_to_tuple_pat(&mut self, tuple: Tuple) -> ParseResult<VarTuplePattern> {
        debug_call_info!(self);
        let mut vars = Vars::empty();
        match tuple {
            Tuple::Normal(tup) => {
                let (pos_args, _kw_args, paren) = tup.elems.deconstruct();
                for arg in pos_args {
                    let sig = self
                        .convert_rhs_to_sig(arg.expr)
                        .map_err(|_| self.stack_dec())?;
                    match sig {
                        Signature::Var(var) => {
                            vars.push(var);
                        }
                        other => {
                            self.level -= 1;
                            let err =
                                ParseError::simple_syntax_error(line!() as usize, other.loc());
                            self.errs.push(err);
                            return Err(());
                        }
                    }
                }
                let tuple = VarTuplePattern::new(paren, vars);
                self.level -= 1;
                Ok(tuple)
            }
        }
    }

    fn convert_type_asc_to_sig(&mut self, tasc: TypeAscription) -> ParseResult<Signature> {
        debug_call_info!(self);
        let sig = self
            .convert_rhs_to_sig(*tasc.expr)
            .map_err(|_| self.stack_dec())?;
        let sig = match sig {
            Signature::Var(var) => {
                let var = VarSignature::new(var.pat, Some(tasc.t_spec));
                Signature::Var(var)
            }
            Signature::Subr(subr) => {
                let subr = SubrSignature::new(
                    subr.decorators,
                    subr.ident,
                    subr.bounds,
                    subr.params,
                    Some(tasc.t_spec),
                );
                Signature::Subr(subr)
            }
        };
        self.level -= 1;
        Ok(sig)
    }

    fn convert_call_to_subr_sig(&mut self, call: Call) -> ParseResult<SubrSignature> {
        debug_call_info!(self);
        let (ident, bounds) = match *call.obj {
            Expr::Accessor(acc) => self
                .convert_accessor_to_ident(acc)
                .map_err(|_| self.stack_dec())?,
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                return Err(());
            }
        };
        let params = self
            .convert_args_to_params(call.args)
            .map_err(|_| self.stack_dec())?;
        let sig = SubrSignature::new(set! {}, ident, bounds, params, None);
        self.level -= 1;
        Ok(sig)
    }

    fn convert_accessor_to_ident(
        &mut self,
        accessor: Accessor,
    ) -> ParseResult<(Identifier, TypeBoundSpecs)> {
        debug_call_info!(self);
        let (ident, bounds) = match accessor {
            Accessor::Ident(ident) => (ident, TypeBoundSpecs::empty()),
            Accessor::TypeApp(t_app) => {
                let sig = self
                    .convert_rhs_to_sig(*t_app.obj)
                    .map_err(|_| self.stack_dec())?;
                let pat = option_enum_unwrap!(sig, Signature::Var)
                    .unwrap_or_else(|| todo!())
                    .pat;
                let ident = option_enum_unwrap!(pat, VarPattern::Ident).unwrap_or_else(|| todo!());
                let bounds = self
                    .convert_type_args_to_bounds(t_app.type_args)
                    .map_err(|_| self.stack_dec())?;
                (ident, bounds)
            }
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                return Err(());
            }
        };
        self.level -= 1;
        Ok((ident, bounds))
    }

    pub(crate) fn convert_type_args_to_bounds(
        &mut self,
        type_args: TypeAppArgs,
    ) -> ParseResult<TypeBoundSpecs> {
        debug_call_info!(self);
        let mut bounds = vec![];
        let (pos_args, _kw_args, _paren) = type_args.args.deconstruct();
        for arg in pos_args.into_iter() {
            let bound = self
                .convert_type_arg_to_bound(arg)
                .map_err(|_| self.stack_dec())?;
            bounds.push(bound);
        }
        let bounds = TypeBoundSpecs::new(bounds);
        self.level -= 1;
        Ok(bounds)
    }

    fn convert_type_arg_to_bound(&mut self, arg: PosArg) -> ParseResult<TypeBoundSpec> {
        match arg.expr {
            Expr::TypeAsc(tasc) => {
                let lhs = self
                    .convert_rhs_to_sig(*tasc.expr)
                    .map_err(|_| self.stack_dec())?;
                let lhs = option_enum_unwrap!(lhs, Signature::Var)
                    .unwrap_or_else(|| todo!())
                    .pat;
                let lhs = option_enum_unwrap!(lhs, VarPattern::Ident).unwrap_or_else(|| todo!());
                let spec_with_op = TypeSpecWithOp::new(tasc.op, tasc.t_spec);
                let bound = TypeBoundSpec::non_default(lhs.name.into_token(), spec_with_op);
                Ok(bound)
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                Err(())
            }
        }
    }

    pub(crate) fn convert_args_to_params(&mut self, args: Args) -> ParseResult<Params> {
        debug_call_info!(self);
        let (pos_args, kw_args, parens) = args.deconstruct();
        let mut params = Params::new(vec![], None, vec![], parens);
        for (i, arg) in pos_args.into_iter().enumerate() {
            let nd_param = self
                .convert_pos_arg_to_non_default_param(arg, i == 0)
                .map_err(|_| self.stack_dec())?;
            params.non_defaults.push(nd_param);
        }
        // TODO: varargs
        for arg in kw_args.into_iter() {
            let d_param = self
                .convert_kw_arg_to_default_param(arg)
                .map_err(|_| self.stack_dec())?;
            params.defaults.push(d_param);
        }
        self.level -= 1;
        Ok(params)
    }

    fn convert_pos_arg_to_non_default_param(
        &mut self,
        arg: PosArg,
        allow_self: bool,
    ) -> ParseResult<NonDefaultParamSignature> {
        debug_call_info!(self);
        let param = self
            .convert_rhs_to_param(arg.expr, allow_self)
            .map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(param)
    }

    fn convert_rhs_to_param(
        &mut self,
        expr: Expr,
        allow_self: bool,
    ) -> ParseResult<NonDefaultParamSignature> {
        debug_call_info!(self);
        match expr {
            Expr::Accessor(Accessor::Ident(ident)) => {
                if &ident.inspect()[..] == "self" && !allow_self {
                    self.level -= 1;
                    let err = ParseError::simple_syntax_error(line!() as usize, ident.loc());
                    self.errs.push(err);
                    return Err(());
                }
                // FIXME deny: public
                let pat = ParamPattern::VarName(ident.name);
                let param = NonDefaultParamSignature::new(pat, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::Lit(lit) => {
                let pat = ParamPattern::Lit(lit);
                let param = NonDefaultParamSignature::new(pat, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::Array(array) => {
                let array_pat = self
                    .convert_array_to_param_array_pat(array)
                    .map_err(|_| self.stack_dec())?;
                let pat = ParamPattern::Array(array_pat);
                let param = NonDefaultParamSignature::new(pat, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::Record(record) => {
                let record_pat = self
                    .convert_record_to_param_record_pat(record)
                    .map_err(|_| self.stack_dec())?;
                let pat = ParamPattern::Record(record_pat);
                let param = NonDefaultParamSignature::new(pat, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::Tuple(tuple) => {
                let tuple_pat = self
                    .convert_tuple_to_param_tuple_pat(tuple)
                    .map_err(|_| self.stack_dec())?;
                let pat = ParamPattern::Tuple(tuple_pat);
                let param = NonDefaultParamSignature::new(pat, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::TypeAsc(tasc) => {
                let param = self
                    .convert_type_asc_to_param_pattern(tasc, allow_self)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(param)
            }
            Expr::UnaryOp(unary) => match unary.op.kind {
                TokenKind::RefOp => {
                    let var = unary.args.into_iter().next().unwrap();
                    let var = option_enum_unwrap!(*var, Expr::Accessor:(Accessor::Ident:(_)))
                        .unwrap_or_else(|| todo!());
                    let pat = ParamPattern::Ref(var.name);
                    let param = NonDefaultParamSignature::new(pat, None);
                    self.level -= 1;
                    Ok(param)
                }
                TokenKind::RefMutOp => {
                    let var = unary.args.into_iter().next().unwrap();
                    let var = option_enum_unwrap!(*var, Expr::Accessor:(Accessor::Ident:(_)))
                        .unwrap_or_else(|| todo!());
                    let pat = ParamPattern::RefMut(var.name);
                    let param = NonDefaultParamSignature::new(pat, None);
                    self.level -= 1;
                    Ok(param)
                }
                // TODO: Spread
                _other => {
                    self.level -= 1;
                    let err = ParseError::simple_syntax_error(line!() as usize, unary.loc());
                    self.errs.push(err);
                    Err(())
                }
            },
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_kw_arg_to_default_param(
        &mut self,
        arg: KwArg,
    ) -> ParseResult<DefaultParamSignature> {
        debug_call_info!(self);
        let pat = ParamPattern::VarName(VarName::new(arg.keyword));
        let sig = NonDefaultParamSignature::new(pat, arg.t_spec);
        let param = DefaultParamSignature::new(sig, arg.expr);
        self.level -= 1;
        Ok(param)
    }

    fn convert_array_to_param_array_pat(&mut self, array: Array) -> ParseResult<ParamArrayPattern> {
        debug_call_info!(self);
        match array {
            Array::Normal(arr) => {
                let mut params = vec![];
                for arg in arr.elems.into_iters().0 {
                    params.push(self.convert_pos_arg_to_non_default_param(arg, false)?);
                }
                let params = Params::new(params, None, vec![], None);
                self.level -= 1;
                Ok(ParamArrayPattern::new(arr.l_sqbr, params, arr.r_sqbr))
            }
            other => {
                self.level -= 1;
                let err = ParseError::feature_error(line!() as usize, other.loc(), "?");
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_def_to_param_record_attr(&mut self, mut attr: Def) -> ParseResult<ParamRecordAttr> {
        let lhs = option_enum_unwrap!(attr.sig, Signature::Var).unwrap_or_else(|| todo!());
        let lhs = option_enum_unwrap!(lhs.pat, VarPattern::Ident).unwrap_or_else(|| todo!());
        assert_eq!(attr.body.block.len(), 1);
        let rhs = option_enum_unwrap!(attr.body.block.remove(0), Expr::Accessor)
            .unwrap_or_else(|| todo!());
        let rhs = self.convert_accessor_to_param_sig(rhs)?;
        Ok(ParamRecordAttr::new(lhs, rhs))
    }

    fn convert_record_to_param_record_pat(
        &mut self,
        record: Record,
    ) -> ParseResult<ParamRecordPattern> {
        debug_call_info!(self);
        match record {
            Record::Normal(rec) => {
                let pats = rec
                    .attrs
                    .into_iter()
                    .map(|attr| self.convert_def_to_param_record_attr(attr))
                    .collect::<ParseResult<Vec<_>>>()
                    .map_err(|_| self.stack_dec())?;
                let attrs = ParamRecordAttrs::new(pats);
                self.stack_dec();
                Ok(ParamRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
            Record::Mixed(rec) => {
                let pats = rec
                    .attrs
                    .into_iter()
                    .map(|attr_or_ident| match attr_or_ident {
                        RecordAttrOrIdent::Attr(attr) => {
                            self.convert_def_to_param_record_attr(attr)
                        }
                        RecordAttrOrIdent::Ident(ident) => {
                            let rhs = NonDefaultParamSignature::new(
                                ParamPattern::VarName(ident.name.clone()),
                                None,
                            );
                            Ok(ParamRecordAttr::new(ident, rhs))
                        }
                    })
                    .collect::<ParseResult<Vec<_>>>()
                    .map_err(|_| self.stack_dec())?;
                let attrs = ParamRecordAttrs::new(pats);
                self.stack_dec();
                Ok(ParamRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
        }
    }

    fn convert_tuple_to_param_tuple_pat(&mut self, tuple: Tuple) -> ParseResult<ParamTuplePattern> {
        debug_call_info!(self);
        match tuple {
            Tuple::Normal(tup) => {
                let mut params = vec![];
                let (elems, _, parens) = tup.elems.deconstruct();
                for arg in elems.into_iter() {
                    params.push(self.convert_pos_arg_to_non_default_param(arg, false)?);
                }
                let params = Params::new(params, None, vec![], parens);
                self.level -= 1;
                Ok(ParamTuplePattern::new(params))
            }
        }
    }

    fn convert_type_asc_to_param_pattern(
        &mut self,
        tasc: TypeAscription,
        allow_self: bool,
    ) -> ParseResult<NonDefaultParamSignature> {
        debug_call_info!(self);
        let param = self
            .convert_rhs_to_param(*tasc.expr, allow_self)
            .map_err(|_| self.stack_dec())?;
        let t_spec = TypeSpecWithOp::new(tasc.op, tasc.t_spec);
        let param = NonDefaultParamSignature::new(param.pat, Some(t_spec));
        self.level -= 1;
        Ok(param)
    }

    pub(crate) fn convert_rhs_to_lambda_sig(&mut self, rhs: Expr) -> ParseResult<LambdaSignature> {
        debug_call_info!(self);
        match rhs {
            Expr::Lit(lit) => {
                let param = NonDefaultParamSignature::new(ParamPattern::Lit(lit), None);
                let params = Params::new(vec![param], None, vec![], None);
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Accessor(accessor) => {
                let param = self
                    .convert_accessor_to_param_sig(accessor)
                    .map_err(|_| self.stack_dec())?;
                let params = Params::new(vec![param], None, vec![], None);
                self.level -= 1;
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Tuple(tuple) => {
                let params = self
                    .convert_tuple_to_params(tuple)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Array(array) => {
                let arr = self
                    .convert_array_to_param_array_pat(array)
                    .map_err(|_| self.stack_dec())?;
                let param = NonDefaultParamSignature::new(ParamPattern::Array(arr), None);
                let params = Params::new(vec![param], None, vec![], None);
                self.level -= 1;
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Record(record) => {
                let rec = self
                    .convert_record_to_param_record_pat(record)
                    .map_err(|_| self.stack_dec())?;
                let param = NonDefaultParamSignature::new(ParamPattern::Record(rec), None);
                let params = Params::new(vec![param], None, vec![], None);
                self.level -= 1;
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::TypeAsc(tasc) => {
                let sig = self
                    .convert_type_asc_to_lambda_sig(tasc)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(sig)
            }
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_accessor_to_param_sig(
        &mut self,
        accessor: Accessor,
    ) -> ParseResult<NonDefaultParamSignature> {
        debug_call_info!(self);
        match accessor {
            Accessor::Ident(ident) => {
                let pat = if &ident.name.inspect()[..] == "_" {
                    ParamPattern::Discard(ident.name.into_token())
                } else {
                    ParamPattern::VarName(ident.name)
                };
                self.level -= 1;
                Ok(NonDefaultParamSignature::new(pat, None))
            }
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_tuple_to_params(&mut self, tuple: Tuple) -> ParseResult<Params> {
        debug_call_info!(self);
        match tuple {
            Tuple::Normal(tup) => {
                let (pos_args, kw_args, paren) = tup.elems.deconstruct();
                let mut params = Params::new(vec![], None, vec![], paren);
                for (i, arg) in pos_args.into_iter().enumerate() {
                    let param = self
                        .convert_pos_arg_to_non_default_param(arg, i == 0)
                        .map_err(|_| self.stack_dec())?;
                    params.non_defaults.push(param);
                }
                for arg in kw_args {
                    let param = self
                        .convert_kw_arg_to_default_param(arg)
                        .map_err(|_| self.stack_dec())?;
                    params.defaults.push(param);
                }
                self.level -= 1;
                Ok(params)
            }
        }
    }

    fn convert_type_asc_to_lambda_sig(
        &mut self,
        tasc: TypeAscription,
    ) -> ParseResult<LambdaSignature> {
        debug_call_info!(self);
        let sig = self
            .convert_rhs_to_param(Expr::TypeAsc(tasc), true)
            .map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(LambdaSignature::new(
            Params::new(vec![sig], None, vec![], None),
            None,
            TypeBoundSpecs::empty(),
        ))
    }
}
