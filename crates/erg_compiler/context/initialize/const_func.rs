use std::fmt::Display;
use std::mem;
use std::path::Path;

use erg_common::dict::Dict;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::traits::Stream;
use erg_common::{dict, set};

use crate::context::eval::UndoableLinkedList;
use crate::context::initialize::closed_range;
use crate::context::Context;
use crate::feature_error;
use crate::ty::constructors::{and, mono, tuple_t, v_enum};
use crate::ty::value::{EvalValueError, EvalValueResult, GenTypeObj, TypeObj, ValueObj};
use crate::ty::{Field, TyParam, Type, ValueArgs};
use erg_common::error::{ErrorCore, ErrorKind, Location, SubMessage};
use erg_common::style::{Color, StyledStr, StyledString, THEME};

const ERR: Color = THEME.colors.error;
const WARN: Color = THEME.colors.warning;

fn not_passed(t: impl Display) -> EvalValueError {
    let text = t.to_string();
    let param = StyledStr::new(&text, Some(ERR), None);
    ErrorCore::new(
        vec![SubMessage::only_loc(Location::Unknown)],
        format!("{param} is not passed"),
        line!() as usize,
        ErrorKind::KeyError,
        Location::Unknown,
    )
    .into()
}

fn no_key(slf: impl Display, key: impl Display) -> EvalValueError {
    ErrorCore::new(
        vec![SubMessage::only_loc(Location::Unknown)],
        format!("{slf} has no key {key}"),
        line!() as usize,
        ErrorKind::KeyError,
        Location::Unknown,
    )
    .into()
}

fn type_mismatch(expected: impl Display, got: impl Display, param: &str) -> EvalValueError {
    let got = StyledString::new(format!("{got}"), Some(ERR), None);
    let param = StyledStr::new(param, Some(WARN), None);
    ErrorCore::new(
        vec![SubMessage::only_loc(Location::Unknown)],
        format!("non-{expected} object {got} is passed to {param}"),
        line!() as usize,
        ErrorKind::TypeError,
        Location::Unknown,
    )
    .into()
}

/// Base := Type or NoneType, Impl := Type -> ClassType
pub(crate) fn class_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let base = args.remove_left_or_key("Base");
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.and_then(|v| v.as_type(ctx));
    let t = mono(ctx.name.clone());
    match base {
        Some(value) => {
            if let Some(base) = value.as_type(ctx) {
                Ok(ValueObj::gen_t(GenTypeObj::class(t, Some(base), impls, true)).into())
            } else {
                Err(type_mismatch("type", value, "Base"))
            }
        }
        None => Ok(ValueObj::gen_t(GenTypeObj::class(t, None, impls, true)).into()),
    }
}

/// Super: ClassType, Impl := Type, Additional := Type -> ClassType
pub(crate) fn inherit_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let sup = args
        .remove_left_or_key("Super")
        .ok_or_else(|| not_passed("Super"))?;
    let Some(sup) = sup.as_type(ctx) else {
        return Err(type_mismatch("class", sup, "Super"));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.and_then(|v| v.as_type(ctx));
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.and_then(|v| v.as_type(ctx));
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::inherited(t, sup, impls, additional)).into())
}

/// Class: ClassType -> ClassType (with `InheritableType`)
/// This function is used by the compiler to mark a class as inheritable and does nothing in terms of actual operation.
pub(crate) fn inheritable_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let class = args
        .remove_left_or_key("Class")
        .ok_or_else(|| not_passed("Class"))?;
    match class {
        ValueObj::Type(TypeObj::Generated(mut gen)) => {
            if let Some(typ) = gen.impls_mut() {
                match typ.as_mut().map(|x| x.as_mut()) {
                    Some(TypeObj::Generated(gen)) => {
                        *gen.typ_mut() = and(mem::take(gen.typ_mut()), mono("InheritableType"));
                    }
                    Some(TypeObj::Builtin { t, .. }) => {
                        *t = and(mem::take(t), mono("InheritableType"));
                    }
                    _ => {
                        *typ = Some(Box::new(TypeObj::builtin_trait(mono("InheritableType"))));
                    }
                }
            }
            Ok(ValueObj::Type(TypeObj::Generated(gen)).into())
        }
        other => feature_error!(
            EvalValueError,
            _ctx,
            Location::Unknown,
            &format!("Inheritable {other}")
        ),
    }
}

pub(crate) fn override_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let func = args
        .remove_left_or_key("func")
        .ok_or_else(|| not_passed("func"))?;
    Ok(func.into())
}

/// Base: Type, Impl := Type -> TraitType
pub(crate) fn trait_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let req = args
        .remove_left_or_key("Requirement")
        .ok_or_else(|| not_passed("Requirement"))?;
    let Some(req) = req.as_type(ctx) else {
        return Err(type_mismatch("type", req, "Requirement"));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.and_then(|v| v.as_type(ctx));
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::trait_(t, req, impls, true)).into())
}

/// Base: Type, Impl := Type -> Patch
pub(crate) fn patch_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let base = args
        .remove_left_or_key("Base")
        .ok_or_else(|| not_passed("Base"))?;
    let Some(base) = base.as_type(ctx) else {
        return Err(type_mismatch("type", base, "Base"));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.and_then(|v| v.as_type(ctx));
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::patch(t, base, impls)).into())
}

/// Super: TraitType, Impl := Type, Additional := Type -> TraitType
pub(crate) fn subsume_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let sup = args
        .remove_left_or_key("Super")
        .ok_or_else(|| not_passed("Super"))?;
    let Some(sup) = sup.as_type(ctx) else {
        return Err(type_mismatch("trait", sup, "Super"));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.and_then(|v| v.as_type(ctx));
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.and_then(|v| v.as_type(ctx));
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::subsumed(t, sup, impls, additional)).into())
}

