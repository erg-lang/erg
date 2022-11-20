use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Deref};

use crate::style::{StyledString, StyledStrings};
use crate::Str;

pub type ArcStr = std::sync::Arc<str>;

/// Used to hold an immutable string.
///
/// It can construct as a const (by AtomicStr::ever).
#[derive(Debug, Clone, Eq)]
pub enum AtomicStr {
    Arc(ArcStr),
    Static(&'static str),
}

// unsafe impl Sync for AtomicStr {}

impl PartialEq for AtomicStr {
    #[inline]
    fn eq(&self, other: &AtomicStr) -> bool {
        self[..] == other[..]
    }
}

impl Add<&str> for AtomicStr {
    type Output = AtomicStr;
    #[inline]
    fn add(self, other: &str) -> AtomicStr {
        AtomicStr::from(&format!("{self}{other}"))
    }
}

impl Hash for AtomicStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            AtomicStr::Arc(s) => s[..].hash(state),
            AtomicStr::Static(s) => (*s).hash(state),
        }
    }
}

impl fmt::Display for AtomicStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AtomicStr::Arc(s) => write!(f, "{s}"),
            AtomicStr::Static(s) => write!(f, "{s}"),
        }
    }
}

// &'static str -> &strになってしまわないように
// あえて`impl<S: Into<Str>> From<S> for AtomicStr { ... }`はしない
impl From<&'static str> for AtomicStr {
    #[inline]
    fn from(s: &'static str) -> Self {
        AtomicStr::ever(s)
    }
}

impl From<&String> for AtomicStr {
    #[inline]
    fn from(s: &String) -> Self {
        AtomicStr::Arc((s[..]).into())
    }
}

impl From<String> for AtomicStr {
    #[inline]
    fn from(s: String) -> Self {
        AtomicStr::Arc((s[..]).into())
    }
}

impl From<&ArcStr> for AtomicStr {
    #[inline]
    fn from(s: &ArcStr) -> Self {
        AtomicStr::Arc(s.clone())
    }
}

impl From<ArcStr> for AtomicStr {
    #[inline]
    fn from(s: ArcStr) -> Self {
        AtomicStr::Arc(s)
    }
}

impl From<&AtomicStr> for AtomicStr {
    #[inline]
    fn from(s: &AtomicStr) -> Self {
        match s {
            AtomicStr::Arc(s) => AtomicStr::Arc(s.clone()),
            AtomicStr::Static(s) => AtomicStr::Static(s),
        }
    }
}

impl From<Str> for AtomicStr {
    #[inline]
    fn from(s: Str) -> Self {
        match s {
            Str::Rc(s) => AtomicStr::Arc((&s[..]).into()),
            Str::Static(s) => AtomicStr::Static(s),
        }
    }
}

impl From<&Str> for AtomicStr {
    #[inline]
    fn from(s: &Str) -> Self {
        match s {
            Str::Rc(s) => AtomicStr::Arc((&s[..]).into()),
            Str::Static(s) => AtomicStr::Static(s),
        }
    }
}

impl From<StyledString> for AtomicStr {
    #[inline]
    fn from(s: StyledString) -> Self {
        AtomicStr::Arc(s.to_string().into())
    }
}

impl From<StyledStrings> for AtomicStr {
    #[inline]
    fn from(s: StyledStrings) -> Self {
        AtomicStr::Arc(s.to_string().into())
    }
}

impl Deref for AtomicStr {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}

impl Borrow<str> for AtomicStr {
    #[inline]
    fn borrow(&self) -> &str {
        match self {
            AtomicStr::Arc(s) => s.borrow(),
            AtomicStr::Static(s) => s,
        }
    }
}

impl AsRef<str> for AtomicStr {
    fn as_ref(&self) -> &str {
        self.borrow()
    }
}

impl AtomicStr {
    pub const fn ever(s: &'static str) -> Self {
        AtomicStr::Static(s)
    }

    pub fn arc(s: &str) -> Self {
        AtomicStr::Arc(s.into())
    }

    pub fn into_rc(self) -> ArcStr {
        match self {
            AtomicStr::Arc(s) => s,
            AtomicStr::Static(s) => ArcStr::from(s),
        }
    }

    pub fn is_uppercase(&self) -> bool {
        self.chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    }
}
