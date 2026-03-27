use porcupine_rs::{Event, EventModel, Model};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum SetOp {
    Read(Vec<u32>, bool),
    Write(u32),
}

#[derive(Debug, Clone)]
pub struct SetModel;

impl Model for SetModel {
    type State = Vec<u32>;
    type Op = SetOp;
    type Metadata = u32;

    fn init() -> Vec<u32> {
        Vec::new()
    }

    fn step(state: &Vec<u32>, op: &SetOp) -> (bool, Vec<u32>) {
        match op {
            SetOp::Read(items, exists) => {
                let mut next_state = items.clone();
                next_state.sort();
                (*exists || state == items, next_state)
            }
            SetOp::Write(value) => {
                if !state.contains(value) {
                    let mut next_state = state.clone();
                    next_state.push(*value);
                    next_state.sort();
                    (true, next_state)
                } else {
                    (true, state.clone())
                }
            }
        }
    }

    fn equal(s1: &Vec<u32>, s2: &Self::State) -> bool {
        s1 == s2
    }
}

#[derive(Clone, Debug)]
pub enum SetInput {
    Read,
    Write(u32),
}

#[derive(Clone, Debug)]
pub enum SetOutput {
    Read(Vec<u32>, bool),
    Write,
}

impl EventModel for SetModel {
    type Input = SetInput;
    type Output = SetOutput;

    fn combine(input: &SetInput, output: &SetOutput) -> SetOp {
        match (input, output) {
            (SetInput::Read, SetOutput::Read(items, exists)) => SetOp::Read(items.clone(), *exists),
            (SetInput::Write(value), SetOutput::Write) => SetOp::Write(*value),
            _ => panic!("unexpected input/output combination found"),
        }
    }
}

fn write_call(client_id: u32, id: usize, v: u32) -> Event<SetModel> {
    Event::Call {
        client_id: Some(client_id),
        value: SetInput::Write(v),
        id,
        metadata: None,
    }
}

fn write_return(client_id: u32, id: usize) -> Event<SetModel> {
    Event::Return {
        client_id: Some(client_id),
        value: SetOutput::Write,
        id,
        metadata: None,
    }
}

fn read_call(client_id: u32, id: usize) -> Event<SetModel> {
    Event::Call {
        client_id: Some(client_id),
        value: SetInput::Read,
        id,
        metadata: None,
    }
}

fn read_return(client_id: u32, id: usize, v: Vec<u32>, u: bool) -> Event<SetModel> {
    Event::Return {
        client_id: Some(client_id),
        value: SetOutput::Read(v, u),
        id,
        metadata: None,
    }
}

#[test]
fn test_set_model() {
    let o1 = vec![
        write_call(0, 0, 100),
        write_call(1, 1, 0),
        read_call(2, 2),
        read_return(2, 2, vec![100], false),
        write_return(1, 1),
        write_return(0, 0),
    ];
    let linearizable = porcupine_rs::check_events(&o1);
    assert!(linearizable, "expected operations to be linearizable");

    let o2 = vec![
        write_call(0, 0, 100),
        write_call(1, 1, 110),
        read_call(2, 2),
        read_return(2, 2, vec![100, 110], false),
        write_return(1, 1),
        write_return(0, 0),
    ];
    let linearizable = porcupine_rs::check_events(&o2);
    assert!(linearizable, "expected operations to be linearizable");

    let o3 = vec![
        write_call(0, 0, 100),
        write_call(1, 1, 110),
        read_call(2, 2),
        read_return(2, 2, vec![], true),
        write_return(1, 1),
        write_return(0, 0),
    ];
    let linearizable = porcupine_rs::check_events(&o3);
    assert!(linearizable, "expected operations to be linearizable");

    let o4 = vec![
        write_call(0, 0, 100),
        write_call(1, 1, 110),
        read_call(2, 2),
        read_return(2, 2, vec![100, 100, 110], false),
        write_return(1, 1),
        write_return(0, 0),
    ];
    let linearizable = porcupine_rs::check_events(&o4);
    assert!(!linearizable, "expected operations not to be linearizable");
}
