use erg_common::astr::AtomicStr;
use erg_common::enum_unwrap;

use erg_type::typaram::TyParam;
use erg_type::Type;

use crate::context::Context;

#[derive(PartialEq, Eq)]
enum Sequence {
    Forward,
    Backward,
}

impl Context {
    fn readable_type(&self, typ: &Type) -> Type {
        match typ {
            Type::FreeVar(fv) if fv.constraint_is_sandwiched() => {
                let (sub, sup) = fv.get_bound_types().unwrap();
                if sup == Type::Obj {
                    return sub;
                }
                Type::FreeVar(fv.clone())
            }
            other => other.clone(),
        }
    }

    pub(crate) fn get_type_mismatch_hint(
        &self,
        expected: &Type,
        found: &Type,
    ) -> Option<AtomicStr> {
        let expected = if let Type::FreeVar(fv) = expected {
            if fv.is_linked() {
                fv.crack().clone()
            } else {
                let (_sub, sup) = fv.get_bound_types().unwrap();
                sup
            }
        } else {
            expected.clone()
        };
        match (&expected.name()[..], &found.name()[..]) {
            ("Eq", "Float") => Some(AtomicStr::ever("Float has no equivalence relation defined. you should use `l - r <= Float.EPSILON` instead of `l == r`.")),
            _ => None,
        }
    }

    pub(crate) fn get_no_candidate_hint(&self, proj: &Type) -> Option<AtomicStr> {
        match proj {
            Type::Proj { lhs, rhs: _ } => {
                if let Type::FreeVar(fv) = lhs.as_ref() {
                    let (sub, sup) = fv.get_bound_types()?;
                    // TODO: automating
                    let (verb, preposition, sequence) = match &sup.name()[..] {
                        "Add" => Some(("add", "and", Sequence::Forward)),
                        "Sub" => Some(("subtract", "from", Sequence::Backward)),
                        "Mul" => Some(("multiply", "and", Sequence::Forward)),
                        "Div" => Some(("divide", "by", Sequence::Forward)),
                        "Eq" => Some(("compare", "and", Sequence::Forward)),
                        "Ord" => Some(("compare", "and", Sequence::Forward)),
                        _ => None,
                    }?;
                    let sup = enum_unwrap!(sup.typarams().remove(0), TyParam::Type);
                    let sup = self.readable_type(&sup);
                    let (l, r) = if sequence == Sequence::Forward {
                        (sub, sup)
                    } else {
                        (sup, sub)
                    };
                    Some(AtomicStr::from(format!(
                        "cannot {verb} {l} {preposition} {r}"
                    )))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
