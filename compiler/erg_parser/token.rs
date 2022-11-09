//! defines `Token` (The minimum unit in the Erg source code that serves as input to the parser).
//!
//! Token(パーサーへの入力となる、Ergソースコードにおける最小単位)を定義する
use std::fmt;
use std::hash::{Hash, Hasher};

use erg_common::error::Location;
use erg_common::impl_displayable_stream_for_wrapper;
use erg_common::opcode311::BinOpCode;
use erg_common::str::Str;
use erg_common::traits::{Locational, Stream};
// use erg_common::ty::Type;
// use erg_common::typaram::OpKind;
// use erg_common::value::ValueObj;

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
    /// `ref` (special unary)
    RefOp,
    /// `ref!` (special unary)
    RefMutOp,
    /// =
    Equal,
    /// <-
    Inclusion,
    /// :=
    Walrus,
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
    /// , : :: :> <: . |> :=
    SpecialBinOp,
    /// =
    DefOp,
    /// -> =>
    LambdaOp,
    /// \n ;
    Separator,
    /// ^ &
    Reserved,
    /// @
    AtSign,
    /// |
    VBar,
    /// _
    UBar,
    BOF,
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
            NatLit | IntLit | RatioLit | StrLit | BoolLit | NoneLit | EllipsisLit | NoImplLit
            | InfLit => TokenCategory::Literal,
            PrePlus | PreMinus | PreBitNot | Mutate | RefOp | RefMutOp => TokenCategory::UnaryOp,
            Try => TokenCategory::PostfixOp,
            Comma | Colon | DblColon | SupertypeOf | SubtypeOf | Dot | Pipe | Walrus
            | Inclusion => TokenCategory::SpecialBinOp,
            Equal => TokenCategory::DefOp,
            FuncArrow | ProcArrow => TokenCategory::LambdaOp,
            Semi | Newline => TokenCategory::Separator,
            LParen | LBrace | LSqBr | Indent => TokenCategory::LEnclosure,
            RParen | RBrace | RSqBr | Dedent => TokenCategory::REnclosure,
            Caret | Amper => TokenCategory::Reserved,
            AtSign => TokenCategory::AtSign,
            VBar => TokenCategory::VBar,
            UBar => TokenCategory::UBar,
            BOF => TokenCategory::BOF,
            EOF => TokenCategory::EOF,
            Illegal => TokenCategory::Illegal,
            _ => TokenCategory::BinOp,
        }
    }

    pub const fn precedence(&self) -> Option<usize> {
        let prec = match self {
            Dot | DblColon => 200,                                    // .
            Pow => 190,                                               // **
            PrePlus | PreMinus | PreBitNot | RefOp | RefMutOp => 180, // (unary) + - * ~ ref ref!
            Star | Slash | FloorDiv | Mod | CrossOp | DotOp => 170,   // * / // % cross dot
            Plus | Minus => 160,                                      // + -
            Shl | Shr => 150,                                         // << >>
            BitAnd => 140,                                            // &&
            BitXor => 130,                                            // ^^
            BitOr => 120,                                             // ||
            Closed | LeftOpen | RightOpen | Open => 100,              // range operators
            Less | Gre | LessEq | GreEq | DblEq | NotEq | InOp | NotInOp | IsOp | IsNotOp => 90, // < > <= >= == != in notin is isnot
            AndOp => 80,                             // and
            OrOp => 70,                              // or
            FuncArrow | ProcArrow | Inclusion => 60, // -> => <-
            Colon | SupertypeOf | SubtypeOf => 50,   // : :> <:
            Comma => 40,                             // ,
            Equal | Walrus => 20,                    // = :=
            Newline | Semi => 10,                    // \n ;
            LParen | LBrace | LSqBr | Indent => 0,   // ( { [ Indent
            _ => return None,
        };
        Some(prec)
    }

    pub const fn is_right_associative(&self) -> bool {
        matches!(
            self,
            FuncArrow | ProcArrow | Equal /* | PreDollar | PreAt */
        )
    }

    pub const fn is_range_op(&self) -> bool {
        matches!(self, Closed | LeftOpen | RightOpen | Open)
    }
}

