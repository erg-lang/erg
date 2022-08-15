//! defines `ValueObj` (used in the compiler, VM).
//!
//! コンパイラ、VM等で使われる(データも保持した)値オブジェクトを定義する
use std::cmp::Ordering;
use std::fmt::{self, Write};
use std::hash::{Hash, Hasher};
use std::ops::Neg;
use std::rc::Rc;

use crate::codeobj::CodeObj;
use crate::serialize::*;
use crate::set;
use crate::traits::HasType;
use crate::ty::{fresh_varname, ConstObj, Predicate, TyParam, Type};
use crate::{fmt_iter, impl_display_from_debug, switch_lang};
use crate::{RcArray, Str};

/// 値オブジェクト
/// コンパイル時評価ができ、シリアライズも可能
#[derive(Clone)]
pub enum ValueObj {
    Int(i32),
    Nat(u64),
    Float(f64),
    Str(Str),
    True,
    False,
    Array(Rc<[ValueObj]>),
    Dict(Rc<[(ValueObj, ValueObj)]>),
    Code(Box<CodeObj>),
    None,
    Ellipsis,
    NotImplemented,
    NegInf,
    Inf,
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
            Self::True => write!(f, "True"),
            Self::False => write!(f, "False"),
            Self::Array(arr) => write!(f, "[{}]", fmt_iter(arr.iter())),
            Self::Dict(dict) => {
                let mut s = "".to_string();
                for (k, v) in dict.iter() {
                    write!(s, "{k}: {v}, ")?;
                }
                s.pop();
                s.pop();
                write!(f, "[{s}]")
            }
            Self::Code(code) => write!(f, "{code}"),
            Self::None => write!(f, "None"),
            Self::Ellipsis => write!(f, "Ellipsis"),
            Self::NotImplemented => write!(f, "NotImplemented"),
            Self::NegInf => write!(f, "-Inf"),
            Self::Inf => write!(f, "Inf"),
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

impl PartialEq for ValueObj {
    fn eq(&self, other: &ValueObj) -> bool {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => i == j,
            (Self::Nat(n), Self::Nat(m)) => n == m,
            (Self::Float(fl), Self::Float(fr)) => fl == fr,
            (Self::Str(s), Self::Str(t)) => s == t,
            (Self::True, Self::True) => true,
            (Self::False, Self::False) => true,
            (Self::Array(arr), Self::Array(arr2)) => arr == arr2,
            (Self::Dict(dict), Self::Dict(dict2)) => dict == dict2,
            (Self::Code(code), Self::Code(code2)) => code == code2,
            (Self::None, Self::None) => true,
            (Self::Ellipsis, Self::Ellipsis) => true,
            (Self::NotImplemented, Self::NotImplemented) => true,
            (Self::NegInf, Self::NegInf) => true,
            (Self::Inf, Self::Inf) => true,
            (Self::Illegal, Self::Illegal) => true,
            _ => false,
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
            Self::True => true.hash(state),
            Self::False => false.hash(state),
            Self::Array(arr) => arr.hash(state),
            Self::Dict(dict) => dict.hash(state),
            Self::Code(code) => code.hash(state),
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
        if item {
            ValueObj::True
        } else {
            ValueObj::False
        }
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
            _ => Err(()),
        }
    }
}

impl HasType for ValueObj {
    fn ref_t(&self) -> &Type {
        panic!("cannot get reference of the const")
    }
    /// その要素だけの集合型を返す、クラスが欲しい場合は.classで
    #[inline]
    fn t(&self) -> Type {
        let name = Str::from(fresh_varname());
        let pred = Predicate::eq(
            name.clone(),
            TyParam::ConstObj(ConstObj::Value(self.clone())),
        );
        Type::refinement(name, self.class(), set! {pred})
    }
    fn signature_t(&self) -> Option<&Type> {
        None
    }
}

