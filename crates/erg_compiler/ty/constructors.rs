use std::convert::TryInto;

use erg_common::consts::PYTHON_MODE;
use erg_common::dict;
use erg_common::fresh::FRESH_GEN;

use crate::ty::*;

#[macro_export]
macro_rules! fn_t {
    ($input: expr => $ret: expr) => {
        Type::Subr($crate::ty::SubrType::new(
            $crate::ty::SubrKind::Func,
            vec![$crate::ty::ParamTy::Pos($input)],
            None,
            vec![],
            None,
            $ret,
        ))
    };
}

#[inline]
pub fn pos(ty: Type) -> ParamTy {
    ParamTy::Pos(ty)
}

#[inline]
pub fn kw(name: &'static str, ty: Type) -> ParamTy {
    ParamTy::kw(Str::ever(name), ty)
}

#[inline]
pub const fn kw_default(name: &'static str, ty: Type, default: Type) -> ParamTy {
    ParamTy::kw_default(Str::ever(name), ty, default)
}

#[inline]
pub const fn anon(ty: Type) -> ParamTy {
    ParamTy::Pos(ty)
}

#[inline]
pub fn free_var(level: usize, constraint: Constraint) -> Type {
    Type::FreeVar(Free::new_unbound(level, constraint.to_type_constraint()))
}

#[inline]
pub fn named_free_var(name: Str, level: usize, constraint: Constraint) -> Type {
    Type::FreeVar(Free::new_named_unbound(
        name,
        level,
        constraint.to_type_constraint(),
    ))
}

#[inline]
pub fn named_uninit_var(name: Str) -> Type {
    Type::FreeVar(Free::new_named_unbound(name, 1, Constraint::Uninited))
}

pub fn list_t(elem_t: Type, len: TyParam) -> Type {
    poly("List", vec![TyParam::t(elem_t), len])
}

pub fn list_mut(elem_t: Type, len: TyParam) -> Type {
    poly("List!", vec![TyParam::t(elem_t), len])
}

pub fn out_list_t(elem_t: Type, len: TyParam) -> Type {
    if PYTHON_MODE {
        list_mut(elem_t, len)
    } else {
        list_t(elem_t, len)
    }
}

pub fn unknown_len_list_t(elem_t: Type) -> Type {
    list_t(elem_t, TyParam::erased(Type::Nat))
}

pub fn unknown_len_list_mut(elem_t: Type) -> Type {
    list_mut(elem_t, TyParam::erased(Type::Nat))
}

pub fn out_unknown_len_list_t(elem_t: Type) -> Type {
    if PYTHON_MODE {
        unknown_len_list_mut(elem_t)
    } else {
        unknown_len_list_t(elem_t)
    }
}

pub fn str_dict_t(value: Type) -> Type {
    dict! { Type::Str => value }.into()
}

/// `UnsizedList` is a type of `[x; _]` (unsized list literal).
/// `UnsizedList(T) != List(T, _)`
pub fn unsized_list_t(elem_t: Type) -> Type {
    poly("UnsizedList", vec![TyParam::t(elem_t)])
}

pub fn tuple_t(args: Vec<Type>) -> Type {
    poly(
        "Tuple",
        vec![TyParam::List(args.into_iter().map(TyParam::t).collect())],
    )
}

pub fn homo_tuple_t(t: Type) -> Type {
    poly("HomogenousTuple", vec![TyParam::t(t)])
}

pub fn set_t(elem_t: Type, len: TyParam) -> Type {
    poly("Set", vec![TyParam::t(elem_t), len])
}

pub fn unknown_len_set_t(elem_t: Type) -> Type {
    set_t(elem_t, TyParam::erased(Type::Nat))
}

pub fn set_mut(elem_t: Type, len: TyParam) -> Type {
    poly("Set!", vec![TyParam::t(elem_t), len])
}

pub fn out_set_t(elem_t: Type, len: TyParam) -> Type {
    if PYTHON_MODE {
        set_mut(elem_t, len)
    } else {
        set_t(elem_t, len)
    }
}

pub fn dict_t(dict: TyParam) -> Type {
    poly("Dict", vec![dict])
}

pub fn dict_mut(dict: TyParam) -> Type {
    poly("Dict!", vec![dict])
}

pub fn out_dict_t(dict: TyParam) -> Type {
    if PYTHON_MODE {
        dict_mut(dict)
    } else {
        dict_t(dict)
    }
}

#[inline]
pub fn range(t: Type) -> Type {
    poly("Range", vec![TyParam::t(t)])
}

