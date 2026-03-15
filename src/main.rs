// ── Porcupine-style linearizability checker using P-compositionality ─────
//
// Based on:
//   Wing & Gong (1993) — "Testing and Verifying Concurrent Objects"
//   Lowe (2017)        — "Testing for Linearisability"
//   Anagnostakis et al — Porcupine (Go) checker
//
// Core idea (P-compositionality):
//   A history H is linearizable iff for every key k, the sub-history
//   H|k (operations restricted to k) is linearizable. This lets us
//   check each key independently and in parallel, turning an O(n!)
//   global search into many smaller O(m!) searches (m << n).
//
// Within each sub-history we use the Wing-Gong search:
//   - Lift all completed operations into a linearization candidate set.
//   - Try prepending each minimal (no-predecessor) op as the next LP.
//   - Verify the model state after applying it.
//   - Recurse on the remaining ops; backtrack on failure.

use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt; // cargo add rayon

// ── 1. Abstract sequential specification (the "model") ────────────────────

/// The sequential model your concurrent object must match.
/// For a KV register: state = Option<String>.
pub trait Model: Clone + Send + Sync {
    type State: Clone + Send + Sync + fmt::Debug;
    type Op: Clone + Send + Sync + fmt::Debug;
    type Ret: Clone + PartialEq + Send + Sync + fmt::Debug;

    fn init() -> Self::State;

    /// Apply `op` to `state`; return (new_state, expected_return_value).
    fn step(state: &Self::State, op: &Self::Op) -> (Self::State, Self::Ret);
}

/// A single-register (per-key) KV model.
#[derive(Clone)]
pub struct KVModel;

#[derive(Clone, Debug)]
pub enum KVOp {
    Write(String),
    Read,
}

impl Model for KVModel {
    type State = Option<String>; // None = uninitialized
    type Op = KVOp;
    type Ret = Option<String>; // None = write ack / uninit read

    fn init() -> Self::State {
        None
    }

    fn step(state: &Self::State, op: &Self::Op) -> (Self::State, Self::Ret) {
        match op {
            KVOp::Write(v) => (Some(v.clone()), None),
            KVOp::Read => (state.clone(), state.clone()),
        }
    }
}

// ── 2. History representation ─────────────────────────────────────────────

/// Interval [call_time, return_time] of a single client operation.
/// `ret_value` is None for pending (crashed / in-flight) ops.
#[derive(Clone, Debug)]
pub struct Entry<M: Model> {
    pub id: usize,
    pub key: String, // partition key (P-compositionality axis)
    pub op: M::Op,
    pub ret: Option<M::Ret>, // None = pending
    pub call_time: u64,
    pub ret_time: u64, // u64::MAX if pending
}

// ── 3. P-compositionality partition ───────────────────────────────────────

/// Partition the history by key.  Returns a map key → sub-history.
pub fn partition<M: Model>(history: &[Entry<M>]) -> HashMap<String, Vec<Entry<M>>> {
    let mut map: HashMap<String, Vec<Entry<M>>> = HashMap::new();
    for e in history {
        map.entry(e.key.clone()).or_default().push(e.clone());
    }
    map
}

// ── 4. Wing-Gong linearization search ─────────────────────────────────────

/// Result of checking a single sub-history.
#[derive(Debug)]
pub enum CheckResult {
    Linearizable,
    NotLinearizable { witness: Vec<usize> }, // op IDs in the failing prefix
}

/// Internal state for the Wing-Gong backtracking search.
struct Searcher<M: Model> {
    ops: Vec<Entry<M>>, // remaining unchecked ops
    model: M::State,
    order: Vec<usize>, // linearization order built so far (op IDs)
}

impl<M: Model> Searcher<M> {
    fn new(ops: Vec<Entry<M>>) -> Self {
        // Pending ops are treated as completed at infinity —
        // they can be ordered anywhere (conservative: assume worst case).
        Self {
            ops,
            model: M::init(),
            order: Vec::new(),
        }
    }

