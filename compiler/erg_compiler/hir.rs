/// defines High-level Intermediate Representation
use std::fmt;

use erg_common::error::Location;
use erg_common::traits::{HasType, Locational, NestedDisplay, Stream};
use erg_common::ty::{Constraint, TyParam, Type};
use erg_common::value::ValueObj;
use erg_common::Str;
use erg_common::{
    impl_display_for_enum, impl_display_from_nested, impl_locational, impl_locational_for_enum,
    impl_nested_display_for_enum, impl_stream_for_wrapper,
};

use erg_parser::ast::{fmt_lines, DefId, Params, VarName, VarPattern};
use erg_parser::token::{Token, TokenKind};

use crate::error::readable_name;

#[derive(Debug, Clone)]
pub struct Literal {
    pub data: ValueObj, // for constant folding
    pub token: Token,   // for Locational
    t: Type,
}

impl HasType for Literal {
    #[inline]
    fn ref_t(&self) -> &Type {
        &self.t
    }
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

impl NestedDisplay for Literal {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{}", self.token)
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
        let data = ValueObj::from_str(Type::from(token.kind), token.content.clone());
        Self {
            t: data.t(),
            data,
            token,
        }
    }
}

impl Literal {
    pub fn new(c: ValueObj, lineno: usize, col: usize) -> Self {
        let kind = TokenKind::from(&c);
        let token = Token::new(kind, c.to_string(), lineno, col);
        Self {
            t: c.t(),
            data: c,
            token,
        }
    }

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
        write!(f, "{}:\n", self.keyword)?;
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
}

/// represents local variables
#[derive(Debug, Clone)]
pub struct Local {
    pub name: Token,
    /// オブジェクト自身の名前
    __name__: Option<Str>,
    t: Type,
}

impl fmt::Display for Local {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let __name__ = if let Some(__name__) = self.__name__() {
            format!("(__name__ = {__name__})")
        } else {
            "".to_string()
        };
        if self.t != Type::ASTOmitted {
            write!(f, "{} (: {}){}", self.name.content, self.t, __name__)
        } else {
            write!(f, "{}{}", self.name.content, __name__)
        }
    }
}

impl HasType for Local {
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
pub struct Attribute {
    pub obj: Box<Expr>,
    pub name: Token,
    t: Type,
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}).{}", self.obj, self.name)
    }
}

impl_locational!(Attribute, obj, name);

impl HasType for Attribute {
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

impl Attribute {
    pub fn new(obj: Expr, name: Token, t: Type) -> Self {
        Self {
            obj: Box::new(obj),
            name,
            t,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Subscript {
    obj: Box<Expr>,
    index: Box<Expr>,
    t: Type,
}

impl fmt::Display for Subscript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})[{}]", self.obj, self.index)
    }
}

impl_locational!(Subscript, obj, index);

impl HasType for Subscript {
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
    SelfDot(Local),
    Attr(Attribute),
    Subscr(Subscript),
}

impl NestedDisplay for Accessor {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        match self {
            Self::Local(name) => write!(f, "{}", name),
            Self::SelfDot(attr) => write!(f, ".{}", attr),
            Self::Attr(attr) => write!(f, "{}", attr),
            Self::Subscr(subscr) => write!(f, "{}", subscr),
        }
    }
}

impl_display_from_nested!(Accessor);
impl_locational_for_enum!(Accessor; Local, SelfDot, Attr, Subscr);

