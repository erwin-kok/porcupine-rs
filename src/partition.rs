use std::collections::HashMap;

use crate::model::{Event, Model, Operation};

#[derive(Clone, Copy, Debug)]
pub enum CheckEntry {
    Call { cr_index: usize, time: i64 },
    Return { cr_index: usize, time: i64 },
}

impl CheckEntry {
    #[cfg(test)]
    fn cr_index(&self) -> usize {
        match self {
            CheckEntry::Call { cr_index, .. } => *cr_index,
            CheckEntry::Return { cr_index, .. } => *cr_index,
        }
    }

    fn time(&self) -> i64 {
        match self {
            CheckEntry::Call { time, .. } => *time,
            CheckEntry::Return { time, .. } => *time,
        }
    }

    fn kind_order(&self) -> u8 {
        // Call sorts before Return at equal timestamps.
        match self {
            CheckEntry::Call { .. } => 0,
            CheckEntry::Return { .. } => 1,
        }
    }
}

pub struct CallReturn<M: Model> {
    pub id: usize,
    pub call_time: i64,
    pub return_time: i64,
    pub input: M::Input,
    pub output: M::Output,
    pub client_id: Option<u32>,
    pub metadata: Option<M::Metadata>,
}

impl<M: Model> Clone for CallReturn<M> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            call_time: self.call_time,
            return_time: self.return_time,
            input: self.input.clone(),
            output: self.output.clone(),
            client_id: self.client_id,
            metadata: self.metadata.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Partition
// ---------------------------------------------------------------------------

pub struct Partition<M: Model> {
    pub check_history: Vec<CheckEntry>,
    pub call_returns: Vec<CallReturn<M>>,
}

impl<M: Model> Partition<M> {
    pub fn from_operations(ops: &[Operation<M>]) -> Self {
        let call_returns: Vec<CallReturn<M>> = ops
            .iter()
            .enumerate()
            .map(|(i, op)| CallReturn {
                id: i,
                call_time: op.call_time,
                return_time: op.return_time,
                input: op.input.clone(),
                output: op.output.clone(),
                client_id: op.client_id,
                metadata: op.metadata.clone(),
            })
            .collect();

        let mut check_history: Vec<CheckEntry> = ops
            .iter()
            .enumerate()
            .flat_map(|(i, op)| {
                [
                    CheckEntry::Call {
                        cr_index: i,
                        time: op.call_time,
                    },
                    CheckEntry::Return {
                        cr_index: i,
                        time: op.return_time,
                    },
                ]
            })
            .collect();

        check_history.sort_by_key(|e| (e.time(), e.kind_order()));

        Self {
            check_history,
            call_returns,
        }
    }

