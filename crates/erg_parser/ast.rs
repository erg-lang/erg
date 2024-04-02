//! defines `Expr` (Expression, the minimum executing unit of Erg).
use std::borrow::Borrow;
use std::fmt;
use std::fmt::Write as _;

use erg_common::consts::ERG_MODE;
use erg_common::error::Location;
use erg_common::io::Input;
use erg_common::set::Set as HashSet;
// use erg_common::dict::Dict as HashMap;
use erg_common::traits::{Locational, NestedDisplay, Stream};
use erg_common::{
    fmt_option, fmt_vec, impl_display_for_enum, impl_display_from_nested,
    impl_displayable_stream_for_wrapper, impl_from_trait_for_enum, impl_locational,
    impl_locational_for_enum, impl_nested_display_for_chunk_enum, impl_nested_display_for_enum,
    impl_stream,
};
use erg_common::{fmt_vec_split_with, Str};

use crate::token::{Token, TokenKind, EQUAL};

#[cfg(not(feature = "pylib"))]
use erg_proc_macros::staticmethod as to_owned;
#[cfg(feature = "pylib")]
use erg_proc_macros::to_owned;
#[cfg(not(feature = "pylib"))]
use erg_proc_macros::{getter, pyclass, pymethods, pyo3, setter, staticmethod};
#[cfg(feature = "pylib")]
use pyo3::prelude::*;

macro_rules! impl_into_py_for_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        #[cfg(feature = "pylib")]
        impl IntoPy<PyObject> for $Enum {
            fn into_py(self, py: Python<'_>) -> PyObject {
                match self {
                    $(Self::$Variant(v) => v.into_py(py),)*
                }
            }
        }
    };
}

macro_rules! impl_from_py_for_enum {
    ($Ty: ty; $($Variant: ident ($inner: ident) $(,)*)*) => {
        #[cfg(feature = "pylib")]
        impl FromPyObject<'_> for $Ty {
            fn extract(ob: &PyAny) -> PyResult<Self> {
                $(if let Ok(extracted) = ob.extract::<$inner>() {
                    Ok(Self::$Variant(extracted))
                } else)* {
                    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                        format!("expected one of {:?}, but got {}", &[$(stringify!($Variant),)*], ob.get_type().name()?),
                    ))
                }
            }
        }
    };
    ($Ty: ty; $($Variant: ident $(,)*)*) => {
        #[cfg(feature = "pylib")]
        impl FromPyObject<'_> for $Ty {
            fn extract(ob: &PyAny) -> PyResult<Self> {
                $(if let Ok(extracted) = ob.extract::<$Variant>() {
                    Ok(Self::$Variant(extracted))
                } else)* {
                    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                        format!("expected one of {:?}, but got {}", &[$(stringify!($Variant),)*], ob.get_type().name()?),
                    ))
                }
            }
        }
    };
}

macro_rules! impl_py_iter {
    ($Ty: ident <$inner: ident>, $Iter: ident, 0) => {
        #[cfg(feature = "pylib")]
        #[pyclass]
        struct $Iter {
            inner: std::vec::IntoIter<$inner>,
        }

        #[cfg(feature = "pylib")]
        #[pymethods]
        impl $Iter {
            #[allow(clippy::self_named_constructors)]
            fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
                slf
            }
            fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<$inner> {
                slf.inner.next()
            }
        }

        #[cfg(feature = "pylib")]
        #[pymethods]
        impl $Ty {
            fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<$Iter>> {
                let iter = $Iter {
                    inner: slf.clone().into_iter(),
                };
                Py::new(slf.py(), iter)
            }

            #[pyo3(name = "pop")]
            fn _pop(mut slf: PyRefMut<'_, Self>) -> Option<$inner> {
                slf.0.pop()
            }

            #[pyo3(name = "push")]
            fn _push(mut slf: PyRefMut<'_, Self>, item: $inner) {
                slf.0.push(item);
            }

            #[pyo3(name = "remove")]
            fn _remove(mut slf: PyRefMut<'_, Self>, idx: usize) -> $inner {
                slf.0.remove(idx)
            }

            #[pyo3(name = "insert")]
            fn _insert(mut slf: PyRefMut<'_, Self>, idx: usize, item: $inner) {
                slf.0.insert(idx, item);
            }
        }
    };
    ($Ty: ident <$inner: ident>, $Iter: ident, $attr: ident) => {
        #[cfg(feature = "pylib")]
        #[pyclass]
        struct $Iter {
            inner: std::vec::IntoIter<$inner>,
        }

        #[cfg(feature = "pylib")]
        #[pymethods]
        impl $Iter {
            #[allow(clippy::self_named_constructors)]
            fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
                slf
            }
            fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<$inner> {
                slf.inner.next()
            }
        }

        #[cfg(feature = "pylib")]
        #[pymethods]
        impl $Ty {
            fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<$Iter>> {
                let iter = $Iter {
                    inner: slf.clone().into_iter(),
                };
                Py::new(slf.py(), iter)
            }

            #[pyo3(name = "pop")]
            fn _pop(mut slf: PyRefMut<'_, Self>) -> Option<$inner> {
                slf.$attr.pop()
            }

            #[pyo3(name = "push")]
            fn _push(mut slf: PyRefMut<'_, Self>, item: $inner) {
                slf.$attr.push(item);
            }

            #[pyo3(name = "remove")]
            fn _remove(mut slf: PyRefMut<'_, Self>, idx: usize) -> $inner {
                slf.$attr.remove(idx)
            }

            #[pyo3(name = "insert")]
            fn _insert(mut slf: PyRefMut<'_, Self>, idx: usize, item: $inner) {
                slf.$attr.insert(idx, item);
            }
        }
    };
    ($Ty: ident <$inner: ident>, $Iter: ident) => {
        #[cfg(feature = "pylib")]
        #[pyclass]
        struct $Iter {
            inner: std::vec::IntoIter<$inner>,
        }

        #[cfg(feature = "pylib")]
        #[pymethods]
        impl $Iter {
            #[allow(clippy::self_named_constructors)]
            fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
                slf
            }
            fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<$inner> {
                slf.inner.next()
            }
        }

        #[cfg(feature = "pylib")]
        #[pymethods]
        impl $Ty {
            fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<$Iter>> {
                let iter = $Iter {
                    inner: slf.clone().into_iter(),
                };
                Py::new(slf.py(), iter)
            }
        }
    };
}

/// Some Erg functions require additional operation by the compiler.
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Import,
    PyImport,
    RsImport,
    Del,
    Assert,
    Class,
    Inherit,
    Trait,
    Subsume,
    Return,
    Yield,
    Cast,
}

#[pymethods]
impl OperationKind {
    pub const fn is_erg_import(&self) -> bool {
        matches!(self, Self::Import)
    }
    pub const fn is_py_import(&self) -> bool {
        matches!(self, Self::PyImport)
    }
    pub const fn is_import(&self) -> bool {
        matches!(self, Self::Import | Self::PyImport | Self::RsImport)
    }
}

pub fn fmt_lines<'a, T: NestedDisplay + 'a>(
    mut iter: impl Iterator<Item = &'a T>,
    f: &mut fmt::Formatter<'_>,
    level: usize,
) -> fmt::Result {
    if let Some(first) = iter.next() {
        first.fmt_nest(f, level)?;
    }
    for arg in iter {
        writeln!(f)?;
        arg.fmt_nest(f, level)?;
    }
    Ok(())
}

/// リテラルに実際の値が格納された構造体(定数畳み込み用)
/// ArrayやDictはまた別に
#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Literal {
    pub token: Token,
}

impl NestedDisplay for Literal {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", self.token.content)
    }
}

impl_display_from_nested!(Literal);
impl_locational!(Literal, token);

impl From<Token> for Literal {
    #[inline]
    fn from(token: Token) -> Self {
        Self { token }
    }
}

impl Literal {
    pub fn str(s: impl Into<Str>, line: u32) -> Self {
        let token = Token::new_fake(TokenKind::StrLit, s, line, 0, 0);
        Self { token }
    }
}

#[pymethods]
impl Literal {
    #[staticmethod]
    pub const fn new(token: Token) -> Self {
        Self { token }
    }

    #[staticmethod]
    pub fn nat(n: usize, line: u32) -> Self {
        let token = Token::new_fake(TokenKind::NatLit, Str::from(n.to_string()), line, 0, 0);
        Self { token }
    }

    #[staticmethod]
    pub fn bool(b: bool, line: u32) -> Self {
        let b = if b { "True" } else { "False" };
        let token = Token::new_fake(TokenKind::BoolLit, b, line, 0, 0);
        Self { token }
    }

    #[inline]
    pub fn is(&self, kind: TokenKind) -> bool {
        self.token.is(kind)
    }

    #[inline]
    pub fn is_doc_comment(&self) -> bool {
        self.token.is(TokenKind::DocComment)
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PosArg {
    pub expr: Expr,
}

impl NestedDisplay for PosArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        self.expr.fmt_nest(f, level)
    }
}

impl_display_from_nested!(PosArg);
impl_locational!(PosArg, expr);

impl PosArg {
    pub const fn new(expr: Expr) -> Self {
        Self { expr }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KwArg {
    pub keyword: Token,
    pub t_spec: Option<TypeSpecWithOp>,
    pub expr: Expr,
}

impl NestedDisplay for KwArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(
            f,
            "{}{} := {}",
            self.keyword.content,
            fmt_option!(self.t_spec),
            self.expr,
        )
    }
}

impl_display_from_nested!(KwArg);
impl_locational!(KwArg, keyword, expr);

#[pymethods]
impl KwArg {
    #[staticmethod]
    #[pyo3(signature = (keyword, t_spec, expr))]
    pub const fn new(keyword: Token, t_spec: Option<TypeSpecWithOp>, expr: Expr) -> Self {
        Self {
            keyword,
            t_spec,
            expr,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Args {
    pub pos_args: Vec<PosArg>,
    pub var_args: Option<Box<PosArg>>,
    pub kw_args: Vec<KwArg>,
    pub kw_var_args: Option<Box<PosArg>>,
    // these are for ELS
    pub paren: Option<(Token, Token)>,
}

impl NestedDisplay for Args {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        fmt_lines(self.pos_args.iter(), f, level)?;
        writeln!(f)?;
        if let Some(var) = &self.var_args {
            writeln!(f, "*{var}")?;
        }
        fmt_lines(self.kw_args.iter(), f, level)?;
        if let Some(var) = &self.kw_var_args {
            writeln!(f)?;
            write!(f, "**{var}")?;
        }
        Ok(())
    }
}

impl_display_from_nested!(Args);

impl Locational for Args {
    fn loc(&self) -> Location {
        if let Some((l, r)) = &self.paren {
            let loc = Location::concat(l, r);
            if !loc.is_unknown() {
                return loc;
            }
        }
        match (
            self.pos_args.first().zip(self.pos_args.last()),
            self.var_args.as_ref(),
            self.kw_args.first().zip(self.kw_args.last()),
        ) {
            (Some((l, _)), _, Some((_, r))) => Location::concat(l, r),
            (Some((l, _)), Some(r), None) => Location::concat(l, r.as_ref()),
            (Some((l, r)), None, None) => Location::concat(l, r),
            (None, Some(l), Some((_, r))) => Location::concat(l.as_ref(), r),
            (None, None, Some((l, r))) => Location::concat(l, r),
            (None, Some(l), None) => l.loc(),
            (None, None, None) => Location::Unknown,
        }
    }
}

// impl_stream!(Args, Arg, args);

impl Args {
    // for replacing to hir::Args
    #[allow(clippy::type_complexity)]
    pub fn deconstruct(
        self,
    ) -> (
        Vec<PosArg>,
        Option<PosArg>,
        Vec<KwArg>,
        Option<PosArg>,
        Option<(Token, Token)>,
    ) {
        (
            self.pos_args,
            self.var_args.map(|x| *x),
            self.kw_args,
            self.kw_var_args.map(|x| *x),
            self.paren,
        )
    }

    pub fn into_iters(
        self,
    ) -> (
        impl IntoIterator<Item = PosArg>,
        impl IntoIterator<Item = KwArg>,
    ) {
        (self.pos_args.into_iter(), self.kw_args.into_iter())
    }

    pub fn extend_pos<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = PosArg>,
    {
        self.pos_args.extend(iter);
    }

    pub fn pos_args(&self) -> &[PosArg] {
        &self.pos_args[..]
    }
}

#[pymethods]
impl Args {
    #[staticmethod]
    #[pyo3(signature = (pos_args, var_args, kw_args, kw_var_args=None, paren=None))]
    pub fn new(
        pos_args: Vec<PosArg>,
        var_args: Option<PosArg>,
        kw_args: Vec<KwArg>,
        kw_var_args: Option<PosArg>,
        paren: Option<(Token, Token)>,
    ) -> Self {
        Self {
            pos_args,
            var_args: var_args.map(Box::new),
            kw_args,
            kw_var_args: kw_var_args.map(Box::new),
            paren,
        }
    }

    #[getter]
    #[pyo3(name = "pos_args")]
    fn _pos_args(&self) -> Vec<PosArg> {
        self.pos_args.clone()
    }

    #[getter]
    pub fn var_args(&self) -> Option<PosArg> {
        self.var_args.as_ref().map(|x| *x.clone())
    }

    #[getter]
    #[pyo3(name = "kw_args")]
    fn _kw_args(&self) -> Vec<KwArg> {
        self.kw_args.clone()
    }

    #[getter]
    pub fn kw_var_args(&self) -> Option<PosArg> {
        self.kw_var_args.as_ref().map(|x| *x.clone())
    }

    #[staticmethod]
    pub fn pos_only(pos_arg: Vec<PosArg>, paren: Option<(Token, Token)>) -> Self {
        Self::new(pos_arg, None, vec![], None, paren)
    }

    #[staticmethod]
    pub fn single(pos_args: PosArg) -> Self {
        Self::pos_only(vec![pos_args], None)
    }

    #[staticmethod]
    pub fn empty() -> Self {
        Self::new(vec![], None, vec![], None, None)
    }

    pub fn is_empty(&self) -> bool {
        self.pos_args.is_empty() && self.kw_args.is_empty()
    }

    pub fn len(&self) -> usize {
        self.pos_args.len() + self.kw_args.len()
    }

    pub fn kw_is_empty(&self) -> bool {
        self.kw_args.is_empty()
    }

    #[to_owned]
    pub fn kw_args(&self) -> &[KwArg] {
        &self.kw_args[..]
    }

    pub fn has_pos_arg(&self, pa: &PosArg) -> bool {
        self.pos_args.contains(pa)
    }

    pub fn push_pos(&mut self, arg: PosArg) {
        self.pos_args.push(arg);
    }

    pub fn remove_pos(&mut self, index: usize) -> PosArg {
        self.pos_args.remove(index)
    }

    pub fn insert_pos(&mut self, index: usize, arg: PosArg) {
        self.pos_args.insert(index, arg);
    }

    pub fn set_var_args(&mut self, arg: PosArg) {
        self.var_args = Some(Box::new(arg));
    }

    pub fn push_kw(&mut self, arg: KwArg) {
        self.kw_args.push(arg);
    }

    pub fn set_kw_var(&mut self, arg: PosArg) {
        self.kw_var_args = Some(Box::new(arg));
    }

    pub fn set_parens(&mut self, paren: (Token, Token)) {
        self.paren = Some(paren);
    }

    #[to_owned(cloned)]
    pub fn get_left_or_key(&self, key: &str) -> Option<&Expr> {
        if !self.pos_args.is_empty() {
            self.pos_args.first().map(|a| &a.expr)
        } else {
            self.kw_args.iter().find_map(|a| {
                if &a.keyword.content[..] == key {
                    Some(&a.expr)
                } else {
                    None
                }
            })
        }
    }

    #[to_owned(cloned)]
    pub fn nth_or_key(&self, nth: usize, key: &str) -> Option<&Expr> {
        if !self.pos_args.is_empty() {
            self.pos_args.get(nth).map(|a| &a.expr)
        } else {
            self.kw_args.iter().find_map(|a| {
                if &a.keyword.content[..] == key {
                    Some(&a.expr)
                } else {
                    None
                }
            })
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Attribute {
    pub obj: Box<Expr>,
    pub ident: Identifier,
}

impl NestedDisplay for Attribute {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({}){}", self.obj, self.ident)
        } else {
            write!(f, "{}{}", self.obj, self.ident)
        }
    }
}

impl_display_from_nested!(Attribute);
impl_locational!(Attribute, obj, ident);

#[pymethods]
impl Attribute {
    #[getter]
    pub fn obj(&self) -> Expr {
        self.obj.as_ref().clone()
    }

    #[getter]
    pub fn ident(&self) -> Identifier {
        self.ident.clone()
    }

    #[staticmethod]
    pub fn new(obj: Expr, ident: Identifier) -> Self {
        Self {
            obj: Box::new(obj),
            ident,
        }
    }
}

#[pyclass]
/// e.g. obj.0, obj.1
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TupleAttribute {
    pub obj: Box<Expr>,
    pub index: Literal,
}

impl NestedDisplay for TupleAttribute {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({}).{}", self.obj, self.index)
        } else {
            write!(f, "{}.{}", self.obj, self.index)
        }
    }
}

impl_display_from_nested!(TupleAttribute);
impl_locational!(TupleAttribute, obj, index);

#[pymethods]
impl TupleAttribute {
    #[getter]
    pub fn obj(&self) -> Expr {
        self.obj.as_ref().clone()
    }

    #[getter]
    pub fn index(&self) -> Literal {
        self.index.clone()
    }

    #[staticmethod]
    pub fn new(obj: Expr, index: Literal) -> Self {
        Self {
            obj: Box::new(obj),
            index,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Subscript {
    pub obj: Box<Expr>,
    pub index: Box<Expr>,
    pub r_sqbr: Token,
}

impl NestedDisplay for Subscript {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({})[{}]", self.obj, self.index)
        } else {
            write!(f, "{}[{}]", self.obj, self.index)
        }
    }
}

impl_display_from_nested!(Subscript);
impl_locational!(Subscript, obj, r_sqbr);

#[pymethods]
impl Subscript {
    #[getter]
    pub fn obj(&self) -> Expr {
        self.obj.as_ref().clone()
    }

    #[getter]
    pub fn index(&self) -> Expr {
        self.index.as_ref().clone()
    }

    #[staticmethod]
    pub fn new(obj: Expr, index: Expr, r_sqbr: Token) -> Self {
        Self {
            obj: Box::new(obj),
            index: Box::new(index),
            r_sqbr,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeAppArgsKind {
    SubtypeOf(Box<TypeSpecWithOp>),
    Args(Args),
}

#[cfg(feature = "pylib")]
impl IntoPy<PyObject> for TypeAppArgsKind {
    fn into_py(self, py: Python<'_>) -> PyObject {
        match self {
            Self::SubtypeOf(ty) => ty.into_py(py),
            Self::Args(args) => args.into_py(py),
        }
    }
}

#[cfg(feature = "pylib")]
impl FromPyObject<'_> for TypeAppArgsKind {
    fn extract(ob: &PyAny) -> PyResult<Self> {
        if let Ok(ty) = ob.extract::<TypeSpecWithOp>() {
            Ok(Self::SubtypeOf(Box::new(ty)))
        } else if let Ok(args) = ob.extract::<Args>() {
            Ok(Self::Args(args))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "expected one of {:?}",
                &["TypeSpecWithOp", "Args"]
            )))
        }
    }
}

impl NestedDisplay for TypeAppArgsKind {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        match self {
            Self::SubtypeOf(ty) => write!(f, "{ty}"),
            Self::Args(args) => write!(f, "{args}"),
        }
    }
}

impl_display_from_nested!(TypeAppArgsKind);
impl_locational_for_enum!(TypeAppArgsKind; SubtypeOf, Args);

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeAppArgs {
    pub l_vbar: Token,
    pub args: TypeAppArgsKind,
    pub r_vbar: Token,
}

impl NestedDisplay for TypeAppArgs {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "|{}|", self.args)
    }
}

impl_display_from_nested!(TypeAppArgs);
impl_locational!(TypeAppArgs, l_vbar, args, r_vbar);

#[pymethods]
impl TypeAppArgs {
    #[staticmethod]
    pub const fn new(l_vbar: Token, args: TypeAppArgsKind, r_vbar: Token) -> Self {
        Self {
            l_vbar,
            args,
            r_vbar,
        }
    }
}

/// f|T := Int|
#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeApp {
    pub obj: Box<Expr>,
    pub type_args: TypeAppArgs,
}

