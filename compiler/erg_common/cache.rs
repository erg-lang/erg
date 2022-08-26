use std::borrow::{Borrow, ToOwned};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use crate::set::Set;
use crate::{RcArray, Str};

#[derive(Debug)]
pub struct CacheSet<T: ?Sized>(RefCell<Set<Rc<T>>>);

impl<T: ?Sized> Default for CacheSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> CacheSet<T> {
    pub fn new() -> Self {
        Self(RefCell::new(Set::new()))
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
    pub fn get(&self, q: &[T]) -> Rc<[T]> {
        if let Some(cached) = self.0.borrow().get(q) {
            return cached.clone();
        } // &self.0 is dropped
        let s = RcArray::from(q);
        self.0.borrow_mut().insert(s.clone());
        s
    }
}

impl<T: Hash + Eq> CacheSet<T> {
    pub fn get<Q: ?Sized + Hash + Eq>(&self, q: &Q) -> Rc<T>
    where
        Rc<T>: Borrow<Q>,
        Q: ToOwned<Owned = T>,
    {
        if let Some(cached) = self.0.borrow().get(q) {
            return cached.clone();
        } // &self.0 is dropped
        let s = Rc::from(q.to_owned());
        self.0.borrow_mut().insert(s.clone());
        s
    }
}

pub struct CacheDict<T: ?Sized>(RefCell<Set<Rc<T>>>);
