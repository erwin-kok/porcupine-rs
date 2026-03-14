use std::fmt::Debug;

/// Event kinds for the Event struct
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Call,
    Return,
}

/// Represents an operation in the history
#[derive(Debug, Clone)]
pub struct Operation<I: Clone, O: Clone, M: Clone> {
    /// Optional client identifier for visualization.
    pub client_id: Option<usize>,
    /// The input for the operation.
    pub input: I,
    /// Invocation timestamp.
    pub call: i64,
    /// The output resulting from the operation.
    pub output: O,
    /// Response timestamp.
    pub return_time: i64,
    /// Optional arbitrary metadata for visualization.
    pub metadata: Option<M>,
}

impl<I: Clone, O: Clone, M: Clone> Operation<I, O, M> {
    /// Create a new operation
    pub fn new(client_id: usize, input: I, call: i64, output: O, return_time: i64) -> Self {
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

/// Represents an event (call or return)
#[derive(Debug, Clone)]
pub struct Event<V, M> {
    pub client_id: Option<usize>, // optional, for visualization
    pub kind: EventKind,
    pub value: V,
    pub id: usize,
    pub metadata: Option<M>, // metadata (ReturnEvent takes precedence if both have it)
}

impl<V, M> Event<V, M> {
    /// Create a new event
    pub fn new(client_id: Option<usize>, kind: EventKind, value: V, id: usize) -> Self {
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
    type Input: Clone + Debug;
    type Output: Clone + Debug;
    type Metadata: Clone;
    type Value: Clone;

    /// Partition function: splits history into independent partitions
    fn partition(
        history: &[Operation<Self::Input, Self::Output, Self::Metadata>],
    ) -> Vec<Vec<Operation<Self::Input, Self::Output, Self::Metadata>>> {
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
    fn step(state: &Self::State, input: &Self::Input, output: &Self::Output)
    -> (bool, Self::State);

    /// State equality checker (optional, defaults to PartialEq)
    fn equal(state1: &Self::State, state2: &Self::State) -> bool {
        state1 == state2
    }

    /// Operation description for visualization
    fn describe_operation(input: &Self::Input, output: &Self::Output) -> String {
        format!("{:?} -> {:?}", input, output)
    }

    /// State description for visualization
    fn describe_state(state: &Self::State) -> String {
        format!("{:?}", state)
    }

    /// Metadata description for visualization
    fn describe_metadata(info: Option<&Self::Input>) -> String {
        info.map_or_else(String::new, |i| format!("{:?}", i))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    Unknown,
    Ok,
    Illegal,
}

pub trait NondeterministicModel<
    S: Clone = (),
    I: Clone = (),
    O: Clone = (),
    M: Clone = (),
    V: Clone = (),
>
{
    /// Partition function: splits history into independent partitions
    fn partition(history: &[Operation<I, O, M>]) -> Vec<Vec<Operation<I, O, M>>>;

    /// Partition function for events (alternative to Operation partitioning)
    fn partition_event(history: &[Event<V, M>]) -> Vec<Vec<Event<V, M>>>;

    /// Initial state generator
    fn init() -> S;

    /// Step function: (state, input, output) -> (success, new_state)
    fn step(state: &S, input: &I, output: &O) -> Vec<S>;

    /// State equality checker (optional, defaults to PartialEq)
    fn equal(state1: &S, state2: &S) -> bool;

    /// Operation description for visualization
    fn describe_operation(input: &I, output: &O) -> String;

    /// State description for visualization
    fn describe_state(state: &S) -> String;

    /// Metadata description for visualization
    fn describe_metadata(info: &I) -> String;
}
