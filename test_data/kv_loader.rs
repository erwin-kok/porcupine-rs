use porcupine_rs::{Event, EventModel, Model, Operation};
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader},
};

#[derive(Clone, Debug)]
pub enum KvInput {
    Read { key: String },
    Write { key: String, value: String },
    Append { key: String, value: String },
}

impl KvInput {
    fn key(&self) -> String {
        match self {
            KvInput::Read { key } => key.clone(),
            KvInput::Write { key, .. } => key.clone(),
            KvInput::Append { key, .. } => key.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum KvOutput {
    Read { value: String },
    Write,
    Append,
}

#[derive(Clone, Debug)]
pub enum KvOp {
    Read { key: String, value: String },
    Write { key: String, value: String },
    Append { key: String, value: String },
}

impl KvOp {
    fn key(&self) -> &str {
        match self {
            KvOp::Read { key, .. } => key,
            KvOp::Write { key, .. } => key,
            KvOp::Append { key, .. } => key,
        }
    }
}

fn combine_kv(input: &KvInput, output: &KvOutput) -> KvOp {
    match (input, output) {
        (KvInput::Read { key }, KvOutput::Read { value }) => KvOp::Read {
            key: key.clone(),
            value: value.clone(),
        },
        (KvInput::Write { key, value }, KvOutput::Write) => KvOp::Write {
            key: key.clone(),
            value: value.clone(),
        },
        (KvInput::Append { key, value }, KvOutput::Append) => KvOp::Append {
            key: key.clone(),
            value: value.clone(),
        },
        _ => panic!("unexpected KvInput/KvOutput combination"),
    }
}

// ---------------------------------------------------------------------------
// KvModel — partitions by key, State = last written value for that key
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct KvModel;

impl Model for KvModel {
    type State = String;
    type Op = KvOp;
    type Metadata = ();

    fn init() -> String {
        String::new()
    }

    fn step(state: &String, op: &KvOp) -> (bool, String) {
        match op {
            KvOp::Read { value, .. } => (*value == *state, state.clone()),
            KvOp::Write { value, .. } => (true, value.clone()),
            KvOp::Append { value, .. } => (true, state.clone() + value),
        }
    }

    fn partition_operations(history: &[Operation<Self>]) -> Vec<Vec<Operation<Self>>> {
        // Each key is independent — check them separately.
        let mut by_key: HashMap<String, Vec<Operation<Self>>> = HashMap::new();
        for op in history {
            by_key
                .entry(op.op.key().to_owned())
                .or_default()
                .push(op.clone());
        }
        by_key.into_values().collect()
    }
}

impl EventModel for KvModel {
    type Input = KvInput;
    type Output = KvOutput;

    fn combine(input: &KvInput, output: &KvOutput) -> KvOp {
        combine_kv(input, output)
    }

    /// Partition events by key.
    ///
    /// Return events are matched to their call via a `pending` map so we know
    /// which key a return event belongs to.
    fn partition_events(history: &[Event<Self>]) -> Vec<Vec<Event<Self>>> {
        let mut by_key: HashMap<String, Vec<Event<Self>>> = HashMap::new();
        let mut pending: HashMap<usize, String> = HashMap::new();

        for e in history {
            match e {
                Event::Call { value, id, .. } => {
                    let key = value.key().to_owned();
                    by_key.entry(key.clone()).or_default().push(e.clone());
                    pending.insert(*id, key);
                }
                Event::Return { id, .. } => {
                    let key = pending
                        .get(id)
                        .unwrap_or_else(|| panic!("return id={id} has no matching call"))
                        .clone();
                    by_key.entry(key).or_default().push(e.clone());
                }
            }
        }
        by_key.into_values().collect()
    }
}

// ---------------------------------------------------------------------------
// KvNoPartitionModel — no partitioning, State = BTreeMap of all keys
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct KvNoPartitionModel;

impl Model for KvNoPartitionModel {
    type State = BTreeMap<String, String>;
    type Op = KvOp;
    type Metadata = ();

    fn init() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn step(state: &BTreeMap<String, String>, op: &KvOp) -> (bool, BTreeMap<String, String>) {
        match op {
            KvOp::Read { key, value } => {
                let ok = match state.get(key) {
                    Some(v) => v == value,
                    None => value.is_empty(),
                };
                (ok, state.clone())
            }
            KvOp::Write { key, value } => {
                let mut next = state.clone();
                next.insert(key.clone(), value.clone());
                (true, next)
            }
            KvOp::Append { key, value } => {
                let mut next = state.clone();
                next.entry(key.clone())
                    .and_modify(|old| old.push_str(value))
                    .or_insert_with(|| value.clone());
                (true, next)
            }
        }
    }
    // No partition_operations override → single partition (slow but correct).
}

impl EventModel for KvNoPartitionModel {
    type Input = KvInput;
    type Output = KvOutput;

    fn combine(input: &KvInput, output: &KvOutput) -> KvOp {
        combine_kv(input, output)
    }
    // No partition_events override → single partition.
}

// ---------------------------------------------------------------------------
// Event constructors (generic over any model with KvInput/KvOutput)
// ---------------------------------------------------------------------------

fn read_call<M>(client_id: u32, id: usize, key: String) -> Event<M>
where
    M: EventModel<Input = KvInput>,
{
    Event::Call {
        client_id: Some(client_id),
        value: KvInput::Read { key },
        id,
        metadata: None,
    }
}

fn read_return<M>(client_id: u32, id: usize, value: String) -> Event<M>
where
    M: EventModel<Output = KvOutput>,
{
    Event::Return {
        client_id: Some(client_id),
        value: KvOutput::Read { value },
        id,
        metadata: None,
    }
}

fn write_call<M>(client_id: u32, id: usize, key: String, value: String) -> Event<M>
where
    M: EventModel<Input = KvInput>,
{
    Event::Call {
        client_id: Some(client_id),
        value: KvInput::Write { key, value },
        id,
        metadata: None,
    }
}

fn write_return<M>(client_id: u32, id: usize) -> Event<M>
where
    M: EventModel<Output = KvOutput>,
{
    Event::Return {
        client_id: Some(client_id),
        value: KvOutput::Write,
        id,
        metadata: None,
    }
}

fn append_call<M>(client_id: u32, id: usize, key: String, value: String) -> Event<M>
where
    M: EventModel<Input = KvInput>,
{
    Event::Call {
        client_id: Some(client_id),
        value: KvInput::Append { key, value },
        id,
        metadata: None,
    }
}

fn append_return<M>(client_id: u32, id: usize) -> Event<M>
where
    M: EventModel<Output = KvOutput>,
{
    Event::Return {
        client_id: Some(client_id),
        value: KvOutput::Append,
        id,
        metadata: None,
    }
}

// ---------------------------------------------------------------------------
// Log file parser
// ---------------------------------------------------------------------------

/// Which kind of operation a pending call is.
#[derive(Clone, Copy)]
enum PendingOp {
    Read,
    Write,
    Append,
}

/// Parse a jepsen-format KV log file into an event history.
///
/// Lines not matching any known pattern are silently skipped.
/// Operations that were invoked but never returned are given a synthetic
/// return event (with an empty value for reads, or a no-op ack for writes).
fn parse_kv_log<M>(path: &str) -> Vec<Event<M>>
where
    M: EventModel<Input = KvInput, Output = KvOutput>,
{
    let file = File::open(path).unwrap_or_else(|_| panic!("cannot open {path}"));
    let reader = BufReader::new(file);

    let invoke_read =
        Regex::new(r#"\{:process (\d+), :type :invoke, :f :get, :key "(.*)", :value nil\}"#)
            .unwrap();
    let invoke_write =
        Regex::new(r#"\{:process (\d+), :type :invoke, :f :put, :key "(.*)", :value "(.*)"\}"#)
            .unwrap();
    let invoke_append =
        Regex::new(r#"\{:process (\d+), :type :invoke, :f :append, :key "(.*)", :value "(.*)"\}"#)
            .unwrap();
    let return_read =
        Regex::new(r#"\{:process (\d+), :type :ok, :f :get, :key ".*", :value "(.*)"\}"#).unwrap();
    let return_write =
        Regex::new(r#"\{:process (\d+), :type :ok, :f :put, :key ".*", :value ".*"\}"#).unwrap();
    let return_append =
        Regex::new(r#"\{:process (\d+), :type :ok, :f :append, :key ".*", :value ".*"\}"#).unwrap();

    let mut events: Vec<Event<M>> = Vec::new();
    let mut next_id: usize = 0;
    let mut pending: HashMap<u32, (usize, PendingOp)> = HashMap::new();

    for line in reader.lines() {
        let line = line.expect("error reading log line");

        if let Some(c) = invoke_read.captures(&line) {
            let proc: u32 = c[1].parse().unwrap();
            events.push(read_call(proc, next_id, c[2].to_owned()));
            pending.insert(proc, (next_id, PendingOp::Read));
            next_id += 1;
        } else if let Some(c) = invoke_write.captures(&line) {
            let proc: u32 = c[1].parse().unwrap();
            events.push(write_call(proc, next_id, c[2].to_owned(), c[3].to_owned()));
            pending.insert(proc, (next_id, PendingOp::Write));
            next_id += 1;
        } else if let Some(c) = invoke_append.captures(&line) {
            let proc: u32 = c[1].parse().unwrap();
            events.push(append_call(proc, next_id, c[2].to_owned(), c[3].to_owned()));
            pending.insert(proc, (next_id, PendingOp::Append));
            next_id += 1;
        } else if let Some(c) = return_read.captures(&line) {
            let proc: u32 = c[1].parse().unwrap();
            if let Some((id, _)) = pending.remove(&proc) {
                events.push(read_return(proc, id, c[2].to_owned()));
            }
        } else if let Some(c) = return_write.captures(&line) {
            let proc: u32 = c[1].parse().unwrap();
            if let Some((id, _)) = pending.remove(&proc) {
                events.push(write_return(proc, id));
            }
        } else if let Some(c) = return_append.captures(&line) {
            let proc: u32 = c[1].parse().unwrap();
            if let Some((id, _)) = pending.remove(&proc) {
                events.push(append_return(proc, id));
            }
        }
    }

    // Synthesise return events for operations that never completed.
    for (proc, (id, kind)) in pending {
        match kind {
            PendingOp::Read => events.push(read_return(proc, id, String::new())),
            PendingOp::Write => events.push(write_return(proc, id)),
            PendingOp::Append => events.push(append_return(proc, id)),
        }
    }

    events
}

pub fn load_kv_log(name: &str) -> Vec<Event<KvModel>> {
    parse_kv_log(&format!("./test_data/kv/{name}.txt"))
}

pub fn load_kv_log_no_part(name: &str) -> Vec<Event<KvNoPartitionModel>> {
    parse_kv_log(&format!("./test_data/kv/{name}.txt"))
}
