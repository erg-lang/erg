/// defines High-level Intermediate Representation
use std::fmt;

use erg_common::dict::Dict as HashMap;
use erg_common::error::Location;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::traits::{Locational, NestedDisplay, Stream};
use erg_common::vis::{Field, Visibility};
use erg_common::Str;
use erg_common::{
    enum_unwrap, fmt_option, fmt_vec, impl_display_for_enum, impl_display_from_nested,
    impl_locational, impl_locational_for_enum, impl_nested_display_for_chunk_enum,
    impl_nested_display_for_enum, impl_stream_for_wrapper,
};

use erg_parser::ast::{
    fmt_lines, DefId, DefKind, NonDefaultParamSignature, OperationKind, TypeSpec, VarName,
};
use erg_parser::token::{Token, TokenKind, DOT};

use crate::ty::constructors::{array_t, dict_t, set_t, tuple_t};
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, ValueObj};
use crate::ty::{HasType, Type};

use crate::context::eval::type_from_token_kind;
use crate::error::readable_name;
use crate::varinfo::VarInfo;
use crate::{impl_t, impl_t_for_enum};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Literal {
    pub value: ValueObj,
    pub token: Token, // for Locational
    t: Type,
}

impl_t!(Literal);

impl NestedDisplay for Literal {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{} (: {})", self.token.content, self.t)
    }
}

impl_display_from_nested!(Literal);

impl Locational for Literal {
    #[inline]
    fn loc(&self) -> Location {
        self.token.loc()
    }
}

impl TryFrom<Token> for Literal {
    type Error = ();
    fn try_from(token: Token) -> Result<Self, ()> {
        let data =
            ValueObj::from_str(type_from_token_kind(token.kind), token.content.clone()).ok_or(())?;
        Ok(Self {
            t: data.t(),
            value: data,
            token,
        })
    }
}

impl Literal {
    #[inline]
    pub fn is(&self, kind: TokenKind) -> bool {
        self.token.is(kind)
    }
}

