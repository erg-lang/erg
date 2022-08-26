use std::cell::RefCell;
use std::thread::LocalKey;

use erg_common::dict::Dict;

use erg_type::Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypePair {
    pub sub: Type,
    pub sup: Type,
}

impl TypePair {
    pub const fn new(sub: Type, sup: Type) -> Self {
        Self { sub, sup }
    }
}

#[derive(Debug, Default)]
pub struct TypeCmpCache {
    cache: Dict<TypePair, bool>,
}

impl TypeCmpCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, pair: &TypePair) -> Option<bool> {
        self.cache.get(pair).map(|b| *b)
    }

    pub fn register(&mut self, pair: &TypePair, b: bool) {
        self.cache.insert(pair.clone(), b);
    }
}

thread_local! {
    static TYPE_CACHE: RefCell<TypeCmpCache> = RefCell::new(TypeCmpCache::default());
}

#[derive(Debug)]
pub struct GlobalTypeCmpCache(LocalKey<RefCell<TypeCmpCache>>);

pub static GLOBAL_TYPE_CACHE: GlobalTypeCmpCache = GlobalTypeCmpCache(TYPE_CACHE);

impl GlobalTypeCmpCache {
    pub fn get(&'static self, pair: &TypePair) -> Option<bool> {
        self.0.with(|s| s.borrow().get(pair))
    }

    pub fn register(&'static self, pair: &TypePair, b: bool) {
        self.0.with(|s| s.borrow_mut().register(pair, b));
    }
}
