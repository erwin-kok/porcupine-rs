#[path = "../test_data/kv_loader.rs"]
mod kv_loader;
use kv_loader::load_kv_log_no_part;

use criterion::{Criterion, criterion_group, criterion_main};

fn benchmark_kv_log(c: &mut Criterion) {
    c.bench_function("check_kv_log_c10-ok", |b| {
        b.iter(|| {
            let events = load_kv_log_no_part("c10-ok");
            porcupine_rs::check_events(&events);
        });
    });
}

criterion_group!(benches, benchmark_kv_log);
criterion_main!(benches);
