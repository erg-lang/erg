use erg_common::enum_unwrap;

use crate::ty::typaram::TyParam;
use crate::ty::{HasType, Type};

use crate::context::Context;

#[derive(PartialEq, Eq)]
enum Sequence {
    Forward,
    Backward,
}

// TODO: these should not be in Context
impl Context {
    fn readable_type(typ: &Type) -> Type {
        match typ {
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = fv.get_subsup().unwrap();
                if sup == Type::Obj {
                    return sub;
                }
                Type::FreeVar(fv.clone())
            }
            other => other.clone(),
        }
    }

    /// TODO: custom types
    fn get_verb_and_preposition(trait_: &Type) -> Option<(&str, &str, Sequence)> {
        match &trait_.qual_name()[..] {
            "Add" => Some(("add", "and", Sequence::Forward)),
            "Sub" => Some(("subtract", "from", Sequence::Backward)),
            "Mul" => Some(("multiply", "and", Sequence::Forward)),
            "Div" => Some(("divide", "by", Sequence::Forward)),
            "Eq" => Some(("compare", "and", Sequence::Forward)),
            "Ord" => Some(("compare", "and", Sequence::Forward)),
            _ => None,
        }
    }

    pub(crate) fn get_type_mismatch_hint(expected: &Type, found: &Type) -> Option<String> {
        let expected = if let Type::FreeVar(fv) = expected {
            if fv.is_linked() {
                fv.crack().clone()
            } else {
                let (_sub, sup) = fv.get_subsup().unwrap();
                sup
            }
        } else {
            expected.clone()
        };
        match (&expected.qual_name()[..], &found.qual_name()[..]) {
            ("Eq", "Float") => Some(String::from("Float has no equivalence relation defined. you should use `l - r <= Float.EPSILON` instead of `l == r`.")),
            _ => {
                let (verb, preposition, _sequence) = Self::get_verb_and_preposition(&expected)?;
                found.union_types()
                    .map(|(t1, t2)| format!("cannot {verb} {t1} {preposition} {t2}"))
                    .or_else(|| {
                        let expected_inner = Self::readable_type(&expected.inner_ts()[0]);
                        Some(format!("cannot {verb} {found} {preposition} {expected_inner}"))
                    })
            }
        }
    }

    pub(crate) fn get_no_candidate_hint(proj: &Type) -> Option<String> {
        match proj {
            Type::Proj { lhs, rhs: _ } => {
                if let Type::FreeVar(fv) = lhs.as_ref() {
                    let (sub, sup) = fv.get_subsup()?;
                    let (verb, preposition, sequence) = Self::get_verb_and_preposition(&sup)?;
                    let sup = enum_unwrap!(sup.typarams().remove(0), TyParam::Type);
                    let sup = Self::readable_type(&sup);
                    let (l, r) = if sequence == Sequence::Forward {
                        (sub, sup)
                    } else {
                        (sup, sub)
                    };
                    Some(format!("cannot {verb} {l} {preposition} {r}"))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
