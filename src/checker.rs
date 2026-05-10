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

use crate::linearization_info::LinearizationInfo;
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
    let partitions = build_operation_partitions::<M>(history);
    check_partitions::<M>(partitions, timeout, false).0
}

/// Check an operation history and return both the verdict and diagnostic info.
pub fn check_operations_info<M: Model>(
    history: &[Operation<M>],
    timeout: Option<Duration>,
) -> (CheckResult, LinearizationInfo<M>) {
    let partitions = build_operation_partitions::<M>(history);
    check_partitions::<M>(partitions, timeout, true)
}

/// Check whether an event history is linearizable, returning only the verdict.
pub fn check_events<M: EventModel>(history: &[Event<M>], timeout: Option<Duration>) -> CheckResult {
    let partitions = build_event_partitions::<M>(history);
    check_partitions::<M>(partitions, timeout, false).0
}

/// Check an event history and return both the verdict and diagnostic info.
pub fn check_events_info<M: EventModel>(
    history: &[Event<M>],
    timeout: Option<Duration>,
) -> (CheckResult, LinearizationInfo<M>) {
    let partitions = build_event_partitions::<M>(history);
    check_partitions::<M>(partitions, timeout, true)
}

// ---------------------------------------------------------------------------
// Partition builders
// ---------------------------------------------------------------------------

fn build_operation_partitions<M: Model>(history: &[Operation<M>]) -> Vec<Partition<M>> {
    M::partition_operations(history)
        .into_iter()
        .map(|ops| Partition::from_operations(&ops))
        .collect()
}

