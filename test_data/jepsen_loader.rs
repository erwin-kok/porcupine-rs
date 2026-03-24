use porcupine::{Event, EventModel, Model, Operation};
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader},
};

#[derive(Clone, Debug)]
pub enum EtcdOp {
    Read {
        exists: bool,
        value: i32,
        unknown: bool,
    },
    Write {
        value: i32,
    },
    Cas {
        from: i32,
        to: i32,
        ok: bool,
        unknown: bool,
    },
}

#[derive(Debug, Clone)]
pub struct EtcdModel;

impl Model for EtcdModel {
    type State = i32;
    type Op = EtcdOp;
    type Metadata = u32;

    fn init() -> i32 {
        -1000000
    }

    fn step(state: &i32, op: &EtcdOp) -> (bool, Self::State) {
        match op {
            EtcdOp::Read {
                exists,
                value,
                unknown,
            } => {
                let ok =
                    (!*exists && *state == -1000000) || (*exists && *state == *value || *unknown);
                (ok, *state)
            }
            EtcdOp::Write { value } => (true, *value),
            EtcdOp::Cas {
                from,
                to,
                ok,
                unknown,
            } => {
                let rok = (*from == *state && *ok) || (*from != *state && !*ok) || *unknown;
                let mut result = *state;
                if *from == *state {
                    result = *to;
                }
                (rok, result)
            }
        }
    }

    fn describe_operation(_op: &Operation<EtcdModel>) -> String {
        String::from("")
    }
}

#[derive(Clone, Debug)]
pub enum EtcdInput {
    Read,
    Write { value: i32 },
    Cas { from: i32, to: i32 },
}

#[derive(Clone, Debug)]
pub enum EtcdOutput {
    Read {
        exists: bool,
        value: i32,
        unknown: bool,
    },
    Write,
    Cas {
        ok: bool,
        unknown: bool,
    },
}

impl EventModel for EtcdModel {
    type Input = EtcdInput;
    type Output = EtcdOutput;

    fn combine(input: &EtcdInput, output: &EtcdOutput) -> EtcdOp {
        match (input, output) {
            (
                EtcdInput::Read,
                EtcdOutput::Read {
                    exists,
                    value,
                    unknown,
                },
            ) => EtcdOp::Read {
                exists: *exists,
                value: *value,
                unknown: *unknown,
            },
            (EtcdInput::Write { value }, EtcdOutput::Write) => EtcdOp::Write { value: *value },
            (EtcdInput::Cas { from, to }, EtcdOutput::Cas { ok, unknown }) => EtcdOp::Cas {
                from: *from,
                to: *to,
                ok: *ok,
                unknown: *unknown,
            },
            _ => panic!("unexpected input/output combination found"),
        }
    }
}

fn read_call(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event::Call {
        client_id: Some(client_id),
        value: EtcdInput::Read,
        id,
        metadata: None,
    }
}

fn read_return(client_id: u32, id: usize, exists: bool, value: i32) -> Event<EtcdModel> {
    Event::Return {
        client_id: Some(client_id),
        value: EtcdOutput::Read {
            exists,
            value,
            unknown: false,
        },
        id,
        metadata: None,
    }
}

fn write_call(client_id: u32, id: usize, value: i32) -> Event<EtcdModel> {
    Event::Call {
        client_id: Some(client_id),
        value: EtcdInput::Write { value },
        id,
        metadata: None,
    }
}

fn write_return(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event::Return {
        client_id: Some(client_id),
        value: EtcdOutput::Write,
        id,
        metadata: None,
    }
}

fn cas_call(client_id: u32, id: usize, from: i32, to: i32) -> Event<EtcdModel> {
    Event::Call {
        client_id: Some(client_id),
        value: EtcdInput::Cas { from, to },
        id,
        metadata: None,
    }
}

fn cas_return(client_id: u32, id: usize, ok: bool) -> Event<EtcdModel> {
    Event::Return {
        client_id: Some(client_id),
        value: EtcdOutput::Cas { ok, unknown: false },
        id,
        metadata: None,
    }
}

fn read_timeout(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event::Return {
        client_id: Some(client_id),
        value: EtcdOutput::Read {
            exists: false,
            value: 0,
            unknown: true,
        },
        id,
        metadata: None,
    }
}

fn write_timeout(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event::Return {
        client_id: Some(client_id),
        value: EtcdOutput::Write,
        id,
        metadata: None,
    }
}

fn cas_timeout(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event::Return {
        client_id: Some(client_id),
        value: EtcdOutput::Cas {
            ok: false,
            unknown: true,
        },
        id,
        metadata: None,
    }
}

#[derive(Clone, Copy)]
enum PendingOp {
    Read,
    Write,
    Cas,
}

