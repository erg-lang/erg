use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

pub fn random() -> u64 {
    let state = RandomState::new();
    state.build_hasher().finish()
}
