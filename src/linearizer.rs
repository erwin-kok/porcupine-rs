use std::mem;
use std::sync::Arc;

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
    compute_info: bool,
    longest: Vec<Option<Arc<Vec<usize>>>>,
}

impl<'a, M: Model> Linearizer<'a, M> {
    pub fn new(partition: &'a Partition<M>, compute_info: bool) -> Self {
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
            compute_info,
            longest: if compute_info {
                vec![None; m]
            } else {
                Vec::new()
            },
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

        // Temporarily mark the bit so we can probe the cache with the
        // would-be linearized set, without allocating a clone upfront.
        self.linearized.set(op_index);

        if self.cache.cache_contains(&self.linearized, &next_state) {
            // Already explored — prune.  Restore the bit and bail.
            self.linearized.clear(op_index);
            return None;
        }

        // Cache miss: store a snapshot, then restore the bit.
        // lift() will set it permanently when the caller commits this choice.
        self.cache
            .cache_insert(self.linearized.clone(), next_state.clone());
        self.linearized.clear(op_index);
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

    // -----------------------------------------------------------------------
    // Partial linearization tracking
    // -----------------------------------------------------------------------

    /// Update `longest` for every operation currently on the stack.
    ///
    /// Called just before [`backtrack`] when a `Return` entry is at the head
    /// of the active history.  At that moment the stack represents the longest
    /// prefix we've found so far that includes each of those operations.
    ///
    /// The sequence is built lazily (at most once per call) and shared by all
    /// entries that benefit from it — an `Arc` clone is O(1).
    pub fn update_longest(&mut self) {
        if !self.compute_info {
            return;
        }

        let depth = self.stack.len();
        // Shared sequence built at most once per call.
        let mut seq: Option<Arc<Vec<usize>>> = None;

        for frame in &self.stack {
            let id = frame.op_index;
            let needs_update = match &self.longest[id] {
                None => true,
                Some(prev) => depth > prev.len(),
            };
            if needs_update {
                if seq.is_none() {
                    // Build lazily: collect all op_indices currently on the stack
                    // in linearization order.
                    let s: Vec<usize> = self.stack.iter().map(|f| f.op_index).collect();
                    seq = Some(Arc::new(s));
                }
                self.longest[id] = seq.clone();
            }
        }
    }

    /// Called when the search completes with a full linearization.
    ///
    /// Sets every entry in `longest` to the complete linearization sequence
    /// (the entire stack, in order).  All entries share the same `Arc`.
    pub fn finalize_longest(&mut self) {
        if !self.compute_info {
            return;
        }

        let seq = Arc::new(self.stack.iter().map(|f| f.op_index).collect::<Vec<_>>());
        for slot in &mut self.longest {
            *slot = Some(seq.clone());
        }
    }

    /// Consume the `Linearizer` and return the `longest` array.
    pub fn into_longest(self) -> Vec<Option<Arc<Vec<usize>>>> {
        self.longest
    }
}
