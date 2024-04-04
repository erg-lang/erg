use erg_common::traits::{Locational, Stream};
use erg_common::{fn_name, log, set};

use crate::ast::*;
use crate::debug_call_info;
use crate::debug_exit_info;
use crate::error::{ParseError, ParseResult};
use crate::token::{Token, TokenKind};
use crate::Parser;

impl Parser {
    /// Call: F(x) -> SubrSignature: F(x)
    pub(crate) fn convert_rhs_to_sig(&mut self, rhs: Expr) -> ParseResult<Signature> {
        debug_call_info!(self);
        match rhs {
            Expr::Accessor(accessor) => {
                let var = self
                    .convert_accessor_to_var_sig(accessor)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(Signature::Var(var))
            }
            Expr::Call(call) => {
                let subr = self
                    .convert_call_to_subr_sig(call)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(Signature::Subr(subr))
            }
            Expr::List(list) => {
                let list_pat = self
                    .convert_list_to_list_pat(list)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let var = VarSignature::new(VarPattern::List(list_pat), None);
                debug_exit_info!(self);
                Ok(Signature::Var(var))
            }
            Expr::Record(record) => {
                let record_pat = self
                    .convert_record_to_record_pat(record)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let var = VarSignature::new(VarPattern::Record(record_pat), None);
                debug_exit_info!(self);
                Ok(Signature::Var(var))
            }
            Expr::DataPack(pack) => {
                let data_pack = self
                    .convert_data_pack_to_data_pack_pat(pack)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let var = VarSignature::new(VarPattern::DataPack(data_pack), None);
                debug_exit_info!(self);
                Ok(Signature::Var(var))
            }
            Expr::Tuple(tuple) => {
                let tuple_pat = self
                    .convert_tuple_to_tuple_pat(tuple)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let var = VarSignature::new(VarPattern::Tuple(tuple_pat), None);
                debug_exit_info!(self);
                Ok(Signature::Var(var))
            }
            Expr::TypeAscription(tasc) => {
                let sig = self
                    .convert_type_asc_to_sig(tasc)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(sig)
            }
            other => {
                debug_exit_info!(self);
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
                debug_exit_info!(self);
                Ok(VarSignature::new(pat, None))
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                debug_exit_info!(self);
                Err(())
            }
        }
    }

