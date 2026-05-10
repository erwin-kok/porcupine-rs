#[path = "../test_data/jepsen_loader.rs"]
mod jepsen_loader;
use jepsen_loader::load_jepsen_log;

#[path = "../test_data/kv_loader.rs"]
mod kv_loader;

use porcupine_rs::{
    Annotation, CheckResult, LinearizationInfo, Model, Operation,
    visualization::{
        HistoryElement, LinearizationStep, PartitionVisualizationData, compute_visualization_data,
    },
    visualize_path,
};
use rand::{RngExt, distr::Alphanumeric};
use std::{collections::BTreeMap, path::Path};

use crate::{
    common::register_model::{get_call, get_return, put_call, put_return},
    kv_loader::{KvModel, KvOp},
};

mod common;

#[test]
fn test_visualization_multiple_lengths() {
    let o1 = vec![
        read(0, 0, 100, "x", "w"),
        write(1, 5, 10, "x", "y"),
        write(2, 0, 10, "x", "z"),
        read(1, 20, 30, "x", "y"),
        write(1, 35, 45, "x", "w"),
        read(5, 25, 35, "x", "z"),
        read(3, 30, 40, "x", "y"),
        read(4, 50, 90, "y", "a"),
        write(2, 55, 85, "y", "a"),
    ];
    let (result, info) = porcupine_rs::check_operations_info(&o1);
    assert_eq!(
        result,
        CheckResult::Illegal,
        "expected events to not be linearizable"
    );

    let data = compute_visualization_data(&info);
    let mut largest1 = BTreeMap::new();
    largest1.insert(0, 0);
    largest1.insert(1, 0);
    largest1.insert(2, 0);
    largest1.insert(3, 0);
    largest1.insert(4, 0);
    largest1.insert(5, 1);
    largest1.insert(6, 0);
    let mut largest2 = BTreeMap::new();
    largest2.insert(0, 0);
    largest2.insert(1, 0);
    let expected = vec![
        PartitionVisualizationData {
            history: vec![
                history(0, 0, "0", 1300, "100", "get('x') -> 'w'"),
                history(1, 100, "5", 200, "10", "put('x', 'y')"),
                history(2, 0, "0", 200, "10", "put('x', 'z')"),
                history(1, 300, "20", 500, "30", "get('x') -> 'y'"),
                history(1, 600, "35", 800, "45", "put('x', 'w')"),
                history(5, 400, "25", 600, "35", "get('x') -> 'z'"),
                history(3, 500, "30", 700, "40", "get('x') -> 'y'"),
            ],
            partial_linearizations: vec![
                vec![
                    part_lin(2, "z"),
                    part_lin(1, "y"),
                    part_lin(3, "y"),
                    part_lin(6, "y"),
                    part_lin(4, "w"),
                    part_lin(0, "w"),
                ],
                vec![part_lin(1, "y"), part_lin(2, "z"), part_lin(5, "z")],
            ],
            largest: largest1,
        },
        PartitionVisualizationData {
            history: vec![
                history(4, 900, "50", 1200, "90", "get('y') -> 'a'"),
                history(2, 1000, "55", 1100, "85", "put('y', 'a')"),
            ],
            partial_linearizations: vec![vec![part_lin(1, "a"), part_lin(0, "a")]],
            largest: largest2,
        },
    ];

    assert_eq!(
        normalize_partition_data(data.partitions),
        normalize_partition_data(expected),
        "Expected partition data to be equal"
    );
}

#[test]
fn test_register_model_readme() {
    let e1 = vec![
        put_call(0, 0, 100),
        get_call(1, 1),
        get_call(1, 2),
        get_return(2, 2, Some(0)),
        get_return(1, 1, Some(100)),
        put_return(0, 0),
    ];
    let (result, info) = porcupine_rs::check_events_info(&e1);
    assert_eq!(
        result,
        CheckResult::Ok,
        "expected events to be linearizable"
    );
    visualize_temp_file(info);

    let e2 = vec![
        put_call(0, 0, 200),
        get_call(1, 1),
        get_return(1, 1, Some(200)),
        get_call(2, 2),
        get_return(2, 2, Some(0)),
        put_return(0, 0),
    ];
    let (result, info) = porcupine_rs::check_events_info(&e2);
    assert_eq!(
        result,
        CheckResult::Illegal,
        "expected events to not be linearizable"
    );
    visualize_temp_file(info);
}

