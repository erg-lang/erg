use std::fmt;

use erg_common::set::Set;
use erg_common::vis::Visibility;
use erg_common::Str;
use Visibility::*;

use erg_parser::ast::DefId;

use crate::ty::{HasType, Type};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VarKind {
    Defined(DefId),
    Declared,
    // TODO: flatten
    Parameter { def_id: DefId, default: DefaultInfo },
    Auto,
    FixedAuto,
    DoesNotExist,
    Builtin,
}

impl VarKind {
    pub const fn parameter(def_id: DefId, default: DefaultInfo) -> Self {
        Self::Parameter { def_id, default }
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
}

/// Has information about the type, variability, visibility, and where the variable was defined (or declared, generated)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarInfo {
    pub t: Type,
    pub muty: Mutability,
    pub vis: Visibility,
    pub kind: VarKind,
    pub comptime_decos: Option<Set<Str>>,
    pub impl_of: Option<Type>,
    pub py_name: Option<Str>,
}

impl fmt::Display for VarInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VarInfo{{t: {}, muty: {:?}, vis: {:?}, kind: {:?}, py_name: {:?}}}",
            self.t, self.muty, self.vis, self.kind, self.py_name,
        )
    }
}

impl HasType for VarInfo {
    #[inline]
    fn ref_t(&self) -> &Type {
        &self.t
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        &mut self.t
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        None
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        None
    }
}

impl Default for VarInfo {
    fn default() -> Self {
        Self::const_default()
    }
}

impl VarInfo {
    pub const ILLEGAL: &'static Self = &Self::const_default();

    pub const fn const_default() -> Self {
        Self::new(
            Type::Untyped,
            Immutable,
            Private,
            VarKind::DoesNotExist,
            None,
            None,
            None,
        )
    }

    pub const fn new(
        t: Type,
        muty: Mutability,
        vis: Visibility,
        kind: VarKind,
        comptime_decos: Option<Set<Str>>,
        impl_of: Option<Type>,
        py_name: Option<Str>,
    ) -> Self {
        Self {
            t,
            muty,
            vis,
            kind,
            comptime_decos,
            impl_of,
            py_name,
        }
    }

    pub fn same_id_as(&self, id: DefId) -> bool {
        match self.kind {
            VarKind::Defined(i) | VarKind::Parameter { def_id: i, .. } => id == i,
            _ => false,
        }
    }
}