pub(crate) fn structural_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let type_ = args
        .remove_left_or_key("Type")
        .ok_or_else(|| not_passed("Type"))?;
    let Some(base) = type_.as_type(ctx) else {
        return Err(type_mismatch("type", type_, "Type"));
    };
    let t = base.typ().clone().structuralize();
    Ok(ValueObj::gen_t(GenTypeObj::structural(t, base)).into())
}

pub(crate) fn __list_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let slf = match ctx.convert_value_into_list(slf) {
        Ok(slf) => slf,
        Err(val) => {
            return Err(type_mismatch("List", val, "Self"));
        }
    };
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    let Ok(index) = usize::try_from(&index) else {
        return Err(type_mismatch("Nat", index, "Index"));
    };
    if let Some(v) = slf.get(index) {
        Ok(v.clone().into())
    } else {
        Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!(
                "[{}] has {} elements, but accessed {}th element",
                erg_common::fmt_vec(&slf),
                slf.len(),
                index
            ),
            line!() as usize,
            ErrorKind::IndexError,
            Location::Unknown,
        )
        .into())
    }
}

pub(crate) fn sub_vdict_get<'d>(
    dict: &'d Dict<ValueObj, ValueObj>,
    key: &ValueObj,
    ctx: &Context,
) -> Option<&'d ValueObj> {
    let mut matches = vec![];
    for (k, v) in dict.iter() {
        match (key, k) {
            (ValueObj::Type(idx), ValueObj::Type(kt))
                if ctx.subtype_of(&idx.typ().lower_bounded(), &kt.typ().lower_bounded()) =>
            {
                matches.push((idx, kt, v));
            }
            (idx, k) if idx == k => {
                return Some(v);
            }
            _ => {}
        }
    }
    for (idx, kt, v) in matches.into_iter() {
        let list = UndoableLinkedList::new();
        match ctx.undoable_sub_unify(idx.typ(), kt.typ(), &(), &list, None) {
            Ok(_) => {
                return Some(v);
            }
            Err(_err) => {
                erg_common::log!(err "{idx} <!: {kt} => {v}");
            }
        }
    }
    None
}

pub(crate) fn sub_tpdict_get<'d>(
    dict: &'d Dict<TyParam, TyParam>,
    key: &TyParam,
    ctx: &Context,
) -> Option<&'d TyParam> {
    let mut matches = vec![];
    for (k, v) in dict.iter() {
        match (<&Type>::try_from(key), <&Type>::try_from(k)) {
            (Ok(idx), Ok(kt))
                if ctx.subtype_of(&idx.lower_bounded(), &kt.lower_bounded()) || dict.len() == 1 =>
            {
                matches.push((idx, kt, v));
            }
            (_, _) if key == k => {
                return Some(v);
            }
            _ => {}
        }
    }
    for (idx, kt, v) in matches.into_iter() {
        let list = UndoableLinkedList::new();
        match ctx.undoable_sub_unify(idx, kt, &(), &list, None) {
            Ok(_) => {
                return Some(v);
            }
            Err(_err) => {
                erg_common::log!(err "{idx} <!: {kt} => {v}");
            }
        }
    }
    None
}

/// `{{"a"}: Int, {"b"}: Float} ==> {{"a", "b"}: Float}`
fn homogenize_dict_type(dict: &Dict<Type, Type>, ctx: &Context) -> Dict<Type, Type> {
    let mut union_key = Type::Never;
    let mut union_value = Type::Never;
    for (k, v) in dict.iter() {
        union_key = ctx.union(&union_key, k);
        union_value = ctx.union(&union_value, v);
    }
    dict! { union_key => union_value }
}

/// see `homogenize_dict_type`
fn homogenize_dict(dict: &Dict<ValueObj, ValueObj>, ctx: &Context) -> Dict<ValueObj, ValueObj> {
    let mut type_dict = Dict::new();
    for (k, v) in dict.iter() {
        match (k, v) {
            (ValueObj::Type(k), ValueObj::Type(v)) => {
                type_dict.insert(k.typ().clone(), v.typ().clone());
            }
            _ => {
                return dict.clone();
            }
        }
    }
    let dict_t = homogenize_dict_type(&type_dict, ctx);
    let mut value_dict = Dict::new();
    for (k, v) in dict_t.iter() {
        let k = ValueObj::builtin_type(k.clone());
        let v = ValueObj::builtin_type(v.clone());
        value_dict.insert(k, v);
    }
    value_dict
}

pub(crate) fn __dict_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Dict(slf) = slf else {
        return Err(type_mismatch("Dict", slf, "Self"));
    };
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    if let Some(v) = slf.get(&index).or_else(|| sub_vdict_get(&slf, &index, ctx)) {
        Ok(v.clone().into())
    } else if let Some(v) = sub_vdict_get(&homogenize_dict(&slf, ctx), &index, ctx).cloned() {
        Ok(v.into())
    } else {
        let index = if let ValueObj::Type(t) = &index {
            let derefed = ctx.coerce(t.typ().clone(), &()).unwrap_or(t.typ().clone());
            ValueObj::builtin_type(derefed)
        } else {
            index
        };
        Err(no_key(slf, index))
    }
}

