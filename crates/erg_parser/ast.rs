//! defines `Expr` (Expression, the minimum executing unit of Erg).
use std::borrow::Borrow;
use std::fmt;
use std::fmt::Write as _;

use erg_common::error::Location;
use erg_common::set::Set as HashSet;
// use erg_common::dict::Dict as HashMap;
use erg_common::traits::{Locational, NestedDisplay, Stream};
use erg_common::{
    fmt_option, fmt_vec, impl_display_for_enum, impl_display_for_single_struct,
    impl_display_from_nested, impl_displayable_stream_for_wrapper, impl_from_trait_for_enum,
    impl_locational, impl_locational_for_enum, impl_nested_display_for_chunk_enum,
    impl_nested_display_for_enum, impl_stream, option_enum_unwrap,
};
use erg_common::{fmt_vec_split_with, Str};

use crate::token::{Token, TokenKind, EQUAL};

/// Some Erg functions require additional operation by the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Import,
    PyImport,
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

impl OperationKind {
    pub const fn is_erg_import(&self) -> bool {
        matches!(self, Self::Import)
    }
    pub const fn is_py_import(&self) -> bool {
        matches!(self, Self::PyImport)
    }
    pub const fn is_import(&self) -> bool {
        matches!(self, Self::Import | Self::PyImport)
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
    pub const fn new(token: Token) -> Self {
        Self { token }
    }

    pub fn nat(n: usize, line: u32) -> Self {
        let token = Token::new(TokenKind::NatLit, Str::from(n.to_string()), line, 0);
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

impl KwArg {
    pub const fn new(keyword: Token, t_spec: Option<TypeSpecWithOp>, expr: Expr) -> Self {
        Self {
            keyword,
            t_spec,
            expr,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Args {
    pos_args: Vec<PosArg>,
    pub(crate) var_args: Option<Box<PosArg>>,
    kw_args: Vec<KwArg>,
    // these are for ELS
    pub paren: Option<(Token, Token)>,
}

impl NestedDisplay for Args {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        fmt_lines(self.pos_args.iter(), f, level)?;
        writeln!(f)?;
        fmt_lines(self.kw_args.iter(), f, level)
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
        match (self.pos_args.first(), self.kw_args.last()) {
            (Some(l), Some(r)) => Location::concat(l, r),
            (Some(l), None) => Location::concat(l, self.pos_args.last().unwrap()),
            (None, Some(r)) => Location::concat(self.kw_args.first().unwrap(), r),
            _ => Location::Unknown,
        }
    }
}

// impl_stream!(Args, Arg, args);

impl Args {
    pub fn new(
        pos_args: Vec<PosArg>,
        var_args: Option<PosArg>,
        kw_args: Vec<KwArg>,
        paren: Option<(Token, Token)>,
    ) -> Self {
        Self {
            pos_args,
            var_args: var_args.map(Box::new),
            kw_args,
            paren,
        }
    }

    pub fn pos_only(pos_arg: Vec<PosArg>, paren: Option<(Token, Token)>) -> Self {
        Self::new(pos_arg, None, vec![], paren)
    }

    pub fn single(pos_args: PosArg) -> Self {
        Self::pos_only(vec![pos_args], None)
    }

    pub fn empty() -> Self {
        Self::new(vec![], None, vec![], None)
    }

    // for replacing to hir::Args
    #[allow(clippy::type_complexity)]
    pub fn deconstruct(
        self,
    ) -> (
        Vec<PosArg>,
        Option<PosArg>,
        Vec<KwArg>,
        Option<(Token, Token)>,
    ) {
        (
            self.pos_args,
            self.var_args.map(|x| *x),
            self.kw_args,
            self.paren,
        )
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

    pub fn pos_args(&self) -> &[PosArg] {
        &self.pos_args[..]
    }

    pub fn kw_args(&self) -> &[KwArg] {
        &self.kw_args[..]
    }

    pub fn has_pos_arg(&self, pa: &PosArg) -> bool {
        self.pos_args.contains(pa)
    }

    pub fn into_iters(
        self,
    ) -> (
        impl IntoIterator<Item = PosArg>,
        impl IntoIterator<Item = KwArg>,
    ) {
        (self.pos_args.into_iter(), self.kw_args.into_iter())
    }

    pub fn push_pos(&mut self, arg: PosArg) {
        self.pos_args.push(arg);
    }

    pub fn extend_pos<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = PosArg>,
    {
        self.pos_args.extend(iter);
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

    pub fn set_parens(&mut self, paren: (Token, Token)) {
        self.paren = Some(paren);
    }

    pub fn get_left_or_key(&self, key: &str) -> Option<&Expr> {
        if !self.pos_args.is_empty() {
            self.pos_args.get(0).map(|a| &a.expr)
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

impl Attribute {
    pub fn new(obj: Expr, ident: Identifier) -> Self {
        Self {
            obj: Box::new(obj),
            ident,
        }
    }
}

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

impl TupleAttribute {
    pub fn new(obj: Expr, index: Literal) -> Self {
        Self {
            obj: Box::new(obj),
            index,
        }
    }
}

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

impl Subscript {
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

impl TypeAppArgs {
    pub const fn new(l_vbar: Token, args: TypeAppArgsKind, r_vbar: Token) -> Self {
        Self {
            l_vbar,
            args,
            r_vbar,
        }
    }
}

/// f|T := Int|
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

impl TypeApp {
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
}

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

impl NormalArray {
    pub const fn new(l_sqbr: Token, r_sqbr: Token, elems: Args) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elems,
        }
    }
}

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

impl ArrayWithLength {
    pub fn new(l_sqbr: Token, r_sqbr: Token, elem: PosArg, len: Expr) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elem: Box::new(elem),
            len: Box::new(len),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayComprehension {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub elem: Box<Expr>,
    pub generators: Vec<(Identifier, Expr)>,
    pub guards: Vec<Expr>,
}

impl NestedDisplay for ArrayComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        let mut generators = String::new();
        for (name, gen) in self.generators.iter() {
            write!(generators, "{name} <- {gen}, ")?;
        }
        write!(
            f,
            "[{}| {}{}]",
            self.elem,
            generators,
            fmt_vec(&self.guards)
        )
    }
}

impl_display_from_nested!(ArrayComprehension);
impl_locational!(ArrayComprehension, l_sqbr, elem, r_sqbr);

impl ArrayComprehension {
    pub fn new(
        l_sqbr: Token,
        r_sqbr: Token,
        elem: Expr,
        generators: Vec<(Identifier, Expr)>,
        guards: Vec<Expr>,
    ) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elem: Box::new(elem),
            generators,
            guards,
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

impl NormalTuple {
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

impl Tuple {
    pub fn paren(&self) -> Option<&(Token, Token)> {
        match self {
            Self::Normal(tuple) => tuple.elems.paren.as_ref(),
        }
    }
}

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

impl KeyValue {
    pub const fn new(key: Expr, value: Expr) -> Self {
        Self { key, value }
    }
}

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

impl NormalDict {
    pub const fn new(l_brace: Token, r_brace: Token, kvs: Vec<KeyValue>) -> Self {
        Self {
            l_brace,
            r_brace,
            kvs,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DictComprehension {
    l_brace: Token,
    r_brace: Token,
    pub attrs: Args,
    guards: Vec<Expr>,
}

// TODO:
impl NestedDisplay for DictComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{} | {}}}", self.attrs, fmt_vec(&self.guards))
    }
}

impl_display_from_nested!(DictComprehension);
impl_locational!(DictComprehension, l_brace, attrs, r_brace);

impl DictComprehension {
    pub const fn new(l_brace: Token, r_brace: Token, attrs: Args, guards: Vec<Expr>) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
            guards,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClassAttr {
    Def(Def),
    Decl(TypeAscription),
    Doc(Literal),
}

impl_nested_display_for_enum!(ClassAttr; Def, Decl, Doc);
impl_display_for_enum!(ClassAttr; Def, Decl, Doc);
impl_locational_for_enum!(ClassAttr; Def, Decl, Doc);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassAttrs(Vec<ClassAttr>);

impl NestedDisplay for ClassAttrs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)?;
        writeln!(f)
    }
}

impl Locational for ClassAttrs {
    fn loc(&self) -> Location {
        Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
    }
}

impl From<Vec<ClassAttr>> for ClassAttrs {
    fn from(attrs: Vec<ClassAttr>) -> Self {
        Self(attrs)
    }
}

impl ClassAttrs {
    pub const fn new(attrs: Vec<ClassAttr>) -> Self {
        Self(attrs)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ClassAttr> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ClassAttr> {
        self.0.iter_mut()
    }
}

impl IntoIterator for ClassAttrs {
    type Item = ClassAttr;
    type IntoIter = <Vec<ClassAttr> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordAttrs(Vec<Def>);

impl_stream!(RecordAttrs, Def);

impl NestedDisplay for RecordAttrs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)?;
        writeln!(f)
    }
}

impl Locational for RecordAttrs {
    fn loc(&self) -> Location {
        Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
    }
}

impl From<Vec<Def>> for RecordAttrs {
    fn from(attrs: Vec<Def>) -> Self {
        Self(attrs)
    }
}

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

impl NormalRecord {
    pub const fn new(l_brace: Token, r_brace: Token, attrs: RecordAttrs) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
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
}

/// Record can be defined with shorthend/normal mixed style, i.e. {x; y=expr; z; ...}
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

impl MixedRecord {
    pub const fn new(l_brace: Token, r_brace: Token, attrs: Vec<RecordAttrOrIdent>) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
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

impl NormalSet {
    pub const fn new(l_brace: Token, r_brace: Token, elems: Args) -> Self {
        Self {
            l_brace,
            r_brace,
            elems,
        }
    }
}

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

impl SetWithLength {
    pub fn new(l_brace: Token, r_brace: Token, elem: PosArg, len: Expr) -> Self {
        Self {
            l_brace,
            r_brace,
            elem: Box::new(elem),
            len: Box::new(len),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetComprehension {
    pub l_brace: Token,
    pub r_brace: Token,
    pub var: Token,
    pub op: Token, // <- or :
    pub iter: Box<Expr>,
    pub pred: Box<Expr>,
}

impl NestedDisplay for SetComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(
            f,
            "{{{} {} {} | {}}}",
            self.var, self.op, self.iter, self.pred
        )
    }
}

impl_display_from_nested!(SetComprehension);
impl_locational!(SetComprehension, l_brace, r_brace);

impl SetComprehension {
    pub fn new(
        l_brace: Token,
        r_brace: Token,
        var: Token,
        op: Token,
        iter: Expr,
        pred: Expr,
    ) -> Self {
        Self {
            l_brace,
            r_brace,
            var,
            op,
            iter: Box::new(iter),
            pred: Box::new(pred),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BinOp {
    pub op: Token,
    pub args: [Box<Expr>; 2],
}

impl NestedDisplay for BinOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "`{}`:", self.op.content)?;
        self.args[0].fmt_nest(f, level + 1)?;
        writeln!(f)?;
        self.args[1].fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(BinOp);

impl Locational for BinOp {
    fn loc(&self) -> Location {
        Location::concat(&self.op, self.args[1].as_ref())
    }
}

impl BinOp {
    pub fn new(op: Token, lhs: Expr, rhs: Expr) -> Self {
        Self {
            op,
            args: [Box::new(lhs), Box::new(rhs)],
        }
    }

    pub fn deconstruct(self) -> (Token, Expr, Expr) {
        let mut exprs = self.args.into_iter();
        (self.op, *exprs.next().unwrap(), *exprs.next().unwrap())
    }
}

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
    pub fn new(op: Token, expr: Expr) -> Self {
        Self {
            op,
            args: [Box::new(expr)],
        }
    }

    pub fn deconstruct(self) -> (Token, Expr) {
        let mut exprs = self.args.into_iter();
        (self.op, *exprs.next().unwrap())
    }
}

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
        } else {
            writeln!(f, ":")?;
            self.args.fmt_nest(f, level + 1)
        }
    }
}

impl_display_from_nested!(Call);

impl Locational for Call {
    fn loc(&self) -> Location {
        Location::concat(self.obj.as_ref(), &self.args)
    }
}

impl Call {
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

    pub fn is_assert_cast(&self) -> bool {
        self.obj
            .get_name()
            .map(|s| &s[..] == "assert")
            .unwrap_or(false)
            && self
                .args
                .get_left_or_key("pred")
                .map(|pred| pred.is_bin_in())
                .unwrap_or(false)
    }

    pub fn assert_cast_target_type(&self) -> Option<&Expr> {
        self.args
            .get_left_or_key("pred")
            .and_then(|pred| option_enum_unwrap!(pred, Expr::BinOp))
            .map(|bin| bin.args[1].as_ref())
    }

    pub fn additional_operation(&self) -> Option<OperationKind> {
        self.obj.get_name().and_then(|s| match &s[..] {
            "import" => Some(OperationKind::Import),
            "pyimport" | "py" | "__import__" => Some(OperationKind::PyImport),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block(Vec<Expr>);

impl NestedDisplay for Block {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)
    }
}

impl_display_from_nested!(Block);

impl Locational for Block {
    fn loc(&self) -> Location {
        if self.0.is_empty() {
            Location::Unknown
        } else {
            Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
        }
    }
}

impl_stream!(Block, Expr);

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
        &self.exprs.0
    }
    fn ref_mut_payload(&mut self) -> &mut Vec<Expr> {
        &mut self.exprs.0
    }
    fn payload(self) -> Vec<Expr> {
        self.exprs.0
    }
}

impl Dummy {
    pub const fn new(loc: Option<Location>, exprs: Vec<Expr>) -> Self {
        Self {
            loc,
            exprs: Block(exprs),
        }
    }
}

pub type ConstIdentifier = Identifier;

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

impl ConstAttribute {
    pub fn new(expr: ConstExpr, name: ConstIdentifier) -> Self {
        Self {
            obj: Box::new(expr),
            name,
        }
    }

    pub fn downgrade(self) -> Attribute {
        Attribute::new(self.obj.downgrade(), self.name)
    }
}

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
    pub fn new(tup: ConstExpr, index: Literal) -> Self {
        Self {
            tup: Box::new(tup),
            index,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstSubscript {
    obj: Box<ConstExpr>,
    index: Box<ConstExpr>,
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
impl_locational!(ConstSubscript, obj, index);

impl ConstSubscript {
    pub fn new(obj: ConstExpr, index: ConstExpr) -> Self {
        Self {
            obj: Box::new(obj),
            index: Box::new(index),
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

    pub fn subscr(obj: ConstExpr, index: ConstExpr) -> Self {
        Self::Subscr(ConstSubscript::new(obj, index))
    }

    pub fn downgrade(self) -> Accessor {
        match self {
            Self::Local(local) => Accessor::Ident(local),
            Self::Attr(attr) => Accessor::Attr(attr.downgrade()),
            // Self::TupleAttr(attr) => Accessor::TupleAttr(attr.downgrade()),
            // Self::Subscr(subscr) => Accessor::Subscr(subscr.downgrade()),
            _ => todo!(),
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

impl ConstArray {
    pub fn downgrade(self) -> Array {
        match self {
            Self::Normal(normal) => Array::Normal(normal.downgrade()),
            Self::WithLength(with_length) => Array::WithLength(with_length.downgrade()),
        }
    }
}

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

impl ConstNormalArray {
    pub fn new(l_sqbr: Token, r_sqbr: Token, elems: ConstArgs, guard: Option<ConstExpr>) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elems,
            guard: guard.map(Box::new),
        }
    }

    pub fn downgrade(self) -> NormalArray {
        NormalArray::new(self.l_sqbr, self.r_sqbr, self.elems.downgrade())
    }
}

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

impl ConstArrayWithLength {
    pub fn new(l_sqbr: Token, r_sqbr: Token, elem: ConstExpr, length: ConstExpr) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elem: Box::new(elem),
            length: Box::new(length),
        }
    }

    pub fn downgrade(self) -> ArrayWithLength {
        ArrayWithLength::new(
            self.l_sqbr,
            self.r_sqbr,
            PosArg::new(self.elem.downgrade()),
            self.length.downgrade(),
        )
    }
}

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

impl ConstNormalSet {
    pub fn new(l_brace: Token, r_brace: Token, elems: ConstArgs) -> Self {
        Self {
            l_brace,
            r_brace,
            elems,
        }
    }

    pub fn downgrade(self) -> NormalSet {
        NormalSet::new(self.l_brace, self.r_brace, self.elems.downgrade())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstSetComprehension {
    pub l_brace: Token,
    pub r_brace: Token,
    pub var: Token,
    pub op: Token,
    pub iter: Box<ConstExpr>,
    pub pred: Box<ConstExpr>,
}

impl NestedDisplay for ConstSetComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(
            f,
            "{{{} {} {} | {}}}",
            self.var, self.op, self.iter, self.pred
        )
    }
}

impl_display_from_nested!(ConstSetComprehension);
impl_locational!(ConstSetComprehension, l_brace, var, r_brace);

impl ConstSetComprehension {
    pub fn new(
        l_brace: Token,
        r_brace: Token,
        var: Token,
        op: Token,
        iter: ConstExpr,
        pred: ConstExpr,
    ) -> Self {
        Self {
            l_brace,
            r_brace,
            var,
            op,
            iter: Box::new(iter),
            pred: Box::new(pred),
        }
    }

    pub fn downgrade(self) -> SetComprehension {
        SetComprehension::new(
            self.l_brace,
            self.r_brace,
            self.var,
            self.op,
            self.iter.downgrade(),
            self.pred.downgrade(),
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

impl ConstSet {
    pub fn downgrade(self) -> Set {
        match self {
            Self::Normal(normal) => Set::Normal(normal.downgrade()),
            Self::Comprehension(comp) => Set::Comprehension(comp.downgrade()),
        }
    }
}

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

impl ConstKeyValue {
    pub const fn new(key: ConstExpr, value: ConstExpr) -> Self {
        Self { key, value }
    }

    pub fn downgrade(self) -> KeyValue {
        KeyValue::new(self.key.downgrade(), self.value.downgrade())
    }
}

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

impl ConstDict {
    pub const fn new(l_brace: Token, r_brace: Token, kvs: Vec<ConstKeyValue>) -> Self {
        Self {
            l_brace,
            r_brace,
            kvs,
        }
    }

    pub fn downgrade(self) -> Dict {
        Dict::Normal(NormalDict::new(
            self.l_brace,
            self.r_brace,
            self.kvs.into_iter().map(|kv| kv.downgrade()).collect(),
        ))
    }
}

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

impl ConstTuple {
    pub const fn new(elems: ConstArgs) -> Self {
        Self { elems }
    }

    pub fn downgrade(self) -> Tuple {
        Tuple::Normal(NormalTuple::new(self.elems.downgrade()))
    }
}

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

impl ConstBlock {
    pub fn downgrade(self) -> Block {
        Block::new(self.0.into_iter().map(|e| e.downgrade()).collect())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstDefBody {
    pub op: Token,
    pub block: ConstBlock,
    pub id: DefId,
}

impl_locational!(ConstDefBody, lossy op, block);

impl ConstDefBody {
    pub const fn new(op: Token, block: ConstBlock, id: DefId) -> Self {
        Self { op, block, id }
    }

    pub fn downgrade(self) -> DefBody {
        DefBody::new(self.op, self.block.downgrade(), self.id)
    }
}

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

impl ConstDef {
    pub const fn new(ident: ConstIdentifier, body: ConstDefBody) -> Self {
        Self { ident, body }
    }

    pub fn downgrade(self) -> Def {
        Def::new(Signature::new_var(self.ident), self.body.downgrade())
    }
}

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

impl ConstLambda {
    pub fn new(sig: LambdaSignature, op: Token, body: ConstBlock, id: DefId) -> Self {
        Self {
            sig: Box::new(sig),
            op,
            body,
            id,
        }
    }

    pub fn downgrade(self) -> Lambda {
        Lambda::new(*self.sig, self.op, self.body.downgrade(), self.id)
    }
}

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

impl ConstRecord {
    pub const fn new(l_brace: Token, r_brace: Token, attrs: Vec<ConstDef>) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
    }

    pub fn downgrade(self) -> Record {
        Record::Normal(NormalRecord::new(
            self.l_brace,
            self.r_brace,
            self.attrs.into_iter().map(|d| d.downgrade()).collect(),
        ))
    }
}

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

impl ConstBinOp {
    pub fn new(op: Token, lhs: ConstExpr, rhs: ConstExpr) -> Self {
        Self {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn downgrade(self) -> BinOp {
        BinOp::new(self.op, self.lhs.downgrade(), self.rhs.downgrade())
    }
}

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

impl ConstUnaryOp {
    pub fn new(op: Token, expr: ConstExpr) -> Self {
        Self {
            op,
            expr: Box::new(expr),
        }
    }

    pub fn downgrade(self) -> UnaryOp {
        UnaryOp::new(self.op, self.expr.downgrade())
    }
}

/// Application
/// ex. `Vec Int` of `Option Vec Int`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstApp {
    pub acc: ConstAccessor,
    pub args: ConstArgs,
}

impl NestedDisplay for ConstApp {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        writeln!(f, "{}:", self.acc)?;
        self.args.fmt_nest(f, level + 1)
    }
}

impl Locational for ConstApp {
    fn loc(&self) -> Location {
        if self.args.is_empty() {
            self.acc.loc()
        } else {
            Location::concat(&self.acc, &self.args)
        }
    }
}

impl ConstApp {
    pub const fn new(acc: ConstAccessor, args: ConstArgs) -> Self {
        Self { acc, args }
    }

    pub fn downgrade(self) -> Call {
        Expr::Accessor(self.acc.downgrade()).call(self.args.downgrade())
    }
}

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

impl ConstTypeAsc {
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

impl ConstExpr {
    pub fn need_to_be_closed(&self) -> bool {
        matches!(self, Self::BinOp(_) | Self::UnaryOp(_))
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
}

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

impl ConstPosArg {
    pub const fn new(expr: ConstExpr) -> Self {
        Self { expr }
    }
}

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

impl_locational!(ConstKwArg, keyword, expr);

impl ConstKwArg {
    pub const fn new(keyword: Token, expr: ConstExpr) -> Self {
        Self { keyword, expr }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstArgs {
    pos_args: Vec<ConstPosArg>,
    var_args: Option<Box<ConstPosArg>>,
    kw_args: Vec<ConstKwArg>,
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

impl ConstArgs {
    pub fn new(
        pos_args: Vec<ConstPosArg>,
        var_args: Option<ConstPosArg>,
        kw_args: Vec<ConstKwArg>,
        paren: Option<(Token, Token)>,
    ) -> Self {
        Self {
            pos_args,
            var_args: var_args.map(Box::new),
            kw_args,
            paren,
        }
    }

    pub fn pos_only(pos_args: Vec<ConstPosArg>, paren: Option<(Token, Token)>) -> Self {
        Self::new(pos_args, None, vec![], paren)
    }

    #[allow(clippy::type_complexity)]
    pub fn deconstruct(
        self,
    ) -> (
        Vec<ConstPosArg>,
        Option<ConstPosArg>,
        Vec<ConstKwArg>,
        Option<(Token, Token)>,
    ) {
        (
            self.pos_args,
            self.var_args.map(|x| *x),
            self.kw_args,
            self.paren,
        )
    }

    pub fn empty() -> Self {
        Self::new(vec![], None, vec![], None)
    }

    pub fn is_empty(&self) -> bool {
        self.pos_args.is_empty() && self.kw_args.is_empty()
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

    pub fn push_pos(&mut self, arg: ConstPosArg) {
        self.pos_args.push(arg);
    }

    pub fn push_kw(&mut self, arg: ConstKwArg) {
        self.kw_args.push(arg);
    }

    pub fn downgrade(self) -> Args {
        let (pos_args, var_args, kw_args, paren) = self.deconstruct();
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
            paren,
        )
    }
}

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

impl PolyTypeSpec {
    pub const fn new(acc: ConstAccessor, args: ConstArgs) -> Self {
        Self { acc, args }
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
}

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubrTypeSpec {
    pub bounds: TypeBoundSpecs,
    pub lparen: Option<Token>,
    pub non_defaults: Vec<ParamTySpec>,
    pub var_params: Option<Box<ParamTySpec>>,
    pub defaults: Vec<DefaultParamTySpec>,
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
            "({}, {}, {}) {} {}",
            fmt_vec(&self.non_defaults),
            fmt_option!(pre "*", &self.var_params),
            fmt_vec(&self.defaults),
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
    pub fn new(
        bounds: TypeBoundSpecs,
        lparen: Option<Token>,
        non_defaults: Vec<ParamTySpec>,
        var_params: Option<ParamTySpec>,
        defaults: Vec<DefaultParamTySpec>,
        arrow: Token,
        return_t: TypeSpec,
    ) -> Self {
        Self {
            bounds,
            lparen,
            non_defaults,
            var_params: var_params.map(Box::new),
            defaults,
            arrow,
            return_t: Box::new(return_t),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayTypeSpec {
    pub ty: Box<TypeSpec>,
    pub len: ConstExpr,
}

impl fmt::Display for ArrayTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}; {}]", self.ty, self.len)
    }
}

impl_locational!(ArrayTypeSpec, ty, len);

impl ArrayTypeSpec {
    pub fn new(ty: TypeSpec, len: ConstExpr) -> Self {
        Self {
            ty: Box::new(ty),
            len,
        }
    }
}

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
        } else {
            Location::concat(self.tys.first().unwrap(), self.tys.last().unwrap())
        }
    }
}

impl TupleTypeSpec {
    pub const fn new(parens: Option<(Token, Token)>, tys: Vec<TypeSpec>) -> Self {
        Self { parens, tys }
    }
}

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
    Dict(Vec<(TypeSpec, TypeSpec)>),
    Record(Vec<(Identifier, TypeSpec)>),
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
            Self::Dict(dict) => {
                write!(f, "{{")?;
                for (k, v) in dict.iter() {
                    write!(f, "{k}: {v}, ")?;
                }
                write!(f, "}}")
            }
            Self::Record(rec) => {
                write!(f, "{{")?;
                for (k, v) in rec.iter() {
                    write!(f, "{k} = {v}; ")?;
                }
                write!(f, "}}")
            }
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
            Self::Dict(dict) => Location::concat(&dict.first().unwrap().0, &dict.last().unwrap().1),
            Self::Record(rec) => Location::concat(&rec.first().unwrap().0, &rec.last().unwrap().1),
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
        ))
    }

    pub fn mono(ident: Identifier) -> Self {
        Self::PreDeclTy(PreDeclTypeSpec::Mono(ident))
    }

    pub fn poly(acc: ConstAccessor, args: ConstArgs) -> Self {
        Self::PreDeclTy(PreDeclTypeSpec::Poly(PolyTypeSpec::new(acc, args)))
    }
}

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeBoundSpecs(Vec<TypeBoundSpec>);

impl_displayable_stream_for_wrapper!(TypeBoundSpecs, TypeBoundSpec);

impl Locational for TypeBoundSpecs {
    fn loc(&self) -> Location {
        Location::concat(self.first().unwrap(), self.last().unwrap())
    }
}

/// デコレータは関数を返す関数オブジェクトならば何でも指定できる
/// e.g. @(x -> x)
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

impl VarName {
    pub const fn new(symbol: Token) -> Self {
        Self(symbol)
    }

    pub const fn from_static(symbol: &'static str) -> Self {
        Self(Token::static_symbol(symbol))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(symbol: Str) -> Self {
        Self(Token::from_str(TokenKind::Symbol, &symbol))
    }

    pub fn from_str_and_line(symbol: Str, line: u32) -> Self {
        Self(Token::new(TokenKind::Symbol, symbol, line, 0))
    }

    #[inline]
    pub fn is_const(&self) -> bool {
        self.0
            .content
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    }

    #[inline]
    pub fn is_procedural(&self) -> bool {
        self.0
            .content
            .chars()
            .last()
            .map(|c| c == '!')
            .unwrap_or(false)
    }

    pub fn is_raw(&self) -> bool {
        self.0.content.starts_with('\'')
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
}

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
        Location::concat(self.first().unwrap(), self.last().unwrap())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisRestriction {
    Namespaces(Namespaces),
    SubtypeOf(Box<TypeSpec>),
}

impl_locational_for_enum!(VisRestriction; Namespaces, SubtypeOf);
impl_display_from_nested!(VisRestriction);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessModifier {
    Private, // `::`
    Public,  // `.`
    Auto,    // record unpacking
    Force,   // can access any identifiers
}

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

impl Identifier {
    pub const fn new(vis: VisModifierSpec, name: VarName) -> Self {
        Self { vis, name }
    }

    pub fn static_public(name: &'static str) -> Self {
        Self::new(
            VisModifierSpec::Public(Token::from_str(TokenKind::Dot, ".")),
            VarName::from_static(name),
        )
    }

    pub fn public(name: Str) -> Self {
        Self::new(
            VisModifierSpec::Public(Token::from_str(TokenKind::Dot, ".")),
            VarName::from_str(name),
        )
    }

    pub fn private(name: Str) -> Self {
        Self::new(VisModifierSpec::Private, VarName::from_str(name))
    }

    pub fn private_from_token(symbol: Token) -> Self {
        Self::new(VisModifierSpec::Private, VarName::new(symbol))
    }

    pub fn private_from_varname(name: VarName) -> Self {
        Self::new(VisModifierSpec::Private, name)
    }

    pub fn private_with_line(name: Str, line: u32) -> Self {
        Self::new(
            VisModifierSpec::Private,
            VarName::from_str_and_line(name, line),
        )
    }

    pub fn public_with_line(dot: Token, name: Str, line: u32) -> Self {
        Self::new(
            VisModifierSpec::Public(dot),
            VarName::from_str_and_line(name, line),
        )
    }

    pub fn public_from_token(dot: Token, symbol: Token) -> Self {
        Self::new(VisModifierSpec::Public(dot), VarName::new(symbol))
    }

    pub fn is_const(&self) -> bool {
        self.name.is_const()
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

    pub const fn inspect(&self) -> &Str {
        self.name.inspect()
    }

    pub fn is_procedural(&self) -> bool {
        self.name.is_procedural()
    }

    pub fn trim_end_proc_mark(&mut self) {
        self.name.trim_end_proc_mark();
    }
}

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
            None => Location::concat(&self.elems[0], self.elems.last().unwrap()),
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
        }
    }
}

impl_display_from_nested!(VarPattern);
impl_locational_for_enum!(VarPattern; Discard, Ident, Array, Tuple, Record, DataPack);

impl VarPattern {
    pub const fn inspect(&self) -> Option<&Str> {
        match self {
            Self::Ident(ident) => Some(ident.inspect()),
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

    pub fn vis(&self) -> &VisModifierSpec {
        match self {
            Self::Ident(ident) => &ident.vis,
            // TODO: `[.x, .y]`?
            _ => &VisModifierSpec::Private,
        }
    }

    pub fn ident(&self) -> Option<&Identifier> {
        match self {
            Self::Ident(ident) => Some(ident),
            _ => None,
        }
    }
}

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

impl VarSignature {
    pub const fn new(pat: VarPattern, t_spec: Option<TypeSpecWithOp>) -> Self {
        Self { pat, t_spec }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }

    pub fn is_const(&self) -> bool {
        self.pat.is_const()
    }

    pub fn vis(&self) -> &VisModifierSpec {
        self.pat.vis()
    }

    pub fn ident(&self) -> Option<&Identifier> {
        match &self.pat {
            VarPattern::Ident(ident) => Some(ident),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Vars {
    pub(crate) elems: Vec<VarSignature>,
}

impl NestedDisplay for Vars {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", fmt_vec(&self.elems))
    }
}

impl_display_from_nested!(Vars);
impl_stream!(Vars, VarSignature, elems);

impl Vars {
    pub const fn new(elems: Vec<VarSignature>) -> Self {
        Self { elems }
    }

    pub const fn empty() -> Self {
        Self::new(vec![])
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamArrayPattern {
    l_sqbr: Token,
    pub elems: Params,
    r_sqbr: Token,
}

impl NestedDisplay for ParamArrayPattern {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "[{}]", self.elems)
    }
}

impl_display_from_nested!(ParamArrayPattern);
impl_locational!(ParamArrayPattern, l_sqbr, r_sqbr);

impl ParamArrayPattern {
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

impl ParamTuplePattern {
    pub const fn new(elems: Params) -> Self {
        Self { elems }
    }
}

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

impl ParamRecordAttrs {
    pub const fn new(elems: Vec<ParamRecordAttr>) -> Self {
        Self { elems }
    }

    pub const fn empty() -> Self {
        Self::new(vec![])
    }
}

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

impl ParamRecordPattern {
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
}

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
        }
    }
}

impl_display_from_nested!(ParamPattern);
impl_locational_for_enum!(ParamPattern; Discard, VarName, Lit, Array, Tuple, Record, Ref, RefMut);

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
            Location::concat(&self.pat, t_spec)
        } else {
            self.pat.loc()
        }
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

impl DefaultParamSignature {
    pub const fn new(sig: NonDefaultParamSignature, default_val: Expr) -> Self {
        Self { sig, default_val }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.sig.pat.inspect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Params {
    pub non_defaults: Vec<NonDefaultParamSignature>,
    pub var_params: Option<Box<NonDefaultParamSignature>>,
    pub defaults: Vec<DefaultParamSignature>,
    pub parens: Option<(Token, Token)>,
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {})",
            fmt_vec(&self.non_defaults),
            fmt_option!(pre "*", &self.var_params),
            fmt_vec(&self.defaults)
        )
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
            self.non_defaults.first(),
            self.var_params.as_ref(),
            self.defaults.last(),
        ) {
            (Some(l), _, Some(r)) => Location::concat(l, r),
            (Some(l), Some(r), None) => Location::concat(l, r.as_ref()),
            (None, Some(l), Some(r)) => Location::concat(l.as_ref(), r),
            (Some(l), None, None) => Location::concat(l, self.non_defaults.last().unwrap()),
            (None, Some(var), None) => var.loc(),
            (None, None, Some(r)) => Location::concat(self.defaults.first().unwrap(), r),
            _ => Location::Unknown,
        }
    }
}

type RawParams = (
    Vec<NonDefaultParamSignature>,
    Option<Box<NonDefaultParamSignature>>,
    Vec<DefaultParamSignature>,
    Option<(Token, Token)>,
);

impl Params {
    pub fn new(
        non_defaults: Vec<NonDefaultParamSignature>,
        var_params: Option<NonDefaultParamSignature>,
        defaults: Vec<DefaultParamSignature>,
        parens: Option<(Token, Token)>,
    ) -> Self {
        Self {
            non_defaults,
            var_params: var_params.map(Box::new),
            defaults,
            parens,
        }
    }

    pub fn single(non_default: NonDefaultParamSignature) -> Self {
        Self::new(vec![non_default], None, vec![], None)
    }

    pub fn deconstruct(self) -> RawParams {
        (
            self.non_defaults,
            self.var_params,
            self.defaults,
            self.parens,
        )
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.non_defaults.len() + self.defaults.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 引数を取るならTypeでもSubr扱い
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubrSignature {
    pub decorators: HashSet<Decorator>,
    pub ident: Identifier,
    pub bounds: TypeBoundSpecs,
    pub params: Params,
    pub return_t_spec: Option<TypeSpec>,
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
        return_t: Option<TypeSpec>,
    ) -> Self {
        Self {
            decorators,
            ident,
            bounds,
            params,
            return_t_spec: return_t,
        }
    }

    pub fn is_const(&self) -> bool {
        self.ident.is_const()
    }

    pub fn vis(&self) -> &VisModifierSpec {
        &self.ident.vis
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LambdaSignature {
    pub bounds: TypeBoundSpecs,
    pub params: Params,
    pub return_t_spec: Option<TypeSpec>,
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
        return_t_spec: Option<TypeSpec>,
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
            Params::new(vec![], None, vec![], parens),
            None,
            TypeBoundSpecs::empty(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DefId(pub usize);

impl DefId {
    pub fn inc(&mut self) {
        self.0 += 1;
    }
}

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

impl Lambda {
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
            Self::Subr(c) => c.return_t_spec.as_ref(),
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

    pub fn vis(&self) -> &VisModifierSpec {
        match self {
            Self::Var(var) => var.vis(),
            Self::Subr(subr) => subr.vis(),
        }
    }
}

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

impl TypeAscription {
    pub fn new(expr: Expr, t_spec: TypeSpecWithOp) -> Self {
        Self {
            expr: Box::new(expr),
            t_spec,
        }
    }

    pub fn kind(&self) -> AscriptionKind {
        self.t_spec.ascription_kind()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DefKind {
    Class,
    Inherit,
    Trait,
    Subsume,
    StructuralTrait,
    ErgImport,
    PyImport,
    Patch,
    /// type alias included
    Other,
}

impl DefKind {
    pub const fn is_trait(&self) -> bool {
        matches!(self, Self::Trait | Self::Subsume | Self::StructuralTrait)
    }

    pub const fn is_class(&self) -> bool {
        matches!(self, Self::Class | Self::Inherit)
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

    pub const fn is_import(&self) -> bool {
        self.is_erg_import() || self.is_py_import()
    }

    pub const fn is_other(&self) -> bool {
        matches!(self, Self::Other)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefBody {
    pub op: Token,
    pub block: Block,
    pub id: DefId,
}

impl_locational!(DefBody, lossy op, block);

impl DefBody {
    pub const fn new(op: Token, block: Block, id: DefId) -> Self {
        Self { op, block, id }
    }

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
                _ => DefKind::Other,
            },
            _ => DefKind::Other,
        }
    }
}

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

impl Def {
    pub const fn new(sig: Signature, body: DefBody) -> Self {
        Self { sig, body }
    }

    pub fn is_const(&self) -> bool {
        self.sig.is_const()
    }

    pub const fn is_subr(&self) -> bool {
        matches!(&self.sig, Signature::Subr(_))
    }

    pub fn def_kind(&self) -> DefKind {
        self.body.def_kind()
    }
}

/// This is not necessary for Erg syntax, but necessary for mapping ASTs in Python
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

impl ReDef {
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Methods {
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
        class: TypeSpec,
        class_as_expr: Expr,
        vis: VisModifierSpec,
        attrs: ClassAttrs,
    ) -> Self {
        Self {
            class,
            class_as_expr: Box::new(class_as_expr),
            vis,
            attrs,
        }
    }
}

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

impl ClassDef {
    pub const fn new(def: Def, methods: Vec<Methods>) -> Self {
        Self {
            def,
            methods_list: methods,
        }
    }
}

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

impl PatchDef {
    pub const fn new(def: Def, methods: Vec<Methods>) -> Self {
        Self {
            def,
            methods_list: methods,
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
    /// for mapping to Python AST
    Dummy(Dummy),
}

impl_nested_display_for_chunk_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAscription, Def, Methods, ClassDef, PatchDef, ReDef, Dummy);
impl_from_trait_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAscription, Def, Methods, ClassDef, PatchDef, ReDef, Dummy);
impl_display_from_nested!(Expr);
impl_locational_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAscription, Def, Methods, ClassDef, PatchDef, ReDef, Dummy);

impl Expr {
    pub fn is_match_call(&self) -> bool {
        matches!(self, Expr::Call(call) if call.is_match())
    }

    pub fn is_bin_in(&self) -> bool {
        matches!(self, Expr::BinOp(bin) if bin.op.is(TokenKind::InOp))
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
            Self::Dummy(_) => "dummy",
        }
    }

    pub fn need_to_be_closed(&self) -> bool {
        matches!(
            self,
            Expr::BinOp(_) | Expr::UnaryOp(_) | Expr::Lambda(_) | Expr::TypeAscription(_)
        )
    }

    pub fn get_name(&self) -> Option<&Str> {
        match self {
            Expr::Accessor(acc) => acc.name(),
            _ => None,
        }
    }

    pub fn local(name: &str, lineno: u32, col_begin: u32) -> Self {
        Self::Accessor(Accessor::local(Token::new(
            TokenKind::Symbol,
            Str::rc(name),
            lineno,
            col_begin,
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

    pub fn type_asc(self, t_spec: TypeSpecWithOp) -> TypeAscription {
        TypeAscription::new(self, t_spec)
    }

    pub fn type_asc_expr(self, t_spec: TypeSpecWithOp) -> Self {
        Self::TypeAscription(self.type_asc(t_spec))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Module(Block);

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_lines(self.0.iter(), f, 0)
    }
}

impl Locational for Module {
    fn loc(&self) -> Location {
        Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
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
}

#[derive(Debug)]
pub struct AST {
    pub name: Str,
    pub module: Module,
}

impl_display_for_single_struct!(AST, module);

impl AST {
    pub const fn new(name: Str, module: Module) -> Self {
        Self { name, module }
    }

    pub fn is_empty(&self) -> bool {
        self.module.is_empty()
    }
}
