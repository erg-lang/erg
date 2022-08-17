//! implements `Parser`.
//!
//! パーサーを実装する
//!
use std::fmt::Debug;
use std::mem;

use erg_common::color::{GREEN, RED, RESET};
use erg_common::config::ErgConfig;
use erg_common::config::Input;
use erg_common::error::Location;
use erg_common::set::Set;
use erg_common::traits::Runnable;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{
    caused_by, debug_power_assert, enum_unwrap, fn_name, log, set, switch_lang, switch_unreachable,
};

use crate::ast::*;
use crate::desugar::Desugarer;
use crate::error::{ParseError, ParseErrors, ParseResult, ParserRunnerError, ParserRunnerErrors};
use crate::lex::Lexer;
use crate::token::{Token, TokenCategory, TokenKind, TokenStream};

use TokenCategory as TC;
use TokenKind::*;

/// Display the name of the called function for debugging the parser
/// I thought about displaying the nesting level, but due to `?` operator there is not a single exit point for the function
/// So it is not possible to lower the level
macro_rules! debug_call_info {
    ($self: ident) => {
        log!(
            "[DEBUG] entered {}, cur: {}",
            fn_name!(),
            $self.peek().unwrap()
        );
    };
}

enum ExprOrOp {
    Expr(Expr),
    Op(Token),
}

enum PosOrKwArg {
    Pos(PosArg),
    Kw(KwArg),
}

pub enum Side {
    LhsAssign,
    LhsLambda,
    Rhs,
}

/// To enhance error descriptions, the parsing process will continue as long as it's not fatal.
#[derive(Debug)]
pub struct Parser {
    counter: DefId,
    tokens: TokenStream,
    warns: ParseErrors,
    errs: ParseErrors,
}

impl Parser {
    const fn new(ts: TokenStream) -> Self {
        Self {
            counter: DefId(0),
            tokens: ts,
            warns: ParseErrors::empty(),
            errs: ParseErrors::empty(),
        }
    }

    #[inline]
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(0)
    }

    #[inline]
    fn nth(&self, idx: usize) -> Option<&Token> {
        self.tokens.get(idx)
    }

    #[inline]
    fn skip(&mut self) {
        self.tokens.remove(0);
    }

    #[inline]
    fn lpop(&mut self) -> Token {
        self.tokens.remove(0)
    }

    fn cur_category_is(&self, category: TokenCategory) -> bool {
        self.peek()
            .map(|t| t.category_is(category))
            .unwrap_or(false)
    }

    fn cur_is(&self, kind: TokenKind) -> bool {
        self.peek().map(|t| t.is(kind)).unwrap_or(false)
    }

    fn nth_is(&self, idx: usize, kind: TokenKind) -> bool {
        self.nth(idx).map(|t| t.is(kind)).unwrap_or(false)
    }

    fn nth_category(&self, idx: usize) -> Option<TokenCategory> {
        self.nth(idx).map(|t| t.category())
    }

    /// `+1`: true
    /// `+ 1`: false
    /// `F()`: true
    /// `F ()`: false
    fn cur_is_in_contact_with_next(&self) -> bool {
        let cur_loc = self.peek().unwrap().ln_end().unwrap();
        let next_loc = self.nth(1).unwrap().ln_end().unwrap();
        cur_loc + 1 == next_loc
    }

    /// returns if the current position is a left-hand side value.
    ///
    /// ```
    /// f(x: Int) = { y = x+1; z = [v: Int, w: Int] -> w + x }
    /// LhsAssign |      Rhs       |   LhsLambda    |   Rhs
    /// ```
    /// `(Rhs) ; (LhsAssign) =`
    /// `(Rhs) ; (LhsLambda) ->`
    /// `(Rhs) , (LhsLambda) ->`
    /// `(Rhs) (LhsLambda) -> (Rhs);`
    fn cur_side(&self) -> Side {
        // 以降に=, ->などがないならすべて右辺値
        let opt_equal_pos = self.tokens.iter().skip(1).position(|t| t.is(Equal));
        let opt_arrow_pos = self
            .tokens
            .iter()
            .skip(1)
            .position(|t| t.category_is(TC::LambdaOp));
        let opt_sep_pos = self
            .tokens
            .iter()
            .skip(1)
            .position(|t| t.category_is(TC::Separator));
        match (opt_equal_pos, opt_arrow_pos, opt_sep_pos) {
            (Some(equal), Some(arrow), Some(sep)) => {
                let min = [equal, arrow, sep].into_iter().min().unwrap();
                if min == sep {
                    Side::Rhs
                } else if min == equal {
                    Side::LhsAssign
                } else {
                    // (cur) -> ... = ... ;
                    if equal < sep {
                        Side::LhsAssign
                    }
                    // (cur) -> ... ; ... =
                    else if self.arrow_distance(0, 0) == 1 {
                        Side::LhsLambda
                    } else {
                        Side::Rhs
                    }
                }
            }
            // (cur) = ... -> ...
            // (cur) -> ... = ...
            (Some(_eq), Some(_arrow), None) => Side::LhsAssign,
            // (cur) = ... ;
            // (cur) ; ... =
            (Some(equal), None, Some(sep)) => {
                if equal < sep {
                    Side::LhsAssign
                } else {
                    Side::Rhs
                }
            }
            (None, Some(arrow), Some(sep)) => {
                // (cur) -> ... ;
                if arrow < sep {
                    if self.arrow_distance(0, 0) == 1 {
                        Side::LhsLambda
                    } else {
                        Side::Rhs
                    }
                }
                // (cur) ; ... ->
                else {
                    Side::Rhs
                }
            }
            (Some(_eq), None, None) => Side::LhsAssign,
            (None, Some(_arrow), None) => {
                if self.arrow_distance(0, 0) == 1 {
                    Side::LhsLambda
                } else {
                    Side::Rhs
                }
            }
            (None, None, Some(_)) | (None, None, None) => Side::Rhs,
        }
    }

    /// `->`: 0
    /// `i ->`: 1
    /// `i: Int ->`: 1
    /// `a: Array(Int) ->`: 1
    /// `(i, j) ->`: 1
    /// `F () ->`: 2
    /// `F() ->`: 1
    /// `if True, () ->`: 3
    fn arrow_distance(&self, cur: usize, enc_nest_level: usize) -> usize {
        match self.nth_category(cur).unwrap() {
            TC::LambdaOp => 0,
            TC::LEnclosure => {
                if self.nth_category(cur + 1).unwrap() == TC::REnclosure {
                    1 + self.arrow_distance(cur + 2, enc_nest_level)
                } else {
                    self.arrow_distance(cur + 1, enc_nest_level + 1)
                }
            }
            TC::REnclosure => self.arrow_distance(cur + 1, enc_nest_level - 1),
            _ => match self.nth_category(cur + 1).unwrap() {
                TC::SpecialBinOp => self.arrow_distance(cur + 1, enc_nest_level),
                TC::LEnclosure if self.cur_is_in_contact_with_next() => {
                    self.arrow_distance(cur + 2, enc_nest_level + 1)
                }
                _ if enc_nest_level == 0 => 1 + self.arrow_distance(cur + 1, enc_nest_level),
                _ => self.arrow_distance(cur + 1, enc_nest_level),
            },
        }
    }

    /// 解析を諦めて次の解析できる要素に移行する
    fn next_expr(&mut self) {
        while let Some(t) = self.peek() {
            match t.category() {
                TC::Separator | TC::DefOp | TC::LambdaOp => {
                    self.skip();
                    return;
                }
                TC::EOF => {
                    return;
                }
                _ => {
                    self.skip();
                }
            }
        }
    }

    fn skip_and_throw_syntax_err(&mut self, caused_by: &str) -> ParseError {
        let loc = self.peek().unwrap().loc();
        log!("{RED}[DEBUG] error caused by: {caused_by}{GREEN}");
        self.next_expr();
        ParseError::simple_syntax_error(0, loc)
    }

    #[inline]
    fn restore(&mut self, token: Token) {
        self.tokens.insert(0, token);
    }
}