impl fmt::Display for TokenKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<TokenKind> for BinOpCode {
    fn from(tk: TokenKind) -> Self {
        match tk {
            Plus => BinOpCode::Add,
            Minus => BinOpCode::Subtract,
            Star => BinOpCode::Multiply,
            Slash => BinOpCode::TrueDivide,
            FloorDiv => BinOpCode::FloorDiv,
            Mod => BinOpCode::Remainder,
            Pow => BinOpCode::Power,
            BitAnd => BinOpCode::And,
            BitOr => BinOpCode::Or,
            BitXor => BinOpCode::Xor,
            Shl => BinOpCode::LShift,
            Shr => BinOpCode::RShift,
            _ => panic!("invalid token kind for binop"),
        }
    }
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

pub const COLON: Token = Token::dummy(TokenKind::Colon, ":");
pub const DOT: Token = Token::dummy(TokenKind::Dot, ".");
pub const EQUAL: Token = Token::dummy(TokenKind::Equal, "=");

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("kind", &self.kind)
            .field("content", &self.content.replace('\n', "\\n"))
            .field("lineno", &self.lineno)
            .field("col_begin", &self.col_begin)
            .finish()
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {}", self.kind, self.content.replace('\n', "\\n"))
    }
}

// the values of lineno and col are not relevant for comparison
impl PartialEq for Token {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.is(other.kind) && self.content == other.content
    }
}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.content.hash(state);
    }
}

impl Locational for Token {
    fn loc(&self) -> Location {
        if self.lineno == 0 {
            Location::Unknown
        } else {
            Location::range(
                self.lineno,
                self.col_begin,
                self.lineno,
                self.col_begin + self.content.len(),
            )
        }
    }

    #[inline]
    fn col_end(&self) -> Option<usize> {
        Some(self.col_begin + self.content.len())
    }
}

impl Token {
    pub const DUMMY: Token = Token {
        kind: TokenKind::Illegal,
        content: Str::ever("DUMMY"),
        lineno: 1,
        col_begin: 0,
    };

    pub const fn dummy(kind: TokenKind, content: &'static str) -> Self {
        Self {
            kind,
            content: Str::ever(content),
            lineno: 1,
            col_begin: 0,
        }
    }

    #[inline]
    pub fn new<S: Into<Str>>(kind: TokenKind, cont: S, lineno: usize, col_begin: usize) -> Self {
        Token {
            kind,
            content: cont.into(),
            lineno,
            col_begin,
        }
    }

    #[inline]
    pub fn from_str(kind: TokenKind, cont: &str) -> Self {
        Token {
            kind,
            content: Str::rc(cont),
            lineno: 0,
            col_begin: 0,
        }
    }

    #[inline]
    pub fn symbol(cont: &str) -> Self {
        Self::from_str(TokenKind::Symbol, cont)
    }

    #[inline]
    pub fn symbol_with_line(cont: &str, line: usize) -> Self {
        Token {
            kind: TokenKind::Symbol,
            content: Str::rc(cont),
            lineno: line,
            col_begin: 0,
        }
    }

    pub const fn static_symbol(s: &'static str) -> Self {
        Token {
            kind: TokenKind::Symbol,
            content: Str::ever(s),
            lineno: 0,
            col_begin: 0,
        }
    }

    pub const fn category(&self) -> TokenCategory {
        self.kind.category()
    }

    pub fn category_is(&self, category: TokenCategory) -> bool {
        self.kind.category() == category
    }

    pub fn is(&self, kind: TokenKind) -> bool {
        self.kind == kind
    }

    pub const fn is_block_op(&self) -> bool {
        self.category().is_block_op()
    }

    pub const fn inspect(&self) -> &Str {
        &self.content
    }

    pub fn is_procedural(&self) -> bool {
        self.inspect().ends_with('!')
    }
}

#[derive(Debug, Clone)]
pub struct TokenStream(Vec<Token>);

impl_displayable_stream_for_wrapper!(TokenStream, Token);
