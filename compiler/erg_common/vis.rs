use std::borrow::Borrow;
use std::fmt;

use crate::Str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Visibility {
    Private,
    Public,
}

impl Visibility {
    pub const fn is_public(&self) -> bool {
        matches!(self, Self::Public)
    }
    pub const fn is_private(&self) -> bool {
        matches!(self, Self::Private)
    }
}

/// same structure as `Identifier`, but only for Record fields.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    pub vis: Visibility,
    pub symbol: Str,
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.vis == Visibility::Public {
            write!(f, ".{}", self.symbol)
        } else {
            write!(f, "{}", self.symbol)
        }
    }
}

impl Borrow<str> for Field {
    #[inline]
    fn borrow(&self) -> &str {
        &self.symbol[..]
    }
}

impl Borrow<Str> for Field {
    #[inline]
    fn borrow(&self) -> &Str {
        &self.symbol
    }
}

impl Field {
    pub const fn new(vis: Visibility, symbol: Str) -> Self {
        Field { vis, symbol }
    }
}