/// `{Str: Int, Int: Float}.keys() == Str or Int`
pub(crate) fn dict_keys(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Dict(slf) = slf else {
        return Err(type_mismatch("Dict", slf, "Self"));
    };
    let dict_type = slf
        .iter()
        .map(|(k, v)| {
            let k = ctx.convert_value_into_type(k.clone())?;
            let v = ctx.convert_value_into_type(v.clone())?;
            Ok((k, v))
        })
        .collect::<Result<Dict<_, _>, ValueObj>>();
    if let Ok(slf) = dict_type {
        let union = slf
            .keys()
            .fold(Type::Never, |union, t| ctx.union(&union, t));
        // let keys = poly(DICT_KEYS, vec![ty_tp(union)]);
        Ok(ValueObj::builtin_type(union).into())
    } else {
        Ok(ValueObj::List(slf.into_keys().collect::<Vec<_>>().into()).into())
    }
}

/// `{Str: Int, Int: Float}.values() == Int or Float`
pub(crate) fn dict_values(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Dict(slf) = slf else {
        return Err(type_mismatch("Dict", slf, "Self"));
    };
    let dict_type = slf
        .iter()
        .map(|(k, v)| {
            let k = ctx.convert_value_into_type(k.clone())?;
            let v = ctx.convert_value_into_type(v.clone())?;
            Ok((k, v))
        })
        .collect::<Result<Dict<_, _>, ValueObj>>();
    if let Ok(slf) = dict_type {
        let union = slf
            .values()
            .fold(Type::Never, |union, t| ctx.union(&union, t));
        // let values = poly(DICT_VALUES, vec![ty_tp(union)]);
        Ok(ValueObj::builtin_type(union).into())
    } else {
        Ok(ValueObj::List(slf.into_values().collect::<Vec<_>>().into()).into())
    }
}

/// `{Str: Int, Int: Float}.items() == (Str, Int) or (Int, Float)`
pub(crate) fn dict_items(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Dict(slf) = slf else {
        return Err(type_mismatch("Dict", slf, "Self"));
    };
    let dict_type = slf
        .iter()
        .map(|(k, v)| {
            let k = ctx.convert_value_into_type(k.clone())?;
            let v = ctx.convert_value_into_type(v.clone())?;
            Ok((k, v))
        })
        .collect::<Result<Dict<_, _>, ValueObj>>();
    if let Ok(slf) = dict_type {
        let union = slf.iter().fold(Type::Never, |union, (k, v)| {
            ctx.union(&union, &tuple_t(vec![k.clone(), v.clone()]))
        });
        // let items = poly(DICT_ITEMS, vec![ty_tp(union)]);
        Ok(ValueObj::builtin_type(union).into())
    } else {
        Ok(ValueObj::List(
            slf.into_iter()
                .map(|(k, v)| ValueObj::Tuple(vec![k, v].into()))
                .collect::<Vec<_>>()
                .into(),
        )
        .into())
    }
}

/// If the key is duplicated, the value of the right dict is used.
/// `{Str: Int, Int: Float}.concat({Int: Str, Float: Bool}) == {Str: Int, Int: Str, Float: Bool}`
pub(crate) fn dict_concat(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Dict(slf) = slf else {
        return Err(type_mismatch("Dict", slf, "Self"));
    };
    let other = args
        .remove_left_or_key("Other")
        .ok_or_else(|| not_passed("Other"))?;
    let ValueObj::Dict(other) = other else {
        return Err(type_mismatch("Dict", other, "Other"));
    };
    Ok(ValueObj::Dict(slf.concat(other)).into())
}

pub(crate) fn dict_diff(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Dict(slf) = slf else {
        return Err(type_mismatch("Dict", slf, "Self"));
    };
    let other = args
        .remove_left_or_key("Other")
        .ok_or_else(|| not_passed("Other"))?;
    let ValueObj::Dict(other) = other else {
        return Err(type_mismatch("Dict", other, "Other"));
    };
    Ok(ValueObj::Dict(slf.diff(&other)).into())
}

/// `[Int, Str].union() == Int or Str`
pub(crate) fn list_union(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::List(slf) = slf else {
        return Err(type_mismatch("List", slf, "Self"));
    };
    let slf = slf
        .iter()
        .flat_map(|t| ctx.convert_value_into_type(t.clone()))
        .collect::<Vec<_>>();
    let union = slf
        .iter()
        .fold(Type::Never, |union, t| ctx.union(&union, t));
    Ok(ValueObj::builtin_type(union).into())
}

fn _lis_shape(arr: ValueObj, ctx: &Context) -> Result<Vec<TyParam>, String> {
    let mut shape = vec![];
    let mut arr = arr;
    loop {
        match arr {
            ValueObj::List(a) => {
                shape.push(ValueObj::from(a.len()).into());
                match a.first() {
                    Some(arr_ @ (ValueObj::List(_) | ValueObj::Type(_))) => {
                        arr = arr_.clone();
                    }
                    _ => {
                        break;
                    }
                }
            }
            ValueObj::Type(ref t) if &t.typ().qual_name()[..] == "List" => {
                let mut tps = t.typ().typarams();
                let elem = match ctx.convert_tp_into_type(tps.remove(0)) {
                    Ok(elem) => elem,
                    Err(err) => {
                        return Err(err.to_string());
                    }
                };
                let len = tps.remove(0);
                shape.push(len);
                arr = ValueObj::builtin_type(elem);
            }
            _ => {
                break;
            }
        }
    }
    Ok(shape)
}

/// ```erg
/// List(Int, 2).shape() == [2,]
/// List(List(Int, 2), N).shape() == [N, 2]
/// [1, 2].shape() == [2,]
/// [[1, 2], [3, 4], [5, 6]].shape() == [3, 2]
/// ```
pub(crate) fn list_shape(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let res = _lis_shape(val, ctx).unwrap();
    let lis = TyParam::List(res);
    Ok(lis)
}