#[inline]
pub fn module(path: TyParam) -> Type {
    poly("Module", vec![path])
}

#[inline]
pub fn py_module(path: TyParam) -> Type {
    poly("PyModule", vec![path])
}

pub fn module_from_path<P: Into<PathBuf>>(path: P) -> Type {
    let s = ValueObj::Str(Str::rc(&path.into().to_string_lossy()));
    module(TyParam::Value(s))
}

pub fn try_v_enum(s: Set<ValueObj>) -> Result<Type, Set<ValueObj>> {
    if !is_homogeneous(&s) {
        return Err(s);
    }
    let name = FRESH_GEN.fresh_varname();
    let t = inner_class(&s);
    let preds = s
        .into_iter()
        .map(|o| Predicate::eq(name.clone(), TyParam::value(o)))
        .fold(Predicate::FALSE, |acc, p| acc | p);
    let refine = RefinementType::new(name, t, preds);
    Ok(Type::Refinement(refine))
}

pub fn v_enum(s: Set<ValueObj>) -> Type {
    try_v_enum(s).unwrap_or_else(|set| panic!("not homogeneous: {}", set))
}

pub fn t_enum(s: Set<Type>) -> Type {
    try_v_enum(s.into_iter().map(ValueObj::builtin_type).collect())
        .unwrap_or_else(|set| panic!("not homogeneous: {}", set))
}

pub fn t_singleton(t: Type) -> Type {
    t_enum(set! {t})
}

pub fn tp_enum(ty: Type, s: Set<TyParam>) -> Type {
    let name = FRESH_GEN.fresh_varname();
    let preds = s
        .into_iter()
        .map(|tp| Predicate::eq(name.clone(), tp))
        .fold(Predicate::FALSE, |acc, p| acc | p);
    let refine = RefinementType::new(name, ty, preds);
    Type::Refinement(refine)
}

pub fn singleton(ty: Type, tp: TyParam) -> Type {
    let name = FRESH_GEN.fresh_varname();
    let preds = Predicate::eq(name.clone(), tp);
    let refine = RefinementType::new(name, ty, preds);
    Type::Refinement(refine)
}

#[inline]
pub fn int_interval<P, PErr, Q, QErr>(op: IntervalOp, l: P, r: Q) -> Type
where
    P: TryInto<TyParam, Error = PErr>,
    PErr: fmt::Debug,
    Q: TryInto<TyParam, Error = QErr>,
    QErr: fmt::Debug,
{
    interval(op, Type::Int, l, r)
}

#[inline]
pub fn closed_range<P, PErr, Q, QErr>(t: Type, l: P, r: Q) -> Type
where
    P: TryInto<TyParam, Error = PErr>,
    PErr: fmt::Debug,
    Q: TryInto<TyParam, Error = QErr>,
    QErr: fmt::Debug,
{
    interval(IntervalOp::Closed, t, l, r)
}

#[inline]
pub fn interval<P, PErr, Q, QErr>(op: IntervalOp, t: Type, l: P, r: Q) -> Type
where
    P: TryInto<TyParam, Error = PErr>,
    PErr: fmt::Debug,
    Q: TryInto<TyParam, Error = QErr>,
    QErr: fmt::Debug,
{
    let l = l.try_into().unwrap_or_else(|l| todo!("{l:?}"));
    let r = r.try_into().unwrap_or_else(|r| todo!("{r:?}"));
    let name = FRESH_GEN.fresh_varname();
    let pred = match op {
        IntervalOp::LeftOpen if l == TyParam::value(NegInf) => Predicate::le(name.clone(), r),
        // l<..r => {I: classof(l) | I >= l+ε and I <= r}
        IntervalOp::LeftOpen => Predicate::and(
            Predicate::ge(name.clone(), TyParam::succ(l)),
            Predicate::le(name.clone(), r),
        ),
        IntervalOp::RightOpen if r == TyParam::value(Inf) => Predicate::ge(name.clone(), l),
        // l..<r => {I: classof(l) | I >= l and I <= r-ε}
        IntervalOp::RightOpen => Predicate::and(
            Predicate::ge(name.clone(), l),
            Predicate::le(name.clone(), TyParam::pred(r)),
        ),
        // l..r => {I: classof(l) | I >= l and I <= r}
        IntervalOp::Closed => Predicate::and(
            Predicate::ge(name.clone(), l),
            Predicate::le(name.clone(), r),
        ),
        IntervalOp::Open if l == TyParam::value(NegInf) && r == TyParam::value(Inf) => {
            return refinement(name, t, Predicate::TRUE)
        }
        // l<..<r => {I: classof(l) | I >= l+ε and I <= r-ε}
        IntervalOp::Open => Predicate::and(
            Predicate::ge(name.clone(), TyParam::succ(l)),
            Predicate::le(name.clone(), TyParam::pred(r)),
        ),
    };
    refinement(name, t, pred)
}

