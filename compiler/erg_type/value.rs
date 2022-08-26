//! defines `ValueObj` (used in the compiler, VM).
//!
//! コンパイラ、VM等で使われる(データも保持した)値オブジェクトを定義する
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops::Neg;
use std::rc::Rc;

use erg_common::dict::Dict;
use erg_common::rccell::RcCell;
use erg_common::serialize::*;
use erg_common::set;
use erg_common::vis::Field;
use erg_common::{fmt_iter, impl_display_from_debug, switch_lang};
use erg_common::{RcArray, Str};

use crate::codeobj::CodeObj;
use crate::constructors::{array, class, poly_class, refinement};
use crate::free::fresh_varname;
use crate::typaram::TyParam;
use crate::{ConstSubr, HasType, Predicate, Type};

/// 値オブジェクト
/// コンパイル時評価ができ、シリアライズも可能
#[derive(Clone, PartialEq, Default)]
pub enum ValueObj {
    Int(i32),
    Nat(u64),
    Float(f64),
    Str(Str),
    Bool(bool),
    Array(Rc<[ValueObj]>),
    Dict(Rc<[(ValueObj, ValueObj)]>),
    Record(Dict<Field, ValueObj>),
    Code(Box<CodeObj>),
    Subr(ConstSubr),
    Type(Box<Type>),
    None,
    Ellipsis,
    NotImplemented,
    NegInf,
    Inf,
    Mut(RcCell<ValueObj>),
    #[default]
    Illegal, // to avoid conversions with TryFrom
}

impl fmt::Debug for ValueObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{i}"),
            Self::Nat(n) => write!(f, "{n}"),
            Self::Float(fl) => {
                // In Rust, .0 is shown omitted.
                if fl.fract() < 1e-10 {
                    write!(f, "{fl:.1}")
                } else {
                    write!(f, "{fl}")
                }
            }
            Self::Str(s) => write!(f, "\"{s}\""),
            Self::Bool(b) => {
                if *b {
                    write!(f, "True")
                } else {
                    write!(f, "False")
                }
            }
            Self::Array(arr) => write!(f, "[{}]", fmt_iter(arr.iter())),
            Self::Dict(dict) => {
                write!(f, "{{")?;
                for (i, (k, v)) in dict.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Self::Code(code) => write!(f, "{code}"),
            Self::Record(rec) => {
                write!(f, "{{")?;
                for (i, (k, v)) in rec.iter().enumerate() {
                    if i != 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{k} = {v}")?;
                }
                write!(f, "}}")
            }
            Self::Subr(subr) => write!(f, "{subr:?}"),
            Self::Type(t) => write!(f, "{t}"),
            Self::None => write!(f, "None"),
            Self::Ellipsis => write!(f, "Ellipsis"),
            Self::NotImplemented => write!(f, "NotImplemented"),
            Self::NegInf => write!(f, "-Inf"),
            Self::Inf => write!(f, "Inf"),
            Self::Mut(v) => write!(f, "!{:?}", v.borrow()),
            Self::Illegal => write!(f, "<illegal>"),
        }
    }
}

impl_display_from_debug!(ValueObj);

impl Eq for ValueObj {}

impl Neg for ValueObj {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        match self {
            Self::Int(i) => Self::Int(-i),
            Self::Nat(n) => Self::Int(-(n as i32)),
            Self::Float(fl) => Self::Float(-fl),
            Self::Inf => Self::NegInf,
            Self::NegInf => Self::Inf,
            other => panic!("cannot negate {other}"),
        }
    }
}

