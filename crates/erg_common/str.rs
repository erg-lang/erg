use std::borrow::{Borrow, Cow};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Deref};

#[cfg(feature = "pylib")]
use pyo3::{FromPyObject, IntoPy, PyAny, PyObject, Python};

pub type ArcStr = std::sync::Arc<str>;

/// Used to hold an immutable string.
///
/// It can construct as a const (by Str::ever).
#[derive(Debug, Clone, Eq)]
pub enum Str {
    Rc(ArcStr),
    Static(&'static str),
}

#[cfg(feature = "pylib")]
impl FromPyObject<'_> for Str {
    fn extract(ob: &PyAny) -> pyo3::PyResult<Self> {
        let s = ob.extract::<String>()?;
        Ok(Str::Rc(s.into()))
    }
}

#[cfg(feature = "pylib")]
impl IntoPy<PyObject> for Str {
    fn into_py(self, py: Python<'_>) -> PyObject {
        (&self[..]).into_py(py)
    }
}

impl PartialEq for Str {
    #[inline]
    fn eq(&self, other: &Str) -> bool {
        self[..] == other[..]
    }
}

impl PartialEq<str> for Str {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self[..] == other[..]
    }
}

impl PartialEq<String> for Str {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self[..] == other[..]
    }
}

impl Add<&str> for Str {
    type Output = Str;
    #[inline]
    fn add(self, other: &str) -> Str {
        Str::from(&format!("{self}{other}"))
    }
}

impl Hash for Str {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Str::Rc(s) => s[..].hash(state),
            Str::Static(s) => (*s).hash(state),
        }
    }
}

impl fmt::Display for Str {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Str::Rc(s) => write!(f, "{s}"),
            Str::Static(s) => write!(f, "{s}"),
        }
    }
}

impl From<&Str> for String {
    #[inline]
    fn from(s: &Str) -> Self {
        s.to_string()
    }
}

impl From<Str> for String {
    #[inline]
    fn from(s: Str) -> Self {
        s.to_string()
    }
}

impl<'a> From<Str> for Cow<'a, str> {
    fn from(s: Str) -> Self {
        match s {
            Str::Static(s) => Cow::Borrowed(s),
            Str::Rc(s) => Cow::Owned(s.to_string()),
        }
    }
}

// &'static str -> &strになってしまわないように
// あえて`impl<S: Into<Str>> From<S> for Str { ... }`はしない
impl From<&'static str> for Str {
    #[inline]
    fn from(s: &'static str) -> Self {
        Str::ever(s)
    }
}

impl From<&String> for Str {
    #[inline]
    fn from(s: &String) -> Self {
        Str::Rc((s[..]).into())
    }
}

impl From<String> for Str {
    #[inline]
    fn from(s: String) -> Self {
        Str::Rc((s[..]).into())
    }
}

impl From<&ArcStr> for Str {
    #[inline]
    fn from(s: &ArcStr) -> Self {
        Str::Rc(s.clone())
    }
}

impl From<ArcStr> for Str {
    #[inline]
    fn from(s: ArcStr) -> Self {
        Str::Rc(s)
    }
}

impl From<&Str> for Str {
    #[inline]
    fn from(s: &Str) -> Self {
        match s {
            Str::Rc(s) => Str::Rc(s.clone()),
            Str::Static(s) => Str::Static(s),
        }
    }
}

impl From<Cow<'_, str>> for Str {
    #[inline]
    fn from(s: Cow<'_, str>) -> Self {
        match s {
            Cow::Borrowed(s) => Str::rc(s),
            Cow::Owned(s) => Str::Rc(s.into()),
        }
    }
}

impl Deref for Str {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}

impl Borrow<str> for Str {
    #[inline]
    fn borrow(&self) -> &str {
        match self {
            Str::Rc(s) => &s[..],
            Str::Static(s) => s,
        }
    }
}

impl AsRef<str> for Str {
    fn as_ref(&self) -> &str {
        self.borrow()
    }
}

impl Str {
    pub const fn ever(s: &'static str) -> Self {
        Str::Static(s)
    }

    pub fn rc(s: &str) -> Self {
        Str::Rc(s.into())
    }

    pub fn into_rc(self) -> ArcStr {
        match self {
            Str::Rc(s) => s,
            Str::Static(s) => ArcStr::from(s),
        }
    }

    pub fn is_uppercase(&self) -> bool {
        self.chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    }

    /// split string with multiple separators
    /// ```rust
    /// # use erg_common::str::Str;
    /// let s = Str::rc("a.b::c");
    /// assert_eq!(s.split_with(&[".", "::"]), vec!["a", "b", "c"]);
    /// let s = Str::rc("ああ.いい::うう");
    /// assert_eq!(s.split_with(&[".", "::"]), vec!["ああ", "いい", "うう"]);
    /// ```
    pub fn split_with(&self, seps: &[&str]) -> Vec<&str> {
        let mut result = vec![];
        let mut last_offset = 0;
        for (offset, _c) in self.char_indices() {
            for sep in seps {
                if self[offset..].starts_with(sep) {
                    result.push(&self[last_offset..offset]);
                    last_offset = offset + sep.len();
                }
            }
        }
        result.push(&self[last_offset..]);
        result
    }

    pub fn reversed(&self) -> Str {
        Str::rc(&self.chars().rev().collect::<String>())
    }

    /// Note that replacements may be chained because it attempt to rewrite in sequence
    pub fn multi_replace(&self, paths: &[(&str, &str)]) -> Self {
        let mut self_ = self.to_string();
        for (from, to) in paths {
            self_ = self_.replace(from, to);
        }
        Str::rc(&self_)
    }

    pub fn is_snake_case(&self) -> bool {
        self.chars().all(|c| !c.is_uppercase())
    }

    pub fn to_snake_case(&self) -> Str {
        let mut ret = String::new();
        let mut prev = '_';
        for c in self.chars() {
            if c.is_ascii_uppercase() {
                if prev != '_' {
                    ret.push('_');
                }
                ret.push(c.to_ascii_lowercase());
            } else {
                ret.push(c);
            }
            prev = c;
        }
        Str::rc(&ret)
    }

    pub fn find_sub<'a>(&self, pats: &[&'a str]) -> Option<&'a str> {
        pats.iter().find(|&&pat| self.contains(pat)).copied()
    }

    /// ```
    /// # use erg_common::str::Str;
    /// let s = Str::rc("\n");
    /// assert_eq!(&s.escape()[..], "\\n");
    /// let s = Str::rc("\\");
    /// assert_eq!(&s.escape()[..], "\\\\");
    /// ```
    pub fn escape(&self) -> Str {
        self.multi_replace(&[
            ("\\", "\\\\"),
            ("\0", "\\0"),
            ("\r", "\\r"),
            ("\n", "\\n"),
            ("\"", "\\\""),
            ("\'", "\\'"),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_with() {
        assert_eq!(
            Str::ever("aa::bb.cc").split_with(&[".", "::"]),
            vec!["aa", "bb", "cc"]
        );
        assert_eq!(
            Str::ever("aa::bb.cc").split_with(&["::", "."]),
            vec!["aa", "bb", "cc"]
        );
        assert_eq!(
            Str::ever("aaxxbbyycc").split_with(&["xx", "yy"]),
            vec!["aa", "bb", "cc"]
        );
        assert_ne!(
            Str::ever("aaxxbbyycc").split_with(&["xx", "yy"]),
            vec!["aa", "bb", "ff"]
        );
    }
}
