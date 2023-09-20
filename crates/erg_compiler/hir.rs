/// defines High-level Intermediate Representation
use std::fmt;

use erg_common::consts::ERG_MODE;
use erg_common::dict::Dict as HashMap;
use erg_common::error::Location;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::set::Set as HashSet;
use erg_common::traits::{Locational, NestedDisplay, NoTypeDisplay, Stream};
use erg_common::{
    enum_unwrap, fmt_option, fmt_vec, impl_display_for_enum, impl_display_from_nested,
    impl_locational, impl_locational_for_enum, impl_nested_display_for_chunk_enum,
    impl_nested_display_for_enum, impl_no_type_display_for_enum, impl_stream,
};
use erg_common::{impl_from_trait_for_enum, impl_try_from_trait_for_enum, Str};

use erg_parser::ast::{self, AscriptionKind};
use erg_parser::ast::{
    fmt_lines, DefId, DefKind, OperationKind, TypeBoundSpecs, TypeSpec, VarName,
};
use erg_parser::token::{Token, TokenKind, DOT};

use crate::ty::constructors::{dict_t, set_t, tuple_t};
use crate::ty::typaram::TyParam;
use crate::ty::value::{GenTypeObj, ValueObj};
use crate::ty::{Field, HasType, Type, VisibilityModifier};

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

