use std::borrow::Borrow;
use std::fmt;

#[allow(unused_imports)]
use erg_common::log;
use erg_common::set::Set;
use erg_common::traits::Immutable;
use erg_common::{switch_lang, Str};

use erg_parser::ast::AccessModifier;

use crate::context::Context;
use crate::ty::Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisibilityModifier {
    Public,
    Private,
    Restricted(Set<Str>),
    // use Box to reduce the size of enum
    SubtypeRestricted(Box<Type>),
}

impl fmt::Display for VisibilityModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Private => write!(f, "::"),
            Self::Public => write!(f, "."),
            Self::Restricted(namespaces) => write!(f, "::[{namespaces}]"),
            Self::SubtypeRestricted(typ) => write!(f, "::[<: {typ}]"),
        }
    }
}

impl VisibilityModifier {
    pub const fn is_public(&self) -> bool {
        matches!(self, Self::Public)
    }
    pub const fn is_private(&self) -> bool {
        matches!(self, Self::Private)
    }

    pub const fn display_as_accessor(&self) -> &'static str {
        match self {
            Self::Public => ".",
            Self::Private | Self::Restricted(_) | Self::SubtypeRestricted(_) => "::",
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Private => switch_lang!(
                "japanese" => "非公開",
                "simplified_chinese" => "私有",
                "traditional_chinese" => "私有",
                "english" => "private",
            )
            .into(),
            Self::Public => switch_lang!(
                "japanese" => "公開",
                "simplified_chinese" => "公开",
                "traditional_chinese" => "公開",
                "english" => "public",
            )
            .into(),
            Self::Restricted(namespaces) => switch_lang!(
                "japanese" => format!("制限付き公開({namespaces}でのみ公開)"),
                "simplified_chinese" => format!("受限公开({namespaces}中可见)"),
                "traditional_chinese" => format!("受限公開({namespaces}中可見)"),
                "english" => format!("restricted public ({namespaces} only)"),
            ),
            Self::SubtypeRestricted(typ) => switch_lang!(
                "japanese" => format!("制限付き公開({typ}の部分型でのみ公開)"),
                "simplified_chinese" => format!("受限公开({typ}的子类型中可见)"),
                "traditional_chinese" => format!("受限公開({typ}的子類型中可見)"),
                "english" => format!("restricted public (subtypes of {typ} only)"),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Visibility {
    pub modifier: VisibilityModifier,
    pub def_namespace: Str,
}

impl Visibility {
    pub const DUMMY_PRIVATE: Self = Self {
        modifier: VisibilityModifier::Private,
        def_namespace: Str::ever("<dummy>"),
    };
    pub const DUMMY_PUBLIC: Self = Self {
        modifier: VisibilityModifier::Public,
        def_namespace: Str::ever("<dummy>"),
    };
    pub const BUILTIN_PRIVATE: Self = Self {
        modifier: VisibilityModifier::Private,
        def_namespace: Str::ever("<builtins>"),
    };
    pub const BUILTIN_PUBLIC: Self = Self {
        modifier: VisibilityModifier::Public,
        def_namespace: Str::ever("<builtins>"),
    };

    pub const fn new(modifier: VisibilityModifier, def_namespace: Str) -> Self {
        Self {
            modifier,
            def_namespace,
        }
    }

    pub fn private<S: Into<Str>>(namespace: S) -> Self {
        Self {
            modifier: VisibilityModifier::Private,
            def_namespace: namespace.into(),
        }
    }

    pub fn public(namespace: Str) -> Self {
        Self {
            modifier: VisibilityModifier::Public,
            def_namespace: namespace,
        }
    }

    pub const fn is_public(&self) -> bool {
        self.modifier.is_public()
    }
    pub const fn is_private(&self) -> bool {
        self.modifier.is_private()
    }

    pub fn compatible(&self, access: &AccessModifier, namespace: &Context) -> bool {
        match (&self.modifier, access) {
            (_, AccessModifier::Force) => true,
            (VisibilityModifier::Public, AccessModifier::Auto | AccessModifier::Public) => true,
            // compatible example:
            //   def_namespace: <module>::C
            //   namespace: <module>::C::f
            (VisibilityModifier::Private, AccessModifier::Auto | AccessModifier::Private) => {
                &self.def_namespace[..] == "<builtins>"
                    || namespace.name.starts_with(&self.def_namespace[..])
            }
            (
                VisibilityModifier::Restricted(namespaces),
                AccessModifier::Auto | AccessModifier::Private,
            ) => {
                namespace.name.starts_with(&self.def_namespace[..])
                    || namespaces.contains(&namespace.name)
            }
            (
                VisibilityModifier::SubtypeRestricted(typ),
                AccessModifier::Auto | AccessModifier::Private,
            ) => {
                namespace.name.starts_with(&self.def_namespace[..]) || {
                    let Some(space_t) = namespace.rec_get_self_t() else {
                        return false;
                    };
                    namespace.subtype_of(&space_t, typ)
                }
            }
            _ => false,
        }
    }
}

/// same structure as `Identifier`, but only for Record fields.
#[derive(Debug, Clone, Eq)]
pub struct Field {
    pub vis: VisibilityModifier,
    pub symbol: Str,
}

impl Immutable for Field {}

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
        write!(f, "{}{}", self.vis, self.symbol)
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
    pub const fn new(vis: VisibilityModifier, symbol: Str) -> Self {
        Field { vis, symbol }
    }

    pub fn private(symbol: Str) -> Self {
        Field::new(VisibilityModifier::Private, symbol)
    }

    pub fn public(symbol: Str) -> Self {
        Field::new(VisibilityModifier::Public, symbol)
    }

    pub fn is_const(&self) -> bool {
        self.symbol.starts_with(char::is_uppercase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erg_common::dict;

    #[test]
    fn test_std_key() {
        let dict = dict! {Str::ever("a") => 1, Str::rc("b") => 2};
        assert_eq!(dict.get("a"), Some(&1));
        assert_eq!(dict.get("b"), Some(&2));
        assert_eq!(dict.get(&Str::ever("b")), Some(&2));
        assert_eq!(dict.get(&Str::rc("b")), Some(&2));

        let dict = dict! {Field::private(Str::ever("a")) => 1, Field::public(Str::ever("b")) => 2};
        assert_eq!(dict.get("a"), Some(&1));
        assert_eq!(dict.get("b"), Some(&2));
    }
}