impl ValueObj {
    pub const fn is_num(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Nat(_) | Self::Float(_))
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
            Type::Bool => {
                if &content[..] == "True" {
                    Self::True
                } else {
                    Self::False
                }
            }
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
            Self::True => vec![DataTypePrefix::True as u8],
            Self::False => vec![DataTypePrefix::False as u8],
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
                        format!("this object cannot be serialized: {other}"),
                        format!("このオブジェクトはシリアライズできません: {other}")
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
            Self::True | Self::False => Type::Bool,
            // TODO:
            Self::Array(arr) => Type::array(
                arr.iter().next().unwrap().class(),
                TyParam::value(arr.len()),
            ),
            Self::Dict(_dict) => todo!(),
            Self::Code(_) => Type::Code,
            Self::None => Type::NoneType,
            Self::Ellipsis => Type::Ellipsis,
            Self::NotImplemented => Type::NotImplemented,
            Self::Inf => Type::Inf,
            Self::NegInf => Type::NegInf,
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
            /* (Self::PlusEpsilon(l), r) => l.try_cmp(r)
                .map(|o| if matches!(o, Ordering::Equal) { Ordering::Less } else { o }),
            (l, Self::PlusEpsilon(r)) => l.try_cmp(r)
                .map(|o| if matches!(o, Ordering::Equal) { Ordering::Greater } else { o }),
            */
            // TODO: cmp with str
            (_s, _o) => None,
        }
    }

    // REVIEW: allow_divergenceオプションを付けるべきか?
    pub fn try_add(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l + r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l + r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l + r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Int(l + r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 + r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float(l + r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::Float(l as f64 + r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::Float(l + r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::Float(l as f64 + r)),
            (Self::Str(l), Self::Str(r)) => Some(Self::Str(Str::from(format!("{}{}", l, r)))),
            (inf @ (Self::Inf | Self::NegInf), _) | (_, inf @ (Self::Inf | Self::NegInf)) => {
                Some(inf)
            }
            _ => None,
        }
    }

    pub fn try_sub(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l - r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l - r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l - r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Int(l - r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 - r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float(l - r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::Float(l as f64 - r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::Float(l - r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::Float(l as f64 - r)),
            (inf @ (Self::Inf | Self::NegInf), other)
            | (other, inf @ (Self::Inf | Self::NegInf))
                if other != Self::Inf && other != Self::NegInf =>
            {
                Some(inf)
            }
            _ => None,
        }
    }

    pub fn try_mul(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l * r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l * r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l * r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Int(l * r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 * r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float(l * r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::Float(l as f64 * r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::Float(l * r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::Float(l as f64 * r)),
            (Self::Str(l), Self::Nat(r)) => Some(Self::Str(Str::from(l.repeat(r as usize)))),
            (inf @ (Self::Inf | Self::NegInf), _) | (_, inf @ (Self::Inf | Self::NegInf)) => {
                Some(inf)
            }
            _ => None,
        }
    }

    pub fn try_div(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(l), Self::Int(r)) => Some(Self::Int(l / r)),
            (Self::Nat(l), Self::Nat(r)) => Some(Self::Nat(l / r)),
            (Self::Float(l), Self::Float(r)) => Some(Self::Float(l / r)),
            (Self::Int(l), Self::Nat(r)) => Some(Self::Int(l / r as i32)),
            (Self::Nat(l), Self::Int(r)) => Some(Self::Int(l as i32 / r)),
            (Self::Float(l), Self::Nat(r)) => Some(Self::Float(l / r as f64)),
            (Self::Nat(l), Self::Float(r)) => Some(Self::Float(l as f64 / r)),
            (Self::Float(l), Self::Int(r)) => Some(Self::Float(l / r as f64)),
            (Self::Int(l), Self::Float(r)) => Some(Self::Float(l as f64 / r)),
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
            (Self::True, Self::True) | (Self::False, Self::False) => Some(Self::True),
            (Self::True, Self::False) | (Self::False, Self::True) => Some(Self::False),
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
            (Self::True, Self::True) | (Self::False, Self::False) => Some(Self::False),
            (Self::True, Self::False) | (Self::False, Self::True) => Some(Self::True),
            _ => None,
        }
    }
}