impl NestedDisplay for TypeApp {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({}){}", self.obj, self.type_args)
        } else {
            write!(f, "{}{}", self.obj, self.type_args)
        }
    }
}

impl_display_from_nested!(TypeApp);
impl_locational!(TypeApp, obj, type_args);

#[pymethods]
impl TypeApp {
    #[staticmethod]
    pub fn new(obj: Expr, type_args: TypeAppArgs) -> Self {
        Self {
            obj: Box::new(obj),
            type_args,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Accessor {
    Ident(Identifier),
    Attr(Attribute),
    TupleAttr(TupleAttribute),
    Subscr(Subscript),
    TypeApp(TypeApp),
}

impl_nested_display_for_enum!(Accessor; Ident, Attr, TupleAttr, Subscr, TypeApp);
impl_display_from_nested!(Accessor);
impl_locational_for_enum!(Accessor; Ident, Attr, TupleAttr, Subscr, TypeApp);
impl_into_py_for_enum!(Accessor; Ident, Attr, TupleAttr, Subscr, TypeApp);
impl_from_py_for_enum!(Accessor; Ident(Identifier), Attr(Attribute), TupleAttr(TupleAttribute), Subscr(Subscript), TypeApp(TypeApp));

impl Accessor {
    pub const fn local(symbol: Token) -> Self {
        Self::Ident(Identifier::new(
            VisModifierSpec::Private,
            VarName::new(symbol),
        ))
    }

    pub const fn public(dot: Token, symbol: Token) -> Self {
        Self::Ident(Identifier::new(
            VisModifierSpec::Public(dot),
            VarName::new(symbol),
        ))
    }

    pub const fn explicit_local(dcolon: Token, symbol: Token) -> Self {
        Self::Ident(Identifier::new(
            VisModifierSpec::ExplicitPrivate(dcolon),
            VarName::new(symbol),
        ))
    }

    pub const fn restricted(rest: VisRestriction, symbol: Token) -> Self {
        Self::Ident(Identifier::new(
            VisModifierSpec::Restricted(rest),
            VarName::new(symbol),
        ))
    }

    pub fn attr(obj: Expr, ident: Identifier) -> Self {
        Self::Attr(Attribute::new(obj, ident))
    }

    pub fn tuple_attr(obj: Expr, index: Literal) -> Self {
        Self::TupleAttr(TupleAttribute::new(obj, index))
    }

    pub fn subscr(obj: Expr, index: Expr, r_sqbr: Token) -> Self {
        Self::Subscr(Subscript::new(obj, index, r_sqbr))
    }

    pub fn type_app(obj: Expr, type_args: TypeAppArgs) -> Self {
        Self::TypeApp(TypeApp::new(obj, type_args))
    }

    pub const fn name(&self) -> Option<&Str> {
        match self {
            Self::Ident(ident) => Some(ident.inspect()),
            _ => None,
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Ident(ident) => ident.is_const(),
            Self::Subscr(subscr) => subscr.obj.is_const_acc(),
            Self::TupleAttr(attr) => attr.obj.is_const_acc(),
            Self::Attr(attr) => attr.obj.is_const_acc() && attr.ident.is_const(),
            Self::TypeApp(app) => app.obj.is_const_acc(),
        }
    }

    pub fn to_str_literal(&self) -> Literal {
        match self {
            Self::Ident(ident) => ident.to_str_literal(),
            other => todo!("{other}"),
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalArray {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub elems: Args,
}

impl NestedDisplay for NormalArray {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "[")?;
        self.elems.fmt_nest(f, level + 1)?;
        write!(f, "{}]", "    ".repeat(level))
    }
}

impl_display_from_nested!(NormalArray);
impl_locational!(NormalArray, l_sqbr, elems, r_sqbr);

#[pymethods]
impl NormalArray {
    #[pyo3(name = "get")]
    fn _get(&self, index: usize) -> Option<Expr> {
        self.get(index).cloned()
    }

    #[staticmethod]
    pub const fn new(l_sqbr: Token, r_sqbr: Token, elems: Args) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elems,
        }
    }
}

impl NormalArray {
    pub fn get(&self, index: usize) -> Option<&Expr> {
        self.elems.pos_args.get(index).map(|a| &a.expr)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Expr> {
        self.elems.pos_args.iter().map(|a| &a.expr)
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayWithLength {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub elem: Box<PosArg>,
    pub len: Box<Expr>,
}

impl NestedDisplay for ArrayWithLength {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "[{}; {}]", self.elem, self.len)
    }
}

impl_display_from_nested!(ArrayWithLength);
impl_locational!(ArrayWithLength, l_sqbr, elem, r_sqbr);

#[pymethods]
impl ArrayWithLength {
    #[staticmethod]
    pub fn new(l_sqbr: Token, r_sqbr: Token, elem: PosArg, len: Expr) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elem: Box::new(elem),
            len: Box::new(len),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayComprehension {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub layout: Option<Box<Expr>>,
    pub generators: Vec<(Identifier, Expr)>,
    pub guard: Option<Box<Expr>>,
}

impl NestedDisplay for ArrayComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        let mut generators = String::new();
        for (name, gen) in self.generators.iter() {
            write!(generators, "{name} <- {gen}; ")?;
        }
        write!(
            f,
            "[{}{}{}]",
            fmt_option!(self.layout, post " | "),
            generators,
            fmt_option!(pre " | ", &self.guard)
        )
    }
}

impl_display_from_nested!(ArrayComprehension);
impl_locational!(ArrayComprehension, l_sqbr, r_sqbr);

#[pymethods]
impl ArrayComprehension {
    #[staticmethod]
    #[pyo3(signature = (l_sqbr, r_sqbr, layout, generators, guard=None))]
    pub fn new(
        l_sqbr: Token,
        r_sqbr: Token,
        layout: Option<Expr>,
        generators: Vec<(Identifier, Expr)>,
        guard: Option<Expr>,
    ) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            layout: layout.map(Box::new),
            generators,
            guard: guard.map(Box::new),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Array {
    Normal(NormalArray),
    WithLength(ArrayWithLength),
    Comprehension(ArrayComprehension),
}

impl_nested_display_for_enum!(Array; Normal, WithLength, Comprehension);
impl_display_for_enum!(Array; Normal, WithLength, Comprehension);
impl_locational_for_enum!(Array; Normal, WithLength, Comprehension);
impl_into_py_for_enum!(Array; Normal, WithLength, Comprehension);
impl_from_py_for_enum!(Array; Normal(NormalArray), WithLength(ArrayWithLength), Comprehension(ArrayComprehension));

impl Array {
    pub fn get(&self, index: usize) -> Option<&Expr> {
        match self {
            Self::Normal(array) => array.get(index),
            _ => None,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalTuple {
    pub elems: Args,
}

impl NestedDisplay for NormalTuple {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "(")?;
        self.elems.fmt_nest(f, level + 1)?;
        write!(f, "{})", "    ".repeat(level))
    }
}

impl_display_from_nested!(NormalTuple);
impl_locational!(NormalTuple, elems, elems);

impl From<NormalTuple> for Expr {
    fn from(tuple: NormalTuple) -> Self {
        Self::Tuple(Tuple::Normal(tuple))
    }
}

#[pymethods]
impl NormalTuple {
    #[staticmethod]
    pub const fn new(elems: Args) -> Self {
        Self { elems }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tuple {
    Normal(NormalTuple),
    // Comprehension(TupleComprehension),
}

impl_nested_display_for_enum!(Tuple; Normal);
impl_display_for_enum!(Tuple; Normal);
impl_locational_for_enum!(Tuple; Normal);
impl_into_py_for_enum!(Tuple; Normal);
impl_from_py_for_enum!(Tuple; Normal(NormalTuple));

impl Tuple {
    pub fn paren(&self) -> Option<&(Token, Token)> {
        match self {
            Self::Normal(tuple) => tuple.elems.paren.as_ref(),
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyValue {
    pub key: Expr,
    pub value: Expr,
}

impl NestedDisplay for KeyValue {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.value)
    }
}

impl_display_from_nested!(KeyValue);
impl_locational!(KeyValue, key, value);

#[pymethods]
impl KeyValue {
    #[staticmethod]
    pub const fn new(key: Expr, value: Expr) -> Self {
        Self { key, value }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NormalDict {
    pub l_brace: Token,
    pub r_brace: Token,
    pub kvs: Vec<KeyValue>,
}

impl NestedDisplay for NormalDict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", fmt_vec(&self.kvs))
    }
}

impl_display_from_nested!(NormalDict);
impl_locational!(NormalDict, l_brace, r_brace);

#[pymethods]
impl NormalDict {
    #[staticmethod]
    pub const fn new(l_brace: Token, r_brace: Token, kvs: Vec<KeyValue>) -> Self {
        Self {
            l_brace,
            r_brace,
            kvs,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DictComprehension {
    l_brace: Token,
    r_brace: Token,
    pub kv: Box<KeyValue>,
    pub generators: Vec<(Identifier, Expr)>,
    pub guard: Option<Box<Expr>>,
}

impl NestedDisplay for DictComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        let mut generators = String::new();
        for (name, gen) in self.generators.iter() {
            write!(generators, "{name} <- {gen}; ")?;
        }
        write!(
            f,
            "{{{} | {generators}{}}}",
            self.kv,
            fmt_option!(pre " | ", &self.guard)
        )
    }
}

impl_display_from_nested!(DictComprehension);
impl_locational!(DictComprehension, l_brace, kv, r_brace);

#[pymethods]
impl DictComprehension {
    #[staticmethod]
    pub fn new(
        l_brace: Token,
        r_brace: Token,
        kv: KeyValue,
        generators: Vec<(Identifier, Expr)>,
        guard: Option<Expr>,
    ) -> Self {
        Self {
            l_brace,
            r_brace,
            kv: Box::new(kv),
            generators,
            guard: guard.map(Box::new),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Dict {
    Normal(NormalDict),
    Comprehension(DictComprehension),
}

impl_nested_display_for_enum!(Dict; Normal, Comprehension);
impl_display_for_enum!(Dict; Normal, Comprehension);
impl_locational_for_enum!(Dict; Normal, Comprehension);
impl_into_py_for_enum!(Dict; Normal, Comprehension);
impl_from_py_for_enum!(Dict; Normal(NormalDict), Comprehension(DictComprehension));

impl Dict {
    pub fn braces(&self) -> (&Token, &Token) {
        match self {
            Self::Normal(dict) => (&dict.l_brace, &dict.r_brace),
            Self::Comprehension(dict) => (&dict.l_brace, &dict.r_brace),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClassAttr {
    Def(Def),
    Decl(TypeAscription),
    Doc(Literal),
}

impl_nested_display_for_enum!(ClassAttr; Def, Decl, Doc);
impl_display_for_enum!(ClassAttr; Def, Decl, Doc);
impl_locational_for_enum!(ClassAttr; Def, Decl, Doc);
impl_into_py_for_enum!(ClassAttr; Def, Decl, Doc);

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassAttrs(Vec<ClassAttr>);

impl_stream!(ClassAttrs, ClassAttr);

impl NestedDisplay for ClassAttrs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)?;
        writeln!(f)
    }
}

impl Locational for ClassAttrs {
    fn loc(&self) -> Location {
        if self.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
        }
    }
}

impl From<Vec<ClassAttr>> for ClassAttrs {
    fn from(attrs: Vec<ClassAttr>) -> Self {
        Self(attrs)
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordAttrs(Vec<Def>);

impl_py_iter!(RecordAttrs<Def>, DefIter);
impl_stream!(RecordAttrs, Def);

impl NestedDisplay for RecordAttrs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)?;
        writeln!(f)
    }
}

impl Locational for RecordAttrs {
    fn loc(&self) -> Location {
        if self.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
        }
    }
}

impl From<Vec<Def>> for RecordAttrs {
    fn from(attrs: Vec<Def>) -> Self {
        Self(attrs)
    }
}

impl RecordAttrs {
    pub fn get(&self, name: &str) -> Option<&Def> {
        self.0
            .iter()
            .find(|attr| attr.sig.ident().is_some_and(|n| n.inspect() == name))
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NormalRecord {
    pub l_brace: Token,
    pub r_brace: Token,
    pub attrs: RecordAttrs,
}

impl NestedDisplay for NormalRecord {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{{")?;
        self.attrs.fmt_nest(f, level + 1)?;
        writeln!(f, "{}}}", "    ".repeat(level))
    }
}

impl_display_from_nested!(NormalRecord);
impl_locational!(NormalRecord, l_brace, attrs, r_brace);

impl From<NormalRecord> for Expr {
    fn from(record: NormalRecord) -> Self {
        Self::Record(Record::Normal(record))
    }
}

impl From<MixedRecord> for NormalRecord {
    fn from(value: MixedRecord) -> Self {
        let mut attrs = vec![];
        for attr in value.attrs.into_iter() {
            match attr {
                RecordAttrOrIdent::Ident(ident) => {
                    let pat = VarPattern::Ident(ident.clone());
                    let sig = Signature::Var(VarSignature::new(pat, None));
                    let block = Block::new(vec![Expr::Accessor(Accessor::Ident(ident))]);
                    let body = DefBody::new(Token::DUMMY, block, DefId(0));
                    let def = Def::new(sig, body);
                    attrs.push(def);
                }
                RecordAttrOrIdent::Attr(def) => {
                    attrs.push(def);
                }
            }
        }
        Self::new(value.l_brace, value.r_brace, RecordAttrs::new(attrs))
    }
}

#[pymethods]
impl NormalRecord {
    #[pyo3(name = "get")]
    fn _get(&self, name: &str) -> Option<Def> {
        self.get(name).cloned()
    }

    #[pyo3(name = "keys")]
    fn _keys(&self) -> Vec<Identifier> {
        self.keys().cloned().collect()
    }

    #[staticmethod]
    pub const fn new(l_brace: Token, r_brace: Token, attrs: RecordAttrs) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
    }
}

impl NormalRecord {
    pub fn get(&self, name: &str) -> Option<&Def> {
        self.attrs.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Def> {
        self.attrs.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.attrs.iter().filter_map(|attr| match &attr.sig {
            Signature::Var(var) => var.pat.ident(),
            Signature::Subr(subr) => Some(&subr.ident),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Record {
    Normal(NormalRecord),
    Mixed(MixedRecord),
}

impl_nested_display_for_enum!(Record; Normal, Mixed);
impl_display_for_enum!(Record; Normal, Mixed);
impl_locational_for_enum!(Record; Normal, Mixed);
impl_into_py_for_enum!(Record; Normal, Mixed);
impl_from_py_for_enum!(Record; Normal(NormalRecord), Mixed(MixedRecord));

impl Record {
    pub const fn new_mixed(l_brace: Token, r_brace: Token, attrs: Vec<RecordAttrOrIdent>) -> Self {
        Self::Mixed(MixedRecord {
            l_brace,
            r_brace,
            attrs,
        })
    }

    pub fn new_normal(l_brace: Token, r_brace: Token, attrs: RecordAttrs) -> Self {
        Self::Normal(NormalRecord {
            l_brace,
            r_brace,
            attrs,
        })
    }

    pub fn empty(l_brace: Token, r_brace: Token) -> Self {
        Self::Normal(NormalRecord {
            l_brace,
            r_brace,
            attrs: RecordAttrs::new(Vec::with_capacity(0)),
        })
    }

    pub fn braces(&self) -> (&Token, &Token) {
        match self {
            Self::Normal(record) => (&record.l_brace, &record.r_brace),
            Self::Mixed(record) => (&record.l_brace, &record.r_brace),
        }
    }

    pub fn normalize(self) -> NormalRecord {
        match self {
            Self::Normal(normal) => normal,
            Self::Mixed(mixed) => NormalRecord::from(mixed),
        }
    }

    pub fn keys(&self) -> Vec<&Identifier> {
        match self {
            Self::Normal(normal) => normal.keys().collect(),
            Self::Mixed(mixed) => mixed.keys().collect(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Def> {
        match self {
            Self::Normal(normal) => normal.get(name),
            Self::Mixed(mixed) => mixed.get(name).and_then(|attr| match attr {
                RecordAttrOrIdent::Attr(attr) => Some(attr),
                RecordAttrOrIdent::Ident(_) => None,
            }),
        }
    }
}

/// Record can be defined with shorthend/normal mixed style, i.e. {x; y=expr; z; ...}
#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MixedRecord {
    pub l_brace: Token,
    pub r_brace: Token,
    pub attrs: Vec<RecordAttrOrIdent>,
}

impl NestedDisplay for MixedRecord {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{")?;
        for attr in self.attrs.iter() {
            write!(f, "{attr}; ")?;
        }
        write!(f, "}}")
    }
}

impl_display_from_nested!(MixedRecord);
impl_locational!(MixedRecord, l_brace, r_brace);

#[pymethods]
impl MixedRecord {
    #[pyo3(name = "get")]
    fn _get(&self, name: &str) -> Option<RecordAttrOrIdent> {
        self.get(name).cloned()
    }

    #[pyo3(name = "keys")]
    fn _keys(&self) -> Vec<Identifier> {
        self.keys().cloned().collect()
    }

    #[staticmethod]
    pub const fn new(l_brace: Token, r_brace: Token, attrs: Vec<RecordAttrOrIdent>) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
    }
}

impl MixedRecord {
    pub fn get(&self, name: &str) -> Option<&RecordAttrOrIdent> {
        for attr in self.attrs.iter() {
            match attr {
                RecordAttrOrIdent::Attr(def) => {
                    if def.sig.ident().is_some_and(|n| n.inspect() == name) {
                        return Some(attr);
                    }
                }
                RecordAttrOrIdent::Ident(ident) => {
                    if ident.inspect() == name {
                        return Some(attr);
                    }
                }
            }
        }
        None
    }

    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.attrs.iter().filter_map(|attr| match attr {
            RecordAttrOrIdent::Attr(attr) => attr.sig.ident(),
            RecordAttrOrIdent::Ident(ident) => Some(ident),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RecordAttrOrIdent {
    Attr(Def),
    Ident(Identifier),
}

impl_nested_display_for_enum!(RecordAttrOrIdent; Attr, Ident);
impl_display_for_enum!(RecordAttrOrIdent; Attr, Ident);
impl_locational_for_enum!(RecordAttrOrIdent; Attr, Ident);
impl_into_py_for_enum!(RecordAttrOrIdent; Attr, Ident);
impl_from_py_for_enum!(RecordAttrOrIdent; Attr(Def), Ident(Identifier));

impl RecordAttrOrIdent {
    pub fn ident(&self) -> Option<&Identifier> {
        match self {
            Self::Attr(attr) => attr.sig.ident(),
            Self::Ident(ident) => Some(ident),
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NormalSet {
    pub l_brace: Token,
    pub r_brace: Token,
    pub elems: Args,
}

impl NestedDisplay for NormalSet {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{{")?;
        self.elems.fmt_nest(f, level + 1)?;
        write!(f, "{}}}", "    ".repeat(level))
    }
}

impl_display_from_nested!(NormalSet);
impl_locational!(NormalSet, l_brace, elems, r_brace);

impl From<NormalSet> for Expr {
    fn from(set: NormalSet) -> Self {
        Self::Set(Set::Normal(set))
    }
}

#[pymethods]
impl NormalSet {
    #[pyo3(name = "get")]
    fn _get(&self, index: usize) -> Option<Expr> {
        self.get(index).cloned()
    }

    #[staticmethod]
    pub const fn new(l_brace: Token, r_brace: Token, elems: Args) -> Self {
        Self {
            l_brace,
            r_brace,
            elems,
        }
    }
}

impl NormalSet {
    pub fn get(&self, index: usize) -> Option<&Expr> {
        self.elems.pos_args.get(index).map(|a| &a.expr)
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetWithLength {
    pub l_brace: Token,
    pub r_brace: Token,
    pub elem: Box<PosArg>,
    pub len: Box<Expr>,
}

impl NestedDisplay for SetWithLength {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}; {}}}", self.elem, self.len)
    }
}

impl_display_from_nested!(SetWithLength);
impl_locational!(SetWithLength, l_brace, elem, r_brace);

#[pymethods]
impl SetWithLength {
    #[staticmethod]
    pub fn new(l_brace: Token, r_brace: Token, elem: PosArg, len: Expr) -> Self {
        Self {
            l_brace,
            r_brace,
            elem: Box::new(elem),
            len: Box::new(len),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetComprehension {
    pub l_brace: Token,
    pub r_brace: Token,
    pub layout: Option<Box<Expr>>,
    pub generators: Vec<(Identifier, Expr)>,
    pub guard: Option<Box<Expr>>,
}

impl NestedDisplay for SetComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        let mut generators = String::new();
        for (name, gen) in self.generators.iter() {
            write!(generators, "{name} <- {gen}; ")?;
        }
        write!(
            f,
            "{{{}{generators}{}}}",
            fmt_option!(self.layout, post " | "),
            fmt_option!(pre " | ", self.guard)
        )
    }
}

impl_display_from_nested!(SetComprehension);
impl_locational!(SetComprehension, l_brace, r_brace);

#[pymethods]
impl SetComprehension {
    #[staticmethod]
    #[pyo3(signature = (l_brace, r_brace, layout, generators, guard=None))]
    pub fn new(
        l_brace: Token,
        r_brace: Token,
        layout: Option<Expr>,
        generators: Vec<(Identifier, Expr)>,
        guard: Option<Expr>,
    ) -> Self {
        Self {
            l_brace,
            r_brace,
            layout: layout.map(Box::new),
            generators,
            guard: guard.map(Box::new),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Set {
    Normal(NormalSet),
    WithLength(SetWithLength),
    Comprehension(SetComprehension),
}

impl_nested_display_for_enum!(Set; Normal, WithLength, Comprehension);
impl_display_for_enum!(Set; Normal, WithLength, Comprehension);
impl_locational_for_enum!(Set; Normal, WithLength, Comprehension);
impl_into_py_for_enum!(Set; Normal, WithLength, Comprehension);
impl_from_py_for_enum!(Set; Normal(NormalSet), WithLength(SetWithLength), Comprehension(SetComprehension));

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BinOp {
    pub op: Token,
    pub args: [Box<Expr>; 2],
}

impl NestedDisplay for BinOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(
            f,
            "`{}`({}, {})",
            self.op.content, self.args[0], self.args[1]
        )
    }
}

impl_display_from_nested!(BinOp);

impl Locational for BinOp {
    fn loc(&self) -> Location {
        Location::concat(&self.op, self.args[1].as_ref())
    }
}

impl BinOp {
    pub fn deconstruct(self) -> (Token, Expr, Expr) {
        let mut exprs = self.args.into_iter();
        (self.op, *exprs.next().unwrap(), *exprs.next().unwrap())
    }
}

#[pymethods]
impl BinOp {
    #[staticmethod]
    pub fn new(op: Token, lhs: Expr, rhs: Expr) -> Self {
        Self {
            op,
            args: [Box::new(lhs), Box::new(rhs)],
        }
    }

    pub fn lhs(&self) -> Expr {
        self.args[0].as_ref().clone()
    }

    pub fn rhs(&self) -> Expr {
        self.args[1].as_ref().clone()
    }

    pub fn set_lhs(&mut self, lhs: Expr) {
        self.args[0] = Box::new(lhs);
    }

    pub fn set_rhs(&mut self, rhs: Expr) {
        self.args[1] = Box::new(rhs);
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnaryOp {
    pub op: Token,
    pub args: [Box<Expr>; 1],
}

impl NestedDisplay for UnaryOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "`{}`:", self.op.content)?;
        self.args[0].fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(UnaryOp);

impl Locational for UnaryOp {
    fn loc(&self) -> Location {
        Location::concat(&self.op, self.args[0].as_ref())
    }
}

impl UnaryOp {
    pub fn deconstruct(self) -> (Token, Expr) {
        let mut exprs = self.args.into_iter();
        (self.op, *exprs.next().unwrap())
    }
}

#[pymethods]
impl UnaryOp {
    #[staticmethod]
    pub fn new(op: Token, expr: Expr) -> Self {
        Self {
            op,
            args: [Box::new(expr)],
        }
    }

    pub fn value(&self) -> Expr {
        self.args[0].as_ref().clone()
    }

    pub fn set_value(&mut self, value: Expr) {
        self.args[0] = Box::new(value);
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Call {
    pub obj: Box<Expr>,
    pub attr_name: Option<Identifier>,
    pub args: Args,
}

impl NestedDisplay for Call {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({})", self.obj)?;
        } else {
            write!(f, "{}", self.obj)?;
        }
        if let Some(attr_name) = self.attr_name.as_ref() {
            write!(f, "{attr_name}")?;
        }
        if self.args.is_empty() {
            write!(f, "()")
        } else if self.args.len() < 6 {
            write!(f, "(")?;
            for (i, arg) in self.args.pos_args().iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            if let Some(rest) = self.args.var_args.as_ref() {
                if !self.args.pos_args().is_empty() {
                    write!(f, ", ")?;
                }
                write!(f, "*{}", rest)?;
            }
            for (i, kw_arg) in self.args.kw_args().iter().enumerate() {
                if i != 0 || !self.args.pos_args().is_empty() || self.args.var_args.is_some() {
                    write!(f, ", ")?;
                }
                write!(f, "{}", kw_arg)?;
            }
            write!(f, ")")
        } else {
            writeln!(f, ":")?;
            self.args.fmt_nest(f, level + 1)
        }
    }
}

impl TryFrom<Expr> for Call {
    type Error = ();
    fn try_from(expr: Expr) -> Result<Self, Self::Error> {
        match expr {
            Expr::Call(call) => Ok(call),
            Expr::TypeAscription(tasc) => Self::try_from(*tasc.expr),
            Expr::Accessor(Accessor::TypeApp(tapp)) => Self::try_from(*tapp.obj),
            _ => Err(()),
        }
    }
}

impl_display_from_nested!(Call);

impl Locational for Call {
    fn loc(&self) -> Location {
        Location::concat(self.obj.as_ref(), &self.args)
    }
}

#[pymethods]
impl Call {
    #[getter]
    pub fn get_obj(&self) -> Expr {
        self.obj.as_ref().clone()
    }

    #[setter]
    pub fn set_obj(&mut self, obj: Expr) {
        self.obj = Box::new(obj);
    }

    #[getter]
    pub fn get_attr_name(&self) -> Option<Identifier> {
        self.attr_name.clone()
    }

    #[setter]
    pub fn set_attr_name(&mut self, attr_name: Option<Identifier>) {
        self.attr_name = attr_name;
    }

    #[getter]
    pub fn get_args(&self) -> Args {
        self.args.clone()
    }

    #[setter]
    pub fn set_args(&mut self, args: Args) {
        self.args = args;
    }

    #[staticmethod]
    #[pyo3(signature = (obj, attr_name, args))]
    pub fn new(obj: Expr, attr_name: Option<Identifier>, args: Args) -> Self {
        Self {
            obj: Box::new(obj),
            attr_name,
            args,
        }
    }

    pub fn is_match(&self) -> bool {
        self.obj
            .get_name()
            .map(|s| &s[..] == "match" || &s[..] == "match!")
            .unwrap_or(false)
    }

    pub fn additional_operation(&self) -> Option<OperationKind> {
        self.obj.get_name().and_then(|s| match &s[..] {
            "import" => Some(OperationKind::Import),
            "pyimport" | "py" | "__import__" => Some(OperationKind::PyImport),
            "rsimport" => Some(OperationKind::RsImport),
            "Del" => Some(OperationKind::Del),
            "Class" => Some(OperationKind::Class),
            "Inherit" => Some(OperationKind::Inherit),
            "Trait" => Some(OperationKind::Trait),
            "Subsume" => Some(OperationKind::Subsume),
            _ => None,
        })
    }
}

/// e.g. `Data::{x = 1; y = 2}`
#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataPack {
    pub class: Box<Expr>,
    pub connector: VisModifierSpec,
    pub args: Record,
}

impl NestedDisplay for DataPack {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "{}::{}", self.class, self.args)
    }
}

impl_display_from_nested!(DataPack);

impl Locational for DataPack {
    fn loc(&self) -> Location {
        Location::concat(self.class.as_ref(), &self.args)
    }
}

impl DataPack {
    pub fn new(class: Expr, connector: VisModifierSpec, args: Record) -> Self {
        Self {
            class: Box::new(class),
            connector,
            args,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block {
    pub splice_id: Option<Str>,
    chunks: Vec<Expr>,
}

impl NestedDisplay for Block {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.chunks.iter(), f, level)
    }
}

impl_display_from_nested!(Block);

impl Locational for Block {
    fn loc(&self) -> Location {
        if self.chunks.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.chunks.first().unwrap(), self.chunks.last().unwrap())
        }
    }
}

impl_stream!(Block, Expr, chunks);
impl_py_iter!(Block<Expr>, BlockIter, chunks);

impl FromIterator<Expr> for Block {
    fn from_iter<I: IntoIterator<Item = Expr>>(iter: I) -> Self {
        Block::new(iter.into_iter().collect())
    }
}

impl Block {
    pub const fn new(chunks: Vec<Expr>) -> Block {
        Block {
            chunks,
            splice_id: None,
        }
    }
    pub const fn placeholder(id: Str) -> Block {
        Block {
            chunks: Vec::new(),
            splice_id: Some(id),
        }
    }
    pub const fn empty() -> Block {
        Block::new(Vec::new())
    }
    #[inline]
    pub fn with_capacity(capacity: usize) -> Block {
        Block::new(Vec::with_capacity(capacity))
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Dummy {
    pub loc: Option<Location>,
    pub exprs: Block,
}

impl NestedDisplay for Dummy {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "Dummy: [")?;
        fmt_lines(self.exprs.iter(), f, level)?;
        writeln!(f, "]")
    }
}

impl_display_from_nested!(Dummy);

impl Locational for Dummy {
    fn loc(&self) -> Location {
        if self.loc.is_some() {
            self.loc.unwrap_or(Location::Unknown)
        } else if self.exprs.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.exprs.first().unwrap(), self.exprs.last().unwrap())
        }
    }
}

impl IntoIterator for Dummy {
    type Item = Expr;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload().into_iter()
    }
}

impl Stream<Expr> for Dummy {
    fn ref_payload(&self) -> &Vec<Expr> {
        &self.exprs.chunks
    }
    fn ref_mut_payload(&mut self) -> &mut Vec<Expr> {
        &mut self.exprs.chunks
    }
    fn payload(self) -> Vec<Expr> {
        self.exprs.chunks
    }
}

#[pymethods]
impl Dummy {
    #[staticmethod]
    #[pyo3(signature = (loc, exprs))]
    pub const fn new(loc: Option<Location>, exprs: Vec<Expr>) -> Self {
        Self {
            loc,
            exprs: Block::new(exprs),
        }
    }
}

pub type ConstIdentifier = Identifier;

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstAttribute {
    pub obj: Box<ConstExpr>,
    pub name: ConstIdentifier,
}

impl NestedDisplay for ConstAttribute {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({}){}", self.obj, self.name)
        } else {
            write!(f, "{}{}", self.obj, self.name)
        }
    }
}

impl_display_from_nested!(ConstAttribute);
impl_locational!(ConstAttribute, obj, name);

#[pymethods]
impl ConstAttribute {
    #[staticmethod]
    pub fn new(expr: ConstExpr, name: ConstIdentifier) -> Self {
        Self {
            obj: Box::new(expr),
            name,
        }
    }
}

impl ConstAttribute {
    pub fn downgrade(self) -> Attribute {
        Attribute::new(self.obj.downgrade(), self.name)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstTupleAttribute {
    tup: Box<ConstExpr>,
    index: Literal,
}

impl NestedDisplay for ConstTupleAttribute {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if self.tup.need_to_be_closed() {
            write!(f, "({}).{}", self.tup, self.index)
        } else {
            write!(f, "{}.{}", self.tup, self.index)
        }
    }
}

impl_display_from_nested!(ConstTupleAttribute);
impl_locational!(ConstTupleAttribute, tup, index);

impl ConstTupleAttribute {
    pub fn downgrade(self) -> TupleAttribute {
        TupleAttribute::new(self.tup.downgrade(), self.index)
    }
}

#[pymethods]
impl ConstTupleAttribute {
    #[staticmethod]
    pub fn new(tup: ConstExpr, index: Literal) -> Self {
        Self {
            tup: Box::new(tup),
            index,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstSubscript {
    obj: Box<ConstExpr>,
    index: Box<ConstExpr>,
    r_sqbr: Token,
}

impl NestedDisplay for ConstSubscript {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({})[{}]", self.obj, self.index)
        } else {
            write!(f, "{}[{}]", self.obj, self.index)
        }
    }
}

impl_display_from_nested!(ConstSubscript);
impl_locational!(ConstSubscript, obj, r_sqbr);

impl ConstSubscript {
    pub fn downgrade(self) -> Subscript {
        Subscript::new(self.obj.downgrade(), self.index.downgrade(), self.r_sqbr)
    }
}

#[pymethods]
impl ConstSubscript {
    #[staticmethod]
    pub fn new(obj: ConstExpr, index: ConstExpr, r_sqbr: Token) -> Self {
        Self {
            obj: Box::new(obj),
            index: Box::new(index),
            r_sqbr,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstAccessor {
    Local(ConstIdentifier),
    Attr(ConstAttribute),
    TupleAttr(ConstTupleAttribute),
    Subscr(ConstSubscript),
}

impl_nested_display_for_enum!(ConstAccessor; Local, Attr, TupleAttr, Subscr);
impl_display_from_nested!(ConstAccessor);
impl_locational_for_enum!(ConstAccessor; Local, Attr, TupleAttr, Subscr);
impl_into_py_for_enum!(ConstAccessor; Local, Attr, TupleAttr, Subscr);
impl_from_py_for_enum!(ConstAccessor; Local(ConstIdentifier), Attr(ConstAttribute), TupleAttr(ConstTupleAttribute), Subscr(ConstSubscript));

impl ConstAccessor {
    pub const fn local(symbol: Token) -> Self {
        Self::Local(ConstIdentifier::new(
            VisModifierSpec::Private,
            VarName::new(symbol),
        ))
    }

    pub fn attr(obj: ConstExpr, name: ConstIdentifier) -> Self {
        Self::Attr(ConstAttribute::new(obj, name))
    }

    pub fn subscr(obj: ConstExpr, index: ConstExpr, r_sqbr: Token) -> Self {
        Self::Subscr(ConstSubscript::new(obj, index, r_sqbr))
    }

    pub fn downgrade(self) -> Accessor {
        match self {
            Self::Local(local) => Accessor::Ident(local),
            Self::Attr(attr) => Accessor::Attr(attr.downgrade()),
            Self::TupleAttr(attr) => Accessor::TupleAttr(attr.downgrade()),
            Self::Subscr(subscr) => Accessor::Subscr(subscr.downgrade()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstArray {
    Normal(ConstNormalArray),
    WithLength(ConstArrayWithLength),
}

impl_nested_display_for_enum!(ConstArray; Normal, WithLength);
impl_display_from_nested!(ConstArray);
impl_locational_for_enum!(ConstArray; Normal, WithLength);
impl_into_py_for_enum!(ConstArray; Normal, WithLength);
impl_from_py_for_enum!(ConstArray; Normal(ConstNormalArray), WithLength(ConstArrayWithLength));

impl ConstArray {
    pub fn downgrade(self) -> Array {
        match self {
            Self::Normal(normal) => Array::Normal(normal.downgrade()),
            Self::WithLength(with_length) => Array::WithLength(with_length.downgrade()),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstNormalArray {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub elems: ConstArgs,
    pub guard: Option<Box<ConstExpr>>,
}

impl NestedDisplay for ConstNormalArray {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if let Some(guard) = &self.guard {
            write!(f, "[{} | {}]", self.elems, guard)
        } else {
            write!(f, "[{}]", self.elems)
        }
    }
}

impl_display_from_nested!(ConstNormalArray);
impl_locational!(ConstNormalArray, l_sqbr, elems, r_sqbr);

#[pymethods]
impl ConstNormalArray {
    #[staticmethod]
    pub fn new(l_sqbr: Token, r_sqbr: Token, elems: ConstArgs, guard: Option<ConstExpr>) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elems,
            guard: guard.map(Box::new),
        }
    }
}

impl ConstNormalArray {
    pub fn downgrade(self) -> NormalArray {
        NormalArray::new(self.l_sqbr, self.r_sqbr, self.elems.downgrade())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstArrayWithLength {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub elem: Box<ConstExpr>,
    pub length: Box<ConstExpr>,
}

impl NestedDisplay for ConstArrayWithLength {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "[{}; {}]", self.elem, self.length)
    }
}

impl_display_from_nested!(ConstArrayWithLength);
impl_locational!(ConstArrayWithLength, l_sqbr, elem, r_sqbr);

#[pymethods]
impl ConstArrayWithLength {
    #[staticmethod]
    pub fn new(l_sqbr: Token, r_sqbr: Token, elem: ConstExpr, length: ConstExpr) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elem: Box::new(elem),
            length: Box::new(length),
        }
    }
}

impl ConstArrayWithLength {
    pub fn downgrade(self) -> ArrayWithLength {
        ArrayWithLength::new(
            self.l_sqbr,
            self.r_sqbr,
            PosArg::new(self.elem.downgrade()),
            self.length.downgrade(),
        )
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstNormalSet {
    pub l_brace: Token,
    pub r_brace: Token,
    pub elems: ConstArgs,
}

impl NestedDisplay for ConstNormalSet {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", self.elems)
    }
}

impl_display_from_nested!(ConstNormalSet);
impl_locational!(ConstNormalSet, l_brace, elems, r_brace);

#[pymethods]
impl ConstNormalSet {
    #[staticmethod]
    pub const fn new(l_brace: Token, r_brace: Token, elems: ConstArgs) -> Self {
        Self {
            l_brace,
            r_brace,
            elems,
        }
    }
}

impl ConstNormalSet {
    pub fn downgrade(self) -> NormalSet {
        NormalSet::new(self.l_brace, self.r_brace, self.elems.downgrade())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstSetComprehension {
    pub l_brace: Token,
    pub r_brace: Token,
    pub layout: Option<Box<ConstExpr>>,
    pub generators: Vec<(ConstIdentifier, ConstExpr)>,
    pub guard: Option<Box<ConstExpr>>,
}

impl NestedDisplay for ConstSetComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        let mut generators = String::new();
        for (name, gen) in self.generators.iter() {
            write!(generators, "{name} <- {gen}, ")?;
        }
        write!(
            f,
            "{{{}{generators}{}}}",
            fmt_option!(self.layout, post " | "),
            fmt_option!(pre " | ", self.guard)
        )
    }
}

impl_display_from_nested!(ConstSetComprehension);
impl_locational!(ConstSetComprehension, l_brace, r_brace);

#[pymethods]
impl ConstSetComprehension {
    #[staticmethod]
    #[pyo3(signature = (l_brace, r_brace, elem, generators, guard=None))]
    pub fn new(
        l_brace: Token,
        r_brace: Token,
        elem: Option<ConstExpr>,
        generators: Vec<(ConstIdentifier, ConstExpr)>,
        guard: Option<ConstExpr>,
    ) -> Self {
        Self {
            l_brace,
            r_brace,
            layout: elem.map(Box::new),
            generators,
            guard: guard.map(Box::new),
        }
    }
}

impl ConstSetComprehension {
    pub fn downgrade(self) -> SetComprehension {
        SetComprehension::new(
            self.l_brace,
            self.r_brace,
            self.layout.map(|ex| ex.downgrade()),
            self.generators
                .into_iter()
                .map(|(name, gen)| (name, gen.downgrade()))
                .collect(),
            self.guard.map(|ex| ex.downgrade()),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstSet {
    Normal(ConstNormalSet),
    Comprehension(ConstSetComprehension),
}

impl_nested_display_for_enum!(ConstSet; Normal, Comprehension);
impl_display_from_nested!(ConstSet);
impl_locational_for_enum!(ConstSet; Normal, Comprehension);
impl_into_py_for_enum!(ConstSet; Normal, Comprehension);
impl_from_py_for_enum!(ConstSet; Normal(ConstNormalSet), Comprehension(ConstSetComprehension));

impl ConstSet {
    pub fn downgrade(self) -> Set {
        match self {
            Self::Normal(normal) => Set::Normal(normal.downgrade()),
            Self::Comprehension(comp) => Set::Comprehension(comp.downgrade()),
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstKeyValue {
    pub key: ConstExpr,
    pub value: ConstExpr,
}

impl NestedDisplay for ConstKeyValue {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.value)
    }
}

impl_display_from_nested!(ConstKeyValue);
impl_locational!(ConstKeyValue, key, value);

#[pymethods]
impl ConstKeyValue {
    #[staticmethod]
    pub const fn new(key: ConstExpr, value: ConstExpr) -> Self {
        Self { key, value }
    }
}

impl ConstKeyValue {
    pub fn downgrade(self) -> KeyValue {
        KeyValue::new(self.key.downgrade(), self.value.downgrade())
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstDict {
    l_brace: Token,
    r_brace: Token,
    pub kvs: Vec<ConstKeyValue>,
}

impl NestedDisplay for ConstDict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", fmt_vec(&self.kvs))
    }
}

impl_display_from_nested!(ConstDict);
impl_locational!(ConstDict, l_brace, r_brace);

#[pymethods]
impl ConstDict {
    #[staticmethod]
    pub const fn new(l_brace: Token, r_brace: Token, kvs: Vec<ConstKeyValue>) -> Self {
        Self {
            l_brace,
            r_brace,
            kvs,
        }
    }
}

impl ConstDict {
    pub fn downgrade(self) -> Dict {
        Dict::Normal(NormalDict::new(
            self.l_brace,
            self.r_brace,
            self.kvs.into_iter().map(|kv| kv.downgrade()).collect(),
        ))
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstTuple {
    pub elems: ConstArgs,
}

impl NestedDisplay for ConstTuple {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "({})", self.elems)
    }
}

impl_display_from_nested!(ConstTuple);
impl_locational!(ConstTuple, elems);

#[pymethods]
impl ConstTuple {
    #[staticmethod]
    pub const fn new(elems: ConstArgs) -> Self {
        Self { elems }
    }
}

impl ConstTuple {
    pub fn downgrade(self) -> Tuple {
        Tuple::Normal(NormalTuple::new(self.elems.downgrade()))
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstBlock(Vec<ConstExpr>);

impl NestedDisplay for ConstBlock {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, _level)
    }
}

impl_display_from_nested!(ConstBlock);

impl Locational for ConstBlock {
    fn loc(&self) -> Location {
        if self.0.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
        }
    }
}

impl_stream!(ConstBlock, ConstExpr);
impl_py_iter!(ConstBlock<ConstExpr>, ConstExprIter);

impl ConstBlock {
    pub fn downgrade(self) -> Block {
        Block::new(self.0.into_iter().map(|e| e.downgrade()).collect())
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstDefBody {
    pub op: Token,
    pub block: ConstBlock,
    pub id: DefId,
}

impl_locational!(ConstDefBody, lossy op, block);

#[pymethods]
impl ConstDefBody {
    #[staticmethod]
    pub const fn new(op: Token, block: ConstBlock, id: DefId) -> Self {
        Self { op, block, id }
    }
}

impl ConstDefBody {
    pub fn downgrade(self) -> DefBody {
        DefBody::new(self.op, self.block.downgrade(), self.id)
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstDef {
    pub ident: ConstIdentifier,
    pub body: ConstDefBody,
}

impl NestedDisplay for ConstDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} = {}", self.ident, self.body.block)
    }
}

impl_display_from_nested!(ConstDef);
impl_locational!(ConstDef, ident, body);

#[pymethods]
impl ConstDef {
    #[staticmethod]
    pub const fn new(ident: ConstIdentifier, body: ConstDefBody) -> Self {
        Self { ident, body }
    }
}

impl ConstDef {
    pub fn downgrade(self) -> Def {
        Def::new(Signature::new_var(self.ident), self.body.downgrade())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstLambda {
    pub sig: Box<LambdaSignature>,
    pub op: Token,
    pub body: ConstBlock,
    pub id: DefId,
}

impl NestedDisplay for ConstLambda {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "({}) {} {}", self.sig, self.op.content, self.body)
    }
}

impl_display_from_nested!(ConstLambda);
impl_locational!(ConstLambda, sig, body);

#[pymethods]
impl ConstLambda {
    #[staticmethod]
    pub fn new(sig: LambdaSignature, op: Token, body: ConstBlock, id: DefId) -> Self {
        Self {
            sig: Box::new(sig),
            op,
            body,
            id,
        }
    }
}

impl ConstLambda {
    pub fn downgrade(self) -> Lambda {
        Lambda::new(*self.sig, self.op, self.body.downgrade(), self.id)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstRecord {
    pub l_brace: Token,
    pub r_brace: Token,
    pub attrs: Vec<ConstDef>,
}

impl NestedDisplay for ConstRecord {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "{{{}}}", fmt_vec_split_with(&self.attrs, "; "))
    }
}

impl Locational for ConstRecord {
    fn loc(&self) -> Location {
        Location::concat(&self.l_brace, &self.r_brace)
    }
}

impl_display_from_nested!(ConstRecord);

#[pymethods]
impl ConstRecord {
    #[staticmethod]
    pub const fn new(l_brace: Token, r_brace: Token, attrs: Vec<ConstDef>) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
    }
}

impl ConstRecord {
    pub fn downgrade(self) -> Record {
        Record::Normal(NormalRecord::new(
            self.l_brace,
            self.r_brace,
            self.attrs.into_iter().map(|d| d.downgrade()).collect(),
        ))
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstBinOp {
    pub op: Token,
    pub lhs: Box<ConstExpr>,
    pub rhs: Box<ConstExpr>,
}

impl NestedDisplay for ConstBinOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "`{}`({}, {})", self.op.content, self.lhs, self.rhs)
    }
}

impl_display_from_nested!(ConstBinOp);
impl_locational!(ConstBinOp, lhs, rhs);

#[pymethods]
impl ConstBinOp {
    #[staticmethod]
    pub fn new(op: Token, lhs: ConstExpr, rhs: ConstExpr) -> Self {
        Self {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

impl ConstBinOp {
    pub fn downgrade(self) -> BinOp {
        BinOp::new(self.op, self.lhs.downgrade(), self.rhs.downgrade())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstUnaryOp {
    pub op: Token,
    pub expr: Box<ConstExpr>,
}

impl NestedDisplay for ConstUnaryOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "`{}`({})", self.op.content, self.expr)
    }
}

impl_display_from_nested!(ConstUnaryOp);
impl_locational!(ConstUnaryOp, op, expr);

#[pymethods]
impl ConstUnaryOp {
    #[staticmethod]
    pub fn new(op: Token, expr: ConstExpr) -> Self {
        Self {
            op,
            expr: Box::new(expr),
        }
    }
}

impl ConstUnaryOp {
    pub fn downgrade(self) -> UnaryOp {
        UnaryOp::new(self.op, self.expr.downgrade())
    }
}

/// Application
/// ex. `Vec Int` of `Option Vec Int`
#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstApp {
    pub obj: Box<ConstExpr>,
    pub attr_name: Option<ConstIdentifier>,
    pub args: ConstArgs,
}

impl NestedDisplay for ConstApp {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        writeln!(f, "{}", self.obj)?;
        if let Some(attr_name) = &self.attr_name {
            writeln!(f, "{}", attr_name)?;
        }
        writeln!(f, "(")?;
        self.args.fmt_nest(f, level + 1)?;
        writeln!(f, ")")
    }
}

impl_display_from_nested!(ConstApp);

impl Locational for ConstApp {
    fn loc(&self) -> Location {
        if self.args.is_empty() {
            self.obj.loc()
        } else {
            Location::concat(self.obj.as_ref(), &self.args)
        }
    }
}

#[pymethods]
impl ConstApp {
    #[staticmethod]
    #[pyo3(signature = (obj, attr_name, args))]
    pub fn new(obj: ConstExpr, attr_name: Option<ConstIdentifier>, args: ConstArgs) -> Self {
        Self {
            obj: Box::new(obj),
            attr_name,
            args,
        }
    }
}

impl ConstApp {
    pub fn downgrade(self) -> Call {
        if let Some(attr_name) = self.attr_name {
            self.obj
                .downgrade()
                .attr_expr(attr_name)
                .call(self.args.downgrade())
        } else {
            self.obj.downgrade().call(self.args.downgrade())
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstTypeAsc {
    pub expr: Box<ConstExpr>,
    pub t_spec: Box<TypeSpecWithOp>,
}

impl NestedDisplay for ConstTypeAsc {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        writeln!(f, "{}{}", self.expr, self.t_spec)
    }
}

impl_display_from_nested!(ConstTypeAsc);
impl_locational!(ConstTypeAsc, expr, t_spec);

#[pymethods]
impl ConstTypeAsc {
    #[staticmethod]
    pub fn new(expr: ConstExpr, t_spec: TypeSpecWithOp) -> Self {
        Self {
            expr: Box::new(expr),
            t_spec: Box::new(t_spec),
        }
    }

    pub fn is_instance_ascription(&self) -> bool {
        self.t_spec.op.is(TokenKind::Colon)
    }

    pub fn is_subtype_ascription(&self) -> bool {
        self.t_spec.op.is(TokenKind::SubtypeOf)
    }
}

impl ConstTypeAsc {
    pub fn downgrade(self) -> TypeAscription {
        TypeAscription::new(self.expr.downgrade(), *self.t_spec)
    }
}

/// valid expression for an argument of polymorphic types
/// 多相型の実引数として有効な式
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstExpr {
    Lit(Literal),
    Accessor(ConstAccessor),
    App(ConstApp),
    Array(ConstArray),
    Set(ConstSet),
    Dict(ConstDict),
    Tuple(ConstTuple),
    Record(ConstRecord),
    Def(ConstDef),
    Lambda(ConstLambda),
    BinOp(ConstBinOp),
    UnaryOp(ConstUnaryOp),
    TypeAsc(ConstTypeAsc),
}

impl_nested_display_for_chunk_enum!(ConstExpr; Lit, Accessor, App, Array, Set, Dict, Tuple, Record, BinOp, UnaryOp, Def, Lambda, Set, TypeAsc);
impl_display_from_nested!(ConstExpr);
impl_locational_for_enum!(ConstExpr; Lit, Accessor, App, Array, Set, Dict, Tuple, Record, BinOp, UnaryOp, Def, Lambda, Set, TypeAsc);
impl_into_py_for_enum!(ConstExpr; Lit, Accessor, App, Array, Set, Dict, Tuple, Record, Def, Lambda, BinOp, UnaryOp, TypeAsc);
impl_from_py_for_enum!(ConstExpr; Lit(Literal), Accessor(ConstAccessor), App(ConstApp), Array(ConstArray), Set(ConstSet), Dict(ConstDict), Tuple(ConstTuple), Record(ConstRecord), Def(ConstDef), Lambda(ConstLambda), BinOp(ConstBinOp), UnaryOp(ConstUnaryOp), TypeAsc(ConstTypeAsc));

impl TryFrom<&ParamPattern> for ConstExpr {
    type Error = ();
    fn try_from(value: &ParamPattern) -> Result<Self, Self::Error> {
        match value {
            ParamPattern::VarName(name) => {
                Ok(ConstExpr::Accessor(ConstAccessor::local(name.0.clone())))
            }
            ParamPattern::Lit(lit) => Ok(ConstExpr::Lit(lit.clone())),
            ParamPattern::Array(array) => ConstExpr::try_from(array),
            ParamPattern::Tuple(tuple) => ConstExpr::try_from(tuple),
            _ => Err(()),
        }
    }
}

impl ConstExpr {
    pub fn need_to_be_closed(&self) -> bool {
        match self {
            Self::BinOp(_) | Self::UnaryOp(_) | Self::Lambda(_) | Self::TypeAsc(_) => true,
            Self::Tuple(tup) => tup.elems.paren.is_none(),
            Self::App(app) if ERG_MODE => app.args.paren.is_none(),
            _ => false,
        }
    }

    pub fn downgrade(self) -> Expr {
        match self {
            Self::Lit(lit) => Expr::Literal(lit),
            Self::Accessor(acc) => Expr::Accessor(acc.downgrade()),
            Self::App(app) => Expr::Call(app.downgrade()),
            Self::Array(arr) => Expr::Array(arr.downgrade()),
            Self::Set(set) => Expr::Set(set.downgrade()),
            Self::Dict(dict) => Expr::Dict(dict.downgrade()),
            Self::Tuple(tuple) => Expr::Tuple(tuple.downgrade()),
            Self::Record(record) => Expr::Record(record.downgrade()),
            Self::Lambda(lambda) => Expr::Lambda(lambda.downgrade()),
            Self::Def(def) => Expr::Def(def.downgrade()),
            Self::BinOp(binop) => Expr::BinOp(binop.downgrade()),
            Self::UnaryOp(unop) => Expr::UnaryOp(unop.downgrade()),
            Self::TypeAsc(type_asc) => Expr::TypeAscription(type_asc.downgrade()),
        }
    }

    pub fn attr(self, attr_name: ConstIdentifier) -> ConstAccessor {
        ConstAccessor::attr(self, attr_name)
    }

    pub fn attr_expr(self, attr_name: ConstIdentifier) -> Self {
        Self::Accessor(self.attr(attr_name))
    }

    pub fn call(self, args: ConstArgs) -> ConstApp {
        ConstApp::new(self, None, args)
    }

    pub fn call_expr(self, args: ConstArgs) -> Self {
        Self::App(self.call(args))
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstPosArg {
    pub expr: ConstExpr,
}

impl NestedDisplay for ConstPosArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        self.expr.fmt_nest(f, level)
    }
}

impl_locational!(ConstPosArg, expr);

#[pymethods]
impl ConstPosArg {
    #[staticmethod]
    pub const fn new(expr: ConstExpr) -> Self {
        Self { expr }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstKwArg {
    pub keyword: Token,
    pub expr: ConstExpr,
}

impl NestedDisplay for ConstKwArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "{} := {}", self.keyword.content, self.expr)
    }
}

impl_display_from_nested!(ConstKwArg);
impl_locational!(ConstKwArg, keyword, expr);

#[pymethods]
impl ConstKwArg {
    #[staticmethod]
    pub const fn new(keyword: Token, expr: ConstExpr) -> Self {
        Self { keyword, expr }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstArgs {
    pub pos_args: Vec<ConstPosArg>,
    pub var_args: Option<Box<ConstPosArg>>,
    pub kw_args: Vec<ConstKwArg>,
    pub kw_var: Option<Box<ConstPosArg>>,
    paren: Option<(Token, Token)>,
}

impl NestedDisplay for ConstArgs {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        fmt_lines(self.pos_args(), f, level)?;
        writeln!(f)?;
        fmt_lines(self.kw_args(), f, level)
    }
}

impl_display_from_nested!(ConstArgs);

impl Locational for ConstArgs {
    fn loc(&self) -> Location {
        if let Some((l, r)) = &self.paren {
            Location::concat(l, r)
        } else if let Some(last) = self.kw_args.last() {
            Location::concat(self.pos_args.first().unwrap(), last)
        } else if let Some(last) = self.pos_args.last() {
            Location::concat(self.pos_args.first().unwrap(), last)
        } else {
            Location::Unknown
        }
    }
}

// impl_stream!(ConstArgs, ConstKwArg, pos_args);

#[pymethods]
impl ConstArgs {
    #[staticmethod]
    #[pyo3(signature = (pos_args, var_args, kw_args, kw_var=None, paren=None))]
    pub fn new(
        pos_args: Vec<ConstPosArg>,
        var_args: Option<ConstPosArg>,
        kw_args: Vec<ConstKwArg>,
        kw_var: Option<ConstPosArg>,
        paren: Option<(Token, Token)>,
    ) -> Self {
        Self {
            pos_args,
            var_args: var_args.map(Box::new),
            kw_args,
            kw_var: kw_var.map(Box::new),
            paren,
        }
    }

    #[staticmethod]
    pub fn pos_only(pos_args: Vec<ConstPosArg>, paren: Option<(Token, Token)>) -> Self {
        Self::new(pos_args, None, vec![], None, paren)
    }

    #[staticmethod]
    pub fn single(expr: ConstExpr) -> Self {
        Self::pos_only(vec![ConstPosArg::new(expr)], None)
    }

    #[staticmethod]
    pub fn empty() -> Self {
        Self::new(vec![], None, vec![], None, None)
    }

    pub fn is_empty(&self) -> bool {
        self.pos_args.is_empty() && self.kw_args.is_empty()
    }

    pub fn push_pos(&mut self, arg: ConstPosArg) {
        self.pos_args.push(arg);
    }

    pub fn push_kw(&mut self, arg: ConstKwArg) {
        self.kw_args.push(arg);
    }
}

impl ConstArgs {
    #[allow(clippy::type_complexity)]
    pub fn deconstruct(
        self,
    ) -> (
        Vec<ConstPosArg>,
        Option<ConstPosArg>,
        Vec<ConstKwArg>,
        Option<ConstPosArg>,
        Option<(Token, Token)>,
    ) {
        (
            self.pos_args,
            self.var_args.map(|x| *x),
            self.kw_args,
            self.kw_var.map(|x| *x),
            self.paren,
        )
    }

    pub fn pos_args(&self) -> impl Iterator<Item = &ConstPosArg> {
        self.pos_args.iter()
    }

    pub fn kw_args(&self) -> impl Iterator<Item = &ConstKwArg> {
        self.kw_args.iter()
    }

    pub fn into_iters(
        self,
    ) -> (
        impl IntoIterator<Item = ConstPosArg>,
        impl IntoIterator<Item = ConstKwArg>,
    ) {
        (self.pos_args.into_iter(), self.kw_args.into_iter())
    }

    pub fn downgrade(self) -> Args {
        let (pos_args, var_args, kw_args, kw_var, paren) = self.deconstruct();
        Args::new(
            pos_args
                .into_iter()
                .map(|arg| PosArg::new(arg.expr.downgrade()))
                .collect(),
            var_args.map(|arg| PosArg::new(arg.expr.downgrade())),
            kw_args
                .into_iter()
                // TODO t_spec
                .map(|arg| KwArg::new(arg.keyword, None, arg.expr.downgrade()))
                .collect(),
            kw_var.map(|arg| PosArg::new(arg.expr.downgrade())),
            paren,
        )
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PolyTypeSpec {
    pub acc: ConstAccessor,
    pub args: ConstArgs, // args can be nested (e.g. Vec Vec Int)
}

impl fmt::Display for PolyTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.acc, self.args)
    }
}

impl Locational for PolyTypeSpec {
    fn loc(&self) -> Location {
        if let Some(last) = self.args.kw_args.last() {
            Location::concat(&self.acc, last)
        } else if let Some(last) = self.args.pos_args.last() {
            Location::concat(&self.acc, last)
        } else {
            self.acc.loc()
        }
    }
}

#[pymethods]
impl PolyTypeSpec {
    #[staticmethod]
    pub const fn new(acc: ConstAccessor, args: ConstArgs) -> Self {
        Self { acc, args }
    }

    pub fn ident(&self) -> String {
        self.acc.to_string()
    }
}

// OK:
//   ts = [T, U]; x: ts[0] = ...
//   ts = {.T: T, .U: U}; x: ts.T = ...
//   ...; x: foo.bar.ts[0] = ...
// NG:
//   ts = {"T": T, "U": U}; x: ts["T"] = ...
//   f T = T; x: f(T) = ...
//   ...; x: foo[0].T = ...
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PreDeclTypeSpec {
    Mono(Identifier),
    Poly(PolyTypeSpec),
    Attr {
        namespace: Box<Expr>,
        t: Identifier,
    },
    Subscr {
        namespace: Box<Expr>,
        ident: Identifier,
        index: Token,
    },
}

impl fmt::Display for PreDeclTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreDeclTypeSpec::Mono(mono) => write!(f, "{mono}"),
            PreDeclTypeSpec::Poly(poly) => write!(f, "{poly}"),
            PreDeclTypeSpec::Attr { namespace, t } => {
                write!(f, "{namespace}{t}")
            }
            PreDeclTypeSpec::Subscr {
                namespace,
                ident,
                index,
            } => write!(f, "{namespace}{ident}[{index}]"),
        }
    }
}

impl Locational for PreDeclTypeSpec {
    fn loc(&self) -> Location {
        match self {
            Self::Mono(m) => m.loc(),
            Self::Poly(poly) => poly.loc(),
            Self::Attr { namespace, t } => Location::concat(namespace.as_ref(), t),
            Self::Subscr {
                namespace, index, ..
            } => Location::concat(namespace.as_ref(), index),
        }
    }
}

impl PreDeclTypeSpec {
    pub fn attr(namespace: Expr, t: Identifier) -> Self {
        Self::Attr {
            namespace: Box::new(namespace),
            t,
        }
    }

    pub fn poly(acc: ConstAccessor, args: ConstArgs) -> Self {
        Self::Poly(PolyTypeSpec::new(acc, args))
    }

    pub fn ident(&self) -> String {
        match self {
            Self::Mono(name) => name.inspect().to_string(),
            Self::Poly(poly) => poly.ident(),
            Self::Attr { namespace, t } => format!("{namespace}{t}"),
            other => todo!("{other}"),
        }
    }
}

#[pyclass(get_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamTySpec {
    pub name: Option<Token>,
    pub ty: TypeSpec,
}

impl fmt::Display for ParamTySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{}: {}", name.inspect(), self.ty)
        } else {
            write!(f, "{}", self.ty)
        }
    }
}

impl Locational for ParamTySpec {
    fn loc(&self) -> Location {
        if let Some(name) = &self.name {
            Location::concat(name, &self.ty)
        } else {
            self.ty.loc()
        }
    }
}

impl ParamTySpec {
    pub const fn new(name: Option<Token>, ty: TypeSpec) -> Self {
        Self { name, ty }
    }

    pub const fn anonymous(ty: TypeSpec) -> Self {
        Self::new(None, ty)
    }
}

#[pyclass(get_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefaultParamTySpec {
    pub param: ParamTySpec,
    pub default: TypeSpec,
}

impl fmt::Display for DefaultParamTySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} := {}", self.param, self.default)
    }
}

impl DefaultParamTySpec {
    pub const fn new(param: ParamTySpec, default: TypeSpec) -> Self {
        Self { param, default }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubrTypeSpec {
    pub bounds: TypeBoundSpecs,
    pub lparen: Option<Token>,
    pub non_defaults: Vec<ParamTySpec>,
    pub var_params: Option<Box<ParamTySpec>>,
    pub defaults: Vec<DefaultParamTySpec>,
    pub kw_var_params: Option<Box<ParamTySpec>>,
    pub arrow: Token,
    pub return_t: Box<TypeSpec>,
}

impl fmt::Display for SubrTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.bounds.is_empty() {
            write!(f, "|{}|", self.bounds)?;
        }
        write!(
            f,
            "({}, {}{}{}) {} {}",
            fmt_vec(&self.non_defaults),
            fmt_option!("*", &self.var_params, ", "),
            fmt_vec(&self.defaults),
            fmt_option!(pre ", **", &self.kw_var_params),
            self.arrow.content,
            self.return_t
        )
    }
}

impl Locational for SubrTypeSpec {
    fn loc(&self) -> Location {
        if !self.bounds.is_empty() {
            Location::concat(&self.bounds[0], self.return_t.as_ref())
        } else if let Some(lparen) = &self.lparen {
            Location::concat(lparen, self.return_t.as_ref())
        } else if let Some(nd_param) = self.non_defaults.first() {
            Location::concat(nd_param, self.return_t.as_ref())
        } else if let Some(var_params) = self.var_params.as_deref() {
            Location::concat(var_params, self.return_t.as_ref())
        } else if let Some(d_param) = self.defaults.first() {
            Location::concat(&d_param.param, self.return_t.as_ref())
        } else {
            self.return_t.loc()
        }
    }
}

impl SubrTypeSpec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bounds: TypeBoundSpecs,
        lparen: Option<Token>,
        non_defaults: Vec<ParamTySpec>,
        var_params: Option<ParamTySpec>,
        defaults: Vec<DefaultParamTySpec>,
        kw_var_params: Option<ParamTySpec>,
        arrow: Token,
        return_t: TypeSpec,
    ) -> Self {
        Self {
            bounds,
            lparen,
            non_defaults,
            var_params: var_params.map(Box::new),
            defaults,
            kw_var_params: kw_var_params.map(Box::new),
            arrow,
            return_t: Box::new(return_t),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayTypeSpec {
    pub sqbrs: Option<(Token, Token)>,
    pub ty: Box<TypeSpec>,
    pub len: ConstExpr,
}

impl fmt::Display for ArrayTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}; {}]", self.ty, self.len)
    }
}

impl Locational for ArrayTypeSpec {
    fn loc(&self) -> Location {
        if let Some((lsqbr, rsqbr)) = &self.sqbrs {
            Location::concat(lsqbr, rsqbr)
        } else {
            Location::concat(self.ty.as_ref(), &self.len)
        }
    }
}

impl ArrayTypeSpec {
    pub fn new(ty: TypeSpec, len: ConstExpr, sqbrs: Option<(Token, Token)>) -> Self {
        Self {
            ty: Box::new(ty),
            len,
            sqbrs,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetWithLenTypeSpec {
    pub ty: Box<TypeSpec>,
    pub len: ConstExpr,
}

impl fmt::Display for SetWithLenTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{}; {}}}", self.ty, self.len)
    }
}

impl_locational!(SetWithLenTypeSpec, ty, len);

impl SetWithLenTypeSpec {
    pub fn new(ty: TypeSpec, len: ConstExpr) -> Self {
        Self {
            ty: Box::new(ty),
            len,
        }
    }
}

#[pyclass(get_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleTypeSpec {
    pub parens: Option<(Token, Token)>,
    pub tys: Vec<TypeSpec>,
}

impl fmt::Display for TupleTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})", fmt_vec(&self.tys))
    }
}

impl Locational for TupleTypeSpec {
    fn loc(&self) -> Location {
        if let Some((lparen, rparen)) = &self.parens {
            Location::concat(lparen, rparen)
        } else if !self.tys.is_empty() {
            Location::concat(self.tys.first().unwrap(), self.tys.last().unwrap())
        } else {
            Location::Unknown
        }
    }
}

impl TupleTypeSpec {
    pub const fn new(parens: Option<(Token, Token)>, tys: Vec<TypeSpec>) -> Self {
        Self { parens, tys }
    }
}

#[pyclass(get_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DictTypeSpec {
    pub braces: Option<(Token, Token)>,
    pub kvs: Vec<(TypeSpec, TypeSpec)>,
}

impl fmt::Display for DictTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (k, v) in self.kvs.iter() {
            write!(f, "{k}: {v}, ")?;
        }
        write!(f, "}}")
    }
}

impl Locational for DictTypeSpec {
    fn loc(&self) -> Location {
        if let Some((lparen, rparen)) = &self.braces {
            Location::concat(lparen, rparen)
        } else if !self.kvs.is_empty() {
            let (first, _) = self.kvs.first().unwrap();
            let (_, last) = self.kvs.last().unwrap();
            Location::concat(first, last)
        } else {
            Location::Unknown
        }
    }
}

impl DictTypeSpec {
    pub const fn new(braces: Option<(Token, Token)>, kvs: Vec<(TypeSpec, TypeSpec)>) -> Self {
        Self { braces, kvs }
    }
}

#[pyclass(get_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordTypeSpec {
    pub braces: Option<(Token, Token)>,
    pub attrs: Vec<(Identifier, TypeSpec)>,
}

impl fmt::Display for RecordTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (k, v) in self.attrs.iter() {
            write!(f, "{k} = {v}; ")?;
        }
        write!(f, "}}")
    }
}

