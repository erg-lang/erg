use std::fmt;

use common::Str;
use common::ty::{Type};
use common::traits::HasType;

use parser::ast::DefId;

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
        } else { Self::Immutable }
    }
}

impl Mutability {
    pub const fn is_const(&self) -> bool { matches!(self, Self::Const) }
}

use Mutability::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Visibility {
    Private,
    Public,
}

impl Visibility {
    pub const fn is_public(&self) -> bool { matches!(self, Self::Public) }
    pub const fn is_private(&self) -> bool { matches!(self, Self::Private) }
}

use Visibility::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParamId {
    /// 変数でないパターン
    /// e.g. `[x, y]` of `f [x, y], z = ...`
    PatNonDefault(usize),
    /// e.g. `[x, y]` of `f [x, y] |= [0, 1] = ...`
    PatWithDefault(usize),
    /// 変数パターン
    /// e.g. `z` of `f [x, y], z = ...`
    VarNonDefault{ keyword: Str, pos: usize },
    /// e.g. `z` of `f [x, y], z |= 0 = ...`
    VarWithDefault{ keyword: Str, pos: usize },
    /// パターンに埋め込まれた変数パターン
    /// この場合デフォルト値はない
    /// e.g. `x` or `y` of `f [x, y], z = ...`
    Embedded(Str),
}

impl ParamId {
    pub const fn var_default(keyword: Str, pos: usize) -> Self { Self::VarWithDefault{ keyword, pos } }
    pub const fn var_non_default(keyword: Str, pos: usize) -> Self { Self::VarNonDefault{ keyword, pos } }

    pub const fn pos(&self) -> Option<usize> {
        match self {
            Self::PatNonDefault(pos)
            | Self::PatWithDefault(pos)
            | Self::VarNonDefault{ pos, .. }
            | Self::VarWithDefault{ pos, .. } => Some(*pos),
            _ => None,
        }
    }

    pub const fn has_default(&self) -> bool {
        matches!(self, Self::PatWithDefault(_) | Self::VarWithDefault{ .. })
    }

    pub const fn is_embedded(&self) -> bool { matches!(self, Self::Embedded(_)) }
}

#[derive(Debug, Clone,  PartialEq, Eq, Hash)]
pub enum VarKind {
    Defined(DefId),
    Declared,
    Parameter{ def_id: DefId, param_id: ParamId },
    Generated,
    DoesNotExist,
    Builtin,
}

impl VarKind {
    pub const fn parameter(def_id: DefId, param_id: ParamId) -> Self {
        Self::Parameter{ def_id, param_id }
    }

    pub const fn pos_as_param(&self) -> Option<usize> {
        match self {
            Self::Parameter{ param_id, .. } => param_id.pos(),
            _ => None,
        }
    }

    pub const fn has_default(&self) -> bool {
        match self {
            Self::Parameter{ param_id, .. } => param_id.has_default(),
            _ => false,
        }
    }

    pub const fn is_parameter(&self) -> bool {
        matches!(self, Self::Parameter{ .. })
    }

    pub const fn is_embedded_param(&self) -> bool {
        matches!(self, Self::Parameter{ param_id, .. } if param_id.is_embedded())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarInfo {
    pub t: Type,
    pub muty: Mutability,
    pub vis: Visibility,
    pub kind: VarKind,
}

impl fmt::Display for VarInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VarInfo{{t: {}, muty: {:?}, vis: {:?} kind: {:?}}}", self.t, self.muty, self.vis, self.kind)
    }
}

impl HasType for VarInfo {
    #[inline]
    fn ref_t(&self) -> &Type { &self.t }
    #[inline]
    fn signature_t(&self) -> Option<&Type> { None }
}

impl VarInfo {
    pub const ILLEGAL: &'static Self = &VarInfo::new(Type::Failure, Immutable, Private, VarKind::DoesNotExist);

    pub const fn new(t: Type, muty: Mutability, vis: Visibility, kind: VarKind) -> Self {
        Self { t, muty, vis, kind }
    }

    pub fn same_id_as(&self, id: DefId) -> bool {
        match self.kind {
            VarKind::Defined(i) | VarKind::Parameter{ def_id: i, .. } => id == i,
            _ => false,
        }
    }
}
