//! Escape sequences change the color of the terminal

pub const RESET: &'static str = "\x1b[m";
pub const DEEP_RED: &'static str = "\x1b[31m";
pub const RED: &'static str = "\x1b[91m";
pub const GREEN: &'static str = "\x1b[92m";
pub const YELLOW: &'static str = "\x1b[93m";
pub const BLUE: &'static str = "\x1b[94m";
pub const CYAN: &'static str = "\x1b[96m";
