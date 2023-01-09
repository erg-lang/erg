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
use erg_common::str::Str;
use erg_common::traits::DequeStream;
use erg_common::traits::Runnable;
use erg_common::traits::{Locational, Stream};
use erg_common::{
    caused_by, debug_power_assert, enum_unwrap, fn_name, impl_locational_for_enum, log, set,
    switch_lang, switch_unreachable,
};

use crate::ast::*;
use crate::error::{ParseError, ParseErrors, ParseResult, ParserRunnerError, ParserRunnerErrors};
use crate::lex::Lexer;
use crate::token::{Token, TokenCategory, TokenKind, TokenStream};

use TokenCategory as TC;
use TokenKind::*;

#[macro_export]
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
        generators: Vec<(Identifier, Expr)>,
        guards: Vec<Expr>,
    },
}

pub enum BraceContainer {
    Set(Set),
    Dict(Dict),
    Record(Record),
}

impl_locational_for_enum!(BraceContainer; Set, Dict, Record);

/// Perform recursive descent parsing.
///
/// `level` is raised by 1 by `debug_call_info!` in each analysis method and lowered by 1 when leaving (`.map_err` is called to lower the level).
///
/// To enhance error descriptions, the parsing process will continue as long as it's not fatal.
#[derive(Debug)]
pub struct Parser {
    counter: DefId,
    pub(super) level: usize, // nest level (for debugging)
    tokens: TokenStream,
    warns: ParseErrors,
    pub(crate) errs: ParseErrors,
}

impl Parser {
    pub const fn new(ts: TokenStream) -> Self {
        Self {
            counter: DefId(0),
            level: 0,
            tokens: ts,
            warns: ParseErrors::empty(),
            errs: ParseErrors::empty(),
        }
    }

    #[inline]
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.first()
    }

    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|tok| tok.kind)
    }

    #[inline]
    fn nth(&self, idx: usize) -> Option<&Token> {
        self.tokens.get(idx)
    }

    #[inline]
    fn skip(&mut self) {
        self.tokens.pop_front();
    }

    #[inline]
    fn lpop(&mut self) -> Token {
        self.tokens.pop_front().unwrap()
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

    fn next_line(&mut self) {
        while let Some(t) = self.peek() {
            match t.kind {
                Newline => {
                    self.skip();
                    return;
                }
                EOF => return,
                _ => {
                    self.skip();
                }
            }
        }
    }

    fn skip_and_throw_syntax_err(&mut self, caused_by: &str) -> ParseError {
        let loc = self.peek().map(|t| t.loc()).unwrap_or_default();
        log!(err "error caused by: {caused_by}");
        self.next_expr();
        ParseError::simple_syntax_error(0, loc)
    }

    fn skip_and_throw_invalid_chunk_err(&mut self, caused_by: &str, loc: Location) -> ParseError {
        log!(err "error caused by: {caused_by}");
        self.next_line();
        ParseError::invalid_chunk_error(line!() as usize, loc)
    }

    #[inline]
    fn restore(&mut self, token: Token) {
        self.tokens.push_front(token);
    }

    pub(crate) fn stack_dec(&mut self) {
        self.level -= 1;
    }
}

#[derive(Debug, Default)]
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
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        &mut self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

    #[inline]
    fn initialize(&mut self) {}

    #[inline]
    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let ast = self.parse(self.input().read())?;
        println!("{ast}");
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, ParserRunnerErrors> {
        let ast = self.parse(src)?;
        Ok(format!("{ast}"))
    }
}

impl ParserRunner {
    pub fn parse_token_stream(&mut self, ts: TokenStream) -> Result<Module, ParserRunnerErrors> {
        Parser::new(ts)
            .parse()
            .map_err(|errs| ParserRunnerErrors::convert(self.input(), errs))
    }

    pub fn parse(&mut self, src: String) -> Result<Module, ParserRunnerErrors> {
        let ts = Lexer::new(Input::Str(src))
            .lex()
            .map_err(|errs| ParserRunnerErrors::convert(self.input(), errs))?;
        Parser::new(ts)
            .parse()
            .map_err(|errs| ParserRunnerErrors::convert(self.input(), errs))
    }
}

impl Parser {
    pub fn parse(&mut self) -> Result<Module, ParseErrors> {
        if self.tokens.is_empty() {
            return Ok(Module::empty());
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
            let loc = self.peek().map(|t| t.loc()).unwrap_or_default();
            self.errs
                .push(ParseError::compiler_bug(0, loc, fn_name!(), line!()));
            return Err(mem::take(&mut self.errs));
        }
        log!(info "the parsing process has completed (errs: {}).", self.errs.len());
        log!(info "AST:\n{module}");
        if self.errs.is_empty() {
            Ok(module)
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
            match self.peek_kind() {
                Some(Newline | Semi) => {
                    self.skip();
                }
                Some(EOF) => {
                    break;
                }
                Some(_) => {
                    if let Ok(expr) = self.try_reduce_chunk(true, false) {
                        chunks.push(expr);
                        if !self.cur_is(EOF) && !self.cur_category_is(TC::Separator) {
                            let err = self.skip_and_throw_invalid_chunk_err(
                                caused_by!(),
                                chunks.last().unwrap().loc(),
                            );
                            self.errs.push(err);
                        }
                    }
                }
                None => {
                    if !self.errs.is_empty() {
                        self.level -= 1;
                        return Err(());
                    } else {
                        switch_unreachable!()
                    }
                }
            }
        }
        self.level -= 1;
        Ok(chunks)
    }