impl Locational for RecordTypeSpec {
    fn loc(&self) -> Location {
        if let Some((lparen, rparen)) = &self.braces {
            Location::concat(lparen, rparen)
        } else if !self.attrs.is_empty() {
            let (first, _) = self.attrs.first().unwrap();
            let (_, last) = self.attrs.last().unwrap();
            Location::concat(first, last)
        } else {
            Location::Unknown
        }
    }
}

impl RecordTypeSpec {
    pub const fn new(braces: Option<(Token, Token)>, attrs: Vec<(Identifier, TypeSpec)>) -> Self {
        Self { braces, attrs }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefinementTypeSpec {
    pub var: Token,
    pub typ: Box<TypeSpec>,
    pub pred: ConstExpr,
}

impl fmt::Display for RefinementTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{}: {} | {}}}", self.var, self.typ, self.pred)
    }
}

impl Locational for RefinementTypeSpec {
    fn loc(&self) -> Location {
        Location::concat(&self.var, &self.pred)
    }
}

impl RefinementTypeSpec {
    pub fn new(var: Token, typ: TypeSpec, pred: ConstExpr) -> Self {
        Self {
            var,
            typ: Box::new(typ),
            pred,
        }
    }
}

/// * Array: `[Int; 3]`, `[Int, Ratio, Complex]`, etc.
/// * Dict: `[Str: Str]`, etc.
/// * And (Intersection type): Add and Sub and Mul (== Num), etc.
/// * Not (Diff type): Pos == Nat not {0}, etc.
/// * Or (Union type): Int or None (== Option Int), etc.
/// * Enum: `{0, 1}` (== Binary), etc.
/// * Range: 1..12, 0.0<..1.0, etc.
/// * Record: {.into_s: Self.() -> Str }, etc.
/// * Subr: Int -> Int, Int => None, T.(X) -> Int, etc.
/// * TypeApp: F|...|
/// * Refinement: {I: Int | I >= 0}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeSpec {
    Infer(Token),
    PreDeclTy(PreDeclTypeSpec),
    /* Composite types */
    Array(ArrayTypeSpec),
    SetWithLen(SetWithLenTypeSpec),
    Tuple(TupleTypeSpec),
    Dict(DictTypeSpec),
    Record(RecordTypeSpec),
    // Option(),
    And(Box<TypeSpec>, Box<TypeSpec>),
    Not(Box<TypeSpec>),
    Or(Box<TypeSpec>, Box<TypeSpec>),
    Enum(ConstArgs),
    Interval {
        op: Token,
        lhs: ConstExpr,
        rhs: ConstExpr,
    },
    Subr(SubrTypeSpec),
    TypeApp {
        spec: Box<TypeSpec>,
        args: TypeAppArgs,
    },
    Refinement(RefinementTypeSpec),
}

