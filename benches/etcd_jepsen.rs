extern crate criterion;

use criterion::{Criterion, criterion_group, criterion_main};
use porcupine::{Event, EventKind, EventValue, Model};
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    hash::Hash,
    io::{BufRead, BufReader},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EtcdInput {
    Read,
    Write(i32),
    Cas { from: i32, to: i32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EtcdOutput {
    ReadAck {
        exists: bool,
        value: i32,
        unknown: bool,
    },
    WriteAck {
        unknown: bool,
    },
    CasAck {
        ok: bool,
        unknown: bool,
    },
}

#[derive(Debug, Clone)]
pub struct EtcdModel {}

impl Model for EtcdModel {
    type State = i32;
    type Input = EtcdInput;
    type Output = EtcdOutput;
    type Metadata = u32;

    fn init() -> i32 {
        -1000000
    }

    fn step(
        state: &Self::State,
        input: &Self::Input,
        output: &Self::Output,
    ) -> (bool, Self::State) {
        match (input, output) {
            (
                EtcdInput::Read,
                EtcdOutput::ReadAck {
                    exists,
                    value,
                    unknown,
                },
            ) => {
                let ok =
                    (!*exists && *state == -1000000) || (*exists && *state == *value || *unknown);
                (ok, *state)
            }
            (EtcdInput::Write(arg), EtcdOutput::WriteAck { unknown: _ }) => (true, *arg),
            (
                EtcdInput::Cas {
                    from: arg1,
                    to: arg2,
                },
                EtcdOutput::CasAck { ok, unknown },
            ) => {
                let rok = (*arg1 == *state && *ok) || (*arg1 != *state && !*ok) || *unknown;
                let mut result = *state;
                if *arg1 == *state {
                    result = *arg2;
                }
                (rok, result)
            }
            _ => panic!("unexpected input/output combination found"),
        }
    }

    fn describe_operation(_input: &Self::Input, _output: &Self::Output) -> String {
        String::from("")
    }
}

fn read_call(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Call,
        value: EventValue::Call(EtcdInput::Read),
        id,
        metadata: None,
    }
}

fn read_return(client_id: u32, id: usize, exists: bool, value: i32) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Return,
        value: EventValue::Return(EtcdOutput::ReadAck {
            exists,
            value,
            unknown: false,
        }),
        id,
        metadata: None,
    }
}

fn write_call(client_id: u32, id: usize, value: i32) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Call,
        value: EventValue::Call(EtcdInput::Write(value)),
        id,
        metadata: None,
    }
}

fn write_return(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Return,
        value: EventValue::Return(EtcdOutput::WriteAck { unknown: false }),
        id,
        metadata: None,
    }
}

fn cas_call(client_id: u32, id: usize, from: i32, to: i32) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Call,
        value: EventValue::Call(EtcdInput::Cas { from, to }),
        id,
        metadata: None,
    }
}

fn cas_return(client_id: u32, id: usize, ok: bool) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Return,
        value: EventValue::Return(EtcdOutput::CasAck { ok, unknown: false }),
        id,
        metadata: None,
    }
}

fn read_timeout(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Return,
        value: EventValue::Return(EtcdOutput::ReadAck {
            exists: false,
            value: 0,
            unknown: true,
        }),
        id,
        metadata: None,
    }
}

fn write_timeout(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Return,
        value: EventValue::Return(EtcdOutput::WriteAck { unknown: true }),
        id,
        metadata: None,
    }
}

fn cas_timeout(client_id: u32, id: usize) -> Event<EtcdModel> {
    Event {
        client_id: Some(client_id),
        kind: EventKind::Return,
        value: EventValue::Return(EtcdOutput::CasAck {
            ok: false,
            unknown: true,
        }),
        id,
        metadata: None,
    }
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
    let mut proc_id_map: HashMap<u32, usize> = HashMap::new();
    let mut proc_kind_map: HashMap<u32, usize> = HashMap::new();

    // Read the file line by line
    for line in reader.lines() {
        let line = line.expect("error reading line");

        if let Some(caps) = invoke_read.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in invoke_read");
            events.push(read_call(proc, id));
            proc_id_map.insert(proc, id);
            proc_kind_map.insert(proc, 0);
            id += 1;
        }
        if let Some(caps) = invoke_write.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in invoke_write");
            let value: i32 = caps[2]
                .parse::<i32>()
                .expect("could not parse value in invoke_write");
            events.push(write_call(proc, id, value));
            proc_id_map.insert(proc, id);
            proc_kind_map.insert(proc, 1);
            id += 1;
        }
        if let Some(caps) = invoke_cas.captures(&line) {
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
            proc_id_map.insert(proc, id);
            proc_kind_map.insert(proc, 2);
            id += 1;
        }

        if let Some(caps) = return_read.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in return_read");
            let match_id = proc_id_map[&proc];
            proc_id_map.remove(&proc);
            proc_kind_map.remove(&proc);
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
        }
        if let Some(caps) = return_write.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in return_write");
            let match_id = proc_id_map[&proc];
            proc_id_map.remove(&proc);
            proc_kind_map.remove(&proc);
            events.push(write_return(proc, match_id));
        }
        if let Some(caps) = return_cas.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in return_cas");
            let match_id = proc_id_map[&proc];
            proc_id_map.remove(&proc);
            proc_kind_map.remove(&proc);
            if let Some(m) = caps.get(2)
                && m.as_str().eq("ok")
            {
                events.push(cas_return(proc, match_id, true));
            } else {
                events.push(cas_return(proc, match_id, false));
            }
        }

        if let Some(caps) = timeout_read.captures(&line) {
            let proc: u32 = caps[1]
                .parse::<u32>()
                .expect("could not parse proc in return_cas");
            let match_id = proc_id_map[&proc];
            proc_id_map.remove(&proc);

            let kind = proc_kind_map[&proc];
            proc_kind_map.remove(&proc);

            match kind {
                0 => {
                    events.push(read_timeout(proc, match_id));
                }

                1 => {
                    events.push(write_timeout(proc, match_id));
                }

                2 => {
                    events.push(cas_timeout(proc, match_id));
                }

                _ => panic!("unknown kind"),
            }
        }
    }

    for (proc, match_id) in proc_id_map {
        let kind = proc_kind_map[&proc];
        match kind {
            0 => {
                events.push(read_timeout(proc, match_id));
            }

            1 => {
                events.push(write_timeout(proc, match_id));
            }

            2 => {
                events.push(cas_timeout(proc, match_id));
            }

            _ => panic!("unknown kind"),
        }
    }

    events
}

fn check_jepsen(log_num: u32, correct: bool) {
    let events = parse_jepsen_log(&format!("./test_data/jepsen/etcd_{:03}.log", log_num));
    let res = porcupine::check_events(&events);
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