fn build_event_partitions<M: EventModel>(history: &[Event<M>]) -> Vec<Partition<M>> {
    M::partition_events(history)
        .into_iter()
        .map(|evs| Partition::from_events(&evs))
        .collect()
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

fn check_partitions<M: Model>(
    partitions: Vec<Partition<M>>,
    timeout: Option<Duration>,
    compute_info: bool,
) -> (CheckResult, LinearizationInfo<M>) {
    match partitions.len() {
        0 => (CheckResult::Ok, LinearizationInfo::empty()),

        // Fast path: single partition, no timeout, no info needed.
        1 if timeout.is_none() && !compute_info => {
            let kill = AtomicBool::new(false);
            let (ok, _) = check_single(&partitions[0], &kill, false);
            let result = if ok {
                CheckResult::Ok
            } else {
                CheckResult::Illegal
            };
            (result, LinearizationInfo::empty())
        }

        _ => check_parallel(partitions, timeout, compute_info),
    }
}

// ---------------------------------------------------------------------------
// Parallel driver
// ---------------------------------------------------------------------------

fn check_parallel<M: Model>(
    partitions: Vec<Partition<M>>,
    timeout: Option<Duration>,
    compute_info: bool,
) -> (CheckResult, LinearizationInfo<M>) {
    let total = partitions.len();

    // Extract the ops vecs upfront so we can move them into LinearizationInfo
    // without also moving the partitions (which are borrowed by the threads).
    let partition_ops: Vec<Vec<crate::model::Operation<M>>> =
        partitions.iter().map(|p| p.ops.clone()).collect();

    // Each thread sends (partition_index, ok, longest).
    // bounded(total): threads never block on send, even after the receiver
    // has stopped reading.
    let (tx, rx) = bounded::<(usize, bool, Vec<Option<Arc<Vec<usize>>>>)>(total);
    let kill = Arc::new(AtomicBool::new(false));

    thread::scope(|s| {
        for (i, partition) in partitions.iter().enumerate() {
            let tx = tx.clone();
            let kill = Arc::clone(&kill);
            s.spawn(move |_| {
                let (ok, longest) = check_single(partition, &kill, compute_info);
                let _ = tx.send((i, ok, longest)); // never blocks: bounded(total)
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
        // Per-partition longest arrays, collected only when compute_info=true.
        let mut all_longest: Vec<Option<Vec<Option<Arc<Vec<usize>>>>>> = vec![None; total];

        // ---------------------------------------------------------------
        // Phase 1 — main receive loop
        //
        // Exits when:
        //   (a) all results received,
        //   (b) a false result arrives and compute_info=false (kill+break), or
        //   (c) the timeout fires.
        // ---------------------------------------------------------------
        'recv: loop {
            if received >= total {
                break;
            }

            let msg = if let Some(ref t) = timeout_ch {
                select! {
                    recv(rx) -> m => match m {
                        Ok(v)  => Some(v),
                        Err(_) => break 'recv,
                    },
                    recv(t) -> _ => {
                        timed_out = true;
                        kill.store(true, Ordering::Relaxed);
                        None // signals timeout
                    }
                }
            } else {
                match rx.recv() {
                    Ok(v) => Some(v),
                    Err(_) => break 'recv,
                }
            };

            match msg {
                Some((i, ok, longest)) => {
                    received += 1;
                    if compute_info {
                        all_longest[i] = Some(longest);
                    }
                    if !ok {
                        ok_all = false;
                        if !compute_info {
                            // Fast termination: no more info needed.
                            kill.store(true, Ordering::Relaxed);
                            break 'recv;
                        }
                        // compute_info=true: keep running to collect all
                        // partial linearizations.
                    }
                }
                None => break 'recv, // timeout fired
            }
        }

        // ---------------------------------------------------------------
        // Phase 2 — drain remaining results when compute_info=true
        //
        // If we exited phase 1 early (timeout, or channel closed), there
        // may be results in the buffer that we haven't read yet.  Drain
        // them so we collect complete data for all partitions that
        // finished.  Since the channel is bounded(total) and threads
        // always send exactly once, this loop terminates.
        //
        // The crossbeam scope join (implicit at the end of this closure)
        // waits for any threads still running, so by the time we reach
        // the end of this closure all threads have completed and all
        // results are either already received or sitting in the buffer.
        // ---------------------------------------------------------------
        if compute_info {
            while received < total {
                match rx.recv() {
                    Ok((i, ok, longest)) => {
                        received += 1;
                        if all_longest[i].is_none() {
                            all_longest[i] = Some(longest);
                        }
                        if !ok {
                            ok_all = false;
                        }
                    }
                    Err(_) => break, // channel closed
                }
            }
        }

        // Build the info object from per-partition longest arrays.
        let info = if compute_info {
            LinearizationInfo::from_longest(
                all_longest
                    .into_iter()
                    .map(|opt| opt.unwrap_or_default())
                    .collect(),
                partition_ops,
            )
        } else {
            LinearizationInfo::empty()
        };

        let result = if !ok_all {
            CheckResult::Illegal
        } else if timed_out {
            CheckResult::Unknown
        } else {
            CheckResult::Ok
        };

        (result, info)
    })
    .unwrap()
}

// ---------------------------------------------------------------------------
// Core loop — one partition
// ---------------------------------------------------------------------------

/// Run the search on one partition.
///
/// Returns `(ok, longest)` where `ok` is `true` iff the history is
/// linearizable (or was killed before a definitive answer), and `longest`
/// holds the per-operation partial linearization data (empty when
/// `compute_info` is `false`).
fn check_single<M: Model>(
    partition: &Partition<M>,
    kill: &AtomicBool,
    compute_info: bool,
) -> (bool, Vec<Option<Arc<Vec<usize>>>>) {
    let n = partition.check_history.len();
    let mut linearizer = Linearizer::<M>::new(partition, compute_info);
    let mut current = linearizer.front();

    while current < n {
        // Cooperative cancellation — checked at the top of every iteration.
        if kill.load(Ordering::Relaxed) {
            return (false, linearizer.into_longest());
        }

        match partition.check_history[current] {
            CheckEntry::Call { .. } => {
                if let Some(next_state) = linearizer.try_linearize(current) {
                    linearizer.lift(current, next_state);
                    current = linearizer.front(); // restart from the head
                } else {
                    current = linearizer.next_of(current); // this candidate is exhausted
                }
            }
            CheckEntry::Return { .. } => {
                // A Return is at the head without its Call being lifted — every
                // candidate that could precede it has been tried.  Before
                // backtracking, record the current stack depth as the longest
                // partial linearization seen so far for each stacked operation.
                linearizer.update_longest();

                match linearizer.backtrack() {
                    Some(pos) => current = linearizer.next_of(pos),
                    None => return (false, linearizer.into_longest()),
                }
            }
        }
    }

    // Every entry was lifted: complete linearization found.
    linearizer.finalize_longest();
    (true, linearizer.into_longest())
}
