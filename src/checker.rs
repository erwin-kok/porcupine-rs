use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crossbeam::{
    channel::{bounded, select},
    thread,
};

use crate::linearizer::Linearizer;
use crate::model::{CheckResult, Event, EventModel, Model, Operation};
use crate::partition::{CheckEntry, Partition};

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Check whether an operation history is linearizable.
///
/// Pass `timeout = Some(d)` to bound the search.  Returns [`CheckResult::Unknown`]
/// if the timeout elapses before a definitive answer is found.
/// Pass `timeout = None` for an unlimited search.
pub fn check_operations<M: Model>(
    history: &[Operation<M>],
    timeout: Option<Duration>,
) -> CheckResult {
    let partitions: Vec<Partition<M>> = M::partition_operations(history)
        .into_iter()
        .map(|ops| Partition::from_operations(&ops))
        .collect();

    check_partitions::<M>(partitions, timeout)
}

/// Check whether an event history (interleaved calls and returns) is
/// linearizable.
pub fn check_events<M: EventModel>(history: &[Event<M>], timeout: Option<Duration>) -> CheckResult {
    let partitions: Vec<Partition<M>> = M::partition_events(history)
        .into_iter()
        .map(|evs| Partition::from_events(&evs))
        .collect();

    check_partitions::<M>(partitions, timeout)
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

fn check_partitions<M: Model>(
    partitions: Vec<Partition<M>>,
    timeout: Option<Duration>,
) -> CheckResult {
    match partitions.len() {
        0 => CheckResult::Ok,

        1 if timeout.is_none() => {
            let kill = AtomicBool::new(false);
            if check_single(&partitions[0], &kill) {
                CheckResult::Ok
            } else {
                CheckResult::Illegal
            }
        }

        _ => check_parallel(partitions, timeout),
    }
}

// ---------------------------------------------------------------------------
// Parallel driver
// ---------------------------------------------------------------------------

fn check_parallel<M: Model>(
    partitions: Vec<Partition<M>>,
    timeout: Option<Duration>,
) -> CheckResult {
    let total = partitions.len();
    let (tx, rx) = bounded::<bool>(total); // capacity = total so sends never block
    let kill = Arc::new(AtomicBool::new(false));

    thread::scope(|s| {
        // Spawn one thread per partition.
        for partition in &partitions {
            let tx = tx.clone();
            let kill = Arc::clone(&kill);

            s.spawn(move |_| {
                let ok = check_single(partition, &kill);
                let _ = tx.send(ok); // never blocks: bounded(total)
            });
        }

        // Drop our copy of the sender so the channel closes when all threads
        // finish and their sender clones are dropped.  Without this, rx.recv()
        // would never return Err.
        drop(tx);

        // Optional timeout channel: fires once after the deadline.
        let timeout_ch = timeout.map(crossbeam::channel::after);

        let mut ok_all = true;
        let mut received = 0;
        let mut timed_out = false;

        loop {
            if received >= total {
                break;
            }

            let result: Option<bool> = if let Some(ref t) = timeout_ch {
                select! {
                    recv(rx) -> msg => msg.ok(),
                    recv(t)  -> _   => {
                        timed_out = true;
                        kill.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            } else {
                rx.recv().ok()
            };

            match result {
                Some(true) => {
                    received += 1;
                }
                Some(false) => {
                    // One partition is not linearizable — stop immediately.
                    ok_all = false;
                    kill.store(true, Ordering::Relaxed);
                    break;
                }
                None => break, // all senders dropped (all threads done)
            }
        }

        if !ok_all {
            CheckResult::Illegal
        } else if timed_out {
            CheckResult::Unknown
        } else {
            CheckResult::Ok
        }
    })
    .unwrap()
}

// ---------------------------------------------------------------------------
// Core loop — one partition
// ---------------------------------------------------------------------------

/// Run the search on one partition.
///
/// Returns `true` if the history is linearizable, `false` if it is not or if
/// `kill` was set before the search completed.
fn check_single<M: Model>(partition: &Partition<M>, kill: &AtomicBool) -> bool {
    let n = partition.check_history.len();
    let mut linearizer = Linearizer::<M>::new(partition);
    let mut current = linearizer.front();

    while current < n {
        // Cooperative cancellation — checked before every decision.
        if kill.load(Ordering::Relaxed) {
            return false;
        }

        match partition.check_history[current] {
            CheckEntry::Call { .. } => {
                if let Some(next_state) = linearizer.try_linearize(current) {
                    linearizer.lift(current, next_state);
                    current = linearizer.front(); // restart scan from the head
                } else {
                    current = linearizer.next_of(current); // this candidate is exhausted
                }
            }
            CheckEntry::Return { .. } => {
                // A Return is visible before its Call was lifted — every
                // candidate that could precede it has been tried.  Backtrack.
                match linearizer.backtrack() {
                    Some(pos) => current = linearizer.next_of(pos),
                    None => return false, // stack empty — not linearizable
                }
            }
        }
    }

    true // every entry lifted — complete linearization found
}