// FIXME:
impl Hash for ValueObj {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Int(i) => i.hash(state),
            Self::Nat(n) => n.hash(state),
            // TODO:
            Self::Float(f) => f.to_bits().hash(state),
            Self::Str(s) => s.hash(state),
            Self::Bool(b) => b.hash(state),
            Self::Array(arr) => arr.hash(state),
            Self::Dict(dict) => dict.hash(state),
            Self::Code(code) => code.hash(state),
            Self::Record(rec) => rec.hash(state),
            Self::Subr(subr) => subr.hash(state),
            Self::Type(t) => t.hash(state),
            Self::None => {
                "literal".hash(state);
                "None".hash(state)
            }
            Self::Ellipsis => {
                "literal".hash(state);
                "Ellipsis".hash(state)
            }
            Self::NotImplemented => {
                "literal".hash(state);
                "NotImplemented".hash(state)
            }
            Self::NegInf => {
                "literal".hash(state);
                "NegInf".hash(state)
            }
            Self::Inf => {
                "literal".hash(state);
                "Inf".hash(state)
            }
            Self::Mut(v) => v.borrow().hash(state),
            Self::Illegal => {
                "literal".hash(state);
                "illegal".hash(state)
            }
        }
    }
}

impl From<i32> for ValueObj {
    fn from(item: i32) -> Self {
        ValueObj::Int(item)
    }
}

impl From<u64> for ValueObj {
    fn from(item: u64) -> Self {
        ValueObj::Nat(item)
    }
}

impl From<usize> for ValueObj {
    fn from(item: usize) -> Self {
        ValueObj::Nat(item as u64)
    }
}

impl From<f64> for ValueObj {
    fn from(item: f64) -> Self {
        ValueObj::Float(item)
    }
}

impl From<&str> for ValueObj {
    fn from(item: &str) -> Self {
        ValueObj::Str(Str::rc(item))
    }
}

impl From<Str> for ValueObj {
    fn from(item: Str) -> Self {
        ValueObj::Str(item)
    }
}

impl From<bool> for ValueObj {
    fn from(item: bool) -> Self {
        ValueObj::Bool(item)
    }
}

impl From<CodeObj> for ValueObj {
    fn from(item: CodeObj) -> Self {
        ValueObj::Code(Box::new(item))
    }
}

impl From<Vec<ValueObj>> for ValueObj {
    fn from(item: Vec<ValueObj>) -> Self {
        ValueObj::Array(RcArray::from(&item[..]))
    }
}

impl TryFrom<&ValueObj> for f64 {
    type Error = ();
    fn try_from(val: &ValueObj) -> Result<f64, Self::Error> {
        match val {
            ValueObj::Int(i) => Ok(*i as f64),
            ValueObj::Nat(n) => Ok(*n as f64),
            ValueObj::Float(f) => Ok(*f),
            ValueObj::Inf => Ok(f64::INFINITY),
            ValueObj::NegInf => Ok(f64::NEG_INFINITY),
            ValueObj::Mut(v) => f64::try_from(&*v.borrow()).map_err(|_| ()),
            _ => Err(()),
        }
    }
}

impl HasType for ValueObj {
    fn ref_t(&self) -> &Type {
        panic!("cannot get reference of the const")
    }
    fn ref_mut_t(&mut self) -> &mut Type {
        panic!("cannot get mutable reference of the const")
    }
    /// その要素だけの集合型を返す、クラスが欲しい場合は.classで
    #[inline]
    fn t(&self) -> Type {
        let name = Str::from(fresh_varname());
        let pred = Predicate::eq(name.clone(), TyParam::Value(self.clone()));
        refinement(name, self.class(), set! {pred})
    }
    fn signature_t(&self) -> Option<&Type> {
        None
    }
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        None
    }
}

impl ValueObj {
    pub fn t(t: Type) -> Self {
        ValueObj::Type(Box::new(t))
    }

    pub fn is_num(&self) -> bool {
        match self {
            Self::Int(_) | Self::Nat(_) | Self::Float(_) | Self::Bool(_) => true,
            Self::Mut(n) => n.borrow().is_num(),
            _ => false,
        }
    }

    pub const fn is_mut(&self) -> bool {
        matches!(self, Self::Mut(_))
    }