// TODO:
#[cfg(feature = "pylib")]
impl IntoPy<PyObject> for TypeSpec {
    fn into_py(self, py: Python<'_>) -> PyObject {
        pyo3::types::PyNone::get(py).into()
    }
}

impl fmt::Display for TypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Infer(_) => write!(f, "?"),
            Self::PreDeclTy(ty) => write!(f, "{ty}"),
            Self::And(lhs, rhs) => write!(f, "{lhs} and {rhs}"),
            Self::Not(ty) => write!(f, "not {ty}"),
            Self::Or(lhs, rhs) => write!(f, "{lhs} or {rhs}"),
            Self::Array(arr) => write!(f, "{arr}"),
            Self::SetWithLen(set) => write!(f, "{set}"),
            Self::Tuple(tup) => write!(f, "{tup}"),
            Self::Dict(dict) => dict.fmt(f),
            Self::Record(rec) => rec.fmt(f),
            Self::Enum(elems) => {
                write!(f, "{{")?;
                for elem in elems.pos_args() {
                    write!(f, "{}, ", elem.expr)?;
                }
                write!(f, "}}")
            }
            Self::Interval { op, lhs, rhs } => write!(f, "{lhs}{}{rhs}", op.inspect()),
            Self::Subr(s) => write!(f, "{s}"),
            Self::TypeApp { spec, args } => write!(f, "{spec}{args}"),
            Self::Refinement(r) => write!(f, "{r}"),
        }
    }
}