impl Literal {
    pub fn new(value: ValueObj, token: Token) -> Self {
        Self {
            t: value.t(),
            value,
            token,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PosArg {
    pub expr: Expr,
}

impl NestedDisplay for PosArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        self.expr.fmt_nest(f, level)
    }
}

impl_display_from_nested!(PosArg);

impl Locational for PosArg {
    fn loc(&self) -> Location {
        self.expr.loc()
    }
}

impl PosArg {
    pub const fn new(expr: Expr) -> Self {
        Self { expr }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KwArg {
    pub keyword: Token,
    pub expr: Expr,
}

impl NestedDisplay for KwArg {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        writeln!(f, "{} := ", self.keyword)?;
        self.expr.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(KwArg);

impl Locational for KwArg {
    fn loc(&self) -> Location {
        Location::concat(&self.keyword, &self.expr)
    }
}

impl KwArg {
    pub const fn new(keyword: Token, expr: Expr) -> Self {
        Self { keyword, expr }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Args {
    pub pos_args: Vec<PosArg>,
    pub var_args: Option<Box<PosArg>>,
    pub kw_args: Vec<KwArg>,
    paren: Option<(Token, Token)>,
}

impl NestedDisplay for Args {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        if !self.pos_args.is_empty() {
            fmt_lines(self.pos_args.iter(), f, level)?;
        }
        if let Some(var_args) = &self.var_args {
            writeln!(f, "...")?;
            var_args.fmt_nest(f, level)?;
        }
        if !self.kw_args.is_empty() {
            fmt_lines(self.kw_args.iter(), f, level)?;
        }
        Ok(())
    }
}

impl From<Vec<Expr>> for Args {
    fn from(exprs: Vec<Expr>) -> Self {
        Self {
            pos_args: exprs.into_iter().map(PosArg::new).collect(),
            var_args: None,
            kw_args: Vec::new(),
            paren: None,
        }
    }
}

impl_display_from_nested!(Args);

impl Locational for Args {
    fn loc(&self) -> Location {
        if let Some((l, r)) = &self.paren {
            Location::concat(l, r)
        } else if !self.kw_args.is_empty() {
            Location::concat(self.kw_args.first().unwrap(), self.kw_args.last().unwrap())
        } else if !self.pos_args.is_empty() {
            Location::concat(
                self.pos_args.first().unwrap(),
                self.pos_args.last().unwrap(),
            )
        } else {
            Location::Unknown
        }
    }
}

// impl_stream!(Args, KwArg, kw_args);

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

    pub fn empty() -> Self {
        Self::new(vec![], None, vec![], None)
    }

    #[inline]
    pub fn len(&self) -> usize {
        let var_argc = if self.var_args.is_none() { 0 } else { 1 };
        self.pos_args.len() + var_argc + self.kw_args.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos_args.is_empty() && self.var_args.is_none() && self.kw_args.is_empty()
    }

    #[inline]
    pub fn kw_len(&self) -> usize {
        self.kw_args.len()
    }

    pub fn push_pos(&mut self, pos: PosArg) {
        self.pos_args.push(pos);
    }

    pub fn push_kw(&mut self, kw: KwArg) {
        self.kw_args.push(kw);
    }

    pub fn remove(&mut self, index: usize) -> Expr {
        if self.pos_args.get(index).is_some() {
            self.pos_args.remove(index).expr
        } else {
            self.kw_args.remove(index - self.pos_args.len()).expr
        }
    }

    /// try_remove((1, 2, z: 3), 2) == Some(3)
    pub fn try_remove(&mut self, index: usize) -> Option<Expr> {
        if self.pos_args.get(index).is_some() {
            Some(self.pos_args.remove(index).expr)
        } else {
            self.kw_args.get(index - self.pos_args.len())?;
            Some(self.kw_args.remove(index - self.pos_args.len()).expr)
        }
    }

    pub fn try_remove_pos(&mut self, index: usize) -> Option<PosArg> {
        self.pos_args.get(index)?;
        Some(self.pos_args.remove(index))
    }

    pub fn try_remove_kw(&mut self, index: usize) -> Option<KwArg> {
        self.kw_args.get(index)?;
        Some(self.kw_args.remove(index))
    }

    pub fn get(&self, index: usize) -> Option<&Expr> {
        if self.pos_args.get(index).is_some() {
            self.pos_args.get(index).map(|a| &a.expr)
        } else {
            self.kw_args
                .get(index - self.pos_args.len())
                .map(|a| &a.expr)
        }
    }

    pub fn remove_left_or_key(&mut self, key: &str) -> Option<Expr> {
        if !self.pos_args.is_empty() {
            Some(self.pos_args.remove(0).expr)
        } else if let Some(pos) = self
            .kw_args
            .iter()
            .position(|arg| &arg.keyword.inspect()[..] == key)
        {
            Some(self.kw_args.remove(pos).expr)
        } else {
            None
        }
    }

    pub fn get_left_or_key(&self, key: &str) -> Option<&Expr> {
        if !self.pos_args.is_empty() {
            Some(&self.pos_args.get(0)?.expr)
        } else if let Some(pos) = self
            .kw_args
            .iter()
            .position(|arg| &arg.keyword.inspect()[..] == key)
        {
            Some(&self.kw_args.get(pos)?.expr)
        } else {
            None
        }
    }

    pub fn get_mut_left_or_key(&mut self, key: &str) -> Option<&mut Expr> {
        if !self.pos_args.is_empty() {
            Some(&mut self.pos_args.get_mut(0)?.expr)
        } else if let Some(pos) = self
            .kw_args
            .iter()
            .position(|arg| &arg.keyword.inspect()[..] == key)
        {
            Some(&mut self.kw_args.get_mut(pos)?.expr)
        } else {
            None
        }
    }

    pub fn insert_pos(&mut self, idx: usize, pos: PosArg) {
        self.pos_args.insert(idx, pos);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub dot: Option<Token>,
    pub name: VarName,
    pub qual_name: Option<Str>,
    pub vi: VarInfo,
}

impl NestedDisplay for Identifier {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match &self.dot {
            Some(_dot) => {
                write!(f, ".{}", self.name)?;
            }
            None => {
                write!(f, "::{}", self.name)?;
            }
        }
        if let Some(qn) = &self.qual_name {
            write!(f, "(qual_name: {})", qn)?;
        }
        if self.vi.t != Type::Uninited {
            write!(f, "(: {})", self.vi.t)?;
        }
        Ok(())
    }
}

impl_display_from_nested!(Identifier);

impl HasType for Identifier {
    #[inline]
    fn ref_t(&self) -> &Type {
        &self.vi.t
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        &mut self.vi.t
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
    pub const fn new(
        dot: Option<Token>,
        name: VarName,
        qual_name: Option<Str>,
        vi: VarInfo,
    ) -> Self {
        Self {
            dot,
            name,
            qual_name,
            vi,
        }
    }

    pub fn public(name: &'static str) -> Self {
        Self::bare(
            Some(Token::from_str(TokenKind::Dot, ".")),
            VarName::from_static(name),
        )
    }

    pub fn private(name: &'static str) -> Self {
        Self::bare(None, VarName::from_static(name))
    }

    pub fn private_with_line(name: Str, line: usize) -> Self {
        Self::bare(None, VarName::from_str_and_line(name, line))
    }

    pub fn public_with_line(dot: Token, name: Str, line: usize) -> Self {
        Self::bare(Some(dot), VarName::from_str_and_line(name, line))
    }

    pub const fn bare(dot: Option<Token>, name: VarName) -> Self {
        Self::new(dot, name, None, VarInfo::const_default())
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

    pub fn downcast(self) -> erg_parser::ast::Identifier {
        erg_parser::ast::Identifier::new(self.dot, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    pub obj: Box<Expr>,
    pub ident: Identifier,
}

impl NestedDisplay for Attribute {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "({}){}", self.obj, self.ident)
    }
}

impl_display_from_nested!(Attribute);
impl_locational!(Attribute, obj, ident);

impl HasType for Attribute {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.ident.ref_t()
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        self.ident.ref_mut_t()
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        self.ident.signature_t()
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        self.ident.signature_mut_t()
    }
}

impl Attribute {
    pub fn new(obj: Expr, ident: Identifier) -> Self {
        Self {
            obj: Box::new(obj),
            ident,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Accessor {
    Ident(Identifier),
    Attr(Attribute),
}

impl_nested_display_for_enum!(Accessor; Ident, Attr);
impl_display_from_nested!(Accessor);
impl_locational_for_enum!(Accessor; Ident, Attr);
impl_t_for_enum!(Accessor; Ident, Attr);

impl Accessor {
    pub fn private_with_line(name: Str, line: usize) -> Self {
        Self::Ident(Identifier::private_with_line(name, line))
    }

    pub fn public_with_line(name: Str, line: usize) -> Self {
        Self::Ident(Identifier::public_with_line(DOT, name, line))
    }

    pub const fn private(name: Token, vi: VarInfo) -> Self {
        Self::Ident(Identifier::new(None, VarName::new(name), None, vi))
    }

    pub fn public(name: Token, vi: VarInfo) -> Self {
        Self::Ident(Identifier::new(Some(DOT), VarName::new(name), None, vi))
    }

    pub fn attr(obj: Expr, ident: Identifier) -> Self {
        Self::Attr(Attribute::new(obj, ident))
    }

    pub fn show(&self) -> String {
        match self {
            Self::Ident(ident) => readable_name(ident.inspect()).to_string(),
            Self::Attr(attr) => {
                attr.obj
                    .show_acc()
                    .unwrap_or_else(|| attr.obj.ref_t().to_string())
                    + "." // TODO: visibility
                    + readable_name(attr.ident.inspect())
            }
        }
    }

    // 参照するオブジェクト自体が持っている固有の名前(クラス、モジュールなど)
    pub fn qual_name(&self) -> Option<&str> {
        match self {
            Self::Ident(ident) => ident.qual_name.as_ref().map(|s| &s[..]),
            _ => None,
        }
    }

    pub fn local_name(&self) -> Option<&str> {
        match self {
            Self::Ident(ident) => ident
                .qual_name
                .as_ref()
                .map(|s| {
                    let mut seps = s.split_with(&[".", "::"]);
                    seps.remove(seps.len() - 1)
                })
                .or_else(|| {
                    let mut raw_parts = ident.name.inspect().split_with(&["'"]);
                    // "'aaa'".split_with(&["'"]) == ["", "aaa", ""]
                    if raw_parts.len() == 3 || raw_parts.len() == 4 {
                        Some(raw_parts.remove(1))
                    } else {
                        Some(ident.name.inspect())
                    }
                }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayWithLength {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub t: Type,
    pub elem: Box<Expr>,
    pub len: Box<Expr>,
}

impl NestedDisplay for ArrayWithLength {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "[{}; {}](: {})", self.elem, self.len, self.t)
    }
}

impl_display_from_nested!(ArrayWithLength);
impl_locational!(ArrayWithLength, l_sqbr, r_sqbr);
impl_t!(ArrayWithLength);

impl ArrayWithLength {
    pub fn new(l_sqbr: Token, r_sqbr: Token, t: Type, elem: Expr, len: Expr) -> Self {
        Self {
            l_sqbr,
            r_sqbr,
            t,
            elem: Box::new(elem),
            len: Box::new(len),
        }
    }
}

// TODO: generators
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayComprehension {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub t: Type,
    pub elem: Box<Expr>,
    pub guard: Box<Expr>,
}

impl NestedDisplay for ArrayComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "[{} | {}](: {})", self.elem, self.guard, self.t)
    }
}

impl_display_from_nested!(ArrayComprehension);
impl_locational!(ArrayComprehension, l_sqbr, r_sqbr);
impl_t!(ArrayComprehension);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalArray {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub t: Type,
    pub elems: Args,
}

impl NestedDisplay for NormalArray {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "[")?;
        self.elems.fmt_nest(f, level + 1)?;
        write!(f, "\n{}](: {})", "    ".repeat(level), self.t)
    }
}

impl_display_from_nested!(NormalArray);
impl_locational!(NormalArray, l_sqbr, r_sqbr);
impl_t!(NormalArray);

impl NormalArray {
    pub fn new(l_sqbr: Token, r_sqbr: Token, elem_t: Type, elems: Args) -> Self {
        let t = array_t(elem_t, TyParam::value(elems.len()));
        Self {
            l_sqbr,
            r_sqbr,
            t,
            elems,
        }
    }

    pub fn push(&mut self, elem: Expr) {
        self.elems.push_pos(PosArg::new(elem));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Array {
    Normal(NormalArray),
    Comprehension(ArrayComprehension),
    WithLength(ArrayWithLength),
}

impl_nested_display_for_enum!(Array; Normal, Comprehension, WithLength);
impl_display_for_enum!(Array; Normal, Comprehension, WithLength);
impl_locational_for_enum!(Array; Normal, Comprehension, WithLength);
impl_t_for_enum!(Array; Normal, Comprehension, WithLength);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalTuple {
    pub elems: Args,
    t: Type,
}

impl NestedDisplay for NormalTuple {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "(")?;
        self.elems.fmt_nest(f, level + 1)?;
        write!(f, "\n{})(: {})", "    ".repeat(level), self.t)
    }
}

impl_display_from_nested!(NormalTuple);
impl_locational!(NormalTuple, elems);
impl_t!(NormalTuple);

impl NormalTuple {
    pub fn new(elems: Args) -> Self {
        let t = tuple_t(elems.pos_args.iter().map(|a| a.expr.t()).collect());
        Self { elems, t }
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
impl_t_for_enum!(Tuple; Normal);

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalDict {
    pub l_brace: Token,
    pub r_brace: Token,
    pub t: Type,
    pub kvs: Vec<KeyValue>,
}

impl_t!(NormalDict);

impl NestedDisplay for NormalDict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}(: {})", fmt_vec(&self.kvs), self.t)
    }
}

impl_display_from_nested!(NormalDict);
impl_locational!(NormalDict, l_brace, r_brace);

impl NormalDict {
    pub fn new(
        l_brace: Token,
        r_brace: Token,
        kv_ts: HashMap<TyParam, TyParam>,
        kvs: Vec<KeyValue>,
    ) -> Self {
        Self {
            l_brace,
            r_brace,
            t: dict_t(TyParam::Dict(kv_ts)),
            kvs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DictComprehension {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    pub t: Type,
    pub key: Box<Expr>,
    pub value: Box<Expr>,
    pub guard: Box<Expr>,
}

impl NestedDisplay for DictComprehension {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(
            f,
            "[{}: {} | {}](: {})",
            self.key, self.value, self.guard, self.t
        )
    }
}

impl_display_from_nested!(DictComprehension);
impl_locational!(DictComprehension, l_sqbr, r_sqbr);
impl_t!(DictComprehension);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Dict {
    Normal(NormalDict),
    Comprehension(DictComprehension),
}

impl_nested_display_for_enum!(Dict; Normal, Comprehension);
impl_display_for_enum!(Dict; Normal, Comprehension);
impl_locational_for_enum!(Dict; Normal, Comprehension);
impl_t_for_enum!(Dict; Normal, Comprehension);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalSet {
    pub l_brace: Token,
    pub r_brace: Token,
    pub t: Type,
    pub elems: Args,
}

impl NestedDisplay for NormalSet {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{{")?;
        self.elems.fmt_nest(f, level + 1)?;
        write!(f, "\n{}}}(: {})", "    ".repeat(level), self.t)
    }
}

impl_display_from_nested!(NormalSet);
impl_locational!(NormalSet, l_brace, r_brace);
impl_t!(NormalSet);

impl NormalSet {
    pub fn new(l_brace: Token, r_brace: Token, elem_t: Type, elems: Args) -> Self {
        let t = set_t(elem_t, TyParam::value(elems.len()));
        Self {
            l_brace,
            r_brace,
            t,
            elems,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetWithLength {
    pub l_brace: Token,
    pub r_brace: Token,
    pub t: Type,
    pub elem: Box<Expr>,
    pub len: Box<Expr>,
}

impl NestedDisplay for SetWithLength {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "[{}; {}](: {})", self.elem, self.len, self.t)
    }
}

impl_display_from_nested!(SetWithLength);
impl_locational!(SetWithLength, l_brace, r_brace);
impl_t!(SetWithLength);

impl SetWithLength {
    pub fn new(l_brace: Token, r_brace: Token, t: Type, elem: Expr, len: Expr) -> Self {
        Self {
            l_brace,
            r_brace,
            t,
            elem: Box::new(elem),
            len: Box::new(len),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Set {
    Normal(NormalSet),
    WithLength(SetWithLength),
}

impl_nested_display_for_enum!(Set; Normal, WithLength);
impl_display_for_enum!(Set; Normal, WithLength);
impl_locational_for_enum!(Set; Normal, WithLength);
impl_t_for_enum!(Set; Normal, WithLength);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordAttrs(Vec<Def>);

impl NestedDisplay for RecordAttrs {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)
    }
}

impl_display_from_nested!(RecordAttrs);
impl_stream_for_wrapper!(RecordAttrs, Def);

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
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Def> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Def> {
        self.0.iter_mut()
    }

    pub fn push(&mut self, attr: Def) {
        self.0.push(attr);
    }

    pub fn extend(&mut self, attrs: RecordAttrs) {
        self.0.extend(attrs.0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Record {
    l_brace: Token,
    r_brace: Token,
    pub attrs: RecordAttrs,
    t: Type,
}

impl NestedDisplay for Record {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{{")?;
        self.attrs.fmt_nest(f, level + 1)?;
        writeln!(f, "\n{}}}", "    ".repeat(level))
    }
}

impl_display_from_nested!(Record);
impl_locational!(Record, l_brace, r_brace);
impl_t!(Record);

impl Record {
    pub fn new(l_brace: Token, r_brace: Token, attrs: RecordAttrs) -> Self {
        let rec = attrs
            .iter()
            .map(|def| (Field::from(def.sig.ident()), def.body.block.t()))
            .collect();
        let t = Type::Record(rec);
        Self {
            l_brace,
            r_brace,
            attrs,
            t,
        }
    }

    pub fn push(&mut self, attr: Def) {
        let t = enum_unwrap!(&mut self.t, Type::Record);
        t.insert(Field::from(attr.sig.ident()), attr.body.block.t());
        self.attrs.push(attr);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinOp {
    pub op: Token,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
    pub info: VarInfo, // e.g. (Int, Int) -> Int
}

impl NestedDisplay for BinOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "`{}`(: {}):", self.op.content, self.info.t)?;
        self.lhs.fmt_nest(f, level + 1)?;
        writeln!(f)?;
        self.rhs.fmt_nest(f, level + 1)
    }
}

impl HasType for BinOp {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.info.t.return_t().unwrap()
    }
    fn ref_mut_t(&mut self) -> &mut Type {
        self.info.t.mut_return_t().unwrap()
    }
    #[inline]
    fn lhs_t(&self) -> &Type {
        self.info.t.lhs_t()
    }
    #[inline]
    fn rhs_t(&self) -> &Type {
        self.info.t.rhs_t()
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        Some(&self.info.t)
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        Some(&mut self.info.t)
    }
}

impl_display_from_nested!(BinOp);
impl_locational!(BinOp, lhs, rhs);

impl BinOp {
    pub fn new(op: Token, lhs: Expr, rhs: Expr, info: VarInfo) -> Self {
        Self {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            info,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnaryOp {
    pub op: Token,
    pub expr: Box<Expr>,
    pub info: VarInfo, // e.g. Neg -> Nat
}

impl HasType for UnaryOp {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.info.t.return_t().unwrap()
    }
    fn ref_mut_t(&mut self) -> &mut Type {
        self.info.t.mut_return_t().unwrap()
    }
    #[inline]
    fn lhs_t(&self) -> &Type {
        self.expr.ref_t()
    }
    #[inline]
    fn rhs_t(&self) -> &Type {
        panic!("invalid operation")
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        Some(&self.info.t)
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        Some(&mut self.info.t)
    }
}

impl NestedDisplay for UnaryOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "`{}`(: {}):", self.op, self.info.t)?;
        self.expr.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(UnaryOp);
impl_locational!(UnaryOp, op, expr);

impl UnaryOp {
    pub fn new(op: Token, expr: Expr, info: VarInfo) -> Self {
        Self {
            op,
            expr: Box::new(expr),
            info,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Call {
    pub obj: Box<Expr>,
    pub attr_name: Option<Identifier>,
    pub args: Args,
}

impl NestedDisplay for Call {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        writeln!(f, "({}){}:", self.obj, fmt_option!(self.attr_name),)?;
        self.args.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Call);

impl HasType for Call {
    #[inline]
    fn ref_t(&self) -> &Type {
        if let Some(attr) = self.attr_name.as_ref() {
            attr.ref_t().return_t().unwrap()
        } else {
            self.obj.ref_t().return_t().unwrap()
        }
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        if let Some(attr) = self.attr_name.as_mut() {
            attr.ref_mut_t().mut_return_t().unwrap()
        } else {
            self.obj.ref_mut_t().mut_return_t().unwrap()
        }
    }
    #[inline]
    fn lhs_t(&self) -> &Type {
        if let Some(attr) = self.attr_name.as_ref() {
            attr.ref_t().lhs_t()
        } else {
            self.obj.lhs_t()
        }
    }
    #[inline]
    fn rhs_t(&self) -> &Type {
        if let Some(attr) = self.attr_name.as_ref() {
            attr.ref_t().rhs_t()
        } else {
            self.obj.rhs_t()
        }
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        if let Some(attr) = self.attr_name.as_ref() {
            Some(attr.ref_t())
        } else {
            Some(self.obj.ref_t())
        }
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        if let Some(attr) = self.attr_name.as_mut() {
            Some(attr.ref_mut_t())
        } else {
            Some(self.obj.ref_mut_t())
        }
    }
}

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

    pub fn is_method_call(&self) -> bool {
        self.signature_t()
            .map(|t| t.self_t().is_some())
            .unwrap_or(false)
    }

    pub fn additional_operation(&self) -> Option<OperationKind> {
        self.obj.show_acc().and_then(|s| match &s[..] {
            "import" => Some(OperationKind::Import),
            "pyimport" | "py" => Some(OperationKind::PyImport),
            "Del" => Some(OperationKind::Del),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block(Vec<Expr>);

impl HasType for Block {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.last().unwrap().ref_t()
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        self.last_mut().unwrap().ref_mut_t()
    }
    #[inline]
    fn t(&self) -> Type {
        self.last().unwrap().t()
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        self.last().unwrap().signature_t()
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        self.last_mut().unwrap().signature_mut_t()
    }
}

impl NestedDisplay for Block {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        fmt_lines(self.0.iter(), f, level)
    }
}

impl_display_from_nested!(Block);
impl_stream_for_wrapper!(Block, Expr);

impl Locational for Block {
    fn loc(&self) -> Location {
        Location::concat(self.0.first().unwrap(), self.0.last().unwrap())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarSignature {
    pub ident: Identifier,
}

impl NestedDisplay for VarSignature {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", self.ident)
    }
}

impl_display_from_nested!(VarSignature);
impl_locational!(VarSignature, ident);

impl HasType for VarSignature {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.ident.ref_t()
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        self.ident.ref_mut_t()
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        self.ident.signature_t()
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        self.ident.signature_mut_t()
    }
}

impl VarSignature {
    pub const fn new(ident: Identifier) -> Self {
        Self { ident }
    }

    pub fn inspect(&self) -> &Str {
        self.ident.inspect()
    }

    pub fn vis(&self) -> Visibility {
        self.ident.vis()
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
    pub var_args: Option<Box<NonDefaultParamSignature>>,
    pub defaults: Vec<DefaultParamSignature>,
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
    Vec<NonDefaultParamSignature>,
    Option<Box<NonDefaultParamSignature>>,
    Vec<DefaultParamSignature>,
    Option<(Token, Token)>,
);

impl Params {
    pub const fn new(
        non_defaults: Vec<NonDefaultParamSignature>,
        var_args: Option<Box<NonDefaultParamSignature>>,
        defaults: Vec<DefaultParamSignature>,
        parens: Option<(Token, Token)>,
    ) -> Self {
        Self {
            non_defaults,
            var_args,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubrSignature {
    pub ident: Identifier,
    pub params: Params,
    pub t: Type,
}

impl NestedDisplay for SubrSignature {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}{} (: {})", self.ident, self.params, self.t,)
    }
}

impl_display_from_nested!(SubrSignature);
impl_locational!(SubrSignature, ident, params);
impl_t!(SubrSignature);

impl SubrSignature {
    pub const fn new(ident: Identifier, params: Params, t: Type) -> Self {
        Self { ident, params, t }
    }

    pub fn is_procedural(&self) -> bool {
        self.ident.is_procedural()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lambda {
    pub params: Params,
    op: Token,
    pub body: Block,
    pub id: usize,
    pub t: Type,
}

impl NestedDisplay for Lambda {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{} {} (: {})", self.params, self.op.content, self.t)?;
        self.body.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Lambda);
impl_locational!(Lambda, params, body);
impl_t!(Lambda);

impl Lambda {
    pub const fn new(id: usize, params: Params, op: Token, body: Block, t: Type) -> Self {
        Self {
            id,
            params,
            op,
            body,
            t,
        }
    }

    pub fn is_procedural(&self) -> bool {
        self.op.is(TokenKind::ProcArrow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Signature {
    Var(VarSignature),
    Subr(SubrSignature),
}

impl_nested_display_for_chunk_enum!(Signature; Var, Subr);
impl_display_for_enum!(Signature; Var, Subr,);
impl_t_for_enum!(Signature; Var, Subr);
impl_locational_for_enum!(Signature; Var, Subr,);

impl Signature {
    pub const fn is_subr(&self) -> bool {
        matches!(self, Self::Subr(_))
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Var(v) => v.ident.is_const(),
            Self::Subr(s) => s.ident.is_const(),
        }
    }

    pub fn is_procedural(&self) -> bool {
        match self {
            Self::Var(v) => v.ident.is_procedural(),
            Self::Subr(s) => s.ident.is_procedural(),
        }
    }

    pub const fn vis(&self) -> Visibility {
        match self {
            Self::Var(v) => v.ident.vis(),
            Self::Subr(s) => s.ident.vis(),
        }
    }

    pub const fn ident(&self) -> &Identifier {
        match self {
            Self::Var(v) => &v.ident,
            Self::Subr(s) => &s.ident,
        }
    }

    pub fn ident_mut(&mut self) -> &mut Identifier {
        match self {
            Self::Var(v) => &mut v.ident,
            Self::Subr(s) => &mut s.ident,
        }
    }

    pub fn into_ident(self) -> Identifier {
        match self {
            Self::Var(v) => v.ident,
            Self::Subr(s) => s.ident,
        }
    }
}

/// represents a declaration of a variable
/// necessary for type field declaration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decl {
    pub sig: Signature,
    pub t: Type,
}

impl NestedDisplay for Decl {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}: {}", self.sig, self.t)
    }
}

impl_display_from_nested!(Decl);

impl Locational for Decl {
    #[inline]
    fn loc(&self) -> Location {
        self.sig.loc()
    }
}

impl HasType for Decl {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::NONE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        todo!()
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

impl Decl {
    pub const fn spec_t(&self) -> &Type {
        &self.t
    }

    pub const fn is_sub(&self) -> bool {
        self.sig.is_subr()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        writeln!(f, "{} {}", self.sig, self.body.op.content)?;
        self.body.block.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Def);
impl_locational!(Def, sig, body);

impl HasType for Def {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::NONE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        todo!()
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

impl Def {
    pub const fn new(sig: Signature, body: DefBody) -> Self {
        Self { sig, body }
    }

    pub fn def_kind(&self) -> DefKind {
        match self.body.block.first().unwrap() {
            Expr::Call(call) => match call.obj.show_acc().as_ref().map(|n| &n[..]) {
                Some("Class") => DefKind::Class,
                Some("Inherit") => DefKind::Inherit,
                Some("Trait") => DefKind::Trait,
                Some("Subsume") => DefKind::Subsume,
                Some("Inheritable") => {
                    if let Some(Expr::Call(inner)) = call.args.get_left_or_key("Class") {
                        match inner.obj.show_acc().as_ref().map(|n| &n[..]) {
                            Some("Class") => DefKind::Class,
                            Some("Inherit") => DefKind::Inherit,
                            _ => DefKind::Other,
                        }
                    } else {
                        DefKind::Other
                    }
                }
                Some("import") => DefKind::ErgImport,
                Some("pyimport") | Some("py") => DefKind::PyImport,
                _ => DefKind::Other,
            },
            _ => DefKind::Other,
        }
    }
}

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

impl HasType for Methods {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::NONE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        todo!()
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

impl Methods {
    pub const fn new(class: TypeSpec, vis: Token, defs: RecordAttrs) -> Self {
        Self { class, vis, defs }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassDef {
    pub obj: GenTypeObj,
    pub sig: Signature,
    pub require_or_sup: Box<Expr>,
    /// The type of `new` that is automatically defined if not defined
    pub need_to_gen_new: bool,
    pub __new__: Type,
    pub methods: Block,
}

impl NestedDisplay for ClassDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        self.sig.fmt_nest(f, level)?;
        writeln!(f, ":")?;
        self.methods.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(ClassDef);
impl_locational!(ClassDef, sig);

impl HasType for ClassDef {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::NONE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        todo!()
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

impl ClassDef {
    pub fn new(
        obj: GenTypeObj,
        sig: Signature,
        require_or_sup: Expr,
        need_to_gen_new: bool,
        __new__: Type,
        methods: Block,
    ) -> Self {
        Self {
            obj,
            sig,
            require_or_sup: Box::new(require_or_sup),
            need_to_gen_new,
            __new__,
            methods,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttrDef {
    pub attr: Accessor,
    pub block: Block,
}

impl NestedDisplay for AttrDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        self.attr.fmt_nest(f, level)?;
        writeln!(f, " = ")?;
        self.block.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(AttrDef);
impl_locational!(AttrDef, attr, block);

impl HasType for AttrDef {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::NONE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        todo!()
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

impl AttrDef {
    pub const fn new(attr: Accessor, block: Block) -> Self {
        Self { attr, block }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeAscription {
    pub expr: Box<Expr>,
    pub spec: TypeSpec,
}

impl NestedDisplay for TypeAscription {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        writeln!(f, "{}: {}", self.expr, self.spec)
    }
}

impl_display_from_nested!(TypeAscription);
impl_locational!(TypeAscription, expr, spec);

impl HasType for TypeAscription {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.expr.ref_t()
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        self.expr.ref_mut_t()
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        self.expr.signature_t()
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        self.expr.signature_mut_t()
    }
}

impl TypeAscription {
    pub fn new(expr: Expr, spec: TypeSpec) -> Self {
        Self {
            expr: Box::new(expr),
            spec,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Lit(Literal),
    Accessor(Accessor),
    Array(Array),
    Tuple(Tuple),
    Set(Set),
    Dict(Dict),
    Record(Record),
    BinOp(BinOp),
    UnaryOp(UnaryOp),
    Call(Call),
    Lambda(Lambda),
    Def(Def),
    ClassDef(ClassDef),
    AttrDef(AttrDef),
    TypeAsc(TypeAscription),
    Code(Block),     // code object
    Compound(Block), // compound statement
    Import(Accessor),
}

impl_nested_display_for_chunk_enum!(Expr; Lit, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, AttrDef, Code, Compound, TypeAsc, Set, Import);
impl_display_from_nested!(Expr);
impl_locational_for_enum!(Expr; Lit, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, AttrDef, Code, Compound, TypeAsc, Set, Import);
impl_t_for_enum!(Expr; Lit, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, AttrDef, Code, Compound, TypeAsc, Set, Import);

impl Default for Expr {
    fn default() -> Self {
        Self::Code(Block::default())
    }
}

impl Expr {
    pub fn receiver_t(&self) -> Option<&Type> {
        match self {
            Self::Accessor(Accessor::Attr(attr)) => Some(attr.obj.ref_t()),
            _other => None,
        }
    }

    pub fn show_acc(&self) -> Option<String> {
        match self {
            Expr::Accessor(acc) => Some(acc.show()),
            _ => None,
        }
    }

    /// 参照するオブジェクト自体が持っている名前(e.g. Int.qual_name == Some("int"), Socket!.qual_name == Some("io.Socket!"))
    pub fn qual_name(&self) -> Option<&str> {
        match self {
            Expr::Accessor(acc) => acc.qual_name(),
            _ => None,
        }
    }

    /// e.g. Int.local_name == Some("int"), Socket!.local_name == Some("Socket!")
    pub fn local_name(&self) -> Option<&str> {
        match self {
            Expr::Accessor(acc) => acc.local_name(),
            _ => None,
        }
    }

    pub fn is_type_asc(&self) -> bool {
        matches!(self, Expr::TypeAsc(_))
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

    pub fn attr(self, ident: Identifier) -> Accessor {
        Accessor::attr(self, ident)
    }

    pub fn attr_expr(self, ident: Identifier) -> Self {
        Self::Accessor(self.attr(ident))
    }

    pub fn type_asc(self, t_spec: TypeSpec) -> TypeAscription {
        TypeAscription::new(self, t_spec)
    }

    pub fn type_asc_expr(self, t_spec: TypeSpec) -> Self {
        Self::TypeAsc(self.type_asc(t_spec))
    }
}

/// Toplevel grammar unit
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Module(Vec<Expr>);

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

impl_stream_for_wrapper!(Module, Expr);

/// High-level Intermediate Representation
/// AST with type information added
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HIR {
    pub name: Str,
    pub module: Module,
}

impl std::fmt::Display for HIR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.module)
    }
}

impl Default for HIR {
    fn default() -> Self {
        Self {
            name: Str::ever("<module>"),
            module: Module(vec![]),
        }
    }
}

impl HIR {
    pub const fn new(name: Str, module: Module) -> Self {
        Self { name, module }
    }
}
