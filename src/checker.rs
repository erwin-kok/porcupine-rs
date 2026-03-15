use crate::model::{CheckResult, Entry, EntryKind, Event, EventKind, Model, Operation};
use crossbeam::channel;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::{
    cmp::Ordering,
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct LinearizationInfo<V, M> {
    history: Vec<Vec<Entry<V, M>>>,
    partial_linearizations: Vec<Vec<Vec<i32>>>,
}

pub(crate) fn check_events<M: Model>(
    history: &[Event<M::Value, M::Metadata>],
    verbose: bool,
    timeout: Duration,
) -> (CheckResult, LinearizationInfo<M::Value, M::Metadata>) {
    let partitions = M::partition_event(history);
    let mut l = Vec::with_capacity(partitions.len());
    for subhistory in partitions {
        l.push(convert_entries::<M>(renumber::<M>(subhistory)));
    }
    check_parallel::<M>(l, verbose, timeout)
}

pub(crate) fn check_operations<M: Model>(
    history: &[Operation<M::Value, M::Metadata>],
    verbose: bool,
    timeout: Duration,
) -> (CheckResult, LinearizationInfo<M::Value, M::Metadata>) {
    let partitions = M::partition(history);
    let mut l = Vec::with_capacity(partitions.len());
    for subhistory in partitions {
        l.push(make_entries::<M>(subhistory));
    }
    check_parallel::<M>(l, verbose, timeout)
}

fn compute<V: Clone, M: Clone>(h: &[Entry<V, M>]) -> (bool, Vec<Vec<u32>>) {
    (true, vec![])
}

fn check_parallel<M: Model>(
    history: Vec<Vec<Entry<M::Value, M::Metadata>>>,
    compute_info: bool,
    timeout: Duration,
) -> (CheckResult, LinearizationInfo<M::Value, M::Metadata>) {
    if history.is_empty() {
        return (
            CheckResult::Ok,
            LinearizationInfo {
                history: vec![],
                partial_linearizations: vec![],
            },
        );
    }
    let ok = Arc::new(Mutex::new(true));
    let timed_out = Arc::new(Mutex::new(false));
    let (sender, receiver) = channel::<bool>();
    let longest = Arc::new(Mutex::new(vec![Vec::new(); history.len()]));
    let kill = Arc::new(AtomicBool::new(false));
    let start_time = Instant::now();

    let x: Vec<(bool, Vec<Vec<u32>>)> = history.par_iter().map(|h| compute(h)).collect();

    for (i, subhistory) in history.iter().enumerate() {
        let sender = sender.clone();
        let longest = longest.clone();
        let kill = kill.clone();
        let subhistory = subhistory.to_vec();
        thread::spawn(move || {
            let (result, l) = check_single::<M>(&subhistory, compute_info, &kill);
            if let Ok(mut longest_guard) = longest.lock() {
                longest_guard[i] = l; // Store results
            }
            sender.send(result).unwrap(); // Send the result to the channel
        });
    }
    let mut count = 0;
    loop {
        if let Ok(result) = receiver.recv_timeout(timeout) {
            count += 1;
            if !result {
                *ok.lock().unwrap() = false; // Update ok status
                if !compute_info {
                    kill.store(true, atomic::Ordering::SeqCst); // Signal to stop 
                    break;
                }
            }

            if count >= history.len() {
                break;
            }
        }

        if Instant::now().duration_since(start_time) >= timeout {
            *timed_out.lock().unwrap() = true; // Set the timed_out flag
            kill.store(true, atomic::Ordering::SeqCst); // Signal to stop
            break;
        }
    }

    (
        CheckResult::Ok,
        LinearizationInfo {
            history: vec![],
            partial_linearizations: vec![],
        },
    )
}

fn check_single<M: Model>(
    history: &[Entry<M::Value, M::Metadata>],
    compute_partial: bool,
    _kill: &Arc<AtomicBool>,
) -> (bool, Vec<Vec<u32>>) {
    todo!()
}

fn make_entries<M: Model>(
    history: Vec<Operation<M::Value, M::Metadata>>,
) -> Vec<Entry<M::Value, M::Metadata>> {
    let mut entries = Vec::new();
    for (i, elem) in history.into_iter().enumerate() {
        entries.push(Entry {
            kind: EntryKind::Call,
            value: elem.input.clone(),
            id: i,
            time: elem.call,
            client_id: elem.client_id,
            metadata: elem.metadata.clone(),
        });
        entries.push(Entry {
            kind: EntryKind::Return,
            value: elem.output,
            id: i,
            time: elem.return_time,
            client_id: elem.client_id,
            metadata: elem.metadata,
        });
    }
    entries.sort_by(by_time);
    entries
}

fn by_time<V, M>(a: &Entry<V, M>, b: &Entry<V, M>) -> Ordering {
    match a.time.cmp(&b.time) {
        Ordering::Equal => match (a.kind, b.kind) {
            (EntryKind::Call, EntryKind::Return) => Ordering::Less,
            (EntryKind::Return, EntryKind::Call) => Ordering::Greater,
            _ => Ordering::Equal,
        },
        other => other,
    }
}

fn renumber<M: Model>(
    events: Vec<Event<M::Value, M::Metadata>>,
) -> Vec<Event<M::Value, M::Metadata>> {
    let mut entries = Vec::new();
    let mut m = HashMap::new();
    let mut id = 0;
    for v in events {
        if let Some(&r) = m.get(&v.id) {
            entries.push(Event {
                client_id: v.client_id,
                kind: v.kind,
                value: v.value,
                id: r,
                metadata: v.metadata,
            });
        } else {
            entries.push(Event {
                client_id: v.client_id,
                kind: v.kind,
                value: v.value,
                id,
                metadata: v.metadata,
            });
            m.insert(v.id, id);
            id += 1;
        }
    }
    entries
}

fn convert_entries<M: Model>(
    events: Vec<Event<M::Value, M::Metadata>>,
) -> Vec<Entry<M::Value, M::Metadata>> {
    let mut entries = Vec::new();

    for (i, elem) in events.iter().enumerate() {
        let kind = match elem.kind {
            EventKind::Return => EntryKind::Return,
            _ => EntryKind::Call,
        };
        entries.push(Entry {
            kind,
            value: elem.value.clone(),
            id: elem.id,
            time: i as i64, // Use index as "time"
            client_id: elem.client_id,
            metadata: elem.metadata.clone(),
        });
    }

    entries
}
