use std::borrow::Cow;

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
pub const CUSTOM_RED: &str = "\x1b[38;2;255;76;76m";
pub const CUSTOM_BLUE: &str = "\x1b[38;2;76;76;255";
pub const CUSTOM_GRAY: &str = "\x1b[38;2;231;231;235m";
pub const CUSTOM_CYAN: &str = "\x1b[38;2;76;255;255";
pub const CUSTOM_MAGENTA: &str = "\x1b[38;2;165;76;255";
pub const CUSTOM_GREEN: &str = "\x1b[38;2;76;255;76m";
pub const CUSTOM_YELLOW: &str = "\x1b[38;2;255;255;76m";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeColors {
    pub error: Color,
    pub warning: Color,
    pub exception: Color,
    pub gutter: Color,
    pub hint: Color,
    pub accent: Color,
}

#[cfg(not(feature = "pretty"))]
pub const COLORS: ThemeColors = ThemeColors {
    error: Color::Red,
    warning: Color::Yellow,
    exception: Color::Magenta,
    gutter: Color::Cyan,
    hint: Color::Green,
    accent: Color::White,
};

#[cfg(feature = "pretty")]
pub const COLORS: ThemeColors = ThemeColors {
    error: Color::CustomRed,
    warning: Color::CustomYellow,
    exception: Color::CustomMagenta,
    gutter: Color::CustomCyan,
    hint: Color::CustomGreen,
    accent: Color::CustomGray,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    // "`- "
    #[cfg(not(feature = "unicode"))]
    pub fn left_bottom_line(&self) -> String {
        format!("{}{} ", self.lbot, self.line)
    }

    // `╰─ `
    #[cfg(feature = "unicode")]
    pub fn left_bottom_line(&self) -> String {
        format!("{}{} ", self.lbot, self.line)
    }

    // "|- "
    #[cfg(not(feature = "unicode"))]
    pub fn left_cross(&self) -> String {
        format!("{}{} ", self.vbar, self.line)
    }

    // "│─ "
    #[cfg(feature = "unicode")]
    pub fn left_cross(&self) -> String {
        format!("{}{} ", self.vbar, self.line)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
/// # use erg_common::style::{Color, Attribute, StyledStr};
/// const URL: StyledStr = StyledStr::new(
///    "https://github.com/erg-lang/erg",
///    Some(Color::White),
///    Some(Attribute::Underline),
/// );
/// ```
#[derive(Debug)]
pub struct StyledStr<'a> {
    text: &'a str,
    color: Option<Color>,
    attribute: Option<Attribute>,
}

impl<'a> StyledStr<'a> {
    pub const fn new(text: &'a str, color: Option<Color>, attribute: Option<Attribute>) -> Self {
        Self {
            text,
            color,
            attribute,
        }
    }
}

impl std::fmt::Display for StyledStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.color, self.attribute) {
            (None, None) => todo!(),
            (None, Some(attr)) => write!(f, "{}{}{}", attr.as_str(), self.text, ATTR_RESET),
            (Some(color), None) => write!(f, "{}{}{}", color.as_str(), self.text, RESET),
            (Some(color), Some(attr)) => {
                write!(
                    f,
                    "{}{}{}{}{}",
                    color.as_str(),
                    attr.as_str(),
                    self.text,
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
    text: String,
    color: Option<Color>,
    attribute: Option<Attribute>,
}

impl StyledString {
    pub fn new<'a, S: Into<Cow<'a, str>>>(
        s: S,
        color: Option<Color>,
        attribute: Option<Attribute>,
    ) -> Self {
        let text: Cow<'a, str> = s.into();
        Self {
            text: text.into_owned(),
            color,
            attribute,
        }
    }

    ///
    /// Methods for pushing additional &str for strings that already have attributes or colors.
    ///
    /// # Example
    /// ```
    /// # use erg_common::style::{Color, Attribute, StyledString};
    /// let mut text = StyledString::new("sample text", None, Some(Attribute::Underline));
    /// text.push_str("\n");
    /// text.push_str("Next break line text");
    /// println!("{text}"); // Two lines of text underlined are displayed
    /// ```
    pub fn push_str(&mut self, s: &str) {
        self.text.push_str(s)
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl std::fmt::Display for StyledString {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
        match (self.color, self.attribute) {
            (None, None) => write!(f, "{}", self.text),
            (None, Some(attr)) => write!(f, "{}{}{}", attr.as_str(), self.text, ATTR_RESET),
            (Some(color), None) => write!(f, "{}{}{}", color.as_str(), self.text, RESET),
            (Some(color), Some(attr)) => write!(
                f,
                "{}{}{}{}{}",
                attr.as_str(),
                color.as_str(),
                self.text,
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
/// # use erg_common::style::{Color, Attribute, StyledStrings};
/// let mut texts = StyledStrings::default();
/// texts.push_str("Default color is gray, ");
/// texts.push_str_with_color("and it is possible to color text.\n", Color::Red);
/// texts.push_str("Basically, this `StyledStrings` is one sentence, ");
/// texts.push_str_with_color("so if you want to multiline sentences, you need to add `\n`.", Color::Magenta);
/// println!("{}", texts); // Pushed colored text are displayed
/// ```
/// Basically,initialize by default with mutable.
/// Then, &str(s) are pushed to the Vec, specifying colors or attributes.
///
#[derive(Debug, Default)]
pub struct StyledStrings {
    texts: Vec<StyledString>,
}

impl StyledStrings {
    ///
    /// It is possible push &str type with gray color to Vector.
    ///
    ///  # Example
    /// ```
    /// # use erg_common::style::StyledStrings;
    /// let mut texts = StyledStrings::default();
    /// texts.push_str("sample text");
    /// texts.push_str("\n");
    /// texts.push_str("new text here");
    /// println!("{}", texts);
    ///  /*
    ///     sample text
    ///     new text here
    ///  */
    ///
    /// ```
    pub fn push_str(&mut self, s: &str) {
        if self.is_same_color(Color::Gray) {
            self.texts.last_mut().unwrap().text.push_str(s);
        } else {
            self.texts.push(StyledString::new(s, None, None));
        }
    }

    ///
    /// It is possible to push &str type with specify color to Vector.
    ///
    /// # Example
    /// ```
    /// # use erg_common::style::{Color, Attribute, StyledStrings};
    /// let mut texts = StyledStrings::default();
    /// texts.push_str_with_color("Cyan color text", Color::Cyan);
    /// texts.push_str_with_color("Red color text", Color::Red);
    /// texts.push_str_with_color(", pushed texts become a single String.", Color::Yellow);
    /// texts.push_str_with_color("\n If you want to add break lines, you should add `\n`.", Color::Magenta);
    /// println!("{}", texts);
    /// ```
    pub fn push_str_with_color<'a, S: Into<Cow<'a, str>>>(&mut self, s: S, color: Color) {
        if self.is_same_color(color) {
            let text = s.into();
            self.texts.last_mut().unwrap().text.push_str(&text);
        } else {
            self.texts.push(StyledString::new(s, Some(color), None));
        }
    }

    ///
    /// Text can be pushed color and attribute to Vector.
    /// When color or attribute are different, it will be pushed as different String.
    ///
    /// # Example
    /// ```
    /// # use erg_common::style::{Color, Attribute, StyledStrings};
    /// let mut texts = StyledStrings::default();
    /// texts.push_str_with_color_and_attribute("Magenta and bold text\n", Color::Magenta, Attribute::Bold);
    /// texts.push_str_with_color_and_attribute("White and underlined text", Color::White, Attribute::Underline);
    /// // texts.push_str_with_color_and_attribute("Must be specify the color and attribute", None, Attribute::Underline);
    /// println!("{}", texts);
    /// ```
    pub fn push_str_with_color_and_attribute<'a, S: Into<Cow<'a, str>>>(
        &mut self,
        s: S,
        color: Color,
        attr: Attribute,
    ) {
        if self.is_same_color(color) && self.is_same_attribute(attr) {
            let text = s.into();
            self.texts.last_mut().unwrap().text.push_str(&text);
        } else {
            self.texts
                .push(StyledString::new(s, Some(color), Some(attr)));
        }
    }

    pub fn is_same_color(&self, color: Color) -> bool {
        if let Some(text) = self.texts.last() {
            return text.color == Some(color);
        }
        false
    }

    fn is_same_attribute(&self, attr: Attribute) -> bool {
        if let Some(text) = self.texts.last() {
            if let Some(text_attr) = text.attribute {
                return text_attr == attr;
            }
        }
        false
    }
}

impl std::fmt::Display for StyledStrings {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for text in self.texts.iter() {
            write!(f, "{}", text)?;
        }
        Ok(())
    }
}

impl From<StyledStrings> for String {
    fn from(s: StyledStrings) -> Self {
        s.to_string()
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
    fn str_texts_test() {
        let mut texts = StyledStrings::default();
        texts.push_str("Gray is the default color\n");
        texts.push_str("If you specify the color, ");
        texts.push_str("you should use `push_str_with_color()`\n");

        texts.push_str_with_color(
            "It is possible to change text foreground color...\n",
            Color::White,
        );
        texts.push_str_with_color("Cyan text, ", Color::Cyan);
        texts.push_str_with_color("Black text, ", Color::Black);
        texts.push_str_with_color("Blue text, ", Color::Blue);
        texts.push_str_with_color("Red text, ", Color::Red);
        texts.push_str_with_color("pushed texts become a String.", Color::Yellow);
        texts.push_str_with_color(
            "\nIf you want to add break lines, you should add `\\n`.\n",
            Color::Magenta,
        );

        texts.push_str_with_color(
            "It is also possible to change text attribute...\n",
            Color::White,
        );
        texts.push_str_with_color_and_attribute(
            "Green and bold text\n",
            Color::Green,
            Attribute::Bold,
        );
        texts.push_str_with_color_and_attribute(
            "Blue and underlined text\n",
            Color::Blue,
            Attribute::Underline,
        );
        texts.push_str_with_color_and_attribute(
            "Red and reversed text",
            Color::Red,
            Attribute::Reversed,
        );
        println!("{}", texts);
    }
}