fn _list_scalar_type(mut typ: Type, ctx: &Context) -> Result<Type, String> {
    loop {
        if matches!(&typ.qual_name()[..], "List" | "List!" | "UnsizedList") {
            let tp = typ.typarams().remove(0);
            match ctx.convert_tp_into_type(tp) {
                Ok(typ_) => {
                    typ = typ_;
                }
                Err(err) => {
                    return Err(format!("Cannot convert {err} into type"));
                }
            }
        } else {
            return Ok(typ);
        }
    }
}

pub(crate) fn list_scalar_type(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let Ok(slf) = ctx.convert_value_into_type(slf.clone()) else {
        return Err(type_mismatch("Type", slf, "Self"));
    };
    let res = _list_scalar_type(slf, ctx).unwrap();
    Ok(TyParam::t(res))
}

fn _scalar_type(mut value: ValueObj, _ctx: &Context) -> Result<Type, String> {
    loop {
        match value {
            ValueObj::List(a) => match a.first() {
                Some(elem) => {
                    value = elem.clone();
                }
                None => {
                    return Ok(Type::Never);
                }
            },
            ValueObj::Set(s) => match s.iter().next() {
                Some(elem) => {
                    value = elem.clone();
                }
                None => {
                    return Ok(Type::Never);
                }
            },
            ValueObj::Tuple(t) => match t.first() {
                Some(elem) => {
                    value = elem.clone();
                }
                None => {
                    return Ok(Type::Never);
                }
            },
            ValueObj::UnsizedList(a) => {
                value = *a.clone();
            }
            other => {
                return Ok(other.class());
            }
        }
    }
}

/// ```erg
/// [1, 2].scalar_type() == Nat
/// [[1, 2], [3, 4], [5, 6]].scalar_type() == Nat
/// ```
#[allow(unused)]
pub(crate) fn scalar_type(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let res = _scalar_type(val, ctx).unwrap();
    let lis = TyParam::t(res);
    Ok(lis)
}

fn _list_sum(arr: ValueObj, _ctx: &Context) -> Result<ValueObj, String> {
    match arr {
        ValueObj::List(a) => {
            let mut sum = 0f64;
            for v in a.iter() {
                match v {
                    ValueObj::Nat(n) => {
                        sum += *n as f64;
                    }
                    ValueObj::Int(n) => {
                        sum += *n as f64;
                    }
                    ValueObj::Float(n) => {
                        sum += *n;
                    }
                    ValueObj::Inf => {
                        return Ok(ValueObj::Inf);
                    }
                    ValueObj::NegInf => {
                        return Ok(ValueObj::NegInf);
                    }
                    _ => {
                        return Err(format!("Cannot sum {v}"));
                    }
                }
            }
            if sum.round() == sum && sum >= 0.0 {
                Ok(ValueObj::Nat(sum as u64))
            } else if sum.round() == sum {
                Ok(ValueObj::Int(sum as i32))
            } else {
                Ok(ValueObj::Float(sum))
            }
        }
        _ => Err(format!("Cannot sum {arr}")),
    }
}

/// ```erg
/// [1, 2].sum() == [3,]
/// ```
pub(crate) fn list_sum(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let res = _list_sum(val, ctx).unwrap();
    let lis = TyParam::Value(res);
    Ok(lis)
}

fn _list_prod(lis: ValueObj, _ctx: &Context) -> Result<ValueObj, String> {
    match lis {
        ValueObj::List(a) => {
            let mut prod = 1f64;
            for v in a.iter() {
                match v {
                    ValueObj::Nat(n) => {
                        prod *= *n as f64;
                    }
                    ValueObj::Int(n) => {
                        prod *= *n as f64;
                    }
                    ValueObj::Float(n) => {
                        prod *= *n;
                    }
                    ValueObj::Inf => {
                        return Ok(ValueObj::Inf);
                    }
                    ValueObj::NegInf => {
                        return Ok(ValueObj::NegInf);
                    }
                    _ => {
                        return Err(format!("Cannot prod {v}"));
                    }
                }
            }
            if prod.round() == prod && prod >= 0.0 {
                Ok(ValueObj::Nat(prod as u64))
            } else if prod.round() == prod {
                Ok(ValueObj::Int(prod as i32))
            } else {
                Ok(ValueObj::Float(prod))
            }
        }
        _ => Err(format!("Cannot prod {lis}")),
    }
}

/// ```erg
/// [1, 2].prod() == [2,]
/// ```
pub(crate) fn list_prod(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let res = _list_prod(val, ctx).unwrap();
    let lis = TyParam::Value(res);
    Ok(lis)
}

fn _list_reversed(lis: ValueObj, _ctx: &Context) -> Result<ValueObj, String> {
    match lis {
        ValueObj::List(a) => {
            let mut vec = a.to_vec();
            vec.reverse();
            Ok(ValueObj::List(vec.into()))
        }
        _ => Err(format!("Cannot reverse {lis}")),
    }
}

pub(crate) fn list_reversed(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let res = _list_reversed(val, ctx).unwrap();
    let lis = TyParam::Value(res);
    Ok(lis)
}

fn _list_insert_at(
    lis: ValueObj,
    index: usize,
    value: ValueObj,
    _ctx: &Context,
) -> Result<ValueObj, String> {
    match lis {
        ValueObj::List(a) => {
            let mut a = a.to_vec();
            if index > a.len() {
                return Err(format!("Index out of range: {index}"));
            }
            a.insert(index, value);
            Ok(ValueObj::List(a.into()))
        }
        _ => Err(format!("Cannot insert into {lis}")),
    }
}

pub(crate) fn list_insert_at(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let lis = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    let value = args
        .remove_left_or_key("Value")
        .ok_or_else(|| not_passed("Value"))?;
    let Ok(index) = usize::try_from(&index) else {
        return Err(type_mismatch("Nat", index, "Index"));
    };
    let res = _list_insert_at(lis, index, value, ctx).unwrap();
    let lis = TyParam::Value(res);
    Ok(lis)
}

