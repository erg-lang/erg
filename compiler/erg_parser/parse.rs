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
use erg_common::{
    caused_by, debug_power_assert, enum_unwrap, fn_name, log, set, switch_lang, switch_unreachable,
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
        let loc = self.peek().unwrap().loc();
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
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

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
            let loc = self.peek().unwrap().loc();
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
                Some(t) if t.is(Indent) => {
                    switch_unreachable!()
                }
                Some(t) if t.is(Dedent) => {
                    switch_unreachable!()
                }
                Some(_) => {
                    if let Ok(expr) = self.try_reduce_chunk(true) {
                        chunks.push(expr);
                    }
                }
                None => switch_unreachable!(),
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
            let chunk = self.try_reduce_chunk(true).map_err(|_| self.stack_dec())?;
            block.push(chunk);
            self.level -= 1;
            return Ok(block);
        }
        assert!(self.cur_is(Newline));
        self.skip();
        assert!(self.cur_is(Indent));
        self.skip();
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
                Some(t) if t.is(EOF) => {
                    break;
                }
                Some(_) => {
                    if let Ok(expr) = self.try_reduce_chunk(true) {
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
                .try_reduce_expr(false, false)
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

    fn try_reduce_acc(&mut self, in_type_args: bool) -> ParseResult<Accessor> {
        debug_call_info!(self);
        let mut acc = match self.peek() {
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
        loop {
            match self.peek() {
                Some(t) if t.is(Dot) => {
                    let vis = self.lpop();
                    let token = self.lpop();
                    match token.kind {
                        Symbol => {
                            let ident = Identifier::new(Some(vis), VarName::new(token));
                            acc = Accessor::attr(Expr::Accessor(acc), ident);
                        }
                        NatLit => {
                            let attr = Literal::from(token);
                            acc = Accessor::tuple_attr(Expr::Accessor(acc), attr);
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
                Some(t) if t.is(DblColon) => {
                    let vis = self.lpop();
                    let token = self.lpop();
                    match token.kind {
                        Symbol => {
                            let ident = Identifier::new(None, VarName::new(token));
                            acc = Accessor::attr(Expr::Accessor(acc), ident);
                        }
                        // DataPack
                        LBrace => {
                            self.restore(token);
                            self.restore(vis);
                            break;
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
                Some(t) if t.is(LSqBr) => {
                    self.skip();
                    let index = self
                        .try_reduce_expr(false, false)
                        .map_err(|_| self.stack_dec())?;
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
                Some(t) if t.is(VBar) && !in_type_args => {
                    let type_args = self
                        .try_reduce_type_app_args()
                        .map_err(|_| self.stack_dec())?;
                    acc = Accessor::TypeApp(TypeApp::new(Expr::Accessor(acc), type_args));
                }
                _ => {
                    break;
                }
            }
        }
        self.level -= 1;
        Ok(acc)
    }

    fn validate_const_expr(&mut self, expr: Expr) -> ParseResult<ConstExpr> {
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
                        let const_expr = self.validate_const_expr(elem.expr)?;
                        const_elems.push(ConstPosArg::new(const_expr));
                    }
                    let elems = ConstArgs::new(const_elems, vec![], None);
                    let const_arr = ConstArray::new(arr.l_sqbr, arr.r_sqbr, elems, None);
                    Ok(ConstExpr::Array(const_arr))
                }
                other => {
                    self.errs.push(ParseError::feature_error(
                        line!() as usize,
                        other.loc(),
                        "???",
                    ));
                    Err(())
                }
            },
            // TODO: App, Record, BinOp, UnaryOp,
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
                    .try_reduce_expr(false, false)
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
                    .try_reduce_expr(false, false)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(PosArg::new(expr))
            }
            None => switch_unreachable!(),
        }
    }

    fn opt_reduce_args(&mut self, in_type_args: bool) -> Option<ParseResult<Args>> {
        // debug_call_info!(self);
        match self.peek() {
            Some(t)
                if t.category_is(TC::Literal)
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
                    debug_power_assert!(self.cur_is(Indent));
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
                    let acc = self
                        .try_reduce_acc(in_type_args)
                        .map_err(|_| self.stack_dec())?;
                    // TODO: type specification
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
                        .try_reduce_expr(false, in_type_args)
                        .map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(PosOrKwArg::Kw(KwArg::new(kw, None, expr)))
                } else {
                    let expr = self
                        .try_reduce_expr(false, in_type_args)
                        .map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(PosOrKwArg::Pos(PosArg::new(expr)))
                }
            }
            Some(_) => {
                let expr = self
                    .try_reduce_expr(false, in_type_args)
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
                    let acc = self
                        .try_reduce_acc(in_type_args)
                        .map_err(|_| self.stack_dec())?;
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
                        .try_reduce_expr(false, in_type_args)
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
        let first = self.try_reduce_chunk(false).map_err(|_| self.stack_dec())?;
        let first = if let Some(fst) = option_enum_unwrap!(first, Expr::Def) {
            fst
        } else {
            // self.restore();
            self.level -= 1;
            let err = self.skip_and_throw_syntax_err(caused_by!());
            self.errs.push(err);
            return Err(());
        };
        let mut defs = vec![first];
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
                    let def = self.try_reduce_chunk(false).map_err(|_| self.stack_dec())?;
                    match def {
                        Expr::Def(def) => {
                            defs.push(def);
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
        let defs = RecordAttrs::from(defs);
        let class = self
            .convert_rhs_to_type_spec(class)
            .map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(Methods::new(class, vis, defs))
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
                .try_reduce_expr(true, false)
                .map_err(|_| self.stack_dec())?;
            let block = Block::new(vec![expr]);
            self.level -= 1;
            Ok(Lambda::new(sig, op, block, self.counter))
        }
    }

    /// chunk = normal expr + def
    fn try_reduce_chunk(&mut self, winding: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut stack = Vec::<ExprOrOp>::new();
        stack.push(ExprOrOp::Expr(
            self.try_reduce_bin_lhs(false)
                .map_err(|_| self.stack_dec())?,
        ));
        loop {
            match self.peek() {
                Some(arg) if arg.is(Symbol) || arg.category_is(TC::Literal) => {
                    let args = self.try_reduce_args(false).map_err(|_| self.stack_dec())?;
                    let obj = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    stack.push(ExprOrOp::Expr(Expr::Call(Call::new(obj, None, args))));
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
                Some(op)
                    if (op.is(Colon) && !self.nth_is(1, Newline))
                        || (op.is(SubtypeOf) || op.is(SupertypeOf)) =>
                {
                    let op = self.lpop();
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let t_spec = self
                        .try_reduce_expr(false, false)
                        .map_err(|_| self.stack_dec())?;
                    let t_spec = self
                        .convert_rhs_to_type_spec(t_spec)
                        .map_err(|_| self.stack_dec())?;
                    let expr = Expr::TypeAsc(TypeAscription::new(lhs, op, t_spec));
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
                        self.try_reduce_bin_lhs(false)
                            .map_err(|_| self.stack_dec())?,
                    ));
                }
                Some(t) if t.is(DblColon) => {
                    let vis = self.lpop();
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
                                .opt_reduce_args(false)
                                .transpose()
                                .map_err(|_| self.stack_dec())?
                            {
                                let ident = Identifier::new(None, VarName::new(symbol));
                                let call = Call::new(obj, Some(ident), args);
                                stack.push(ExprOrOp::Expr(Expr::Call(call)));
                            } else {
                                let ident = Identifier::new(None, VarName::new(symbol));
                                let acc = Accessor::attr(obj, ident);
                                stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
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
                            let obj = if let Some(ExprOrOp::Expr(expr)) = stack.pop() {
                                expr
                            } else {
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
                                    call = Expr::Call(Call::new(call, None, args));
                                }
                                stack.push(ExprOrOp::Expr(call));
                            } else {
                                let ident = Identifier::new(Some(vis), VarName::new(symbol));
                                let acc = Accessor::attr(obj, ident);
                                stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
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
                Some(t) if t.is(Comma) && winding => {
                    let first_elem = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let tup = self
                        .try_reduce_tuple(first_elem)
                        .map_err(|_| self.stack_dec())?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tup)));
                }
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

    /// chunk = expr + def
    /// winding: true => parse paren-less tuple
    fn try_reduce_expr(&mut self, winding: bool, in_type_args: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut stack = Vec::<ExprOrOp>::new();
        stack.push(ExprOrOp::Expr(
            self.try_reduce_bin_lhs(in_type_args)
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
                Some(op)
                    if (op.is(Colon) && !self.nth_is(1, Newline))
                        || (op.is(SubtypeOf) || op.is(SupertypeOf)) =>
                {
                    let op = self.lpop();
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let t_spec = self
                        .try_reduce_expr(false, in_type_args)
                        .map_err(|_| self.stack_dec())?;
                    let t_spec = self
                        .convert_rhs_to_type_spec(t_spec)
                        .map_err(|_| self.stack_dec())?;
                    let expr = Expr::TypeAsc(TypeAscription::new(lhs, op, t_spec));
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
                        self.try_reduce_bin_lhs(in_type_args)
                            .map_err(|_| self.stack_dec())?,
                    ));
                }
                Some(t) if t.is(Dot) => {
                    let vis = self.lpop();
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
                                .opt_reduce_args(in_type_args)
                                .transpose()
                                .map_err(|_| self.stack_dec())?
                            {
                                let ident = Identifier::new(Some(vis), VarName::new(symbol));
                                let call = Call::new(obj, Some(ident), args);
                                stack.push(ExprOrOp::Expr(Expr::Call(call)));
                            } else {
                                let ident = Identifier::new(Some(vis), VarName::new(symbol));
                                let acc = Accessor::attr(obj, ident);
                                stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
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
                    let first_elem = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let tup = self
                        .try_reduce_tuple(first_elem)
                        .map_err(|_| self.stack_dec())?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tup)));
                }
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
    fn try_reduce_bin_lhs(&mut self, in_type_args: bool) -> ParseResult<Expr> {
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
                let expr = self.try_reduce_chunk(false)?;
                let mut def = if let Some(def) = option_enum_unwrap!(expr, Expr::Def) {
                    def
                } else {
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
                            last = Expr::Call(Call::new(
                                deco.into_expr(),
                                None,
                                Args::new(vec![PosArg::new(last)], vec![], None),
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
                if self.cur_is(RParen) {
                    let rparen = self.lpop();
                    let args = Args::new(vec![], vec![], Some((lparen, rparen)));
                    let unit = Tuple::Normal(NormalTuple::new(args));
                    self.level -= 1;
                    return Ok(Expr::Tuple(unit));
                }
                let mut expr = self
                    .try_reduce_expr(true, false)
                    .map_err(|_| self.stack_dec())?;
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
        let acc = self
            .try_reduce_acc(in_type_args)
            .map_err(|_| self.stack_dec())?;
        if let Some(res) = self.opt_reduce_args(in_type_args) {
            let args = res.map_err(|_| self.stack_dec())?;
            let (obj, method_name) = match acc {
                Accessor::Attr(attr) => (*attr.obj, Some(attr.ident)),
                other => (Expr::Accessor(other), None),
            };
            let mut call = Expr::Call(Call::new(obj, method_name, args));
            // e.g.
            // f(x).y == Attr { obj: f(x) attr: .y }
            // f(x) .y == Call { obj: f(x), args: [.y] } (with warning)
            // f(x) g(x) == f(x)(g(x))
            while let Some(res) = self.opt_reduce_args(in_type_args) {
                let args = res.map_err(|_| self.stack_dec())?;
                call = Expr::Call(Call::new(call, None, args));
            }
            self.level -= 1;
            Ok(call)
        } else {
            self.level -= 1;
            Ok(Expr::Accessor(acc))
        }
    }

    #[inline]
    fn try_reduce_unary(&mut self) -> ParseResult<UnaryOp> {
        debug_call_info!(self);
        let op = self.lpop();
        let expr = self
            .try_reduce_expr(false, false)
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
        let first = self.try_reduce_chunk(false).map_err(|_| self.stack_dec())?;
        match first {
            Expr::Def(def) => {
                let record = self
                    .try_reduce_normal_record(l_brace, def)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(BraceContainer::Record(Record::Normal(record)))
            }
            // Dict
            Expr::TypeAsc(_) => todo!(),
            Expr::Accessor(acc) if self.cur_is(Semi) => {
                let record = self
                    .try_reduce_shortened_record(l_brace, acc)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(BraceContainer::Record(Record::Shortened(record)))
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

    fn try_reduce_normal_record(
        &mut self,
        l_brace: Token,
        first: Def,
    ) -> ParseResult<NormalRecord> {
        debug_call_info!(self);
        let mut attrs = vec![first];
        loop {
            match self.peek() {
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(t) if t.is(Dedent) => {
                    self.skip();
                    if self.cur_is(RBrace) {
                        let r_brace = self.lpop();
                        self.level -= 1;
                        let attrs = RecordAttrs::from(attrs);
                        return Ok(NormalRecord::new(l_brace, r_brace, attrs));
                    } else {
                        // TODO: not closed
                        // self.restore(other);
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                }
                Some(term) if term.is(RBrace) => {
                    let r_brace = self.lpop();
                    self.level -= 1;
                    let attrs = RecordAttrs::from(attrs);
                    return Ok(NormalRecord::new(l_brace, r_brace, attrs));
                }
                Some(_) => {
                    let def = self.try_reduce_chunk(false).map_err(|_| self.stack_dec())?;
                    let def = if let Some(def) = option_enum_unwrap!(def, Expr::Def) {
                        def
                    } else {
                        // self.restore(other);
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    };
                    attrs.push(def);
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

    fn try_reduce_shortened_record(
        &mut self,
        l_brace: Token,
        first: Accessor,
    ) -> ParseResult<ShortenedRecord> {
        debug_call_info!(self);
        let first = match first {
            Accessor::Ident(ident) => ident,
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                return Err(());
            }
        };
        let mut idents = vec![first];
        loop {
            match self.peek() {
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(t) if t.is(Dedent) => {
                    self.skip();
                    if self.cur_is(RBrace) {
                        let r_brace = self.lpop();
                        self.level -= 1;
                        return Ok(ShortenedRecord::new(l_brace, r_brace, idents));
                    } else {
                        // self.restore(other);
                        self.level -= 1;
                        let err = self.skip_and_throw_syntax_err(caused_by!());
                        self.errs.push(err);
                        return Err(());
                    }
                }
                Some(term) if term.is(RBrace) => {
                    let r_brace = self.lpop();
                    self.level -= 1;
                    return Ok(ShortenedRecord::new(l_brace, r_brace, idents));
                }
                Some(_) => {
                    let acc = self.try_reduce_acc(false).map_err(|_| self.stack_dec())?;
                    let acc = match acc {
                        Accessor::Ident(ident) => ident,
                        other => {
                            self.level -= 1;
                            let err =
                                ParseError::simple_syntax_error(line!() as usize, other.loc());
                            self.errs.push(err);
                            return Err(());
                        }
                    };
                    idents.push(acc);
                }
                _ => {
                    self.level -= 1;
                    let err = self.skip_and_throw_syntax_err(caused_by!());
                    self.errs.push(err);
                    return Err(());
                }
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
                    } else if self.cur_is(RParen) {
                        break;
                    }
                    match self.try_reduce_arg(false).map_err(|_| self.stack_dec())? {
                        PosOrKwArg::Pos(arg) => match arg.expr {
                            Expr::Tuple(Tuple::Normal(tup)) if tup.elems.paren.is_none() => {
                                args.extend_pos(tup.elems.into_iters().0);
                            }
                            other => {
                                args.push_pos(PosArg::new(other));
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

    fn convert_record_to_record_pat(&mut self, record: Record) -> ParseResult<VarRecordPattern> {
        debug_call_info!(self);
        match record {
            Record::Normal(rec) => {
                let mut pats = vec![];
                for mut attr in rec.attrs.into_iter() {
                    let lhs =
                        option_enum_unwrap!(attr.sig, Signature::Var).unwrap_or_else(|| todo!());
                    let lhs =
                        option_enum_unwrap!(lhs.pat, VarPattern::Ident).unwrap_or_else(|| todo!());
                    assert_eq!(attr.body.block.len(), 1);
                    let rhs = option_enum_unwrap!(attr.body.block.remove(0), Expr::Accessor)
                        .unwrap_or_else(|| todo!());
                    let rhs = self
                        .convert_accessor_to_var_sig(rhs)
                        .map_err(|_| self.stack_dec())?;
                    pats.push(VarRecordAttr::new(lhs, rhs));
                }
                let attrs = VarRecordAttrs::new(pats);
                self.level -= 1;
                Ok(VarRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
            Record::Shortened(rec) => {
                let mut pats = vec![];
                for ident in rec.idents.into_iter() {
                    let rhs = VarSignature::new(VarPattern::Ident(ident.clone()), None);
                    pats.push(VarRecordAttr::new(ident.clone(), rhs));
                }
                let attrs = VarRecordAttrs::new(pats);
                self.level -= 1;
                Ok(VarRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
        }
    }

    fn convert_data_pack_to_data_pack_pat(
        &mut self,
        pack: DataPack,
    ) -> ParseResult<VarDataPackPattern> {
        debug_call_info!(self);
        let class = self
            .convert_rhs_to_type_spec(*pack.class)
            .map_err(|_| self.stack_dec())?;
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
    ) -> ParseResult<ParamSignature> {
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
    ) -> ParseResult<ParamSignature> {
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
                let param = ParamSignature::new(pat, None, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::Lit(lit) => {
                let pat = ParamPattern::Lit(lit);
                let param = ParamSignature::new(pat, None, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::Array(array) => {
                let array_pat = self
                    .convert_array_to_param_array_pat(array)
                    .map_err(|_| self.stack_dec())?;
                let pat = ParamPattern::Array(array_pat);
                let param = ParamSignature::new(pat, None, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::Record(record) => {
                let record_pat = self
                    .convert_record_to_param_record_pat(record)
                    .map_err(|_| self.stack_dec())?;
                let pat = ParamPattern::Record(record_pat);
                let param = ParamSignature::new(pat, None, None);
                self.level -= 1;
                Ok(param)
            }
            Expr::Tuple(tuple) => {
                let tuple_pat = self
                    .convert_tuple_to_param_tuple_pat(tuple)
                    .map_err(|_| self.stack_dec())?;
                let pat = ParamPattern::Tuple(tuple_pat);
                let param = ParamSignature::new(pat, None, None);
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
                    let param = ParamSignature::new(pat, None, None);
                    self.level -= 1;
                    Ok(param)
                }
                TokenKind::RefMutOp => {
                    let var = unary.args.into_iter().next().unwrap();
                    let var = option_enum_unwrap!(*var, Expr::Accessor:(Accessor::Ident:(_)))
                        .unwrap_or_else(|| todo!());
                    let pat = ParamPattern::RefMut(var.name);
                    let param = ParamSignature::new(pat, None, None);
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

    fn convert_kw_arg_to_default_param(&mut self, arg: KwArg) -> ParseResult<ParamSignature> {
        debug_call_info!(self);
        let pat = ParamPattern::VarName(VarName::new(arg.keyword));
        let expr = self.validate_const_expr(arg.expr)?;
        let param = ParamSignature::new(pat, arg.t_spec, Some(expr));
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

    fn convert_record_to_param_record_pat(
        &mut self,
        record: Record,
    ) -> ParseResult<ParamRecordPattern> {
        debug_call_info!(self);
        match record {
            Record::Normal(rec) => {
                let mut pats = vec![];
                for mut attr in rec.attrs.into_iter() {
                    let lhs =
                        option_enum_unwrap!(attr.sig, Signature::Var).unwrap_or_else(|| todo!());
                    let lhs =
                        option_enum_unwrap!(lhs.pat, VarPattern::Ident).unwrap_or_else(|| todo!());
                    assert_eq!(attr.body.block.len(), 1);
                    let rhs = option_enum_unwrap!(attr.body.block.remove(0), Expr::Accessor)
                        .unwrap_or_else(|| todo!());
                    let rhs = self
                        .convert_accessor_to_param_sig(rhs)
                        .map_err(|_| self.stack_dec())?;
                    pats.push(ParamRecordAttr::new(lhs, rhs));
                }
                let attrs = ParamRecordAttrs::new(pats);
                self.level -= 1;
                Ok(ParamRecordPattern::new(rec.l_brace, attrs, rec.r_brace))
            }
            Record::Shortened(rec) => {
                let mut pats = vec![];
                for ident in rec.idents.into_iter() {
                    let rhs =
                        ParamSignature::new(ParamPattern::VarName(ident.name.clone()), None, None);
                    pats.push(ParamRecordAttr::new(ident.clone(), rhs));
                }
                let attrs = ParamRecordAttrs::new(pats);
                self.level -= 1;
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
    ) -> ParseResult<ParamSignature> {
        debug_call_info!(self);
        let param = self
            .convert_rhs_to_param(*tasc.expr, allow_self)
            .map_err(|_| self.stack_dec())?;
        let t_spec = TypeSpecWithOp::new(tasc.op, tasc.t_spec);
        let param = ParamSignature::new(param.pat, Some(t_spec), None);
        self.level -= 1;
        Ok(param)
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
                    .convert_array_to_param_array_pat(array)
                    .map_err(|_| self.stack_dec())?;
                let param = ParamSignature::new(ParamPattern::Array(arr), None, None);
                let params = Params::new(vec![param], None, vec![], None);
                self.level -= 1;
                Ok(LambdaSignature::new(params, None, TypeBoundSpecs::empty()))
            }
            Expr::Record(record) => {
                let rec = self
                    .convert_record_to_param_record_pat(record)
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
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_accessor_to_param_sig(&mut self, accessor: Accessor) -> ParseResult<ParamSignature> {
        debug_call_info!(self);
        match accessor {
            Accessor::Ident(ident) => {
                let pat = if &ident.name.inspect()[..] == "_" {
                    ParamPattern::Discard(ident.name.into_token())
                } else {
                    ParamPattern::VarName(ident.name)
                };
                self.level -= 1;
                Ok(ParamSignature::new(pat, None, None))
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
            .convert_rhs_to_param(*tasc.expr, true)
            .map_err(|_| self.stack_dec())?;
        self.level -= 1;
        Ok(LambdaSignature::new(
            Params::new(vec![sig], None, vec![], None),
            None,
            TypeBoundSpecs::empty(),
        ))
    }

    fn convert_rhs_to_type_spec(&mut self, rhs: Expr) -> ParseResult<TypeSpec> {
        debug_call_info!(self);
        match rhs {
            Expr::Accessor(acc) => {
                let t_spec = self
                    .convert_accessor_to_type_spec(acc)
                    .map_err(|_| self.stack_dec())?;
                self.level -= 1;
                Ok(t_spec)
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
            Expr::BinOp(bin) => {
                if bin.op.kind.is_range_op() {
                    let op = bin.op;
                    let mut args = bin.args.into_iter();
                    let lhs = self
                        .validate_const_expr(*args.next().unwrap())
                        .map_err(|_| self.stack_dec())?;
                    let rhs = self
                        .validate_const_expr(*args.next().unwrap())
                        .map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(TypeSpec::Interval { op, lhs, rhs })
                } else if bin.op.kind == TokenKind::AndOp {
                    let mut args = bin.args.into_iter();
                    let lhs = self
                        .convert_rhs_to_type_spec(*args.next().unwrap())
                        .map_err(|_| self.stack_dec())?;
                    let rhs = self
                        .convert_rhs_to_type_spec(*args.next().unwrap())
                        .map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(TypeSpec::and(lhs, rhs))
                } else if bin.op.kind == TokenKind::OrOp {
                    let mut args = bin.args.into_iter();
                    let lhs = self
                        .convert_rhs_to_type_spec(*args.next().unwrap())
                        .map_err(|_| self.stack_dec())?;
                    let rhs = self
                        .convert_rhs_to_type_spec(*args.next().unwrap())
                        .map_err(|_| self.stack_dec())?;
                    self.level -= 1;
                    Ok(TypeSpec::or(lhs, rhs))
                } else {
                    self.level -= 1;
                    let err = ParseError::simple_syntax_error(line!() as usize, bin.loc());
                    self.errs.push(err);
                    Err(())
                }
            }
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                Err(())
            }
        }
    }

    fn convert_accessor_to_type_spec(&mut self, accessor: Accessor) -> ParseResult<TypeSpec> {
        debug_call_info!(self);
        let t_spec = match accessor {
            Accessor::Ident(ident) => {
                let predecl =
                    PreDeclTypeSpec::Simple(SimpleTypeSpec::new(ident.name, ConstArgs::empty()));
                TypeSpec::PreDeclTy(predecl)
            }
            Accessor::TypeApp(tapp) => {
                let spec = self
                    .convert_rhs_to_type_spec(*tapp.obj)
                    .map_err(|_| self.stack_dec())?;
                TypeSpec::type_app(spec, tapp.type_args)
            }
            other => {
                self.level -= 1;
                let err = ParseError::simple_syntax_error(line!() as usize, other.loc());
                self.errs.push(err);
                return Err(());
            }
        };
        self.level -= 1;
        Ok(t_spec)
    }

    fn convert_call_to_predecl_type_spec(&mut self, _call: Call) -> ParseResult<PreDeclTypeSpec> {
        debug_call_info!(self);
        todo!()
    }

    fn convert_lambda_to_subr_type_spec(
        &mut self,
        mut lambda: Lambda,
    ) -> ParseResult<SubrTypeSpec> {
        debug_call_info!(self);
        let bounds = lambda.sig.bounds;
        let lparen = lambda.sig.params.parens.map(|(l, _)| l);
        let mut non_defaults = vec![];
        for param in lambda.sig.params.non_defaults.into_iter() {
            let param = match (param.pat, param.t_spec) {
                (ParamPattern::VarName(name), Some(t_spec_with_op)) => {
                    ParamTySpec::new(Some(name.into_token()), t_spec_with_op.t_spec)
                }
                (ParamPattern::VarName(name), None) => ParamTySpec::anonymous(TypeSpec::PreDeclTy(
                    PreDeclTypeSpec::Simple(SimpleTypeSpec::new(name, ConstArgs::empty())),
                )),
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
                            SimpleTypeSpec::new(name, ConstArgs::empty()),
                        )))
                    }
                    _ => todo!(),
                });
        let mut defaults = vec![];
        for param in lambda.sig.params.defaults.into_iter() {
            let param = match (param.pat, param.t_spec) {
                (ParamPattern::VarName(name), Some(t_spec_with_op)) => {
                    ParamTySpec::new(Some(name.into_token()), t_spec_with_op.t_spec)
                }
                (ParamPattern::VarName(name), None) => ParamTySpec::anonymous(TypeSpec::PreDeclTy(
                    PreDeclTypeSpec::Simple(SimpleTypeSpec::new(name, ConstArgs::empty())),
                )),
                _ => todo!(),
            };
            defaults.push(param);
        }
        let return_t = self.convert_rhs_to_type_spec(lambda.body.remove(0))?;
        self.level -= 1;
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

    fn convert_array_to_array_type_spec(&mut self, array: Array) -> ParseResult<ArrayTypeSpec> {
        debug_call_info!(self);
        match array {
            Array::Normal(arr) => {
                // TODO: add hint
                self.errs
                    .push(ParseError::simple_syntax_error(line!() as usize, arr.loc()));
                Err(())
            }
            Array::WithLength(arr) => {
                let t_spec = self.convert_rhs_to_type_spec(arr.elem.expr)?;
                let len = self.validate_const_expr(*arr.len)?;
                self.level -= 1;
                Ok(ArrayTypeSpec::new(t_spec, len))
            }
            Array::Comprehension(arr) => {
                // TODO: add hint
                self.errs
                    .push(ParseError::simple_syntax_error(line!() as usize, arr.loc()));
                Err(())
            }
        }
    }
}
