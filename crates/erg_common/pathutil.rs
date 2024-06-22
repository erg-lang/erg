use std::borrow::Borrow;
use std::ffi::OsStr;
use std::fmt;
use std::ops::Deref;
use std::path::{Component, Path, PathBuf};

use crate::env::erg_pkgs_path;
use crate::{normalize_path, Str};

/// Guaranteed equivalence path.
///
/// `PathBuf` may give false equivalence decisions in non-case-sensitive file systems.
/// Use this for dictionary keys, etc.
/// See also: `els::util::NormalizedUrl`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct NormalizedPathBuf(PathBuf);

impl fmt::Display for NormalizedPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl<P: Into<PathBuf>> From<P> for NormalizedPathBuf {
    fn from(path: P) -> Self {
        NormalizedPathBuf::new(path.into())
    }
}

impl AsRef<Path> for NormalizedPathBuf {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

impl Borrow<PathBuf> for NormalizedPathBuf {
    fn borrow(&self) -> &PathBuf {
        &self.0
    }
}

impl Borrow<Path> for NormalizedPathBuf {
    fn borrow(&self) -> &Path {
        self.0.as_path()
    }
}

impl Deref for NormalizedPathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.as_path()
    }
}

impl NormalizedPathBuf {
    pub fn new(path: PathBuf) -> Self {
        NormalizedPathBuf(normalize_path(path.canonicalize().unwrap_or(path)))
    }

    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub fn to_path_buf(&self) -> PathBuf {
        self.0.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirKind {
    ErgModule,
    PyModule,
    Other,
    NotDir,
}

impl From<&Path> for DirKind {
    fn from(path: &Path) -> Self {
        let Ok(dir) = path.read_dir() else {
            return DirKind::NotDir;
        };
        for ent in dir {
            let Ok(ent) = ent else {
                continue;
            };
            if ent.path().file_name() == Some(OsStr::new("__init__.er")) {
                return DirKind::ErgModule;
            } else if ent.path().file_name() == Some(OsStr::new("__init__.py")) {
                return DirKind::PyModule;
            }
        }
        DirKind::Other
    }
}

impl DirKind {
    pub const fn is_erg_module(&self) -> bool {
        matches!(self, DirKind::ErgModule)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    InitEr,
    InitPy,
    Er,
    Py,
    Other,
    NotFile,
}

impl From<&Path> for FileKind {
    fn from(path: &Path) -> Self {
        if path.is_file() {
            match path.file_name() {
                Some(name) if name == OsStr::new("__init__.er") => FileKind::InitEr,
                Some(name) if name == OsStr::new("__init__.py") => FileKind::InitPy,
                Some(name) if name.to_string_lossy().ends_with(".er") => FileKind::Er,
                Some(name) if name.to_string_lossy().ends_with(".py") => FileKind::Py,
                _ => FileKind::Other,
            }
        } else {
            FileKind::NotFile
        }
    }
}

impl FileKind {
    pub const fn is_init_er(&self) -> bool {
        matches!(self, FileKind::InitEr)
    }
    pub const fn is_simple_erg_file(&self) -> bool {
        matches!(self, FileKind::Er)
    }
}

pub fn is_cur_dir<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .components()
        .next()
        .map_or(false, |c| c == Component::CurDir)
}

/// ```
/// # use std::path::{PathBuf};
/// # use erg_common::pathutil::add_postfix_foreach;
/// let path = PathBuf::from("erg");
/// let path = add_postfix_foreach(path, ".d");
/// assert_eq!(path, PathBuf::from("erg.d"));
/// let path = PathBuf::from("erg/foo/bar");
/// let path = add_postfix_foreach(path, ".d");
/// assert_eq!(path, PathBuf::from("erg.d/foo.d/bar.d"));
/// ```
pub fn add_postfix_foreach<P: AsRef<Path>, Q: AsRef<Path>>(path: P, postfix: Q) -> PathBuf {
    let mut result = PathBuf::new();
    for c in path.as_ref().components() {
        match c {
            Component::Prefix(_) => result.push(c),
            Component::RootDir => result.push(c),
            Component::CurDir => result.push(c),
            Component::ParentDir => result.push(c),
            Component::Normal(os_str) => {
                let mut os_string = os_str.to_os_string();
                os_string.push(postfix.as_ref().as_os_str());
                result.push(PathBuf::from(os_string));
            }
        }
    }
    result
}

pub fn remove_postfix_foreach<P: AsRef<Path>>(path: P, extension: &str) -> PathBuf {
    let mut result = PathBuf::new();
    for c in path.as_ref().components() {
        match c {
            Component::Prefix(_) => result.push(c),
            Component::RootDir => result.push(c),
            Component::CurDir => result.push(c),
            Component::ParentDir => result.push(c),
            Component::Normal(os_str) => {
                let string = os_str.to_string_lossy();
                result.push(string.trim_end_matches(extension));
            }
        }
    }
    result
}

/// cutout the extension from the path, and let file name be the directory name.
/// ```
/// # use std::path::{PathBuf};
/// # use erg_common::pathutil::remove_postfix;
/// let path = PathBuf::from("erg.d.er");
/// let path = remove_postfix(path, ".er");
/// assert_eq!(path, PathBuf::from("erg.d"));
/// let path = PathBuf::from("erg.d/foo.d/bar.d");
/// let path = remove_postfix(path, ".d");
/// assert_eq!(path, PathBuf::from("erg.d/foo.d/bar"));
pub fn remove_postfix<P: AsRef<Path>>(path: P, extension: &str) -> PathBuf {
    let string = path.as_ref().to_string_lossy();
    PathBuf::from(string.trim_end_matches(extension))
}

///
/// ```
/// # use std::path::{PathBuf};
/// # use erg_common::pathutil::squash;
/// let path = PathBuf::from("erg/../foo");
/// let path = squash(path);
/// assert_eq!(path, PathBuf::from("foo"));
/// let path = PathBuf::from("erg/./foo");
/// let path = squash(path);
/// assert_eq!(path, PathBuf::from("erg/foo"));
/// ```
pub fn squash(path: PathBuf) -> PathBuf {
    let mut result = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(_) => result.push(c),
            Component::RootDir => result.push(c),
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(os_str) => {
                result.push(os_str);
            }
        }
    }
    result
}

