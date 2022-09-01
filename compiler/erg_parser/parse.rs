//! implements `Parser`.
//!
//! パーサーを実装する
//!
use std::fmt::Debug;
use std::mem;

use erg_common::config::ErgConfig;
use erg_common::config::Input;
use erg_common::error::Location;
use erg_common::option_enum_unwrap;
use erg_common::set::Set as HashSet;
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
macro_rules! debug_call_info {
    ($self: ident) => {
        $self.level += 1;
        log!(
            c GREEN,
            "\n{} ({}) entered {}, cur: {}",
            " ".repeat($self.level),
            $self.level,
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

pub enum ArrayInner {
    Normal(Args),
    WithLength(PosArg, Expr),
    Comprehension {
        elem: PosArg,
        generators: Vec<(Local, Expr)>,
        guards: Vec<Expr>,
    },
}

pub enum BraceContainer {
    Set(Set),
    Dict(Dict),
    Record(Record),
}

/// Perform recursive descent parsing.
///
/// `level` is raised by 1 by `debug_call_info!` in each analysis method and lowered by 1 when leaving (`.map_err` is called to lower the level).
///
/// To enhance error descriptions, the parsing process will continue as long as it's not fatal.
#[derive(Debug)]
pub struct Parser {
    counter: DefId,
    level: usize, // nest level (for debugging)
    tokens: TokenStream,
    warns: ParseErrors,
    errs: ParseErrors,
}

impl Parser {
    const fn new(ts: TokenStream) -> Self {
        Self {
            counter: DefId(0),
            level: 0,
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

    /// 解析を諦めて次の解析できる要素に移行する
    /// give up parsing and move to the next element that can be parsed
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
        log!(err "error caused by: {caused_by}");
        self.next_expr();
        ParseError::simple_syntax_error(0, loc)
    }

    #[inline]
    fn restore(&mut self, token: Token) {
        self.tokens.insert(0, token);
    }

    fn stack_dec(&mut self) -> () {
        self.level -= 1;
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
        self.cfg.input = Input::Str(src);
        let ast = self.parse()?;
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

    /// Parses with default configuration
    pub fn parse_with_default_config(input: Input) -> Result<AST, ParserRunnerErrors> {
        let mut cfg = ErgConfig::default();
        cfg.input = input;
        let mut self_ = Self::new(cfg);
        self_.parse()
    }
}

impl Parser {
    pub fn parse(&mut self, mod_name: Str) -> Result<AST, ParseErrors> {
        if self.tokens.is_empty() {
            return Ok(AST::new(mod_name, Module::empty()));
        }
        log!(info "the parsing process has started.");
        log!(info "token stream: {}", self.tokens);
        let module = match self.try_reduce_module() {
            Ok(module) => module,
            Err(_) => {
                return Err(mem::take(&mut self.errs));
            }
        };
        if !self.cur_is(EOF) {
            let loc = self.peek().unwrap().loc();
            self.errs
                .push(ParseError::compiler_bug(0, loc, fn_name!(), line!()));
            return Err(mem::take(&mut self.errs));
        }
        log!(info "the parsing process has completed.");
        log!(info "AST:\n{module}");
        log!(info "the desugaring process has started.");
        let mut desugarer = Desugarer::new();
        let module = desugarer.desugar(module);
        log!(info "AST (desugared):\n{module}");
        log!(info "the desugaring process has completed.{RESET}");
        if self.errs.is_empty() {
            Ok(AST::new(mod_name, module))
        } else {
            Err(mem::take(&mut self.errs))
        }
    }

    /// Reduce to the largest unit of syntax, the module (this is called only once)
    /// 構文の最大単位であるモジュールに還元する(これが呼ばれるのは一度きり)
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
                Some(_) => match self.try_reduce_expr(true) {
                    Ok(expr) => {
                        chunks.push(expr);
                    }
                    Err(_) => {}
                },
                _ => switch_unreachable!(),
            }
        }
        self.level -= 1;
        Ok(chunks)
    }

    fn try_reduce_block(&mut self) -> ParseResult<Block> {
        debug_call_info!(self);
        let mut block = Block::with_capacity(2);
        // single line block
        if !self.cur_is(Newline) {
            let chunk = self.try_reduce_expr(true).map_err(|_| self.stack_dec())?;
            block.push(chunk);
            self.level -= 1;
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
                    match self.try_reduce_expr(true) {
                        Ok(expr) => {
                            block.push(expr);
                            if self.cur_is(Dedent) {
                                self.skip();
                                break;
                            }
                        }
                        Err(_) => {}
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
                line!() as usize,
                loc,
                switch_lang!(
                    "japanese" => "ブロックの解析に失敗しました",
                    "simplified_chinese" => "无法解析块",
                    "traditional_chinese" => "無法解析塊",
                    "english" => "failed to parse a block",
                ),
                None,
            );
            self.level -= 1;
            self.errs.push(err);
            Err(())
        } else {
            self.level -= 1;
            Ok(block)
        }
    }

    #[inline]
    fn opt_reduce_decorator(&mut self) -> ParseResult<Option<Decorator>> {
        debug_call_info!(self);
        if self.cur_is(TokenKind::AtSign) {
            self.lpop();
            let expr = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
            self.level -= 1;
            Ok(Some(Decorator::new(expr)))
        } else {
            self.level -= 1;
            Ok(None)
        }
    }

    #[inline]
    fn opt_reduce_decorators(&mut self) -> ParseResult<HashSet<Decorator>> {
        debug_call_info!(self);
        let mut decs = set![];
        while let Some(deco) = self.opt_reduce_decorator().map_err(|_| self.stack_dec())? {
            decs.insert(deco);
        }
        self.level -= 1;
        Ok(decs)
    }

    fn try_reduce_acc(&mut self) -> ParseResult<Accessor> {
        debug_call_info!(self);
        let mut acc = match self.peek() {
            Some(t) if t.is(Symbol) => Accessor::local(self.lpop()),
            Some(t) if t.is(Dot) => {
                let dot = self.lpop();
                let maybe_symbol = self.lpop();
                if maybe_symbol.is(Symbol) {
                    Accessor::public(dot, maybe_symbol)
                } else {
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
            }
            _ => {
                self.level -= 1;
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                return Err(());
            }
        };
        loop {
            match self.peek() {
                Some(t) if t.is(Dot) => {
                    self.skip();
                    let token = self.lpop();
                    match token.kind {
                        Symbol => {
                            let attr = Local::new(token);
                            acc = Accessor::attr(Expr::Accessor(acc), attr);
                        }
                        NatLit => {
                            let attr = Literal::from(token);
                            acc = Accessor::tuple_attr(Expr::Accessor(acc), attr);
                        }
                        _ => {
                            self.restore(token);
                            self.level -= 1;
                            let err = self.skip_and_throw_syntax_err(caused_by!());
                            self.errs.push(err);
                            return Err(());
                        }
                    }
                }
                Some(t) if t.is(LSqBr) => {
                    self.skip();
                    let index = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                    if self.cur_is(RSqBr) {
                        self.skip();
                    } else {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                    acc = Accessor::subscr(Expr::Accessor(acc), index);
                    if self.cur_is(RSqBr) {
                        self.lpop();
                    } else {
                        self.level -= 1;
                        // TODO: error report: RSqBr not found
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                }
                _ => {
                    break;
                }
            }
        }
        self.level -= 1;
        Ok(acc)
    }

    fn _validate_const_expr(&mut self, expr: Expr) -> ParseResult<ConstExpr> {
        match expr {
            Expr::Lit(l) => Ok(ConstExpr::Lit(l)),
            Expr::Accessor(Accessor::Local(local)) => {
                let local = ConstLocal::new(local.symbol);
                Ok(ConstExpr::Accessor(ConstAccessor::Local(local)))
            }
            // TODO: App, Array, Record, BinOp, UnaryOp,
            other => {
                self.errs.push(ParseError::syntax_error(
                0,
                other.loc(),
                switch_lang!(
                    "japanese" => "この式はコンパイル時計算できないため、型引数には使用できません",
                    "simplified_chinese" => "此表达式在编译时不可计算，因此不能用作类型参数",
                    "traditional_chinese" => "此表達式在編譯時不可計算，因此不能用作類型參數",
                    "english" => "this expression is not computable at the compile-time, so cannot used as a type-argument",
                ),
                None,
            ));
                Err(())
            }
        }
    }

    fn _validate_const_pos_arg(&mut self, arg: PosArg) -> ParseResult<ConstPosArg> {
        let expr = self._validate_const_expr(arg.expr)?;
        Ok(ConstPosArg::new(expr))
    }

    fn _validate_const_kw_arg(&mut self, arg: KwArg) -> ParseResult<ConstKwArg> {
        let expr = self._validate_const_expr(arg.expr)?;
        Ok(ConstKwArg::new(arg.keyword, expr))
    }

    // exprが定数式か確認する
    fn _validate_const_args(&mut self, args: Args) -> ParseResult<ConstArgs> {
        let (pos, kw, paren) = args.deconstruct();
        let mut const_args = ConstArgs::new(vec![], vec![], paren);
        for arg in pos.into_iter() {
            match self._validate_const_pos_arg(arg) {
                Ok(arg) => {
                    const_args.push_pos(arg);
                }
                Err(e) => return Err(e),
            }
        }
        for arg in kw.into_iter() {
            match self._validate_const_kw_arg(arg) {
                Ok(arg) => {
                    const_args.push_kw(arg);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(const_args)
    }

    /// For parsing elements of arrays and tuples
    fn try_reduce_elems(&mut self) -> ParseResult<ArrayInner> {
        debug_call_info!(self);
        if self.cur_category_is(TC::REnclosure) {
            let args = Args::new(vec![], vec![], None);
            self.level -= 1;
            return Ok(ArrayInner::Normal(args));
        }
        let first = self.try_reduce_elem().map_err(|_| self.stack_dec())?;
        let mut elems = Args::new(vec![first], vec![], None);
        match self.peek() {
            Some(semi) if semi.is(Semi) => {
                self.lpop();
                let len = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                self.level -= 1;
                return Ok(ArrayInner::WithLength(elems.remove_pos(0), len));
            }
            Some(vbar) if vbar.is(VBar) => {
                let err = ParseError::feature_error(line!() as usize, vbar.loc(), "comprehension");
                self.lpop();
                self.errs.push(err);
                self.level -= 1;
                return Err(());
            }
            Some(t) if t.category_is(TC::REnclosure) || t.is(Comma) => {}
            Some(_) => {
                let elem = self.try_reduce_elem().map_err(|_| self.stack_dec())?;
                elems.push_pos(elem);
            }
            None => {
                self.level -= 1;
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                return Err(());
            }
        }
        loop {
            match self.peek() {
                Some(comma) if comma.is(Comma) => {
                    self.skip();
                    if self.cur_is(Comma) {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                    elems.push_pos(self.try_reduce_elem().map_err(|_| self.stack_dec())?);
                }
                Some(t) if t.category_is(TC::REnclosure) => {
                    break;
                }
                _ => {
                    self.skip();
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
            }
        }
        self.level -= 1;
        Ok(ArrayInner::Normal(elems))
    }

    fn try_reduce_elem(&mut self) -> ParseResult<PosArg> {
        debug_call_info!(self);
        match self.peek() {
            Some(_) => {
                let expr = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(PosArg::new(expr))
            }
            None => switch_unreachable!(),
        }
    }

    fn opt_reduce_args(&mut self) -> Option<ParseResult<Args>> {
        // debug_call_info!(self);
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
    /// x = if True, 1, 2
    /// # is equal to
    /// x = if True:
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
            self.level -= 1;
            return Ok(Args::new(vec![], vec![], Some((lp.unwrap(), rp.unwrap()))));
        } else if self.cur_category_is(TC::REnclosure) {
            self.level -= 1;
            return Ok(Args::new(vec![], vec![], None));
        }
        let mut args = match self.try_reduce_arg().map_err(|_| self.stack_dec())? {
            PosOrKwArg::Pos(arg) => Args::new(vec![arg], vec![], None),
            PosOrKwArg::Kw(arg) => Args::new(vec![], vec![arg], None),
        };
        let mut colon_style = false;
        loop {
            match self.peek() {
                Some(t) if t.is(Colon) && colon_style => {
                    self.skip();
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
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
                        args.push_kw(self.try_reduce_kw_arg().map_err(|_| self.stack_dec())?);
                    } else {
                        match self.try_reduce_arg().map_err(|_| self.stack_dec())? {
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
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                    if !args.kw_is_empty() {
                        args.push_kw(self.try_reduce_kw_arg().map_err(|_| self.stack_dec())?);
                    } else {
                        match self.try_reduce_arg().map_err(|_| self.stack_dec())? {
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
                        args.push_kw(self.try_reduce_kw_arg().map_err(|_| self.stack_dec())?);
                    } else {
                        match self.try_reduce_arg().map_err(|_| self.stack_dec())? {
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
        self.level -= 1;
        Ok(args)
    }

    fn try_reduce_arg(&mut self) -> ParseResult<PosOrKwArg> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => {
                if &t.inspect()[..] == "do" || &t.inspect()[..] == "do!" {
                    let lambda = self.try_reduce_do_block().map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    return Ok(PosOrKwArg::Pos(PosArg::new(Expr::Lambda(lambda))));
                }
                if self.nth_is(1, Colon) {
                    let acc = self.try_reduce_acc().map_err(|_| self.stack_dec())?;
                    debug_power_assert!(self.cur_is(Colon));
                    if self.nth_is(1, Newline) {
                        self.level -= 1;
                        // colon style call
                        Ok(PosOrKwArg::Pos(PosArg::new(Expr::Accessor(acc))))
                    } else {
                        self.skip();
                        let kw = if let Accessor::Local(n) = acc {
                            n.symbol
                        } else {
                            self.next_expr();
                            self.level -= 1;
                            let err = ParseError::simple_syntax_error(0, acc.loc());
                            self.errs.push(err);
                            return Err(());
                        };
                        let expr = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                        self.level -= 1;
                        Ok(PosOrKwArg::Kw(KwArg::new(kw, expr)))
                    }
                } else {
                    let expr = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(PosOrKwArg::Pos(PosArg::new(expr)))
                }
            }
            Some(_) => {
                let expr = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(PosOrKwArg::Pos(PosArg::new(expr)))
            }
            None => switch_unreachable!(),
        }
    }

    fn try_reduce_kw_arg(&mut self) -> ParseResult<KwArg> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => {
                if self.nth_is(1, Colon) {
                    let acc = self.try_reduce_acc().map_err(|_| self.stack_dec())?;
                    debug_power_assert!(self.cur_is(Colon));
                    self.skip();
                    let keyword = if let Accessor::Local(n) = acc {
                        n.symbol
                    } else {
                        self.next_expr();
                        self.level -= 1;
                        self.errs
                            .push(ParseError::simple_syntax_error(0, acc.loc()));
                        return Err(());
                    };
                    let expr = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(KwArg::new(keyword, expr))
                } else {
                    let loc = t.loc();
                    self.level -= 1;
                    self.errs.push(ParseError::simple_syntax_error(0, loc));
                    Err(())
                }
            }
            Some(other) => {
                let loc = other.loc();
                self.level -= 1;
                self.errs.push(ParseError::simple_syntax_error(0, loc));
                Err(())
            }
            None => switch_unreachable!(),
        }
    }

    fn try_reduce_method_defs(&mut self, class: Expr, vis: Token) -> ParseResult<MethodDefs> {
        debug_call_info!(self);
        if self.cur_is(Indent) {
            self.skip();
        } else {
            todo!()
        }
        let first = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
        let first = option_enum_unwrap!(first, Expr::Def).unwrap_or_else(|| todo!());
        let mut defs = vec![first];
        loop {
            match self.peek() {
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                    if self.cur_is(Dedent) {
                        self.skip();
                        break;
                    }
                    let def = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                    let def = option_enum_unwrap!(def, Expr::Def).unwrap_or_else(|| todo!());
                    defs.push(def);
                }
                _ => todo!(),
            }
        }
        let defs = RecordAttrs::from(defs);
        let class = self
            .convert_rhs_to_type_spec(class)
            .map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(MethodDefs::new(class, vis, defs))
    }

    fn try_reduce_do_block(&mut self) -> ParseResult<Lambda> {
        debug_call_info!(self);
        let do_symbol = self.lpop();
        let sig = LambdaSignature::do_sig(&do_symbol);
        let op = match &do_symbol.inspect()[..] {
            "do" => Token::from_str(FuncArrow, "->"),
            "do!" => Token::from_str(ProcArrow, "=>"),
            _ => todo!(),
        };
        if self.cur_is(Colon) {
            self.lpop();
            let body = self.try_reduce_block().map_err(|_| self.stack_dec())?;
            self.counter.inc();
            self.level -= 1;
            Ok(Lambda::new(sig, op, body, self.counter))
        } else {
            let expr = self.try_reduce_expr(true).map_err(|_| self.stack_dec())?;
            let block = Block::new(vec![expr]);
            self.level -= 1;
            Ok(Lambda::new(sig, op, block, self.counter))
        }
    }

    /// winding: true => parse paren-less tuple
    fn try_reduce_expr(&mut self, winding: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut stack = Vec::<ExprOrOp>::new();
        stack.push(ExprOrOp::Expr(
            self.try_reduce_bin_lhs().map_err(|_| self.stack_dec())?,
        ));
        loop {
            match self.peek() {
                Some(op) if op.category_is(TC::DefOp) => {
                    let op = self.lpop();
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self.convert_rhs_to_sig(lhs).map_err(|_| self.stack_dec())?;
                    self.counter.inc();
                    let block = self.try_reduce_block().map_err(|_| self.stack_dec())?;
                    let body = DefBody::new(op, block, self.counter);
                    stack.push(ExprOrOp::Expr(Expr::Def(Def::new(sig, body))));
                }
                Some(op) if op.category_is(TC::LambdaOp) => {
                    let op = self.lpop();
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self
                        .convert_rhs_to_lambda_sig(lhs)
                        .map_err(|_| self.stack_dec())?;
                    self.counter.inc();
                    let block = self.try_reduce_block().map_err(|_| self.stack_dec())?;
                    stack.push(ExprOrOp::Expr(Expr::Lambda(Lambda::new(
                        sig,
                        op,
                        block,
                        self.counter,
                    ))));
                }
                Some(op) if op.category_is(TC::BinOp) => {
                    let op_prec = op.kind.precedence();
                    if stack.len() >= 2 {
                        while let Some(ExprOrOp::Op(prev_op)) = stack.get(stack.len() - 2) {
                            if prev_op.category_is(TC::BinOp)
                                && prev_op.kind.precedence() >= op_prec
                            {
                                let rhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                                let prev_op = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Op:(_)));
                                let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
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
                    stack.push(ExprOrOp::Expr(
                        self.try_reduce_bin_lhs().map_err(|_| self.stack_dec())?,
                    ));
                }
                Some(t) if t.is(DblColon) => {
                    let dbl_colon = self.lpop();
                    match self.lpop() {
                        line_break if line_break.is(Newline) => {
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            let defs = self
                                .try_reduce_method_defs(maybe_class, dbl_colon)
                                .map_err(|_| self.stack_dec())?;
                            return Ok(Expr::MethodDefs(defs));
                        }
                        other => {
                            self.restore(other);
                            self.level -= 1;
                            let err = self.skip_and_throw_syntax_err(caused_by!());
                            self.errs.push(err);
                            return Err(());
                        }
                    }
                }
                Some(t) if t.is(Dot) => {
                    let dot = self.lpop();
                    match self.lpop() {
                        symbol if symbol.is(Symbol) => {
                            let obj = if let Some(ExprOrOp::Expr(expr)) = stack.pop() {
                                expr
                            } else {
                                self.level -= 1;
                                let err = self.skip_and_throw_syntax_err(caused_by!());
                                self.errs.push(err);
                                return Err(());
                            };
                            if let Some(args) = self
                                .opt_reduce_args()
                                .transpose()
                                .map_err(|_| self.stack_dec())?
                            {
                                let call = Call::new(obj, Some(symbol), args);
                                stack.push(ExprOrOp::Expr(Expr::Call(call)));
                            } else {
                                let acc = Accessor::attr(obj, Local::new(symbol));
                                stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
                            }
                        }
                        line_break if line_break.is(Newline) => {
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            let defs = self
                                .try_reduce_method_defs(maybe_class, dot)
                                .map_err(|_| self.stack_dec())?;
                            return Ok(Expr::MethodDefs(defs));
                        }
                        other => {
                            self.restore(other);
                            self.level -= 1;
                            let err = self.skip_and_throw_syntax_err(caused_by!());
                            self.errs.push(err);
                            return Err(());
                        }
                    }
                }
                Some(t) if t.is(Comma) && winding => {
                    let first_elem = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let tup = self
                        .try_reduce_tuple(first_elem)
                        .map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    return Ok(Expr::Tuple(tup));
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
            Some(ExprOrOp::Expr(expr)) if stack.is_empty() => {
                self.level -= 1;
                Ok(expr)
            }
            Some(ExprOrOp::Expr(expr)) => {
                let extra = stack.pop().unwrap();
                let loc = match extra {
                    ExprOrOp::Expr(expr) => expr.loc(),
                    ExprOrOp::Op(op) => op.loc(),
                };
                self.warns
                    .push(ParseError::compiler_bug(0, loc, fn_name!(), line!()));
                self.level -= 1;
                Ok(expr)
            }
            Some(ExprOrOp::Op(op)) => {
                self.level -= 1;
                self.errs
                    .push(ParseError::compiler_bug(0, op.loc(), fn_name!(), line!()));
                Err(())
            }
            _ => switch_unreachable!(),
        }
    }

    /// "LHS" is the smallest unit that can be the left-hand side of an BinOp.
    /// e.g. Call, Name, UnaryOp, Lambda
    fn try_reduce_bin_lhs(&mut self) -> ParseResult<Expr> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.category_is(TC::Literal) => {
                // TODO: 10.times ...などメソッド呼び出しもある
                let lit = self.try_reduce_lit().map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(Expr::Lit(lit))
            }
            Some(t) if t.is(AtSign) => {
                let decos = self.opt_reduce_decorators()?;
                let expr = self.try_reduce_expr(false)?;
                let def = option_enum_unwrap!(expr, Expr::Def).unwrap_or_else(|| todo!());
                let mut subr_sig =
                    option_enum_unwrap!(def.sig, Signature::Subr).unwrap_or_else(|| todo!());
                subr_sig.decorators = decos;
                let expr = Expr::Def(Def::new(Signature::Subr(subr_sig), def.body));
                Ok(expr)
            }
            Some(t) if t.is(Symbol) || t.is(Dot) => {
                let call_or_acc = self
                    .try_reduce_call_or_acc()
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(call_or_acc)
            }
            Some(t) if t.category_is(TC::UnaryOp) => {
                let unaryop = self.try_reduce_unary().map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(Expr::UnaryOp(unaryop))
            }
            Some(t) if t.is(LParen) => {
                let lparen = self.lpop();
                if self.cur_is(RParen) {
                    let rparen = self.lpop();
                    let args = Args::new(vec![], vec![], Some((lparen, rparen)));
                    let unit = Tuple::Normal(NormalTuple::new(args));
                    self.level -= 1;
                    return Ok(Expr::Tuple(unit));
                }
                let mut expr = self.try_reduce_expr(true).map_err(|_| self.stack_dec())?;
                let rparen = self.lpop();
                match &mut expr {
                    Expr::Tuple(Tuple::Normal(tup)) => {
                        tup.elems.paren = Some((lparen, rparen));
                    }
                    _ => {}
                }
                self.level -= 1;
                Ok(expr)
            }
            Some(t) if t.is(LSqBr) => {
                let array = self.try_reduce_array().map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(Expr::Array(array))
            }
            Some(t) if t.is(LBrace) => {
                match self
                    .try_reduce_brace_container()
                    .map_err(|_| self.stack_dec())?
                {
                    BraceContainer::Dict(dic) => {
                        self.level -= 1;
                        Ok(Expr::Dict(dic))
                    }
                    BraceContainer::Record(rec) => {
                        self.level -= 1;
                        Ok(Expr::Record(rec))
                    }
                    BraceContainer::Set(set) => {
                        self.level -= 1;
                        Ok(Expr::Set(set))
                    }
                }
            }
            Some(t) if t.is(UBar) => {
                let token = self.lpop();
                self.level -= 1;
                self.errs.push(ParseError::feature_error(
                    line!() as usize,
                    token.loc(),
                    "discard pattern",
                ));
                Err(())
            }
            _other => {
                self.level -= 1;
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                return Err(());
            }
        }
    }

    #[inline]
    fn try_reduce_call_or_acc(&mut self) -> ParseResult<Expr> {
        debug_call_info!(self);
        let acc = self.try_reduce_acc().map_err(|_| self.stack_dec())?;
        log!(err "{acc}");
        if let Some(res) = self.opt_reduce_args() {
            let args = res.map_err(|_| self.stack_dec())?;
            log!(err "{args}");
            let (obj, method_name) = match acc {
                Accessor::Attr(attr) => (*attr.obj, Some(attr.name.symbol)),
                Accessor::Local(local) => (Expr::Accessor(Accessor::Local(local)), None),
                _ => todo!(),
            };
            let call = Call::new(obj, method_name, args);
            self.level -= 1;
            Ok(Expr::Call(call))
        } else {
            log!(err "not args");
            self.level -= 1;
            Ok(Expr::Accessor(acc))
        }
    }

    #[inline]
    fn try_reduce_unary(&mut self) -> ParseResult<UnaryOp> {
        debug_call_info!(self);
        let op = self.lpop();
        let expr = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(UnaryOp::new(op, expr))
    }

    #[inline]
    fn try_reduce_array(&mut self) -> ParseResult<Array> {
        debug_call_info!(self);
        let l_sqbr = self.lpop();
        let inner = self.try_reduce_elems().map_err(|_| self.stack_dec())?;
        let r_sqbr = self.lpop();
        if !r_sqbr.is(RSqBr) {
            self.level -= 1;
            self.errs
                .push(ParseError::simple_syntax_error(0, r_sqbr.loc()));
            return Err(());
        }
        let arr = match inner {
            ArrayInner::Normal(mut elems) => {
                let elems = if elems
                    .pos_args()
                    .get(0)
                    .map(|pos| match &pos.expr {
                        Expr::Tuple(tup) => tup.paren().is_none(),
                        _ => false,
                    })
                    .unwrap_or(false)
                {
                    enum_unwrap!(elems.remove_pos(0).expr, Expr::Tuple:(Tuple::Normal:(_))).elems
                } else {
                    elems
                };
                Array::Normal(NormalArray::new(l_sqbr, r_sqbr, elems))
            }
            ArrayInner::WithLength(elem, len) => {
                Array::WithLength(ArrayWithLength::new(l_sqbr, r_sqbr, elem, len))
            }
            ArrayInner::Comprehension { .. } => todo!(),
        };
        self.level -= 1;
        Ok(arr)
    }

    /// Set, Dict, Record
    fn try_reduce_brace_container(&mut self) -> ParseResult<BraceContainer> {
        debug_call_info!(self);
        let l_brace = self.lpop();
        if self.cur_is(Newline) {
            self.skip();
            if self.cur_is(Indent) {
                self.skip();
            } else {
                todo!()
            }
        }
        // TODO: K: V
        let first = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
        match first {
            Expr::Def(def) => {
                let record = self
                    .try_reduce_record(l_brace, def)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(BraceContainer::Record(record))
            }
            Expr::TypeAsc(_) => todo!(), // invalid syntax
            other => {
                let set = self
                    .try_reduce_set(l_brace, other)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(BraceContainer::Set(set))
            }
        }
    }

    fn try_reduce_record(&mut self, l_brace: Token, first: Def) -> ParseResult<Record> {
        debug_call_info!(self);
        let mut attrs = vec![first];
        loop {
            match self.peek() {
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                    if self.cur_is(Dedent) {
                        self.skip();
                        if self.cur_is(RBrace) {
                            let r_brace = self.lpop();
                            self.level -= 1;
                            let attrs = RecordAttrs::from(attrs);
                            return Ok(Record::new(l_brace, r_brace, attrs));
                        } else {
                            todo!()
                        }
                    }
                    let def = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                    let def = option_enum_unwrap!(def, Expr::Def).unwrap_or_else(|| todo!());
                    attrs.push(def);
                }
                Some(term) if term.is(RBrace) => {
                    let r_brace = self.lpop();
                    self.level -= 1;
                    let attrs = RecordAttrs::from(attrs);
                    return Ok(Record::new(l_brace, r_brace, attrs));
                }
                _ => todo!(),
            }
        }
    }

    fn _try_reduce_dict() -> ParseResult<Dict> {
        todo!()
    }

    fn try_reduce_set(&mut self, _l_brace: Token, _first: Expr) -> ParseResult<Set> {
        todo!()
    }

    fn try_reduce_tuple(&mut self, first_elem: Expr) -> ParseResult<Tuple> {
        debug_call_info!(self);
        let mut args = Args::new(vec![PosArg::new(first_elem)], vec![], None);
        loop {
            match self.peek() {
                Some(t) if t.is(Comma) => {
                    self.skip();
                    if self.cur_is(Comma) {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                    match self.try_reduce_arg().map_err(|_| self.stack_dec())? {
                        PosOrKwArg::Pos(arg) => match arg.expr {
                            Expr::Tuple(Tuple::Normal(tup)) if tup.elems.paren.is_none() => {
                                args.extend_pos(tup.elems.into_iters().0);
                            }
                            other => {
                                args.push_pos(PosArg::new(other));
                            }
                        },
                        PosOrKwArg::Kw(_arg) => todo!(),
                    }
                }
                Some(t) if t.is(Newline) => {
                    while self.cur_is(Newline) {
                        self.skip();
                    }
                    match self.try_reduce_arg().map_err(|_| self.stack_dec())? {
                        PosOrKwArg::Pos(arg) => {
                            args.push_pos(arg);
                        }
                        PosOrKwArg::Kw(_arg) => todo!(),
                    }
                }
                _ => {
                    break;
                }
            }
        }
        let tup = Tuple::Normal(NormalTuple::new(args));
        self.level -= 1;
        Ok(tup)
    }

    #[inline]
    fn try_reduce_lit(&mut self) -> ParseResult<Literal> {
        debug_call_info!(self);
        self.level -= 1;
        match self.peek() {
            Some(t) if t.category_is(TC::Literal) => Ok(Literal::from(self.lpop())),
            _ => {
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                Err(())
            }
        }
    }

    /// Call: F(x) -> SubrSignature: F(x)
    fn convert_rhs_to_sig(&mut self, rhs: Expr) -> ParseResult<Signature> {
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
            other => todo!("{other}"), // Error
        }
    }

    fn convert_call_to_subr_sig(&mut self, _call: Call) -> ParseResult<SubrSignature> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_accessor_to_var_sig(&mut self, _accessor: Accessor) -> ParseResult<VarSignature> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_array_to_array_pat(&mut self, _array: Array) -> ParseResult<VarArrayPattern> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_record_to_record_pat(&mut self, _record: Record) -> ParseResult<VarRecordPattern> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_tuple_to_tuple_pat(&mut self, _tuple: Tuple) -> ParseResult<VarTuplePattern> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_type_asc_to_sig(&mut self, _tasc: TypeAscription) -> ParseResult<Signature> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_rhs_to_lambda_sig(&mut self, rhs: Expr) -> ParseResult<LambdaSignature> {
        debug_call_info!(self);
        match rhs {
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
                    .convert_array_to_param_pat(array)
                    .map_err(|_| self.stack_dec())?;
                let param = ParamSignature::new(ParamPattern::Array(arr), None, None);
                let params = Params::new(vec![param], None, vec![], None);
                self.level -= 1;
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Record(record) => {
                let rec = self
                    .convert_record_to_param_pat(record)
                    .map_err(|_| self.stack_dec())?;
                let param = ParamSignature::new(ParamPattern::Record(rec), None, None);
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
            other => todo!("{other}"), // Error
        }
    }

    fn convert_accessor_to_param_sig(
        &mut self,
        _accessor: Accessor,
    ) -> ParseResult<ParamSignature> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_tuple_to_params(&mut self, _tuple: Tuple) -> ParseResult<Params> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_array_to_param_pat(&mut self, _array: Array) -> ParseResult<ParamArrayPattern> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_record_to_param_pat(&mut self, _record: Record) -> ParseResult<ParamRecordPattern> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_type_asc_to_lambda_sig(
        &mut self,
        _tasc: TypeAscription,
    ) -> ParseResult<LambdaSignature> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_rhs_to_type_spec(&mut self, rhs: Expr) -> ParseResult<TypeSpec> {
        match rhs {
            Expr::Accessor(acc) => {
                let predecl = self
                    .convert_accessor_to_predecl_type_spec(acc)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(TypeSpec::PreDeclTy(predecl))
            }
            Expr::Call(call) => {
                let predecl = self
                    .convert_call_to_predecl_type_spec(call)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(TypeSpec::PreDeclTy(predecl))
            }
            Expr::Lambda(lambda) => {
                let lambda = self
                    .convert_lambda_to_subr_type_spec(lambda)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(TypeSpec::Subr(lambda))
            }
            Expr::Array(array) => {
                let array = self
                    .convert_array_to_array_type_spec(array)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(TypeSpec::Array(array))
            }
            other => todo!("{other}"),
        }
    }

    fn convert_accessor_to_predecl_type_spec(
        &mut self,
        _accessor: Accessor,
    ) -> ParseResult<PreDeclTypeSpec> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_call_to_predecl_type_spec(&mut self, _call: Call) -> ParseResult<PreDeclTypeSpec> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_lambda_to_subr_type_spec(&mut self, _lambda: Lambda) -> ParseResult<SubrTypeSpec> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_array_to_array_type_spec(&mut self, _array: Array) -> ParseResult<ArrayTypeSpec> {
        debug_call_info!(self);
        todo!()
    }
}
