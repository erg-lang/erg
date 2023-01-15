use std::path::PathBuf;

use crate::normalize_path;

fn _erg_path() -> PathBuf {
    #[cfg(feature = "no_std")]
    let path = "./".to_string();
    #[cfg(not(feature = "no_std"))]
    let path = std::env::var("ERG_PATH").unwrap_or_else(|_| env!("CARGO_ERG_PATH").to_string());
    PathBuf::from(path)
        .canonicalize()
        .expect("ERG_PATH not found")
}
fn _erg_std_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("std")
        .canonicalize()
        .expect("ERG_PATH/lib/std not found")
}
fn _erg_pystd_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("pystd")
        .canonicalize()
        .expect("ERG_PATH/lib/pystd not found")
}
fn _erg_external_lib_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("external")
        .canonicalize()
        .expect("ERG_PATH/lib/external not found")
}

thread_local! {
    pub static ERG_PATH: PathBuf = normalize_path(_erg_path());
    pub static ERG_STD_PATH: PathBuf = normalize_path(_erg_std_path());
    pub static ERG_PYSTD_PATH: PathBuf = normalize_path(_erg_pystd_path());
    pub static ERG_EXTERNAL_LIB_PATH: PathBuf = normalize_path(_erg_external_lib_path());
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
