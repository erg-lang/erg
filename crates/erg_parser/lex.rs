//! defines and implements `Lexer` (Tokenizer).
use std::cmp::Ordering;

use unicode_xid::UnicodeXID;

use erg_common::cache::CacheSet;
use erg_common::config::ErgConfig;
use erg_common::config::Input;
use erg_common::traits::DequeStream;
use erg_common::traits::{Locational, Runnable, Stream};
use erg_common::{debug_power_assert, fn_name_full, normalize_newline, switch_lang};

use crate::error::{LexError, LexErrors, LexResult, LexerRunnerError, LexerRunnerErrors};
use crate::token::{Token, TokenCategory, TokenKind, TokenStream};
use TokenKind::*;

/// Lexerは使い捨てなので、Runnerを用意
#[derive(Debug, Default)]
pub struct LexerRunner {
    cfg: ErgConfig,
}

impl Runnable for LexerRunner {
    type Err = LexerRunnerError;
    type Errs = LexerRunnerErrors;
    const NAME: &'static str = "Erg lexer";

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
        let lexer = Lexer::from_str(self.input().read());
        let ts = lexer
            .lex()
            .map_err(|errs| LexerRunnerErrors::convert(self.input(), errs))?;
        println!("{ts}");
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, LexerRunnerErrors> {
        let lexer = Lexer::from_str(src);
        if cfg!(feature = "debug") {
            let ts = lexer
                .lex()
                .map_err(|errs| LexerRunnerErrors::convert(self.input(), errs))?;
            println!("{ts}");
            Ok(ts.to_string())
        } else {
            Ok(lexer
                .lex()
                .map_err(|errs| LexerRunnerErrors::convert(self.input(), errs))?
                .to_string())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    SingleLine,
    MultiLine(Quote),
    Not,
}

impl Interpolation {
    pub const fn is_in(&self) -> bool {
        matches!(self, Self::SingleLine | Self::MultiLine(_))
    }

    pub const fn quote(&self) -> Option<Quote> {
        match self {
            Self::MultiLine(q) => Some(*q),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quote {
    Single,
    Double,
}

impl Quote {
    pub const fn quotes(&self) -> &str {
        match self {
            Self::Single => "'''",
            Self::Double => "\"\"\"",
        }
    }

    pub const fn char(&self) -> char {
        match self {
            Self::Single => '\'',
            Self::Double => '"',
        }
    }

    pub const fn token_kind(&self) -> TokenKind {
        match self {
            Self::Single => TokenKind::DocComment,
            Self::Double => TokenKind::StrLit,
        }
    }
}

/// Lexes a source code and iterates tokens.
///
/// This can be used as an iterator or to generate a `TokenStream`.
///
/// NOTE: `Lexer` checks if indent/dedent is used in the correct format,
/// but cannot check if indent/dedent is used in the grammatically correct position; `Parser` does that.
#[derive(Debug)]
pub struct Lexer /*<'a>*/ {
    str_cache: CacheSet<str>,
    chars: Vec<char>,
    indent_stack: Vec<usize>,
    /// indicates the position in the entire source code
    cursor: usize,
    /// to determine the type of operators, etc.
    prev_token: Token,
    /// 0-origin, but Token.lineno will 1-origin
    lineno_token_starts: u32,
    /// 0-origin, indicates the column number in which the token appears
    col_token_starts: u32,
    interpol_stack: Vec<Interpolation>,
}

impl Lexer /*<'a>*/ {
    pub fn new(input: Input) -> Self {
        let normed = normalize_newline(&input.read());
        Lexer {
            str_cache: CacheSet::new(),
            chars: normed.chars().collect::<Vec<char>>(),
            indent_stack: vec![],
            cursor: 0,
            prev_token: Token::new(TokenKind::BOF, "", 0, 0),
            lineno_token_starts: 0,
            col_token_starts: 0,
            interpol_stack: vec![Interpolation::Not],
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(src: String) -> Self {
        let escaped = normalize_newline(&src);
        Lexer {
            str_cache: CacheSet::new(),
            chars: escaped.chars().collect::<Vec<char>>(),
            indent_stack: vec![],
            cursor: 0,
            prev_token: Token::new(TokenKind::BOF, "", 0, 0),
            lineno_token_starts: 0,
            col_token_starts: 0,
            interpol_stack: vec![Interpolation::Not],
        }
    }

    pub fn lex(self) -> Result<TokenStream, LexErrors> {
        let mut result = TokenStream::empty();
        let mut errs = LexErrors::empty();
        for i in self {
            match i {
                Ok(token) => result.push(token),
                Err(err) => {
                    errs.push(err);
                }
            }
        }
        if errs.is_empty() {
            Ok(result)
        } else {
            Err(errs)
        }
    }

    fn emit_token(&mut self, kind: TokenKind, cont: &str) -> Token {
        let cont = self.str_cache.get(cont);
        // cannot use String::len() for multi-byte characters
        let cont_len = cont.chars().count();
        let token = Token::new(
            kind,
            cont,
            self.lineno_token_starts + 1,
            self.col_token_starts,
        );
        self.prev_token = token.clone();
        self.col_token_starts += cont_len as u32;
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

    fn is_valid_start_symbol_ch(c: char) -> bool {
        // fast-path
        // https://github.com/rust-lang/rust/blob/6b5f9b2e973e438fc1726a2d164d046acd80b170/src/librustc_lexer/src/lib.rs#L153
        ('a'..='z').contains(&c)
            || ('A'..='Z').contains(&c)
            || ('0'..='9').contains(&c)
            || c == '_'
            || c.is_xid_start()
    }

    fn is_valid_continue_symbol_ch(c: char) -> bool {
        ('a'..='z').contains(&c)
            || ('A'..='Z').contains(&c)
            || ('0'..='9').contains(&c)
            || c == '_'
            || (c.is_xid_continue() && !('０'..='９').contains(&c))
    }

    /// Detect `c` is a bidirectional overriding character.
    /// [CVE-2021-42574: homoglyph atack](https://blog.rust-lang.org/2021/11/01/cve-2021-42574.html) countermeasures.
    pub fn is_bidi(c: char) -> bool {
        matches!(c, '\u{200F}' | '\u{202B}' | '\u{202E}' | '\u{2067}')
    }

    #[inline]
    fn is_definable_operator(s: &str) -> bool {
        matches!(
            s,
            "+_" | "_+_"
                | "-"
                | "*"
                | "/"
                | "//"
                | "**"
                | "%"
                | ".."
                | "..="
                | "~"
                | "&&"
                | "||"
                | "^^"
                | ">>"
                | "<<"
                | "=="
                | "!="
                | ">"
                | "<"
                | ">="
                | "<="
                | "dot"
                | "cross"
        )
    }

    // +, -, * etc. may be pre/bin
    // and, or, is!, isnot!, in, notin, as, dot, cross may be bin/function
    const fn is_bin_position(&self) -> Option<bool> {
        match self.prev_token.category() {
            // unary: `[ +`, `= +`, `+ +`, `, +`, `:: +`
            TokenCategory::LEnclosure
            | TokenCategory::BinOp
            | TokenCategory::UnaryOp
            | TokenCategory::Separator
            | TokenCategory::SpecialBinOp
            | TokenCategory::DefOp
            | TokenCategory::LambdaOp
            | TokenCategory::StrInterpLeft
            | TokenCategory::StrInterpMid
            | TokenCategory::BOF => Some(false),
            // bin: `] +`, `1 +`, `true and[true]`, `}aaa"`
            TokenCategory::REnclosure | TokenCategory::Literal | TokenCategory::StrInterpRight => {
                Some(true)
            }
            // bin: `fn +1`
            // NOTE: if semantic analysis shows `fn` is a function, should this be rewritten to be unary?
            TokenCategory::Symbol => Some(true),
            _ => None,
        }
    }

    fn is_zero(s: &str) -> bool {
        s.replace("-0", "").replace('0', "").is_empty()
    }

    /// emit_tokenで一気にcol_token_startsを移動させるのでここでは移動させない
    fn consume(&mut self) -> Option<char> {
        let now = self.cursor;
        self.cursor += 1;
        self.chars.get(now).copied()
    }

    fn peek_prev_ch(&self) -> Option<char> {
        if self.cursor == 0 {
            None
        } else {
            self.chars.get(self.cursor - 1).copied()
        }
    }

    #[inline]
    fn peek_cur_ch(&self) -> Option<char> {
        self.chars.get(self.cursor).copied()
    }

    #[inline]
    fn peek_next_ch(&self) -> Option<char> {
        self.chars.get(self.cursor + 1).copied()
    }

    fn lex_comment(&mut self) -> LexResult<()> {
        // debug_power_assert!(self.consume(), ==, Some('#'));
        let mut s = "".to_string();
        while self.peek_cur_ch().map(|cur| cur != '\n').unwrap_or(false) {
            if Self::is_bidi(self.peek_cur_ch().unwrap()) {
                let comment = self.emit_token(Illegal, &s);
                return Err(LexError::syntax_error(
                    line!() as usize,
                    comment.loc(),
                    switch_lang!(
                        "japanese" => "不正なユニコード文字(双方向オーバーライド)がコメント中に使用されています",
                        "simplified_chinese" => "注释中使用了非法的unicode字符（双向覆盖）",
                        "traditional_chinese" => "註釋中使用了非法的unicode字符（雙向覆蓋）",
                        "english" => "invalid unicode character (bi-directional override) in comments",
                    ),
                    None,
                ));
            }
            s.push(self.consume().unwrap());
        }
        Ok(())
    }

    fn lex_multi_line_comment(&mut self) -> LexResult<()> {
        let mut s = "".to_string();
        let mut nest_level = 0;
        while let Some(c) = self.peek_cur_ch() {
            if let Some(next_c) = self.peek_next_ch() {
                match (c, next_c) {
                    ('#', '[') => nest_level += 1,
                    (']', '#') => {
                        nest_level -= 1;
                        if nest_level == 0 {
                            self.consume(); // ]
                            self.consume(); // #
                            return Ok(());
                        }
                    }
                    _ => {}
                }
                if c == '\n' {
                    self.lineno_token_starts += 1;
                    self.col_token_starts = 0;
                    s.clear();
                    self.consume();
                    continue;
                }
            }
            if Self::is_bidi(c) {
                let comment = self.emit_token(Illegal, &s);
                return Err(LexError::syntax_error(
                    line!() as usize,
                    comment.loc(),
                    switch_lang!(
                        "japanese" => "不正なユニコード文字(双方向オーバーライド)がコメント中に使用されています",
                        "simplified_chinese" => "注释中使用了非法的unicode字符（双向覆盖）",
                        "traditional_chinese" => "註釋中使用了非法的unicode字符（雙向覆蓋）",
                        "english" => "invalid unicode character (bi-directional override) in comments",
                    ),
                    None,
                ));
            }
            s.push(self.consume().unwrap());
        }
        let comment = self.emit_token(Illegal, &s);
        let hint = switch_lang!(
            "japanese" => format!("`]#`の数があと{}個必要です", nest_level),
            "simplified_chinese" => format!("需要{}个`]#`", nest_level),
            "traditional_chinese" => format!("需要{}個`]#`", nest_level),
            "english" => format!("{} `]#`(s) are needed", nest_level),
        );
        Err(LexError::syntax_error(
            line!() as usize,
            comment.loc(),
            switch_lang!(
            "japanese" => "複数行コメントが]#で閉じられていません",
            "simplified_chinese" => "未用]#号结束的多处评论",
            "traditional_chinese" => "多條評論未用]#關閉",
            "english" => "multi-comment is not closed with ]#",
            ),
            Some(hint),
        ))
    }

    fn lex_space_indent_dedent(&mut self) -> Option<LexResult<Token>> {
        let is_line_break_after =
            self.cursor > 0 && !self.indent_stack.is_empty() && self.peek_prev_ch() == Some('\n');
        let is_space = self.peek_cur_ch() == Some(' ');
        let is_linebreak = self.peek_cur_ch() == Some('\n');
        let is_empty = is_space || is_linebreak;
        let is_toplevel = is_line_break_after && !is_empty;
        if is_toplevel {
            let dedent = self.emit_token(Dedent, "");
            self.indent_stack.pop();
            self.col_token_starts = 0;
            return Some(Ok(dedent));
        } else if is_linebreak {
            self.consume();
            let token = self.emit_token(Newline, "\n");
            self.lineno_token_starts += 1;
            self.col_token_starts = 0;
            return Some(Ok(token));
        }
        let mut spaces = "".to_string();
        while let Some(' ') = self.peek_cur_ch() {
            spaces.push(self.consume().unwrap());
        }
        // indent in the first line: error
        if !spaces.is_empty() && self.prev_token.is(BOF) {
            let space = self.emit_token(Illegal, &spaces);
            Some(Err(LexError::syntax_error(
                line!() as usize,
                space.loc(),
                switch_lang!(
                    "japanese" => "インデントが不正です",
                    "simplified_chinese" => "无效缩进",
                    "traditional_chinese" => "無效縮進",
                    "english" => "invalid indent",
                ),
                None,
            )))
        } else if self.prev_token.is(Newline) || self.prev_token.is(Dedent) {
            self.lex_indent_dedent(spaces)
        } else {
            self.col_token_starts += spaces.len() as u32;
            None
        }
    }

    /// The semantic correctness of the use of indent/dedent will be analyzed with `Parser`
    fn lex_indent_dedent(&mut self, spaces: String) -> Option<LexResult<Token>> {
        let spaces_len = spaces.len();
        // same as the CPython's limit
        if spaces_len > 100 {
            let token = self.emit_token(Indent, &spaces);
            return Some(Err(LexError::syntax_error(
                line!() as usize,
                token.loc(),
                switch_lang!(
                    "japanese" => "インデントが深すぎます",
                    "simplified_chinese" => "缩进太深",
                    "traditional_chinese" => "縮進太深",
                    "english" => "indentation is too deep",
                ),
                Some(
                    switch_lang!(
                        "japanese" => "コードが複雑すぎます。処理を分割してください",
                        "simplified_chinese" => "代码过于复杂，请拆分过程",
                        "traditional_chinese" => "代碼過於復雜，請拆分過程",
                        "english" => "The code is too complicated. Please split the process",
                    )
                    .into(),
                ),
            )));
        }
        // ignore indents if the current line is a comment
        if let Some('#') = self.peek_cur_ch() {
            if let Some('[') = self.peek_next_ch() {
                if let Err(e) = self.lex_multi_line_comment() {
                    return Some(Err(e));
                }
            } else if let Err(e) = self.lex_comment() {
                return Some(Err(e));
            }
        }
        let mut is_valid_dedent = false;
        let calc_indent_and_validate = |sum: usize, x: &usize| {
            if sum + *x == spaces_len || spaces_len == 0 {
                is_valid_dedent = true;
            }
            sum + *x
        };
        let sum_indent = self.indent_stack.iter().fold(0, calc_indent_and_validate);
        match sum_indent.cmp(&spaces_len) {
            Ordering::Less => {
                let indent_len = spaces_len - sum_indent;
                self.col_token_starts += sum_indent as u32;
                let indent = self.emit_token(Indent, &" ".repeat(indent_len));
                self.indent_stack.push(indent_len);
                Some(Ok(indent))
            }
            Ordering::Greater => {
                self.cursor -= spaces_len;
                self.indent_stack.pop();
                if is_valid_dedent {
                    let dedent = self.emit_token(Dedent, "");
                    Some(Ok(dedent))
                } else {
                    let invalid_dedent = self.emit_token(Dedent, "");
                    let hint = if self.peek_cur_ch() == Some('\n') {
                        Some("unnecessary spaces after linebreak".into())
                    } else { None };
                    Some(Err(LexError::syntax_error(
                        line!() as usize,
                        invalid_dedent.loc(),
                        switch_lang!(
                            "japanese" => "インデントが不正です",
                            "simplified_chinese" => "无效缩进",
                            "traditional_chinese" => "無效縮進",
                            "english" => "invalid indent",
                        ),
                        hint,
                    )))
                }
            }
            Ordering::Equal /* if indent_sum == space.len() */ => {
                self.col_token_starts += spaces_len as u32;
                None
            }
        }
    }

    fn lex_exponent(&mut self, mantissa: String) -> LexResult<Token> {
        let mut num = mantissa;
        debug_power_assert!(self.peek_cur_ch(), ==, Some('e'));
        num.push(self.consume().unwrap()); // e
        if self.peek_cur_ch().is_some() {
            num.push(self.consume().unwrap()); // + | -
            while let Some(cur) = self.peek_cur_ch() {
                if cur.is_ascii_digit() || cur == '_' {
                    num.push(self.consume().unwrap());
                } else {
                    break;
                }
            }
            Ok(self.emit_token(RatioLit, &num))
        } else {
            let token = self.emit_token(RatioLit, &num);
            Err(LexError::syntax_error(
                line!() as usize,
                token.loc(),
                switch_lang!(
                    "japanese" => format!("`{}`は無効な十進数リテラルです", &token.content),
                    "simplified_chinese" => format!("`{}`是无效的十进制字面量", &token.content),
                    "traditional_chinese" => format!("`{}`是無效的十進位字面量", &token.content),
                    "english" => format!("`{}` is invalid decimal literal", &token.content),
                ),
                None,
            ))
        }
    }

    /// `_` will be removed at compiletime
    fn lex_num(&mut self, first_ch: char) -> LexResult<Token> {
        let mut num = first_ch.to_string();
        while let Some(ch) = self.peek_cur_ch() {
            match ch {
                // `.` may be a dot operator, don't consume
                '.' => {
                    return self.lex_num_dot(num);
                }
                n if n.is_ascii_digit() || n == '_' => {
                    num.push(self.consume().unwrap());
                }
                c if Self::is_valid_continue_symbol_ch(c) => {
                    // exponent (e.g. 10e+3)
                    if c == 'e'
                        && (self.peek_next_ch() == Some('+') || self.peek_next_ch() == Some('-'))
                    {
                        return self.lex_exponent(num);
                    } else {
                        // IntLit * Symbol(e.g. 3x + 1)
                        let token = self.emit_token(Illegal, &(num + &c.to_string()));
                        return Err(LexError::feature_error(
                            line!() as usize,
                            token.loc(),
                            "*-less multiply",
                        ));
                    }
                }
                _ => {
                    break;
                }
            }
        }
        let kind = if num.starts_with('-') && !Self::is_zero(&num) {
            IntLit
        } else {
            NatLit
        };
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
            Some(c) if Self::is_valid_continue_symbol_ch(c) || c == '.' => {
                let kind = if num.starts_with('-') && !Self::is_zero(&num) {
                    IntLit
                } else {
                    NatLit
                };
                Ok(self.emit_token(kind, &num))
            }
            Some('_') => {
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
                return self.lex_exponent(num);
            } else {
                break;
            }
        }
        Ok(self.emit_token(RatioLit, &num))
    }

    fn lex_symbol(&mut self, first_ch: char) -> LexResult<Token> {
        let mut cont = first_ch.to_string();
        while let Some(c) = self.peek_cur_ch() {
            if Self::is_valid_continue_symbol_ch(c) {
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
            return Err(LexError::compiler_bug(
                0,
                token.loc(),
                fn_name_full!(),
                line!(),
            ));
        }
        // dot: scalar product, cross: vector product
        // An alphabetical operator can also declare as a function, so checking is necessary
        // e.g. and(true, true, true) = true
        let kind = match &cont[..] {
            "and" => AndOp,
            "or" => OrOp,
            "in" => InOp,
            "notin" => NotInOp,
            "is!" => IsOp,
            "isnot!" => IsNotOp,
            "dot" => DotOp,
            "cross" => CrossOp,
            "ref" => RefOp,
            "ref!" => RefMutOp,
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

    fn str_line_break_error(token: Token, line: usize) -> LexError {
        LexError::syntax_error(
            line,
            token.loc(),
            switch_lang!(
                "japanese" => "文字列内で改行をすることはできません",
                "simplified_chinese" => "在一个字符串中不允许有换行符",
                "traditional_chinese" => "在一個字符串中不允許有換行符",
                "english" => "Line breaks are not allowed within a string",
            ),
            Some(
                switch_lang!(
                    "japanese" => "\"\"内で改行を使いたい場合は'\\n'を利用してください",
                    "simplified_chinese" => "如果你想在\"\"中使用换行符,请使用'\\n'",
                    "traditional_chinese" => "如果你想在\"\"中使用換行符,請使用'\\n'",
                    "english" => "If you want to use line breaks within \"\", use '\\n'",
                )
                .into(),
            ),
        )
    }

    fn invalid_escape_error(ch: char, token: Token) -> LexError {
        LexError::syntax_error(
            line!() as usize,
            token.loc(),
            switch_lang!(
                "japanese" => format!("不正なエスケープシーケンスです: \\{}", ch),
                "simplified_chinese" => format!("不合法的转义序列: \\{}", ch),
                "traditional_chinese" => format!("不合法的轉義序列: \\{}", ch),
                "english" => format!("illegal escape sequence: \\{}", ch),
            ),
            None,
        )
    }

    fn unclosed_string_error(token: Token, by: &str, line: usize) -> LexError {
        let by = if by.is_empty() {
            "".to_string()
        } else {
            switch_lang!(
                "japanese" => format!("{by}によって"),
                "simplified_chinese" => format!("{by}关"),
                "traditional_chinese" => format!("{by}关"),
                "english" => format!("by {by}"),
            )
        };
        LexError::syntax_error(
            line,
            token.loc(),
            switch_lang!(
                "japanese" => format!("文字列が{by}閉じられていません"),
                "simplified_chinese" => format!("字符串没有被{by}闭"),
                "traditional_chinese" => format!("字符串没有被{by}闭"),
                "english" => format!("the string is not closed {by}"),
            ),
            None,
        )
    }

    fn unclosed_interpol_error(token: Token) -> LexError {
        LexError::syntax_error(
            line!() as usize,
            token.loc(),
            switch_lang!(
                "japanese" => "文字列内の補間が閉じられていません",
                "simplified_chinese" => "字符串内的插值没有被闭",
                "traditional_chinese" => "字符串內的插值沒有被閉",
                "english" => "the interpolation in the string is not closed",
            ),
            None,
        )
    }

    fn invalid_unicode_character(&mut self, s: &str) -> LexError {
        let token = self.emit_token(Illegal, s);
        LexError::syntax_error(
            line!() as usize,
            token.loc(),
            switch_lang!(
                "japanese" => "不正なユニコード文字(双方向オーバーライド)が文字列中に使用されています",
                "simplified_chinese" => "注释中使用了非法的unicode字符（双向覆盖）",
                "traditional_chinese" => "註釋中使用了非法的unicode字符（雙向覆蓋）",
                "english" => "invalid unicode character (bi-directional override) in string literal",
            ),
            None,
        )
    }

    fn lex_single_str(&mut self) -> LexResult<Token> {
        let mut s = "\"".to_string();
        while let Some(c) = self.peek_cur_ch() {
            match c {
                '\n' => match self.interpol_stack.last().unwrap() {
                    Interpolation::SingleLine if self.interpol_stack.len() == 1 => {
                        let token = self.emit_token(Illegal, &s);
                        return Err(Self::str_line_break_error(token, line!() as usize));
                    }
                    _ => {
                        let token = self.emit_token(Illegal, &s);
                        return Err(Self::unclosed_interpol_error(token));
                    }
                },
                '"' => {
                    s.push(self.consume().unwrap());
                    let token = self.emit_token(StrLit, &s);
                    return Ok(token);
                }
                _ => {
                    let c = self.consume().unwrap();
                    if c == '\\' {
                        let next_c = self.consume().unwrap();
                        match next_c {
                            '{' => {
                                s.push_str("\\{");
                                self.interpol_stack.push(Interpolation::SingleLine);
                                let token = self.emit_token(StrInterpLeft, &s);
                                return Ok(token);
                            }
                            '0' => s.push('\0'),
                            'r' => s.push('\r'),
                            'n' => s.push('\n'),
                            '\'' => s.push('\''),
                            '"' => s.push('"'),
                            't' => s.push_str("    "), // tab is invalid, so changed into 4 whitespace
                            '\\' => s.push('\\'),
                            _ => {
                                let token = self.emit_token(Illegal, &format!("\\{next_c}"));
                                return Err(Self::invalid_escape_error(next_c, token));
                            }
                        }
                    } else {
                        s.push(c);
                        if Self::is_bidi(c) {
                            return Err(self.invalid_unicode_character(&s));
                        }
                    }
                }
            }
        }
        let token = self.emit_token(Illegal, &s);
        Err(Self::unclosed_string_error(token, "\"", line!() as usize))
    }

    fn lex_multi_line_str(&mut self, quote: Quote) -> LexResult<Token> {
        let mut s = quote.quotes().to_string();
        while let Some(c) = self.peek_cur_ch() {
            if c == quote.char() {
                let c = self.consume().unwrap();
                let next_c = self.peek_cur_ch();
                let aft_next_c = self.peek_next_ch();
                if next_c.is_none() {
                    let token = self.emit_token(Illegal, &s);
                    return Err(Self::unclosed_string_error(
                        token,
                        quote.quotes(),
                        line!() as usize,
                    ));
                }
                if aft_next_c.is_none() {
                    s.push(self.consume().unwrap());
                    let token = self.emit_token(Illegal, &s);
                    return Err(Self::unclosed_string_error(
                        token,
                        quote.quotes(),
                        line!() as usize,
                    ));
                }
                if next_c.unwrap() == quote.char() && aft_next_c.unwrap() == quote.char() {
                    self.consume().unwrap();
                    self.consume().unwrap();
                    s.push_str(quote.quotes());
                    let token = self.emit_token(quote.token_kind(), &s);
                    return Ok(token);
                }
                // else unclosed_string_error
                s.push(c);
            } else {
                let c = self.consume().unwrap();
                match c {
                    '\\' => {
                        let next_c = self.consume().unwrap();
                        match next_c {
                            '{' => {
                                s.push_str("\\{");
                                self.interpol_stack.push(Interpolation::MultiLine(quote));
                                let token = self.emit_token(StrInterpLeft, &s);
                                return Ok(token);
                            }
                            '0' => s.push('\0'),
                            'r' => s.push('\r'),
                            '\'' => s.push('\''),
                            '\"' => s.push('\"'),
                            't' => s.push_str("    "), // tab is invalid, so changed into 4 whitespace
                            '\\' => s.push('\\'),
                            'n' => s.push('\n'),
                            '\n' => {
                                self.lineno_token_starts += 1;
                                self.col_token_starts = 0;
                                continue;
                            }
                            _ => {
                                let token = self.emit_token(Illegal, &format!("\\{next_c}"));
                                return Err(Self::invalid_escape_error(next_c, token));
                            }
                        }
                    }
                    '\n' => {
                        self.lineno_token_starts += 1;
                        self.col_token_starts = 0;
                        s.push('\n');
                    }
                    _ => {
                        s.push(c);
                        if Self::is_bidi(c) {
                            return Err(self.invalid_unicode_character(&s));
                        }
                    }
                }
            }
        }
        let token = self.emit_token(Illegal, &s);
        if self.interpol_stack.len() == 1 {
            Err(Self::unclosed_string_error(
                token,
                quote.quotes(),
                line!() as usize,
            ))
        } else {
            Err(Self::unclosed_interpol_error(token))
        }
    }

    /// e.g. `}aaa"`, `}aaa{`
    fn lex_interpolation_mid(&mut self) -> LexResult<Token> {
        let mut s = "}".to_string();
        while let Some(c) = self.peek_cur_ch() {
            match c {
                '\n' => match self.interpol_stack.last().unwrap() {
                    Interpolation::MultiLine(_) => {
                        self.lineno_token_starts += 1;
                        self.col_token_starts = 0;
                        self.consume().unwrap();
                        s.push('\n');
                    }
                    Interpolation::SingleLine => {
                        if self.peek_next_ch().is_some() {
                            let token = self.emit_token(Illegal, &s);
                            return Err(Self::str_line_break_error(token, line!() as usize));
                        } else {
                            let token = self.emit_token(Illegal, &s);
                            return Err(Self::unclosed_string_error(token, "", line!() as usize));
                        }
                    }
                    Interpolation::Not => {
                        let token = self.emit_token(Illegal, &s);
                        return Err(Self::unclosed_interpol_error(token));
                    }
                },
                '"' => {
                    s.push(self.consume().unwrap());
                    match self.interpol_stack.last().copied().unwrap() {
                        Interpolation::MultiLine(quote) => {
                            let next_c = self.peek_cur_ch();
                            let aft_next_c = self.peek_next_ch();
                            if next_c.is_none() {
                                self.interpol_stack.pop();
                                let token = self.emit_token(Illegal, &s);
                                return Err(Self::unclosed_string_error(
                                    token,
                                    quote.quotes(),
                                    line!() as usize,
                                ));
                            }
                            if aft_next_c.is_none() {
                                self.interpol_stack.pop();
                                s.push(self.consume().unwrap());
                                let token = self.emit_token(Illegal, &s);
                                return Err(Self::unclosed_string_error(
                                    token,
                                    quote.quotes(),
                                    line!() as usize,
                                ));
                            }
                            if next_c.unwrap() == quote.char()
                                && aft_next_c.unwrap() == quote.char()
                            {
                                self.interpol_stack.pop();
                                self.consume().unwrap();
                                self.consume().unwrap();
                                s.push_str(quote.quotes());
                                let token = self.emit_token(StrInterpRight, &s);
                                return Ok(token);
                            }
                            // else unclosed_string_error
                        }
                        Interpolation::SingleLine => {
                            self.interpol_stack.pop();
                            let token = self.emit_token(StrInterpRight, &s);
                            return Ok(token);
                        }
                        Interpolation::Not => {}
                    }
                }
                _ => {
                    let c = self.consume().unwrap();
                    if c == '\\' {
                        let next_c = self.consume().unwrap();
                        match next_c {
                            '{' => {
                                s.push_str("\\{");
                                let token = self.emit_token(StrInterpMid, &s);
                                return Ok(token);
                            }
                            '0' => s.push('\0'),
                            'r' => s.push('\r'),
                            'n' => s.push('\n'),
                            '\'' => s.push('\''),
                            '"' => s.push('"'),
                            't' => s.push_str("    "), // tab is invalid, so changed into 4 whitespace
                            '\\' => s.push('\\'),
                            _ => {
                                let token = self.emit_token(Illegal, &format!("\\{next_c}"));
                                return Err(Self::invalid_escape_error(next_c, token));
                            }
                        }
                    } else {
                        s.push(c);
                        if Self::is_bidi(c) {
                            return Err(self.invalid_unicode_character(&s));
                        }
                    }
                }
            }
        }
        let token = self.emit_token(Illegal, &s);
        Err(Self::unclosed_string_error(token, "", line!() as usize))
    }

    fn lex_raw_ident(&mut self) -> LexResult<Token> {
        let mut s = "\'".to_string();
        while let Some(c) = self.peek_cur_ch() {
            match c {
                '\n' => {
                    let token = self.emit_token(Illegal, &s);
                    return Err(LexError::simple_syntax_error(line!() as usize, token.loc()));
                }
                '\'' => {
                    s.push(self.consume().unwrap());
                    if self.peek_cur_ch() == Some('!') {
                        s.push(self.consume().unwrap());
                    }
                    let token = self.emit_token(Symbol, &s);
                    return Ok(token);
                }
                _ => {
                    let c = self.consume().unwrap();
                    s.push(c);
                    if Self::is_bidi(c) {
                        return Err(self.invalid_unicode_character(&s));
                    }
                }
            }
        }
        let token = self.emit_token(Illegal, &s);
        Err(LexError::syntax_error(
            line!() as usize,
            token.loc(),
            switch_lang!(
                "japanese" => "raw識別子が'によって閉じられていません",
                "simplified_chinese" => "raw标识符没有被'关闭",
                "traditional_chinese" => "raw標誌符沒有被'關閉",
                "english" => "raw identifier is not closed by '",
            ),
            None,
        ))
    }
}

impl Iterator for Lexer /*<'a>*/ {
    type Item = LexResult<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.prev_token.is(TokenKind::EOF) {
            return None;
        }
        let indent_dedent = self.lex_space_indent_dedent();
        if indent_dedent.is_some() {
            return indent_dedent;
        }
        if let Some('#') = self.peek_cur_ch() {
            if let Some('[') = self.peek_next_ch() {
                if let Err(e) = self.lex_multi_line_comment() {
                    return Some(Err(e));
                }
            } else if let Err(e) = self.lex_comment() {
                return Some(Err(e));
            }
        }
        match self.consume() {
            Some('(') => self.accept(LParen, "("),
            Some(')') => self.accept(RParen, ")"),
            Some('[') => self.accept(LSqBr, "["),
            Some(']') => self.accept(RSqBr, "]"),
            Some('{') => self.accept(LBrace, "{"),
            Some('}') => {
                if self.interpol_stack.last().unwrap().is_in() {
                    Some(self.lex_interpolation_mid())
                } else {
                    self.accept(RBrace, "}")
                }
            }
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
                        Some(Err(LexError::syntax_error(
                            line!() as usize,
                            token.loc(),
                            switch_lang!(
                                "japanese" => "<.という演算子はありません",
                                "simplified_chinese" => "没有这样的运算符: <.",
                                "traditional_chinese" => "沒有這樣的運算符: <.",
                                "english" => "no such operator: <.",
                            ),
                            None,
                        )))
                    }
                }
                Some('-') => {
                    self.consume();
                    self.accept(Inclusion, "<-")
                }
                Some('=') => {
                    self.consume();
                    self.accept(LessEq, "<=")
                }
                Some('<') => {
                    self.consume();
                    self.accept(Shl, "<<")
                }
                Some(':') => {
                    self.consume();
                    self.accept(SubtypeOf, "<:")
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
            Some('.') => match self.peek_cur_ch() {
                Some('.') => {
                    self.consume();
                    match self.peek_cur_ch() {
                        Some('<') => {
                            self.consume();
                            self.accept(RightOpen, "..<")
                        }
                        Some('.') => {
                            self.consume();
                            self.accept(EllipsisLit, "...")
                        }
                        _ => self.accept(Closed, ".."),
                    }
                }
                // prev_token is Symbol => TupleAttribute
                // else: RatioLit (e.g. .0)
                Some(c) if c.is_ascii_digit() && !self.prev_token.is(Symbol) => {
                    Some(self.lex_ratio(".".into()))
                }
                _ => self.accept(Dot, "."),
            },
            Some(',') => self.accept(Comma, ","),
            Some(':') => match self.peek_cur_ch() {
                Some(':') => {
                    self.consume();
                    self.accept(DblColon, "::")
                }
                Some('=') => {
                    self.consume();
                    self.accept(Walrus, ":=")
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
            Some('|') => match self.peek_cur_ch() {
                Some('|') => {
                    self.consume();
                    self.accept(BitOr, "||")
                }
                Some('>') => {
                    self.consume();
                    self.accept(Pipe, "|>")
                }
                _ => self.accept(VBar, "|"),
            },
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
            Some('@') => self.accept(AtSign, "@"),
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
                let kind = match self.is_bin_position() {
                    Some(true) => Plus,
                    Some(false) => PrePlus,
                    None => {
                        let token = self.emit_token(Illegal, "+");
                        return Some(Err(LexError::simple_syntax_error(0, token.loc())));
                    }
                };
                self.accept(kind, "+")
            }
            Some('-') => match self.peek_cur_ch() {
                Some('>') => {
                    self.consume();
                    self.accept(FuncArrow, "->")
                }
                _ => {
                    match self.is_bin_position() {
                        Some(true) => self.accept(Minus, "-"),
                        Some(false) => {
                            // IntLit (negative number)
                            if self
                                .peek_cur_ch()
                                .map(|t| t.is_ascii_digit())
                                .unwrap_or(false)
                            {
                                Some(self.lex_num('-'))
                            } else {
                                self.accept(PreMinus, "-")
                            }
                        }
                        None => {
                            let token = self.emit_token(Illegal, "-");
                            Some(Err(LexError::simple_syntax_error(0, token.loc())))
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
                    let kind = match self.is_bin_position() {
                        Some(true) => Star,
                        _ => {
                            let token = self.emit_token(Illegal, "*");
                            return Some(Err(LexError::simple_syntax_error(0, token.loc())));
                        }
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
                Some(Err(LexError::syntax_error(
                    line!() as usize,
                    token.loc(),
                    switch_lang!(
                        "japanese" => "タブ文字は使用できません",
                        "simplified_chinese" => "不能将制表符用作空格",
                        "traditional_chinese" => "不能將製表符用作空格",
                        "english" => "cannot use a tab as a space",
                    ),
                    Some(
                        switch_lang!(
                            "japanese" => "スペース( )を使用してください",
                            "simplified_chinese" => "使用空格( )",
                            "traditional_chinese" => "使用空格( )",
                            "english" => "use spaces ( )",
                        )
                        .into(),
                    ),
                )))
            }
            // TODO:
            Some('\\') => self.deny_feature("\\", "ignoring line break"),
            // Single StrLit and Multi-line StrLit
            Some('\"') => {
                let c = self.peek_cur_ch();
                let next_c = self.peek_next_ch();
                match (c, next_c) {
                    (None, _) => {
                        let token = self.emit_token(Illegal, "\"");
                        Some(Err(LexError::syntax_error(
                            line!() as usize,
                            token.loc(),
                            switch_lang!(
                                "japanese" => "文字列が\"によって閉じられていません",
                                "simplified_chinese" => "字符串没有被\"关闭",
                                "traditional_chinese" => "字符串没有被\"关闭",
                                "english" => "the string is not closed by \"",
                            ),
                            None,
                        )))
                    }
                    (Some('"'), Some('"')) => {
                        self.consume(); // consume second '"'
                        self.consume(); // consume third '"'
                        Some(self.lex_multi_line_str(Quote::Double))
                    }
                    (Some('"'), None) => {
                        self.consume(); // consume second '"'
                        let token = self.emit_token(StrLit, "\"\"");
                        Some(Ok(token))
                    }
                    _ => Some(self.lex_single_str()),
                }
            }
            Some('\'') => {
                let c = self.peek_cur_ch();
                let next_c = self.peek_next_ch();
                match (c, next_c) {
                    (None, _) => {
                        let token = self.emit_token(Illegal, "'");
                        Some(Err(LexError::syntax_error(
                            line!() as usize,
                            token.loc(),
                            switch_lang!(
                                "japanese" => "raw識別子が'によって閉じられていません",
                                "simplified_chinese" => "raw識別子没被'关闭",
                                "traditional_chinese" => "raw識別字沒被'關閉",
                                "english" => "raw identifier is not ended with '",
                            ),
                            None,
                        )))
                    }
                    (Some('\''), Some('\'')) => {
                        self.consume(); // consume second '
                        self.consume(); // consume third '
                        Some(self.lex_multi_line_str(Quote::Single))
                    }
                    (Some('\''), _) => {
                        self.consume(); // consume second '\''
                        let token = self.emit_token(Illegal, "''");
                        Some(Err(LexError::simple_syntax_error(0, token.loc())))
                    }
                    _ => Some(self.lex_raw_ident()),
                }
            }
            // Symbolized operators (シンボル化された演算子)
            // e.g. `-`(l, r) = l + (-r)
            Some('`') => {
                let mut op = "".to_string();
                while let Some(c) = self.consume() {
                    if c == '`' {
                        if Self::is_definable_operator(&op[..]) {
                            return self.accept(Symbol, &op);
                        } else {
                            let token = self.emit_token(Illegal, &op);
                            let hint = if op.contains('+') {
                                Some(
                                    switch_lang!(
                                        "japanese" => "二項演算子の+は`_+_`、単項演算子の+は`+_`です",
                                        "simplified_chinese" => "二元运算符+是`_+_`，一元运算符+是`+_`",
                                        "traditional_chinese" => "二元運算符+是`_+_`，一元運算符+是`+_`",
                                        "english" => "the binary operator + is `_+_`, the unary operator + is `+_`",
                                    ).into(),
                                )
                            } else {
                                None
                            };
                            return Some(Err(LexError::syntax_error(
                                line!() as usize,
                                token.loc(),
                                switch_lang!(
                                    "japanese" => format!("`{}`はユーザー定義できません", &token.content),
                                    "simplified_chinese" => format!("`{}`不能由用户定义", &token.content),
                                    "traditional_chinese" => format!("`{}`不能由用戶定義", &token.content),
                                    "english" => format!("`{}` cannot be defined by user", &token.content),
                                ),
                                hint,
                            )));
                        }
                    }
                    op.push(c);
                }
                let token = self.emit_token(Illegal, &op);
                Some(Err(LexError::syntax_error(
                    line!() as usize,
                    token.loc(),
                    switch_lang!(
                        "japanese" => format!("バッククォート(`)が閉じられていません"),
                        "simplified_chinese" => format!("反引号(`)未关闭"),
                        "traditional_chinese" => format!("反引號(`)未關閉"),
                        "english" => format!("back quotes (`) not closed"),
                    ),
                    None,
                )))
            }
            // IntLit or RatioLit
            Some(n) if n.is_ascii_digit() => Some(self.lex_num(n)),
            // Symbol (includes '_')
            Some(c) if Self::is_valid_start_symbol_ch(c) => Some(self.lex_symbol(c)),
            // Invalid character (e.g. space-like character)
            Some(invalid) => {
                let token = self.emit_token(Illegal, &invalid.to_string());
                Some(Err(LexError::syntax_error(
                    line!() as usize,
                    token.loc(),
                    switch_lang!(
                        "japanese" => format!("この文字は使用できません: '{invalid}'"),
                        "simplified_chinese" => format!("无效字符: '{invalid}'"),
                        "traditional_chinese" => format!("無效字符: '{invalid}'"),
                        "english" => format!("invalid character: '{invalid}'"),
                    ),
                    None,
                )))
            }
            None => {
                if self.indent_stack.is_empty() {
                    self.accept(EOF, "")
                } else {
                    self.indent_stack.pop();
                    self.accept(Dedent, "")
                }
            }
        }
    }
}
