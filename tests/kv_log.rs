#[path = "../test_data/kv_loader.rs"]
mod kv_loader;
use kv_loader::{load_kv_log, load_kv_log_no_part};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn check_kv(log_name: &str, expected: bool) {
    let events = load_kv_log(log_name);
    let ok = porcupine_rs::check_events(&events);
    assert_eq!(
        expected, ok,
        "log={log_name}: expected {expected}, got {ok}"
    );
}

fn check_kv_no_part(log_name: &str, expected: bool) {
    let events = load_kv_log_no_part(log_name);
    let ok = porcupine_rs::check_events(&events);
    assert_eq!(
        expected, ok,
        "log={log_name}: expected {expected}, got {ok}"
    );
}

#[test]
fn test_kv_1_client_ok() {
    check_kv("c01-ok", true)
}

#[test]
fn test_kv_1_client_bad() {
    check_kv("c01-bad", false)
}

#[test]
fn test_kv_10_clients_ok() {
    check_kv("c10-ok", true)
}

#[test]
fn test_kv_10_clients_bad() {
    check_kv("c10-bad", false)
}

#[test]
fn test_kv_50_clients_ok() {
    check_kv("c50-ok", true)
}

#[test]
fn test_kv_50_clients_bad() {
    check_kv("c50-bad", false)
}

#[test]
fn test_kv_no_partition_1_client_ok() {
    check_kv_no_part("c01-ok", true)
}

#[test]
fn test_kv_no_partition_1_client_bad() {
    check_kv_no_part("c01-bad", false)
}

#[test]
fn test_kv_no_partition_10_clients_ok() {
    check_kv_no_part("c10-ok", true)
}

#[test]
fn test_kv_no_partition_10_clients_bad() {
    check_kv_no_part("c10-bad", false)
}
