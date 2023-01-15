//! test module for `Context`
use erg_common::error::Location;
use erg_common::Str;
// use erg_common::error::Location;
use erg_common::set;

use crate::ty::constructors::{func1, mono, mono_q, poly, refinement};
use crate::ty::free::Constraint;
use crate::ty::typaram::TyParam;
use crate::ty::{Predicate, Type};
use Type::*;

use crate::context::instantiate::TyVarCache;
use crate::context::Context;

impl Context {
    pub fn test_refinement_subtyping(&self) -> Result<(), ()> {
        // Nat :> {I: Int | I >= 1} ?
        let lhs = Nat;
        let var = Str::ever("I");
        let rhs = refinement(
            var.clone(),
            Type::Int,
            set! { Predicate::eq(var, TyParam::value(1)) },
        );
        if self.supertype_of(&lhs, &rhs) {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn test_resolve_trait_inner1(&self) -> Result<(), ()> {
        let name = Str::ever("Add");
        let params = vec![TyParam::t(Nat)];
        let maybe_trait = poly(name, params);
        let mut min = Type::Obj;
        for pair in self.get_trait_impls(&maybe_trait) {
            if self.supertype_of(&pair.sup_trait, &maybe_trait) {
                min = self.min(&min, &pair.sub_type).unwrap_or(&min).clone();
            }
        }
        if min == Nat {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn test_instantiation_and_generalization(&self) -> Result<(), ()> {
        use crate::ty::free::HasLevel;
        let t = mono_q("T", Constraint::new_subtype_of(mono("Eq")));
        let unbound = func1(t.clone(), t);
        let quantified = unbound.clone().quantify();
        println!("quantified      : {quantified}");
        let mut tv_cache = TyVarCache::new(self.level + 1, self);
        println!("tv_cache: {tv_cache}");
        let inst = self
            .instantiate_t_inner(unbound, &mut tv_cache, Location::Unknown)
            .map_err(|_| ())?;
        println!("inst: {inst}");
        inst.lift();
        let quantified_again = self.generalize_t(inst);
        println!("quantified_again: {quantified_again}");
        assert_eq!(quantified, quantified_again);
        Ok(())
    }
}
