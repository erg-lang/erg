use std::borrow::Borrow;
use std::collections::hash_set::Iter;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

use crate::fxhash::FxHashSet;
use crate::traits::Immutable;
use crate::{debug_fmt_iter, fmt_iter, get_hash};

#[cfg(feature = "pylib")]
use pyo3::prelude::PyAnyMethods;
#[cfg(feature = "pylib")]
use pyo3::{FromPyObject, IntoPy, PyAny, PyObject, Python};

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

#[cfg(feature = "pylib")]
impl<T: Hash + Eq + IntoPy<PyObject>> IntoPy<PyObject> for Set<T> {
    fn into_py(self, py: Python<'_>) -> PyObject {
        self.elems.into_py(py)
    }
}

#[cfg(feature = "pylib")]
impl<'source, T> FromPyObject<'source> for Set<T>
where
    T: Hash + Eq + FromPyObject<'source>,
{
    fn extract_bound(ob: &pyo3::Bound<'source, PyAny>) -> pyo3::PyResult<Self> {
        Ok(Set {
            elems: ob.extract::<FxHashSet<T>>()?,
        })
    }
}

impl<T: Hash + Eq + Immutable> PartialEq for Set<T> {
    fn eq(&self, other: &Set<T>) -> bool {
        self.elems.eq(&other.elems)
    }
}

impl<T: Hash + Eq + Immutable> Eq for Set<T> {}

impl<T> Default for Set<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Hash> Hash for Set<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        let sum = self
            .iter()
            .map(get_hash)
            .fold(0usize, |acc, x| acc.wrapping_add(x));
        sum.hash(state);
    }
}

impl<T: Hash + Eq> From<Vec<T>> for Set<T> {
    fn from(vec: Vec<T>) -> Self {
        vec.into_iter().collect()
    }
}

impl<T: Hash + Eq, const N: usize> From<[T; N]> for Set<T> {
    fn from(arr: [T; N]) -> Self {
        arr.into_iter().collect()
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

    pub fn get_by<Q>(&self, value: &Q, cmp: impl Fn(&Q, &Q) -> bool) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: ?Sized,
    {
        self.elems.iter().find(|&v| cmp(v.borrow(), value))
    }

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

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.iter().cloned().collect()
    }

    pub fn into_vec(self) -> Vec<T> {
        self.elems.into_iter().collect()
    }
}

impl<T: Hash> IntoIterator for Set<T> {
    type Item = T;
    type IntoIter = <FxHashSet<T> as IntoIterator>::IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.elems.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Set<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Iter<'a, T> {
        self.elems.iter()
    }
}

impl<T: Eq> Set<T> {
    pub fn linear_get<Q>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: ?Sized + Eq,
    {
        self.elems.iter().find(|x| (*x).borrow() == value)
    }

    pub fn linear_contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Eq,
    {
        self.elems.iter().any(|x| (*x).borrow() == value)
    }

    pub fn linear_eq(&self, other: &Set<T>) -> bool {
        self.len() == other.len() && self.iter().all(|x| other.linear_contains(x))
    }

    pub fn linear_remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Eq,
    {
        let mut found = false;
        self.elems.retain(|x| {
            let eq = (*x).borrow() == value;
            if eq {
                found = true;
            }
            !eq
        });
        found
    }

    pub fn linear_exclude(mut self, other: &T) -> Set<T> {
        self.linear_remove(other);
        self
    }
}

impl<T: Hash + Eq + Immutable> Set<T> {
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
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.elems.remove(value)
    }

    pub fn exclude(mut self, other: &T) -> Set<T> {
        self.remove(other);
        self
    }
}

