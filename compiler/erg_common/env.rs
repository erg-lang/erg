use std::path::PathBuf;

fn _erg_path() -> PathBuf {
    let path = option_env!("ERG_PATH").unwrap_or(env!("CARGO_ERG_PATH"));
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

use std::cell::RefCell;
thread_local! {
    pub static ERG_PATH: RefCell<PathBuf> = RefCell::new(_erg_path());
    pub static ERG_STD_PATH: RefCell<PathBuf> = RefCell::new(_erg_std_path());
    pub static ERG_PYSTD_PATH: RefCell<PathBuf> = RefCell::new(_erg_pystd_path());
    pub static ERG_EXTERNAL_LIB_PATH: RefCell<PathBuf> = RefCell::new(_erg_external_lib_path());
}

pub fn erg_path() -> PathBuf {
    ERG_PATH.with(|s| s.borrow().clone())
}

pub fn erg_std_path() -> PathBuf {
    ERG_STD_PATH.with(|s| s.borrow().clone())
}

pub fn erg_pystd_path() -> PathBuf {
    ERG_PYSTD_PATH.with(|s| s.borrow().clone())
}

pub fn erg_external_lib_path() -> PathBuf {
    ERG_EXTERNAL_LIB_PATH.with(|s| s.borrow().clone())
}