pub fn load_jepsen_log(log_num: u32) -> Vec<Event<EtcdModel>> {
    parse_jepsen_log(&format!("./test_data/jepsen/etcd_{:03}.log", log_num))
}

fn parse_jepsen_log(file_name: &str) -> Vec<Event<EtcdModel>> {
    let file = File::open(file_name).unwrap_or_else(|_| panic!("can't open file {}", file_name));
    let reader = BufReader::new(file);

    let invoke_read = Regex::new(r"^INFO\s+jepsen\.util\s+-\s+(\d+)\s+:invoke\s+:read\s+nil$")
        .expect("can not compile regex");
    let invoke_write = Regex::new(r"^INFO\s+jepsen\.util\s+-\s+(\d+)\s+:invoke\s+:write\s+(\d+)$")
        .expect("can not compile regex");
    let invoke_cas =
        Regex::new(r"^INFO\s+jepsen\.util\s+-\s+(\d+)\s+:invoke\s+:cas\s+\[(\d+)\s+(\d+)\]$")
            .expect("can not compile regex");
    let return_read = Regex::new(r"^INFO\s+jepsen\.util\s+-\s+(\d+)\s+:ok\s+:read\s+(nil|\d+)$")
        .expect("can not compile regex");
    let return_write = Regex::new(r"^INFO\s+jepsen\.util\s+-\s+(\d+)\s+:ok\s+:write\s+(\d+)$")
        .expect("can not compile regex");
    let return_cas =
        Regex::new(r"^INFO\s+jepsen\.util\s+-\s+(\d+)\s+:(ok|fail)\s+:cas\s+\[(\d+)\s+(\d+)\]$")
            .expect("can not compile regex");
    let timeout_read =
        Regex::new(r"^INFO\s+jepsen\.util\s+-\s+(\d+)\s+:fail\s+:read\s+:timed-out$")
            .expect("can not compile regex");

    let mut events: Vec<Event<EtcdModel>> = Vec::new();
    let mut id: usize = 0;
    let mut pending_map: HashMap<u32, (usize, PendingOp)> = HashMap::new();

    // Read the file line by line
    for line in reader.lines() {
        let line = line.expect("error reading line");

        if let Some(caps) = invoke_read.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in invoke_read");
            events.push(read_call(proc, id));
            pending_map.insert(proc, (id, PendingOp::Read));
            id += 1;
        } else if let Some(caps) = invoke_write.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in invoke_write");
            let value: i32 = caps[2]
                .parse::<i32>()
                .expect("could not parse value in invoke_write");
            events.push(write_call(proc, id, value));
            pending_map.insert(proc, (id, PendingOp::Write));
            id += 1;
        } else if let Some(caps) = invoke_cas.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in invoke_cas");
            let from: i32 = caps[2]
                .parse::<i32>()
                .expect("could not parse from in invoke_cas");
            let to: i32 = caps[3]
                .parse::<i32>()
                .expect("could not parse to in invoke_cas");
            events.push(cas_call(proc, id, from, to));
            pending_map.insert(proc, (id, PendingOp::Cas));
            id += 1;
        } else if let Some(caps) = return_read.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in return_read");
            let (match_id, _) = pending_map[&proc];
            pending_map.remove(&proc);
            if let Some(m) = caps.get(2)
                && m.as_str().ne("nil")
            {
                let v = m
                    .as_str()
                    .parse::<i32>()
                    .expect("could not parse value in return_read");
                events.push(read_return(proc, match_id, true, v));
            } else {
                events.push(read_return(proc, match_id, false, 0));
            }
        } else if let Some(caps) = return_write.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in return_write");
            let (match_id, _) = pending_map[&proc];
            pending_map.remove(&proc);
            events.push(write_return(proc, match_id));
        } else if let Some(caps) = return_cas.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in return_cas");
            let (match_id, _) = pending_map[&proc];
            pending_map.remove(&proc);
            if let Some(m) = caps.get(2)
                && m.as_str().eq("ok")
            {
                events.push(cas_return(proc, match_id, true));
            } else {
                events.push(cas_return(proc, match_id, false));
            }
        } else if let Some(caps) = timeout_read.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in return_cas");
            let (match_id, pending) = pending_map[&proc];
            pending_map.remove(&proc);
            events.push(timeout(&pending, proc, match_id));
        }
    }

    for (proc, (match_id, pending)) in pending_map {
        events.push(timeout(&pending, proc, match_id));
    }

    events
}

fn timeout(pending: &PendingOp, proc: u32, match_id: usize) -> Event<EtcdModel> {
    match pending {
        PendingOp::Read => read_timeout(proc, match_id),
        PendingOp::Write => write_timeout(proc, match_id),
        PendingOp::Cas => cas_timeout(proc, match_id),
    }
}
