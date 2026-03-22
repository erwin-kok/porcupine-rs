use std::mem;

use crate::bitset::Bitset;
use crate::cache::Cache;
use crate::model::Model;
use crate::partition::{CheckEntry, Partition};
use crate::skip_list::SkipList;

struct Frame<S> {
    history_pos: usize,
    cr_index: usize,
    prior_state: S,
}

// ---------------------------------------------------------------------------
// Linearizer
// ---------------------------------------------------------------------------

pub struct Linearizer<'a, M: Model> {
    partition: &'a Partition<M>,
    call_pos: Vec<usize>,
    ret_pos: Vec<usize>,
    sl: SkipList,
    state: M::State,
    linearized: Bitset,
    cache: Cache<M>,
    stack: Vec<Frame<M::State>>,
}

impl<'a, M: Model> Linearizer<'a, M> {
    pub fn new(partition: &'a Partition<M>) -> Self {
        let n = partition.check_history.len();
        let m = partition.call_returns.len();

        let mut call_pos = vec![0usize; m];
        let mut ret_pos = vec![0usize; m];
        for (pos, entry) in partition.check_history.iter().enumerate() {
            match entry {
                CheckEntry::Call { cr_index, .. } => call_pos[*cr_index] = pos,
                CheckEntry::Return { cr_index, .. } => ret_pos[*cr_index] = pos,
            }
        }

        Self {
            partition,
            call_pos,
            ret_pos,
            sl: SkipList::new(n),
            state: M::init(),
            linearized: Bitset::new(m),
            cache: Cache::new(),
            stack: Vec::new(),
        }
    }

    pub fn front(&self) -> usize {
        self.sl.front()
    }

    /// The active entry immediately after `cur`.
    pub fn next_of(&self, cur: usize) -> usize {
        self.sl.next_of(cur)
    }

    pub fn try_linearize(&mut self, pos: usize) -> Option<M::State> {
        let cr_index = match self.partition.check_history[pos] {
            CheckEntry::Call { cr_index, .. } => cr_index,
            CheckEntry::Return { .. } => {
                panic!("try_linearize called on a Return entry at pos={pos}")
            }
        };

        let (accepted, next_state) = {
            let cr = &self.partition.call_returns[cr_index];
            M::step(&self.state, &cr.input, &cr.output)
        };

        if !accepted {
            return None;
        }

        let mut next_linearized = self.linearized.clone();
        next_linearized.set(cr_index);

        if self.cache.cache_contains(&next_linearized, &next_state) {
            return None; // already explored — prune
        }

        self.cache.cache_insert(next_linearized, next_state.clone());
        Some(next_state)
    }

    pub fn lift(&mut self, pos: usize, next_state: M::State) {
        let cr_index = match self.partition.check_history[pos] {
            CheckEntry::Call { cr_index, .. } => cr_index,
            CheckEntry::Return { .. } => panic!("lift called on a Return entry at pos={pos}"),
        };

        let prior = mem::replace(&mut self.state, next_state);
        self.linearized.set(cr_index); // cr.id == cr_index

        self.stack.push(Frame {
            history_pos: pos,
            cr_index,
            prior_state: prior,
        });

        // Remove call first, return second.  restore() must be the reverse.
        self.sl.remove(self.call_pos[cr_index]);
        self.sl.remove(self.ret_pos[cr_index]);
    }

    pub fn backtrack(&mut self) -> Option<usize> {
        let frame = self.stack.pop()?;

        self.state = frame.prior_state;
        self.linearized.clear(frame.cr_index); // cr.id == cr_index

        // Restore in reverse order: return first, then call.
        self.sl.restore(self.ret_pos[frame.cr_index]);
        self.sl.restore(self.call_pos[frame.cr_index]);

        Some(frame.history_pos)
    }
}
