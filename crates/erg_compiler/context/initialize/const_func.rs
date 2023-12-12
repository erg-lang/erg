use std::fmt::Display;
use std::mem;

use erg_common::dict::Dict;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::{dict, set};

use crate::context::eval::UndoableLinkedList;
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
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
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
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(|v| v.as_type(ctx).unwrap());
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
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
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
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
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
    let impls = impls.map(|v| v.as_type(ctx).unwrap());
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(|v| v.as_type(ctx).unwrap());
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

pub(crate) fn __array_getitem__(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let slf = match ctx.convert_value_into_array(slf) {
        Ok(slf) => slf,
        Err(val) => {
            return Err(type_mismatch("Array", val, "Self"));
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
    // let keys = poly(DICT_KEYS, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_type(union).into())
}

/// `{Str: Int, Int: Float}.values() == Int or Float`
pub(crate) fn dict_values(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Dict(slf) = slf else {
        return Err(type_mismatch("Dict", slf, "Self"));
    };
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
    // let values = poly(DICT_VALUES, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_type(union).into())
}

/// `{Str: Int, Int: Float}.items() == (Str, Int) or (Int, Float)`
pub(crate) fn dict_items(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Dict(slf) = slf else {
        return Err(type_mismatch("Dict", slf, "Self"));
    };
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
    // let items = poly(DICT_ITEMS, vec![ty_tp(union)]);
    Ok(ValueObj::builtin_type(union).into())
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
pub(crate) fn array_union(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let slf = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let ValueObj::Array(slf) = slf else {
        return Err(type_mismatch("Array", slf, "Self"));
    };
    let slf = slf
        .iter()
        .map(|t| ctx.convert_value_into_type(t.clone()).unwrap())
        .collect::<Vec<_>>();
    let union = slf
        .iter()
        .fold(Type::Never, |union, t| ctx.union(&union, t));
    Ok(ValueObj::builtin_type(union).into())
}

fn _arr_shape(arr: ValueObj, ctx: &Context) -> Result<Vec<TyParam>, String> {
    let mut shape = vec![];
    let mut arr = arr;
    loop {
        match arr {
            ValueObj::Array(a) => {
                shape.push(ValueObj::from(a.len()).into());
                match a.get(0) {
                    Some(arr_ @ (ValueObj::Array(_) | ValueObj::Type(_))) => {
                        arr = arr_.clone();
                    }
                    _ => {
                        break;
                    }
                }
            }
            ValueObj::Type(ref t) if &t.typ().qual_name()[..] == "Array" => {
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
/// Array(Int, 2).shape() == [2,]
/// Array(Array(Int, 2), N).shape() == [N, 2]
/// [1, 2].shape() == [2,]
/// [[1, 2], [3, 4], [5, 6]].shape() == [3, 2]
/// ```
pub(crate) fn array_shape(mut args: ValueArgs, ctx: &Context) -> EvalValueResult<TyParam> {
    let arr = args
        .remove_left_or_key("Self")
        .ok_or_else(|| not_passed("Self"))?;
    let res = _arr_shape(arr, ctx).unwrap();
    let arr = TyParam::Array(res);
    Ok(arr)
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