impl HasType for Accessor {
    #[inline]
    fn ref_t(&self) -> &Type {
        match self {
            Self::Local(n) | Self::SelfDot(n) => n.ref_t(),
            Self::Attr(a) => a.ref_t(),
            Self::Subscr(s) => s.ref_t(),
        }
    }
    #[inline]
    fn ref_mut_t(&mut self) -> &mut Type {
        match self {
            Self::Local(n) | Self::SelfDot(n) => n.ref_mut_t(),
            Self::Attr(a) => a.ref_mut_t(),
            Self::Subscr(s) => s.ref_mut_t(),
        }
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

impl Accessor {
    pub const fn local(symbol: Token, t: Type) -> Self {
        Self::Local(Local::new(symbol, None, t))
    }

    pub const fn self_dot(name: Token, t: Type) -> Self {
        Self::SelfDot(Local::new(name, None, t))
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
            Self::Subscr(_) | Self::SelfDot(_) => todo!(),
        }
    }

    // 参照するオブジェクト自体が持っている固有の名前
    pub fn __name__(&self) -> Option<&str> {
        match self {
            Self::Local(local) | Self::SelfDot(local) => local.__name__().map(|s| &s[..]),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Array {
    pub l_sqbr: Token,
    pub r_sqbr: Token,
    t: Type,
    pub elems: Args,
    pub guard: Option<Box<Expr>>,
}

impl HasType for Array {
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

impl NestedDisplay for Array {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        if let Some(guard) = &self.guard {
            write!(f, "[{} | {}]", self.elems, guard)
        } else {
            write!(f, "[{}]", self.elems)
        }
    }
}

impl_display_from_nested!(Array);
impl_locational!(Array, l_sqbr, r_sqbr);

impl Array {
    pub fn new(
        l_sqbr: Token,
        r_sqbr: Token,
        level: usize,
        elems: Args,
        guard: Option<Expr>,
    ) -> Self {
        let elem_t = elems
            .pos_args
            .first()
            .map(|a| a.expr.t())
            .unwrap_or_else(|| Type::free_var(level, Constraint::TypeOf(Type::Type)));
        let t = Type::array(elem_t, TyParam::value(elems.len()));
        Self {
            l_sqbr,
            r_sqbr,
            t,
            elems,
            guard: guard.map(Box::new),
        }
    }

    pub fn push(&mut self, elem: Expr) {
        self.elems.push_pos(PosArg::new(elem));
    }
}

#[derive(Debug, Clone)]
pub struct Dict {
    pub l_brace: Token,
    pub r_brace: Token,
    pub attrs: Args, // TODO: keyをTokenではなくExprにする
}

impl HasType for Dict {
    fn ref_t(&self) -> &Type {
        todo!()
    }
    fn ref_mut_t(&mut self) -> &mut Type {
        todo!()
    }
    fn t(&self) -> Type {
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

impl NestedDisplay for Dict {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, _level: usize) -> fmt::Result {
        write!(f, "{{{}}}", self.attrs)
    }
}

impl_display_from_nested!(Dict);
impl_locational!(Dict, l_brace, r_brace);

impl Dict {
    pub const fn new(l_brace: Token, r_brace: Token, attrs: Args) -> Self {
        Self {
            l_brace,
            r_brace,
            attrs,
        }
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
        write!(f, "`{}`: {}:\n", self.op.content, self.sig_t)?;
        self.lhs.fmt_nest(f, level + 1)?;
        write!(f, "\n")?;
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
        write!(f, "`{}`: {}:\n", self.op, self.sig_t)?;
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
    pub args: Args,
    /// 全体の型、e.g. `abs(-1)` -> `Neg -> Nat`
    /// necessary for mangling
    pub sig_t: Type,
}

impl NestedDisplay for Call {
    fn fmt_nest(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        write!(f, "({}): {}:\n", self.obj, self.sig_t)?;
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
    pub fn new(obj: Expr, args: Args, sig_t: Type) -> Self {
        Self {
            obj: Box::new(obj),
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
    pub pat: VarPattern,
    pub t: Type,
}

impl fmt::Display for VarSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (: {})", self.pat, self.t)
    }
}

impl Locational for VarSignature {
    fn loc(&self) -> Location {
        self.pat.loc()
    }
}

impl VarSignature {
    pub const fn new(pat: VarPattern, t: Type) -> Self {
        Self { pat, t }
    }

    pub fn inspect(&self) -> Option<&Str> {
        self.pat.inspect()
    }
}

#[derive(Debug, Clone)]
pub struct SubrSignature {
    pub name: VarName,
    pub params: Params,
    pub t: Type,
}

impl fmt::Display for SubrSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{} (: {})", self.name, self.params, self.t)
    }
}

impl Locational for SubrSignature {
    fn loc(&self) -> Location {
        Location::concat(&self.name, &self.params)
    }
}

impl SubrSignature {
    pub const fn new(name: VarName, params: Params, t: Type) -> Self {
        Self { name, params, t }
    }

    pub fn is_procedural(&self) -> bool {
        self.name.is_procedural()
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

impl HasType for Lambda {
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

impl NestedDisplay for Lambda {
    fn fmt_nest(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "{} {}\n", self.params, self.op.content)?;
        self.body.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Lambda);
impl_locational!(Lambda, params, body);

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

impl_display_for_enum!(Signature; Var, Subr,);
impl_locational_for_enum!(Signature; Var, Subr,);

impl Signature {
    pub const fn is_subr(&self) -> bool {
        matches!(self, Self::Subr(_))
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Var(v) => v.pat.is_const(),
            Self::Subr(s) => s.name.is_const(),
        }
    }

    pub fn is_procedural(&self) -> bool {
        match self {
            Self::Var(v) => v.pat.is_procedural(),
            Self::Subr(s) => s.name.is_procedural(),
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
        write!(f, "{} {}\n", self.sig, self.body.op.content)?;
        self.body.block.fmt_nest(f, level + 1)
    }
}

impl_display_from_nested!(Def);
impl_locational!(Def, sig, body);

impl Def {
    pub const fn new(sig: Signature, body: DefBody) -> Self {
        Self { sig, body }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Literal),
    Accessor(Accessor),
    Array(Array),
    // Dict(Dict),
    // Set(Set),
    Dict(Dict),
    BinOp(BinOp),
    UnaryOp(UnaryOp),
    Call(Call),
    Lambda(Lambda),
    Decl(Decl),
    Def(Def),
}

impl_nested_display_for_enum!(Expr; Lit, Accessor, Array, Dict, BinOp, UnaryOp, Call, Lambda, Decl, Def);
impl_display_from_nested!(Expr);
impl_locational_for_enum!(Expr; Lit, Accessor, Array, Dict, BinOp, UnaryOp, Call, Lambda, Decl, Def);

impl HasType for Expr {
    fn ref_t(&self) -> &Type {
        match self {
            Expr::Lit(lit) => lit.ref_t(),
            Expr::Accessor(accessor) => accessor.ref_t(),
            Expr::Array(array) => array.ref_t(),
            Expr::Dict(dict) => dict.ref_t(),
            Expr::BinOp(bin) => bin.ref_t(),
            Expr::UnaryOp(unary) => unary.ref_t(),
            Expr::Call(call) => call.ref_t(),
            Expr::Lambda(lambda) => lambda.ref_t(),
            _ => &Type::NoneType,
        }
    }
    fn ref_mut_t(&mut self) -> &mut Type {
        match self {
            Expr::Lit(lit) => lit.ref_mut_t(),
            Expr::Accessor(accessor) => accessor.ref_mut_t(),
            Expr::Array(array) => array.ref_mut_t(),
            Expr::Dict(dict) => dict.ref_mut_t(),
            Expr::BinOp(bin) => bin.ref_mut_t(),
            Expr::UnaryOp(unary) => unary.ref_mut_t(),
            Expr::Call(call) => call.ref_mut_t(),
            Expr::Lambda(lambda) => lambda.ref_mut_t(),
            _ => todo!(),
        }
    }
    fn signature_t(&self) -> Option<&Type> {
        match self {
            Expr::BinOp(bin) => bin.signature_t(),
            Expr::UnaryOp(unary) => unary.signature_t(),
            Expr::Call(call) => call.signature_t(),
            _ => None,
        }
    }
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        match self {
            Expr::BinOp(bin) => bin.signature_mut_t(),
            Expr::UnaryOp(unary) => unary.signature_mut_t(),
            Expr::Call(call) => call.signature_mut_t(),
            _ => None,
        }
    }
}

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
            _ => todo!(),
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