impl<T: Hash + Eq + Clone + Immutable> Set<T> {
    /// ```
    /// # use erg_common::set;
    /// # use erg_common::set::Set;
    /// assert_eq!(Set::multi_intersection([set!{1, 3}, set!{1, 2}].into_iter()), set!{1});
    /// assert_eq!(Set::multi_intersection([set!{1, 3}, set!{1, 2}, set!{2}].into_iter()), set!{1, 2});
    /// assert_eq!(Set::multi_intersection([set!{1, 3}, set!{1, 2}, set!{2, 3}].into_iter()), set!{1, 2, 3});
    /// ```
    pub fn multi_intersection<I>(mut i: I) -> Set<T>
    where
        I: Iterator<Item = Set<T>> + Clone,
    {
        let mut res = set! {};
        while let Some(s) = i.next() {
            res = res.union_from_iter(
                s.into_iter()
                    .filter(|x| i.clone().any(|set| set.contains(x))),
            );
        }
        res
    }
}

impl<T: Hash + Eq> Set<T> {
    /// newly inserted: true, already present: false
    #[inline]
    pub fn insert(&mut self, value: T) -> bool {
        self.elems.insert(value)
    }

    #[inline]
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.elems.extend(iter);
    }

    pub fn extended<I: IntoIterator<Item = T>>(mut self, iter: I) -> Self {
        self.elems.extend(iter);
        self
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

    /// remove all elements for which the predicate returns false
    #[inline]
    pub fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        self.elems.retain(f);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.elems.clear();
    }

    #[inline]
    pub fn take_all(&mut self) -> Self {
        Self {
            elems: self.elems.drain().collect(),
        }
    }

    pub fn inplace_map<F: FnMut(T) -> T>(&mut self, f: F) {
        *self = self.take_all().into_iter().map(f).collect();
    }
}

impl<T: Hash + Eq + Clone> Set<T> {
    /// ```
    /// # use erg_common::set;
    /// assert_eq!(set!{1, 2, 3}.union(&set!{2, 3, 4}), set!{1, 2, 3, 4});
    /// ```
    #[inline]
    pub fn union(&self, other: &Set<T>) -> Set<T> {
        let u = self.elems.union(&other.elems);
        Self {
            elems: u.into_iter().cloned().collect(),
        }
    }

    pub fn union_iter<'a>(&'a self, other: &'a Set<T>) -> impl Iterator<Item = &'a T> {
        self.elems.union(&other.elems)
    }

    pub fn union_from_iter<I: Iterator<Item = T>>(&self, iter: I) -> Set<T> {
        self.union(&iter.collect())
    }

    /// ```
    /// # use erg_common::set;
    /// assert_eq!(set!{1, 2, 3}.intersection(&set!{2, 3, 4}), set!{2, 3});
    /// ```
    #[inline]
    pub fn intersection(&self, other: &Set<T>) -> Set<T> {
        let u = self.elems.intersection(&other.elems);
        Self {
            elems: u.into_iter().cloned().collect(),
        }
    }

    pub fn intersec_iter<'a>(&'a self, other: &'a Set<T>) -> impl Iterator<Item = &'a T> {
        self.elems.intersection(&other.elems)
    }

    pub fn intersec_from_iter<I: Iterator<Item = T>>(&self, iter: I) -> Set<T> {
        self.intersection(&iter.collect())
    }

    pub fn difference(&self, other: &Set<T>) -> Set<T> {
        let u = self.elems.difference(&other.elems);
        Self {
            elems: u.into_iter().cloned().collect(),
        }
    }

    pub fn diff_iter<'a>(&'a self, other: &'a Set<T>) -> impl Iterator<Item = &'a T> {
        self.elems.difference(&other.elems)
    }

    pub fn include(mut self, other: T) -> Set<T> {
        self.insert(other);
        self
    }

    /// ```
    /// # use erg_common::set;
    /// assert_eq!(set!{1, 2}.product(&set!{3, 4}), set!{(&1, &3), (&1, &4), (&2, &3), (&2, &4)});
    /// ```
    pub fn product<'l, 'r, U: Hash + Eq>(&'l self, other: &'r Set<U>) -> Set<(&'l T, &'r U)> {
        let mut res = set! {};
        for x in self.iter() {
            for y in other.iter() {
                res.insert((x, y));
            }
        }
        res
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

impl<T: Hash + fmt::Display> Set<T> {
    pub fn folded_display(&self) -> String {
        self.iter()
            .fold("".to_string(), |acc, x| acc + &x.to_string() + "\n")
    }
}
