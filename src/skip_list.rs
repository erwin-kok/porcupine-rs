pub struct SkipList {
    next: Vec<usize>,
    prev: Vec<usize>,
    pub head: usize,
}

impl SkipList {
    pub fn new(n: usize) -> Self {
        let mut next = vec![0usize; n + 1];
        let mut prev = vec![0usize; n + 1];

        for i in 0..n {
            next[i] = i + 1; // last entry (n-1) points to n = head
            prev[i] = if i == 0 { n } else { i - 1 };
        }
        next[n] = if n == 0 { n } else { 0 };

        Self {
            next,
            prev,
            head: n,
        }
    }

    #[inline]
    pub fn front(&self) -> usize {
        self.next[self.head]
    }

    /// The active entry immediately after `i`.
    #[inline]
    pub fn next_of(&self, i: usize) -> usize {
        self.next[i]
    }

    pub fn remove(&mut self, i: usize) {
        let p = self.prev[i];
        let nx = self.next[i];
        self.next[p] = nx;
        self.prev[nx] = p;
    }

    pub fn restore(&mut self, i: usize) {
        let p = self.prev[i];
        let nx = self.next[i];
        self.next[p] = i;
        self.prev[nx] = i;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: collect all active entries in order.
    fn active(sl: &SkipList, n: usize) -> Vec<usize> {
        let mut result = Vec::new();
        let mut cur = sl.front();
        while cur < n {
            result.push(cur);
            cur = sl.next_of(cur);
        }
        result
    }

    #[test]
    fn empty_list_n0_front_is_head() {
        let sl = SkipList::new(0);
        // front() returns head (= 0), and 0 < 0 is false, so the loop never runs.
        assert_eq!(sl.front(), sl.head);
        assert_eq!(active(&sl, 0), vec![]);
    }

    #[test]
    fn single_entry_initial_state() {
        let sl = SkipList::new(1);
        assert_eq!(active(&sl, 1), vec![0]);
    }

    #[test]
    fn four_entries_initial_order() {
        let sl = SkipList::new(4);
        assert_eq!(active(&sl, 4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn remove_first_entry() {
        let mut sl = SkipList::new(4);
        sl.remove(0);
        assert_eq!(active(&sl, 4), vec![1, 2, 3]);
    }

    #[test]
    fn remove_middle_entry() {
        let mut sl = SkipList::new(4);
        sl.remove(2);
        assert_eq!(active(&sl, 4), vec![0, 1, 3]);
    }

    #[test]
    fn remove_last_entry() {
        let mut sl = SkipList::new(4);
        sl.remove(3);
        assert_eq!(active(&sl, 4), vec![0, 1, 2]);
    }

    #[test]
    fn remove_all_entries_makes_list_empty() {
        let n = 4;
        let mut sl = SkipList::new(n);
        for i in 0..n {
            sl.remove(i);
        }
        assert_eq!(active(&sl, n), vec![]);
        // front() == head, and head >= n, so loop condition is false.
        assert!(sl.front() >= n);
    }

    #[test]
    fn restore_reverses_remove() {
        let mut sl = SkipList::new(4);
        sl.remove(2);
        assert_eq!(active(&sl, 4), vec![0, 1, 3]);
        sl.restore(2);
        assert_eq!(active(&sl, 4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn remove_adjacent_pair_restore_reverse_order() {
        // Remove 0 then 1; restore 1 then 0. (Correct reverse order.)
        let mut sl = SkipList::new(4);
        sl.remove(0);
        sl.remove(1);
        assert_eq!(active(&sl, 4), vec![2, 3]);
        sl.restore(1);
        sl.restore(0);
        assert_eq!(active(&sl, 4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn remove_non_adjacent_pair_either_restore_order_works() {
        // Remove 0 and 2 (not adjacent); restore in any order should be fine.
        let mut sl = SkipList::new(4);
        sl.remove(0);
        sl.remove(2);
        assert_eq!(active(&sl, 4), vec![1, 3]);

        // Restore 0 first, then 2.
        sl.restore(0);
        sl.restore(2);
        assert_eq!(active(&sl, 4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn lift_and_restore_both_entries_of_one_operation() {
        // Simulates lifting op 0 whose call is at pos 0 and return at pos 2.
        let n = 4; // [call0, call1, ret0, ret1]
        let call_pos = [0usize, 1];
        let ret_pos = [2usize, 3];
        let mut sl = SkipList::new(n);

        // Lift op 0: remove call first, then return.
        sl.remove(call_pos[0]);
        sl.remove(ret_pos[0]);
        assert_eq!(active(&sl, n), vec![1, 3]);

        // Backtrack: restore in reverse order (ret first, then call).
        sl.restore(ret_pos[0]);
        sl.restore(call_pos[0]);
        assert_eq!(active(&sl, n), vec![0, 1, 2, 3]);
    }

    #[test]
    fn next_of_advances_correctly_after_remove() {
        let mut sl = SkipList::new(4);
        sl.remove(1);
        // After removing 1, next_of(0) should skip straight to 2.
        assert_eq!(sl.next_of(0), 2);
    }

    #[test]
    fn full_round_trip_all_entries() {
        let n = 5;
        let mut sl = SkipList::new(n);

        // Remove in forward order.
        for i in 0..n {
            sl.remove(i);
        }
        assert_eq!(active(&sl, n), vec![]);

        // Restore in reverse order (correct LIFO order).
        for i in (0..n).rev() {
            sl.restore(i);
        }
        assert_eq!(active(&sl, n), vec![0, 1, 2, 3, 4]);
    }
}
