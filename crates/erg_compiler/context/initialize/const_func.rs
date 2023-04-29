use std::mem;

use erg_common::dict::Dict;
use erg_common::enum_unwrap;

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

const SUP_ERR: StyledStr = StyledStr::new("Super", Some(ERR), None);
const SUP_WARN: StyledStr = StyledStr::new("Super", Some(WARN), None);
const CLASS_ERR: StyledStr = StyledStr::new("Class", Some(ERR), None);
const REQ_ERR: StyledStr = StyledStr::new("Requirement", Some(ERR), None);
const REQ_WARN: StyledStr = StyledStr::new("Requirement", Some(WARN), None);
const BASE_ERR: StyledStr = StyledStr::new("Base", Some(ERR), None);
const BASE_WARN: StyledStr = StyledStr::new("Base", Some(WARN), None);

/// Base := Type or NoneType, Impl := Type -> ClassType
pub(crate) fn class_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let base = args.remove_left_or_key("Base");
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    match base {
        Some(value) => {
            if let Some(base) = value.as_type() {
                Ok(ValueObj::gen_t(GenTypeObj::class(t, Some(base), impls)))
            } else {
                let base = StyledString::new(format!("{value}"), Some(ERR), None);
                Err(ErrorCore::new(
                    vec![SubMessage::only_loc(Location::Unknown)],
                    format!("non-type object {base} is passed to {BASE_WARN}",),
                    line!() as usize,
                    ErrorKind::TypeError,
                    Location::Unknown,
                )
                .into())
            }
        }
        None => Ok(ValueObj::gen_t(GenTypeObj::class(t, None, impls))),
    }
}

/// Super: ClassType, Impl := Type, Additional := Type -> ClassType
pub(crate) fn inherit_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let sup = args.remove_left_or_key("Super").ok_or_else(|| {
        let sup = StyledStr::new("Super", Some(ERR), None);
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{sup} is not passed"),
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
        )
    })?;
    let Some(sup) = sup.as_type() else {
        let sup_ty = StyledString::new(format!("{sup}"), Some(ERR), None);
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!(
                "non-class object {sup_ty} is passed to {SUP_WARN}",
            ),
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
        ).into());
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::inherited(
        t, sup, impls, additional,
    )))
}

/// Class: ClassType -> ClassType (with `InheritableType`)
/// This function is used by the compiler to mark a class as inheritable and does nothing in terms of actual operation.
pub(crate) fn inheritable_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let class = args.remove_left_or_key("Class").ok_or_else(|| {
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{CLASS_ERR} is not passed"),
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
        )
    })?;
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
    let req = args.remove_left_or_key("Requirement").ok_or_else(|| {
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{REQ_ERR} is not passed"),
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
        )
    })?;
    let Some(req) = req.as_type() else {
        let req = StyledString::new(format!("{req}"), Some(ERR), None);
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!(
                "non-type object {req} is passed to {REQ_WARN}",
            ),
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
        ).into());
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::trait_(t, req, impls)))
}

/// Base: Type, Impl := Type -> Patch
pub(crate) fn patch_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let base = args.remove_left_or_key("Base").ok_or_else(|| {
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{BASE_ERR} is not passed"),
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
        )
    })?;
    let Some(base) = base.as_type() else {
        let base = StyledString::new(format!("{base}"), Some(ERR), None);
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!(
                "non-type object {base} is passed to {BASE_WARN}",
            ),
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
        ).into());
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::patch(t, base, impls)))
}

/// Super: TraitType, Impl := Type, Additional := Type -> TraitType
pub(crate) fn subsume_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let sup = args.remove_left_or_key("Super").ok_or_else(|| {
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{SUP_ERR} is not passed"),
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
        )
    })?;
    let Some(sup) = sup.as_type() else {
        let sup = StyledString::new(format!("{sup}"), Some(ERR), None);
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!(
                "non-trait object {sup} is passed to {SUP_WARN}",
            ),
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
        ).into());
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::subsumed(
        t, sup, impls, additional,
    )))
}

