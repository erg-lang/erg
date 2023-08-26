use std::fmt;
use std::path::PathBuf;

use erg_common::error::Location;
use erg_common::set::Set;
use erg_common::{switch_lang, Str};

use erg_parser::ast::DefId;

use crate::context::DefaultInfo;
use crate::ty::{Field, HasType, Type, Visibility};

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
    InstanceAttr,
    Parameter {
        def_id: DefId,
        var: bool,
        default: DefaultInfo,
    },
    Auto,
    FixedAuto,
    DoesNotExist,
    Builtin,
}

impl VarKind {
    pub const fn parameter(def_id: DefId, var: bool, default: DefaultInfo) -> Self {
        Self::Parameter {
            def_id,
            var,
            default,
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

    pub const fn is_var_params(&self) -> bool {
        match self {
            Self::Parameter { var, .. } => *var,
            _ => false,
        }
    }

    pub const fn is_defined(&self) -> bool {
        matches!(self, Self::Defined(_))
    }

    pub const fn does_not_exist(&self) -> bool {
        matches!(self, Self::DoesNotExist)
    }

    pub const fn is_builtin(&self) -> bool {
        matches!(self, Self::Builtin)
    }

    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    pub const fn is_instance_attr(&self) -> bool {
        matches!(self, Self::InstanceAttr)
    }

    pub const fn display(&self) -> &'static str {
        match self {
            Self::Auto | Self::FixedAuto => switch_lang!(
                "japanese" => "自動",
                "simplified_chinese" => "自动",
                "traditional_chinese" => "自動",
                "english" => "auto",
            ),
            Self::Builtin => switch_lang!(
                "japanese" => "組み込み",
                "simplified_chinese" => "内置",
                "traditional_chinese" => "內置",
                "english" => "builtin",
            ),
            Self::InstanceAttr => switch_lang!(
                "japanese" => "インスタンス",
                "simplified_chinese" => "实例",
                "traditional_chinese" => "實例",
                "english" => "instance",
            ),
            _ => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbsLocation {
    pub module: Option<PathBuf>,
    pub loc: Location,
}

impl fmt::Display for AbsLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(module) = &self.module {
            write!(f, "{}@{}", module.display(), self.loc)
        } else {
            write!(f, "?@{}", self.loc)
        }
    }
}

impl std::str::FromStr for AbsLocation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('@');
        let module = split.next().map(PathBuf::from);
        let loc = split.next().ok_or(())?.parse().map_err(|_| ())?;
        Ok(Self { module, loc })
    }
}

impl AbsLocation {
    pub const fn new(module: Option<PathBuf>, loc: Location) -> Self {
        Self { module, loc }
    }

    pub const fn unknown() -> Self {
        Self::new(None, Location::Unknown)
    }

    pub fn code(&self) -> Option<String> {
        use std::io::{BufRead, BufReader};
        self.module.as_ref().and_then(|module| {
            let file = std::fs::File::open(module).ok()?;
            let reader = BufReader::new(file);
            reader
                .lines()
                .nth(
                    self.loc
                        .ln_begin()
                        .map(|l| l.saturating_sub(1))
                        .unwrap_or(0) as usize,
                )
                .and_then(|res| {
                    let res = res.ok()?;
                    let begin = self.loc.col_begin().unwrap_or(0) as usize;
                    let end = self.loc.col_end().unwrap_or(0) as usize;
                    if begin > res.len() || end > res.len() || begin > end {
                        return None;
                    }
                    let end = end.min(res.len());
                    let res = res[begin..end].to_string();
                    Some(res)
                })
        })
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
    pub def_loc: AbsLocation,
}

impl fmt::Display for VarInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VarInfo{{t: {}, muty: {:?}, vis: {:?}, kind: {:?}, py_name: {:?}, def_loc: {} }}",
            self.t, self.muty, self.vis, self.kind, self.py_name, self.def_loc
        )
    }
}

impl HasType for VarInfo {
    #[inline]
    fn ref_t(&self) -> &Type {
        &self.t
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        Some(&mut self.t)
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
        Self::const_default_private()
    }
}

impl VarInfo {
    pub const ILLEGAL: Self = Self::const_default_private();

    pub const fn const_default_private() -> Self {
        Self::new(
            Type::Failure,
            Immutable,
            Visibility::DUMMY_PRIVATE,
            VarKind::DoesNotExist,
            None,
            None,
            None,
            AbsLocation::unknown(),
        )
    }

    pub const fn const_default_public() -> Self {
        Self::new(
            Type::Failure,
            Immutable,
            Visibility::DUMMY_PUBLIC,
            VarKind::DoesNotExist,
            None,
            None,
            None,
            AbsLocation::unknown(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        t: Type,
        muty: Mutability,
        vis: Visibility,
        kind: VarKind,
        comptime_decos: Option<Set<Str>>,
        impl_of: Option<Type>,
        py_name: Option<Str>,
        def_loc: AbsLocation,
    ) -> Self {
        Self {
            t,
            muty,
            vis,
            kind,
            comptime_decos,
            impl_of,
            py_name,
            def_loc,
        }
    }

    pub fn same_id_as(&self, id: DefId) -> bool {
        match self.kind {
            VarKind::Defined(i) | VarKind::Parameter { def_id: i, .. } => id == i,
            _ => false,
        }
    }

    pub fn nd_parameter(t: Type, def_loc: AbsLocation, namespace: Str) -> Self {
        let kind = VarKind::Parameter {
            def_id: DefId(0),
            var: false,
            default: DefaultInfo::NonDefault,
        };
        Self::new(
            t,
            Immutable,
            Visibility::private(namespace),
            kind,
            None,
            None,
            None,
            def_loc,
        )
    }

    pub fn d_parameter(t: Type, def_loc: AbsLocation, namespace: Str) -> Self {
        let kind = VarKind::Parameter {
            def_id: DefId(0),
            var: false,
            default: DefaultInfo::WithDefault,
        };
        Self::new(
            t,
            Immutable,
            Visibility::private(namespace),
            kind,
            None,
            None,
            None,
            def_loc,
        )
    }

    pub fn instance_attr(field: Field, t: Type, impl_of: Option<Type>, namespace: Str) -> Self {
        let muty = if field.is_const() {
            Mutability::Const
        } else {
            Mutability::Immutable
        };
        Self::new(
            t,
            muty,
            Visibility::new(field.vis, namespace),
            VarKind::InstanceAttr,
            None,
            impl_of,
            None,
            AbsLocation::unknown(),
        )
    }

    pub fn type_var(t: Type, def_loc: AbsLocation, namespace: Str) -> Self {
        Self::new(
            t,
            Const,
            Visibility::private(namespace),
            VarKind::Declared,
            None,
            None,
            None,
            def_loc,
        )
    }

    pub fn record_field(t: Type, def_loc: AbsLocation, vis: Visibility) -> Self {
        Self::new(
            t,
            Immutable,
            vis,
            VarKind::Declared,
            None,
            None,
            None,
            def_loc,
        )
    }

    pub fn is_untyped_parameter(&self) -> bool {
        self.kind.is_parameter() && self.t.is_unbound_var()
    }
}
