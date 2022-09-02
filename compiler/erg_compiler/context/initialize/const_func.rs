use erg_common::Str;

use erg_type::constructors::class;
use erg_type::value::ValueObj;
use erg_type::ValueArgs;

/// Type -> Type
pub fn class_func(_args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    let t = class(__name__.unwrap_or(Str::ever("<Lambda>")));
    ValueObj::t(t)
}

/// Type -> Type
pub fn inherit_func(_args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    let t = class(__name__.unwrap_or(Str::ever("<Lambda>")));
    ValueObj::t(t)
}

/// Type -> Type
/// This function is used by the compiler to mark a class as inheritable and does nothing in terms of actual operation.
pub fn inheritable_func(args: ValueArgs, __name__: Option<Str>) -> ValueObj {
    args.pos_args.into_iter().next().unwrap()
}
