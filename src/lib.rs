use std::time::Duration;

use crate::{
    checker::LinearizationInfo,
    model::{CheckResult, Event, Model, Operation},
};

mod bitset;
mod checker;
mod model;

/// CheckOperations checks whether a history is linearizable.
pub fn check_operations<M: Model>(history: &[Operation<M::Value, M::Metadata>]) -> bool {
    let (res, _) = checker::check_operations::<M>(history, false, Duration::ZERO);
    res == CheckResult::Ok
}

// CheckOperationsTimeout checks whether a history is linearizable, with a
// timeout.
//
// A timeout of 0 is interpreted as an unlimited timeout.
pub fn check_operations_timeout<M: Model>(
    history: &[Operation<M::Value, M::Metadata>],
    timeout: Duration,
) -> CheckResult {
    let (res, _) = checker::check_operations::<M>(history, false, timeout);
    res
}

// CheckOperationsVerbose checks whether a history is linearizable while
// computing data that can be used to visualize the history and linearization.
//
// The returned LinearizationInfo can be used with [Visualize].
pub fn check_operations_verbose<M: Model>(
    history: &[Operation<M::Value, M::Metadata>],
    timeout: Duration,
) -> (CheckResult, LinearizationInfo) {
    checker::check_operations::<M>(history, true, timeout)
}

// CheckEvents checks whether a history is linearizable.
pub fn check_events<M: Model>(history: &[Event<M::Value, M::Metadata>]) -> bool {
    let (res, _) = checker::check_events::<M>(history, false, Duration::ZERO);
    res == CheckResult::Ok
}

// CheckEventsTimeout checks whether a history is linearizable, with a timeout.
//
// A timeout of 0 is interpreted as an unlimited timeout.
pub fn check_events_timeout<M: Model>(
    history: &[Event<M::Value, M::Metadata>],
    timeout: Duration,
) -> CheckResult {
    let (res, _) = checker::check_events::<M>(history, false, timeout);
    res
}

// CheckEventsVerbose checks whether a history is linearizable while computing
// data that can be used to visualize the history and linearization.
//
// The returned LinearizationInfo can be used with [Visualize].
pub fn check_events_verbose<M: Model>(
    history: &[Event<M::Value, M::Metadata>],
    timeout: Duration,
) -> (CheckResult, LinearizationInfo) {
    checker::check_events::<M>(history, true, timeout)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::{Operation, model::Model};

    #[derive(Debug, Clone)]
    pub struct RegisterInput {
        op: bool,
        value: u32,
    }

    impl RegisterInput {
        pub fn input(op: bool, value: u32) -> Self {
            Self { op, value }
        }

        pub fn output(value: u32) -> Self {
            Self { op: false, value }
        }
    }

    pub struct RegisterModel {}

    impl Model for RegisterModel {
        type State = u32;
        type Value = RegisterInput;
        type Metadata = u32;

        fn init() -> u32 {
            0
        }

        fn step(
            state: &u32,
            register_input: &RegisterInput,
            output: &RegisterInput,
        ) -> (bool, u32) {
            if !register_input.op {
                (true, register_input.value)
            } else {
                (output.value == *state, *state)
            }
        }

        fn describe_operation(register_input: &RegisterInput, output: &RegisterInput) -> String {
            if register_input.op {
                format!("get() -> '{}'", output.value)
            } else {
                format!("put('{}')", register_input.value)
            }
        }
    }

    #[test]
    fn test_register_model() {
        let ops = vec![
            Operation::new(
                0,
                RegisterInput::input(false, 100),
                0,
                RegisterInput::output(0),
                100,
            ),
            Operation::new(
                1,
                RegisterInput::input(true, 0),
                25,
                RegisterInput::output(100),
                75,
            ),
            Operation::new(
                2,
                RegisterInput::input(true, 0),
                30,
                RegisterInput::output(0),
                60,
            ),
        ];
        let res = crate::check_operations::<RegisterModel>(&ops);
        assert!(res, "expected operations to be linearizable");
    }
}