pub fn iter(t: Type) -> Type {
    poly("Iter", vec![TyParam::t(t)])
}

pub fn ref_(t: Type) -> Type {
    Type::Ref(Box::new(t))
}

pub fn ref_mut(before: Type, after: Option<Type>) -> Type {
    Type::RefMut {
        before: Box::new(before),
        after: after.map(Box::new),
    }
}

/*pub fn option(t: Type) -> Type {
    builtin_poly("Option", vec![TyParam::t(t)])
}

pub fn option_mut(t: Type) -> Type {
    builtin_poly("Option!", vec![TyParam::t(t)])
}*/

pub fn subr_t(
    kind: SubrKind,
    non_default_params: Vec<ParamTy>,
    var_params: Option<ParamTy>,
    default_params: Vec<ParamTy>,
    kw_var_params: Option<ParamTy>,
    return_t: Type,
) -> Type {
    Type::Subr(SubrType::new(
        kind,
        non_default_params,
        var_params,
        default_params,
        kw_var_params,
        return_t,
    ))
}

pub fn func(
    non_default_params: Vec<ParamTy>,
    var_params: Option<ParamTy>,
    default_params: Vec<ParamTy>,
    kw_var_params: Option<ParamTy>,
    return_t: Type,
) -> Type {
    Type::Subr(SubrType::new(
        SubrKind::Func,
        non_default_params,
        var_params,
        default_params,
        kw_var_params,
        return_t,
    ))
}

pub fn no_var_func(
    non_default_params: Vec<ParamTy>,
    default_params: Vec<ParamTy>,
    return_t: Type,
) -> Type {
    func(non_default_params, None, default_params, None, return_t)
}

pub fn func0(return_t: Type) -> Type {
    func(vec![], None, vec![], None, return_t)
}

pub fn func1(param_t: Type, return_t: Type) -> Type {
    func(vec![ParamTy::Pos(param_t)], None, vec![], None, return_t)
}

pub fn kind1(param: Type) -> Type {
    func1(param, Type::Type)
}

pub fn func2(l: Type, r: Type, return_t: Type) -> Type {
    func(
        vec![ParamTy::Pos(l), ParamTy::Pos(r)],
        None,
        vec![],
        None,
        return_t,
    )
}

pub fn func3(l: Type, m: Type, r: Type, return_t: Type) -> Type {
    func(
        vec![ParamTy::Pos(l), ParamTy::Pos(m), ParamTy::Pos(r)],
        None,
        vec![],
        None,
        return_t,
    )
}

pub fn default_func(defaults: Vec<ParamTy>, return_t: Type) -> Type {
    func(vec![], None, defaults, None, return_t)
}

pub fn bin_op(l: Type, r: Type, return_t: Type) -> Type {
    nd_func(
        vec![
            ParamTy::kw(Str::ever("lhs"), l),
            ParamTy::kw(Str::ever("rhs"), r),
        ],
        None,
        return_t,
    )
}

pub fn proc(
    non_default_params: Vec<ParamTy>,
    var_params: Option<ParamTy>,
    default_params: Vec<ParamTy>,
    kw_var_params: Option<ParamTy>,
    return_t: Type,
) -> Type {
    Type::Subr(SubrType::new(
        SubrKind::Proc,
        non_default_params,
        var_params,
        default_params,
        kw_var_params,
        return_t,
    ))
}

pub fn no_var_proc(
    non_default_params: Vec<ParamTy>,
    default_params: Vec<ParamTy>,
    return_t: Type,
) -> Type {
    proc(non_default_params, None, default_params, None, return_t)
}

pub fn proc0(return_t: Type) -> Type {
    proc(vec![], None, vec![], None, return_t)
}

pub fn proc1(param_t: Type, return_t: Type) -> Type {
    proc(vec![ParamTy::Pos(param_t)], None, vec![], None, return_t)
}

pub fn proc2(l: Type, r: Type, return_t: Type) -> Type {
    proc(
        vec![ParamTy::Pos(l), ParamTy::Pos(r)],
        None,
        vec![],
        None,
        return_t,
    )
}

