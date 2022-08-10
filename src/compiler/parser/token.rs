//! defines `Token` (The minimum unit in the Erg source code that serves as input to the parser).
//!
//! Token(パーサーへの入力となる、Ergソースコードにおける最小単位)を定義する
use std::fmt;
use std::hash::{Hash, Hasher};

use common::str::Str;
use common::error::Location;
use common::impl_displayable_stream_for_wrapper;
use common::traits::{Stream, Locational};
use common::value::ValueObj;
use common::ty::Type;

/// 意味論的名前と記号自体の名前が混在しているが、Pythonの名残である
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TokenKind {
    /// e.g. i, p!, $s, T, `+`, `and`, 'd/dx'
    Symbol,
    // e.g. 0, 1
    NatLit,
    // e.g. -1, -2
    IntLit,
    RatioLit,
    BoolLit,
    StrLit,
    NoneLit,
    NoImplLit,
    EllipsisLit,
    InfLit,
    /// `+` (unary)
    PrePlus,
    /// `-` (unary)
    PreMinus,
    /// `*` (unary)
    PreStar,
    /// ~ (unary)
    PreBitNot,
    // PreAmp,    // & (unary)
    // PreAt,     // @ (unary)
    /// ! (unary)
    Mutate,
    /// ? (postfix)
    Try,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// /
    Slash,
    /// //
    FloorDiv,
    /// **
    Pow,
    /// %
    Mod,
    /// ..
    Closed,
    /// ..<
    RightOpen,
    /// <..
    LeftOpen,
    /// <..<
    Open,
    /// &&
    BitAnd,
    /// ||
    BitOr,
    /// ^^
    BitXor,
    /// <<
    Shl,
    /// >>
    Shr,
    /// <
    Less,
    /// >
    Gre,
    /// <=
    LessEq,
    /// >=
    GreEq,
    /// ==
    DblEq,
    /// !=
    NotEq,
    /// `in`
    InOp,
    /// `notin`
    NotInOp,
    /// `sub` (subtype of)
    SubOp,
    /// `is`
    IsOp,
    /// `isnot`
    IsNotOp,
    /// `and`
    AndOp,
    /// `or`
    OrOp,
    /// `dot` (scalar product)
    DotOp,
    /// `cross` (vector product)
    CrossOp,
    /// =
    Equal,
    /// |=
    OrEqual,
    /// ->
    FuncArrow,
    /// =>
    ProcArrow,
    /// (
    LParen,
    /// )
    RParen,
    /// [
    LSqBr,
    /// ]
    RSqBr,
    /// {
    LBrace,
    /// }
    RBrace,
    Indent,
    Dedent,
    /// .
    Dot,
    /// |>
    Pipe,
    /// :
    Colon,
    /// ::
    DblColon,
    /// :>
    SupertypeOf,
    /// <:
    SubtypeOf,
    /// ,
    Comma,
    /// ^
    Caret,
    /// &
    Amper,
    /// @
    AtSign,
    /// |
    VBar,
    /// _
    UBar,
    /// ...
    Spread,
    /// \n
    Newline,
    /// ;
    Semi,
    Illegal,
    /// Beginning Of File
    BOF,
    EOF,
}

use TokenKind::*;

impl From<TokenKind> for Type {
    #[inline]
    fn from(tok: TokenKind) -> Type {
        match tok {
            NatLit => Type::Nat,
            IntLit => Type::Int,
            RatioLit => Type::Float,
            StrLit => Type::Str,
            BoolLit => Type::Bool,
            NoneLit => Type::NoneType,
            NoImplLit => Type::NotImplemented,
            EllipsisLit => Type::Ellipsis,
            InfLit => Type::Inf,
            other => panic!("this has not type: {other}"),
        }
    }
}

