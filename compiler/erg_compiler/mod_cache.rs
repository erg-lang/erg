use std::borrow::Borrow;
use std::hash::Hash;
use std::rc::Rc;

use erg_common::dict::Dict;
use erg_common::shared::Shared;

use erg_parser::ast::VarName;

use crate::context::Context;
use crate::hir::HIR;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModId(usize);

impl ModId {
    pub const fn new(id: usize) -> Self {
        Self(id)
    }
    pub const fn builtin() -> Self {
        Self(0)
    }
    pub const fn main() -> Self {
        Self(1)
    }
}

#[derive(Debug)]
pub struct ModuleEntry {
    id: ModId, // builtin == 0, __main__ == 1
    hir: Option<HIR>,
    ctx: Rc<Context>,
}

impl ModuleEntry {
    pub fn new(id: ModId, hir: Option<HIR>, ctx: Context) -> Self {
        Self {
            id,
            hir,
            ctx: Rc::new(ctx),
        }
    }

    pub fn builtin(ctx: Context) -> Self {
        Self {
            id: ModId::builtin(),
            hir: None,
            ctx: Rc::new(ctx),
        }
    }
}

#[derive(Debug, Default)]
pub struct ModuleCache {
    cache: Dict<VarName, ModuleEntry>,
}

impl ModuleCache {
    pub fn new() -> Self {
        Self { cache: Dict::new() }
    }

    pub fn get<Q: Eq + Hash + ?Sized>(&self, name: &Q) -> Option<&ModuleEntry>
    where
        VarName: Borrow<Q>,
    {
        self.cache.get(name)
    }

    pub fn register(&mut self, name: VarName, entry: ModuleEntry) {
        self.cache.insert(name, entry);
    }

    pub fn remove<Q: Eq + Hash + ?Sized>(&mut self, name: &Q) -> Option<ModuleEntry>
    where
        VarName: Borrow<Q>,
    {
        self.cache.remove(name)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedModuleCache(Shared<ModuleCache>);

impl SharedModuleCache {
    pub fn new() -> Self {
        Self(Shared::new(ModuleCache::new()))
    }

    pub fn get_ctx<Q: Eq + Hash + ?Sized>(&self, name: &Q) -> Option<Rc<Context>>
    where
        VarName: Borrow<Q>,
    {
        self.0.borrow().get(name).map(|entry| entry.ctx.clone())
    }

    pub fn ref_ctx<Q: Eq + Hash + ?Sized>(&self, name: &Q) -> Option<&Context>
    where
        VarName: Borrow<Q>,
    {
        let ref_ = unsafe { self.0.as_ptr().as_ref().unwrap() };
        ref_.get(name).map(|entry| entry.ctx.as_ref())
    }

    pub fn register(&self, name: VarName, entry: ModuleEntry) {
        self.0.borrow_mut().register(name, entry);
    }

    pub fn remove<Q: Eq + Hash + ?Sized>(&mut self, name: &Q) -> Option<ModuleEntry>
    where
        VarName: Borrow<Q>,
    {
        self.0.borrow_mut().remove(name)
    }
}