pub fn fn_met(
    self_t: Type,
    mut non_default_params: Vec<ParamTy>,
    var_params: Option<ParamTy>,
    default_params: Vec<ParamTy>,
    kw_var_params: Option<ParamTy>,
    return_t: Type,
) -> Type {
    non_default_params.insert(0, ParamTy::kw(Str::ever("self"), self_t));
    Type::Subr(SubrType::new(
        SubrKind::Func,
        non_default_params,
        var_params,
        default_params,
        kw_var_params,
        return_t,
    ))
}

pub fn no_var_fn_met(
    self_t: Type,
    non_default_params: Vec<ParamTy>,
    default_params: Vec<ParamTy>,
    return_t: Type,
) -> Type {
    fn_met(
        self_t,
        non_default_params,
        None,
        default_params,
        None,
        return_t,
    )
}

pub fn fn0_met(self_t: Type, return_t: Type) -> Type {
    fn_met(self_t, vec![], None, vec![], None, return_t)
}

pub fn fn1_met(self_t: Type, input_t: Type, return_t: Type) -> Type {
    fn_met(
        self_t,
        vec![ParamTy::Pos(input_t)],
        None,
        vec![],
        None,
        return_t,
    )
}

pub fn fn1_kw_met(self_t: Type, input: ParamTy, return_t: Type) -> Type {
    fn_met(self_t, vec![input], None, vec![], None, return_t)
}

pub fn fn2_met(self_t: Type, l: Type, r: Type, return_t: Type) -> Type {
    fn_met(
        self_t,
        vec![ParamTy::Pos(l), ParamTy::Pos(r)],
        None,
        vec![],
        None,
        return_t,
    )
}

pub fn pr_met(
    self_t: Type,
    mut non_default_params: Vec<ParamTy>,
    var_params: Option<ParamTy>,
    default_params: Vec<ParamTy>,
    return_t: Type,
) -> Type {
    non_default_params.insert(0, ParamTy::kw(Str::ever("self"), self_t));
    Type::Subr(SubrType::new(
        SubrKind::Proc,
        non_default_params,
        var_params,
        default_params,
        None,
        return_t,
    ))
}

pub fn pr0_met(self_t: Type, return_t: Type) -> Type {
    pr_met(self_t, vec![], None, vec![], return_t)
}

pub fn pr1_met(self_t: Type, input_t: Type, return_t: Type) -> Type {
    pr_met(self_t, vec![ParamTy::Pos(input_t)], None, vec![], return_t)
}

pub fn pr1_kw_met(self_t: Type, input: ParamTy, return_t: Type) -> Type {
    pr_met(self_t, vec![input], None, vec![], return_t)
}

/// function type with non-default parameters
#[inline]
pub fn nd_func(params: Vec<ParamTy>, var_params: Option<ParamTy>, ret: Type) -> Type {
    func(params, var_params, vec![], None, ret)
}

#[inline]
pub fn nd_proc(params: Vec<ParamTy>, var_params: Option<ParamTy>, ret: Type) -> Type {
    proc(params, var_params, vec![], None, ret)
}

#[inline]
pub fn d_func(default_params: Vec<ParamTy>, return_t: Type) -> Type {
    func(vec![], None, default_params, None, return_t)
}

#[inline]
pub fn nd_proc1(pt: ParamTy, ret: Type) -> Type {
    nd_proc(vec![pt], None, ret)
}

pub fn callable(param_ts: Vec<Type>, return_t: Type) -> Type {
    Type::Callable {
        param_ts,
        return_t: Box::new(return_t),
    }
}

#[inline]
pub fn mono_q<S: Into<Str>>(name: S, constr: Constraint) -> Type {
    named_free_var(name.into(), free::GENERIC_LEVEL, constr)
}

#[inline]
pub fn type_q<S: Into<Str>>(name: S) -> Type {
    mono_q(name, instanceof(Type::Type))
}

#[inline]
pub fn subtype_q<S: Into<Str>>(name: S, sup: Type) -> Type {
    mono_q(name, subtypeof(sup))
}

#[inline]
pub fn mono<S: Into<Str>>(name: S) -> Type {
    let name = name.into();
    if cfg!(feature = "debug") {
        // do not use for: `Int`, `Nat`, ...
        match &name[..] {
            "Obj" | "Int" | "Nat" | "Ratio" | "Float" | "Complex" | "Bool" | "Str" | "NoneType"
            | "Code" | "Frame" | "Error" | "Inf" | "NegInf" | "Type" | "ClassType"
            | "TraitType" | "Patch" | "NotImplementedType" | "Ellipsis" | "Never" => {
                unreachable!("built-in type: {name}")
            }
            _ => {}
        }
    }
    Type::Mono(name)
}