    // expect the block`= ; . -> =>`
    fn try_reduce_block(&mut self) -> ParseResult<Block> {
        debug_call_info!(self);
        let mut block = Block::with_capacity(2);
        // single line block
        if !self.cur_is(Newline) {
            let chunk = self
                .try_reduce_chunk(true, false)
                .map_err(|_| self.stack_dec())?;
            block.push(chunk);
            if !self.cur_is(Dedent) && !self.cur_category_is(TC::Separator) {
                let err = self
                    .skip_and_throw_invalid_chunk_err(caused_by!(), block.last().unwrap().loc());
                self.level -= 1;
                self.errs.push(err);
            }
            if block.last().unwrap().is_definition() {
                let err = ParseError::simple_syntax_error(0, block.last().unwrap().loc());
                self.level -= 1;
                self.errs.push(err);
                return Err(());
            } else {
                self.level -= 1;
                return Ok(block);
            }
        }
        if !self.cur_is(Newline) {
            let err = self.skip_and_throw_syntax_err(caused_by!());
            self.level -= 1;
            self.errs.push(err);
            return Err(());
        }
        while self.cur_is(Newline) {
            self.skip();
        }
        if !self.cur_is(Indent) {
            let err = self.skip_and_throw_syntax_err(caused_by!());
            self.level -= 1;
            self.errs.push(err);
            return Err(());
        }
        self.skip(); // Indent
        loop {
            match self.peek_kind() {
                Some(Newline) if self.nth_is(1, Dedent) => {
                    let nl = self.lpop();
                    self.skip();
                    self.restore(nl);
                    break;
                }
                // last line dedent without newline
                Some(Dedent) => {
                    self.skip();
                    break;
                }
                Some(Newline | Semi) => {
                    self.skip();
                }
                Some(EOF) => {
                    break;
                }
                Some(_) => {
                    if let Ok(expr) = self.try_reduce_chunk(true, false) {
                        block.push(expr);
                        if !self.cur_is(Dedent) && !self.cur_category_is(TC::Separator) {
                            let err = self.skip_and_throw_invalid_chunk_err(
                                caused_by!(),
                                block.last().unwrap().loc(),
                            );
                            self.level -= 1;
                            self.errs.push(err);
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
            let err = ParseError::failed_to_analyze_block(line!() as usize, loc);
            self.level -= 1;
            self.errs.push(err);
            Err(())
        } else if block.last().unwrap().is_definition() {
            let err = ParseError::invalid_chunk_error(line!() as usize, block.loc());
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
            let expr = self
                .try_reduce_expr(false, false, false, false)
                .map_err(|_| self.stack_dec())?;
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
            if self.cur_is(Newline) {
                self.skip();
            } else {
                self.level -= 1;
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                return Err(());
            }
        }
        self.level -= 1;
        Ok(decs)
    }

    fn try_reduce_type_app_args(&mut self) -> ParseResult<TypeAppArgs> {
        debug_call_info!(self);
        assert!(self.cur_is(VBar));
        let l_vbar = self.lpop();
        let args = self.try_reduce_args(true).map_err(|_| self.stack_dec())?;
        if self.cur_is(VBar) {
            let r_vbar = self.lpop();
            self.level -= 1;
            Ok(TypeAppArgs::new(l_vbar, args, r_vbar))
        } else {
            self.level -= 1;
            let err = self.skip_and_throw_syntax_err(caused_by!());
            self.errs.push(err);
            Err(())
        }
    }

    fn try_reduce_acc_lhs(&mut self) -> ParseResult<Accessor> {
        debug_call_info!(self);
        let acc = match self.peek_kind() {
            Some(Symbol | UBar) => Accessor::local(self.lpop()),
            Some(Dot) => {
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
        self.level -= 1;
        Ok(acc)
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
        match self.peek_kind() {
            Some(Semi) => {
                self.lpop();
                let len = self
                    .try_reduce_expr(false, false, false, false)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                return Ok(ArrayInner::WithLength(elems.remove_pos(0), len));
            }
            Some(VBar) => {
                let err = ParseError::feature_error(
                    line!() as usize,
                    self.peek().unwrap().loc(),
                    "comprehension",
                );
                self.lpop();
                self.errs.push(err);
                self.level -= 1;
                return Err(());
            }
            Some(RParen | RSqBr | RBrace | Dedent | Comma) => {}
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
            match self.peek_kind() {
                Some(Comma) => {
                    self.skip();
                    if self.cur_is(Comma) {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                    elems.push_pos(self.try_reduce_elem().map_err(|_| self.stack_dec())?);
                }
                Some(RParen | RSqBr | RBrace | Dedent) => {
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
                let expr = self
                    .try_reduce_expr(false, false, false, false)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(PosArg::new(expr))
            }
            None => switch_unreachable!(),
        }
    }

    fn opt_reduce_args(&mut self, in_type_args: bool) -> Option<ParseResult<Args>> {
        debug_call_info!(self);
        match self.peek() {
            Some(t)
                if t.category_is(TC::Literal)
                    || t.is(StrInterpLeft)
                    || t.is(Symbol)
                    || t.category_is(TC::UnaryOp)
                    || t.is(LParen)
                    || t.is(LSqBr)
                    || t.is(LBrace)
                    || t.is(UBar) =>
            {
                Some(self.try_reduce_args(in_type_args))
            }
            Some(t)
                if (t.is(Dot) || t.is(DblColon))
                    && !self.nth_is(1, Newline)
                    && !self.nth_is(1, LBrace) =>
            {
                Some(self.try_reduce_args(in_type_args))
            }
            _ => None,
        }
    }

    /// 引数はインデントで区切ることができる(ただしコンマに戻すことはできない)
    ///
    /// ```erg
    /// x = if True, 1, 2
    /// # is equal to
    /// x = if True:
    ///     1
    ///     2
    /// ```
    fn try_reduce_args(&mut self, in_type_args: bool) -> ParseResult<Args> {
        debug_call_info!(self);
        let mut lp = None;
        let rp;
        if self.cur_is(LParen) {
            lp = Some(self.lpop());
        }
        match self.peek_kind() {
            Some(RParen) => {
                rp = Some(self.lpop());
                self.level -= 1;
                return Ok(Args::new(vec![], vec![], Some((lp.unwrap(), rp.unwrap()))));
            }
            Some(RBrace | RSqBr | Dedent) => {
                self.level -= 1;
                return Ok(Args::new(vec![], vec![], None));
            }
            Some(Newline) => {
                self.skip();
                if self.cur_is(Indent) {
                    self.skip();
                }
            }
            _ => {}
        }
        let mut args = match self
            .try_reduce_arg(in_type_args)
            .map_err(|_| self.stack_dec())?
        {
            PosOrKwArg::Pos(arg) => Args::new(vec![arg], vec![], None),
            PosOrKwArg::Kw(arg) => Args::new(vec![], vec![arg], None),
        };
        let mut colon_style = false;
        loop {
            match self.peek_kind() {
                Some(Colon) if colon_style => {
                    self.skip();
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
                Some(Colon) => {
                    self.skip();
                    colon_style = true;
                    while self.cur_is(Newline) {
                        self.skip();
                    }
                    if !self.cur_is(Indent) {
                        let err = self.skip_and_throw_syntax_err("try_reduce_block");
                        self.level -= 1;
                        self.errs.push(err);
                        return Err(());
                    }
                    self.skip();
                }
                Some(Comma) => {
                    self.skip();
                    if colon_style || self.cur_is(Comma) {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                    if !args.kw_is_empty() {
                        args.push_kw(
                            self.try_reduce_kw_arg(in_type_args)
                                .map_err(|_| self.stack_dec())?,
                        );
                    } else {
                        match self
                            .try_reduce_arg(in_type_args)
                            .map_err(|_| self.stack_dec())?
                        {
                            PosOrKwArg::Pos(arg) => {
                                args.push_pos(arg);
                            }
                            PosOrKwArg::Kw(arg) => {
                                args.push_kw(arg);
                            }
                        }
                    }
                }
                Some(RParen) => {
                    if let Some(lp) = lp {
                        let rp = self.lpop();
                        let (pos_args, kw_args, _) = args.deconstruct();
                        args = Args::new(pos_args, kw_args, Some((lp, rp)));
                    } else {
                        // e.g. f(g 1)
                        let (pos_args, kw_args, _) = args.deconstruct();
                        args = Args::new(pos_args, kw_args, None);
                    }
                    break;
                }
                Some(Newline) => {
                    if !colon_style {
                        break;
                    }
                    let last = self.lpop();
                    if self.cur_is(Dedent) {
                        self.skip();
                        self.restore(last);
                        break;
                    }
                }
                Some(_) if colon_style => {
                    if !args.kw_is_empty() {
                        args.push_kw(
                            self.try_reduce_kw_arg(in_type_args)
                                .map_err(|_| self.stack_dec())?,
                        );
                    } else {
                        match self
                            .try_reduce_arg(in_type_args)
                            .map_err(|_| self.stack_dec())?
                        {
                            PosOrKwArg::Pos(arg) => {
                                args.push_pos(arg);
                            }
                            PosOrKwArg::Kw(arg) => {
                                args.push_kw(arg);
                            }
                        }
                    }
                }
                _ => {
                    break;
                }
            }
        }
        self.level -= 1;
        Ok(args)
    }

    fn try_reduce_arg(&mut self, in_type_args: bool) -> ParseResult<PosOrKwArg> {
        debug_call_info!(self);
        match self.peek_kind() {
            Some(Symbol) => {
                if self.nth_is(1, Walrus) {
                    let acc = self.try_reduce_acc_lhs().map_err(|_| self.stack_dec())?;
                    debug_power_assert!(self.cur_is(Walrus));
                    self.skip();
                    let kw = if let Accessor::Ident(n) = acc {
                        n.name.into_token()
                    } else {
                        self.next_expr();
                        self.level -= 1;
                        let err = ParseError::simple_syntax_error(0, acc.loc());
                        self.errs.push(err);
                        return Err(());
                    };
                    let expr = self
                        .try_reduce_expr(false, in_type_args, false, false)
                        .map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(PosOrKwArg::Kw(KwArg::new(kw, None, expr)))
                } else {
                    let expr = self
                        .try_reduce_expr(false, in_type_args, false, false)
                        .map_err(|_| self.stack_dec())?;
                    if self.cur_is(Walrus) {
                        self.skip();
                        let (kw, t_spec) = match expr {
                            Expr::Accessor(Accessor::Ident(n)) => (n.name.into_token(), None),
                            Expr::TypeAsc(tasc) => {
                                if let Expr::Accessor(Accessor::Ident(n)) = *tasc.expr {
                                    let t_spec = TypeSpecWithOp::new(tasc.op, tasc.t_spec);
                                    (n.name.into_token(), Some(t_spec))
                                } else {
                                    self.next_expr();
                                    self.level -= 1;
                                    let err = ParseError::simple_syntax_error(0, tasc.loc());
                                    self.errs.push(err);
                                    return Err(());
                                }
                            }
                            _ => {
                                self.next_expr();
                                self.level -= 1;
                                let err = ParseError::simple_syntax_error(0, expr.loc());
                                self.errs.push(err);
                                return Err(());
                            }
                        };
                        let expr = self
                            .try_reduce_expr(false, in_type_args, false, false)
                            .map_err(|_| self.stack_dec())?;
                        self.level -= 1;
                        Ok(PosOrKwArg::Kw(KwArg::new(kw, t_spec, expr)))
                    } else {
                        self.level -= 1;
                        Ok(PosOrKwArg::Pos(PosArg::new(expr)))
                    }
                }
            }
            Some(_) => {
                let expr = self
                    .try_reduce_expr(false, in_type_args, false, false)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(PosOrKwArg::Pos(PosArg::new(expr)))
            }
            None => switch_unreachable!(),
        }
    }

    fn try_reduce_kw_arg(&mut self, in_type_args: bool) -> ParseResult<KwArg> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => {
                if self.nth_is(1, Walrus) {
                    let acc = self.try_reduce_acc_lhs().map_err(|_| self.stack_dec())?;
                    debug_power_assert!(self.cur_is(Walrus));
                    self.skip();
                    let keyword = if let Accessor::Ident(n) = acc {
                        n.name.into_token()
                    } else {
                        self.next_expr();
                        self.level -= 1;
                        self.errs
                            .push(ParseError::simple_syntax_error(0, acc.loc()));
                        return Err(());
                    };
                    let expr = self
                        .try_reduce_expr(false, in_type_args, false, false)
                        .map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(KwArg::new(keyword, None, expr))
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

    fn try_reduce_method_defs(&mut self, class: Expr, vis: Token) -> ParseResult<Methods> {
        debug_call_info!(self);
        if self.cur_is(Indent) {
            self.skip();
        } else {
            self.level -= 1;
            let err = self.skip_and_throw_syntax_err(caused_by!());
            self.errs.push(err);
            return Err(());
        }
        while self.cur_is(Newline) {
            self.skip();
        }
        let first = self
            .try_reduce_chunk(false, false)
            .map_err(|_| self.stack_dec())?;
        let first = match first {
            Expr::Def(def) => ClassAttr::Def(def),
            Expr::TypeAsc(tasc) => ClassAttr::Decl(tasc),
            _ => {
                // self.restore();
                self.level -= 1;
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                return Err(());
            }
        };
        let mut attrs = vec![first];
        loop {
            match self.peek() {
                Some(t) if t.is(Newline) && self.nth_is(1, Dedent) => {
                    let nl = self.lpop();
                    self.skip();
                    self.restore(nl);
                    break;
                }
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(_) => {
                    let def = self
                        .try_reduce_chunk(false, false)
                        .map_err(|_| self.stack_dec())?;
                    match def {
                        Expr::Def(def) => {
                            attrs.push(ClassAttr::Def(def));
                        }
                        Expr::TypeAsc(tasc) => {
                            attrs.push(ClassAttr::Decl(tasc));
                        }
                        other => {
                            self.errs
                                .push(ParseError::simple_syntax_error(0, other.loc()));
                        }
                    }
                }
                _ => {
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
            }
        }
        let attrs = ClassAttrs::from(attrs);
        let class = Self::expr_to_type_spec(class).map_err(|e| self.errs.push(e))?;
        self.level -= 1;
        Ok(Methods::new(class, vis, attrs))
    }

    fn try_reduce_do_block(&mut self) -> ParseResult<Lambda> {
        debug_call_info!(self);
        let do_symbol = self.lpop();
        let sig = LambdaSignature::do_sig(&do_symbol);
        let op = match &do_symbol.inspect()[..] {
            "do" => Token::from_str(FuncArrow, "->"),
            "do!" => Token::from_str(ProcArrow, "=>"),
            _ => unreachable!(),
        };
        if self.cur_is(Colon) {
            self.lpop();
            let body = self.try_reduce_block().map_err(|_| self.stack_dec())?;
            self.counter.inc();
            self.level -= 1;
            Ok(Lambda::new(sig, op, body, self.counter))
        } else {
            let expr = self
                .try_reduce_expr(false, false, false, false)
                .map_err(|_| self.stack_dec())?;
            let block = Block::new(vec![expr]);
            self.level -= 1;
            Ok(Lambda::new(sig, op, block, self.counter))
        }
    }

    /// chunk = normal expr + def
    fn try_reduce_chunk(&mut self, winding: bool, in_brace: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut stack = Vec::<ExprOrOp>::new();
        stack.push(ExprOrOp::Expr(
            self.try_reduce_bin_lhs(false, in_brace)
                .map_err(|_| self.stack_dec())?,
        ));
        loop {
            match self.peek() {
                Some(arg) if arg.is(Symbol) || arg.category_is(TC::Literal) => {
                    let args = self.try_reduce_args(false).map_err(|_| self.stack_dec())?;
                    let obj = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    stack.push(ExprOrOp::Expr(obj.call_expr(args)));
                }
                Some(op) if op.category_is(TC::DefOp) => {
                    let op = self.lpop();
                    let is_multiline_block = self.cur_is(Newline);
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self.convert_rhs_to_sig(lhs).map_err(|_| self.stack_dec())?;
                    self.counter.inc();
                    let block = if is_multiline_block {
                        self.try_reduce_block().map_err(|_| self.stack_dec())?
                    } else {
                        // precedence: `=` < `,`
                        let expr = self
                            .try_reduce_expr(true, false, false, false)
                            .map_err(|_| self.stack_dec())?;
                        Block::new(vec![expr])
                    };
                    let body = DefBody::new(op, block, self.counter);
                    self.level -= 1;
                    return Ok(Expr::Def(Def::new(sig, body)));
                }
                Some(op) if op.category_is(TC::LambdaOp) => {
                    let op = self.lpop();
                    let is_multiline_block = self.cur_is(Newline);
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self
                        .convert_rhs_to_lambda_sig(lhs)
                        .map_err(|_| self.stack_dec())?;
                    self.counter.inc();
                    let block = if is_multiline_block {
                        self.try_reduce_block().map_err(|_| self.stack_dec())?
                    } else {
                        // precedence: `->` > `,`
                        let expr = self
                            .try_reduce_expr(false, false, false, false)
                            .map_err(|_| self.stack_dec())?;
                        Block::new(vec![expr])
                    };
                    stack.push(ExprOrOp::Expr(Expr::Lambda(Lambda::new(
                        sig,
                        op,
                        block,
                        self.counter,
                    ))));
                }
                // type ascription
                Some(op)
                    if (op.is(Colon) && !self.nth_is(1, Newline))
                        || (op.is(SubtypeOf) || op.is(SupertypeOf)) =>
                {
                    // "a": 1 (key-value pair)
                    if in_brace {
                        while stack.len() >= 3 {
                            collect_last_binop_on_stack(&mut stack);
                        }
                        break;
                    }
                    let op = self.lpop();
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let t_spec = self
                        .try_reduce_expr(false, false, false, false)
                        .map_err(|_| self.stack_dec())?;
                    let t_spec = Self::expr_to_type_spec(t_spec).map_err(|e| self.errs.push(e))?;
                    let expr = lhs.type_asc_expr(op, t_spec);
                    stack.push(ExprOrOp::Expr(expr));
                }
                Some(op) if op.category_is(TC::BinOp) => {
                    let op_prec = op.kind.precedence();
                    if stack.len() >= 2 {
                        while let Some(ExprOrOp::Op(prev_op)) = stack.get(stack.len() - 2) {
                            if prev_op.category_is(TC::BinOp)
                                && prev_op.kind.precedence() >= op_prec
                            {
                                collect_last_binop_on_stack(&mut stack);
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
                        self.try_reduce_bin_lhs(false, in_brace)
                            .map_err(|_| self.stack_dec())?,
                    ));
                }
                Some(t) if t.is(DblColon) => {
                    let vis = self.lpop();
                    match self.lpop() {
                        symbol if symbol.is(Symbol) => {
                            let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                                self.level -= 1;
                                let err = self.skip_and_throw_syntax_err(caused_by!());
                                self.errs.push(err);
                                return Err(());
                            };
                            if let Some(args) = self
                                .opt_reduce_args(false)
                                .transpose()
                                .map_err(|_| self.stack_dec())?
                            {
                                let ident = Identifier::new(None, VarName::new(symbol));
                                let call = Call::new(obj, Some(ident), args);
                                stack.push(ExprOrOp::Expr(Expr::Call(call)));
                            } else {
                                let ident = Identifier::new(None, VarName::new(symbol));
                                stack.push(ExprOrOp::Expr(obj.attr_expr(ident)));
                            }
                        }
                        line_break if line_break.is(Newline) => {
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            let defs = self
                                .try_reduce_method_defs(maybe_class, vis)
                                .map_err(|_| self.stack_dec())?;
                            let expr = Expr::Methods(defs);
                            assert_eq!(stack.len(), 0);
                            self.level -= 1;
                            return Ok(expr);
                        }
                        l_brace if l_brace.is(LBrace) => {
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            self.restore(l_brace);
                            let container = self
                                .try_reduce_brace_container()
                                .map_err(|_| self.stack_dec())?;
                            match container {
                                BraceContainer::Record(args) => {
                                    let pack = DataPack::new(maybe_class, vis, args);
                                    stack.push(ExprOrOp::Expr(Expr::DataPack(pack)));
                                }
                                BraceContainer::Dict(_) | BraceContainer::Set(_) => {
                                    // self.restore(other);
                                    self.level -= 1;
                                    let err = self.skip_and_throw_syntax_err(caused_by!());
                                    self.errs.push(err);
                                    return Err(());
                                }
                            }
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
                    let vis = self.lpop();
                    match self.lpop() {
                        symbol if symbol.is(Symbol) => {
                            let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                                self.level -= 1;
                                let err = self.skip_and_throw_syntax_err(caused_by!());
                                self.errs.push(err);
                                return Err(());
                            };
                            if let Some(args) = self
                                .opt_reduce_args(false)
                                .transpose()
                                .map_err(|_| self.stack_dec())?
                            {
                                let ident = Identifier::new(Some(vis), VarName::new(symbol));
                                let call = Expr::Call(Call::new(obj, Some(ident), args));
                                stack.push(ExprOrOp::Expr(call));
                            } else {
                                let ident = Identifier::new(Some(vis), VarName::new(symbol));
                                stack.push(ExprOrOp::Expr(obj.attr_expr(ident)));
                            }
                        }
                        line_break if line_break.is(Newline) => {
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            let defs = self
                                .try_reduce_method_defs(maybe_class, vis)
                                .map_err(|_| self.stack_dec())?;
                            return Ok(Expr::Methods(defs));
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
                Some(t) if t.is(LSqBr) => {
                    let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    };
                    self.skip();
                    let index = self
                        .try_reduce_expr(false, false, in_brace, false)
                        .map_err(|_| self.stack_dec())?;
                    let r_sqbr = self.lpop();
                    if !r_sqbr.is(RSqBr) {
                        self.restore(r_sqbr);
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                    let acc = Accessor::subscr(obj, index, r_sqbr);
                    stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
                }
                Some(t) if t.is(Comma) && winding => {
                    let first_elem = PosOrKwArg::Pos(PosArg::new(
                        enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_))),
                    ));
                    let tup = self
                        .try_reduce_nonempty_tuple(first_elem, false)
                        .map_err(|_| self.stack_dec())?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tup)));
                }
                Some(t) if t.is(Walrus) && winding => {
                    let tuple = self
                        .try_reduce_default_parameters(&mut stack, in_brace)
                        .map_err(|_| self.stack_dec())?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tuple)));
                }
                Some(t) if t.is(Pipe) => self
                    .try_reduce_stream_operator(&mut stack)
                    .map_err(|_| self.stack_dec())?,
                Some(t) if t.category_is(TC::Reserved) => {
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
                _ => {
                    if stack.len() <= 1 {
                        break;
                    }
                    // else if stack.len() == 2 { switch_unreachable!() }
                    else {
                        while stack.len() >= 3 {
                            collect_last_binop_on_stack(&mut stack);
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

    /// chunk = expr + def
    /// winding: true => parse paren-less tuple
    /// in_brace: true => (1: 1) will not be a syntax error (key-value pair)
    fn try_reduce_expr(
        &mut self,
        winding: bool,
        in_type_args: bool,
        in_brace: bool,
        line_break: bool,
    ) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut stack = Vec::<ExprOrOp>::new();
        stack.push(ExprOrOp::Expr(
            self.try_reduce_bin_lhs(in_type_args, in_brace)
                .map_err(|_| self.stack_dec())?,
        ));
        loop {
            match self.peek() {
                Some(op) if op.category_is(TC::LambdaOp) => {
                    let op = self.lpop();
                    let is_multiline_block = self.cur_is(Newline);
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self
                        .convert_rhs_to_lambda_sig(lhs)
                        .map_err(|_| self.stack_dec())?;
                    self.counter.inc();
                    let block = if is_multiline_block {
                        self.try_reduce_block().map_err(|_| self.stack_dec())?
                    } else {
                        let expr = self
                            .try_reduce_expr(false, false, false, false)
                            .map_err(|_| self.stack_dec())?;
                        Block::new(vec![expr])
                    };
                    stack.push(ExprOrOp::Expr(Expr::Lambda(Lambda::new(
                        sig,
                        op,
                        block,
                        self.counter,
                    ))));
                }
                // type ascription
                Some(op)
                    if (op.is(Colon) && !self.nth_is(1, Newline))
                        || (op.is(SubtypeOf) || op.is(SupertypeOf)) =>
                {
                    // "a": 1 (key-value pair)
                    if in_brace {
                        break;
                    }
                    let op = self.lpop();
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let t_spec = self
                        .try_reduce_expr(false, in_type_args, in_brace, false)
                        .map_err(|_| self.stack_dec())?;
                    let t_spec = Self::expr_to_type_spec(t_spec).map_err(|e| self.errs.push(e))?;
                    let expr = lhs.type_asc_expr(op, t_spec);
                    stack.push(ExprOrOp::Expr(expr));
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
                        self.try_reduce_bin_lhs(in_type_args, in_brace)
                            .map_err(|_| self.stack_dec())?,
                    ));
                }
                Some(t) if t.is(Dot) => {
                    let vis = self.lpop();
                    match self.lpop() {
                        symbol if symbol.is(Symbol) => {
                            let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                                self.level -= 1;
                                let err = self.skip_and_throw_syntax_err(caused_by!());
                                self.errs.push(err);
                                return Err(());
                            };
                            if let Some(args) = self
                                .opt_reduce_args(in_type_args)
                                .transpose()
                                .map_err(|_| self.stack_dec())?
                            {
                                let ident = Identifier::new(Some(vis), VarName::new(symbol));
                                let call = Call::new(obj, Some(ident), args);
                                stack.push(ExprOrOp::Expr(Expr::Call(call)));
                            } else {
                                let ident = Identifier::new(Some(vis), VarName::new(symbol));
                                stack.push(ExprOrOp::Expr(obj.attr_expr(ident)));
                            }
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
                Some(t) if t.is(LSqBr) => {
                    let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    };
                    self.skip();
                    let index = self
                        .try_reduce_expr(false, false, in_brace, false)
                        .map_err(|_| self.stack_dec())?;
                    let r_sqbr = self.lpop();
                    if !r_sqbr.is(RSqBr) {
                        self.restore(r_sqbr);
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                    let acc = Accessor::subscr(obj, index, r_sqbr);
                    stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
                }
                Some(t) if t.is(Comma) && winding => {
                    let first_elem = PosOrKwArg::Pos(PosArg::new(
                        enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_))),
                    ));
                    let tup = self
                        .try_reduce_nonempty_tuple(first_elem, line_break)
                        .map_err(|_| self.stack_dec())?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tup)));
                }
                Some(t) if t.is(Walrus) && winding => {
                    let tuple = self
                        .try_reduce_default_parameters(&mut stack, in_brace)
                        .map_err(|_| self.stack_dec())?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tuple)));
                }
                Some(t) if t.is(Pipe) => self
                    .try_reduce_stream_operator(&mut stack)
                    .map_err(|_| self.stack_dec())?,
                Some(t) if t.category_is(TC::Reserved) => {
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
                _ => {
                    if stack.len() <= 1 {
                        break;
                    }
                    // else if stack.len() == 2 { switch_unreachable!() }
                    else {
                        while stack.len() >= 3 {
                            collect_last_binop_on_stack(&mut stack);
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

    #[inline]
    fn try_reduce_default_parameters(
        &mut self,
        stack: &mut Vec<ExprOrOp>,
        in_brace: bool,
    ) -> ParseResult<Tuple> {
        debug_call_info!(self);
        let first_elem = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
        let (keyword, t_spec) = match first_elem {
            Expr::Accessor(Accessor::Ident(ident)) => (ident.name.into_token(), None),
            Expr::TypeAsc(tasc) => {
                if let Expr::Accessor(Accessor::Ident(ident)) = *tasc.expr {
                    (
                        ident.name.into_token(),
                        Some(TypeSpecWithOp::new(tasc.op, tasc.t_spec)),
                    )
                } else {
                    self.level -= 1;
                    let err = ParseError::simple_syntax_error(line!() as usize, tasc.loc());
                    self.errs.push(err);
                    return Err(());
                }
            }
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                return Err(());
            }
        };
        self.skip(); // :=
        let rhs = self
            .try_reduce_expr(false, false, in_brace, false)
            .map_err(|_| self.stack_dec())?;
        let first_elem = PosOrKwArg::Kw(KwArg::new(keyword, t_spec, rhs));
        let tuple = self
            .try_reduce_nonempty_tuple(first_elem, false)
            .map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(tuple)
    }

    /// "LHS" is the smallest unit that can be the left-hand side of an BinOp.
    /// e.g. Call, Name, UnaryOp, Lambda
    fn try_reduce_bin_lhs(&mut self, in_type_args: bool, in_brace: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if &t.inspect()[..] == "do" || &t.inspect()[..] == "do!" => {
                let lambda = self.try_reduce_do_block().map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(Expr::Lambda(lambda))
            }
            Some(t) if t.category_is(TC::Literal) => {
                let lit = self.try_reduce_lit().map_err(|_| self.stack_dec())?;
                if let Some(tk) = self.peek() {
                    if tk.is(Mutate) {
                        self.level -= 1;
                        let err = ParseError::invalid_mutable_symbol(
                            line!() as usize,
                            &lit.token.inspect()[..],
                            lit.loc(),
                        );
                        self.errs.push(err);
                        return Err(());
                    }
                }
                self.level -= 1;
                Ok(Expr::Lit(lit))
            }
            Some(t) if t.is(StrInterpLeft) => {
                let str_interp = self
                    .try_reduce_string_interpolation()
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(str_interp)
            }
            Some(t) if t.is(AtSign) => {
                let decos = self.opt_reduce_decorators()?;
                let expr = self.try_reduce_chunk(false, in_brace)?;
                let Some(mut def) = option_enum_unwrap!(expr, Expr::Def) else {
                    // self.restore(other);
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                };
                match def.sig {
                    Signature::Subr(mut subr) => {
                        subr.decorators = decos;
                        let expr = Expr::Def(Def::new(Signature::Subr(subr), def.body));
                        Ok(expr)
                    }
                    Signature::Var(var) => {
                        let mut last = def.body.block.pop().unwrap();
                        for deco in decos.into_iter() {
                            last = deco.into_expr().call_expr(Args::new(
                                vec![PosArg::new(last)],
                                vec![],
                                None,
                            ));
                        }
                        def.body.block.push(last);
                        let expr = Expr::Def(Def::new(Signature::Var(var), def.body));
                        Ok(expr)
                    }
                }
            }
            Some(t) if t.is(Symbol) || t.is(Dot) || t.is(UBar) => {
                let call_or_acc = self
                    .try_reduce_call_or_acc(in_type_args)
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
                while self.cur_is(Newline) {
                    self.skip();
                }
                let line_break = if self.cur_is(Indent) {
                    self.skip();
                    true
                } else {
                    false
                };
                if self.cur_is(RParen) {
                    let rparen = self.lpop();
                    let args = Args::new(vec![], vec![], Some((lparen, rparen)));
                    let unit = Tuple::Normal(NormalTuple::new(args));
                    self.level -= 1;
                    return Ok(Expr::Tuple(unit));
                }
                let mut expr = self
                    .try_reduce_expr(true, false, false, line_break)
                    .map_err(|_| self.stack_dec())?;
                while self.cur_is(Newline) {
                    self.skip();
                }
                if self.cur_is(Dedent) {
                    self.skip();
                }
                let rparen = self.lpop();
                if let Expr::Tuple(Tuple::Normal(tup)) = &mut expr {
                    tup.elems.paren = Some((lparen, rparen));
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
            Some(t) if t.is(VBar) => {
                let type_args = self
                    .try_reduce_type_app_args()
                    .map_err(|_| self.stack_dec())?;
                let bounds = self
                    .convert_type_args_to_bounds(type_args)
                    .map_err(|_| self.stack_dec())?;
                let args = self.try_reduce_args(false).map_err(|_| self.stack_dec())?;
                let params = self
                    .convert_args_to_params(args)
                    .map_err(|_| self.stack_dec())?;
                if !self.cur_category_is(TC::LambdaOp) {
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
                let sig = LambdaSignature::new(params, None, bounds);
                let op = self.lpop();
                let block = self.try_reduce_block().map_err(|_| self.stack_dec())?;
                self.counter.inc();
                self.level -= 1;
                let lambda = Lambda::new(sig, op, block, self.counter);
                Ok(Expr::Lambda(lambda))
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
                Err(())
            }
        }
    }

    #[inline]
    fn try_reduce_call_or_acc(&mut self, in_type_args: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        let acc = self.try_reduce_acc_lhs().map_err(|_| self.stack_dec())?;
        let mut call_or_acc = self.try_reduce_acc_chain(acc, in_type_args)?;
        while let Some(res) = self.opt_reduce_args(in_type_args) {
            let args = res.map_err(|_| self.stack_dec())?;
            let (receiver, attr_name) = match call_or_acc {
                Expr::Accessor(Accessor::Attr(attr)) => (*attr.obj, Some(attr.ident)),
                other => (other, None),
            };
            let call = Call::new(receiver, attr_name, args);
            call_or_acc = Expr::Call(call);
        }
        self.level -= 1;
        Ok(call_or_acc)
    }

    /// [y], .0, .attr, .method(...), (...)
    #[inline]
    fn try_reduce_acc_chain(&mut self, acc: Accessor, in_type_args: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut obj = Expr::Accessor(acc);
        loop {
            match self.peek() {
                Some(t) if t.is(LSqBr) && obj.col_end() == t.col_begin() => {
                    let _l_sqbr = self.lpop();
                    let index = self
                        .try_reduce_expr(true, false, false, false)
                        .map_err(|_| self.stack_dec())?;
                    let r_sqbr = if self.cur_is(RSqBr) {
                        self.lpop()
                    } else {
                        self.level -= 1;
                        // TODO: error report: RSqBr not found
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    };
                    obj = Expr::Accessor(Accessor::subscr(obj, index, r_sqbr));
                }
                Some(t) if t.is(Dot) && obj.col_end() == t.col_begin() => {
                    let vis = self.lpop();
                    let token = self.lpop();
                    match token.kind {
                        Symbol => {
                            let ident = Identifier::new(Some(vis), VarName::new(token));
                            obj = obj.attr_expr(ident);
                        }
                        NatLit => {
                            let index = Literal::from(token);
                            obj = obj.tuple_attr_expr(index);
                        }
                        Newline => {
                            self.restore(token);
                            self.restore(vis);
                            break;
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
                // e.g. l[0].0
                Some(t) if t.is(RatioLit) && obj.col_end() == t.col_begin() => {
                    let mut token = self.lpop();
                    token.content = Str::rc(&token.content[1..]);
                    token.kind = NatLit;
                    token.col_begin += 1;
                    obj = obj.tuple_attr_expr(Literal::from(token));
                }
                Some(t) if t.is(DblColon) && obj.col_end() == t.col_begin() => {
                    let vis = self.lpop();
                    let token = self.lpop();
                    match token.kind {
                        Symbol => {
                            let ident = Identifier::new(None, VarName::new(token));
                            obj = obj.attr_expr(ident);
                        }
                        LBrace => {
                            self.restore(token);
                            let args = self
                                .try_reduce_brace_container()
                                .map_err(|_| self.stack_dec())?;
                            match args {
                                BraceContainer::Record(args) => {
                                    obj = Expr::DataPack(DataPack::new(obj, vis, args));
                                }
                                other => {
                                    self.level -= 1;
                                    let err = ParseError::simple_syntax_error(
                                        line!() as usize,
                                        other.loc(),
                                    );
                                    self.errs.push(err);
                                    return Err(());
                                }
                            }
                        }
                        // MethodDefs
                        Newline => {
                            self.restore(token);
                            self.restore(vis);
                            break;
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
                Some(t) if t.is(LParen) && obj.col_end() == t.col_begin() => {
                    let args = self.try_reduce_args(false).map_err(|_| self.stack_dec())?;
                    let (receiver, attr_name) = match obj {
                        Expr::Accessor(Accessor::Attr(attr)) => (*attr.obj, Some(attr.ident)),
                        other => (other, None),
                    };
                    let call = Call::new(receiver, attr_name, args);
                    obj = Expr::Call(call);
                }
                Some(t) if t.is(VBar) && !in_type_args => {
                    let type_args = self
                        .try_reduce_type_app_args()
                        .map_err(|_| self.stack_dec())?;
                    obj = Expr::Accessor(Accessor::TypeApp(TypeApp::new(obj, type_args)));
                }
                _ => {
                    break;
                }
            }
        }
        self.level -= 1;
        Ok(obj)
    }

    #[inline]
    fn try_reduce_unary(&mut self) -> ParseResult<UnaryOp> {
        debug_call_info!(self);
        let op = self.lpop();
        let expr = self
            .try_reduce_expr(false, false, false, false)
            .map_err(|_| self.stack_dec())?;
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
            ArrayInner::Comprehension { .. } => {
                self.level -= 1;
                self.errs.push(ParseError::feature_error(
                    line!() as usize,
                    Location::concat(&l_sqbr, &r_sqbr),
                    "array comprehension",
                ));
                return Err(());
            }
        };
        self.level -= 1;
        Ok(arr)
    }

    /// Set, Dict, Record
    fn try_reduce_brace_container(&mut self) -> ParseResult<BraceContainer> {
        debug_call_info!(self);
        assert!(self.cur_is(LBrace));
        let l_brace = self.lpop();
        if self.cur_is(Newline) {
            self.skip();
            if self.cur_is(Indent) {
                self.skip();
            } else {
                self.level -= 1;
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                return Err(());
            }
        }

        // Empty brace literals
        if let Some(first) = self.peek() {
            if first.is(RBrace) {
                let r_brace = self.lpop();
                let arg = Args::empty();
                let set = NormalSet::new(l_brace, r_brace, arg);
                return Ok(BraceContainer::Set(Set::Normal(set)));
            }
            if first.is(Equal) {
                let _eq = self.lpop();
                if let Some(t) = self.peek() {
                    if t.is(RBrace) {
                        let r_brace = self.lpop();
                        return Ok(BraceContainer::Record(Record::empty(l_brace, r_brace)));
                    }
                }
                self.level -= 1;
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                return Err(());
            }
            if first.is(Colon) {
                let _colon = self.lpop();
                if let Some(t) = self.peek() {
                    if t.is(RBrace) {
                        let r_brace = self.lpop();
                        let dict = NormalDict::new(l_brace, r_brace, vec![]);
                        return Ok(BraceContainer::Dict(Dict::Normal(dict)));
                    }
                }
                self.level -= 1;
                let err = self.skip_and_throw_syntax_err(caused_by!());
                self.errs.push(err);
                return Err(());
            }
        }

        let first = self
            .try_reduce_chunk(false, true)
            .map_err(|_| self.stack_dec())?;
        match first {
            Expr::Def(def) => {
                let attr = RecordAttrOrIdent::Attr(def);
                let record = self
                    .try_reduce_record(l_brace, attr)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(BraceContainer::Record(record))
            }
            // TODO: {X; Y} will conflict with Set
            Expr::Accessor(acc)
                if self.cur_is(Semi)
                    && !self.nth_is(1, TokenKind::NatLit)
                    && !self.nth_is(1, UBar) =>
            {
                let ident = match acc {
                    Accessor::Ident(ident) => ident,
                    other => {
                        self.level -= 1;
                        let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                        self.errs.push(err);
                        return Err(());
                    }
                };
                let attr = RecordAttrOrIdent::Ident(ident);
                let record = self
                    .try_reduce_record(l_brace, attr)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(BraceContainer::Record(record))
            }
            // Dict
            other if self.cur_is(Colon) => {
                let dict = self
                    .try_reduce_normal_dict(l_brace, other)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(BraceContainer::Dict(Dict::Normal(dict)))
            }
            other => {
                let set = self
                    .try_reduce_set(l_brace, other)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(BraceContainer::Set(set))
            }
        }
    }

    // Note that this accepts:
    //  - {x=expr;y=expr;...}
    //  - {x;y}
    //  - {x;y=expr} (shorthand/normal mixed)
    fn try_reduce_record(
        &mut self,
        l_brace: Token,
        first_attr: RecordAttrOrIdent,
    ) -> ParseResult<Record> {
        debug_call_info!(self);
        let mut attrs = vec![first_attr];
        loop {
            match self.peek_kind() {
                Some(Newline | Semi) => {
                    self.skip();
                }
                Some(Dedent) => {
                    self.skip();
                    if self.cur_is(RBrace) {
                        let r_brace = self.lpop();
                        self.stack_dec();
                        return Ok(Record::new_mixed(l_brace, r_brace, attrs));
                    } else {
                        // TODO: not closed
                        // self.restore(other);
                        self.stack_dec();
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                }
                Some(RBrace) => {
                    let r_brace = self.lpop();
                    self.stack_dec();
                    return Ok(Record::new_mixed(l_brace, r_brace, attrs));
                }
                Some(_) => {
                    let next = self
                        .try_reduce_chunk(false, false)
                        .map_err(|_| self.stack_dec())?;
                    match next {
                        Expr::Def(def) => {
                            attrs.push(RecordAttrOrIdent::Attr(def));
                        }
                        Expr::Accessor(acc) => {
                            let ident = match acc {
                                Accessor::Ident(ident) => ident,
                                other => {
                                    self.stack_dec();
                                    let err = ParseError::simple_syntax_error(
                                        line!() as usize,
                                        other.loc(),
                                    );
                                    self.errs.push(err);
                                    return Err(());
                                }
                            };
                            attrs.push(RecordAttrOrIdent::Ident(ident));
                        }
                        _ => {
                            self.stack_dec();
                            let err = self.skip_and_throw_syntax_err(caused_by!());
                            self.errs.push(err);
                            return Err(());
                        }
                    }
                }
                _ => {
                    //  self.restore(other);
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
            }
        }
    }

    fn try_reduce_normal_dict(
        &mut self,
        l_brace: Token,
        first_key: Expr,
    ) -> ParseResult<NormalDict> {
        debug_call_info!(self);
        assert!(self.cur_is(Colon));
        self.skip();
        let value = self
            .try_reduce_chunk(false, false)
            .map_err(|_| self.stack_dec())?;
        let mut kvs = vec![KeyValue::new(first_key, value)];
        loop {
            match self.peek_kind() {
                Some(Comma) => {
                    self.skip();
                    match self.peek_kind() {
                        Some(Comma) => {
                            self.level -= 1;
                            let err = self.skip_and_throw_syntax_err(caused_by!());
                            self.errs.push(err);
                            return Err(());
                        }
                        Some(RBrace) => {
                            let dict = NormalDict::new(l_brace, self.lpop(), kvs);
                            self.level -= 1;
                            return Ok(dict);
                        }
                        Some(Newline) => {
                            self.skip();
                        }
                        _ => {}
                    }
                    let key = self
                        .try_reduce_expr(false, false, true, false)
                        .map_err(|_| self.stack_dec())?;
                    if self.cur_is(Colon) {
                        self.skip();
                        let value = self
                            .try_reduce_chunk(false, false)
                            .map_err(|_| self.stack_dec())?;
                        kvs.push(KeyValue::new(key, value));
                    } else {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                }
                Some(Newline | Indent | Dedent) => {
                    self.skip();
                }
                Some(RBrace) => {
                    let dict = NormalDict::new(l_brace, self.lpop(), kvs);
                    self.level -= 1;
                    return Ok(dict);
                }
                _ => {
                    break;
                }
            }
        }
        Err(())
    }

    fn try_reduce_set(&mut self, l_brace: Token, first_elem: Expr) -> ParseResult<Set> {
        debug_call_info!(self);
        if self.cur_is(Semi) {
            self.skip();
            let len = self
                .try_reduce_expr(false, false, false, false)
                .map_err(|_| self.stack_dec())?;
            let r_brace = self.lpop();
            return Ok(Set::WithLength(SetWithLength::new(
                l_brace,
                r_brace,
                PosArg::new(first_elem),
                len,
            )));
        }
        let mut args = Args::new(vec![PosArg::new(first_elem)], vec![], None);
        loop {
            match self.peek_kind() {
                Some(Comma) => {
                    self.skip();
                    match self.peek_kind() {
                        Some(Comma) => {
                            self.level -= 1;
                            let err = self.skip_and_throw_syntax_err(caused_by!());
                            self.errs.push(err);
                            return Err(());
                        }
                        Some(RBrace) => {
                            let set = Set::Normal(NormalSet::new(l_brace, self.lpop(), args));
                            self.level -= 1;
                            return Ok(set);
                        }
                        Some(Newline | Indent | Dedent) => {
                            self.skip();
                        }
                        _ => {}
                    }
                    match self.try_reduce_arg(false).map_err(|_| self.stack_dec())? {
                        PosOrKwArg::Pos(arg) => match arg.expr {
                            Expr::Set(Set::Normal(set)) if set.elems.paren.is_none() => {
                                args.extend_pos(set.elems.into_iters().0);
                            }
                            other => {
                                let pos = PosArg::new(other);
                                if !args.has_pos_arg(&pos) {
                                    args.push_pos(pos);
                                }
                            }
                        },
                        PosOrKwArg::Kw(arg) => {
                            self.level -= 1;
                            let err = ParseError::simple_syntax_error(line!() as usize, arg.loc());
                            self.errs.push(err);
                            return Err(());
                        }
                    }
                }
                Some(Newline | Indent | Dedent) => {
                    self.skip();
                }
                Some(RBrace) => {
                    let set = Set::Normal(NormalSet::new(l_brace, self.lpop(), args));
                    self.level -= 1;
                    return Ok(set);
                }
                _ => {
                    break;
                }
            }
        }
        Err(())
    }

    fn try_reduce_nonempty_tuple(
        &mut self,
        first_elem: PosOrKwArg,
        line_break: bool,
    ) -> ParseResult<Tuple> {
        debug_call_info!(self);
        let mut args = match first_elem {
            PosOrKwArg::Pos(pos) => Args::new(vec![pos], vec![], None),
            PosOrKwArg::Kw(kw) => Args::new(vec![], vec![kw], None),
        };
        #[allow(clippy::while_let_loop)]
        loop {
            match self.peek_kind() {
                Some(Comma) => {
                    self.skip();
                    while self.cur_is(Newline) && line_break {
                        self.skip();
                    }
                    if self.cur_is(Comma) {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    } else if self.cur_is(Dedent) || self.cur_is(RParen) {
                        break;
                    }
                    match self.try_reduce_arg(false).map_err(|_| self.stack_dec())? {
                        PosOrKwArg::Pos(arg) if args.kw_is_empty() => match arg.expr {
                            Expr::Tuple(Tuple::Normal(tup)) if tup.elems.paren.is_none() => {
                                args.extend_pos(tup.elems.into_iters().0);
                            }
                            other => {
                                args.push_pos(PosArg::new(other));
                            }
                        },
                        PosOrKwArg::Pos(arg) => {
                            let err = ParseError::syntax_error(
                                line!() as usize,
                                arg.loc(),
                                switch_lang!(
                                    "japanese" => "非デフォルト引数はデフォルト引数の後に指定できません",
                                    "simplified_chinese" => "默认实参后面跟着非默认实参",
                                    "traditional_chinese" => "默認實參後面跟著非默認實參",
                                    "english" => "non-default argument follows default argument",
                                ),
                                None,
                            );
                            self.errs.push(err);
                            return Err(());
                        }
                        // e.g. (x, y:=1) -> ...
                        // Syntax error will occur when trying to use it as a tuple
                        PosOrKwArg::Kw(arg) => {
                            args.push_kw(arg);
                        }
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

    /// "...\{, expr, }..." ==> "..." + str(expr) + "..."
    /// "...\{, expr, }..." ==> "..." + str(expr) + "..."
    fn try_reduce_string_interpolation(&mut self) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut left = self.lpop();
        left.content = Str::from(left.content.trim_end_matches("\\{").to_string() + "\"");
        left.kind = StrLit;
        let mut expr = Expr::Lit(Literal::from(left));
        loop {
            match self.peek() {
                Some(l) if l.is(StrInterpRight) => {
                    let mut right = self.lpop();
                    right.content =
                        Str::from(format!("\"{}", right.content.trim_start_matches('}')));
                    right.kind = StrLit;
                    let right = Expr::Lit(Literal::from(right));
                    let op = Token::new(
                        Plus,
                        "+",
                        right.ln_begin().unwrap(),
                        right.col_begin().unwrap(),
                    );
                    expr = Expr::BinOp(BinOp::new(op, expr, right));
                    self.level -= 1;
                    return Ok(expr);
                }
                Some(_) => {
                    let mid_expr = self.try_reduce_expr(true, false, false, false)?;
                    let str_func = Expr::local(
                        "str",
                        mid_expr.ln_begin().unwrap(),
                        mid_expr.col_begin().unwrap(),
                    );
                    let call = Call::new(
                        str_func,
                        None,
                        Args::new(vec![PosArg::new(mid_expr)], vec![], None),
                    );
                    let op = Token::new(
                        Plus,
                        "+",
                        call.ln_begin().unwrap(),
                        call.col_begin().unwrap(),
                    );
                    let bin = BinOp::new(op, expr, Expr::Call(call));
                    expr = Expr::BinOp(bin);
                    if self.cur_is(StrInterpMid) {
                        let mut mid = self.lpop();
                        mid.content = Str::from(format!(
                            "\"{}\"",
                            mid.content.trim_start_matches('}').trim_end_matches("\\{")
                        ));
                        mid.kind = StrLit;
                        let mid = Expr::Lit(Literal::from(mid));
                        let op = Token::new(
                            Plus,
                            "+",
                            mid.ln_begin().unwrap(),
                            mid.col_begin().unwrap(),
                        );
                        expr = Expr::BinOp(BinOp::new(op, expr, mid));
                    }
                }
                None => {
                    self.level -= 1;
                    let err = ParseError::syntax_error(
                        line!() as usize,
                        expr.loc(),
                        switch_lang!(
                            "japanese" => "文字列補間の終わりが見つかりませんでした",
                            "simplified_chinese" => "未找到字符串插值结束",
                            "traditional_chinese" => "未找到字符串插值結束",
                            "english" => "end of string interpolation not found",
                        ),
                        None,
                    );
                    self.errs.push(err);
                    return Err(());
                }
            }
        }
    }

    /// x |> f() => f(x)
    fn try_reduce_stream_operator(&mut self, stack: &mut Vec<ExprOrOp>) -> ParseResult<()> {
        debug_call_info!(self);
        let op = self.lpop();
        while stack.len() >= 3 {
            collect_last_binop_on_stack(stack);
        }
        if stack.len() == 2 {
            self.errs
                .push(ParseError::compiler_bug(0, op.loc(), fn_name!(), line!()));
            self.stack_dec();
            return Err(());
        }

        fn get_stream_op_syntax_error(loc: Location) -> ParseError {
            ParseError::syntax_error(
                0,
                loc,
                switch_lang!(
                    "japanese" => "パイプ演算子の後には関数・メソッド・サブルーチン呼び出しのみが使用できます。",
                    "simplified_chinese" => "流操作符后只能调用函数、方法或子程序",
                    "traditional_chinese" => "流操作符後只能調用函數、方法或子程序",
                    "english" => "Only a call of function, method or subroutine is available after stream operator.",
                ),
                None,
            )
        }

        if matches!(self.peek_kind(), Some(Dot)) {
            // obj |> .method(...)
            let vis = self.lpop();
            match self.lpop() {
                symbol if symbol.is(Symbol) => {
                    let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    };
                    if let Some(args) = self
                        .opt_reduce_args(false)
                        .transpose()
                        .map_err(|_| self.stack_dec())?
                    {
                        let ident = Identifier::new(Some(vis), VarName::new(symbol));
                        let mut call = Expr::Call(Call::new(obj, Some(ident), args));
                        while let Some(res) = self.opt_reduce_args(false) {
                            let args = res.map_err(|_| self.stack_dec())?;
                            call = call.call_expr(args);
                        }
                        stack.push(ExprOrOp::Expr(call));
                    } else {
                        self.errs.push(get_stream_op_syntax_error(obj.loc()));
                        self.stack_dec();
                        return Err(());
                    }
                }
                other => {
                    self.restore(other);
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
            }
        } else {
            let expect_call = self
                .try_reduce_call_or_acc(false)
                .map_err(|_| self.stack_dec())?;
            let Expr::Call(mut call) = expect_call else {
                self.errs.push(get_stream_op_syntax_error(expect_call.loc()));
                self.stack_dec();
                return Err(());
            };
            let ExprOrOp::Expr(first_arg) = stack.pop().unwrap() else {
                self.errs
                    .push(ParseError::compiler_bug(0, call.loc(), fn_name!(), line!()));
                self.stack_dec();
                return Err(());
            };
            call.args.insert_pos(0, PosArg::new(first_arg));
            stack.push(ExprOrOp::Expr(Expr::Call(call)));
        }
        self.stack_dec();
        Ok(())
    }
}

fn collect_last_binop_on_stack(stack: &mut Vec<ExprOrOp>) {
    let rhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
    let op = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Op:(_)));
    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
    let bin = BinOp::new(op, lhs, rhs);
    stack.push(ExprOrOp::Expr(Expr::BinOp(bin)));
}
