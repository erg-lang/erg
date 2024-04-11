use erg_common::error::{ErrorCore, ErrorKind, Location, SubMessage};
use erg_common::io::Input;
use erg_common::traits::NoTypeDisplay;
use erg_compiler::error::CompileWarning;
use erg_compiler::hir::Expr;

pub(crate) fn too_many_params(input: Input, caused_by: String, loc: Location) -> CompileWarning {
    CompileWarning::new(
        ErrorCore::new(
            vec![],
            "too many parameters".to_string(),
            0,
            ErrorKind::Warning,
            loc,
        ),
        input,
        caused_by,
    )
}

pub(crate) fn true_comparison(
    expr: &Expr,
    input: Input,
    caused_by: String,
    loc: Location,
) -> CompileWarning {
    CompileWarning::new(
        ErrorCore::new(
            vec![SubMessage::ambiguous_new(
                loc,
                vec![],
                Some(format!("just write: {}", expr.to_string_notype())),
            )],
            "equality checks against True are redundant".to_string(),
            0,
            ErrorKind::Warning,
            loc,
        ),
        input,
        caused_by,
    )
}

pub(crate) fn false_comparison(
    expr: &Expr,
    input: Input,
    caused_by: String,
    loc: Location,
) -> CompileWarning {
    CompileWarning::new(
        ErrorCore::new(
            vec![SubMessage::ambiguous_new(
                loc,
                vec![],
                Some(format!("just write: not {}", expr.to_string_notype())),
            )],
            "equality checks against False are redundant".to_string(),
            0,
            ErrorKind::Warning,
            loc,
        ),
        input,
        caused_by,
    )
}