pub fn from_str(name: impl Into<Str>) -> Type {
    let name = name.into();
    match &name[..] {
        "Obj" => Type::Obj,
        "Int" => Type::Int,
        "Nat" => Type::Nat,
        "Ratio" => Type::Ratio,
        "Float" => Type::Float,
        "Complex" => Type::Complex,
        "Bool" => Type::Bool,
        "Str" => Type::Str,
        "NoneType" => Type::NoneType,
        "Code" => Type::Code,
        "Frame" => Type::Frame,
        "Error" => Type::Error,
        "Inf" => Type::Inf,
        "NegInf" => Type::NegInf,
        "Type" => Type::Type,
        "ClassType" => Type::ClassType,
        "TraitType" => Type::TraitType,
        "Patch" => Type::Patch,
        "NotImplementedType" => Type::NotImplementedType,
        "Ellipsis" => Type::Ellipsis,
        "Never" => Type::Never,
        _ => Type::Mono(name),
    }
}

#[inline]
pub fn poly<S: Into<Str>>(name: S, params: Vec<TyParam>) -> Type {
    Type::Poly {
        name: name.into(),
        params,
    }
}

pub fn type_poly<S: Into<Str>>(name: S, ts: Vec<Type>) -> Type {
    poly(name, ts.into_iter().map(TyParam::t).collect())
}

#[inline]
pub fn proj<S: Into<Str>>(lhs: Type, rhs: S) -> Type {
    Type::Proj {
        lhs: Box::new(lhs),
        rhs: rhs.into(),
    }
}

#[inline]
pub fn proj_call<S: Into<Str>>(lhs: TyParam, attr_name: S, args: Vec<TyParam>) -> Type {
    Type::ProjCall {
        lhs: Box::new(lhs),
        attr_name: attr_name.into(),
        args,
    }
}

/// ```erg
/// {I: Int | I >= 0}
/// => Refinement{
///     layout: TyParam::MonoQ "I",
///     bounds: [TyBound::Instance("I", "Int")],
///     pred: Predicate::GreaterEqual("I", 0)
/// }
/// ```
#[inline]
pub fn refinement(var: Str, t: Type, pred: Predicate) -> Type {
    Type::Refinement(RefinementType::new(var, t, pred))
}

pub fn and(lhs: Type, rhs: Type) -> Type {
    lhs & rhs
}

pub fn or(lhs: Type, rhs: Type) -> Type {
    lhs | rhs
}

pub fn ors(tys: impl IntoIterator<Item = Type>) -> Type {
    tys.into_iter().fold(Type::Never, or)
}

pub fn ands(tys: impl IntoIterator<Item = Type>) -> Type {
    tys.into_iter().fold(Type::Obj, and)
}

pub fn not(ty: Type) -> Type {
    Type::Not(Box::new(ty))
}

pub fn guard(namespace: Str, target: CastTarget, to: Type) -> Type {
    Type::Guard(GuardType::new(namespace, target, to))
}

pub fn bounded(sub: Type, sup: Type) -> Type {
    if sub == Type::Never {
        sup
    } else {
        Type::Bounded {
            sub: Box::new(sub),
            sup: Box::new(sup),
        }
    }
}

#[inline]
pub fn instanceof(t: Type) -> Constraint {
    Constraint::new_type_of(t)
}

/// Sub <: Sup
#[inline]
pub fn subtypeof(sup: Type) -> Constraint {
    Constraint::new_sandwiched(Type::Never, sup)
}

#[inline]
pub fn supertypeof(sub: Type) -> Constraint {
    Constraint::new_sandwiched(sub, Type::Obj)
}

#[inline]
pub fn mono_q_tp<S: Into<Str>>(name: S, constr: Constraint) -> TyParam {
    TyParam::mono_q(name, constr)
}

#[inline]
pub fn mono_tp<S: Into<Str>>(name: S) -> TyParam {
    TyParam::mono(name)
}

#[inline]
pub fn ty_tp(t: Type) -> TyParam {
    TyParam::t(t)
}

/// NOTE: Always add postfix when entering numbers. For example, `value(1)` will be of type Int.
#[inline]
pub fn value<V: Into<ValueObj>>(v: V) -> TyParam {
    TyParam::value(v)
}
