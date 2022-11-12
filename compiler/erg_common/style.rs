pub const ATTR_RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const UNDERLINE: &str = "\x1b[4m";

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
}

impl Attribute {
    pub fn as_str(&self) -> &'static str {
        match self {
            Attribute::Reset => ATTR_RESET,
            Attribute::Underline => UNDERLINE,
            Attribute::Bold => BOLD,
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

#[derive(Debug, Clone, Copy)]
pub struct Characters {
    pub hat: char,    // error
    pub wave: char,   // exception
    pub line: char,   // warning and left bottom line
    pub vbar: char,   // gutter separator
    pub lbot: char,   // left bottom curve
    pub vbreak: char, // gutter omission
    pub lbrac: char,  // error kind modifier left bracket
    pub rbrac: char,  // error kind modifier right bracket
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
    #[cfg(not(feature = "unicode"))]
    pub fn left_bottom_line(&self) -> String {
        format!(" {}{} ", self.lbot, self.line)
    }

    #[cfg(feature = "unicode")]
    pub fn left_bottom_line(&self) -> String {
        format!("{}{} ", self.lbot, self.line)
    }
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

    pub fn error_kind_format(&self, kind: &str, err_num: usize) -> String {
        self.characters.error_kind_format(kind, err_num)
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

#[derive(Debug)]
pub struct StrSpan<'a> {
    span: &'a str,
    color: Option<Color>,
    attribute: Option<Attribute>,
}

impl<'a> StrSpan<'a> {
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

impl std::fmt::Display for StrSpan<'_> {
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
#[derive(Debug)]
pub struct StringSpan {
    span: String,
    color: Option<Color>,
    attribute: Option<Attribute>,
}

impl StringSpan {
    pub fn new(s: &str, color: Option<Color>, attribute: Option<Attribute>) -> Self {
        Self {
            span: String::from(s),
            color,
            attribute,
        }
    }

    pub fn push_str(&mut self, s: &str) {
        self.span.push_str(s);
    }
}

impl std::fmt::Display for StringSpan {
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

#[derive(Debug, Default)]
pub struct StringSpans {
    spans: Vec<StringSpan>,
}

impl StringSpans {
    pub fn push_str(&mut self, s: &str) {
        if self.is_same_color(Color::Gray) {
            self.spans.last_mut().unwrap().span.push_str(s);
        } else {
            self.spans.push(StringSpan::new(s, None, None));
        }
    }

    pub fn push_str_with_color(&mut self, s: &str, color: Color) {
        if self.is_same_color(color) {
            self.spans.last_mut().unwrap().span.push_str(s);
        } else {
            self.spans.push(StringSpan::new(s, Some(color), None));
        }
    }

    pub fn push_str_with_color_and_attribute(&mut self, s: &str, color: Color, attr: Attribute) {
        if self.is_same_color(color) && self.is_same_attribute(attr) {
            self.spans.last_mut().unwrap().span.push_str(s);
        } else {
            self.spans.push(StringSpan::new(s, Some(color), Some(attr)));
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

impl std::fmt::Display for StringSpans {
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
    fn colorings_fg() {
        println!("{DEEP_RED}Hello{RESET}, {RED}World{RESET}");
        println!("{DEEP_GREEN}Hello{RESET}, {GREEN}World{RESET}");
        println!("{YELLOW}Hello{RESET}, {DEEP_YELLOW}World{RESET}");
        println!("{DEEP_BLUE}Hello{RESET}, {BLUE}World{RESET}");
        println!("{CYAN}Hello{RESET}, {DEEP_CYAN}World{RESET}");
        println!("{MAGENTA}Hello{RESET}, {DEEP_MAGENTA}World{RESET}");
        println!("{GRAY}Hello{RESET}, {WHITE}World{RESET}");
    }

    #[test]
    fn style_test() {
        println!("{BOLD}bold{ATT_RESET}");
        println!("{UNDERLINE}UNDERLINED{ATT_RESET}");
    }
}
