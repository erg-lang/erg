use std::collections::hash_map::{Iter, Keys, Values};
use std::fmt;

use erg_common::dict::Dict;
use erg_common::set;
use erg_common::set::Set;
use erg_common::shared::Shared;

use crate::varinfo::{AbsLocation, VarInfo};

#[derive(Debug, Clone, Default)]
pub struct ModuleIndexValue {
    pub vi: VarInfo,
    pub referrers: Set<AbsLocation>,
}

impl fmt::Display for ModuleIndexValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ vi: {}, referrers: {} }}", self.vi, self.referrers)
    }
}

impl ModuleIndexValue {
    pub const fn new(vi: VarInfo, referrers: Set<AbsLocation>) -> Self {
        Self { vi, referrers }
    }

    pub fn push_ref(&mut self, referrer: AbsLocation) {
        self.referrers.insert(referrer);
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModuleIndex {
    members: Dict<AbsLocation, ModuleIndexValue>,
}

impl fmt::Display for ModuleIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.members.fmt(f)
    }
}

impl ModuleIndex {
    pub fn new() -> Self {
        Self {
            members: Dict::new(),
        }
    }

    pub fn inc_ref(&mut self, vi: &VarInfo, referrer: AbsLocation) {
        let referee = vi.def_loc.clone();
        if let Some(referrers) = self.members.get_mut(&referee) {
            referrers.push_ref(referrer);
        } else {
            let value = ModuleIndexValue::new(vi.clone(), set! {referrer});
            self.members.insert(referee, value);
        }
    }

    pub fn register(&mut self, vi: &VarInfo) {
        let referee = vi.def_loc.clone();
        let value = ModuleIndexValue::new(vi.clone(), set! {});
        self.members.insert(referee, value);
    }

    pub fn get_refs(&self, referee: &AbsLocation) -> Option<&ModuleIndexValue> {
        self.members.get(referee)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedModuleIndex(Shared<ModuleIndex>);

impl fmt::Display for SharedModuleIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.borrow().fmt(f)
    }
}

impl SharedModuleIndex {
    pub fn new() -> Self {
        Self(Shared::new(ModuleIndex::new()))
    }

    pub fn inc_ref(&self, vi: &VarInfo, referrer: AbsLocation) {
        self.0.borrow_mut().inc_ref(vi, referrer);
    }

    pub fn register(&self, vi: &VarInfo) {
        self.0.borrow_mut().register(vi);
    }

    pub fn get_refs(&self, referee: &AbsLocation) -> Option<&ModuleIndexValue> {
        unsafe { self.0.as_ptr().as_ref().unwrap().get_refs(referee) }
    }

    pub fn referees(&self) -> Keys<AbsLocation, ModuleIndexValue> {
        unsafe { self.0.as_ptr().as_ref().unwrap().members.keys() }
    }

    pub fn referrers(&self) -> Values<AbsLocation, ModuleIndexValue> {
        unsafe { self.0.as_ptr().as_ref().unwrap().members.values() }
    }

    pub fn iter(&self) -> Iter<AbsLocation, ModuleIndexValue> {
        unsafe { self.0.as_ptr().as_ref().unwrap().members.iter() }
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().members.clear();
    }
}
