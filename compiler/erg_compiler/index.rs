use std::collections::hash_map::{Keys, Values};

use erg_common::dict::Dict;
use erg_common::shared::Shared;

use crate::varinfo::AbsLocation;

#[derive(Debug, Clone, Default)]
pub struct ModuleIndex {
    attrs: Dict<AbsLocation, Vec<AbsLocation>>,
}

impl ModuleIndex {
    pub fn new() -> Self {
        Self { attrs: Dict::new() }
    }

    pub fn add_ref(&mut self, referee: AbsLocation, referrer: AbsLocation) {
        if let Some(referrers) = self.attrs.get_mut(&referee) {
            referrers.push(referrer);
        } else {
            self.attrs.insert(referee, vec![referrer]);
        }
    }

    pub fn get_refs(&self, referee: &AbsLocation) -> Option<&Vec<AbsLocation>> {
        self.attrs.get(referee)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedModuleIndex(Shared<ModuleIndex>);

impl SharedModuleIndex {
    pub fn new() -> Self {
        Self(Shared::new(ModuleIndex::new()))
    }

    pub fn add_ref(&self, referee: AbsLocation, referrer: AbsLocation) {
        self.0.borrow_mut().add_ref(referee, referrer);
    }

    pub fn get_refs(&self, referee: &AbsLocation) -> Option<&Vec<AbsLocation>> {
        unsafe { self.0.as_ptr().as_ref().unwrap().get_refs(referee) }
    }

    pub fn keys(&self) -> Keys<AbsLocation, Vec<AbsLocation>> {
        unsafe { self.0.as_ptr().as_ref().unwrap().attrs.keys() }
    }

    pub fn values(&self) -> Values<AbsLocation, Vec<AbsLocation>> {
        unsafe { self.0.as_ptr().as_ref().unwrap().attrs.values() }
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().attrs.clear();
    }
}