    pub fn from_str(t: Type, content: Str) -> Self {
        match t {
            Type::Int => Self::Int(content.replace('_', "").parse::<i32>().unwrap()),
            Type::Nat => Self::Nat(content.replace('_', "").parse::<u64>().unwrap()),
            Type::Float => Self::Float(content.replace('_', "").parse::<f64>().unwrap()),
            // TODO:
            Type::Ratio => Self::Float(content.replace('_', "").parse::<f64>().unwrap()),
            Type::Str => {
                if &content[..] == "\"\"" {
                    Self::Str(Str::from(""))
                } else {
                    let replaced = content.trim_start_matches('\"').trim_end_matches('\"');
                    Self::Str(Str::rc(replaced))
                }
            }
            Type::Bool => Self::Bool(&content[..] == "True"),
            Type::NoneType => Self::None,
            Type::Ellipsis => Self::Ellipsis,
            Type::NotImplemented => Self::NotImplemented,
            Type::Inf => Self::Inf,
            Type::NegInf => Self::NegInf,
            _ => todo!("{t} {content}"),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Int(i) => [vec![DataTypePrefix::Int32 as u8], i.to_le_bytes().to_vec()].concat(),
            // TODO: Natとしてシリアライズ
            Self::Nat(n) => [
                vec![DataTypePrefix::Int32 as u8],
                (n as i32).to_le_bytes().to_vec(),
            ]
            .concat(),
            Self::Float(f) => [
                vec![DataTypePrefix::BinFloat as u8],
                f.to_le_bytes().to_vec(),
            ]
            .concat(),
            Self::Str(s) => str_into_bytes(s, false),
            Self::Bool(true) => vec![DataTypePrefix::True as u8],
            Self::Bool(false) => vec![DataTypePrefix::False as u8],
            // TODO: SmallTuple
            Self::Array(arr) => {
                let mut bytes = Vec::with_capacity(arr.len());
                bytes.push(DataTypePrefix::Tuple as u8);
                bytes.append(&mut (arr.len() as u32).to_le_bytes().to_vec());
                for obj in arr.iter().cloned() {
                    bytes.append(&mut obj.into_bytes());
                }
                bytes
            }
            Self::None => {
                vec![DataTypePrefix::None as u8]
            }
            Self::Code(c) => c.into_bytes(3425),
            // Dict
            other => {
                panic!(
                    "{}",
                    switch_lang!(
                        "japanese" => format!("このオブジェクトはシリアライズできません: {other}"),
                        "simplified_chinese" => format!("此对象无法序列化：{other}"),
                        "traditional_chinese" => format!("此對象無法序列化：{other}"),
                        "english" => format!("this object cannot be serialized: {other}"),
                    )
                )
            }
        }
    }

    pub fn class(&self) -> Type {
        match self {
            Self::Int(_) => Type::Int,
            Self::Nat(_) => Type::Nat,
            Self::Float(_) => Type::Float,
            Self::Str(_) => Type::Str,
            Self::Bool(_) => Type::Bool,
            // TODO:
            Self::Array(arr) => array(
                arr.iter().next().unwrap().class(),
                TyParam::value(arr.len()),
            ),
            Self::Dict(_dict) => todo!(),
            Self::Code(_) => Type::Code,
            Self::Record(rec) => {
                Type::Record(rec.iter().map(|(k, v)| (k.clone(), v.class())).collect())
            }
            Self::Subr(_) => todo!(),
            Self::Type(_) => Type::Type,
            Self::None => Type::NoneType,
            Self::Ellipsis => Type::Ellipsis,
            Self::NotImplemented => Type::NotImplemented,
            Self::Inf => Type::Inf,
            Self::NegInf => Type::NegInf,
            Self::Mut(m) => match &*m.borrow() {
                Self::Int(_) => class("Int!"),
                Self::Nat(_) => class("Nat!"),
                Self::Float(_) => class("Float!"),
                Self::Str(_) => class("Str!"),
                Self::Bool(_) => class("Bool!"),
                Self::Array(arr) => poly_class(
                    "Array!",
                    vec![
                        TyParam::t(arr.iter().next().unwrap().class()),
                        TyParam::value(arr.len()).mutate(),
                    ],
                ),
                Self::Dict(_dict) => todo!(),
                Self::Code(_) => Type::Code,
                Self::None => Type::NoneType,
                other => panic!("{other} object cannot be mutated"),
            },
            Self::Illegal => todo!(),
        }
    }

