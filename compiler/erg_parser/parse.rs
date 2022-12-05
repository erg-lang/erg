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
    level: usize, // nest level (for debugging)
    tokens: TokenStream,
    warns: ParseErrors,
    errs: ParseErrors,
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
        let loc = self.peek().map(|t| t.loc()).unwrap_or_default();
        log!(err "error caused by: {caused_by}");
        self.next_expr();
        ParseError::simple_syntax_error(0, loc)
    }

    #[inline]
    fn restore(&mut self, token: Token) {
        self.tokens.insert(0, token);
    }

    fn stack_dec(&mut self) {
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
            match self.peek() {
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(t) if t.is(EOF) => {
                    break;
                }
                /*Some(t) if t.is(Indent) => {
                    switch_unreachable!()
                }
                Some(t) if t.is(Dedent) => {
                    switch_unreachable!()
                }*/
                Some(_) => {
                    if let Ok(expr) = self.try_reduce_chunk(true, false) {
                        chunks.push(expr);
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

    fn try_reduce_block(&mut self) -> ParseResult<Block> {
        debug_call_info!(self);
        let mut block = Block::with_capacity(2);
        // single line block
        if !self.cur_is(Newline) {
            let chunk = self
                .try_reduce_chunk(true, false)
                .map_err(|_| self.stack_dec())?;
            block.push(chunk);
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
            let err = self.skip_and_throw_syntax_err("try_reduce_block");
            self.level -= 1;
            self.errs.push(err);
            return Err(());
        }
        self.skip();
        if !self.cur_is(Indent) {
            let err = self.skip_and_throw_syntax_err("try_reduce_block");
            self.level -= 1;
            self.errs.push(err);
            return Err(());
        }
        self.skip();
        loop {
            match self.peek() {
                Some(t) if t.is(Newline) && self.nth_is(1, Dedent) => {
                    let nl = self.lpop();
                    self.skip();
                    self.restore(nl);
                    break;
                }
                // last line dedent without newline
                Some(t) if t.is(Dedent) => {
                    self.skip();
                    break;
                }
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(t) if t.is(EOF) => {
                    break;
                }
                Some(_) => {
                    if let Ok(expr) = self.try_reduce_chunk(true, false) {
                        block.push(expr);
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
        } else if block.last().unwrap().is_definition() {
            let err = ParseError::syntax_error(
                line!() as usize,
                block.last().unwrap().loc(),
                switch_lang!(
                    "japanese" => "ブロックの終端で変数を定義することは出来ません",
                    "simplified_chinese" => "无法在块的末尾定义变量",
                    "traditional_chinese" => "無法在塊的末尾定義變量",
                    "english" => "cannot define a variable at the end of a block",
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
        let acc = match self.peek() {
            Some(t) if t.is(Symbol) || t.is(UBar) => Accessor::local(self.lpop()),
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
        match self.peek() {
            Some(semi) if semi.is(Semi) => {
                self.lpop();
                let len = self
                    .try_reduce_expr(false, false, false, false)
                    .map_err(|_| self.stack_dec())?;
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
    /// ```
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
        if self.cur_is(RParen) {
            rp = Some(self.lpop());
            self.level -= 1;
            return Ok(Args::new(vec![], vec![], Some((lp.unwrap(), rp.unwrap()))));
        } else if self.cur_category_is(TC::REnclosure) {
            self.level -= 1;
            return Ok(Args::new(vec![], vec![], None));
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
                    if !self.cur_is(Indent) {
                        let err = self.skip_and_throw_syntax_err("try_reduce_block");
                        self.level -= 1;
                        self.errs.push(err);
                        return Err(());
                    }
                    self.skip();
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
                Some(t) if t.is(RParen) => {
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
                Some(t) if t.is(Newline) => {
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
        match self.peek() {
            Some(t) if t.is(Symbol) => {
                if &t.inspect()[..] == "do" || &t.inspect()[..] == "do!" {
                    let lambda = self.try_reduce_do_block().map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    return Ok(PosOrKwArg::Pos(PosArg::new(Expr::Lambda(lambda))));
                }
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
                    /*let t_spec = if self.cur_is(Colon) {
                        self.skip();
                        let expr = self.try_reduce_expr(false).map_err(|_| self.stack_dec())?;
                        let t_spec = self
                            .convert_rhs_to_type_spec(expr)
                            .map_err(|_| self.stack_dec())?;
                        Some(t_spec)
                    } else {
                        None
                    };*/
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
                .try_reduce_expr(true, false, false, false)
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
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self.convert_rhs_to_sig(lhs).map_err(|_| self.stack_dec())?;
                    self.counter.inc();
                    let block = self.try_reduce_block().map_err(|_| self.stack_dec())?;
                    let body = DefBody::new(op, block, self.counter);
                    self.level -= 1;
                    return Ok(Expr::Def(Def::new(sig, body)));
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
                        .try_reduce_tuple(first_elem, false)
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
                Some(t) if t.is(Comma) && winding => {
                    let first_elem = PosOrKwArg::Pos(PosArg::new(
                        enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_))),
                    ));
                    let tup = self
                        .try_reduce_tuple(first_elem, line_break)
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
            .try_reduce_tuple(first_elem, false)
            .map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(tuple)
    }

    /// "LHS" is the smallest unit that can be the left-hand side of an BinOp.
    /// e.g. Call, Name, UnaryOp, Lambda
    fn try_reduce_bin_lhs(&mut self, in_type_args: bool, in_brace: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.category_is(TC::Literal) => {
                // TODO: 10.times ...などメソッド呼び出しもある
                let lit = self.try_reduce_lit().map_err(|_| self.stack_dec())?;
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
            match self.peek() {
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(t) if t.is(Dedent) => {
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
                Some(t) if t.is(RBrace) => {
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
            match self.peek() {
                Some(t) if t.is(Comma) => {
                    self.skip();
                    if self.cur_is(Comma) {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    } else if self.cur_is(RBrace) {
                        let dict = NormalDict::new(l_brace, self.lpop(), kvs);
                        self.level -= 1;
                        return Ok(dict);
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
                Some(t) if t.is(RBrace) => {
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
            match self.peek() {
                Some(t) if t.is(Comma) => {
                    self.skip();
                    if self.cur_is(Comma) {
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    } else if self.cur_is(RBrace) {
                        let set = Set::Normal(NormalSet::new(l_brace, self.lpop(), args));
                        self.level -= 1;
                        return Ok(set);
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
                Some(t) if t.is(RBrace) => {
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

    fn try_reduce_tuple(&mut self, first_elem: PosOrKwArg, line_break: bool) -> ParseResult<Tuple> {
        debug_call_info!(self);
        let mut args = match first_elem {
            PosOrKwArg::Pos(pos) => Args::new(vec![pos], vec![], None),
            PosOrKwArg::Kw(kw) => Args::new(vec![], vec![kw], None),
        };
        loop {
            match self.peek() {
                Some(t) if t.is(Comma) => {
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
                    "english" => "Only a call of function, method or subroutine is available after stream operator.",
                ),
                None,
            )
        }

        if matches!(self.peek(), Some(t) if t.is(Dot)) {
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

    fn convert_type_args_to_bounds(
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

    fn convert_args_to_params(&mut self, args: Args) -> ParseResult<Params> {
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

    fn convert_rhs_to_lambda_sig(&mut self, rhs: Expr) -> ParseResult<LambdaSignature> {
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

// The APIs defined below are also used by `ASTLowerer` to interpret expressions as types.
impl Parser {
    pub fn validate_const_expr(expr: Expr) -> Result<ConstExpr, ParseError> {
        match expr {
            Expr::Lit(l) => Ok(ConstExpr::Lit(l)),
            Expr::Accessor(Accessor::Ident(local)) => {
                let local = ConstLocal::new(local.name.into_token());
                Ok(ConstExpr::Accessor(ConstAccessor::Local(local)))
            }
            Expr::Array(array) => match array {
                Array::Normal(arr) => {
                    let (elems, _, _) = arr.elems.deconstruct();
                    let mut const_elems = vec![];
                    for elem in elems.into_iter() {
                        let const_expr = Self::validate_const_expr(elem.expr)?;
                        const_elems.push(ConstPosArg::new(const_expr));
                    }
                    let elems = ConstArgs::new(const_elems, vec![], None);
                    let const_arr = ConstArray::new(arr.l_sqbr, arr.r_sqbr, elems, None);
                    Ok(ConstExpr::Array(const_arr))
                }
                other => Err(ParseError::feature_error(
                    line!() as usize,
                    other.loc(),
                    "???",
                )),
            },
            // TODO: App, Record, BinOp, UnaryOp,
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
                let (_pos_args, _kw_args, paren) = call.args.deconstruct();
                let mut pos_args = vec![];
                for arg in _pos_args.into_iter() {
                    let const_expr = Self::validate_const_expr(arg.expr)?;
                    pos_args.push(ConstPosArg::new(const_expr));
                }
                let mut kw_args = vec![];
                for arg in _kw_args.into_iter() {
                    let const_expr = Self::validate_const_expr(arg.expr)?;
                    kw_args.push(ConstKwArg::new(arg.keyword, const_expr));
                }
                Ok(PreDeclTypeSpec::Simple(SimpleTypeSpec::new(
                    ident,
                    ConstArgs::new(pos_args, kw_args, paren),
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
        let var_args =
            lambda
                .sig
                .params
                .var_args
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
            var_args,
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
                Ok(TypeSpec::Enum(ConstArgs::new(elem_ts, vec![], paren)))
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

    fn tuple_to_tuple_type_spec(tuple: Tuple) -> Result<Vec<TypeSpec>, ParseError> {
        match tuple {
            Tuple::Normal(tup) => {
                let mut tup_spec = vec![];
                let (elems, ..) = tup.elems.deconstruct();
                for elem in elems.into_iter() {
                    let value = Self::expr_to_type_spec(elem.expr)?;
                    tup_spec.push(value);
                }
                Ok(tup_spec)
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
            Expr::Lit(lit) => {
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

fn collect_last_binop_on_stack(stack: &mut Vec<ExprOrOp>) {
    let rhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
    let op = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Op:(_)));
    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
    let bin = BinOp::new(op, lhs, rhs);
    stack.push(ExprOrOp::Expr(Expr::BinOp(bin)));
}