pub fn remove_verbatim(path: &Path) -> String {
    path.to_string_lossy().replace("\\\\?\\", "")
}

/// e.g.
/// ```txt
/// http.d/client.d.er -> http.client
/// $ERG_PATH/pkgs/certified/torch/1.0.0/src/lib.d.er -> torch
/// $ERG_PATH/pkgs/certified/torch/1.0.0/src/random.d.er -> torch/random
/// /users/foo/torch/src/lib.d.er -> torch
/// foo/__pycache__/__init__.d.er -> foo
/// math.d.er -> math
/// foo.py -> foo
/// ```
/// FIXME: split by `.` instead of `/`
pub fn mod_name(path: &Path) -> Str {
    let path = match path.strip_prefix(erg_pkgs_path()) {
        Ok(path) => {
            // <namespace>/<mod_root>/<version>/src/<sub>
            let mod_root = path
                .components()
                .nth(1)
                .unwrap()
                .as_os_str()
                .to_string_lossy();
            let sub = path
                .components()
                .skip(4)
                .map(|c| {
                    c.as_os_str()
                        .to_string_lossy()
                        .trim_end_matches("lib.d.er")
                        .trim_end_matches(".d.er")
                        .trim_end_matches(".d")
                        .trim_end_matches(".py")
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join("/");
            return Str::rc(format!("{mod_root}/{sub}").trim_end_matches('/'));
        }
        // using local or git path
        Err(_) if path.display().to_string().split("/src/").count() > 1 => {
            // <mod_root>/src/<sub>
            let path = path.display().to_string();
            let mod_root = path
                .split("/src/")
                .next()
                .unwrap()
                .split('/')
                .last()
                .unwrap();
            let sub = path
                .split("/src/")
                .nth(1)
                .unwrap()
                .split('/')
                .map(|c| {
                    c.trim_end_matches("lib.d.er")
                        .trim_end_matches(".d.er")
                        .trim_end_matches(".d")
                        .trim_end_matches(".py")
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join("/");
            return Str::rc(format!("{mod_root}/{sub}").trim_end_matches('/'));
        }
        Err(_) => path,
    };
    let mut name = path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .trim_end_matches(".d.er")
        .trim_end_matches(".py")
        .to_string();
    let mut parents = path.components().rev().skip(1);
    while let Some(parent) = parents.next() {
        let parent = parent.as_os_str().to_string_lossy();
        if parent == "__pycache__" {
            if name == "__init__" {
                let p = parents
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_string_lossy()
                    .trim_end_matches(".d")
                    .to_string();
                name = p;
            }
            break;
        } else if parent.ends_with(".d") {
            let p = parent.trim_end_matches(".d").to_string();
            if name == "__init__" {
                name = p;
            } else {
                name = p + "." + &name;
            }
        } else {
            break;
        }
    }
    Str::from(name)
}

pub fn project_root_dir_of(path: &Path) -> Option<PathBuf> {
    if path.is_dir() && path.join("package.er").exists() {
        return Some(path.to_path_buf());
    }
    let mut path = path.to_path_buf();
    while let Some(parent) = path.parent() {
        if parent.join("package.er").exists() {
            return Some(parent.to_path_buf());
        }
        path = parent.to_path_buf();
    }
    None
}

pub fn project_entry_file_of(path: &Path) -> Option<PathBuf> {
    let project_root = project_root_dir_of(path)?;
    if project_root.join("src/lib.er").exists() {
        Some(project_root.join("src/lib.er"))
    } else if project_root.join("src/main.er").exists() {
        Some(project_root.join("src/main.er"))
    } else if project_root.join("src/lib.d.er").exists() {
        Some(project_root.join("src/lib.d.er"))
    } else {
        None
    }
}
