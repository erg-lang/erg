//! defines `Expr` (Expression, the minimum executing unit of Erg).
use std::borrow::Borrow;
use std::fmt;
use std::fmt::Write as _;

use erg_common::error::Location;
use erg_common::set::Set as HashSet;
use erg_common::traits::{Locational, NestedDisplay, Stream};
use erg_common::vis::{Field, Visibility};
use erg_common::{
    fmt_option, fmt_vec, impl_display_for_enum, impl_display_for_single_struct,
    impl_display_from_nested, impl_displayable_stream_for_wrapper, impl_locational,
    impl_locational_for_enum, impl_nested_display_for_chunk_enum, impl_nested_display_for_enum,
    impl_stream, impl_stream_for_wrapper,
};
use erg_common::{fmt_vec_split_with, Str};

use crate::token::{Token, TokenKind};

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

    pub fn nat(n: usize, line: usize) -> Self {
        let token = Token::new(TokenKind::NatLit, Str::from(n.to_string()), line, 0);
        Self { token }
    }

    #[inline]
    pub fn is(&self, kind: TokenKind) -> bool {
        self.token.is(kind)
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
    kw_args: Vec<KwArg>,
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
            Location::concat(l, r)
        } else {
            Location::concat(&self.pos_args[0], self.pos_args.last().unwrap())
        }
    }
}

// impl_stream!(Args, Arg, args);

impl Args {
    pub const fn new(
        pos_args: Vec<PosArg>,
        kw_args: Vec<KwArg>,
        paren: Option<(Token, Token)>,
    ) -> Self {
        Self {
            pos_args,
            kw_args,
            paren,
        }
    }

    pub const fn empty() -> Self {
        Self::new(vec![], vec![], None)
    }

