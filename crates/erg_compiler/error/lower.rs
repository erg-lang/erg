use erg_common::error::{ErrorCore, ErrorKind::*, Location, SubMessage};
use erg_common::io::Input;
use erg_common::style::{StyledStr, StyledString, StyledStrings, Stylize};
use erg_common::traits::Locational;
use erg_common::{switch_lang, Str};

use crate::error::*;
use crate::hir::{Expr, Identifier};
use crate::ty::{HasType, Type, Visibility};
use crate::varinfo::VarInfo;

pub type LowerError = CompileError;
pub type LowerWarning = LowerError;
pub type LowerErrors = CompileErrors;
pub type LowerWarnings = LowerErrors;
pub type LowerResult<T> = CompileResult<T>;
pub type SingleLowerResult<T> = SingleCompileResult<T>;

impl LowerError {
    pub fn syntax_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        desc: String,
        hint: Option<String>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                desc,
                errno,
                SyntaxError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn unused_expr_warning(input: Input, errno: usize, expr: &Expr, caused_by: String) -> Self {
        let desc = switch_lang!(
            "japanese" => format!("式の評価結果(: {})が使われていません", expr.ref_t()),
            "simplified_chinese" => format!("表达式评估结果(: {})未使用", expr.ref_t()),
            "traditional_chinese" => format!("表達式評估結果(: {})未使用", expr.ref_t()),
            "english" => format!("the evaluation result of the expression (: {}) is not used", expr.ref_t()),
        );
        let discard = StyledString::new("discard", Some(HINT), Some(ATTR));
        let hint = switch_lang!(
            "japanese" => format!("値を使わない場合は、{discard}関数を使用してください"),
            "simplified_chinese" => format!("如果您不想使用该值，请使用{discard}函数"),
            "traditional_chinese" => format!("如果您不想使用該值，請使用{discard}函數"),
            "english" => format!("if you don't use the value, use {discard} function"),
        );
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(expr.loc(), vec![], Some(hint))],
                desc,
                errno,
                UnusedWarning,
                expr.loc(),
            ),
            input,
            caused_by,
        )
    }

    pub fn unused_subroutine_warning(
        input: Input,
        errno: usize,
        expr: &Expr,
        caused_by: String,
    ) -> Self {
        let desc = switch_lang!(
            "japanese" => format!("式の評価結果(: {})が使われていません", expr.ref_t()),
            "simplified_chinese" => format!("表达式评估结果(: {})未使用", expr.ref_t()),
            "traditional_chinese" => format!("表達式評估結果(: {})未使用", expr.ref_t()),
            "english" => format!("the evaluation result of the expression (: {}) is not used", expr.ref_t()),
        );
        let hint = switch_lang!(
            "japanese" => format!("呼び出しの()を忘れていませんか?"),
            "simplified_chinese" => format!("忘记了调用的()吗?"),
            "traditional_chinese" => format!("忘記了調用的()嗎?"),
            "english" => format!("perhaps you forgot the () in the call?"),
        );
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(expr.loc(), vec![], Some(hint))],
                desc,
                errno,
                UnusedWarning,
                expr.loc(),
            ),
            input,
            caused_by,
        )
    }

    pub fn duplicate_decl_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}は既に宣言されています"),
                    "simplified_chinese" => format!("{name}已声明"),
                    "traditional_chinese" => format!("{name}已聲明"),
                    "english" => format!("{name} is already declared"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn duplicate_definition_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}は既に定義されています"),
                    "simplified_chinese" => format!("{name}已定义"),
                    "traditional_chinese" => format!("{name}已定義"),
                    "english" => format!("{name} is already defined"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn violate_decl_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        spec_t: &Type,
        found_t: &Type,
    ) -> Self {
        let name = readable_name(name).with_color(WARN);
        let expect = format!("{spec_t}").with_color_and_attr(HINT, ATTR);
        let found = format!("{found_t}").with_color_and_attr(ERR, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}は{expect}型として宣言されましたが、{found}型のオブジェクトが代入されています"),
                    "simplified_chinese" => format!("{name}被声明为{expect}，但分配了一个{found}对象"),
                    "traditional_chinese" => format!("{name}被聲明為{expect}，但分配了一個{found}對象"),
                    "english" => format!("{name} was declared as {expect}, but an {found} object is assigned"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_var_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = n.with_color_and_attr(HINT, ATTR);
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
        });
        let found = name.with_color_and_attr(ERR, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{found}という変数は定義されていません"),
                    "simplified_chinese" => format!("{found}未定义"),
                    "traditional_chinese" => format!("{found}未定義"),
                    "english" => format!("{found} is not defined"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn not_comptime_fn_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = n.with_color_and_attr(HINT, ATTR);
            switch_lang!(
                "japanese" => format!("似た名前の関数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称函数: {n}"),
                "traditional_chinese" => format!("存在相同名稱函數: {n}"),
                "english" => format!("exists a similar name function: {n}"),
            )
        });
        let found = name.with_color_and_attr(ERR, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{found}はコンパイル時関数ではありません"),
                    "simplified_chinese" => format!("{found}不是编译时函数"),
                    "traditional_chinese" => format!("{found}不是編譯時函數"),
                    "english" => format!("{found} is not a compile-time function"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    /// TODO: replace `no_var_error` with this function
    pub fn detailed_no_var_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        similar_name: Option<&str>,
        similar_info: Option<&VarInfo>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let vis = similar_info.map_or("".into(), |vi| vi.vis.modifier.display());
            let n = n.with_color_and_attr(HINT, ATTR);
            switch_lang!(
                "japanese" => format!("似た名前の{vis}変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称{vis}变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱{vis}變量: {n}"),
                "english" => format!("exists a similar name {vis} variable: {n}"),
            )
        });
        let found = name.with_color_and_attr(ERR, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{found}という変数は定義されていません"),
                    "simplified_chinese" => format!("{found}未定义"),
                    "traditional_chinese" => format!("{found}未定義"),
                    "english" => format!("{found} is not defined"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn access_before_def_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        defined_line: u32,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = n.with_color_and_attr(HINT, ATTR);
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
        });
        let found = name.with_color_and_attr(ERR, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("定義({defined_line}行目)より前で{found}を参照することは出来ません"),
                    "simplified_chinese" => format!("在{found}定义({defined_line}行)之前引用是不允许的"),
                    "traditional_chinese" => format!("在{found}定義({defined_line}行)之前引用是不允許的"),
                    "english" => format!("cannot access {found} before its definition (line {defined_line})"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn access_deleted_var_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        del_line: u32,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = n.with_color_and_attr(HINT, ATTR);
            switch_lang!(
                "japanese" => format!("似た名前の変数があります: {n}"),
                "simplified_chinese" => format!("存在相同名称变量: {n}"),
                "traditional_chinese" => format!("存在相同名稱變量: {n}"),
                "english" => format!("exists a similar name variable: {n}"),
            )
        });
        let found = name.with_color_and_attr(ERR, ATTR);
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("削除された変数{found}を参照することは出来ません({del_line}行目で削除)"),
                    "simplified_chinese" => format!("不能引用已删除的变量{found}({del_line}行)"),
                    "traditional_chinese" => format!("不能引用已刪除的變量{found}({del_line}行)"),
                    "english" => format!("cannot access deleted variable {found} (deleted at line {del_line})"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_type_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let name = readable_name(name);
        let hint = similar_name.map(|n| {
            let n = StyledStr::new(n, Some(HINT), Some(ATTR));
            switch_lang!(
                "japanese" => format!("似た名前の型があります: {n}"),
                "simplified_chinese" => format!("存在相同名称类型: {n}"),
                "traditional_chinese" => format!("存在相同名稱類型: {n}"),
                "english" => format!("exists a similar name type: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{found}という型は定義されていません"),
                    "simplified_chinese" => format!("{found}未定义"),
                    "traditional_chinese" => format!("{found}未定義"),
                    "english" => format!("Type {found} is not defined"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn not_a_type_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = readable_name(name);
        let hint = {
            let n = StyledStr::new(name, Some(HINT), Some(ATTR));
            Some(switch_lang!(
                "japanese" => format!("{{{n}}}({n}のみを要素に持つ型)ではありませんか?"),
                "simplified_chinese" => format!("{{{n}}}({n}的元素只有{n})是不是?"),
                "traditional_chinese" => format!("{{{n}}}({n}的元素只有{n})是不是?"),
                "english" => format!("Do you mean {{{n}}}, a type that has only {n}?"),
            ))
        };
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{found}は型ではありません"),
                    "simplified_chinese" => format!("{found}不是类型"),
                    "traditional_chinese" => format!("{found}不是類型"),
                    "english" => format!("{found} is not a type"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn type_not_found(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        typ: &Type,
    ) -> Self {
        let typ = StyledString::new(typ.to_string(), Some(ERR), Some(ATTR));
        let hint = Some(switch_lang!(
            "japanese" => format!("恐らくこれはErgコンパイラのバグです、{URL}へ報告してください"),
            "simplified_chinese" => format!("这可能是Erg编译器的错误，请报告给{URL}"),
            "traditional_chinese" => format!("這可能是Erg編譯器的錯誤，請報告給{URL}"),
            "english" => format!("This may be a bug of Erg compiler, please report to {URL}"),
        ));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{typ}という型が見つかりませんでした"),
                    "simplified_chinese" => format!("{typ}未定义"),
                    "traditional_chinese" => format!("{typ}未定義"),
                    "english" => format!("Type {typ} is not found"),
                ),
                errno,
                NameError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn no_attr_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        obj_t: &Type,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let hint = similar_name.map(|n| {
            switch_lang!(
                "japanese" => format!("似た名前の属性があります: {n}"),
                "simplified_chinese" => format!("具有相同名称的属性: {n}"),
                "traditional_chinese" => format!("具有相同名稱的屬性: {n}"),
                "english" => format!("has a similar name attribute: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{obj_t}型オブジェクトに{found}という属性はありません"),
                    "simplified_chinese" => format!("{obj_t}对象没有属性{found}"),
                    "traditional_chinese" => format!("{obj_t}對像沒有屬性{found}"),
                    "english" => format!("{obj_t} object has no attribute {found}"),
                ),
                errno,
                AttributeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn detailed_no_attr_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        obj_t: &Type,
        name: &str,
        similar_name: Option<&str>,
        similar_info: Option<&VarInfo>,
    ) -> Self {
        let hint = similar_name.map(|n| {
            let vis = similar_info.map_or("".into(), |vi| vi.vis.modifier.display());
            let kind = similar_info.map_or("", |vi| vi.kind.display());
            switch_lang!(
                "japanese" => format!("似た名前の{vis}{kind}属性があります: {n}"),
                "simplified_chinese" => format!("具有相同名称的{vis}{kind}属性: {n}"),
                "traditional_chinese" => format!("具有相同名稱的{vis}{kind}屬性: {n}"),
                "english" => format!("has a similar name {vis} {kind} attribute: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{obj_t}型オブジェクトに{found}という属性はありません"),
                    "simplified_chinese" => format!("{obj_t}对象没有属性{found}"),
                    "traditional_chinese" => format!("{obj_t}對像沒有屬性{found}"),
                    "english" => format!("{obj_t} object has no attribute {found}"),
                ),
                errno,
                AttributeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn singular_no_attr_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        obj_name: &str,
        obj_t: &Type,
        name: &str,
        similar_name: Option<&str>,
    ) -> Self {
        let hint = similar_name.map(|n| {
            let n = StyledStr::new(n, Some(HINT), Some(ATTR));
            switch_lang!(
                "japanese" => format!("似た名前の属性があります: {n}"),
                "simplified_chinese" => format!("具有相同名称的属性: {n}"),
                "traditional_chinese" => format!("具有相同名稱的屬性: {n}"),
                "english" => format!("has a similar name attribute: {n}"),
            )
        });
        let found = StyledString::new(name, Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{obj_name}(: {obj_t})に{found}という属性はありません"),
                    "simplified_chinese" => format!("{obj_name}(: {obj_t})没有属性{found}"),
                    "traditional_chinese" => format!("{obj_name}(: {obj_t})沒有屬性{found}"),
                    "english" => format!("{obj_name}(: {obj_t}) has no attribute {found}"),
                ),
                errno,
                AttributeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn shadow_special_namespace_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = StyledStr::new(readable_name(name), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("特殊名前空間{name}と同名の変数は定義できません"),
                    "simplified_chinese" => format!("不能定义与特殊命名空间{name}同名的变量"),
                    "traditional_chinese" => format!("不能定義與特殊命名空間{name}同名的變量"),
                    "english" => format!("cannot define variable with the same name as special namespace {name}"),
                ),
                errno,
                AssignError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn reassign_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = StyledStr::new(readable_name(name), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("変数{name}に複数回代入することはできません"),
                    "simplified_chinese" => format!("不能为变量{name}分配多次"),
                    "traditional_chinese" => format!("不能為變量{name}分配多次"),
                    "english" => format!("variable {name} cannot be assigned more than once"),
                ),
                errno,
                AssignError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn del_error(
        input: Input,
        errno: usize,
        ident: &Identifier,
        is_const: bool,
        caused_by: String,
    ) -> Self {
        let prefix = if is_const {
            switch_lang!(
                "japanese" => "定数",
                "simplified_chinese" => "定数",
                "traditional_chinese" => "定數",
                "english" => "constant",
            )
        } else {
            switch_lang!(
                "japanese" => "組み込み変数",
                "simplified_chinese" => "内置变量",
                "traditional_chinese" => "内置變量",
                "english" => "built-in variable",
            )
        };
        let name = StyledString::new(readable_name(ident.inspect()), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(ident.loc())],
                switch_lang!(
                    "japanese" => format!("{prefix}{name}は削除できません"),
                    "simplified_chinese" => format!("{prefix}{name}不能删除"),
                    "traditional_chinese" => format!("{prefix}{name}不能刪除"),
                    "english" => format!("{prefix} {name} cannot be deleted"),
                ),
                errno,
                NameError,
                ident.loc(),
            ),
            input,
            caused_by,
        )
    }

    pub fn visibility_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        vis: Visibility,
    ) -> Self {
        let visibility = vis.modifier.display();
        let found = StyledString::new(readable_name(name), Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{found}は{visibility}変数です"),
                    "simplified_chinese" => format!("{found}是{visibility}变量",),
                    "traditional_chinese" => format!("{found}是{visibility}變量",),
                    "english" => format!("{found} is {visibility} variable",),
                ),
                errno,
                VisibilityError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn override_error<S: Into<String>>(
        input: Input,
        errno: usize,
        name: &str,
        name_loc: Location,
        superclass: &Type,
        caused_by: S,
    ) -> Self {
        let name = StyledString::new(name, Some(ERR), Some(ATTR));
        let superclass = StyledString::new(format!("{superclass}"), Some(WARN), Some(ATTR));
        let hint = Some(
            switch_lang!(
                "japanese" => {
                    let mut ovr = StyledStrings::default();
                    ovr.push_str_with_color_and_attr("@Override", HINT, ATTR);
                    ovr.push_str("デコレータを使用してください");
                    ovr
            },
                "simplified_chinese" => {
                    let mut ovr = StyledStrings::default();
                    ovr.push_str("请使用");
                    ovr.push_str_with_color_and_attr("@Override", HINT, ATTR);
                    ovr.push_str("装饰器");
                    ovr
                },
                "traditional_chinese" => {
                    let mut ovr = StyledStrings::default();
                    ovr.push_str("請使用");
                    ovr.push_str_with_color_and_attr("@Override", HINT, ATTR);
                    ovr.push_str("裝飾器");
                    ovr
                },
                "english" => {
                    let mut ovr = StyledStrings::default();
                    ovr.push_str("use ");
                    ovr.push_str_with_color_and_attr("@Override", HINT, ATTR);
                    ovr.push_str(" decorator");
                    ovr
                },
            )
            .to_string(),
        );
        let sub_msg = switch_lang!(
            "japanese" => "デフォルトでオーバーライドはできません",
            "simplified_chinese" => "默认不可重写",
            "simplified_chinese" => "默認不可重寫",
            "english" => "cannot override by default",
        );
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(
                    name_loc,
                    vec![sub_msg.to_string()],
                    hint,
                )],
                switch_lang!(
                    "japanese" => format!(
                        "{name}は{superclass}で既に定義されています",
                    ),
                    "simplified_chinese" => format!(
                        "{name}已在{superclass}中定义",
                    ),
                    "traditional_chinese" => format!(
                        "{name}已在{superclass}中定義",
                    ),
                    "english" => format!(
                        "{name} is already defined in {superclass}",
                    ),
                ),
                errno,
                NameError,
                name_loc,
            ),
            input,
            caused_by.into(),
        )
    }

    pub fn inheritance_error(
        input: Input,
        errno: usize,
        class: String,
        loc: Location,
        caused_by: String,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{class}は継承できません"),
                    "simplified_chinese" => format!("{class}不可继承"),
                    "traditional_chinese" => format!("{class}不可繼承"),
                    "english" => format!("{class} is not inheritable"),
                ),
                errno,
                InheritanceError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn file_error(
        input: Input,
        errno: usize,
        desc: String,
        loc: Location,
        caused_by: String,
        hint: Option<String>,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                desc,
                errno,
                IoError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn module_env_error(
        input: Input,
        errno: usize,
        mod_name: &str,
        loc: Location,
        caused_by: String,
    ) -> Self {
        let desc = switch_lang!(
            "japanese" => format!("{mod_name}モジュールはお使いの環境をサポートしていません"),
            "simplified_chinese" => format!("{mod_name}模块不支持您的环境"),
            "traditional_chinese" => format!("{mod_name}模塊不支持您的環境"),
            "english" => format!("module {mod_name} is not supported in your environment"),
        );
        Self::file_error(input, errno, desc, loc, caused_by, None)
    }

    pub fn import_error(
        input: Input,
        errno: usize,
        desc: String,
        loc: Location,
        caused_by: String,
        similar_erg_mod: Option<Str>,
        similar_py_mod: Option<Str>,
    ) -> Self {
        let mut erg_str = StyledStrings::default();
        let mut py_str = StyledStrings::default();
        let hint = switch_lang!(
        "japanese" => {
            match (similar_erg_mod, similar_py_mod) {
                (Some(erg), Some(py)) => {
                    erg_str.push_str("似た名前のergモジュールが存在します: ");
                    erg_str.push_str_with_color_and_attr(erg, HINT, ATTR);
                    py_str.push_str("似た名前のpythonモジュールが存在します: ");
                    py_str.push_str_with_color_and_attr(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("pythonのモジュールをインポートするためには");
                    hint.push_str_with_color_and_attr("pyimport", ACCENT, ATTR);
                    hint.push_str("を使用してください");
                    Some(hint.to_string())
                }
                (Some(erg), None) => {
                    erg_str.push_str("似た名前のergモジュールが存在します");
                    erg_str.push_str_with_color_and_attr(erg, ACCENT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("ergのモジュールをインポートするためには、pyimportではなく");
                    hint.push_str_with_color_and_attr("import", ACCENT, ATTR);
                    hint.push_str("を使用してください");
                    Some(hint.to_string())
                }
                (None, Some(py)) => {
                    py_str.push_str("似た名前のpythonモジュールが存在します");
                    py_str.push_str_with_color_and_attr(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("pythonのモジュールをインポートするためには、importではなく");
                    hint.push_str_with_color_and_attr("pyimport", ACCENT, ATTR);
                    hint.push_str("を使用してください");
                    Some(hint.to_string())
                }
                (None, None) => None,
            }
        },
        "simplified_chinese" => {
            match (similar_erg_mod, similar_py_mod) {
                (Some(erg), Some(py)) => {
                    erg_str.push_str("存在相似名称的erg模块: ");
                    erg_str.push_str_with_color_and_attr(erg, HINT, ATTR);
                    py_str.push_str("存在相似名称的python模块: ");
                    py_str.push_str_with_color_and_attr(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要导入python模块,请使用");
                    hint.push_str_with_color_and_attr("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (Some(erg), None) => {
                    erg_str.push_str("存在相似名称的erg模块: ");
                    erg_str.push_str_with_color_and_attr(erg, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要导入erg模块,请使用");
                    hint.push_str_with_color_and_attr("import", ACCENT, ATTR);
                    hint.push_str("而不是pyimport");
                    Some(hint.to_string())
                }
                (None, Some(py)) => {
                    py_str.push_str("存在相似名称的python模块: ");
                    py_str.push_str_with_color_and_attr(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要导入python模块,请使用");
                    hint.push_str_with_color_and_attr("pyimport", ACCENT, ATTR);
                    hint.push_str("而不是import");
                    Some(hint.to_string())
                }
                (None, None) => None,
            }
        },
        "traditional_chinese" => {
            match (similar_erg_mod, similar_py_mod) {
                (Some(erg), Some(py)) => {
                    erg_str.push_str("存在類似名稱的erg模塊: ");
                    erg_str.push_str_with_color_and_attr(erg, HINT, ATTR);
                    py_str.push_str("存在類似名稱的python模塊: ");
                    py_str.push_str_with_color_and_attr(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要導入python模塊, 請使用");
                    hint.push_str_with_color_and_attr("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (Some(erg), None) => {
                    erg_str.push_str("存在類似名稱的erg模塊: ");
                    erg_str.push_str_with_color_and_attr(erg, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要導入erg模塊, 請使用");
                    hint.push_str_with_color_and_attr("import", ACCENT, ATTR);
                    hint.push_str("而不是pyimport");
                    Some(hint.to_string())
                }
                (None, Some(py)) => {
                    py_str.push_str("存在類似名稱的python模塊: ");
                    py_str.push_str_with_color_and_attr(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("要導入python模塊, 請使用");
                    hint.push_str_with_color_and_attr("pyimport", ACCENT, ATTR);
                    hint.push_str("而不是import");
                    Some(hint.to_string())
                }
                (None, None) => None,
            }
        },
        "english" => {
            match (similar_erg_mod, similar_py_mod) {
                (Some(erg), Some(py)) => {
                    erg_str.push_str("similar name erg module exists: ");
                    erg_str.push_str_with_color_and_attr(erg, HINT, ATTR);
                    py_str.push_str("similar name python module exists: ");
                    py_str.push_str_with_color_and_attr(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("to import python modules, use ");
                    hint.push_str_with_color_and_attr("pyimport", ACCENT, ATTR);
                    Some(hint.to_string())
                }
                (Some(erg), None) => {
                    erg_str.push_str("similar name erg module exists: ");
                    erg_str.push_str_with_color_and_attr(erg, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("to import erg modules, use ");
                    hint.push_str_with_color_and_attr("import", ACCENT, ATTR);
                    hint.push_str(" (not pyimport)");
                    Some(hint.to_string())
                }
                (None, Some(py)) => {
                    py_str.push_str("similar name python module exists: ");
                    py_str.push_str_with_color_and_attr(py, HINT, ATTR);
                    let mut hint  = StyledStrings::default();
                    hint.push_str("to import python modules, use ");
                    hint.push_str_with_color_and_attr("pyimport", ACCENT, ATTR);
                    hint.push_str(" (not import)");
                    Some(hint.to_string())
                }
                (None, None) => None,
            }
        },
        );
        // .to_string().is_empty() is not necessarily empty because there are Color or Attribute that are not displayed
        let msg = match (erg_str.is_empty(), py_str.is_empty()) {
            (false, false) => vec![erg_str.to_string(), py_str.to_string()],
            (false, true) => vec![erg_str.to_string()],
            (true, false) => vec![py_str.to_string()],
            (true, true) => vec![],
        };
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, msg, hint)],
                desc,
                errno,
                ImportError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn inner_typedef_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
    ) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("型はトップレベルで定義されなければなりません"),
                    "simplified_chinese" => format!("类型必须在顶层定义"),
                    "traditional_chinese" => format!("類型必須在頂層定義"),
                    "english" => format!("types must be defined at the top level"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn declare_error(input: Input, errno: usize, loc: Location, caused_by: String) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("d.erファイル内では宣言、別名定義のみが許可されています"),
                    "simplified_chinese" => format!("在d.er文件中只允许声明和别名定义"),
                    "traditional_chinese" => format!("在d.er文件中只允許聲明和別名定義"),
                    "english" => format!("declarations and alias definitions are only allowed in d.er files"),
                ),
                errno,
                SyntaxError,
                loc,
            ),
            input,
            caused_by,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn invalid_type_cast_error(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
        base: &Type,
        cast_to: &Type,
        hint: Option<String>,
    ) -> Self {
        let name = StyledString::new(name, Some(WARN), Some(ATTR));
        let base = StyledString::new(format!("{base}"), Some(WARN), Some(ATTR));
        let found = StyledString::new(format!("{cast_to}"), Some(ERR), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], hint)],
                switch_lang!(
                    "japanese" => format!("{name}: {base}を{found}にキャストすることはできません"),
                    "simplified_chinese" => format!("{name}: {base}无法转换为{found}"),
                    "traditional_chinese" => format!("{name}: {base}無法轉換為{found}"),
                    "english" => format!("{name}: {base} cannot be cast to {found}"),
                ),
                errno,
                TypeError,
                loc,
            ),
            input,
            caused_by,
        )
    }
}

impl LowerWarning {
    pub fn unused_warning(
        input: Input,
        errno: usize,
        loc: Location,
        name: &str,
        caused_by: String,
    ) -> Self {
        let name = StyledString::new(readable_name(name), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("{name}は使用されていません"),
                    "simplified_chinese" => format!("{name}未使用"),
                    "traditional_chinese" => format!("{name}未使用"),
                    "english" => format!("{name} is not used"),
                ),
                errno,
                UnusedWarning,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn union_return_type_warning(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        fn_name: &str,
        typ: &Type,
    ) -> Self {
        let fn_name = fn_name.with_color(Color::Yellow);
        let hint = switch_lang!(
            "japanese" => format!("`{fn_name}(...): {typ} = ...`など明示的に戻り値型を指定してください"),
            "simplified_chinese" => format!("请明确指定函数{fn_name}的返回类型，例如`{fn_name}(...): {typ} = ...`"),
            "traditional_chinese" => format!("請明確指定函數{fn_name}的返回類型，例如`{fn_name}(...): {typ} = ...`"),
            "english" => format!("please explicitly specify the return type of function {fn_name}, for example `{fn_name}(...): {typ} = ...`"),
        );
        LowerError::new(
            ErrorCore::new(
                vec![SubMessage::ambiguous_new(loc, vec![], Some(hint))],
                switch_lang!(
                    "japanese" => format!("関数{fn_name}の戻り値型が単一ではありません"),
                    "simplified_chinese" => format!("函数{fn_name}的返回类型不是单一的"),
                    "traditional_chinese" => format!("函數{fn_name}的返回類型不是單一的"),
                    "english" => format!("the return type of function {fn_name} is not single"),
                ),
                errno,
                TypeWarning,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn builtin_exists_warning(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = StyledStr::new(readable_name(name), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("同名の組み込み関数{name}が既に存在します"),
                    "simplified_chinese" => format!("已存在同名的内置函数{name}"),
                    "traditional_chinese" => format!("已存在同名的內置函數{name}"),
                    "english" => format!("a built-in function named {name} already exists"),
                ),
                errno,
                NameWarning,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn use_cast_warning(input: Input, errno: usize, loc: Location, caused_by: String) -> Self {
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => "typing.castの使用は推奨されません、type narrowingなどを使ってください",
                    "simplified_chinese" => "不推荐使用typing.cast、请使用type narrowing等",
                    "traditional_chinese" => "不推薦使用typing.cast、請使用type narrowing等",
                    "english" => "using typing.cast is not recommended, please use type narrowing etc. instead",
                ),
                errno,
                TypeWarning,
                loc,
            ),
            input,
            caused_by,
        )
    }

    pub fn same_name_instance_attr_warning(
        input: Input,
        errno: usize,
        loc: Location,
        caused_by: String,
        name: &str,
    ) -> Self {
        let name = StyledStr::new(readable_name(name), Some(WARN), Some(ATTR));
        Self::new(
            ErrorCore::new(
                vec![SubMessage::only_loc(loc)],
                switch_lang!(
                    "japanese" => format!("同名のインスタンス属性{name}が存在します"),
                    "simplified_chinese" => format!("同名的实例属性{name}已存在"),
                    "traditional_chinese" => format!("同名的實例屬性{name}已存在"),
                    "english" => format!("an instance attribute named {name} already exists"),
                ),
                errno,
                NameWarning,
                loc,
            ),
            input,
            caused_by,
        )
    }
}
