pub const ATTR_RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const UNDERLINE: &str = "\x1b[4m";
pub const REVERSED: &str = "\x1b[7m";

// Escape sequences change the color of the terminal
pub const RESET: &str = "\x1b[m";
pub const BLACK: &str = "\x1b[30m";
pub const BLUE: &str = "\x1b[94m";
pub const CYAN: &str = "\x1b[96m";
pub const GRAY: &str = "\x1b[37m";
pub const GREEN: &str = "\x1b[92m";
pub const MAGENTA: &str = "\x1b[95m";
pub const RED: &str = "\x1b[91m";
pub const WHITE: &str = "\x1b[97m";
pub const YELLOW: &str = "\x1b[93m";
// custom colors when use `pretty`
pub const CUSTOM_RED: &str = "\x1b[38;2;185;64;71m";
pub const CUSTOM_BLUE: &str = "\x1b[38;2;230;234;227m";
pub const CUSTOM_GRAY: &str = "\x1b[38;2;244;0;25m";
pub const CUSTOM_CYAN: &str = "\x1b[38;2;160;216;239m";
pub const CUSTOM_MAGENTA: &str = "\x1b[38;2;103;65;150m";
pub const CUSTOM_GREEN: &str = "\x1b[38;2;170;209;71m";
pub const CUSTOM_YELLOW: &str = "\x1b[38;2;230;180;34m";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Color {
    Reset,
    Black,
    Blue,
    Cyan,
    Gray,
    Green,
    Magenta,
    Red,
    White,
    Yellow,
    CustomRed,
    CustomBlue,
    CustomGray,
    CustomCyan,
    CustomMagenta,
    CustomGreen,
    CustomYellow,
}

