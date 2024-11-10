use std::env::var;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::normalize_path;
use crate::python_util::{_opt_which_python, get_sys_path};
use crate::style::colors::*;
use crate::style::RESET;

fn fallback_erg_path() -> PathBuf {
    #[allow(deprecated)]
    std::env::home_dir().map_or(PathBuf::from(".erg"), |path| path.join(".erg"))
}

fn _erg_path() -> PathBuf {
    let path = var("ERG_PATH").unwrap_or_else(|_| env!("CARGO_ERG_PATH").to_string());
    PathBuf::from(path).canonicalize().unwrap_or_else(|_| {
        let fallback = fallback_erg_path();
        if !fallback.exists() {
            eprintln!("{RED}[ERR] ERG_PATH not found{RESET}");
        }
        fallback
    })
}
fn _erg_std_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("std")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("{RED}[ERR] ERG_PATH/lib/std not found{RESET}");
            fallback_erg_path().join("lib/std")
        })
}
fn _erg_core_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("core")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("{RED}[ERR] ERG_PATH/lib/core not found{RESET}");
            fallback_erg_path().join("lib/core")
        })
}
fn _erg_core_decl_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("core.d")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("{RED}[ERR] ERG_PATH/lib/core.d not found {RESET}");
            fallback_erg_path().join("lib/core.d")
        })
}
fn _erg_pystd_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("pystd")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("{RED}[ERR] ERG_PATH/lib/pystd not found {RESET}");
            fallback_erg_path().join("lib/pystd")
        })
}
fn _erg_pkgs_path() -> PathBuf {
    _erg_path()
        .join("lib")
        .join("pkgs")
        .canonicalize()
        .unwrap_or_else(|_| {
            // eprintln!("{RED}[ERR] ERG_PATH/lib/pkgs not found {RESET}");
            fallback_erg_path().join("lib/pkgs")
        })
}
fn _sys_path() -> impl Iterator<Item = PathBuf> {
    get_sys_path(None).unwrap_or_default().into_iter().map(|p| {
        p.canonicalize().unwrap_or_else(|_| {
            // eprintln!("{RED}[ERR] {} not found {RESET}", p.display());
            fallback_erg_path().join("lib/pkgs")
        })
    })
}
fn _python_site_packages() -> impl Iterator<Item = PathBuf> {
    let paths = if Path::new("./.venv/lib").is_dir() {
        let mut paths = vec![];
        for entry in Path::new("./.venv/lib").read_dir().unwrap().flatten() {
            if entry.file_type().unwrap().is_dir() {
                let mut path = entry.path();
                path.push("site-packages");
                if path.is_dir() {
                    paths.push(path);
                }
            }
        }
        paths
    } else {
        get_sys_path(None).unwrap_or_default()
    };
    paths
        .into_iter()
        .filter(|p| p.ends_with("site-packages"))
        .map(|p| {
            p.canonicalize().unwrap_or_else(|_| {
                // eprintln!("{RED}[ERR] {} not found {RESET}", p.display());
                fallback_erg_path().join("lib/pkgs")
            })
        })
}

pub static ERG_PATH: OnceLock<PathBuf> = OnceLock::new();
pub static ERG_CORE_PATH: OnceLock<PathBuf> = OnceLock::new();
pub static ERG_CORE_DECL_PATH: OnceLock<PathBuf> = OnceLock::new();
pub static ERG_STD_PATH: OnceLock<PathBuf> = OnceLock::new();
pub static ERG_PYSTD_PATH: OnceLock<PathBuf> = OnceLock::new();
pub static ERG_PKGS_PATH: OnceLock<PathBuf> = OnceLock::new();
pub static PYTHON_SYS_PATH: OnceLock<Vec<PathBuf>> = OnceLock::new();
pub static PYTHON_SITE_PACKAGES: OnceLock<Vec<PathBuf>> = OnceLock::new();
pub static PYTHON_PATH: OnceLock<Result<String, String>> = OnceLock::new();

/// == `Path::new("~/.erg")` if ERG_PATH is not set
pub fn erg_path() -> &'static PathBuf {
    ERG_PATH.get_or_init(|| normalize_path(_erg_path())) // .with(|s| s.clone())
}

/// == `Path::new("~/.erg/lib/core")` if ERG_PATH is not set
pub fn erg_core_path() -> &'static PathBuf {
    ERG_CORE_PATH.get_or_init(|| normalize_path(_erg_core_path()))
}

/// == `Path::new("~/.erg/lib/core.d")` if ERG_PATH is not set
pub fn erg_core_decl_path() -> &'static PathBuf {
    ERG_CORE_DECL_PATH.get_or_init(|| normalize_path(_erg_core_decl_path()))
}

/// == `Path::new("~/.erg/lib/std")` if ERG_PATH is not set
pub fn erg_std_path() -> &'static PathBuf {
    ERG_STD_PATH.get_or_init(|| normalize_path(_erg_std_path()))
}

/// == `Path::new("~/.erg/lib/pystd")` if ERG_PATH is not set
pub fn erg_pystd_path() -> &'static PathBuf {
    ERG_PYSTD_PATH.get_or_init(|| normalize_path(_erg_pystd_path()))
}

/// == `Path::new("~/.erg/lib/pkgs")` if ERG_PATH is not set
pub fn erg_pkgs_path() -> &'static PathBuf {
    ERG_PKGS_PATH.get_or_init(|| normalize_path(_erg_pkgs_path()))
}

pub fn python_sys_path() -> &'static Vec<PathBuf> {
    PYTHON_SYS_PATH.get_or_init(|| _sys_path().collect())
}

pub fn python_site_packages() -> &'static Vec<PathBuf> {
    PYTHON_SITE_PACKAGES.get_or_init(|| _python_site_packages().collect())
}

pub fn opt_which_python() -> Result<&'static String, &'static String> {
    PYTHON_PATH.get_or_init(_opt_which_python).as_ref()
}

pub fn is_std_decl_path(path: &Path) -> bool {
    path.starts_with(erg_pystd_path().as_path())
}

pub fn is_pystd_main_module(path: &Path) -> bool {
    let mut path = PathBuf::from(path);
    if path.ends_with("__init__.d.er") {
        path.pop();
        path.pop();
    } else {
        path.pop();
    }
    path == erg_pystd_path().as_path()
}