    /// Return indices (into self.ops) of ops with no predecessor in the
    /// remaining set — i.e. no other remaining op ended before this one started.
    fn minimal_ops(&self) -> Vec<usize> {
        self.ops
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                !self
                    .ops
                    .iter()
                    .enumerate()
                    .any(|(j, b)| j != *i && b.ret_time <= a.call_time)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Recursive Wing-Gong search.
    /// Returns true iff a valid linearization exists.
    fn search(&mut self) -> bool {
        if self.ops.is_empty() {
            return true;
        }

        let candidates = self.minimal_ops();

        for idx in candidates {
            let entry = self.ops[idx].clone();
            let (new_state, expected_ret) = M::step(&self.model, &entry.op);

            // If op completed, its return value must match the model.
            let ret_ok = match &entry.ret {
                None => true, // pending — any return is fine
                Some(ret) => ret == &expected_ret,
            };

            if ret_ok {
                // Commit this op as the next linearization point.
                let saved_model = self.model.clone();
                self.model = new_state;
                self.order.push(entry.id);
                self.ops.remove(idx);

                if self.search() {
                    return true;
                }

                // Backtrack.
                self.ops.insert(idx, entry);
                self.order.pop();
                self.model = saved_model;
            }
        }

        false
    }
}

/// Check one sub-history (single key) for linearizability.
pub fn check_subhistory<M: Model>(ops: Vec<Entry<M>>) -> CheckResult {
    let mut s = Searcher::new(ops);
    if s.search() {
        CheckResult::Linearizable
    } else {
        // Return the IDs that were linearized before the contradiction.
        CheckResult::NotLinearizable {
            witness: s.order.clone(),
        }
    }
}

// ── 5. Top-level parallel checker (P-compositionality) ────────────────────

/// Check the full history using P-compositionality + Wing-Gong per partition.
/// Runs each key's sub-history in parallel via Rayon.
pub fn check_linearizable<M: Model>(history: Vec<Entry<M>>) -> CheckResult {
    let partitions: Vec<(String, Vec<Entry<M>>)> = partition(&history).into_iter().collect();

    // Each key is independent — check in parallel.
    let results: Vec<(String, CheckResult)> = partitions
        .into_par_iter()
        .map(|(key, ops)| {
            let result = check_subhistory::<M>(ops);
            (key, result)
        })
        .collect();

    // Any failing sub-history falsifies the whole history.
    for (key, result) in results {
        if let CheckResult::NotLinearizable { witness } = result {
            println!(
                "  ✗ Violation on key '{key}': witness prefix = {:?}",
                witness
            );
            return CheckResult::NotLinearizable { witness };
        }
    }
    CheckResult::Linearizable
}

// ── 6. History builder (test harness) ─────────────────────────────────────

pub struct HistoryBuilder<M: Model> {
    entries: Vec<Entry<M>>,
    next_id: usize,
}

impl<M: Model> HistoryBuilder<M> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 0,
        }
    }

    pub fn add(
        &mut self,
        key: &str,
        op: M::Op,
        ret: Option<M::Ret>,
        call_time: u64,
        ret_time: u64,
    ) -> &mut Self {
        self.entries.push(Entry {
            id: self.next_id,
            key: key.into(),
            op,
            ret,
            call_time,
            ret_time,
        });
        self.next_id += 1;
        self
    }

    pub fn pending(&mut self, key: &str, op: M::Op, call_time: u64) -> &mut Self {
        self.add(key, op, None, call_time, u64::MAX)
    }

    pub fn build(self) -> Vec<Entry<M>> {
        self.entries
    }
}

// ── 7. Demo ────────────────────────────────────────────────────────────────

fn main() {
    // ── Linearizable history ──────────────────────────────────────────────
    //
    //  A: |── write(x,1) ──|
    //  B:         |── read→1 ──|
    //  C:                  |── write(x,2) ──|
    //  D:                           |── read→2 ──|
    //
    // Valid linearization: write(x,1) → read→1 → write(x,2) → read→2
    println!("── History 1: linearizable ──");

    let h1 = {
        let mut b = HistoryBuilder::<KVModel>::new();
        b.add("x", KVOp::Write("1".into()), None, 0, 40);
        b.add("x", KVOp::Read, Some(Some("1".into())), 20, 60);
        b.add("x", KVOp::Write("2".into()), None, 50, 90);
        b.add("x", KVOp::Read, Some(Some("2".into())), 80, 120);
        b.build()
    };
    println!("  result: {:?}", check_linearizable::<KVModel>(h1));

    // ── Non-linearizable history ──────────────────────────────────────────
    //
    //  A: |── write(x,1) ──|
    //  B: |─────── read→2 ─────────|    ← reads a value never written!
    //
    // No valid sequential order exists.
    println!("\n── History 2: NOT linearizable ──");
    let h2 = {
        let mut b = HistoryBuilder::<KVModel>::new();
        b.add("x", KVOp::Write("1".into()), None, 0, 50);
        b.add("x", KVOp::Read, Some(Some("2".into())), 0, 80);
        b.build()
    };
    println!("  result: {:?}", check_linearizable::<KVModel>(h2));

    // ── P-compositionality: two independent keys, one bad ─────────────────
    println!("\n── History 3: 'y' is clean, 'z' violates ──");
    let h3 = {
        let mut b = HistoryBuilder::<KVModel>::new();
        // key y — fine
        b.add("y", KVOp::Write("a".into()), None, 0, 30);
        b.add("y", KVOp::Read, Some(Some("a".into())), 20, 50);
        // key z — violation: read sees "b" but only "a" was written
        b.add("z", KVOp::Write("a".into()), None, 0, 30);
        b.add("z", KVOp::Read, Some(Some("b".into())), 20, 50);
        b.build()
    };
    println!("  result: {:?}", check_linearizable::<KVModel>(h3));

    // ── Pending (in-flight) op ─────────────────────────────────────────────
    println!("\n── History 4: pending write, read may see either ──");
    let h4 = {
        let mut b = HistoryBuilder::<KVModel>::new();
        b.add("x", KVOp::Write("9".into()), None, 0, u64::MAX); // pending
        b.add("x", KVOp::Read, Some(None), 5, 40); // read→None ok
        b.build()
    };
    println!("  result: {:?}", check_linearizable::<KVModel>(h4));
}
