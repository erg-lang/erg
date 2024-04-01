pub const SEMVER: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH_SHORT: &str = env!("GIT_HASH_SHORT");
pub const CASE_SENSITIVE: bool = matches!(env!("CASE_SENSITIVE").as_bytes(), b"true");

pub const PYTHON_MODE: bool = cfg!(feature = "py_compat");
pub const ERG_MODE: bool = !cfg!(feature = "py_compat");
pub const ELS: bool = cfg!(feature = "els");
pub const DEBUG_MODE: bool = cfg!(feature = "debug");
pub const EXPERIMENTAL_MODE: bool = cfg!(feature = "experimental");
pub const BACKTRACE_MODE: bool = cfg!(feature = "backtrace");
pub const GAL: bool = cfg!(feature = "gal");

pub fn build_date() -> String {
    use std::io::BufRead;
    let path = crate::env::erg_path().join("build.data");
    let Ok(file) = std::fs::File::open(path) else {
        return "???".to_string();
    };
    let reader = std::io::BufReader::new(file);
    let Some(Ok(date)) = reader.lines().next() else {
        return "???".to_string();
    };
    date
}