    pub fn from_events(events: &[Event<M>]) -> Self {
        let events = Self::renumber(events);

        let m = events
            .iter()
            .filter(|e| matches!(e, Event::Return { .. }))
            .count();

        struct Pending<M: Model> {
            call_time: i64,
            input: M::Input,
            client_id: Option<u32>,
            metadata: Option<M::Metadata>,
        }

        let mut pending: HashMap<usize, Pending<M>> = HashMap::new();
        let mut call_returns: Vec<Option<CallReturn<M>>> = (0..m).map(|_| None).collect();
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
                        cr_index: *id,
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

                    // Slot by id so call_returns[id].id == id holds regardless of
                    // return arrival order.
                    call_returns[*id] = Some(CallReturn {
                        id: *id,
                        call_time: p.call_time,
                        return_time: time,
                        input: p.input,
                        output: value.clone(),
                        client_id: p.client_id,
                        metadata: p.metadata.clone().or_else(|| metadata.clone()),
                    });
                    check_history.push(CheckEntry::Return {
                        cr_index: *id,
                        time,
                    });
                }
            }
        }

        let call_returns: Vec<CallReturn<M>> = call_returns
            .into_iter()
            .map(|x| x.expect("call event has no matching return"))
            .collect();

        // Already in position order; sort only for the Call-before-Return tie-break.
        check_history.sort_by_key(|e| (e.time(), e.kind_order()));

        Self {
            check_history,
            call_returns,
        }
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
        type Input = i32;
        type Output = i32;
        type Metadata = ();
        fn init() {}
        fn step(_: &(), _: &i32, _: &i32) -> (bool, ()) {
            (true, ())
        }
    }

    fn op(call_time: i64, return_time: i64, input: i32) -> Operation<DummyModel> {
        Operation {
            client_id: None,
            input,
            call_time,
            output: 0,
            return_time,
            metadata: None,
        }
    }

    fn call_ev(id: usize, v: i32) -> Event<DummyModel> {
        Event::Call {
            client_id: None,
            value: v,
            id,
            metadata: None,
        }
    }

    fn ret_ev(id: usize, v: i32) -> Event<DummyModel> {
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
        assert!(p.call_returns.is_empty());
    }

    #[test]
    fn from_operations_single_op_two_entries() {
        let p = Partition::from_operations(&[op(0, 10, 1)]);
        assert_eq!(p.check_history.len(), 2);
        assert_eq!(p.call_returns.len(), 1);
    }

    #[test]
    fn from_operations_id_equals_index() {
        let p = Partition::from_operations(&[op(0, 5, 0), op(1, 6, 0), op(2, 7, 0)]);
        for (i, cr) in p.call_returns.iter().enumerate() {
            assert_eq!(cr.id, i);
        }
    }

    #[test]
    fn from_operations_sorted_by_time() {
        // op0: call=5, ret=20 | op1: call=1, ret=10
        // expected: op1_call(1), op0_call(5), op1_ret(10), op0_ret(20)
        let p = Partition::from_operations(&[op(5, 20, 0), op(1, 10, 0)]);
        let times: Vec<i64> = p.check_history.iter().map(|e| e.time()).collect();
        let mut sorted = times.clone();
        sorted.sort();
        assert_eq!(times, sorted);
    }

    #[test]
    fn from_operations_call_before_return_on_equal_time() {
        let p = Partition::from_operations(&[op(5, 5, 0)]);
        assert!(matches!(p.check_history[0], CheckEntry::Call { .. }));
        assert!(matches!(p.check_history[1], CheckEntry::Return { .. }));
    }

    #[test]
    fn from_operations_cr_index_matches_id() {
        let p = Partition::from_operations(&[op(0, 2, 0), op(1, 3, 0)]);
        for entry in &p.check_history {
            let cr_index = entry.cr_index();
            assert_eq!(p.call_returns[cr_index].id, cr_index);
        }
    }

    // --- from_events ---

    #[test]
    fn from_events_empty() {
        let p = Partition::<DummyModel>::from_events(&[]);
        assert!(p.check_history.is_empty());
        assert!(p.call_returns.is_empty());
    }

    #[test]
    fn from_events_sequential() {
        let evs = vec![call_ev(0, 10), ret_ev(0, 10), call_ev(1, 20), ret_ev(1, 20)];
        let p = Partition::from_events(&evs);
        assert_eq!(p.call_returns.len(), 2);
        assert_eq!(p.check_history.len(), 4);
        for (i, cr) in p.call_returns.iter().enumerate() {
            assert_eq!(cr.id, i);
        }
    }

    #[test]
    fn from_events_out_of_order_returns_invariant_holds() {
        // call(7), call(3), return(3), return(7) — out-of-order returns
        // renumber: 7→0, 3→1
        let evs = vec![call_ev(7, 0), call_ev(3, 0), ret_ev(3, 0), ret_ev(7, 0)];
        let p = Partition::from_events(&evs);
        assert_eq!(p.call_returns.len(), 2);
        for (i, cr) in p.call_returns.iter().enumerate() {
            assert_eq!(cr.id, i, "invariant violated at index {i}");
        }
    }

    #[test]
    fn from_events_timestamps_are_positions() {
        // positions: 0=call(0), 1=call(1), 2=ret(0), 3=ret(1)
        let evs = vec![call_ev(0, 0), call_ev(1, 0), ret_ev(0, 0), ret_ev(1, 0)];
        let p = Partition::from_events(&evs);
        assert_eq!(p.call_returns[0].call_time, 0);
        assert_eq!(p.call_returns[0].return_time, 2);
        assert_eq!(p.call_returns[1].call_time, 1);
        assert_eq!(p.call_returns[1].return_time, 3);
    }

    #[test]
    fn renumber_makes_ids_dense() {
        // original ids: 5, 99 — renumbered to 0, 1
        let evs = vec![call_ev(5, 0), call_ev(99, 0), ret_ev(5, 0), ret_ev(99, 0)];
        let p = Partition::from_events(&evs);
        assert_eq!(p.call_returns[0].id, 0);
        assert_eq!(p.call_returns[1].id, 1);
    }
}