impl From<&ValueObj> for TokenKind {
    fn from(c: &ValueObj) -> TokenKind {
        match c {
            ValueObj::Int(_) => TokenKind::IntLit,
            ValueObj::Nat(_) => TokenKind::NatLit,
            ValueObj::Float(_) => TokenKind::RatioLit,
            ValueObj::Str(_) => TokenKind::StrLit,
            ValueObj::True => TokenKind::BoolLit,
            ValueObj::False => TokenKind::BoolLit,
            ValueObj::None => TokenKind::NoneLit,
            ValueObj::Ellipsis => TokenKind::EllipsisLit,
            ValueObj::Inf => TokenKind::InfLit,
            _ => TokenKind::Illegal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Symbol,
    Literal,
    BinOp,
    UnaryOp,
    /// ? <.. ..
    PostfixOp,
    /// ( [ { Indent
    LEnclosure,
    /// ) } } Dedent
    REnclosure,
    /// , : :: :> <: . |> |=
    SpecialBinOp,
    /// =
    DefOp,
    /// -> =>
    LambdaOp,
    /// \n ;
    Separator,
    /// ^ (reserved)
    Caret,
    /// &
    Amper,
    /// @
    AtSign,
    /// |
    VBar,
    /// _
    UBar,
    EOF,
    Illegal,
}

impl TokenCategory {
    pub const fn is_block_op(&self) -> bool {
        matches!(self, Self::DefOp | Self::LambdaOp)
    }
}

impl TokenKind {
    pub const fn category(&self) -> TokenCategory {
        match self {
            Symbol => TokenCategory::Symbol,
            NatLit | IntLit | RatioLit | StrLit | BoolLit
            | NoneLit | EllipsisLit | NoImplLit | InfLit => TokenCategory::Literal,
            PrePlus | PreMinus | PreStar | PreBitNot | Mutate => TokenCategory::UnaryOp,
            Try => TokenCategory::PostfixOp,
            Comma | Colon | DblColon | SupertypeOf | SubtypeOf | Dot | Pipe | OrEqual => TokenCategory::SpecialBinOp,
            Equal => TokenCategory::DefOp,
            FuncArrow | ProcArrow => TokenCategory::LambdaOp,
            Semi | Newline => TokenCategory::Separator,
            LParen | LBrace | LSqBr | Indent => TokenCategory::LEnclosure,
            RParen | RBrace | RSqBr | Dedent => TokenCategory::REnclosure,
            Caret => TokenCategory::Caret,
            Amper => TokenCategory::Amper,
            AtSign => TokenCategory::AtSign,
            VBar => TokenCategory::VBar,
            UBar => TokenCategory::UBar,
            EOF => TokenCategory::EOF,
            Illegal | BOF => TokenCategory::Illegal,
            _ => TokenCategory::BinOp,
        }
    }

    pub const fn precedence(&self) -> Option<usize> {
        let prec = match self {
            Dot | DblColon => 200, // .
            Pow => 190, // **
            PrePlus | PreMinus | PreBitNot  => 180, // (unary) + - * ~
            Star | Slash | FloorDiv | Mod | CrossOp | DotOp => 170, // * / // % cross dot
            Plus | Minus => 160, // + -
            Shl | Shr => 150, // << >>
            BitAnd => 140, // &&
            BitXor => 130, // ^^
            BitOr => 120, // ||
            Closed | LeftOpen | RightOpen | Open => 100, // range operators
            Less | Gre | LessEq | GreEq | DblEq | NotEq
            | InOp | NotInOp | IsOp | IsNotOp => 90, // < > <= >= == != in notin is isnot
            AndOp => 80, // and
            OrOp => 70, // or
            FuncArrow | ProcArrow => 60, // -> =>
            Colon | SupertypeOf | SubtypeOf => 50, // : :> <:
            Comma => 40, // ,
            Equal | OrEqual => 20, // = |=
            Newline | Semi => 10, // \n ;
            LParen | LBrace | LSqBr | Indent => 0, // ( { [ Indent
            _ => { return None },
        };
        Some(prec)
    }

    pub const fn is_right_associative(&self) -> bool {
        match self {
            FuncArrow | ProcArrow | Equal => true,
            // PreDollar | PreAt => true,
            _ => false,
        }
    }
}

impl fmt::Display for TokenKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{self:?}") }
}

#[derive(Clone, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub content: Str,
    /// 1 origin
    // TODO: 複数行文字列リテラルもあるのでタプルにするのが妥当?
    pub lineno: usize,
    /// a pointer from which the token starts (0 origin)
    pub col_begin: usize,
}

impl From<Token> for ValueObj {
    #[inline]
    fn from(tok: Token) -> ValueObj {
        ValueObj::from_str(Type::from(tok.kind), tok.content)
    }
}

impl From<&Token> for ValueObj {
    #[inline]
    fn from(tok: &Token) -> ValueObj {
        ValueObj::from_str(Type::from(tok.kind), tok.content.clone())
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("kind", &self.kind)
            .field("content", &self.content.replace("\n", "\\n"))
            .field("lineno", &self.lineno)
            .field("col_begin", &self.col_begin)
            .finish()
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {}", self.kind, self.content.replace("\n", "\\n"))
    }
}

// the values of lineno and col are not relevant for comparison
impl PartialEq for Token {
    #[inline]
    fn eq(&self, other: &Self) -> bool { self.is(other.kind) && self.content == other.content }
}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.content.hash(state);
    }
}

impl Locational for Token {
    fn loc(&self) -> Location {
        if self.lineno == 0 { Location::Unknown } else {
            Location::range(
                self.lineno,
                self.col_begin,
                self.lineno,
                self.col_begin + self.content.len(),
            )
        }
    }

    #[inline]
    fn col_end(&self) -> Option<usize> { Some(self.col_begin + self.content.len()) }
}

impl Token {
    #[inline]
    pub fn dummy() -> Self {
        Token{ kind: TokenKind::Illegal, content: "DUMMY".into(), lineno: 1, col_begin: 0 }
    }

    #[inline]
    pub fn new<S: Into<Str>>(kind: TokenKind, cont: S, lineno: usize, col_begin: usize) -> Self {
        Token{ kind, content: cont.into(), lineno, col_begin }
    }

    #[inline]
    pub fn from_str(kind: TokenKind, cont: &str) -> Self {
        Token{ kind, content: Str::rc(cont), lineno: 0, col_begin: 0 }
    }

    #[inline]
    pub fn symbol(cont: &str) -> Self { Self::from_str(TokenKind::Symbol, cont) }

    pub const fn static_symbol(s: &'static str) -> Self {
        Token{ kind: TokenKind::Symbol, content: Str::ever(s), lineno: 0, col_begin: 0 }
    }

    pub const fn category(&self) -> TokenCategory { self.kind.category() }

    pub fn category_is(&self, category: TokenCategory) -> bool { self.kind.category() == category }

    pub fn is(&self, kind: TokenKind) -> bool { self.kind == kind }

    pub const fn is_block_op(&self) -> bool { self.category().is_block_op() }

    pub const fn inspect(&self) -> &Str { &self.content }

    pub fn is_procedural(&self) -> bool { self.inspect().ends_with("!") }
}

#[derive(Debug, Clone)]
pub struct TokenStream(Vec<Token>);

impl_displayable_stream_for_wrapper!(TokenStream, Token);
