use std::borrow::{Borrow, ToOwned};
use std::hash::Hash;
use std::sync::Arc;

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
    pub fn get<Q>(&self, q: &Q) -> Arc<T>
    where
        Arc<T>: Borrow<Q>,
        Q: ?Sized + Hash + Eq + ToOwned<Owned = T>,
    {
        if let Some(cached) = self.0.borrow().get(q) {
            return cached.clone();
        } // &self.0 is dropped
        let s = Arc::from(q.to_owned());
        self.0.borrow_mut().insert(s.clone());
        s
    }
}

#[derive(Debug, Clone)]
pub struct CacheDict<K, V: ?Sized>(Shared<Dict<K, Arc<V>>>);

impl<K: Hash + Eq, V: ?Sized> Default for CacheDict<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq, V: ?Sized> CacheDict<K, V> {
    pub fn new() -> Self {
        Self(Shared::new(Dict::new()))
    }
}

impl<K: Hash + Eq, V> CacheDict<K, V> {
    pub fn get<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> Option<Arc<V>>
    where
        K: Borrow<Q>,
    {
        self.0.borrow().get(k).cloned()
    }

    pub fn insert(&self, k: K, v: V) {
        self.0.borrow_mut().insert(k, Arc::new(v));
    }
}
