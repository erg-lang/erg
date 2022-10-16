use std::path::PathBuf;

pub fn erg_path() -> Option<PathBuf> {
    option_env!("ERG_PATH")
        .or_else(|| option_env!("CARGO_ERG_PATH"))
        .map(PathBuf::from)
}

pub fn erg_std_path() -> Option<PathBuf> {
    erg_path().map(|path| path.join("lib").join("std"))
}

pub fn erg_external_lib_path() -> Option<PathBuf> {
    erg_path().map(|path| path.join("lib").join("external"))
}
