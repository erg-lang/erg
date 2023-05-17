use std::env::var;
use std::path::{Path, PathBuf};

use crate::normalize_path;
use crate::python_util::get_sys_path;
use crate::style::colors::*;
use crate::style::RESET;

fn _erg_path() -> PathBuf {
    let path = var("ERG_PATH").unwrap_or_else(|_| env!("CARGO_ERG_PATH").to_string());
    PathBuf::from(path).canonicalize().unwrap_or_else(|_| {
        eprintln!("{RED}[ERR] ERG_PATH not found{RESET}");
        PathBuf::from(".")
    })
}
fn _erg_std_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("std")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("{RED}[ERR] ERG_PATH/lib/std not found{RESET}");
            PathBuf::from("lib/std/")
        })
}
fn _erg_std_decl_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("std.d")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("{RED}[ERR] ERG_PATH/lib/std.d not found {RESET}");
            PathBuf::from("lib/std.d/")
        })
}
fn _erg_pystd_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("pystd")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("{RED}[ERR] ERG_PATH/lib/pystd not found {RESET}");
            PathBuf::from("lib/pystd/")
        })
}
fn _erg_external_lib_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("external")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("{RED}[ERR] ERG_PATH/lib/external not found {RESET}");
            PathBuf::from("lib/external/")
        })
}
fn _python_site_packages() -> impl Iterator<Item = PathBuf> {
    get_sys_path()
        .into_iter()
        .filter(|p| p.ends_with("site-packages"))
        .map(|p| {
            p.canonicalize().unwrap_or_else(|_| {
                eprintln!("{RED}[ERR] ERG_PATH/lib/external not found {RESET}");
                PathBuf::from("lib/external/")
            })
        })
}

thread_local! {
    pub static ERG_PATH: PathBuf = normalize_path(_erg_path());
    pub static ERG_STD_PATH: PathBuf = normalize_path(_erg_std_path());
    pub static ERG_STD_DECL_PATH: PathBuf = normalize_path(_erg_std_decl_path());
    pub static ERG_PYSTD_PATH: PathBuf = normalize_path(_erg_pystd_path());
    pub static ERG_EXTERNAL_LIB_PATH: PathBuf = normalize_path(_erg_external_lib_path());
    pub static PYTHON_SITE_PACKAGES: Vec<PathBuf> = _python_site_packages().collect();
}

pub fn erg_path() -> PathBuf {
    ERG_PATH.with(|s| s.clone())
}

pub fn erg_std_path() -> PathBuf {
    ERG_STD_PATH.with(|s| s.clone())
}

pub fn erg_std_decl_path() -> PathBuf {
    ERG_STD_DECL_PATH.with(|s| s.clone())
}

pub fn erg_pystd_path() -> PathBuf {
    ERG_PYSTD_PATH.with(|s| s.clone())
}

pub fn erg_py_external_lib_path() -> PathBuf {
    ERG_EXTERNAL_LIB_PATH.with(|s| s.clone())
}

pub fn python_site_packages() -> Vec<PathBuf> {
    PYTHON_SITE_PACKAGES.with(|s| s.clone())
}

pub fn is_pystd_main_module(path: &Path) -> bool {
    let mut path = PathBuf::from(path);
    if path.ends_with("__init__.d.er") {
        path.pop();
        path.pop();
    } else {
        path.pop();
    }
    let pystd_path = erg_pystd_path();
    path == pystd_path
}
