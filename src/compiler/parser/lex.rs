//! defines and implements `Lexer` (Tokenizer).
use common::cache::Cache;
use common::Str;
use common::{fn_name_full, switch_lang, debug_power_assert, normalize_newline};
use common::config::Input;
use common::config::ErgConfig;
use common::traits::{Locational, Runnable, Stream};

use crate::error::{LexerRunnerError, LexerRunnerErrors, LexError, LexErrors, LexResult};
use crate::token::{Token, TokenCategory, TokenKind, TokenStream};
use TokenKind::*;

/// Lexerは使い捨てなので、Runnerを用意
pub struct LexerRunner {
    cfg: ErgConfig,
}

impl Runnable for LexerRunner {
    type Err = LexerRunnerError;
    type Errs = LexerRunnerErrors;

    #[inline]
    fn new(cfg: ErgConfig) -> Self { Self { cfg } }

    #[inline]
    fn input(&self) -> &Input { &self.cfg.input }

    #[inline]
    fn start_message(&self) -> String { "Erg lexer\n".to_string() }

    #[inline]
    fn clear(&mut self) {}

    fn eval(&mut self, src: Str) -> Result<String, LexerRunnerErrors> {
        let lexer = Lexer::from_str(src);
        if cfg!(feature = "debug") {
            let ts = lexer.lex().map_err(|errs| LexerRunnerErrors::convert(self.input(), errs))?;
            println!("{ts}");
            Ok(ts.to_string())
        } else {
            Ok(lexer.lex().map_err(|errs| LexerRunnerErrors::convert(self.input(), errs))?.to_string())
        }
    }
}

/// Lexes a source code and iterates tokens.
///
/// This can be used as an iterator or to generate a `TokenStream`.
#[derive(Debug)]
pub struct Lexer /*<'a>*/ {
    str_cache: Cache<str>,
    chars: Vec<char>,
    indent_stack: Vec<usize>,
    /// indicates the position in the entire source code
    cursor: usize,
    /// to determine the type of operators, etc.
    prev_token: Token,
    /// 0-origin, but Token.lineno will 1-origin
    lineno_token_starts: usize,
    /// 0-origin, indicates the column number in which the token appears
    col_token_starts: usize,
}

impl Lexer /*<'a>*/ {
    pub fn new(input: Input) -> Self {
        let normed = normalize_newline(&input.read());
        Lexer {
            str_cache: Cache::new(),
            chars: normed.chars().collect::<Vec<char>>(),
            indent_stack: vec![],
            cursor: 0,
            prev_token: Token::new(TokenKind::BOF, "", 0, 0),
            lineno_token_starts: 0,
            col_token_starts: 0,
        }
    }

    pub fn from_str(src: Str) -> Self {
        let escaped = normalize_newline(&src);
        Lexer {
            str_cache: Cache::new(),
            chars: escaped.chars().collect::<Vec<char>>(),
            indent_stack: vec![],
            cursor: 0,
            prev_token: Token::new(TokenKind::BOF, "", 0, 0),
            lineno_token_starts: 0,
            col_token_starts: 0,
        }
    }

    pub fn lex(self) -> Result<TokenStream, LexErrors> {
        let mut result = TokenStream::empty();
        let mut errs = LexErrors::empty();
        for i in self.into_iter() {
            match i {
                Ok(token) => { result.push(token) }
                Err(err) => { errs.push(err); }
            }
        }
        if errs.is_empty() { Ok(result) } else { Err(errs) }
    }

    fn emit_token(&mut self, kind: TokenKind, cont: &str) -> Token {
        let cont = self.str_cache.get(cont);
        // cannot use String::len() for multi-byte characters
        let cont_len = cont.chars().count();
        let token = Token::new(kind, cont, self.lineno_token_starts + 1, self.col_token_starts);
        self.prev_token = token.clone();
        self.col_token_starts += cont_len;
        token
    }

    #[inline]
    fn accept(&mut self, kind: TokenKind, cont: &str) -> Option<LexResult<Token>> {
        Some(Ok(self.emit_token(kind, cont)))
    }

    fn deny_feature(&mut self, cont: &str, feat_name: &str) -> Option<LexResult<Token>> {
        let token = self.emit_token(Illegal, cont);
        Some(Err(LexError::feature_error(0, token.loc(), feat_name)))
    }