    fn convert_list_to_list_pat(&mut self, list: List) -> ParseResult<VarListPattern> {
        debug_call_info!(self);
        match list {
            List::Normal(lis) => {
                let mut vars = Vars::empty();
                for elem in lis.elems.pos_args {
                    let pat = self
                        .convert_rhs_to_sig(elem.expr)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    match pat {
                        Signature::Var(v) => {
                            vars.push(v);
                        }
                        Signature::Subr(subr) => {
                            let err = ParseError::simple_syntax_error(line!() as usize, subr.loc());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                if let Some(var) = lis.elems.var_args {
                    let pat = self
                        .convert_rhs_to_sig(var.expr)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    match pat {
                        Signature::Var(v) => {
                            vars.starred = Some(Box::new(v));
                        }
                        Signature::Subr(subr) => {
                            let err = ParseError::simple_syntax_error(line!() as usize, subr.loc());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                let pat = VarListPattern::new(lis.l_sqbr, vars, lis.r_sqbr);
                debug_exit_info!(self);
                Ok(pat)
            }
            List::Comprehension(lis) => {
                let err = ParseError::simple_syntax_error(line!() as usize, lis.loc());
                self.errs.push(err);
                debug_exit_info!(self);
                Err(())
            }
            List::WithLength(lis) => {
                let err = ParseError::feature_error(
                    line!() as usize,
                    lis.loc(),
                    "list-with-length pattern",
                );
                self.errs.push(err);
                debug_exit_info!(self);
                Err(())
            }
        }
    }

    fn convert_def_to_var_record_attr(&mut self, mut attr: Def) -> ParseResult<VarRecordAttr> {
        debug_call_info!(self);
        let Signature::Var(VarSignature {
            pat: VarPattern::Ident(lhs),
            ..
        }) = attr.sig
        else {
            let err = ParseError::simple_syntax_error(line!() as usize, attr.sig.loc());
            self.errs.push(err);
            return Err(());
        };
        assert_eq!(attr.body.block.len(), 1);
        let first = attr.body.block.remove(0);
        let Expr::Accessor(rhs) = first else {
            let err = ParseError::simple_syntax_error(line!() as usize, first.loc());
            self.errs.push(err);
            return Err(());
        };
        let rhs = self.convert_accessor_to_var_sig(rhs)?;
        debug_exit_info!(self);
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
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let attrs = VarRecordAttrs::new(pats);
                debug_exit_info!(self);
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
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let attrs = VarRecordAttrs::new(pats);
                debug_exit_info!(self);
                Ok(VarRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
        }
    }

    fn convert_data_pack_to_data_pack_pat(
        &mut self,
        pack: DataPack,
    ) -> ParseResult<VarDataPackPattern> {
        debug_call_info!(self);
        let class = Self::expr_to_type_spec(*pack.class.clone()).map_err(|e| self.errs.push(e))?;
        let args = self
            .convert_record_to_record_pat(pack.args)
            .map_err(|_| self.stack_dec(fn_name!()))?;
        debug_exit_info!(self);
        Ok(VarDataPackPattern::new(class, pack.class, args))
    }

    fn convert_tuple_to_tuple_pat(&mut self, tuple: Tuple) -> ParseResult<VarTuplePattern> {
        debug_call_info!(self);
        let mut vars = Vars::empty();
        match tuple {
            Tuple::Normal(tup) => {
                let (pos_args, var_args, _kw_args, _kw_var, paren) = tup.elems.deconstruct();
                for arg in pos_args {
                    let sig = self
                        .convert_rhs_to_sig(arg.expr)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    match sig {
                        Signature::Var(var) => {
                            vars.push(var);
                        }
                        other => {
                            let err =
                                ParseError::simple_syntax_error(line!() as usize, other.loc());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                if let Some(var_args) = var_args {
                    let sig = self
                        .convert_rhs_to_sig(var_args.expr)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    match sig {
                        Signature::Var(var) => {
                            vars.starred = Some(Box::new(var));
                        }
                        other => {
                            let err =
                                ParseError::simple_syntax_error(line!() as usize, other.loc());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                let tuple = VarTuplePattern::new(paren, vars);
                debug_exit_info!(self);
                Ok(tuple)
            }
        }
    }

    fn convert_type_asc_to_sig(&mut self, tasc: TypeAscription) -> ParseResult<Signature> {
        debug_call_info!(self);
        let sig = self
            .convert_rhs_to_sig(*tasc.expr)
            .map_err(|_| self.stack_dec(fn_name!()))?;
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
        debug_exit_info!(self);
        Ok(sig)
    }

    fn convert_call_to_subr_sig(&mut self, call: Call) -> ParseResult<SubrSignature> {
        debug_call_info!(self);
        let (ident, bounds) = match *call.obj {
            Expr::Accessor(acc) => self
                .convert_accessor_to_ident(acc)
                .map_err(|_| self.stack_dec(fn_name!()))?,
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            }
        };
        let params = self
            .convert_args_to_params(call.args)
            .map_err(|_| self.stack_dec(fn_name!()))?;
        let sig = SubrSignature::new(set! {}, ident, bounds, params, None);
        debug_exit_info!(self);
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
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let Signature::Var(VarSignature {
                    pat: VarPattern::Ident(ident),
                    ..
                }) = sig
                else {
                    let err = ParseError::simple_syntax_error(line!() as usize, sig.loc());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                };
                let bounds = self
                    .convert_type_args_to_bounds(t_app.type_args)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                (ident, bounds)
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            }
        };
        debug_exit_info!(self);
        Ok((ident, bounds))
    }

    pub(crate) fn convert_type_args_to_bounds(
        &mut self,
        type_args: TypeAppArgs,
    ) -> ParseResult<TypeBoundSpecs> {
        debug_call_info!(self);
        let TypeAppArgsKind::Args(args) = type_args.args else {
            debug_exit_info!(self);
            return Ok(TypeBoundSpecs::empty());
        };
        let mut bounds = vec![];
        let (pos_args, _var_args, _kw_args, _kw_var, _paren) = args.deconstruct();
        for arg in pos_args.into_iter() {
            let bound = self
                .convert_type_arg_to_bound(arg)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            bounds.push(bound);
        }
        let bounds = TypeBoundSpecs::new(bounds);
        debug_exit_info!(self);
        Ok(bounds)
    }

    fn convert_type_arg_to_bound(&mut self, arg: PosArg) -> ParseResult<TypeBoundSpec> {
        match arg.expr {
            Expr::TypeAscription(tasc) => {
                let lhs = self
                    .convert_rhs_to_sig(*tasc.expr)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let Signature::Var(VarSignature {
                    pat: VarPattern::Ident(lhs),
                    ..
                }) = lhs
                else {
                    let err = ParseError::simple_syntax_error(line!() as usize, lhs.loc());
                    self.errs.push(err);
                    return Err(());
                };
                let bound = TypeBoundSpec::non_default(lhs.name, tasc.t_spec);
                Ok(bound)
            }
            Expr::Accessor(Accessor::Ident(ident)) => {
                let bound = TypeBoundSpec::Omitted(ident.name);
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
        let (pos_args, var_args, kw_args, kw_var, parens) = args.deconstruct();
        let mut params = Params::new(vec![], None, vec![], None, parens);
        for (i, arg) in pos_args.into_iter().enumerate() {
            let nd_param = self
                .convert_pos_arg_to_non_default_param(arg, i == 0)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            params.non_defaults.push(nd_param);
        }
        if let Some(var_args) = var_args {
            let var_params = self
                .convert_pos_arg_to_non_default_param(var_args, false)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            params.var_params = Some(Box::new(var_params));
        }
        // TODO: varargs
        for arg in kw_args.into_iter() {
            let d_param = self
                .convert_kw_arg_to_default_param(arg)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            params.defaults.push(d_param);
        }
        if let Some(kw_var) = kw_var {
            let kw_var_params = self
                .convert_pos_arg_to_non_default_param(kw_var, false)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            params.kw_var_params = Some(Box::new(kw_var_params));
        }
        debug_exit_info!(self);
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
            .map_err(|_| self.stack_dec(fn_name!()))?;
        debug_exit_info!(self);
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
                    let err = ParseError::simple_syntax_error(line!() as usize, ident.loc());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
                // FIXME deny: public
                let pat = ParamPattern::VarName(ident.name);
                let param = NonDefaultParamSignature::new(pat, None);
                debug_exit_info!(self);
                Ok(param)
            }
            Expr::Literal(lit) => {
                let pat = ParamPattern::Lit(lit);
                let param = NonDefaultParamSignature::new(pat, None);
                debug_exit_info!(self);
                Ok(param)
            }
            Expr::List(list) => {
                let list_pat = self
                    .convert_list_to_param_list_pat(list)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let pat = ParamPattern::List(list_pat);
                let param = NonDefaultParamSignature::new(pat, None);
                debug_exit_info!(self);
                Ok(param)
            }
            Expr::Record(record) => {
                let record_pat = self
                    .convert_record_to_param_record_pat(record)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let pat = ParamPattern::Record(record_pat);
                let param = NonDefaultParamSignature::new(pat, None);
                debug_exit_info!(self);
                Ok(param)
            }
            Expr::Tuple(tuple) => {
                let tuple_pat = self
                    .convert_tuple_to_param_tuple_pat(tuple)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let pat = ParamPattern::Tuple(tuple_pat);
                let param = NonDefaultParamSignature::new(pat, None);
                debug_exit_info!(self);
                Ok(param)
            }
            Expr::TypeAscription(tasc) => {
                let param = self
                    .convert_type_asc_to_param_pattern(tasc, allow_self)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(param)
            }
            Expr::UnaryOp(unary) => match unary.op.kind {
                TokenKind::RefOp => {
                    let var = unary.args.into_iter().next().unwrap();
                    let Expr::Accessor(Accessor::Ident(var)) = *var else {
                        let err = ParseError::simple_syntax_error(line!() as usize, var.loc());
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    };
                    let pat = ParamPattern::Ref(var.name);
                    let param = NonDefaultParamSignature::new(pat, None);
                    debug_exit_info!(self);
                    Ok(param)
                }
                TokenKind::RefMutOp => {
                    let var = unary.args.into_iter().next().unwrap();
                    let Expr::Accessor(Accessor::Ident(var)) = *var else {
                        let err = ParseError::simple_syntax_error(line!() as usize, var.loc());
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    };
                    let pat = ParamPattern::RefMut(var.name);
                    let param = NonDefaultParamSignature::new(pat, None);
                    debug_exit_info!(self);
                    Ok(param)
                }
                // TODO: Spread
                _other => {
                    let err = ParseError::simple_syntax_error(line!() as usize, unary.loc());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    Err(())
                }
            },
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                debug_exit_info!(self);
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
        debug_exit_info!(self);
        Ok(param)
    }

    fn convert_list_to_param_list_pat(&mut self, list: List) -> ParseResult<ParamListPattern> {
        debug_call_info!(self);
        match list {
            List::Normal(lis) => {
                let mut params = vec![];
                for arg in lis.elems.into_iters().0 {
                    params.push(self.convert_pos_arg_to_non_default_param(arg, false)?);
                }
                let params = Params::new(params, None, vec![], None, None);
                debug_exit_info!(self);
                Ok(ParamListPattern::new(lis.l_sqbr, params, lis.r_sqbr))
            }
            other => {
                let err = ParseError::feature_error(line!() as usize, other.loc(), "?");
                self.errs.push(err);
                debug_exit_info!(self);
                Err(())
            }
        }
    }

    fn convert_def_to_param_record_attr(&mut self, mut attr: Def) -> ParseResult<ParamRecordAttr> {
        let Signature::Var(VarSignature {
            pat: VarPattern::Ident(lhs),
            ..
        }) = attr.sig
        else {
            let err = ParseError::simple_syntax_error(line!() as usize, attr.sig.loc());
            self.errs.push(err);
            debug_exit_info!(self);
            return Err(());
        };
        assert_eq!(attr.body.block.len(), 1);
        let first = attr.body.block.remove(0);
        let rhs = self.convert_rhs_to_param(first, false)?;
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
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let attrs = ParamRecordAttrs::new(pats);
                self.stack_dec(fn_name!());
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
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let attrs = ParamRecordAttrs::new(pats);
                self.stack_dec(fn_name!());
                Ok(ParamRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
        }
    }

    fn convert_tuple_to_param_tuple_pat(&mut self, tuple: Tuple) -> ParseResult<ParamTuplePattern> {
        debug_call_info!(self);
        match tuple {
            Tuple::Normal(tup) => {
                let mut params = vec![];
                let (elems, var_args, _, _, parens) = tup.elems.deconstruct();
                for arg in elems.into_iter() {
                    params.push(self.convert_pos_arg_to_non_default_param(arg, false)?);
                }
                let var_params = if let Some(var_args) = var_args {
                    let var_params = self.convert_pos_arg_to_non_default_param(var_args, false)?;
                    Some(var_params)
                } else {
                    None
                };
                let params = Params::new(params, var_params, vec![], None, parens);
                debug_exit_info!(self);
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
            .map_err(|_| self.stack_dec(fn_name!()))?;
        let param = NonDefaultParamSignature::new(param.pat, Some(tasc.t_spec));
        debug_exit_info!(self);
        Ok(param)
    }

    pub(crate) fn convert_rhs_to_lambda_sig(&mut self, rhs: Expr) -> ParseResult<LambdaSignature> {
        debug_call_info!(self);
        match rhs {
            Expr::Literal(lit) => {
                let param = NonDefaultParamSignature::new(ParamPattern::Lit(lit), None);
                let params = Params::single(param);
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Accessor(accessor) => {
                let param = self
                    .convert_accessor_to_param_sig(accessor)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let params = Params::single(param);
                debug_exit_info!(self);
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Call(call) => {
                let param = self
                    .convert_call_to_param_sig(call)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let params = Params::single(param);
                debug_exit_info!(self);
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Tuple(tuple) => {
                let params = self
                    .convert_tuple_to_params(tuple)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::List(list) => {
                let lis = self
                    .convert_list_to_param_list_pat(list)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let param = NonDefaultParamSignature::new(ParamPattern::List(lis), None);
                let params = Params::single(param);
                debug_exit_info!(self);
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Record(record) => {
                let rec = self
                    .convert_record_to_param_record_pat(record)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let param = NonDefaultParamSignature::new(ParamPattern::Record(rec), None);
                let params = Params::single(param);
                debug_exit_info!(self);
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::TypeAscription(tasc) => {
                let sig = self
                    .convert_type_asc_to_lambda_sig(tasc)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(sig)
            }
            Expr::BinOp(bin) => match bin.op.kind {
                TokenKind::OrOp | TokenKind::AndOp => {
                    let pat = ParamPattern::Discard(bin.op.clone());
                    let expr = Expr::BinOp(bin);
                    let t_spec = Self::expr_to_type_spec(expr.clone())
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    let t_spec = TypeSpecWithOp::new(Token::DUMMY, t_spec, expr);
                    let param = NonDefaultParamSignature::new(pat, Some(t_spec));
                    let params = Params::single(param);
                    Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
                }
                _ => {
                    let err = ParseError::simple_syntax_error(line!() as usize, bin.loc());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    Err(())
                }
            },
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                debug_exit_info!(self);
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
                debug_exit_info!(self);
                Ok(NonDefaultParamSignature::new(pat, None))
            }
            other => {
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                debug_exit_info!(self);
                Err(())
            }
        }
    }

    fn convert_call_to_param_sig(&mut self, call: Call) -> ParseResult<NonDefaultParamSignature> {
        let predecl = Self::call_to_predecl_type_spec(call.clone()).map_err(|_| ())?;
        let t_spec =
            TypeSpecWithOp::new(Token::DUMMY, TypeSpec::PreDeclTy(predecl), Expr::Call(call));
        Ok(NonDefaultParamSignature::new(
            ParamPattern::Discard(Token::DUMMY),
            Some(t_spec),
        ))
    }

    fn convert_tuple_to_params(&mut self, tuple: Tuple) -> ParseResult<Params> {
        debug_call_info!(self);
        match tuple {
            Tuple::Normal(tup) => {
                let (pos_args, var_args, kw_args, kw_var, paren) = tup.elems.deconstruct();
                let mut params = Params::new(vec![], None, vec![], None, paren);
                for (i, arg) in pos_args.into_iter().enumerate() {
                    let param = self
                        .convert_pos_arg_to_non_default_param(arg, i == 0)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    params.non_defaults.push(param);
                }
                if let Some(var_args) = var_args {
                    let param = self
                        .convert_pos_arg_to_non_default_param(var_args, false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    params.var_params = Some(Box::new(param));
                }
                for arg in kw_args {
                    let param = self
                        .convert_kw_arg_to_default_param(arg)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    params.defaults.push(param);
                }
                if let Some(kw_var) = kw_var {
                    let param = self
                        .convert_pos_arg_to_non_default_param(kw_var, false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    params.kw_var_params = Some(Box::new(param));
                }
                debug_exit_info!(self);
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
            .convert_rhs_to_param(Expr::TypeAscription(tasc), true)
            .map_err(|_| self.stack_dec(fn_name!()))?;
        debug_exit_info!(self);
        Ok(LambdaSignature::new(
            Params::single(sig),
            None,
            TypeBoundSpecs::empty(),
        ))
    }
}
