#[path = "../test_data/jepsen_loader.rs"]
mod jepsen_loader;
use jepsen_loader::load_jepsen_log;

use criterion::{Criterion, criterion_group, criterion_main};

fn check_jepsen(log_num: u32, correct: bool) {
    let events = load_jepsen_log(log_num);
    let res = porcupine_rs::check_events(&events);
    assert_eq!(correct, res, "expected output {correct}, got output {res}")
}

fn benchmark_check_jepsen(c: &mut Criterion) {
    let test_cases = vec![(0, false), (1, false), (2, true), (3, false), (4, false)];

    for (log_num, expected) in test_cases {
        c.bench_function(&format!("check_jepsen_log_{}", log_num), |b| {
            b.iter(|| {
                check_jepsen(log_num, expected);
            });
        });
    }
}

criterion_group!(benches, benchmark_check_jepsen);
criterion_main!(benches);
