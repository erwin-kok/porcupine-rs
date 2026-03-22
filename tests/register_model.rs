use porcupine::{Event, Model, Operation};
use std::{
    fmt::{Debug, Display, Formatter, Result},
    hash::Hash,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RegisterInput {
    Put(u32),
    Get,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisterOutput {
    PutAck,
    GetResult(Option<u32>),
}

impl Display for RegisterInput {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}
impl Display for RegisterOutput {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct RegisterModel {}

impl Model for RegisterModel {
    type State = u32;
    type Input = RegisterInput;
    type Output = RegisterOutput;
    type Metadata = u32;

    fn init() -> u32 {
        0
    }

    fn step(state: &u32, input: &RegisterInput, output: &RegisterOutput) -> (bool, u32) {
        match input {
            RegisterInput::Put(v) => (output == &RegisterOutput::PutAck, *v),
            RegisterInput::Get => (output == &RegisterOutput::GetResult(Some(*state)), *state), // state is unchanged
        }
    }
}

fn put(client_id: u32, call: i64, ret: i64, v: u32) -> Operation<RegisterModel> {
    Operation {
        client_id: Some(client_id),
        input: RegisterInput::Put(v),
        call_time: call,
        output: RegisterOutput::PutAck,
        return_time: ret,
        metadata: None,
    }
}

fn get(client_id: u32, call: i64, ret: i64, v: Option<u32>) -> Operation<RegisterModel> {
    Operation {
        client_id: Some(client_id),
        input: RegisterInput::Get,
        call_time: call,
        output: RegisterOutput::GetResult(v),
        return_time: ret,
        metadata: None,
    }
}

fn put_call(client_id: u32, id: usize, v: u32) -> Event<RegisterModel> {
    Event::Call {
        client_id: Some(client_id),
        value: RegisterInput::Put(v),
        id,
        metadata: None,
    }
}

fn put_return(client_id: u32, id: usize) -> Event<RegisterModel> {
    Event::Return {
        client_id: Some(client_id),
        value: RegisterOutput::PutAck,
        id,
        metadata: None,
    }
}

fn get_call(client_id: u32, id: usize) -> Event<RegisterModel> {
    Event::Call {
        client_id: Some(client_id),
        value: RegisterInput::Get,
        id,
        metadata: None,
    }
}

fn get_return(client_id: u32, id: usize, v: Option<u32>) -> Event<RegisterModel> {
    Event::Return {
        client_id: Some(client_id),
        value: RegisterOutput::GetResult(v),
        id,
        metadata: None,
    }
}

#[test]
fn test_register_model() {
    let o1 = vec![
        put(0, 0, 100, 100),
        get(1, 25, 75, Some(100)),
        get(2, 30, 60, Some(0)),
    ];
    let linearizable = porcupine::check_operations(&o1);
    assert!(linearizable, "expected operations to be linearizable");

    let e1 = vec![
        put_call(0, 0, 100),
        get_call(1, 1),
        get_call(1, 2),
        get_return(2, 2, Some(0)),
        get_return(1, 1, Some(100)),
        put_return(0, 0),
    ];
    let linearizable = porcupine::check_events(&e1);
    assert!(linearizable, "expected events to be linearizable");

    let o2 = vec![
        put(0, 0, 100, 200),
        get(1, 10, 30, Some(200)),
        get(2, 40, 90, Some(0)),
    ];
    let linearizable = porcupine::check_operations(&o2);
    assert!(!linearizable, "expected operations to not be linearizable");

    let e2 = vec![
        put_call(0, 0, 200),
        get_call(1, 1),
        get_return(1, 1, Some(200)),
        get_call(2, 2),
        get_return(2, 2, Some(0)),
        put_return(0, 0),
    ];
    let linearizable = porcupine::check_events(&e2);
    assert!(!linearizable, "expected events to not be linearizable");
}

#[test]
fn test_zero_duration() {
    let o1 = vec![
        put(0, 0, 100, 100),
        get(1, 25, 75, Some(100)),
        get(2, 30, 30, Some(0)),
        get(3, 30, 30, Some(0)),
    ];
    let linearizable = porcupine::check_operations(&o1);
    assert!(linearizable, "expected operations to be linearizable");

    let o2 = vec![
        put(0, 0, 100, 200),
        get(1, 10, 10, Some(200)),
        get(2, 10, 10, Some(200)),
        get(3, 40, 90, Some(0)),
    ];
    let linearizable = porcupine::check_operations(&o2);
    assert!(!linearizable, "expected operations to not be linearizable");
}
