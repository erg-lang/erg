use std::borrow::{Borrow, ToOwned};
use std::hash::Hash;
use std::sync::Arc;
use std::thread::LocalKey;

use crate::dict::Dict;
use crate::set::Set;
use crate::shared::Shared;
use crate::{ArcArray, Str};

#[derive(Debug)]
pub struct CacheSet<T: ?Sized>(Shared<Set<Arc<T>>>);

impl<T: ?Sized> Default for CacheSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> CacheSet<T> {
    pub fn new() -> Self {
        Self(Shared::new(Set::new()))
    }
}

impl Clone for CacheSet<str> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Hash + Eq> Clone for CacheSet<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CacheSet<str> {
    pub fn get(&self, s: &str) -> Str {
        if let Some(cached) = self.0.borrow().get(s) {
            return cached.clone().into();
        } // &self.0 is dropped
        let s = Str::rc(s);
        self.0.borrow_mut().insert(s.clone().into_rc());
        s
    }
}

impl<T: Hash + Eq + Clone> CacheSet<[T]> {
    pub fn get(&self, q: &[T]) -> Arc<[T]> {
        if let Some(cached) = self.0.borrow().get(q) {
            return cached.clone();
        } // &self.0 is dropped
        let s = ArcArray::from(q);
        self.0.borrow_mut().insert(s.clone());
        s
    }
}

impl<T: Hash + Eq> CacheSet<T> {
    pub fn get<Q: ?Sized + Hash + Eq>(&self, q: &Q) -> Arc<T>
    where
        Arc<T>: Borrow<Q>,
        Q: ToOwned<Owned = T>,
    {
        if let Some(cached) = self.0.borrow().get(q) {
            return cached.clone();
        } // &self.0 is dropped
        let s = Arc::from(q.to_owned());
        self.0.borrow_mut().insert(s.clone());
        s
    }
}

pub struct CacheDict<K, V: ?Sized>(Shared<Dict<K, Arc<V>>>);

pub struct GlobalCacheDict<K: 'static, V: ?Sized + 'static>(LocalKey<Shared<CacheDict<K, V>>>);