    const fn is_valid_symbol_ch(c: char) -> bool {
        match c {
            '0'..='9' => true,
            // control characters
            '\0' | '\u{0009}'..='\u{001F}' => false,
            // white spaces
            ' ' | '\u{00A0}' => false,
            '\u{007F}' | '\u{0085}' | '\u{05C1}' | '\u{05C2}' => false,
            '\u{0701}'..='\u{070d}' => false,
            '\u{07B2}'..='\u{07BF}' => false,
            '\u{1680}' | '\u{180E}' => false,
            '\u{2000}'..='\u{200F}' => false,
            '\u{2028}'..='\u{202F}' => false,
            '\u{205F}'..='\u{206F}' => false,
            '\u{3000}' | '\u{3164}' | '\u{FEFF}' => false,
            // operator characters + special markers
            '<' | '>' | '$' | '%' | '.' | ',' | ':' | ';' | '+' | '-' | '*' | '/' | '=' | '#'
            | '&' | '|' | '^' | '~' | '@' | '!' | '?' | '\\' => false,
            // enclosures
            '[' | ']' | '(' | ')' | '{' | '}' | '\"' | '\'' | '`' => false,
            _ => true,
        }
    }

    /// Detect `c` is a bidirectional overriding character.
    /// [CVE-2021-42574: homoglyph atack](https://blog.rust-lang.org/2021/11/01/cve-2021-42574.html) countermeasures.
    pub fn is_bidi(c: char) -> bool {
        match c {
            '\u{200F}' | '\u{202B}' | '\u{202E}' | '\u{2067}' => true,
            _ => false,
        }
    }

    #[inline]
    fn is_definable_operator(s: &str) -> bool {
        match s {
            "+" | "-" | "*" | "/" | "//" | "**" | "%" | ".." | "..=" | "~" | "&&" | "||" | "^^"
            | ">>" | "<<" | "==" | "!=" | ">" | "<" | ">=" | "<="
            | "dot" | "cross" => true,
            _ => false,
        }
    }

    // +, -, * etc. may be pre/bin
    // and, or, is, isnot, in, notin, as, dot, cross may be bin/function
    const fn is_bin_position(&self) -> Option<bool> {
        match self.prev_token.category() {
            // unary: `[ +`, `= +`, `+ +`, `, +`, `:: +`
            TokenCategory::LEnclosure
            | TokenCategory::BinOp
            | TokenCategory::UnaryOp
            | TokenCategory::Separator
            | TokenCategory::SpecialBinOp
            | TokenCategory::DefOp
            | TokenCategory::LambdaOp => Some(false),
            // bin: `] +`, `1 +`, `true and[true]`
            TokenCategory::REnclosure | TokenCategory::Literal => Some(true),
            // bin: `fn +1`
            // NOTE: if semantic analysis shows `fn` is a function, should this be rewritten to be unary?
            TokenCategory::Symbol => Some(true),
            _ => None,
        }
    }

    fn is_zero(s: &str) -> bool { s.replace("-0", "").replace("0", "").is_empty() }

    /// emit_tokenで一気にcol_token_startsを移動させるのでここでは移動させない
    fn consume(&mut self) -> Option<char> {
        let now = self.cursor;
        self.cursor += 1;
        self.chars.get(now).map(|x| *x)
    }

    fn peek_prev_ch(&self) -> Option<char> {
        if self.cursor == 0 {
            None
        } else {
            self.chars.get(self.cursor - 1).map(|x| *x)
        }
    }

    #[inline]
    fn peek_cur_ch(&self) -> Option<char> {
        self.chars.get(self.cursor).map(|x| *x)
    }

    #[inline]
    fn peek_next_ch(&self) -> Option<char> {
        self.chars.get(self.cursor + 1).map(|x| *x)
    }

    fn lex_comment(&mut self) -> LexResult<()> {
        // debug_power_assert!(self.consume(), ==, Some('#'));
        let mut s = "".to_string();
        while self.peek_cur_ch().map(|cur| cur != '\n').unwrap_or(false) {
            if Self::is_bidi(self.peek_cur_ch().unwrap()) {
                let comment = self.emit_token(Illegal, &s);
                return Err(LexError::syntax_error(0, comment.loc(), switch_lang!(
                    "invalid unicode character (bi-directional override) in comments",
                    "不正なユニコード文字(双方向オーバーライド)がコメント中に使用されています"
                ), None))
            }
            s.push(self.consume().unwrap());
        }
        Ok(())
    }

