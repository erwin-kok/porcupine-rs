use std::time::Duration;

use crate::model::{CheckResult, Event, Model, Operation};

pub struct LinearizationInfo {}

pub(crate) fn check_events<M: Model>(
    history: &[Event<M::Value, M::Metadata>],
    verbose: bool,
    timeout: Duration,
) -> (CheckResult, LinearizationInfo) {
    (CheckResult::Ok, LinearizationInfo {})
}

pub(crate) fn check_operations<M: Model>(
    history: &[Operation<M::Input, M::Output, M::Metadata>],
    verbose: bool,
    timeout: Duration,
) -> (CheckResult, LinearizationInfo) {
    (CheckResult::Ok, LinearizationInfo {})
}