#[derive(Debug)]
pub struct ParserRunner {
    cfg: ErgConfig,
}

impl Runnable for ParserRunner {
    type Err = ParserRunnerError;
    type Errs = ParserRunnerErrors;
    const NAME: &'static str = "Erg parser";

    #[inline]
    fn new(cfg: ErgConfig) -> Self {
        Self { cfg }
    }

    #[inline]
    fn input(&self) -> &Input {
        &self.cfg.input
    }

    #[inline]
    fn finish(&mut self) {}

    #[inline]
    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<(), Self::Errs> {
        todo!()
    }

    fn eval(&mut self, src: Str) -> Result<String, ParserRunnerErrors> {
        let ast = Self::parse_from_str(src)?;
        Ok(format!("{ast}"))
    }
}

impl ParserRunner {
    pub fn parse_token_stream(&mut self, ts: TokenStream) -> Result<AST, ParserRunnerErrors> {
        Parser::new(ts)
            .parse(Str::ever(self.cfg.module))
            .map_err(|errs| ParserRunnerErrors::convert(self.input(), errs))
    }

    pub fn parse(&mut self) -> Result<AST, ParserRunnerErrors> {
        let ts = Lexer::new(self.input().clone())
            .lex()
            .map_err(|errs| ParserRunnerErrors::convert(self.input(), errs))?;
        self.parse_token_stream(ts)
    }

    pub fn parse_from_str(src: Str) -> Result<AST, ParserRunnerErrors> {
        let mut cfg = ErgConfig::default();
        cfg.input = Input::Str(src);
        let mut self_ = Self::new(cfg);
        self_.parse()
    }
}

impl Parser {
    pub fn parse(&mut self, mod_name: Str) -> Result<AST, ParseErrors> {
        if self.tokens.is_empty() {
            return Ok(AST::new(mod_name, Module::empty()));
        }
        log!("{GREEN}[DEBUG] the parsing process has started.");
        log!("token stream: {}", self.tokens);
        let module = match self.try_reduce_module() {
            Ok(module) => module,
            Err(e) => {
                self.errs.push(e);
                return Err(mem::take(&mut self.errs));
            }
        };
        if !self.cur_is(EOF) {
            let loc = self.peek().unwrap().loc();
            self.errs
                .push(ParseError::compiler_bug(0, loc, fn_name!(), line!()));
            return Err(mem::take(&mut self.errs));
        }
        log!("[DEBUG] the parsing process has completed.");
        log!("AST:\n{module}");
        log!("[DEBUG] the desugaring process has started.");
        let mut desugarer = Desugarer::new();
        let module = desugarer.desugar(module);
        log!("AST (desugared):\n{module}");
        log!("[DEBUG] the desugaring process has completed.{RESET}");
        if self.errs.is_empty() {
            Ok(AST::new(mod_name, module))
        } else {
            Err(mem::take(&mut self.errs))
        }
    }

    /// 構文の最大単位であるモジュールに還元する(これが呼ばれるのは一度きり)
    /// プログラムのトップレベルに来てよいのは単独の式のみなので、例えばBinOpが先頭に来るのは構文エラー
    #[inline]
    fn try_reduce_module(&mut self) -> ParseResult<Module> {
        debug_call_info!(self);
        let mut chunks = Module::empty();
        loop {
            match self.peek() {
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(t) if t.is(EOF) => {
                    break;
                }
                Some(t) if t.is(Indent) || t.is(Dedent) => {
                    switch_unreachable!()
                }
                Some(_) => match self.try_reduce_expr() {
                    Ok(expr) => {
                        chunks.push(expr);
                    }
                    Err(e) => {
                        self.errs.push(e);
                    }
                },
                _ => switch_unreachable!(),
            }
        }
        Ok(chunks)
    }

