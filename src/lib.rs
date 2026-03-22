mod bitset;
mod cache;
mod checker;
mod linearizer;
mod model;
mod partition;
mod skip_list;

pub use model::{CheckResult, Event, Model, Operation};

/// Returns `true` if `history` is linearizable.
pub fn check_operations<M: Model>(history: &[Operation<M>]) -> bool {
    checker::check_operations::<M>(history) == CheckResult::Ok
}

/// Returns `true` if the event history is linearizable.
pub fn check_events<M: Model>(history: &[Event<M>]) -> bool {
    checker::check_events::<M>(history) == CheckResult::Ok
}

/// Like [`check_operations`] but returns a [`CheckResult`] so the caller can
/// distinguish `Ok`, `Illegal`, and `Unknown`.
///
/// The `timeout` parameter is accepted for API compatibility but is not enforced yet.
pub fn check_operations_timeout<M: Model>(
    history: &[Operation<M>],
    _timeout: std::time::Duration,
) -> CheckResult {
    checker::check_operations::<M>(history)
}

/// Like [`check_events`] but returns a [`CheckResult`].
///
/// The `timeout` parameter is accepted for API compatibility but is not enforced yet.
pub fn check_events_timeout<M: Model>(
    history: &[Event<M>],
    _timeout: std::time::Duration,
) -> CheckResult {
    checker::check_events::<M>(history)
}