impl Locational for TypeSpec {
    fn loc(&self) -> Location {
        match self {
            Self::Infer(t) => t.loc(),
            Self::PreDeclTy(sig) => sig.loc(),
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                Location::concat(lhs.as_ref(), rhs.as_ref())
            }
            Self::Not(ty) => ty.loc(),
            Self::Array(arr) => arr.loc(),
            Self::SetWithLen(set) => set.loc(),
            Self::Tuple(tup) => tup.loc(),
            Self::Dict(dict) => dict.loc(),
            Self::Record(rec) => rec.loc(),
            Self::Enum(set) => set.loc(),
            Self::Interval { lhs, rhs, .. } => Location::concat(lhs, rhs),
            Self::Subr(s) => s.loc(),
            Self::TypeApp { spec, args } => Location::concat(spec.as_ref(), args),
            Self::Refinement(r) => r.loc(),
        }
    }
}

impl TypeSpec {
    pub fn and(lhs: TypeSpec, rhs: TypeSpec) -> Self {
        Self::And(Box::new(lhs), Box::new(rhs))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(lhs: TypeSpec) -> Self {
        Self::Not(Box::new(lhs))
    }

    pub fn or(lhs: TypeSpec, rhs: TypeSpec) -> Self {
        Self::Or(Box::new(lhs), Box::new(rhs))
    }

    pub const fn interval(op: Token, lhs: ConstExpr, rhs: ConstExpr) -> Self {
        Self::Interval { op, lhs, rhs }
    }

    pub fn type_app(spec: TypeSpec, args: TypeAppArgs) -> Self {
        Self::TypeApp {
            spec: Box::new(spec),
            args,
        }
    }

    pub fn enum_t_spec(elems: Vec<Literal>) -> Self {
        Self::Enum(ConstArgs::new(
            elems
                .into_iter()
                .map(|lit| ConstPosArg::new(ConstExpr::Lit(lit)))
                .collect(),
            None,
            vec![],
            None,
            None,
        ))
    }

    pub fn mono(ident: Identifier) -> Self {
        Self::PreDeclTy(PreDeclTypeSpec::Mono(ident))
    }

    pub fn poly(acc: ConstAccessor, args: ConstArgs) -> Self {
        Self::PreDeclTy(PreDeclTypeSpec::Poly(PolyTypeSpec::new(acc, args)))
    }

    pub fn ident(&self) -> Option<String> {
        match self {
            Self::PreDeclTy(predecl) => Some(predecl.ident()),
            Self::TypeApp { spec, .. } => spec.ident(),
            _ => None,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeSpecWithOp {
    pub op: Token,
    pub t_spec: TypeSpec,
    /// Required for dynamic type checking
    pub t_spec_as_expr: Box<Expr>,
}

impl NestedDisplay for TypeSpecWithOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} {}", self.op.content, self.t_spec)
    }
}

impl_display_from_nested!(TypeSpecWithOp);
impl_locational!(TypeSpecWithOp, lossy op, t_spec);

impl TypeSpecWithOp {
    pub fn new(op: Token, t_spec: TypeSpec, t_spec_as_expr: Expr) -> Self {
        Self {
            op,
            t_spec,
            t_spec_as_expr: Box::new(t_spec_as_expr),
        }
    }

    pub fn ascription_kind(&self) -> AscriptionKind {
        match self.op.kind {
            TokenKind::Colon => AscriptionKind::TypeOf,
            TokenKind::SubtypeOf => AscriptionKind::SubtypeOf,
            TokenKind::SupertypeOf => AscriptionKind::SupertypeOf,
            TokenKind::As => AscriptionKind::AsCast,
            kind => todo!("{kind}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeBoundSpec {
    Omitted(VarName),
    NonDefault {
        lhs: VarName,
        spec: TypeSpecWithOp,
    },
    WithDefault {
        lhs: VarName,
        spec: Box<TypeSpecWithOp>,
        default: ConstExpr,
    }, // e.g. S: Show := Str
}

impl NestedDisplay for TypeBoundSpec {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Omitted(name) => write!(f, "{name}"),
            Self::NonDefault { lhs, spec } => write!(f, "{lhs} {spec}"),
            Self::WithDefault { lhs, spec, default } => {
                write!(f, "{lhs} {spec} := {default}")
            }
        }
    }
}

impl_display_from_nested!(TypeBoundSpec);

impl Locational for TypeBoundSpec {
    fn loc(&self) -> Location {
        match self {
            Self::Omitted(name) => name.loc(),
            Self::NonDefault { lhs, spec } => Location::concat(lhs, spec),
            Self::WithDefault { lhs, default, .. } => Location::concat(lhs, default),
        }
    }
}

impl TypeBoundSpec {
    pub fn non_default(lhs: VarName, spec: TypeSpecWithOp) -> Self {
        Self::NonDefault { lhs, spec }
    }

    pub fn default(lhs: VarName, spec: TypeSpecWithOp, default: ConstExpr) -> Self {
        Self::WithDefault {
            lhs,
            spec: Box::new(spec),
            default,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeBoundSpecs(Vec<TypeBoundSpec>);

impl_displayable_stream_for_wrapper!(TypeBoundSpecs, TypeBoundSpec);

impl Locational for TypeBoundSpecs {
    fn loc(&self) -> Location {
        if self.0.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.first().unwrap(), self.last().unwrap())
        }
    }
}

/// デコレータは関数を返す関数オブジェクトならば何でも指定できる
/// e.g. @(x -> x)
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decorator(pub Expr);

impl Decorator {
    pub const fn new(expr: Expr) -> Self {
        Self(expr)
    }

    pub fn expr(&self) -> &Expr {
        &self.0
    }

    pub fn into_expr(self) -> Expr {
        self.0
    }
}

/// symbol as a left value
#[pyclass]
#[derive(Debug, Clone, Eq)]
pub struct VarName(Token);

impl PartialEq for VarName {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::hash::Hash for VarName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl Borrow<str> for VarName {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0.content[..]
    }
}

impl Borrow<Str> for VarName {
    #[inline]
    fn borrow(&self) -> &Str {
        &self.0.content
    }
}

impl From<&'static str> for VarName {
    #[inline]
    fn from(s: &'static str) -> Self {
        Self::from_static(s)
    }
}

impl Locational for VarName {
    #[inline]
    fn loc(&self) -> Location {
        self.0.loc()
    }
}

impl fmt::Display for VarName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.inspect())
    }
}

#[pymethods]
impl VarName {
    pub fn __repr__(&self) -> String {
        format!("VarName({})", self)
    }

    pub fn __str__(&self) -> String {
        format!("VarName({})", self)
    }

    #[staticmethod]
    pub const fn new(symbol: Token) -> Self {
        Self(symbol)
    }

    #[staticmethod]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(symbol: Str) -> Self {
        Self(Token::from_str(TokenKind::Symbol, &symbol))
    }

    #[staticmethod]
    pub fn from_str_and_line(symbol: Str, line: u32) -> Self {
        Self(Token::new_fake(TokenKind::Symbol, symbol, line, 0, 0))
    }

    #[staticmethod]
    pub fn from_str_and_loc(symbol: Str, loc: Location) -> Self {
        Self(Token::new_with_loc(TokenKind::Symbol, symbol, loc))
    }

    #[inline]
    pub fn is_const(&self) -> bool {
        self.0
            .content
            .chars()
            .next()
            .map_or(false, |c| c.is_uppercase())
    }

    #[inline]
    pub fn is_discarded(&self) -> bool {
        &self.0.content[..] == "_"
    }

    #[inline]
    pub fn is_procedural(&self) -> bool {
        self.0.content.chars().last().map_or(false, |c| c == '!')
    }

    pub fn is_raw(&self) -> bool {
        self.0.content.starts_with('\'')
    }

    pub fn is_generated(&self) -> bool {
        self.0.content.starts_with('%')
    }
}

impl VarName {
    pub const fn from_static(symbol: &'static str) -> Self {
        Self(Token::static_symbol(symbol))
    }

    pub const fn token(&self) -> &Token {
        &self.0
    }

    pub fn into_token(self) -> Token {
        self.0
    }

    /// Q: Why this does not return `&str`?
    /// A: `const`
    pub const fn inspect(&self) -> &Str {
        &self.0.content
    }

    /// Remove `!` from the end of the identifier.
    /// Procedures defined in `d.er` automatically register the name without `!` as `py_name`.
    /// This method is for undoing it (e.g. pylyzer-mode)
    pub fn trim_end_proc_mark(&mut self) {
        self.0.content = Str::rc(self.0.content.trim_end_matches('!'));
    }

    pub fn rename(&mut self, new: Str) {
        self.0.content = new;
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Namespaces(Vec<Accessor>);

impl_displayable_stream_for_wrapper!(Namespaces, Accessor);

impl NestedDisplay for Namespaces {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        for (i, ns) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{ns}")?;
        }
        Ok(())
    }
}

impl Locational for Namespaces {
    fn loc(&self) -> Location {
        if self.0.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.first().unwrap(), self.last().unwrap())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisRestriction {
    Namespaces(Namespaces),
    SubtypeOf(Box<TypeSpec>),
}

impl_locational_for_enum!(VisRestriction; Namespaces, SubtypeOf);
impl_display_from_nested!(VisRestriction);
impl_into_py_for_enum!(VisRestriction; Namespaces, SubtypeOf);

impl NestedDisplay for VisRestriction {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Namespaces(ns) => write!(f, "{ns}"),
            Self::SubtypeOf(ty) => write!(f, "<: {ty}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisModifierSpec {
    Private,
    Auto,
    Public(Token),
    ExplicitPrivate(Token),
    Restricted(VisRestriction),
}

#[cfg(feature = "pylib")]
impl IntoPy<PyObject> for VisModifierSpec {
    fn into_py(self, py: Python<'_>) -> PyObject {
        match self {
            Self::Private => py.None(),
            Self::Auto => py.None(),
            Self::Public(token) => token.into_py(py),
            Self::ExplicitPrivate(token) => token.into_py(py),
            Self::Restricted(rest) => rest.into_py(py),
        }
    }
}

impl NestedDisplay for VisModifierSpec {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Private => Ok(()),
            Self::Auto => write!(f, ":auto:"),
            Self::Public(_token) => write!(f, "."),
            Self::ExplicitPrivate(_token) => write!(f, "::"),
            Self::Restricted(rest) => write!(f, "::[{rest}]"),
        }
    }
}