fn _list_remove_at(lis: ValueObj, index: usize, _ctx: &Context) -> Result<ValueObj, String> {
    match lis {
        ValueObj::List(a) => {
            let mut a = a.to_vec();
            if index >= a.len() {
                return Err(format!("Index out of range: {index}"));
            }
            a.remove(index);
            Ok(ValueObj::List(a.into()))
        }
        _ => Err(format!("Cannot remove from {lis}")),
    }
}

pub(crate) fn list_remove_at(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    let Ok(index) = usize::try_from(&index) else {
        return Err(type_mismatch("Nat", index, "Index"));
    };
    let res = _list_remove_at(val, index, ctx).unwrap();
    let lis = TyParam::Value(res);
    Ok(lis)
}

fn _list_remove_all(lis: ValueObj, value: ValueObj, _ctx: &Context) -> Result<ValueObj, String> {
    match lis {
        ValueObj::List(a) => {
            let mut a = a.to_vec();
            a.retain(|v| v != &value);
            Ok(ValueObj::List(a.into()))
        }
        _ => Err(format!("Cannot remove from {lis}")),
    }
}

pub(crate) fn list_remove_all(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let value = args
        .remove_left_or_key("Value")
        .ok_or_else(|| not_passed("Value"))?;
    let res = _list_remove_all(val, value, ctx).unwrap();
    let lis = TyParam::Value(res);
    Ok(lis)
}

pub(crate) fn __range_getitem__(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::DataClass { name: _, fields } = slf else {
        return Err(type_mismatch("Range", slf, "Self"));
    };
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    let Ok(index) = usize::try_from(&index) else {
        return Err(type_mismatch("Nat", index, "Index"));
    };
    let start = fields
        .get("start")
        .ok_or_else(|| no_key(&fields, "start"))?;
    let Ok(start) = usize::try_from(start) else {
        return Err(type_mismatch("Nat", start, "start"));
    };
    let end = fields.get("end").ok_or_else(|| no_key(&fields, "end"))?;
    let Ok(end) = usize::try_from(end) else {
        return Err(type_mismatch("Nat", end, "end"));
    };
    // FIXME <= if inclusive
    if start + index < end {
        Ok(ValueObj::Nat((start + index) as u64).into())
    } else {
        Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("Index out of range: {index}"),
            line!() as usize,
            ErrorKind::IndexError,
            Location::Unknown,
        )
        .into())
    }
}

pub(crate) fn __named_tuple_getitem__(
    mut args: ValueArgs,
    ctx: &Context,
) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let fields = match ctx.convert_value_into_type(slf) {
        Ok(Type::NamedTuple(fields)) => fields,
        Ok(other) => {
            return Err(type_mismatch("NamedTuple", other, "Self"));
        }
        Err(val) => {
            return Err(type_mismatch("NamedTuple", val, "Self"));
        }
    };
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    let Ok(index) = usize::try_from(&index) else {
        return Err(type_mismatch("Nat", index, "Index"));
    };
    if let Some((_, t)) = fields.get(index) {
        Ok(TyParam::t(t.clone()))
    } else {
        Err(no_key(Type::NamedTuple(fields), index))
    }
}

/// `NamedTuple({ .x = Int; .y = Str }).union() == Int or Str`
/// `GenericNamedTuple.union() == Obj`
pub(crate) fn named_tuple_union(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let fields = match ctx.convert_value_into_type(slf) {
        Ok(Type::NamedTuple(fields)) => fields,
        Ok(Type::Mono(n)) if &n == "GenericNamedTuple" => {
            return Ok(ValueObj::builtin_type(Type::Obj).into());
        }
        Ok(other) => {
            return Err(type_mismatch("NamedTuple", other, "Self"));
        }
        Err(val) => {
            return Err(type_mismatch("NamedTuple", val, "Self"));
        }
    };
    let union = fields
        .iter()
        .fold(Type::Never, |union, (_, t)| ctx.union(&union, t));
    Ok(ValueObj::builtin_type(union).into())
}

/// `{ .x = Int; .y = Str }.as_dict() == { "x": Int, "y": Str }`
/// `Record.as_dict() == { Obj: Obj }`
pub(crate) fn as_dict(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let fields = match ctx.convert_value_into_type(slf) {
        Ok(Type::Record(fields)) => fields,
        Ok(Type::Mono(n)) if &n == "Record" => {
            let dict = dict! { Type::Obj => Type::Obj };
            return Ok(ValueObj::builtin_type(Type::from(dict)).into());
        }
        Ok(other) => {
            return Err(type_mismatch("Record", other, "Self"));
        }
        Err(val) => {
            return Err(type_mismatch("Record", val, "Self"));
        }
    };
    let dict = fields
        .into_iter()
        .map(|(k, v)| (v_enum(set! {  k.symbol.into() }), v))
        .collect::<Dict<_, _>>();
    Ok(ValueObj::builtin_type(Type::from(dict)).into())
}

