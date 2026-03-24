use std::collections::HashMap;

use crate::model::{Event, EventModel, Model, Operation};

#[derive(Clone, Copy, Debug)]
pub enum CheckEntry {
    Call { op_index: usize, time: i64 },
    Return { op_index: usize, time: i64 },
}

impl CheckEntry {
    #[cfg(test)]
    pub fn op_index(&self) -> usize {
        match self {
            CheckEntry::Call { op_index, .. } => *op_index,
            CheckEntry::Return { op_index, .. } => *op_index,
        }
    }

    fn time(&self) -> i64 {
        match self {
            CheckEntry::Call { time, .. } => *time,
            CheckEntry::Return { time, .. } => *time,
        }
    }

    fn kind_order(&self) -> u8 {
        match self {
            CheckEntry::Call { .. } => 0,
            CheckEntry::Return { .. } => 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Partition
// ---------------------------------------------------------------------------

pub struct Partition<M: Model> {
    /// Time-sorted interleaved call and return entries.
    pub check_history: Vec<CheckEntry>,
    /// Full operation data.  `ops[i]` is the operation with id `i`.
    pub ops: Vec<Operation<M>>,
}

impl<M: Model> Partition<M> {
    // -----------------------------------------------------------------------
    // from_operations
    // -----------------------------------------------------------------------
    pub fn from_operations(operations: &[Operation<M>]) -> Self {
        let ops = operations.to_vec();

        let mut check_history: Vec<CheckEntry> = ops
            .iter()
            .enumerate()
            .flat_map(|(i, op)| {
                [
                    CheckEntry::Call {
                        op_index: i,
                        time: op.call_time,
                    },
                    CheckEntry::Return {
                        op_index: i,
                        time: op.return_time,
                    },
                ]
            })
            .collect();

        check_history.sort_by_key(|e| (e.time(), e.kind_order()));

        Self { check_history, ops }
        }
    }

impl<M: EventModel> Partition<M> {
    // -----------------------------------------------------------------------
    // from_events
    // -----------------------------------------------------------------------
    pub fn from_events(events: &[Event<M>]) -> Self {
        let events = Self::renumber(events);

        let m = events
            .iter()
            .filter(|e| matches!(e, Event::Return { .. }))
            .count();

        struct Pending<M: EventModel> {
            call_time: i64,
            input: M::Input,
            client_id: Option<u32>,
            metadata: Option<M::Metadata>,
        }

        let mut pending: HashMap<usize, Pending<M>> = HashMap::new();
        let mut ops: Vec<Option<Operation<M>>> = (0..m).map(|_| None).collect();
        let mut check_history: Vec<CheckEntry> = Vec::with_capacity(events.len());

        for (pos, e) in events.iter().enumerate() {
            let time = pos as i64;

            match e {
                Event::Call {
                    client_id,
                    value,
                    id,
                    metadata,
                } => {
                    pending.insert(
                        *id,
                        Pending {
                            call_time: time,
                            input: value.clone(),
                            client_id: *client_id,
                            metadata: metadata.clone(),
                        },
                    );
                    check_history.push(CheckEntry::Call {
                        op_index: *id,
                        time,
                    });
                }

                Event::Return {
                    value,
                    id,
                    metadata,
                    ..
                } => {
                    let p = pending
                        .remove(id)
                        .unwrap_or_else(|| panic!("return event id={id} has no matching call"));

                    // Combine input and output into an Op
                    let op = M::combine(&p.input, &value.clone());
                    // Slot by id so ops[id] is always the right operation.
                    ops[*id] = Some(Operation {
                        client_id: p.client_id,
                        call_time: p.call_time,
                        return_time: time,
                        op,
                        metadata: p.metadata.clone().or_else(|| metadata.clone()),
                    });
                    check_history.push(CheckEntry::Return {
                        op_index: *id,
                        time,
                    });
                }
            }
        }

        let ops: Vec<Operation<M>> = ops
            .into_iter()
            .map(|x| x.expect("call event has no matching return"))
            .collect();

        check_history.sort_by_key(|e| (e.time(), e.kind_order()));

        Self { check_history, ops }
    }

    // -----------------------------------------------------------------------
    // renumber
    // -----------------------------------------------------------------------
    fn renumber(events: &[Event<M>]) -> Vec<Event<M>> {
        let mut remap: HashMap<usize, usize> = HashMap::new();
        let mut next_id: usize = 0;

        events
            .iter()
            .map(|e| {
                let old_id = match e {
                    Event::Call { id, .. } => *id,
                    Event::Return { id, .. } => *id,
                };
                let new_id = *remap.entry(old_id).or_insert_with(|| {
                    let id = next_id;
                    next_id += 1;
                    id
                });
                match e {
                    Event::Call {
                        client_id,
                        value,
                        metadata,
                        ..
                    } => Event::Call {
                        client_id: *client_id,
                        value: value.clone(),
                        id: new_id,
                        metadata: metadata.clone(),
                    },
                    Event::Return {
                        client_id,
                        value,
                        metadata,
                        ..
                    } => Event::Return {
                        client_id: *client_id,
                        value: value.clone(),
                        id: new_id,
                        metadata: metadata.clone(),
                    },
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Model, Operation};

    #[derive(Clone)]
    struct DummyModel;
    impl Model for DummyModel {
        type State = ();
        type Op = ();
        type Metadata = ();
        fn init() {}

        fn step(_: &Self::State, _: &Self::Op) -> (bool, Self::State) {
            (true, ())
        }
    }

    impl EventModel for DummyModel {
        type Input = i32;
        type Output = i32;
        fn combine(_: &Self::Input, _: &Self::Output) {}
    }
    fn op(call: i64, ret: i64) -> Operation<DummyModel> {
        Operation {
            client_id: None,
            call_time: call,
            return_time: ret,
            op: (),
            metadata: None,
        }
    }

    fn call_event(id: usize, v: i32) -> Event<DummyModel> {
        Event::Call {
            client_id: None,
            value: v,
            id,
            metadata: None,
        }
    }

    fn ret_event(id: usize, v: i32) -> Event<DummyModel> {
        Event::Return {
            client_id: None,
            value: v,
            id,
            metadata: None,
        }
    }

    // --- from_operations ---

    #[test]
    fn from_operations_empty() {
        let p = Partition::<DummyModel>::from_operations(&[]);
        assert!(p.check_history.is_empty());
        assert!(p.ops.is_empty());
    }

    #[test]
    fn from_operations_single_op_two_entries() {
        let p = Partition::from_operations(&[op(0, 10)]);
        assert_eq!(p.check_history.len(), 2);
        assert_eq!(p.ops.len(), 1);
    }

    #[test]
    fn from_operations_sorted_by_time() {
        // op0: call=5, ret=20 | op1: call=1, ret=10
        // expected: op1_call(1), op0_call(5), op1_ret(10), op0_ret(20)
        let p = Partition::from_operations(&[op(5, 20), op(1, 10)]);
        let times: Vec<i64> = p.check_history.iter().map(|e| e.time()).collect();
        let mut sorted = times.clone();
        sorted.sort();
        assert_eq!(times, sorted);
    }

    #[test]
    fn from_operations_call_before_return_on_equal_time() {
        let p = Partition::from_operations(&[op(5, 5)]);
        assert!(matches!(p.check_history[0], CheckEntry::Call { .. }));
        assert!(matches!(p.check_history[1], CheckEntry::Return { .. }));
    }

    #[test]
    fn from_operations_op_index_matches_position() {
        let p = Partition::from_operations(&[op(0, 2), op(1, 3)]);
        for entry in &p.check_history {
            let i = entry.op_index();
            // ops[i] must exist and its position in the vec equals i.
            assert!(i < p.ops.len());
        }
    }

    // --- from_events ---

    #[test]
    fn from_events_empty() {
        let p = Partition::<DummyModel>::from_events(&[]);
        assert!(p.check_history.is_empty());
        assert!(p.ops.is_empty());
    }

    #[test]
    fn from_events_sequential() {
        let evs = vec![
            call_event(0, 10),
            ret_event(0, 10),
            call_event(1, 20),
            ret_event(1, 20),
        ];
        let p = Partition::from_events(&evs);
        assert_eq!(p.ops.len(), 2);
        assert_eq!(p.check_history.len(), 4);
    }

    #[test]
    fn from_events_out_of_order_returns() {
        // call(7), call(3), return(3), return(7)
        // renumber: 7→0, 3→1 — ops[0] is op originally id=7, ops[1] is op originally id=3
        let evs = vec![
            call_event(7, 0),
            call_event(3, 0),
            ret_event(3, 0),
            ret_event(7, 0),
        ];
        let p = Partition::from_events(&evs);
        assert_eq!(p.ops.len(), 2);
        // ops[0] was called first (original id 7, renumbered to 0)
        // call_time = 0 (position of call_ev(7))
        assert_eq!(p.ops[0].call_time, 0);
        // ops[1] was called second (original id 3, renumbered to 1)
        assert_eq!(p.ops[1].call_time, 1);
    }

    #[test]
    fn from_events_timestamps_are_positions() {
        // positions: 0=call(0), 1=call(1), 2=ret(0), 3=ret(1)
        let evs = vec![
            call_event(0, 0),
            call_event(1, 0),
            ret_event(0, 0),
            ret_event(1, 0),
        ];
        let p = Partition::from_events(&evs);
        assert_eq!(p.ops[0].call_time, 0);
        assert_eq!(p.ops[0].return_time, 2);
        assert_eq!(p.ops[1].call_time, 1);
        assert_eq!(p.ops[1].return_time, 3);
    }

    #[test]
    fn renumber_makes_ids_dense() {
        let evs = vec![
            call_event(5, 0),
            call_event(99, 0),
            ret_event(5, 0),
            ret_event(99, 0),
        ];
        let p = Partition::from_events(&evs);
        // ops[0] = originally id 5, ops[1] = originally id 99
        assert_eq!(p.ops.len(), 2);
    }
}
