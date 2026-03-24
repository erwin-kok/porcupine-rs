use std::fmt::Debug;
use std::hash::Hash;

// ---------------------------------------------------------------------------
// Model trait
// ---------------------------------------------------------------------------

pub trait Model: Clone + Send + Sync {
    type State: Clone + Send + Sync + Debug + Hash + Eq;
    type Op: Clone + Send + Sync + Debug;
    type Metadata: Clone + Send + Sync + Debug;

    /// Partition an operation history into independent sub-histories that can
    /// be checked in isolation. Defaults to a single partition.
    fn partition_operations(history: &[Operation<Self>]) -> Vec<Vec<Operation<Self>>> {
        vec![history.to_vec()]
    }


    fn init() -> Self::State;

    /// Checks whether the given operation is acceptable with given the state.
    /// Returns:
    /// (false, _) - When operation was not acceptable
    /// (true, new_state) - When operation was acceptable, also returns new_state.
    fn step(state: &Self::State, op: &Self::Op) -> (bool, Self::State);

    /// State equality. Defaults to `==`.
    fn equal(s1: &Self::State, s2: &Self::State) -> bool {
        s1 == s2
    }

    fn describe_operation(op: &Operation<Self>) -> String {
        format!("{:?}", op.op)
    }

    fn describe_state(state: &Self::State) -> String {
        format!("{:?}", state)
    }

    fn describe_metadata(info: Option<&Self::Metadata>) -> String {
        info.map_or_else(String::new, |i| format!("{:?}", i))
    }
}

// ---------------------------------------------------------------------------
// Operation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Operation<M: Model> {
    pub client_id: Option<u32>,
    pub call_time: i64,
    pub return_time: i64,
    pub op: M::Op,
    pub metadata: Option<M::Metadata>,
}

// ---------------------------------------------------------------------------
// EventModel
// ---------------------------------------------------------------------------
pub trait EventModel: Model {
    type Input: Clone + Send + Sync + Debug;
    type Output: Clone + Send + Sync + Debug;

    /// Partition an operation history into independent sub-histories that can
    /// be checked in isolation. Defaults to a single partition.
    fn partition_events(history: &[Event<Self>]) -> Vec<Vec<Event<Self>>> {
        vec![history.to_vec()]
    }

    /// Combines the Input and Output into an Op
    fn combine(input: &Self::Input, output: &Self::Output) -> Self::Op;
}

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

pub enum Event<M: EventModel> {
    Call {
        client_id: Option<u32>,
        value: M::Input,
        id: usize,
        metadata: Option<M::Metadata>,
    },
    Return {
        client_id: Option<u32>,
        value: M::Output,
        id: usize,
        metadata: Option<M::Metadata>,
    },
}

impl<M: EventModel> Clone for Event<M> {
    fn clone(&self) -> Self {
        match self {
            Event::Call {
                client_id,
                value,
                id,
                metadata,
            } => Event::Call {
                client_id: *client_id,
                value: value.clone(),
                id: *id,
                metadata: metadata.clone(),
            },
            Event::Return {
                client_id,
                value,
                id,
                metadata,
            } => Event::Return {
                client_id: *client_id,
                value: value.clone(),
                id: *id,
                metadata: metadata.clone(),
            },
        }
    }
}

impl<M: EventModel> Debug for Event<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Call {
                client_id,
                value,
                id,
                metadata,
            } => f
                .debug_struct("Call")
                .field("client_id", client_id)
                .field("value", value)
                .field("id", id)
                .field("metadata", metadata)
                .finish(),
            Event::Return {
                client_id,
                value,
                id,
                metadata,
            } => f
                .debug_struct("Return")
                .field("client_id", client_id)
                .field("value", value)
                .field("id", id)
                .field("metadata", metadata)
                .finish(),
        }
    }
}

// ---------------------------------------------------------------------------
// CheckResult
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    Unknown,
    Ok,
    Illegal,
}