#[test]
fn test_visualization_large() {
    let events = load_jepsen_log(70);
    let (result, info) = porcupine_rs::check_events_info(&events);
    assert_eq!(
        result,
        CheckResult::Illegal,
        "expected events to not be linearizable"
    );
    visualize_temp_file(info);
}

#[test]
fn test_visualization_annotations() {
    let o1 = vec![
        read(0, 0, 100, "x", "w"),
        write(1, 5, 10, "x", "y"),
        write(2, 0, 10, "x", "z"),
        read(1, 20, 30, "x", "y"),
        write(1, 35, 45, "x", "w"),
        read(5, 25, 35, "x", "z"),
        read(3, 30, 40, "x", "y"),
        read(4, 50, 90, "y", "a"),
        write(2, 55, 85, "y", "a"),
    ];
    let (result, mut info) = porcupine_rs::check_operations_info(&o1);
    let annotations = vec![
        // let's say that there was a "failed get" by client 4 early on
        annotation1(4, 10, Some(31), "get('y') timeout", "#ff9191"),
        // and a failed get by client 5 later
        annotation1(5, 80, None, "get('x') timeout", "#ff9191"),
        // and some tagged annotations
        annotation2(
            "Server 1",
            30,
            None,
            "leader",
            "became leader in term 3 with 2 votes",
            "",
        ),
        annotation2(
            "Server 3",
            10,
            None,
            "duplicate",
            "saw duplicate operation put('x', 'y')",
            "",
        ),
        annotation2("Server 2", 80, None, "restart", "", ""),
        annotation2(
            "Server 3",
            0,
            None,
            "leader",
            "became leader in term 1 with 3 votes",
            "",
        ),
        // and some "test framework" annotations
        annotation2(
            "Test Framework",
            20,
            Some(35),
            "partition [3] [1 2]",
            "",
            "#efaefc",
        ),
        annotation2(
            "Test Framework",
            40,
            Some(100),
            "partition [2] [1 3]",
            "",
            "#efaefc",
        ),
    ];
    info.add_annotations(annotations);
    assert_eq!(
        result,
        CheckResult::Illegal,
        "expected events to not be linearizable"
    );
    visualize_temp_file(info);
}

#[test]
fn test_visualize_point_in_time_annotations_end() {
    let o1 = vec![read(0, 0, 100, "x", "w"), write(1, 50, 60, "x", "y")];
    let (result, mut info) = porcupine_rs::check_operations_info(&o1);
    assert_eq!(
        result,
        CheckResult::Illegal,
        "expected events to not be linearizable"
    );
    let annotations = vec![
        annotation2("Server 1", 30, None, "leader change", "became leader", ""),
        annotation2("Server 2", 50, None, "heartbeat", "", ""),
        // point-in-time annotation at the end
        annotation2("Server 1", 100, None, "shutdown", "", ""),
        annotation2(
            "Test Framework",
            20,
            Some(40),
            "network stable",
            "",
            "#efaefc",
        ),
    ];
    info.add_annotations(annotations);
    visualize_temp_file(info);
}

#[test]
fn test_visualize_matching_start_end() {
    let o1 = vec![read(0, 0, 50, "x", "w"), write(1, 50, 80, "x", "y")];
    let (result, mut info) = porcupine_rs::check_operations_info(&o1);
    assert_eq!(
        result,
        CheckResult::Illegal,
        "expected events to not be linearizable"
    );
    let annotations = vec![
        annotation2("Test Framework", 0, Some(20), "partition", "", ""),
        annotation2("Test Framework", 20, Some(20), "point in time 1", "", ""),
        annotation2("Test Framework", 20, Some(40), "network stable", "", ""),
    ];
    info.add_annotations(annotations);
    visualize_temp_file(info);
}

#[test]
fn test_visualize_annotations_no_events() {
    let mut info = LinearizationInfo::<KvModel>::empty();
    let annotations = vec![
        annotation2(
            "$ Test Info",
            1739938076171778000,
            Some(1739938076171778000),
            "TestPersist33C (3 servers)",
            "",
            "",
        ),
        annotation2(
            "$ Checker",
            1739938076171786000,
            Some(1739938086186709000),
            "agreement of 101 failed",
            "",
            "",
        ),
        annotation2(
            "$ Test Info",
            1739938086187103000,
            Some(1739938086187104000),
            "test failed",
            "",
            "",
        ),
    ];
    info.add_annotations(annotations);
    visualize_temp_file(info);
}

