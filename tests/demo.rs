use porcupine::{Model, Operation};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum KVOp {
    Write(String),
    Read(Option<String>),
}

#[derive(Clone)]
pub struct KVModel;

impl Model for KVModel {
    type State = Option<String>;
    type Op = KVOp;
    type Metadata = ();

    fn init() -> Self::State {
        None
    }

    fn step(state: &Option<String>, op: &KVOp) -> (bool, Option<String>) {
        match op {
            KVOp::Write(value) => (true, Some(value.clone())),
            KVOp::Read(value) => (*value == *state, state.clone()),
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
            call_time: 0,
            return_time: 40,
            op: KVOp::Write("1".into()),
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(1),
            call_time: 20,
            return_time: 60,
            op: KVOp::Read(Some("1".into())),
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(2),
            call_time: 50,
            return_time: 90,
            op: KVOp::Write("2".into()),
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(3),
            call_time: 80,
            return_time: 120,
            op: KVOp::Read(Some("2".into())),
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
            call_time: 0,
            return_time: 50,
            op: KVOp::Write("1".into()),
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(1),
            call_time: 0,
            return_time: 80,
            op: KVOp::Read(Some("2".into())),
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
            call_time: 0,
            return_time: 30,
            op: KVOp::Write("a".into()),
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(1),
            call_time: 20,
            return_time: 50,
            op: KVOp::Read(Some("a".into())),
            metadata: None,
        },
        Operation::<KVModel> {
            client_id: Some(2),
            call_time: 0,
            return_time: 30,
            op: KVOp::Write("a".into()),
            metadata: None,
        },
        // "b" was never written — violation
        Operation::<KVModel> {
            client_id: Some(3),
            call_time: 20,
            return_time: 50,
            op: KVOp::Read(Some("b".into())),
            metadata: None,
        },
    ];
    let linearizable = porcupine::check_operations(&h3);
    assert!(!linearizable);
}
