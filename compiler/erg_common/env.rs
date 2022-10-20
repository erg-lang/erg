use std::env::var;
use std::path::PathBuf;

fn _erg_path() -> PathBuf {
    let path = var("ERG_PATH").unwrap_or_else(|_| env!("CARGO_ERG_PATH").to_string());
    PathBuf::from(path).canonicalize().unwrap()
}
fn _erg_std_path() -> PathBuf {
    _erg_path().join("lib").join("std").canonicalize().unwrap()
}
fn _erg_pystd_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("pystd")
        .canonicalize()
        .unwrap()
}
fn _erg_external_lib_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("external")
        .canonicalize()
        .unwrap()
}

thread_local! {
    pub static ERG_PATH: PathBuf = _erg_path();
    pub static ERG_STD_PATH: PathBuf = _erg_std_path();
    pub static ERG_PYSTD_PATH: PathBuf = _erg_pystd_path();
    pub static ERG_EXTERNAL_LIB_PATH: PathBuf = _erg_external_lib_path();
}

pub fn erg_path() -> PathBuf {
    ERG_PATH.with(|s| s.clone())
}

pub fn erg_std_path() -> PathBuf {
    ERG_STD_PATH.with(|s| s.clone())
}

pub fn erg_pystd_path() -> PathBuf {
    ERG_PYSTD_PATH.with(|s| s.clone())
}

pub fn erg_external_lib_path() -> PathBuf {
    ERG_EXTERNAL_LIB_PATH.with(|s| s.clone())
}
