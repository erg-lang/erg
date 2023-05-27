use std::env::var;
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;

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
    get_sys_path(None)
        .unwrap_or(vec![])
        .into_iter()
        .filter(|p| p.ends_with("site-packages"))
        .map(|p| {
            p.canonicalize().unwrap_or_else(|_| {
                eprintln!("{RED}[ERR] ERG_PATH/lib/external not found {RESET}");
                PathBuf::from("lib/external/")
            })
        })
}

pub static ERG_PATH: Lazy<PathBuf> = Lazy::new(|| normalize_path(_erg_path()));
pub static ERG_STD_PATH: Lazy<PathBuf> = Lazy::new(|| normalize_path(_erg_std_path()));
pub static ERG_STD_DECL_PATH: Lazy<PathBuf> = Lazy::new(|| normalize_path(_erg_std_decl_path()));
pub static ERG_PYSTD_PATH: Lazy<PathBuf> = Lazy::new(|| normalize_path(_erg_pystd_path()));
pub static ERG_EXTERNAL_LIB_PATH: Lazy<PathBuf> =
    Lazy::new(|| normalize_path(_erg_external_lib_path()));
pub static PYTHON_SITE_PACKAGES: Lazy<Vec<PathBuf>> =
    Lazy::new(|| _python_site_packages().collect());

pub fn is_std_decl_path(path: &Path) -> bool {
    path.starts_with(ERG_PYSTD_PATH.as_path())
        || path.starts_with(ERG_STD_DECL_PATH.as_path())
        || path.starts_with(ERG_EXTERNAL_LIB_PATH.as_path())
}

pub fn is_pystd_main_module(path: &Path) -> bool {
    let mut path = PathBuf::from(path);
    if path.ends_with("__init__.d.er") {
        path.pop();
        path.pop();
    } else {
        path.pop();
    }
    // let pystd_path = erg_pystd_path();
    path == ERG_PYSTD_PATH.as_path()
}
