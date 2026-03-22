/// A dense, fixed-size bitset backed by a `Vec<u64>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitset {
    words: Vec<u64>,
    /// Number of valid bits
    n: usize,
}

impl Bitset {
    /// Create a bitset of `n` bits, all cleared.
    pub fn new(n: usize) -> Self {
        let num_words = n.div_ceil(64);
        Self {
            words: vec![0u64; num_words],
            n,
        }
    }

    /// Set bit `i`.
    pub fn set(&mut self, i: usize) {
        self.words[i / 64] |= 1u64 << (i % 64);
    }

    /// Clear bit `i`.
    pub fn clear(&mut self, i: usize) {
        self.words[i / 64] &= !(1u64 << (i % 64));
    }

    /// Returns `true` if bit `i` is set.
    #[cfg(test)]
    pub fn is_set(&self, i: usize) -> bool {
        self.words[i / 64] & (1u64 << (i % 64)) != 0
    }

    /// Returns `true` when every bit is clear.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// FNV-1a hash over all words.
    pub fn hash_val(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut h = OFFSET;
        for &w in &self.words {
            h ^= w;
            h = h.wrapping_mul(PRIME);
        }
        h
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_all_clear() {
        let bs = Bitset::new(128);
        for i in 0..128 {
            assert!(!bs.is_set(i), "bit {i} should be clear after new");
        }
        assert!(bs.is_empty());
    }

    #[test]
    fn new_zero_bits_is_empty() {
        let bs = Bitset::new(0);
        assert!(bs.is_empty());
    }

    #[test]
    fn set_makes_bit_visible() {
        let mut bs = Bitset::new(64);
        bs.set(7);
        assert!(bs.is_set(7));
        assert!(!bs.is_empty());
    }

    #[test]
    fn clear_removes_bit() {
        let mut bs = Bitset::new(64);
        bs.set(7);
        bs.clear(7);
        assert!(!bs.is_set(7));
        assert!(bs.is_empty());
    }

    #[test]
    fn set_and_clear_across_word_boundary() {
        let mut bs = Bitset::new(128);
        bs.set(63); // last bit of word 0
        bs.set(64); // first bit of word 1
        assert!(bs.is_set(63));
        assert!(bs.is_set(64));
        assert!(!bs.is_set(62));
        assert!(!bs.is_set(65));
        bs.clear(63);
        assert!(!bs.is_set(63));
        assert!(bs.is_set(64));
    }

    #[test]
    fn is_empty_only_when_all_bits_clear() {
        let mut bs = Bitset::new(5);
        bs.set(0);
        bs.set(4);
        assert!(!bs.is_empty());
        bs.clear(0);
        assert!(!bs.is_empty()); // bit 4 still set
        bs.clear(4);
        assert!(bs.is_empty());
    }

    #[test]
    fn equal_bitsets_same_hash() {
        let mut a = Bitset::new(64);
        let mut b = Bitset::new(64);
        a.set(3);
        a.set(17);
        b.set(3);
        b.set(17);
        assert_eq!(a, b);
        assert_eq!(a.hash_val(), b.hash_val());
    }

    #[test]
    fn different_bitsets_different_hash() {
        let mut a = Bitset::new(64);
        let mut b = Bitset::new(64);
        a.set(3);
        b.set(4);
        assert_ne!(a, b);
        // Hashes *should* differ.
        assert_ne!(a.hash_val(), b.hash_val());
    }

    #[test]
    fn clone_is_independent() {
        let mut a = Bitset::new(32);
        a.set(5);
        let mut b = a.clone();
        b.set(10);
        // Modifying b must not affect a.
        assert!(!a.is_set(10));
        assert!(b.is_set(5));
        assert!(b.is_set(10));
    }

    #[test]
    fn hash_stable_across_set_clear_roundtrip() {
        let mut bs = Bitset::new(64);
        let empty_hash = bs.hash_val();
        bs.set(42);
        bs.clear(42);
        assert_eq!(bs.hash_val(), empty_hash);
    }
}
