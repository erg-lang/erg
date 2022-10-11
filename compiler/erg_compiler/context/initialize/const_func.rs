use std::mem;

use erg_common::enum_unwrap;

use crate::context::Context;
use crate::ty::constructors::{and, mono};
use crate::ty::value::{EvalValueResult, TypeKind, TypeObj, ValueObj};
use crate::ty::ValueArgs;
use erg_common::astr::AtomicStr;
use erg_common::color::{RED, RESET, YELLOW};
use erg_common::error::{ErrorCore, ErrorKind, Location};

/// Requirement: Type, Impl := Type -> ClassType
pub fn class_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let require = args.remove_left_or_key("Requirement").ok_or_else(|| {
        ErrorCore::new(
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
            AtomicStr::from(format!("{RED}Requirement{RESET} is not passed")),
            None,
        )
    })?;
    let require = if let Some(t) = require.as_type() {
        t
    } else {
        return Err(ErrorCore::new(
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
            AtomicStr::from(format!(
                "non-type object {RED}{require}{RESET} is passed to {YELLOW}Requirement{RESET}",
            )),
            None,
        ));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(TypeKind::Class, t, require, impls, None))
}

/// Super: ClassType, Impl := Type, Additional := Type -> ClassType
pub fn inherit_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let sup = args.remove_left_or_key("Super").ok_or_else(|| {
        ErrorCore::new(
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
            AtomicStr::from(format!("{RED}Super{RESET} is not passed")),
            None,
        )
    })?;
    let sup = if let Some(t) = sup.as_type() {
        t
    } else {
        return Err(ErrorCore::new(
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
            AtomicStr::from(format!(
                "non-class object {RED}{sup}{RESET} is passed to {YELLOW}Super{RESET}",
            )),
            None,
        ));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(
        TypeKind::Subclass,
        t,
        sup,
        impls,
        additional,
    ))
}

/// Class: ClassType -> ClassType (with `InheritableType`)
/// This function is used by the compiler to mark a class as inheritable and does nothing in terms of actual operation.
pub fn inheritable_func(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let class = args.remove_left_or_key("Class").ok_or_else(|| {
        ErrorCore::new(
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
            AtomicStr::from(format!("{RED}Class{RESET} is not passed")),
            None,
        )
    })?;
    match class {
        ValueObj::Type(TypeObj::Generated(mut gen)) => {
            if let Some(typ) = &mut gen.impls {
                match typ.as_mut() {
                    TypeObj::Generated(gen) => {
                        gen.t = and(mem::take(&mut gen.t), mono("InheritableType"));
                    }
                    TypeObj::Builtin(t) => {
                        *t = and(mem::take(t), mono("InheritableType"));
                    }
                }
            } else {
                gen.impls = Some(Box::new(TypeObj::Builtin(mono("InheritableType"))));
            }
            Ok(ValueObj::Type(TypeObj::Generated(gen)))
        }
        _ => todo!(),
    }
}

/// Requirement: Type, Impl := Type -> TraitType
pub fn trait_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let require = args.remove_left_or_key("Requirement").ok_or_else(|| {
        ErrorCore::new(
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
            AtomicStr::from(format!("{RED}Requirement{RESET} is not passed")),
            None,
        )
    })?;
    let require = if let Some(t) = require.as_type() {
        t
    } else {
        return Err(ErrorCore::new(
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
            AtomicStr::from(format!(
                "non-type object {RED}{require}{RESET} is passed to {YELLOW}Requirement{RESET}",
            )),
            None,
        ));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(TypeKind::Trait, t, require, impls, None))
}

/// Super: TraitType, Impl := Type, Additional := Type -> TraitType
pub fn subsume_func(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let sup = args.remove_left_or_key("Super").ok_or_else(|| {
        ErrorCore::new(
            line!() as usize,
            ErrorKind::KeyError,
            Location::Unknown,
            AtomicStr::from(format!("{RED}Super{RESET} is not passed")),
            None,
        )
    })?;
    let sup = if let Some(t) = sup.as_type() {
        t
    } else {
        return Err(ErrorCore::new(
            line!() as usize,
            ErrorKind::TypeError,
            Location::Unknown,
            AtomicStr::from(format!(
                "non-trait object {RED}{sup}{RESET} is passed to {YELLOW}Super{RESET}",
            )),
            None,
        ));
    };
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(|v| v.as_type().unwrap());
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(|v| v.as_type().unwrap());
    let t = mono(ctx.name.clone());
    Ok(ValueObj::gen_t(
        TypeKind::Subtrait,
        t,
        sup,
        impls,
        additional,
    ))
}

pub fn __array_getitem__(mut args: ValueArgs, _ctx: &Context) -> EvalValueResult<ValueObj> {
    let _self = enum_unwrap!(args.remove_left_or_key("Self").unwrap(), ValueObj::Array);
    let index = enum_unwrap!(args.remove_left_or_key("Index").unwrap(), ValueObj::Nat);
    let res = _self[index as usize].clone();
    Ok(res)
}

pub fn __dict_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
    let _self = args.remove_left_or_key("Self").unwrap();
    let _self = enum_unwrap!(_self, ValueObj::Dict);
    let index = args.remove_left_or_key("Index").unwrap();
    let res = _self
        .get(&index)
        .or_else(|| {
            for (k, v) in _self.iter() {
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
        })
        .unwrap()
        .clone();
    Ok(res)
}