    // for replacing to hir::Args
    pub fn deconstruct(self) -> (Vec<PosArg>, Vec<KwArg>, Option<(Token, Token)>) {
        (self.pos_args, self.kw_args, self.paren)
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

    pub fn push_kw(&mut self, arg: KwArg) {
        self.kw_args.push(arg);
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
            write!(f, "({}).{}", self.obj, self.ident)
        } else {
            write!(f, "{}.{}", self.obj, self.ident)
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
impl_locational!(Subscript, obj, index);

impl Subscript {
    pub fn new(obj: Expr, index: Expr) -> Self {
        Self {
            obj: Box::new(obj),
            index: Box::new(index),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeAppArgs {
    l_vbar: Token,
    args: Args,
    r_vbar: Token,
}

impl NestedDisplay for TypeAppArgs {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "|{}|", self.args)
    }
}

impl_display_from_nested!(TypeAppArgs);
impl_locational!(TypeAppArgs, l_vbar, r_vbar);

impl TypeAppArgs {
    pub fn new(l_vbar: Token, args: Args, r_vbar: Token) -> Self {
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
        Self::Ident(Identifier::new(None, VarName::new(symbol)))
    }

    pub const fn public(dot: Token, symbol: Token) -> Self {
        Self::Ident(Identifier::new(Some(dot), VarName::new(symbol)))
    }

    pub fn attr(obj: Expr, ident: Identifier) -> Self {
        Self::Attr(Attribute::new(obj, ident))
    }

    pub fn tuple_attr(obj: Expr, index: Literal) -> Self {
        Self::TupleAttr(TupleAttribute::new(obj, index))
    }

    pub fn subscr(obj: Expr, index: Expr) -> Self {
        Self::Subscr(Subscript::new(obj, index))
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
        write!(f, "\n{}]", "    ".repeat(level))
    }
}

impl_display_from_nested!(NormalArray);
impl_locational!(NormalArray, l_sqbr, r_sqbr);

impl NormalArray {
    pub fn new(l_sqbr: Token, r_sqbr: Token, elems: Args) -> Self {
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
impl_locational!(ArrayWithLength, l_sqbr, r_sqbr);

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
            write!(generators, "{} <- {}, ", name, gen).unwrap();
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
impl_locational!(ArrayComprehension, l_sqbr, r_sqbr);

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
        write!(f, "\n{})", "    ".repeat(level))
    }
}

impl_display_from_nested!(NormalTuple);
impl_locational!(NormalTuple, elems, elems);

impl NormalTuple {
    pub fn new(elems: Args) -> Self {
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NormalDict {
    l_brace: Token,
    r_brace: Token,
    pub attrs: Args, // TODO: Impl K: V Pair
}

impl NestedDisplay for NormalDict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", self.attrs)
    }
}

impl_display_from_nested!(NormalDict);
impl_locational!(NormalDict, l_brace, r_brace);

impl NormalDict {
    pub fn new(l_brace: Token, r_brace: Token, attrs: Args) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
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
impl_locational!(DictComprehension, l_brace, r_brace);

impl DictComprehension {
    pub fn new(l_brace: Token, r_brace: Token, attrs: Args, guards: Vec<Expr>) -> Self {
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
pub struct RecordAttrs(Vec<Def>);

impl NestedDisplay for RecordAttrs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)
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

impl RecordAttrs {
    pub const fn new(attrs: Vec<Def>) -> Self {
        Self(attrs)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Def> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Def> {
        self.0.iter_mut()
    }
}

impl IntoIterator for RecordAttrs {
    type Item = Def;
    type IntoIter = <Vec<Def> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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
        writeln!(f, "\n{}}}", "    ".repeat(level))
    }
}

impl_display_from_nested!(NormalRecord);
impl_locational!(NormalRecord, l_brace, r_brace);

impl NormalRecord {
    pub fn new(l_brace: Token, r_brace: Token, attrs: RecordAttrs) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
    }
}

/// e.g. {x; y; z} (syntax sugar of {x = x; y = y; z = z})
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShortenedRecord {
    pub l_brace: Token,
    pub r_brace: Token,
    pub idents: Vec<Identifier>,
}

impl NestedDisplay for ShortenedRecord {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{")?;
        for ident in self.idents.iter() {
            write!(f, "{}; ", ident)?;
        }
        write!(f, "}}")
    }
}

impl_display_from_nested!(ShortenedRecord);
impl_locational!(ShortenedRecord, l_brace, r_brace);

impl ShortenedRecord {
    pub const fn new(l_brace: Token, r_brace: Token, idents: Vec<Identifier>) -> Self {
        Self {
            l_brace,
            r_brace,
            idents,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Record {
    Normal(NormalRecord),
    Shortened(ShortenedRecord),
}

impl_nested_display_for_enum!(Record; Normal, Shortened);
impl_display_for_enum!(Record; Normal, Shortened);
impl_locational_for_enum!(Record; Normal, Shortened);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NormalSet {
    l_brace: Token,
    r_brace: Token,
    pub elems: Args,
}

impl NestedDisplay for NormalSet {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", self.elems)
    }
}

impl_display_from_nested!(NormalSet);
impl_locational!(NormalSet, l_brace, r_brace);

impl NormalSet {
    pub fn new(l_brace: Token, r_brace: Token, elems: Args) -> Self {
        Self {
            l_brace,
            r_brace,
            elems,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Set {
    Normal(NormalSet),
    // Comprehension(SetComprehension),
}

impl_nested_display_for_enum!(Set; Normal);
impl_display_for_enum!(Set; Normal);
impl_locational_for_enum!(Set; Normal);

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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Call {
    pub obj: Box<Expr>,
    pub method_name: Option<Identifier>,
    pub args: Args,
}

impl NestedDisplay for Call {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({})", self.obj)?;
        } else {
            write!(f, "{}", self.obj)?;
        }
        if let Some(method_name) = self.method_name.as_ref() {
            write!(f, "{}", method_name)?;
        }
        writeln!(f, ":")?;
        self.args.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Call);

impl Locational for Call {
    fn loc(&self) -> Location {
        if self.args.is_empty() {
            self.obj.loc()
        } else {
            Location::concat(self.obj.as_ref(), &self.args)
        }
    }
}

impl Call {
    pub fn new(obj: Expr, method_name: Option<Identifier>, args: Args) -> Self {
        Self {
            obj: Box::new(obj),
            method_name,
            args,
        }
    }
}

/// e.g. `Data::{x = 1; y = 2}`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataPack {
    pub class: Box<Expr>,
    pub connector: Token,
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
    pub fn new(class: Expr, connector: Token, args: Record) -> Self {
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
        Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
    }
}

impl_stream_for_wrapper!(Block, Expr);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstLocal {
    pub symbol: Token,
}

impl NestedDisplay for ConstLocal {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl_display_from_nested!(ConstLocal);
impl_locational!(ConstLocal, symbol);

impl ConstLocal {
    pub const fn new(symbol: Token) -> Self {
        Self { symbol }
    }

    pub fn dummy(name: &'static str) -> Self {
        Self::new(Token::from_str(TokenKind::Symbol, name))
    }

    // &strにするとクローンしたいときにアロケーションコストがかかるので&Strのままで
    pub const fn inspect(&self) -> &Str {
        &self.symbol.content
    }
}

/// type variables
pub type ConstVar = ConstLocal;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstAttribute {
    pub obj: Box<ConstExpr>,
    pub name: ConstLocal,
}

impl NestedDisplay for ConstAttribute {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if self.obj.need_to_be_closed() {
            write!(f, "({}).{}", self.obj, self.name)
        } else {
            write!(f, "{}.{}", self.obj, self.name)
        }
    }
}

impl_display_from_nested!(ConstAttribute);
impl_locational!(ConstAttribute, obj, name);

impl ConstAttribute {
    pub fn new(expr: ConstExpr, name: ConstLocal) -> Self {
        Self {
            obj: Box::new(expr),
            name,
        }
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
    Local(ConstLocal),
    SelfDot(ConstLocal),
    Attr(ConstAttribute),
    TupleAttr(ConstTupleAttribute),
    Subscr(ConstSubscript),
}

impl_nested_display_for_enum!(ConstAccessor; Local, SelfDot, Attr, TupleAttr, Subscr);
impl_display_from_nested!(ConstAccessor);
impl_locational_for_enum!(ConstAccessor; Local, SelfDot, Attr, TupleAttr, Subscr);

impl ConstAccessor {
    pub const fn local(symbol: Token) -> Self {
        Self::Local(ConstLocal::new(symbol))
    }

    pub const fn dot_self(attr: Token) -> Self {
        Self::SelfDot(ConstLocal::new(attr))
    }

    pub fn attr(obj: ConstExpr, name: ConstLocal) -> Self {
        Self::Attr(ConstAttribute::new(obj, name))
    }

    pub fn subscr(obj: ConstExpr, index: ConstExpr) -> Self {
        Self::Subscr(ConstSubscript::new(obj, index))
    }
}

/// DictはキーつきArray(型としては別物)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstArray {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub elems: ConstArgs,
    pub guard: Option<Box<ConstExpr>>,
}

impl NestedDisplay for ConstArray {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if let Some(guard) = &self.guard {
            write!(f, "[{} | {}]", self.elems, guard)
        } else {
            write!(f, "[{}]", self.elems)
        }
    }
}

impl_display_from_nested!(ConstArray);
impl_locational!(ConstArray, l_sqbr, r_sqbr);

impl ConstArray {
    pub fn new(l_sqbr: Token, r_sqbr: Token, elems: ConstArgs, guard: Option<ConstExpr>) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            elems,
            guard: guard.map(Box::new),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstDict {
    l_brace: Token,
    r_brace: Token,
    pub attrs: ConstArgs,
}

impl NestedDisplay for ConstDict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", self.attrs)
    }
}

impl_display_from_nested!(ConstDict);
impl_locational!(ConstDict, l_brace, r_brace);

impl ConstDict {
    pub fn new(l_brace: Token, r_brace: Token, attrs: ConstArgs) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
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
}

/// valid expression for an argument of polymorphic types
/// 多相型の実引数として有効な式
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstExpr {
    Lit(Literal),
    Erased(Literal), // _
    Accessor(ConstAccessor),
    App(ConstApp),
    Array(ConstArray),
    // Set(Set),
    Dict(ConstDict),
    BinOp(ConstBinOp),
    UnaryOp(ConstUnaryOp),
}

impl_nested_display_for_chunk_enum!(ConstExpr; Lit, Accessor, App, Array, Dict, BinOp, UnaryOp, Erased);
impl_display_from_nested!(ConstExpr);
impl_locational_for_enum!(ConstExpr; Lit, Accessor, App, Array, Dict, BinOp, UnaryOp, Erased);

impl ConstExpr {
    pub fn need_to_be_closed(&self) -> bool {
        matches!(self, Self::BinOp(_) | Self::UnaryOp(_))
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
            unreachable!()
        }
    }
}

// impl_stream!(ConstArgs, ConstKwArg, pos_args);

impl ConstArgs {
    pub const fn new(
        pos_args: Vec<ConstPosArg>,
        kw_args: Vec<ConstKwArg>,
        paren: Option<(Token, Token)>,
    ) -> Self {
        Self {
            pos_args,
            kw_args,
            paren,
        }
    }

    pub const fn empty() -> Self {
        Self::new(vec![], vec![], None)
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SimpleTypeSpec {
    pub name: VarName,
    pub args: ConstArgs, // args can be nested (e.g. Vec Vec Int)
}

impl fmt::Display for SimpleTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.args.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}{}", self.name, self.args)
        }
    }
}

impl Locational for SimpleTypeSpec {
    fn loc(&self) -> Location {
        if let Some(last) = self.args.kw_args.last() {
            Location::concat(&self.name, last)
        } else if let Some(last) = self.args.pos_args.last() {
            Location::concat(&self.name, last)
        } else {
            self.name.loc()
        }
    }
}

impl SimpleTypeSpec {
    pub const fn new(name: VarName, args: ConstArgs) -> Self {
        Self { name, args }
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
    Simple(SimpleTypeSpec),
    Attr {
        namespace: Vec<VarName>,
        t: SimpleTypeSpec,
    },
    Subscr {
        namespace: Vec<VarName>,
        name: VarName,
        index: Token,
    },
}

impl fmt::Display for PreDeclTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreDeclTypeSpec::Simple(ts) => write!(f, "{}", ts),
            PreDeclTypeSpec::Attr { namespace, t } => {
                write!(f, "{}.{}", namespace.join("."), t)
            }
            PreDeclTypeSpec::Subscr {
                namespace,
                name,
                index,
            } => {
                write!(f, "{}.{}[{}]", namespace.join("."), name, index)
            }
        }
    }
}

impl Locational for PreDeclTypeSpec {
    fn loc(&self) -> Location {
        match self {
            Self::Simple(s) => s.loc(),
            Self::Attr { namespace, t } => Location::concat(&namespace[0], t),
            Self::Subscr {
                namespace, index, ..
            } => Location::concat(&namespace[0], index),
        }
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubrTypeSpec {
    pub lparen: Option<Token>,
    pub non_defaults: Vec<ParamTySpec>,
    pub var_args: Option<Box<ParamTySpec>>,
    pub defaults: Vec<ParamTySpec>,
    pub arrow: Token,
    pub return_t: Box<TypeSpec>,
}

impl fmt::Display for SubrTypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {}, := {}) {} {}",
            fmt_vec(&self.non_defaults),
            fmt_option!(pre "...", &self.var_args),
            fmt_vec(&self.defaults),
            self.arrow.content,
            self.return_t
        )
    }
}

