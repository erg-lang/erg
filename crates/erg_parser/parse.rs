//! implements `Parser`.
//!
//! パーサーを実装する
//!
use std::fmt;
use std::mem;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict as HashMap;
use erg_common::error::Location;
use erg_common::io::{Input, InputKind};
use erg_common::set::Set as HashSet;
use erg_common::str::Str;
use erg_common::traits::{DequeStream, ExitStatus, Locational, New, Runnable, Stream};
use erg_common::{
    caused_by, debug_power_assert, enum_unwrap, fn_name, impl_display_for_enum,
    impl_locational_for_enum, log, set, switch_lang, switch_unreachable,
};

use crate::ast::*;
use crate::desugar::Desugarer;
use crate::error::{
    CompleteArtifact, IncompleteArtifact, ParseError, ParseErrors, ParseResult, ParserRunnerError,
    ParserRunnerErrors,
};
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
            c DEBUG_MAIN,
            "\n{} ({}) entered {}, cur: {}",
            "･".repeat(($self.level as f32 / 4.0).floor() as usize),
            $self.level,
            fn_name!(),
            $self.peek().unwrap_or(&$crate::token::Token::DUMMY)
        );
    };
}

#[macro_export]
macro_rules! debug_exit_info {
    ($self: ident) => {
        $self.level -= 1;
        log!(
            c DEBUG_MAIN,
            "\n{} ({}) exit {}, cur: {}",
            "･".repeat(($self.level as f32 / 4.0).floor() as usize),
            $self.level,
            fn_name!(),
            $self.peek().unwrap_or(&$crate::token::Token::DUMMY)
        );
    };
}

macro_rules! expect_pop {
    ($self: ident, category $cate: expr) => {
        if $self.cur_category_is($cate) {
            $self.lpop()
        } else {
            let loc = $self.peek().map(|t| t.loc()).unwrap_or(Location::Unknown);
            let got = $self.peek().map(|t| t.kind).unwrap_or(EOF);
            let err = ParseError::unexpected_token(line!() as usize, loc, $cate, got);
            $self.errs.push(err);
            debug_exit_info!($self);
            return Err(());
        }
    };
    ($self: ident, fail_next $kind: expr) => {
        if $self.cur_is($kind) {
            $self.lpop()
        } else {
            let loc = $self.peek().map(|t| t.loc()).unwrap_or(Location::Unknown);
            let got = $self.peek().map(|t| t.kind).unwrap_or(EOF);
            let err = ParseError::unexpected_token(line!() as usize, loc, $kind, got);
            $self.next_line();
            $self.errs.push(err);
            debug_exit_info!($self);
            return Err(());
        }
    };
    ($self: ident, $kind: expr) => {
        if $self.cur_is($kind) {
            $self.lpop()
        } else {
            let loc = $self.peek().map(|t| t.loc()).unwrap_or(Location::Unknown);
            let got = $self.peek().map(|t| t.kind).unwrap_or(EOF);
            let err = ParseError::unexpected_token(line!() as usize, loc, $kind, got);
            $self.errs.push(err);
            debug_exit_info!($self);
            return Err(());
        }
    };
}

pub trait Parsable: 'static {
    fn parse(code: String) -> Result<CompleteArtifact, IncompleteArtifact<Module, ParseErrors>>;
}

#[cfg_attr(feature = "pylib", pyo3::pyclass)]
pub struct SimpleParser {}

impl Parsable for SimpleParser {
    fn parse(code: String) -> Result<CompleteArtifact, IncompleteArtifact> {
        let ts = Lexer::from_str(code).lex()?;
        let mut parser = Parser::new(ts);
        let mut desugarer = Desugarer::new();
        let artifact = parser
            .parse()
            .map_err(|iart| iart.map_mod(|module| desugarer.desugar(module)))?;
        Ok(artifact.map(|module| desugarer.desugar(module)))
    }
}

impl SimpleParser {
    pub fn parse(code: String) -> Result<CompleteArtifact, IncompleteArtifact> {
        <Self as Parsable>::parse(code)
    }
}

enum ExprOrOp {
    Expr(Expr),
    Op(Token),
}

enum ArgKind {
    Pos(PosArg),
    Var(PosArg),
    Kw(KwArg),
    KwVar(PosArg),
}

pub enum ArrayInner {
    Normal(Args),
    WithLength(PosArg, Expr),
    Comprehension {
        layout: Option<Expr>,
        generators: Vec<(Identifier, Expr)>,
        guard: Option<Expr>,
    },
}

impl ArrayInner {
    pub const fn comp(
        layout: Option<Expr>,
        generators: Vec<(Identifier, Expr)>,
        guard: Option<Expr>,
    ) -> Self {
        Self::Comprehension {
            layout,
            generators,
            guard,
        }
    }
}

pub enum BraceContainer {
    Set(Set),
    Dict(Dict),
    Record(Record),
}

impl_locational_for_enum!(BraceContainer; Set, Dict, Record);
impl_display_for_enum!(BraceContainer; Set, Dict, Record);

impl BraceContainer {
    pub const fn kind(&self) -> &str {
        match self {
            BraceContainer::Set(_) => "Set",
            BraceContainer::Dict(_) => "Dict",
            BraceContainer::Record(_) => "Record",
        }
    }
}

pub enum ArgsStyle {
    SingleCommaWithParen,
    SingleCommaNoParen,
    MultiComma, // with parentheses
    Colon,      // with no parentheses
}

impl ArgsStyle {
    pub const fn needs_parens(&self) -> bool {
        match self {
            Self::SingleCommaWithParen | Self::MultiComma => true,
            Self::SingleCommaNoParen | Self::Colon => false,
        }
    }

    pub const fn is_colon(&self) -> bool {
        matches!(self, Self::Colon)
    }

    pub const fn is_multi_comma(&self) -> bool {
        matches!(self, Self::MultiComma)
    }
}

#[derive(Debug, Clone)]
pub enum MacroArgSpec {
    Expr,
    Name,
    Block,
    WithPrefix(Str, Box<MacroArgSpec>),
}

impl fmt::Display for MacroArgSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expr => write!(f, "Expr"),
            Self::Name => write!(f, "Name"),
            Self::Block => write!(f, "Block"),
            Self::WithPrefix(kw, arg) => write!(f, "{}: {}", kw, arg),
        }
    }
}

impl MacroArgSpec {
    pub fn with_prefix(name: impl Into<Str>, arg: Self) -> Self {
        Self::WithPrefix(name.into(), Box::new(arg))
    }