impl NoTypeDisplay for Literal {
    fn to_string_notype(&self) -> String {
        format!("{}", self.token.content)
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

    #[inline]
    pub fn is_doc_comment(&self) -> bool {
        self.token.is(TokenKind::DocComment)
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

impl NoTypeDisplay for PosArg {
    fn to_string_notype(&self) -> String {
        self.expr.to_string_notype()
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

impl NoTypeDisplay for KwArg {
    fn to_string_notype(&self) -> String {
        format!(
            "{} := {}",
            self.keyword.content,
            self.expr.to_string_notype()
        )
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
    pub paren: Option<(Token, Token)>,
}

impl NestedDisplay for Args {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        if !self.pos_args.is_empty() {
            fmt_lines(self.pos_args.iter(), f, level)?;
        }
        if let Some(var_args) = &self.var_args {
            writeln!(f, "*")?;
            var_args.fmt_nest(f, level)?;
        }
        if !self.kw_args.is_empty() {
            fmt_lines(self.kw_args.iter(), f, level)?;
        }
        Ok(())
    }
}

impl NoTypeDisplay for Args {
    fn to_string_notype(&self) -> String {
        let mut s = String::new();
        if !self.pos_args.is_empty() {
            s += &self
                .pos_args
                .iter()
                .map(|x| x.to_string_notype())
                .fold("".to_string(), |acc, s| acc + &s + ", ");
        }
        if let Some(var_args) = &self.var_args {
            s += &format!(", ...{}", var_args.to_string_notype());
        }
        if !self.kw_args.is_empty() {
            s += &self
                .kw_args
                .iter()
                .map(|x| x.to_string_notype())
                .fold("".to_string(), |acc, s| acc + &s + ", ");
        }
        s
    }
}

// do not implement From<Vec<Expr>> to Args, because it will miss paren info (use `values`)

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
            self.pos_args.first(),
            self.var_args.as_ref(),
            self.kw_args.last(),
        ) {
            (Some(l), _, Some(r)) => Location::concat(l, r),
            (Some(l), Some(r), None) => Location::concat(l, r.as_ref()),
            (Some(l), None, None) => Location::concat(l, self.pos_args.last().unwrap()),
            (None, Some(l), Some(r)) => Location::concat(l.as_ref(), r),
            (None, None, Some(r)) => Location::concat(self.kw_args.first().unwrap(), r),
            _ => Location::Unknown,
        }
    }
}

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

    pub fn values(exprs: Vec<Expr>, paren: Option<(Token, Token)>) -> Self {
        Self::pos_only(exprs.into_iter().map(PosArg::new).collect(), paren)
    }

    pub fn single(pos_arg: PosArg) -> Self {
        Self::pos_only(vec![pos_arg], None)
    }

    pub fn pos_only(pos_args: Vec<PosArg>, paren: Option<(Token, Token)>) -> Self {
        Self::new(pos_args, None, vec![], paren)
    }

    pub fn empty() -> Self {
        Self::new(vec![], None, vec![], None)
    }

    #[inline]
    pub fn len(&self) -> usize {
        #[allow(clippy::bool_to_int_with_if)]
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

    pub fn last(&self) -> Option<&Expr> {
        if self.kw_args.is_empty() {
            self.pos_args.last().map(|a| &a.expr)
        } else {
            self.kw_args.last().map(|a| &a.expr)
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

    pub fn get_kw(&self, keyword: &str) -> Option<&Expr> {
        self.kw_args
            .iter()
            .find(|kw| kw.keyword.inspect() == keyword)
            .map(|kw| &kw.expr)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub raw: ast::Identifier,
    pub qual_name: Option<Str>,
    pub vi: VarInfo,
}

impl NestedDisplay for Identifier {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", self.raw)?;
        if let Some(qn) = &self.qual_name {
            write!(f, "(qual_name: {qn})")?;
        }
        if self.vi.t != Type::Uninited {
            write!(f, "(: {})", self.vi.t)?;
        }
        Ok(())
    }
}

impl NoTypeDisplay for Identifier {
    fn to_string_notype(&self) -> String {
        self.raw.to_string()
    }
}

impl_display_from_nested!(Identifier);

impl HasType for Identifier {
    #[inline]
    fn ref_t(&self) -> &Type {
        &self.vi.t
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        Some(&mut self.vi.t)
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
        self.raw.loc()
    }
}

impl From<&Identifier> for Field {
    fn from(ident: &Identifier) -> Self {
        Self::new(ident.vis().clone(), ident.inspect().clone())
    }
}

impl From<Identifier> for Expr {
    fn from(ident: Identifier) -> Self {
        Expr::Accessor(Accessor::Ident(ident))
    }
}

impl Identifier {
    pub const fn new(raw: ast::Identifier, qual_name: Option<Str>, vi: VarInfo) -> Self {
        Self { raw, qual_name, vi }
    }

    pub fn public(name: &'static str) -> Self {
        let ident = ast::Identifier::public_from_token(
            Token::from_str(TokenKind::Dot, "."),
            Token::static_symbol(name),
        );
        Self::bare(ident)
    }

    pub fn private(name: &'static str) -> Self {
        let ident = ast::Identifier::private_from_token(Token::static_symbol(name));
        Self::bare(ident)
    }

    pub fn private_with_line(name: Str, line: u32) -> Self {
        let ident = ast::Identifier::private_from_token(Token::symbol_with_line(&name, line));
        Self::bare(ident)
    }

    pub fn public_with_line(dot: Token, name: Str, line: u32) -> Self {
        let ident = ast::Identifier::public_from_token(dot, Token::symbol_with_line(&name, line));
        Self::bare(ident)
    }

    pub const fn bare(ident: ast::Identifier) -> Self {
        if ident.vis.is_public() {
            Self::new(ident, None, VarInfo::const_default_public())
        } else {
            Self::new(ident, None, VarInfo::const_default_private())
        }
    }

    pub fn is_py_api(&self) -> bool {
        self.vi.py_name.is_some()
    }

    pub fn is_const(&self) -> bool {
        self.raw.is_const()
    }

    pub fn is_discarded(&self) -> bool {
        self.raw.is_discarded()
    }

    pub fn vis(&self) -> &VisibilityModifier {
        &self.vi.vis.modifier
    }

    pub const fn inspect(&self) -> &Str {
        self.raw.inspect()
    }

    /// show dot + name (no qual_name & type)
    pub fn to_string_notype(&self) -> String {
        NoTypeDisplay::to_string_notype(self)
    }

    pub fn is_procedural(&self) -> bool {
        self.raw.is_procedural()
    }

    pub fn downgrade(self) -> ast::Identifier {
        self.raw
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

impl NoTypeDisplay for Attribute {
    fn to_string_notype(&self) -> String {
        format!(
            "({}){}",
            self.obj.to_string_notype(),
            self.ident.to_string_notype()
        )
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
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
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
impl_no_type_display_for_enum!(Accessor; Ident, Attr);
impl_display_from_nested!(Accessor);
impl_locational_for_enum!(Accessor; Ident, Attr);
impl_t_for_enum!(Accessor; Ident, Attr);

impl Accessor {
    pub fn private_with_line(name: Str, line: u32) -> Self {
        Self::Ident(Identifier::private_with_line(name, line))
    }

    pub fn public_with_line(name: Str, line: u32) -> Self {
        Self::Ident(Identifier::public_with_line(DOT, name, line))
    }

    pub fn private(name: Token, vi: VarInfo) -> Self {
        Self::Ident(Identifier::new(
            ast::Identifier::private_from_token(name),
            None,
            vi,
        ))
    }

    pub fn public(name: Token, vi: VarInfo) -> Self {
        Self::Ident(Identifier::new(
            ast::Identifier::public_from_token(DOT, name),
            None,
            vi,
        ))
    }

    pub fn attr(obj: Expr, ident: Identifier) -> Self {
        Self::Attr(Attribute::new(obj, ident))
    }

    pub fn var_info(&self) -> &VarInfo {
        match self {
            Self::Ident(ident) => &ident.vi,
            Self::Attr(attr) => &attr.ident.vi,
        }
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

    pub fn is_py_api(&self) -> bool {
        match self {
            Self::Ident(ident) => ident.is_py_api(),
            Self::Attr(attr) => attr.ident.is_py_api(),
        }
    }

    /// Unique name of the object (class, module, etc.) to which the accessor points
    pub fn qual_name(&self) -> Option<Str> {
        match self {
            Self::Attr(attr) => attr.obj.qual_name().map(|obj_qn| {
                if let Some(id_qn) = attr.ident.qual_name.as_ref() {
                    Str::from(format!("{obj_qn}.{id_qn}"))
                } else {
                    Str::from(format!("{obj_qn}.{}", attr.ident.inspect()))
                }
            }),
            Self::Ident(ident) => ident.qual_name.as_ref().cloned(),
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
                    let mut raw_parts = ident.inspect().split_with(&["'"]);
                    // "'aaa'".split_with(&["'"]) == ["", "aaa", ""]
                    if raw_parts.len() == 3 || raw_parts.len() == 4 {
                        Some(raw_parts.remove(1))
                    } else {
                        Some(ident.inspect())
                    }
                }),
            _ => None,
        }
    }

    pub fn last_name(&self) -> &VarName {
        match self {
            Self::Ident(ident) => &ident.raw.name,
            Self::Attr(attr) => &attr.ident.raw.name,
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

impl NoTypeDisplay for ArrayWithLength {
    fn to_string_notype(&self) -> String {
        format!(
            "[{}; {}]",
            self.elem.to_string_notype(),
            self.len.to_string_notype()
        )
    }
}

impl_display_from_nested!(ArrayWithLength);
impl_locational!(ArrayWithLength, l_sqbr, elem, r_sqbr);
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

impl NoTypeDisplay for ArrayComprehension {
    fn to_string_notype(&self) -> String {
        format!(
            "[{} | {}]",
            self.elem.to_string_notype(),
            self.guard.to_string_notype()
        )
    }
}

impl_display_from_nested!(ArrayComprehension);
impl_locational!(ArrayComprehension, l_sqbr, elem, r_sqbr);
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

impl NoTypeDisplay for NormalArray {
    fn to_string_notype(&self) -> String {
        format!(
            "[{}]",
            self.elems
                .pos_args
                .iter()
                .map(|arg| arg.to_string_notype())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl_display_from_nested!(NormalArray);
impl_locational!(NormalArray, l_sqbr, elems, r_sqbr);
impl_t!(NormalArray);

impl NormalArray {
    pub fn new(l_sqbr: Token, r_sqbr: Token, t: Type, elems: Args) -> Self {
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
impl_no_type_display_for_enum!(Array; Normal, Comprehension, WithLength);
impl_display_for_enum!(Array; Normal, Comprehension, WithLength);
impl_locational_for_enum!(Array; Normal, Comprehension, WithLength);
impl_t_for_enum!(Array; Normal, Comprehension, WithLength);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalTuple {
    pub elems: Args,
    pub(crate) t: Type,
}

impl NestedDisplay for NormalTuple {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "(")?;
        self.elems.fmt_nest(f, level + 1)?;
        write!(f, "\n{})(: {})", "    ".repeat(level), self.t)
    }
}

impl NoTypeDisplay for NormalTuple {
    fn to_string_notype(&self) -> String {
        format!(
            "({})",
            self.elems
                .pos_args
                .iter()
                .map(|arg| arg.to_string_notype())
                .collect::<Vec<_>>()
                .join(", ")
        )
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
impl_no_type_display_for_enum!(Tuple; Normal);
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

impl NoTypeDisplay for KeyValue {
    fn to_string_notype(&self) -> String {
        format!(
            "{}: {}",
            self.key.to_string_notype(),
            self.value.to_string_notype()
        )
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

impl NoTypeDisplay for NormalDict {
    fn to_string_notype(&self) -> String {
        format!(
            "{{{}}}",
            self.kvs
                .iter()
                .map(|kv| kv.to_string_notype())
                .collect::<Vec<_>>()
                .join(", ")
        )
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

impl NoTypeDisplay for DictComprehension {
    fn to_string_notype(&self) -> String {
        format!(
            "[{}: {} | {}]",
            self.key.to_string_notype(),
            self.value.to_string_notype(),
            self.guard.to_string_notype()
        )
    }
}

impl_display_from_nested!(DictComprehension);
impl_locational!(DictComprehension, l_sqbr, key, r_sqbr);
impl_t!(DictComprehension);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Dict {
    Normal(NormalDict),
    Comprehension(DictComprehension),
}

impl_nested_display_for_enum!(Dict; Normal, Comprehension);
impl_no_type_display_for_enum!(Dict; Normal, Comprehension);
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

impl NoTypeDisplay for NormalSet {
    fn to_string_notype(&self) -> String {
        format!(
            "{{{}}}",
            self.elems
                .pos_args
                .iter()
                .map(|e| e.to_string_notype())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl_display_from_nested!(NormalSet);
impl_locational!(NormalSet, l_brace, elems, r_brace);
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
        write!(f, "{{{}; {}}}(: {})", self.elem, self.len, self.t)
    }
}

impl NoTypeDisplay for SetWithLength {
    fn to_string_notype(&self) -> String {
        format!(
            "{{{}; {}}}",
            self.elem.to_string_notype(),
            self.len.to_string_notype()
        )
    }
}

impl_display_from_nested!(SetWithLength);
impl_locational!(SetWithLength, l_brace, elem, r_brace);
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
impl_no_type_display_for_enum!(Set; Normal, WithLength);
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

impl NoTypeDisplay for RecordAttrs {
    fn to_string_notype(&self) -> String {
        self.0
            .iter()
            .map(|a| a.to_string_notype())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl_display_from_nested!(RecordAttrs);
impl_stream!(RecordAttrs, Def);

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
    pub(crate) t: Type,
}

impl NestedDisplay for Record {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "{{")?;
        self.attrs.fmt_nest(f, level + 1)?;
        writeln!(f, "\n{}}}", "    ".repeat(level))
    }
}

impl NoTypeDisplay for Record {
    fn to_string_notype(&self) -> String {
        format!(
            "{{{}}}",
            self.attrs
                .iter()
                .map(|a| a.to_string_notype())
                .collect::<Vec<_>>()
                .join("; ")
        )
    }
}

impl_display_from_nested!(Record);
impl_locational!(Record, l_brace, attrs, r_brace);
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

impl NoTypeDisplay for BinOp {
    fn to_string_notype(&self) -> String {
        format!(
            "`{}`({}, {})",
            self.op.content,
            self.lhs.to_string_notype(),
            self.rhs.to_string_notype()
        )
    }
}

impl HasType for BinOp {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.info.t.return_t().unwrap()
    }
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        self.info.t.mut_return_t()
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
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        self.info.t.mut_return_t()
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

impl NoTypeDisplay for UnaryOp {
    fn to_string_notype(&self) -> String {
        format!("`{}`({})", self.op.content, self.expr.to_string_notype())
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
        writeln!(f, "({}){}", self.obj, fmt_option!(self.attr_name),)?;
        if self.args.is_empty() {
            write!(f, "()")
        } else {
            writeln!(f, ":")?;
            self.args.fmt_nest(f, level + 1)
        }
    }
}

impl NoTypeDisplay for Call {
    fn to_string_notype(&self) -> String {
        format!(
            "({}){}({})",
            self.obj.to_string_notype(),
            fmt_option!(self.attr_name),
            self.args.to_string_notype()
        )
    }
}

impl_display_from_nested!(Call);

impl HasType for Call {
    #[inline]
    fn ref_t(&self) -> &Type {
        if let Some(attr) = self.attr_name.as_ref() {
            attr.ref_t().return_t().unwrap_or(Type::FAILURE)
        } else {
            self.obj.ref_t().return_t().unwrap_or(Type::FAILURE)
        }
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        if let Some(attr) = self.attr_name.as_mut() {
            attr.ref_mut_t()?.mut_return_t()
        } else {
            self.obj.ref_mut_t()?.mut_return_t()
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
            attr.ref_mut_t()
        } else {
            if let Expr::Call(call) = self.obj.as_ref() {
                call.return_t()?;
            }
            self.obj.ref_mut_t()
        }
    }
}

impl_locational!(Call, obj, args);

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
            "pyimport" | "py" | "__import__" => Some(OperationKind::PyImport),
            "Del" => Some(OperationKind::Del),
            "assert" => Some(OperationKind::Assert),
            _ => {
                if self.obj.qual_name() == Some("typing".into())
                    && self
                        .attr_name
                        .as_ref()
                        .map_or(false, |ident| &ident.inspect()[..] == "cast")
                {
                    return Some(OperationKind::Cast);
                }
                if self.obj.ref_t().is_callable() {
                    match self.attr_name.as_ref().map(|i| &i.inspect()[..]) {
                        Some("return") => Some(OperationKind::Return),
                        Some("yield") => Some(OperationKind::Yield),
                        _ => None,
                    }
                } else {
                    None
                }
            }
        })
    }

    pub fn return_t(&self) -> Option<&Type> {
        if let Some(attr) = self.attr_name.as_ref() {
            attr.ref_t().return_t()
        } else {
            self.obj.ref_t().return_t()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block(Vec<Expr>);

impl HasType for Block {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.last()
            .map(|last| last.ref_t())
            .unwrap_or(Type::FAILURE)
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        self.last_mut()?.ref_mut_t()
    }
    #[inline]
    fn t(&self) -> Type {
        self.last().map(|last| last.t()).unwrap_or(Type::Failure)
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        self.last().and_then(|last| last.signature_t())
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

impl NoTypeDisplay for Block {
    fn to_string_notype(&self) -> String {
        self.0
            .iter()
            .map(|e| e.to_string_notype())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

impl_display_from_nested!(Block);
impl_stream!(Block, Expr);

impl Locational for Block {
    fn loc(&self) -> Location {
        Location::stream(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dummy(Vec<Expr>);

impl HasType for Dummy {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::FAILURE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        None
    }
    #[inline]
    fn t(&self) -> Type {
        Type::Failure
    }
    #[inline]
    fn signature_t(&self) -> Option<&Type> {
        Some(Type::FAILURE)
    }
    #[inline]
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        todo!()
    }
}

impl NestedDisplay for Dummy {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        writeln!(f, "Dummy:")?;
        fmt_lines(self.0.iter(), f, level)
    }
}

impl NoTypeDisplay for Dummy {
    fn to_string_notype(&self) -> String {
        self.0
            .iter()
            .map(|e| e.to_string_notype())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

impl_display_from_nested!(Dummy);
impl_stream!(Dummy, Expr);

impl Locational for Dummy {
    fn loc(&self) -> Location {
        Location::stream(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarSignature {
    pub ident: Identifier,
    /// if this flag is `true`, the variable is stored as `global`
    pub global: bool,
    pub t_spec: Option<TypeSpecWithOp>,
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
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
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
    pub const fn new(ident: Identifier, t_spec: Option<TypeSpecWithOp>) -> Self {
        Self {
            ident,
            global: false,
            t_spec,
        }
    }

    pub const fn global(ident: Identifier, t_spec: Option<TypeSpecWithOp>) -> Self {
        Self {
            ident,
            global: true,
            t_spec,
        }
    }

    pub fn inspect(&self) -> &Str {
        self.ident.inspect()
    }

    pub fn vis(&self) -> &VisibilityModifier {
        self.ident.vis()
    }

    pub const fn name(&self) -> &VarName {
        &self.ident.raw.name
    }
}

/// Once the default_value is set to Some, all subsequent values must be Some
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NonDefaultParamSignature {
    pub raw: ast::NonDefaultParamSignature,
    pub vi: VarInfo,
    pub t_spec_as_expr: Option<Expr>,
}

impl NestedDisplay for NonDefaultParamSignature {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, _level: usize) -> std::fmt::Result {
        write!(f, "{}(: {})", self.raw, self.vi.t)
    }
}

impl_display_from_nested!(NonDefaultParamSignature);

impl Locational for NonDefaultParamSignature {
    fn loc(&self) -> Location {
        self.raw.loc()
    }
}

impl NonDefaultParamSignature {
    pub const fn new(
        sig: ast::NonDefaultParamSignature,
        vi: VarInfo,
        t_spec_as_expr: Option<Expr>,
    ) -> Self {
        Self {
            raw: sig,
            vi,
            t_spec_as_expr,
        }
    }

    pub const fn inspect(&self) -> Option<&Str> {
        self.raw.pat.inspect()
    }

    pub const fn name(&self) -> Option<&VarName> {
        self.raw.pat.name()
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

impl NoTypeDisplay for DefaultParamSignature {
    fn to_string_notype(&self) -> String {
        format!("{} := {}", self.sig, self.default_val.to_string_notype())
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
        self.sig.inspect()
    }

    pub const fn name(&self) -> Option<&VarName> {
        self.sig.name()
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

impl NoTypeDisplay for Params {
    fn to_string_notype(&self) -> String {
        format!(
            "({}, {}, {})",
            fmt_vec(&self.non_defaults),
            fmt_option!(pre "*", &self.var_params),
            self.defaults
                .iter()
                .map(|p| p.to_string_notype())
                .fold("".to_string(), |acc, e| acc + &e + ", ")
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

type RefRawParams<'a> = (
    &'a Vec<NonDefaultParamSignature>,
    &'a Option<Box<NonDefaultParamSignature>>,
    &'a Vec<DefaultParamSignature>,
    &'a Option<(Token, Token)>,
);

impl Params {
    pub const fn new(
        non_defaults: Vec<NonDefaultParamSignature>,
        var_params: Option<Box<NonDefaultParamSignature>>,
        defaults: Vec<DefaultParamSignature>,
        parens: Option<(Token, Token)>,
    ) -> Self {
        Self {
            non_defaults,
            var_params,
            defaults,
            parens,
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], None, vec![], None)
    }

    pub fn single(sig: NonDefaultParamSignature) -> Self {
        Self::new(vec![sig], None, vec![], None)
    }

    pub const fn ref_deconstruct(&self) -> RefRawParams {
        (
            &self.non_defaults,
            &self.var_params,
            &self.defaults,
            &self.parens,
        )
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

pub type Decorator = Expr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubrSignature {
    pub decorators: HashSet<Decorator>,
    pub ident: Identifier,
    pub bounds: TypeBoundSpecs,
    pub params: Params,
    pub return_t_spec: Option<TypeSpecWithOp>,
}

impl NestedDisplay for SubrSignature {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(
            f,
            "{}{} (: {})",
            self.ident.to_string_notype(),
            self.params,
            self.ident.t()
        )
    }
}

impl NoTypeDisplay for SubrSignature {
    fn to_string_notype(&self) -> String {
        format!(
            "{}{}",
            self.ident.to_string_notype(),
            self.params.to_string_notype()
        )
    }
}

impl_display_from_nested!(SubrSignature);
impl_locational!(SubrSignature, ident, params);

impl HasType for SubrSignature {
    #[inline]
    fn ref_t(&self) -> &Type {
        self.ident.ref_t()
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
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

    pub fn is_procedural(&self) -> bool {
        self.ident.is_procedural()
    }

    pub const fn name(&self) -> &VarName {
        &self.ident.raw.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lambda {
    pub params: Params,
    pub op: Token,
    pub pre_block: Block,
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

impl NoTypeDisplay for Lambda {
    fn to_string_notype(&self) -> String {
        format!(
            "{} {} {}",
            self.params.to_string_notype(),
            self.op.content,
            self.body.to_string_notype()
        )
    }
}

impl_display_from_nested!(Lambda);
impl_locational!(Lambda, params, body);
impl_t!(Lambda);

impl Lambda {
    pub const fn new(
        id: usize,
        params: Params,
        op: Token,
        pre_block: Block,
        body: Block,
        t: Type,
    ) -> Self {
        Self {
            id,
            params,
            op,
            pre_block,
            body,
            t,
        }
    }

    pub fn is_procedural(&self) -> bool {
        self.op.is(TokenKind::ProcArrow)
    }

    pub fn name_to_string(&self) -> String {
        format!("::<lambda_{}>", self.id)
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

    pub fn vis(&self) -> &VisibilityModifier {
        match self {
            Self::Var(v) => v.ident.vis(),
            Self::Subr(s) => s.ident.vis(),
        }
    }

    pub const fn inspect(&self) -> &Str {
        match self {
            Self::Var(v) => v.ident.inspect(),
            Self::Subr(s) => s.ident.inspect(),
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

    pub const fn name(&self) -> &VarName {
        match self {
            Self::Var(v) => v.name(),
            Self::Subr(s) => s.name(),
        }
    }

    pub fn t_spec(&self) -> Option<&TypeSpec> {
        match self {
            Self::Var(v) => v.t_spec.as_ref().map(|t| &t.raw.t_spec),
            Self::Subr(s) => s.return_t_spec.as_ref().map(|t| &t.raw.t_spec),
        }
    }

    pub fn t_spec_with_op(&self) -> Option<&TypeSpecWithOp> {
        match self {
            Self::Var(v) => v.t_spec.as_ref(),
            Self::Subr(s) => s.return_t_spec.as_ref(),
        }
    }

    pub fn params_mut(&mut self) -> Option<&mut Params> {
        match self {
            Self::Var(_) => None,
            Self::Subr(s) => Some(&mut s.params),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefBody {
    pub op: Token,
    pub pre_block: Block,
    pub block: Block,
    pub id: DefId,
}

impl_locational!(DefBody, lossy op, block);
impl_t!(DefBody, delegate block);

impl DefBody {
    pub const fn new(op: Token, pre_block: Block, block: Block, id: DefId) -> Self {
        Self {
            op,
            pre_block,
            block,
            id,
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
        writeln!(f, "{} {}", self.sig, self.body.op.content)?;
        self.body.block.fmt_nest(f, level + 1)
    }
}

impl NoTypeDisplay for Def {
    fn to_string_notype(&self) -> String {
        format!(
            "{} {} {}",
            self.sig,
            self.body.op.content,
            self.body.block.to_string_notype()
        )
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
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        None
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
                Some("Patch") => DefKind::Patch,
                Some("import") => DefKind::ErgImport,
                Some("pyimport") | Some("__import__") => DefKind::PyImport,
                #[cfg(feature = "debug")]
                Some("py") => DefKind::PyImport,
                _ => DefKind::Other,
            },
            _ => DefKind::Other,
        }
    }

    pub fn get_base(&self) -> Option<&Record> {
        match self.body.block.first().unwrap() {
            Expr::Call(call) => match call.obj.show_acc().as_ref().map(|n| &n[..]) {
                Some("Class") | Some("Trait") => {
                    if let Some(Expr::Record(rec)) = call.args.get_left_or_key("Base") {
                        Some(rec)
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
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

// TODO
impl NoTypeDisplay for Methods {
    fn to_string_notype(&self) -> String {
        format!(
            "{}{} {}",
            self.class,
            self.vis.content,
            self.defs.to_string_notype()
        )
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
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        None
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
    pub require_or_sup: Option<Box<Expr>>,
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

// TODO
impl NoTypeDisplay for ClassDef {
    fn to_string_notype(&self) -> String {
        format!("{}: {}", self.sig, self.methods.to_string_notype())
    }
}

impl_display_from_nested!(ClassDef);
impl_locational!(ClassDef, sig, lossy methods);

impl HasType for ClassDef {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::NONE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        None
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
        require_or_sup: Option<Expr>,
        need_to_gen_new: bool,
        __new__: Type,
        methods: Block,
    ) -> Self {
        Self {
            obj,
            sig,
            require_or_sup: require_or_sup.map(Box::new),
            need_to_gen_new,
            __new__,
            methods,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatchDef {
    pub sig: Signature,
    pub base: Box<Expr>,
    pub methods: Block,
}

impl NestedDisplay for PatchDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "{} = Patch ", self.sig)?;
        self.base.fmt_nest(f, level)?;
        writeln!(f, ":")?;
        self.methods.fmt_nest(f, level + 1)
    }
}

// TODO
impl NoTypeDisplay for PatchDef {
    fn to_string_notype(&self) -> String {
        format!(
            "{} = Patch {}: {}",
            self.sig,
            self.base.to_string_notype(),
            self.methods.to_string_notype()
        )
    }
}

impl_display_from_nested!(PatchDef);
impl_locational!(PatchDef, sig, methods);

impl HasType for PatchDef {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::NONE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        None
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

impl PatchDef {
    pub fn new(sig: Signature, base: Expr, methods: Block) -> Self {
        Self {
            sig,
            base: Box::new(base),
            methods,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReDef {
    pub attr: Accessor,
    pub block: Block,
}

impl NestedDisplay for ReDef {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        self.attr.fmt_nest(f, level)?;
        writeln!(f, " = ")?;
        self.block.fmt_nest(f, level + 1)
    }
}

impl NoTypeDisplay for ReDef {
    fn to_string_notype(&self) -> String {
        format!(
            "{} = {}",
            self.attr.to_string_notype(),
            self.block.to_string_notype()
        )
    }
}

impl_display_from_nested!(ReDef);
impl_locational!(ReDef, attr, block);

impl HasType for ReDef {
    #[inline]
    fn ref_t(&self) -> &Type {
        Type::NONE
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        None
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

impl ReDef {
    pub const fn new(attr: Accessor, block: Block) -> Self {
        Self { attr, block }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeSpecWithOp {
    pub raw: ast::TypeSpecWithOp,
    /// Required for dynamic type checking
    pub expr: Box<Expr>,
    pub spec_t: Type,
}

impl NestedDisplay for TypeSpecWithOp {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl_display_from_nested!(TypeSpecWithOp);
impl_locational!(TypeSpecWithOp, raw);

impl TypeSpecWithOp {
    pub fn new(raw: ast::TypeSpecWithOp, expr: Expr, spec_t: Type) -> Self {
        Self {
            raw,
            expr: Box::new(expr),
            spec_t,
        }
    }

    pub fn kind(&self) -> AscriptionKind {
        match self.raw.op.kind {
            TokenKind::Colon => AscriptionKind::TypeOf,
            TokenKind::SubtypeOf => AscriptionKind::SubtypeOf,
            TokenKind::SupertypeOf => AscriptionKind::SupertypeOf,
            TokenKind::As => AscriptionKind::AsCast,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeAscription {
    pub expr: Box<Expr>,
    pub spec: TypeSpecWithOp,
}

impl NestedDisplay for TypeAscription {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        writeln!(f, "{} {}", self.expr, self.spec)
    }
}

impl NoTypeDisplay for TypeAscription {
    fn to_string_notype(&self) -> String {
        format!("{}{}", self.expr.to_string_notype(), self.spec)
    }
}

impl_display_from_nested!(TypeAscription);
impl_locational!(TypeAscription, expr, spec);

impl HasType for TypeAscription {
    #[inline]
    fn ref_t(&self) -> &Type {
        if self.spec.kind().is_force_cast() {
            &self.spec.spec_t
        } else {
            self.expr.ref_t()
        }
    }
    #[inline]
    fn ref_mut_t(&mut self) -> Option<&mut Type> {
        if self.spec.kind().is_force_cast() {
            Some(&mut self.spec.spec_t)
        } else {
            self.expr.ref_mut_t()
        }
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
    pub fn new(expr: Expr, spec: TypeSpecWithOp) -> Self {
        Self {
            expr: Box::new(expr),
            spec,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Literal(Literal),
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
    PatchDef(PatchDef),
    ReDef(ReDef),
    TypeAsc(TypeAscription),
    Code(Block),     // code object
    Compound(Block), // compound statement
    Import(Accessor),
    Dummy(Dummy), // for mapping to Python AST
}

impl_nested_display_for_chunk_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, PatchDef, ReDef, Code, Compound, TypeAsc, Set, Import, Dummy);
impl_no_type_display_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, PatchDef, ReDef, Code, Compound, TypeAsc, Set, Import, Dummy);
impl_display_from_nested!(Expr);
impl_locational_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, PatchDef, ReDef, Code, Compound, TypeAsc, Set, Import, Dummy);
impl_t_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, PatchDef, ReDef, Code, Compound, TypeAsc, Set, Import, Dummy);
impl_from_trait_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, PatchDef, ReDef, Set, Dummy);
impl_try_from_trait_for_enum!(Expr; Literal, Accessor, Array, Tuple, Dict, Record, BinOp, UnaryOp, Call, Lambda, Def, ClassDef, PatchDef, ReDef, Set, Dummy);

impl Default for Expr {
    fn default() -> Self {
        Self::Code(Block::default())
    }
}

impl Expr {
    pub fn receiver_t(&self) -> Option<&Type> {
        match self {
            Self::Accessor(Accessor::Attr(attr)) => Some(attr.obj.ref_t()),
            Self::TypeAsc(t_asc) => t_asc.expr.receiver_t(),
            _other => None,
        }
    }

    pub fn show_acc(&self) -> Option<String> {
        match self {
            Expr::Accessor(acc) => Some(acc.show()),
            Expr::TypeAsc(t_asc) => t_asc.expr.show_acc(),
            _ => None,
        }
    }

    pub fn var_info(&self) -> Option<&VarInfo> {
        match self {
            Expr::Accessor(acc) => Some(acc.var_info()),
            Expr::TypeAsc(t_asc) => t_asc.expr.var_info(),
            _ => None,
        }
    }

    /// 参照するオブジェクト自体が持っている名前(e.g. Int.qual_name == Some("int"), Socket!.qual_name == Some("io.Socket!"))
    pub fn qual_name(&self) -> Option<Str> {
        match self {
            Expr::Accessor(acc) => acc.qual_name(),
            Expr::TypeAsc(tasc) => tasc.expr.qual_name(),
            _ => None,
        }
    }

    /// e.g. Int.local_name == Some("int"), Socket!.local_name == Some("Socket!")
    pub fn local_name(&self) -> Option<&str> {
        match self {
            Expr::Accessor(acc) => acc.local_name(),
            Expr::TypeAsc(tasc) => tasc.expr.local_name(),
            _ => None,
        }
    }

    pub fn is_py_api(&self) -> bool {
        match self {
            Expr::Accessor(acc) => acc.is_py_api(),
            Expr::TypeAsc(tasc) => tasc.expr.is_py_api(),
            _ => false,
        }
    }

    pub fn last_name(&self) -> Option<&VarName> {
        match self {
            Expr::Accessor(acc) => Some(acc.last_name()),
            Expr::TypeAsc(tasc) => tasc.expr.last_name(),
            _ => None,
        }
    }

    pub fn is_type_asc(&self) -> bool {
        matches!(self, Expr::TypeAsc(_))
    }

    pub fn is_doc_comment(&self) -> bool {
        match self {
            Expr::Literal(lit) => lit.is_doc_comment(),
            _ => false,
        }
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
            Self::Lambda(_) => "lambda",
            Self::TypeAsc(_) => "type ascription",
            Self::Def(_) => "definition",
            Self::Code(_) => "code",
            Self::Compound(_) => "compound expression",
            Self::Import(_) => "import",
            Self::ClassDef(_) => "class definition",
            Self::PatchDef(_) => "patch definition",
            Self::ReDef(_) => "re-definition",
            Self::Dummy(_) => "dummy",
        }
    }

    pub fn need_to_be_closed(&self) -> bool {
        match self {
            Self::BinOp(_) | Self::UnaryOp(_) | Self::Lambda(_) | Self::TypeAsc(_) => true,
            Self::Tuple(tup) => match tup {
                Tuple::Normal(tup) => tup.elems.paren.is_none(),
            },
            Self::Call(call) if ERG_MODE => call.args.paren.is_none(),
            _ => false,
        }
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
        self.call_expr(Args::single(PosArg::new(expr)))
    }

    pub fn call2(self, expr1: Expr, expr2: Expr) -> Self {
        self.call_expr(Args::pos_only(
            vec![PosArg::new(expr1), PosArg::new(expr2)],
            None,
        ))
    }

    pub fn attr(self, ident: Identifier) -> Accessor {
        Accessor::attr(self, ident)
    }

    pub fn attr_expr(self, ident: Identifier) -> Self {
        Self::Accessor(self.attr(ident))
    }

    pub fn type_asc(self, t_spec: TypeSpecWithOp) -> TypeAscription {
        TypeAscription::new(self, t_spec)
    }

    pub fn type_asc_expr(self, t_spec: TypeSpecWithOp) -> Self {
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
        Location::stream(&self.0)
    }
}

impl_stream!(Module, Expr);

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