impl Locational for SubrTypeSpec {
    fn loc(&self) -> Location {
        if let Some(lparen) = &self.lparen {
            Location::concat(lparen, self.return_t.as_ref())
        } else {
            // FIXME: only default subrs
            Location::concat(self.non_defaults.first().unwrap(), self.return_t.as_ref())
        }
    }
}

impl SubrTypeSpec {
    pub fn new(
        lparen: Option<Token>,
        non_defaults: Vec<ParamTySpec>,
        var_args: Option<ParamTySpec>,
        defaults: Vec<ParamTySpec>,
        arrow: Token,
        return_t: TypeSpec,
    ) -> Self {
        Self {
            lparen,
            non_defaults,
            var_args: var_args.map(Box::new),
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

/// * Array: `[Int; 3]`, `[Int, Ratio, Complex]`, etc.
/// * Dict: `[Str: Str]`, etc.
/// * And (Intersection type): Add and Sub and Mul (== Num), etc.
/// * Not (Diff type): Pos == Nat not {0}, etc.
/// * Or (Union type): Int or None (== Option Int), etc.
/// * Enum: `{0, 1}` (== Binary), etc.
/// * Range: 1..12, 0.0<..1.0, etc.
/// * Record: {.into_s: Self.() -> Str }, etc.
/// * Subr: Int -> Int, Int => None, T.(X) -> Int, etc.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeSpec {
    PreDeclTy(PreDeclTypeSpec),
    /* Composite types */
    Array(ArrayTypeSpec),
    Tuple(Vec<TypeSpec>),
    // Dict(),
    // Option(),
    And(Box<TypeSpec>, Box<TypeSpec>),
    Not(Box<TypeSpec>, Box<TypeSpec>),
    Or(Box<TypeSpec>, Box<TypeSpec>),
    Enum(ConstArgs),
    Interval {
        op: Token,
        lhs: ConstExpr,
        rhs: ConstExpr,
    },
    // Record(),
    Subr(SubrTypeSpec),
    TypeApp {
        spec: Box<TypeSpec>,
        args: TypeAppArgs,
    },
}

impl fmt::Display for TypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreDeclTy(ty) => write!(f, "{ty}"),
            Self::And(lhs, rhs) => write!(f, "{lhs} and {rhs}"),
            Self::Not(lhs, rhs) => write!(f, "{lhs} not {rhs}"),
            Self::Or(lhs, rhs) => write!(f, "{lhs} or {rhs}"),
            Self::Array(arr) => write!(f, "{arr}"),
            Self::Tuple(tys) => write!(f, "({})", fmt_vec(tys)),
            Self::Enum(elems) => write!(f, "{{{elems}}}"),
            Self::Interval { op, lhs, rhs } => write!(f, "{lhs}{}{rhs}", op.inspect()),
            Self::Subr(s) => write!(f, "{s}"),
            Self::TypeApp { spec, args } => write!(f, "{spec}{args}"),
        }
    }
}

