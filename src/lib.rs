mod bitset;
mod cache;
mod checker;
mod linearizer;
mod model;
mod partition;
mod skip_list;

pub use model::{CheckResult, Event, EventModel, Model, Operation};

/// Returns `true` if `history` is linearizable.
pub fn check_operations<M: Model>(history: &[Operation<M>]) -> bool {
    checker::check_operations::<M>(history, None) == CheckResult::Ok
}

/// Returns `true` if the event history is linearizable.
pub fn check_events<M: EventModel>(history: &[Event<M>]) -> bool {
    checker::check_events::<M>(history, None) == CheckResult::Ok
}

/// Like [`check_operations`] but returns a [`CheckResult`] so the caller can
/// distinguish `Ok`, `Illegal`, and `Unknown`.
pub fn check_operations_timeout<M: Model>(
    history: &[Operation<M>],
    timeout: std::time::Duration,
) -> CheckResult {
    checker::check_operations::<M>(history, Some(timeout))
}

/// Like [`check_events`] but returns a [`CheckResult`].
pub fn check_events_timeout<M: EventModel>(
    history: &[Event<M>],
    timeout: std::time::Duration,
) -> CheckResult {
    checker::check_events::<M>(history, Some(timeout))
}