fn visualize_temp_file<M: Model>(info: LinearizationInfo<M>) {
    let name = &format!("./test-{}.html", random_string(8));
    visualize_path(&info, Path::new(&name)).expect("could not write visualization file");
    println!("wrote visualization to {name}")
}

fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn read(client_id: u32, call: i64, ret: i64, key: &str, value: &str) -> Operation<KvModel> {
    Operation {
        client_id: Some(client_id),
        call_time: call,
        return_time: ret,
        op: KvOp::Read {
            key: key.to_string(),
            value: value.to_string(),
        },
        metadata: None,
    }
}

pub fn write(client_id: u32, call: i64, ret: i64, key: &str, value: &str) -> Operation<KvModel> {
    Operation {
        client_id: Some(client_id),
        call_time: call,
        return_time: ret,
        op: KvOp::Write {
            key: key.to_string(),
            value: value.to_string(),
        },
        metadata: None,
    }
}

fn history(
    client_id: u32,
    start: i64,
    original_start: &str,
    end: i64,
    original_end: &str,
    description: &str,
) -> HistoryElement {
    HistoryElement {
        client_id,
        start,
        original_start: original_start.to_string(),
        end,
        original_end: original_end.to_string(),
        description: description.to_string(),
        metadata: "".to_string(),
        annotation: false,
    }
}

fn part_lin(index: usize, state_description: &str) -> LinearizationStep {
    LinearizationStep {
        index,
        state_description: state_description.to_string(),
    }
}

fn annotation1(
    client_id: u32,
    start: i64,
    end: Option<i64>,
    description: &str,
    background_color: &str,
) -> Annotation {
    Annotation {
        client_id: Some(client_id),
        tag: Some("".to_string()),
        start,
        end,
        description: description.to_string(),
        details: "".to_string(),
        text_color: "".to_string(),
        background_color: background_color.to_string(),
    }
}

fn annotation2(
    tag: &str,
    start: i64,
    end: Option<i64>,
    description: &str,
    details: &str,
    background_color: &str,
) -> Annotation {
    Annotation {
        client_id: Some(0),
        tag: Some(tag.to_string()),
        start,
        end,
        description: description.to_string(),
        details: details.to_string(),
        text_color: "".to_string(),
        background_color: background_color.to_string(),
    }
}

fn normalize_partition_data(
    data: Vec<PartitionVisualizationData>,
) -> Vec<PartitionVisualizationData> {
    let mut partitions: Vec<_> = data.iter().map(normalize_partition).collect();
    partitions.sort_by(partition_cmp);
    partitions
}

fn normalize_partition(p: &PartitionVisualizationData) -> PartitionVisualizationData {
    let mut history = p.history.clone();
    let mut partial_linearizations = p.partial_linearizations.clone();

    // Sort nested vecs
    history.sort_by(history_cmp);

    for steps in &mut partial_linearizations {
        steps.sort_by(linearization_step_cmp);
    }

    // Sort outer vec
    partial_linearizations.sort_by(|a, b| {
        let a_key: Vec<_> = a.iter().map(|s| s.index).collect();
        let b_key: Vec<_> = b.iter().map(|s| s.index).collect();
        a_key.cmp(&b_key)
    });

    PartitionVisualizationData {
        history,
        partial_linearizations,
        largest: p.largest.clone(),
    }
}

fn history_cmp(a: &HistoryElement, b: &HistoryElement) -> std::cmp::Ordering {
    (
        a.client_id,
        a.start,
        a.end,
        &a.description,
        &a.metadata,
        a.annotation,
    )
        .cmp(&(
            b.client_id,
            b.start,
            b.end,
            &b.description,
            &b.metadata,
            b.annotation,
        ))
}

fn linearization_step_cmp(a: &LinearizationStep, b: &LinearizationStep) -> std::cmp::Ordering {
    (a.index, &a.state_description).cmp(&(b.index, &b.state_description))
}

fn partition_cmp(
    a: &PartitionVisualizationData,
    b: &PartitionVisualizationData,
) -> std::cmp::Ordering {
    (&a.history, &a.partial_linearizations, &a.largest).cmp(&(
        &b.history,
        &b.partial_linearizations,
        &b.largest,
    ))
}
