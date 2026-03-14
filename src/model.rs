use std::fmt::Debug;

/// Represents an operation in the history
#[derive(Debug, Clone)]
pub struct Operation<V: Clone, M: Clone> {
    /// Optional client identifier for visualization.
    pub client_id: Option<u32>,
    /// The input for the operation.
    pub input: V,
    /// Invocation timestamp.
    pub call: i64,
    /// The output resulting from the operation.
    pub output: V,
    /// Response timestamp.
    pub return_time: i64,
    /// Optional arbitrary metadata for visualization.
    pub metadata: Option<M>,
}

impl<V: Clone, M: Clone> Operation<V, M> {
    /// Create a new operation
    pub fn new(client_id: u32, input: V, call: i64, output: V, return_time: i64) -> Self {
        Self {
            client_id: Some(client_id),
            input,
            call,
            output,
            return_time,
            metadata: None,
        }
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: M) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Calculate duration
    pub fn duration(&self) -> i64 {
        self.return_time - self.call
    }
}

/// Event kinds for the Event struct
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Call,
    Return,
}

/// Represents an event (call or return)
#[derive(Debug, Clone)]
pub struct Event<V: Clone, M: Clone> {
    /// Optional client identifier for visualization.
    pub client_id: Option<u32>,
    /// Kind of the event
    pub kind: EventKind,
    /// Value of the event
    pub value: V,
    /// Used to match a function call event with its corresponding return event
    pub id: usize,
    /// Optional arbitrary metadata for visualization.
    pub metadata: Option<M>,
}

impl<V: Clone, M: Clone> Event<V, M> {
    /// Create a new event
    pub fn new(client_id: Option<u32>, kind: EventKind, value: V, id: usize) -> Self {
        Self {
            client_id,
            kind,
            value,
            id,
            metadata: None,
        }
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: M) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Check if this is a call event
    pub fn is_call(&self) -> bool {
        matches!(self.kind, EventKind::Call)
    }

    /// Check if this is a return event
    pub fn is_return(&self) -> bool {
        matches!(self.kind, EventKind::Return)
    }
}

/// The core Model struct
pub trait Model {
    type State: Eq + PartialEq + Debug;
    type Value: Clone + Debug;
    type Metadata: Clone;

    /// Partition function: splits history into independent partitions
    fn partition(
        history: &[Operation<Self::Value, Self::Metadata>],
    ) -> Vec<Vec<Operation<Self::Value, Self::Metadata>>> {
        vec![history.to_vec()]
    }

    /// Partition function for events (alternative to Operation partitioning)
    fn partition_event(
        history: &[Event<Self::Value, Self::Metadata>],
    ) -> Vec<Vec<Event<Self::Value, Self::Metadata>>> {
        vec![history.to_vec()]
    }

    /// Initial state generator
    fn init() -> Self::State;

    /// Step function: (state, input, output) -> (success, new_state)
    fn step(state: &Self::State, input: &Self::Value, output: &Self::Value) -> (bool, Self::State);

    /// State equality checker (optional, defaults to PartialEq)
    fn equal(state1: &Self::State, state2: &Self::State) -> bool {
        state1 == state2
    }

    /// Operation description for visualization
    fn describe_operation(input: &Self::Value, output: &Self::Value) -> String {
        format!("{:?} -> {:?}", input, output)
    }

    /// State description for visualization
    fn describe_state(state: &Self::State) -> String {
        format!("{:?}", state)
    }

    /// Metadata description for visualization
    fn describe_metadata(info: Option<&Self::Value>) -> String {
        info.map_or_else(String::new, |i| format!("{:?}", i))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    Unknown,
    Ok,
    Illegal,
}

pub trait NondeterministicModel {
    type State: Eq + PartialEq + Debug;
    type Value: Clone + Debug;
    type Metadata: Clone;

    /// Partition function: splits history into independent partitions
    fn partition(
        history: &[Operation<Self::Value, Self::Metadata>],
    ) -> Vec<Vec<Operation<Self::Value, Self::Metadata>>>;

    /// Partition function for events (alternative to Operation partitioning)
    fn partition_event(
        history: &[Event<Self::Value, Self::Metadata>],
    ) -> Vec<Vec<Event<Self::Value, Self::Metadata>>>;

    /// Initial state generator
    fn init() -> Self::State;

    /// Step function: (state, input, output) -> (success, new_state)
    fn step(state: &Self::State, input: &Self::Value, output: &Self::Value) -> Vec<Self::State>;

    /// State equality checker (optional, defaults to PartialEq)
    fn equal(state1: &Self::State, state2: &Self::State) -> bool;

    /// Operation description for visualization
    fn describe_operation(input: &Self::Value, output: &Self::Value) -> String;

    /// State description for visualization
    fn describe_state(state: &Self::State) -> String;

    /// Metadata description for visualization
    fn describe_metadata(info: &Self::Value) -> String;
}