impl Locational for TypeSpec {
    fn loc(&self) -> Location {
        match self {
            Self::PreDeclTy(sig) => sig.loc(),
            Self::And(lhs, rhs) | Self::Not(lhs, rhs) | Self::Or(lhs, rhs) => {
                Location::concat(lhs.as_ref(), rhs.as_ref())
            }
            Self::Array(arr) => arr.loc(),
            // TODO: ユニット
            Self::Tuple(tys) => Location::concat(tys.first().unwrap(), tys.last().unwrap()),
            Self::Enum(set) => set.loc(),
            Self::Interval { lhs, rhs, .. } => Location::concat(lhs, rhs),
            Self::Subr(s) => s.loc(),
            Self::TypeApp { spec, args } => Location::concat(spec.as_ref(), args),
        }
    }
}

impl TypeSpec {
    pub fn and(lhs: TypeSpec, rhs: TypeSpec) -> Self {
        Self::And(Box::new(lhs), Box::new(rhs))
    }

    pub fn not(lhs: TypeSpec, rhs: TypeSpec) -> Self {
        Self::Not(Box::new(lhs), Box::new(rhs))
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeSpecWithOp {
    pub op: Token,
    pub t_spec: TypeSpec,
}

impl NestedDisplay for TypeSpecWithOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} {}", self.op.content, self.t_spec)
    }
}