impl Color {
    pub fn as_str(&self) -> &'static str {
        match self {
            Color::Reset => RESET,
            Color::Black => BLACK,
            Color::Blue => BLUE,
            Color::Cyan => CYAN,
            Color::Gray => GRAY,
            Color::Green => GREEN,
            Color::Magenta => MAGENTA,
            Color::Red => RED,
            Color::Yellow => YELLOW,
            Color::White => WHITE,
            Color::CustomRed => CUSTOM_RED,
            Color::CustomBlue => CUSTOM_BLUE,
            Color::CustomGray => CUSTOM_GRAY,
            Color::CustomCyan => CUSTOM_CYAN,
            Color::CustomMagenta => CUSTOM_MAGENTA,
            Color::CustomGreen => CUSTOM_GREEN,
            Color::CustomYellow => CUSTOM_YELLOW,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Attribute {
    Reset,
    Underline,
    Bold,
    Reversed,
}

impl Attribute {
    pub fn as_str(&self) -> &'static str {
        match self {
            Attribute::Reset => ATTR_RESET,
            Attribute::Underline => UNDERLINE,
            Attribute::Bold => BOLD,
            Attribute::Reversed => REVERSED,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub error: Color,
    pub warning: Color,
    pub exception: Color,
    pub gutter: Color,
    pub hint: Color,
}

#[cfg(not(feature = "pretty"))]
pub const COLORS: ThemeColors = ThemeColors {
    error: Color::Red,
    warning: Color::Yellow,
    exception: Color::Magenta,
    gutter: Color::Cyan,
    hint: Color::Green,
};

#[cfg(feature = "pretty")]
pub const COLORS: ThemeColors = ThemeColors {
    error: Color::CustomRed,
    warning: Color::CustomYellow,
    exception: Color::CustomMagenta,
    gutter: Color::CustomCyan,
    hint: Color::CustomGreen,
};

#[derive(Debug, Clone, Copy)]
pub struct Characters {
    hat: char,    // error
    wave: char,   // exception
    line: char,   // warning and left bottom line
    vbar: char,   // gutter separator
    lbot: char,   // left bottom curve
    vbreak: char, // gutter omission
    lbrac: char,  // error kind modifier left bracket
    rbrac: char,  // error kind modifier right bracket
}

impl Characters {
    pub fn mark(&self, kind: &str) -> String {
        let mark = match kind {
            "Error" => self.hat,
            "Warning" => self.line,
            "Exception" => self.wave,
            invalid => panic!("In Characters, Invalid parameter: {invalid}"),
        };
        mark.to_string()
    }

    pub fn gutters(&self) -> (char, char) {
        (self.vbreak, self.vbar)
    }

    // " `- "
    #[cfg(not(feature = "unicode"))]
    pub fn left_bottom_line(&self) -> String {
        format!(" {}{} ", self.lbot, self.line)
    }

    // `╰─ `
    #[cfg(feature = "unicode")]
    pub fn left_bottom_line(&self) -> String {
        format!("{}{} ", self.lbot, self.line)
    }

    // kind[padded error number]
    #[cfg(not(feature = "pretty"))]
    pub fn error_kind_format(&self, kind: &str, err_num: usize) -> String {
        const PADDING: usize = 4;
        format!("{kind}{}#{err_num:>0PADDING$}{}", self.lbrac, self.rbrac,)
    }

    #[cfg(feature = "pretty")]
    pub fn error_kind_format(&self, kind: &str, err_num: usize) -> String {
        const PADDING: usize = 4;
        let emoji = if kind == "Error" {
            "🚫"
        } else if kind == "Warning" {
            "⚠"
        } else {
            "😱"
        };
        format!(
            "{emoji} {kind}{}#{err_num:>0PADDING$}{}",
            self.lbrac, self.rbrac,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub colors: ThemeColors,
    pub characters: Characters,
}

impl Theme {
    pub const fn characters(&self) -> (Color, &Characters) {
        (self.colors.gutter, &self.characters)
    }

    pub const fn error(&self) -> (Color, char) {
        (self.colors.error, self.characters.hat)
    }

    pub const fn warning(&self) -> (Color, char) {
        (self.colors.warning, self.characters.line)
    }

    pub const fn exception(&self) -> (Color, char) {
        (self.colors.exception, self.characters.wave)
    }

    pub const fn hint(&self) -> (Color, char) {
        (self.colors.hint, self.characters.wave)
    }
}

pub const THEME: Theme = Theme {
    colors: COLORS,
    characters: CHARS,
};

#[cfg(not(feature = "unicode"))]
pub const CHARS: Characters = Characters {
    hat: '-',
    line: '-',
    vbar: '|',
    wave: '~',
    lbot: '`',
    vbreak: ':',
    lbrac: '[',
    rbrac: ']',
};

#[cfg(feature = "unicode")]
pub const CHARS: Characters = Characters {
    hat: '-',
    line: '─',
    vbar: '│',
    wave: '~',
    lbot: '╰',
    vbreak: '·',
    lbrac: '[',
    rbrac: ']',
};

///
/// `StyledStr` is for const color and attribute &str.
/// It is an immutable string.
/// # Example
/// ```
/// const URL: StyledStr = StyledStr::new(
///    "https://github.com/erg-lang/erg",
///    Some(Color::White),
///    Some(Attribute::Underline),
/// );
/// ```
#[derive(Debug)]
pub struct StyledStr<'a> {
    span: &'a str,
    color: Option<Color>,
    attribute: Option<Attribute>,
}

impl<'a> StyledStr<'a> {
    pub const fn new<'b: 'a>(
        span: &'b str,
        color: Option<Color>,
        attribute: Option<Attribute>,
    ) -> Self {
        Self {
            span,
            color,
            attribute,
        }
    }
}

impl std::fmt::Display for StyledStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.color, self.attribute) {
            (None, None) => todo!(),
            (None, Some(attr)) => write!(f, "{}{}{}", attr.as_str(), self.span, ATTR_RESET),
            (Some(color), None) => write!(f, "{}{}{}", color.as_str(), self.span, RESET),
            (Some(color), Some(attr)) => {
                write!(
                    f,
                    "{}{}{}{}{}",
                    color.as_str(),
                    attr.as_str(),
                    self.span,
                    RESET,
                    ATTR_RESET
                )
            }
        }
    }
}

///
/// `StyledString` is for coloring and attribute text.
/// String, Color(&str) and Attribute(&str)
///
#[derive(Debug)]
pub struct StyledString {
    span: String,
    color: Option<Color>,
    attribute: Option<Attribute>,
}

impl StyledString {
    pub fn new(s: &str, color: Option<Color>, attribute: Option<Attribute>) -> Self {
        Self {
            span: String::from(s),
            color,
            attribute,
        }
    }

