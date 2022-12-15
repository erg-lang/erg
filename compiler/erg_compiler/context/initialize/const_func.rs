use std::mem;

use erg_common::enum_unwrap;

use crate::context::Context;
use crate::feature_error;
use crate::ty::constructors::{and, mono};
use crate::ty::value::{EvalValueError, EvalValueResult, GenTypeObj, TypeObj, ValueObj};
use crate::ty::ValueArgs;
use erg_common::error::{ErrorCore, ErrorKind, Location, SubMessage};
use erg_common::style::{Color, StyledStr, StyledString, THEME};

const ERR: Color = THEME.colors.error;
const WARN: Color = THEME.colors.warning;

const SUP_ERR: StyledStr = StyledStr::new("Super", Some(ERR), None);
const SUP_WARN: StyledStr = StyledStr::new("Super", Some(WARN), None);
const CLASS_ERR: StyledStr = StyledStr::new("Class", Some(ERR), None);
const REQ_ERR: StyledStr = StyledStr::new("Requirement", Some(ERR), None);
const REQ_WARN: StyledStr = StyledStr::new("Requirement", Some(WARN), None);

/// Requirement: Type, Impl := Type -> ClassType
pub fn class_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let require = args.remove_left_or_key("Requirement").ok_or_else(|| {
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{REQ_ERR} is not passed"),
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
        )
    })?;
    let Some(require) = require.as_type() else {
        let require = StyledString::new(format!("{}", require), Some(ERR), None);
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!(
                "non-type object {require} is passed to {REQ_WARN}",
            ),
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
        ).into());
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::class(t, require, impls)))
}

/// Super: ClassType, Impl := Type, Additional := Type -> ClassType
pub fn inherit_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
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
pub fn inheritable_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
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
                    Some(TypeObj::Builtin(t)) => {
                        *t = and(mem::take(t), mono("InheritableType"));
                    }
                    _ => {
                        *typ = Some(Box::new(TypeObj::Builtin(mono("InheritableType"))));
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

/// Requirement: Type, Impl := Type -> TraitType
pub fn trait_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let require = args.remove_left_or_key("Requirement").ok_or_else(|| {
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{REQ_ERR} is not passed"),
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
        )
    })?;
    let Some(require) = require.as_type() else {
        let require = StyledString::new(format!("{require}"), Some(ERR), None);
        return Err(ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!(
                "non-type object {require} is passed to {REQ_WARN}",
            ),
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
        ).into());
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(GenTypeObj::trait_(t, require, impls)))
}

/// Base: Type, Impl := Type -> Patch
pub fn patch_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let base = args.remove_left_or_key("Base").ok_or_else(|| {
        ErrorCore::new(
            vec![SubMessage::only_loc(Location::Unknown)],
            format!("{REQ_ERR} is not passed"),
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
                "non-type object {base} is passed to {REQ_WARN}",
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
pub fn subsume_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
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

pub fn __array_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
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

pub fn __dict_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let slf = args.remove_left_or_key("Self").unwrap();
    let slf = enum_unwrap!(slf, ValueObj::Dict);
    let index = args.remove_left_or_key("Index").unwrap();
    if let Some(v) = slf.get(&index).or_else(|| {
        for (k, v) in slf.iter() {
            match (&index, k) {
                (ValueObj::Type(idx), ValueObj::Type(kt)) => {
                    if ctx.subtype_of(idx.typ(), kt.typ()) {
                        return Some(v);
                    }
                }
                (idx, k) => {
                    if idx == k {
                        return Some(v);
                    }
                }
            }
        }
        None
    }) {
        Ok(v.clone())
    } else {
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

pub fn __range_getitem__(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
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
            format!("Index out of range: {}", index),
            line!() as usize,
            ErrorKind::IndexError,
            Location::Unknown,
        )
        .into())
    }
}
