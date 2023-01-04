use erg_common::dict::Dict;
use erg_common::error::Location;

#[derive(Debug, Clone, Default)]
pub struct ModuleIndex {
    attrs: Dict<Location, Vec<Location>>,
}

impl ModuleIndex {
    pub fn new() -> Self {
        Self { attrs: Dict::new() }
    }

    pub fn add_ref(&mut self, referee: Location, referrer: Location) {
        if let Some(referrers) = self.attrs.get_mut(&referee) {
            referrers.push(referrer);
        } else {
            self.attrs.insert(referee, vec![referrer]);
        }
    }
}
