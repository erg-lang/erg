use std::path::PathBuf;

pub fn erg_path() -> PathBuf {
    let path = option_env!("ERG_PATH").unwrap_or(env!("CARGO_ERG_PATH"));
    PathBuf::from(path)
}

pub fn erg_std_path() -> PathBuf {
    erg_path().join("lib").join("std")
}

pub fn erg_pystd_path() -> PathBuf {
    erg_path().join("lib").join("pystd")
}

pub fn erg_external_lib_path() -> PathBuf {
    erg_path().join("lib").join("external")
}
