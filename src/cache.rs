use std::collections::HashMap;

use crate::bitset::Bitset;
use crate::model::Model;

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
        self.map.get(&linearized.hash_val()).is_some_and(|bucket| {
            bucket
                .iter()
                .any(|(bs, s)| bs == linearized && M::equal(s, state))
        })
    }

    /// Record `(linearized, state)` as visited.
    pub fn cache_insert(&mut self, linearized: Bitset, state: M::State) {
        self.map
            .entry(linearized.hash_val())
            .or_default()
            .push((linearized, state));
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
        type Input = i32;
        type Output = i32;
        type Metadata = ();
        fn init() -> i32 {
            0
        }
        fn step(s: &i32, i: &i32, _: &i32) -> (bool, i32) {
            (true, s + i)
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
    fn two_entries_in_same_bucket() {
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
    fn empty_bitset_key() {
        let mut cache = Cache::<IntModel>::new();
        let key = Bitset::new(8); // all zeros
        cache.cache_insert(key.clone(), 0);
        assert!(cache.cache_contains(&key, &0));
        assert!(!cache.cache_contains(&key, &1));
    }
}
