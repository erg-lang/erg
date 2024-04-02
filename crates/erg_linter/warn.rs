use erg_common::error::{ErrorCore, ErrorKind, Location};
use erg_common::io::Input;
use erg_compiler::error::CompileWarning;

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
