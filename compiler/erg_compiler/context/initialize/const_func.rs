use std::mem;

use erg_common::Str;

use erg_type::constructors::{and, mono};
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

/// Requirement: Type, Impl := Type -> ClassType
pub fn class_func(mut args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    let require = args.remove_left_or_key("Requirement").unwrap();
    let require = value_obj_to_t(require);
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(value_obj_to_t);
    let t = mono(__name__.unwrap_or(Str::ever("<Lambda>")));
    ValueObj::gen_t(TypeKind::Class, t, require, impls, None)
}

/// Super: Type, Impl := Type, Additional := Type -> ClassType
pub fn inherit_func(mut args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    let sup = args.remove_left_or_key("Super").unwrap();
    let sup = value_obj_to_t(sup);
    let impls = args.remove_left_or_key("Impl");
    let impls = impls.map(value_obj_to_t);
    let additional = args.remove_left_or_key("Additional");
    let additional = additional.map(value_obj_to_t);
    let t = mono(__name__.unwrap_or(Str::ever("<Lambda>")));
    ValueObj::gen_t(TypeKind::Subclass, t, sup, impls, additional)
}

/// Class: ClassType -> ClassType (with `InheritableType`)
/// This function is used by the compiler to mark a class as inheritable and does nothing in terms of actual operation.
pub fn inheritable_func(mut args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    let class = args.remove_left_or_key("Class").unwrap();
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
            ValueObj::Type(TypeObj::Generated(gen))
        }
        _ => todo!(),
    }
}
