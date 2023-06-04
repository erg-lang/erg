pub const SEMVER: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH_SHORT: &str = env!("GIT_HASH_SHORT");
pub const BUILD_DATE: &str = env!("BUILD_DATE");

pub const PYTHON_MODE: bool = cfg!(feature = "py_compat");
pub const ERG_MODE: bool = !cfg!(feature = "py_compat");
pub const ELS: bool = cfg!(feature = "els");
pub const DEBUG_MODE: bool = cfg!(feature = "debug");
pub const EXPERIMENTAL_MODE: bool = cfg!(feature = "experimental");
