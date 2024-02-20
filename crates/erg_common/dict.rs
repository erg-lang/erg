use std::borrow::Borrow;
use std::collections::hash_map::{
    Entry, IntoKeys, IntoValues, Iter, IterMut, Keys, Values, ValuesMut,
};
use std::fmt::{self, Write};
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};

use crate::fxhash::FxHashMap;

#[macro_export]
macro_rules! dict {
    () => { $crate::dict::Dict::new() };
    ($($k: expr => $v: expr),+ $(,)?) => {{
        let mut dict = $crate::dict::Dict::new();
        $(dict.insert($k, $v);)+
        dict
    }};
}

#[derive(Debug, Clone)]
pub struct Dict<K, V> {
    dict: FxHashMap<K, V>,
}

impl<K: Hash + Eq, V: Hash + Eq> PartialEq for Dict<K, V> {
    fn eq(&self, other: &Dict<K, V>) -> bool {
        self.dict == other.dict
    }
}

impl<K: Hash + Eq, V: Hash + Eq> Eq for Dict<K, V> {}

impl<K: Hash, V: Hash> Hash for Dict<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let keys = self.dict.keys().collect::<Vec<_>>();
        keys.hash(state);
        let vals = self.dict.values().collect::<Vec<_>>();
        vals.hash(state);
    }
}

impl<K: fmt::Display, V: fmt::Display> fmt::Display for Dict<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = "".to_string();
        for (k, v) in self.dict.iter() {
            write!(s, "{k}: {v}, ")?;
        }
        s.pop();
        s.pop();
        write!(f, "{{{s}}}")
    }
}

impl<K: Hash + Eq, V> FromIterator<(K, V)> for Dict<K, V> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Dict<K, V> {
        let mut dict = Dict::new();
        dict.extend(iter);
        dict
    }
}

impl<K: Hash + Eq, V> Extend<(K, V)> for Dict<K, V> {
    #[inline]
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.guaranteed_extend(iter);
    }
}

impl<K: Hash + Eq, V> From<Vec<(K, V)>> for Dict<K, V> {
    #[inline]
    fn from(v: Vec<(K, V)>) -> Dict<K, V> {
        v.into_iter().collect()
    }
}

impl<K: Hash + Eq, V, Q: ?Sized> Index<&Q> for Dict<K, V>
where
    K: Borrow<Q>,
    Q: Hash + Eq,
{
    type Output = V;
    #[inline]
    fn index(&self, index: &Q) -> &V {
        self.dict.get(index).unwrap()
    }
}

impl<K: Hash + Eq, V, Q: ?Sized> IndexMut<&Q> for Dict<K, V>
where
    K: Borrow<Q>,
    Q: Hash + Eq,
{
    #[inline]
    fn index_mut(&mut self, index: &Q) -> &mut V {
        self.dict.get_mut(index).unwrap()
    }
}

impl<K, V> Default for Dict<K, V> {
    fn default() -> Dict<K, V> {
        Dict::new()
    }
}

impl<K, V> Dict<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            dict: FxHashMap::default(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            dict: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.dict.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dict.is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.dict.capacity()
    }

    #[inline]
    pub fn keys(&self) -> Keys<K, V> {
        self.dict.keys()
    }

    #[inline]
    pub fn values(&self) -> Values<K, V> {
        self.dict.values()
    }

    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<K, V> {
        self.dict.values_mut()
    }

    #[inline]
    pub fn into_values(self) -> IntoValues<K, V> {
        self.dict.into_values()
    }

    #[inline]
    pub fn into_keys(self) -> IntoKeys<K, V> {
        self.dict.into_keys()
    }

    #[inline]
    pub fn iter(&self) -> Iter<K, V> {
        self.dict.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        self.dict.iter_mut()
    }

    pub fn clear(&mut self) {
        self.dict.clear();
    }

    /// remove all elements for which the predicate returns false
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.dict.retain(f);
    }
}

impl<K, V> IntoIterator for Dict<K, V> {
    type Item = (K, V);
    type IntoIter = <FxHashMap<K, V> as IntoIterator>::IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.dict.into_iter()
    }
}

impl<K: Hash + Eq, V> Dict<K, V> {
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.dict.get(k)
    }

    pub fn get_by(&self, k: &K, cmp: impl Fn(&K, &K) -> bool) -> Option<&V> {
        for (k_, v) in self.dict.iter() {
            if cmp(k, k_) {
                return Some(v);
            }
        }
        None
    }

    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.dict.get_mut(k)
    }

    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.dict.get_key_value(k)
    }

    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.dict.contains_key(k)
    }

    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.dict.insert(k, v)
    }

    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.dict.remove(k)
    }

    pub fn remove_entry<Q: ?Sized>(&mut self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.dict.remove_entry(k)
    }

    /// NOTE: This method does not consider pairing with values and keys. That is, a value may be paired with a different key (can be considered equal).
    /// If you need to consider the pairing of the keys and values, use `guaranteed_extend` instead.
    #[inline]
    pub fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.dict.extend(iter);
    }

    #[inline]
    pub fn guaranteed_extend<I: IntoIterator<Item = (K, V)>>(&mut self, other: I) {
        for (k, v) in other {
            self.dict.entry(k).or_insert(v);
        }
    }

    #[inline]
    pub fn merge(&mut self, other: Self) {
        self.dict.extend(other.dict);
    }

    #[inline]
    pub fn concat(mut self, other: Self) -> Self {
        self.merge(other);
        self
    }

    #[inline]
    pub fn diff(mut self, other: &Self) -> Self {
        for k in other.dict.keys() {
            self.dict.remove(k);
        }
        self
    }

    pub fn entry(&mut self, k: K) -> Entry<K, V> {
        self.dict.entry(k)
    }
}
