use erg_common::style::{Attribute, Color, StyledStrings, THEME};
use erg_common::{enum_unwrap, switch_lang};

use crate::ty::typaram::TyParam;
use crate::ty::{HasType, SubrKind, Type};

use crate::context::Context;

const HINT: Color = THEME.colors.hint;
const ERR: Color = THEME.colors.error;
#[cfg(not(feature = "pretty"))]
const ATTR: Attribute = Attribute::Bold;
#[cfg(feature = "pretty")]
const ATTR: Attribute = Attribute::Underline;

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
        let mut hint = StyledStrings::default();

        if let (Type::Subr(expt), Type::Subr(fnd)) = (&expected, &found) {
            if let (SubrKind::Func, SubrKind::Proc) = (expt.kind, fnd.kind) {
                switch_lang!(
                    "japanese" => {
                        hint.push_str("この仮引数は(副作用のない)関数を受け取りますが、プロシージャは副作用があるため受け取りません。副作用を取り除き、");
                        hint.push_str_with_color_and_attribute("=>", ERR, ATTR);
                        hint.push_str("の代わりに");
                        hint.push_str_with_color_and_attribute("->", HINT, ATTR);
                        hint.push_str("を使用する必要があります");
                    },
                    "simplified_chinese" => {
                        hint.push_str("此参数接受func（无副作用）但由于副作用而不接受proc。你应该使用 ");
                        hint.push_str_with_color_and_attribute("->", HINT, ATTR);
                        hint.push_str("而不是 ");
                        hint.push_str_with_color_and_attribute("=>", ERR, ATTR);
                    },
                    "traditional_chinese" => {
                        hint.push_str("此参数接受 func（无副作用）但由于副作用而不接受proc。你應該使用 ");
                        hint.push_str_with_color_and_attribute("->", HINT, ATTR);
                        hint.push_str("而不是 ");
                        hint.push_str_with_color_and_attribute("=>", ERR, ATTR);
                    },
                    "english" => {
                        hint.push_str("This param accepts func (without side-effects) but not proc because of side-effects. You should use ");
                        hint.push_str_with_color_and_attribute("->", HINT, ATTR);
                        hint.push_str(" instead of ");
                        hint.push_str_with_color_and_attribute("=>", ERR, ATTR);
                    },
                );
                return Some(hint.to_string());
            }
        }

        match (&expected.qual_name()[..], &found.qual_name()[..]) {
            ("Eq", "Float") => {
                switch_lang!(
                    "japanese" => {
                        hint.push_str("Floatは等価関係が定義されていません。");
                        hint.push_str_with_color_and_attribute("l == R", ERR, ATTR);
                        hint.push_str("ではなく、");
                        hint.push_str_with_color_and_attribute("l - r <= Float.EPSILON", HINT, ATTR);
                        hint.push_str("を使用してください");
                    },
                    "simplified_chinese" => {
                        hint.push_str("Float没有定义等价关系。你应该使用");
                        hint.push_str_with_color_and_attribute("l == R", ERR, ATTR);
                        hint.push_str("而不是");
                        hint.push_str_with_color_and_attribute("l - r <= Float.EPSILON", HINT, ATTR);
                    },
                    "traditional_chinese" => {
                        hint.push_str("Float沒有定義等價關係。你應該使用");
                        hint.push_str_with_color_and_attribute("l == R", ERR, ATTR);
                        hint.push_str(" instead of ");
                        hint.push_str_with_color_and_attribute("l - r <= Float.EPSILON", HINT, ATTR);
                    },
                    "english" => {
                        hint.push_str("Float has no equivalence relation defined. you should use ");
                        hint.push_str_with_color_and_attribute("l == R", ERR, ATTR);
                        hint.push_str(" instead of ");
                        hint.push_str_with_color_and_attribute("l - r <= Float.EPSILON", HINT, ATTR);
                    },
                );
                Some(hint.to_string())
            }
            _ => {
                let (verb, preposition, _sequence) = Self::get_verb_and_preposition(&expected)?;
                found
                    .union_types()
                    .map(|(t1, t2)| format!("cannot {verb} {t1} {preposition} {t2}"))
                    .or_else(|| {
                        let expected_inner = Self::readable_type(&expected.inner_ts()[0]);
                        Some(format!(
                            "cannot {verb} {found} {preposition} {expected_inner}"
                        ))
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
