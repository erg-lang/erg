use std::fmt::Display;
use std::mem;

use erg_common::dict::Dict;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::{enum_unwrap, ArcArray};

use crate::context::Context;
use crate::feature_error;
use crate::ty::constructors::{and, mono, poly, tuple_t, ty_tp};
use crate::ty::value::{EvalValueError, EvalValueResult, GenTypeObj, TypeObj, ValueObj};
use crate::ty::{TyParam, Type, ValueArgs};
use erg_common::error::{ErrorCore, ErrorKind, Location, SubMessage};
use erg_common::style::{Color, StyledStr, StyledString, THEME};

use super::{DICT_ITEMS, DICT_KEYS, DICT_VALUES};

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
pub(crate) fn class_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let base = args.remove_left_or_key("Base");
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
    let t = mono(ctx.name.clone());
    match base {
        Some(value) => {
            if let Some(base) = value.as_type(ctx) {
                Ok(ValueObj::gen_t(GenTypeObj::class(t, Some(base), impls)))
            } else {
                Err(type_mismatch("type", value, "Base"))
            }
        }
        None => Ok(ValueObj::gen_t(GenTypeObj::class(t, None, impls))),
    }
}

/// Super: ClassType, Impl := Type, Additional := Type -> ClassType
pub(crate) fn inherit_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let sup = args
        .remove_left_or_key("Super")
        .ok_or_else(|| not_passed("Super"))?;
    let Some(sup) = sup.as_type(ctx) else {
        return Err(type_mismatch("class", sup, "Super"));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(|v| v.as_type(ctx).unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::inherited(
        t, sup, impls, additional,
    )))
}

/// Class: ClassType -> ClassType (with `InheritableType`)
/// This function is used by the compiler to mark a class as inheritable and does nothing in terms of actual operation.
pub(crate) fn inheritable_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
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
            Ok(ValueObj::Type(TypeObj::Generated(gen)))
        }
        other => feature_error!(
            EvalValueError,
            _ctx,
            Location::Unknown,
            &format!("Inheritable {other}")
        ),
    }
}

/// Base: Type, Impl := Type -> TraitType
pub(crate) fn trait_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let req = args
        .remove_left_or_key("Requirement")
        .ok_or_else(|| not_passed("Requirement"))?;
    let Some(req) = req.as_type(ctx) else {
        return Err(type_mismatch("type", req, "Requirement"));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::trait_(t, req, impls)))
}

/// Base: Type, Impl := Type -> Patch
pub(crate) fn patch_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let base = args
        .remove_left_or_key("Base")
        .ok_or_else(|| not_passed("Base"))?;
    let Some(base) = base.as_type(ctx) else {
        return Err(type_mismatch("type", base, "Base"));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::patch(t, base, impls)))
}

/// Super: TraitType, Impl := Type, Additional := Type -> TraitType
pub(crate) fn subsume_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let sup = args
        .remove_left_or_key("Super")
        .ok_or_else(|| not_passed("Super"))?;
    let Some(sup) = sup.as_type(ctx) else {
        return Err(type_mismatch("trait", sup, "Super"));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(|v| v.as_type(ctx).unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::subsumed(
        t, sup, impls, additional,
    )))
}

pub(crate) fn structural_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let type_ = args
        .remove_left_or_key("Type")
        .ok_or_else(|| not_passed("Type"))?;
    let Some(base) = type_.as_type(ctx) else {
        return Err(type_mismatch("type", type_, "Type"));
    };
    let t = base.typ().clone().structuralize();
    Ok(ValueObj::gen_t(GenTypeObj::structural(t, base)))
}