    fn lex_space_indent_dedent(&mut self) -> Option<LexResult<Token>> {
        let is_toplevel = self.cursor > 0
            && !self.indent_stack.is_empty()
            && self.peek_prev_ch() == Some('\n')
            && self.peek_cur_ch() != Some(' ');
        if is_toplevel {
            let dedent = self.emit_token(Dedent, "");
            self.indent_stack.pop();
            self.col_token_starts = 0;
            return Some(Ok(dedent))
        }
        let mut spaces = "".to_string();
        while let Some(' ') = self.peek_cur_ch() {
            spaces.push(self.consume().unwrap());
        }
        // indent in the first line: error
        if !spaces.is_empty() && self.cursor == 0 {
            let space = self.emit_token(Illegal, &spaces);
            Some(Err(LexError::syntax_error(0, space.loc(), switch_lang!(
                "invalid indent",
                "インデントが不正です"
            ), None)))
        } else if self.prev_token.is(Newline) {
            self.lex_indent_dedent(spaces)
        } else {
            self.col_token_starts += spaces.len();
            None
        }
    }

    /// The semantic correctness of the use of indent/dedent will be analyzed with `Parser`
    fn lex_indent_dedent(&mut self, spaces: String) -> Option<LexResult<Token>> {
        // same as the CPython's limit
        if spaces.len() > 100 {
            let token = self.emit_token(Indent, &spaces);
            return Some(Err(LexError::syntax_error(0, token.loc(), switch_lang!(
                "indentation is too deep",
                "インデントが深すぎます"
            ), Some(switch_lang!(
                "The code is too complicated. Please split the process.",
                "コードが複雑すぎます。処理を分割してください"
            ).into()))))
        }
        // ignore indents if the current line is a comment
        if let Some('#') = self.peek_cur_ch() {
            if let Err(e) = self.lex_comment() { return Some(Err(e)) }
        }
        let mut is_valid_dedent = false;
        let calc_indent_and_validate = |sum: usize, x: &usize| {
            if sum + *x == spaces.len() { is_valid_dedent = true; }
            sum + *x
        };
        let sum_indent = self.indent_stack.iter().fold(0, calc_indent_and_validate);
        if sum_indent < spaces.len() {
            let indent_len = spaces.len() - sum_indent;
            self.col_token_starts += sum_indent;
            let indent = self.emit_token(Indent, &" ".repeat(indent_len));
            self.indent_stack.push(indent_len);
            Some(Ok(indent))
        } else if sum_indent > spaces.len() {
            if is_valid_dedent {
                let dedent = self.emit_token(Dedent, "");
                self.indent_stack.pop();
                Some(Ok(dedent))
            } else {
                let invalid_dedent = self.emit_token(Dedent, "");
                Some(Err(LexError::syntax_error(0, invalid_dedent.loc(), switch_lang!(
                    "invalid indent",
                    "インデントが不正です"
                ), None)))
            }
        } else /* if indent_sum == space.len() */ {
            self.col_token_starts += spaces.len();
            None
        }
    }

    fn lex_exponent(&mut self, mantissa: String) -> LexResult<Token> {
        let mut num = mantissa;
        debug_power_assert!(self.peek_cur_ch(), ==, Some('e'));
        num.push(self.consume().unwrap()); // e
        num.push(self.consume().unwrap()); // + | -
        while let Some(cur) = self.peek_cur_ch() {
            if cur.is_ascii_digit() || cur == '_' {
                num.push(self.consume().unwrap());
            } else {
                break;
            }
        }
        Ok(self.emit_token(RatioLit, &num))
    }