/// `{ {"x"}: Int, {"y"}: Str }.as_record() == { .x = Int, .y = Str }`
pub(crate) fn as_record(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let fields = match ctx.convert_value_into_type(slf) {
        Ok(Type::Poly { name, params }) if &name == "Dict" => {
            Dict::try_from(params[0].clone()).unwrap()
        }
        Ok(other) => {
            return Err(type_mismatch("Dict", other, "Self"));
        }
        Err(val) => {
            return Err(type_mismatch("Dict", val, "Self"));
        }
    };
    let mut dict = Dict::new();
    for (k, v) in fields {
        match (ctx.convert_tp_into_type(k), ctx.convert_tp_into_type(v)) {
            (Ok(k_t), Ok(v_t)) => {
                if let Some(values) = k_t.refinement_values() {
                    for value in values {
                        if let TyParam::Value(ValueObj::Str(field)) = value {
                            dict.insert(Field::public(field.clone()), v_t.clone());
                        } else {
                            return Err(type_mismatch("Str", value, "Key"));
                        }
                    }
                } else {
                    return Err(type_mismatch("Str refinement type", k_t, "Key"));
                }
            }
            (Ok(_), Err(err)) | (Err(err), Ok(_)) => {
                return Err(type_mismatch("Type", err, "Self"));
            }
            (Err(k), Err(_v)) => {
                return Err(type_mismatch("Type", k, "Self"));
            }
        };
    }
    Ok(ValueObj::builtin_type(Type::Record(dict)).into())
}

pub(crate) fn int_abs(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let Some(slf) = slf.as_int() else {
        return Err(type_mismatch("Int", slf, "self"));
    };
    Ok(ValueObj::Int(slf.abs()).into())
}

pub(crate) fn str_endswith(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let suffix = args
        .remove_left_or_key("suffix")
        .ok_or_else(|| not_passed("suffix"))?;
    let Some(slf) = slf.as_str() else {
        return Err(type_mismatch("Str", slf, "self"));
    };
    let Some(suffix) = suffix.as_str() else {
        return Err(type_mismatch("Str", suffix, "suffix"));
    };
    Ok(ValueObj::Bool(slf.ends_with(&suffix[..])).into())
}

pub(crate) fn str_find(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let sub = args
        .remove_left_or_key("sub")
        .ok_or_else(|| not_passed("sub"))?;
    let Some(slf) = slf.as_str() else {
        return Err(type_mismatch("Str", slf, "self"));
    };
    let Some(sub) = sub.as_str() else {
        return Err(type_mismatch("Str", sub, "sub"));
    };
    Ok(ValueObj::Int(slf.find(&sub[..]).map_or(-1, |i| i as i32)).into())
}

pub(crate) fn str_isalpha(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let Some(slf) = slf.as_str() else {
        return Err(type_mismatch("Str", slf, "self"));
    };
    Ok(ValueObj::Bool(slf.chars().all(|c| c.is_alphabetic())).into())
}

pub(crate) fn str_isascii(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let Some(slf) = slf.as_str() else {
        return Err(type_mismatch("Str", slf, "self"));
    };
    Ok(ValueObj::Bool(slf.is_ascii()).into())
}

pub(crate) fn str_isdecimal(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let Some(slf) = slf.as_str() else {
        return Err(type_mismatch("Str", slf, "self"));
    };
    Ok(ValueObj::Bool(slf.chars().all(|c| c.is_ascii_digit())).into())
}

pub(crate) fn str_join(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let iterable = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let Some(slf) = slf.as_str() else {
        return Err(type_mismatch("Str", slf, "self"));
    };
    let arr = match iterable {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        ValueObj::Dict(d) => d.into_keys().collect(),
        _ => {
            return Err(type_mismatch("Iterable(Str)", iterable, "iterable"));
        }
    };
    let mut joined = String::new();
    for v in arr.iter() {
        let Some(v) = v.as_str() else {
            return Err(type_mismatch("Str", v, "arr.next()"));
        };
        joined.push_str(&v[..]);
        joined.push_str(&slf[..]);
    }
    joined.pop();
    Ok(ValueObj::Str(joined.into()).into())
}

pub(crate) fn str_replace(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let old = args
        .remove_left_or_key("old")
        .ok_or_else(|| not_passed("old"))?;
    let new = args
        .remove_left_or_key("new")
        .ok_or_else(|| not_passed("new"))?;
    let Some(slf) = slf.as_str() else {
        return Err(type_mismatch("Str", slf, "self"));
    };
    let Some(old) = old.as_str() else {
        return Err(type_mismatch("Str", old, "old"));
    };
    let Some(new) = new.as_str() else {
        return Err(type_mismatch("Str", new, "new"));
    };
    Ok(ValueObj::Str(slf.replace(&old[..], new).into()).into())
}

pub(crate) fn str_startswith(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("self")
        .ok_or_else(|| not_passed("self"))?;
    let prefix = args
        .remove_left_or_key("prefix")
        .ok_or_else(|| not_passed("prefix"))?;
    let Some(slf) = slf.as_str() else {
        return Err(type_mismatch("Str", slf, "self"));
    };
    let Some(prefix) = prefix.as_str() else {
        return Err(type_mismatch("Str", prefix, "prefix"));
    };
    Ok(ValueObj::Bool(slf.starts_with(&prefix[..])).into())
}

pub(crate) fn abs_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let num = args
        .remove_left_or_key("num")
        .ok_or_else(|| not_passed("num"))?;
    match num {
        ValueObj::Nat(n) => Ok(ValueObj::Nat(n).into()),
        ValueObj::Int(n) => Ok(ValueObj::Nat(n.unsigned_abs() as u64).into()),
        ValueObj::Bool(b) => Ok(ValueObj::Nat(b as u64).into()),
        ValueObj::Float(n) => Ok(ValueObj::Float(n.abs()).into()),
        ValueObj::Inf => Ok(ValueObj::Inf.into()),
        ValueObj::NegInf => Ok(ValueObj::Inf.into()),
        _ => Err(type_mismatch("Num", num, "num")),
    }
}

pub(crate) fn all_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let iterable = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let arr = match iterable {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        _ => {
            return Err(type_mismatch("Iterable(Bool)", iterable, "iterable"));
        }
    };
    let mut all = true;
    for v in arr.iter() {
        match v {
            ValueObj::Bool(b) => {
                all &= *b;
            }
            _ => {
                return Err(type_mismatch("Bool", v, "iterable.next()"));
            }
        }
    }
    Ok(ValueObj::Bool(all).into())
}