pub(crate) fn __array_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let slf = ctx
        .convert_value_into_array(slf)
        .unwrap_or_else(|err| panic!("{err}, {args}"));
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    let index = enum_unwrap!(index, ValueObj::Nat);
    if let Some(v) = slf.get(index as usize) {
        Ok(v.clone())
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
        match ctx.sub_unify(idx.typ(), kt.typ(), &(), None) {
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
        match ctx.sub_unify(idx, kt, &(), None) {
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

pub(crate) fn __dict_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    if let Some(v) = slf.get(&index).or_else(|| sub_vdict_get(&slf, &index, ctx)) {
        Ok(v.clone())
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

/// `{Str: Int, Int: Float}.keys() == DictKeys(Str or Int)`
pub(crate) fn dict_keys(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let slf = slf
        .into_iter()
        .map(|(k, v)| {
            (
                ctx.convert_value_into_type(k).unwrap(),
                ctx.convert_value_into_type(v).unwrap(),
            )
        })
        .collect::<Dict<_, _>>();
    let union = slf
        .keys()
        .fold(Type::Never, |union, t| ctx.union(&union, t));
    let keys = poly(DICT_KEYS, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_class(keys))
}

/// `{Str: Int, Int: Float}.values() == DictValues(Int or Float)`
pub(crate) fn dict_values(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let slf = slf
        .into_iter()
        .map(|(k, v)| {
            (
                ctx.convert_value_into_type(k).unwrap(),
                ctx.convert_value_into_type(v).unwrap(),
            )
        })
        .collect::<Dict<_, _>>();
    let union = slf
        .values()
        .fold(Type::Never, |union, t| ctx.union(&union, t));
    let values = poly(DICT_VALUES, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_class(values))
}

/// `{Str: Int, Int: Float}.items() == DictItems((Str, Int) or (Int, Float))`
pub(crate) fn dict_items(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let slf = slf
        .into_iter()
        .map(|(k, v)| {
            (
                ctx.convert_value_into_type(k).unwrap(),
                ctx.convert_value_into_type(v).unwrap(),
            )
        })
        .collect::<Dict<_, _>>();
    let union = slf.iter().fold(Type::Never, |union, (k, v)| {
        ctx.union(&union, &tuple_t(vec![k.clone(), v.clone()]))
    });
    let items = poly(DICT_ITEMS, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_class(items))
}

/// `[Int, Str].union() == Int or Str`
pub(crate) fn array_union(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let slf = enum_unwrap!(slf, ValueObj::Array);
    let slf = slf
        .iter()
        .map(|t| ctx.convert_value_into_type(t.clone()).unwrap())
        .collect::<Vec<_>>();
    let union = slf
        .iter()
        .fold(Type::Never, |union, t| ctx.union(&union, t));
    Ok(ValueObj::builtin_type(union))
}

// TODO
fn _arr_shape(arr: ArcArray<ValueObj>, ctx: &Context) -> Result<Vec<ValueObj>, String> {
    let mut shape = vec![];
    let mut arr = arr;
    loop {
        shape.push(ValueObj::from(arr.len()));
        match arr.get(0) {
            Some(ValueObj::Array(arr_)) => {
                arr = arr_.clone();
            }
            Some(ValueObj::Type(t)) => {
                let Ok(arr_) = ctx.convert_type_to_array(t.typ().clone()) else {
                    break;
                };
                arr = arr_.into();
            }
            _ => {
                break;
            }
        }
    }
    Ok(shape)
}

/// ```erg
/// Array(Int, 2).shape() == [2,]
/// Array(Array(Int, 2), N).shape() == [N, 2]
/// [1, 2].shape() == [2,]
/// [[1, 2], [3, 4], [5, 6]].shape() == [3, 2]
/// ```
pub(crate) fn array_shape(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let arr = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let arr = match arr {
        ValueObj::Array(arr) => arr,
        ValueObj::Type(t) => ctx
            .convert_type_to_array(t.into_typ())
            .map_err(|arr| type_mismatch("array", arr, "Self"))?
            .into(),
        _ => {
            return Err(type_mismatch("array", arr, "Self"));
        }
    };
    let res = _arr_shape(arr, ctx).unwrap();
    let arr = ValueObj::Array(ArcArray::from(res));
    Ok(arr)
}

pub(crate) fn __range_getitem__(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let (_name, fields) = enum_unwrap!(
        args.remove_left_or_key("Self")
            .ok_or_else(|| not_passed("Self"))?,
        ValueObj::DataClass { name, fields }
    );
    let index = args
        .remove_left_or_key("Index")
        .ok_or_else(|| not_passed("Index"))?;
    let index = enum_unwrap!(index, ValueObj::Nat);
    let start = fields
        .get("start")
        .ok_or_else(|| no_key(&fields, "start"))?;
    let start = *enum_unwrap!(start, ValueObj::Nat);
    let end = fields.get("end").ok_or_else(|| no_key(&fields, "end"))?;
    let end = *enum_unwrap!(end, ValueObj::Nat);
    // FIXME <= if inclusive
    if start + index < end {
        Ok(ValueObj::Nat(start + index))
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
