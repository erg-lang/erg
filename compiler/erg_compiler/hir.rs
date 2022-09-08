/// defines High-level Intermediate Representation
use std::fmt;

use erg_common::error::Location;
use erg_common::traits::{Locational, NestedDisplay, Stream};
use erg_common::vis::{Field, Visibility};
use erg_common::Str;
use erg_common::{
    enum_unwrap, fmt_option, impl_display_for_enum, impl_display_from_nested, impl_locational,
    impl_locational_for_enum, impl_nested_display_for_chunk_enum, impl_nested_display_for_enum,
    impl_stream_for_wrapper,
};

use erg_parser::ast::{fmt_lines, DefId, Identifier, Params, TypeSpec};
use erg_parser::token::{Token, TokenKind};

use erg_type::constructors::{array, tuple};
use erg_type::typaram::TyParam;
use erg_type::value::{TypeKind, ValueObj};
use erg_type::{impl_t, impl_t_for_enum, HasType, Type};

use crate::context::eval::type_from_token_kind;
use crate::error::readable_name;

#[derive(Debug, Clone)]
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

impl From<Token> for Literal {
    fn from(token: Token) -> Self {
        let data = ValueObj::from_str(type_from_token_kind(token.kind), token.content.clone());
        Self {
            t: data.t(),
            value: data,
            token,
        }
    }
}

impl Literal {
    #[inline]
    pub fn is(&self, kind: TokenKind) -> bool {
        self.token.is(kind)
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Args {
    pub pos_args: Vec<PosArg>,
    pub kw_args: Vec<KwArg>,
    paren: Option<(Token, Token)>,
}

impl NestedDisplay for Args {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        if !self.pos_args.is_empty() {
            fmt_lines(self.pos_args.iter(), f, level)?;
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

    #[inline]
    pub fn len(&self) -> usize {
        self.pos_args.len() + self.kw_args.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos_args.is_empty() && self.kw_args.is_empty()
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
        } else {
            if let Some(pos) = self
                .kw_args
                .iter()
                .position(|arg| &arg.keyword.inspect()[..] == key)
            {
                Some(self.kw_args.remove(pos).expr)
            } else {
                None
            }
        }
    }
}

/// represents local variables
#[derive(Debug, Clone)]
pub struct Local {
    pub name: Token,
    /// オブジェクト自身の名前
    __name__: Option<Str>,
    pub(crate) t: Type,
}

impl NestedDisplay for Local {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        let __name__ = if let Some(__name__) = self.__name__() {
            format!("(__name__ = {__name__})")
        } else {
            "".to_string()
        };
        write!(f, "{} (: {}){}", self.name.content, self.t, __name__)
    }
}

impl_display_from_nested!(Local);
impl_t!(Local);

impl Locational for Local {
    #[inline]
    fn loc(&self) -> Location {
        self.name.loc()
    }
}

impl Local {
    pub const fn new(name: Token, __name__: Option<Str>, t: Type) -> Self {
        Self { name, __name__, t }
    }

    // &strにするとクローンしたいときにアロケーションコストがかかるので&Strのままで
    #[inline]
    pub fn inspect(&self) -> &Str {
        &self.name.content
    }

    pub const fn __name__(&self) -> Option<&Str> {
        self.__name__.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct Public {
    pub dot: Token,
    pub name: Token,
    /// オブジェクト自身の名前
    __name__: Option<Str>,
    t: Type,
}

impl NestedDisplay for Public {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        let __name__ = if let Some(__name__) = self.__name__() {
            format!("(__name__ = {__name__})")
        } else {
            "".to_string()
        };
        write!(f, ".{} (: {}){}", self.name.content, self.t, __name__)
    }
}

impl_display_from_nested!(Public);
impl_t!(Public);

impl Locational for Public {
    #[inline]
    fn loc(&self) -> Location {
        Location::concat(&self.dot, &self.name)
    }
}

impl Public {
    pub const fn new(dot: Token, name: Token, __name__: Option<Str>, t: Type) -> Self {
        Self {
            dot,
            name,
            __name__,
            t,
        }
    }

    // &strにするとクローンしたいときにアロケーションコストがかかるので&Strのままで
    #[inline]
    pub fn inspect(&self) -> &Str {
        &self.name.content
    }