    pub fn try_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (l, r) if l.is_num() && r.is_num() => f64::try_from(l)
                .unwrap()
                .partial_cmp(&f64::try_from(r).unwrap()),
            (Self::Inf, n) | (n, Self::NegInf) if n.is_num() => Some(Ordering::Greater),
            (n, Self::Inf) | (Self::NegInf, n) if n.is_num() => Some(Ordering::Less),
            (Self::NegInf, Self::Inf) => Some(Ordering::Less),
            (Self::Inf, Self::NegInf) => Some(Ordering::Greater),
            // REVIEW: 等しいとみなしてよいのか?
            (Self::Inf, Self::Inf) | (Self::NegInf, Self::NegInf) => Some(Ordering::Equal),
            (Self::Mut(m), other) => m.borrow().try_cmp(other),
            (self_, Self::Mut(m)) => self_.try_cmp(&*m.borrow()),
            /* (Self::PlusEpsilon(l), r) => l.try_cmp(r)
                .map(|o| if matches!(o, Ordering::Equal) { Ordering::Less } else { o }),
            (l, Self::PlusEpsilon(r)) => l.try_cmp(r)
                .map(|o| if matches!(o, Ordering::Equal) { Ordering::Greater } else { o }),
            */
            (_s, _o) => {
                if let Some(ValueObj::Bool(b)) = _s.clone().try_eq(_o.clone()) {
                    if b {
                        Some(Ordering::Equal)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    // REVIEW: allow_divergenceオプションを付けるべきか?
    pub fn try_add(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l + r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l + r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l + r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l + r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 + r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float(l - r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::Float(l as f64 - r)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::Float(l as f64 - r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::Float(l - r as f64)),
            (Self::Str(l), Self::Str(r)) => Some(Self::Str(Str::from(format!("{}{}", l, r)))),
            (inf @ (Self::Inf | Self::NegInf), _) | (_, inf @ (Self::Inf | Self::NegInf)) => {
                Some(inf)
            }
            (Self::Mut(m), other) => {
                {
                    let ref_m = &mut *m.borrow_mut();
                    *ref_m = mem::take(ref_m).try_add(other)?;
                }
                Some(Self::Mut(m))
            }
            (self_, Self::Mut(m)) => self_.try_add(m.borrow().clone()),
            _ => None,
        }
    }

    pub fn try_sub(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l - r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Int((l - r) as i32)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l - r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l - r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 - r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l - r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 - r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l - r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 - r)),
            (inf @ (Self::Inf | Self::NegInf), other)
            | (other, inf @ (Self::Inf | Self::NegInf))
                if other != Self::Inf && other != Self::NegInf =>
            {
                Some(inf)
            }
            (Self::Mut(m), other) => {
                {
                    let ref_m = &mut *m.borrow_mut();
                    *ref_m = mem::take(ref_m).try_sub(other)?;
                }
                Some(Self::Mut(m))
            }
            (self_, Self::Mut(m)) => self_.try_sub(m.borrow().clone()),
            _ => None,
        }
    }

