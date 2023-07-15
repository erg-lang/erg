use std::borrow::Borrow;
use std::ffi::OsStr;
use std::ops::Deref;
use std::path::{Component, Path, PathBuf};

use crate::normalize_path;

/// Guaranteed equivalence path.
///
/// `PathBuf` may give false equivalence decisions in non-case-sensitive file systems.
/// Use this for dictionary keys, etc.
/// See also: `els::util::NormalizedUrl`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct NormalizedPathBuf(PathBuf);

impl<P: Into<PathBuf>> From<P> for NormalizedPathBuf {
    fn from(path: P) -> Self {
        NormalizedPathBuf::new(path.into())
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