impl_display_from_nested!(VisModifierSpec);

impl Locational for VisModifierSpec {
    fn loc(&self) -> Location {
        match self {
            Self::Private | Self::Auto => Location::Unknown,
            Self::Public(token) => token.loc(),
            Self::ExplicitPrivate(token) => token.loc(),
            Self::Restricted(rest) => rest.loc(),
        }
    }
}

impl VisModifierSpec {
    pub const fn is_public(&self) -> bool {
        matches!(self, Self::Public(_))
    }

    pub const fn is_private(&self) -> bool {
        matches!(self, Self::Private | Self::ExplicitPrivate(_))
    }

    pub const fn display_as_accessor(&self) -> &'static str {
        match self {
            Self::Auto => ":auto:",
            Self::Public(_) => ".",
            Self::Private | Self::Restricted(_) | Self::ExplicitPrivate(_) => "::",
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessModifier {
    Private, // `::`
    Public,  // `.`
    Auto,    // record unpacking
    Force,   // can access any identifiers
}

#[pyclass(get_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub vis: VisModifierSpec,
    pub name: VarName,
}

impl NestedDisplay for Identifier {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}{}", self.vis.display_as_accessor(), self.name)
    }
}

impl_display_from_nested!(Identifier);

impl Locational for Identifier {
    fn loc(&self) -> Location {
        // `ASTLinker` generates `vis` for the methods, so `vis.loc()` information cannot be relied upon.
        self.name.loc()
    }
}

impl From<Identifier> for Expr {
    fn from(ident: Identifier) -> Self {
        Self::Accessor(Accessor::Ident(ident))
    }
}

#[pymethods]
impl Identifier {
    pub fn __repr__(&self) -> String {
        format!("Identifier({})", self)
    }

    pub fn __str__(&self) -> String {
        format!("Identifier({})", self)
    }

    pub fn to_str_literal(&self) -> Literal {
        Literal::new(Token::new_with_loc(
            TokenKind::StrLit,
            Str::from(format!("\"{}\"", self.inspect())),
            self.loc(),
        ))
    }

    pub fn is_const(&self) -> bool {
        self.name.is_const()
    }

    pub fn is_discarded(&self) -> bool {
        self.name.is_discarded()
    }

    pub fn is_raw(&self) -> bool {
        self.name.is_raw()
    }

    pub fn acc_kind(&self) -> AccessModifier {
        match &self.vis {
            VisModifierSpec::Auto => AccessModifier::Auto,
            VisModifierSpec::Public(_) => AccessModifier::Public,
            VisModifierSpec::ExplicitPrivate(_)
            | VisModifierSpec::Restricted(_)
            | VisModifierSpec::Private => AccessModifier::Private,
        }
    }

    pub fn is_procedural(&self) -> bool {
        self.name.is_procedural()
    }

    pub fn trim_end_proc_mark(&mut self) {
        self.name.trim_end_proc_mark();
    }

    #[pyo3(name = "inspect")]
    fn _inspect(&self) -> Str {
        self.name.inspect().clone()
    }

    #[setter]
    pub fn set_name(&mut self, name: VarName) {
        self.name = name;
    }

    #[staticmethod]
    pub fn public(name: Str) -> Self {
        Self::new(
            VisModifierSpec::Public(Token::from_str(TokenKind::Dot, ".")),
            VarName::from_str(name),
        )
    }

    #[staticmethod]
    pub fn private(name: Str) -> Self {
        Self::new(VisModifierSpec::Private, VarName::from_str(name))
    }

    #[staticmethod]
    pub fn private_from_token(symbol: Token) -> Self {
        Self::new(VisModifierSpec::Private, VarName::new(symbol))
    }

    #[staticmethod]
    pub fn private_from_varname(name: VarName) -> Self {
        Self::new(VisModifierSpec::Private, name)
    }

    #[staticmethod]
    pub fn private_with_line(name: Str, line: u32) -> Self {
        Self::new(
            VisModifierSpec::Private,
            VarName::from_str_and_line(name, line),
        )
    }

    #[staticmethod]
    pub fn private_with_loc(name: Str, loc: Location) -> Self {
        Self::new(
            VisModifierSpec::Private,
            VarName::from_str_and_loc(name, loc),
        )
    }

    #[staticmethod]
    pub fn public_with_line(dot: Token, name: Str, line: u32) -> Self {
        Self::new(
            VisModifierSpec::Public(dot),
            VarName::from_str_and_line(name, line),
        )
    }

    #[staticmethod]
    pub fn public_with_loc(dot: Token, name: Str, loc: Location) -> Self {
        Self::new(
            VisModifierSpec::Public(dot),
            VarName::from_str_and_loc(name, loc),
        )
    }

    #[staticmethod]
    pub fn public_from_token(dot: Token, symbol: Token) -> Self {
        Self::new(VisModifierSpec::Public(dot), VarName::new(symbol))
    }

    #[staticmethod]
    pub fn auto(name: Str) -> Self {
        Self::new(VisModifierSpec::Auto, VarName::from_str(name))
    }
}

impl Identifier {
    pub fn static_public(name: &'static str) -> Self {
        Self::new(
            VisModifierSpec::Public(Token::from_str(TokenKind::Dot, ".")),
            VarName::from_static(name),
        )
    }

    pub const fn new(vis: VisModifierSpec, name: VarName) -> Self {
        Self { vis, name }
    }

    pub const fn inspect(&self) -> &Str {
        self.name.inspect()
    }

    pub fn call1(self, arg: Expr) -> Call {
        Call::new(
            self.into(),
            None,
            Args::pos_only(vec![PosArg::new(arg)], None),
        )
    }

    pub fn call2(self, arg1: Expr, arg2: Expr) -> Call {
        Call::new(
            self.into(),
            None,
            Args::pos_only(vec![PosArg::new(arg1), PosArg::new(arg2)], None),
        )
    }

    pub fn call(self, args: Args) -> Call {
        Call::new(self.into(), None, args)
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarArrayPattern {
    l_sqbr: Token,
    pub(crate) elems: Vars,
    r_sqbr: Token,
}

impl fmt::Display for VarArrayPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.elems)
    }
}

impl_locational!(VarArrayPattern, l_sqbr, r_sqbr);

impl Stream<VarSignature> for VarArrayPattern {
    #[inline]
    fn payload(self) -> Vec<VarSignature> {
        self.elems.payload()
    }
    #[inline]
    fn ref_payload(&self) -> &Vec<VarSignature> {
        self.elems.ref_payload()
    }
    #[inline]
    fn ref_mut_payload(&mut self) -> &mut Vec<VarSignature> {
        self.elems.ref_mut_payload()
    }
}

impl VarArrayPattern {
    pub const fn new(l_sqbr: Token, elems: Vars, r_sqbr: Token) -> Self {
        Self {
            l_sqbr,
            elems,
            r_sqbr,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarTuplePattern {
    pub(crate) paren: Option<(Token, Token)>,
    pub(crate) elems: Vars,
}

impl fmt::Display for VarTuplePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})", self.elems)
    }
}

impl Locational for VarTuplePattern {
    fn loc(&self) -> Location {
        match &self.paren {
            Some((l, r)) => Location::concat(l, r),
            None => self.elems.loc(),
        }
    }
}

impl Stream<VarSignature> for VarTuplePattern {
    #[inline]
    fn payload(self) -> Vec<VarSignature> {
        self.elems.payload()
    }
    #[inline]
    fn ref_payload(&self) -> &Vec<VarSignature> {
        self.elems.ref_payload()
    }
    #[inline]
    fn ref_mut_payload(&mut self) -> &mut Vec<VarSignature> {
        self.elems.ref_mut_payload()
    }
}

impl VarTuplePattern {
    pub const fn new(paren: Option<(Token, Token)>, elems: Vars) -> Self {
        Self { paren, elems }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarRecordAttr {
    pub lhs: Identifier,
    pub rhs: VarSignature,
}

impl NestedDisplay for VarRecordAttr {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} = {}", self.lhs, self.rhs)
    }
}

impl_display_from_nested!(VarRecordAttr);
impl_locational!(VarRecordAttr, lhs, rhs);

impl VarRecordAttr {
    pub const fn new(lhs: Identifier, rhs: VarSignature) -> Self {
        Self { lhs, rhs }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarRecordAttrs {
    pub(crate) elems: Vec<VarRecordAttr>,
}

impl NestedDisplay for VarRecordAttrs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", fmt_vec_split_with(&self.elems, "; "))
    }
}

impl_display_from_nested!(VarRecordAttrs);
impl_stream!(VarRecordAttrs, VarRecordAttr, elems);

impl VarRecordAttrs {
    pub const fn new(elems: Vec<VarRecordAttr>) -> Self {
        Self { elems }
    }

    pub const fn empty() -> Self {
        Self::new(vec![])
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarRecordPattern {
    l_brace: Token,
    pub(crate) attrs: VarRecordAttrs,
    r_brace: Token,
}

impl fmt::Display for VarRecordPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{}}}", self.attrs)
    }
}

impl_locational!(VarRecordPattern, l_brace, r_brace);

impl VarRecordPattern {
    pub const fn new(l_brace: Token, attrs: VarRecordAttrs, r_brace: Token) -> Self {
        Self {
            l_brace,
            attrs,
            r_brace,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarDataPackPattern {
    pub class: TypeSpec,
    pub class_as_expr: Box<Expr>,
    pub args: VarRecordPattern,
}

impl fmt::Display for VarDataPackPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.class, self.args)
    }
}

impl_locational!(VarDataPackPattern, class, args);

impl VarDataPackPattern {
    pub const fn new(class: TypeSpec, class_as_expr: Box<Expr>, args: VarRecordPattern) -> Self {
        Self {
            class,
            class_as_expr,
            args,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VarPattern {
    Discard(Token),
    Ident(Identifier),
    /// e.g. `[x, y, z]` of `[x, y, z] = [1, 2, 3]`
    Array(VarArrayPattern),
    /// e.g. `(x, y, z)` of `(x, y, z) = (1, 2, 3)`
    Tuple(VarTuplePattern),
    // e.g. `{name; age}`, `{_; [car, cdr]}`
    Record(VarRecordPattern),
    // e.g. `Data::{x, y}`
    DataPack(VarDataPackPattern),
    Splice(Splice),
}

impl NestedDisplay for VarPattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Discard(_) => write!(f, "_"),
            Self::Ident(ident) => write!(f, "{ident}"),
            Self::Array(a) => write!(f, "{a}"),
            Self::Tuple(t) => write!(f, "{t}"),
            Self::Record(r) => write!(f, "{r}"),
            Self::DataPack(d) => write!(f, "{d}"),
            Self::Splice(s) => write!(f, "{s}"),
        }
    }
}

impl_display_from_nested!(VarPattern);
impl_locational_for_enum!(VarPattern; Discard, Ident, Array, Tuple, Record, DataPack, Splice);
impl_into_py_for_enum!(VarPattern; Discard, Ident, Array, Tuple, Record, DataPack, Splice);
impl_from_py_for_enum!(VarPattern; Discard(Token), Ident(Identifier), Array(VarArrayPattern), Tuple(VarTuplePattern), Record(VarRecordPattern), DataPack(VarDataPackPattern), Splice(Splice));

impl VarPattern {
    pub const fn inspect(&self) -> Option<&Str> {
        match self {
            Self::Ident(ident) => Some(ident.inspect()),
            _ => None,
        }
    }

    pub fn escaped(&self) -> Option<Str> {
        match self {
            Self::Ident(ident) => {
                let inspect = ident.inspect();
                Some(Str::rc(
                    inspect.trim_end_matches('!').trim_start_matches('$'),
                ))
            }
            _ => None,
        }
    }

    // _!(...) = ... is invalid
    pub fn is_procedural(&self) -> bool {
        match self {
            Self::Ident(ident) => ident.is_procedural(),
            _ => false,
        }
    }

    // _ = (type block) is invalid
    pub fn is_const(&self) -> bool {
        match self {
            Self::Ident(ident) => ident.is_const(),
            _ => false,
        }
    }

    pub const fn vis(&self) -> &VisModifierSpec {
        match self {
            Self::Ident(ident) => &ident.vis,
            // TODO: `[.x, .y]`?
            _ => &VisModifierSpec::Private,
        }
    }

    pub const fn ident(&self) -> Option<&Identifier> {
        match self {
            Self::Ident(ident) => Some(ident),
            _ => None,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarSignature {
    pub pat: VarPattern,
    pub t_spec: Option<TypeSpecWithOp>,
}

impl NestedDisplay for VarSignature {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}{}", self.pat, fmt_option!(pre ": ", self.t_spec))
    }
}

impl_display_from_nested!(VarSignature);

impl Locational for VarSignature {
    fn loc(&self) -> Location {
        if let Some(t_spec) = &self.t_spec {
            Location::concat(&self.pat, t_spec)
        } else {
            self.pat.loc()
        }
    }
}

#[pymethods]
impl VarSignature {
    #[pyo3(name = "inspect")]
    fn _inspect(&self) -> Option<Str> {
        self.pat.inspect().cloned()
    }

    #[pyo3(name = "ident")]
    fn _ident(&self) -> Option<Identifier> {
        match &self.pat {
            VarPattern::Ident(ident) => Some(ident),
            _ => None,
        }
        .cloned()
    }

    #[staticmethod]
    pub const fn new(pat: VarPattern, t_spec: Option<TypeSpecWithOp>) -> Self {
        Self { pat, t_spec }
    }

    pub fn is_const(&self) -> bool {
        self.pat.is_const()
    }

    pub fn __repr__(&self) -> String {
        format!("VarSignature({})", self)
    }

    pub fn __str__(&self) -> String {
        format!("VarSignature({})", self)
    }
}

impl VarSignature {
    pub const fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }

    pub fn escaped(&self) -> Option<Str> {
        self.pat.escaped()
    }

    pub const fn vis(&self) -> &VisModifierSpec {
        self.pat.vis()
    }

    pub fn ident(&self) -> Option<&Identifier> {
        match &self.pat {
            VarPattern::Ident(ident) => Some(ident),
            _ => None,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Vars {
    pub(crate) elems: Vec<VarSignature>,
    pub(crate) starred: Option<Box<VarSignature>>,
}

impl NestedDisplay for Vars {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", fmt_vec(&self.elems))?;
        if let Some(starred) = &self.starred {
            write!(f, ", *{starred}")?;
        }
        Ok(())
    }
}

impl_display_from_nested!(Vars);
impl_stream!(Vars, VarSignature, elems);

impl Locational for Vars {
    fn loc(&self) -> Location {
        if self.elems.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.first().unwrap(), self.last().unwrap())
        }
    }
}

impl Vars {
    pub fn new(elems: Vec<VarSignature>, starred: Option<VarSignature>) -> Self {
        Self {
            elems,
            starred: starred.map(Box::new),
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], None)
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamArrayPattern {
    pub l_sqbr: Token,
    pub elems: Params,
    pub r_sqbr: Token,
}

impl NestedDisplay for ParamArrayPattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "[{}]", self.elems)
    }
}

impl_display_from_nested!(ParamArrayPattern);
impl_locational!(ParamArrayPattern, l_sqbr, r_sqbr);

impl TryFrom<&ParamArrayPattern> for Expr {
    type Error = ();

    fn try_from(value: &ParamArrayPattern) -> Result<Self, Self::Error> {
        let mut new = vec![];
        for elem in value.elems.non_defaults.iter() {
            new.push(PosArg::new(Expr::try_from(&elem.pat)?));
        }
        let elems = Args::pos_only(new, None);
        Ok(Expr::Array(Array::Normal(NormalArray::new(
            value.l_sqbr.clone(),
            value.r_sqbr.clone(),
            elems,
        ))))
    }
}

impl TryFrom<&ParamArrayPattern> for ConstExpr {
    type Error = ();

    fn try_from(value: &ParamArrayPattern) -> Result<Self, Self::Error> {
        let mut new = vec![];
        for elem in value.elems.non_defaults.iter() {
            new.push(ConstPosArg::new(ConstExpr::try_from(&elem.pat)?));
        }
        let elems = ConstArgs::pos_only(new, None);
        Ok(ConstExpr::Array(ConstArray::Normal(ConstNormalArray::new(
            value.l_sqbr.clone(),
            value.r_sqbr.clone(),
            elems,
            None,
        ))))
    }
}

#[pymethods]
impl ParamArrayPattern {
    #[staticmethod]
    pub const fn new(l_sqbr: Token, elems: Params, r_sqbr: Token) -> Self {
        Self {
            l_sqbr,
            elems,
            r_sqbr,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }
    pub fn len(&self) -> usize {
        self.elems.len()
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamTuplePattern {
    pub elems: Params,
}

impl NestedDisplay for ParamTuplePattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "({})", self.elems)
    }
}

impl_display_from_nested!(ParamTuplePattern);
impl_locational!(ParamTuplePattern, elems);

impl TryFrom<&ParamTuplePattern> for Expr {
    type Error = ();

    fn try_from(value: &ParamTuplePattern) -> Result<Self, Self::Error> {
        let mut new = vec![];
        for elem in value.elems.non_defaults.iter() {
            new.push(PosArg::new(Expr::try_from(&elem.pat)?));
        }
        let elems = Args::pos_only(new, value.elems.parens.clone());
        Ok(Expr::Tuple(Tuple::Normal(NormalTuple::new(elems))))
    }
}

impl TryFrom<&ParamTuplePattern> for ConstExpr {
    type Error = ();

    fn try_from(value: &ParamTuplePattern) -> Result<Self, Self::Error> {
        let mut new = vec![];
        for elem in value.elems.non_defaults.iter() {
            new.push(ConstPosArg::new(ConstExpr::try_from(&elem.pat)?));
        }
        let elems = ConstArgs::pos_only(new, value.elems.parens.clone());
        Ok(ConstExpr::Tuple(ConstTuple::new(elems)))
    }
}

impl ParamTuplePattern {
    pub const fn new(elems: Params) -> Self {
        Self { elems }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamRecordAttr {
    pub lhs: Identifier,
    pub rhs: NonDefaultParamSignature,
}

impl NestedDisplay for ParamRecordAttr {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} = {}", self.lhs, self.rhs)
    }
}

impl_display_from_nested!(ParamRecordAttr);
impl_locational!(ParamRecordAttr, lhs, rhs);

impl ParamRecordAttr {
    pub const fn new(lhs: Identifier, rhs: NonDefaultParamSignature) -> Self {
        Self { lhs, rhs }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamRecordAttrs {
    pub(crate) elems: Vec<ParamRecordAttr>,
}

impl NestedDisplay for ParamRecordAttrs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", fmt_vec_split_with(&self.elems, "; "))
    }
}

impl_display_from_nested!(ParamRecordAttrs);
impl_stream!(ParamRecordAttrs, ParamRecordAttr, elems);

#[pymethods]
impl ParamRecordAttrs {
    #[staticmethod]
    pub const fn new(elems: Vec<ParamRecordAttr>) -> Self {
        Self { elems }
    }

    #[staticmethod]
    pub const fn empty() -> Self {
        Self::new(vec![])
    }
}

impl ParamRecordAttrs {
    pub fn keys(&self) -> impl Iterator<Item = &Identifier> {
        self.elems.iter().map(|attr| &attr.lhs)
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamRecordPattern {
    pub(crate) l_brace: Token,
    pub(crate) elems: ParamRecordAttrs,
    pub(crate) r_brace: Token,
}

impl NestedDisplay for ParamRecordPattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "{}{{{}}}", " ".repeat(level), self.elems)
    }
}

impl_display_from_nested!(ParamRecordPattern);
impl_locational!(ParamRecordPattern, l_brace, r_brace);

#[pymethods]
impl ParamRecordPattern {
    #[staticmethod]
    pub const fn new(l_brace: Token, elems: ParamRecordAttrs, r_brace: Token) -> Self {
        Self {
            l_brace,
            elems,
            r_brace,
        }
    }
}