    pub const fn __name__(&self) -> Option<&Str> {
        self.__name__.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub obj: Box<Expr>,
    pub name: Token,
    t: Type,
}

impl NestedDisplay for Attribute {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if self.t != Type::Uninited {
            write!(f, "({}).{}(: {})", self.obj, self.name.content, self.t)
        } else {
            write!(f, "({}).{}", self.obj, self.name.content)
        }
    }
}

impl_display_from_nested!(Attribute);
impl_locational!(Attribute, obj, name);
impl_t!(Attribute);

impl Attribute {
    pub fn new(obj: Expr, name: Token, t: Type) -> Self {
        Self {
            obj: Box::new(obj),
            name,
            t,
        }
    }
}

/// e.g. obj.0, obj.1
#[derive(Clone, Debug)]
pub struct TupleAttribute {
    pub obj: Box<Expr>,
    pub index: Literal,
    t: Type,
}

impl NestedDisplay for TupleAttribute {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "({}).{}", self.obj, self.index)
    }
}

impl_display_from_nested!(TupleAttribute);
impl_locational!(TupleAttribute, obj, index);
impl_t!(TupleAttribute);

impl TupleAttribute {
    pub fn new(obj: Expr, index: Literal, t: Type) -> Self {
        Self {
            obj: Box::new(obj),
            index,
            t,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Subscript {
    pub(crate) obj: Box<Expr>,
    pub(crate) index: Box<Expr>,
    t: Type,
}

impl NestedDisplay for Subscript {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "({})[{}](: {})", self.obj, self.index, self.t)
    }
}

impl_display_from_nested!(Subscript);
impl_locational!(Subscript, obj, index);
impl_t!(Subscript);

impl Subscript {
    pub fn new(obj: Expr, index: Expr, t: Type) -> Self {
        Self {
            obj: Box::new(obj),
            index: Box::new(index),
            t,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Accessor {
    Local(Local),
    Public(Public),
    Attr(Attribute),
    TupleAttr(TupleAttribute),
    Subscr(Subscript),
}

impl_nested_display_for_enum!(Accessor; Local, Public, Attr, TupleAttr, Subscr);
impl_display_from_nested!(Accessor);
impl_locational_for_enum!(Accessor; Local, Public, Attr, TupleAttr, Subscr);
impl_t_for_enum!(Accessor; Local, Public, Attr, TupleAttr, Subscr);

impl Accessor {
    pub const fn local(symbol: Token, t: Type) -> Self {
        Self::Local(Local::new(symbol, None, t))
    }

    pub const fn public(dot: Token, name: Token, t: Type) -> Self {
        Self::Public(Public::new(dot, name, None, t))
    }

    pub fn attr(obj: Expr, name: Token, t: Type) -> Self {
        Self::Attr(Attribute::new(obj, name, t))
    }

    pub fn subscr(obj: Expr, index: Expr, t: Type) -> Self {
        Self::Subscr(Subscript::new(obj, index, t))
    }

    pub fn var_full_name(&self) -> Option<String> {
        match self {
            Self::Local(local) => Some(readable_name(local.inspect()).to_string()),
            Self::Attr(attr) => attr
                .obj
                .var_full_name()
                .map(|n| n + "." + readable_name(attr.name.inspect())),
            Self::TupleAttr(t_attr) => t_attr
                .obj
                .var_full_name()
                .map(|n| n + "." + t_attr.index.token.inspect()),
            Self::Subscr(_) | Self::Public(_) => todo!(),
        }
    }

    // 参照するオブジェクト自体が持っている固有の名前
    pub fn __name__(&self) -> Option<&str> {
        match self {
            Self::Local(local) => local.__name__().map(|s| &s[..]),
            Self::Public(public) => public.__name__().map(|s| &s[..]),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
        let t = array(elem_t, TyParam::value(elems.len()));
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

#[derive(Debug, Clone)]
pub enum Array {
    Normal(NormalArray),
    Comprehension(ArrayComprehension),
    WithLength(ArrayWithLength),
}

impl_nested_display_for_enum!(Array; Normal, Comprehension, WithLength);
impl_display_for_enum!(Array; Normal, Comprehension, WithLength);
impl_locational_for_enum!(Array; Normal, Comprehension, WithLength);
impl_t_for_enum!(Array; Normal, Comprehension, WithLength);

#[derive(Debug, Clone)]
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
        let t = tuple(elems.pos_args.iter().map(|a| a.expr.t()).collect());
        Self { elems, t }
    }
}

#[derive(Debug, Clone)]
pub enum Tuple {
    Normal(NormalTuple),
    // Comprehension(TupleComprehension),
}

impl_nested_display_for_enum!(Tuple; Normal);
impl_display_for_enum!(Tuple; Normal);
impl_locational_for_enum!(Tuple; Normal);
impl_t_for_enum!(Tuple; Normal);

#[derive(Debug, Clone)]
pub struct NormalDict {
    pub l_brace: Token,
    pub r_brace: Token,
    pub t: Type,
    pub attrs: Args, // TODO: keyをTokenではなくExprにする
}

impl_t!(NormalDict);

impl NestedDisplay for NormalDict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}(: {})", self.attrs, self.t)
    }
}

impl_display_from_nested!(NormalDict);
impl_locational!(NormalDict, l_brace, r_brace);

impl NormalDict {
    pub const fn new(l_brace: Token, r_brace: Token, t: Type, attrs: Args) -> Self {
        Self {
            l_brace,
            r_brace,
            t,
            attrs,
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum Dict {
    Normal(NormalDict),
    Comprehension(DictComprehension),
}

impl_nested_display_for_enum!(Dict; Normal, Comprehension);
impl_display_for_enum!(Dict; Normal, Comprehension);
impl_locational_for_enum!(Dict; Normal, Comprehension);
impl_t_for_enum!(Dict; Normal, Comprehension);

#[derive(Debug, Clone)]
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

    pub fn iter(&self) -> impl Iterator<Item = &Def> {
        self.0.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = Def> {
        self.0.into_iter()
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

#[derive(Clone, Debug)]
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

#[derive(Debug, Clone)]
pub struct BinOp {
    pub op: Token,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
    pub sig_t: Type, // e.g. (Int, Int) -> Int
}

impl NestedDisplay for BinOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "`{}`(: {}):\n", self.op.content, self.sig_t)?;
        self.lhs.fmt_nest(f, level + 1)?;
        writeln!(f)?;
        self.rhs.fmt_nest(f, level + 1)
    }
}

impl HasType for BinOp {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.sig_t.return_t().unwrap()
    }
    fn ref_mut_t(&mut self) -> &mut Type {
        self.sig_t.mut_return_t().unwrap()
    }
    #[inline]
    fn lhs_t(&self) -> &Type {
        self.sig_t.lhs_t()
    }
    #[inline]
    fn rhs_t(&self) -> &Type {
        self.sig_t.rhs_t()
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        Some(&self.sig_t)
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        Some(&mut self.sig_t)
    }
}

impl_display_from_nested!(BinOp);
impl_locational!(BinOp, lhs, rhs);

impl BinOp {
    pub fn new(op: Token, lhs: Expr, rhs: Expr, sig_t: Type) -> Self {
        Self {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            sig_t,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnaryOp {
    pub op: Token,
    pub expr: Box<Expr>,
    pub sig_t: Type, // e.g. Neg -> Nat
}

impl HasType for UnaryOp {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.sig_t.return_t().unwrap()
    }
    fn ref_mut_t(&mut self) -> &mut Type {
        self.sig_t.mut_return_t().unwrap()
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
        Some(&self.sig_t)
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        Some(&mut self.sig_t)
    }
}

impl NestedDisplay for UnaryOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "`{}`(: {}):", self.op, self.sig_t)?;
        self.expr.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(UnaryOp);
impl_locational!(UnaryOp, op, expr);

impl UnaryOp {
    pub fn new(op: Token, expr: Expr, sig_t: Type) -> Self {
        Self {
            op,
            expr: Box::new(expr),
            sig_t,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Call {
    pub obj: Box<Expr>,
    pub method_name: Option<Token>,
    pub args: Args,
    /// 全体の型(引数自体の型は関係ない)、e.g. `abs(-1)` -> `Neg -> Nat`
    pub sig_t: Type,
}

impl NestedDisplay for Call {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        writeln!(
            f,
            "({}){} (: {}):",
            self.obj,
            fmt_option!(pre ".", self.method_name.as_ref().map(|t| t.inspect())),
            self.sig_t
        )?;
        self.args.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Call);

impl HasType for Call {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.sig_t.return_t().unwrap()
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        self.sig_t.mut_return_t().unwrap()
    }
    #[inline]
    fn lhs_t(&self) -> &Type {
        self.sig_t.lhs_t()
    }
    #[inline]
    fn rhs_t(&self) -> &Type {
        self.sig_t.rhs_t()
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        Some(&self.sig_t)
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        Some(&mut self.sig_t)
    }
}

impl Locational for Call {
    fn loc(&self) -> Location {
        Location::concat(self.obj.as_ref(), &self.args)
    }
}

impl Call {
    pub fn new(obj: Expr, method_name: Option<Token>, args: Args, sig_t: Type) -> Self {
        Self {
            obj: Box::new(obj),
            method_name,
            args,
            sig_t,
        }
    }

    pub fn is_import_call(&self) -> bool {
        self.obj
            .var_full_name()
            .map(|s| &s[..] == "import" || &s[..] == "pyimport" || &s[..] == "py")
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Hash)]
pub struct VarSignature {
    pub ident: Identifier,
    pub t: Type,
}

impl NestedDisplay for VarSignature {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}(: {})", self.ident, self.t)
    }
}

impl_display_from_nested!(VarSignature);
impl_locational!(VarSignature, ident);
impl_t!(VarSignature);

impl VarSignature {
    pub const fn new(ident: Identifier, t: Type) -> Self {
        Self { ident, t }
    }

