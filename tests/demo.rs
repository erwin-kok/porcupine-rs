use porcupine::{Model, Operation};
use std::{
    fmt::{Debug, Display, Formatter, Result},
    hash::Hash,
};

#[derive(Clone)]
pub struct KVModel;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KVInput {
    Write(String),
    Read,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KVOutput {
    WriteAck,
    ReadResult(Option<String>),
}

impl Display for KVInput {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}
impl Display for KVOutput {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

impl Model for KVModel {
    type State = Option<String>;
    type Input = KVInput;
    type Output = KVOutput;
    type Metadata = ();

    fn init() -> Self::State {
        None
    }

    fn step(
        state: &Self::State,
        input: &Self::Input,
        output: &Self::Output,
    ) -> (bool, Self::State) {
        match input {
            KVInput::Write(v) => (output == &KVOutput::WriteAck, Some(v.clone())),
            KVInput::Read => (
                output == &KVOutput::ReadResult(state.clone()),
                state.clone(),
            ),
        }
    }
}

#[test]
fn test_demo() {
    // ── History 1: linearizable ───────────────────────────────────────────
    //
    //  C0: |── write(x,1) ──|
    //  C1:        |── read→1 ──|
    //  C2:                 |── write(x,2) ──|
    //  C3:                          |── read→2 ──|
    println!("── History 1: should be linearizable");
    let h1 = vec![
        Operation::<KVModel> {
            client_id: Some(0),
            input: KVInput::Write("1".into()),
            call_time: 0,
            output: KVOutput::WriteAck,
            return_time: 40,
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(1),
            input: KVInput::Read,
            call_time: 20,
            output: KVOutput::ReadResult(Some("1".into())),
            return_time: 60,
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(2),
            input: KVInput::Write("2".into()),
            call_time: 50,
            output: KVOutput::WriteAck,
            return_time: 90,
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(3),
            input: KVInput::Read,
            call_time: 80,
            output: KVOutput::ReadResult(Some("2".into())),
            return_time: 120,
            metadata: None,
        },
    ];
    let result = porcupine::check_operations(&h1);
    assert!(result);

    // ── History 2: violation — read sees a value never written ────────────
    println!("\n── History 2: should NOT be linearizable");
    let h2 = vec![
        Operation::<KVModel> {
            client_id: Some(0),
            input: KVInput::Write("1".into()),
            call_time: 0,
            output: KVOutput::WriteAck,
            return_time: 50,
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(1),
            input: KVInput::Read,
            call_time: 0,
            output: KVOutput::ReadResult(Some("2".into())),
            return_time: 80,
            metadata: None,
        },
    ];
    let linearizable = porcupine::check_operations(&h2);
    assert!(!linearizable);

    // ── History 3: two keys, one fine, one violated ───────────────────────
    println!("\n── History 3: 'y' clean, 'z' violated");
    let h3 = vec![
        Operation::<KVModel> {
            client_id: Some(0),
            input: KVInput::Write("a".into()),
            call_time: 0,
            output: KVOutput::WriteAck,
            return_time: 30,
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(1),
            input: KVInput::Read,
            call_time: 20,
            output: KVOutput::ReadResult(Some("a".into())),
            return_time: 50,
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(2),
            input: KVInput::Write("a".into()),
            call_time: 0,
            output: KVOutput::WriteAck,
            return_time: 30,
            metadata: None,
        },
        // "b" was never written — violation
        Operation::<KVModel> {
            client_id: Some(3),
            input: KVInput::Read,
            call_time: 20,
            output: KVOutput::ReadResult(Some("b".into())),
            return_time: 50,
            metadata: None,
        },
    ];
    let linearizable = porcupine::check_operations(&h3);
    assert!(!linearizable);
}