pub(crate) fn structural_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let type_ = args.remove_left_or_key("Type").ok_or_else(|| {
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{BASE_ERR} is not passed"),
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
        )
    })?;
    let Some(base) = type_.as_type() else {
        let type_ = StyledString::new(format!("{type_}"), Some(ERR), None);
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!(
                "non-type object {type_} is passed to {BASE_WARN}",
            ),
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
        ).into());
    };
    let t = base.typ().clone().structuralize();
    Ok(ValueObj::gen_t(GenTypeObj::structural(t, base)))
}

pub(crate) fn __array_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = ctx
        .convert_value_into_array(args.remove_left_or_key("Self").unwrap())
        .unwrap();
    let index = enum_unwrap!(args.remove_left_or_key("Index").unwrap(), ValueObj::Nat);
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
            (Ok(idx), Ok(kt)) if ctx.subtype_of(&idx.lower_bounded(), &kt.lower_bounded()) => {
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
    let slf = args.remove_left_or_key("Self").unwrap();
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let index = args.remove_left_or_key("Index").unwrap();
    if let Some(v) = slf.get(&index).or_else(|| sub_vdict_get(&slf, &index, ctx)) {
        Ok(v.clone())
    } else {
        let index = if let ValueObj::Type(t) = &index {
            let derefed = ctx.coerce(t.typ().clone(), &()).unwrap_or(t.typ().clone());
            ValueObj::builtin_type(derefed)
        } else {
            index
        };
        Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{slf} has no key {index}"),
            line!() as usize,
            ErrorKind::IndexError,
            Location::Unknown,
        )
        .into())
    }
}

/// `{Str: Int, Int: Float}.keys() == DictKeys(Str or Int)`
pub(crate) fn dict_keys(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args.remove_left_or_key("Self").unwrap();
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let slf = slf
        .into_iter()
        .map(|(k, v)| (Type::try_from(k).unwrap(), Type::try_from(v).unwrap()))
        .collect::<Dict<_, _>>();
    let union = slf
        .keys()
        .fold(Type::Never, |union, t| _ctx.union(&union, t));
    let keys = poly(DICT_KEYS, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_type(keys))
}

/// `{Str: Int, Int: Float}.values() == DictValues(Int or Float)`
pub(crate) fn dict_values(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args.remove_left_or_key("Self").unwrap();
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let slf = slf
        .into_iter()
        .map(|(k, v)| (Type::try_from(k).unwrap(), Type::try_from(v).unwrap()))
        .collect::<Dict<_, _>>();
    let union = slf
        .values()
        .fold(Type::Never, |union, t| _ctx.union(&union, t));
    let values = poly(DICT_VALUES, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_type(values))
}

/// `{Str: Int, Int: Float}.items() == DictItems((Str, Int) or (Int, Float))`
pub(crate) fn dict_items(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args.remove_left_or_key("Self").unwrap();
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let slf = slf
        .into_iter()
        .map(|(k, v)| (Type::try_from(k).unwrap(), Type::try_from(v).unwrap()))
        .collect::<Dict<_, _>>();
    let union = slf.iter().fold(Type::Never, |union, (k, v)| {
        _ctx.union(&union, &tuple_t(vec![k.clone(), v.clone()]))
    });
    let items = poly(DICT_ITEMS, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_type(items))
}

pub(crate) fn __range_getitem__(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let (_name, fields) = enum_unwrap!(
        args.remove_left_or_key("Self").unwrap(),
        ValueObj::DataClass { name, fields }
    );
    let index = enum_unwrap!(args.remove_left_or_key("Index").unwrap(), ValueObj::Nat);
    let start = fields.get("start").unwrap();
    let start = *enum_unwrap!(start, ValueObj::Nat);
    let end = fields.get("end").unwrap();
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
