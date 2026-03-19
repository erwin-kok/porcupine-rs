mod checker;
pub mod model;

pub use model::{CheckResult, Event, EventKind, Model, Operation};

pub fn check_operations<M: Model>(history: &[Operation<M>]) -> bool {
    checker::check_operations::<M>(history) == CheckResult::Ok
}

pub fn check_operations_timeout<M: Model>(
    history: &[Operation<M>],
    _timeout: std::time::Duration,
) -> CheckResult {
    checker::check_operations::<M>(history)
}