impl_display_from_nested!(TypeSpecWithOp);
impl_locational!(TypeSpecWithOp, op, t_spec);

impl TypeSpecWithOp {
    pub fn new(op: Token, t_spec: TypeSpec) -> Self {
        Self { op, t_spec }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeBoundSpec {
    NonDefault {
        lhs: Token,
        spec: TypeSpecWithOp,
    },
    WithDefault {
        lhs: Token,
        spec: Box<TypeSpecWithOp>,
        default: ConstExpr,
    }, // e.g. S: Show := Str
}

impl NestedDisplay for TypeBoundSpec {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::NonDefault { lhs, spec } => write!(f, "{}{spec}", lhs.content),
            Self::WithDefault { lhs, spec, default } => {
                write!(f, "{}{} := {}", lhs.content, spec, default)
            }
        }
    }
}

impl_display_from_nested!(TypeBoundSpec);

impl Locational for TypeBoundSpec {
    fn loc(&self) -> Location {
        match self {
            Self::NonDefault { lhs, spec } => Location::concat(lhs, spec),
            Self::WithDefault { lhs, default, .. } => Location::concat(lhs, default),
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarName(Token);

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

    pub fn from_str_and_line(symbol: Str, line: usize) -> Self {
        Self(Token::new(TokenKind::Symbol, &symbol, line, 0))
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

    pub const fn token(&self) -> &Token {
        &self.0
    }

    pub fn into_token(self) -> Token {
        self.0
    }

    pub const fn inspect(&self) -> &Str {
        &self.0.content
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub dot: Option<Token>,
    pub name: VarName,
}

impl NestedDisplay for Identifier {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match &self.dot {
            Some(_dot) => write!(f, ".{}", self.name),
            None => write!(f, "::{}", self.name),
        }
    }
}

impl_display_from_nested!(Identifier);

impl Locational for Identifier {
    fn loc(&self) -> Location {
        if let Some(dot) = &self.dot {
            Location::concat(dot, &self.name)
        } else {
            self.name.loc()
        }
    }
}

impl From<&Identifier> for Field {
    fn from(ident: &Identifier) -> Self {
        Self::new(ident.vis(), ident.inspect().clone())
    }
}

impl Identifier {
    pub const fn new(dot: Option<Token>, name: VarName) -> Self {
        Self { dot, name }
    }

    pub fn public(name: &'static str) -> Self {
        Self::new(
            Some(Token::from_str(TokenKind::Dot, ".")),
            VarName::from_static(name),
        )
    }

    pub fn private(name: Str) -> Self {
        Self::new(None, VarName::from_str(name))
    }

    pub fn private_with_line(name: Str, line: usize) -> Self {
        Self::new(None, VarName::from_str_and_line(name, line))
    }

    pub fn public_with_line(dot: Token, name: Str, line: usize) -> Self {
        Self::new(Some(dot), VarName::from_str_and_line(name, line))
    }

    pub fn is_const(&self) -> bool {
        self.name.is_const()
    }

    pub const fn vis(&self) -> Visibility {
        match &self.dot {
            Some(_) => Visibility::Public,
            None => Visibility::Private,
        }
    }

    pub const fn inspect(&self) -> &Str {
        self.name.inspect()
    }

    pub fn is_procedural(&self) -> bool {
        self.name.is_procedural()
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
    pub args: VarRecordPattern,
}

impl fmt::Display for VarDataPackPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.class, self.args)
    }
}

impl_locational!(VarDataPackPattern, class, args);

impl VarDataPackPattern {
    pub const fn new(class: TypeSpec, args: VarRecordPattern) -> Self {
        Self { class, args }
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
            Self::Ident(ident) => write!(f, "{}", ident),
            Self::Array(a) => write!(f, "{}", a),
            Self::Tuple(t) => write!(f, "{}", t),
            Self::Record(r) => write!(f, "{}", r),
            Self::DataPack(d) => write!(f, "{}", d),
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

    pub const fn vis(&self) -> Visibility {
        match self {
            Self::Ident(ident) => ident.vis(),
            // TODO: `[.x, .y]`?
            _ => Visibility::Private,
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
    pub t_spec: Option<TypeSpec>,
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
    pub const fn new(pat: VarPattern, t_spec: Option<TypeSpec>) -> Self {
        Self { pat, t_spec }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }

    pub fn is_const(&self) -> bool {
        self.pat.is_const()
    }

    pub const fn vis(&self) -> Visibility {
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
    pub rhs: ParamSignature,
}

impl NestedDisplay for ParamRecordAttr {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} = {}", self.lhs, self.rhs)
    }
}

impl_display_from_nested!(ParamRecordAttr);
impl_locational!(ParamRecordAttr, lhs, rhs);

impl ParamRecordAttr {
    pub const fn new(lhs: Identifier, rhs: ParamSignature) -> Self {
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
    l_brace: Token,
    pub(crate) elems: ParamRecordAttrs,
    r_brace: Token,
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
            Self::Discard(tok) => write!(f, "{}", tok),
            Self::VarName(var_name) => write!(f, "{}", var_name),
            Self::Lit(lit) => write!(f, "{}", lit),
            Self::Array(array) => write!(f, "{}", array),
            Self::Tuple(tuple) => write!(f, "{}", tuple),
            Self::Record(record) => write!(f, "{}", record),
            Self::Ref(var_name) => write!(f, "ref {}", var_name),
            Self::RefMut(var_name) => write!(f, "ref! {}", var_name),
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
}

/// Once the default_value is set to Some, all subsequent values must be Some
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamSignature {
    pub pat: ParamPattern,
    pub t_spec: Option<TypeSpecWithOp>,
    pub opt_default_val: Option<ConstExpr>,
}

impl NestedDisplay for ParamSignature {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        if let Some(default_val) = &self.opt_default_val {
            write!(
                f,
                "{}{} := {}",
                self.pat,
                fmt_option!(self.t_spec),
                default_val,
            )
        } else {
            write!(f, "{}{}", self.pat, fmt_option!(self.t_spec),)
        }
    }
}

impl_display_from_nested!(ParamSignature);

impl Locational for ParamSignature {
    fn loc(&self) -> Location {
        if let Some(default) = &self.opt_default_val {
            Location::concat(&self.pat, default)
        } else if let Some(t_spec) = &self.t_spec {
            Location::concat(&self.pat, t_spec)
        } else {
            self.pat.loc()
        }
    }
}

impl ParamSignature {
    pub const fn new(
        pat: ParamPattern,
        t_spec: Option<TypeSpecWithOp>,
        opt_default_val: Option<ConstExpr>,
    ) -> Self {
        Self {
            pat,
            t_spec,
            opt_default_val,
        }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }

    pub fn has_default(&self) -> bool {
        self.opt_default_val.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Params {
    pub non_defaults: Vec<ParamSignature>,
    pub var_args: Option<Box<ParamSignature>>,
    pub defaults: Vec<ParamSignature>,
    pub parens: Option<(Token, Token)>,
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {})",
            fmt_vec(&self.non_defaults),
            fmt_option!(pre "...", &self.var_args),
            fmt_vec(&self.defaults)
        )
    }
}

impl Locational for Params {
    fn loc(&self) -> Location {
        // FIXME: varargs
        if let Some((l, r)) = &self.parens {
            Location::concat(l, r)
        } else if !self.non_defaults.is_empty() {
            Location::concat(&self.non_defaults[0], self.non_defaults.last().unwrap())
        } else if let Some(var_args) = &self.var_args {
            if !self.defaults.is_empty() {
                Location::concat(var_args.as_ref(), self.defaults.last().unwrap())
            } else {
                var_args.loc()
            }
        } else if !self.defaults.is_empty() {
            Location::concat(&self.defaults[0], self.defaults.last().unwrap())
        } else {
            panic!()
        }
    }
}

type RawParams = (
    Vec<ParamSignature>,
    Option<Box<ParamSignature>>,
    Vec<ParamSignature>,
    Option<(Token, Token)>,
);

impl Params {
    pub fn new(
        non_defaults: Vec<ParamSignature>,
        var_args: Option<ParamSignature>,
        defaults: Vec<ParamSignature>,
        parens: Option<(Token, Token)>,
    ) -> Self {
        Self {
            non_defaults,
            var_args: var_args.map(Box::new),
            defaults,
            parens,
        }
    }

