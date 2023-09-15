use std::path::Path;
use std::sync::OnceLock;

use crate::dict::Dict;
use crate::pathutil::NormalizedPathBuf;
use crate::shared::Shared;

/// In-memory virtual file system.
/// This is for cases where the contents of the code and the real file system do not link (e.g. ELS).
#[derive(Debug, Default)]
pub struct VirtualFileSystem {
    cache: Shared<Dict<NormalizedPathBuf, String>>,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        Self {
            cache: Shared::new(Dict::new()),
        }
    }

    pub fn update(&self, path: impl AsRef<Path>, contents: String) {
        let path = NormalizedPathBuf::from(path.as_ref());
        self.cache.borrow_mut().insert(path, contents);
    }

    pub fn remove(&self, path: impl AsRef<Path>) {
        let path = NormalizedPathBuf::from(path.as_ref());
        self.cache.borrow_mut().remove(&path);
    }

    pub fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) {
        let from = NormalizedPathBuf::from(from.as_ref());
        let to = NormalizedPathBuf::from(to.as_ref());
        let contents = self.cache.borrow_mut().remove(&from);
        self.cache.borrow_mut().insert(to, contents.unwrap());
    }

    pub fn read(&self, path: impl AsRef<Path>) -> std::io::Result<String> {
        let path = NormalizedPathBuf::from(path.as_ref());
        if let Some(cache) = self.cache.borrow().get(&path) {
            return Ok(cache.clone());
        }
        let contents = std::fs::read_to_string(&path)?;
        self.cache
            .borrow_mut()
            .insert(path.clone(), contents.clone());
        Ok(contents)
    }
}

pub struct SharedVFS(OnceLock<VirtualFileSystem>);

impl SharedVFS {
    pub fn read(&self, path: impl AsRef<Path>) -> std::io::Result<String> {
        self.0.get_or_init(VirtualFileSystem::new).read(path)
    }

    pub fn update(&self, path: impl AsRef<Path>, contents: String) {
        self.0
            .get_or_init(VirtualFileSystem::new)
            .update(path, contents)
    }

    pub fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) {
        self.0.get_or_init(VirtualFileSystem::new).rename(from, to)
    }

    pub fn remove(&self, path: impl AsRef<Path>) {
        self.0.get_or_init(VirtualFileSystem::new).remove(path)
    }
}

pub static VFS: SharedVFS = SharedVFS(OnceLock::new());
