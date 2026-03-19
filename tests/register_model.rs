use porcupine::{Model, Operation};
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
            RegisterInput::Get => (output == &RegisterOutput::GetResult(Some(*state)), *state),
        }
    }
}

#[test]
fn test_register_model() {
    let h1 = vec![
        Operation::<RegisterModel> {
            client_id: Some(0),
            input: RegisterInput::Put(100),
            call: 0,
            output: RegisterOutput::PutAck,
            return_time: 100,
            metadata: None,
        },
        Operation::<RegisterModel> {
            client_id: Some(1),
            input: RegisterInput::Get,
            call: 25,
            output: RegisterOutput::GetResult(Some(100)),
            return_time: 75,
            metadata: None,
        },
        Operation::<RegisterModel> {
            client_id: Some(2),
            input: RegisterInput::Get,
            call: 30,
            output: RegisterOutput::GetResult(Some(0)),
            return_time: 60,
            metadata: None,
        },
    ];
    let linearizable = porcupine::check_operations(&h1);
    assert!(linearizable, "expected operations to be linearizable");

    let h2 = vec![
        Operation::<RegisterModel> {
            client_id: Some(0),
            input: RegisterInput::Put(200),
            call: 0,
            output: RegisterOutput::PutAck,
            return_time: 100,
            metadata: None,
        },
        Operation::<RegisterModel> {
            client_id: Some(1),
            input: RegisterInput::Get,
            call: 10,
            output: RegisterOutput::GetResult(Some(200)),
            return_time: 30,
            metadata: None,
        },
        Operation::<RegisterModel> {
            client_id: Some(2),
            input: RegisterInput::Get,
            call: 40,
            output: RegisterOutput::GetResult(Some(0)),
            return_time: 90,
            metadata: None,
        },
    ];
    let linearizable = porcupine::check_operations(&h2);
    assert!(!linearizable, "expected operations to not be linearizable");
}
