use erg_common::astr::AtomicStr;

use erg_type::Type;

use crate::context::Context;

impl Context {
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
}