pub(crate) fn any_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let iterable = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let arr = match iterable {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        _ => {
            return Err(type_mismatch("Iterable(Bool)", iterable, "iterable"));
        }
    };
    let mut any = false;
    for v in arr.iter() {
        match v {
            ValueObj::Bool(b) => {
                any |= *b;
            }
            _ => {
                return Err(type_mismatch("Bool", v, "iterable.next()"));
            }
        }
    }
    Ok(ValueObj::Bool(any).into())
}

pub(crate) fn filter_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let func = args
        .remove_left_or_key("func")
        .ok_or_else(|| not_passed("func"))?;
    let iterable = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let arr = match iterable {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        _ => {
            return Err(type_mismatch("Iterable(T)", iterable, "iterable"));
        }
    };
    let subr = match func {
        ValueObj::Subr(f) => f,
        _ => {
            return Err(type_mismatch("Subr", func, "func"));
        }
    };
    let mut filtered = vec![];
    for v in arr.into_iter() {
        let args = ValueArgs::pos_only(vec![v.clone()]);
        match ctx.call(subr.clone(), args, Location::Unknown) {
            Ok(res) => match ctx.convert_tp_into_value(res) {
                Ok(res) => {
                    if res.is_true() {
                        filtered.push(v);
                    }
                }
                Err(tp) => {
                    return Err(type_mismatch("Bool", tp, "func"));
                }
            },
            Err((_res, mut err)) => {
                return Err(EvalValueError::from(*err.remove(0).core));
            }
        }
    }
    Ok(TyParam::Value(ValueObj::List(filtered.into())))
}

pub(crate) fn len_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let container = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let len = match container {
        ValueObj::List(a) => a.len(),
        ValueObj::Tuple(t) => t.len(),
        ValueObj::Set(s) => s.len(),
        ValueObj::Dict(d) => d.len(),
        ValueObj::Record(r) => r.len(),
        ValueObj::Str(s) => s.len(),
        _ => {
            return Err(type_mismatch("Container", container, "container"));
        }
    };
    Ok(ValueObj::Nat(len as u64).into())
}

pub(crate) fn map_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let func = args
        .remove_left_or_key("func")
        .ok_or_else(|| not_passed("func"))?;
    let iterable = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let arr = match iterable {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        _ => {
            return Err(type_mismatch("Iterable(Bool)", iterable, "iterable"));
        }
    };
    let subr = match func {
        ValueObj::Subr(f) => f,
        _ => {
            return Err(type_mismatch("Subr", func, "func"));
        }
    };
    let mut mapped = vec![];
    for v in arr.into_iter() {
        let args = ValueArgs::pos_only(vec![v]);
        match ctx.call(subr.clone(), args, Location::Unknown) {
            Ok(res) => {
                mapped.push(res);
            }
            Err((_res, mut err)) => {
                return Err(EvalValueError::from(*err.remove(0).core));
            }
        }
    }
    Ok(TyParam::List(mapped))
}

pub(crate) fn max_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let iterable = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let arr = match iterable {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        _ => {
            return Err(type_mismatch("Iterable(Ord)", iterable, "iterable"));
        }
    };
    let mut max = ValueObj::NegInf;
    if arr.is_empty() {
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            "max() arg is an empty sequence",
            line!() as usize,
            ErrorKind::ValueError,
            Location::Unknown,
        )
        .into());
    }
    for v in arr.into_iter() {
        if v.is_num() {
            if max.clone().try_lt(v.clone()).is_some_and(|b| b.is_true()) {
                max = v;
            }
        } else {
            return Err(type_mismatch("Ord", v, "iterable.next()"));
        }
    }
    Ok(max.into())
}

pub(crate) fn min_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let iterable = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let arr = match iterable {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        _ => {
            return Err(type_mismatch("Iterable(Ord)", iterable, "iterable"));
        }
    };
    let mut min = ValueObj::Inf;
    if arr.is_empty() {
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            "min() arg is an empty sequence",
            line!() as usize,
            ErrorKind::ValueError,
            Location::Unknown,
        )
        .into());
    }
    for v in arr.into_iter() {
        if v.is_num() {
            if min.clone().try_gt(v.clone()).is_some_and(|b| b.is_true()) {
                min = v;
            }
        } else {
            return Err(type_mismatch("Ord", v, "iterable.next()"));
        }
    }
    Ok(min.into())
}

pub(crate) fn not_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("val")
        .ok_or_else(|| not_passed("val"))?;
    match val {
        ValueObj::Bool(b) => Ok(ValueObj::Bool(!b).into()),
        _ => Err(type_mismatch("Bool", val, "val")),
    }
}

pub(crate) fn reversed_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let reversible = args
        .remove_left_or_key("reversible")
        .ok_or_else(|| not_passed("reversible"))?;
    let arr = match reversible {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        _ => {
            return Err(type_mismatch("Reversible", reversible, "reversible"));
        }
    };
    let mut reversed = vec![];
    for v in arr.into_iter().rev() {
        reversed.push(v);
    }
    Ok(TyParam::Value(ValueObj::List(reversed.into())))
}

pub(crate) fn str_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("val")
        .ok_or_else(|| not_passed("val"))?;
    Ok(ValueObj::Str(val.to_string().into()).into())
}

