use erg_common::style::{Attribute, Color, StyledStrings, THEME};
use erg_common::{option_enum_unwrap, switch_lang};

use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{HasType, Predicate, SubrKind, Type};

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

    pub(crate) fn get_call_type_mismatch_hint(
        &self,
        callee_t: &Type,
        attr: Option<&str>,
        nth: usize,
        expected: &Type,
        found: &Type,
    ) -> Option<String> {
        if &callee_t.qual_name()[..] == "Array" && attr == Some("__getitem__") && nth == 1 {
            let len = &callee_t.typarams().get(1).cloned()?;
            let (_, _, preds) = found.clone().deconstruct_refinement().ok()?;
            if let Predicate::Equal {
                lhs: _,
                rhs: accessed,
            } = preds.iter().next()?
            {
                let accessed = if let TyParam::Value(value) = accessed {
                    value
                        .clone()
                        .try_add(ValueObj::Nat(1))
                        .map(TyParam::Value)
                        .unwrap_or_else(|| accessed.clone())
                } else {
                    accessed.clone()
                };
                return Some(switch_lang! {
                    "japanese" => format!("配列の長さは{len}ですが、{accessed}番目の要素にアクセスしようとしています"),
                    "simplified_chinese" => format!("数组长度为{len}但尝试访问第{accessed}个元素"),
                    "traditional_chinese" => format!("陣列長度為{len}但嘗試訪問第{accessed}個元素"),
                    "english" => format!("Array length is {len} but tried to access the {accessed}th element"),
                });
            }
        }
        self.get_simple_type_mismatch_hint(expected, found)
    }

    pub(crate) fn get_simple_type_mismatch_hint(
        &self,
        expected: &Type,
        found: &Type,
    ) -> Option<String> {
        let expected = if let Type::FreeVar(fv) = expected {
            if fv.is_linked() {
                fv.crack().clone()
            } else {
                let (_sub, sup) = fv.get_subsup()?;
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
                        hint.push_str("此参数接受函数(无副作用)，但不接受过程，因为过程有副作用。你应该使用");
                        hint.push_str_with_color_and_attribute("=>", HINT, ATTR);
                        hint.push_str("而不是");
                        hint.push_str_with_color_and_attribute("->", ERR, ATTR);
                    },
                    "traditional_chinese" => {
                        hint.push_str("此參數接受函數(無副作用)，但不接受過程，因為過程有副作用。你應該使用");
                        hint.push_str_with_color_and_attribute("=>", HINT, ATTR);
                        hint.push_str("而不是");
                        hint.push_str_with_color_and_attribute("->", ERR, ATTR);
                    },
                    "english" => {
                        hint.push_str("This param accepts func (without side-effects) but not proc because of side-effects. You should use ");
                        hint.push_str_with_color_and_attribute("=>", HINT, ATTR);
                        hint.push_str(" instead of ");
                        hint.push_str_with_color_and_attribute("->", ERR, ATTR);
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
                        expected.inner_ts().get(0).map(|expected_inner| {
                            let expected_inner = self.readable_type(expected_inner.clone(), false);
                            format!("cannot {verb} {found} {preposition} {expected_inner}")
                        })
                    })
            }
        }
    }

    pub(crate) fn get_no_candidate_hint(&self, proj: &Type) -> Option<String> {
        match proj {
            Type::Proj { lhs, rhs: _ } => {
                if let Type::FreeVar(fv) = lhs.as_ref() {
                    let (sub, sup) = fv.get_subsup()?;
                    let (verb, preposition, sequence) = Self::get_verb_and_preposition(&sup)?;
                    let sup = *option_enum_unwrap!(sup.typarams().get(0)?.clone(), TyParam::Type)?;
                    let sup = self.readable_type(sup, false);
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
