use std::collections::hash_map::{Keys, Values};
use std::path::PathBuf;

use erg_common::dict::Dict;
use erg_common::set;
use erg_common::set::Set;
use erg_common::shared::Shared;
use erg_common::tsort::{tsort, Graph, Node};

use crate::varinfo::AbsLocation;

#[derive(Debug, Clone, Default)]
pub struct ModuleIndex {
    attrs: Dict<AbsLocation, Set<AbsLocation>>,
}

impl ModuleIndex {
    pub fn new() -> Self {
        Self { attrs: Dict::new() }
    }

    pub fn add_ref(&mut self, referee: AbsLocation, referrer: AbsLocation) {
        if let Some(referrers) = self.attrs.get_mut(&referee) {
            referrers.insert(referrer);
        } else {
            self.attrs.insert(referee, set! {referrer});
        }
    }

    pub fn get_refs(&self, referee: &AbsLocation) -> Option<&Set<AbsLocation>> {
        self.attrs.get(referee)
    }

    pub fn unsorted_module_graph(&self) -> Graph<PathBuf, ()> {
        let mut graph = Graph::new();
        for (referee, referrers) in self.attrs.iter() {
            if let Some(referee_path) = &referee.module {
                for referrer in referrers.iter() {
                    if let Some(referrer_path) = &referrer.module {
                        if let Some(referrer) = graph.iter_mut().find(|n| &n.id == referrer_path) {
                            if referee_path == referrer_path {
                                continue;
                            }
                            referrer.push_dep(referee_path.clone());
                        } else {
                            let depends_on = if referee_path == referrer_path {
                                set! {}
                            } else {
                                set! {referee_path.clone()}
                            };
                            let node = Node::new(referrer_path.clone(), (), depends_on);
                            graph.push(node);
                        }
                    }
                }
            }
        }
        graph
    }

    pub fn module_graph(&self) -> Graph<PathBuf, ()> {
        tsort(self.unsorted_module_graph()).unwrap()
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

    pub fn get_refs(&self, referee: &AbsLocation) -> Option<&Set<AbsLocation>> {
        unsafe { self.0.as_ptr().as_ref().unwrap().get_refs(referee) }
    }

    pub fn referees(&self) -> Keys<AbsLocation, Set<AbsLocation>> {
        unsafe { self.0.as_ptr().as_ref().unwrap().attrs.keys() }
    }

    pub fn referrers(&self) -> Values<AbsLocation, Set<AbsLocation>> {
        unsafe { self.0.as_ptr().as_ref().unwrap().attrs.values() }
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().attrs.clear();
    }

    pub fn unsorted_module_graph(&self) -> Graph<PathBuf, ()> {
        self.0.borrow().unsorted_module_graph()
    }

    pub fn module_graph(&self) -> Graph<PathBuf, ()> {
        self.0.borrow().module_graph()
    }
}
