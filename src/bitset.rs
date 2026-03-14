use std::hash::{Hash, Hasher};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitset {
    data: Vec<u64>,
}

impl Bitset {
    pub fn new(bits: usize) -> Self {
        let chunks = bits.div_ceil(64);
        Bitset {
            data: vec![0; chunks],
        }
    }

    pub fn clone(&self) -> Self {
        Bitset {
            data: self.data.clone(),
        }
    }

    fn index(&self, pos: usize) -> (usize, u32) {
        (pos / 64, (pos % 64) as u32)
    }

    pub fn set(&mut self, pos: usize) {
        let (major, minor) = self.index(pos);
        // Safety: new() ensures data is large enough for 'bits',
        // but if pos exceeds the intended capacity, we should probably panic or resize.
        // For now, assuming pos is within the bounds of the created bitset.
        if major >= self.data.len() {
            panic!(
                "Bit position {} out of bounds for bitset of size {}",
                pos,
                self.data.len() * 64
            );
        }
        self.data[major] |= 1u64 << minor;
    }

    pub fn clear(&mut self, pos: usize) {
        let (major, minor) = self.index(pos);
        if major >= self.data.len() {
            panic!(
                "Bit position {} out of bounds for bitset of size {}",
                pos,
                self.data.len() * 64
            );
        }
        self.data[major] &= !(1u64 << minor);
    }

    pub fn get(&self, pos: usize) -> bool {
        let (major, minor) = self.index(pos);
        if major >= self.data.len() {
            return false;
        }
        (self.data[major] >> minor) & 1 == 1
    }

    pub fn as_slice(&self) -> &[u64] {
        &self.data
    }

    pub fn popcnt(&self) -> usize {
        self.data.iter().map(|v| v.count_ones() as usize).sum()
    }

    pub fn hash(&self) -> u64 {
        let mut hash = self.popcnt() as u64;
        for &v in &self.data {
            hash ^= v;
        }
        hash
    }

    pub fn equals(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Default for Bitset {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Hash for Bitset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_single_bit() {
        let mut bs = Bitset::new(100); // Creates 2 chunks (128 bits capacity)

        // Set bit 0
        bs.set(0);
        assert_eq!(bs.data[0], 1u64); // Only bit 0 is set

        // Set bit 63
        bs.set(63);
        // Now bits 0 and 63 are set: 1 | (1<<63)
        assert_eq!(bs.data[0], 1u64 | (1u64 << 63));

        // Set bit 64 (first bit of second chunk)
        bs.set(64);
        assert_eq!(bs.data[1], 1u64);
        assert_eq!(bs.data[0], 1u64 | (1u64 << 63)); // Unchanged
    }

    #[test]
    fn test_clear_single_bit() {
        let mut bs = Bitset::new(100);

        // Set bits 0, 63, 64
        bs.set(0);
        bs.set(63);
        bs.set(64);

        assert_eq!(bs.popcnt(), 3);
        assert_eq!(bs.data[0], 1u64 | (1u64 << 63));
        assert_eq!(bs.data[1], 1u64);

        // Clear bit 0
        bs.clear(0);
        assert_eq!(bs.popcnt(), 2);
        // Bit 0 cleared, bit 63 remains
        assert_eq!(bs.data[0], 1u64 << 63);
        assert_eq!(bs.data[1], 1u64);

        // Clear bit 63
        bs.clear(63);
        assert_eq!(bs.popcnt(), 1);
        assert_eq!(bs.data[0], 0u64);
        assert_eq!(bs.data[1], 1u64);
    }

    #[test]
    fn test_get_method() {
        let mut bs = Bitset::new(100);

        // Initially all false
        assert!(!bs.get(0));
        assert!(!bs.get(63));
        assert!(!bs.get(64));
        assert!(!bs.get(99));

        // Set specific bits
        bs.set(0);
        bs.set(63);
        bs.set(64);
        bs.set(99);

        // Verify set bits
        assert!(bs.get(0));
        assert!(bs.get(63));
        assert!(bs.get(64));
        assert!(bs.get(99));

        // Verify unset bits
        assert!(!bs.get(1));
        assert!(!bs.get(62));
        assert!(!bs.get(65));
        assert!(!bs.get(98));

        // Test boundary: bit 127 (if capacity allows)
        let mut bs_large = Bitset::new(128);
        bs_large.set(127);
        assert!(bs_large.get(127));
        assert!(!bs_large.get(128));
    }

    #[test]
    fn test_as_slice() {
        let mut bs = Bitset::new(100);
        bs.set(0);
        bs.set(64);

        let slice = bs.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 1u64);
        assert_eq!(slice[1], 1u64);
    }

    #[test]
    fn test_popcnt_with_get() {
        let mut bs = Bitset::new(200);

        // Set every 10th bit
        for i in (0..200).step_by(10) {
            bs.set(i);
        }

        assert_eq!(bs.popcnt(), 20); // 0, 10, ..., 190

        // Verify via get()
        let mut count_via_get = 0;
        for i in 0..200 {
            if bs.get(i) {
                count_via_get += 1;
            }
        }
        assert_eq!(count_via_get, 20);
    }

    #[test]
    fn test_clear_and_get_interaction() {
        let mut bs = Bitset::new(64);
        bs.set(32);
        assert!(bs.get(32));

        bs.clear(32);
        assert!(!bs.get(32));
        assert_eq!(bs.popcnt(), 0);
    }

    #[test]
    fn test_edge_cases_get() {
        // Empty bitset
        let bs = Bitset::new(0);
        assert!(!bs.get(0));

        // Single bit bitset
        let mut bs = Bitset::new(1);
        bs.set(0);
        assert!(bs.get(0));
        assert!(!bs.get(1));
    }
}