    pub fn try_mul(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l * r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l * r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l * r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Int(l * r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 * r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l * r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 * r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l * r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 * r)),
            (Self::Str(l), Self::Nat(r)) => Some(Self::Str(Str::from(l.repeat(r as usize)))),
            (inf @ (Self::Inf | Self::NegInf), _) | (_, inf @ (Self::Inf | Self::NegInf)) => {
                Some(inf)
            }
            (Self::Mut(m), other) => {
                {
                    let ref_m = &mut *m.borrow_mut();
                    *ref_m = mem::take(ref_m).try_mul(other)?;
                }
                Some(Self::Mut(m))
            }
            (self_, Self::Mut(m)) => self_.try_mul(m.borrow().clone()),
            _ => None,
        }
    }

    pub fn try_div(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Float(l as f64 / r as f64)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Float(l as f64 / r as f64)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l / r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Float(l as f64 / r as f64)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Float(l as f64 / r as f64)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float(l / r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 / r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l / r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 / r)),
            (Self::Mut(m), other) => {
                {
                    let ref_m = &mut *m.borrow_mut();
                    *ref_m = mem::take(ref_m).try_div(other)?;
                }
                Some(Self::Mut(m))
            }
            (self_, Self::Mut(m)) => self_.try_div(m.borrow().clone()),
            // TODO: x/±Inf = 0
            _ => None,
        }
    }

    pub fn try_gt(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l > r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l > r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l > r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l > r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 > r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l > r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 > r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l > r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 > r)),
            (Self::Mut(m), other) => {
                {
                    let ref_m = &mut *m.borrow_mut();
                    *ref_m = mem::take(ref_m).try_gt(other)?;
                }
                Some(Self::Mut(m))
            }
            (self_, Self::Mut(m)) => self_.try_gt(m.borrow().clone()),
            _ => None,
        }
    }

    pub fn try_ge(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l >= r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l >= r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l >= r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l >= r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 >= r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l >= r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 >= r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l >= r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 >= r)),
            (Self::Mut(m), other) => {
                {
                    let ref_m = &mut *m.borrow_mut();
                    *ref_m = mem::take(ref_m).try_ge(other)?;
                }
                Some(Self::Mut(m))
            }
            (self_, Self::Mut(m)) => self_.try_ge(m.borrow().clone()),
            _ => None,
        }
    }

    pub fn try_eq(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l == r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l == r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l == r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l == r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 == r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l == r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 == r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l == r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 == r)),
            (Self::Str(l), Self::Str(r)) => Some(Self::from(l == r)),
            (Self::Bool(l), Self::Bool(r)) => Some(Self::from(l == r)),
            (Self::Mut(m), other) => {
                {
                    let ref_m = &mut *m.borrow_mut();
                    *ref_m = mem::take(ref_m).try_eq(other)?;
                }
                Some(Self::Mut(m))
            }
            (self_, Self::Mut(m)) => self_.try_eq(m.borrow().clone()),
            // TODO:
            _ => None,
        }
    }

    pub fn try_ne(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::from(l != r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::from(l != r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::from(l != r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::from(l != r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::from(l as i32 != r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::from(l != r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::from(l as f64 != r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::from(l != r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::from(l as f64 != r)),
            (Self::Str(l), Self::Str(r)) => Some(Self::from(l != r)),
            (Self::Bool(l), Self::Bool(r)) => Some(Self::from(l != r)),
            (Self::Mut(m), other) => {
                {
                    let ref_m = &mut *m.borrow_mut();
                    *ref_m = mem::take(ref_m).try_ne(other)?;
                }
                Some(Self::Mut(m))
            }
            (self_, Self::Mut(m)) => self_.try_ne(m.borrow().clone()),
            _ => None,
        }
    }
}

pub mod value_set {
    use crate::{Type, ValueObj};
    use erg_common::set::Set;

    // false -> SyntaxError
    pub fn is_homogeneous(set: &Set<ValueObj>) -> bool {
        let l_first = set.iter().next().unwrap().class();
        set.iter().all(|c| c.class() == l_first)
    }

    pub fn inner_class(set: &Set<ValueObj>) -> Type {
        set.iter().next().unwrap().class()
    }

    pub fn max(set: &Set<ValueObj>) -> Option<ValueObj> {
        if !is_homogeneous(set) {
            return None;
        }
        set.iter()
            .max_by(|x, y| x.try_cmp(y).unwrap())
            .map(Clone::clone)
    }

    pub fn min(set: &Set<ValueObj>) -> Option<ValueObj> {
        if !is_homogeneous(set) {
            return None;
        }
        set.iter()
            .min_by(|x, y| x.try_cmp(y).unwrap())
            .map(Clone::clone)
    }
}
