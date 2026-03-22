use crate::{
    linearizer::Linearizer,
    model::{CheckResult, Event, Model, Operation},
    partition::{CheckEntry, Partition},
};

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

pub fn check_operations<M: Model>(history: &[Operation<M>]) -> CheckResult {
    for partition_ops in M::partition_operations(history) {
        let part = Partition::from_operations(&partition_ops);
        if !check_single::<M>(&part) {
            return CheckResult::Illegal;
        }
    }
    CheckResult::Ok
}

pub fn check_events<M: Model>(history: &[Event<M>]) -> CheckResult {
    for partition_events in M::partition_events(history) {
        let part = Partition::from_events(&partition_events);
        if !check_single::<M>(&part) {
            return CheckResult::Illegal;
        }
    }
    CheckResult::Ok
}

// ---------------------------------------------------------------------------
// Core iterative search
// ---------------------------------------------------------------------------
fn check_single<M: Model>(partition: &Partition<M>) -> bool {
    let n = partition.check_history.len();
    let mut lz = Linearizer::<M>::new(partition);
    let mut cur = lz.front();

    while cur < n {
        match partition.check_history[cur] {
            CheckEntry::Call { .. } => {
                if let Some(next_state) = lz.try_linearize(cur) {
                    lz.lift(cur, next_state);
                    cur = lz.front(); // restart scan from the head
                } else {
                    cur = lz.next_of(cur); // skip this candidate
                }
            }
            CheckEntry::Return { .. } => {
                match lz.backtrack() {
                    Some(pos) => cur = lz.next_of(pos), // resume after un-lifted call
                    None => return false,               // stack empty — not linearizable
                }
            }
        }
    }

    true // every entry lifted — complete linearization found
}
