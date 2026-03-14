use crate::model::{CheckResult, Event, EventKind, Model, Operation};
use std::{cmp::Ordering, collections::HashMap, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Call,
    Return,
}

#[derive(Debug, Clone)]
struct Entry<V, M> {
    kind: EntryKind,
    value: V,
    id: usize,
    time: i64,
    client_id: Option<u32>,
    metadata: Option<M>,
}

pub struct LinearizationInfo {}

pub(crate) fn check_events<M: Model>(
    history: &[Event<M::Value, M::Metadata>],
    verbose: bool,
    timeout: Duration,
) -> (CheckResult, LinearizationInfo) {
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
) -> (CheckResult, LinearizationInfo) {
    let partitions = M::partition(history);
    let mut l = Vec::with_capacity(partitions.len());
    for subhistory in partitions {
        l.push(make_entries::<M>(subhistory));
    }
    check_parallel::<M>(l, verbose, timeout)
}

fn check_parallel<M: Model>(
    history: Vec<Vec<Entry<M::Value, M::Metadata>>>,
    verbose: bool,
    timeout: Duration,
) -> (CheckResult, LinearizationInfo) {
    (CheckResult::Ok, LinearizationInfo {})
}

fn make_entries<M: Model>(
    history: Vec<Operation<M::Value, M::Metadata>>,
) -> Vec<Entry<M::Value, M::Metadata>> {
    let mut entries = Vec::new();
    let mut i = 0;
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
