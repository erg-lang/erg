use erg_common::Str;

use erg_type::constructors::mono;
use erg_type::value::{TypeKind, TypeObj, ValueObj};
use erg_type::Type;
use erg_type::ValueArgs;

fn value_obj_to_t(value: ValueObj) -> TypeObj {
    match value {
        ValueObj::Type(t) => t,
        ValueObj::Record(rec) => TypeObj::Builtin(Type::Record(
            rec.into_iter()
                .map(|(k, v)| (k, value_obj_to_t(v).typ().clone()))
                .collect(),
        )),
        other => todo!("{other}"),
    }
}

/// Requirement: Type, Impl := Type -> Type
pub fn class_func(mut args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    let require = args.pos_args.remove(0);
    let require = value_obj_to_t(require);
    let impls = args.pos_args.pop().or_else(|| args.kw_args.remove("Impl"));
    let impls = impls.map(|v| value_obj_to_t(v));
    let t = mono(__name__.unwrap_or(Str::ever("<Lambda>")));
    ValueObj::gen_t(TypeKind::Class, t, require, impls, None)
}

/// Super: Type, Impl := Type, Additional := Type -> Type
pub fn inherit_func(mut args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    let sup = args.pos_args.remove(0);
    let sup = value_obj_to_t(sup);
    let impls = args.pos_args.pop().or_else(|| args.kw_args.remove("Impl"));
    let impls = impls.map(|v| value_obj_to_t(v));
    let additional = args
        .pos_args
        .pop()
        .or_else(|| args.kw_args.remove("Additional"));
    let additional = additional.map(|v| value_obj_to_t(v));
    let t = mono(__name__.unwrap_or(Str::ever("<Lambda>")));
    ValueObj::gen_t(TypeKind::InheritedClass, t, sup, impls, additional)
}

/// Type -> Type
/// This function is used by the compiler to mark a class as inheritable and does nothing in terms of actual operation.
pub fn inheritable_func(args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    args.pos_args.into_iter().next().unwrap()
}