    pub fn get_prefix(&self) -> Option<Str> {
        if let Self::WithPrefix(kw, _) = self {
            Some(kw.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MacroInfo {
    non_defaults: Vec<MacroArgSpec>,
    #[allow(unused)]
    var_params: Option<MacroArgSpec>,
    #[allow(unused)]
    defaults: Vec<MacroArgSpec>,
}

impl MacroInfo {
    pub fn keywords(&self) -> impl Iterator<Item = MacroKeyword> + '_ {
        self.non_defaults
            .iter()
            .filter_map(|arg| arg.get_prefix().map(MacroKeyword::NonDefault))
            .chain(
                self.var_params
                    .iter()
                    .filter_map(|arg| arg.get_prefix().map(MacroKeyword::Var)),
            )
            .chain(
                self.defaults
                    .iter()
                    .filter_map(|arg| arg.get_prefix().map(MacroKeyword::Default)),
            )
    }
}

#[derive(Debug, Default)]
pub struct MacroContext(HashMap<Str, MacroInfo>);

impl MacroContext {
    pub fn empty() -> Self {
        Self(HashMap::default())
    }

    pub fn new() -> Self {
        let mut ctx = Self::empty();
        let for_ = MacroInfo {
            non_defaults: vec![
                MacroArgSpec::Name,
                MacroArgSpec::with_prefix("in", MacroArgSpec::Expr),
                MacroArgSpec::Block,
            ],
            var_params: None,
            defaults: vec![],
        };
        ctx.insert("for!".into(), for_);
        let import = MacroInfo {
            non_defaults: vec![MacroArgSpec::Name],
            var_params: None,
            defaults: vec![],
        };
        ctx.insert("import".into(), import);
        let if_ = MacroInfo {
            non_defaults: vec![MacroArgSpec::Expr, MacroArgSpec::Block],
            var_params: Some(MacroArgSpec::with_prefix("elif!", MacroArgSpec::Block)),
            defaults: vec![MacroArgSpec::with_prefix("else!", MacroArgSpec::Block)],
        };
        ctx.insert("if!".into(), if_);
        ctx
    }

    fn insert(&mut self, name: Str, info: MacroInfo) {
        self.0.insert(name, info);
    }

    fn get(&self, name: &str) -> Option<&MacroInfo> {
        self.0.get(name)
    }
}

#[derive(Debug, Clone)]
pub enum MacroKeyword {
    NonDefault(Str),
    Var(Str),
    Default(Str),
}

impl MacroKeyword {
    pub const fn inspect(&self) -> &Str {
        match self {
            Self::NonDefault(name) | Self::Var(name) | Self::Default(name) => name,
        }
    }

    pub const fn is_non_default(&self) -> bool {
        matches!(self, Self::NonDefault(_))
    }
}

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
    keywords: Vec<MacroKeyword>,
    macros: MacroContext,
}

impl Parsable for Parser {
    fn parse(code: String) -> Result<CompleteArtifact, IncompleteArtifact<Module, ParseErrors>> {
        let ts = Lexer::from_str(code).lex()?;
        Parser::new(ts).parse()
    }
}

impl Parser {
    pub fn new(ts: TokenStream) -> Self {
        Self {
            counter: DefId(0),
            level: 0,
            tokens: ts,
            warns: ParseErrors::empty(),
            errs: ParseErrors::empty(),
            keywords: vec![],
            macros: MacroContext::new(),
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

    fn until_dedent(&mut self) {
        let mut nest_cnt = 1;
        while let Some(t) = self.peek() {
            match t.kind {
                Indent => {
                    self.skip();
                    nest_cnt += 1;
                }
                Dedent => {
                    self.skip();
                    nest_cnt -= 1;
                    if nest_cnt <= 0 {
                        return;
                    }
                }
                EOF => return,
                _ => {
                    self.skip();
                }
            }
        }
    }

    fn unexpected_none(&self, errno: u32, caused_by: &str) -> ParseError {
        log!(err "error caused by: {caused_by}");
        ParseError::invalid_none_match(0, Location::Unknown, file!(), errno)
    }

    fn skip_and_throw_syntax_err(&mut self, errno: u32, caused_by: &str) -> ParseError {
        let loc = self.peek().map(|t| t.loc()).unwrap_or_default();
        log!(err "error caused by: {caused_by}");
        self.next_expr();
        ParseError::simple_syntax_error(errno as usize, loc)
    }

    fn skip_and_throw_invalid_unclosed_err(
        &mut self,
        caused_by: &str,
        line: u32,
        closer: &str,
        ty: &str,
    ) -> ParseError {
        log!(err "error caused by: {caused_by}");
        let loc = self.peek().map(|t| t.loc()).unwrap_or_default();
        self.next_expr();
        ParseError::unclosed_error(line as usize, loc, closer, ty)
    }

    fn skip_and_throw_invalid_seq_err(
        &mut self,
        caused_by: &str,
        errno: usize,
        expected: &[impl std::fmt::Display],
        found: TokenKind,
    ) -> ParseError {
        log!(err "error caused by: {caused_by}");
        let loc = self.peek().map(|t| t.loc()).unwrap_or_default();
        self.next_expr();
        ParseError::invalid_seq_elems_error(errno, loc, expected, found)
    }

    fn skip_and_throw_invalid_chunk_err(
        &mut self,
        caused_by: &str,
        line: u32,
        loc: Location,
    ) -> ParseError {
        log!(err "error caused by: {caused_by}");
        self.next_line();
        ParseError::invalid_chunk_error(line as usize, loc)
    }

    fn get_stream_op_syntax_error(
        &mut self,
        errno: usize,
        loc: Location,
        caused_by: &str,
    ) -> ParseError {
        log!(err "error caused by: {caused_by}");
        self.next_expr();
        ParseError::syntax_error(
            errno,
            loc,
            switch_lang!(
                "japanese" => "パイプ演算子の後には関数・メソッド・サブルーチンのみ呼び出しができます",
                "simplified_chinese" => "流操作符后只能调用函数、方法或子程序",
                "traditional_chinese" => "流操作符後只能調用函數、方法或子程序",
                "english" => "Only a call of function, method or subroutine is available after stream operator",
            ),
            None,
        )
    }

    #[inline]
    fn restore(&mut self, token: Token) {
        self.tokens.push_front(token);
    }

    pub(crate) fn stack_dec(&mut self, fn_name: &str) {
        self.level -= 1;
        log!(
            c DEBUG_MAIN,
            "\n{} ({}) exit {}, cur: {}",
            "･".repeat((self.level as f32 / 4.0).floor() as usize),
            self.level,
            fn_name,
            self.peek().unwrap()
        );
    }
}

#[derive(Debug, Default)]
pub struct ParserRunner {
    cfg: ErgConfig,
}

impl New for ParserRunner {
    #[inline]
    fn new(cfg: ErgConfig) -> Self {
        Self { cfg }
    }
}

impl Runnable for ParserRunner {
    type Err = ParserRunnerError;
    type Errs = ParserRunnerErrors;
    const NAME: &'static str = "Erg parser";

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

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let src = self.cfg_mut().input.read();
        let artifact = self.parse(src).map_err(|iart| iart.errors)?;
        println!("{}", artifact.ast);
        Ok(ExitStatus::OK)
    }

    fn eval(&mut self, src: String) -> Result<String, ParserRunnerErrors> {
        let artifact = self.parse(src).map_err(|iart| iart.errors)?;
        Ok(format!("{}", artifact.ast))
    }
}

impl ParserRunner {
    #[inline]
    pub fn new(cfg: ErgConfig) -> Self {
        New::new(cfg)
    }

    pub fn parse_token_stream(
        &mut self,
        ts: TokenStream,
    ) -> Result<CompleteArtifact, IncompleteArtifact<Module, ParserRunnerErrors>> {
        Parser::new(ts)
            .parse()
            .map_err(|iart| iart.map_errs(|errs| ParserRunnerErrors::convert(self.input(), errs)))
    }

    pub fn parse(
        &mut self,
        src: String,
    ) -> Result<CompleteArtifact, IncompleteArtifact<Module, ParserRunnerErrors>> {
        let ts = Lexer::new(Input::new(InputKind::Str(src), self.cfg.input.id()))
            .lex()
            .map_err(|errs| ParserRunnerErrors::convert(self.input(), errs))?;
        Parser::new(ts)
            .parse()
            .map_err(|iart| iart.map_errs(|errs| ParserRunnerErrors::convert(self.input(), errs)))
    }
}

impl Parser {
    pub fn parse(&mut self) -> Result<CompleteArtifact, IncompleteArtifact> {
        if self.tokens.is_empty() {
            return Ok(CompleteArtifact::new(Module::empty(), ParseErrors::empty()));
        }
        log!(info "the parsing process has started.");
        log!(info "token stream: {}", self.tokens);
        let module = match self.try_reduce_module() {
            Ok(module) => module,
            Err(_) => {
                return Err(IncompleteArtifact::new(
                    None,
                    mem::take(&mut self.warns),
                    mem::take(&mut self.errs),
                ));
            }
        };
        log!(info "the parsing process has completed (errs: {}).", self.errs.len());
        log!(info "AST:\n{module}");
        if self.errs.is_empty() {
            Ok(CompleteArtifact::new(module, mem::take(&mut self.warns)))
        } else {
            Err(IncompleteArtifact::new(
                Some(module),
                mem::take(&mut self.warns),
                mem::take(&mut self.errs),
            ))
        }
    }

    /// Reduce to the largest unit of syntax, the module (this is called only once)
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
                        if !self.cur_is(EOF) && !self.cur_category_is(TC::Separator) {
                            let err = self.skip_and_throw_invalid_chunk_err(
                                caused_by!(),
                                line!(),
                                expr.loc(),
                            );
                            self.errs.push(err);
                        }
                        chunks.push(expr);
                    }
                }
                None => {
                    if !self.errs.is_empty() {
                        debug_exit_info!(self);
                        let err = if let Some(last) = chunks.last() {
                            self.skip_and_throw_invalid_chunk_err(caused_by!(), line!(), last.loc())
                        } else {
                            self.unexpected_none(line!(), caused_by!())
                        };
                        self.errs.push(err);
                        break;
                    } else {
                        switch_unreachable!()
                    }
                }
            }
        }
        debug_exit_info!(self);
        Ok(chunks)
    }

    // expect the block`= ; . -> =>`
    fn try_reduce_block(&mut self) -> ParseResult<Block> {
        debug_call_info!(self);
        let mut block = Block::with_capacity(2);
        // single line block
        if !self.cur_is(Newline) {
            let expr = self
                .try_reduce_expr(true, false, false, false)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            block.push(expr);
            if !self.cur_is(Dedent)
                && !self.cur_category_is(TC::Separator)
                && !self.cur_category_is(TC::REnclosure)
            {
                let err = self.skip_and_throw_invalid_chunk_err(
                    caused_by!(),
                    line!(),
                    block.last().unwrap().loc(),
                );
                debug_exit_info!(self);
                self.errs.push(err);
            }
            if block.last().unwrap().is_definition() {
                let err = ParseError::invalid_definition_of_last_block(
                    line!() as usize,
                    block.last().unwrap().loc(),
                );
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            } else {
                debug_exit_info!(self);
                return Ok(block);
            }
        }
        expect_pop!(self, Newline);
        while self.cur_is(Newline) {
            self.skip();
        }
        expect_pop!(self, Indent);
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
                        if !self.cur_is(Dedent) && !self.cur_category_is(TC::Separator) {
                            let err = self.skip_and_throw_invalid_chunk_err(
                                caused_by!(),
                                line!(),
                                expr.loc(),
                            );
                            debug_exit_info!(self);
                            self.errs.push(err);
                        }
                        block.push(expr);
                    }
                }
                None => {
                    let err =
                        ParseError::failed_to_analyze_block(line!() as usize, Location::Unknown);
                    self.errs.push(err);
                    break;
                }
            }
        }
        if block.is_empty() {
            let loc = if let Some(u) = self.peek() {
                u.loc()
            } else {
                Location::Unknown
            };
            let err = ParseError::failed_to_analyze_block(line!() as usize, loc);
            self.errs.push(err);
            debug_exit_info!(self);
            Err(())
        } else if block.last().unwrap().is_definition() {
            let err =
                ParseError::invalid_chunk_error(line!() as usize, block.last().unwrap().loc());
            self.errs.push(err);
            debug_exit_info!(self);
            Err(())
        } else {
            debug_exit_info!(self);
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
                .map_err(|_| {
                    if let Some(err) = self.errs.last_mut() {
                        err.set_hint(switch_lang!(
                            "japanese" => "予期: デコレータ",
                            "simplified_chinese" => "期望: 装饰器",
                            "traditional_chinese" => "期望: 裝飾器",
                            "english" => "expect: decorator",
                        ))
                    }
                    self.stack_dec(fn_name!())
                })?;
            debug_exit_info!(self);
            Ok(Some(Decorator::new(expr)))
        } else {
            debug_exit_info!(self);
            Ok(None)
        }
    }

    #[inline]
    fn opt_reduce_decorators(&mut self) -> ParseResult<HashSet<Decorator>> {
        debug_call_info!(self);
        let mut decs = set![];
        while let Some(deco) = self
            .opt_reduce_decorator()
            .map_err(|_| self.stack_dec(fn_name!()))?
        {
            if self.cur_is(EOF) {
                let err =
                    ParseError::expect_next_line_error(line!() as usize, deco.0.loc(), "AtMark");
                self.errs.push(err);
                return Err(());
            }
            decs.insert(deco);
            expect_pop!(self, fail_next Newline);
        }
        debug_exit_info!(self);
        Ok(decs)
    }

    fn try_reduce_type_app_args(&mut self) -> ParseResult<TypeAppArgs> {
        debug_call_info!(self);
        let l_vbar = expect_pop!(self, VBar);
        let args = match self.peek_kind() {
            Some(SubtypeOf) => {
                let op = self.lpop();
                let t_spec_as_expr =
                    self.try_reduce_expr(false, true, false, false)
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: 型指定",
                                    "simplified_chinese" => "期望: 类型规范",
                                    "traditional_chinese" => "期望: 類型規範",
                                    "english" => "expect: type specification",
                                ))
                            }
                            self.stack_dec(fn_name!())
                        })?;
                match Parser::expr_to_type_spec(t_spec_as_expr.clone()) {
                    Ok(t_spec) => {
                        let t_spec = TypeSpecWithOp::new(op, t_spec, t_spec_as_expr);
                        TypeAppArgsKind::SubtypeOf(Box::new(t_spec))
                    }
                    Err(_) => {
                        let err = ParseError::simple_syntax_error(0, t_spec_as_expr.loc());
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    }
                }
            }
            _ => {
                let args = self
                    .try_reduce_args(true)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                TypeAppArgsKind::Args(args)
            }
        };
        let r_vbar = expect_pop!(self, VBar);
        debug_exit_info!(self);
        Ok(TypeAppArgs::new(l_vbar, args, r_vbar))
    }

    fn try_reduce_restriction(&mut self) -> ParseResult<VisRestriction> {
        debug_call_info!(self);
        expect_pop!(self, LSqBr);
        let rest = match self.peek_kind() {
            Some(SubtypeOf) => {
                self.lpop();
                let t_spec_as_expr = self
                    .try_reduce_expr(false, true, false, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                match Parser::expr_to_type_spec(t_spec_as_expr) {
                    Ok(t_spec) => VisRestriction::SubtypeOf(Box::new(t_spec)),
                    Err(err) => {
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    }
                }
            }
            _ => {
                // FIXME: reduce namespaces
                let acc = self
                    .try_reduce_acc_lhs()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                VisRestriction::Namespaces(Namespaces::new(vec![acc]))
            }
        };
        expect_pop!(self, RSqBr);
        debug_exit_info!(self);
        Ok(rest)
    }

    fn try_reduce_ident(&mut self) -> ParseResult<Identifier> {
        debug_call_info!(self);
        let ident = match self.peek_kind() {
            Some(Symbol) => {
                let symbol = self.lpop();
                Identifier::private_from_token(symbol)
            }
            Some(Dot) => {
                let dot = self.lpop();
                let symbol = expect_pop!(self, Symbol);
                Identifier::public_from_token(dot, symbol)
            }
            _ => {
                let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            }
        };
        debug_exit_info!(self);
        Ok(ident)
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
                    let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
            }
            Some(DblColon) => {
                let dbl_colon = self.lpop();
                if let Some(LSqBr) = self.peek_kind() {
                    let rest = self
                        .try_reduce_restriction()
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    let symbol = expect_pop!(self, Symbol);
                    Accessor::restricted(rest, symbol)
                } else {
                    let symbol = expect_pop!(self, Symbol);
                    Accessor::explicit_local(dbl_colon, symbol)
                }
            }
            _ => {
                let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            }
        };
        debug_exit_info!(self);
        Ok(acc)
    }

    fn try_reduce_array_elems(&mut self) -> ParseResult<ArrayInner> {
        debug_call_info!(self);
        if self.cur_is(EOF) {
            let tk = self.tokens.last().unwrap();
            let err = ParseError::expect_next_line_error(line!() as usize, tk.loc(), "Collections");
            self.errs.push(err);
            return Err(());
        }
        if self.cur_category_is(TC::REnclosure) {
            let args = Args::empty();
            debug_exit_info!(self);
            return Ok(ArrayInner::Normal(args));
        }
        let first = self
            .try_reduce_elem()
            .map_err(|_| self.stack_dec(fn_name!()))?;
        let mut elems = Args::single(first);
        match self.peek_kind() {
            Some(Semi) => {
                self.lpop();
                let len = self
                    .try_reduce_expr(false, false, false, false)
                    .map_err(|_| {
                        if let Some(err) = self.errs.last_mut() {
                            err.set_hint(switch_lang!(
                                "japanese" => "予期: Nat型",
                                "simplified_chinese" => "期望: Nat类型",
                                "traditional_chinese" => "期望: Nat類型",
                                "english" => "expect: Nat type",
                            ))
                        }
                        self.stack_dec(fn_name!())
                    })?;
                debug_exit_info!(self);
                return Ok(ArrayInner::WithLength(elems.remove_pos(0), len));
            }
            Some(PreStar) => {
                self.lpop();
                let rest = self
                    .try_reduce_expr(false, false, false, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                elems.set_var_args(PosArg::new(rest));
                debug_exit_info!(self);
                return Ok(ArrayInner::Normal(elems));
            }
            Some(Inclusion) => {
                self.lpop();
                let Expr::Accessor(Accessor::Ident(sym)) = elems.remove_pos(0).expr else {
                    let err = self.skip_and_throw_invalid_seq_err(
                        caused_by!(),
                        line!() as usize,
                        &["identifier"],
                        Inclusion,
                    );
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                };
                let mut generators = vec![];
                let expr = self
                    .try_reduce_expr(false, false, false, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                generators.push((sym, expr));
                let _ = expect_pop!(self, VBar);
                let guard = self
                    .try_reduce_expr(false, false, false, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                return Ok(ArrayInner::comp(None, generators, Some(guard)));
            }
            Some(VBar) => {
                self.lpop();
                let elem = elems.remove_pos(0).expr;
                let mut generators = vec![];
                loop {
                    let sym = self
                        .try_reduce_ident()
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    let _ = expect_pop!(self, Inclusion);
                    let expr = self
                        .try_reduce_expr(false, false, false, false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    generators.push((sym, expr));
                    if !self.cur_is(Semi) {
                        break;
                    } else {
                        self.lpop();
                    }
                }
                let guard = if self.cur_is(VBar) {
                    self.lpop();
                    let expr = self
                        .try_reduce_expr(false, false, false, false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    Some(expr)
                } else {
                    None
                };
                debug_exit_info!(self);
                return Ok(ArrayInner::comp(Some(elem), generators, guard));
            }
            Some(RParen | RSqBr | RBrace | Dedent | Comma) => {}
            Some(_) => {
                let elem = self
                    .try_reduce_elem()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                elems.push_pos(elem);
            }
            None => {
                self.errs.push(self.unexpected_none(line!(), caused_by!()));
                debug_exit_info!(self);
                return Err(());
            }
        }
        loop {
            match self.peek_kind() {
                Some(Comma) => {
                    self.skip();
                    match self.peek_kind() {
                        Some(Comma) => {
                            let err = self.skip_and_throw_invalid_seq_err(
                                caused_by!(),
                                line!() as usize,
                                &["]", "element"],
                                Comma,
                            );
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                        Some(RParen | RSqBr | RBrace | Dedent) => {
                            break;
                        }
                        Some(PreStar) => {
                            self.lpop();
                            let rest = self
                                .try_reduce_expr(false, false, false, false)
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            elems.set_var_args(PosArg::new(rest));
                            break;
                        }
                        _ => {}
                    }
                    elems.push_pos(
                        self.try_reduce_elem()
                            .map_err(|_| self.stack_dec(fn_name!()))?,
                    );
                }
                Some(RParen | RSqBr | RBrace | Dedent) => {
                    break;
                }
                Some(_other) => {
                    let err = self.skip_and_throw_invalid_unclosed_err(
                        caused_by!(),
                        line!(),
                        "]",
                        "array",
                    );
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
                None => {
                    self.errs.push(self.unexpected_none(line!(), caused_by!()));
                    debug_exit_info!(self);
                    return Err(());
                }
            }
        }
        debug_exit_info!(self);
        Ok(ArrayInner::Normal(elems))
    }

    fn try_reduce_elem(&mut self) -> ParseResult<PosArg> {
        debug_call_info!(self);
        match self.peek() {
            Some(_) => {
                let expr = self
                    .try_reduce_expr(false, false, false, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(PosArg::new(expr))
            }
            None => {
                self.errs.push(self.unexpected_none(line!(), caused_by!()));
                debug_exit_info!(self);
                Err(())
            }
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
                    && !self.nth_is(1, LBrace)
                    && !self.nth_is(1, LSqBr) =>
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
        let mut style = if lp.is_some() {
            ArgsStyle::SingleCommaWithParen
        } else {
            ArgsStyle::SingleCommaNoParen
        };
        match self.peek_kind() {
            Some(RParen) => {
                rp = Some(self.lpop());
                debug_exit_info!(self);
                return Ok(Args::pos_only(vec![], Some((lp.unwrap(), rp.unwrap()))));
            }
            Some(RBrace | RSqBr | Dedent) => {
                debug_exit_info!(self);
                return Ok(Args::empty());
            }
            Some(Newline) if style.needs_parens() => {
                self.skip();
                if self.cur_is(Indent) {
                    self.skip();
                }
                style = ArgsStyle::MultiComma;
            }
            _ => {}
        }
        let mut args = match self
            .try_reduce_arg(in_type_args)
            .map_err(|_| self.stack_dec(fn_name!()))?
        {
            ArgKind::Pos(arg) => Args::single(arg),
            ArgKind::Var(arg) => Args::new(vec![], Some(arg), vec![], None, None),
            ArgKind::Kw(arg) => Args::new(vec![], None, vec![arg], None, None),
            ArgKind::KwVar(arg) => Args::new(vec![], None, vec![], Some(arg), None),
        };
        loop {
            match self.peek_kind() {
                Some(Colon) if style.is_colon() || lp.is_some() => {
                    self.skip();
                    let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
                Some(Colon) => {
                    self.skip();
                    style = ArgsStyle::Colon;
                    while self.cur_is(Newline) {
                        self.skip();
                    }
                    expect_pop!(self, fail_next Indent);
                }
                Some(Comma) => {
                    self.skip();
                    if style.is_colon() || self.cur_is(Comma) {
                        let caused_by = caused_by!();
                        log!(err "error caused by: {caused_by}");
                        let loc = self.peek().map(|t| t.loc()).unwrap_or_default();
                        let err = ParseError::invalid_colon_style(line!() as usize, loc);
                        self.errs.push(err);
                        self.until_dedent();
                        debug_exit_info!(self);
                        return Err(());
                    }
                    if style.is_multi_comma() {
                        while self.cur_is(Newline) {
                            self.skip();
                        }
                        if self.cur_is(Dedent) {
                            self.skip();
                        }
                    }
                    if style.needs_parens() && self.cur_is(RParen) {
                        let rp = self.lpop();
                        args.set_parens((lp.unwrap(), rp));
                        break;
                    }
                    if !args.kw_is_empty() {
                        args.push_kw(
                            self.try_reduce_kw_arg(in_type_args)
                                .map_err(|_| self.stack_dec(fn_name!()))?,
                        );
                    } else {
                        match self
                            .try_reduce_arg(in_type_args)
                            .map_err(|_| self.stack_dec(fn_name!()))?
                        {
                            ArgKind::Pos(arg) => {
                                args.push_pos(arg);
                            }
                            ArgKind::Var(var) => {
                                args.set_var_args(var);
                            }
                            ArgKind::Kw(arg) => {
                                args.push_kw(arg);
                            }
                            ArgKind::KwVar(arg) => {
                                args.set_kw_var(arg);
                            }
                        }
                    }
                }
                Some(RParen) => {
                    if let Some(lp) = lp {
                        let rp = self.lpop();
                        args.set_parens((lp, rp));
                    } else {
                        // e.g. f(g 1)
                        let (pos_args, var_args, kw_args, kw_var, _) = args.deconstruct();
                        args = Args::new(pos_args, var_args, kw_args, kw_var, None);
                    }
                    break;
                }
                Some(Newline) => {
                    if !style.is_colon() {
                        if style.needs_parens() && !style.is_multi_comma() {
                            let err = self.skip_and_throw_invalid_seq_err(
                                caused_by!(),
                                line!() as usize,
                                &[")"],
                                Newline,
                            );
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                        if style.is_multi_comma() {
                            self.skip();
                            while self.cur_is(Dedent) {
                                self.skip();
                            }
                            let rp = expect_pop!(self, fail_next RParen);
                            args.set_parens((lp.unwrap(), rp));
                        }
                        break;
                    }
                    let last = self.lpop();
                    if self.cur_is(Dedent) {
                        self.skip();
                        self.restore(last);
                        break;
                    }
                }
                Some(Dedent) if style.is_colon() => {
                    self.skip();
                    break;
                }
                Some(_) if style.is_colon() => {
                    if !args.kw_is_empty() {
                        args.push_kw(
                            self.try_reduce_kw_arg(in_type_args)
                                .map_err(|_| self.stack_dec(fn_name!()))?,
                        );
                    } else {
                        match self
                            .try_reduce_arg(in_type_args)
                            .map_err(|_| self.stack_dec(fn_name!()))?
                        {
                            ArgKind::Pos(arg) => {
                                args.push_pos(arg);
                            }
                            ArgKind::Var(var) => {
                                args.set_var_args(var);
                            }
                            ArgKind::Kw(arg) => {
                                args.push_kw(arg);
                            }
                            ArgKind::KwVar(arg) => {
                                args.set_kw_var(arg);
                            }
                        }
                    }
                }
                None => {
                    self.errs.push(self.unexpected_none(line!(), caused_by!()));
                    debug_exit_info!(self);
                    return Err(());
                }
                _ => break,
            }
        }
        debug_exit_info!(self);
        Ok(args)
    }

    fn try_reduce_arg(&mut self, in_type_args: bool) -> ParseResult<ArgKind> {
        debug_call_info!(self);
        match self.peek_kind() {
            Some(Symbol) => {
                if self.nth_is(1, Walrus) {
                    let acc = self
                        .try_reduce_acc_lhs()
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    debug_power_assert!(self.cur_is(Walrus));
                    self.skip();
                    let kw = if let Accessor::Ident(n) = acc {
                        n.name.into_token()
                    } else {
                        let caused_by = caused_by!();
                        log!(err "error caused by: {caused_by}");
                        let err = ParseError::expect_keyword(line!() as usize, acc.loc());
                        self.errs.push(err);
                        self.next_expr();
                        debug_exit_info!(self);
                        return Err(());
                    };
                    let expr = self
                        .try_reduce_expr(false, in_type_args, false, false)
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: Nat型",
                                    "simplified_chinese" => "期望: Nat类型",
                                    "traditional_chinese" => "期望: Nat類型",
                                    "english" => "expect: Nat type",
                                ))
                            }
                            self.stack_dec(fn_name!())
                        })?;
                    debug_exit_info!(self);
                    Ok(ArgKind::Kw(KwArg::new(kw, None, expr)))
                } else {
                    let expr = self
                        .try_reduce_expr(false, in_type_args, false, false)
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: 型指定",
                                    "simplified_chinese" => "期望: 类型规范",
                                    "traditional_chinese" => "期望: 類型規範",
                                    "english" => "expect: type specification",
                                ))
                            }
                            self.stack_dec(fn_name!())
                        })?;
                    if self.cur_is(Walrus) {
                        self.skip();
                        let (kw, t_spec) = match expr {
                            Expr::Accessor(Accessor::Ident(n)) => (n.name.into_token(), None),
                            Expr::TypeAscription(tasc) => {
                                if let Expr::Accessor(Accessor::Ident(n)) = *tasc.expr {
                                    (n.name.into_token(), Some(tasc.t_spec))
                                } else {
                                    let err = self.skip_and_throw_invalid_seq_err(
                                        caused_by!(),
                                        line!() as usize,
                                        &["right enclosure", "element"],
                                        Comma,
                                    );
                                    self.errs.push(err);
                                    debug_exit_info!(self);
                                    return Err(());
                                }
                            }
                            _ => {
                                let caused_by = caused_by!();
                                log!(err "error caused by: {caused_by}");
                                let err = ParseError::expect_keyword(line!() as usize, expr.loc());
                                self.errs.push(err);
                                self.next_expr();
                                debug_exit_info!(self);
                                return Err(());
                            }
                        };
                        let expr = self
                            .try_reduce_expr(false, in_type_args, false, false)
                            .map_err(|_| {
                                if let Some(err) = self.errs.last_mut() {
                                    err.set_hint(switch_lang!(
                                        "japanese" => "予期: 型指定",
                                        "simplified_chinese" => "期望: 类型规范",
                                        "traditional_chinese" => "期望: 類型規範",
                                        "english" => "expect: type specification",
                                    ))
                                }
                                self.stack_dec(fn_name!())
                            })?;
                        debug_exit_info!(self);
                        Ok(ArgKind::Kw(KwArg::new(kw, t_spec, expr)))
                    } else {
                        debug_exit_info!(self);
                        Ok(ArgKind::Pos(PosArg::new(expr)))
                    }
                }
            }
            Some(star @ (PreStar | PreDblStar)) => {
                self.skip();
                let expr = self
                    .try_reduce_expr(false, in_type_args, false, false)
                    .map_err(|_| {
                        if let Some(err) = self.errs.last_mut() {
                            err.set_hint(switch_lang!(
                                "japanese" => "予期: 型指定",
                                "simplified_chinese" => "期望: 类型规范",
                                "traditional_chinese" => "期望: 類型規範",
                                "english" => "expect: type specification",
                            ))
                        }
                        self.stack_dec(fn_name!())
                    })?;
                debug_exit_info!(self);
                if star == PreStar {
                    Ok(ArgKind::Var(PosArg::new(expr)))
                } else {
                    Ok(ArgKind::KwVar(PosArg::new(expr)))
                }
            }
            Some(_) => {
                let expr = self
                    .try_reduce_expr(false, in_type_args, false, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(ArgKind::Pos(PosArg::new(expr)))
            }
            None => {
                self.errs.push(self.unexpected_none(line!(), caused_by!()));
                debug_exit_info!(self);
                Err(())
            }
        }
    }

    fn try_reduce_kw_arg(&mut self, in_type_args: bool) -> ParseResult<KwArg> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.is(Symbol) => {
                if self.nth_is(1, Walrus) {
                    let acc = self
                        .try_reduce_acc_lhs()
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    debug_power_assert!(self.cur_is(Walrus));
                    self.skip();
                    let keyword = if let Accessor::Ident(n) = acc {
                        n.name.into_token()
                    } else {
                        let caused_by = caused_by!();
                        log!(err "error caused by: {caused_by}");
                        let err = ParseError::expect_keyword(line!() as usize, acc.loc());
                        self.errs.push(err);
                        self.next_expr();
                        debug_exit_info!(self);
                        return Err(());
                    };
                    let expr = self
                        .try_reduce_expr(false, in_type_args, false, false)
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: 引数",
                                    "simplified_chinese" => "期望: 参数",
                                    "traditional_chinese" => "期望: 參數",
                                    "english" => "expect: an argument",
                                ))
                            }
                            self.stack_dec(fn_name!())
                        })?;
                    debug_exit_info!(self);
                    Ok(KwArg::new(keyword, None, expr))
                } else if self.nth_is(1, Colon) {
                    let acc = self
                        .try_reduce_acc_lhs()
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    let colon = expect_pop!(self, Colon);
                    let t_spec_as_expr = self
                        .try_reduce_expr(false, true, false, false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    let t_spec = match Parser::expr_to_type_spec(t_spec_as_expr.clone()) {
                        Ok(t_spec) => TypeSpecWithOp::new(colon, t_spec, t_spec_as_expr),
                        Err(err) => {
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    };
                    debug_power_assert!(self.cur_is(Walrus));
                    self.skip();
                    let keyword = if let Accessor::Ident(n) = acc {
                        n.name.into_token()
                    } else {
                        let caused_by = caused_by!();
                        log!(err "error caused by: {caused_by}");
                        let err = ParseError::expect_keyword(line!() as usize, acc.loc());
                        self.errs.push(err);
                        self.next_expr();
                        debug_exit_info!(self);
                        return Err(());
                    };
                    let expr = self
                        .try_reduce_expr(false, in_type_args, false, false)
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: 引数",
                                    "simplified_chinese" => "期望: 参数",
                                    "traditional_chinese" => "期望: 參數",
                                    "english" => "expect: an argument",
                                ))
                            }
                            self.stack_dec(fn_name!())
                        })?;
                    debug_exit_info!(self);
                    Ok(KwArg::new(keyword, Some(t_spec), expr))
                } else {
                    let caused_by = caused_by!();
                    log!(err "error caused by: {caused_by}");
                    let err = ParseError::invalid_non_default_parameter(line!() as usize, t.loc());
                    self.errs.push(err);
                    self.next_expr();
                    debug_exit_info!(self);
                    Err(())
                }
            }
            Some(lit) if lit.category_is(TC::Literal) => {
                let caused_by = caused_by!();
                log!(err "error caused by: {caused_by}");
                let err = ParseError::invalid_non_default_parameter(line!() as usize, lit.loc());
                self.errs.push(err);
                self.next_expr();
                debug_exit_info!(self);
                Err(())
            }
            Some(other) => {
                let caused_by = caused_by!();
                log!(err "error caused by: {caused_by}");
                let err = ParseError::expect_keyword(line!() as usize, other.loc());
                self.errs.push(err);
                self.next_expr();
                debug_exit_info!(self);
                Err(())
            }
            None => {
                self.errs.push(self.unexpected_none(line!(), caused_by!()));
                debug_exit_info!(self);
                Err(())
            }
        }
    }

    fn try_reduce_class_attr_defs(
        &mut self,
        class: Expr,
        vis: VisModifierSpec,
    ) -> ParseResult<Methods> {
        debug_call_info!(self);
        expect_pop!(self, fail_next Indent);
        while self.cur_is(Newline) {
            self.skip();
        }
        let first = self.try_reduce_chunk(false, false).map_err(|_| {
            if let Some(err) = self.errs.last_mut() {
                err.set_hint(switch_lang!(
                    "japanese" => "メソッドか属性のみ定義できます",
                    "simplified_chinese" => "只能定义方法或属性",
                    "traditional_chinese" => "只能定義方法或屬性",
                    "english" => "only a method or attribute can be defined",
                ))
            }
            self.stack_dec(fn_name!())
        })?;
        let first = match first {
            Expr::Def(def) => ClassAttr::Def(def),
            Expr::TypeAscription(tasc) => ClassAttr::Decl(tasc),
            Expr::Literal(lit) if lit.is_doc_comment() => ClassAttr::Doc(lit),
            other => {
                let caused_by = caused_by!();
                log!(err "error caused by: {caused_by}");
                let hint = switch_lang!(
                    "japanese" => "メソッドか属性のみ定義できます",
                    "simplified_chinese" => "只能定义方法或属性",
                    "traditional_chinese" => "只能定義方法或屬性",
                    "english" => "only a method or attribute can be defined",
                )
                .to_string();
                let err = ParseError::syntax_error(
                    line!() as usize,
                    other.loc(),
                    switch_lang!(
                        "japanese" => "クラス属性を定義するのに失敗しました",
                        "simplified_chinese" => "定义类属性失败",
                        "traditional_chinese" => "定義類屬性失敗",
                        "english" => "failed to define a class attribute",
                    ),
                    Some(hint),
                );
                self.errs.push(err);
                self.until_dedent();
                debug_exit_info!(self);
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
                Some(t) if t.is(Dedent) => {
                    self.skip();
                    break;
                }
                Some(t) if t.category_is(TC::Separator) => {
                    self.skip();
                }
                Some(_) => {
                    let def = self.try_reduce_chunk(false, false).map_err(|_| {
                        if let Some(err) = self.errs.last_mut() {
                            err.set_hint(switch_lang!(
                                "japanese" => "クラス属性かメソッドを定義してください",
                                "simplified_chinese" => "应声明类属性或方法",
                                "traditional_chinese" => "應聲明類屬性或方法",
                                "english" => "class attribute or method should be declared",
                            ))
                        }
                        self.stack_dec(fn_name!())
                    })?;
                    match def {
                        Expr::Def(def) => {
                            attrs.push(ClassAttr::Def(def));
                        }
                        Expr::TypeAscription(tasc) => {
                            attrs.push(ClassAttr::Decl(tasc));
                        }
                        Expr::Literal(lit) if lit.is_doc_comment() => {
                            attrs.push(ClassAttr::Doc(lit));
                        }
                        other => {
                            let caused_by = caused_by!();
                            log!(err "error caused by: {caused_by}");
                            let err = ParseError::syntax_error(
                                line!() as usize,
                                other.loc(),
                                switch_lang!(
                                    "japanese" => "クラス属性を定義するのに失敗しました",
                                    "simplified_chinese" => "定义类属性失败",
                                    "traditional_chinese" => "定義類屬性失敗",
                                    "english" => "failed to define a class attribute",
                                ),
                                None,
                            );
                            self.errs.push(err);
                            self.next_expr();
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                    match self.peek() {
                        Some(t) if !t.is(Dedent) && !t.category_is(TC::Separator) => {
                            let err = self.skip_and_throw_invalid_chunk_err(
                                caused_by!(),
                                line!(),
                                t.loc(),
                            );
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                        Some(_) => {}
                        None => {
                            self.errs.push(self.unexpected_none(line!(), caused_by!()));
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                None => {
                    self.errs.push(self.unexpected_none(line!(), caused_by!()));
                    debug_exit_info!(self);
                    return Err(());
                }
            }
        }
        let attrs = ClassAttrs::from(attrs);
        let t_spec = Self::expr_to_type_spec(class.clone()).map_err(|e| {
            self.errs.push(e);
            self.stack_dec(fn_name!())
        })?;
        debug_exit_info!(self);
        self.counter.inc();
        Ok(Methods::new(self.counter, t_spec, class, vis, attrs))
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
            if self.cur_is(EOF) {
                let err = ParseError::expect_next_line_error(line!() as usize, op.loc(), "Lambda");
                self.errs.push(err);
                return Err(());
            }
            let body = self
                .try_reduce_block()
                .map_err(|_| self.stack_dec(fn_name!()))?;
            self.counter.inc();
            debug_exit_info!(self);
            Ok(Lambda::new(sig, op, body, self.counter))
        } else {
            let expr = self
                .try_reduce_expr(false, false, false, false)
                .map_err(|_| {
                    if let Some(err) = self.errs.last_mut() {
                        err.set_hint(switch_lang!(
                            "japanese" => "予期: 式",
                            "simplified_chinese" => "期望: 表达",
                            "traditional_chinese" => "期望: 表達",
                            "english" => "expect: expression",
                        ))
                    }
                    self.stack_dec(fn_name!())
                })?;
            let block = Block::new(vec![expr]);
            debug_exit_info!(self);
            Ok(Lambda::new(sig, op, block, self.counter))
        }
    }

    /// chunk = normal expr + def
    fn try_reduce_chunk(&mut self, winding: bool, in_brace: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        let mut stack = Vec::<ExprOrOp>::new();
        let lhs = self
            .try_reduce_bin_lhs(false, in_brace)
            .map_err(|_| self.stack_dec(fn_name!()))?;
        if lhs.is_macro_call() {
            debug_exit_info!(self);
            return Ok(lhs);
        }
        stack.push(ExprOrOp::Expr(lhs));
        loop {
            match self.peek() {
                Some(arg) if arg.is(Symbol) || arg.category_is(TC::Literal) => {
                    let args = self
                        .try_reduce_args(false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    let obj = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    stack.push(ExprOrOp::Expr(obj.call_expr(args)));
                }
                Some(op) if op.category_is(TC::DefOp) => {
                    let op = self.lpop();
                    if self.cur_is(EOF) {
                        let err = ParseError::expect_next_line_error(
                            line!() as usize,
                            op.loc(),
                            "Assignment",
                        );
                        self.errs.push(err);
                        return Err(());
                    }
                    let is_multiline_block = self.cur_is(Newline);
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self
                        .convert_rhs_to_sig(lhs)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    self.counter.inc();
                    let block = if is_multiline_block {
                        self.try_reduce_block()
                            .map_err(|_| self.stack_dec(fn_name!()))?
                    } else {
                        // precedence: `=` < `,`
                        let expr =
                            self.try_reduce_expr(true, false, false, false)
                                .map_err(|_| {
                                    if let Some(err) = self.errs.last_mut() {
                                        err.set_hint(switch_lang!(
                                            "japanese" => "予期: 式",
                                            "simplified_chinese" => "期望: 表达",
                                            "traditional_chinese" => "期望: 表達",
                                            "english" => "expect: expression",
                                        ))
                                    }
                                    self.stack_dec(fn_name!())
                                })?;
                        Block::new(vec![expr])
                    };
                    let body = DefBody::new(op, block, self.counter);
                    debug_exit_info!(self);
                    return Ok(Expr::Def(Def::new(sig, body)));
                }
                Some(op) if op.category_is(TC::LambdaOp) => {
                    let op = self.lpop();
                    if self.cur_is(EOF) {
                        let err = ParseError::expect_next_line_error(
                            line!() as usize,
                            op.loc(),
                            "Lambda",
                        );
                        self.errs.push(err);
                        return Err(());
                    }
                    let is_multiline_block = self.cur_is(Newline);
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self
                        .convert_rhs_to_lambda_sig(lhs)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    self.counter.inc();
                    let block = if is_multiline_block {
                        self.try_reduce_block()
                            .map_err(|_| self.stack_dec(fn_name!()))?
                    } else {
                        // precedence: `->` > `,`
                        let expr =
                            self.try_reduce_expr(false, false, false, false)
                                .map_err(|_| {
                                    if let Some(err) = self.errs.last_mut() {
                                        err.set_hint(switch_lang!(
                                            "japanese" => "予期: 式",
                                            "simplified_chinese" => "期望: 表达",
                                            "traditional_chinese" => "期望: 表達",
                                            "english" => "expect: expression",
                                        ))
                                    }
                                    self.stack_dec(fn_name!())
                                })?;
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
                        || (op.is(SubtypeOf) || op.is(SupertypeOf) || op.is(As)) =>
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
                    let t_spec_as_expr = self
                        .try_reduce_expr(false, false, false, false)
                        .map(Desugarer::desugar_simple_expr)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    let t_spec = Self::expr_to_type_spec(t_spec_as_expr.clone()).map_err(|e| {
                        self.errs.push(e);
                        self.stack_dec(fn_name!())
                    })?;
                    let t_spec_op = TypeSpecWithOp::new(op, t_spec, t_spec_as_expr);
                    let expr = lhs.type_asc_expr(t_spec_op);
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
                        self.try_reduce_bin_lhs(false, in_brace).map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: 数値、式",
                                    "simplified_chinese" => "期望: 数字或表达式",
                                    "traditional_chinese" => "期望: 數字或表達式",
                                    "english" => "expect: number or expression",
                                ));
                            }
                            self.stack_dec(fn_name!())
                        })?,
                    ));
                }
                Some(t) if t.is(DblColon) => {
                    let dcolon = self.lpop();
                    let token = self.lpop();
                    match token.kind {
                        Symbol => {
                            let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                                let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                                self.errs.push(err);
                                debug_exit_info!(self);
                                return Err(());
                            };
                            if let Some(args) = self
                                .opt_reduce_args(false)
                                .transpose()
                                .map_err(|_| self.stack_dec(fn_name!()))?
                            {
                                let ident =
                                    Identifier::new(VisModifierSpec::Private, VarName::new(token));
                                let call = Call::new(obj, Some(ident), args);
                                stack.push(ExprOrOp::Expr(Expr::Call(call)));
                            } else {
                                let ident =
                                    Identifier::new(VisModifierSpec::Private, VarName::new(token));
                                stack.push(ExprOrOp::Expr(obj.attr_expr(ident)));
                            }
                        }
                        Newline => {
                            let vis = VisModifierSpec::ExplicitPrivate(dcolon);
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            let defs = self
                                .try_reduce_class_attr_defs(maybe_class, vis)
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            let expr = Expr::Methods(defs);
                            assert_eq!(stack.len(), 0);
                            debug_exit_info!(self);
                            return Ok(expr);
                        }
                        LSqBr => {
                            self.restore(token);
                            let restriction = self
                                .try_reduce_restriction()
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            let vis = VisModifierSpec::Restricted(restriction);
                            expect_pop!(self, Newline);
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            let defs = self
                                .try_reduce_class_attr_defs(maybe_class, vis)
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            let expr = Expr::Methods(defs);
                            assert_eq!(stack.len(), 0);
                            debug_exit_info!(self);
                            return Ok(expr);
                        }
                        LBrace => {
                            let vis = VisModifierSpec::ExplicitPrivate(dcolon);
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            self.restore(token);
                            let container = self
                                .try_reduce_brace_container()
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            match container {
                                BraceContainer::Record(args) => {
                                    let pack = DataPack::new(maybe_class, vis, args);
                                    stack.push(ExprOrOp::Expr(Expr::DataPack(pack)));
                                }
                                other => {
                                    let err = ParseError::invalid_data_pack_definition(
                                        line!() as usize,
                                        other.loc(),
                                        other.kind(),
                                    );
                                    self.errs.push(err);
                                    debug_exit_info!(self);
                                    return Err(());
                                }
                            }
                        }
                        _ => {
                            self.restore(token);
                            let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                Some(t) if t.is(Dot) => {
                    let dot = self.lpop();
                    let token = self.lpop();
                    match token.kind {
                        Symbol => {
                            let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                                let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                                self.errs.push(err);
                                debug_exit_info!(self);
                                return Err(());
                            };
                            if let Some(args) = self
                                .opt_reduce_args(false)
                                .transpose()
                                .map_err(|_| self.stack_dec(fn_name!()))?
                            {
                                let ident = Identifier::public_from_token(dot, token);
                                let call = Expr::Call(Call::new(obj, Some(ident), args));
                                stack.push(ExprOrOp::Expr(call));
                            } else {
                                let ident = Identifier::public_from_token(dot, token);
                                stack.push(ExprOrOp::Expr(obj.attr_expr(ident)));
                            }
                        }
                        Newline => {
                            let vis = VisModifierSpec::Public(dot);
                            let maybe_class = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                            let defs = self
                                .try_reduce_class_attr_defs(maybe_class, vis)
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            debug_exit_info!(self);
                            return Ok(Expr::Methods(defs));
                        }
                        _ => {
                            self.restore(token);
                            let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                Some(t) if t.is(LSqBr) => {
                    let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                        let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    };
                    self.skip();
                    let index = self
                        .try_reduce_expr(false, false, in_brace, false)
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: Nat型",
                                    "simplified_chinese" => "期望: Nat类型",
                                    "traditional_chinese" => "期望: Nat類型",
                                    "english" => "expect: Nat type",
                                ))
                            }
                            self.stack_dec(fn_name!())
                        })?;
                    let r_sqbr = expect_pop!(self, fail_next RSqBr);
                    let acc = Accessor::subscr(obj, index, r_sqbr);
                    stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
                }
                Some(t) if t.is(Comma) && winding => {
                    let first_elem = ArgKind::Pos(PosArg::new(
                        enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_))),
                    ));
                    let tup = self
                        .try_reduce_nonempty_tuple(first_elem, false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tup)));
                }
                Some(t) if t.is(Walrus) && winding => {
                    let tuple = self
                        .try_reduce_default_parameters(&mut stack, in_brace)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tuple)));
                }
                Some(t) if t.is(Pipe) => self
                    .try_reduce_stream_operator(&mut stack)
                    .map_err(|_| self.stack_dec(fn_name!()))?,
                Some(t) if t.category_is(TC::Reserved) => {
                    let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                    self.errs.push(err);
                    debug_exit_info!(self);
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
                debug_exit_info!(self);
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
                debug_exit_info!(self);
                Ok(expr)
            }
            Some(ExprOrOp::Op(op)) => {
                self.errs
                    .push(ParseError::compiler_bug(0, op.loc(), fn_name!(), line!()));
                debug_exit_info!(self);
                Err(())
            }
            _ => switch_unreachable!(),
        }
    }

    fn try_consume_macro_keyword(&mut self) -> bool {
        let Some(idx) = self.peek().and_then(|tok| {
            self.keywords
                .iter()
                .position(|kw| kw.inspect() == tok.inspect())
        }) else {
            return false;
        };
        let found = &self.keywords[idx];
        if idx > 0 && found.is_non_default() {
            false
        } else {
            self.keywords.drain(0..=idx);
            true
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
                .map_err(|_| self.stack_dec(fn_name!()))?,
        ));
        loop {
            if self.try_consume_macro_keyword() {
                match stack.pop() {
                    Some(ExprOrOp::Expr(expr)) => {
                        // self.skip();
                        debug_exit_info!(self);
                        return Ok(expr);
                    }
                    _ => {
                        let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    }
                }
            }
            match self.peek() {
                Some(op) if op.category_is(TC::LambdaOp) => {
                    let op = self.lpop();
                    let is_multiline_block = self.cur_is(Newline);
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let sig = self
                        .convert_rhs_to_lambda_sig(lhs)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    self.counter.inc();
                    if self.cur_is(EOF) {
                        let err = ParseError::expect_next_line_error(
                            line!() as usize,
                            op.loc(),
                            "Lambda",
                        );
                        self.errs.push(err);
                        return Err(());
                    }
                    let block = if is_multiline_block {
                        self.try_reduce_block()
                            .map_err(|_| self.stack_dec(fn_name!()))?
                    } else {
                        let expr = self
                            .try_reduce_expr(false, false, false, false)
                            .map_err(|_| self.stack_dec(fn_name!()))?;
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
                        || (op.is(SubtypeOf) || op.is(SupertypeOf) || op.is(As)) =>
                {
                    // "a": 1 (key-value pair)
                    if in_brace {
                        break;
                    }
                    let op = self.lpop();
                    if self.cur_is(EOF) {
                        let err = ParseError::expect_next_line_error(
                            line!() as usize,
                            op.loc(),
                            "Lambda",
                        );
                        self.errs.push(err);
                        return Err(());
                    }
                    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
                    let t_spec_as_expr = self
                        .try_reduce_expr(false, in_type_args, in_brace, false)
                        .map(Desugarer::desugar_simple_expr)
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: Nat型",
                                    "simplified_chinese" => "期望: Nat类型",
                                    "traditional_chinese" => "期望: Nat類型",
                                    "english" => "expect: Nat type",
                                ))
                            }
                            self.stack_dec(fn_name!())
                        })?;
                    let t_spec = Self::expr_to_type_spec(t_spec_as_expr.clone())
                        .map_err(|e| self.errs.push(e))?;
                    let t_spec_op = TypeSpecWithOp::new(op, t_spec, t_spec_as_expr);
                    let expr = lhs.type_asc_expr(t_spec_op);
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
                            .map_err(|_| {
                                if let Some(err) = self.errs.last_mut() {
                                    err.set_hint(switch_lang!(
                                    "japanese" => "予期: 式、被演算子",
                                    "simplified_chinese" => "期望：表达式或操作数",
                                    "traditional_chinese" => "期望：表達式或操作數",
                                    "english" => "expect: expression or operand",
                                    ))
                                }
                                self.stack_dec(fn_name!())
                            })?,
                    ));
                }
                Some(t) if t.is(Dot) => {
                    let vis = self.lpop();
                    match self.lpop() {
                        symbol if symbol.is(Symbol) => {
                            let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                                let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                                self.errs.push(err);
                                debug_exit_info!(self);
                                return Err(());
                            };
                            if let Some(args) = self
                                .opt_reduce_args(in_type_args)
                                .transpose()
                                .map_err(|_| self.stack_dec(fn_name!()))?
                            {
                                let ident = Identifier::new(
                                    VisModifierSpec::Public(vis),
                                    VarName::new(symbol),
                                );
                                let call = Call::new(obj, Some(ident), args);
                                stack.push(ExprOrOp::Expr(Expr::Call(call)));
                            } else {
                                let ident = Identifier::new(
                                    VisModifierSpec::Public(vis),
                                    VarName::new(symbol),
                                );
                                stack.push(ExprOrOp::Expr(obj.attr_expr(ident)));
                            }
                        }
                        other => {
                            self.restore(other);
                            let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                Some(t) if t.is(LSqBr) => {
                    let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                        let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    };
                    self.skip(); // l_sqbr
                    let index = self
                        .try_reduce_expr(false, false, in_brace, false)
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "予期: Nat型",
                                    "simplified_chinese" => "期望: Nat类型",
                                    "traditional_chinese" => "期望: Nat類型",
                                    "english" => "expect: Nat type",
                                ))
                            }
                            self.stack_dec(fn_name!());
                        })?;
                    let r_sqbr = self.lpop();
                    if !r_sqbr.is(RSqBr) {
                        let caused_by = caused_by!();
                        log!(err "error caused by: {caused_by}");
                        let err = ParseError::expect_accessor(line!() as usize, index.loc());
                        self.errs.push(err);
                        self.next_expr();
                        debug_exit_info!(self);
                        return Err(());
                    }
                    let acc = Accessor::subscr(obj, index, r_sqbr);
                    stack.push(ExprOrOp::Expr(Expr::Accessor(acc)));
                }
                Some(t) if t.is(Comma) && winding => {
                    let first_elem = ArgKind::Pos(PosArg::new(
                        enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_))),
                    ));
                    let tup = self
                        .try_reduce_nonempty_tuple(first_elem, line_break)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tup)));
                }
                Some(t) if t.is(Walrus) && winding => {
                    let tuple = self
                        .try_reduce_default_parameters(&mut stack, in_brace)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    stack.push(ExprOrOp::Expr(Expr::Tuple(tuple)));
                }
                Some(t) if t.is(Pipe) => self
                    .try_reduce_stream_operator(&mut stack)
                    .map_err(|_| self.stack_dec(fn_name!()))?,
                Some(t) if t.category_is(TC::Reserved) => {
                    let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                    self.errs.push(err);
                    debug_exit_info!(self);
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
                debug_exit_info!(self);
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
                debug_exit_info!(self);
                Ok(expr)
            }
            Some(ExprOrOp::Op(op)) => {
                self.errs
                    .push(ParseError::compiler_bug(0, op.loc(), fn_name!(), line!()));
                debug_exit_info!(self);
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
            Expr::TypeAscription(tasc) => {
                if let Expr::Accessor(Accessor::Ident(ident)) = *tasc.expr {
                    (ident.name.into_token(), Some(tasc.t_spec))
                } else {
                    let caused_by = caused_by!();
                    log!(err "error caused by: {caused_by}");
                    let err = ParseError::expect_keyword(line!() as usize, tasc.loc());
                    self.errs.push(err);
                    self.next_expr();
                    debug_exit_info!(self);
                    return Err(());
                }
            }
            other => {
                let caused_by = caused_by!();
                log!(err "error caused by: {caused_by}");
                let err = ParseError::expect_keyword(line!() as usize, other.loc());
                self.errs.push(err);
                self.next_expr();
                debug_exit_info!(self);
                return Err(());
            }
        };
        self.skip(); // :=
        let rhs = self
            .try_reduce_expr(false, false, in_brace, false)
            .map_err(|_| {
                if let Some(err) = self.errs.last_mut() {
                    err.set_hint(switch_lang!(
                        "japanese" => "予期: デフォルト引数",
                        "simplified_chinese" => "期望: 默认参数",
                        "traditional_chinese" => "期望: 默認參數",
                        "english" => "expect: default parameter",
                    ))
                }
                self.stack_dec(fn_name!())
            })?;
        let first_elem = ArgKind::Kw(KwArg::new(keyword, t_spec, rhs));
        let tuple = self
            .try_reduce_nonempty_tuple(first_elem, self.nth_is(1, Newline))
            .map_err(|_| self.stack_dec(fn_name!()))?;
        debug_exit_info!(self);
        Ok(tuple)
    }

    /// "LHS" is the smallest unit that can be the left-hand side of an BinOp.
    /// e.g. Call, Name, UnaryOp, Lambda
    fn try_reduce_bin_lhs(&mut self, in_type_args: bool, in_brace: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if &t.inspect()[..] == "do" || &t.inspect()[..] == "do!" => {
                let lambda = self
                    .try_reduce_do_block()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(Expr::Lambda(lambda))
            }
            Some(t) if t.category_is(TC::Literal) => {
                let lit = self
                    .try_reduce_lit()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                if let Some(tk) = self.peek() {
                    if tk.is(Mutate) {
                        self.skip();
                        let main_msg = switch_lang!(
                            "japanese" => "可変演算子は、後置演算子ではなく前置演算子です ",
                            "simplified_chinese" => "突变运算符是前缀运算符，不是后缀运算符",
                            "traditional_chinese" => "突變運算符是前綴運算符，不是後綴運算符",
                            "english" => "the mutation operator is a prefix operator, not a postfix operator ",
                        );
                        let lit_loc = lit.loc();
                        let lit = lit.token.inspect();
                        let err = ParseError::invalid_token_error(
                            line!() as usize,
                            lit_loc,
                            main_msg,
                            &format!("!{lit}"),
                            &format!("{lit}!"),
                        );
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    }
                }
                debug_exit_info!(self);
                Ok(Expr::Literal(lit))
            }
            Some(t) if t.is(StrInterpLeft) => {
                let str_interp = self
                    .try_reduce_string_interpolation()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(str_interp)
            }
            Some(t) if t.is(AtSign) => {
                let decos = self.opt_reduce_decorators()?;
                let expr = self.try_reduce_chunk(false, in_brace).map_err(|_| {
                    if let Some(err) = self.errs.last_mut() {
                        err.set_hint(switch_lang!(
                            "japanese" => "期待: デコレータ",
                            "simplified_chinese" => "期望: 装饰器",
                            "traditions_chinese" => "期望: 裝飾器",
                            "english" => "expect: decorator",
                        ))
                    }
                    self.stack_dec(fn_name!())
                })?;
                let Expr::Def(mut def) = expr else {
                    // self.restore(other);
                    let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                };
                match def.sig {
                    Signature::Subr(mut subr) => {
                        subr.decorators = decos;
                        let expr = Expr::Def(Def::new(Signature::Subr(subr), def.body));
                        debug_exit_info!(self);
                        Ok(expr)
                    }
                    Signature::Var(var) => {
                        let mut last = def.body.block.pop().unwrap();
                        for deco in decos.into_iter() {
                            last = deco.into_expr().call_expr(Args::single(PosArg::new(last)));
                        }
                        def.body.block.push(last);
                        let expr = Expr::Def(Def::new(Signature::Var(var), def.body));
                        debug_exit_info!(self);
                        Ok(expr)
                    }
                }
            }
            Some(t) if t.is(Symbol) || t.is(Dot) || t.is(DblColon) || t.is(UBar) => {
                let call_or_acc = self
                    .try_reduce_call_or_acc(in_type_args)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(call_or_acc)
            }
            // REVIEW: correct?
            Some(t) if t.is(PreStar) || t.is(PreDblStar) => {
                let kind = t.kind;
                let _ = self.lpop();
                let expr = self
                    .try_reduce_expr(false, in_type_args, in_brace, false)
                    .map_err(|_| {
                        if let Some(err) = self.errs.last_mut() {
                            err.set_hint(switch_lang!(
                                "japanese" => "期待: 可変長引数",
                                "simplified_chinese" => "期望: 可变长度参数",
                                "traditional_chinese" => "期望: 可變長度參數",
                                "english" => "expect: variable-length arguments",
                            ))
                        }
                        self.stack_dec(fn_name!())
                    })?;
                let arg = match kind {
                    PreStar => ArgKind::Var(PosArg::new(expr)),
                    PreDblStar => ArgKind::KwVar(PosArg::new(expr)),
                    _ => switch_unreachable!(),
                };
                let tuple = self
                    .try_reduce_nonempty_tuple(arg, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(Expr::Tuple(tuple))
            }
            Some(t) if t.category_is(TC::UnaryOp) => {
                let unaryop = self
                    .try_reduce_unary()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
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
                    let args = Args::pos_only(vec![], Some((lparen, rparen)));
                    let unit = Tuple::Normal(NormalTuple::new(args));
                    debug_exit_info!(self);
                    return Ok(Expr::Tuple(unit));
                }
                let mut expr = self
                    .try_reduce_expr(true, false, false, line_break)
                    .map_err(|_| {
                        if let Some(err) = self.errs.last_mut() {
                            err.set_hint(switch_lang!(
                                "japanese" => "期待: 要素",
                                "simplified_chinese" => "期望: 元素",
                                "traditional_chinese" => "期望: 元素",
                                "english" => "expect: an element",
                            ))
                        }
                        self.stack_dec(fn_name!())
                    })?;
                if line_break {
                    while self.cur_is(Newline) {
                        self.skip();
                    }
                    if self.cur_is(Dedent) {
                        self.skip();
                    }
                }
                let rparen = match self.peek_kind() {
                    Some(RParen) => self.lpop(),
                    Some(_) => {
                        let err = self.skip_and_throw_invalid_unclosed_err(
                            caused_by!(),
                            line!(),
                            ")",
                            "tuple",
                        );
                        self.errs.push(err);
                        return Err(());
                    }
                    None => {
                        self.errs.push(self.unexpected_none(line!(), caused_by!()));
                        debug_exit_info!(self);
                        return Err(());
                    }
                };
                if let Expr::Tuple(Tuple::Normal(tup)) = &mut expr {
                    tup.elems.paren = Some((lparen, rparen));
                }
                debug_exit_info!(self);
                Ok(expr)
            }
            Some(t) if t.is(LSqBr) => {
                let array = self
                    .try_reduce_array()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(Expr::Array(array))
            }
            Some(t) if t.is(LBrace) => {
                match self
                    .try_reduce_brace_container()
                    .map_err(|_| self.stack_dec(fn_name!()))?
                {
                    BraceContainer::Dict(dic) => {
                        debug_exit_info!(self);
                        Ok(Expr::Dict(dic))
                    }
                    BraceContainer::Record(rec) => {
                        debug_exit_info!(self);
                        Ok(Expr::Record(rec))
                    }
                    BraceContainer::Set(set) => {
                        debug_exit_info!(self);
                        Ok(Expr::Set(set))
                    }
                }
            }
            Some(t) if t.is(VBar) => {
                let type_args = self
                    .try_reduce_type_app_args()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let bounds = self
                    .convert_type_args_to_bounds(type_args)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let args = self
                    .try_reduce_args(false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let params = self
                    .convert_args_to_params(args)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                let sig = LambdaSignature::new(params, None, bounds);
                let op = expect_pop!(self, category TC::LambdaOp);
                let block = self
                    .try_reduce_block()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                self.counter.inc();
                let lambda = Lambda::new(sig, op, block, self.counter);
                debug_exit_info!(self);
                Ok(Expr::Lambda(lambda))
            }
            Some(t) if t.is(UBar) => {
                let token = self.lpop();
                let caused_by = caused_by!();
                log!(err "error caused by: {caused_by}");
                self.errs.push(ParseError::feature_error(
                    line!() as usize,
                    token.loc(),
                    "discard pattern",
                ));
                debug_exit_info!(self);
                Err(())
            }
            Some(_other) => {
                let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                self.errs.push(err);
                debug_exit_info!(self);
                Err(())
            }
            None => {
                self.errs.push(self.unexpected_none(line!(), caused_by!()));
                debug_exit_info!(self);
                Err(())
            }
        }
    }

    #[inline]
    fn try_reduce_call_or_acc(&mut self, in_type_args: bool) -> ParseResult<Expr> {
        debug_call_info!(self);
        if let Some(mac) = self
            .peek()
            .and_then(|tok| self.macros.get(&tok.content).cloned())
        {
            let mac = self.try_reduce_macro_call(mac);
            debug_exit_info!(self);
            return mac;
        }
        let acc = self
            .try_reduce_acc_lhs()
            .map_err(|_| self.stack_dec(fn_name!()))?;
        let mut call_or_acc = self
            .try_reduce_acc_chain(acc, in_type_args)
            .map_err(|_| self.stack_dec(fn_name!()))?;
        while let Some(res) = self.opt_reduce_args(in_type_args) {
            let args = res.map_err(|_| self.stack_dec(fn_name!()))?;
            let call = call_or_acc.call(args);
            call_or_acc = Expr::Call(call);
        }
        debug_exit_info!(self);
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
                        .map_err(|_| {
                            if let Some(err) = self.errs.last_mut() {
                                err.set_hint(switch_lang!(
                                    "japanese" => "期待: Nat型",
                                    "simplified_chinese" => "期望: Nat类型",
                                    "traditional_chinese" => "期望: Nat類型",
                                    "english" => "expect: Nat type",
                                ))
                            }
                            self.stack_dec(fn_name!())
                        })?;
                    let r_sqbr = expect_pop!(self, fail_next RSqBr);
                    obj = Expr::Accessor(Accessor::subscr(obj, index, r_sqbr));
                }
                Some(t) if t.is(Dot) && obj.col_end() == t.col_begin() => {
                    let vis = self.lpop();
                    let token = self.lpop();
                    match token.kind {
                        Symbol => {
                            let ident =
                                Identifier::new(VisModifierSpec::Public(vis), VarName::new(token));
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
                        EOF => {
                            let err = ParseError::expect_next_line_error(
                                line!() as usize,
                                token.loc(),
                                "ClassPub",
                            );
                            self.errs.push(err);
                            self.restore(token);
                            self.level -= 1;
                            return Err(());
                        }
                        _ => {
                            let err = ParseError::invalid_acc_chain(
                                line!() as usize,
                                token.loc(),
                                &token.inspect()[..],
                            );
                            self.errs.push(err);
                            debug_exit_info!(self);
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
                            let ident = Identifier::new(
                                VisModifierSpec::ExplicitPrivate(vis),
                                VarName::new(token),
                            );
                            obj = obj.attr_expr(ident);
                        }
                        LBrace => {
                            self.restore(token);
                            let args = self
                                .try_reduce_brace_container()
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            match args {
                                BraceContainer::Record(args) => {
                                    let vis = VisModifierSpec::ExplicitPrivate(vis);
                                    obj = Expr::DataPack(DataPack::new(obj, vis, args));
                                }
                                other => {
                                    let err = ParseError::invalid_data_pack_definition(
                                        line!() as usize,
                                        other.loc(),
                                        other.kind(),
                                    );
                                    self.errs.push(err);
                                    debug_exit_info!(self);
                                    return Err(());
                                }
                            }
                        }
                        // MethodDefs
                        Newline | LSqBr => {
                            self.restore(token);
                            self.restore(vis);
                            break;
                        }
                        EOF => {
                            let err = ParseError::expect_next_line_error(
                                line!() as usize,
                                token.loc(),
                                "ClassPriv",
                            );
                            self.errs.push(err);
                            self.restore(token);
                            self.level -= 1;
                            return Err(());
                        }
                        _ => {
                            self.restore(token);
                            let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                Some(t) if t.is(LParen) && obj.col_end() == t.col_begin() => {
                    let args = self
                        .try_reduce_args(false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
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
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    obj = Expr::Accessor(Accessor::TypeApp(TypeApp::new(obj, type_args)));
                }
                _ => {
                    break;
                }
            }
        }
        debug_exit_info!(self);
        Ok(obj)
    }

    #[inline]
    fn try_reduce_unary(&mut self) -> ParseResult<UnaryOp> {
        debug_call_info!(self);
        let op = self.lpop();
        let expr = self
            .try_reduce_expr(false, false, false, false)
            .map_err(|_| {
                if let Some(err) = self.errs.last_mut() {
                    err.set_hint(switch_lang!(
                        "japanese" => "予期: 式",
                        "simplified_chinese" => "期待：表达式",
                        "traditional_chinese" => "期待：表達式",
                        "english" => "expect: expression",
                    ))
                }
                self.stack_dec(fn_name!())
            })?;
        debug_exit_info!(self);
        Ok(UnaryOp::new(op, expr))
    }

    #[inline]
    fn try_reduce_array(&mut self) -> ParseResult<Array> {
        debug_call_info!(self);
        let l_sqbr = expect_pop!(self, fail_next LSqBr);
        let inner = self
            .try_reduce_array_elems()
            .map_err(|_| self.stack_dec(fn_name!()))?;
        let r_sqbr = expect_pop!(self, fail_next RSqBr);
        let arr = match inner {
            ArrayInner::Normal(mut elems) => {
                let elems = if elems
                    .pos_args()
                    .first()
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
            ArrayInner::Comprehension {
                layout,
                generators,
                guard,
            } => Array::Comprehension(ArrayComprehension::new(
                l_sqbr, r_sqbr, layout, generators, guard,
            )),
        };
        debug_exit_info!(self);
        Ok(arr)
    }

    /// Set, Dict, Record
    fn try_reduce_brace_container(&mut self) -> ParseResult<BraceContainer> {
        debug_call_info!(self);
        let l_brace = expect_pop!(self, fail_next LBrace);
        if self.cur_is(EOF) {
            let err =
                ParseError::expect_next_line_error(line!() as usize, l_brace.loc(), "Collections");
            self.errs.push(err);
            return Err(());
        }
        // Empty brace literals
        match self.peek_kind() {
            Some(RBrace) => {
                let r_brace = self.lpop();
                let arg = Args::empty();
                let set = NormalSet::new(l_brace, r_brace, arg);
                debug_exit_info!(self);
                return Ok(BraceContainer::Set(Set::Normal(set)));
            }
            Some(Assign) => {
                let _eq = self.lpop();
                if let Some(t) = self.peek() {
                    if t.is(RBrace) {
                        let r_brace = self.lpop();
                        debug_exit_info!(self);
                        return Ok(BraceContainer::Record(Record::empty(l_brace, r_brace)));
                    }
                } else {
                    let caused_by = caused_by!();
                    let err = self.unexpected_none(line!(), caused_by);
                    self.errs.push(err);
                    return Err(());
                }
                let t = self.lpop();
                let mut err = ParseError::invalid_token_error(
                    line!() as usize,
                    t.loc(),
                    switch_lang!(
                        "japanese" => "無効なレコードの宣言です",
                        "simplified_chinese" => "无效的Record定义",
                        "traditional_chinese" => "無效的Record定義",
                        "english" => "invalid record",
                    ),
                    "}",
                    &t.inspect()[..],
                );
                err.set_hint(switch_lang!(
                    "japanese" => "空のレコードが期待されています: {=}",
                    "simplified_chinese" => "期望空Record: {=}",
                    "traditional_chinese" => "期望空Record: {=}",
                    "english" => "expect empty record: {=}",
                ));
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            }
            Some(Colon) => {
                let _colon = self.lpop();
                if let Some(t) = self.peek() {
                    if t.is(RBrace) {
                        let r_brace = self.lpop();
                        let dict = NormalDict::new(l_brace, r_brace, vec![]);
                        debug_exit_info!(self);
                        return Ok(BraceContainer::Dict(Dict::Normal(dict)));
                    }
                } else {
                    let caused_by = caused_by!();
                    let err = self.unexpected_none(line!(), caused_by);
                    self.errs.push(err);
                    return Err(());
                }
                let t = self.lpop();
                let mut err = ParseError::invalid_token_error(
                    line!() as usize,
                    t.loc(),
                    switch_lang!(
                        "japanese" => "無効な辞書の宣言です",
                        "simplified_chinese" => "无效的字典定义",
                        "traditional_chinese" => "無效的字典定義",
                        "english" => "invalid dict",
                    ),
                    "}",
                    &t.inspect()[..],
                );
                err.set_hint(switch_lang!(
                    "japanese" => "空の辞書が期待されています: {:}",
                    "simplified_chinese" => "期望空字典: {:}",
                    "traditional_chinese" => "期望空字典: {:}",
                    "english" => "expect empty dict: {:}",
                ));
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            }
            _ => {}
        }

        let first = self.try_reduce_chunk(false, true).map_err(|_| {
            if let Some(err) = self.errs.last_mut() {
                err.set_hint(switch_lang!(
                    "japanese" => "期待: 要素",
                    "simplified_chinese" => "期望: 元素",
                    "traditional_chinese" => "期望: 元素",
                    "english" => "expect: an element",
                ))
            }
            self.stack_dec(fn_name!())
        })?;
        match first {
            Expr::Def(def) => {
                let attr = RecordAttrOrIdent::Attr(def);
                let record = self
                    .try_reduce_record(l_brace, attr)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
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
                        let caused_by = caused_by!();
                        log!(err "error caused by: {caused_by}");
                        let err =
                            ParseError::invalid_record_element_err(line!() as usize, other.loc());
                        self.errs.push(err);
                        self.next_expr();
                        debug_exit_info!(self);
                        return Err(());
                    }
                };
                let attr = RecordAttrOrIdent::Ident(ident);
                let record = self
                    .try_reduce_record(l_brace, attr)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(BraceContainer::Record(record))
            }
            other => {
                match self.peek_kind() {
                    Some(Colon) => {
                        let res = self
                            .try_reduce_normal_dict_or_refine_type(l_brace, other)
                            .map_err(|_| self.stack_dec(fn_name!()))?;
                        debug_exit_info!(self);
                        return Ok(res);
                    }
                    Some(Inclusion) => {
                        self.skip();
                        let mut generators = vec![];
                        let Expr::Accessor(Accessor::Ident(ident)) = other else {
                            let caused_by = caused_by!();
                            log!(err "error caused by: {caused_by}");
                            let err = ParseError::invalid_record_element_err(
                                line!() as usize,
                                other.loc(),
                            );
                            self.errs.push(err);
                            self.next_expr();
                            debug_exit_info!(self);
                            return Err(());
                        };
                        let expr = self
                            .try_reduce_expr(false, false, false, false)
                            .map_err(|_| self.stack_dec(fn_name!()))?;
                        generators.push((ident, expr));
                        let _ = expect_pop!(self, VBar);
                        let guard = self
                            .try_reduce_expr(false, false, false, false)
                            .map_err(|_| self.stack_dec(fn_name!()))?;
                        let r_brace = expect_pop!(self, fail_next RBrace);
                        debug_exit_info!(self);
                        let comp =
                            SetComprehension::new(l_brace, r_brace, None, generators, Some(guard));
                        return Ok(BraceContainer::Set(Set::Comprehension(comp)));
                    }
                    Some(VBar) => {
                        self.skip();
                        let mut generators = vec![];
                        loop {
                            let ident = self.try_reduce_ident()?;
                            let _ = expect_pop!(self, fail_next Inclusion);
                            let expr = self
                                .try_reduce_expr(false, false, false, false)
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            generators.push((ident, expr));
                            if self.cur_is(Semi) {
                                self.skip();
                            } else {
                                break;
                            }
                        }
                        let guard = if self.cur_is(VBar) {
                            self.skip();
                            let expr = self
                                .try_reduce_expr(false, false, false, false)
                                .map_err(|_| self.stack_dec(fn_name!()))?;
                            Some(expr)
                        } else {
                            None
                        };
                        let r_brace = expect_pop!(self, fail_next RBrace);
                        debug_exit_info!(self);
                        let comp =
                            SetComprehension::new(l_brace, r_brace, Some(other), generators, guard);
                        return Ok(BraceContainer::Set(Set::Comprehension(comp)));
                    }
                    Some(RBrace) => {
                        let arg = Args::new(vec![PosArg::new(other)], None, vec![], None, None);
                        let r_brace = self.lpop();
                        debug_exit_info!(self);
                        return Ok(BraceContainer::Set(Set::Normal(NormalSet::new(
                            l_brace, r_brace, arg,
                        ))));
                    }
                    _ => {}
                }
                let set = self
                    .try_reduce_set(l_brace, other)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
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
                    match self.peek() {
                        Some(t) if t.is(Semi) => {
                            let err = self.skip_and_throw_invalid_seq_err(
                                caused_by!(),
                                line!() as usize,
                                &["}", "element"],
                                Semi,
                            );
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                        Some(_) => {}
                        None => {
                            self.errs.push(self.unexpected_none(line!(), caused_by!()));
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                Some(Dedent) => {
                    self.skip();
                    let r_brace = expect_pop!(self, fail_next RBrace);
                    debug_exit_info!(self);
                    return Ok(Record::new_mixed(l_brace, r_brace, attrs));
                }
                Some(RBrace) => {
                    let r_brace = self.lpop();
                    debug_exit_info!(self);
                    return Ok(Record::new_mixed(l_brace, r_brace, attrs));
                }
                Some(_) => {
                    let next = self.try_reduce_chunk(false, false).map_err(|_| {
                        if let Some(err) = self.errs.last_mut() {
                            err.set_hint(switch_lang!(
                                "japanese" => "予期: 属性",
                                "simplified_chinese" => "期望: 属性",
                                "traditional_chinese" => "期望: 屬性",
                                "english" => "expect: an attribute",
                            ))
                        }
                        self.stack_dec(fn_name!())
                    })?;
                    match next {
                        Expr::Def(def) => {
                            if attrs.iter().any(|attr| {
                                attr.ident()
                                    .zip(def.sig.ident())
                                    .is_some_and(|(l, r)| l == r)
                            }) {
                                self.warns.push(ParseError::duplicate_elem_warning(
                                    line!() as usize,
                                    def.sig.loc(),
                                    def.sig.to_string(),
                                ));
                            }
                            attrs.push(RecordAttrOrIdent::Attr(def));
                        }
                        Expr::Accessor(acc) => {
                            let ident = match acc {
                                Accessor::Ident(ident) => ident,
                                other => {
                                    let err = ParseError::invalid_record_element_err(
                                        line!() as usize,
                                        other.loc(),
                                    );
                                    self.errs.push(err);
                                    debug_exit_info!(self);
                                    return Err(());
                                }
                            };
                            attrs.push(RecordAttrOrIdent::Ident(ident));
                        }
                        other => {
                            let caused_by = caused_by!();
                            log!(err "error caused by: {caused_by}");
                            let err = ParseError::invalid_record_element_err(
                                line!() as usize,
                                other.loc(),
                            );
                            self.errs.push(err);
                            self.next_expr();
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                None => {
                    self.errs.push(self.unexpected_none(line!(), caused_by!()));
                    debug_exit_info!(self);
                    return Err(());
                }
            }
        }
    }

    fn try_reduce_normal_dict_or_refine_type(
        &mut self,
        l_brace: Token,
        lhs: Expr,
    ) -> ParseResult<BraceContainer> {
        debug_call_info!(self);
        let _colon = expect_pop!(self, fail_next Colon);
        let rhs = self
            .try_reduce_expr(false, true, false, false)
            .map_err(|_| self.stack_dec(fn_name!()))?;
        if self.cur_is(VBar) {
            self.skip();
            let Expr::Accessor(Accessor::Ident(var)) = lhs else {
                let err = ParseError::simple_syntax_error(line!() as usize, lhs.loc());
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            };
            let generators = vec![(var, rhs)];
            let guard = self
                .try_reduce_chunk(false, false)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            let r_brace = expect_pop!(self, fail_next RBrace);
            let set_comp = SetComprehension::new(l_brace, r_brace, None, generators, Some(guard));
            debug_exit_info!(self);
            Ok(BraceContainer::Set(Set::Comprehension(set_comp)))
        } else {
            let dict = self
                .try_reduce_normal_dict(l_brace, lhs, rhs)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            debug_exit_info!(self);
            Ok(BraceContainer::Dict(Dict::Normal(dict)))
        }
    }

    fn try_reduce_normal_dict(
        &mut self,
        l_brace: Token,
        first_key: Expr,
        value: Expr,
    ) -> ParseResult<NormalDict> {
        debug_call_info!(self);
        let mut kvs = vec![KeyValue::new(first_key, value)];
        loop {
            match self.peek_kind() {
                Some(Comma) => {
                    self.skip();
                    match self.peek_kind() {
                        Some(Comma) => {
                            let err = self.skip_and_throw_invalid_seq_err(
                                caused_by!(),
                                line!() as usize,
                                &["}", "element"],
                                Comma,
                            );
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                        Some(RBrace) => {
                            let dict = NormalDict::new(l_brace, self.lpop(), kvs);
                            debug_exit_info!(self);
                            return Ok(dict);
                        }
                        Some(Newline) => {
                            self.skip();
                        }
                        _ => {}
                    }
                    let key = self
                        .try_reduce_expr(false, false, true, false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    expect_pop!(self, fail_next Colon);
                    let value = self.try_reduce_chunk(false, false).map_err(|_| {
                        if let Some(err) = self.errs.last_mut() {
                            err.set_hint(switch_lang!(
                                "japanese" => "予期: キー",
                                "simplified_chinese" => "期望: 关键",
                                "traditional_chinese" => "期望: 關鍵",
                                "english" => "expect: key",
                            ))
                        }
                        self.stack_dec(fn_name!())
                    })?;
                    kvs.push(KeyValue::new(key, value));
                }
                Some(Newline | Indent | Dedent) => {
                    self.skip();
                }
                Some(RBrace) => {
                    let dict = NormalDict::new(l_brace, self.lpop(), kvs);
                    debug_exit_info!(self);
                    return Ok(dict);
                }
                Some(_) => {
                    let caused_by = caused_by!();
                    log!(err "error caused by: {caused_by}");
                    let err = ParseError::unclosed_error(
                        line!() as usize,
                        self.lpop().loc(),
                        "}",
                        "dict",
                    );
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
                _ => break,
            }
        }
        let caused_by = caused_by!();
        log!(err "error caused by: {caused_by}");
        debug_exit_info!(self);
        Err(())
    }

    fn try_reduce_set(&mut self, l_brace: Token, first_elem: Expr) -> ParseResult<Set> {
        debug_call_info!(self);
        if self.cur_is(Semi) {
            match first_elem {
                Expr::Accessor(_) => {}
                other => {
                    let err = ParseError::expect_type_specified(line!() as usize, other.loc());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
            }
            self.skip();
            let len = self
                .try_reduce_expr(false, false, false, false)
                .map_err(|_| {
                    if let Some(err) = self.errs.last_mut() {
                        err.set_hint(switch_lang!(
                            "japanese" => "予期: }か要素",
                            "simplified_chinese" => "期望: }或元素",
                            "traditional_chinese" => "期望: }或元素",
                            "english" => "expect: } or element",
                        ))
                    }
                    self.stack_dec(fn_name!())
                })?;
            let r_brace = self.lpop();
            if !r_brace.is(RBrace) {
                let err = self.skip_and_throw_invalid_unclosed_err(
                    caused_by!(),
                    line!(),
                    "}",
                    "set type specification",
                );
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            }
            return Ok(Set::WithLength(SetWithLength::new(
                l_brace,
                r_brace,
                PosArg::new(first_elem),
                len,
            )));
        }
        let mut args = Args::single(PosArg::new(first_elem));
        loop {
            match self.peek_kind() {
                Some(Comma) => {
                    self.skip();
                    match self.peek_kind() {
                        Some(Comma) => {
                            let err = self.skip_and_throw_invalid_seq_err(
                                caused_by!(),
                                line!() as usize,
                                &["}", "element"],
                                Comma,
                            );
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                        Some(RBrace) => {
                            let set = Set::Normal(NormalSet::new(l_brace, self.lpop(), args));
                            debug_exit_info!(self);
                            return Ok(set);
                        }
                        Some(Newline | Indent | Dedent) => {
                            self.skip();
                        }
                        _ => {}
                    }
                    match self
                        .try_reduce_arg(false)
                        .map_err(|_| self.stack_dec(fn_name!()))?
                    {
                        ArgKind::Pos(arg) => match arg.expr {
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
                        ArgKind::Var(var) | ArgKind::KwVar(var) => {
                            let err = ParseError::simple_syntax_error(line!() as usize, var.loc());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                        ArgKind::Kw(arg) => {
                            let err = ParseError::simple_syntax_error(line!() as usize, arg.loc());
                            self.errs.push(err);
                            debug_exit_info!(self);
                            return Err(());
                        }
                    }
                }
                Some(Newline | Indent | Dedent) => {
                    self.skip();
                }
                Some(RBrace) => {
                    let set = Set::Normal(NormalSet::new(l_brace, self.lpop(), args));
                    debug_exit_info!(self);
                    return Ok(set);
                }
                Some(other) => {
                    let err = self.skip_and_throw_invalid_seq_err(
                        caused_by!(),
                        line!() as usize,
                        &["}", "element"],
                        other,
                    );
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
                None => {
                    self.errs.push(self.unexpected_none(line!(), caused_by!()));
                    debug_exit_info!(self);
                    return Err(());
                }
            }
        }
    }

    fn try_reduce_nonempty_tuple(
        &mut self,
        first_elem: ArgKind,
        line_break: bool,
    ) -> ParseResult<Tuple> {
        debug_call_info!(self);
        let mut args = match first_elem {
            ArgKind::Pos(pos) => Args::single(pos),
            ArgKind::Var(var) => Args::new(vec![], Some(var), vec![], None, None),
            ArgKind::Kw(kw) => Args::new(vec![], None, vec![kw], None, None),
            ArgKind::KwVar(kw_var) => Args::new(vec![], None, vec![], Some(kw_var), None),
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
                        let err = self.skip_and_throw_invalid_seq_err(
                            caused_by!(),
                            line!() as usize,
                            &[")", "element"],
                            Comma,
                        );
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    } else if self.cur_is(Dedent) || self.cur_is(RParen) {
                        break;
                    }
                    match self
                        .try_reduce_arg(false)
                        .map_err(|_| self.stack_dec(fn_name!()))?
                    {
                        ArgKind::Pos(arg) if args.kw_is_empty() && args.var_args.is_none() => {
                            match arg.expr {
                                Expr::Tuple(Tuple::Normal(tup)) if tup.elems.paren.is_none() => {
                                    args.extend_pos(tup.elems.into_iters().0);
                                }
                                other => {
                                    args.push_pos(PosArg::new(other));
                                }
                            }
                        }
                        ArgKind::Var(var) => {
                            args.set_var_args(var);
                        }
                        ArgKind::Pos(arg) => {
                            let err = ParseError::syntax_error(
                                line!() as usize,
                                arg.loc(),
                                switch_lang!(
                                    "japanese" => "非デフォルト引数はデフォルト引数の後に指定できません",
                                    "simplified_chinese" => "不能在默认参数之后指定非默认参数",
                                    "traditional_chinese" => "不能在默認參數之後指定非默認參數",
                                    "english" => "Non-default arguments cannot be specified after default arguments",
                                ),
                                None,
                            );
                            self.errs.push(err);
                            self.next_expr();
                            debug_exit_info!(self);
                            return Err(());
                        }
                        // e.g. (x, y:=1) -> ...
                        // Syntax error will occur when trying to use it as a tuple
                        ArgKind::Kw(arg) => {
                            args.push_kw(arg);
                        }
                        ArgKind::KwVar(arg) => {
                            args.set_kw_var(arg);
                        }
                    }
                }
                Some(_other) => {
                    break;
                }
                None => {
                    self.errs.push(self.unexpected_none(line!(), caused_by!()));
                    debug_exit_info!(self);
                    return Err(());
                }
            }
        }
        let tup = Tuple::Normal(NormalTuple::new(args));
        debug_exit_info!(self);
        Ok(tup)
    }

    #[inline]
    fn try_reduce_lit(&mut self) -> ParseResult<Literal> {
        debug_call_info!(self);
        match self.peek() {
            Some(t) if t.category_is(TC::Literal) => Ok(Literal::from(self.lpop())),
            Some(other) => {
                let caused_by = caused_by!();
                log!(err "error caused by: {caused_by}");
                let err = ParseError::unexpected_token_error(
                    line!() as usize,
                    other.loc(),
                    &other.inspect()[..],
                );
                self.errs.push(err);
                debug_exit_info!(self);
                Err(())
            }
            None => {
                self.errs.push(self.unexpected_none(line!(), caused_by!()));
                debug_exit_info!(self);
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
        let mut expr = Expr::Literal(Literal::from(left));
        loop {
            match self.peek() {
                Some(l) if l.is(StrInterpRight) => {
                    let mut right = self.lpop();
                    right.content =
                        Str::from(format!("\"{}", right.content.trim_start_matches('}')));
                    right.kind = StrLit;
                    let right = Expr::Literal(Literal::from(right));
                    let op = Token::new_fake(
                        Plus,
                        "+",
                        right.ln_begin().unwrap(),
                        right.col_begin().unwrap(),
                        right.col_end().unwrap(),
                    );
                    expr = Expr::BinOp(BinOp::new(op, expr, right));
                    debug_exit_info!(self);
                    return Ok(expr);
                }
                Some(t) if t.is(EOF) => {
                    let caused_by = caused_by!();
                    log!(err "error caused by: {caused_by}");
                    let err = ParseError::syntax_error(
                        line!() as usize,
                        expr.loc(),
                        switch_lang!(
                            "japanese" => "文字列補間の終わりが見つかりませんでした",
                            "simplified_chinese" => "未找到字符串的结束插值",
                            "traditional_chinese" => "未找到字符串的結束插值",
                            "english" => "end of a string interpolation not found",
                        ),
                        None,
                    );
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
                Some(_) => {
                    let mid_expr = self
                        .try_reduce_expr(true, false, false, false)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    let str_func = Expr::local(
                        "str",
                        mid_expr.ln_begin().unwrap(),
                        mid_expr.col_begin().unwrap(),
                        mid_expr.col_end().unwrap(),
                    );
                    let call = Call::new(str_func, None, Args::single(PosArg::new(mid_expr)));
                    let op = Token::new_fake(
                        Plus,
                        "+",
                        call.ln_begin().unwrap(),
                        call.col_begin().unwrap(),
                        call.col_end().unwrap(),
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
                        let mid = Expr::Literal(Literal::from(mid));
                        let op = Token::new_fake(
                            Plus,
                            "+",
                            mid.ln_begin().unwrap(),
                            mid.col_begin().unwrap(),
                            mid.col_end().unwrap(),
                        );
                        expr = Expr::BinOp(BinOp::new(op, expr, mid));
                    }
                }
                None => {
                    self.errs.push(self.unexpected_none(line!(), caused_by!()));
                    debug_exit_info!(self);
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
            let caused_by = caused_by!();
            log!(err "error caused by: {caused_by}");
            let err = ParseError::compiler_bug(0, op.loc(), fn_name!(), line!());
            self.errs.push(err);
            debug_exit_info!(self);
            return Err(());
        }

        if matches!(self.peek_kind(), Some(Dot)) {
            // obj |> .method(...)
            let vis = self.lpop();
            match self.lpop() {
                symbol if symbol.is(Symbol) => {
                    let Some(ExprOrOp::Expr(obj)) = stack.pop() else {
                        let err = self.skip_and_throw_syntax_err(line!(), caused_by!());
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    };
                    if let Some(args) = self
                        .opt_reduce_args(false)
                        .transpose()
                        .map_err(|_| self.stack_dec(fn_name!()))?
                    {
                        let ident =
                            Identifier::new(VisModifierSpec::Public(vis), VarName::new(symbol));
                        let mut call = Expr::Call(Call::new(obj, Some(ident), args));
                        while let Some(res) = self.opt_reduce_args(false) {
                            let args = res.map_err(|_| self.stack_dec(fn_name!()))?;
                            call = call.call_expr(args);
                        }
                        stack.push(ExprOrOp::Expr(call));
                    } else {
                        let err = self.get_stream_op_syntax_error(
                            line!() as usize,
                            obj.loc(),
                            caused_by!(),
                        );
                        self.errs.push(err);
                        debug_exit_info!(self);
                        return Err(());
                    }
                }
                other => {
                    let caused_by = caused_by!();
                    log!(err "error caused by: {caused_by}");
                    let err = ParseError::expect_method_error(line!() as usize, other.loc());
                    self.errs.push(err);
                    debug_exit_info!(self);
                    return Err(());
                }
            }
        } else {
            let expect_call = self
                .try_reduce_call_or_acc(false)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            let Expr::Call(mut call) = expect_call else {
                let caused_by = caused_by!();
                log!(err "error caused by: {caused_by}");
                let err = self.get_stream_op_syntax_error(
                    line!() as usize,
                    expect_call.loc(),
                    caused_by!(),
                );
                self.errs.push(err);
                debug_exit_info!(self);
                return Err(());
            };
            let ExprOrOp::Expr(first_arg) = stack.pop().unwrap() else {
                let caused_by = caused_by!();
                log!(err "error caused by: {caused_by}");
                self.errs.push(ParseError::compiler_bug(
                    line!() as usize,
                    call.loc(),
                    fn_name!(),
                    line!(),
                ));
                debug_exit_info!(self);
                return Err(());
            };
            call.args.insert_pos(0, PosArg::new(first_arg));
            stack.push(ExprOrOp::Expr(Expr::Call(call)));
        }
        debug_exit_info!(self);
        Ok(())
    }

    fn try_reduce_macro_arg(&mut self, arg: &MacroArgSpec) -> ParseResult<crate::ast::MacroArg> {
        debug_call_info!(self);
        match arg {
            MacroArgSpec::Expr => {
                let expr = self
                    .try_reduce_expr(true, false, false, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(crate::ast::MacroArg::Expr(expr))
            }
            MacroArgSpec::Block => {
                expect_pop!(self, Colon);
                let block = self
                    .try_reduce_block()
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(crate::ast::MacroArg::Block(block))
            }
            // TODO:
            MacroArgSpec::Name => {
                let name = self
                    .try_reduce_expr(true, false, false, false)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                debug_exit_info!(self);
                Ok(crate::ast::MacroArg::Expr(name))
            }
            MacroArgSpec::WithPrefix(kw, arg) => {
                self.skip_newlines_until_next_keyword(set! {kw.clone()});
                if self.peek().is_some_and(|tok| &tok.content == kw) {
                    let kw = self.lpop();
                    let arg = self
                        .try_reduce_macro_arg(arg)
                        .map_err(|_| self.stack_dec(fn_name!()))?;
                    Ok(crate::ast::MacroArg::with_prefix(kw, arg))
                } else {
                    todo!("{kw} / {:?}", self.peek())
                }
            }
        }
    }

    fn skip_newlines_until_next_keyword(&mut self, possible_keys: HashSet<Str>) {
        let mut newlines = vec![];
        while self.cur_is(Newline) {
            newlines.push(self.lpop());
        }
        if !self
            .peek()
            .is_some_and(|tok| possible_keys.contains(tok.inspect()))
        {
            for newline in newlines {
                self.tokens.push_front(newline);
            }
        }
    }

    fn try_reduce_macro_call(&mut self, mac: MacroInfo) -> ParseResult<Expr> {
        debug_call_info!(self);
        let name = VarName::new(self.lpop());
        self.keywords = [mac.keywords().collect(), mem::take(&mut self.keywords)].concat();
        let mut pos_args = vec![];
        for non_default in mac.non_defaults.iter() {
            let arg = self
                .try_reduce_macro_arg(non_default)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            pos_args.push(arg);
        }
        let mut var_args = vec![];
        if let Some(var_params) = mac.var_params {
            let key = var_params.get_prefix().map_or(set! {}, |s| set! {s});
            self.skip_newlines_until_next_keyword(key.clone());
            while self.peek().map_or(true, |tok| {
                Some(&tok.content) == var_params.get_prefix().as_ref()
            }) {
                let arg = self
                    .try_reduce_macro_arg(&var_params)
                    .map_err(|_| self.stack_dec(fn_name!()))?;
                var_args.push(arg);
                self.skip_newlines_until_next_keyword(key.clone());
            }
        }
        let mut kw_args = vec![];
        let keys = mac
            .defaults
            .iter()
            .flat_map(|arg| arg.get_prefix())
            .collect::<HashSet<_>>();
        let mut passed_keys = HashSet::new();
        self.skip_newlines_until_next_keyword(keys.clone());
        while let Some(default) = self.peek().and_then(|tok| {
            mac.defaults
                .iter()
                .find(|arg| tok.content == arg.get_prefix().unwrap())
        }) {
            let arg = self
                .try_reduce_macro_arg(default)
                .map_err(|_| self.stack_dec(fn_name!()))?;
            let pre = arg.get_prefix().unwrap();
            if passed_keys.contains(pre.inspect()) {
                let err = ParseError::duplicate_arg_error(
                    line!() as usize,
                    arg.loc(),
                    pre.content.to_string(),
                );
                self.errs.push(err);
            }
            passed_keys.insert(pre.content.clone());
            kw_args.push(arg);
            self.skip_newlines_until_next_keyword(keys.clone());
        }
        let args = MacroArgs::new(pos_args, var_args, kw_args);
        let macro_call = Expr::MacroCall(MacroCall::new(name, args));
        debug_exit_info!(self);
        Ok(macro_call)
    }
}

fn collect_last_binop_on_stack(stack: &mut Vec<ExprOrOp>) {
    let rhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
    let op = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Op:(_)));
    let lhs = enum_unwrap!(stack.pop(), Some:(ExprOrOp::Expr:(_)));
    let bin = BinOp::new(op, lhs, rhs);
    stack.push(ExprOrOp::Expr(Expr::BinOp(bin)));
}