    /// `_` will be removed at compiletime
    fn lex_num(&mut self, first_ch: char) -> LexResult<Token> {
        let mut num = first_ch.to_string();
        while let Some(ch) = self.peek_cur_ch() {
            match ch {
                // `.` may be a dot operator, don't consume
                '.' => { return self.lex_num_dot(num) },
                n if n.is_ascii_digit() || n == '_' => {
                    num.push(self.consume().unwrap());
                }
                c if Self::is_valid_symbol_ch(c) => {
                    // exponent (e.g. 10e+3)
                    if c == 'e'
                        && (self.peek_next_ch() == Some('+') || self.peek_next_ch() == Some('-')) {
                        return self.lex_exponent(num)
                    } else {
                        // IntLit * Symbol(e.g. 3x + 1)
                        let token = self.emit_token(Illegal, &(num + &c.to_string()));
                        return Err(LexError::feature_error(0, token.loc(), "*-less multiply"))
                    }
                }
                _ => {
                    break;
                }
            }
        }
        let kind = if num.starts_with('-') && !Self::is_zero(&num) { IntLit } else { NatLit };
        Ok(self.emit_token(kind, &num))
    }

    /// number '.' ~~
    /// Possibility: RatioLit or Int/NatLit call
    fn lex_num_dot(&mut self, mut num: String) -> LexResult<Token> {
        match self.peek_next_ch() {
            // RatioLit
            Some(n) if n.is_ascii_digit() => {
                num.push(self.consume().unwrap());
                self.lex_ratio(num)
            }
            // method call of IntLit
            // or range operator (e.g. 1..)
            Some(c) if Self::is_valid_symbol_ch(c) || c == '.' => Ok(self.emit_token(IntLit, &num)),
            Some('_')  => {
                self.consume();
                let token = self.emit_token(Illegal, &(num + "_"));
                Err(LexError::simple_syntax_error(0, token.loc()))
            }
            // RatioLit without zero (e.g. 3.)
            _ => {
                num.push(self.consume().unwrap());
                self.lex_ratio(num)
            }
        }
    }

    /// int_part_and_point must be like `12.`
    fn lex_ratio(&mut self, intpart_and_point: String) -> LexResult<Token> {
        let mut num = intpart_and_point;
        while let Some(cur) = self.peek_cur_ch() {
            if cur.is_ascii_digit() || cur == '_' {
                num.push(self.consume().unwrap());
            } else if cur == 'e' {
                return self.lex_exponent(num)
            } else {
                break;
            }
        }
        Ok(self.emit_token(RatioLit, &num))
    }

    fn lex_symbol(&mut self, first_ch: char) -> LexResult<Token> {
        let mut cont = first_ch.to_string();
        while let Some(c) = self.peek_cur_ch() {
            if Self::is_valid_symbol_ch(c) {
                cont.push(self.consume().unwrap());
            } else {
                break;
            }
        }
        if let Some('!') = self.peek_cur_ch() {
            cont.push(self.consume().unwrap());
        }
        if cont.is_empty() {
            let token = self.emit_token(Illegal, &self.peek_cur_ch().unwrap().to_string());
            return Err(LexError::compiler_bug(0, token.loc(), fn_name_full!(), line!()))
        }
        // dot: scalar product, cross: vector product
        // An alphabetical operator can also declare as a function, so checking is necessary
        // e.g. and(true, true, true) = true
        let kind = match &cont[..] {
            "and" => AndOp,
            "or" => OrOp,
            "in" => InOp,
            "notin" => NotInOp,
            "is" => IsOp,
            "isnot" => IsNotOp,
            "dot" => DotOp,
            "cross" => CrossOp,
            // これらはリテラルというより定数だが便宜的にリテラルということにしておく
            "True" | "False" => BoolLit,
            "None" => NoneLit,
            "NotImplemented" => NoImplLit,
            "Ellipsis" => EllipsisLit,
            "Inf" => InfLit,
            "_" => UBar,
            _ => Symbol,
        };
        Ok(self.emit_token(kind, &cont))
    }

    fn lex_str(&mut self) -> LexResult<Token> {
        let mut s = "\"".to_string();
        while let Some(c) = self.peek_cur_ch() {
            if c == '\"' && s.chars().last() != Some('\\') {
                s.push(self.consume().unwrap());
                let token = self.emit_token(StrLit, &s);
                return Ok(token)
            } else {
                let c = self.consume().unwrap();
                s.push(c);
                if Self::is_bidi(c) {
                    let token = self.emit_token(Illegal, &s);
                    return Err(LexError::syntax_error(0, token.loc(), switch_lang!(
                        "invalid unicode character (bi-directional override) in string literal",
                        "不正なユニコード文字(双方向オーバーライド)が文字列中に使用されています"
                    ), None))
                }
            }
        }
        let token = self.emit_token(Illegal, &s);
        Err(LexError::syntax_error(0, token.loc(), switch_lang!(
            "the string is not closed by \"",
            "文字列が\"によって閉じられていません"
        ), None))
    }
}

