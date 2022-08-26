use crate::*;

#[inline]
pub const fn param_t(name: &'static str, ty: Type) -> ParamTy {
    ParamTy::new(Some(Str::ever(name)), ty)
}

#[inline]
pub const fn anon(ty: Type) -> ParamTy {
    ParamTy::anonymous(ty)
}

#[inline]
pub fn mono<S: Into<Str>>(name: S) -> Type {
    Type::mono(name)
}

#[inline]
pub fn mono_q<S: Into<Str>>(name: S) -> Type {
    Type::mono_q(name)
}

#[inline]
pub fn mono_proj<S: Into<Str>>(lhs: Type, rhs: S) -> Type {
    Type::mono_proj(lhs, rhs)
}

#[inline]
pub fn poly<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Type {
    Type::poly(name, params)
}

#[inline]
pub fn poly_q<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Type {
    Type::poly_q(name, params)
}

#[inline]
pub fn func(non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, ret: Type) -> Type {
    Type::func(non_default_params, default_params, ret)
}

#[inline]
pub fn proc(non_default_params: Vec<ParamTy>, default_params: Vec<ParamTy>, ret: Type) -> Type {
    Type::proc(non_default_params, default_params, ret)
}

#[inline]
pub fn nd_func(params: Vec<ParamTy>, ret: Type) -> Type {
    Type::nd_func(params, ret)
}

#[inline]
pub fn nd_proc(params: Vec<ParamTy>, ret: Type) -> Type {
    Type::nd_proc(params, ret)
}

#[inline]
pub fn fn0_met(self_t: Type, return_t: Type) -> Type {
    Type::fn0_met(self_t, return_t)
}

#[inline]
pub fn fn1_met(self_t: Type, input_t: Type, return_t: Type) -> Type {
    Type::fn1_met(self_t, input_t, return_t)
}

#[inline]
pub fn quant(unbound_t: Type, bounds: Set<TyBound>) -> Type {
    Type::quantified(unbound_t, bounds)
}

#[inline]
pub fn instance(name: Str, t: Type) -> TyBound {
    TyBound::instance(name, t)
}

#[inline]
pub fn static_instance(name: &'static str, t: Type) -> TyBound {
    TyBound::static_instance(name, t)
}

/// Sub <: Sup
#[inline]
pub fn subtypeof(sub: Type, sup: Type) -> TyBound {
    TyBound::sandwiched(Type::Never, sub, sup)
}

#[inline]
pub fn mono_q_tp<S: Into<Str>>(name: S) -> TyParam {
    TyParam::mono_q(name)
}

#[inline]
pub fn mono_tp<S: Into<Str>>(name: S) -> TyParam {
    TyParam::mono(name)
}

#[inline]
pub fn ty_tp(t: Type) -> TyParam {
    TyParam::t(t)
}

#[inline]
pub fn value<V: Into<ValueObj>>(v: V) -> TyParam {
    TyParam::value(v)
}
