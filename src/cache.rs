use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::bitset::Bitset;
use crate::model::Model;

// ---------------------------------------------------------------------------
// Combined hash key
// ---------------------------------------------------------------------------
/// Mix the FNV-1a hash of `linearized` with the SipHash of `state` into a
/// single u64 key.
///
/// Using both hashes makes it overwhelmingly unlikely that two distinct
/// `(linearized, state)` pairs share the same bucket.
fn cache_key<M: Model>(linearized: &Bitset, state: &M::State) -> u64 {
    let bitset_hash = linearized.hash_val();
    // Hash the state with the standard library's SipHash (good avalanche).
    let state_hash = {
        use std::collections::hash_map::DefaultHasher;
        let mut h = DefaultHasher::new();
        state.hash(&mut h);
        h.finish()
    };
    // Fibonacci/golden-ratio mix so that small differences in either input
    // produce very different combined keys.
    bitset_hash
        ^ state_hash
            .wrapping_add(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(bitset_hash << 6)
            .wrapping_add(bitset_hash >> 2)
}
// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------
pub struct Cache<M: Model> {
    map: HashMap<u64, Vec<(Bitset, M::State)>>,
}

impl<M: Model> Cache<M> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Returns `true` if `(linearized, state)` has already been visited.
    pub fn cache_contains(&self, linearized: &Bitset, state: &M::State) -> bool {
        let key = cache_key::<M>(linearized, state);
        self.map.get(&key).is_some_and(|bucket| {
            bucket
                .iter()
                .any(|(bs, s)| bs == linearized && M::equal(s, state))
        })
    }

    /// Record `(linearized, state)` as visited.
    pub fn cache_insert(&mut self, linearized: Bitset, state: M::State) {
        let key = cache_key::<M>(&linearized, &state);
        self.map.entry(key).or_default().push((linearized, state));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Model;

    #[derive(Clone)]
    struct IntModel;

    impl Model for IntModel {
        type State = i32;
        type Op = i32;
        type Metadata = ();

        fn init() -> i32 {
            0
        }
        fn step(s: &i32, op: &i32) -> (bool, i32) {
            (true, s + op)
        }
    }

    fn bs(n: usize, set_bits: &[usize]) -> Bitset {
        let mut b = Bitset::new(n);
        for &i in set_bits {
            b.set(i);
        }
        b
    }

    #[test]
    fn miss_on_empty_cache() {
        let cache = Cache::<IntModel>::new();
        assert!(!cache.cache_contains(&bs(4, &[0, 2]), &42));
    }

    #[test]
    fn hit_after_insert() {
        let mut cache = Cache::<IntModel>::new();
        let key = bs(4, &[1, 3]);
        cache.cache_insert(key.clone(), 7);
        assert!(cache.cache_contains(&key, &7));
    }

    #[test]
    fn miss_wrong_state() {
        let mut cache = Cache::<IntModel>::new();
        let key = bs(4, &[0]);
        cache.cache_insert(key.clone(), 99);
        assert!(!cache.cache_contains(&key, &100));
    }

    #[test]
    fn miss_different_bitset_same_state() {
        let mut cache = Cache::<IntModel>::new();
        let key0 = bs(4, &[0]);
        let key1 = bs(4, &[1]);
        cache.cache_insert(key0.clone(), 5);
        assert!(!cache.cache_contains(&key1, &5));
    }

    #[test]
    fn two_entries_both_found() {
        let mut cache = Cache::<IntModel>::new();
        let key0 = bs(4, &[0]);
        let key1 = bs(4, &[1]);
        cache.cache_insert(key0.clone(), 10);
        cache.cache_insert(key1.clone(), 20);
        assert!(cache.cache_contains(&key0, &10));
        assert!(cache.cache_contains(&key1, &20));
        assert!(!cache.cache_contains(&key0, &20));
        assert!(!cache.cache_contains(&key1, &10));
    }

    #[test]
    fn empty_bitset_zero_state() {
        let mut cache = Cache::<IntModel>::new();
        let key = Bitset::new(8);
        cache.cache_insert(key.clone(), 0);
        assert!(cache.cache_contains(&key, &0));
        assert!(!cache.cache_contains(&key, &1));
    }
    #[test]
    fn same_bitset_different_states_both_stored() {
        // Two entries sharing the same bitset but different states must both be
        // findable — this exercises the bucket collision path.
        let mut cache = Cache::<IntModel>::new();
        let key = bs(4, &[2]);
        cache.cache_insert(key.clone(), 100);
        cache.cache_insert(key.clone(), 200);
        assert!(cache.cache_contains(&key, &100));
        assert!(cache.cache_contains(&key, &200));
        assert!(!cache.cache_contains(&key, &300));
    }
}