    pub fn inspect(&self) -> &Str {
        self.ident.inspect()
    }

    pub fn vis(&self) -> Visibility {
        self.ident.vis()
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

    pub fn ident(&self) -> &Identifier {
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
}

/// represents a declaration of a variable
/// necessary for type field declaration
#[derive(Debug, Clone)]
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

#[derive(Clone, Debug)]
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

    pub fn is_type(&self) -> bool {
        match self.block.first().unwrap() {
            Expr::Call(call) => {
                if let Expr::Accessor(Accessor::Local(local)) = call.obj.as_ref() {
                    &local.inspect()[..] == "Type"
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub kind: TypeKind,
    pub sig: Signature,
    pub require_or_sup: Box<Expr>,
    /// The type of `new` and `__new__` that is automatically defined if not defined
    pub need_to_gen_new: bool,
    pub __new__: Type,
    pub private_methods: RecordAttrs,
    pub public_methods: RecordAttrs,
}

impl NestedDisplay for ClassDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        self.sig.fmt_nest(f, level)?;
        writeln!(f, ":")?;
        self.private_methods.fmt_nest(f, level + 1)?;
        self.public_methods.fmt_nest(f, level + 1)
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
        kind: TypeKind,
        sig: Signature,
        require_or_sup: Expr,
        need_to_gen_new: bool,
        __new__: Type,
        private_methods: RecordAttrs,
        public_methods: RecordAttrs,
    ) -> Self {
        Self {
            kind,
            sig,
            require_or_sup: Box::new(require_or_sup),
            need_to_gen_new,
            __new__,
            private_methods,
            public_methods,
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Literal),
    Accessor(Accessor),
    Array(Array),
    Tuple(Tuple),
    // Set(Set),
    Dict(Dict),
    Record(Record),
    BinOp(BinOp),
    UnaryOp(UnaryOp),
    Call(Call),
    Lambda(Lambda),
    Decl(Decl),
    Def(Def),
    ClassDef(ClassDef),
    AttrDef(AttrDef),
}

impl_nested_display_for_chunk_enum!(Expr; Lit, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Decl, Def, ClassDef, AttrDef);
impl_display_from_nested!(Expr);
impl_locational_for_enum!(Expr; Lit, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Decl, Def, ClassDef, AttrDef);
impl_t_for_enum!(Expr; Lit, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Decl, Def, ClassDef, AttrDef);

impl Expr {
    pub fn receiver_t(&self) -> Option<&Type> {
        match self {
            Self::Accessor(Accessor::Attr(attr)) => Some(attr.obj.ref_t()),
            _other => None,
        }
    }

    pub fn var_full_name(&self) -> Option<String> {
        match self {
            Expr::Accessor(acc) => acc.var_full_name(),
            _ => None,
        }
    }

    /// 参照するオブジェクト自体が持っている名前(e.g. Int.__name__ == Some("int"))
    pub fn __name__(&self) -> Option<&str> {
        match self {
            Expr::Accessor(acc) => acc.__name__(),
            _ => None,
        }
    }
}

/// Toplevel grammar unit
#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct HIR {
    pub name: Str,
    pub module: Module,
}

impl std::fmt::Display for HIR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.module)
    }
}

impl HIR {
    pub const fn new(name: Str, module: Module) -> Self {
        Self { name, module }
    }
}
