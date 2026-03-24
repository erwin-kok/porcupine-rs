use crate::{
    linearizer::Linearizer,
    model::{CheckResult, Event, EventModel, Model, Operation},
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

pub fn check_events<M: EventModel>(history: &[Event<M>]) -> CheckResult {
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
    let mut linearizer = Linearizer::<M>::new(partition);
    let mut current = linearizer.front();

    while current < n {
        match partition.check_history[current] {
            CheckEntry::Call { .. } => {
                if let Some(next_state) = linearizer.try_linearize(current) {
                    linearizer.lift(current, next_state);
                    current = linearizer.front(); // restart scan from the head
                } else {
                    current = linearizer.next_of(current); // skip this candidate
                }
            }
            CheckEntry::Return { .. } => {
                match linearizer.backtrack() {
                    Some(pos) => current = linearizer.next_of(pos), // resume after un-lifted call
                    None => return false, // stack empty — not linearizable
                }
            }
        }
    }

    true // every entry lifted — complete linearization found
}
