use std::mem;

use crate::bitset::Bitset;
use crate::cache::Cache;
use crate::model::Model;
use crate::partition::{CheckEntry, Partition};
use crate::skip_list::SkipList;

struct Frame<S> {
    /// Position in `check_history` of the lifted Call entry.
    history_pos: usize,
    /// Index into `partition.ops`; equals the operation's bitset index.
    op_index: usize,
    /// Model state before the lift — restored on backtrack.
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
        let m = partition.ops.len();

        let mut call_pos = vec![0usize; m];
        let mut ret_pos = vec![0usize; m];
        for (pos, entry) in partition.check_history.iter().enumerate() {
            match entry {
                CheckEntry::Call { op_index, .. } => call_pos[*op_index] = pos,
                CheckEntry::Return { op_index, .. } => ret_pos[*op_index] = pos,
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

    pub fn next_of(&self, cur: usize) -> usize {
        self.sl.next_of(cur)
    }

    pub fn try_linearize(&mut self, pos: usize) -> Option<M::State> {
        let op_index = match self.partition.check_history[pos] {
            CheckEntry::Call { op_index, .. } => op_index,
            CheckEntry::Return { .. } => panic!("try_linearize called on a Return at pos={pos}"),
        };

        // Borrow partition immutably; drop before touching self.cache.
        let (accepted, next_state) = {
            let op = &self.partition.ops[op_index];
            M::step(&self.state, &op.op)
        };

        if !accepted {
            return None;
        }

        let mut next_linearized = self.linearized.clone();
        next_linearized.set(op_index);

        if self.cache.cache_contains(&next_linearized, &next_state) {
            return None;
        }

        self.cache.cache_insert(next_linearized, next_state.clone());
        Some(next_state)
    }

    pub fn lift(&mut self, pos: usize, next_state: M::State) {
        let op_index = match self.partition.check_history[pos] {
            CheckEntry::Call { op_index, .. } => op_index,
            CheckEntry::Return { .. } => panic!("lift called on a Return at pos={pos}"),
        };

        let prior = mem::replace(&mut self.state, next_state);
        self.linearized.set(op_index);
        self.stack.push(Frame {
            history_pos: pos,
            op_index,
            prior_state: prior,
        });

        // Remove call first, return second; restore must be the reverse.
        self.sl.remove(self.call_pos[op_index]);
        self.sl.remove(self.ret_pos[op_index]);
    }

    pub fn backtrack(&mut self) -> Option<usize> {
        let frame = self.stack.pop()?;

        self.state = frame.prior_state;
        self.linearized.clear(frame.op_index);

        // Restore in reverse order: return first, then call.
        self.sl.restore(self.ret_pos[frame.op_index]);
        self.sl.restore(self.call_pos[frame.op_index]);

        Some(frame.history_pos)
    }
}
