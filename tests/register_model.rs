use porcupine::{Event, EventModel, Model, Operation};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum RegisterOp {
    Get(Option<u32>),
    Put(u32),
}

#[derive(Debug, Clone)]
pub struct RegisterModel;

impl Model for RegisterModel {
    type State = u32;
    type Op = RegisterOp;
    type Metadata = u32;

    fn init() -> u32 {
        0
    }

    fn step(state: &u32, op: &RegisterOp) -> (bool, u32) {
        match op {
            RegisterOp::Get(value) => (*value == Some(*state), *state),
            RegisterOp::Put(value) => (true, *value),
        }
    }
}

#[derive(Clone, Debug)]
pub enum RegisterInput {
    Get,
    Put(u32),
}

#[derive(Clone, Debug)]
pub enum RegisterOutput {
    Get(Option<u32>),
    Put,
}

impl EventModel for RegisterModel {
    type Input = RegisterInput;
    type Output = RegisterOutput;

    fn combine(input: &RegisterInput, output: &RegisterOutput) -> RegisterOp {
        match (input, output) {
            (RegisterInput::Get, RegisterOutput::Get(value)) => RegisterOp::Get(*value),
            (RegisterInput::Put(value), RegisterOutput::Put) => RegisterOp::Put(*value),
            _ => panic!("unexpected input/output combination found"),
        }
    }
}

fn put(client_id: u32, call: i64, ret: i64, v: u32) -> Operation<RegisterModel> {
    Operation {
        client_id: Some(client_id),
        call_time: call,
        return_time: ret,
        op: RegisterOp::Put(v),
        metadata: None,
    }
}

fn get(client_id: u32, call: i64, ret: i64, v: Option<u32>) -> Operation<RegisterModel> {
    Operation {
        client_id: Some(client_id),
        call_time: call,
        return_time: ret,
        op: RegisterOp::Get(v),
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
        value: RegisterOutput::Put,
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
        value: RegisterOutput::Get(v),
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