impl Iterator for Lexer /*<'a>*/ {
    type Item = LexResult<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.prev_token.is(TokenKind::EOF) {
            return None
        }
        let indent_dedent = self.lex_space_indent_dedent();
        if indent_dedent.is_some() {
            return indent_dedent
        }
        if let Some('#') = self.peek_cur_ch() {
            if let Err(e) = self.lex_comment() { return Some(Err(e)) }
        }
        match self.consume() {
            Some('(') => self.accept(LParen, "("),
            Some(')') => self.accept(RParen, ")"),
            Some('[') => self.accept(LSqBr, "["),
            Some(']') => self.accept(RSqBr, "]"),
            Some('{') => self.accept(LBrace, "{"),
            Some('}') => self.accept(RBrace, "}"),
            Some('<') => match self.peek_cur_ch() {
                Some('.') => {
                    self.consume();
                    if let Some('.') = self.peek_cur_ch() {
                        self.consume();
                        match self.peek_cur_ch() {
                            Some('<') => {
                                self.consume();
                                self.accept(Open, "<..<")
                            }
                            _ => self.accept(LeftOpen, "<.."),
                        }
                    } else {
                        let token = self.emit_token(Illegal, "<.");
                        Some(Err(LexError::syntax_error(0, token.loc(), switch_lang!(
                            "no such operator: <.",
                            "<.という演算子はありません"
                        ), None)))
                    }
                }
                Some('=') => {
                    self.consume();
                    self.accept(LessEq, "<=")
                }
                Some('<') => {
                    self.consume();
                    self.accept(Shl, "<<")
                }
                _ => self.accept(Less, "<"),
            },
            Some('>') => match self.peek_cur_ch() {
                Some('=') => {
                    self.consume();
                    self.accept(GreEq, ">=")
                }
                Some('>') => {
                    self.consume();
                    self.accept(Shr, ">>")
                }
                _ => self.accept(Gre, ">"),
            },
            Some('.') => {
                match self.peek_cur_ch() {
                    Some('.') => {
                        self.consume();
                        match self.peek_cur_ch() {
                            Some('<') => {
                                self.consume();
                                self.accept(RightOpen, "..<")
                            },
                            Some('.') => {
                                self.consume();
                                self.accept(EllipsisLit, "...")
                            },
                            _ => {
                                self.accept(Closed, "..")
                            }
                        }
                    }
                    Some(c) if c.is_ascii_digit() => {
                        Some(self.lex_ratio(".".into()))
                    }
                    _ => self.accept(Dot, ".")
                }
            }
            Some(',') => self.accept(Comma, ","),
            Some(':') => match self.peek_cur_ch() {
                Some(':') => {
                    self.consume();
                    self.accept(DblColon, "::")
                }
                Some('>') => {
                    self.consume();
                    self.accept(SupertypeOf, ":>")
                }
                _ => self.accept(Colon, ":"),
            },
            Some(';') => self.accept(Semi, ";"),
            Some('&') => {
                if let Some('&') = self.peek_cur_ch() {
                    self.consume();
                    self.accept(BitAnd, "&&")
                } else {
                    // let kind = if self.is_bin_position().unwrap() { Amper } else { PreAmp };
                    self.accept(Amper, "&")
                }
            }
            Some('|') => {
                match self.peek_cur_ch() {
                    Some('|') => {
                        self.consume();
                        self.accept(BitOr, "||")
                    }
                    Some('=') => {
                        self.consume();
                        self.accept(OrEqual, "|=")
                    }
                    _ => {
                        self.accept(VBar, "|")
                    }
                }
            }
            Some('^') => {
                if let Some('^') = self.peek_cur_ch() {
                    self.consume();
                    self.accept(BitXor, "^^")
                } else {
                    self.accept(Caret, "^")
                }
            }
            Some('~') => self.accept(PreBitNot, "~"),
            // TODO:
            Some('$') => self.deny_feature("$", "shared variables"),
            Some('@') =>  self.accept(AtSign, "@"),
            Some('=') => match self.peek_cur_ch() {
                Some('=') => {
                    self.consume();
                    self.accept(DblEq, "==")
                }
                Some('>') => {
                    self.consume();
                    self.accept(ProcArrow, "=>")
                }
                _ => self.accept(Equal, "="),
            },
            Some('!') => {
                if let Some('=') = self.peek_cur_ch() {
                    self.consume();
                    self.accept(NotEq, "!=")
                } else {
                    self.accept(Mutate, "!")
                }
            }
            Some('?') => self.accept(Try, "?"),
            Some('+') => {
                let kind = if self.is_bin_position().unwrap() {
                    Plus
                } else {
                    PrePlus
                };
                self.accept(kind, "+")
            }
            Some('-') => match self.peek_cur_ch() {
                Some('>') => {
                    self.consume();
                    self.accept(FuncArrow, "->")
                }
                _ => {
                    if self.is_bin_position().unwrap() {
                        self.accept(Minus, "-")
                    } else {
                        // IntLit (negative number)
                        if self.peek_cur_ch().map(|t| t.is_ascii_digit()).unwrap_or(false) {
                            Some(self.lex_num('-'))
                        } else {
                            self.accept(Minus, "-")
                        }
                    }
                }
            },
            Some('*') => match self.peek_cur_ch() {
                Some('*') => {
                    self.consume();
                    self.accept(Pow, "**")
                }
                _ => {
                    let kind = if self.is_bin_position().unwrap() {
                        Star
                    } else {
                        PreStar
                    };
                    self.accept(kind, "*")
                }
            },
            Some('/') => match self.peek_cur_ch() {
                Some('/') => {
                    self.consume();
                    self.accept(FloorDiv, "//")
                }
                _ => self.accept(Slash, "/"),
            },
            Some('%') => self.accept(Mod, "%"),
            // Newline
            // 改行記号はLexer新規生成時に全て\nにreplaceしてある
            Some('\n') => {
                let token = self.emit_token(Newline, "\n");
                self.lineno_token_starts += 1;
                self.col_token_starts = 0;
                Some(Ok(token))
            }
            Some('\t') => {
                let token = self.emit_token(Illegal, "\t");
                Some(Err(LexError::syntax_error(0, token.loc(), switch_lang!(
                    "cannot use a tab as a space",
                    "タブ文字は使用できません"
                ), Some(switch_lang!("use spaces", "スペースを使用してください").into()))))
            }
            // TODO:
            Some('\\') => self.deny_feature("\\", "ignoring line break"),
            // StrLit
            Some('\"') => Some(self.lex_str()),
            // TODO:
            Some('\'') => self.deny_feature("'", "raw identifier"),
            // Symbolized operators (シンボル化された演算子)
            // e.g. `-`(l, r) = l + (-r)
            Some('`') => {
                let mut op = "".to_string();
                while let Some(c) = self.consume() {
                    if c == '`' {
                        if Self::is_definable_operator(&op[..]) {
                            return self.accept(Symbol, &op)
                        } else {
                            let token = self.emit_token(Illegal, &op);
                            return Some(Err(LexError::syntax_error(0, token.loc(), switch_lang!(
                                format!("`{}` cannot be defined by user", &token.content),
                                format!("`{}`はユーザー定義できません", &token.content)
                            ), None)))
                        }
                    }
                    op.push(c);
                }
                let token = self.emit_token(Illegal, &op);
                Some(Err(LexError::syntax_error(0, token.loc(), switch_lang!(
                    format!("back quotes (`) not closed"),
                    format!("バッククォート(`)が閉じられていません")
                ), None)))
            }
            // IntLit or RatioLit
            Some(n) if n.is_ascii_digit() => Some(self.lex_num(n)),
            // Symbol (includes '_')
            Some(c) if Self::is_valid_symbol_ch(c) => Some(self.lex_symbol(c)),
            // Invalid character (e.g. space-like character)
            Some(invalid) => {
                let token = self.emit_token(Illegal, &invalid.to_string());
                Some(Err(LexError::syntax_error(0, token.loc(), switch_lang!(
                    format!("invalid character: '{invalid}'"),
                    format!("この文字は使用できません: '{invalid}'")
                ), None)))
            }
            None => {
                if self.indent_stack.len() == 0 {
                    self.accept(EOF, "")
                } else {
                    self.indent_stack.pop();
                    self.accept(Dedent, "")
                }
            }
        }
    }
}
