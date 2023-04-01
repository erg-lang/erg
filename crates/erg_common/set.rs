use std::borrow::Borrow;
use std::collections::hash_set::Iter;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

use crate::fxhash::FxHashSet;
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

// Use `fast_eq` for faster comparisons
impl<T: Hash + Eq> PartialEq for Set<T> {
    fn eq(&self, other: &Set<T>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .all(|l_key| other.iter().any(|r_key| l_key == r_key))
    }
}

impl<T: Hash + Eq> Eq for Set<T> {}

impl<T> Default for Set<T> {
    fn default() -> Self {
        Self::new()
    }
}

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
}

impl<T: Hash> IntoIterator for Set<T> {
    type Item = T;
    type IntoIter = <FxHashSet<T> as IntoIterator>::IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
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

    pub fn get_by<Q>(&self, value: &Q, cmp: impl Fn(&Q, &Q) -> bool) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.elems.iter().find(|&v| cmp(v.borrow(), value))
    }

    #[inline]
    pub fn fast_eq(&self, other: &Set<T>) -> bool {
        self.elems == other.elems
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
    pub fn insert(&mut self, value: T) -> bool {
        self.elems.insert(value)
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

    pub fn difference(&self, other: &Set<T>) -> Set<T> {
        let u = self.elems.difference(&other.elems);
        Self {
            elems: u.into_iter().cloned().collect(),
        }
    }

    pub fn diff_iter<'a>(&'a self, other: &'a Set<T>) -> impl Iterator<Item = &'a T> {
        self.elems.difference(&other.elems)
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