    fn try_reduce_block(&mut self) -> ParseResult<Block> {
        debug_call_info!(self);
        let mut block = Block::with_capacity(2);
        // single line block
        if !self.cur_is(Newline) {
            block.push(self.try_reduce_expr()?);
            return Ok(block);
        }
        loop {
            match self.peek() {
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(t) => {
                    if t.is(Indent) {
                        self.skip();
                        while self.cur_is(Newline) {
                            self.skip();
                        }
                    } else if self.cur_is(Dedent) {
                        self.skip();
                        break;
                    } else if t.is(EOF) {
                        break;
                    }
                    match self.try_reduce_expr() {
                        Ok(expr) => {
                            block.push(expr);
                            if self.cur_is(Dedent) {
                                self.skip();
                                break;
                            }
                        }
                        Err(e) => {
                            self.errs.push(e);
                        }
                    }
                }
                _ => switch_unreachable!(),
            }
        }
        if block.is_empty() {
            let loc = if let Some(u) = self.peek() {
                u.loc()
            } else {
                Location::Unknown
            };
            let err = ParseError::syntax_error(
                0,
                loc,
                switch_lang!("failed to parse a block", "ブロックの解析に失敗しました"),
                None,
            );
            Err(err)
        } else {
            Ok(block)
        }
    }

