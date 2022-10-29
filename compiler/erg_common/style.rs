use std::fmt::Display;

pub const ATT_RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const UNDERLINE: &str = "\x1b[4m";

// Escape sequences change the color of the terminal
pub const RESET: &str = "\x1b[m";
pub const BLACK: &str = "\x1b[30m";
pub const DEEP_RED: &str = "\x1b[31m";
pub const DEEP_GREEN: &str = "\x1b[32m";
pub const DEEP_YELLOW: &str = "\x1b[33m";
pub const DEEP_BLUE: &str = "\x1b[34m";
pub const DEEP_MAGENTA: &str = "\x1b[35m";
pub const DEEP_CYAN: &str = "\x1b[36m";
pub const GRAY: &str = "\x1b[37m";
pub const RED: &str = "\x1b[91m";
pub const GREEN: &str = "\x1b[92m";
pub const YELLOW: &str = "\x1b[93m";
pub const BLUE: &str = "\x1b[94m";
pub const MAGENTA: &str = "\x1b[95m";
pub const CYAN: &str = "\x1b[96m";
pub const WHITE: &str = "\x1b[97m";

#[derive(Debug)]
pub enum Color {
    Cyan,
    Green,
    Gray,
    Magenta,
    Red,
    Yellow,
}

impl Color {
    fn as_str<'a>(self) -> &'a str {
        match self {
            Color::Cyan => CYAN,
            Color::Green => GREEN,
            Color::Gray => GRAY,
            Color::Magenta => MAGENTA,
            Color::Red => RED,
            Color::Yellow => YELLOW,
        }
    }
}

pub struct Span<'a> {
    text: &'a str,
    color: Color,
}

impl<'a> Span<'a> {
    pub fn new(text: &'a str, color: Color) -> Self {
        Self { text, color }
    }
}

impl Display for Span<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color = self.color.as_str();
        write!(f, "{}{}{RESET}", color, self.text)
    }
}

pub struct Spans<'a>(Vec<Span<'a>>);

impl<'a> Spans<'a> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from(s: Vec<Span<'a>>) -> Self {
        Self(s)
    }

    pub fn push_str(&mut self, text: &str, color: Color) {
        let span = Span::new(text, color);
    }

    pub fn push_span(&mut self, span: Span<'a>) {
        self.0.push(span);
    }

    fn connect(self) -> String {
        let mut s = String::new();
        for x in self.0.into_iter() {
            s.push_str(x.color.as_str());
            s.push_str(x.text);
        }
        s + RESET
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
