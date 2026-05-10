mod bitset;
mod cache;
mod checker;
mod linearization_info;
mod linearizer;
pub mod model;
mod partition;
mod skip_list;
pub mod visualization;

pub use linearization_info::{Annotation, LinearizationInfo};
pub use model::{CheckResult, Event, EventModel, Model, Operation};

use std::path::Path;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Public API — operations
// ---------------------------------------------------------------------------

/// Returns `true` if `history` is linearizable.
///
/// Runs without a time limit.  For large or adversarial histories use
/// [`check_operations_timeout`] to bound the search.
pub fn check_operations<M: Model>(history: &[Operation<M>]) -> bool {
    checker::check_operations::<M>(history, None) == CheckResult::Ok
}

/// Check `history` with a wall-clock time limit, returning a [`CheckResult`].
///
/// - `Ok`      — linearizable.
/// - `Illegal` — not linearizable.
/// - `Unknown` — the timeout elapsed before a definitive answer was found.
pub fn check_operations_timeout<M: Model>(
    history: &[Operation<M>],
    timeout: Duration,
) -> CheckResult {
    checker::check_operations::<M>(history, Some(timeout))
}

/// Check `history` and return both the verdict and [`LinearizationInfo`].
///
/// The info contains the longest partial linearization sequences found for
/// each operation, plus the original operations needed by [`visualize`] and
/// [`visualize_path`].
pub fn check_operations_info<M: Model>(
    history: &[Operation<M>],
) -> (CheckResult, LinearizationInfo<M>) {
    checker::check_operations_info::<M>(history, None)
}

/// Like [`check_operations_info`] but with a wall-clock time limit.
pub fn check_operations_info_timeout<M: Model>(
    history: &[Operation<M>],
    timeout: Duration,
) -> (CheckResult, LinearizationInfo<M>) {
    checker::check_operations_info::<M>(history, Some(timeout))
}

// ---------------------------------------------------------------------------
// Public API — events
// ---------------------------------------------------------------------------

/// Returns `true` if the event history is linearizable.
pub fn check_events<M: EventModel>(history: &[Event<M>]) -> bool {
    checker::check_events::<M>(history, None) == CheckResult::Ok
}

/// Check an event history with a wall-clock time limit, returning a [`CheckResult`].
pub fn check_events_timeout<M: EventModel>(history: &[Event<M>], timeout: Duration) -> CheckResult {
    checker::check_events::<M>(history, Some(timeout))
}

/// Check an event history and return both the verdict and [`LinearizationInfo`].
pub fn check_events_info<M: EventModel>(
    history: &[Event<M>],
) -> (CheckResult, LinearizationInfo<M>) {
    checker::check_events_info::<M>(history, None)
}

/// Like [`check_events_info`] but with a wall-clock time limit.
pub fn check_events_info_timeout<M: EventModel>(
    history: &[Event<M>],
    timeout: Duration,
) -> (CheckResult, LinearizationInfo<M>) {
    checker::check_events_info::<M>(history, Some(timeout))
}

// ---------------------------------------------------------------------------
// Public API — visualization
// ---------------------------------------------------------------------------

/// Write an HTML visualization of `info` to `output`.
///
/// The type parameter `M` must be specified so the function can call
/// `M::describe_operation`, `M::describe_state`, `M::describe_metadata`,
/// and replay `M::step` over partial linearizations to annotate each step.
pub fn visualize<M: Model, W: std::io::Write>(
    info: &LinearizationInfo<M>,
    output: &mut W,
) -> std::io::Result<()> {
    visualization::visualize::<M, W>(info, output)
}

/// Write an HTML visualization of `info` to the file at `path`.
///
/// Convenience wrapper around [`visualize`].
pub fn visualize_path<M: Model>(info: &LinearizationInfo<M>, path: &Path) -> std::io::Result<()> {
    visualization::visualize_path::<M>(info, path)
}
