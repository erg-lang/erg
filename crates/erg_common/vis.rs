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
#[derive(Debug, Clone, Eq)]
pub struct Field {
    pub vis: Visibility,
    pub symbol: Str,
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol
    }
}

impl std::hash::Hash for Field {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.symbol.hash(state);
    }
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

    pub const fn private(symbol: Str) -> Self {
        Field::new(Visibility::Private, symbol)
    }

    pub const fn public(symbol: Str) -> Self {
        Field::new(Visibility::Public, symbol)
    }

    pub fn is_const(&self) -> bool {
        self.symbol.starts_with(char::is_uppercase)
    }

    pub const fn vis(&self) -> Visibility {
        self.vis
    }
}
