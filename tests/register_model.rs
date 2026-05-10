use crate::common::register_model::{get, get_call, get_return, put, put_call, put_return};

mod common;

#[test]
fn test_register_model() {
    let o1 = vec![
        put(0, 0, 100, 100),
        get(1, 25, 75, Some(100)),
        get(2, 30, 60, Some(0)),
    ];
    let linearizable = porcupine_rs::check_operations(&o1);
    assert!(linearizable, "expected operations to be linearizable");

    let e1 = vec![
        put_call(0, 0, 100),
        get_call(1, 1),
        get_call(1, 2),
        get_return(2, 2, Some(0)),
        get_return(1, 1, Some(100)),
        put_return(0, 0),
    ];
    let linearizable = porcupine_rs::check_events(&e1);
    assert!(linearizable, "expected events to be linearizable");

    let o2 = vec![
        put(0, 0, 100, 200),
        get(1, 10, 30, Some(200)),
        get(2, 40, 90, Some(0)),
    ];
    let linearizable = porcupine_rs::check_operations(&o2);
    assert!(!linearizable, "expected operations to not be linearizable");

    let e2 = vec![
        put_call(0, 0, 200),
        get_call(1, 1),
        get_return(1, 1, Some(200)),
        get_call(2, 2),
        get_return(2, 2, Some(0)),
        put_return(0, 0),
    ];
    let linearizable = porcupine_rs::check_events(&e2);
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
    let linearizable = porcupine_rs::check_operations(&o1);
    assert!(linearizable, "expected operations to be linearizable");

    let o2 = vec![
        put(0, 0, 100, 200),
        get(1, 10, 10, Some(200)),
        get(2, 10, 10, Some(200)),
        get(3, 40, 90, Some(0)),
    ];
    let linearizable = porcupine_rs::check_operations(&o2);
    assert!(!linearizable, "expected operations to not be linearizable");
}