pub(crate) fn sum_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let iterable = args
        .remove_left_or_key("iterable")
        .ok_or_else(|| not_passed("iterable"))?;
    let arr = match iterable {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        ValueObj::Dict(d) => d.into_keys().collect(),
        ValueObj::Record(r) => r.into_values().collect(),
        _ => {
            return Err(type_mismatch("Iterable(Add)", iterable, "iterable"));
        }
    };
    let mut sum = ValueObj::Nat(0);
    for v in arr.into_iter() {
        if v.is_num() {
            sum = sum.try_add(v).unwrap();
        } else {
            return Err(type_mismatch("Add", v, "iterable.next()"));
        }
    }
    Ok(sum.into())
}

pub(crate) fn resolve_path_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let path = args
        .remove_left_or_key("Path")
        .ok_or_else(|| not_passed("Path"))?;
    let path = match &path {
        ValueObj::Str(s) => Path::new(&s[..]),
        other => {
            return Err(type_mismatch("Str", other, "Path"));
        }
    };
    let Some(path) = ctx.cfg.input.resolve_path(path, &ctx.cfg) else {
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("Path {} is not found", path.display()),
            line!() as usize,
            ErrorKind::IoError,
            Location::Unknown,
        )
        .into());
    };
    Ok(ValueObj::Str(path.to_string_lossy().into()).into())
}

pub(crate) fn resolve_decl_path_func(
    mut args: ValueArgs,
    ctx: &Context,
) -> EvalValueResult<TyParam> {
    let path = args
        .remove_left_or_key("Path")
        .ok_or_else(|| not_passed("Path"))?;
    let path = match &path {
        ValueObj::Str(s) => Path::new(&s[..]),
        other => {
            return Err(type_mismatch("Str", other, "Path"));
        }
    };
    let Some(path) = ctx.cfg.input.resolve_decl_path(path, &ctx.cfg) else {
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("Path {} is not found", path.display()),
            line!() as usize,
            ErrorKind::IoError,
            Location::Unknown,
        )
        .into());
    };
    Ok(ValueObj::Str(path.to_string_lossy().into()).into())
}

pub(crate) fn succ_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Value")
        .ok_or_else(|| not_passed("Value"))?;
    let val = match &val {
        ValueObj::Bool(b) => ValueObj::Nat(*b as u64 + 1),
        ValueObj::Nat(n) => ValueObj::Nat(n + 1),
        ValueObj::Int(n) => ValueObj::Int(n + 1),
        ValueObj::Float(n) => ValueObj::Float(n + f64::EPSILON),
        v @ (ValueObj::Inf | ValueObj::NegInf) => v.clone(),
        _ => {
            return Err(type_mismatch("Number", val, "Value"));
        }
    };
    Ok(val.into())
}

pub(crate) fn pred_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("Value")
        .ok_or_else(|| not_passed("Value"))?;
    let val = match &val {
        ValueObj::Bool(b) => ValueObj::Nat((*b as u64).saturating_sub(1)),
        ValueObj::Nat(n) => ValueObj::Nat(n.saturating_sub(1)),
        ValueObj::Int(n) => ValueObj::Int(n - 1),
        ValueObj::Float(n) => ValueObj::Float(n - f64::EPSILON),
        v @ (ValueObj::Inf | ValueObj::NegInf) => v.clone(),
        _ => {
            return Err(type_mismatch("Number", val, "Value"));
        }
    };
    Ok(val.into())
}

// TODO: varargs
pub(crate) fn zip_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<TyParam> {
    let iterable1 = args
        .remove_left_or_key("iterable1")
        .ok_or_else(|| not_passed("iterable1"))?;
    let iterable2 = args
        .remove_left_or_key("iterable2")
        .ok_or_else(|| not_passed("iterable2"))?;
    let iterable1 = match iterable1 {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        _ => {
            return Err(type_mismatch("Iterable(T)", iterable1, "iterable1"));
        }
    };
    let iterable2 = match iterable2 {
        ValueObj::List(a) => a.to_vec(),
        ValueObj::Tuple(t) => t.to_vec(),
        ValueObj::Set(s) => s.into_iter().collect(),
        _ => {
            return Err(type_mismatch("Iterable(T)", iterable2, "iterable2"));
        }
    };
    let mut zipped = vec![];
    for (v1, v2) in iterable1.into_iter().zip(iterable2.into_iter()) {
        zipped.push(ValueObj::Tuple(vec![v1, v2].into()));
    }
    Ok(TyParam::Value(ValueObj::List(zipped.into())))
}

/// ```erg
/// derefine({X: T | ...}) == T
/// derefine({1}) == Nat
/// derefine(List!({1, 2}, 2)) == List!(Nat, 2)
/// ```
pub(crate) fn derefine_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("T")
        .ok_or_else(|| not_passed("T"))?;
    let t = match ctx.convert_value_into_type(val) {
        Ok(t) => t.derefine(),
        Err(val) => {
            return Err(type_mismatch("Type", val, "T"));
        }
    };
    Ok(TyParam::t(t))
}

/// ```erg
/// fill_ord({1, 4}) == {1, 2, 3, 4}
/// fill_ord({"a", "c"}) == {"a", "b", "c"}
/// ```
pub(crate) fn fill_ord_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let val = args
        .remove_left_or_key("T")
        .ok_or_else(|| not_passed("T"))?;
    let t = match ctx.convert_value_into_type(val) {
        Ok(t) => {
            let coerced = ctx.coerce(t.clone(), &()).unwrap_or(t);
            let inf = ctx.inf(&coerced);
            let sup = ctx.sup(&coerced);
            let der = coerced.derefine();
            match (inf, sup) {
                (Some(inf), Some(sup)) => closed_range(der, inf, sup),
                _ => coerced,
            }
        }
        Err(val) => {
            return Err(type_mismatch("Type", val, "T"));
        }
    };
    Ok(TyParam::t(t))
}