    pub fn deconstruct(self) -> RawParams {
        (self.non_defaults, self.var_args, self.defaults, self.parens)
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
    pub params: Params,
    pub return_t_spec: Option<TypeSpec>,
    pub bounds: TypeBoundSpecs,
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
        params: Params,
        return_t: Option<TypeSpec>,
        bounds: TypeBoundSpecs,
    ) -> Self {
        Self {
            decorators,
            ident,
            params,
            return_t_spec: return_t,
            bounds,
        }
    }

    pub fn is_const(&self) -> bool {
        self.ident.is_const()
    }

    pub const fn vis(&self) -> Visibility {
        self.ident.vis()
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
            unreachable!()
        } else {
            self.params.loc()
        }
    }
}

impl LambdaSignature {
    pub const fn new(params: Params, return_t: Option<TypeSpec>, bounds: TypeBoundSpecs) -> Self {
        Self {
            params,
            return_t_spec: return_t,
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

    pub fn t_spec(&self) -> Option<&TypeSpec> {
        match self {
            Self::Var(v) => v.t_spec.as_ref(),
            Self::Subr(c) => c.return_t_spec.as_ref(),
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Var(var) => var.is_const(),
            Self::Subr(subr) => subr.is_const(),
        }
    }

    pub const fn vis(&self) -> Visibility {
        match self {
            Self::Var(var) => var.vis(),
            Self::Subr(subr) => subr.vis(),
        }
    }
}

/// type_ascription ::= expr ':' type
///                   | expr '<:' type
///                   | expr ':>' type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeAscription {
    pub expr: Box<Expr>,
    pub op: Token,
    pub t_spec: TypeSpec,
}

impl NestedDisplay for TypeAscription {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        writeln!(f, "{}{} {}", self.expr, self.op.content, self.t_spec)
    }
}

impl_display_from_nested!(TypeAscription);
impl_locational!(TypeAscription, expr, t_spec);

impl TypeAscription {
    pub fn new(expr: Expr, op: Token, t_spec: TypeSpec) -> Self {
        Self {
            expr: Box::new(expr),
            op,
            t_spec,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefBody {
    pub op: Token,
    pub block: Block,
    pub id: DefId,
}

impl_locational!(DefBody, op, block);

impl DefBody {
    pub const fn new(op: Token, block: Block, id: DefId) -> Self {
        Self { op, block, id }
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
    pub vis: Token,        // `.` or `::`
    pub defs: RecordAttrs, // TODO: allow declaration
}

impl NestedDisplay for Methods {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{}{}", self.class, self.vis.content)?;
        self.defs.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Methods);
impl_locational!(Methods, class, defs);

impl Methods {
    pub const fn new(class: TypeSpec, vis: Token, defs: RecordAttrs) -> Self {
        Self { class, vis, defs }
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

/// Expression(式)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Lit(Literal),
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
    TypeAsc(TypeAscription),
    Def(Def),
    Methods(Methods),
    ClassDef(ClassDef),
}

impl_nested_display_for_chunk_enum!(Expr; Lit, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAsc, Def, Methods, ClassDef);
impl_display_from_nested!(Expr);
impl_locational_for_enum!(Expr; Lit, Accessor, Array, Tuple, Dict, Set, Record, BinOp, UnaryOp, Call, DataPack, Lambda, TypeAsc, Def, Methods, ClassDef);

impl Expr {
    pub fn is_match_call(&self) -> bool {
        matches!(self, Expr::Call(Call{ obj, .. }) if obj.get_name().map(|s| &s[..] == "match").unwrap_or(false))
    }

    pub fn is_const_acc(&self) -> bool {
        matches!(self, Expr::Accessor(acc) if acc.is_const())
    }

    pub fn need_to_be_closed(&self) -> bool {
        matches!(
            self,
            Expr::BinOp(_) | Expr::UnaryOp(_) | Expr::Lambda(_) | Expr::TypeAsc(_)
        )
    }

    pub fn get_name(&self) -> Option<&Str> {
        match self {
            Expr::Accessor(acc) => acc.name(),
            _ => None,
        }
    }

    pub fn local(name: &str, lineno: usize, col_begin: usize) -> Self {
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
