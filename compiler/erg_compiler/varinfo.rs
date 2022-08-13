use std::fmt;

use erg_common::traits::HasType;
use erg_common::ty::Type;

use erg_parser::ast::DefId;

use crate::context::DefaultInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Mutability {
    Immutable,
    Const,
}

impl From<&str> for Mutability {
    fn from(item: &str) -> Self {
        if item.chars().next().unwrap().is_uppercase() {
            Self::Const
        } else {
            Self::Immutable
        }
    }
}

impl Mutability {
    pub const fn is_const(&self) -> bool {
        matches!(self, Self::Const)
    }
}

use Mutability::*;

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

use Visibility::*;

/// e.g.
/// ```
/// K(T, [U, V]) = ...
/// U.idx == Nested(Just(1), 0)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParamIdx {
    Nth(usize),
    Nested(Box<ParamIdx>, usize),
}

impl ParamIdx {
    pub fn nested(outer: ParamIdx, nth: usize) -> Self {
        Self::Nested(Box::new(outer), nth)
    }

    pub const fn is_nested(&self) -> bool {
        matches!(self, Self::Nested(_, _))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VarKind {
    Defined(DefId),
    Declared,
    Parameter { def_id: DefId, idx: ParamIdx, default: DefaultInfo },
    Generated,
    DoesNotExist,
    Builtin,
}

impl VarKind {
    pub const fn parameter(def_id: DefId, idx: ParamIdx, default: DefaultInfo) -> Self {
        Self::Parameter { def_id, idx, default }
    }

    pub const fn idx(&self) -> Option<&ParamIdx> {
        match self {
            Self::Parameter { idx, .. } => Some(idx),
            _ => None,
        }
    }

    pub const fn has_default(&self) -> bool {
        match self {
            Self::Parameter { default, .. } => default.has_default(),
            _ => false,
        }
    }

    pub const fn is_parameter(&self) -> bool {
        matches!(self, Self::Parameter { .. })
    }

    pub const fn is_nested_param(&self) -> bool {
        matches!(self, Self::Parameter{ idx, .. } if idx.is_nested())
    }
}

/// Has information about the type, variability, visibility, and where the variable was defined (or declared, generated)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarInfo {
    pub t: Type,
    pub muty: Mutability,
    pub vis: Visibility,
    pub kind: VarKind,
}

impl fmt::Display for VarInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VarInfo{{t: {}, muty: {:?}, vis: {:?}, kind: {:?}}}",
            self.t, self.muty, self.vis, self.kind
        )
    }
}

impl HasType for VarInfo {
    #[inline]
    fn ref_t(&self) -> &Type {
        &self.t
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        None
    }
}

impl VarInfo {
    pub const ILLEGAL: &'static Self =
        &VarInfo::new(Type::Failure, Immutable, Private, VarKind::DoesNotExist);

    pub const fn new(t: Type, muty: Mutability, vis: Visibility, kind: VarKind) -> Self {
        Self { t, muty, vis, kind }
    }

    pub fn same_id_as(&self, id: DefId) -> bool {
        match self.kind {
            VarKind::Defined(i) | VarKind::Parameter { def_id: i, .. } => id == i,
            _ => false,
        }
    }
}
