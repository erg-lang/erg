use std::borrow::Borrow;
use std::collections::hash_set::{IntoIter, Iter};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

use crate::fxhash::FxHashSet;
use crate::ty::Type;
use crate::value::ValueObj;
use crate::{debug_fmt_iter, fmt_iter};

#[macro_export]
macro_rules! set {
    () => { $crate::set::Set::new() };
    ($($x: expr),+ $(,)?) => {{
        let mut set = $crate::set::Set::new();
        $(set.insert($x);)+
        set
    }};
}

#[derive(Clone)]
pub struct Set<T> {
    elems: FxHashSet<T>,
}

impl<T: Hash + Eq> PartialEq for Set<T> {
    fn eq(&self, other: &Set<T>) -> bool {
        self.len() == other.len() && self.iter().all(|key| other.contains(key))
    }
}

impl<T> Default for Set<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Hash + Eq> Eq for Set<T> {}

impl<T: Hash> Hash for Set<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.elems.iter().collect::<Vec<_>>().hash(state);
    }
}

impl<T: Hash + Eq> From<Vec<T>> for Set<T> {
    fn from(vec: Vec<T>) -> Self {
        vec.into_iter().collect()
    }
}

impl<T: fmt::Debug> fmt::Debug for Set<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{}}}", debug_fmt_iter(self.elems.iter()))
    }
}

impl<T: fmt::Display> fmt::Display for Set<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{}}}", fmt_iter(self.elems.iter()))
    }
}

impl<T: Hash + Eq> FromIterator<T> for Set<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Set<T> {
        let mut set = Set::new();
        set.extend(iter);
        set
    }
}

impl<T> Set<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            elems: FxHashSet::default(),
        }
    }
}

impl<T: Hash> Set<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elems: FxHashSet::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.elems.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.elems.iter()
    }

    #[inline]
    pub fn into_iter(self) -> IntoIter<T> {
        self.elems.into_iter()
    }
}

impl<T: Hash + Eq> Set<T> {
    #[inline]
    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.elems.get(value)
    }

    #[inline]
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.elems.contains(value)
    }

    #[inline]
    pub fn insert(&mut self, value: T) {
        self.elems.insert(value);
    }

    #[inline]
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.elems.remove(value)
    }

    #[inline]
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.elems.extend(iter);
    }

    #[inline]
    pub fn is_superset(&self, other: &Set<T>) -> bool {
        self.elems.is_superset(&other.elems)
    }

    #[inline]
    pub fn merge(&mut self, other: Self) {
        self.elems.extend(other.elems);
    }

    #[inline]
    pub fn concat(mut self, other: Self) -> Self {
        self.elems.extend(other.elems);
        self
    }
}

impl<T: Hash + Eq + Clone> Set<T> {
    #[inline]
    pub fn union(&self, other: &Set<T>) -> Set<T> {
        let u = self.elems.union(&other.elems);
        Self {
            elems: u.into_iter().cloned().collect(),
        }
    }

    #[inline]
    pub fn intersection(&self, other: &Set<T>) -> Set<T> {
        let u = self.elems.intersection(&other.elems);
        Self {
            elems: u.into_iter().cloned().collect(),
        }
    }
}

impl<T: Hash + Ord> Set<T> {
    pub fn max(&self) -> Option<&T> {
        self.iter().max_by(|x, y| x.cmp(y))
    }

    pub fn min(&self) -> Option<&T> {
        self.iter().min_by(|x, y| x.cmp(y))
    }
}

impl Set<ValueObj> {
    // false -> SyntaxError
    pub fn is_homogeneous(&self) -> bool {
        let l_first = self.iter().next().unwrap().class();
        self.iter().all(|c| c.class() == l_first)
    }

    pub fn inner_class(&self) -> Type {
        self.iter().next().unwrap().class()
    }

    pub fn max(&self) -> Option<ValueObj> {
        if !self.is_homogeneous() {
            return None;
        }
        self.iter()
            .max_by(|x, y| x.try_cmp(y).unwrap())
            .map(Clone::clone)
    }

    pub fn min(&self) -> Option<ValueObj> {
        if !self.is_homogeneous() {
            return None;
        }
        self.iter()
            .min_by(|x, y| x.try_cmp(y).unwrap())
            .map(Clone::clone)
    }
}
