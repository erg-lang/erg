#[allow(unused_imports)]
use erg_common::log;
use erg_common::vis::Visibility;

use crate::ty::constructors::*;
use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::Type;
use Type::*;

use crate::context::initialize::*;
use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(super) fn init_builtin_patches(&mut self) {
        let m = mono_q_tp("M", instanceof(Int));
        let n = mono_q_tp("N", instanceof(Int));
        let o = mono_q_tp("O", instanceof(Int));
        let p = mono_q_tp("P", instanceof(Int));
        let params = vec![
            PS::named_nd("M", Int),
            PS::named_nd("N", Int),
            PS::named_nd("O", Int),
            PS::named_nd("P", Int),
        ];
        let class = Type::from(&m..=&n);
        let impls = poly("Add", vec![TyParam::from(&o..=&p)]);
        // Interval is a bounding patch connecting M..N and (Add(O..P, M+O..N..P), Sub(O..P, M-P..N-O))
        let mut interval =
            Self::builtin_poly_glue_patch("Interval", class.clone(), impls.clone(), params, 2);
        let op_t = fn1_met(
            class.clone(),
            Type::from(&o..=&p),
            Type::from(m.clone() + o.clone()..=n.clone() + p.clone()),
        )
        .quantify();
        let mut interval_add = Self::builtin_methods(Some(impls), 2);
        interval_add.register_builtin_erg_impl("__add__", op_t, Const, Public);
        interval_add.register_builtin_const(
            "Output",
            Public,
            ValueObj::builtin_t(Type::from(m.clone() + o.clone()..=n.clone() + p.clone())),
        );
        interval.register_trait(class.clone(), interval_add);
        let mut interval_sub =
            Self::builtin_methods(Some(poly("Sub", vec![TyParam::from(&o..=&p)])), 2);
        let op_t = fn1_met(
            class.clone(),
            Type::from(&o..=&p),
            Type::from(m.clone() - p.clone()..=n.clone() - o.clone()),
        )
        .quantify();
        interval_sub.register_builtin_erg_impl("__sub__", op_t, Const, Public);
        interval_sub.register_builtin_const(
            "Output",
            Public,
            ValueObj::builtin_t(Type::from(m - p..=n - o)),
        );
        interval.register_trait(class, interval_sub);
        self.register_builtin_patch("Interval", interval, Private, Const);
        // eq.register_impl("__ne__", op_t,         Const, Public);
        // ord.register_impl("__le__", op_t.clone(), Const, Public);
        // ord.register_impl("__gt__", op_t.clone(), Const, Public);
        // ord.register_impl("__ge__", op_t,         Const, Public);
        let E = mono_q("E", subtypeof(mono("Eq")));
        let base = or(E, NoneType);
        let impls = mono("Eq");
        let params = vec![PS::named_nd("E", Type)];
        let mut option_eq =
            Self::builtin_poly_glue_patch("OptionEq", base.clone(), impls.clone(), params, 1);
        let mut option_eq_impl = Self::builtin_methods(Some(impls), 1);
        let op_t = fn1_met(base.clone(), base.clone(), Bool).quantify();
        option_eq_impl.register_builtin_erg_impl("__eq__", op_t, Const, Public);
        option_eq.register_trait(base, option_eq_impl);
        self.register_builtin_patch("OptionEq", option_eq, Private, Const);
    }
}