/// 関数定義や無名関数で使えるパターン
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamPattern {
    Discard(Token),
    VarName(VarName),
    // TODO: ConstAttr(),
    Lit(Literal),
    Array(ParamArrayPattern),
    Tuple(ParamTuplePattern),
    Record(ParamRecordPattern),
    // DataPack(ParamDataPackPattern),
    Ref(VarName),
    RefMut(VarName),
    Splice(Splice),
}

impl_into_py_for_enum!(ParamPattern; Discard, VarName, Lit, Array, Tuple, Record, Ref, RefMut, Splice);

impl NestedDisplay for ParamPattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Discard(tok) => write!(f, "{tok}"),
            Self::VarName(var_name) => write!(f, "{var_name}"),
            Self::Lit(lit) => write!(f, "{lit}"),
            Self::Array(array) => write!(f, "{array}"),
            Self::Tuple(tuple) => write!(f, "{tuple}"),
            Self::Record(record) => write!(f, "{record}"),
            Self::Ref(var_name) => write!(f, "ref {var_name}"),
            Self::RefMut(var_name) => write!(f, "ref! {var_name}"),
            Self::Splice(splice) => write!(f, "{splice}"),
        }
    }
}

impl TryFrom<&ParamPattern> for Expr {
    type Error = ();
    fn try_from(value: &ParamPattern) -> Result<Self, Self::Error> {
        match value {
            // ParamPattern::Discard(token) => Ok(Expr::Accessor(Accessor::local(token.clone()))),
            ParamPattern::VarName(name) if name.inspect() != "_" => {
                Ok(Expr::Accessor(Accessor::local(name.0.clone())))
            }
            ParamPattern::Lit(lit) => Ok(Expr::Literal(lit.clone())),
            ParamPattern::Array(array) => Expr::try_from(array),
            ParamPattern::Tuple(tuple) => Expr::try_from(tuple),
            // ParamPattern::Record(record) => Expr::try_from(record),
            ParamPattern::Splice(splice) => Ok(Expr::Splice(splice.clone())),
            _ => Err(()),
        }
    }
}

impl_display_from_nested!(ParamPattern);
impl_locational_for_enum!(ParamPattern; Discard, VarName, Lit, Array, Tuple, Record, Ref, RefMut, Splice);

impl ParamPattern {
    pub const fn inspect(&self) -> Option<&Str> {
        match self {
            Self::VarName(n) | Self::Ref(n) | Self::RefMut(n) => Some(n.inspect()),
            _ => None,
        }
    }

    pub const fn is_lit(&self) -> bool {
        matches!(self, Self::Lit(_))
    }

    pub fn is_procedural(&self) -> bool {
        match self {
            Self::Discard(_) => true,
            Self::VarName(n) | Self::Ref(n) | Self::RefMut(n) => n.is_procedural(),
            _ => false,
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Discard(_) => true,
            Self::VarName(n) | Self::Ref(n) | Self::RefMut(n) => n.is_const(),
            _ => false,
        }
    }

    pub const fn name(&self) -> Option<&VarName> {
        match self {
            Self::VarName(n) | Self::Ref(n) | Self::RefMut(n) => Some(n),
            _ => None,
        }
    }
}

/// Once the default_value is set to Some, all subsequent values must be Some
#[pyclass(get_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NonDefaultParamSignature {
    pub pat: ParamPattern,
    pub t_spec: Option<TypeSpecWithOp>,
}

impl NestedDisplay for NonDefaultParamSignature {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "{}{}", self.pat, fmt_option!(self.t_spec),)
    }
}

impl_display_from_nested!(NonDefaultParamSignature);

impl Locational for NonDefaultParamSignature {
    fn loc(&self) -> Location {
        if let Some(t_spec) = &self.t_spec {
            Location::left_main_concat(&self.pat, t_spec)
        } else {
            self.pat.loc()
        }
    }
}

#[pymethods]
impl NonDefaultParamSignature {
    #[staticmethod]
    pub fn simple(name: Str) -> Self {
        Self::new(ParamPattern::VarName(VarName::from_str(name)), None)
    }
}

impl NonDefaultParamSignature {
    pub const fn new(pat: ParamPattern, t_spec: Option<TypeSpecWithOp>) -> Self {
        Self { pat, t_spec }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }

    pub const fn name(&self) -> Option<&VarName> {
        self.pat.name()
    }
}

/// Once the default_value is set to Some, all subsequent values must be Some
#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefaultParamSignature {
    pub sig: NonDefaultParamSignature,
    pub default_val: Expr,
}

impl NestedDisplay for DefaultParamSignature {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "{} := {}", self.sig, self.default_val,)
    }
}

impl_display_from_nested!(DefaultParamSignature);

impl Locational for DefaultParamSignature {
    fn loc(&self) -> Location {
        Location::concat(&self.sig, &self.default_val)
    }
}

#[pymethods]
impl DefaultParamSignature {
    #[staticmethod]
    pub const fn new(sig: NonDefaultParamSignature, default_val: Expr) -> Self {
        Self { sig, default_val }
    }
}

impl DefaultParamSignature {
    pub const fn inspect(&self) -> Option<&Str> {
        self.sig.pat.inspect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GuardClause {
    Condition(Expr),
    Bind(Def),
}

impl NestedDisplay for GuardClause {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        match self {
            Self::Condition(cond) => write!(f, "{}", cond),
            Self::Bind(def) => write!(f, "{}", def),
        }
    }
}

impl_display_from_nested!(GuardClause);
impl_into_py_for_enum!(GuardClause; Condition, Bind);
impl_from_py_for_enum!(GuardClause; Condition(Expr), Bind(Def));

impl Locational for GuardClause {
    fn loc(&self) -> Location {
        match self {
            Self::Condition(cond) => cond.loc(),
            Self::Bind(def) => def.loc(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Params {
    pub non_defaults: Vec<NonDefaultParamSignature>,
    pub var_params: Option<Box<NonDefaultParamSignature>>,
    pub defaults: Vec<DefaultParamSignature>,
    pub kw_var_params: Option<Box<NonDefaultParamSignature>>,
    /// match conditions
    pub guards: Vec<GuardClause>,
    pub parens: Option<(Token, Token)>,
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}", fmt_vec(&self.non_defaults))?;
        if let Some(var_params) = &self.var_params {
            write!(f, ", *{var_params}")?;
        }
        if !self.defaults.is_empty() {
            write!(f, ", {}", fmt_vec(&self.defaults))?;
        }
        if let Some(kw_var_params) = &self.kw_var_params {
            write!(f, ", **{kw_var_params}")?;
        }
        if !self.guards.is_empty() {
            write!(f, " if ")?;
        }
        for (i, guard) in self.guards.iter().enumerate() {
            if i > 0 {
                write!(f, " and ")?;
            }
            write!(f, "{guard}")?;
        }
        write!(f, ")")
    }
}

impl Locational for Params {
    fn loc(&self) -> Location {
        if let Some((l, r)) = &self.parens {
            let loc = Location::concat(l, r);
            if !loc.is_unknown() {
                return loc;
            }
        }
        match (
            self.non_defaults.first().zip(self.non_defaults.last()),
            self.var_params.as_ref(),
            self.defaults.first().zip(self.defaults.last()),
        ) {
            (Some((l, _)), _, Some((_, r))) => Location::concat(l, r),
            (Some((l, _)), Some(r), None) => Location::concat(l, r.as_ref()),
            (Some((l, r)), None, None) => Location::concat(l, r),
            (None, Some(l), Some((_, r))) => Location::concat(l.as_ref(), r),
            (None, None, Some((l, r))) => Location::concat(l, r),
            (None, Some(l), None) => l.loc(),
            (None, None, None) => Location::Unknown,
        }
    }
}

type RawParams = (
    Vec<NonDefaultParamSignature>,
    Option<Box<NonDefaultParamSignature>>,
    Vec<DefaultParamSignature>,
    Option<Box<NonDefaultParamSignature>>,
    Vec<GuardClause>,
    Option<(Token, Token)>,
);

#[pymethods]
impl Params {
    #[staticmethod]
    #[pyo3(signature = (non_defaults, var_params, defaults, kw_var_params=None, parens=None))]
    pub fn new(
        non_defaults: Vec<NonDefaultParamSignature>,
        var_params: Option<NonDefaultParamSignature>,
        defaults: Vec<DefaultParamSignature>,
        kw_var_params: Option<NonDefaultParamSignature>,
        parens: Option<(Token, Token)>,
    ) -> Self {
        Self {
            non_defaults,
            var_params: var_params.map(Box::new),
            defaults,
            kw_var_params: kw_var_params.map(Box::new),
            guards: Vec::new(),
            parens,
        }
    }

    #[staticmethod]
    pub fn empty() -> Self {
        Self::new(vec![], None, vec![], None, None)
    }

    #[staticmethod]
    pub fn single(non_default: NonDefaultParamSignature) -> Self {
        Self::new(vec![non_default], None, vec![], None, None)
    }

    #[getter]
    pub fn non_defaults(&self) -> Vec<NonDefaultParamSignature> {
        self.non_defaults.clone()
    }

    #[getter]
    pub fn var_params(&self) -> Option<NonDefaultParamSignature> {
        self.var_params.as_deref().cloned()
    }

    #[getter]
    pub fn defaults(&self) -> Vec<DefaultParamSignature> {
        self.defaults.clone()
    }

    #[getter]
    pub fn kw_var_params(&self) -> Option<NonDefaultParamSignature> {
        self.kw_var_params.as_deref().cloned()
    }

    #[getter]
    pub fn guards(&self) -> Vec<GuardClause> {
        self.guards.clone()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.non_defaults.len() + self.defaults.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn add_guard(&mut self, guard: GuardClause) {
        self.guards.push(guard);
    }

    pub fn extend_guards(&mut self, guards: Vec<GuardClause>) {
        self.guards.extend(guards);
    }
}

impl Params {
    pub fn deconstruct(self) -> RawParams {
        (
            self.non_defaults,
            self.var_params,
            self.defaults,
            self.kw_var_params,
            self.guards,
            self.parens,
        )
    }

    pub fn sigs(&self) -> impl Iterator<Item = &NonDefaultParamSignature> {
        self.non_defaults
            .iter()
            .chain(self.var_params.as_deref())
            .chain(self.defaults.iter().map(|d| &d.sig))
    }
}

/// 引数を取るならTypeでもSubr扱い
#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubrSignature {
    pub decorators: HashSet<Decorator>,
    pub ident: Identifier,
    pub bounds: TypeBoundSpecs,
    pub params: Params,
    pub return_t_spec: Option<TypeSpecWithOp>,
}

impl NestedDisplay for SubrSignature {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if self.bounds.is_empty() {
            write!(
                f,
                "{}{}{}",
                self.ident,
                self.params,
                fmt_option!(pre ": ", self.return_t_spec)
            )
        } else {
            write!(
                f,
                "{}|{}|{}{}",
                self.ident,
                self.bounds,
                self.params,
                fmt_option!(pre ": ", self.return_t_spec)
            )
        }
    }
}

impl_display_from_nested!(SubrSignature);

impl Locational for SubrSignature {
    fn loc(&self) -> Location {
        if !self.bounds.is_empty() {
            Location::concat(&self.ident, &self.bounds)
        } else if let Some(return_t) = &self.return_t_spec {
            Location::concat(&self.ident, return_t)
        } else {
            Location::concat(&self.ident, &self.params)
        }
    }
}

impl SubrSignature {
    pub const fn new(
        decorators: HashSet<Decorator>,
        ident: Identifier,
        bounds: TypeBoundSpecs,
        params: Params,
        return_t_spec: Option<TypeSpecWithOp>,
    ) -> Self {
        Self {
            decorators,
            ident,
            bounds,
            params,
            return_t_spec,
        }
    }

    pub fn is_const(&self) -> bool {
        self.ident.is_const()
    }

    pub fn vis(&self) -> &VisModifierSpec {
        &self.ident.vis
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LambdaSignature {
    pub bounds: TypeBoundSpecs,
    pub params: Params,
    pub return_t_spec: Option<TypeSpecWithOp>,
}

impl fmt::Display for LambdaSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bounds.is_empty() {
            write!(
                f,
                "{}{}",
                self.params,
                fmt_option!(pre ": ", self.return_t_spec)
            )
        } else {
            write!(
                f,
                "|{}|{}{}",
                self.bounds,
                self.params,
                fmt_option!(pre ": ", self.return_t_spec)
            )
        }
    }
}

impl Locational for LambdaSignature {
    fn loc(&self) -> Location {
        if !self.bounds.is_empty() {
            Location::concat(&self.params, &self.bounds)
        } else if let Some(return_t) = &self.return_t_spec {
            Location::concat(&self.params, return_t)
        } else if self.params.is_empty() && self.params.parens.is_none() {
            Location::Unknown
        } else {
            self.params.loc()
        }
    }
}

impl LambdaSignature {
    pub const fn new(
        params: Params,
        return_t_spec: Option<TypeSpecWithOp>,
        bounds: TypeBoundSpecs,
    ) -> Self {
        Self {
            params,
            return_t_spec,
            bounds,
        }
    }

    pub fn do_sig(do_symbol: &Token) -> Self {
        let parens = Some((do_symbol.clone(), do_symbol.clone()));
        Self::new(
            Params::new(vec![], None, vec![], None, parens),
            None,
            TypeBoundSpecs::empty(),
        )
    }
}

#[pyclass(subclass)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DefId(pub usize);

impl DefId {
    pub fn inc(&mut self) {
        self.0 += 1;
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Lambda {
    pub sig: LambdaSignature,
    /// for detecting func/proc
    pub op: Token,
    pub body: Block,
    pub id: DefId,
}

impl NestedDisplay for Lambda {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{} {}", self.sig, self.op.content)?;
        self.body.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Lambda);

#[pymethods]
impl Lambda {
    #[staticmethod]
    pub const fn new(sig: LambdaSignature, op: Token, body: Block, id: DefId) -> Self {
        Self { sig, op, body, id }
    }

    pub fn is_procedural(&self) -> bool {
        self.op.is(TokenKind::ProcArrow)
    }
}

impl_locational!(Lambda, sig, body);

/// represents a declaration of a variable
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Signature {
    Var(VarSignature),
    Subr(SubrSignature),
}

impl_nested_display_for_chunk_enum!(Signature; Var, Subr);
impl_display_from_nested!(Signature);
impl_locational_for_enum!(Signature; Var, Subr);
impl_into_py_for_enum!(Signature; Var, Subr);
impl_from_py_for_enum!(Signature; Var(VarSignature), Subr(SubrSignature));

impl Signature {
    pub fn name_as_str(&self) -> Option<&Str> {
        match self {
            Self::Var(var) => var.pat.inspect(),
            Self::Subr(subr) => Some(subr.ident.inspect()),
        }
    }

    pub fn new_var(ident: Identifier) -> Self {
        Self::Var(VarSignature::new(VarPattern::Ident(ident), None))
    }

    pub fn new_subr(ident: Identifier, params: Params) -> Self {
        Self::Subr(SubrSignature::new(
            HashSet::new(),
            ident,
            TypeBoundSpecs::empty(),
            params,
            None,
        ))
    }

    pub fn ident(&self) -> Option<&Identifier> {
        match self {
            Self::Var(var) => {
                if let VarPattern::Ident(ident) = &var.pat {
                    Some(ident)
                } else {
                    None
                }
            }
            Self::Subr(subr) => Some(&subr.ident),
        }
    }

    pub fn ident_mut(&mut self) -> Option<&mut Identifier> {
        match self {
            Self::Var(var) => {
                if let VarPattern::Ident(ident) = &mut var.pat {
                    Some(ident)
                } else {
                    None
                }
            }
            Self::Subr(subr) => Some(&mut subr.ident),
        }
    }

    pub fn params(self) -> Option<Params> {
        match self {
            Self::Var(_) => None,
            Self::Subr(subr) => Some(subr.params),
        }
    }

    pub fn decorators(&self) -> Option<&HashSet<Decorator>> {
        match self {
            Self::Var(_) => None,
            Self::Subr(subr) => Some(&subr.decorators),
        }
    }

    pub fn t_spec(&self) -> Option<&TypeSpec> {
        match self {
            Self::Var(v) => v.t_spec.as_ref().map(|t| &t.t_spec),
            Self::Subr(c) => c.return_t_spec.as_ref().map(|t| &t.t_spec),
        }
    }

    pub fn t_spec_op_mut(&mut self) -> Option<&mut TypeSpecWithOp> {
        match self {
            Self::Var(v) => v.t_spec.as_mut(),
            Self::Subr(c) => c.return_t_spec.as_mut(),
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Var(var) => var.is_const(),
            Self::Subr(subr) => subr.is_const(),
        }
    }

    pub const fn is_subr(&self) -> bool {
        matches!(self, Self::Subr(_))
    }

    pub const fn is_var(&self) -> bool {
        matches!(self, Self::Var(_))
    }

    pub fn vis(&self) -> &VisModifierSpec {
        match self {
            Self::Var(var) => var.vis(),
            Self::Subr(subr) => subr.vis(),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, Copy)]
pub enum AscriptionKind {
    TypeOf,
    SubtypeOf,
    SupertypeOf,
    AsCast,
}

impl AscriptionKind {
    pub const fn is_force_cast(&self) -> bool {
        matches!(self, Self::AsCast)
    }
}

/// type_ascription ::= expr ':' type
///                   | expr '<:' type
///                   | expr ':>' type
#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeAscription {
    pub expr: Box<Expr>,
    pub t_spec: TypeSpecWithOp,
}

impl NestedDisplay for TypeAscription {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        writeln!(f, "{} {}", self.expr, self.t_spec)
    }
}

impl_display_from_nested!(TypeAscription);
impl_locational!(TypeAscription, expr, t_spec);

#[pymethods]
impl TypeAscription {
    #[staticmethod]
    pub fn new(expr: Expr, t_spec: TypeSpecWithOp) -> Self {
        Self {
            expr: Box::new(expr),
            t_spec,
        }
    }
}

impl TypeAscription {
    pub fn kind(&self) -> AscriptionKind {
        self.t_spec.ascription_kind()
    }
}

#[pyclass]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DefKind {
    Class,
    Inherit,
    Trait,
    Subsume,
    StructuralTrait,
    ErgImport,
    PyImport,
    RsImport,
    Patch,
    InlineModule,
    /// type alias included
    Other,
}

#[pymethods]
impl DefKind {
    pub const fn is_trait(&self) -> bool {
        matches!(self, Self::Trait | Self::Subsume | Self::StructuralTrait)
    }

    pub const fn is_class(&self) -> bool {
        matches!(self, Self::Class | Self::Inherit)
    }

    pub const fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit)
    }

    pub const fn is_class_or_trait(&self) -> bool {
        self.is_class() || self.is_trait()
    }

    pub const fn is_erg_import(&self) -> bool {
        matches!(self, Self::ErgImport)
    }

    pub const fn is_py_import(&self) -> bool {
        matches!(self, Self::PyImport)
    }

    pub const fn is_rs_import(&self) -> bool {
        matches!(self, Self::RsImport)
    }

    pub const fn is_import(&self) -> bool {
        self.is_erg_import() || self.is_py_import() || self.is_rs_import()
    }

    pub fn is_inline_module(&self) -> bool {
        matches!(self, Self::InlineModule)
    }

    pub const fn is_other(&self) -> bool {
        matches!(self, Self::Other)
    }
}

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefBody {
    pub op: Token,
    pub block: Block,
    pub id: DefId,
}

impl_locational!(DefBody, lossy op, block);

#[pymethods]
impl DefBody {
    #[staticmethod]
    pub const fn new(op: Token, block: Block, id: DefId) -> Self {
        Self { op, block, id }
    }

    #[staticmethod]
    pub fn new_single(expr: Expr) -> Self {
        Self::new(EQUAL, Block::new(vec![expr]), DefId(0))
    }