    ///
    /// Methods for pushing additional &str for strings that already have attributes or colors.
    ///
    /// # Example
    /// ```
    /// let mut span = StyledString::new("sample text", None, Attribute::Underline);
    /// span.push_str("\n");
    /// span.push_str("Next break line text");
    /// println!("{span}"); // Two lines of text underlined are displayed
    /// ```
    pub fn push_str(&mut self, s: &str) {
        self.span.push_str(s);
    }
}

impl std::fmt::Display for StyledString {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
        match (self.color, self.attribute) {
            (None, None) => write!(f, "{}", self.span),
            (None, Some(attr)) => write!(f, "{}{}{}", attr.as_str(), self.span, ATTR_RESET),
            (Some(color), None) => write!(f, "{}{}{}", color.as_str(), self.span, RESET),
            (Some(color), Some(attr)) => write!(
                f,
                "{}{}{}{}{}",
                attr.as_str(),
                color.as_str(),
                self.span,
                RESET,
                ATTR_RESET
            ),
        }
    }
}

///
/// `StyledStrings` is vector of `StyledString` and almost the same as Vec\<String\>.
/// It is possible to change the color and attribute of each String.
/// That's why, if you don't change any color or attribute, you should use 'StyledString' not `StyledStrings`
///
/// # Example
/// ```
/// let mut spans = StyledStrings::default();
/// spans.push_srt("Default color is gray, ");
/// spans.push_str_with_color("and it is possible to color text.\n", Color::Red);
/// spans.push_str("Basically, this `StyledStrings` is one sentence, ");
/// spans.push_str_with_color("so if you want to multiline sentences, you need to add `\n`.", Color::Magenta);
/// println!("{}", spans); // Pushed colored text are displayed
/// ```
/// Basically,initialize by default with mutable.
/// Then, &str(s) are pushed to the Vec, specifying colors or attributes.
///
#[derive(Debug, Default)]
pub struct StyledStrings {
    spans: Vec<StyledString>,
}

impl StyledStrings {
    ///
    /// It is possible push &str type with gray color to Vector.
    ///
    ///  # Example
    /// ```
    /// let mut spans = StyledStrings::default()
    /// spans.push_str("sample text");
    /// spans.push_str("\n");
    /// spans.push_str("new text here");
    /// println!("{}", spans);
    ///  /*
    ///     sample text
    ///     new text here
    ///  */
    ///
    /// ```
    pub fn push_str(&mut self, s: &str) {
        if self.is_same_color(Color::Gray) {
            self.spans.last_mut().unwrap().span.push_str(s);
        } else {
            self.spans.push(StyledString::new(s, None, None));
        }
    }

    ///
    /// It is possible to push &str type with specify color to Vector.
    ///
    /// # Example
    /// ```
    /// let mut spans = StyledStrings::default();
    /// spans.push_str_with_color("Cyan color text", Color::Cyan);
    /// spans.push_str_with_color("Red color text", Color::Red);
    /// spans.push_str_with_color(", pushed texts become a single String.", Color::Yellow);
    /// spans.push_str_with_color("\n If you want to add break lines, you should add `\n`.", Color::Magenta);
    /// println!("{}", spans);
    /// ``
    pub fn push_str_with_color(&mut self, s: &str, color: Color) {
        if self.is_same_color(color) {
            self.spans.last_mut().unwrap().span.push_str(s);
        } else {
            self.spans.push(StyledString::new(s, Some(color), None));
        }
    }

