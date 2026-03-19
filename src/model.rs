use std::fmt::Debug;
use std::hash::Hash;

pub trait Model: Clone + Send + Sync {
    type State: Clone + Send + Sync + Debug + Hash + Eq;
    type Input: Clone + Send + Sync + Debug;
    type Output: Clone + PartialEq + Send + Sync + Debug;
    type Metadata: Clone + Send + Sync + Debug;

    fn partition(history: &[Operation<Self>]) -> Vec<Vec<Operation<Self>>> {
        vec![history.to_vec()]
    }

    fn partition_event(history: &[Event<Self>]) -> Vec<Vec<Event<Self>>> {
        vec![history.to_vec()]
    }

    fn init() -> Self::State;

    fn step(state: &Self::State, input: &Self::Input, output: &Self::Output)
    -> (bool, Self::State);

    fn equal(s1: &Self::State, s2: &Self::State) -> bool {
        s1 == s2
    }

    fn describe_operation(input: &Self::Input, output: &Self::Output) -> String {
        format!("{:?} -> {:?}", input, output)
    }

    fn describe_state(state: &Self::State) -> String {
        format!("{:?}", state)
    }

    fn describe_metadata(info: Option<&Self::Metadata>) -> String {
        info.map_or_else(String::new, |i| format!("{:?}", i))
    }
}

#[derive(Debug, Clone)]
pub struct Operation<M: Model> {
    pub client_id: Option<u32>,
    pub input: M::Input,
    pub call: i64,
    pub output: M::Output,
    pub return_time: i64,
    pub metadata: Option<M::Metadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventKind {
    Call,
    Return,
}

pub enum EventValue<M: Model> {
    Call(M::Input),
    Return(M::Output),
}

impl<M: Model> Clone for EventValue<M> {
    fn clone(&self) -> Self {
        match self {
            EventValue::Call(v) => EventValue::Call(v.clone()),
            EventValue::Return(v) => EventValue::Return(v.clone()),
        }
    }
}

impl<M: Model> Debug for EventValue<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventValue::Call(v) => write!(f, "Call({:?})", v),
            EventValue::Return(v) => write!(f, "Return({:?})", v),
        }
    }
}

pub struct Event<M: Model> {
    pub client_id: Option<u32>,
    pub kind: EventKind,
    pub value: EventValue<M>,
    pub id: usize,
    pub metadata: Option<M::Metadata>,
}

impl<M: Model> Clone for Event<M> {
    fn clone(&self) -> Self {
        Self {
            client_id: self.client_id,
            kind: self.kind,
            value: self.value.clone(),
            id: self.id,
            metadata: self.metadata.clone(),
        }
    }
}

impl<M: Model> Debug for Event<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event")
            .field("client_id", &self.client_id)
            .field("kind", &self.kind)
            .field("value", &self.value)
            .field("id", &self.id)
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    Unknown,
    Ok,
    Illegal,
}