    pub fn def_kind(&self) -> DefKind {
        match self.block.first().unwrap() {
            Expr::Call(call) => match call.obj.get_name().map(|n| &n[..]) {
                Some("Class") => DefKind::Class,
                Some("Inherit") => DefKind::Inherit,
                Some("Trait") => DefKind::Trait,
                Some("Subsume") => DefKind::Subsume,
                Some("Inheritable") => {
                    if let Some(Expr::Call(inner)) = call.args.get_left_or_key("Class") {
                        match inner.obj.get_name().map(|n| &n[..]) {
                            Some("Class") => DefKind::Class,
                            Some("Inherit") => DefKind::Inherit,
                            _ => DefKind::Other,
                        }
                    } else {
                        DefKind::Other
                    }
                }
                Some("Patch") => DefKind::Patch,
                Some("import") => DefKind::ErgImport,
                Some("pyimport") | Some("py") | Some("__import__") => DefKind::PyImport,
                Some("rsimport") => DefKind::RsImport,
                _ => DefKind::Other,
            },
            Expr::InlineModule(_) => DefKind::InlineModule,
            _ => DefKind::Other,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Def {
    pub sig: Signature,
    pub body: DefBody,
}

impl NestedDisplay for Def {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        self.sig.fmt_nest(f, level)?;
        writeln!(f, " {}", self.body.op.content)?;
        self.body.block.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Def);
impl_locational!(Def, sig, body);

#[pymethods]
impl Def {
    #[staticmethod]
    pub const fn new(sig: Signature, body: DefBody) -> Self {
        Self { sig, body }
    }

    pub fn is_const(&self) -> bool {
        self.sig.is_const()
    }

    pub const fn is_subr(&self) -> bool {
        self.sig.is_subr()
    }

    pub const fn is_var(&self) -> bool {
        self.sig.is_var()
    }

    pub fn def_kind(&self) -> DefKind {
        self.body.def_kind()
    }
}

/// This is not necessary for Erg syntax, but necessary for mapping ASTs in Python
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReDef {
    pub attr: Accessor,
    pub expr: Box<Expr>,
}

impl NestedDisplay for ReDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        self.attr.fmt_nest(f, level)?;
        writeln!(f, " = ")?;
        self.expr.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(ReDef);
impl_locational!(ReDef, attr, expr);

#[pymethods]
impl ReDef {
    #[staticmethod]
    pub fn new(attr: Accessor, expr: Expr) -> Self {
        Self {
            attr,
            expr: Box::new(expr),
        }
    }
}

/// e.g.
/// ```python
/// T = Class ...
/// T.
///     x = 1
///     f(a) = ...
/// ```
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Methods {
    pub id: DefId,
    pub class: TypeSpec,
    pub class_as_expr: Box<Expr>,
    pub vis: VisModifierSpec, // `.` or `::`
    pub attrs: ClassAttrs,
}

impl NestedDisplay for Methods {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{}{}", self.class, self.vis)?;
        self.attrs.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Methods);
impl_locational!(Methods, class, attrs);

impl Methods {
    pub fn new(
        id: DefId,
        class: TypeSpec,
        class_as_expr: Expr,
        vis: VisModifierSpec,
        attrs: ClassAttrs,
    ) -> Self {
        Self {
            id,
            class,
            class_as_expr: Box::new(class_as_expr),
            vis,
            attrs,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassDef {
    pub def: Def,
    pub methods_list: Vec<Methods>,
}

impl NestedDisplay for ClassDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "(class)")?;
        self.def.fmt_nest(f, level)?;
        for methods in self.methods_list.iter() {
            write!(f, "(methods)")?;
            methods.fmt_nest(f, level + 1)?;
        }
        Ok(())
    }
}

impl_display_from_nested!(ClassDef);
impl_locational!(ClassDef, def);

#[pymethods]
impl ClassDef {
    #[staticmethod]
    pub const fn new(def: Def, methods: Vec<Methods>) -> Self {
        Self {
            def,
            methods_list: methods,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatchDef {
    pub def: Def,
    pub methods_list: Vec<Methods>,
}

impl NestedDisplay for PatchDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "(patch)")?;
        self.def.fmt_nest(f, level)?;
        for methods in self.methods_list.iter() {
            write!(f, "(methods)")?;
            methods.fmt_nest(f, level + 1)?;
        }
        Ok(())
    }
}

impl_display_from_nested!(PatchDef);
impl_locational!(PatchDef, def);

#[pymethods]
impl PatchDef {
    #[staticmethod]
    pub const fn new(def: Def, methods: Vec<Methods>) -> Self {
        Self {
            def,
            methods_list: methods,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Compound {
    pub exprs: Vec<Expr>,
}

impl_stream!(Compound, Expr, exprs);

impl NestedDisplay for Compound {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", fmt_vec(&self.exprs))
    }
}

impl Locational for Compound {
    fn loc(&self) -> Location {
        if let Some(expr) = self.exprs.first() {
            if let Some(last) = self.exprs.last() {
                Location::concat(expr, last)
            } else {
                expr.loc()
            }
        } else {
            Location::Unknown
        }
    }
}

#[pymethods]
impl Compound {
    #[staticmethod]
    pub const fn new(exprs: Vec<Expr>) -> Self {
        Self { exprs }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WithPrefix {
    pub token: Token,
    pub arg: Box<MacroArg>,
}

impl NestedDisplay for WithPrefix {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} {}", self.token, self.arg)
    }
}

impl_display_from_nested!(WithPrefix);
impl_locational!(WithPrefix, token, arg);

impl WithPrefix {
    pub fn new(token: Token, arg: MacroArg) -> Self {
        Self {
            token,
            arg: Box::new(arg),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MacroArg {
    Expr(Expr),
    Block(Block),
    WithPrefix(WithPrefix),
}

impl_nested_display_for_chunk_enum!(MacroArg; Expr, Block, WithPrefix);
impl_from_trait_for_enum!(MacroArg; Expr, Block, WithPrefix);
impl_display_from_nested!(MacroArg);
impl_locational_for_enum!(MacroArg; Expr, Block, WithPrefix);
impl_from_py_for_enum!(MacroArg; Expr, Block, WithPrefix);
impl_into_py_for_enum!(MacroArg; Expr, Block, WithPrefix);

impl MacroArg {
    pub fn with_prefix(token: Token, arg: MacroArg) -> Self {
        Self::WithPrefix(WithPrefix::new(token, arg))
    }

    pub fn get_expr(&self) -> Option<&Expr> {
        match self {
            Self::Expr(expr) => Some(expr),
            Self::WithPrefix(WithPrefix { arg, .. }) => arg.get_expr(),
            _ => None,
        }
    }

    pub fn get_block(&self) -> Option<&Block> {
        match self {
            Self::Block(block) => Some(block),
            Self::WithPrefix(WithPrefix { arg, .. }) => arg.get_block(),
            _ => None,
        }
    }

    pub fn get_prefix(&self) -> Option<&Token> {
        match self {
            Self::WithPrefix(WithPrefix { token, .. }) => Some(token),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[pyclass(get_all, set_all)]
pub struct MacroArgs {
    pub pos_args: Vec<MacroArg>,
    pub var_args: Vec<MacroArg>,
    pub kw_args: Vec<MacroArg>,
}

impl NestedDisplay for MacroArgs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", fmt_vec(&self.pos_args))?;
        if !self.var_args.is_empty() {
            write!(f, ", *{}", fmt_vec(&self.var_args))?;
        }
        if !self.kw_args.is_empty() {
            write!(f, ", {}", fmt_vec(&self.kw_args))?;
        }
        Ok(())
    }
}

impl_display_from_nested!(MacroArgs);

impl Locational for MacroArgs {
    fn loc(&self) -> Location {
        if let Some(last) = self.kw_args.last() {
            Location::concat(&self.pos_args, last)
        } else if let Some(last) = self.var_args.last() {
            Location::concat(&self.pos_args, last)
        } else if let Some(last) = self.pos_args.last() {
            last.loc()
        } else {
            Location::Unknown
        }
    }
}

impl MacroArgs {
    pub const fn new(
        pos_args: Vec<MacroArg>,
        var_args: Vec<MacroArg>,
        kw_args: Vec<MacroArg>,
    ) -> Self {
        Self {
            pos_args,
            var_args,
            kw_args,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[pyclass(get_all, set_all)]
pub struct MacroCall {
    pub name: VarName,
    pub args: MacroArgs,
}

impl NestedDisplay for MacroCall {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} {}", self.name, self.args)
    }
}

impl_display_from_nested!(MacroCall);

impl Locational for MacroCall {
    fn loc(&self) -> Location {
        if !self.args.loc().is_unknown() {
            Location::concat(&self.name, &self.args)
        } else {
            self.name.loc()
        }
    }
}

impl MacroCall {
    pub const fn new(name: VarName, args: MacroArgs) -> Self {
        Self { name, args }
    }
}

/// Quote(`'{...}`) and Splice(`${...}`) are used for macro expansion.
/// Quote makes stage 0 (compiletime) expression from stage 1 (runtime) expression.
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Quote {
    pub expr: Box<Expr>,
}

impl NestedDisplay for Quote {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "'{{")?;
        self.expr.fmt_nest(f, level)?;
        write!(f, "}}")
    }
}

impl_display_from_nested!(Quote);
impl_locational!(Quote, expr);

impl Quote {
    pub fn new(expr: Expr) -> Self {
        Self {
            expr: Box::new(expr),
        }
    }
}

/// Quote(`'{...}`) and Splice(`${...}`) are used for macro expansion.
/// Splice makes stage 1 (runtime) expression from stage 0 (compiletime) expression.
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Splice {
    pub expr: Box<Expr>,
}

impl NestedDisplay for Splice {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "${{")?;
        self.expr.fmt_nest(f, level)?;
        write!(f, "}}")
    }
}

impl_display_from_nested!(Splice);
impl_locational!(Splice, expr);

impl Splice {
    pub fn new(expr: Expr) -> Self {
        Self {
            expr: Box::new(expr),
        }
    }
}

/// Expression(式)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Literal(Literal),
    Accessor(Accessor),
    Array(Array),
    Tuple(Tuple),
    Dict(Dict),
    Set(Set),
    Record(Record),
    BinOp(BinOp),
    UnaryOp(UnaryOp),
    Call(Call),
    DataPack(DataPack),
    Lambda(Lambda),
    TypeAscription(TypeAscription),
    Def(Def),
    Methods(Methods),
    ClassDef(ClassDef),
    PatchDef(PatchDef),
    ReDef(ReDef),
    Compound(Compound),
    InlineModule(InlineModule),
    MacroCall(MacroCall),
    Quote(Quote),
    Splice(Splice),
    /// for mapping to Python AST
    Dummy(Dummy),
}

impl_nested_display_for_chunk_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAscription, Def, Methods, ClassDef, PatchDef, ReDef, Compound, InlineModule, MacroCall, Quote, Splice, Dummy);
impl_from_trait_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAscription, Def, Methods, ClassDef, PatchDef, ReDef, Compound, InlineModule, MacroCall, Quote, Splice, Dummy);
impl_display_from_nested!(Expr);
impl_locational_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAscription, Def, Methods, ClassDef, PatchDef, ReDef, Compound, InlineModule, MacroCall, Quote, Splice, Dummy);
impl_into_py_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAscription, Def, Methods, ClassDef, PatchDef, ReDef, Compound, InlineModule, MacroCall, Quote, Splice, Dummy);
impl_from_py_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAscription, Def, Methods, ClassDef, PatchDef, ReDef, Compound, InlineModule, MacroCall, Quote, Splice, Dummy);

impl Expr {
    pub fn is_match_call(&self) -> bool {
        matches!(self, Expr::Call(call) if call.is_match())
    }

    pub fn is_const_acc(&self) -> bool {
        matches!(self, Expr::Accessor(acc) if acc.is_const())
    }

    pub const fn is_definition(&self) -> bool {
        matches!(
            self,
            Expr::Def(_) | Expr::ClassDef(_) | Expr::PatchDef(_) | Expr::Methods(_)
        )
    }

    pub const fn is_macro_call(&self) -> bool {
        matches!(self, Expr::MacroCall(_))
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::Literal(_) => "literal",
            Self::Accessor(_) => "accessor",
            Self::Array(_) => "array",
            Self::Tuple(_) => "tuple",
            Self::Dict(_) => "dict",
            Self::Set(_) => "set",
            Self::Record(_) => "record",
            Self::BinOp(_) => "binary operator call",
            Self::UnaryOp(_) => "unary operator call",
            Self::Call(_) => "call",
            Self::DataPack(_) => "data pack",
            Self::Lambda(_) => "lambda",
            Self::TypeAscription(_) => "type ascription",
            Self::Def(_) => "definition",
            Self::Methods(_) => "methods",
            Self::ClassDef(_) => "class definition",
            Self::PatchDef(_) => "patch definition",
            Self::ReDef(_) => "re-definition",
            Self::Compound(_) => "compound",
            Self::InlineModule(_) => "inline module",
            Self::MacroCall(_) => "macro call",
            Self::Quote(_) => "quote",
            Self::Splice(_) => "unquote",
            Self::Dummy(_) => "dummy",
        }
    }

    pub fn need_to_be_closed(&self) -> bool {
        match self {
            Self::BinOp(_) | Self::UnaryOp(_) | Self::Lambda(_) | Self::TypeAscription(_) => true,
            Self::Tuple(tup) => tup.paren().is_none(),
            Self::Call(call) if ERG_MODE => call.args.paren.is_none(),
            _ => false,
        }
    }

    pub fn get_name(&self) -> Option<&Str> {
        match self {
            Expr::Accessor(acc) => acc.name(),
            _ => None,
        }
    }

    pub fn local(name: &str, lineno: u32, col_begin: u32, col_end: u32) -> Self {
        Self::Accessor(Accessor::local(Token::new_fake(
            TokenKind::Symbol,
            Str::rc(name),
            lineno,
            col_begin,
            col_end,
        )))
    }

    pub fn dummy_local(name: &str) -> Self {
        Self::Accessor(Accessor::local(Token::from_str(TokenKind::Symbol, name)))
    }

    pub fn static_local(name: &'static str) -> Self {
        Self::Accessor(Accessor::local(Token::static_symbol(name)))
    }

    pub fn attr(self, ident: Identifier) -> Accessor {
        Accessor::attr(self, ident)
    }

    pub fn attr_expr(self, ident: Identifier) -> Self {
        Self::Accessor(self.attr(ident))
    }

    pub fn subscr(self, index: Expr, r_sqbr: Token) -> Accessor {
        Accessor::subscr(self, index, r_sqbr)
    }

    pub fn subscr_expr(self, index: Expr, r_sqbr: Token) -> Self {
        Self::Accessor(self.subscr(index, r_sqbr))
    }

    pub fn tuple_attr(self, index: Literal) -> Accessor {
        Accessor::tuple_attr(self, index)
    }

    pub fn tuple_attr_expr(self, index: Literal) -> Self {
        Self::Accessor(self.tuple_attr(index))
    }

    pub fn type_app(self, type_args: TypeAppArgs) -> Accessor {
        Accessor::type_app(self, type_args)
    }

    pub fn call(self, args: Args) -> Call {
        match self {
            Self::Accessor(Accessor::Attr(attr)) => Call::new(*attr.obj, Some(attr.ident), args),
            other => Call::new(other, None, args),
        }
    }

    pub fn call_expr(self, args: Args) -> Self {
        Self::Call(self.call(args))
    }

    pub fn call1(self, expr: Expr) -> Self {
        self.call_expr(Args::pos_only(vec![PosArg::new(expr)], None))
    }

    pub fn call2(self, expr1: Expr, expr2: Expr) -> Self {
        self.call_expr(Args::pos_only(
            vec![PosArg::new(expr1), PosArg::new(expr2)],
            None,
        ))
    }

    pub fn method_call_expr(self, attr_name: Option<Identifier>, args: Args) -> Self {
        Self::Call(Call::new(self, attr_name, args))
    }

    pub fn type_asc(self, t_spec: TypeSpecWithOp) -> TypeAscription {
        TypeAscription::new(self, t_spec)
    }

    pub fn type_asc_expr(self, t_spec: TypeSpecWithOp) -> Self {
        Self::TypeAscription(self.type_asc(t_spec))
    }

    pub fn bin_op(self, op: Token, rhs: Expr) -> BinOp {
        BinOp::new(op, self, rhs)
    }

    pub fn unary_op(self, op: Token) -> UnaryOp {
        UnaryOp::new(op, self)
    }

    /// Return the complexity of the expression in terms of type inference.
    /// For function calls, type inference is performed sequentially, starting with the least complex argument.
    pub fn complexity(&self) -> usize {
        match self {
            Self::Literal(_) | Self::TypeAscription(_) => 0,
            Self::Accessor(Accessor::Ident(_)) => 1,
            Self::Accessor(Accessor::Attr(attr)) => 1 + attr.obj.complexity(),
            Self::Tuple(Tuple::Normal(tup)) => {
                let mut sum = 0;
                for elem in tup.elems.pos_args.iter() {
                    sum += elem.expr.complexity();
                }
                sum
            }
            Self::Array(Array::Normal(arr)) => {
                let mut sum = 0;
                for elem in arr.elems.pos_args.iter() {
                    sum += elem.expr.complexity();
                }
                sum
            }
            Self::Dict(Dict::Normal(dic)) => {
                let mut sum = 0;
                for kv in dic.kvs.iter() {
                    sum += kv.key.complexity();
                    sum += kv.value.complexity();
                }
                sum
            }
            Self::Set(Set::Normal(set)) => {
                let mut sum = 0;
                for elem in set.elems.pos_args.iter() {
                    sum += elem.expr.complexity();
                }
                sum
            }
            Self::Record(Record::Normal(rec)) => {
                let mut sum = 0;
                for attr in rec.attrs.iter() {
                    for chunk in attr.body.block.iter() {
                        sum += chunk.complexity();
                    }
                }
                sum
            }
            Self::BinOp(bin) => 1 + bin.args[0].complexity() + bin.args[1].complexity(),
            Self::UnaryOp(unary) => 1 + unary.args[0].complexity(),
            Self::Call(call) => {
                let mut sum = 1 + call.obj.complexity();
                for arg in call.args.pos_args.iter() {
                    sum += arg.expr.complexity();
                }
                if let Some(var_params) = call.args.var_args.as_ref() {
                    sum += var_params.expr.complexity();
                }
                for kw_arg in call.args.kw_args.iter() {
                    sum += kw_arg.expr.complexity();
                }
                sum
            }
            Self::Lambda(lambda) => {
                let mut sum = 1
                    + lambda.sig.return_t_spec.is_none() as usize
                    + lambda
                        .sig
                        .params
                        .sigs()
                        .fold(0, |acc, sig| acc + sig.t_spec.is_none() as usize);
                for chunk in lambda.body.iter() {
                    sum += chunk.complexity();
                }
                sum
            }
            _ => 5,
        }
    }

    pub fn to_str_literal(&self) -> Literal {
        match self {
            Self::Accessor(acc) => acc.to_str_literal(),
            other => todo!("{other}"),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Module(Block);

impl NestedDisplay for Module {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)
    }
}

impl_display_from_nested!(Module);

impl Locational for Module {
    fn loc(&self) -> Location {
        if self.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
        }
    }
}

impl Stream<Expr> for Module {
    fn payload(self) -> Vec<Expr> {
        self.0.payload()
    }
    fn ref_payload(&self) -> &Vec<Expr> {
        self.0.ref_payload()
    }
    fn ref_mut_payload(&mut self) -> &mut Vec<Expr> {
        self.0.ref_mut_payload()
    }
}

impl IntoIterator for Module {
    type Item = Expr;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<Expr> for Module {
    fn from_iter<T: IntoIterator<Item = Expr>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl_py_iter!(Module<Expr>, ModuleIter, 0);

impl Module {
    pub const fn empty() -> Self {
        Self(Block::empty())
    }
    pub const fn new(payload: Vec<Expr>) -> Self {
        Self(Block::new(payload))
    }
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Block::with_capacity(capacity))
    }

    pub fn block(&self) -> &Block {
        &self.0
    }

    pub fn get_attr(&self, name: &str) -> Option<&Def> {
        self.0.iter().find_map(|e| match e {
            Expr::Def(def) if def.sig.ident().is_some_and(|id| id.inspect() == name) => Some(def),
            _ => None,
        })
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AST {
    pub name: Str,
    pub module: Module,
}

impl NestedDisplay for AST {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        self.module.fmt_nest(f, level)
    }
}

impl_display_from_nested!(AST);
impl_locational!(AST, module);

#[pymethods]
impl AST {
    #[staticmethod]
    pub const fn new(name: Str, module: Module) -> Self {
        Self { name, module }
    }

    pub fn is_empty(&self) -> bool {
        self.module.is_empty()
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InlineModule {
    pub input: Input,
    pub ast: AST,
    pub import: Call,
}

impl NestedDisplay for InlineModule {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "inline-module({})", self.import)?;
        self.ast.fmt_nest(f, level)
    }
}

impl_display_from_nested!(InlineModule);
impl_locational!(InlineModule, ast);

#[pymethods]
impl InlineModule {
    #[getter]
    pub fn ast(&self) -> AST {
        self.ast.clone()
    }
}

impl InlineModule {
    pub const fn new(input: Input, ast: AST, import: Call) -> Self {
        Self { input, ast, import }
    }
}
