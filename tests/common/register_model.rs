#![allow(dead_code)]

use porcupine_rs::{Event, EventModel, Model, Operation};
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
    type Metadata = ();

    fn init() -> u32 {
        0
    }

    fn step(state: &u32, op: &RegisterOp) -> (bool, u32) {
        match op {
            RegisterOp::Get(value) => (*value == Some(*state), *state),
            RegisterOp::Put(value) => (true, *value),
        }
    }

    fn describe_operation(op: &RegisterOp) -> String {
        match op {
            RegisterOp::Get(value) => {
                let v = value.unwrap_or_default();
                format!("get() -> '{v}'").to_string()
            }
            RegisterOp::Put(value) => format!("put('{value}')").to_string(),
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

pub fn put(client_id: u32, call: i64, ret: i64, v: u32) -> Operation<RegisterModel> {
    Operation {
        client_id: Some(client_id),
        call_time: call,
        return_time: ret,
        op: RegisterOp::Put(v),
        metadata: None,
    }
}

pub fn get(client_id: u32, call: i64, ret: i64, v: Option<u32>) -> Operation<RegisterModel> {
    Operation {
        client_id: Some(client_id),
        call_time: call,
        return_time: ret,
        op: RegisterOp::Get(v),
        metadata: None,
    }
}

pub fn put_call(client_id: u32, id: usize, v: u32) -> Event<RegisterModel> {
    Event::Call {
        client_id: Some(client_id),
        value: RegisterInput::Put(v),
        id,
        metadata: None,
    }
}

pub fn put_return(client_id: u32, id: usize) -> Event<RegisterModel> {
    Event::Return {
        client_id: Some(client_id),
        value: RegisterOutput::Put,
        id,
        metadata: None,
    }
}

pub fn get_call(client_id: u32, id: usize) -> Event<RegisterModel> {
    Event::Call {
        client_id: Some(client_id),
        value: RegisterInput::Get,
        id,
        metadata: None,
    }
}

pub fn get_return(client_id: u32, id: usize, v: Option<u32>) -> Event<RegisterModel> {
    Event::Return {
        client_id: Some(client_id),
        value: RegisterOutput::Get(v),
        id,
        metadata: None,
    }
}