    ///
    /// Text can be pushed color and attribute to Vector.
    /// When color or attribute are different, it will be pushed as different String.
    ///
    /// # Example
    /// ```
    /// let mut spans = StyledStrings::default();
    /// spans.push_str_with_color_and_attribute("Magenta and bold text\n", Color::Magenta, Attribute::Bold);
    /// spans.push_str_with_color_and_attribute("White and underlined text", Color::White, Attribute::Underline);
    /// spans.push_str_with_color_and_attribute("Must be specify the color and attribute", None, Attribute::Underline);
    /// println!("{}", spans);
    /// ```
    pub fn push_str_with_color_and_attribute(&mut self, s: &str, color: Color, attr: Attribute) {
        if self.is_same_color(color) && self.is_same_attribute(attr) {
            self.spans.last_mut().unwrap().span.push_str(s);
        } else {
            self.spans
                .push(StyledString::new(s, Some(color), Some(attr)));
        }
    }

    pub fn is_same_color(&self, color: Color) -> bool {
        if let Some(span) = self.spans.last() {
            return span.color == Some(color);
        }
        false
    }

    pub fn is_same_attribute(&self, attr: Attribute) -> bool {
        if let Some(span) = self.spans.last() {
            if let Some(span_attr) = span.attribute {
                return span_attr == attr;
            }
        }
        false
    }
}

impl std::fmt::Display for StyledStrings {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for span in self.spans.iter() {
            write!(f, "{}", span)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_fg_colorings() {
        println!("{YELLOW}Hello{RESET}, {RED}World{RESET}");
        println!("{BLUE}Hello{RESET}, {GREEN}World{RESET}");
        println!("{MAGENTA}Hello{RESET}, {BLACK}World{RESET}");
        println!("{GRAY}Hello{RESET}, {WHITE}World{RESET}");
        println!("{CUSTOM_BLUE}Hello{RESET}, {CUSTOM_CYAN}World{RESET}");
        println!("{CUSTOM_GRAY}Hello{RESET}, {CUSTOM_GREEN}World{RESET}");
        println!("{CUSTOM_MAGENTA}Hello{RESET}, {CUSTOM_RED}World{RESET}");
    }

    #[test]
    fn text_attribute() {
        println!("{BOLD}BOLD{ATTR_RESET}");
        println!("{UNDERLINE}UNDERLINED{ATTR_RESET}");
        println!("{REVERSED}REVERSED{ATTR_RESET}")
    }

    #[test]
    fn str_spans_test() {
        let mut spans = StyledStrings::default();
        spans.push_str("Gray is the default color\n");
        spans.push_str("If you specify the color, ");
        spans.push_str("you should use `push_str_with_color()`\n");

        spans.push_str_with_color(
            "It is possible to change text foreground color...\n",
            Color::White,
        );
        spans.push_str_with_color("Cyan text, ", Color::Cyan);
        spans.push_str_with_color("Black text, ", Color::Black);
        spans.push_str_with_color("Blue text, ", Color::Blue);
        spans.push_str_with_color("Red text, ", Color::Red);
        spans.push_str_with_color("pushed texts become a String.", Color::Yellow);
        spans.push_str_with_color(
            "\nIf you want to add break lines, you should add `\\n`.\n",
            Color::Magenta,
        );

        spans.push_str_with_color(
            "It is also possible to change text attribute...\n",
            Color::White,
        );
        spans.push_str_with_color_and_attribute(
            "Green and bold text\n",
            Color::Green,
            Attribute::Bold,
        );
        spans.push_str_with_color_and_attribute(
            "White and underlined text",
            Color::White,
            Attribute::Underline,
        );
        println!("{}", spans);
    }
}
