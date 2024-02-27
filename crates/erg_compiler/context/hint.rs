use erg_common::dict::Dict;
use erg_common::style::{Attribute, Color, StyledStrings, THEME};
use erg_common::{option_enum_unwrap, switch_lang};

use crate::ty::typaram::TyParam;
use crate::ty::value::ValueObj;
use crate::ty::{Field, HasType, Predicate, SubrKind, SubrType, Type};

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
            let (_, _, pred) = found.clone().deconstruct_refinement().ok()?;
            if let Predicate::Equal { rhs: accessed, .. } = pred {
                let accessed = if let TyParam::Value(value) = &accessed {
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
        let expected = if let Some(fv) = expected.as_free() {
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
        match (&expected, &found) {
            (Type::Subr(expt), Type::Subr(fnd)) => {
                if let Some(hint) = self.get_subr_type_mismatch_hint(expt, fnd) {
                    return Some(hint);
                }
            }
            (Type::Quantified(expt), Type::Subr(fnd)) => {
                if let Type::Subr(expt) = expt.as_ref() {
                    if let Some(hint) = self.get_subr_type_mismatch_hint(expt, fnd) {
                        return Some(hint);
                    }
                }
            }
            (Type::Quantified(expt), Type::Quantified(fnd)) => {
                if let (Type::Subr(expt), Type::Subr(fnd)) = (expt.as_ref(), fnd.as_ref()) {
                    if let Some(hint) = self.get_subr_type_mismatch_hint(expt, fnd) {
                        return Some(hint);
                    }
                }
            }
            (Type::Record(expt), Type::Record(fnd)) => {
                if let Some(hint) = self.get_record_type_mismatch_hint(expt, fnd) {
                    return Some(hint);
                }
            }
            (Type::NamedTuple(expt), Type::NamedTuple(fnd)) => {
                let expt = Dict::from(expt.clone());
                let fnd = Dict::from(fnd.clone());
                if let Some(hint) = self.get_record_type_mismatch_hint(&expt, &fnd) {
                    return Some(hint);
                }
            }
            (Type::And(l, r), found) => {
                let left = self.readable_type(l.as_ref().clone());
                let right = self.readable_type(r.as_ref().clone());
                if self.supertype_of(l, found) {
                    let msg = switch_lang!(
                        "japanese" => format!("型{found}は{left}のサブタイプですが、{right}のサブタイプではありません"),
                        "simplified_chinese" => format!("类型{found}是{left}的子类型但不是{right}的子类型"),
                        "traditional_chinese" => format!("型別{found}是{left}的子型別但不是{right}的子型別"),
                        "english" => format!("Type {found} is a subtype of {left} but not of {right}"),
                    );
                    hint.push_str(&msg);
                    return Some(hint.to_string());
                } else if self.supertype_of(r, found) {
                    let msg = switch_lang!(
                        "japanese" => format!("型{found}は{right}のサブタイプですが、{left}のサブタイプではありません"),
                        "simplified_chinese" => format!("类型{found}是{right}的子类型但不是{left}的子类型"),
                        "traditional_chinese" => format!("型別{found}是{right}的子型別但不是{left}の子型別"),
                        "english" =>format!("Type {found} is a subtype of {right} but not of {left}"),
                    );
                    hint.push_str(&msg);
                    return Some(hint.to_string());
                }
            }
            _ => {}
        }

        match (&expected.qual_name()[..], &found.qual_name()[..]) {
            ("Eq", "Float") => {
                switch_lang!(
                    "japanese" => {
                        hint.push_str("Floatは等価関係が定義されていません。");
                        hint.push_str_with_color_and_attr("l == r", ERR, ATTR);
                        hint.push_str("ではなく、");
                        hint.push_str_with_color_and_attr("l - r <= Float.EPSILON", HINT, ATTR);
                        hint.push_str("あるいは");
                        hint.push_str_with_color_and_attr("l.nearly_eq(r)", HINT, ATTR);
                        hint.push_str("を使用してください");
                    },
                    "simplified_chinese" => {
                        hint.push_str("Float没有定义等价关系。你应该使用");
                        hint.push_str_with_color_and_attr("l - r <= Float.EPSILON", HINT, ATTR);
                        hint.push_str("或者");
                        hint.push_str_with_color_and_attr("l.nearly_eq(r)", HINT, ATTR);
                        hint.push_str("而不是");
                        hint.push_str_with_color_and_attr("l == r", ERR, ATTR);
                    },
                    "traditional_chinese" => {
                        hint.push_str("Float沒有定義等價關係。你應該使用");
                        hint.push_str_with_color_and_attr("l - r <= Float.EPSILON", HINT, ATTR);
                        hint.push_str("或者");
                        hint.push_str_with_color_and_attr("l.nearly_eq(r)", HINT, ATTR);
                        hint.push_str("而不是");
                        hint.push_str_with_color_and_attr("l == r", ERR, ATTR);
                    },
                    "english" => {
                        hint.push_str("Float has no equivalence relation defined. You should use ");
                        hint.push_str_with_color_and_attr("l - r <= Float.EPSILON", HINT, ATTR);
                        hint.push_str(" or ");
                        hint.push_str_with_color_and_attr("l.nearly_eq(r)", HINT, ATTR);
                        hint.push_str(" instead of ");
                        hint.push_str_with_color_and_attr("l == r", ERR, ATTR);
                    },
                );
                Some(hint.to_string())
            }
            _ => {
                let (verb, preposition, _sequence) = Self::get_verb_and_preposition(&expected)?;
                found
                    .union_pair()
                    .map(|(t1, t2)| format!("cannot {verb} {t1} {preposition} {t2}"))
                    .or_else(|| {
                        expected.inner_ts().first().map(|expected_inner| {
                            let expected_inner = self.readable_type(expected_inner.clone());
                            format!("cannot {verb} {found} {preposition} {expected_inner}")
                        })
                    })
            }
        }
    }

    fn get_record_type_mismatch_hint(
        &self,
        expected: &Dict<Field, Type>,
        found: &Dict<Field, Type>,
    ) -> Option<String> {
        let missing = expected.clone().diff(found);
        if missing.is_empty() {
            let mut mismatched = "".to_string();
            for (field, expected) in expected.iter() {
                if let Some(found) = found.get(field) {
                    if !self.supertype_of(expected, found) {
                        if !mismatched.is_empty() {
                            mismatched.push_str(", ");
                        }
                        mismatched.push_str(&format!("{field}: {expected} but found {found}"));
                    }
                }
            }
            if mismatched.is_empty() {
                None
            } else {
                Some(mismatched)
            }
        } else {
            let mut hint = "missing: ".to_string();
            for (i, (field, typ)) in missing.iter().enumerate() {
                if i > 0 {
                    hint.push_str(", ");
                }
                hint.push_str(&format!("{field}: {typ}"));
            }
            Some(hint)
        }
    }

    // TODO: parameter type mismatches
    fn get_subr_type_mismatch_hint(&self, expected: &SubrType, found: &SubrType) -> Option<String> {
        let mut hint = StyledStrings::default();
        if let (SubrKind::Func, SubrKind::Proc) = (expected.kind, found.kind) {
            switch_lang!(
                "japanese" => {
                    hint.push_str("この仮引数は(副作用のない)関数を受け取りますが、プロシージャは副作用があるため受け取りません。副作用を取り除き、");
                    hint.push_str_with_color_and_attr("=>", ERR, ATTR);
                    hint.push_str("の代わりに");
                    hint.push_str_with_color_and_attr("->", HINT, ATTR);
                    hint.push_str("を使用する必要があります");
                },
                "simplified_chinese" => {
                    hint.push_str("此参数接受函数(无副作用)，但不接受过程，因为过程有副作用。你应该使用");
                    hint.push_str_with_color_and_attr("=>", HINT, ATTR);
                    hint.push_str("而不是");
                    hint.push_str_with_color_and_attr("->", ERR, ATTR);
                },
                "traditional_chinese" => {
                    hint.push_str("此參數接受函數(無副作用)，但不接受過程，因為過程有副作用。你應該使用");
                    hint.push_str_with_color_and_attr("=>", HINT, ATTR);
                    hint.push_str("而不是");
                    hint.push_str_with_color_and_attr("->", ERR, ATTR);
                },
                "english" => {
                    hint.push_str("This param accepts func (without side-effects) but not proc because of side-effects. You should use ");
                    hint.push_str_with_color_and_attr("=>", HINT, ATTR);
                    hint.push_str(" instead of ");
                    hint.push_str_with_color_and_attr("->", ERR, ATTR);
                },
            );
            return Some(hint.to_string());
        }
        if let Some((expect, _found)) = expected
            .non_var_params()
            .zip(found.non_var_params())
            .find(|(expect, found)| expect.typ().is_ref() && !found.typ().is_ref())
        {
            let hint = switch_lang!(
                "japanese" => format!("{expect}は参照を受け取るよう宣言されましたが、実体が渡されています(refプレフィックスを追加してください)"),
                "simplified_chinese" => format!("{expect}被声明为接受引用，但实体被传递(请添加ref前缀)"),
                "traditional_chinese" => format!("{expect}被宣告為接受引用，但實體被傳遞(請添加ref前綴)"),
                "english" => format!("{expect} is declared as a reference parameter but definition is an owned parameter (add `ref` prefix)"),
            );
            return Some(hint);
        }
        None
    }

    pub(crate) fn get_no_candidate_hint(&self, proj: &Type) -> Option<String> {
        match proj {
            Type::Proj { lhs, rhs: _ } => {
                if let Some(fv) = lhs.as_free() {
                    let (sub, sup) = fv.get_subsup()?;
                    let (verb, preposition, sequence) = Self::get_verb_and_preposition(&sup)?;
                    let sup = *option_enum_unwrap!(sup.typarams().first()?.clone(), TyParam::Type)?;
                    let sup = self.readable_type(sup);
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
