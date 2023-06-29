use std::collections::hash_map::{Iter, Keys, Values};
use std::fmt;
use std::path::Path;

use erg_common::dict::Dict;
use erg_common::set;
use erg_common::set::Set;
use erg_common::shared::{MappedRwLockReadGuard, RwLockReadGuard, Shared};
use erg_common::Str;

use crate::varinfo::{AbsLocation, VarInfo};

pub struct Members<'a>(MappedRwLockReadGuard<'a, Dict<AbsLocation, ModuleIndexValue>>);

impl<'a> Members<'a> {
    pub fn iter(&self) -> Iter<AbsLocation, ModuleIndexValue> {
        self.0.iter()
    }

    pub fn keys(&self) -> Keys<AbsLocation, ModuleIndexValue> {
        self.0.keys()
    }

    pub fn values(&self) -> Values<AbsLocation, ModuleIndexValue> {
        self.0.values()
    }
}

#[derive(Debug, Clone)]
pub struct ModuleIndexValue {
    pub name: Str,
    pub vi: VarInfo,
    pub referrers: Set<AbsLocation>,
}

impl fmt::Display for ModuleIndexValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ name: {}, vi: {}, referrers: {} }}",
            self.name, self.vi, self.referrers
        )
    }
}

impl ModuleIndexValue {
    pub const fn new(name: Str, vi: VarInfo, referrers: Set<AbsLocation>) -> Self {
        Self {
            name,
            vi,
            referrers,
        }
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

    pub fn inc_ref(&mut self, name: &Str, vi: &VarInfo, referrer: AbsLocation) {
        let referee = vi.def_loc.clone();
        if let Some(referrers) = self.members.get_mut(&referee) {
            referrers.push_ref(referrer);
        } else {
            let value = ModuleIndexValue::new(name.clone(), vi.clone(), set! {referrer});
            self.members.insert(referee, value);
        }
    }

    pub fn register(&mut self, name: Str, vi: &VarInfo) {
        let referee = vi.def_loc.clone();
        let value = ModuleIndexValue::new(name, vi.clone(), set! {});
        self.members.insert(referee, value);
    }

    pub fn get_refs(&self, referee: &AbsLocation) -> Option<&ModuleIndexValue> {
        self.members.get(referee)
    }

    pub fn initialize(&mut self) {
        self.members.clear();
    }

    pub fn remove_path(&mut self, path: &Path) {
        self.members.retain(|loc, value| {
            value
                .referrers
                .retain(|ref_loc| ref_loc.module.as_deref() != Some(path));
            loc.module.as_deref() != Some(path)
        });
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

    pub fn inc_ref(&self, name: &Str, vi: &VarInfo, referrer: AbsLocation) {
        self.0.borrow_mut().inc_ref(name, vi, referrer);
    }

    pub fn register(&self, name: Str, vi: &VarInfo) {
        self.0.borrow_mut().register(name, vi);
    }

    pub fn get_refs(
        &self,
        referee: &AbsLocation,
    ) -> Option<MappedRwLockReadGuard<ModuleIndexValue>> {
        if self.0.borrow().get_refs(referee).is_some() {
            Some(RwLockReadGuard::map(self.0.borrow(), |index| {
                index.get_refs(referee).unwrap()
            }))
        } else {
            None
        }
    }

    pub fn members(&self) -> Members {
        Members(RwLockReadGuard::map(self.0.borrow(), |mi| &mi.members))
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().initialize();
    }

    pub fn remove_path(&self, path: &Path) {
        self.0.borrow_mut().remove_path(path);
    }
}