    #[inline]
    fn opt_reduce_decorator(&mut self) -> ParseResult<Option<Decorator>> {
        if self.cur_is(TokenKind::AtSign) {
            self.lpop();
            Ok(Some(Decorator::new(self.try_reduce_expr()?)))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn opt_reduce_decorators(&mut self) -> ParseResult<Set<Decorator>> {
        let mut decs = set![];
        while let Some(deco) = self.opt_reduce_decorator()? {
            decs.insert(deco);
        }
        Ok(decs)
    }

    #[inline]
    fn try_reduce_decl(&mut self) -> ParseResult<Signature> {
        debug_call_info!(self);
        if self.peek().unwrap().category_is(TC::LEnclosure) {
            return Ok(Signature::Var(self.try_reduce_var_sig()?));
        }
        let decorators = self.opt_reduce_decorators()?;
        let name = self.try_reduce_name()?;
        // TODO: parse bounds |...|
        let bounds = TypeBoundSpecs::empty();
        if self.cur_is(VBar) {
            todo!("type bounds are not supported yet");
        }
        if let Some(params) = self.opt_reduce_params().transpose()? {
            let t_spec = if self.cur_is(Colon) {
                self.skip();
                Some(self.try_reduce_type_spec()?)
            } else {
                None
            };
            Ok(Signature::Subr(SubrSignature::new(
                decorators, name, params, t_spec, bounds,
            )))
        } else {
            if !bounds.is_empty() {
                let err = ParseError::syntax_error(
                    0,
                    self.peek().unwrap().loc(),
                    switch_lang!(
                        "Cannot use type bounds in a declaration of a variable",
                        "変数宣言で型制約は使えません"
                    ),
                    None,
                );
                self.next_expr();
                return Err(err);
            }
            let t_spec = if name.is_const() {
                if self.cur_is(SubtypeOf) {
                    self.skip();
                    Some(self.try_reduce_type_spec()?)
                } else {
                    if self.cur_is(Colon) {
                        self.warns.push(ParseError::syntax_warning(0, name.loc(), switch_lang!(
                                "Since it is obvious that the variable is of type `Type`, there is no need to specify the type.",
                                "変数がType型であることは明らかなので、型指定は不要です。"
                            ), Some(switch_lang!(
                                "Are you sure you're not confusing it with subclass declaration (<:)?",
                                "サブクラスの宣言(<:)ではありませんか?"
                            ).into())
                        ));
                    }
                    None
                }
            } else if self.cur_is(Colon) {
                self.skip();
                Some(self.try_reduce_type_spec()?)
            } else {
                None
            };

            Ok(Signature::Var(VarSignature::new(
                VarPattern::VarName(name),
                t_spec,
            )))
        }
    }

    #[inline]
    fn try_reduce_var_sig(&mut self) -> ParseResult<VarSignature> {
        debug_call_info!(self);
        let pat = self.try_reduce_var_pattern()?;
        let t_spec = if self.cur_is(Colon) {
            self.skip();
            Some(self.try_reduce_type_spec()?)
        } else {
            None
        };
        if self.cur_is(VBar) {
            todo!()
        }
        Ok(VarSignature::new(pat, t_spec))
    }

    /// default_param ::= non_default_param `|=` const_expr
    fn try_reduce_param_sig(&mut self) -> ParseResult<ParamSignature> {
        debug_call_info!(self);
        let lhs = self.try_reduce_non_default_param_sig()?;
        if self.cur_is(OrEqual) {
            self.skip();
            let val = self.try_reduce_const_expr()?;
            Ok(ParamSignature::new(lhs.pat, lhs.t_spec, Some(val)))
        } else {
            Ok(ParamSignature::new(lhs.pat, lhs.t_spec, None))
        }
    }

    #[inline]
    fn try_reduce_non_default_param_sig(&mut self) -> ParseResult<ParamSignature> {
        debug_call_info!(self);
        let pat = self.try_reduce_param_pattern()?;
        let t_spec = if self.cur_is(Colon) {
            self.skip();
            Some(self.try_reduce_type_spec()?)
        } else {
            None
        };
        Ok(ParamSignature::new(pat, t_spec, None))
    }

    #[inline]
    fn try_reduce_lambda_sig(&mut self) -> ParseResult<LambdaSignature> {
        debug_call_info!(self);
        // let lambda_mark = self.lpop();
        let params = self.try_reduce_params()?;
        let return_t = match self.peek() {
            Some(t) if t.is(SupertypeOf) => {
                self.skip();
                Some(self.try_reduce_type_spec()?)
            }
            _ => None,
        };
        let bounds = match self.peek() {
            Some(t) if t.is(VBar) => {
                self.skip();
                self.try_reduce_bounds()?
            }
            _ => TypeBoundSpecs::empty(),
        };
        Ok(LambdaSignature::new(params, return_t, bounds))
    }

    fn try_reduce_acc(&mut self) -> ParseResult<Accessor> {
        debug_call_info!(self);
        let mut acc = match self.peek() {
            Some(t) if t.is(Symbol) => Accessor::local(self.lpop()),
            // TODO: SelfDot
            _ => return Err(self.skip_and_throw_syntax_err(caused_by!())),
        };
        loop {
            match self.peek() {
                Some(t) if t.is(Dot) => {
                    self.skip();
                    let symbol = self.lpop();
                    debug_power_assert!(symbol.is(Symbol));
                    let attr = Local::new(symbol);
                    acc = Accessor::attr(Expr::Accessor(acc), attr);
                }
                Some(t) if t.is(LSqBr) => {
                    self.skip();
                    let index = self.try_reduce_expr()?;
                    if self.cur_is(RSqBr) {
                        self.skip();
                    } else {
                        return Err(self.skip_and_throw_syntax_err(caused_by!()));
                    }
                    acc = Accessor::subscr(Expr::Accessor(acc), index);
                    if self.cur_is(RSqBr) {
                        self.lpop();
                    } else {
                        // TODO: error report: RSqBr not found
                        return Err(self.skip_and_throw_syntax_err(caused_by!()));
                    }
                }
                _ => {
                    break;
                }
            }
        }
        Ok(acc)
    }

    fn try_reduce_elems(&mut self) -> ParseResult<Vars> {
        debug_call_info!(self);
        let mut elems = Vars::empty();
        match self.peek() {
            Some(t) if t.is_block_op() => return Ok(Vars::empty()),
            Some(_) => {
                let elem = self.try_reduce_var_sig()?;
                elems.push(elem);
            }
            _ => return Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
        loop {
            match self.peek() {
                Some(t) if t.is(Comma) => {
                    self.skip();
                    let elem = self.try_reduce_var_sig()?;
                    elems.push(elem);
                }
                Some(t) if t.category_is(TC::BinOp) => {
                    log!("[DEBUG] error caused by: {}", fn_name!());
                    let err = ParseError::syntax_error(
                        0,
                        t.loc(),
                        switch_lang!(
                            "Binary operators cannot be used in left-values",
                            "左辺値の中で中置演算子は使えません"
                        ),
                        None,
                    );
                    self.next_expr();
                    return Err(err);
                }
                _ => {
                    break;
                }
            }
        }
        Ok(elems)
    }

    fn opt_reduce_params(&mut self) -> Option<ParseResult<Params>> {
        debug_call_info!(self);
        match self.peek() {
            Some(t)
                if t.category_is(TC::Literal)
                    || t.is(Symbol)
                    || t.category_is(TC::UnaryOp)
                    || t.is(Dot)
                    || t.category_is(TC::Caret)
                    || t.is(LParen)
                    || t.is(LSqBr)
                    || t.is(LBrace) =>
            {
                Some(self.try_reduce_params())
            }
            _ => None,
        }
    }

    fn try_reduce_params(&mut self) -> ParseResult<Params> {
        debug_call_info!(self);
        let lp = if self.cur_is(TokenKind::LParen) {
            Some(self.lpop())
        } else {
            None
        };
        let mut non_default_params = vec![];
        let mut default_params = vec![];
        let mut default_appeared = false;
        match self.peek() {
            Some(t) if t.is(RParen) => {
                let parens = (lp.unwrap(), self.lpop());
                return Ok(Params::new(
                    non_default_params,
                    default_params,
                    Some(parens),
                ));
            }
            Some(t) if t.is_block_op() => {
                if lp.is_none() {
                    return Ok(Params::new(non_default_params, default_params, None));
                }
                // TODO: RParen not found
                else {
                    return Err(self.skip_and_throw_syntax_err(caused_by!()));
                }
            }
            Some(_) => {
                let param = self.try_reduce_param_sig()?;
                if param.has_default() {
                    default_appeared = true;
                    default_params.push(param);
                } else {
                    non_default_params.push(param);
                }
            }
            _ => return Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
        loop {
            match self.peek() {
                Some(t) if t.is(Comma) => {
                    self.skip();
                    let param = self.try_reduce_param_sig()?;
                    match (param.has_default(), default_appeared) {
                        (true, true) => {
                            default_params.push(param);
                        }
                        (true, false) => {
                            default_appeared = true;
                            default_params.push(param);
                        }
                        (false, true) => {
                            return Err(ParseError::syntax_error(
                                0,
                                param.loc(),
                                "non-default argument follows default argument",
                                None,
                            ))
                        }
                        (false, false) => {
                            non_default_params.push(param);
                        }
                    }
                }
                Some(t) if t.category_is(TC::BinOp) => {
                    let err = ParseError::syntax_error(
                        0,
                        t.loc(),
                        switch_lang!(
                            "Binary operators cannot be used in parameters",
                            "仮引数の中で中置演算子は使えません"
                        ),
                        None,
                    );
                    self.next_expr();
                    return Err(err);
                }
                Some(t) if t.is(TokenKind::RParen) => {
                    let rp = self.lpop();
                    if let Some(lp) = lp {
                        return Ok(Params::new(
                            non_default_params,
                            default_params,
                            Some((lp, rp)),
                        ));
                    } else {
                        // LParen not found
                        return Err(self.skip_and_throw_syntax_err(caused_by!()));
                    }
                }
                _ if lp.is_none() => {
                    return Ok(Params::new(non_default_params, default_params, None))
                }
                _ => {
                    // TODO: RParen not found
                    return Err(self.skip_and_throw_syntax_err(caused_by!()));
                }
            }
        }
    }

    fn try_reduce_var_pattern(&mut self) -> ParseResult<VarPattern> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => Ok(VarPattern::VarName(self.try_reduce_name()?)),
            Some(t) if t.is(UBar) => Ok(VarPattern::Discard(self.lpop())),
            Some(t) if t.is(LSqBr) => {
                let l_sqbr = self.lpop();
                let elems = self.try_reduce_elems()?;
                if self.cur_is(RSqBr) {
                    let r_sqbr = self.lpop();
                    Ok(VarPattern::Array(VarArrayPattern::new(
                        l_sqbr, elems, r_sqbr,
                    )))
                } else {
                    // TODO: error report: RSqBr not found
                    Err(self.skip_and_throw_syntax_err(caused_by!()))
                }
            }
            Some(t) if t.is(LParen) => {
                self.skip();
                let pat = self.try_reduce_var_pattern()?;
                if self.cur_is(RParen) {
                    self.skip();
                    Ok(pat)
                } else {
                    // TODO: error report: RParen not found
                    Err(self.skip_and_throw_syntax_err(caused_by!()))
                }
            }
            _ => Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
    }

    fn try_reduce_param_pattern(&mut self) -> ParseResult<ParamPattern> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => Ok(ParamPattern::VarName(self.try_reduce_name()?)),
            Some(t) if t.is(UBar) => Ok(ParamPattern::Discard(self.lpop())),
            Some(t) if t.category_is(TC::Literal) => {
                // TODO: range pattern
                Ok(ParamPattern::Lit(self.try_reduce_lit()?))
            }
            Some(t) if t.is(PreStar) => {
                self.skip();
                Ok(ParamPattern::VarArgsName(self.try_reduce_name()?))
            }
            Some(t) if t.is(LSqBr) => {
                let l_sqbr = self.lpop();
                let elems = self.try_reduce_params()?;
                if self.cur_is(RSqBr) {
                    let r_sqbr = self.lpop();
                    Ok(ParamPattern::Array(ParamArrayPattern::new(
                        l_sqbr, elems, r_sqbr,
                    )))
                } else {
                    // TODO: error report: RSqBr not found
                    Err(self.skip_and_throw_syntax_err(caused_by!()))
                }
            }
            Some(t) if t.is(LParen) => {
                self.skip();
                let pat = self.try_reduce_param_pattern()?;
                if self.cur_is(RParen) {
                    self.skip();
                    Ok(pat)
                } else {
                    // TODO: error report: RParen not found
                    Err(self.skip_and_throw_syntax_err(caused_by!()))
                }
            }
            _ => Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
    }

    // TODO: set type
    fn try_reduce_type_spec(&mut self) -> ParseResult<TypeSpec> {
        debug_call_info!(self);
        let mut typ = match self.peek() {
            Some(t) if t.is(Symbol) => {
                TypeSpec::PreDeclTy(PreDeclTypeSpec::Simple(self.try_reduce_simple_type_spec()?))
            }
            Some(t) if t.category_is(TC::Literal) => {
                let lhs = ConstExpr::Lit(self.try_reduce_lit()?);
                let maybe_op = self.lpop();
                let op = if maybe_op.is(Closed)
                    || maybe_op.is(LeftOpen)
                    || maybe_op.is(RightOpen)
                    || maybe_op.is(Open)
                {
                    maybe_op
                } else {
                    return Err(self.skip_and_throw_syntax_err(caused_by!()));
                };
                // TODO: maybe Name
                let rhs = ConstExpr::Lit(self.try_reduce_lit()?);
                TypeSpec::interval(op, lhs, rhs)
            }
            Some(t) if t.is(LParen) => self.try_reduce_func_type()?,
            Some(t) if t.is(LSqBr) => {
                self.skip();
                let mut tys = vec![self.try_reduce_type_spec()?];
                loop {
                    match self.peek() {
                        Some(t) if t.is(Comma) => {
                            self.skip();
                            let t = self.try_reduce_type_spec()?;
                            tys.push(t);
                        }
                        Some(t) if t.is(RSqBr) => {
                            self.skip();
                            break;
                        }
                        _ => return Err(self.skip_and_throw_syntax_err(caused_by!())),
                    }
                }
                TypeSpec::Tuple(tys)
            }
            _ => return Err(self.skip_and_throw_syntax_err(caused_by!())),
        };
        loop {
            match self.peek() {
                Some(t) if t.is(AndOp) => {
                    let rhs = self.try_reduce_type_spec()?;
                    typ = TypeSpec::and(typ, rhs);
                }
                Some(t) if t.is(OrOp) => {
                    let rhs = self.try_reduce_type_spec()?;
                    typ = TypeSpec::or(typ, rhs);
                }
                Some(t) if t.category_is(TC::LambdaOp) => {
                    let is_func = t.is(FuncArrow);
                    self.skip();
                    let rhs = self.try_reduce_type_spec()?;
                    typ = if is_func {
                        TypeSpec::func(None, vec![ParamTySpec::anonymous(typ)], vec![], rhs)
                    } else {
                        TypeSpec::proc(None, vec![ParamTySpec::anonymous(typ)], vec![], rhs)
                    };
                }
                _ => {
                    break;
                }
            }
        }
        Ok(typ)
    }

    fn try_reduce_func_type_param(&mut self) -> ParseResult<ParamTySpec> {
        debug_call_info!(self);
        if self.cur_is(Symbol) && self.nth_is(1, Colon) {
            let name = self.try_reduce_name()?;
            self.skip();
            let typ = self.try_reduce_type_spec()?;
            Ok(ParamTySpec::new(Some(name.into_token()), typ))
        } else {
            Ok(ParamTySpec::anonymous(self.try_reduce_type_spec()?))
        }
    }

    // TODO: default parameters
    fn try_reduce_func_type(&mut self) -> ParseResult<TypeSpec> {
        debug_call_info!(self);
        let lparen = Some(self.lpop());
        let mut non_defaults = vec![self.try_reduce_func_type_param()?];
        loop {
            match self.peek() {
                Some(t) if t.is(Comma) => {
                    self.skip();
                    non_defaults.push(self.try_reduce_func_type_param()?);
                }
                Some(t) if t.is(RParen) => {
                    self.skip();
                    break;
                }
                _ => return Err(self.skip_and_throw_syntax_err(caused_by!())),
            }
        }
        match self.peek() {
            Some(t) if t.category_is(TC::LambdaOp) => {
                let is_func = t.is(FuncArrow);
                self.skip();
                let rhs = self.try_reduce_type_spec()?;
                if is_func {
                    Ok(TypeSpec::func(lparen, non_defaults, vec![], rhs))
                } else {
                    Ok(TypeSpec::proc(lparen, non_defaults, vec![], rhs))
                }
            }
            _ => Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
    }

    #[inline]
    fn try_reduce_simple_type_spec(&mut self) -> ParseResult<SimpleTypeSpec> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => {
                let name = self.try_reduce_name()?;
                if let Some(res) = self.opt_reduce_args() {
                    let args = self.validate_const_args(res?)?;
                    Ok(SimpleTypeSpec::new(name, args))
                } else {
                    Ok(SimpleTypeSpec::new(name, ConstArgs::empty()))
                }
            }
            _ => Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
    }

    fn try_reduce_bounds(&mut self) -> ParseResult<TypeBoundSpecs> {
        todo!()
    }

    fn validate_const_expr(&mut self, expr: Expr) -> ParseResult<ConstExpr> {
        match expr {
            Expr::Lit(l) =>  Ok(ConstExpr::Lit(l)),
            Expr::Accessor(Accessor::Local(local)) => {
                let local = ConstLocal::new(local.symbol);
                Ok(ConstExpr::Accessor(ConstAccessor::Local(local)))
            }
            // TODO: App, Array, Record, BinOp, UnaryOp,
            other => {
                Err(ParseError::syntax_error(0, other.loc(), switch_lang!(
                    "this expression is not computable at the compile-time, so cannot used as a type-argument",
                    "この式はコンパイル時計算できないため、型引数には使用できません",
                ), None))
            }
        }
    }

    fn validate_const_pos_arg(&mut self, arg: PosArg) -> ParseResult<ConstPosArg> {
        let expr = self.validate_const_expr(arg.expr)?;
        Ok(ConstPosArg::new(expr))
    }

    fn validate_const_kw_arg(&mut self, arg: KwArg) -> ParseResult<ConstKwArg> {
        let expr = self.validate_const_expr(arg.expr)?;
        Ok(ConstKwArg::new(arg.keyword, expr))
    }

    // exprが定数式か確認する
    fn validate_const_args(&mut self, args: Args) -> ParseResult<ConstArgs> {
        let (pos, kw, paren) = args.deconstruct();
        let mut const_args = ConstArgs::new(vec![], vec![], paren);
        for arg in pos.into_iter() {
            match self.validate_const_pos_arg(arg) {
                Ok(arg) => {
                    const_args.push_pos(arg);
                }
                Err(e) => return Err(e),
            }
        }
        for arg in kw.into_iter() {
            match self.validate_const_kw_arg(arg) {
                Ok(arg) => {
                    const_args.push_kw(arg);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(const_args)
    }

    fn opt_reduce_args(&mut self) -> Option<ParseResult<Args>> {
        debug_call_info!(self);
        match self.peek() {
            Some(t)
                if t.category_is(TC::Literal)
                    || t.is(Symbol)
                    || t.category_is(TC::UnaryOp)
                    || t.is(Dot)
                    || t.category_is(TC::Caret)
                    || t.is(LParen)
                    || t.is(LSqBr)
                    || t.is(LBrace)
                    || t.is(Colon) =>
            {
                Some(self.try_reduce_args())
            }
            _ => None,
        }
    }

    /// 引数はインデントで区切ることができる(ただしコンマに戻すことはできない)
    ///
    /// ```
    /// x = if true, 1, 2
    /// # is equal to
    /// x = if true:
    ///     1
    ///     2
    /// ```
    fn try_reduce_args(&mut self) -> ParseResult<Args> {
        debug_call_info!(self);
        let mut lp = None;
        let rp;
        if self.cur_is(LParen) {
            lp = Some(self.lpop());
        }
        if self.cur_is(RParen) {
            rp = Some(self.lpop());
            return Ok(Args::new(vec![], vec![], Some((lp.unwrap(), rp.unwrap()))));
        } else if self.cur_category_is(TC::REnclosure) {
            return Ok(Args::new(vec![], vec![], None));
        }
        let mut args = match self.try_reduce_arg()? {
            PosOrKwArg::Pos(arg) => Args::new(vec![arg], vec![], None),
            PosOrKwArg::Kw(arg) => Args::new(vec![], vec![arg], None),
        };
        let mut colon_style = false;
        loop {
            match self.peek() {
                Some(t) if t.is(Colon) && colon_style => {
                    self.skip();
                    return Err(self.skip_and_throw_syntax_err(caused_by!()));
                }
                Some(t) if t.is(Colon) => {
                    self.skip();
                    colon_style = true;
                    while self.cur_is(Newline) {
                        self.skip();
                    }
                    debug_power_assert!(self.cur_is(Indent));
                    self.skip();
                    if !args.kw_is_empty() {
                        args.push_kw(self.try_reduce_kw_arg()?);
                    } else {
                        match self.try_reduce_arg()? {
                            PosOrKwArg::Pos(arg) => {
                                args.push_pos(arg);
                            }
                            PosOrKwArg::Kw(arg) => {
                                args.push_kw(arg);
                            }
                        }
                    }
                }
                Some(t) if t.is(Comma) => {
                    self.skip();
                    if colon_style || self.cur_is(Comma) {
                        return Err(self.skip_and_throw_syntax_err(caused_by!()));
                    }
                    if !args.kw_is_empty() {
                        args.push_kw(self.try_reduce_kw_arg()?);
                    } else {
                        match self.try_reduce_arg()? {
                            PosOrKwArg::Pos(arg) => {
                                args.push_pos(arg);
                            }
                            PosOrKwArg::Kw(arg) => {
                                args.push_kw(arg);
                            }
                        }
                    }
                }
                Some(t) if t.is(Newline) && colon_style => {
                    while self.cur_is(Newline) {
                        self.skip();
                    }
                    if self.cur_is(Dedent) {
                        self.skip();
                        break;
                    }
                    if !args.kw_is_empty() {
                        args.push_kw(self.try_reduce_kw_arg()?);
                    } else {
                        match self.try_reduce_arg()? {
                            PosOrKwArg::Pos(arg) => {
                                args.push_pos(arg);
                            }
                            PosOrKwArg::Kw(arg) => {
                                args.push_kw(arg);
                            }
                        }
                    }
                }
                Some(t) if t.is(RParen) => {
                    rp = Some(self.lpop());
                    let (pos_args, kw_args, _) = args.deconstruct();
                    args = Args::new(pos_args, kw_args, Some((lp.unwrap(), rp.unwrap())));
                    break;
                }
                _ => {
                    break;
                }
            }
        }
        Ok(args)
    }

    fn try_reduce_arg(&mut self) -> ParseResult<PosOrKwArg> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => {
                if self.nth_is(1, Colon) {
                    let acc = self.try_reduce_acc()?;
                    debug_power_assert!(self.cur_is(Colon));
                    if self.nth_is(1, Newline) {
                        // colon style call
                        Ok(PosOrKwArg::Pos(PosArg::new(Expr::Accessor(acc))))
                    } else {
                        self.skip();
                        let kw = if let Accessor::Local(n) = acc {
                            n.symbol
                        } else {
                            self.next_expr();
                            return Err(ParseError::simple_syntax_error(0, acc.loc()));
                        };
                        Ok(PosOrKwArg::Kw(KwArg::new(kw, self.try_reduce_expr()?)))
                    }
                } else {
                    Ok(PosOrKwArg::Pos(PosArg::new(self.try_reduce_expr()?)))
                }
            }
            Some(_) => Ok(PosOrKwArg::Pos(PosArg::new(self.try_reduce_expr()?))),
            None => switch_unreachable!(),
        }
    }

    fn try_reduce_kw_arg(&mut self) -> ParseResult<KwArg> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => {
                if self.nth_is(1, Colon) {
                    let acc = self.try_reduce_acc()?;
                    debug_power_assert!(self.cur_is(Colon));
                    self.skip();
                    let keyword = if let Accessor::Local(n) = acc {
                        n.symbol
                    } else {
                        self.next_expr();
                        return Err(ParseError::simple_syntax_error(0, acc.loc()));
                    };
                    Ok(KwArg::new(keyword, self.try_reduce_expr()?))
                } else {
                    Err(ParseError::simple_syntax_error(0, t.loc()))
                }
            }
            Some(other) => Err(ParseError::simple_syntax_error(0, other.loc())),
            None => switch_unreachable!(),
        }
    }

    fn try_reduce_const_expr(&mut self) -> ParseResult<ConstExpr> {
        debug_call_info!(self);
        let expr = self.try_reduce_expr()?;
        self.validate_const_expr(expr)
    }

    fn try_reduce_expr(&mut self) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut stack = Vec::<ExprOrOp>::new();
        match self.cur_side() {
            Side::LhsAssign => {
                let sig = self.try_reduce_decl()?;
                match self.peek() {
                    Some(t) if t.is(Equal) => {
                        let op = self.lpop();
                        self.counter.inc();
                        let body = DefBody::new(op, self.try_reduce_block()?, self.counter);
                        Ok(Expr::Def(Def::new(sig, body)))
                    }
                    _other => Err(self.skip_and_throw_syntax_err(caused_by!())),
                }
            }
            Side::LhsLambda => {
                let params = self.try_reduce_params()?;
                match self.peek() {
                    Some(t) if t.category_is(TC::LambdaOp) => {
                        let sig = LambdaSignature::new(params, None, TypeBoundSpecs::empty());
                        let op = self.lpop();
                        self.counter.inc();
                        Ok(Expr::Lambda(Lambda::new(
                            sig,
                            op,
                            self.try_reduce_block()?,
                            self.counter,
                        )))
                    }
                    Some(t) if t.is(Colon) => {
                        self.lpop();
                        let spec_t = self.try_reduce_type_spec()?;
                        let sig =
                            LambdaSignature::new(params, Some(spec_t), TypeBoundSpecs::empty());
                        let op = self.lpop();
                        self.counter.inc();
                        Ok(Expr::Lambda(Lambda::new(
                            sig,
                            op,
                            self.try_reduce_block()?,
                            self.counter,
                        )))
                    }
                    _other => Err(self.skip_and_throw_syntax_err(caused_by!())),
                }
            }
            Side::Rhs => {
                stack.push(ExprOrOp::Expr(self.try_reduce_lhs()?));
                loop {
                    match self.peek() {
                        Some(op) if op.category_is(TC::BinOp) => {
                            let op_prec = op.kind.precedence();
                            if stack.len() >= 2 {
                                while let Some(ExprOrOp::Op(prev_op)) = stack.get(stack.len() - 2) {
                                    if prev_op.category_is(TC::BinOp)
                                        && prev_op.kind.precedence() >= op_prec
                                    {
                                        let rhs =
                                            enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                                        let prev_op =
                                            enum_unwrap!(stack.pop(), Some:(ExprOrOp::Op:(_)));
                                        let lhs =
                                            enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                                        let bin = BinOp::new(prev_op, lhs, rhs);
                                        stack.push(ExprOrOp::Expr(Expr::BinOp(bin)));
                                    } else {
                                        break;
                                    }
                                    if stack.len() <= 1 {
                                        break;
                                    }
                                }
                            }
                            stack.push(ExprOrOp::Op(self.lpop()));
                            stack.push(ExprOrOp::Expr(self.try_reduce_lhs()?));
                        }
                        Some(t) if t.category_is(TC::DefOp) => {
                            switch_unreachable!()
                        }
                        Some(t) if t.is(Dot) => {
                            self.skip();
                            match self.lpop() {
                                symbol if symbol.is(Symbol) => {
                                    let obj = if let Some(ExprOrOp::Expr(expr)) = stack.pop() {
                                        expr
                                    } else {
                                        return Err(self.skip_and_throw_syntax_err(caused_by!()));
                                    };
                                    let acc = Accessor::attr(obj, Local::new(symbol));
                                    if let Ok(args) = self.try_reduce_args() {
                                        let call = Call::new(Expr::Accessor(acc), args);
                                        stack.push(ExprOrOp::Expr(Expr::Call(call)));
                                    } else {
                                        stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
                                    }
                                }
                                other => {
                                    self.restore(other);
                                    return Err(self.skip_and_throw_syntax_err(caused_by!()));
                                }
                            }
                        }
                        _ => {
                            if stack.len() <= 1 {
                                break;
                            }
                            // else if stack.len() == 2 { switch_unreachable!() }
                            else {
                                while stack.len() >= 3 {
                                    let rhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                                    let op = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Op:(_)));
                                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                                    let bin = BinOp::new(op, lhs, rhs);
                                    stack.push(ExprOrOp::Expr(Expr::BinOp(bin)));
                                }
                            }
                        }
                    }
                }
                match stack.pop() {
                    Some(ExprOrOp::Expr(expr)) if stack.is_empty() => Ok(expr),
                    Some(ExprOrOp::Expr(expr)) => {
                        let extra = stack.pop().unwrap();
                        let loc = match extra {
                            ExprOrOp::Expr(expr) => expr.loc(),
                            ExprOrOp::Op(op) => op.loc(),
                        };
                        self.warns
                            .push(ParseError::compiler_bug(0, loc, fn_name!(), line!()));
                        Ok(expr)
                    }
                    Some(ExprOrOp::Op(op)) => {
                        Err(ParseError::compiler_bug(0, op.loc(), fn_name!(), line!()))
                    }
                    _ => switch_unreachable!(),
                }
            }
        }
    }

    /// "LHS" is the smallest unit that can be the left-hand side of an BinOp.
    /// e.g. Call, Name, UnaryOp, Lambda
    fn try_reduce_lhs(&mut self) -> ParseResult<Expr> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.category_is(TC::Literal) => {
                // TODO: 10.times ...などメソッド呼び出しもある
                Ok(Expr::Lit(self.try_reduce_lit()?))
            }
            Some(t) if t.is(Symbol) || t.is(Dot) => Ok(self.try_reduce_call_or_acc()?),
            Some(t) if t.category_is(TC::UnaryOp) => Ok(Expr::UnaryOp(self.try_reduce_unary()?)),
            Some(t) if t.category_is(TC::Caret) => {
                let lambda = self.try_reduce_lambda()?;
                Ok(Expr::Lambda(lambda))
            }
            Some(t) if t.is(LParen) => {
                self.skip();
                let expr = self.try_reduce_expr()?;
                if self.cur_is(RParen) {
                    self.skip();
                } else {
                    return Err(self.skip_and_throw_syntax_err(caused_by!()));
                }
                Ok(expr)
            }
            Some(t) if t.is(LSqBr) => Ok(Expr::Array(self.try_reduce_array()?)),
            Some(t) if t.is(LBrace) => {
                todo!()
            }
            Some(t) if t.is(UBar) => {
                let token = self.lpop();
                Err(ParseError::feature_error(0, token.loc(), "discard pattern"))
            }
            _ => Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
    }

    #[inline]
    fn try_reduce_call_or_acc(&mut self) -> ParseResult<Expr> {
        debug_call_info!(self);
        let acc = self.try_reduce_acc()?;
        if let Some(res) = self.opt_reduce_args() {
            let args = res?;
            let call = Call::new(Expr::Accessor(acc), args);
            Ok(Expr::Call(call))
        } else {
            Ok(Expr::Accessor(acc))
        }
    }

    #[inline]
    fn try_reduce_lambda(&mut self) -> ParseResult<Lambda> {
        debug_call_info!(self);
        let sig = self.try_reduce_lambda_sig()?;
        let op = self.lpop();
        if op.category() != TC::LambdaOp {
            return Err(self.skip_and_throw_syntax_err(caused_by!()));
        }
        // REVIEW: この箇所必要か
        while self.cur_is(Newline) {
            self.skip();
        }
        let body = self.try_reduce_block()?;
        self.counter.inc();
        Ok(Lambda::new(sig, op, body, self.counter))
    }

    #[inline]
    fn try_reduce_unary(&mut self) -> ParseResult<UnaryOp> {
        debug_call_info!(self);
        let op = self.lpop();
        let expr = self.try_reduce_expr()?;
        Ok(UnaryOp::new(op, expr))
    }

    #[inline]
    fn try_reduce_array(&mut self) -> ParseResult<Array> {
        debug_call_info!(self);
        let l_sqbr = self.lpop();
        let elems = self.try_reduce_args()?;
        let r_sqbr = self.lpop();
        if !r_sqbr.is(RSqBr) {
            return Err(ParseError::simple_syntax_error(0, r_sqbr.loc()));
        }
        let arr = Array::new(l_sqbr, r_sqbr, elems, None);
        Ok(arr)
    }

    #[inline]
    fn try_reduce_name(&mut self) -> ParseResult<VarName> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => Ok(VarName::new(self.lpop())),
            _ => Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
    }

    #[inline]
    fn try_reduce_lit(&mut self) -> ParseResult<Literal> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.category_is(TC::Literal) => Ok(Literal::from(self.lpop())),
            _ => Err(self.skip_and_throw_syntax_err(caused_by!())),
        }
    }
}
